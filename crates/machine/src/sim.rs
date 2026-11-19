//! A simulated board that behaves like the DMC2's stock controller: a 4 ms
//! segment queue, single-axis moves that stop on an input, home switches at
//! the ends of travel and a probe plate at a fixed Z height.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::backend::{AxisMove, Backend, BackendError, BoardInfo, Outputs, Result, Status};
use crate::config::{Input, MachineConfig};
use crate::planner::Segment;

/// Time source in seconds, so tests can run faster than real time.
pub trait Clock: Send + Sync {
    fn now(&self) -> f64;
}

pub struct RealClock(Instant);

impl RealClock {
    pub fn new() -> Self {
        RealClock(Instant::now())
    }
}

impl Default for RealClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for RealClock {
    fn now(&self) -> f64 {
        self.0.elapsed().as_secs_f64()
    }
}

#[derive(Default)]
pub struct ManualClock(Mutex<f64>);

impl ManualClock {
    pub fn advance(&self, dt: f64) {
        *self.0.lock().unwrap() += dt;
    }
}

impl Clock for ManualClock {
    fn now(&self) -> f64 {
        *self.0.lock().unwrap()
    }
}

pub const PERIOD: f64 = 0.004;
pub const QUEUE: usize = 250;

#[derive(Debug, Clone, Copy)]
struct ActiveMove {
    m: AxisMove,
    done: u32,
    speed: f64,
    stopping: bool,
    carry: f64,
}

/// Physical layout of the simulated machine.
#[derive(Debug, Clone)]
pub struct SimWorld {
    /// Physical travel per axis in steps; the switch sits at 0 or at the end.
    pub travel: [i64; 6],
    /// Probe trips when physical Z is at or below this, in steps.
    pub probe_z: i64,
    /// A block on the table the probe can touch from the side or above:
    /// X and Y ranges and top Z, physical steps.
    pub block: Option<([i64; 2], [i64; 2], i64)>,
    /// Round holes through the block: X/Y middle and radius, physical steps (X/Y share steps per mm).
    pub bores: Vec<([i64; 2], i64)>,
    /// More solid boxes the probe touches (vise jaws): X and Y ranges and top Z, physical steps.
    pub blocks: Vec<([i64; 2], [i64; 2], i64)>,
    /// Steps added to `probe_z` after each move that stops on the probe, in
    /// order: a setter that doesn't read the same twice.
    pub touch_shifts: VecDeque<i64>,
    /// Chips (or a burr): boxes the probe touches like the block, even inside a bore, each knocked
    /// off after it has been touched that many times.
    pub chips: Vec<(([i64; 2], [i64; 2], i64), u32)>,
    /// How far every box in `blocks` leans: its X range shifts by this many X steps for every Y
    /// step away from `lean_about` (a vise that isn't trammed square).
    pub lean: f64,
    pub lean_about: i64,
}

pub struct Simulator {
    clock: Arc<dyn Clock>,
    config: Option<MachineConfig>,
    pub world: SimWorld,
    /// Physical position in steps, 0 at the negative end of travel.
    physical: [i64; 6],
    /// Counter = physical + offset.
    offset: [i64; 6],
    queue: VecDeque<Segment>,
    active: Option<ActiveMove>,
    last_time: f64,
    time_debt: f64,
    line: usize,
    pub outputs: Outputs,
    /// Force the probe input active (for manual probe tests from the UI).
    pub probe_override: bool,
    pub estop_pressed: bool,
    /// Ignore the probe as the stop pin of single-axis moves (a board or pin
    /// setting that doesn't stop probes); home switches still stop homing.
    pub ignore_stop_pin: bool,
}

/// The DMC2 with the tool setter and change position where the simulator's
/// table is once it is homed: the setter is the table (the probe plane).
pub fn dmc2_sim_config() -> MachineConfig {
    let mut c = MachineConfig::dmc2();
    let world = Simulator::new(Arc::new(ManualClock::default())).world;
    let at = |c: &MachineConfig, axis: usize, mm: f64| homed_position(c, &world, axis, mm);
    let table = world.probe_z as f64 / c.axes[2].steps_per_unit;
    c.probing.setter.x = at(&c, 0, 50.0);
    c.probing.setter.y = at(&c, 1, 50.0);
    c.probing.setter.travel_z = at(&c, 2, table + 90.0);
    c.tool_change.x = at(&c, 0, 10.0);
    c.tool_change.z = at(&c, 2, table + 110.0);
    // The simulator's probe is a point.
    c.probing.stylus_diameter = 0.0;
    // Its stock stands on the table, the setter's height: the vise reference (the part's bottom) is there.
    for r in &mut c.probing.setter.references {
        if r.name == "Vise" {
            r.setter_top = 0.0;
        }
    }
    c
}

