//! Guided tool change at M6: stop the spindle, lift Z, move the bed back,
//! wait for the operator to load the new tool, measure it twice on the tool
//! setter (pausing if the two readings disagree), then go back to where the
//! change started, with the tool tip at the same work height and the spindle
//! and coolant as they were.

use std::collections::VecDeque;

use gcode::Axes;

use super::{Controller, Exec, Wait};
use crate::backend::Outputs;
use crate::probing::{self, ProbeSettings, Routine, Step};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolChangeStage {
    /// Stopping the spindle and moving to the change position.
    Moving,
    /// At the change position, waiting for the operator to load the tool.
    Loading,
    /// Measuring the new tool on the tool setter.
    Measuring,
    /// The two readings on the setter disagreed: waiting for the operator to
    /// check things and measure again.
    Mismatch,
    /// Going back to where the tool change started.
    Returning,
}

/// What the UI shows while a tool change runs.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolChangeView {
    /// The tool that was in the spindle (0 if not known).
    pub from: u32,
    pub to: u32,
    pub stage: ToolChangeStage,
    /// Whether the new tool will be measured, unless the operator says otherwise.
    pub measure: bool,
    pub return_to_start: bool,
    /// Why the tool change is paused, at `Mismatch`.
    pub problem: Option<String>,
    /// The new tool's stick-out from its setter measurement, once the check is calibrated.
    pub measured_stickout: Option<f64>,
    /// Loading the touch probe: the checks its probing cycles need, all made here, so those
    /// cycles run straight on after the change. Continue waits until they're all done.
    pub checks: Option<ProbeChecksView>,
}

/// The touch probe's checks at its tool change.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProbeChecksView {
    /// The probe input has been seen on since the change reached the loading stage.
    pub tapped: bool,
    /// Find bores follows: the bores have been cleaned out (`None` when there's no Find bores).
    pub bores: Option<bool>,
    /// Find stock with `VISE=` follows: the saved vise location is still right (`None` without).
    pub vise: Option<bool>,
    /// A Find stock without `VISE=` follows: it still stops for the probe to be jogged over the stock.
    pub jog_over_stock: bool,
    /// The stops before Find bores that the bores check replaces (program lines, 0-based).
    pub bore_stops: Vec<usize>,
}

impl ProbeChecksView {
    /// What's still to do before Continue, if anything.
    pub fn missing(&self) -> Option<&'static str> {
        if !self.tapped {
            Some("Tap the probe's stylus first, so rcnc knows the probe input works")
        } else if self.bores == Some(false) {
            Some("Clean out the bores to be probed, then tick that they're clean")
        } else if self.vise == Some(false) {
            Some("Check the vise hasn't moved since Find vise location, then tick that its location is accurate")
        } else {
            None
        }
    }
}

/// The checks made at the probe's tool change, kept for its probing cycles.
#[derive(Debug, Clone, Default)]
pub(crate) struct ProbeChecks {
    /// The saved vise location was confirmed: Find stock with `VISE=` goes over the stock and probes it.
    pub vise: bool,
    pub bore_stops: Vec<usize>,
}

pub(super) struct ToolChange {
    pub view: ToolChangeView,
    start: Axes,
    /// Program Z offset when the change started, to keep the tip's work height on the way back.
    z_offset: f64,
    outputs: Outputs,
}

