//! The bog guard on the DMC2's real runs (T24, 1/4" 3-flute carbide, 24 out), each replayed through the analysis
//! and the guard with what the machine was doing then from the run's own cut log: where it must hold and where
//! it must not. Clips at 24 kHz in tests/data, with their timelines (time, line, rpm, speed along the path, Y, Z).
//! The recordings carry the holds the build on the machine made (the feed stops there), so a pass's later
//! windows can be missing.

use cnc_listen::{read_wav, Analyzer, Context, Guard, GuardSettings};
use std::collections::BTreeMap;

#[derive(serde::Deserialize)]
struct M {
    t: f64,
    line: usize,
    rpm: f64,
    v: f64,
    y: f64,
    z: f64,
    cutting: bool,
}

#[derive(Default, Debug)]
struct Pass {
    windows: u32,
    heard: u32,
    /// slip over no-load in the cut, rpm
    slip: Vec<f64>,
    holds: Vec<f64>,
}

/// Replays `clip` with its timeline; passes start at the given program lines ("(MSG,Pass N" lines), numbered
/// from `first`.
fn replay(clip: &str, starts: &[usize], first: u32) -> BTreeMap<u32, Pass> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let (x, rate) = read_wav(&dir.join(format!("{clip}-24k.wav"))).unwrap();
    let tl: Vec<M> = serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{clip}.timeline.json"))).unwrap()).unwrap();
    let pass = |line: usize| starts.iter().rposition(|l| line >= *l).map(|i| first + i as u32);
    let mut a = Analyzer::new(rate);
    let mut g = Guard::new(GuardSettings::default());
    let chunk = rate as usize / 10;
    let hop = 2048.0 / rate as f64;
    let (mut fed, mut k) = (0, 0);
    let mut out = BTreeMap::<u32, Pass>::new();
    while fed < x.len() {
        let end = (fed + chunk).min(x.len());
        let t_end = end as f64 / rate as f64;
        while k + 1 < tl.len() && tl[k + 1].t <= t_end {
            k += 1;
        }
        let m = &tl[k];
        let ctx = Context { line: m.line, rpm: m.rpm, teeth: 3, velocity: m.v, cutting: m.cutting, ..Default::default() };
        let frames = a.push(&x[fed..end], &ctx);
        let n = frames.len();
        for (i, f) in frames.iter().enumerate() {
            let t = t_end - (n - 1 - i) as f64 * hop;
            let held = g.update(t, f).is_some();
            let Some(p) = pass(m.line) else { continue };
            let e = out.entry(p).or_default();
            if m.rpm > 0.0 {
                e.windows += 1;
                e.heard += f.droop.is_some() as u32;
            }
            if m.v > 0.0 && m.z < 27.0 && m.y.abs() < 55.0 {
                if let Some(ex) = g.excess(f) {
                    e.slip.push(ex * m.rpm);
                }
            }
            if held {
                e.holds.push(t);
            }
        }
        fed = end;
    }
    for (p, e) in &out {
        let max = e.slip.iter().cloned().fold(0.0, f64::max);
        eprintln!("{clip} pass {p}: heard {}/{}, slip max {max:.0} rpm, holds {:?}", e.heard, e.windows, e.holds);
    }
    out
}

/// v4 run 1 (VFD F004 = 200), the 24k passes 23-32: steady cuts settling at 700-1070 rpm of slip, and the edge
/// (the thin-chip 0.6 x 12 at F1200, 1.0 x 6, 0.45 x 12) creeping at 1230-1630. On the machine the old guard held
/// 17 times, every one on a cut's entry ramp or its re-entry after Cycle Start. Now: only the two creeping
/// edge passes.
#[test]
fn v4_holds_only_at_the_24k_edge() {
    let r = replay("v4-run-1", &[25, 34, 44, 53, 62, 71, 80, 89, 98, 107, 116, 125], 21);
    for (p, e) in &r {
        if *p >= 23 {
            assert!(e.heard as f64 >= 0.9 * e.windows as f64, "pass {p}: heard {}/{}", e.heard, e.windows);
        }
        if !matches!(p, 30 | 31) {
            assert!(e.holds.is_empty(), "pass {p} held at {:?}", e.holds);
        }
    }
}

/// v5 run 1, the 12k ladder (F004 = 200): 247-676 W by rCAD's model settled at 330-700 rpm of slip (5.7%): no
/// hold (the old 6% limit held at 676 W on the entry ramp). 759 W (about 7%) and 839 W (about 9%) are past the
/// planning limit: the 7% cap may hold there.
#[test]
fn v5_the_12k_ladder_runs_free_to_its_planning_limit() {
    let r = replay("v5-run-1", &[25, 34, 43, 52, 61, 70, 79], 1);
    for p in 1..=5 {
        let e = &r[&p];
        assert!(e.heard as f64 >= 0.9 * e.windows as f64, "pass {p}: heard {}/{}", e.heard, e.windows);
        assert!(e.holds.is_empty(), "pass {p} held at {:?}", e.holds);
    }
}
