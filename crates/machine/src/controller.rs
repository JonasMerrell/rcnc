//! The machine controller: owns the backend, interpreter and planner, runs
//! programs, MDI, jogging and homing, and publishes a snapshot for the UI.

use std::collections::{BTreeMap, VecDeque};
use std::path::PathBuf;
use std::sync::{mpsc, Arc, RwLock};
use std::thread;
use std::time::Duration;

use gcode::params::num;
use gcode::{Axes, Interpreter, MotionMode, Op, Plane, Spindle, Tool, Units, AXIS_LETTERS};

use crate::backend::{AxisMove, Backend, BoardInfo, Outputs, Status};
use crate::config::MachineConfig;
use crate::planner::{op_to_moves, Generator};
use crate::sim::Clock;
use crate::probing::{self, ProbeJob, Routine, Step};
use crate::store::{self, SavedState};

/// Segments kept queued on the board while streaming (240 ms at 4 ms).
/// Small enough that feed hold reacts quickly, large enough to ride out USB hiccups.
const TARGET_QUEUE: usize = 60;
/// Moves kept buffered in the planner for look-ahead.
const LOOKAHEAD_MOVES: usize = 200;

#[derive(Debug, Clone)]
pub enum Command {
    Connect,
    Disconnect,
    /// Toggle E-stop/reset, like Mach3's Reset button.
    Reset,
    Jog {
        axis: usize,
        positive: bool,
        step: Option<f64>,
    },
    JogStop,
    /// Home one axis, or all axes in the configured order.
    Home(Option<usize>),
    Mdi(String),
    Load {
        name: String,
        text: String,
    },
    CycleStart,
    FeedHold,
    Stop,
    /// Set the active work offset so this axis reads zero.
    ZeroAxis(usize),
    SetFeedOverride(f64),
    SetRapidOverride(f64),
    SetSpindleOverride(f64),
    SetJogSpeed(f64),
    SetSoftLimits(bool),
    /// Add or replace a tool-table entry; `None` removes it.
    SetTool(u32, Option<Tool>),
    /// Set the offsets of a work coordinate system (0 = G54 .. 5 = G59).
    SetWorkOffset { system: usize, offset: Axes },
    /// Replace the machine configuration. Reconnects the board so it gets the new settings.
    SetConfig(Box<MachineConfig>),
    /// Run a probing routine. Allowed when idle, or while a program is paused (e.g. at M6).
    Probe(ProbeJob),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Disconnected,
    EStop,
    Idle,
    Jog,
    Homing,
    Running,
    Probing,
    Alarm,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub mode: Mode,
    pub machine: Axes,
    pub work: Axes,
    pub coord_system: usize,
    pub tool: u32,
    pub tool_length_offset: f64,
    pub modal: String,
    /// Line currently executing in the loaded program.
    pub line: Option<usize>,
    pub program_name: Option<String>,
    pub program: Arc<String>,
    pub feed_rate: f64,
    pub velocity: f64,
    pub hold: bool,
    pub paused: Option<String>,
    pub spindle: Spindle,
    pub rpm: f64,
    pub flood: bool,
    pub mist: bool,
    pub homed: [bool; 6],
    pub inputs: u32,
    pub probe_active: bool,
    pub estop_input: bool,
    pub feed_override: f64,
    pub rapid_override: f64,
    pub spindle_override: f64,
    pub jog_speed: f64,
    pub soft_limits: bool,
    pub board: Option<BoardInfo>,
    pub alarm: Option<String>,
    pub log: VecDeque<String>,
    pub config: MachineConfig,
    pub tools: BTreeMap<u32, Tool>,
    /// G54..G59 offsets.
    pub work_offsets: [Axes; 6],
    pub g92: Axes,
}

/// Where and what was last saved, so only changes get written.
struct Persist {
    dir: PathBuf,
    state: SavedState,
    config: MachineConfig,
    checked_at: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum HomePhase {
    Seek,
    BackOff,
    Clear,
}

#[derive(Debug)]
enum Wait {
    None,
    /// All queued motion finished and the board idle.
    Motion,
    Until(f64),
    Pause(String),
    /// Paused for a tool change; Cycle Start may touch the new tool off first.
    ToolChange(String),
    Probe,
}

enum Source {
    Program,
    Mdi,
}

struct Exec {
    source: Source,
    pending: VecDeque<(usize, Op)>,
    wait: Wait,
    /// Planner received the last move of the program/MDI.
    exhausted: bool,
    line: usize,
}

pub struct Controller {
    backend: Box<dyn Backend>,
    clock: Arc<dyn Clock>,
    config: MachineConfig,
    interp: Interpreter,
    generator: Generator,
    mode: Mode,
    status: Status,
    outputs: Outputs,
    exec: Option<Exec>,
    home: VecDeque<usize>,
    home_phase: HomePhase,
    homed: [bool; 6],
    spindle_override: f64,
    board: Option<BoardInfo>,
    alarm: Option<String>,
    log: VecDeque<String>,
    program_name: Option<String>,
    program: Arc<String>,
    persist: Option<Persist>,
    routine: Option<Routine>,
    /// The single-axis move a routine step is waiting on: (step, target).
    routine_move: Option<(RoutineMove, f64)>,
    /// Mode to return to when the routine ends.
    routine_return: Mode,
}

#[derive(Debug, Clone, Copy)]
enum RoutineMove {
    Guarded(usize),
    Free,
    Probe(usize, bool),
}

impl Controller {
    pub fn new(config: MachineConfig, backend: Box<dyn Backend>, clock: Arc<dyn Clock>) -> Self {
        let period = backend.segment_period();
        Controller {
            generator: Generator::new(config.clone(), period, [0.0; 6]),
            backend,
            clock,
            config,
            interp: Interpreter::new(""),
            mode: Mode::Disconnected,
            status: Status::default(),
            outputs: Outputs::default(),
            exec: None,
            home: VecDeque::new(),
            home_phase: HomePhase::Seek,
            homed: [false; 6],
            spindle_override: 1.0,
            board: None,
            alarm: None,
            log: VecDeque::new(),
            program_name: None,
            program: Arc::new(String::new()),
            persist: None,
            routine: None,
            routine_move: None,
            routine_return: Mode::Idle,
        }
    }

    /// Keep the tool table, work offsets and machine config in `dir`, loading
    /// what was saved there before. Changes are written within a second.
    pub fn persist_to(&mut self, dir: PathBuf) {
        match store::load::<SavedState>(&dir.join("state.json")) {
            Ok(Some(state)) => {
                self.interp.tools = state.tools.clone();
                for (&k, &v) in &state.params {
                    if num::PERSISTENT.contains(&k) {
                        self.interp.params.set(k, v);
                    }
                }
                self.interp.state.coord_system =
                    (self.interp.params.get(num::COORD_SYSTEM) as usize).clamp(1, 6) - 1;
            }
            Ok(None) => {}
            Err(e) => self.info(format!("Could not load saved state: {e}")),
        }
        self.load_macros(&dir.join("macros"));
        self.persist = Some(Persist {
            dir,
            state: self.saved_state(),
            config: self.config.clone(),
            checked_at: f64::NEG_INFINITY,
        });
    }

