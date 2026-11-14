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
    /// is stopped (M6 waits for motion to finish).
    pub(super) fn start_tool_change(&mut self, tool: u32) {
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
            c.info(format!("Load tool {tool}, then press Continue (or Cycle Start)"));
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
        if let Some(m) = measure {
            t.view.measure = m;
        }
        self.routine_waiting = false;
        self.routine_step();
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
        self.loaded_tool = view.to;
        self.info(format!("Tool {} loaded", view.to));
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
        Ok(hit) => Ok(probing::touch_off(c, &s, hit)),
        Err(e) => {
            c.info(format!("Tool change paused: {e}"));
            c.stage(ToolChangeStage::Mismatch);
            c.set_problem(Some(e));
            let up = s.setter.travel_z.max(c.machine_position()[2]);
            Ok(vec![
                Step::MoveTo { axis: 2, to: up, feed: s.travel_feed, guarded: false },
                Step::Operator,
                Step::Then(Box::new(|c, _| {
                    c.set_problem(None);
                    c.stage(ToolChangeStage::Measuring);
                    c.info("Measuring the tool again");
                    Ok(measure_steps(&c.config.probing))
                })),
            ])
        }
    })));
    steps
}
