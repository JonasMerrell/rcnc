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