    /// Load user M-code macros: files named like `M150.nc` (or `.ngc`, `.txt`)
    /// hold G-code that runs in place of that M-code.
    pub fn load_macros(&mut self, dir: &std::path::Path) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        self.interp.macros.clear();
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let code = stem.strip_prefix(['M', 'm']).and_then(|n| n.parse::<u32>().ok());
            if let (Some(code), true) = (code, matches!(ext.as_str(), "nc" | "ngc" | "txt")) {
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        self.interp.macros.insert(code, text);
                    }
                    Err(e) => self.info(format!("Could not read macro {}: {e}", path.display())),
                }
            }
        }
        if !self.interp.macros.is_empty() {
            let mut codes: Vec<_> = self.interp.macros.keys().map(|c| format!("M{c}")).collect();
            codes.sort();
            self.info(format!("Macros: {}", codes.join(", ")));
        }
    }

    fn saved_state(&self) -> SavedState {
        SavedState {
            tools: self.interp.tools.clone(),
            params: self.interp.params.persistent(),
        }
    }

    fn autosave(&mut self) {
        let now = self.clock.now();
        let Some(p) = &self.persist else { return };
        if now - p.checked_at < 1.0 {
            return;
        }
        let state = self.saved_state();
        let mut errors = Vec::new();
        let p = self.persist.as_mut().unwrap();
        p.checked_at = now;
        if state != p.state {
            match store::save(&p.dir.join("state.json"), &state) {
                Ok(()) => p.state = state,
                Err(e) => errors.push(e),
            }
        }
        if self.config != p.config {
            match store::save(&p.dir.join("machine.json"), &self.config) {
                Ok(()) => p.config = self.config.clone(),
                Err(e) => errors.push(e),
            }
        }
        if !errors.is_empty() {
            // Retry later rather than flooding the log every second.
            p.checked_at = now + 30.0;
        }
        for e in errors {
            self.info(format!("Could not save settings: {e}"));
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn interpreter(&self) -> &Interpreter {
        &self.interp
    }

    fn info(&mut self, msg: impl Into<String>) {
        self.log.push_back(msg.into());
        while self.log.len() > 200 {
            self.log.pop_front();
        }
    }

    fn raise(&mut self, msg: impl Into<String>) {
        let msg = msg.into();
        let _ = self.backend.abort();
        self.generator.clear();
        self.exec = None;
        self.home.clear();
        self.outputs.spindle_on = false;
        self.outputs.flood = false;
        self.outputs.mist = false;
        self.info(format!("Alarm: {msg}"));
        self.alarm = Some(msg);
        self.mode = Mode::Alarm;
    }

    fn machine_position(&self) -> Axes {
        self.config.from_steps(&self.status.position)
    }

    fn sync_position(&mut self) {
        let pos = self.machine_position();
        self.interp.position = pos;
        self.generator.reset_position(pos);
    }

    fn is_ready(&self) -> bool {
        matches!(self.mode, Mode::Idle)
    }

    pub fn handle(&mut self, cmd: Command) {
        match cmd {
            Command::Connect => self.connect(),
            Command::Disconnect => {
                self.backend.disconnect();
                self.mode = Mode::Disconnected;
                self.board = None;
            }
            Command::Reset => self.reset(),
            Command::Jog {
                axis,
                positive,
                step,
            } => self.jog(axis, positive, step),
            Command::JogStop => {
                if self.mode == Mode::Jog {
                    let _ = self.backend.stop_axis_move();
                }
            }
            Command::Home(axis) => self.start_homing(axis),
            Command::Mdi(text) => self.mdi(&text),
            Command::Load { name, text } => {
                if matches!(self.mode, Mode::Running | Mode::Homing | Mode::Probing) {
                    self.info(format!("Cannot load {name} while the machine is running"));
                    return;
                }
                self.interp.load(&text);
                self.program = Arc::new(text);
                self.info(format!("Loaded {name}"));
                self.program_name = Some(name);
            }
            Command::CycleStart => self.cycle_start(),
            Command::FeedHold => {
                if self.mode == Mode::Running {
                    self.generator.hold = true;
                }
            }
            Command::Stop => self.stop(),
            Command::ZeroAxis(axis) => {
                if self.is_ready() {
                    let line = format!("G10 L20 P0 {}0", AXIS_LETTERS[axis]);
                    if let Err(e) = self.interp.execute_line(&line) {
                        self.info(e.to_string());
                    }
                }
            }
            Command::SetFeedOverride(v) => self.generator.feed_override = v.clamp(0.0, 3.0),
            Command::SetRapidOverride(v) => self.generator.rapid_override = v.clamp(0.0, 1.0),
            Command::SetSpindleOverride(v) => self.spindle_override = v.clamp(0.0, 2.0),
            Command::SetJogSpeed(v) => self.config.jog_speed = v.clamp(0.001, 1.0),
            Command::SetSoftLimits(on) => self.config.soft_limits = on,
            Command::SetTool(n, tool) => {
                match tool {
                    Some(t) => {
                        // Keep an active G43 offset in step with an edited length.
                        if n == self.interp.state.tool && self.interp.state.tool_length_offset != 0.0 {
                            self.interp.state.tool_length_offset = t.length;
                        }
                        self.interp.tools.insert(n, t);
                    }
                    None => {
                        self.interp.tools.remove(&n);
                    }
                }
            }
            Command::SetWorkOffset { system, offset } => {
                if system < 6 && !matches!(self.mode, Mode::Running) {
                    self.interp.params.set_coord_offset(system, offset);
                }
            }
            Command::SetConfig(config) => self.set_config(*config),
            Command::Probe(job) => self.start_probe(job),
        }
    }

    fn set_config(&mut self, config: MachineConfig) {
        if matches!(self.mode, Mode::Running | Mode::Homing | Mode::Jog | Mode::Probing) {
            self.info("Stop the machine before changing settings");
            return;
        }
        if config == self.config {
            return;
        }
        let pos = self.machine_position();
        // The board takes axis and pin settings at connect time.
        let reconnect = config.axes != self.config.axes
            || config.probe != self.config.probe
            || config.estop != self.config.estop;
        let (feed, rapid) = (self.generator.feed_override, self.generator.rapid_override);
        self.config = config;
        self.generator = Generator::new(self.config.clone(), self.backend.segment_period(), pos);
        self.generator.feed_override = feed;
        self.generator.rapid_override = rapid;
        if reconnect {
            self.info("Settings applied");
        }
        if reconnect && self.mode != Mode::Disconnected {
            // The board takes axis settings at connect time.
            self.backend.disconnect();
            self.mode = Mode::Disconnected;
            self.connect();
        }
    }

    fn start_probe(&mut self, job: ProbeJob) {
        let paused = self.mode == Mode::Running
            && matches!(&self.exec, Some(Exec { wait: Wait::Pause(_) | Wait::ToolChange(_), .. }))
            && self.generator.is_idle()
            && self.status.idle;
        if !(self.is_ready() || paused) {
            self.info("Probing needs the machine idle, or a program paused");
            return;
        }
        let Some(probe) = self.config.probe else {
            return self.info("No probe input is configured");
        };
        if probe.is_active(self.status.inputs) {
            return self.info("Probe is already touching; check its connection");
        }
        self.sync_position();
        self.info(format!("{} started", job.name()));
        self.routine = Some(probing::build(job, &self.config.probing, self.machine_position()));
        self.routine_move = None;
        self.routine_return = self.mode;
        self.mode = Mode::Probing;
        self.routine_step();
    }

    /// Advance the probing routine; called when the board is idle.
    fn routine_step(&mut self) {
        let pos = self.machine_position();
        let probe_hit = self.config.probe.is_some_and(|p| p.is_active(self.status.inputs));
        if let Some((m, target)) = self.routine_move.take() {
            match m {
                RoutineMove::Guarded(axis) if probe_hit && (pos[axis] - target).abs() > 0.01 => {
                    return self.end_routine(Err(format!(
                        "Probe touched during a {} move",
                        AXIS_LETTERS[axis]
                    )));
                }
                RoutineMove::Probe(axis, record) => {
                    if !probe_hit {
                        return self.end_routine(Err("Probe search finished without touching".into()));
                    }
                    if record {
                        self.routine.as_mut().unwrap().hits.push(pos[axis]);
                    }
                }
                _ => {}
            }
        }
        loop {
            let pos = self.machine_position();
            let Some(routine) = &mut self.routine else { return };
            let Some(step) = routine.steps.pop_front() else {
                return self.end_routine(Ok(()));
            };
            let hits = routine.hits.clone();
            let (axis, to, feed, kind) = match step {
                Step::Then(f) => {
                    match f(self, &hits) {
                        Ok(more) => {
                            let routine = self.routine.as_mut().unwrap();
                            for s in more.into_iter().rev() {
                                routine.steps.push_front(s);
                            }
                            continue;
                        }
                        Err(e) => return self.end_routine(Err(e)),
                    }
                }
                Step::MoveTo { axis, to, feed, guarded } => (axis, to, feed, guarded),
                Step::MoveBy { axis, by, feed, guarded } => (axis, pos[axis] + by, feed, guarded),
                Step::Probe { axis, by, feed, record } => {
                    match self.routine_axis_move(axis, pos[axis] + by, feed, true) {
                        Ok(()) => self.routine_move = Some((RoutineMove::Probe(axis, record), pos[axis] + by)),
                        Err(e) => return self.end_routine(Err(e)),
                    }
                    return;
                }
            };
            if (to - pos[axis]).abs() < 1e-6 {
                continue;
            }
            if let Some(violation) = {
                let mut p = pos;
                p[axis] = to;
                self.config.soft_limit_violation(&p)
            } {
                return self.end_routine(Err(format!("move outside {} soft limit", AXIS_LETTERS[violation])));
            }
            match self.routine_axis_move(axis, to, feed, kind) {
                Ok(()) => {
                    let m = if kind { RoutineMove::Guarded(axis) } else { RoutineMove::Free };
                    self.routine_move = Some((m, to));
                }
                Err(e) => return self.end_routine(Err(e)),
            }
            return;
        }
    }

    fn routine_axis_move(&mut self, axis: usize, to: f64, feed: f64, stop_on_probe: bool) -> Result<(), String> {
        let from = self.machine_position()[axis];
        let probe = self.config.probe.ok_or("no probe input configured")?;
        if stop_on_probe && probe.is_active(self.status.inputs) {
            return Err("Probe is touching before the move".into());
        }
        let a = &self.config.axes[axis];
        let spu = a.steps_per_unit;
        let m = AxisMove {
            axis,
            positive: to > from,
            steps: ((to - from).abs() * spu).round() as u32,
            velocity: (feed / 60.0).min(a.max_velocity) * spu,
            acceleration: a.acceleration * spu,
            stop_on: stop_on_probe.then_some((probe, true)),
        };
        if m.steps == 0 {
            return Ok(());
        }
        self.backend.axis_move(&m).map_err(|e| e.to_string())?;
        self.status.idle = false;
        Ok(())
    }

    fn end_routine(&mut self, result: Result<(), String>) {
        let name = self.routine.take().map(|r| r.name).unwrap_or_default();
        self.routine_move = None;
        let _ = self.backend.abort();
        self.sync_position();
        self.mode = self.routine_return;
        match result {
            Ok(()) => self.info(format!("{name} done")),
            Err(e) => {
                // Never let a program carry on after a failed touch-off.
                if let Some(exec) = &mut self.exec {
                    if matches!(exec.wait, Wait::None) {
                        exec.wait = Wait::Pause(format!("{name} failed: {e}"));
                    }
                }
                self.info(format!("{name} stopped: {e}"));
            }
        }
    }

    /// Set the active work offset so machine position `machine` on `axis` reads `work`.
    pub(crate) fn set_work_axis(&mut self, axis: usize, machine: f64, work: f64) {
        let cs = self.interp.state.coord_system;
        let mut coord = self.interp.params.coord_offset(cs);
        let others = self.interp.program_offset()[axis] - coord[axis];
        coord[axis] = machine - work - others;
        self.interp.params.set_coord_offset(cs, coord);
    }

    pub(crate) fn report(&mut self, msg: String) {
        self.info(msg);
    }

    fn connect(&mut self) {
        match self.backend.connect(&self.config) {
            Ok(info) => {
                self.info(format!("Connected to {} ({})", info.name, info.firmware));
                self.board = Some(info);
                self.mode = Mode::EStop;
                self.homed = [false; 6];
                if let Ok(s) = self.backend.poll(&self.outputs) {
                    self.status = s;
                    self.sync_position();
                }
            }
            Err(e) => self.info(format!("Connect failed: {e}")),
        }
    }

    fn reset(&mut self) {
        match self.mode {
            Mode::Disconnected => self.info("Not connected"),
            Mode::EStop | Mode::Alarm => {
                if self
                    .config
                    .estop
                    .is_some_and(|i| i.is_active(self.status.inputs))
                {
                    self.info("E-stop input is still active");
                    return;
                }
                // Like Mach3, leaving reset repeats the handshake and configuration.
                if let Err(e) = self.backend.connect(&self.config) {
                    self.info(format!("Reset failed: {e}"));
                    return;
                }
                self.alarm = None;
                self.outputs.enable = true;
                self.mode = Mode::Idle;
                self.sync_position();
            }
            _ => self.enter_estop("Reset"),
        }
    }

    fn enter_estop(&mut self, why: &str) {
        self.routine = None;
        self.routine_move = None;
        let _ = self.backend.abort();
        self.generator.clear();
        self.exec = None;
        self.home.clear();
        self.outputs = Outputs::default();
        self.mode = Mode::EStop;
        self.info(format!("E-stop: {why}"));
    }

    fn jog(&mut self, axis: usize, positive: bool, step: Option<f64>) {
        if !self.is_ready() || !self.config.axes[axis].enabled {
            return;
        }
        let a = &self.config.axes[axis];
        let spu = a.steps_per_unit;
        let mut steps: u32 = match step {
            Some(d) => (d.abs() * spu).round() as u32,
            None => 9_999_999,
        };
        if self.config.soft_limits && self.homed[axis] {
            let pos = self.machine_position()[axis];
            let room = if positive {
                a.soft_max - pos
            } else {
                pos - a.soft_min
            };
            steps = steps.min((room.max(0.0) * spu).floor() as u32);
        }
        if steps == 0 {
            return;
        }
        let m = AxisMove {
            axis,
            positive,
            steps,
            velocity: a.max_velocity * self.config.jog_speed * spu,
            acceleration: a.acceleration * spu,
            stop_on: None,
        };
        match self.backend.axis_move(&m) {
            Ok(()) => self.mode = Mode::Jog,
            Err(e) => self.info(format!("Jog failed: {e}")),
        }
    }

    fn start_homing(&mut self, axis: Option<usize>) {
        if !self.is_ready() {
            return;
        }
        self.home = match axis {
            Some(a) => [a].into(),
            None => self.config.home_order.iter().copied().collect(),
        };
        self.home
            .retain(|&a| self.config.axes[a].enabled && self.config.axes[a].home_input.is_some());
        if self.home.is_empty() {
            return;
        }
        self.mode = Mode::Homing;
        self.home_phase = HomePhase::Seek;
        self.home_move();
    }

    fn home_move(&mut self) {
        let Some(&axis) = self.home.front() else {
            return;
        };
        let a = &self.config.axes[axis];
        let spu = a.steps_per_unit;
        let input = a.home_input.unwrap();
        let seek = a.max_velocity * a.home_speed * spu;
        let toward = a.home_positive;
        let m = match self.home_phase {
            HomePhase::Seek => AxisMove {
                axis,
                positive: toward,
                steps: 999_999,
                velocity: seek,
                acceleration: a.acceleration * spu,
                stop_on: Some((input, true)),
            },
            HomePhase::BackOff => AxisMove {
                axis,
                positive: !toward,
                steps: (20.0 * spu) as u32,
                velocity: seek / 10.0,
                acceleration: a.acceleration * spu,
                stop_on: Some((input, false)),
            },
            HomePhase::Clear => AxisMove {
                axis,
                positive: !toward,
                steps: (self.config.home_clearance * spu).round() as u32,
                velocity: seek,
                acceleration: a.acceleration * spu,
                stop_on: None,
            },
        };
        if let Err(e) = self.backend.axis_move(&m) {
            self.raise(format!("Homing failed: {e}"));
        }
    }

    fn homing_step_done(&mut self) {
        let axis = *self.home.front().unwrap();
        let a = self.config.axes[axis].clone();
        let input = a.home_input.unwrap();
        match self.home_phase {
            HomePhase::Seek => {
                if !input.is_active(self.status.inputs) {
                    return self.raise(format!("{} home switch not found", AXIS_LETTERS[axis]));
                }
                self.home_phase = HomePhase::BackOff;
            }
            HomePhase::BackOff => {
                if input.is_active(self.status.inputs) {
                    return self.raise(format!(
                        "{} home switch did not release",
                        AXIS_LETTERS[axis]
                    ));
                }
                self.home_phase = HomePhase::Clear;
            }
            HomePhase::Clear => {
                let mut steps = self.status.position;
                steps[axis] = (a.home_position * a.steps_per_unit).round() as i64;
                if let Err(e) = self.backend.set_position(steps) {
                    return self.raise(format!("Homing failed: {e}"));
                }
                self.status.position = steps;
                self.homed[axis] = true;
                self.info(format!("{} homed", AXIS_LETTERS[axis]));
                self.home.pop_front();
                self.home_phase = HomePhase::Seek;
                if self.home.is_empty() {
                    self.mode = Mode::Idle;
                    self.sync_position();
                    return;
                }
            }
        }
        self.home_move();
    }

    fn mdi(&mut self, text: &str) {
        if !self.is_ready() {
            self.info("MDI needs the machine idle and out of reset");
            return;
        }
        self.sync_position();
        self.info(format!("> {text}"));
        match self.interp.execute_line(text) {
            Ok(ops) => {
                self.exec = Some(Exec {
                    source: Source::Mdi,
                    pending: ops.into_iter().map(|op| (0, op)).collect(),
                    wait: Wait::None,
                    exhausted: false,
                    line: 0,
                });
                self.mode = Mode::Running;
            }
            Err(e) => self.info(e.message),
        }
    }

    fn cycle_start(&mut self) {
        match self.mode {
            Mode::Running => {
                self.generator.hold = false;
                let tool_change = matches!(&self.exec, Some(Exec { wait: Wait::ToolChange(_), .. }));
                if tool_change && self.config.tool_change_touch_off {
                    // Touch off first; the program carries on when it succeeds.
                    self.start_probe(ProbeJob::ToolTouchOff);
                    if self.mode != Mode::Probing {
                        return;
                    }
                }
                if let Some(exec) = &mut self.exec {
                    if matches!(exec.wait, Wait::Pause(_) | Wait::ToolChange(_)) {
                        exec.wait = Wait::None;
                    }
                }
            }
            Mode::Idle => {
                if self.interp.line_count() == 0 {
                    self.info("No program loaded");
                    return;
                }
                if self.interp.is_finished() {
                    self.interp.rewind();
                }
                self.sync_position();
                self.exec = Some(Exec {
                    source: Source::Program,
                    pending: VecDeque::new(),
                    wait: Wait::None,
                    exhausted: false,
                    line: self.interp.pc(),
                });
                self.mode = Mode::Running;
            }
            _ => {}
        }
    }

    fn stop(&mut self) {
        self.routine = None;
        self.routine_move = None;
        match self.mode {
            Mode::Running | Mode::Jog | Mode::Homing | Mode::Probing => {
                let _ = self.backend.abort();
                self.generator.clear();
                self.exec = None;
                self.home.clear();
                self.outputs.spindle_on = false;
                self.outputs.flood = false;
                self.outputs.mist = false;
                self.interp.rewind();
                self.mode = Mode::Idle;
                if let Ok(s) = self.backend.poll(&self.outputs) {
                    self.status = s;
                }
                self.sync_position();
                self.info("Stopped");
            }
            _ => {}
        }
    }

    /// One controller cycle. Call every few milliseconds.
    pub fn update(&mut self) {
        self.autosave();
        if self.mode == Mode::Disconnected {
            return;
        }
        let off = self.interp.program_offset();
        let machine = self.machine_position();
        self.outputs.work_position = std::array::from_fn(|i| machine[i] - off[i]);
        self.outputs.feed_rate = self.interp.state.feed;
        match self.backend.poll(&self.outputs) {
            Ok(s) => self.status = s,
            Err(e) => {
                self.info(format!("Lost connection: {e}"));
                self.mode = Mode::Disconnected;
                return;
            }
        }
        if self.mode != Mode::EStop
            && self
                .config
                .estop
                .is_some_and(|i| i.is_active(self.status.inputs))
        {
            self.enter_estop("E-stop input");
            return;
        }
        match self.mode {
            Mode::Jog if self.status.idle => {
                self.mode = Mode::Idle;
                self.sync_position();
            }
            Mode::Homing if self.status.idle => self.homing_step_done(),
            Mode::Probing if self.status.idle => self.routine_step(),
            Mode::Running => self.run(),
            _ => {}
        }
    }

    fn run(&mut self) {
        self.fill_planner();
        if self.mode != Mode::Running {
            return;
        }
        self.stream();
        if self.mode != Mode::Running {
            return;
        }
        let now = self.clock.now();
        let Some(exec) = &mut self.exec else { return };
        let motion_done = self.generator.is_idle() && self.status.idle;
        match &exec.wait {
            Wait::Motion if motion_done => exec.wait = Wait::None,
            Wait::Until(t) if now >= *t => exec.wait = Wait::None,
            Wait::Probe if self.status.idle => {
                exec.wait = Wait::None;
                let pos = self.config.from_steps(&self.status.position);
                let tripped = self
                    .config
                    .probe
                    .is_some_and(|p| p.is_active(self.status.inputs));
                self.interp.probe_result(pos);
                self.generator.reset_position(pos);
                if !tripped {
                    self.info("Probe reached the end of its move without touching");
                }
            }
            _ => {}
        }
        let exec = self.exec.as_ref().unwrap();
        if exec.exhausted
            && exec.pending.is_empty()
            && matches!(exec.wait, Wait::None)
            && motion_done
        {
            self.finish();
        }
    }

    fn finish(&mut self) {
        if let Some(Exec {
            source: Source::Program,
            ..
        }) = &self.exec
        {
            self.info("Program finished");
        }
        self.exec = None;
        self.mode = Mode::Idle;
        self.sync_position();
    }

    /// Pull ops into the planner until an action needs the machine stopped.
    fn fill_planner(&mut self) {
        loop {
            let Some(exec) = &mut self.exec else { return };
            if !matches!(exec.wait, Wait::None)
                || self.generator.buffered_moves() >= LOOKAHEAD_MOVES
            {
                return;
            }
            if exec.pending.is_empty() {
                if exec.exhausted {
                    return;
                }
                match exec.source {
                    Source::Mdi => {
                        exec.exhausted = true;
                        return;
                    }
                    Source::Program => match self.interp.next_step() {
                        Ok(Some(step)) => {
                            let line = step.line;
                            exec.pending
                                .extend(step.ops.into_iter().map(|op| (line, op)));
                            if self.interp.is_finished() {
                                exec.exhausted = true;
                            }
                        }
                        Ok(None) => {
                            exec.exhausted = true;
                            return;
                        }
                        Err(e) => {
                            let msg = e.to_string();
                            return self.raise(msg);
                        }
                    },
                }
                continue;
            }
            let (line, op) = exec.pending.front().cloned().unwrap();
            let from = self.generator.end_position();
            let moves = op_to_moves(&op, &from, line, self.config.arc_tolerance);
            if !moves.is_empty() {
                exec.pending.pop_front();
                for m in moves {
                    if let Some(axis) = self.config.soft_limit_violation(&m.to) {
                        let msg = format!(
                            "line {}: move outside {} soft limit",
                            line + 1,
                            AXIS_LETTERS[axis]
                        );
                        return self.raise(msg);
                    }
                    self.generator.push(m);
                }
                continue;
            }
            // A non-motion op: wait until everything before it has run.
            if !(self.generator.is_idle() && self.status.idle) {
                exec.wait = Wait::Motion;
                return;
            }
            exec.pending.pop_front();
            exec.line = line;
            self.action(line, op);
        }
    }

    fn action(&mut self, line: usize, op: Op) {
        let now = self.clock.now();
        let wait = match op {
            Op::Dwell { seconds } => Wait::Until(now + seconds),
            Op::Spindle { state, rpm } => {
                if state == Spindle::Ccw && !self.config.spindle_reversible {
                    return self.raise(format!(
                        "line {}: this spindle can't run in reverse (M4 or G84 tapping); use a tapping head",
                        line + 1
                    ));
                }
                let was_on = self.outputs.spindle_on;
                let reversing = was_on && state != Spindle::Off && self.outputs.spindle_ccw != (state == Spindle::Ccw);
                let rpm = rpm * self.spindle_override;
                self.outputs.spindle_on = state != Spindle::Off;
                self.outputs.spindle_ccw = state == Spindle::Ccw;
                self.outputs.spindle_rpm = rpm;
                self.outputs.spindle_duty = (rpm / self.config.spindle_max_rpm).clamp(0.0, 1.0);
                match (was_on, self.outputs.spindle_on) {
                    (false, true) => Wait::Until(now + self.config.spindle_spin_up),
                    (true, false) => Wait::Until(now + self.config.spindle_spin_down),
                    _ if reversing => {
                        Wait::Until(now + self.config.spindle_spin_down + self.config.spindle_spin_up)
                    }
                    _ => Wait::None,
                }
            }
            Op::Coolant { flood, mist } => {
                self.outputs.flood = flood;
                self.outputs.mist = mist;
                Wait::None
            }
            Op::ToolChange { tool } => Wait::ToolChange(if self.config.tool_change_touch_off {
                format!("Load tool {tool}, then press Cycle Start to touch it off and carry on")
            } else {
                format!("Load tool {tool}, touch it off if needed, then press Cycle Start")
            }),
            Op::Pause { optional } => Wait::Pause(if optional {
                "Optional stop (M1)".into()
            } else {
                "Program paused (M0)".into()
            }),
            Op::UserMCode { code, .. } => Wait::Pause(format!(
                "M{code} macro is not available yet; press Cycle Start to skip it"
            )),
            Op::Message(text) => {
                self.info(text);
                Wait::None
            }
            Op::Probe { to, feed } => match self.probe_move(&to, feed) {
                Ok(()) => Wait::Probe,
                Err(msg) => return self.raise(format!("line {}: {msg}", line + 1)),
            },
            Op::ProgramEnd => {
                self.outputs.spindle_on = false;
                self.outputs.flood = false;
                self.outputs.mist = false;
                if let Some(exec) = &mut self.exec {
                    exec.exhausted = true;
                    exec.pending.clear();
                }
                Wait::None
            }
            Op::Rapid { .. } | Op::Linear { .. } | Op::Arc { .. } => Wait::None,
        };
        if let Some(exec) = &mut self.exec {
            exec.wait = wait;
        }
    }

    fn probe_move(&mut self, to: &Axes, feed: f64) -> Result<(), String> {
        let probe = self.config.probe.ok_or("no probe input configured")?;
        if probe.is_active(self.status.inputs) {
            return Err("probe is already touching".into());
        }
        let from = self.machine_position();
        let moving: Vec<usize> = (0..6).filter(|&i| (to[i] - from[i]).abs() > 1e-9).collect();
        let [axis] = moving[..] else {
            return Err("this board can only probe along one axis at a time".into());
        };
        let a = &self.config.axes[axis];
        let spu = a.steps_per_unit;
        let m = AxisMove {
            axis,
            positive: to[axis] > from[axis],
            steps: ((to[axis] - from[axis]).abs() * spu).round() as u32,
            velocity: (feed / 60.0).min(a.max_velocity) * spu,
            acceleration: a.acceleration * spu,
            stop_on: Some((probe, true)),
        };
        self.backend.axis_move(&m).map_err(|e| e.to_string())?;
        self.status.idle = false;
        Ok(())
    }

    /// Keep the board's queue topped up from the planner.
    fn stream(&mut self) {
        let queued = self.backend.queue_capacity() - self.status.queue_free;
        if queued >= TARGET_QUEUE || self.generator.is_empty() {
            return;
        }
        let mut segments = Vec::with_capacity(TARGET_QUEUE - queued);
        while segments.len() < TARGET_QUEUE - queued {
            match self.generator.tick() {
                Some(s) => segments.push(s),
                None => break,
            }
            if self.generator.is_empty() {
                break;
            }
        }
        if segments.is_empty() {
            return;
        }
        let last = self.generator.is_empty();
        match self.backend.push_segments(&segments, last) {
            Ok(()) => {
                // The last poll predates this push.
                self.status.idle = false;
                self.status.queue_free -= segments.len();
            }
            Err(e) => self.raise(format!("Board rejected motion: {e}")),
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        let machine = self.machine_position();
        let off = self.interp.program_offset();
        let st = &self.interp.state;
        let line = match (&self.mode, &self.exec) {
            (
                Mode::Running,
                Some(Exec {
                    source: Source::Program,
                    wait,
                    line,
                    ..
                }),
            ) => Some(
                if matches!(wait, Wait::None) && !self.generator.is_empty() {
                    self.status.line
                } else {
                    *line
                },
            ),
            _ => None,
        };
        let paused = match &self.exec {
            Some(Exec {
                wait: Wait::Pause(msg) | Wait::ToolChange(msg),
                ..
            }) => Some(msg.clone()),
            _ => None,
        };
        Snapshot {
            mode: self.mode,
            machine,
            work: std::array::from_fn(|i| machine[i] - off[i]),
            coord_system: st.coord_system,
            tool: st.tool,
            tool_length_offset: st.tool_length_offset,
            modal: modal_text(&self.interp),
            line,
            program_name: self.program_name.clone(),
            program: self.program.clone(),
            feed_rate: st.feed,
            velocity: self.generator.speed() * 60.0,
            hold: self.generator.hold,
            paused,
            spindle: if self.outputs.spindle_on {
                if self.outputs.spindle_ccw {
                    Spindle::Ccw
                } else {
                    Spindle::Cw
                }
            } else {
                Spindle::Off
            },
            rpm: self.outputs.spindle_rpm,
            flood: self.outputs.flood,
            mist: self.outputs.mist,
            homed: self.homed,
            inputs: self.status.inputs,
            probe_active: self
                .config
                .probe
                .is_some_and(|p| p.is_active(self.status.inputs)),
            estop_input: self
                .config
                .estop
                .is_some_and(|p| p.is_active(self.status.inputs)),
            feed_override: self.generator.feed_override,
            rapid_override: self.generator.rapid_override,
            spindle_override: self.spindle_override,
            jog_speed: self.config.jog_speed,
            soft_limits: self.config.soft_limits,
            board: self.board.clone(),
            alarm: self.alarm.clone(),
            log: self.log.clone(),
            config: self.config.clone(),
            tools: self.interp.tools.clone(),
            work_offsets: std::array::from_fn(|i| self.interp.params.coord_offset(i)),
            g92: self.interp.params.get_axes(num::G92),
        }
    }
}

fn modal_text(interp: &Interpreter) -> String {
    let st = &interp.state;
    let motion = match st.motion {
        MotionMode::Rapid => "G0".to_string(),
        MotionMode::Linear => "G1".into(),
        MotionMode::ArcCw => "G2".into(),
        MotionMode::ArcCcw => "G3".into(),
        MotionMode::Probe => "G31".into(),
        MotionMode::Canned(c) => format!("G{}", c / 10),
        MotionMode::Off => "G80".into(),
    };
    let plane = match st.plane {
        Plane::XY => "G17",
        Plane::ZX => "G18",
        Plane::YZ => "G19",
    };
    let units = if st.units == Units::Mm { "G21" } else { "G20" };
    let dist = if st.absolute { "G90" } else { "G91" };
    let feed = if st.inverse_time { "G93" } else { "G94" };
    let comp = match st.cutter_comp {
        410 => " G41",
        420 => " G42",
        _ => "",
    };
    format!(
        "{motion} {plane} {units} {dist} {feed} G{}{comp}",
        54 + st.coord_system
    )
}

/// Handle to a controller running on its own thread.
#[derive(Clone)]
pub struct ControllerHandle {
    tx: mpsc::Sender<Command>,
    snapshot: Arc<RwLock<Snapshot>>,
}

impl ControllerHandle {
    pub fn spawn(mut controller: Controller) -> Self {
        let (tx, rx) = mpsc::channel::<Command>();
        let snapshot = Arc::new(RwLock::new(controller.snapshot()));
        let shared = snapshot.clone();
        thread::Builder::new()
            .name("machine-controller".into())
            .spawn(move || loop {
                loop {
                    match rx.try_recv() {
                        Ok(cmd) => controller.handle(cmd),
                        Err(mpsc::TryRecvError::Empty) => break,
                        Err(mpsc::TryRecvError::Disconnected) => return,
                    }
                }
                controller.update();
                *shared.write().unwrap() = controller.snapshot();
                thread::sleep(Duration::from_millis(5));
            })
            .expect("spawn controller thread");
        ControllerHandle { tx, snapshot }
    }

    pub fn send(&self, cmd: Command) {
        let _ = self.tx.send(cmd);
    }

    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.read().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::{ManualClock, Simulator};

    fn setup() -> (Controller, Arc<ManualClock>) {
        let clock = Arc::new(ManualClock::default());
        let sim = Simulator::new(clock.clone());
        let mut c = Controller::new(MachineConfig::dmc2(), Box::new(sim), clock.clone());
        c.handle(Command::Connect);
        c.handle(Command::Reset);
        assert_eq!(c.mode(), Mode::Idle);
        (c, clock)
    }

    fn run_until_idle(c: &mut Controller, clock: &ManualClock, max_seconds: f64) {
        let mut t = 0.0;
        loop {
            clock.advance(0.005);
            c.update();
            t += 0.005;
            if c.mode() == Mode::Idle || c.mode() == Mode::Alarm {
                return;
            }
            assert!(t < max_seconds, "still {:?} after {max_seconds}s", c.mode());
        }
    }

    #[test]
    fn saves_and_restores_tools_and_offsets() {
        let dir = std::env::temp_dir().join(format!("rcnc-persist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (mut c, clock) = setup();
        c.persist_to(dir.clone());
        c.handle(Command::SetTool(4, Some(Tool { length: 12.5, diameter: 6.0, description: "6 mm".into() })));
        c.handle(Command::Mdi("G10 L2 P2 X10 Y20 Z-30".into()));
        run_until_idle(&mut c, &clock, 1.0);
        c.handle(Command::Mdi("G55".into()));
        run_until_idle(&mut c, &clock, 1.0);
        c.handle(Command::SetJogSpeed(0.5));
        clock.advance(2.0);
        c.update();

        let (mut d, _) = setup();
        d.persist_to(dir.clone());
        let s = d.snapshot();
        assert_eq!(s.tools[&4].diameter, 6.0);
        assert_eq!(s.work_offsets[1][..3], [10.0, 20.0, -30.0]);
        assert_eq!(s.coord_system, 1);
        let config: MachineConfig = store::load(&dir.join("machine.json")).unwrap().unwrap();
        assert_eq!(config.jog_speed, 0.5);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Simulator with a 60 mm block, X/Y 100..160 mm, top at Z 100, probe at X180 Y130 Z103.
    fn probing_setup() -> (Controller, Arc<ManualClock>) {
        let clock = Arc::new(ManualClock::default());
        let mut sim = Simulator::new(clock.clone());
        sim.world.block = Some(([20_000, 32_000], [20_000, 32_000], 20_000));
        let mut c = Controller::new(MachineConfig::dmc2(), Box::new(sim), clock.clone());
        c.handle(Command::Connect);
        c.handle(Command::Reset);
        c.handle(Command::Mdi("G53 G0 Z103".into()));
        run_until_idle(&mut c, &clock, 5.0);
        (c, clock)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.02
    }

    #[test]
    fn probes_z_surface_and_sets_zero() {
        let (mut c, clock) = probing_setup();
        c.handle(Command::Mdi("G53 G0 X130 Y130".into()));
        run_until_idle(&mut c, &clock, 5.0);
        c.handle(Command::Probe(ProbeJob::ZSurface));
        assert_eq!(c.mode(), Mode::Probing);
        run_until_idle(&mut c, &clock, 60.0);
        let s = c.snapshot();
        assert!(s.log.iter().any(|l| l.contains("Z surface done")), "{:?}", s.log);
        assert!(close(s.machine[2], 103.0), "{:?}", s.machine);
        assert!(close(s.work[2], 3.0), "{:?}", s.work);
    }

    #[test]
    fn probes_edge_with_stylus_radius() {
        let (mut c, clock) = probing_setup();
        c.handle(Command::Probe(ProbeJob::Edge { axis: 0, positive: false }));
        run_until_idle(&mut c, &clock, 60.0);
        let s = c.snapshot();
        assert!(s.log.iter().any(|l| l.contains("X- edge done")), "{:?}", s.log);
        // The sim's probe is a point touching at X160; the 2 mm stylus edge is 1 mm in.
        assert!(close(s.machine[0], 159.0), "{:?}", s.machine);
        assert!(close(s.work[0], 0.0), "{:?}", s.work);
        assert!(close(s.machine[2], 103.0));
    }

    #[test]
    fn tool_touch_off_sets_z_from_setter() {
        let (mut c, clock) = probing_setup();
        let mut config = MachineConfig::dmc2();
        config.probing.setter = crate::probing::ToolSetter { x: 50.0, y: 50.0, travel_z: 120.0, ..Default::default() };
        c.handle(Command::SetConfig(Box::new(config)));
        assert_eq!(c.mode(), Mode::Idle, "probe settings don't need a reconnect");
        c.handle(Command::Probe(ProbeJob::ToolTouchOff));
        run_until_idle(&mut c, &clock, 60.0);
        let s = c.snapshot();
        assert!(s.log.iter().any(|l| l.contains("Tool touch-off done")), "{:?}", s.log);
        // The table (sim probe plane) is at machine Z30, so Z30 now reads 13.31.
        assert!(close(s.work[2] - s.machine[2], 13.31 - 30.0), "{:?} {:?}", s.work, s.machine);
        assert!(close(s.machine[2], 40.0));
    }

    #[test]
    fn finds_boss_centre_and_corner() {
        let (mut c, clock) = probing_setup();
        let mut config = MachineConfig::dmc2();
        config.probing.boss_size = 60.0;
        c.handle(Command::SetConfig(Box::new(config)));
        assert_eq!(c.mode(), Mode::Idle, "probe settings don't need a reconnect");
        c.handle(Command::Mdi("G53 G0 X128 Y133".into()));
        run_until_idle(&mut c, &clock, 5.0);
        c.handle(Command::Probe(ProbeJob::BossCentre));
        run_until_idle(&mut c, &clock, 120.0);
        let s = c.snapshot();
        assert!(s.log.iter().any(|l| l.contains("Boss centre done")), "{:?}", s.log);
        assert!(close(s.machine[0], 130.0) && close(s.machine[1], 130.0), "{:?}", s.machine);
        assert!(close(s.work[0], 0.0) && close(s.work[1], 0.0));

        // Outside corner at X160 Y160, from just outside it.
        c.handle(Command::Mdi("G53 G0 X163 Y163".into()));
        run_until_idle(&mut c, &clock, 5.0);
        c.handle(Command::Probe(ProbeJob::Corner { x_positive: false, y_positive: false }));
        run_until_idle(&mut c, &clock, 120.0);
        let s = c.snapshot();
        assert!(s.log.iter().any(|l| l.contains("corner done")), "{:?}", s.log);
        assert!(close(s.machine[0], 159.0) && close(s.machine[1], 159.0), "{:?}", s.machine);
        assert!(close(s.work[0], 0.0) && close(s.work[1], 0.0));
    }

    #[test]
    fn probe_without_contact_stops_cleanly() {
        let (mut c, clock) = probing_setup();
        let mut config = MachineConfig::dmc2();
        config.probing.max_distance = 2.0;
        c.handle(Command::SetConfig(Box::new(config)));
        assert_eq!(c.mode(), Mode::Idle, "probe settings don't need a reconnect");
        c.handle(Command::Probe(ProbeJob::ZSurface));
        run_until_idle(&mut c, &clock, 30.0);
        let s = c.snapshot();
        assert!(s.log.iter().any(|l| l.contains("without touching")), "{:?}", s.log);
        assert_eq!(s.mode, Mode::Idle);
    }

    #[test]
    fn homes_all_axes() {
        let (mut c, clock) = setup();
        c.handle(Command::Home(None));
        assert_eq!(c.mode(), Mode::Homing);
        run_until_idle(&mut c, &clock, 120.0);
        let s = c.snapshot();
        assert_eq!(s.mode, Mode::Idle, "{:?}", s.alarm);
        assert_eq!(&s.homed[..3], &[true, true, true]);
        assert!((s.machine[0] - -8.0).abs() < 1e-9);
        assert!((s.machine[2] - 8.0).abs() < 1e-9);
    }

    #[test]
    fn runs_a_program() {
        let (mut c, clock) = setup();
        let start = c.snapshot().machine;
        c.handle(Command::Load {
            name: "t.nc".into(),
            text: "G91 G1 X-10 F300\nG0 Y5\nG2 X10 Y0 I5 J0 F600\nG90\nM30".into(),
        });
        c.handle(Command::CycleStart);
        assert_eq!(c.mode(), Mode::Running);
        run_until_idle(&mut c, &clock, 30.0);
        let end = c.snapshot().machine;
        assert!((end[0] - start[0]).abs() < 1e-9);
        assert!((end[1] - (start[1] + 5.0)).abs() < 1e-9);
    }

    #[test]
    fn mdi_and_zero() {
        let (mut c, clock) = setup();
        c.handle(Command::ZeroAxis(0));
        c.handle(Command::Mdi("G0 X12.5".into()));
        run_until_idle(&mut c, &clock, 10.0);
        let s = c.snapshot();
        assert!((s.work[0] - 12.5).abs() < 1e-9);
    }

    #[test]
    fn probe_stops_on_contact() {
        let (mut c, clock) = setup();
        c.handle(Command::Home(None));
        run_until_idle(&mut c, &clock, 120.0);
        // Simulated plate is 30 mm above the bottom of Z travel.
        c.handle(Command::Mdi("G53 G31 Z-150 F300".into()));
        run_until_idle(&mut c, &clock, 60.0);
        let s = c.snapshot();
        assert_eq!(s.mode, Mode::Idle);
        assert!(s.probe_active);
        assert!(s.machine[2] > -150.0);
        let tripped_at = c.interpreter().params.get(gcode::params::num::PROBE_X + 2);
        assert!((tripped_at - s.work[2]).abs() < 1e-9);
    }

    #[test]
    fn feed_hold_pauses_motion() {
        let (mut c, clock) = setup();
        c.handle(Command::Load {
            name: "t".into(),
            text: "G91 G1 X50 F1200\nM30".into(),
        });
        c.handle(Command::CycleStart);
        for _ in 0..200 {
            clock.advance(0.005);
            c.update();
        }
        c.handle(Command::FeedHold);
        for _ in 0..200 {
            clock.advance(0.005);
            c.update();
        }
        let held = c.snapshot().machine[0];
        for _ in 0..100 {
            clock.advance(0.005);
            c.update();
        }
        assert_eq!(c.snapshot().machine[0], held);
        assert_eq!(c.mode(), Mode::Running);
        c.handle(Command::CycleStart);
        run_until_idle(&mut c, &clock, 30.0);
    }

    #[test]
    fn tool_change_pauses_until_cycle_start() {
        let (mut c, clock) = setup();
        c.handle(Command::Load {
            name: "t".into(),
            text: "T2 M6\nG91 G0 X1\nM30".into(),
        });
        c.handle(Command::CycleStart);
        for _ in 0..10 {
            clock.advance(0.005);
            c.update();
        }
        assert!(c.snapshot().paused.unwrap().contains("tool 2"));
        c.handle(Command::CycleStart);
        run_until_idle(&mut c, &clock, 10.0);
        assert_eq!(c.snapshot().tool, 2);
    }

    #[test]
    fn tool_change_touches_off_then_continues() {
        let (mut c, clock) = probing_setup();
        let mut config = MachineConfig::dmc2();
        config.probing.setter = crate::probing::ToolSetter { x: 50.0, y: 50.0, travel_z: 120.0, ..Default::default() };
        config.tool_change_touch_off = true;
        c.handle(Command::SetConfig(Box::new(config)));
        assert_eq!(c.mode(), Mode::Idle, "probe settings don't need a reconnect");
        c.handle(Command::Load {
            name: "t".into(),
            text: "T3 M6\nG0 Z20\nM30".into(),
        });
        c.handle(Command::CycleStart);
        for _ in 0..10 {
            clock.advance(0.005);
            c.update();
        }
        assert!(c.snapshot().paused.unwrap().contains("touch it off"));
        c.handle(Command::CycleStart);
        assert_eq!(c.mode(), Mode::Probing);
        run_until_idle(&mut c, &clock, 60.0);
        let s = c.snapshot();
        assert!(s.log.iter().any(|l| l.contains("Tool touch-off done")), "{:?}", s.log);
        // Work Z20 is 20 - 13.31 above the setter top at machine Z30.
        assert!(close(s.machine[2], 30.0 + 20.0 - 13.31), "{:?}", s.machine);
    }

    #[test]
    fn reset_while_jogging_enters_estop() {
        let (mut c, clock) = setup();
        c.handle(Command::Jog {
            axis: 0,
            positive: true,
            step: None,
        });
        assert_eq!(c.mode(), Mode::Jog);
        clock.advance(0.1);
        c.update();
        c.handle(Command::Reset);
        assert_eq!(c.mode(), Mode::EStop);
        assert!(!c.snapshot().homed[0]);
    }
}