impl Controller {
    /// Start a guided tool change from a running program or MDI. The machine
    /// is stopped (M6 waits for motion to finish). `line` is the M6's program line (not in MDI):
    /// loading the touch probe, what follows it up to the next tool change says what to check.
    pub(super) fn start_tool_change(&mut self, tool: u32, line: Option<usize>) {
        let tc = self.config.tool_change.clone();
        // The change position is in machine coordinates: they mean nothing unhomed.
        let change_x = self.config.change_x();
        if !(self.homed[1] && self.homed[2] && (change_x.is_none() || self.homed[0])) {
            return self.raise("Home the machine before a tool change: its position is in machine coordinates");
        }
        let change_y = self.config.change_y();
        self.sync_position();
        let start = self.machine_position();
        let outputs = self.outputs.clone();
        let spinning = outputs.spindle_on;
        self.tool_measured = false;
        self.setter_hit = None;
        self.probe_checks = None;
        let checks = (tool != 0 && tool == self.config.probing.probe_tool).then(|| {
            let seg = line.map(|l| self.interp.probe_segment(l)).unwrap_or_default();
            ProbeChecksView {
                tapped: false,
                bores: seg.find_bores.then_some(false),
                vise: seg.vise.then_some(false),
                jog_over_stock: seg.find_stock && !seg.vise,
                bore_stops: if seg.find_bores { seg.bore_stops } else { vec![] },
            }
        });
        self.outputs.spindle_on = false;
        self.outputs.flood = false;
        self.outputs.mist = false;

        let feed = self.config.probing.travel_feed;
        let to = |axis, to| Step::MoveTo { axis, to, feed, guarded: true };
        let mut steps = VecDeque::new();
        if spinning {
            steps.push_back(Step::Wait(self.config.spindle_spin_down));
        }
        // Z only ever goes up: to the safe height first, then X and the bed back.
        steps.push_back(to(2, tc.z.max(start[2])));
        if let Some(x) = change_x {
            steps.push_back(to(0, x));
        }
        steps.push_back(to(1, change_y));
        steps.push_back(Step::Then(Box::new(move |c, _| {
            c.stage(ToolChangeStage::Loading);
            let probe = c.tool_change.as_ref().and_then(|t| t.view.checks.as_ref()).map(|k| {
                let mut v = vec!["tap the stylus"];
                if k.bores.is_some() {
                    v.push("clean out the bores");
                }
                if k.vise.is_some() {
                    v.push("check the vise location");
                }
                v.join(", ")
            });
            match probe {
                Some(checks) => c.info(format!("Load the touch probe (T{tool}), {checks}, then press Continue (or Cycle Start)")),
                None => c.info(format!("Load tool {tool}, then press Continue (or Cycle Start)")),
            }
            Ok(vec![])
        })));
        steps.push_back(Step::Operator);
        steps.push_back(Step::Then(Box::new(|c, _| {
            if !c.tool_change.as_ref().is_some_and(|t| t.view.measure) {
                return Ok(vec![]);
            }
            c.stage(ToolChangeStage::Measuring);
            Ok(measure_steps(&c.config.probing))
        })));
        steps.push_back(Step::Then(Box::new(move |c, _| c.tool_change_return())));

        self.info(format!("Tool change to T{tool} started"));
        self.tool_change = Some(ToolChange {
            view: ToolChangeView {
                from: self.loaded_tool,
                to: tool,
                stage: ToolChangeStage::Moving,
                measure: self.config.tool_change_touch_off,
                return_to_start: tc.return_to_start,
                problem: None,
                measured_stickout: None,
                checks,
            },
            start,
            z_offset: self.interp.program_offset()[2],
            outputs,
        });
        self.routine = Some(Routine { name: format!("Tool change to T{tool}"), steps, hits: Vec::new() });
        self.routine_move = None;
        self.routine_return = self.mode;
        self.mode = crate::controller::Mode::Probing;
        if let Some(exec) = &mut self.exec {
            exec.wait = Wait::Routine;
        }
        self.routine_step();
    }

    fn stage(&mut self, stage: ToolChangeStage) {
        if let Some(t) = &mut self.tool_change {
            t.view.stage = stage;
        }
    }

    /// The operator has loaded the tool, or checked things after readings that disagreed.
    pub(super) fn tool_change_continue(&mut self, measure: Option<bool>) {
        let Some(t) = &mut self.tool_change else { return };
        if !self.routine_waiting || !matches!(t.view.stage, ToolChangeStage::Loading | ToolChangeStage::Mismatch) {
            return;
        }
        #[cfg(test)]
        if let Some(k) = t.view.checks.as_mut().filter(|_| self.test_operator_checks) {
            k.tapped = true;
            k.bores = k.bores.map(|_| true);
            k.vise = k.vise.map(|_| true);
        }
        if t.view.stage == ToolChangeStage::Loading {
            if let Some(why) = t.view.checks.as_ref().and_then(|k| k.missing()) {
                return self.info(why);
            }
        }
        if let Some(m) = measure {
            t.view.measure = m;
        }
        self.routine_waiting = false;
        self.routine_step();
    }

