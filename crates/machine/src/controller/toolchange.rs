//! Guided tool change at M6: stop the spindle, lift Z, move to the change
//! position, wait for the operator to load the new tool, measure it on the
//! tool setter, then go back to where the change started, with the tool tip
//! at the same work height and the spindle and coolant as they were.

use std::collections::VecDeque;

use gcode::Axes;

use super::{Controller, Exec, Wait};
use crate::backend::Outputs;
use crate::probing::{self, ProbeJob, Routine, Step};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolChangeStage {
    /// Stopping the spindle and moving to the change position.
    Moving,
    /// At the change position, waiting for the operator to load the tool.
    Loading,
    /// Measuring the new tool on the tool setter.
    Measuring,
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
        self.sync_position();
        let tc = self.config.tool_change.clone();
        let start = self.machine_position();
        let outputs = self.outputs.clone();
        let spinning = outputs.spindle_on;
        self.outputs.spindle_on = false;
        self.outputs.flood = false;
        self.outputs.mist = false;

        let feed = self.config.probing.travel_feed;
        let to = |axis, to| Step::MoveTo { axis, to, feed, guarded: true };
        let mut steps = VecDeque::new();
        if spinning {
            steps.push_back(Step::Wait(self.config.spindle_spin_down));
        }
        // Z only ever goes up: to the safe height first, then over.
        steps.extend([to(2, tc.z.max(start[2])), to(0, tc.x), to(1, tc.y)]);
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
            let routine = probing::build(ProbeJob::ToolTouchOff, &c.config.probing, c.machine_position());
            Ok(routine.steps.into())
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

    /// The operator has loaded the tool.
    pub(super) fn tool_change_continue(&mut self, measure: Option<bool>) {
        let Some(t) = &mut self.tool_change else { return };
        if !self.routine_waiting || t.view.stage != ToolChangeStage::Loading {
            return;
        }
        if let Some(m) = measure {
            t.view.measure = m;
        }
        self.routine_waiting = false;
        self.routine_step();
    }

    fn tool_change_return(&mut self) -> Result<Vec<Step>, String> {
        let Some(t) = &self.tool_change else { return Ok(vec![]) };
        let (start, z_offset, outputs) = (t.start, t.z_offset, t.outputs.clone());
        let view = t.view.clone();
        self.loaded_tool = view.to;
        self.info(format!("Tool {} loaded", view.to));
        if !view.return_to_start {
            return Ok(vec![]);
        }
        self.stage(ToolChangeStage::Returning);
        let tc = self.config.tool_change.clone();
        let feed = self.config.probing.travel_feed;
        let to = |axis, to| Step::MoveTo { axis, to, feed, guarded: true };
        // Same work Z as before: a longer tool stops higher, so it can't hit the part.
        let z = start[2] - z_offset + self.interp.program_offset()[2];
        let pos = self.machine_position();
        let mut steps = vec![to(2, tc.z.max(pos[2])), to(0, start[0]), to(1, start[1])];
        steps.push(Step::Then(Box::new(move |c, _| {
            c.outputs.spindle_on = outputs.spindle_on;
            c.outputs.spindle_ccw = outputs.spindle_ccw;
            c.outputs.spindle_duty = outputs.spindle_duty;
            c.outputs.spindle_rpm = outputs.spindle_rpm;
            c.outputs.flood = outputs.flood;
            c.outputs.mist = outputs.mist;
            Ok(if outputs.spindle_on { vec![Step::Wait(c.config.spindle_spin_up)] } else { vec![] })
        })));
        steps.push(to(2, z));
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
