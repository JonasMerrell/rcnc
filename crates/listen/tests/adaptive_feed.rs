//! The adaptive feed limiter against a model of the DMC2's spindle fitted to the run that broke T24 (stick-out
//! gauge op 1a, 2026-11-17; `broken_tool.rs` has its recording). All sags are on the bog guard's scale: over the
//! no-load sag it learns with the spindle on and the machine standing (0.8%: 397 Hz at S24000; in air near the
//! work it reads the same since the analysis stopped locking onto the 3122 Hz line). Heard, from each cut's
//! start: C1 0.6 x 6 (rCAD 318 W) 1.4%, C2 1.0 x 6 (499 W) 3.0%, C3 1.4 x 6 (about 680 W) 5.6% at 0.5 s and
//! still rising, 7.3% at 3 s, C4 1.8 x 6 (about 860 W) 7.8% at 0.5 s, 11.4% at 0.8 s, running away (the tool
//! broke about 1.6 s in).
//!
//! The model: the sag rises 0.0088% a watt from about 160 W to C2's 3.0% at 499 W, 0.0122% a watt past it
//! (first order, 0.1 s); past its pull-out (650 W) the motor falls behind and keeps falling, faster the further
//! it has fallen. Load goes with the feed to the 0.75 (Kienzle: the chip thickens as the feed rises).
//!
//! The loop as on the machine: windows of 0.17 s every 0.085 s (24 kHz), handed over in the app's 0.1 s
//! batches; a feed change reaches the cut 0.245 s later (the board's queue, measured in the controller's
//! `the_adaptive_feed_takes_hold_after_the_board_queue`).

use cnc_listen::{Context, Frame, Guard, GuardSettings, Limiter, LimiterSettings};

const DT: f64 = 0.005;
/// The no-load sag the guard learns.
const AIR: f64 = 0.008;
const HOP: f64 = 2048.0 / 24_000.0;
const WINDOW: f64 = 4096.0 / 24_000.0;
const POLL: f64 = 0.1;
const QUEUE: f64 = 0.245;

struct Spindle {
    lag: f64,
    behind: f64,
}

impl Spindle {
    /// One step with `p` W of cut: the sag over the no-load sag.
    fn step(&mut self, p: f64) -> f64 {
        let steady = if p <= 499.0 { (0.000088 * (p - 160.0)).max(0.0) } else { 0.030 + 0.000122 * (p - 499.0) };
        self.lag += (steady - self.lag) * DT / 0.1;
        if p > 650.0 {
            self.behind += 0.00015 * (p - 650.0) * (1.0 + self.behind / 0.02) * DT;
        } else {
            self.behind -= self.behind * DT / 0.3;
        }
        self.lag + self.behind
    }
}

#[derive(Default, Debug)]
struct Run {
    /// (t, the sag over no-load (the guard's scale), the feed asked for, the feed at the cut, the sag as last heard)
    trace: Vec<(f64, f64, f64, f64, f64)>,
    peak: f64,
    stalled_at: Option<f64>,
    held_at: Option<f64>,
    lowest_feed: f64,
}

