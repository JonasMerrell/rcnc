//! Find stock: the wizard a job starts with. The touch probe is loaded and
//! measured on the tool setter like any tool; the operator taps the stylus
//! to show the probe works, jogs it to roughly above the middle of the stock
//! and presses Continue. rcnc then touches the top, both X sides and both Y
//! sides, checks the size against the job's, and sets the work offset so the
//! stock's top centre reads what the job expects. The top also becomes the
//! "Stock" Z reference, so every tool measured after it is zeroed on it.

use gcode::StockRequest;

use super::{Controller, Exec, Mode, Wait};
use crate::probing::{self, fmt, Routine, Step, ZReference};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindStockStage {
    /// Waiting for the operator to tap the stylus and jog over the stock.
    Locate,
    /// Touching the top and the sides.
    Probing,
    /// The probed size is off by more than the tolerance; nothing was set.
    Mismatch,
}

/// What the stock turned out to be, in machine coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StockFound {
    pub centre: [f64; 2],
    pub top: f64,
    pub size: [f64; 2],
}

/// What the UI shows while the wizard runs.
#[derive(Debug, Clone, PartialEq)]
pub struct FindStockView {
    pub request: StockRequest,
    pub stage: FindStockStage,
    /// The probe input has been seen on since the operator was asked to tap it.
    pub tapped: bool,
    pub found: Option<StockFound>,
    pub problem: Option<String>,
    pub tolerance: f64,
}

/// Name of the Z reference the stock's top is kept as.
pub const STOCK_REFERENCE: &str = "Stock";

impl Controller {
    pub(crate) fn probe_loaded(&self) -> bool {
        let t = self.config.probing.probe_tool;
        t != 0 && self.loaded_tool == t
    }

    /// Start the wizard, from a program's FIND_STOCK request or from the Probe page.
    pub(super) fn start_find_stock(&mut self, request: StockRequest) {
        let probe_tool = self.config.probing.probe_tool;
        let why = if self.config.probe.is_none() {
            Some("no probe input is configured".to_string())
        } else if !self.probe_loaded() {
            Some(format!("load the touch probe first (T{probe_tool} M6)"))
        } else if !self.tool_measured {
            Some(format!("the probe (T{probe_tool}) hasn't been measured on the tool setter"))
        } else if self.outputs.spindle_on {
            Some("the spindle is on".to_string())
        } else {
            None
        };
        if let Some(why) = why {
            return self.raise(format!("Find stock: {why}"));
        }
        let tolerance = request.tolerance.unwrap_or(self.config.probing.find_stock.tolerance);
        self.find_stock =
            Some(FindStockView { request, stage: FindStockStage::Locate, tapped: false, found: None, problem: None, tolerance });
        self.info("Find stock: tap the probe's stylus, jog it over the middle of the stock, a few mm above the top, then press Continue");
        self.sync_position();
        self.routine = Some(Routine { name: "Find stock".into(), steps: locate_steps().into(), hits: Vec::new() });
        self.routine_move = None;
        self.routine_resume = None;
        self.routine_return = self.mode;
        self.mode = Mode::Probing;
        if let Some(exec) = &mut self.exec {
            exec.wait = Wait::Routine;
        }
        self.routine_step();
    }

    /// The wizard is waiting for the operator to jog over the stock.
    pub(super) fn locating(&self) -> bool {
        self.mode == Mode::Probing
            && self.routine_waiting
            && self.find_stock.as_ref().is_some_and(|f| matches!(f.stage, FindStockStage::Locate | FindStockStage::Mismatch))
    }

    /// Note a tap on the stylus while the wizard waits for one.
    pub(super) fn find_stock_watch(&mut self) {
        let touching = self.config.probe.is_some_and(|p| p.is_active(self.status.inputs));
        if let Some(f) = &mut self.find_stock {
            if touching && f.stage == FindStockStage::Locate && !f.tapped {
                f.tapped = true;
                self.info("Find stock: probe input works");
            }
        }
    }

    /// The operator is over the stock (or wants to try again after a mismatch).
    pub(super) fn find_stock_continue(&mut self) {
        let Some(f) = &self.find_stock else { return };
        if !self.routine_waiting || self.mode != Mode::Probing {
            return;
        }
        if f.stage == FindStockStage::Locate && !f.tapped {
            return self.info("Tap the probe's stylus first, so rcnc knows the probe input works");
        }
        if self.config.probe.is_some_and(|p| p.is_active(self.status.inputs)) {
            return self.info("The probe is touching something: jog it clear first");
        }
        self.routine_waiting = false;
        self.routine_step();
    }

    fn find_stock_stage(&mut self, stage: FindStockStage) {
        if let Some(f) = &mut self.find_stock {
            f.stage = stage;
        }
    }

