//! Probing routines: tool touch-off on the tool setter, Z surface, edges,
//! corners and bore/boss centres. They follow the "verser" Mach3 macros on
//! the DMC2 (M901-M927 and Jonas's touch-off script): every probe is two
//! passes (fast search, back off, slow latch), and every positioning move is
//! guarded, stopping with an error if the probe touches something.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::controller::Controller;

/// Probe and tool-setter settings, in mm and mm/min. Defaults are the DMC2's
/// Mach3 values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProbeSettings {
    /// First, fast search feed.
    pub fast_feed: f64,
    /// Second, slow latch feed.
    pub slow_feed: f64,
    /// Longest search before giving up.
    pub max_distance: f64,
    /// Back-off between the two passes and after the touch.
    pub latch: f64,
    /// Sideways clearance before going down beside an edge.
    pub xy_clearance: f64,
    /// How far below the start height to probe edges.
    pub z_depth: f64,
    /// Expected boss size for boss centre probing.
    pub boss_size: f64,
    /// Stylus ball diameter.
    pub stylus_diameter: f64,
    /// Touch plate thickness for Z surface probing.
    pub plate_thickness: f64,
    /// Set the work zero at what was found.
    pub auto_zero: bool,
    /// Feed for positioning moves.
    pub travel_feed: f64,
    pub setter: ToolSetter,
}

/// A fixed tool setter. Touching a tool off on it sets the work Z zero so
/// the tool tip reads `offset` on the setter, which is how Jonas's Mach3
/// script works: the setter's top is `offset` above the calibrated Z zero.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ToolSetter {
    /// Machine X/Y of the setter.
    pub x: f64,
    pub y: f64,
    /// Machine Z to travel at on the way there.
    pub travel_z: f64,
    pub max_distance: f64,
    /// Work Z of the setter's top.
    pub offset: f64,
    /// Height to retract above the touch.
    pub retract: f64,
}

impl Default for ProbeSettings {
    fn default() -> Self {
        ProbeSettings {
            fast_feed: 500.0,
            slow_feed: 10.0,
            max_distance: 50.0,
            latch: 1.0,
            xy_clearance: 5.0,
            z_depth: 5.0,
            boss_size: 90.0,
            stylus_diameter: 2.0,
            plate_thickness: 0.0,
            auto_zero: true,
            travel_feed: 3000.0,
            setter: ToolSetter::default(),
        }
    }
}

