//! Coolant wash-down: wash the chips off the vise and table with the flood
//! coolant. The spindle stops, Z rises to the wash height (never lower than
//! it is), the coolant comes on, and the spindle head, with the coolant
//! nozzle on it, sweeps the area: a lap round its edge, then rows across it,
//! every other pass crosswise. Then the coolant goes off and the spindle goes
//! back to where it started. Run from the Wash down button, or from a program
//! with an `(RCNC WASH_DOWN)` comment, after which the program carries on with
//! the spindle and coolant as they were.
//!
//! Every move is a single-axis move along X or Y at the wash height, inside
//! the machine's travel (`MachineConfig::wash_area`). Stop, Feed Hold and
//! E-stop work as in any other routine; Stop switches the coolant off.

use std::collections::VecDeque;

use gcode::Axes;

use super::{Controller, Exec, Mode, Wait};
use crate::backend::Outputs;
use crate::probing::{fmt, Routine, Step};

pub(super) struct Wash {
    /// Where the spindle was, to go back to.
    start: Axes,
    /// The spindle and coolant as they were, for a program that carries on.
    outputs: Outputs,
    /// Started by a program's WASH_DOWN comment.
    in_program: bool,
}

/// The sweep, as moves of one axis at a time: for each pass a lap round the
/// area's edge, then rows `spacing` apart (along X on odd passes, along Y on
/// even ones), each row the other way from the one before.
pub(crate) fn sweep(x: [f64; 2], y: [f64; 2], spacing: f64, passes: u32) -> Vec<(usize, f64)> {
    let mut moves = vec![(0, x[0]), (1, y[0])];
    for pass in 0..passes {
        // round the edge, back to the low corner
        moves.extend([(0, x[1]), (1, y[1]), (0, x[0]), (1, y[0])]);
        // rows along `along`, stepping across `across`
        let (along, across, a, b) = if pass % 2 == 0 { (0, 1, x, y) } else { (1, 0, y, x) };
        let rows = ((b[1] - b[0]) / spacing).ceil().max(1.0) as usize;
        for i in 0..=rows {
            let at = (b[0] + i as f64 * (b[1] - b[0]) / rows as f64).min(b[1]);
            if i > 0 {
                moves.push((across, at));
            }
            moves.push((along, if i % 2 == 0 { a[1] } else { a[0] }));
        }
        // back to the low corner for the next pass
        moves.extend([(along, a[0]), (across, b[0])]);
    }
    moves
}