    /// The operator ticked (or unticked) the probe's checks.
    pub(super) fn tool_change_checks(&mut self, bores: bool, vise: bool) {
        let Some(k) = self.tool_change.as_mut().filter(|t| t.view.stage == ToolChangeStage::Loading).and_then(|t| t.view.checks.as_mut()) else {
            return;
        };
        if let Some(b) = &mut k.bores {
            *b = bores;
        }
        if let Some(v) = &mut k.vise {
            *v = vise;
        }
    }

    /// Note a tap on the stylus while the probe's tool change waits for one.
    pub(super) fn tool_change_watch(&mut self) {
        let touching = self.config.probe.is_some_and(|p| p.is_active(self.status.inputs));
        let Some(t) = &mut self.tool_change else { return };
        if !touching || t.view.stage != ToolChangeStage::Loading {
            return;
        }
        if let Some(k) = t.view.checks.as_mut().filter(|k| !k.tapped) {
            k.tapped = true;
            self.info("Probe input works");
        }
    }

    fn set_problem(&mut self, problem: Option<String>) {
        if let Some(t) = &mut self.tool_change {
            t.view.problem = problem;
        }
    }

    fn tool_change_return(&mut self) -> Result<Vec<Step>, String> {
        let Some(t) = &self.tool_change else { return Ok(vec![]) };
        let (start, z_offset, outputs) = (t.start, t.z_offset, t.outputs.clone());
        let view = t.view.clone();
        if self.loaded_tool != view.to {
            // the probe's offset from the spindle axis changes when it's taken out
            self.probe_runout = None;
        }
        self.loaded_tool = view.to;
        self.info(format!("Tool {} loaded", view.to));
        // the probe's checks, for its probing cycles up to the next tool change
        self.probe_checks = view.checks.as_ref().map(|k| ProbeChecks { vise: k.vise == Some(true), bore_stops: k.bore_stops.clone() });
        // The program carries on expecting the spindle and coolant as they
        // were, whether or not the tool goes back to where it started.
        // Never start the spindle with the touch probe in it.
        let probe = view.to != 0 && view.to == self.config.probing.probe_tool;
        if probe && outputs.spindle_on {
            self.info("The touch probe is loaded: the spindle stays off");
        }
        let spin = outputs.spindle_on && !probe;
        let restore = Step::Then(Box::new(move |c, _| {
            c.outputs.spindle_on = spin;
            c.outputs.spindle_ccw = outputs.spindle_ccw;
            c.outputs.spindle_duty = outputs.spindle_duty;
            c.outputs.spindle_rpm = outputs.spindle_rpm;
            c.outputs.flood = outputs.flood;
            c.outputs.mist = outputs.mist;
            Ok(if spin { vec![Step::Wait(c.config.spindle_spin_up)] } else { vec![] })
        }));
        if !view.return_to_start {
            return Ok(vec![restore]);
        }
        self.stage(ToolChangeStage::Returning);
        let tc = self.config.tool_change.clone();
        let feed = self.config.probing.travel_feed;
        let to = |axis, to| Step::MoveTo { axis, to, feed, guarded: true };
        let pos = self.machine_position();
        let mut steps = vec![to(2, tc.z.max(pos[2])), to(0, start[0]), to(1, start[1]), restore];
        if view.measure {
            // Same work Z as before: a longer tool stops higher, so it can't hit
            // the part. Never lower than where the change began, though: with a
            // stale Z zero the same work Z can be down in the part or the setter.
            let z = start[2] - z_offset + self.interp.program_offset()[2];
            if z >= start[2] - 1e-6 {
                steps.push(to(2, z));
            } else {
                self.info("Staying at the safe height: going back to the same work Z would go lower than where the change began");
            }
        } else {
            // An unmeasured tool may be longer than the last: stay at the safe
            // height and let the program's own moves bring it down.
            self.info("The new tool wasn't measured: staying at the safe height");
        }
        Ok(steps)
    }

