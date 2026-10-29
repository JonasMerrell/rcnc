//! Trajectory planning: turns straight moves into fixed-period step segments.
//!
//! The stock DMC2 board plays back a queue of per-axis step counts, one entry
//! per 4 ms period, so all acceleration planning happens here. The generator
//! works online: each tick it picks the fastest speed that still lets it stop
//! (or slow to the next corner's limit) within the moves it has buffered.
//! That makes feed hold and feed override fall out naturally.

use std::collections::VecDeque;

use gcode::{interp::arc_sweep, Axes, Op};

use crate::config::MachineConfig;

/// One straight move, positions in machine units.
#[derive(Debug, Clone, PartialEq)]
pub struct Move {
    pub to: Axes,
    /// Requested speed in mm/s; `f64::INFINITY` for rapids.
    pub speed: f64,
    pub rapid: bool,
    /// Source line, reported back while the move executes.
    pub line: usize,
    /// The previous move belongs to the same G-code move (arc chords), so
    /// the corner may be blended even in exact-stop mode.
    pub continues: bool,
}

#[derive(Debug, Clone)]
struct Planned {
    from: Axes,
    to: Axes,
    length: f64,
    unit: Axes,
    max_speed: f64,
    accel: f64,
    rapid: bool,
    line: usize,
    /// Highest allowed speed at the end of this move.
    exit_limit: f64,
}

/// One period of motion: per-axis step deltas and the line being executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub steps: [i32; 6],
    pub line: usize,
}

pub struct Generator {
    config: MachineConfig,
    period: f64,
    moves: VecDeque<Planned>,
    /// Distance travelled along the front move.
    progress: f64,
    speed: f64,
    /// Position of the last emitted sample.
    position: Axes,
    steps: [i64; 6],
    pub feed_override: f64,
    pub rapid_override: f64,
    pub hold: bool,
}

impl Generator {
    pub fn new(config: MachineConfig, period: f64, start: Axes) -> Self {
        let steps = config.to_steps(&start);
        Generator {
            config,
            period,
            moves: VecDeque::new(),
            progress: 0.0,
            speed: 0.0,
            position: start,
            steps,
            feed_override: 1.0,
            rapid_override: 1.0,
            hold: false,
        }
    }

    /// End point of the last buffered move (where new moves start).
    pub fn end_position(&self) -> Axes {
        self.moves.back().map(|m| m.to).unwrap_or(self.position)
    }

    pub fn position(&self) -> Axes {
        self.position
    }

    pub fn speed(&self) -> f64 {
        self.speed
    }

    pub fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    pub fn buffered_moves(&self) -> usize {
        self.moves.len()
    }

    /// Buffered path length still to run.
    pub fn remaining_length(&self) -> f64 {
        self.moves.iter().map(|m| m.length).sum::<f64>() - self.progress
    }

    /// Stopped with nothing left to run.
    pub fn is_idle(&self) -> bool {
        self.moves.is_empty() && self.speed == 0.0
    }

    /// Drop all buffered motion and stop at the current position.
    pub fn clear(&mut self) {
        self.moves.clear();
        self.progress = 0.0;
        self.speed = 0.0;
    }

    /// Restart from a known position (after jogging, homing or an abort).
    pub fn reset_position(&mut self, pos: Axes) {
        self.clear();
        self.position = pos;
        self.steps = self.config.to_steps(&pos);
    }

    pub fn push(&mut self, m: Move) {
        let from = self.end_position();
        let delta: Axes = std::array::from_fn(|i| m.to[i] - from[i]);
        let length = delta.iter().map(|d| d * d).sum::<f64>().sqrt();
        if length < 1e-9 {
            return;
        }
        let unit: Axes = std::array::from_fn(|i| delta[i] / length);
        let mut max_speed = m.speed;
        let mut accel = f64::INFINITY;
        for (i, a) in self.config.axes.iter().enumerate() {
            if unit[i].abs() > 1e-12 {
                max_speed = max_speed.min(a.max_velocity / unit[i].abs());
                accel = accel.min(a.acceleration / unit[i].abs());
            }
        }
        // Corner between the previous move and this one.
        if let Some(prev) = self.moves.back_mut() {
            let blend = self.config.constant_velocity || m.continues;
            prev.exit_limit = if blend {
                junction_speed(
                    &prev.unit,
                    &unit,
                    accel.min(prev.accel),
                    self.config.junction_deviation,
                )
                .min(prev.max_speed)
                .min(max_speed)
            } else {
                0.0
            };
        }
        self.moves.push_back(Planned {
            from,
            to: m.to,
            length,
            unit,
            max_speed,
            accel,
            rapid: m.rapid,
            line: m.line,
            exit_limit: 0.0,
        });
    }

    fn speed_scale(&self, m: &Planned) -> f64 {
        if m.rapid {
            self.rapid_override
        } else {
            self.feed_override
        }
    }