/// Machine coordinate of physical position `mm` (from the low end of travel)
/// once the axis is homed (to within the home switch's release distance).
pub fn homed_position(c: &MachineConfig, world: &SimWorld, axis: usize, mm: f64) -> f64 {
    let a = &c.axes[axis];
    let length = world.travel[axis] as f64 / a.steps_per_unit;
    if a.home_positive {
        a.home_position - length + mm
    } else {
        a.home_position + mm
    }
}

impl Simulator {
    /// Put an `x` by `y` mm block, `h` mm tall, on the table in the middle of the X/Y travel.
    pub fn add_stock(&mut self, c: &MachineConfig, x: f64, y: f64, h: f64) {
        let w = &self.world;
        let span = |axis: usize, size: f64| {
            let half = (size / 2.0 * c.axes[axis].steps_per_unit) as i64;
            [w.travel[axis] / 2 - half, w.travel[axis] / 2 + half]
        };
        self.world.block = Some((span(0, x), span(1, y), w.probe_z + (h * c.axes[2].steps_per_unit) as i64));
    }

    /// A Ø`d` bore through the block at `x`, `y` mm from its middle; `chip`: with a 0.6 mm chip on its
    /// +X wall that two touches knock off. Call after `add_stock`.
    pub fn add_bore(&mut self, c: &MachineConfig, x: f64, y: f64, d: f64, chip: bool) {
        let Some((bx, by, top)) = self.world.block else { return };
        let spu = c.axes[0].steps_per_unit;
        let st = |mm: f64| (mm * spu) as i64;
        let m = [(bx[0] + bx[1]) / 2 + st(x), (by[0] + by[1]) / 2 + st(y)];
        self.world.bores.push((m, st(d / 2.0)));
        if chip {
            self.world.chips.push((([m[0] + st(d / 2.0 - 0.6), m[0] + st(d / 2.0 + 1.0)], [m[1] - st(1.0), m[1] + st(1.0)], top), 2));
        }
    }

    pub fn new(clock: Arc<dyn Clock>) -> Self {
        let world = SimWorld {
            travel: [72_000, 52_000, 33_000, 0, 0, 0],
            probe_z: 6_000,
            block: None,
            bores: Vec::new(),
            blocks: Vec::new(),
            touch_shifts: VecDeque::new(),
            chips: Vec::new(),
            lean: 0.0,
            lean_about: 0,
        };
        let physical = [36_000, 26_000, 30_000, 0, 0, 0];
        let last_time = clock.now();
        Simulator {
            clock,
            config: None,
            world,
            physical,
            offset: [0; 6],
            queue: VecDeque::new(),
            active: None,
            last_time,
            time_debt: 0.0,
            line: 0,
            outputs: Outputs::default(),
            probe_override: false,
            estop_pressed: false,
            ignore_stop_pin: false,
        }
    }

    fn config(&self) -> Result<&MachineConfig> {
        self.config.as_ref().ok_or(BackendError::NotConnected)
    }

