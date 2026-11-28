//! The simulator (as `cnc-controller --sim` sets it up) running a program shaped like rCAD's rcnc post,
//! cutter -> touch probe -> cutter, each with its coolant: where the flood comes on and goes off.
//!
//! `cargo test -p cnc-machine --test toolchange_coolant_sim -- --nocapture` prints the timeline.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use machine::backend::{AxisMove, Backend, BoardInfo, Outputs, Result, Status};
use machine::controller::{Command, Controller, Mode, ToolChangeStage};
use machine::planner::Segment;
use machine::sim::{dmc2_sim_config, ManualClock, Simulator};
use machine::MachineConfig;

/// The simulator, with the operator's finger on the probe's stylus when `tap` is set.
struct Tap(Simulator, Arc<AtomicBool>, u32);

impl Backend for Tap {
    fn name(&self) -> &str {
        self.0.name()
    }
    fn connect(&mut self, config: &MachineConfig) -> Result<BoardInfo> {
        self.0.connect(config)
    }
    fn disconnect(&mut self) {
        self.0.disconnect()
    }
    fn segment_period(&self) -> f64 {
        self.0.segment_period()
    }
    fn queue_capacity(&self) -> usize {
        self.0.queue_capacity()
    }
    fn poll(&mut self, outputs: &Outputs) -> Result<Status> {
        let mut s = self.0.poll(outputs)?;
        if self.1.load(Ordering::Relaxed) {
            s.inputs ^= self.2;
        }
        Ok(s)
    }
    fn push_segments(&mut self, segments: &[Segment], last: bool) -> Result<()> {
        self.0.push_segments(segments, last)
    }
    fn axis_move(&mut self, m: &AxisMove) -> Result<()> {
        self.0.axis_move(m)
    }
    fn stop_axis_move(&mut self) -> Result<()> {
        self.0.stop_axis_move()
    }
    fn abort(&mut self) -> Result<()> {
        self.0.abort()
    }
    fn set_position(&mut self, steps: [i64; 6]) -> Result<()> {
        self.0.set_position(steps)
    }
}

/// Like rCAD's rcnc post: M5/M9 at the top before every M6, then S/M3 and M8 after it for a cutter; the probe
/// gets neither. One hand-written M8 with the probe in (line 24) to check the interlock.
const PROGRAM: &str = "\
G21 G17 G40 G49 G80 G90 G91.1 G94 G64
G54
(RCNC Z_REFERENCE Vise)
(Face - T3)
G53 G0 Z0
T3 M6
S9000 M3
G4 P2
M8
G0 X0 Y0
G0 Z30
G1 Z25 F600
G1 X20 F1500
G0 Z30
(Probe - T99)
G53 G0 Z0
M5
M9
T99 M6
G0 X10 Y10
G0 Z30
G38.2 Z5 F300
G0 Z30
M8
G0 X0
(Finish - T4)
G53 G0 Z0
T4 M6
S12000 M3
G4 P2
M8
G0 X0 Y0
G0 Z30
G1 Z25 F600
G1 X20 F1500
G0 Z30
G53 G0 Z0
M5
M9
M30
";

