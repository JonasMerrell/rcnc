//! Find vise location: find the two-station vise's fixed centre block with
//! the touch probe and keep where it is, in machine coordinates (homing
//! repeats to a few µm, so it holds until the vise is moved).
//!
//! The operator empties the vise (or opens both stations well clear of the
//! centre block), taps the stylus, jogs it over the middle of the block's
//! top, a few mm above it, and presses Continue. rcnc touches the top, both
//! Y ends of the block, then both fixed jaw faces at two places along Y, and
//! shows what it found beside what is stored: the X of both faces (the left
//! station's fixed jaw is the block's -X face, the right station's its +X
//! face), the block's Y middle, the faces' angle to the Y axis and the jaw
//! top's height. Save keeps it; Find stock with `VISE=` then takes X from it.

use super::{Controller, Mode, Wait};
use crate::probing::{self, fmt, Routine, Step, ViseLocation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindViseStage {
    /// Waiting for the operator to tap the stylus and jog over the block.
    Locate,
    /// Touching the block.
    Probing,
    /// Measured: Save or Cancel.
    Result,
}

/// What the UI shows while Find vise location runs.
#[derive(Debug, Clone, PartialEq)]
pub struct FindViseView {
    pub stage: FindViseStage,
    pub tapped: bool,
    /// The location as stored, and as it would be saved.
    pub old: Option<ViseLocation>,
    pub new: Option<ViseLocation>,
    /// What looks off in the result (a gap or length unlike the settings', an
    /// untrammed vise): shown, but it can still be saved.
    pub warnings: Vec<String>,
}

/// Faces leaning more than this over the jaw length (mm) get a tramming warning.
const TRAM_WARN: f64 = 0.02;
/// The jaw length the lean is quoted over, mm.
const JAW_LENGTH: f64 = 95.5;

impl Controller {
    pub(super) fn start_find_vise(&mut self) {
        if !self.is_ready() {
            return self.info("Find vise location needs the machine idle");
        }
        let why = self.probe_ready_text().or_else(|| {
            if !self.homed[..3].iter().all(|h| *h) {
                Some("home X, Y and Z first: the vise location is kept in machine coordinates".into())
            } else if self.outputs.spindle_on {
                Some("the spindle is on".into())
            } else {
                let v = &self.config.probing.vise;
                if !(v.gap > 0.0 && v.length > 0.0 && v.depth > 0.0 && v.spread > 0.0) {
                    Some("the vise's gap, length, depth and spread must be more than zero (Probe page)".into())
                } else {
                    self.stylus_too_short(v.depth + self.config.probing.stylus_diameter / 2.0)
                        .map(|e| format!("the faces would be touched {} mm below the jaw top, {e}", fmt(v.depth)))
                }
            }
        });
        if let Some(why) = why {
            return self.info(format!("Find vise location: {why}"));
        }
        self.find_vise = Some(FindViseView {
            stage: FindViseStage::Locate,
            tapped: false,
            old: self.config.probing.vise.location,
            new: None,
            warnings: Vec::new(),
        });
        self.info(
            "Find vise location: empty the vise (or open both stations well clear of the fixed centre block) and take any work stops off, \
             tap the probe's stylus, jog it over the middle of the centre block, a few mm above its top, then press Continue",
        );
        self.sync_position();
        self.routine = Some(Routine {
            name: "Find vise location".into(),
            steps: vec![Step::Operator, Step::Then(Box::new(|c, _| c.find_vise_probe()))].into(),
            hits: Vec::new(),
        });
        self.routine_move = None;
        self.routine_resume = None;
        self.routine_return = self.mode;
        self.mode = Mode::Probing;
        self.routine_step();
    }

    pub(super) fn find_vise_watch(&mut self) {
        let touching = self.config.probe.is_some_and(|p| p.is_active(self.status.inputs));
        if let Some(v) = &mut self.find_vise {
            if touching && v.stage == FindViseStage::Locate && !v.tapped {
                v.tapped = true;
                self.info("Find vise location: probe input works");
            }
        }
    }

    /// Over the block: measure. With a result: save it.
    pub(super) fn find_vise_continue(&mut self) {
        let Some(v) = &self.find_vise else { return };
        if !self.routine_waiting || self.mode != Mode::Probing {
            return;
        }
        match v.stage {
            FindViseStage::Locate if !v.tapped => return self.info("Tap the probe's stylus first, so rcnc knows the probe input works"),
            FindViseStage::Locate if self.config.probe.is_some_and(|p| p.is_active(self.status.inputs)) => {
                return self.info("The probe is touching something: jog it clear first")
            }
            FindViseStage::Probing => return,
            FindViseStage::Result => self.find_vise_save(),
            FindViseStage::Locate => {}
        }
        self.routine_waiting = false;
        self.routine_step();
    }

    fn find_vise_stage(&mut self, stage: FindViseStage) {
        if let Some(v) = &mut self.find_vise {
            v.stage = stage;
        }
    }

    /// Touch the top, the Y ends about the start, then each face at two Y's about the middle found.
    fn find_vise_probe(&mut self) -> Result<Vec<Step>, String> {
        self.find_vise_stage(FindViseStage::Probing);
        if let Some(r) = &mut self.routine {
            r.hits.clear();
        }
        let s = self.config.probing.clone();
        let v = s.vise.clone();
        let fs = s.find_stock.clone();
        let start = self.machine_position();
        self.info(format!(
            "Find vise location: probing down from machine Z{}; ends and faces {} mm below the jaw top",
            fmt(start[2]),
            fmt(v.depth)
        ));
        let mut steps = probing::touch(&s, 2, false, fs.z_search);
        steps.push(Step::Then(Box::new(move |c, hits| {
            let top = hits[0] + c.probe_z_offset();
            let mut more = vec![probing::guarded_to(&s, 2, top + fs.z_clearance)];
            if v.ends {
                more.extend(super::findstock::sides(&s, 1, start[1], v.length, top, v.depth));
            }
            let s2 = s.clone();
            more.push(Step::Then(Box::new(move |_, hits| {
                let y0 = if v.ends { (hits[1] + hits[2]) / 2.0 } else { start[1] };
                let ys = [y0 - v.spread, y0 + v.spread];
                let mut more = vec![];
                for y in ys {
                    more.push(probing::guarded_to(&s2, 1, y));
                    more.extend(super::findstock::sides(&s2, 0, start[0], v.gap, top, v.depth));
                }
                more.push(probing::guarded_to(&s2, 0, start[0]));
                more.push(probing::guarded_to(&s2, 1, y0));
                more.push(Step::Then(Box::new(move |c, hits| c.find_vise_done(hits, top, ys))));
                Ok(more)
            })));
            c.info(format!("Find vise location: jaw top at machine Z{}", fmt(top)));
            Ok(more)
        })));
        Ok(steps)
    }

    /// Work out the block from the touches and wait for Save or Cancel.
    /// hits: top, (Y low end, Y high end,) then -X face and +X face at each of `ys`.
    fn find_vise_done(&mut self, hits: &[f64], top: f64, ys: [f64; 2]) -> Result<Vec<Step>, String> {
        let s = self.config.probing.clone();
        let v = &s.vise;
        let f = if v.ends { 3 } else { 1 };
        if hits.len() < f + 4 {
            return Err("not every face was touched".into());
        }
        let (rx, ry) = (s.ball_radius(0), s.ball_radius(1));
        let (ox, oy) = (self.ball_offset(0), self.ball_offset(1));
        // the -X face is touched moving +X (the ball's +X side meets it), the +X face moving -X
        let low = [hits[f] + rx + ox, hits[f + 2] + rx + ox];
        let high = [hits[f + 1] - rx + ox, hits[f + 3] - rx + ox];
        let dy = ys[1] - ys[0];
        let slopes = [(low[1] - low[0]) / dy, (high[1] - high[0]) / dy];
        let slope = (slopes[0] + slopes[1]) / 2.0;
        let y_mid = (ys[0] + ys[1]) / 2.0 + oy;
        let faces = [(low[0] + low[1]) / 2.0, (high[0] + high[1]) / 2.0];
        let length = v.ends.then(|| (hits[2] - ry) - (hits[1] + ry));
        let gap = faces[1] - faces[0];
        let work_top = top - self.interp.program_offset()[2];
        let setter_above_top = s.setter.top() - work_top;
        let loc = ViseLocation { faces, y: y_mid, angle: slope.atan().to_degrees(), setter_above_top, gap, length };

        let mut warnings = vec![];
        if (gap - v.gap).abs() > v.tolerance {
            warnings.push(format!(
                "The fixed jaw faces are {} mm apart, {} mm from the {} set on the Probe page: check the faces were clean and the setting",
                fmt(gap),
                fmt(gap - v.gap),
                fmt(v.gap)
            ));
        }
        if let Some(l) = length.filter(|l| (l - v.length).abs() > v.tolerance) {
            warnings.push(format!("The centre block is {} mm long, {} mm from the {} set on the Probe page", fmt(l), fmt(l - v.length), fmt(v.length)));
        }
        let lean = slope * JAW_LENGTH;
        if lean.abs() > TRAM_WARN {
            warnings.push(format!(
                "The jaws are turned {}° from the Y axis ({} mm over the {} mm jaw): tram the vise if parts must be square to the machine",
                fmt(loc.angle),
                fmt(lean),
                fmt(JAW_LENGTH)
            ));
        }
        let skew = (slopes[1] - slopes[0]) * JAW_LENGTH;
        if skew.abs() > TRAM_WARN {
            warnings.push(format!("The two faces aren't parallel: {} mm over the jaw length (chips or a burr on a face?)", fmt(skew)));
        }
        let vise_floor = s.setter.references.iter().find(|r| r.name.eq_ignore_ascii_case("Vise")).map(|r| r.setter_top - setter_above_top);
        let floor = vise_floor.map(|h| format!(", {} mm above the Vise Z reference", fmt(h))).unwrap_or_default();
        self.info(format!(
            "Find vise location: fixed jaw faces at machine X{} (left station) and X{} (right station), {} mm apart; \
             block middle Y{}{}; jaws turned {}°; jaw top {} mm below the setter top{floor}. Save or Cancel",
            fmt(faces[0]),
            fmt(faces[1]),
            fmt(gap),
            fmt(y_mid),
            length.map(|l| format!(", {} mm long", fmt(l))).unwrap_or_default(),
            fmt(loc.angle),
            fmt(setter_above_top),
        ));
        for w in &warnings {
            self.info(format!("Find vise location: {w}"));
        }
        if let Some(view) = &mut self.find_vise {
            view.new = Some(loc);
            view.warnings = warnings;
            view.stage = FindViseStage::Result;
        }
        // wait for Save or Cancel, then end
        Ok(vec![Step::Operator, Step::Then(Box::new(|_, _| Ok(vec![])))])
    }

    fn find_vise_save(&mut self) {
        let Some(new) = self.find_vise.as_ref().and_then(|v| v.new) else { return };
        let old = self.config.probing.vise.location;
        self.config.probing.vise.location = Some(new);
        let moved = old.map(|o| {
            format!(
                " (moved X {:+.3} / {:+.3}, Y {:+.3} since the last one)",
                new.faces[0] - o.faces[0],
                new.faces[1] - o.faces[1],
                new.y - o.y
            )
        });
        self.report(format!(
            "Vise location saved: fixed jaws at machine X{} (left station) and X{} (right station), middle Y{}, turned {}°{}",
            fmt(new.faces[0]),
            fmt(new.faces[1]),
            fmt(new.y),
            fmt(new.angle),
            moved.unwrap_or_default()
        ));
        // the routine's last step sees this and ends
        self.find_vise_stage(FindViseStage::Probing);
    }

    pub(super) fn end_find_vise(&mut self, result: &Result<(), String>) {
        if self.find_vise.take().is_some() {
            if let Err(e) = result {
                self.info(format!("Find vise location stopped: {e}; nothing was changed"));
            }
            if let Some(exec) = &mut self.exec {
                if matches!(exec.wait, Wait::Routine) {
                    exec.wait = Wait::None;
                }
            }
        }
    }
}