    fn input_active(&self, input: &Input) -> bool {
        let Some(c) = &self.config else { return false };
        if Some(*input) == c.probe {
            let p = &self.physical;
            let in_bore = self.world.bores.iter().any(|(c, r)| {
                let (dx, dy) = ((p[0] - c[0]) as f64, (p[1] - c[1]) as f64);
                dx.hypot(dy) < *r as f64
            });
            let inside = |(x, y, top): ([i64; 2], [i64; 2], i64)| (x[0]..=x[1]).contains(&p[0]) && (y[0]..=y[1]).contains(&p[1]) && p[2] <= top;
            let in_block = (!in_bore && self.world.block.is_some_and(inside))
                || self.world.blocks.iter().any(|&(x, y, top)| {
                    let shift = ((p[1] - self.world.lean_about) as f64 * self.world.lean).round() as i64;
                    inside(([x[0] + shift, x[1] + shift], y, top))
                })
                || self.world.chips.iter().any(|(b, _)| inside(*b));
            return self.probe_override || p[2] <= self.world.probe_z || in_block;
        }
        if Some(*input) == c.estop {
            return self.estop_pressed;
        }
        for (i, a) in c.axes.iter().enumerate() {
            if a.home_input == Some(*input) {
                return if a.home_positive {
                    self.physical[i] >= self.world.travel[i]
                } else {
                    self.physical[i] <= 0
                };
            }
        }
        false
    }

    fn inputs(&self) -> u32 {
        let Some(c) = &self.config else { return 0 };
        let mut bits = 0u32;
        let all = c
            .axes
            .iter()
            .filter_map(|a| a.home_input)
            .chain(c.probe)
            .chain(c.estop);
        for input in all {
            let high = self.input_active(&input) != input.active_low;
            if high {
                bits |= 1 << input.pin;
            }
        }
        bits
    }

    fn step_axis(&mut self, axis: usize, steps: i64) {
        let max = self.world.travel[axis].max(0);
        self.physical[axis] = if max > 0 {
            (self.physical[axis] + steps).clamp(-1000, max + 1000)
        } else {
            self.physical[axis] + steps
        };
    }

    fn advance(&mut self) {
        let now = self.clock.now();
        self.time_debt += now - self.last_time;
        self.last_time = now;
        while self.time_debt >= PERIOD - 1e-9 {
            self.time_debt -= PERIOD;
            if let Some(seg) = self.queue.pop_front() {
                for i in 0..6 {
                    self.step_axis(i, seg.steps[i] as i64);
                }
                self.line = seg.line;
            } else if self.active.is_some() {
                self.advance_axis_move();
            } else {
                self.time_debt = 0.0;
                break;
            }
        }
    }

    fn advance_axis_move(&mut self) {
        let Some(mut a) = self.active else { return };
        let left = a.m.steps.saturating_sub(a.done) as f64;
        let braking = a.speed * a.speed / (2.0 * a.m.acceleration);
        if a.stopping || braking >= left {
            a.speed = (a.speed - a.m.acceleration * PERIOD).max(0.0);
        } else {
            a.speed = (a.speed + a.m.acceleration * PERIOD).min(a.m.velocity);
        }
        if a.speed == 0.0 && !a.stopping && left > 0.0 {
            a.speed = (a.m.acceleration * PERIOD).min(a.m.velocity);
        }
        let exact = a.speed * PERIOD + a.carry;
        let mut n = exact.floor() as u32;
        a.carry = exact - n as f64;
        n = n.min(a.m.steps - a.done);
        let dir = if a.m.positive { 1 } else { -1 };
        for _ in 0..n {
            self.step_axis(a.m.axis, dir);
            a.done += 1;
            let probe = self.config.as_ref().and_then(|c| c.probe);
            if let Some((input, want)) = a.m.stop_on.filter(|(i, _)| !(self.ignore_stop_pin && Some(*i) == probe)) {
                if self.input_active(&input) == want {
                    if Some(input) == self.config.as_ref().and_then(|c| c.probe) {
                        self.world.probe_z += self.world.touch_shifts.pop_front().unwrap_or(0);
                        // a chip knocked off by the touches before goes now (not while it reads touching)
                        self.world.chips.retain(|(_, left)| *left > 0);
                        let p = self.physical;
                        for (b, left) in &mut self.world.chips {
                            if (b.0[0]..=b.0[1]).contains(&p[0]) && (b.1[0]..=b.1[1]).contains(&p[1]) && p[2] <= b.2 {
                                *left = left.saturating_sub(1);
                            }
                        }
                    }
                    self.active = None;
                    return;
                }
            }
        }
        if a.done >= a.m.steps || (a.stopping && a.speed == 0.0) {
            self.active = None;
        } else {
            self.active = Some(a);
        }
    }
}

impl Backend for Simulator {
    fn name(&self) -> &str {
        "Simulator"
    }

