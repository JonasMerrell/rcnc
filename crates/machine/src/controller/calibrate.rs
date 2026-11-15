//! Calibrate a Z reference (the vise's built-in parallels, the table): how
//! far the tool setter's top is above that surface, which every tool measured
//! on the setter is zeroed by. With the touch probe: tap it, jog it over the
//! surface, Continue, and rcnc touches the surface twice. Without it: jog a
//! measured tool down onto a gauge block of known thickness standing on the
//! surface. Either way the new value is shown beside the old one and only
//! kept when the operator saves it.

use super::{Controller, Mode, Wait};
use crate::probing::{self, fmt, Routine, Step};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrateStage {
    /// Waiting for the operator to tap the probe and jog it over the surface,
    /// or to jog the tool down onto the gauge block.
    Locate,
    /// Touching the surface.
    Probing,
    /// The new value is known: Save or Cancel.
    Result,
}

/// What the UI shows while a calibration runs.
#[derive(Debug, Clone, PartialEq)]
pub struct CalibrateView {
    pub reference: String,
    pub stage: CalibrateStage,
    /// Gauge block thickness, mm, when calibrating without the probe.
    pub gauge: Option<f64>,
    pub tapped: bool,
    /// The setter top's height above the surface: as stored, and as measured.
    pub old: f64,
    pub new: Option<f64>,
}

impl Controller {
    /// Start calibrating the Z reference with this index.
    pub(super) fn start_calibrate(&mut self, index: usize, gauge: Option<f64>) {
        if !self.is_ready() {
            return self.info("Calibrating needs the machine idle");
        }
        let Some(r) = self.config.probing.setter.references.get(index).cloned() else {
            return self.info("That Z reference no longer exists");
        };
        let why = match gauge {
            None => self.probe_ready_text(),
            Some(_) if !self.tool_measured => Some("measure the tool on the tool setter first (Touch off tool)".to_string()),
            Some(g) if !(g >= 0.0) => Some("the gauge block thickness must be zero or more".to_string()),
            Some(_) => None,
        };
        if let Some(why) = why {
            return self.info(format!("Calibrate {}: {why}", r.name));
        }
        self.calibrate = Some(CalibrateView {
            reference: r.name.clone(),
            stage: CalibrateStage::Locate,
            gauge,
            tapped: gauge.is_some(),
            old: r.setter_top,
            new: None,
        });
        self.info(match gauge {
            None => format!("Calibrate {}: tap the probe's stylus, jog it over the surface, a few mm above it, then press Continue", r.name),
            Some(g) => format!("Calibrate {}: jog the tool down until it just touches the {} mm gauge block on the surface, then press Continue", r.name, fmt(g)),
        });
        self.sync_position();
        self.routine = Some(Routine {
            name: format!("Calibrate {}", r.name),
            steps: vec![Step::Operator, Step::Then(Box::new(|c, _| c.calibrate_measure()))].into(),
            hits: Vec::new(),
        });
        self.routine_move = None;
        self.routine_resume = None;
        self.routine_return = self.mode;
        self.mode = Mode::Probing;
        self.routine_step();
    }

    pub(super) fn probe_ready_text(&self) -> Option<String> {
        self.config.probe.is_none().then(|| "no probe input is configured".to_string()).or_else(|| {
            let t = self.config.probing.probe_tool;
            if !self.probe_loaded() {
                Some(format!("load the touch probe first (T{t} M6)"))
            } else if !self.tool_measured {
                Some(format!("the probe (T{t}) hasn't been measured on the tool setter"))
            } else {
                None
            }
        })
    }

    pub(super) fn calibrate_watch(&mut self) {
        let touching = self.config.probe.is_some_and(|p| p.is_active(self.status.inputs));
        if let Some(c) = &mut self.calibrate {
            if touching && c.stage == CalibrateStage::Locate && !c.tapped {
                c.tapped = true;
                self.info("Calibrate: probe input works");
            }
        }
    }

