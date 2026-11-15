//! Calibrate the touch probe. A probe triggers a little after its ball first
//! touches, so the ball acts smaller than it is (and by a different amount
//! along X and Y); its ball may sit off the spindle axis; and it triggers on
//! a surface at a slightly different height than on the tool setter it was
//! measured on. These references measure them:
//!
//! - an inner bore (a ring gauge) or inner rectangle (a pocket) of known
//!   size: the ball goes inside, rcnc centres in it and touches X and Y both
//!   ways twice;
//! - an outer cylinder (a gauge pin, a turned part) or outer rectangle (a
//!   1-2-3 block) of known size: the ball goes above its middle, rcnc touches
//!   the top, then X and Y both sides twice, below the top;
//! - a Z reference whose height above the setter is trusted (set with a gauge
//!   block and a real tool): rcnc touches it twice.
//!
//! After a shape, the operator can turn the probe half a turn in the
//! spindle by hand and measure again: the centre moves by twice how far the
//! ball is off the spindle axis. That offset holds until the probe comes out.
//! Every result is shown beside the stored one and only kept on Save.

use super::{Controller, Mode, Wait};
use crate::probing::{self, fmt, ProbeCalibration, ProbeSettings, Routine, Step};

/// A reference shape of known size the ball's diameter is measured on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeCalShape {
    InnerBore,
    OuterCylinder,
    InnerRectangle,
    OuterRectangle,
}

impl ProbeCalShape {
    pub const ALL: [ProbeCalShape; 4] = [ProbeCalShape::InnerBore, ProbeCalShape::OuterCylinder, ProbeCalShape::InnerRectangle, ProbeCalShape::OuterRectangle];

    pub fn name(self) -> &'static str {
        match self {
            ProbeCalShape::InnerBore => "Inner bore",
            ProbeCalShape::OuterCylinder => "Outer cylinder",
            ProbeCalShape::InnerRectangle => "Inner rectangle",
            ProbeCalShape::OuterRectangle => "Outer rectangle",
        }
    }

    pub fn round(self) -> bool {
        matches!(self, ProbeCalShape::InnerBore | ProbeCalShape::OuterCylinder)
    }

    /// The ball touches it from inside.
    pub fn inside(self) -> bool {
        matches!(self, ProbeCalShape::InnerBore | ProbeCalShape::InnerRectangle)
    }
}

/// What the probe is calibrated on.
#[derive(Debug, Clone, PartialEq)]
pub enum ProbeCalTarget {
    /// A shape this big in X and Y, mm (a round one's diameter twice). An outer
    /// shape's sides are touched `depth` mm below its top.
    Shape { shape: ProbeCalShape, size: [f64; 2], depth: f64 },
    /// The Z reference with this index, which must be right (calibrated with a gauge block).
    ZReference(usize),
}