/// A pass of `secs` s at `p100` W (at the programmed feed), from air: with the limiter (`settings`) or without
/// (None), the bog guard behind it as now. The queue delay and the gains can be scaled for the stability runs.
fn run(p100: f64, secs: f64, settings: Option<LimiterSettings>, queue: f64, guard_stops: bool) -> Run {
    let mut sp = Spindle { lag: 0.0, behind: 0.0 };
    let mut lim = settings.map(Limiter::new);
    let mut guard = Guard::new(GuardSettings::default());
    let mut r = Run { lowest_feed: 1.0, ..Default::default() };
    // the guard learns the no-load sag first (2 s settle after the spindle starts, then air)
    let ctx = |cutting| Context { rpm: 24_000.0, teeth: 3, cutting, ..Default::default() };
    for k in 0..40 {
        guard.update(k as f64 * HOP, &Frame { ctx: ctx(false), droop: Some(AIR), ..Default::default() });
    }
    let t0 = 40.0 * HOP;
    let mut asked: Vec<(f64, f64)> = vec![(0.0, 1.0)];
    let mut sag_hist: Vec<(f64, f64)> = vec![];
    let (mut next_frame, mut next_poll, mut pending) = (WINDOW, POLL, vec![]);
    let mut t = 0.0;
    let mut heard = 0.0;
    while t < secs {
        t += DT;
        // the feed at the cut: what was asked `queue` s ago
        let feed_now = asked.iter().rev().find(|(ta, _)| *ta <= t - queue).map(|a| a.1).unwrap_or(1.0);
        let p = p100 * feed_now.powf(0.75);
        let stopped = r.stalled_at.is_some() || (guard_stops && r.held_at.is_some());
        let cut = if stopped { 0.0 } else { sp.step(p) };
        let e = cut;
        sag_hist.push((t, e));
        r.peak = r.peak.max(e);
        if cut > 0.15 && r.stalled_at.is_none() {
            r.stalled_at = Some(t);
        }
        if t >= next_frame {
            let m: Vec<f64> = sag_hist.iter().filter(|(ts, _)| *ts > t - WINDOW).map(|x| x.1).collect();
            let mean = m.iter().sum::<f64>() / m.len() as f64;
            pending.push((t, mean));
            heard = mean;
            next_frame += HOP;
        }
        if t >= next_poll {
            for (tf, mean) in pending.drain(..) {
                let f = Frame { ctx: ctx(true), droop: Some(AIR + mean), ..Default::default() };
                if r.held_at.is_none() && guard.update(t0 + tf, &f).is_some() {
                    r.held_at = Some(t);
                }
                if let Some(l) = lim.as_mut() {
                    let want = l.update(t0 + tf, &f, guard.excess(&f));
                    asked.push((t, want));
                    r.lowest_feed = r.lowest_feed.min(want);
                }
            }
            next_poll += POLL;
        }
        r.trace.push((t, e, asked.last().unwrap().1, feed_now, heard));
    }
    r
}

fn at(r: &Run, t: f64) -> (f64, f64, f64, f64) {
    let x = r.trace.iter().find(|x| x.0 >= t).unwrap();
    (x.1, x.2, x.3, x.4)
}

#[test]
fn the_model_matches_the_recording() {
    // heard (windowed, as the analysis reports it) from each cut's start, against the recording's windows
    let c2 = run(499.0, 3.0, None, QUEUE, false);
    assert!((at(&c2, 2.5).3 - 0.030).abs() < 0.002, "C2 {:?}", at(&c2, 2.5));
    let c3 = run(680.0, 3.1, None, QUEUE, false);
    let (h05, h30) = (at(&c3, 0.5).3, at(&c3, 3.0).3);
    eprintln!("C3 heard: 0.5 s {:.1}% (recorded 5.6), 3 s {:.1}% (7.3)", h05 * 100.0, h30 * 100.0);
    assert!(h05 > 0.045 && h05 < 0.065 && h30 > 0.063 && h30 < 0.085, "C3 {h05} {h30}");
    let c4 = run(860.0, 2.0, None, QUEUE, false);
    let (h05, h08) = (at(&c4, 0.5).3, at(&c4, 0.8).3);
    eprintln!("C4 heard: 0.5 s {:.1}% (recorded 7.8), 0.8 s {:.1}% (11.4); stalls at {:?} (the tool broke about 1.6 s in)", h05 * 100.0, h08 * 100.0, c4.stalled_at);
    assert!(h05 > 0.065 && h05 < 0.095 && h08 > 0.095 && h08 < 0.15, "C4 {h05} {h08}");
    assert!(c4.stalled_at.is_some_and(|t| t > 0.9 && t < 2.0), "{:?}", c4.stalled_at);
    // the guard as it is holds in both (as on the recording)
    assert!(c3.held_at.is_some() && c4.held_at.is_some(), "{:?} {:?}", c3.held_at, c4.held_at);
}