    /// The operator is over the surface (or on the gauge block), or saves the result.
    pub(super) fn calibrate_continue(&mut self) {
        let Some(c) = &self.calibrate else { return };
        if !self.routine_waiting || self.mode != Mode::Probing {
            return;
        }
        match c.stage {
            CalibrateStage::Locate if !c.tapped => return self.info("Tap the probe's stylus first, so rcnc knows the probe input works"),
            CalibrateStage::Locate if c.gauge.is_none() && self.config.probe.is_some_and(|p| p.is_active(self.status.inputs)) => {
                return self.info("The probe is touching something: jog it clear first")
            }
            CalibrateStage::Result => {
                if let Some(new) = c.new {
                    let name = c.reference.clone();
                    if let Some(r) = self.config.probing.setter.references.iter_mut().find(|r| r.name == name) {
                        r.setter_top = new;
                    }
                    self.report(format!("Z reference {name}: the setter's top is now {} mm above it (was {})", fmt(new), fmt(c.old)));
                }
            }
            _ => {}
        }
        self.routine_waiting = false;
        self.routine_step();
    }

    fn calibrate_stage(&mut self, stage: CalibrateStage) {
        if let Some(c) = &mut self.calibrate {
            c.stage = stage;
        }
    }

    /// Where the surface is, by touching it with the probe or from the gauge block.
    fn calibrate_measure(&mut self) -> Result<Vec<Step>, String> {
        let Some(c) = &self.calibrate else { return Ok(vec![]) };
        if let Some(g) = c.gauge {
            // the tool's tip is on the gauge block: the surface is `g` below it
            let tip = self.machine_position()[2];
            return Ok(self.calibrate_result(tip - g));
        }
        self.calibrate_stage(CalibrateStage::Probing);
        let s = self.config.probing.clone();
        let mut steps = probing::touch(&s, 2, false, s.find_stock.z_search);
        // a second slow touch: the two must agree
        steps.push(Step::Probe { axis: 2, by: -2.0 * s.latch, feed: s.slow_feed, record: true });
        steps.push(Step::MoveBy { axis: 2, by: s.latch, feed: s.fast_feed, guarded: false });
        let limit = s.setter.repeat_tolerance;
        steps.push(Step::Then(Box::new(move |c, hits| {
            let [a, b] = hits[hits.len() - 2..] else { return Err("the surface wasn't touched twice".into()) };
            if (a - b).abs() > limit + 1e-9 {
                return Err(format!("the two touches differ by {} mm (limit {}): check the probe and the surface", fmt((a - b).abs()), fmt(limit)));
            }
            Ok(c.calibrate_result((a + b) / 2.0 + c.probe_z_offset()))
        })));
        Ok(steps)
    }

    /// The setter top's height above a surface at machine Z `surface`, shown
    /// for the operator to save; then lift off it.
    pub(super) fn calibrate_result(&mut self, surface: f64) -> Vec<Step> {
        // The loaded tool was measured with the active reference: at the
        // setter it read that reference's setter top, so the setter is
        // `top - work Z of the surface` above the surface.
        let work = surface - self.interp.program_offset()[2];
        let new = self.config.probing.setter.top() - work;
        let mut note = None;
        if let Some(c) = &mut self.calibrate {
            c.new = Some(new);
            c.stage = CalibrateStage::Result;
            note = Some(format!("Calibrate {}: {} mm (stored {}, {} mm different). Save or Stop", c.reference, fmt(new), fmt(c.old), fmt(new - c.old)));
        }
        if let Some(n) = note {
            self.info(n);
        }
        let up = self.machine_position()[2] + self.config.probing.find_stock.z_clearance;
        let feed = self.config.probing.travel_feed;
        vec![Step::MoveTo { axis: 2, to: up, feed, guarded: false }, Step::Operator]
    }

    pub(super) fn end_calibrate(&mut self, result: &Result<(), String>) {
        if self.calibrate.take().is_some() {
            if let Err(e) = result {
                self.info(format!("Calibrate stopped: {e}; nothing was changed"));
            }
            if let Some(exec) = &mut self.exec {
                if matches!(exec.wait, Wait::Routine) {
                    exec.wait = Wait::None;
                }
            }
        }
    }
}
