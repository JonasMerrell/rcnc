//! Find stock: the wizard a job starts with. The touch probe is loaded and
//! measured on the tool setter like any tool; the operator taps the stylus
//! to show the probe works, jogs it to roughly above the middle of the stock
//! and presses Continue. rcnc then touches the top, both X sides and both Y
//! sides, checks the size against the job's, and sets the work offset so the
//! stock's top centre reads what the job expects. The top also becomes the
//! "Stock" Z reference, so every tool measured after it is zeroed on it.

use gcode::{BoresRequest, StockRequest, ZRequest};

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
    /// How far below the top the sides are touched (the program's DEPTH, or the
    /// default); the operator can change it, and the tolerance, before Continue.
    pub depth: f64,
}

/// Name of the Z reference the stock's top is kept as.
pub const STOCK_REFERENCE: &str = "Stock";

impl Controller {
    pub(crate) fn probe_loaded(&self) -> bool {
        let t = self.config.probing.probe_tool;
        t != 0 && self.loaded_tool == t
    }

    /// Start the wizard, from a program's FIND_STOCK request or from the Probe page.
    /// Why the probe can't put its ball's tip `depth` mm below a surface it hovers over (the probe
    /// body would hit it first): the probe's flute length in the tool table is its stylus length.
    fn stylus_too_short(&self, depth: f64) -> Option<String> {
        let t = self.config.probing.probe_tool;
        let reach = self.interp.tools.get(&t).map(|tool| tool.flute_length).filter(|l| *l > 0.0)?;
        (depth > reach + 1e-9).then(|| format!("but the probe's stylus (T{t}) is only {} mm long", fmt(reach)))
    }

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
        let depth = request.depth.unwrap_or(self.config.probing.find_stock.depth);
        // a depth the stylus can't reach is shown, to change in the window (Continue waits for it)
        let problem = self.depth_problem(depth);
        self.find_stock = Some(FindStockView { request, stage: FindStockStage::Locate, tapped: false, found: None, problem, tolerance, depth });
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
            && (self.find_stock.as_ref().is_some_and(|f| matches!(f.stage, FindStockStage::Locate | FindStockStage::Mismatch))
                || self.calibrate.as_ref().is_some_and(|c| c.stage == super::CalibrateStage::Locate))
    }

    /// A jog while locating stops on the probe, except a tool jogged onto a gauge block.
    pub(super) fn locate_guarded(&self) -> bool {
        !self.calibrate.as_ref().is_some_and(|c| c.gauge.is_some())
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
    /// What's wrong with touching the sides `depth` mm below the top, if anything.
    fn depth_problem(&self, depth: f64) -> Option<String> {
        self.stylus_too_short(depth + self.config.probing.stylus_diameter / 2.0)
            .map(|e| format!("The sides would be touched {} mm below the top, {e}. Lower the depth below.", fmt(depth)))
    }

    /// The operator changed how far below the top the sides are touched, or the size tolerance.
    pub(super) fn find_stock_settings(&mut self, depth: f64, tolerance: f64) {
        let Some(f) = &self.find_stock else { return };
        if !matches!(f.stage, FindStockStage::Locate | FindStockStage::Mismatch) {
            return self.info("Find stock: the depth and tolerance can only be changed before Continue");
        }
        if !(depth > 0.0 && depth.is_finite() && tolerance > 0.0 && tolerance.is_finite()) {
            return self.info("Find stock: the depth and the tolerance must be more than zero");
        }
        let problem = self.depth_problem(depth);
        if let Some(f) = &mut self.find_stock {
            f.depth = depth;
            f.request.depth = Some(depth);
            f.tolerance = tolerance;
            // a size mismatch stays on show until the next try
            f.problem = problem.clone().or(f.problem.clone().filter(|_| f.stage == FindStockStage::Mismatch));
        }
        if let Some(p) = problem {
            self.info(format!("Find stock: {p}"));
        }
    }

    pub(super) fn find_stock_continue(&mut self) {
        let Some(f) = &self.find_stock else { return };
        if !self.routine_waiting || self.mode != Mode::Probing {
            return;
        }
        if let Some(p) = self.depth_problem(f.depth) {
            return self.info(format!("Find stock: {p}"));
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
        let (req, depth, tolerance) = (f.request.clone(), f.depth, f.tolerance);
        self.find_stock_stage(FindStockStage::Probing);
        if let Some(r) = &mut self.routine {
            r.hits.clear();
        }
        let s = self.config.probing.clone();
        let fs = s.find_stock.clone();
        let start = self.machine_position();
        self.info(format!(
            "Find stock: probing down from machine Z{}; the sides {} mm below the top, size within ±{} mm",
            fmt(start[2]),
            fmt(depth),
            fmt(tolerance)
        ));
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
        if let Some((side, jaw_z)) = req.jaw {
            // X from the fixed jaw: touch its face beside the stock's end, outside the stock in Y
            let s = self.config.probing.clone();
            let fs = s.find_stock.clone();
            let r = s.stylus_diameter / 2.0;
            let face = centre[0] + side * size[0] / 2.0;
            let start_x = face - side * (fs.clearance + r);
            let y_out = centre[1] + size[1] / 2.0 + fs.clearance + r;
            let z = self.work_to_machine(2, jaw_z);
            let above = top + fs.z_clearance;
            steps.push(probing::guarded_to(&s, 2, above));
            steps.push(probing::guarded_to(&s, 1, y_out));
            steps.push(probing::guarded_to(&s, 0, start_x));
            steps.push(probing::guarded_to(&s, 2, z));
            steps.extend(probing::touch(&s, 0, side > 0.0, 2.0 * fs.clearance));
            steps.push(probing::guarded_to(&s, 2, above));
            let jaw_work = cx + side * req.size[0] / 2.0;
            steps.push(Step::Then(Box::new(move |c, hits| {
                let jaw = hits[hits.len() - 1] + side * r;
                c.set_work_axis(0, jaw, jaw_work);
                c.report(format!(
                    "Fixed jaw at machine X{}: work X set from it (the stock's face against it is X{}; the stock's own face was {} mm from it)",
                    fmt(jaw),
                    fmt(jaw_work),
                    fmt((face - jaw) * -side)
                ));
                Ok(vec![probing::guarded_to(&c.config.probing.clone(), 1, centre[1]), probing::guarded_to(&c.config.probing.clone(), 0, jaw - side * req.size[0] / 2.0)])
            })));
        }
        if req.keep_z {
            // Z comes from the tools' measurements on a reference: the top is only checked
            let work_top = top - self.interp.program_offset()[2];
            let off = work_top - cz;
            let note = if off.abs() > 0.5 {
                format!(" Warning: the top is at Z{}, {} mm from the Z{} the job expects: check the stock height and the Z reference", fmt(work_top), fmt(off), fmt(cz))
            } else {
                format!(" Top at Z{} ({} mm from the job's)", fmt(work_top), fmt(off))
            };
            self.report(format!("Stock found: {summary}, centre machine X{} Y{}; work X/Y set, Z kept.{note}", fmt(centre[0]), fmt(centre[1])));
            return Ok(steps);
        }
        // The probe was measured on the setter: that tells how far the
        // setter's top is above the stock's, which later tools are zeroed by.
        let setter_top = self.zero_z_on(top, cz);
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

    /// Set work Z so machine Z `hit` (a touch with the measured probe) reads
    /// `z`, and keep that as the Z every tool measured after it is zeroed to
    /// (the Stock reference). Returns the setter top's work Z.
    fn zero_z_on(&mut self, hit: f64, z: f64) -> f64 {
        let work = hit - self.interp.program_offset()[2];
        let setter_top = z + self.config.probing.setter.top() - work;
        let refs = &mut self.config.probing.setter.references;
        match refs.iter_mut().find(|r| r.name == STOCK_REFERENCE) {
            Some(r) => r.setter_top = setter_top,
            None => refs.push(ZReference { name: STOCK_REFERENCE.into(), setter_top }),
        }
        self.use_z_reference(STOCK_REFERENCE);
        self.set_work_axis(2, hit, z);
        setter_top
    }

    /// Why an automatic probing request can't start, if it can't.
    fn probe_ready(&self, what: &str) -> Result<(), String> {
        let probe_tool = self.config.probing.probe_tool;
        if self.config.probe.is_none() {
            Err(format!("{what}: no probe input is configured"))
        } else if !self.probe_loaded() {
            Err(format!("{what}: load the touch probe first (T{probe_tool} M6)"))
        } else if !self.tool_measured {
            Err(format!("{what}: the probe (T{probe_tool}) hasn't been measured on the tool setter"))
        } else if self.outputs.spindle_on {
            Err(format!("{what}: the spindle is on"))
        } else {
            Ok(())
        }
    }

    /// Run `steps` as a routine in place of the program until they end.
    fn run_probe_routine(&mut self, name: &str, steps: Vec<Step>) {
        self.sync_position();
        self.routine = Some(Routine { name: name.into(), steps: steps.into(), hits: Vec::new() });
        self.routine_move = None;
        self.routine_resume = None;
        self.routine_return = self.mode;
        self.mode = Mode::Probing;
        if let Some(exec) = &mut self.exec {
            exec.wait = Wait::Routine;
        }
        self.auto_probe = true;
        self.routine_step();
    }

    fn work_to_machine(&self, axis: usize, w: f64) -> f64 {
        w + self.interp.program_offset()[axis]
    }

    /// `(RCNC PROBE_Z ...)`: touch a surface at a point and set work Z from it.
    pub(super) fn start_probe_z(&mut self, r: ZRequest) {
        if let Err(e) = self.probe_ready("Probe Z") {
            return self.raise(e);
        }
        let s = self.config.probing.clone();
        let fs = &s.find_stock;
        let surface = self.work_to_machine(2, r.z);
        let above = surface + fs.z_clearance;
        let pos = self.machine_position();
        let (x, y) = (self.work_to_machine(0, r.at[0]), self.work_to_machine(1, r.at[1]));
        let mut steps = vec![
            probing::guarded_to(&s, 2, pos[2].max(above)),
            probing::guarded_to(&s, 0, x),
            probing::guarded_to(&s, 1, y),
            probing::guarded_to(&s, 2, above),
        ];
        steps.extend(probing::touch(&s, 2, false, fs.z_clearance + r.search.unwrap_or(10.0)));
        let z = r.z;
        steps.push(Step::Then(Box::new(move |c, hits| {
            let hit = hits[0];
            c.zero_z_on(hit, z);
            c.report(format!(
                "Probe Z: surface at machine Z{} is now work Z{} ({} mm from where it was expected); tools are zeroed on it",
                fmt(hit),
                fmt(z),
                fmt(hit - surface)
            ));
            Ok(vec![probing::guarded_to(&c.config.probing.clone(), 2, hit + c.config.probing.find_stock.z_clearance)])
        })));
        self.info(format!("Probe Z at X{} Y{}", fmt(r.at[0]), fmt(r.at[1])));
        self.run_probe_routine("Probe Z", steps);
    }

    /// `(RCNC FIND_BORES ...)`: find two bores' middles and set work X/Y from them.
    pub(super) fn start_find_bores(&mut self, r: BoresRequest) {
        if let Err(e) = self.probe_ready("Find bores") {
            return self.raise(e);
        }
        let s = self.config.probing.clone();
        let ball = s.stylus_diameter / 2.0;
        // the ball's top goes DEPTH below the bores' top
        if let Some(e) = self.stylus_too_short(r.depth + 2.0 * ball) {
            return self.raise(format!("Find bores: the ball's tip goes {} mm into the bores, {e}: lower DEPTH in the program", fmt(r.depth + 2.0 * ball)));
        }
        if r.diameter / 2.0 - ball < 0.3 {
            return self.raise(format!("Find bores: the Ø{} ball doesn't fit Ø{} bores", fmt(2.0 * ball), fmt(r.diameter)));
        }
        let top = self.work_to_machine(2, r.top);
        let above = top + s.find_stock.z_clearance;
        // the ball's middle `DEPTH` below the top
        let down = top - r.depth - ball;
        let reach = r.diameter / 2.0 - ball;
        let pos = self.machine_position();
        // between the bores, back up to where the probe came from: the part may stand higher than the bores' top
        let travel = pos[2].max(above);
        let mut steps = vec![probing::guarded_to(&s, 2, travel)];
        for at in [r.a, r.b] {
            let (x, y) = (self.work_to_machine(0, at[0]), self.work_to_machine(1, at[1]));
            steps.push(probing::guarded_to(&s, 0, x));
            steps.push(probing::guarded_to(&s, 1, y));
            steps.push(probing::guarded_to(&s, 2, down));
            // X both ways from the nominal middle, then Y both ways from the X middle found
            steps.extend(probing::touch(&s, 0, true, reach + 1.5));
            steps.push(probing::guarded_to(&s, 0, x));
            steps.extend(probing::touch(&s, 0, false, reach + 1.5));
            let s2 = s.clone();
            steps.push(Step::Then(Box::new(move |_, hits| {
                let n = hits.len();
                let xc = (hits[n - 2] + hits[n - 1]) / 2.0;
                let mut more = vec![probing::guarded_to(&s2, 0, xc)];
                more.extend(probing::touch(&s2, 1, true, reach + 1.5));
                more.push(probing::guarded_to(&s2, 1, y));
                more.extend(probing::touch(&s2, 1, false, reach + 1.5));
                more.push(probing::guarded_to(&s2, 2, travel));
                Ok(more)
            })));
        }
        let r2 = r.clone();
        steps.push(Step::Then(Box::new(move |c, hits| c.find_bores_done(&r2, hits))));
        self.info(format!("Find bores: Ø{} at X{} Y{} and X{} Y{}", fmt(r.diameter), fmt(r.a[0]), fmt(r.a[1]), fmt(r.b[0]), fmt(r.b[1])));
        self.run_probe_routine("Find bores", steps);
    }

    fn find_bores_done(&mut self, r: &BoresRequest, hits: &[f64]) -> Result<Vec<Step>, String> {
        // hits per bore: X+, X-, Y+, Y-
        let centre = |i: usize| [(hits[4 * i] + hits[4 * i + 1]) / 2.0, (hits[4 * i + 2] + hits[4 * i + 3]) / 2.0];
        let (ca, cb) = (centre(0), centre(1));
        let ball = self.config.probing.stylus_diameter / 2.0;
        let size = |i: usize| (hits[4 * i] - hits[4 * i + 1]).abs() + 2.0 * ball;
        let spacing = (cb[0] - ca[0]).hypot(cb[1] - ca[1]);
        let nominal = (r.b[0] - r.a[0]).hypot(r.b[1] - r.a[1]);
        let turn = ((cb[1] - ca[1]).atan2(cb[0] - ca[0]) - (r.b[1] - r.a[1]).atan2(r.b[0] - r.a[0])).to_degrees();
        let turn = (turn + 540.0).rem_euclid(360.0) - 180.0;
        let summary = format!(
            "bores Ø{} and Ø{}, {} mm apart (should be {}), part turned {}°",
            fmt(size(0)),
            fmt(size(1)),
            fmt(spacing),
            fmt(nominal),
            fmt(turn)
        );
        if (spacing - nominal).abs() > r.tolerance + 1e-9 {
            return Err(format!("Find bores: {summary}: the spacing is more than {} mm off. Nothing was set", fmt(r.tolerance)));
        }
        if turn.abs() > r.rotation + 1e-9 {
            return Err(format!(
                "Find bores: {summary}: more than {}° off square. Nothing was set: square the part in the vise and run it again",
                fmt(r.rotation)
            ));
        }
        let mid = [(ca[0] + cb[0]) / 2.0, (ca[1] + cb[1]) / 2.0];
        self.set_work_axis(0, mid[0], (r.a[0] + r.b[0]) / 2.0);
        self.set_work_axis(1, mid[1], (r.a[1] + r.b[1]) / 2.0);
        self.report(format!("Bores found: {summary}; work X/Y set from their middle"));
        Ok(vec![])
    }

    /// The wizard's routine ended; let the program carry on, or stop it.
    pub(super) fn end_find_stock(&mut self, result: &Result<(), String>) {
        let auto = std::mem::take(&mut self.auto_probe);
        if self.find_stock.take().is_none() && !auto {
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