    /// Advance one period. Returns `None` when there is nothing to move.
    pub fn tick(&mut self) -> Option<Segment> {
        let front = self.moves.front()?;
        let dt = self.period;
        let accel = front.accel;

        // Fastest speed from which every upcoming limit can still be met.
        let mut allowed = f64::INFINITY;
        let mut dist = -self.progress;
        let mut min_accel = f64::INFINITY;
        for m in &self.moves {
            dist += m.length;
            min_accel = min_accel.min(m.accel);
            // Largest v' with v'^2 <= e^2 + 2a(d - (v + v')dt/2): the discrete braking curve.
            let a_dt = min_accel * dt;
            let c = m.exit_limit.powi(2) + 2.0 * min_accel * dist - a_dt * self.speed;
            let limit = if c > 0.0 {
                (-a_dt + (a_dt * a_dt + 4.0 * c).sqrt()) / 2.0
            } else {
                0.0
            };
            allowed = allowed.min(limit);
            if m.exit_limit == 0.0 {
                break;
            }
        }
        let target = if self.hold {
            0.0
        } else {
            (front.max_speed * self.speed_scale(front)).min(allowed)
        };
        let new_speed = if target > self.speed {
            (self.speed + accel * dt).min(target)
        } else {
            // Hold and override ramps use normal deceleration; corner limits are hard caps.
            target.max(self.speed - accel * dt).min(allowed)
        };
        let mut travel = (self.speed + new_speed) / 2.0 * dt;
        self.speed = new_speed;
        let mut line = front.line;
        if self.hold && new_speed == 0.0 && travel == 0.0 {
            return Some(Segment {
                steps: [0; 6],
                line,
            });
        }
        // Avoid creeping forever towards a stop point.
        if new_speed == 0.0 && travel < 1e-6 {
            travel = front.length - self.progress;
        }

        // Walk along the buffered moves.
        while let Some(m) = self.moves.front() {
            line = m.line;
            let left = m.length - self.progress;
            if travel < left - 1e-12 {
                self.progress += travel;
                self.position = std::array::from_fn(|i| m.from[i] + m.unit[i] * self.progress);
                break;
            }
            travel -= left;
            self.position = m.to;
            let stops_here = m.exit_limit == 0.0;
            self.moves.pop_front();
            self.progress = 0.0;
            if stops_here {
                self.speed = 0.0;
                break;
            }
        }
        if self.moves.is_empty() {
            self.speed = 0.0;
        }
        let steps = self.config.to_steps(&self.position);
        let delta = std::array::from_fn(|i| (steps[i] - self.steps[i]) as i32);
        self.steps = steps;
        Some(Segment { steps: delta, line })
    }
}

/// Corner speed limit from the junction deviation model.
fn junction_speed(a: &Axes, b: &Axes, accel: f64, deviation: f64) -> f64 {
    let cos_theta = -(0..6).map(|i| a[i] * b[i]).sum::<f64>();
    if cos_theta > 0.999_999 {
        return 0.0; // full reversal
    }
    if cos_theta < -0.999_999 {
        return f64::INFINITY; // straight on
    }
    let sin_half = ((1.0 - cos_theta) / 2.0).sqrt();
    (accel * deviation * sin_half / (1.0 - sin_half)).sqrt()
}

