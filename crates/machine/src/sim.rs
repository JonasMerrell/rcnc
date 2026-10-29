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
}

pub struct Simulator {
    clock: Arc<dyn Clock>,
    config: Option<MachineConfig>,
    world: SimWorld,
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
}

impl Simulator {
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        let world = SimWorld {
            travel: [72_000, 52_000, 33_000, 0, 0, 0],
            probe_z: 6_000,
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
        }
    }

    fn config(&self) -> Result<&MachineConfig> {
        self.config.as_ref().ok_or(BackendError::NotConnected)
    }

    fn input_active(&self, input: &Input) -> bool {
        let Some(c) = &self.config else { return false };
        if Some(*input) == c.probe {
            return self.probe_override || self.physical[2] <= self.world.probe_z;
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
            if let Some((input, want)) = a.m.stop_on {
                if self.input_active(&input) == want {
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