impl ProbeCalTarget {
    fn describe(&self, c: &Controller) -> String {
        match self {
            ProbeCalTarget::Shape { shape, size, .. } if shape.round() => format!("Ø{} {}", fmt(size[0]), shape.name().to_lowercase()),
            ProbeCalTarget::Shape { shape, size, .. } => format!("{} × {} {}", fmt(size[0]), fmt(size[1]), shape.name().to_lowercase()),
            ProbeCalTarget::ZReference(i) => match c.config.probing.setter.references.get(*i) {
                Some(r) => format!("{} Z reference", r.name),
                None => "Z reference".into(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeCalStage {
    /// Waiting for the operator to tap the probe and jog it into place.
    Locate,
    /// Touching the reference.
    Probing,
    /// Measured: Save, Cancel, or (a shape) turn the probe and measure again.
    Result,
    /// Waiting for the operator to turn the probe half a turn in the spindle.
    Turn,
}

impl ProbeCalStage {
    /// The operator may jog (the probe stops a jog that touches).
    pub fn locating(self) -> bool {
        matches!(self, ProbeCalStage::Locate | ProbeCalStage::Turn)
    }
}

/// What the UI shows while the probe is calibrated.
#[derive(Debug, Clone, PartialEq)]
pub struct ProbeCalView {
    pub target: ProbeCalTarget,
    /// "Ø20.000 bore", "Vise Z reference".
    pub on: String,
    pub stage: ProbeCalStage,
    pub tapped: bool,
    /// The nominal ball diameter from the probe settings.
    pub nominal: f64,
    /// The calibration as stored, and as it would be saved.
    pub old: ProbeCalibration,
    pub new: Option<ProbeCalibration>,
    /// How far apart two measurements of the same span were, mm.
    pub repeat: Option<f64>,
    /// Spindle X/Y with the ball centred, for each measurement (one per probe orientation).
    pub centres: Vec<[f64; 2]>,
    /// The ball's offset from the spindle axis, from the two orientations.
    pub runout: Option<[f64; 2]>,
    /// Stored runout, if measured since the probe went in.
    pub old_runout: Option<[f64; 2]>,
}

/// Effective ball diameters further than this from the nominal ball are refused.
const DIAMETER_SLACK: f64 = 1.0;

impl Controller {
    /// How far the probe's ball is off the spindle axis along `axis`, while the
    /// probe that was measured is loaded; zero otherwise.
    pub(crate) fn ball_offset(&self, axis: usize) -> f64 {
        match self.probe_runout {
            Some(r) if self.probe_loaded() => r[axis],
            _ => 0.0,
        }
    }

    /// Added to a Z touch while the touch probe is loaded.
    pub(crate) fn probe_z_offset(&self) -> f64 {
        if self.probe_loaded() {
            self.config.probing.calibration.z_offset
        } else {
            0.0
        }
    }

    pub(super) fn start_probe_cal(&mut self, target: ProbeCalTarget) {
        if !self.is_ready() {
            return self.info("Calibrating the probe needs the machine idle");
        }
        let s = &self.config.probing;
        let nominal = s.stylus_diameter;
        let why = self.probe_ready_text().or_else(|| match &target {
            ProbeCalTarget::Shape { shape, size, .. } if shape.inside() && !size.iter().all(|w| *w > nominal + 1.0 && w.is_finite()) => {
                Some(format!("it must be more than 1 mm bigger than the Ø{} ball", fmt(nominal)))
            }
            ProbeCalTarget::Shape { size, depth, .. } if !(size.iter().all(|w| *w > 0.0 && w.is_finite()) && *depth > 0.0) => {
                Some("its size and the side depth must be more than zero".into())
            }
            ProbeCalTarget::Shape { shape, depth, .. } if !shape.inside() => {
                self.stylus_too_short(depth + nominal / 2.0).map(|e| format!("its sides would be touched {} mm below the top, {e}", fmt(*depth)))
            }
            ProbeCalTarget::ZReference(i) if s.setter.references.get(*i).is_none() => Some("that Z reference no longer exists".into()),
            _ => None,
        });
        let on = target.describe(self);
        if let Some(why) = why {
            return self.info(format!("Calibrate probe on the {on}: {why}"));
        }
        let prompt = match target {
            ProbeCalTarget::Shape { shape, .. } if shape.inside() => "jog the ball into the middle of it, about 3 mm below its top, then press Continue",
            ProbeCalTarget::Shape { .. } => "jog it over the middle, a few mm above the top, then press Continue",
            ProbeCalTarget::ZReference(_) => "jog it over the surface, a few mm above it, then press Continue",
        };
        self.info(format!("Calibrate probe on the {on}: tap the probe's stylus, {prompt}"));
        self.probe_cal = Some(ProbeCalView {
            target,
            on,
            stage: ProbeCalStage::Locate,
            tapped: false,
            nominal,
            old: self.config.probing.calibration.clone(),
            new: None,
            repeat: None,
            centres: Vec::new(),
            runout: None,
            old_runout: self.probe_runout,
        });
        self.sync_position();
        self.routine = Some(Routine {
            name: "Calibrate probe".into(),
            steps: vec![Step::Operator, Step::Then(Box::new(|c, _| c.probe_cal_measure()))].into(),
            hits: Vec::new(),
        });
        self.routine_move = None;
        self.routine_resume = None;
        self.routine_return = self.mode;
        self.mode = Mode::Probing;
        self.routine_step();
    }

    pub(super) fn probe_cal_watch(&mut self) {
        let touching = self.config.probe.is_some_and(|p| p.is_active(self.status.inputs));
        if let Some(p) = &mut self.probe_cal {
            if touching && p.stage == ProbeCalStage::Locate && !p.tapped {
                p.tapped = true;
                self.info("Calibrate probe: probe input works");
            }
        }
    }

    /// In place (or turned): measure. With a result: save it.
    pub(super) fn probe_cal_continue(&mut self) {
        let Some(p) = &self.probe_cal else { return };
        if !self.routine_waiting || self.mode != Mode::Probing {
            return;
        }
        let touching = self.config.probe.is_some_and(|i| i.is_active(self.status.inputs));
        match p.stage {
            ProbeCalStage::Locate if !p.tapped => return self.info("Tap the probe's stylus first, so rcnc knows the probe input works"),
            ProbeCalStage::Locate | ProbeCalStage::Turn if touching => return self.info("The probe is touching something: jog it clear first"),
            ProbeCalStage::Probing => return,
            ProbeCalStage::Result => self.probe_cal_save(),
            _ => {}
        }
        self.routine_waiting = false;
        self.routine_step();
    }

    /// After a shape's result: wait for the probe to be turned half a turn.
    pub(super) fn probe_cal_turn(&mut self) {
        let Some(p) = &mut self.probe_cal else { return };
        if p.stage != ProbeCalStage::Result || matches!(p.target, ProbeCalTarget::ZReference(_)) || !self.routine_waiting {
            return;
        }
        if p.centres.len() >= 2 {
            return self.info("Calibrate probe: already measured both ways round; Save or Cancel, and start again to measure once more");
        }
        p.stage = ProbeCalStage::Turn;
        self.info("Calibrate probe: with the spindle off, turn the probe half a turn in the spindle by hand, leave the machine where it is, then press Continue");
    }

    fn probe_cal_save(&mut self) {
        let Some(p) = &self.probe_cal else { return };
        let Some(new) = p.new.clone() else { return };
        let (old, on, runout) = (p.old.clone(), p.on.clone(), p.runout);
        let mut said = vec![];
        if new.diameter != old.diameter {
            said.push(format!("ball X {} / Y {} mm on the {on} (was {})", fmt(new.diameter[0]), fmt(new.diameter[1]), diameters(&old)));
        }
        if new.z_offset != old.z_offset || new.z_offset_on != old.z_offset_on {
            said.push(format!("Z offset {:+.3} mm on the {on} (was {:+.3})", new.z_offset, old.z_offset));
        }
        if let Some(r) = runout {
            self.probe_runout = Some(r);
            said.push(format!("ball off the spindle axis by X {:+.3} Y {:+.3} mm, until the probe comes out", r[0], r[1]));
        }
        self.config.probing.calibration = new;
        self.report(format!("Probe calibrated: {}", said.join("; ")));
        if let Some(p) = &mut self.probe_cal {
            // the routine's last step sees this and ends
            p.stage = ProbeCalStage::Probing;
        }
    }

    fn probe_cal_stage(&mut self, stage: ProbeCalStage) {
        if let Some(p) = &mut self.probe_cal {
            p.stage = stage;
        }
    }

    /// Touch the reference from where the operator left the probe.
    fn probe_cal_measure(&mut self) -> Result<Vec<Step>, String> {
        let Some(p) = &self.probe_cal else { return Ok(vec![]) };
        let target = p.target.clone();
        self.probe_cal_stage(ProbeCalStage::Probing);
        if let Some(r) = &mut self.routine {
            r.hits.clear();
        }
        let s = self.config.probing.clone();
        let r = s.stylus_diameter / 2.0;
        // X, Y, X, Y: the middle of a chord is on the centre line whatever the other axis is,
        // so after X then Y the spindle is in the middle and the second X and Y spans are the
        // full size (a round shape's diameter); the two Y spans must agree
        let mut steps = match target {
            ProbeCalTarget::Shape { shape, size, .. } if shape.inside() => {
                let wall = [size[0] / 2.0 - r, size[1] / 2.0 - r];
                let mut steps = inner_pair(&s, 0, wall[0] + 5.0);
                steps.extend(inner_pair(&s, 1, wall[1] + 5.0));
                steps.extend(inner_pair(&s, 0, wall[0] + 1.0));
                steps.extend(inner_pair(&s, 1, wall[1] + 1.0));
                steps.push(Step::Then(Box::new(move |c, hits| c.probe_cal_diameter(hits, true, size))));
                steps
            }
            ProbeCalTarget::Shape { size, depth, .. } => {
                let mut steps = probing::touch(&s, 2, false, s.find_stock.z_search);
                let start = self.machine_position();
                steps.push(Step::Then(Box::new(move |c, hits| {
                    let top = hits[0];
                    let s = c.config.probing.clone();
                    let mut more = vec![];
                    for axis in [0, 1, 0, 1] {
                        more.extend(outer_pair(&s, axis, start[axis], size[axis], top, depth));
                    }
                    more.push(Step::Then(Box::new(move |c, hits| c.probe_cal_diameter(&hits[1..], false, size))));
                    Ok(more)
                })));
                steps
            }
            ProbeCalTarget::ZReference(i) => {
                let mut steps = probing::touch(&s, 2, false, s.find_stock.z_search);
                steps.push(Step::Probe { axis: 2, by: -2.0 * s.latch, feed: s.slow_feed, record: true });
                steps.push(Step::MoveBy { axis: 2, by: s.latch, feed: s.fast_feed, guarded: false });
                let limit = s.setter.repeat_tolerance;
                steps.push(Step::Then(Box::new(move |c, hits| {
                    let [a, b] = hits[hits.len() - 2..] else { return Err("the surface wasn't touched twice".into()) };
                    if (a - b).abs() > limit + 1e-9 {
                        return Err(format!("the two touches differ by {} mm (limit {}): check the probe and the surface", fmt((a - b).abs()), fmt(limit)));
                    }
                    c.probe_cal_z(i, (a + b) / 2.0, (a - b).abs())
                })));
                steps
            }
        };
        // wait for Save / Cancel / Turn; then measure again after a turn, or end
        steps.push(Step::Operator);
        steps.push(Step::Then(Box::new(|c, _| match c.probe_cal.as_ref().map(|p| p.stage) {
            Some(ProbeCalStage::Turn) => c.probe_cal_measure(),
            _ => Ok(vec![]),
        })));
        Ok(steps)
    }

    /// The ball's effective diameter from spans touched inside or outside a
    /// shape `size` big: two touches per pair, pairs X Y X Y.
    fn probe_cal_diameter(&mut self, hits: &[f64], inside: bool, size: [f64; 2]) -> Result<Vec<Step>, String> {
        let Some(p) = &self.probe_cal else { return Ok(vec![]) };
        let span = |i: usize| (hits[2 * i] - hits[2 * i + 1]).abs();
        let mid = |i: usize| (hits[2 * i] + hits[2 * i + 1]) / 2.0;
        if hits.len() < 8 {
            return Err("not every side was touched".into());
        }
        let (x, y) = (2, 3);
        let d = if inside { [size[0] - span(x), size[1] - span(y)] } else { [span(x) - size[0], span(y) - size[1]] };
        let repeat = (span(3) - span(1)).abs();
        let centre = [mid(x), mid(y)];
        let s = &self.config.probing;
        let limit = s.setter.repeat_tolerance;
        if repeat > limit + 1e-9 {
            return Err(format!(
                "the same span measured {} mm differently twice (limit {}): check the probe is tight and the {} is clamped and clean",
                fmt(repeat),
                fmt(limit),
                p.on
            ));
        }
        if let Some(bad) = d.iter().find(|d| !(**d > 0.0 && (**d - p.nominal).abs() <= DIAMETER_SLACK)) {
            return Err(format!(
                "the ball measures Ø{} against Ø{} nominal: check the {} size you entered and the stylus diameter in the probe settings",
                fmt(*bad),
                fmt(p.nominal),
                p.on
            ));
        }
        let on = p.on.clone();
        let mut new = p.old.clone();
        let mut note = String::new();
        let mut centres = p.centres.clone();
        let mut runout = None;
        match centres.first() {
            // measured again after the probe was turned: the centre moved by twice the ball's offset
            Some(first) => {
                let prev = p.new.as_ref().map_or(d, |n| n.diameter);
                new.diameter = [(prev[0] + d[0]) / 2.0, (prev[1] + d[1]) / 2.0];
                // spindle centre with the ball centred is (true centre - ball offset)
                let r = [(first[0] - centre[0]) / 2.0, (first[1] - centre[1]) / 2.0];
                note = format!(
                    "; the ball is X {:+.3} Y {:+.3} mm off the spindle axis ({} mm runout), with the probe as it is now",
                    r[0],
                    r[1],
                    fmt(2.0 * r[0].hypot(r[1]))
                );
                runout = Some(r);
            }
            None => new.diameter = d,
        }
        centres.push(centre);
        new.diameter_on = on.clone();
        let old = diameters(&p.old);
        if let Some(p) = &mut self.probe_cal {
            p.new = Some(new.clone());
            p.repeat = Some(repeat);
            p.centres = centres;
            p.runout = runout;
            p.stage = ProbeCalStage::Result;
        }
        self.info(format!(
            "Calibrate probe on the {on}: ball X {} / Y {} mm (stored {old}), repeat {} mm{note}. Save, Cancel or turn the probe and measure again",
            fmt(new.diameter[0]),
            fmt(new.diameter[1]),
            fmt(repeat)
        ));
        Ok(vec![])
    }

    /// The probe's Z offset from a touch at machine Z `surface` (no offset
    /// applied) on the Z reference `index`, whose stored height is trusted.
    fn probe_cal_z(&mut self, index: usize, surface: f64, repeat: f64) -> Result<Vec<Step>, String> {
        let Some(p) = &self.probe_cal else { return Ok(vec![]) };
        let s = &self.config.probing;
        let r = s.setter.references.get(index).ok_or("that Z reference no longer exists")?;
        // as Calibrate Z does: the probe, measured on the setter with the active
        // reference, says the setter's top is `seen` above this surface
        let work = surface - self.interp.program_offset()[2];
        let seen = s.setter.top() - work;
        // the real surface is z_offset higher than the probe says
        let z_offset = seen - r.setter_top;
        if z_offset.abs() > 1.0 {
            return Err(format!(
                "the probe puts the {} {} mm from where its Z reference says: is the reference right (calibrate it with a gauge block), \
                 and was the probe measured on the setter since it went in?",
                p.on,
                fmt(z_offset)
            ));
        }
        let on = p.on.clone();
        let mut new = p.old.clone();
        new.z_offset = z_offset;
        new.z_offset_on = r.name.clone();
        let old = p.old.z_offset;
        if let Some(p) = &mut self.probe_cal {
            p.new = Some(new);
            p.repeat = Some(repeat);
            p.stage = ProbeCalStage::Result;
        }
        self.info(format!("Calibrate probe on the {on}: Z offset {z_offset:+.3} mm (stored {old:+.3}). Save or Cancel"));
        let up = self.machine_position()[2] + self.config.probing.find_stock.z_clearance;
        Ok(vec![Step::MoveTo { axis: 2, to: up, feed: self.config.probing.travel_feed, guarded: false }])
    }

    pub(super) fn end_probe_cal(&mut self, result: &Result<(), String>) {
        if self.probe_cal.take().is_some() {
            if let Err(e) = result {
                self.info(format!("Calibrate probe stopped: {e}; nothing was changed"));
            }
            if let Some(exec) = &mut self.exec {
                if matches!(exec.wait, Wait::Routine) {
                    exec.wait = Wait::None;
                }
            }
        }
    }
}

fn diameters(c: &ProbeCalibration) -> String {
    if c.diameter.iter().all(|d| *d > 0.0) {
        format!("X {} / Y {}", fmt(c.diameter[0]), fmt(c.diameter[1]))
    } else {
        "not calibrated".into()
    }
}

/// From inside a bore or pocket: touch + then - along `axis` from where the spindle is,
/// then go to the middle. Records the + touch, then the - touch.
fn inner_pair(s: &ProbeSettings, axis: usize, reach: f64) -> Vec<Step> {
    let s = s.clone();
    vec![Step::Then(Box::new(move |c, _| {
        let from = c.machine_position()[axis];
        let mut steps = probing::touch(&s, axis, true, reach);
        steps.push(probing::guarded_to(&s, axis, from));
        steps.extend(probing::touch(&s, axis, false, reach));
        let s2 = s.clone();
        steps.push(Step::Then(Box::new(move |_, hits| {
            let n = hits.len();
            Ok(vec![probing::guarded_to(&s2, axis, (hits[n - 2] + hits[n - 1]) / 2.0)])
        })));
        Ok(steps)
    }))]
}

/// Outside a cylinder or block whose top is at machine Z `top`, from above its middle:
/// touch the low side, then the high side along `axis`, `depth` below the
/// top, and come back up over the middle found. Records the low touch, then the high.
fn outer_pair(s: &ProbeSettings, axis: usize, middle: f64, width: f64, top: f64, depth: f64) -> Vec<Step> {
    let fs = &s.find_stock;
    let out = width / 2.0 + fs.clearance + s.stylus_diameter / 2.0;
    let mut steps = vec![probing::guarded_to(s, 2, top + fs.z_clearance)];
    for positive in [true, false] {
        let dir = if positive { 1.0 } else { -1.0 };
        steps.push(probing::guarded_to(s, axis, middle - dir * out));
        steps.push(probing::guarded_to(s, 2, top - depth));
        steps.extend(probing::touch(s, axis, positive, 2.0 * fs.clearance + 1.0));
        steps.push(probing::guarded_to(s, 2, top + fs.z_clearance));
    }
    let s2 = s.clone();
    steps.push(Step::Then(Box::new(move |_, hits| {
        let n = hits.len();
        Ok(vec![probing::guarded_to(&s2, axis, (hits[n - 2] + hits[n - 1]) / 2.0)])
    })));
    steps
}