    fn connect(&mut self, config: &MachineConfig) -> Result<BoardInfo> {
        self.config = Some(config.clone());
        self.queue.clear();
        self.active = None;
        self.last_time = self.clock.now();
        Ok(BoardInfo {
            name: "Simulated DMC2 board".into(),
            firmware: "sim".into(),
        })
    }

    fn disconnect(&mut self) {
        self.config = None;
    }

    fn segment_period(&self) -> f64 {
        PERIOD
    }

    fn queue_capacity(&self) -> usize {
        QUEUE
    }

    fn poll(&mut self, outputs: &Outputs) -> Result<Status> {
        self.config()?;
        self.outputs = *outputs;
        self.advance();
        Ok(Status {
            position: std::array::from_fn(|i| self.physical[i] + self.offset[i]),
            inputs: self.inputs(),
            queue_free: QUEUE - self.queue.len(),
            idle: self.queue.is_empty() && self.active.is_none(),
            line: self.line,
        })
    }

    fn push_segments(&mut self, segments: &[Segment], _last: bool) -> Result<()> {
        self.config()?;
        if self.active.is_some() {
            return Err(BackendError::Other(
                "segments sent during an axis move".into(),
            ));
        }
        if self.queue.len() + segments.len() > QUEUE {
            return Err(BackendError::Other("segment queue overflow".into()));
        }
        if self.queue.is_empty() {
            // Start timing from now, not from when the queue last ran dry.
            self.last_time = self.clock.now();
            self.time_debt = 0.0;
        }
        self.queue.extend(segments.iter().copied());
        Ok(())
    }

    fn axis_move(&mut self, m: &AxisMove) -> Result<()> {
        self.config()?;
        if !self.queue.is_empty() || self.active.is_some() {
            return Err(BackendError::Other("axis move while busy".into()));
        }
        self.last_time = self.clock.now();
        self.time_debt = 0.0;
        self.active = Some(ActiveMove {
            m: *m,
            done: 0,
            speed: 0.0,
            stopping: false,
            carry: 0.0,
        });
        Ok(())
    }

    fn stop_axis_move(&mut self) -> Result<()> {
        if let Some(a) = &mut self.active {
            a.stopping = true;
        }
        Ok(())
    }

    fn abort(&mut self) -> Result<()> {
        self.queue.clear();
        self.active = None;
        Ok(())
    }

    fn set_position(&mut self, steps: [i64; 6]) -> Result<()> {
        self.offset = std::array::from_fn(|i| steps[i] - self.physical[i]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plays_segments_in_real_time() {
        let clock = Arc::new(ManualClock::default());
        let mut sim = Simulator::new(clock.clone());
        sim.connect(&MachineConfig::dmc2()).unwrap();
        let start = sim.poll(&Outputs::default()).unwrap().position;
        let segs = vec![
            Segment {
                steps: [4, 0, 0, 0, 0, 0],
                line: 9
            };
            100
        ];
        sim.push_segments(&segs, true).unwrap();
        clock.advance(0.2);
        let s = sim.poll(&Outputs::default()).unwrap();
        assert_eq!(s.position[0] - start[0], 200);
        assert_eq!(s.queue_free, QUEUE - 50);
        assert_eq!(s.line, 9);
        clock.advance(1.0);
        let s = sim.poll(&Outputs::default()).unwrap();
        assert!(s.idle);
    }

    #[test]
    fn axis_move_stops_on_switch() {
        let clock = Arc::new(ManualClock::default());
        let mut sim = Simulator::new(clock.clone());
        let config = MachineConfig::dmc2();
        sim.connect(&config).unwrap();
        let z_home = config.axes[2].home_input.unwrap();
        sim.axis_move(&AxisMove {
            axis: 2,
            positive: true,
            steps: 1_000_000,
            velocity: 4000.0,
            acceleration: 400_000.0,
            stop_on: Some((z_home, true)),
        })
        .unwrap();
        for _ in 0..1000 {
            clock.advance(0.01);
            if sim.poll(&Outputs::default()).unwrap().idle {
                break;
            }
        }
        let s = sim.poll(&Outputs::default()).unwrap();
        assert!(s.idle);
        assert!(z_home.is_active(s.inputs));
    }
}
