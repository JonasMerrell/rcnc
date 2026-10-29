//! The interpreter: turns parsed blocks into machine operations in machine
//! coordinates (mm and degrees), keeping modal state, offsets, tools and
//! subprogram calls the way Mach3 does.

use std::collections::{BTreeMap, HashMap};

use crate::params::{num, Params};
use crate::parse::{parse_line, Block};

/// Positions for X Y Z A B C. Linear axes in mm, rotary axes in degrees.
pub type Axes = [f64; 6];

pub const AXIS_LETTERS: [char; 6] = ['X', 'Y', 'Z', 'A', 'B', 'C'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plane {
    XY,
    ZX,
    YZ,
}

impl Plane {
    /// The two in-plane axis indexes and the normal axis index.
    pub fn axes(self) -> (usize, usize, usize) {
        match self {
            Plane::XY => (0, 1, 2),
            Plane::ZX => (2, 0, 1),
            Plane::YZ => (1, 2, 0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Units {
    Mm,
    Inch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spindle {
    Off,
    Cw,
    Ccw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionMode {
    Rapid,
    Linear,
    ArcCw,
    ArcCcw,
    Probe,
    /// Canned cycle, code*10 (810 = G81).
    Canned(u32),
    Off,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Tool {
    pub length: f64,
    pub diameter: f64,
    pub description: String,
}

/// One machine operation. All positions are machine coordinates.
#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Rapid {
        to: Axes,
    },
    /// Feed in mm/min.
    Linear {
        to: Axes,
        feed: f64,
    },
    Arc {
        to: Axes,
        /// Arc centre in machine coordinates (only the plane axes matter).
        center: Axes,
        plane: Plane,
        clockwise: bool,
        /// Extra full turns (the P word).
        turns: u32,
        feed: f64,
    },
    /// Probe (G31 / G38.2) towards `to`, stopping when the probe trips.
    Probe {
        to: Axes,
        feed: f64,
    },
    Dwell {
        seconds: f64,
    },
    Spindle {
        state: Spindle,
        rpm: f64,
    },
    Coolant {
        flood: bool,
        mist: bool,
    },
    ToolChange {
        tool: u32,
    },
    /// M0 (or M1 when `optional`).
    Pause {
        optional: bool,
    },
    /// M-code not built in, to be served by a macro.
    UserMCode {
        code: u32,
        p: Option<f64>,
        q: Option<f64>,
    },
    Message(String),
    ProgramEnd,
}

/// Ops produced by one source line.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    /// 0-based source line index.
    pub line: usize,
    pub ops: Vec<Op>,
}

#[derive(Debug, Clone, thiserror::Error)]
#[error("line {}: {message}", line + 1)]
pub struct Error {
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ModalState {
    pub motion: MotionMode,
    pub plane: Plane,
    pub units: Units,
    pub absolute: bool,
    /// Mach3 defaults to incremental IJK (G91.1).
    pub arc_absolute: bool,
    pub inverse_time: bool,
    /// 0..=5 for G54..G59.
    pub coord_system: usize,
    pub tool_length_offset: f64,
    pub return_to_r: bool,
    /// Feed in mm/min (or inverse-time value when `inverse_time`).
    pub feed: f64,
    pub rpm: f64,
    pub spindle: Spindle,
    pub flood: bool,
    pub mist: bool,
    pub tool: u32,
    pub next_tool: u32,
    canned: CannedSticky,
}

#[derive(Debug, Clone, Default)]
struct CannedSticky {
    r: f64,
    z: f64,
    q: f64,
    p: f64,
    /// Z at the start of the cycle (machine coordinates).
    initial_z: f64,
}

impl Default for ModalState {
    fn default() -> Self {
        ModalState {
            motion: MotionMode::Rapid,
            plane: Plane::XY,
            units: Units::Mm,
            absolute: true,
            arc_absolute: false,
            inverse_time: false,
            coord_system: 0,
            tool_length_offset: 0.0,
            return_to_r: false,
            feed: 0.0,
            rpm: 0.0,
            spindle: Spindle::Off,
            flood: false,
            mist: false,
            tool: 0,
            next_tool: 0,
            canned: CannedSticky::default(),
        }
    }
}

struct Frame {
    return_line: usize,
    start_line: usize,
    remaining: u32,
}

pub struct Interpreter {
    lines: Vec<String>,
    labels: HashMap<u32, usize>,
    pc: usize,
    stack: Vec<Frame>,
    finished: bool,
    pub params: Params,
    pub state: ModalState,
    /// Current machine position.
    pub position: Axes,
    pub tools: BTreeMap<u32, Tool>,
    /// Honour `/` block delete.
    pub block_delete: bool,
    /// Honour M1 optional stop.
    pub optional_stop: bool,
    /// User M-code macros: G-code run in place of `M<code>`, with P and Q in #1 and #2.
    pub macros: HashMap<u32, String>,
    macro_depth: u32,
}

/// Arc endpoint radius mismatch that is still accepted, in mm.
const ARC_TOLERANCE: f64 = 0.01;
/// Peck retract clearance for G83, in mm.
const PECK_CLEARANCE: f64 = 0.25;

impl Interpreter {
    pub fn new(program: &str) -> Self {
        let lines: Vec<String> = program.lines().map(str::to_owned).collect();
        let labels = lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| label_of(l).map(|n| (n, i)))
            .collect();
        Interpreter {
            lines,
            labels,
            pc: 0,
            stack: Vec::new(),
            finished: false,
            params: Params::default(),
            state: ModalState::default(),
            position: [0.0; 6],
            tools: BTreeMap::new(),
            block_delete: true,
            optional_stop: true,
            macros: HashMap::new(),
            macro_depth: 0,
        }
    }

    /// Replace the program, keeping machine state, offsets and tools.
    pub fn load(&mut self, program: &str) {
        let fresh = Interpreter::new(program);
        self.lines = fresh.lines;
        self.labels = fresh.labels;
        self.rewind();
    }

    pub fn rewind(&mut self) {
        self.pc = 0;
        self.stack.clear();
        self.finished = false;
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Next line to execute.
    pub fn pc(&self) -> usize {
        self.pc
    }

    pub fn is_finished(&self) -> bool {
        self.finished || self.pc >= self.lines.len()
    }

    /// Offset from program to machine coordinates for the active state.
    pub fn program_offset(&self) -> Axes {
        let coord = self.params.coord_offset(self.state.coord_system);
        let g92 = self.params.get_axes(num::G92);
        let mut off: Axes = std::array::from_fn(|i| coord[i] + g92[i]);
        off[2] += self.state.tool_length_offset;
        off
    }

    /// Current position in program (work) coordinates.
    pub fn work_position(&self) -> Axes {
        let off = self.program_offset();
        std::array::from_fn(|i| self.position[i] - off[i])
    }

    /// Report where a probe move actually stopped.
    pub fn probe_result(&mut self, machine: Axes) {
        self.position = machine;
        let work = self.work_position();
        self.params.set_axes(num::PROBE_X, work);
    }

    /// Execute one MDI line immediately (not part of the loaded program).
    pub fn execute_line(&mut self, text: &str) -> Result<Vec<Op>, Error> {
        let block = parse_line(text, &self.params).map_err(|message| Error { line: 0, message })?;
        let ops = self
            .execute(&block)
            .map_err(|message| Error { line: 0, message })?;
        Ok(ops)
    }

    /// Execute the next program line. `None` once the program has ended.
    pub fn next_step(&mut self) -> Result<Option<Step>, Error> {
        if self.is_finished() {
            return Ok(None);
        }
        let line = self.pc;
        self.pc += 1;
        let err = |message| Error { line, message };
        let block = parse_line(&self.lines[line], &self.params).map_err(err)?;
        if block.block_delete && self.block_delete {
            return Ok(Some(Step { line, ops: vec![] }));
        }
        let mut ops = self.execute(&block).map_err(err)?;
        if block.codes('M').any(|m| m == 980) {
            self.call(&block).map_err(err)?;
        }
        if block.codes('M').any(|m| m == 990) {
            self.ret();
        }
        if ops.iter().any(|op| matches!(op, Op::ProgramEnd)) {
            self.finished = true;
        }
        if self.is_finished() && !ops.iter().any(|op| matches!(op, Op::ProgramEnd)) {
            ops.push(Op::ProgramEnd);
            self.finished = true;
        }
        Ok(Some(Step { line, ops }))
    }

    /// Run the whole program, assuming every probe reaches its target.
    /// Used for preview and checks.
    pub fn run_to_end(&mut self, max_steps: usize) -> Result<Vec<Step>, Error> {
        let mut steps = Vec::new();
        while let Some(step) = self.next_step()? {
            for op in &step.ops {
                if let Op::Probe { to, .. } = op {
                    self.probe_result(*to);
                }
            }
            steps.push(step);
            if steps.len() >= max_steps {
                return Err(Error {
                    line: self.pc,
                    message: "program too long or loops forever".into(),
                });
            }
        }
        Ok(steps)
    }

    fn call(&mut self, b: &Block) -> Result<(), String> {
        let p = b.get('P').ok_or("M98 needs P")? as u32;
        let start = *self
            .labels
            .get(&p)
            .ok_or_else(|| format!("subprogram O{p} not found"))?;
        let count = b.get('L').map(|l| l as u32).unwrap_or(1);
        if count == 0 {
            return Ok(());
        }
        if self.stack.len() > 32 {
            return Err("subprogram calls nested too deeply".into());
        }
        self.stack.push(Frame {
            return_line: self.pc,
            start_line: start + 1,
            remaining: count - 1,
        });
        self.pc = start + 1;
        Ok(())
    }

    fn ret(&mut self) {
        match self.stack.last_mut() {
            Some(frame) if frame.remaining > 0 => {
                frame.remaining -= 1;
                self.pc = frame.start_line;
            }
            Some(_) => {
                let frame = self.stack.pop().unwrap();
                self.pc = frame.return_line;
            }
            None => self.finished = true,
        }
    }

    fn linear(&self, v: f64) -> f64 {
        match self.state.units {
            Units::Mm => v,
            Units::Inch => v * 25.4,
        }
    }

    fn axis_value(&self, axis: usize, v: f64) -> f64 {
        if axis < 3 {
            self.linear(v)
        } else {
            v
        }
    }

    fn axis_words(&self, b: &Block) -> [Option<f64>; 6] {
        std::array::from_fn(|i| b.get(AXIS_LETTERS[i]).map(|v| self.axis_value(i, v)))
    }

    /// Machine-coordinate target from the axis words of `b`.
    fn target(&self, b: &Block, machine_coords: bool) -> Axes {
        let words = self.axis_words(b);
        let off = self.program_offset();
        std::array::from_fn(|i| match words[i] {
            None => self.position[i],
            Some(v) if machine_coords => v,
            Some(v) if self.state.absolute => v + off[i],
            Some(v) => self.position[i] + v,
        })
    }

    fn execute(&mut self, b: &Block) -> Result<Vec<Op>, String> {
        let mut ops = Vec::new();
        let gs: Vec<u32> = b.codes('G').collect();
        let ms: Vec<u32> = b.codes('M').collect();

        for c in &b.comments {
            if let Some(msg) = c.strip_prefix("MSG,").or_else(|| c.strip_prefix("msg,")) {
                ops.push(Op::Message(msg.trim().to_string()));
            }
        }

        // Feed mode, feed, speed, tool select.
        if gs.contains(&930) {
            self.state.inverse_time = true;
        }
        if gs.contains(&940) {
            self.state.inverse_time = false;
        }
        if let Some(f) = b.get('F') {
            self.state.feed = if self.state.inverse_time {
                f
            } else {
                self.linear(f)
            };
        }
        if let Some(s) = b.get('S') {
            self.state.rpm = s;
            if self.state.spindle != Spindle::Off {
                ops.push(Op::Spindle {
                    state: self.state.spindle,
                    rpm: s,
                });
            }
        }
        if let Some(t) = b.get('T') {
            self.state.next_tool = t as u32;
        }

        // Tool change, spindle, coolant.
        if ms.contains(&60) {
            self.state.tool = self.state.next_tool;
            ops.push(Op::ToolChange {
                tool: self.state.tool,
            });
        }
        for m in &ms {
            let spindle = match m {
                30 => Some(Spindle::Cw),
                40 => Some(Spindle::Ccw),
                50 => Some(Spindle::Off),
                _ => None,
            };
            if let Some(state) = spindle {
                self.state.spindle = state;
                ops.push(Op::Spindle {
                    state,
                    rpm: self.state.rpm,
                });
            }
        }
        if ms.iter().any(|m| matches!(m, 70 | 80 | 90)) {
            if ms.contains(&70) {
                self.state.mist = true;
            }
            if ms.contains(&80) {
                self.state.flood = true;
            }
            if ms.contains(&90) {
                self.state.mist = false;
                self.state.flood = false;
            }
            ops.push(Op::Coolant {
                flood: self.state.flood,
                mist: self.state.mist,
            });
        }

        // Dwell.
        if gs.contains(&40) {
            let p = b.get('P').ok_or("G4 needs P")?;
            // Mach3 treats P as seconds by default.
            ops.push(Op::Dwell { seconds: p });
        }

        // Plane, units, cutter comp, tool length, coordinate system, distance modes.
        for g in &gs {
            match g {
                170 => self.state.plane = Plane::XY,
                180 => self.state.plane = Plane::ZX,
                190 => self.state.plane = Plane::YZ,
                200 => self.state.units = Units::Inch,
                210 => self.state.units = Units::Mm,
                400 => {}
                410 | 420 => {
                    return Err("cutter radius compensation (G41/G42) is not supported yet".into())
                }
                430 => {
                    let h = b.get('H').map(|h| h as u32).unwrap_or(self.state.tool);
                    let tool = self
                        .tools
                        .get(&h)
                        .ok_or_else(|| format!("tool {h} is not in the tool table"))?;
                    self.state.tool_length_offset = tool.length;
                }
                490 => self.state.tool_length_offset = 0.0,
                540..=590 if g % 10 == 0 => self.state.coord_system = ((g - 540) / 10) as usize,
                610 | 611 | 640 => {}
                900 => self.state.absolute = true,
                910 => self.state.absolute = false,
                901 => self.state.arc_absolute = true,
                911 => self.state.arc_absolute = false,
                980 => self.state.return_to_r = false,
                990 => self.state.return_to_r = true,
                _ => {}
            }
        }

        // Non-modal: G10, G28, G30, G92, G53.
        let machine_coords = gs.contains(&530);
        let mut axes_used_by_nonmodal = false;
        for g in &gs {
            match g {
                100 => {
                    self.g10(b)?;
                    axes_used_by_nonmodal = true;
                }
                280 | 300 => {
                    ops.extend(self.go_home(b, *g == 280));
                    axes_used_by_nonmodal = true;
                }
                281 => {
                    self.params.set_axes(num::G28_HOME, self.position);
                }
                301 => {
                    self.params.set_axes(num::G30_HOME, self.position);
                }
                920 => {
                    let words = self.axis_words(b);
                    let coord = self.params.coord_offset(self.state.coord_system);
                    let mut g92 = self.params.get_axes(num::G92);
                    for i in 0..6 {
                        if let Some(v) = words[i] {
                            let tlo = if i == 2 {
                                self.state.tool_length_offset
                            } else {
                                0.0
                            };
                            g92[i] = self.position[i] - coord[i] - tlo - v;
                        }
                    }
                    self.params.set_axes(num::G92, g92);
                    axes_used_by_nonmodal = true;
                }
                921 | 922 => self.params.set_axes(num::G92, [0.0; 6]),
                _ => {}
            }
        }

        // Motion mode.
        for g in &gs {
            let mode = match g {
                0 => MotionMode::Rapid,
                10 => MotionMode::Linear,
                20 => MotionMode::ArcCw,
                30 => MotionMode::ArcCcw,
                310 | 382 => MotionMode::Probe,
                730 | 810 | 820 | 830 | 840 | 850 | 890 => MotionMode::Canned(*g),
                800 => MotionMode::Off,
                _ => continue,
            };
            if let MotionMode::Canned(_) = mode {
                if !matches!(self.state.motion, MotionMode::Canned(_)) {
                    self.state.canned.initial_z = self.position[2];
                }
            }
            self.state.motion = mode;
        }
        check_known_codes(&gs, &ms)?;

        let has_axes = AXIS_LETTERS.iter().any(|&l| b.has(l));
        if has_axes && !axes_used_by_nonmodal {
            ops.extend(self.motion(b, machine_coords)?);
        }

        // Stops and user M-codes.
        for m in &ms {
            match m {
                0 => ops.push(Op::Pause { optional: false }),
                10 if self.optional_stop => ops.push(Op::Pause { optional: true }),
                20 | 300 => {
                    self.end_program();
                    ops.push(Op::ProgramEnd);
                }
                m if m % 10 == 0 && !is_builtin_m(m / 10) => {
                    let code = m / 10;
                    match self.macros.get(&code).cloned() {
                        Some(text) => ops.extend(self.run_macro(code, &text, b)?),
                        None => ops.push(Op::UserMCode {
                            code,
                            p: b.get('P'),
                            q: b.get('Q'),
                        }),
                    }
                }
                _ => {}
            }
        }

        for &(index, value) in &b.assignments {
            self.params.set(index, value);
        }
        self.params
            .set(num::COORD_SYSTEM, self.state.coord_system as f64 + 1.0);
        Ok(ops)
    }

    fn end_program(&mut self) {
        // M2/M30 reset these modes, as in Mach3.
        self.params.set_axes(num::G92, [0.0; 6]);
        self.state.plane = Plane::XY;
        self.state.absolute = true;
        self.state.inverse_time = false;
        self.state.motion = MotionMode::Linear;
        self.state.spindle = Spindle::Off;
        self.state.flood = false;
        self.state.mist = false;
    }

    /// Expand a user M-code macro in place. Macros are straight-line G-code
    /// (no subprograms) and may call other macros, up to a few levels deep.
    fn run_macro(&mut self, code: u32, text: &str, b: &Block) -> Result<Vec<Op>, String> {
        if self.macro_depth >= 8 {
            return Err(format!("M{code}: macros nested too deeply"));
        }
        self.params.set(1, b.get('P').unwrap_or(0.0));
        self.params.set(2, b.get('Q').unwrap_or(0.0));
        self.macro_depth += 1;
        let mut ops = Vec::new();
        let mut result = Ok(());
        for (i, line) in text.lines().enumerate() {
            let step = parse_line(line, &self.params).and_then(|block| {
                if block.block_delete && self.block_delete {
                    return Ok(Vec::new());
                }
                if block.label.is_some() || block.codes('M').any(|m| matches!(m, 980 | 990 | 300 | 20)) {
                    return Err("subprograms and program end are not allowed in macros".into());
                }
                self.execute(&block)
            });
            match step {
                Ok(o) => ops.extend(o),
                Err(e) => {
                    result = Err(format!("M{code} macro line {}: {e}", i + 1));
                    break;
                }
            }
        }
        self.macro_depth -= 1;
        result.map(|()| ops)
    }

    fn g10(&mut self, b: &Block) -> Result<(), String> {
        let l = b.get('L').ok_or("G10 needs L")? as u32;
        let p = b.get('P').ok_or("G10 needs P")?;
        match l {
            1 | 10 => {
                let tool = p as u32;
                let mut entry = self.tools.get(&tool).cloned().unwrap_or_default();
                if let Some(z) = b.get('Z') {
                    let z = self.linear(z);
                    entry.length = if l == 1 {
                        z
                    } else {
                        // L10: length such that the current position reads Z.
                        let coord = self.params.coord_offset(self.state.coord_system);
                        let g92 = self.params.get_axes(num::G92);
                        self.position[2] - coord[2] - g92[2] - z
                    };
                }
                if let Some(r) = b.get('R') {
                    entry.diameter = 2.0 * self.linear(r);
                }
                if let Some(d) = b.get('D') {
                    entry.diameter = self.linear(d);
                }
                self.tools.insert(tool, entry);
            }
            2 | 20 => {
                let index = match p as usize {
                    0 => self.state.coord_system,
                    n @ 1..=6 => n - 1,
                    _ => return Err("G10 L2/L20 P must be 0..6".into()),
                };
                let words = self.axis_words(b);
                let mut off = self.params.coord_offset(index);
                let g92 = self.params.get_axes(num::G92);
                for i in 0..6 {
                    if let Some(v) = words[i] {
                        off[i] = if l == 2 {
                            v
                        } else {
                            let tlo = if i == 2 {
                                self.state.tool_length_offset
                            } else {
                                0.0
                            };
                            self.position[i] - g92[i] - tlo - v
                        };
                    }
                }
                self.params.set_coord_offset(index, off);
            }
            _ => return Err(format!("G10 L{l} is not supported")),
        }
        Ok(())
    }

    fn go_home(&mut self, b: &Block, g28: bool) -> Vec<Op> {
        let home = self
            .params
            .get_axes(if g28 { num::G28_HOME } else { num::G30_HOME });
        let words = self.axis_words(b);
        let any = words.iter().any(Option::is_some);
        let mut ops = Vec::new();
        if any {
            let mid = self.target(b, false);
            if mid != self.position {
                ops.push(Op::Rapid { to: mid });
                self.position = mid;
            }
        }
        let mut to = self.position;
        for i in 0..6 {
            if !any || words[i].is_some() {
                to[i] = home[i];
            }
        }
        ops.push(Op::Rapid { to });
        self.position = to;
        ops
    }

    fn feed_for(&self, length: f64) -> Result<f64, String> {
        if self.state.inverse_time {
            if self.state.feed <= 0.0 {
                return Err("inverse time feed needs F on the line".into());
            }
            return Ok(length * self.state.feed);
        }
        if self.state.feed <= 0.0 {
            return Err("feed rate is zero; set F".into());
        }
        Ok(self.state.feed)
    }

    fn motion(&mut self, b: &Block, machine_coords: bool) -> Result<Vec<Op>, String> {
        let to = self.target(b, machine_coords);
        let mut ops = Vec::new();
        match self.state.motion {
            MotionMode::Off => return Err("axis words with no motion mode (G80 active)".into()),
            MotionMode::Rapid => ops.push(Op::Rapid { to }),
            MotionMode::Linear => {
                let feed = self.feed_for(distance(&self.position, &to))?;
                ops.push(Op::Linear { to, feed });
            }
            MotionMode::Probe => {
                let feed = self.feed_for(distance(&self.position, &to))?;
                ops.push(Op::Probe { to, feed });
            }
            MotionMode::ArcCw | MotionMode::ArcCcw => {
                ops.push(self.arc(b, to, self.state.motion == MotionMode::ArcCw)?);
            }
            MotionMode::Canned(code) => return self.canned(b, code),
        }
        self.position = to;
        Ok(ops)
    }

    fn arc(&self, b: &Block, to: Axes, clockwise: bool) -> Result<Op, String> {
        let plane = self.state.plane;
        let (a0, a1, _) = plane.axes();
        let from = self.position;
        let mut center = from;
        if let Some(r) = b.get('R') {
            let r = self.linear(r);
            let dx = to[a0] - from[a0];
            let dy = to[a1] - from[a1];
            let chord = dx.hypot(dy);
            if chord < 1e-9 {
                return Err("R-format arc needs distinct start and end points".into());
            }
            let half = chord / 2.0;
            if half > r.abs() + ARC_TOLERANCE {
                return Err("arc radius too small for its endpoints".into());
            }
            let h = (r * r - half * half).max(0.0).sqrt();
            // Centre is to the right of the chord for CW minor arcs.
            let side = if clockwise == (r > 0.0) { -1.0 } else { 1.0 };
            center[a0] = from[a0] + dx / 2.0 - side * h * dy / chord;
            center[a1] = from[a1] + dy / 2.0 + side * h * dx / chord;
        } else {
            let letters = match plane {
                Plane::XY => ('I', 'J'),
                Plane::ZX => ('K', 'I'),
                Plane::YZ => ('J', 'K'),
            };
            let o0 = b.get(letters.0).map(|v| self.linear(v));
            let o1 = b.get(letters.1).map(|v| self.linear(v));
            if o0.is_none() && o1.is_none() {
                return Err("arc needs R or centre offsets".into());
            }
            if self.state.arc_absolute {
                let off = self.program_offset();
                center[a0] = o0.map(|v| v + off[a0]).unwrap_or(from[a0]);
                center[a1] = o1.map(|v| v + off[a1]).unwrap_or(from[a1]);
            } else {
                center[a0] = from[a0] + o0.unwrap_or(0.0);
                center[a1] = from[a1] + o1.unwrap_or(0.0);
            }
            let r0 = (from[a0] - center[a0]).hypot(from[a1] - center[a1]);
            let r1 = (to[a0] - center[a0]).hypot(to[a1] - center[a1]);
            if (r0 - r1).abs() > ARC_TOLERANCE + 0.001 * r0 {
                return Err(format!(
                    "arc end radius {r1:.4} differs from start radius {r0:.4}"
                ));
            }
        }
        let turns = b
            .get('P')
            .map(|p| (p as u32).saturating_sub(1))
            .unwrap_or(0);
        let length = arc_length(&from, &to, &center, plane, clockwise, turns);
        let feed = self.feed_for(length)?;
        Ok(Op::Arc {
            to,
            center,
            plane,
            clockwise,
            turns,
            feed,
        })
    }

    fn canned(&mut self, b: &Block, code: u32) -> Result<Vec<Op>, String> {
        if self.state.plane != Plane::XY {
            return Err("canned cycles are only supported in the XY plane (G17)".into());
        }
        let feed = self.feed_for(0.0)?;
        let off = self.program_offset();
        let st = &mut self.state.canned;
        let lin = |v: f64| {
            if self.state.units == Units::Inch {
                v * 25.4
            } else {
                v
            }
        };
        if let Some(r) = b.get('R') {
            st.r = lin(r);
        }
        if let Some(z) = b.get('Z') {
            st.z = lin(z);
        }
        if let Some(q) = b.get('Q') {
            st.q = lin(q).abs();
        }
        if let Some(p) = b.get('P') {
            st.p = p;
        }
        let st = st.clone();
        let repeats = b.get('L').map(|l| l as u32).unwrap_or(1);
        let (r_z, bottom) = if self.state.absolute {
            (st.r + off[2], st.z + off[2])
        } else {
            let r = st.initial_z + st.r;
            (r, r + st.z)
        };
        let clear_z = if self.state.return_to_r {
            r_z
        } else {
            st.initial_z.max(r_z)
        };
        if (code == 830 || code == 730) && st.q <= 0.0 {
            return Err("G73/G83 need a positive Q".into());
        }

        let mut ops = Vec::new();
        let mut pos = self.position;
        let words = self.axis_words(b);
        for _ in 0..repeats {
            for i in 0..2 {
                if let Some(v) = words[i] {
                    pos[i] = if self.state.absolute {
                        v + off[i]
                    } else {
                        pos[i] + v
                    };
                }
            }
            let rapid = |pos: &mut Axes, z: f64, ops: &mut Vec<Op>| {
                pos[2] = z;
                ops.push(Op::Rapid { to: *pos });
            };
            if pos[2] < r_z {
                rapid(&mut pos, r_z, &mut ops);
            }
            ops.push(Op::Rapid { to: pos });
            rapid(&mut pos, r_z, &mut ops);
            let feed_to = |pos: &mut Axes, z: f64, ops: &mut Vec<Op>| {
                pos[2] = z;
                ops.push(Op::Linear { to: *pos, feed });
            };
            match code {
                810 | 820 => {
                    feed_to(&mut pos, bottom, &mut ops);
                    if code == 820 && st.p > 0.0 {
                        ops.push(Op::Dwell { seconds: st.p });
                    }
                }
                830 | 730 => {
                    let mut depth = r_z;
                    while depth > bottom + 1e-9 {
                        let next = (depth - st.q).max(bottom);
                        if code == 830 && depth < r_z {
                            rapid(&mut pos, depth + PECK_CLEARANCE, &mut ops);
                        }
                        feed_to(&mut pos, next, &mut ops);
                        depth = next;
                        if depth > bottom + 1e-9 {
                            if code == 830 {
                                rapid(&mut pos, r_z, &mut ops);
                            } else {
                                rapid(&mut pos, depth + PECK_CLEARANCE, &mut ops);
                            }
                        }
                    }
                }
                840 => {
                    // Tapping: feed in, reverse the spindle, feed out, restore it.
                    let rpm = self.state.rpm;
                    feed_to(&mut pos, bottom, &mut ops);
                    if st.p > 0.0 {
                        ops.push(Op::Dwell { seconds: st.p });
                    }
                    ops.push(Op::Spindle { state: Spindle::Ccw, rpm });
                    feed_to(&mut pos, r_z, &mut ops);
                    ops.push(Op::Spindle { state: self.state.spindle, rpm });
                }
                850 | 890 => {
                    feed_to(&mut pos, bottom, &mut ops);
                    if code == 890 && st.p > 0.0 {
                        ops.push(Op::Dwell { seconds: st.p });
                    }
                    feed_to(&mut pos, r_z, &mut ops);
                }
                _ => unreachable!(),
            }
            rapid(&mut pos, clear_z, &mut ops);
        }
        self.position = pos;
        Ok(ops)
    }
}

fn label_of(line: &str) -> Option<u32> {
    let t = line.trim_start();
    let rest = t.strip_prefix('O').or_else(|| t.strip_prefix('o'))?;
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn is_builtin_m(m: u32) -> bool {
    matches!(m, 0..=9 | 30 | 48 | 49 | 98 | 99)
}

fn check_known_codes(gs: &[u32], _ms: &[u32]) -> Result<(), String> {
    const KNOWN: &[u32] = &[
        0, 10, 20, 30, 40, 100, 170, 180, 190, 200, 210, 280, 281, 300, 301, 310, 382, 400, 410,
        420, 430, 490, 530, 540, 550, 560, 570, 580, 590, 610, 611, 640, 730, 800, 810, 820, 830,
        840, 850, 890, 900, 901, 910, 911, 920, 921, 922, 930, 940, 980, 990,
    ];
    for g in gs {
        if !KNOWN.contains(g) {
            let code = if g % 10 == 0 {
                format!("G{}", g / 10)
            } else {
                format!("G{}.{}", g / 10, g % 10)
            };
            return Err(format!("{code} is not supported"));
        }
    }
    Ok(())
}

pub fn distance(a: &Axes, b: &Axes) -> f64 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f64>().sqrt()
}

/// Signed sweep angle in radians (negative for clockwise).
pub fn arc_sweep(
    from: &Axes,
    to: &Axes,
    center: &Axes,
    plane: Plane,
    clockwise: bool,
    turns: u32,
) -> f64 {
    let (a0, a1, _) = plane.axes();
    let start = (from[a1] - center[a1]).atan2(from[a0] - center[a0]);
    let end = (to[a1] - center[a1]).atan2(to[a0] - center[a0]);
    let tau = std::f64::consts::TAU;
    let mut sweep = end - start;
    if clockwise {
        if sweep >= -1e-9 {
            sweep -= tau;
        }
        sweep - tau * turns as f64
    } else {
        if sweep <= 1e-9 {
            sweep += tau;
        }
        sweep + tau * turns as f64
    }
}

pub fn arc_length(
    from: &Axes,
    to: &Axes,
    center: &Axes,
    plane: Plane,
    clockwise: bool,
    turns: u32,
) -> f64 {
    let (a0, a1, n) = plane.axes();
    let r = (from[a0] - center[a0]).hypot(from[a1] - center[a1]);
    let sweep = arc_sweep(from, to, center, plane, clockwise, turns);
    (r * sweep).hypot(to[n] - from[n])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(program: &str) -> (Interpreter, Vec<Op>) {
        let mut i = Interpreter::new(program);
        let steps = i.run_to_end(10_000).unwrap();
        let ops = steps.into_iter().flat_map(|s| s.ops).collect();
        (i, ops)
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn rapid_and_feed_with_units() {
        let (_, ops) = run("G20 G0 X1\nG1 Y1 F10\nM30");
        assert_eq!(
            ops[0],
            Op::Rapid {
                to: [25.4, 0.0, 0.0, 0.0, 0.0, 0.0]
            }
        );
        assert_eq!(
            ops[1],
            Op::Linear {
                to: [25.4, 25.4, 0.0, 0.0, 0.0, 0.0],
                feed: 254.0
            }
        );
        assert_eq!(ops[2], Op::ProgramEnd);
    }

    #[test]
    fn incremental_moves() {
        let (i, _) = run("G91 G0 X5 Y5\nX5\nM30");
        assert!(close(i.position[0], 10.0));
        assert!(close(i.position[1], 5.0));
    }

    #[test]
    fn work_offsets_and_g92() {
        let (i, ops) = run("G10 L2 P1 X100 Y50\nG54 G0 X0 Y0\nG92 X10\nG0 X0\nM30");
        assert_eq!(
            ops[0],
            Op::Rapid {
                to: [100.0, 50.0, 0.0, 0.0, 0.0, 0.0]
            }
        );
        assert_eq!(
            ops[1],
            Op::Rapid {
                to: [90.0, 50.0, 0.0, 0.0, 0.0, 0.0]
            }
        );
        // M30 clears G92.
        assert_eq!(i.params.get_axes(num::G92), [0.0; 6]);
    }

    #[test]
    fn g10_l20_sets_offset_from_current_position() {
        let (i, _) = run("G53 G0 X30 Y40\nG10 L20 P1 X0 Y0\nM30");
        assert_eq!(i.params.coord_offset(0)[0], 30.0);
        assert_eq!(i.params.coord_offset(0)[1], 40.0);
    }

    #[test]
    fn machine_coords_g53() {
        let (_, ops) = run("G10 L2 P1 X100\nG53 G0 X5\nM30");
        assert_eq!(
            ops[0],
            Op::Rapid {
                to: [5.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            }
        );
    }

    #[test]
    fn tool_length_offset() {
        let (_, ops) = run("G10 L1 P3 Z25\nT3 M6\nG43 H3\nG0 Z0\nM30");
        assert_eq!(ops[0], Op::ToolChange { tool: 3 });
        assert_eq!(
            ops[1],
            Op::Rapid {
                to: [0.0, 0.0, 25.0, 0.0, 0.0, 0.0]
            }
        );
    }

    #[test]
    fn arc_ij_incremental_default() {
        let (_, ops) = run("G0 X10 Y0\nG3 X0 Y10 I-10 J0 F100\nM30");
        match &ops[1] {
            Op::Arc {
                center, clockwise, ..
            } => {
                assert!(close(center[0], 0.0) && close(center[1], 0.0));
                assert!(!clockwise);
            }
            other => panic!("expected arc, got {other:?}"),
        }
    }

    #[test]
    fn arc_r_format_minor_and_major() {
        let (_, ops) = run("G0 X0 Y0\nG2 X10 Y0 R5 F100\nM30");
        let Op::Arc { center, .. } = &ops[1] else {
            panic!()
        };
        assert!(close(center[0], 5.0) && close(center[1], 0.0));

        let (_, ops) = run("G0 X0 Y0\nG2 X10 Y0 R10 F100\nM30");
        let Op::Arc { center, .. } = &ops[1] else {
            panic!()
        };
        // Minor CW arc from (0,0) to (10,0): centre below the chord.
        assert!(center[1] < 0.0);

        let (_, ops) = run("G0 X0 Y0\nG2 X10 Y0 R-10 F100\nM30");
        let Op::Arc { center, .. } = &ops[1] else {
            panic!()
        };
        assert!(center[1] > 0.0);
    }

    #[test]
    fn arc_radius_mismatch_is_error() {
        let mut i = Interpreter::new("G0 X10\nG2 X0 Y5 I-10 F100");
        i.next_step().unwrap();
        assert!(i.next_step().is_err());
    }

    #[test]
    fn arc_sweep_direction() {
        let from = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let to = [0.0, 1.0, 0.0, 0.0, 0.0, 0.0];
        let c = [0.0; 6];
        let ccw = arc_sweep(&from, &to, &c, Plane::XY, false, 0);
        let cw = arc_sweep(&from, &to, &c, Plane::XY, true, 0);
        assert!(close(ccw, std::f64::consts::FRAC_PI_2));
        assert!(close(cw, -3.0 * std::f64::consts::FRAC_PI_2));
        // Full circle when start equals end.
        assert!(close(
            arc_sweep(&from, &from, &c, Plane::XY, false, 0),
            std::f64::consts::TAU
        ));
    }

    #[test]
    fn feed_required() {
        let mut i = Interpreter::new("G1 X10");
        assert!(i.next_step().is_err());
    }

    #[test]
    fn canned_drill_g81_g98() {
        let (i, ops) = run("G0 Z10\nG98 G81 X5 Y5 Z-3 R2 F100\nX10\nG80\nM30");
        let zs: Vec<f64> = ops
            .iter()
            .filter_map(|o| match o {
                Op::Rapid { to } | Op::Linear { to, .. } => Some(to[2]),
                _ => None,
            })
            .collect();
        // Z10, XY at Z10, down to R2, feed to -3, back to 10; then second hole.
        assert_eq!(zs, vec![10.0, 10.0, 2.0, -3.0, 10.0, 10.0, 2.0, -3.0, 10.0]);
        assert!(close(i.position[0], 10.0));
    }

    #[test]
    fn tapping_g84_reverses_spindle() {
        let (_, ops) = run("M3 S500\nG0 Z5\nG98 G84 X0 Y0 Z-6 R1 F500\nG80\nM30");
        let spindle: Vec<Spindle> = ops
            .iter()
            .filter_map(|o| match o {
                Op::Spindle { state, .. } => Some(*state),
                _ => None,
            })
            .collect();
        assert_eq!(spindle, vec![Spindle::Cw, Spindle::Ccw, Spindle::Cw]);
        let ccw = ops.iter().position(|o| matches!(o, Op::Spindle { state: Spindle::Ccw, .. })).unwrap();
        assert!(matches!(ops[ccw - 1], Op::Linear { to, .. } if to[2] == -6.0));
    }

    #[test]
    fn user_macros_expand_in_place() {
        let mut i = Interpreter::new("G0 X1\nM150 P7\nG0 X3\nM30");
        i.macros.insert(150, "G0 X#1\n(MSG,from macro)".into());
        let ops: Vec<Op> = std::iter::from_fn(|| i.next_step().unwrap()).flat_map(|s| s.ops).collect();
        assert!(matches!(ops[1], Op::Rapid { to } if to[0] == 7.0));
        assert_eq!(ops[2], Op::Message("from macro".into()));
        assert!(matches!(ops[3], Op::Rapid { to } if to[0] == 3.0));
    }

    #[test]
    fn canned_peck_g83() {
        let (_, ops) = run("G0 Z5\nG99 G83 X0 Y0 Z-6 R1 Q3 F100\nG80\nM30");
        let feeds: Vec<f64> = ops
            .iter()
            .filter_map(|o| match o {
                Op::Linear { to, .. } => Some(to[2]),
                _ => None,
            })
            .collect();
        assert_eq!(feeds, vec![-2.0, -5.0, -6.0]);
        let Op::Rapid { to } = &ops[ops.len() - 2] else {
            panic!()
        };
        assert_eq!(to[2], 1.0);
    }

    #[test]
    fn subprogram_call_with_repeats() {
        let program = "G0 X0\nM98 P100 L3\nM30\nO100\nG91 G0 X1\nG90\nM99";
        let (i, _) = run(program);
        assert!(close(i.position[0], 3.0));
    }

    #[test]
    fn parameters_flow_between_lines() {
        let (i, _) = run("#100=12.5\nG0 X#100\n#101=[#100*2]\nG0 Y#101\nM30");
        assert!(close(i.position[0], 12.5));
        assert!(close(i.position[1], 25.0));
    }

    #[test]
    fn probe_sets_params() {
        let mut i = Interpreter::new("G10 L2 P1 Z-50\nG31 Z-60 F50\nM30");
        i.next_step().unwrap();
        let step = i.next_step().unwrap().unwrap();
        assert_eq!(
            step.ops,
            vec![Op::Probe {
                to: [0.0, 0.0, -110.0, 0.0, 0.0, 0.0],
                feed: 50.0
            }]
        );
        i.probe_result([0.0, 0.0, -107.5, 0.0, 0.0, 0.0]);
        assert!(close(i.params.get(num::PROBE_X + 2), -57.5));
    }

    #[test]
    fn user_m_codes_and_spindle() {
        let (_, ops) = run("S12000 M3\nM8\nM917\nM5 M9\nM30");
        assert_eq!(
            ops[0],
            Op::Spindle {
                state: Spindle::Cw,
                rpm: 12000.0
            }
        );
        assert_eq!(
            ops[1],
            Op::Coolant {
                flood: true,
                mist: false
            }
        );
        assert_eq!(
            ops[2],
            Op::UserMCode {
                code: 917,
                p: None,
                q: None
            }
        );
        assert_eq!(
            ops[3],
            Op::Spindle {
                state: Spindle::Off,
                rpm: 12000.0
            }
        );
    }

    #[test]
    fn unsupported_g_code_is_error() {
        let mut i = Interpreter::new("G12 I5");
        assert!(i.next_step().is_err());
    }

    #[test]
    fn g28_via_intermediate() {
        let (_, ops) = run("G0 X10 Z5\nG28 Z10\nM30");
        assert_eq!(
            ops[1],
            Op::Rapid {
                to: [10.0, 0.0, 10.0, 0.0, 0.0, 0.0]
            }
        );
        assert_eq!(
            ops[2],
            Op::Rapid {
                to: [10.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            }
        );
    }
}
