//! The run that broke T24 (stick-out gauge op 1a on the 32 mm bar, 2026-11-17), 128-166.5 s of its recording
//! at 24 kHz: the spindle on at 1 s, moving from 12 s; cuts C1 16.6-19.7, C2 22.7-25.9, C3 28.9-32.0 (never
//! settled: 2.5% -> 8% under the programmed speed), C4 from 35.0 (ran away, the tool broke at about 36.8).
//! The bog guard, fed as the app feeds it, must hold in C3 and (had C3 been let through) in C4 before the
//! break, and nowhere else: not in the spin-up, the rapids or C1/C2.

use cnc_listen::{read_wav, Analyzer, Context, Guard, GuardSettings};

#[test]
fn the_guard_holds_in_c3_and_c4_and_nowhere_else() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/broken-tool-run-128s-24k.wav");
    let (x, rate) = read_wav(&path).unwrap();
    let mut a = Analyzer::new(rate);
    let mut g = Guard::new(GuardSettings::default());
    let chunk = rate as usize / 10;
    let hop = if rate >= 32_000 { 4096 } else { 2048 } as f64 / rate as f64;
    let mut holds = vec![];
    let mut fed = 0;
    while fed < x.len() {
        let end = (fed + chunk).min(x.len());
        let t_end = end as f64 / rate as f64;
        let ctx = Context { rpm: if t_end >= 1.0 { 24_000.0 } else { 0.0 }, teeth: 3, cutting: t_end >= 12.0, ..Default::default() };
        let frames = a.push(&x[fed..end], &ctx);
        let k = frames.len();
        for (i, f) in frames.iter().enumerate() {
            let t = t_end - (k - 1 - i) as f64 * hop;
            if g.update(t, f).is_some() {
                holds.push(t);
            }
        }
        fed = end;
    }
    assert_eq!(holds.len(), 2, "{holds:?}");
    // in C3, within about a second of its sag passing 6% (at about 29.7 s)
    assert!(holds[0] > 28.9 && holds[0] < 30.8, "{holds:?}");
    // in C4, before the break at about 36.8 s
    assert!(holds[1] > 35.0 && holds[1] < 36.3, "{holds:?}");
}

/// The bog guard and the adaptive feed read the same sag over the same no-load sag (`Guard::excess`). On the
/// real recording, per pass: what both see, and what the limiter would have asked for in its first second
/// (open loop: the recording can't answer a slower feed).
#[test]
fn the_guard_and_the_limiter_share_one_scale_on_the_recording() {
    use cnc_listen::{Limiter, LimiterSettings};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/broken-tool-run-128s-24k.wav");
    let (x, rate) = read_wav(&path).unwrap();
    let mut a = Analyzer::new(rate);
    let mut g = Guard::new(GuardSettings::default());
    let passes = [("C1 0.6x6", 16.6, 19.7), ("C2 1.0x6", 22.7, 25.9), ("C3 1.4x6", 28.9, 32.0), ("C4 1.8x6", 35.0, 36.3)];
    let mut lims: Vec<Limiter> = passes.iter().map(|_| Limiter::new(LimiterSettings::default())).collect();
    let mut seen: Vec<Vec<(f64, f64, f64)>> = vec![vec![]; passes.len()];
    let chunk = rate as usize / 10;
    let hop = if rate >= 32_000 { 4096 } else { 2048 } as f64 / rate as f64;
    let mut fed = 0;
    let mut air = None;
    while fed < x.len() {
        let end = (fed + chunk).min(x.len());
        let t_end = end as f64 / rate as f64;
        let ctx = Context { rpm: if t_end >= 1.0 { 24_000.0 } else { 0.0 }, teeth: 3, cutting: t_end >= 12.0, ..Default::default() };
        let frames = a.push(&x[fed..end], &ctx);
        let k = frames.len();
        for (i, f) in frames.iter().enumerate() {
            let t = t_end - (k - 1 - i) as f64 * hop;
            g.update(t, f);
            air = g.air_droop(24_000.0);
            if let Some(p) = passes.iter().position(|(_, s, e)| t >= *s && t <= *e) {
                let ex = g.excess(f);
                let want = lims[p].update(t, f, ex);
                if let Some(ex) = ex {
                    seen[p].push((t - passes[p].1, ex, want));
                }
            }
        }
        fed = end;
    }
    let air = air.unwrap();
    eprintln!("no-load sag the guard learned (spindle on, machine standing): {:.2}%", air * 100.0);
    let mut means = vec![];
    for (p, (name, _, _)) in passes.iter().enumerate() {
        let s = &seen[p];
        // the steady part: past the first 0.6 s of the pass
        let steady: Vec<f64> = s.iter().filter(|x| x.0 > 0.6).map(|x| x.1).collect();
        let mean = steady.iter().sum::<f64>() / steady.len().max(1) as f64;
        let max = s.iter().map(|x| x.1).fold(0.0, f64::max);
        let asked = |t: f64| s.iter().filter(|x| x.0 <= t).last().map(|x| x.2).unwrap_or(1.0);
        eprintln!(
            "{name}: over no-load (guard = limiter) mean {:.1}% max {:.1}%; limiter asks {:.0}% by 0.5 s, {:.0}% by 1 s",
            mean * 100.0,
            max * 100.0,
            asked(0.5) * 100.0,
            asked(1.0) * 100.0
        );
        means.push(mean);
    }
    // the scale the coordinator's and the mill PC's analyses read: C1 about 1.5, C2 about 3.2, C3 6 and more
    assert!(means[0] > 0.01 && means[0] < 0.022, "{means:?}");
    assert!(means[1] > 0.026 && means[1] < 0.038, "{means:?}");
    assert!(means[2] > 0.055, "{means:?}");
    // the limiter leaves C1 alone and starts slowing C3 and C4 at once
    assert_eq!(lims[0].feed, 1.0);
    assert!(lims[2].feed < 0.9 && lims[3].feed < 0.9, "{} {}", lims[2].feed, lims[3].feed);
}
