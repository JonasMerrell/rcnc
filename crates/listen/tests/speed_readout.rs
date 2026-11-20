//! The live speed readout on the DMC2's real runs (transducer on the spindle head, clips at 24 kHz in tests/data
//! with their timelines): with the spindle on it shows a speed nearly all the time, holds steady where the spindle
//! does (a window that snaps a harmonic 300-400 rpm off doesn't show), and agrees in air with the long window's
//! finer no-load speed (4x the resolution).

use cnc_listen::{read_wav, Analyzer, Context, SpeedMeter};

#[derive(serde::Deserialize)]
struct M {
    t: f64,
    line: usize,
    rpm: f64,
    v: f64,
    cutting: bool,
}

struct Stats {
    /// Windows with the spindle on and settled (2 s after a new S), and how many had a readout.
    on: u32,
    shown: u32,
    /// Readout steps from one window to the next in air with the spindle steady (rpm), and the readout minus the
    /// long window's no-load speed there.
    air_steps: Vec<f64>,
    vs_long: Vec<f64>,
    /// Steps between single heard windows in steady air, rpm.
    raw_steps: Vec<f64>,
    /// Single windows in steady air more than 100 rpm off the long window's speed, of how many.
    raw_off: (u32, u32),
}

fn replay(clip: &str) -> Stats {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let (x, rate) = read_wav(&dir.join(format!("{clip}-24k.wav"))).unwrap();
    let tl: Vec<M> = serde_json::from_str(&std::fs::read_to_string(dir.join(format!("{clip}.timeline.json"))).unwrap()).unwrap();
    let mut a = Analyzer::new(rate);
    let mut m = SpeedMeter::new();
    let chunk = rate as usize / 10;
    let hop = 2048.0 / rate as f64;
    let (mut fed, mut k) = (0, 0);
    let (mut prog, mut since) = (0.0, 0.0);
    let mut st = Stats { on: 0, shown: 0, air_steps: vec![], vs_long: vec![], raw_steps: vec![], raw_off: (0, 0) };
    let (mut last_shown, mut last_raw, mut last_long): (Option<f64>, Option<f64>, Option<f64>) = (None, None, None);
    while fed < x.len() {
        let end = (fed + chunk).min(x.len());
        let t_end = end as f64 / rate as f64;
        while k + 1 < tl.len() && tl[k + 1].t <= t_end {
            k += 1;
        }
        let tm = &tl[k];
        let ctx = Context { line: tm.line, rpm: tm.rpm, teeth: 3, velocity: tm.v, cutting: tm.cutting, ..Default::default() };
        let frames = a.push(&x[fed..end], &ctx);
        let n = frames.len();
        for (i, f) in frames.iter().enumerate() {
            let t = t_end - (n - 1 - i) as f64 * hop;
            if tm.rpm != prog {
                prog = tm.rpm;
                since = t;
                last_shown = None;
                last_raw = None;
                last_long = None;
            }
            let long = f.air_droop.map(|d| tm.rpm * (1.0 - d));
            // the spindle steady: the long window's no-load speed (0.68 s) moved under 30 rpm since the last
            let steady = matches!((long, last_long), (Some(a), Some(b)) if (a - b).abs() < 30.0);
            m.update(t, f);
            let sp = m.get(t);
            if tm.rpm > 0.0 && t - since > 2.0 {
                st.on += 1;
                st.shown += sp.is_some() as u32;
                if !tm.cutting && steady {
                    if let (Some(s), Some(l)) = (sp, last_shown) {
                        st.air_steps.push((s.rpm - l).abs());
                    }
                    if let (Some(r), Some(l)) = (f.rpm, last_raw) {
                        st.raw_steps.push((r - l).abs());
                    }
                    if let (Some(r), Some(l)) = (f.rpm, long) {
                        st.raw_off.0 += ((r - l).abs() > 100.0) as u32;
                        st.raw_off.1 += 1;
                    }
                    if let (Some(s), Some(l)) = (sp, long) {
                        st.vs_long.push(s.rpm - l);
                    }
                }
            }
            last_shown = sp.map(|s| s.rpm);
            last_raw = f.rpm;
            last_long = long;
        }
        fed = end;
    }
    st
}

fn pct(v: &[f64], q: f64) -> f64 {
    let mut v = v.to_vec();
    v.sort_by(|a, b| a.total_cmp(b));
    v[((v.len() - 1) as f64 * q) as usize]
}

#[test]
fn the_readout_is_there_and_steady_on_the_real_runs() {
    // v2 (12-18k with the VFD's F004 at 400): the short windows lock onto a second comb 1.5-1.9% under the long
    // window's (in a 3.5 s spectrum the 1st-3rd harmonics stand at 194.9 Hz, the 8th, 14th, 16th and 18th at
    // multiples of 191.2), so its readout is checked for being there and steady, not against the long window. The
    // VFD's output frequency will tell which is the rotor (the other is the field)
    for (clip, vs_long) in [("v4-run-1", 20.0), ("v5-run-1", 40.0), ("v2-run-1-18k-to-12k", f64::INFINITY)] {
        let s = replay(clip);
        let shown = s.shown as f64 / s.on as f64;
        let step95 = pct(&s.air_steps, 0.95);
        let step_max = pct(&s.air_steps, 1.0);
        let raw_max = pct(&s.raw_steps, 1.0);
        let mean_vs_long = s.vs_long.iter().sum::<f64>() / s.vs_long.len() as f64;
        eprintln!("{clip}: shown {:.1}% of {} windows; air steps 95% {step95:.0} max {step_max:.0} rpm (raw windows max {raw_max:.0}); readout - long window {mean_vs_long:+.0} rpm over {}; single windows >100 rpm off {} of {}", shown * 100.0, s.on, s.vs_long.len(), s.raw_off.0, s.raw_off.1);
        assert!(shown > 0.98, "{clip}: shown in {:.1}%", shown * 100.0);
        assert!(step95 < 30.0, "{clip}: 95% of air steps under {step95}");
        assert!(mean_vs_long.abs() < vs_long, "{clip}: {mean_vs_long}");
    }
}