impl Controller {
    /// Start the wash-down: by hand (idle) or from a program's comment
    /// (`passes` from it, None: as set up).
    pub(super) fn start_wash_down(&mut self, passes: Option<u32>) {
        let in_program = self.mode == Mode::Running && self.exec.is_some();
        if !(self.is_ready() || in_program) {
            return self.info("Wash down needs the machine idle");
        }
        let fail = |c: &mut Controller, why: String| {
            if in_program {
                c.raise(format!("Wash down: {why}"))
            } else {
                c.info(format!("Wash down: {why}"))
            }
        };
        if !self.homed[..3].iter().all(|h| *h) {
            return fail(self, "home X, Y and Z first: the area it sweeps is in machine coordinates".into());
        }
        if self.config.probe.is_none() {
            return fail(self, "no probe input is configured (its moves stop on it, like a tool change's)".into());
        }
        if self.generator.feed_override == 0.0 {
            return fail(self, "feed override is at 0%".into());
        }
        if self.probe_in_spindle() {
            return fail(self, format!("the touch probe (T{}) is in the spindle and must stay dry: load a cutter first", self.loaded_tool));
        }
        let (x, y) = match self.config.wash_area() {
            Ok(a) => a,
            Err(e) => return fail(self, e),
        };
        let w = self.config.wash_down.clone();
        let passes = passes.unwrap_or(w.passes).max(1);
        self.sync_position();
        let start = self.machine_position();
        let outputs = self.outputs.clone();
        let spinning = outputs.spindle_on;
        self.outputs.spindle_on = false;
        // off until the first corner: no spray on the way there
        self.outputs.flood = false;
        self.outputs.mist = false;

        let feed = w.feed;
        let travel = self.config.probing.travel_feed;
        let mv = |axis, to, feed| Step::MoveTo { axis, to, feed, guarded: true };
        let mut steps = VecDeque::new();
        if spinning {
            steps.push_back(Step::Wait(self.config.spindle_spin_down));
        }
        // Up first (never down), then over to the first corner with the coolant still off.
        let z = w.z.max(start[2]);
        steps.push_back(mv(2, z, travel));
        let mut moves = sweep(x, y, w.spacing, passes).into_iter();
        for (axis, to) in moves.by_ref().take(2) {
            steps.push_back(mv(axis, to, travel));
        }
        steps.push_back(Step::Then(Box::new(|c, _| {
            if c.probe_in_spindle() {
                return Err("the touch probe is in the spindle: no coolant".into());
            }
            c.outputs.flood = true;
            c.info("Wash down: coolant on");
            Ok(vec![])
        })));
        steps.push_back(Step::Wait(w.soak));
        steps.extend(moves.map(|(axis, to)| mv(axis, to, feed)));
        steps.push_back(Step::Then(Box::new(|c, _| {
            c.outputs.flood = false;
            c.info("Wash down: coolant off, going back");
            Ok(vec![])
        })));
        // Back over the start at the wash height, then down to where it was.
        steps.push_back(mv(0, start[0], travel));
        steps.push_back(mv(1, start[1], travel));
        steps.push_back(mv(2, start[2], travel));
        steps.push_back(Step::Then(Box::new(|c, _| {
            let Some(wash) = &c.wash else { return Ok(vec![]) };
            if !wash.in_program {
                return Ok(vec![]);
            }
            // The program carries on expecting the spindle and coolant as they were.
            let o = wash.outputs.clone();
            let probe = c.probe_in_spindle();
            let spin = o.spindle_on && !probe;
            c.outputs.spindle_on = spin;
            c.outputs.spindle_ccw = o.spindle_ccw;
            c.outputs.spindle_duty = o.spindle_duty;
            c.outputs.spindle_rpm = o.spindle_rpm;
            c.outputs.flood = o.flood && !probe;
            c.outputs.mist = o.mist && !probe;
            Ok(if spin { vec![Step::Wait(c.config.spindle_spin_up)] } else { vec![] })
        })));

        self.info(format!(
            "Wash down: {passes} pass{} over machine X{} to X{}, Y{} to Y{} at Z{}, {} mm/min",
            if passes == 1 { "" } else { "es" },
            fmt(x[0]),
            fmt(x[1]),
            fmt(y[0]),
            fmt(y[1]),
            fmt(z),
            fmt(feed)
        ));
        self.wash = Some(Wash { start, outputs, in_program });
        self.routine = Some(Routine { name: "Wash down".into(), steps, hits: Vec::new() });
        self.routine_move = None;
        self.routine_resume = None;
        self.routine_return = self.mode;
        self.mode = Mode::Probing;
        if let Some(exec) = &mut self.exec {
            exec.wait = Wait::Routine;
        }
        self.routine_step();
    }

    pub(super) fn end_wash_down(&mut self, result: &Result<(), String>) {
        let Some(wash) = self.wash.take() else { return };
        self.outputs.flood = result.is_ok() && wash.in_program && wash.outputs.flood && !self.probe_in_spindle();
        match result {
            Ok(()) => {
                if let Some(Exec { wait: wait @ Wait::Routine, .. }) = &mut self.exec {
                    *wait = Wait::None;
                }
            }
            // The program can't carry on from somewhere over the vise with the spindle off.
            Err(e) if wash.in_program => self.raise(format!(
                "Wash down failed: {e} (it started at machine X{} Y{} Z{})",
                fmt(wash.start[0]),
                fmt(wash.start[1]),
                fmt(wash.start[2])
            )),
            Err(_) => {}
        }
    }