fn report(name: &str, r: &Run) {
    let settle = r.trace.last().unwrap();
    eprintln!(
        "{name}: peak sag over no-load {:.1}%, lowest feed asked {:.0}%, at the end feed {:.0}% sag {:.1}%; guard hold {}; stall {}",
        r.peak * 100.0,
        r.lowest_feed * 100.0,
        settle.3 * 100.0,
        settle.1 * 100.0,
        r.held_at.map(|t| format!("at {t:.2} s")).unwrap_or("none".into()),
        r.stalled_at.map(|t| format!("at {t:.2} s")).unwrap_or("none".into())
    );
    for t in [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0] {
        if let Some(x) = r.trace.iter().find(|x| x.0 >= t) {
            eprintln!("   {t:.2} s: sag {:.2}%  asked {:.0}%  at the cut {:.0}%", x.1 * 100.0, x.2 * 100.0, x.3 * 100.0);
        }
    }
}

#[test]
fn what_it_would_have_done_in_c3_and_c4() {
    let s = LimiterSettings::default();
    let c3 = run(680.0, 3.1, Some(s), QUEUE, true);
    report("C3 1.4 x 6 F2400 (about 680 W)", &c3);
    let c4 = run(860.0, 3.1, Some(s), QUEUE, true);
    report("C4 1.8 x 6 F2400 (about 860 W)", &c4);
    // and C4 had the guard's hold been ignored: the limiter alone
    let c4_alone = run(860.0, 3.1, Some(s), QUEUE, false);
    report("C4, the guard's hold ignored", &c4_alone);
    // C3: slowed to about 77% (the 3% target is C2's 499 W) and kept cutting, under the guard's limit
    assert!(c3.held_at.is_none() && c3.stalled_at.is_none(), "C3 held {:?} stalled {:?}", c3.held_at, c3.stalled_at);
    let (e, _, f, _) = at(&c3, 3.0);
    assert!(f > 0.68 && f < 0.85 && (e - s.target).abs() < 0.006, "C3 at 3 s: sag {e} feed {f}");
    // C4: too sudden for the feed (0.4 s before a change shows): the guard holds it, as now, before a stall;
    // the limiter alone takes it to the 60% floor and it doesn't stall either
    assert!(c4.held_at.is_some_and(|t| t < 0.9) && c4.stalled_at.is_none(), "C4 held {:?} stalled {:?}", c4.held_at, c4.stalled_at);
    assert!(c4_alone.stalled_at.is_none() && c4_alone.lowest_feed < 0.61, "C4 alone: stalled {:?}, lowest {}", c4_alone.stalled_at, c4_alone.lowest_feed);
}

#[test]
fn it_is_stable_across_loads_and_slower_reactions() {
    let s = LimiterSettings::default();
    for queue in [QUEUE, 1.5 * QUEUE, 2.0 * QUEUE] {
        for p in [450.0, 520.0, 580.0, 640.0, 700.0, 760.0] {
            let r = run(p, 6.0, Some(s), queue, true);
            assert!(r.stalled_at.is_none(), "{p} W, queue {queue}: stalled");
            // settled: over the last 2 s the feed at the cut moves less than 3 points and doesn't swing
            let tail: Vec<f64> = r.trace.iter().filter(|x| x.0 > 4.0).map(|x| x.3).collect();
            let (lo, hi) = tail.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
            assert!(hi - lo < 0.03, "{p} W, queue {queue}: feed {lo:.3}..{hi:.3}");
            // a cut it can carry at full feed keeps it
            if p <= 450.0 {
                assert!(r.lowest_feed > 0.97, "{p} W: {}", r.lowest_feed);
            }
        }
    }
}
