//! Re-analyse a run's recording with this build's analysis and bog guard, each window given what the machine
//! was doing then from the run's own cut log timeline (line, position, speed along the path, programmed rpm):
//!
//!     cargo run --release -p cnc-listen --example reanalyse -- run.wav run.cutlog.json [--slip 1400 --cap 7] > windows.tsv
//!
//! Prints a row per window: t, line, x, y, z, mm/min, rpm, heard droop, sag over the guard's no-load sag,
//! teeth's beat dB, level dB, chatter Hz, and the guard's hold (1) if it would hold there.

use cnc_listen::{Analyzer, Context, CutLog, Guard, GuardSettings};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (x, rate) = cnc_listen::read_wav(std::path::Path::new(&args[0])).expect("read the wav");
    let log = CutLog::load(std::path::Path::new(&args[1])).expect("read the cut log");
    let tl = &log.timeline;
    let mut a = Analyzer::new(rate);
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).map(|v| v.parse::<f64>().unwrap());
    let mut gs = GuardSettings::default();
    if let Some(v) = get("--slip") {
        gs.slip_rpm = v;
    }
    if let Some(v) = get("--cap") {
        gs.cap = v / 100.0;
    }
    let mut g = Guard::new(gs);
    let chunk = rate as usize / 10;
    let hop = if rate >= 32_000 { 4096 } else { 2048 } as f64 / rate as f64;
    let mut fed = 0;
    let mut k = 0usize;
    println!("t\tline\tx\ty\tz\tv\trpm\tdroop\texcess\ttooth\tlevel\tchatter\thold");
    while fed < x.len() {
        let end = (fed + chunk).min(x.len());
        let t_end = end as f64 / rate as f64;
        while k + 1 < tl.len() && tl[k + 1].t <= t_end {
            k += 1;
        }
        let m = &tl[k.min(tl.len() - 1)];
        let ctx = Context { line: m.line, op: m.op.clone(), rpm: m.rpm, teeth: 3, pos: m.pos, velocity: m.velocity, cutting: m.cutting, ..Default::default() };
        let frames = a.push(&x[fed..end], &ctx);
        let n = frames.len();
        for (i, f) in frames.iter().enumerate() {
            let t = t_end - (n - 1 - i) as f64 * hop;
            let held = g.update(t, f).is_some();
            let ex = g.excess(f);
            let fmt = |v: Option<f64>, s: f64| v.map(|v| format!("{:.3}", v * s)).unwrap_or("nan".into());
            println!(
                "{t:.3}\t{}\t{:.2}\t{:.2}\t{:.2}\t{:.0}\t{:.0}\t{}\t{}\t{}\t{:.1}\t{}\t{}",
                f.ctx.line,
                f.ctx.pos[0],
                f.ctx.pos[1],
                f.ctx.pos[2],
                f.ctx.velocity,
                f.ctx.rpm,
                fmt(f.droop, 100.0),
                fmt(ex, 100.0),
                f.tooth_db.map(|d| format!("{d:.1}")).unwrap_or("nan".into()),
                f.loud_db,
                f.chatter.map(|c| format!("{:.0}", c.0)).unwrap_or("-".into()),
                held as u8
            );
        }
        fed = end;
    }
}