    /// Touch the top, then X both sides, then Y both sides about the X centre.
    fn find_stock_probe(&mut self) -> Result<Vec<Step>, String> {
        let Some(f) = &self.find_stock else { return Ok(vec![]) };
        let req = f.request.clone();
        self.find_stock_stage(FindStockStage::Probing);
        if let Some(r) = &mut self.routine {
            r.hits.clear();
        }
        let s = self.config.probing.clone();
        let fs = s.find_stock.clone();
        let start = self.machine_position();
        self.info(format!("Find stock: probing down from machine Z{}", fmt(start[2])));
        let mut steps = probing::touch(&s, 2, false, fs.z_search);
        steps.push(Step::Then(Box::new(move |c, hits| {
            let top = hits[0];
            let depth = req.depth.unwrap_or(fs.depth);
            let mut more = vec![probing::guarded_to(&s, 2, top + fs.z_clearance)];
            more.extend(sides(&s, 0, start[0], req.size[0], top, depth));
            let s2 = s.clone();
            more.push(Step::Then(Box::new(move |_, hits| {
                let xc = (hits[1] + hits[2]) / 2.0;
                let mut more = vec![probing::guarded_to(&s2, 0, xc)];
                more.extend(sides(&s2, 1, start[1], req.size[1], top, depth));
                more.push(Step::Then(Box::new(move |c, hits| c.find_stock_done(hits, top))));
                Ok(more)
            })));
            c.info(format!("Find stock: top at machine Z{}", fmt(top)));
            Ok(more)
        })));
        Ok(steps)
    }

    /// Check what was found and set the work offset from it, or wait to try again.
    fn find_stock_done(&mut self, hits: &[f64], top: f64) -> Result<Vec<Step>, String> {
        let Some(f) = &self.find_stock else { return Ok(vec![]) };
        let (req, tolerance) = (f.request.clone(), f.tolerance);
        let s = self.config.probing.clone();
        let r = s.stylus_diameter / 2.0;
        // hits: top, X from below, X from above, Y from below, Y from above.
        let centre = [(hits[1] + hits[2]) / 2.0, (hits[3] + hits[4]) / 2.0];
        let size = [hits[2] - hits[1] - 2.0 * r, hits[4] - hits[3] - 2.0 * r];
        let found = StockFound { centre, top, size };
        let over = [0, 1].map(|i| (size[i] - req.size[i]).abs());
        let mut steps = vec![probing::guarded_to(&s, 0, centre[0]), probing::guarded_to(&s, 1, centre[1])];
        if let Some(f) = &mut self.find_stock {
            f.found = Some(found);
        }
        let summary = format!(
            "X {} × Y {} (job: {} × {})",
            fmt(size[0]),
            fmt(size[1]),
            fmt(req.size[0]),
            fmt(req.size[1])
        );
        if over.iter().any(|d| *d > tolerance + 1e-9) {
            let problem = format!(
                "The stock measures {summary}, more than {} mm off. Nothing was set. Check the stock in the vise \
                 and the job's stock size, then jog over the middle again and press Continue, or Stop.",
                fmt(tolerance)
            );
            self.info(format!("Find stock: {problem}"));
            self.find_stock_stage(FindStockStage::Mismatch);
            if let Some(f) = &mut self.find_stock {
                f.problem = Some(problem);
            }
            steps.push(Step::Operator);
            steps.push(Step::Then(Box::new(|c, _| {
                if let Some(f) = &mut c.find_stock {
                    f.problem = None;
                }
                c.find_stock_probe()
            })));
            return Ok(steps);
        }
        let [cx, cy, cz] = req.top_centre;
        self.set_work_axis(0, centre[0], cx);
        self.set_work_axis(1, centre[1], cy);
        // The probe was measured on the setter: that tells how far the
        // setter's top is above the stock's, which later tools are zeroed by.
        let work_top = top - self.interp.program_offset()[2];
        let setter_top = cz + self.config.probing.setter.top() - work_top;
        let refs = &mut self.config.probing.setter.references;
        match refs.iter_mut().find(|r| r.name == STOCK_REFERENCE) {
            Some(r) => r.setter_top = setter_top,
            None => refs.push(ZReference { name: STOCK_REFERENCE.into(), setter_top }),
        }
        self.use_z_reference(STOCK_REFERENCE);
        self.set_work_axis(2, top, cz);
        let rel = setter_top - cz;
        self.report(format!(
            "Stock found: {summary}, centre machine X{} Y{}, top Z{}; tools are now zeroed on the stock top \
             (the setter's top is {} mm {} it)",
            fmt(centre[0]),
            fmt(centre[1]),
            fmt(top),
            fmt(rel.abs()),
            if rel >= 0.0 { "above" } else { "below" }
        ));
        Ok(steps)
    }

    /// The wizard's routine ended; let the program carry on, or stop it.
    pub(super) fn end_find_stock(&mut self, result: &Result<(), String>) {
        if self.find_stock.take().is_none() {
            return;
        }
        match result {
            Ok(()) => {
                if let Some(Exec { wait: wait @ Wait::Routine, .. }) = &mut self.exec {
                    *wait = Wait::None;
                }
            }
            Err(e) => self.raise(format!("Find stock failed: {e}")),
        }
    }
}

fn locate_steps() -> Vec<Step> {
    vec![
        Step::Operator,
        Step::Then(Box::new(|c, _| c.find_stock_probe())),
    ]
}

/// Touch both sides of the stock along `axis`, from outside each, `depth`
/// below the top; records the low side's touch, then the high side's.
fn sides(s: &probing::ProbeSettings, axis: usize, middle: f64, size: f64, top: f64, depth: f64) -> Vec<Step> {
    let fs = &s.find_stock;
    let out = size / 2.0 + fs.clearance + s.stylus_diameter / 2.0;
    let mut steps = Vec::new();
    for positive in [true, false] {
        let dir = if positive { 1.0 } else { -1.0 };
        steps.push(probing::guarded_to(s, axis, middle - dir * out));
        steps.push(probing::guarded_to(s, 2, top - depth));
        steps.extend(probing::touch(s, axis, positive, 2.0 * fs.clearance + 1.0));
        steps.push(probing::guarded_to(s, 2, top + fs.z_clearance));
    }
    steps
}