    pub fn washing(&self) -> bool {
        self.wash.is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::sweep;
    use crate::controller::{Command, Controller, Mode};
    use crate::sim::{ManualClock, Simulator};
    use crate::MachineConfig;
    use gcode::Spindle;

    #[test]
    fn sweep_covers_the_area_inside_it() {
        let (x, y) = ([0.0, 100.0], [10.0, 70.0]);
        let moves = sweep(x, y, 25.0, 2);
        assert_eq!(&moves[..2], &[(0, 0.0), (1, 10.0)]);
        let mut p = [0.0, 10.0];
        let mut rows_x = vec![];
        let mut rows_y = vec![];
        for &(axis, to) in &moves {
            assert!((x[0]..=x[1]).contains(&to) || axis == 1, "{to}");
            assert!((y[0]..=y[1]).contains(&to) || axis == 0, "{to}");
            if axis == 0 { rows_x.push(p[1]) } else { rows_y.push(p[0]) }
            p[axis] = to;
        }
        // pass 1's rows along X: 60 mm in 3 rows of 20 (no more than 25 apart)
        for yy in [10.0, 30.0, 50.0, 70.0] {
            assert!(rows_x.iter().any(|r| (r - yy).abs() < 1e-9), "no row along X at Y{yy}: {rows_x:?}");
        }
        // pass 2 crosswise: 100 mm in 4 columns of 25
        for xx in [0.0, 25.0, 50.0, 75.0, 100.0] {
            assert!(rows_y.iter().any(|r| (r - xx).abs() < 1e-9), "no row along Y at X{xx}: {rows_y:?}");
        }
        assert_eq!(p, [0.0, 10.0], "ends back at the low corner");
    }

    fn homed(config: MachineConfig) -> (Controller, Arc<ManualClock>) {
        let clock = Arc::new(ManualClock::default());
        let sim = Simulator::new(clock.clone());
        let mut c = Controller::new(config, Box::new(sim), clock.clone());
        c.handle(Command::Connect);
        c.handle(Command::Reset);
        c.handle(Command::Home(None));
        run(&mut c, &clock, 120.0, |c| c.mode() == Mode::Idle);
        assert!(c.snapshot().homed[..3].iter().all(|h| *h));
        (c, clock)
    }

    /// Run until `done`, checking every cycle with `each`.
    fn run(c: &mut Controller, clock: &ManualClock, max: f64, done: impl Fn(&Controller) -> bool) {
        let mut t = 0.0;
        while !done(c) {
            clock.advance(0.005);
            c.update();
            t += 0.005;
            assert!(t < max, "still {:?} after {max} s: {:?}", c.mode(), c.snapshot().log);
        }
    }

    fn area_config() -> MachineConfig {
        let mut config = MachineConfig::dmc2();
        let w = &mut config.wash_down;
        w.around_vise = false;
        (w.x, w.y, w.z, w.spacing, w.passes, w.feed) = ([20.0, 120.0], [480.0, 400.0], 0.0, 40.0, 1, 3000.0);
        config
    }

    #[test]
    fn washes_the_area_by_hand_and_comes_back() {
        let (mut c, clock) = homed(area_config());
        c.handle(Command::Mdi("G53 G0 X60 Y450 Z-20".into()));
        run(&mut c, &clock, 30.0, |c| c.mode() == Mode::Idle);
        let start = c.snapshot().machine;
        c.handle(Command::WashDown);
        assert!(c.washing(), "{:?}", c.snapshot().log);
        let (mut lo, mut hi, mut flooded, mut wet_min_x) = ([f64::MAX; 3], [f64::MIN; 3], false, f64::MAX);
        let mut t = 0.0;
        while c.washing() {
            clock.advance(0.005);
            c.update();
            t += 0.005;
            assert!(t < 600.0, "{:?}", c.snapshot().log);
            let s = c.snapshot();
            for i in 0..3 {
                lo[i] = lo[i].min(s.machine[i]);
                hi[i] = hi[i].max(s.machine[i]);
            }
            if s.flood {
                flooded = true;
                wet_min_x = wet_min_x.min(s.machine[0]);
                assert!(s.machine[2] >= -1e-9, "sprays from the wash height, Z{}", s.machine[2]);
            }
        }
        let s = c.snapshot();
        assert!(flooded, "the coolant came on");
        assert!(!s.flood, "and went off again");
        assert_eq!(s.mode, Mode::Idle);
        assert!(s.log.iter().any(|l| l == "Wash down done"), "{:?}", s.log);
        assert!((lo[0] - 20.0).abs() < 1e-6 && (hi[0] - 120.0).abs() < 1e-6, "X {lo:?} {hi:?}");
        assert!((lo[1] - 400.0).abs() < 1e-6 && (hi[1] - 480.0).abs() < 1e-6, "Y {lo:?} {hi:?}");
        assert!(lo[2] >= start[2] - 1e-6 && (hi[2] - 0.0).abs() < 1e-6, "Z only rose to the wash height: {lo:?} {hi:?}");
        assert!((wet_min_x - 20.0).abs() < 1e-6);
        for i in 0..3 {
            assert!((s.machine[i] - start[i]).abs() < 1e-6, "back where it started: {:?} vs {start:?}", s.machine);
        }
    }

    #[test]
    fn a_program_washes_down_and_carries_on_as_it_was() {
        let (mut c, clock) = homed(area_config());
        c.handle(Command::Load {
            name: "t.nc".into(),
            text: "G53 G0 X60 Y450 Z-20\nS10000 M3\nM8\n(RCNC WASH_DOWN PASSES=1)\nG91 G1 X5 F600\nM5 M9\nM30".into(),
        });
        c.handle(Command::CycleStart);
        run(&mut c, &clock, 120.0, |c| c.washing() || c.mode() == Mode::Alarm);
        assert!(c.washing(), "{:?}", c.snapshot().log);
        let mut spun_while_washing = false;
        let mut t = 0.0;
        while c.washing() {
            clock.advance(0.005);
            c.update();
            t += 0.005;
            assert!(t < 600.0);
            let s = c.snapshot();
            // the spindle stops (once its spin-down wait is over, nothing moves with it on)
            spun_while_washing |= s.spindle != Spindle::Off && s.velocity > 0.0;
        }
        assert!(!spun_while_washing);
        let s = c.snapshot();
        assert_eq!(s.mode, Mode::Running, "{:?}", s.log);
        assert!(s.spindle != Spindle::Off && s.flood, "spindle and coolant back on for the rest of the program");
        run(&mut c, &clock, 120.0, |c| c.mode() == Mode::Idle || c.mode() == Mode::Alarm);
        let s = c.snapshot();
        assert_eq!(s.mode, Mode::Idle, "{:?}", s.log);
        assert!((s.machine[0] - 65.0).abs() < 1e-6 && (s.machine[1] - 450.0).abs() < 1e-6 && (s.machine[2] + 20.0).abs() < 1e-6, "{:?}", s.machine);
    }

    #[test]
    fn needs_homing_and_a_vise_location() {
        let clock = Arc::new(ManualClock::default());
        let mut c = Controller::new(area_config(), Box::new(Simulator::new(clock.clone())), clock.clone());
        c.handle(Command::Connect);
        c.handle(Command::Reset);
        c.handle(Command::WashDown);
        assert!(!c.washing());
        assert!(c.snapshot().log.back().unwrap().contains("home X, Y and Z first"));

        let (mut c, _clock) = homed(MachineConfig::dmc2());
        c.handle(Command::WashDown);
        assert!(!c.washing());
        assert!(c.snapshot().log.back().unwrap().contains("Find vise location"), "{:?}", c.snapshot().log);
    }

    #[test]
    fn the_area_around_the_vise_stays_inside_the_travel() {
        let mut config = MachineConfig::dmc2();
        config.probing.vise.location = Some(crate::ViseLocation {
            faces: [10.0, 43.27],
            y: 420.0,
            angle: 0.0,
            setter_above_top: 0.0,
            gap: 33.27,
            length: None,
            web_below_top: 0.0,
        });
        let (x, y) = config.wash_area().unwrap();
        // X can't go below its home at -8; Y: 420 ± (72.25 + 15)
        assert!((x[0] - -8.0).abs() < 1e-9 && (x[1] - (26.635 + 158.85 + 15.0)).abs() < 1e-9, "{x:?}");
        assert!((y[0] - (420.0 - 87.25)).abs() < 1e-9 && (y[1] - (420.0 + 87.25)).abs() < 1e-9, "{y:?}");
        config.probing.vise.location.as_mut().unwrap().y = 500.0;
        assert!((config.wash_area().unwrap().1[1] - 512.0).abs() < 1e-9, "never past the Y home switch");
    }

    #[test]
    fn stop_turns_the_coolant_off() {
        let (mut c, clock) = homed(area_config());
        c.handle(Command::WashDown);
        run(&mut c, &clock, 60.0, |c| c.snapshot().flood);
        c.handle(Command::Stop);
        run(&mut c, &clock, 10.0, |c| c.mode() == Mode::Idle);
        assert!(!c.snapshot().flood && !c.washing());
    }
}