    /// The tool change routine ended; let the program carry on, or stop it.
    pub(super) fn end_tool_change(&mut self, result: &Result<(), String>) {
        if self.tool_change.take().is_none() {
            return;
        }
        match result {
            Ok(()) => {
                if let Some(Exec { wait: wait @ Wait::Routine, .. }) = &mut self.exec {
                    *wait = Wait::None;
                }
            }
            // Don't carry on cutting with an unmeasured tool or from the wrong place.
            Err(e) => self.raise(format!("Tool change failed: {e}")),
        }
    }
}

/// Measure the tool twice on the setter. If the readings agree, set work Z
/// from their average; if not, lift off the setter and wait for the operator
/// to check things, then measure again.
fn measure_steps(s: &ProbeSettings) -> Vec<Step> {
    let mut steps = probing::setter_steps(s);
    let s = s.clone();
    steps.push(Step::Then(Box::new(move |c, hits| match probing::setter_reading(&s, hits) {
        Ok(hit) => match c.stickout_problem(hit) {
            None => Ok(probing::touch_off(c, &s, hit)),
            Some(e) => Ok(mismatch(c, &s, e)),
        },
        Err(e) => Ok(mismatch(c, &s, e)),
    })));
    steps
}

/// Lift off the setter and wait for the operator to check things, then measure again.
fn mismatch(c: &mut Controller, s: &ProbeSettings, e: String) -> Vec<Step> {
    c.info(format!("Tool change paused: {e}"));
    c.stage(ToolChangeStage::Mismatch);
    c.set_problem(Some(e));
    let up = s.setter.travel_z.max(c.machine_position()[2]);
    vec![
        Step::MoveTo { axis: 2, to: up, feed: s.travel_feed, guarded: false },
        Step::Operator,
        Step::Then(Box::new(|c, _| {
            c.set_problem(None);
            c.stage(ToolChangeStage::Measuring);
            c.info("Measuring the tool again");
            Ok(measure_steps(&c.config.probing))
        })),
    ]
}

impl Controller {
    /// The tool's tip touched the setter at machine Z `hit`: remember it, and with the nut face calibrated,
    /// check its stick-out against the program's (the simulation assumed it). Why it's wrong, if it is.
    fn stickout_problem(&mut self, hit: f64) -> Option<String> {
        let to = self.tool_change.as_ref()?.view.to;
        self.last_setter_hit = Some((to, hit));
        let k = self.config.tool_change.nut_face_z?;
        let measured = k - hit;
        let expected = self.interp.tools.get(&to).map(|t| t.stickout).filter(|s| *s > 0.0);
        if let Some(t) = &mut self.tool_change {
            t.view.measured_stickout = Some(measured);
        }
        let Some(expected) = expected else {
            self.info(format!("T{to} stick-out measured {measured:.1} mm (the program gives none)"));
            return None;
        };
        self.info(format!("T{to} stick-out measured {measured:.1} mm (program expects {expected:.1})"));
        let tol = self.config.tool_change.stickout_tolerance;
        ((measured - expected).abs() > tol).then(|| {
            format!(
                "T{to} sticks out {measured:.1} mm, the program expects {expected:.1} (more than {tol:.1} off): the simulation's tool bend and holder clearance assumed {expected:.1}. Set the stick-out, then Continue to measure again."
            )
        })
    }

    /// The tool last measured on the setter has exactly `stickout` mm out of the collet nut: calibrate the
    /// stick-out check from it.
    pub(super) fn calibrate_stickout(&mut self, stickout: f64) {
        let Some((tool, hit)) = self.last_setter_hit else {
            return self.info("Measure a tool on the setter first (a tool change), then calibrate the stick-out check with it");
        };
        if !(stickout > 0.0) {
            return self.info("The stick-out must be more than zero");
        }
        self.config.tool_change.nut_face_z = Some(hit + stickout);
        self.report(format!("Stick-out check calibrated from T{tool} at {stickout:.1} mm: tools measured from now on report their stick-out"));
    }
}