/// Expand an op into straight moves. Non-motion ops yield nothing.
pub fn op_to_moves(op: &Op, from: &Axes, line: usize, tolerance: f64) -> Vec<Move> {
    match op {
        Op::Rapid { to } => vec![Move {
            to: *to,
            speed: f64::INFINITY,
            rapid: true,
            line,
            continues: false,
        }],
        Op::Linear { to, feed } => vec![Move {
            to: *to,
            speed: feed / 60.0,
            rapid: false,
            line,
            continues: false,
        }],
        Op::Arc {
            to,
            center,
            plane,
            clockwise,
            turns,
            feed,
        } => {
            let (a0, a1, n) = plane.axes();
            let radius = (from[a0] - center[a0]).hypot(from[a1] - center[a1]);
            let sweep = arc_sweep(from, to, center, *plane, *clockwise, *turns);
            let max_step = if radius > tolerance {
                2.0 * (1.0 - tolerance / radius).acos()
            } else {
                sweep.abs()
            };
            let count = ((sweep.abs() / max_step).ceil() as usize).max(1);
            let start = (from[a1] - center[a1]).atan2(from[a0] - center[a0]);
            (1..=count)
                .map(|k| {
                    let f = k as f64 / count as f64;
                    let mut p: Axes = std::array::from_fn(|i| from[i] + (to[i] - from[i]) * f);
                    if k < count {
                        let ang = start + sweep * f;
                        p[a0] = center[a0] + radius * ang.cos();
                        p[a1] = center[a1] + radius * ang.sin();
                        p[n] = from[n] + (to[n] - from[n]) * f;
                    } else {
                        p = *to;
                    }
                    Move {
                        to: p,
                        speed: feed / 60.0,
                        rapid: false,
                        line,
                        continues: k > 1,
                    }
                })
                .collect()
        }
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gcode::Plane;

    fn gen() -> Generator {
        Generator::new(MachineConfig::dmc2(), 0.004, [0.0; 6])
    }

    fn run(g: &mut Generator) -> Vec<Segment> {
        let mut out = Vec::new();
        while let Some(s) = g.tick() {
            out.push(s);
            assert!(out.len() < 100_000, "generator never finished");
        }
        out
    }

    #[test]
    fn reaches_target_exactly() {
        let mut g = gen();
        g.push(Move {
            to: [-10.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            speed: 5.0,
            rapid: false,
            line: 3,
            continues: false,
        });
        let segs = run(&mut g);
        let total: i32 = segs.iter().map(|s| s.steps[0]).sum();
        assert_eq!(total, -2000);
        assert!(segs.iter().all(|s| s.line == 3));
        // 10 mm at 5 mm/s is about 2 s = ~500 segments; the capture of Mach3 showed 502.
        assert!((495..=520).contains(&segs.len()), "{} segments", segs.len());
        // Cruise at 5 mm/s = 1000 steps/s = 4 steps per 4 ms.
        assert_eq!(segs[segs.len() / 2].steps[0], -4);
        assert!(g.is_idle());
    }

    #[test]
    fn respects_axis_limits_and_board_step_rate() {
        let mut g = gen();
        g.push(Move {
            to: [100.0, 100.0, -50.0, 0.0, 0.0, 0.0],
            speed: f64::INFINITY,
            rapid: true,
            line: 0,
            continues: false,
        });
        let segs = run(&mut g);
        let max_x = 133.3 * 200.0 * 0.004;
        assert!(segs
            .iter()
            .all(|s| (s.steps[0].abs() as f64) <= max_x + 1.0));
        assert!(segs.iter().all(|s| s.steps.iter().all(|v| v.abs() <= 410)));
        let total: [i32; 3] = std::array::from_fn(|i| segs.iter().map(|s| s.steps[i]).sum());
        assert_eq!(total, [20000, 20000, -10000]);
    }

    #[test]
    fn acceleration_is_bounded() {
        let mut g = gen();
        g.push(Move {
            to: [50.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            speed: 100.0,
            rapid: false,
            line: 0,
            continues: false,
        });
        let segs = run(&mut g);
        let speeds: Vec<f64> = segs
            .iter()
            .map(|s| s.steps[0] as f64 / 200.0 / 0.004)
            .collect();
        for w in speeds.windows(2) {
            let a = (w[1] - w[0]).abs() / 0.004;
            // Step quantisation adds up to a step per segment of jitter (312.5 mm/s² each).
            assert!(a < 2000.0 * 1.05 + 2.0 * 312.5, "accel {a}");
        }
    }

    #[test]
    fn exact_stop_between_moves() {
        let mut g = gen();
        g.push(Move {
            to: [10.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            speed: 50.0,
            rapid: false,
            line: 0,
            continues: false,
        });
        g.push(Move {
            to: [10.0, 10.0, 0.0, 0.0, 0.0, 0.0],
            speed: 50.0,
            rapid: false,
            line: 1,
            continues: false,
        });
        let segs = run(&mut g);
        // Some segment at the corner must be at (near) zero X and Y speed.
        let corner = segs.iter().position(|s| s.line == 1).unwrap();
        let before = segs[corner - 1].steps;
        assert!(before[0].abs() <= 2 && before[1].abs() <= 2, "{before:?}");
    }

    #[test]
    fn feed_hold_stops_and_resumes() {
        let mut g = gen();
        g.push(Move {
            to: [100.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            speed: 50.0,
            rapid: false,
            line: 0,
            continues: false,
        });
        for _ in 0..200 {
            g.tick();
        }
        g.hold = true;
        let mut n = 0;
        while g.speed() > 0.0 {
            g.tick();
            n += 1;
            assert!(n < 100);
        }
        let held_at = g.position();
        g.tick();
        assert_eq!(g.position(), held_at);
        g.hold = false;
        let segs = run(&mut g);
        assert!(!segs.is_empty());
        assert!((g.position()[0] - 100.0).abs() < 1e-9);
    }

    #[test]
    fn arc_linearisation_stays_on_circle() {
        let from = [10.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let op = Op::Arc {
            to: [-10.0, 0.0, -1.0, 0.0, 0.0, 0.0],
            center: [0.0; 6],
            plane: Plane::XY,
            clockwise: false,
            turns: 0,
            feed: 600.0,
        };
        let moves = op_to_moves(&op, &from, 7, 0.002);
        assert!(moves.len() > 20);
        for m in &moves {
            let r = m.to[0].hypot(m.to[1]);
            assert!((r - 10.0).abs() < 1e-9);
            assert!(m.to[1] >= -1e-9, "CCW from +X to -X goes through +Y");
        }
        assert_eq!(moves.last().unwrap().to[2], -1.0);
        assert!(moves[1..].iter().all(|m| m.continues));
    }
}