#[test]
fn cutter_probe_cutter_with_m8_in_the_simulator() {
    let clock = Arc::new(ManualClock::default());
    let config = dmc2_sim_config();
    let mut sim = Simulator::new(clock.clone());
    sim.add_stock(&config, 60.0, 60.0, 20.0);
    // work X/Y zero on the stock's middle (the middle of the table); Z from the tool setter at each M6
    let middle = |axis: usize| machine::sim::homed_position(&config, &sim.world, axis, sim.world.travel[axis] as f64 / config.axes[axis].steps_per_unit / 2.0);
    let (mx, my) = (middle(0), middle(1));
    let probe = config.probe.expect("the DMC2 has a probe input");
    let tap = Arc::new(AtomicBool::new(false));
    let mut c = Controller::new(config.clone(), Box::new(Tap(sim, tap.clone(), 1 << probe.pin)), clock.clone());
    let step = |c: &mut Controller| {
        clock.advance(0.005);
        c.update();
    };
    c.handle(Command::Connect);
    c.handle(Command::Reset);
    c.handle(Command::Home(None));
    for _ in 0..200_000 {
        step(&mut c);
        if c.snapshot().homed[..3].iter().all(|h| *h) && c.mode() == Mode::Idle {
            break;
        }
    }
    assert!(c.snapshot().homed[..3].iter().all(|h| *h), "homed");
    c.handle(Command::SetWorkOffset { system: 0, offset: [mx, my, 0.0, 0.0, 0.0, 0.0] });
    c.handle(Command::Load { name: "toolchange-coolant.nc".into(), text: PROGRAM.into() });
    c.handle(Command::CycleStart);

    let (mut t, mut flood, mut tool) = (0.0, false, None::<u32>);
    let mut timeline = vec![];
    let mut on_while_probe = false;
    let mut released = 0;
    loop {
        step(&mut c);
        t += 0.005;
        let s = c.snapshot();
        let tc = s.tool_change.clone();
        // the operator: taps the probe's stylus when asked, then presses Continue
        if let Some(v) = tc.as_ref().filter(|v| v.stage == ToolChangeStage::Loading) {
            match v.checks.as_ref() {
                Some(k) if !k.tapped => {
                    tap.store(true, Ordering::Relaxed);
                    released = 0;
                }
                _ => {
                    // let go of the stylus, then Continue once the board has seen it let go
                    tap.store(false, Ordering::Relaxed);
                    released += 1;
                    if released > 20 {
                        c.handle(Command::ToolChangeContinue { measure: None });
                    }
                }
            }
        }
        let now = tc.as_ref().map(|v| v.to);
        if now != tool {
            if let Some(to) = now {
                timeline.push(format!("{t:7.2}s  M6 T{to} starts"));
            }
            tool = now;
        }
        if s.flood != flood {
            flood = s.flood;
            let at = match &tc {
                Some(v) => format!("tool change to T{} ({:?})", v.to, v.stage),
                None => format!("program line {}", s.line.map_or(0, |l| l + 1)),
            };
            timeline.push(format!("{t:7.2}s  flood {}  at machine Z{:.3}, {at}", if flood { "ON " } else { "off" }, s.machine[2]));
        }
        if s.flood && (s.loaded_tool == 99 && tc.is_none() || tc.as_ref().is_some_and(|v| v.to == 99)) {
            on_while_probe = true;
        }
        if matches!(s.mode, Mode::Idle | Mode::Alarm) && t > 1.0 {
            break;
        }
        assert!(t < 900.0, "still running: {:?}", s.log);
    }
    let s = c.snapshot();
    println!("--- coolant timeline ---");
    for l in &timeline {
        println!("{l}");
    }
    println!("--- log ---");
    for l in s.log.iter().filter(|l| !l.starts_with("Probe: touched")) {
        println!("{l}");
    }
    assert_eq!(s.mode, Mode::Idle, "{:?}", s.log);
    assert!(s.log.iter().any(|l| l.contains("Program finished")), "{:?}", s.log);
    assert!(!on_while_probe, "never coolant with the probe in");
    assert!(s.log.iter().any(|l| l.contains("Warning: line 24: M8 (flood) refused")), "{:?}", s.log);
    // T3 and T4: on over the setter, off at the fast touch, then on at the program's M8 (lines 9 and 31)
    let ons: Vec<&String> = timeline.iter().filter(|l| l.contains("flood ON")).collect();
    assert_eq!(ons.len(), 4, "{timeline:#?}");
    assert!(ons[0].contains("T3 (Measuring)") && ons[1].contains("program line 9"), "{timeline:#?}");
    assert!(ons[2].contains("T4 (Measuring)") && ons[3].contains("program line 31"), "{timeline:#?}");
    assert_eq!(s.loaded_tool, 4);
    assert!(!s.flood);
}