impl Default for ToolSetter {
    fn default() -> Self {
        // From HiddenScript.m1s: G53 X-5 Y289, CalibratedOffset 13.860 - 0.55, retract 10.
        ToolSetter {
            x: -5.0,
            y: 289.0,
            travel_z: 0.0,
            max_distance: 100.0,
            offset: 13.31,
            retract: 10.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeJob {
    /// Touch the current tool off on the tool setter.
    ToolTouchOff,
    /// Probe down to the surface (or touch plate) below.
    ZSurface,
    /// Probe an edge by moving along `axis` (0 = X, 1 = Y) in the given direction.
    Edge { axis: usize, positive: bool },
    /// Outside corner, starting diagonally off it at probing clearance:
    /// an X edge, then a Y edge, probing in these directions.
    Corner { x_positive: bool, y_positive: bool },
    /// Centre of a bore, starting inside it at probing height.
    BoreCentre,
    /// Centre of a boss, starting above its middle.
    BossCentre,
}

impl ProbeJob {
    pub fn name(&self) -> String {
        let dir = |p: bool| if p { "+" } else { "-" };
        match self {
            ProbeJob::ToolTouchOff => "Tool touch-off".into(),
            ProbeJob::ZSurface => "Z surface".into(),
            ProbeJob::Edge { axis, positive } => format!("{}{} edge", ["X", "Y"][*axis], dir(*positive)),
            ProbeJob::Corner { x_positive, y_positive } => {
                format!("X{} Y{} corner", dir(*x_positive), dir(*y_positive))
            }
            ProbeJob::BoreCentre => "Bore centre".into(),
            ProbeJob::BossCentre => "Boss centre".into(),
        }
    }
}

/// Runs after earlier steps, with the touches recorded so far, and returns more steps.
pub(crate) type Then = Box<dyn FnOnce(&mut Controller, &[f64]) -> Result<Vec<Step>, String> + Send>;

pub(crate) enum Step {
    /// Move one axis to a machine position. Guarded moves stop with an error if the probe touches.
    MoveTo { axis: usize, to: f64, feed: f64, guarded: bool },
    /// Move one axis by a distance from wherever it is when the step starts.
    MoveBy { axis: usize, by: f64, feed: f64, guarded: bool },
    /// Probe along one axis; an error if nothing is touched. `record` keeps the touch position.
    Probe { axis: usize, by: f64, feed: f64, record: bool },
    Then(Then),
}

pub(crate) struct Routine {
    pub name: String,
    pub steps: VecDeque<Step>,
    /// Machine positions of the recorded touches, in order.
    pub hits: Vec<f64>,
}

/// Two-pass probe as in the verser macros' ProbeAxis: search, back off,
/// latch slowly, back off. Records one touch.
fn touch(s: &ProbeSettings, axis: usize, positive: bool, distance: f64) -> Vec<Step> {
    let dir = if positive { 1.0 } else { -1.0 };
    vec![
        Step::Probe { axis, by: dir * distance, feed: s.fast_feed, record: false },
        Step::MoveBy { axis, by: -dir * s.latch, feed: s.fast_feed, guarded: false },
        Step::Probe { axis, by: dir * 2.0 * s.latch, feed: s.slow_feed, record: true },
        Step::MoveBy { axis, by: -dir * s.latch, feed: s.fast_feed, guarded: false },
    ]
}

fn guarded_by(s: &ProbeSettings, axis: usize, by: f64) -> Step {
    Step::MoveBy { axis, by, feed: s.travel_feed, guarded: true }
}

fn guarded_to(s: &ProbeSettings, axis: usize, to: f64) -> Step {
    Step::MoveTo { axis, to, feed: s.travel_feed, guarded: true }
}

fn then(f: impl FnOnce(&mut Controller, &[f64]) -> Result<Vec<Step>, String> + Send + 'static) -> Step {
    Step::Then(Box::new(f))
}

fn fmt(v: f64) -> String {
    format!("{v:.3}")
}

/// Steps for one edge, ending back at the start height with the stylus over the edge.
/// The touch is recorded as the stylus centre; the edge is half a stylus further on.
fn edge_steps(s: &ProbeSettings, axis: usize, positive: bool) -> Vec<Step> {
    let dir = if positive { 1.0 } else { -1.0 };
    let mut steps = vec![guarded_by(s, axis, -dir * s.xy_clearance), guarded_by(s, 2, -s.z_depth)];
    steps.extend(touch(s, axis, positive, s.max_distance));
    steps.push(guarded_by(s, 2, s.z_depth));
    steps
}

pub(crate) fn build(job: ProbeJob, s: &ProbeSettings, start: [f64; 6]) -> Routine {
    let mut steps: Vec<Step> = Vec::new();
    let r = s.stylus_diameter / 2.0;
    let auto = s.auto_zero;
    match job {
        ProbeJob::ToolTouchOff => {
            let t = s.setter.clone();
            steps.push(guarded_to(s, 2, t.travel_z));
            steps.push(guarded_to(s, 0, t.x));
            steps.push(guarded_to(s, 1, t.y));
            steps.extend(touch(s, 2, false, t.max_distance));
            let feed = s.travel_feed;
            steps.push(then(move |c, hits| {
                let hit = hits[0];
                c.set_work_axis(2, hit, t.offset);
                c.report(format!("Tool touched off: Z zero set, setter top at Z{}", fmt(t.offset)));
                // Retract, but no higher than the travel height.
                let to = (hit + t.retract).min(t.travel_z.max(hit));
                Ok(vec![Step::MoveTo { axis: 2, to, feed, guarded: false }])
            }));
        }
        ProbeJob::ZSurface => {
            steps.extend(touch(s, 2, false, s.max_distance));
            let (plate, z0) = (s.plate_thickness, start[2]);
            let feed = s.travel_feed;
            steps.push(then(move |c, hits| {
                let hit = hits[0];
                if auto {
                    c.set_work_axis(2, hit, plate);
                }
                c.report(format!("Z surface at machine Z{}", fmt(hit - plate)));
                Ok(vec![Step::MoveTo { axis: 2, to: z0, feed, guarded: true }])
            }));
        }
        ProbeJob::Edge { axis, positive } => {
            steps.extend(edge_steps(s, axis, positive));
            let feed = s.travel_feed;
            steps.push(then(move |c, hits| {
                let edge = hits[0] + if positive { r } else { -r };
                if auto {
                    c.set_work_axis(axis, edge, 0.0);
                }
                c.report(format!("{} edge at machine {}", ["X", "Y"][axis], fmt(edge)));
                Ok(vec![Step::MoveTo { axis, to: edge, feed, guarded: true }])
            }));
        }
        ProbeJob::Corner { x_positive, y_positive } => {
            // Start diagonally off the corner. Step along the Y face to probe X,
            // back to the start, then along the X face to probe Y.
            let along = 2.0 * s.xy_clearance;
            let (dx, dy) = (if x_positive { 1.0 } else { -1.0 }, if y_positive { 1.0 } else { -1.0 });
            steps.push(guarded_by(s, 1, dy * along));
            steps.extend(edge_steps(s, 0, x_positive));
            steps.push(guarded_to(s, 0, start[0]));
            steps.push(guarded_to(s, 1, start[1]));
            steps.push(guarded_by(s, 0, dx * along));
            steps.extend(edge_steps(s, 1, y_positive));
            steps.push(guarded_to(s, 1, start[1]));
            steps.push(guarded_to(s, 0, start[0]));
            let feed = s.travel_feed;
            steps.push(then(move |c, hits| {
                let x = hits[0] + if x_positive { r } else { -r };
                let y = hits[1] + if y_positive { r } else { -r };
                if auto {
                    c.set_work_axis(0, x, 0.0);
                    c.set_work_axis(1, y, 0.0);
                }
                c.report(format!("Corner at machine X{} Y{}", fmt(x), fmt(y)));
                Ok(vec![
                    Step::MoveTo { axis: 0, to: x, feed, guarded: true },
                    Step::MoveTo { axis: 1, to: y, feed, guarded: true },
                ])
            }));
        }
        ProbeJob::BoreCentre => {
            // Touch X+ and X-, centre in X, then Y+ and Y- from there.
            let feed = s.travel_feed;
            let s2 = s.clone();
            steps.extend(touch(s, 0, true, s.max_distance));
            steps.push(guarded_to(s, 0, start[0]));
            steps.extend(touch(s, 0, false, s.max_distance));
            steps.push(then(move |_, hits| {
                let xc = (hits[0] + hits[1]) / 2.0;
                let mut more = vec![Step::MoveTo { axis: 0, to: xc, feed, guarded: true }];
                more.extend(touch(&s2, 1, true, s2.max_distance));
                more.push(Step::MoveTo { axis: 1, to: start[1], feed, guarded: true });
                more.extend(touch(&s2, 1, false, s2.max_distance));
                Ok(more)
            }));
            steps.push(then(move |c, hits| {
                let xc = (hits[0] + hits[1]) / 2.0;
                let yc = (hits[2] + hits[3]) / 2.0;
                let dx = hits[0] - hits[1] + 2.0 * r;
                let dy = hits[2] - hits[3] + 2.0 * r;
                if auto {
                    c.set_work_axis(0, xc, 0.0);
                    c.set_work_axis(1, yc, 0.0);
                }
                c.report(format!(
                    "Bore centre at machine X{} Y{}, diameter {} (X {} / Y {})",
                    fmt(xc), fmt(yc), fmt((dx + dy) / 2.0), fmt(dx), fmt(dy)
                ));
                Ok(vec![Step::MoveTo { axis: 1, to: yc, feed, guarded: true }])
            }));
        }
        ProbeJob::BossCentre => {
            let out = s.boss_size / 2.0 + s.xy_clearance + r;
            let feed = s.travel_feed;
            for axis in 0..2 {
                for positive in [true, false] {
                    // From above the middle: out past the side, down, probe inward, up, back.
                    let dir = if positive { 1.0 } else { -1.0 };
                    steps.push(guarded_to(s, axis, start[axis] - dir * out));
                    steps.push(guarded_by(s, 2, -s.z_depth));
                    steps.extend(touch(s, axis, positive, out));
                    steps.push(guarded_to(s, 2, start[2]));
                    steps.push(guarded_to(s, axis, start[axis]));
                }
            }
            steps.push(then(move |c, hits| {
                let xc = (hits[0] + hits[1]) / 2.0;
                let yc = (hits[2] + hits[3]) / 2.0;
                let dx = hits[1] - hits[0] - 2.0 * r;
                let dy = hits[3] - hits[2] - 2.0 * r;
                if auto {
                    c.set_work_axis(0, xc, 0.0);
                    c.set_work_axis(1, yc, 0.0);
                }
                c.report(format!(
                    "Boss centre at machine X{} Y{}, size X {} Y {}",
                    fmt(xc), fmt(yc), fmt(dx), fmt(dy)
                ));
                Ok(vec![
                    Step::MoveTo { axis: 0, to: xc, feed, guarded: true },
                    Step::MoveTo { axis: 1, to: yc, feed, guarded: true },
                ])
            }));
        }
    }
    Routine { name: job.name(), steps: steps.into(), hits: Vec::new() }
}
