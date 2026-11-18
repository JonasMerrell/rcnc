//! Stick-out gauge op1a v2, run 1 on the DMC2 (2026-11-18): the rpm sweep's 18k, 15k and 12k passes (0.3 x 12 at
//! the same chip, F = rpm / 10), 405-512 s of its recording at 24 kHz (from 48 kHz), and what the machine was doing
//! then from its own cut log (line, rpm, speed along the path, position). The bog guard held three times in the
//! 12k pass (line 216) on the machine.
//!
//! What the spindle really did there (measured from the recording with 0.5 s windows, each pass against its own
//! 3 s in air): 18k 4.2 / 4.0% sag, 15k 4.8 / 4.8% creeping up, 12k 8.4% and still falling (pull-out). The build
//! that ran found the speed in only a few of the windows at 12-18k (the machine's fixed lines and a loud input
//! drowned the rotation's comb in its scoring), and in air read 2.9% where the spindle ran 1.2%.

use cnc_listen::{read_wav, Analyzer, Context, Guard, GuardSettings};

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

/// The pass a program line belongs to: its "(MSG,Pass N" line and the next.
fn pass(line: usize) -> Option<u32> {
    [(177, 17), (186, 18), (196, 19), (205, 20), (215, 21), (224, 22)].iter().rev().find(|(l, _)| line >= *l).map(|p| p.1)
}

#[test]
fn the_guard_hears_the_spindle_at_12_to_18k_and_holds_in_the_12k_pass() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let (x, rate) = read_wav(&dir.join("v2-run-1-18k-to-12k-24k.wav")).unwrap();
    let tl: Vec<M> = serde_json::from_str(&std::fs::read_to_string(dir.join("v2-run-1-18k-to-12k.timeline.json")).unwrap()).unwrap();
    let mut a = Analyzer::new(rate);
    let mut g = Guard::new(GuardSettings::default());
    let chunk = rate as usize / 10;
    let hop = 2048.0 / rate as f64;
    let (mut fed, mut k) = (0, 0);
    // per pass: windows with the spindle on, those it heard a speed in, the sag over no-load in the cut, holds
    let mut on = std::collections::BTreeMap::<u32, (u32, u32, Vec<f64>, Vec<f64>)>::new();
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
            let e = on.entry(p).or_default();
            if m.rpm > 0.0 {
                e.0 += 1;
                e.1 += f.droop.is_some() as u32;
            }
            // in the cut: feeding at the pass's floor (Z20), inside the bar's length
            if m.v > 0.5 * m.rpm / 10.0 && (m.z - 20.0).abs() < 0.05 && m.y.abs() < 55.0 {
                if let Some(ex) = g.excess(f) {
                    e.2.push(ex);
                }
            }
            if held {
                e.3.push(t);
            }
        }
        fed = end;
    }
    for (p, (n, heard, ex, holds)) in &on {
        let mean = ex.iter().sum::<f64>() / ex.len().max(1) as f64;
        eprintln!("pass {p}: speed heard in {heard}/{n} windows; in the cut over no-load {:.2}% ({} windows); holds at {holds:?}", mean * 100.0, ex.len());
    }
    let mean = |p: u32| on[&p].2.iter().sum::<f64>() / on[&p].2.len() as f64;
    // heard nearly everywhere at every speed
    for p in [17, 18, 19, 20, 21] {
        let (n, heard, _, _) = &on[&p];
        assert!(*heard as f64 >= 0.9 * *n as f64, "pass {p}: heard {heard}/{n}");
    }
    // the sag over no-load as measured (0.5 s windows): 18k 4.2 / 4.0, 15k 4.8 / 4.8, 12k 8.4
    for (p, want) in [(17, 0.0417), (18, 0.0395), (19, 0.0476), (20, 0.0475)] {
        assert!((mean(p) - want).abs() < 0.005, "pass {p}: {:.4} (measured {want})", mean(p));
    }
    assert!(mean(21) > 0.065, "pass 21: {}", mean(21));
    // no hold at 18k; held in the 12k pass, as on the machine
    assert!(on[&17].3.is_empty() && on[&18].3.is_empty(), "{:?} {:?}", on[&17].3, on[&18].3);
    assert!(!on[&21].3.is_empty(), "no hold in the 12k pass");
}
