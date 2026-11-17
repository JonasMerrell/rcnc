//! Print the heard sag per window of a recording, as the app hears it (for fitting the spindle's load model):
//!
//!     cargo run --release -p cnc-listen --example sag_trace -- run.wav --on 1 --move 12 --rpm 24000 [--from 14 --to 38]

use cnc_listen::{Analyzer, Context};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).map(|v| v.parse::<f64>().unwrap());
    let (x, rate) = cnc_listen::read_wav(std::path::Path::new(&args[0])).expect("read the wav");
    let (on, mv, rpm) = (get("--on").unwrap_or(0.0), get("--move").unwrap_or(0.0), get("--rpm").unwrap_or(24_000.0));
    let (from, to) = (get("--from").unwrap_or(0.0), get("--to").unwrap_or(f64::INFINITY));
    let mut a = Analyzer::new(rate);
    let chunk = rate as usize / 10;
    let hop = if rate >= 32_000 { 4096 } else { 2048 } as f64 / rate as f64;
    let mut fed = 0;
    while fed < x.len() {
        let end = (fed + chunk).min(x.len());
        let t_end = end as f64 / rate as f64;
        let ctx = Context { rpm: if t_end >= on { rpm } else { 0.0 }, teeth: 3, cutting: t_end >= mv, ..Default::default() };
        let frames = a.push(&x[fed..end], &ctx);
        let k = frames.len();
        for (i, f) in frames.iter().enumerate() {
            let t = t_end - (k - 1 - i) as f64 * hop;
            if t >= from && t <= to {
                println!("{t:.3} {} {}", f.droop.map(|d| format!("{:.4}", d * 100.0)).unwrap_or("-".into()), f.tooth_db.map(|d| format!("{d:.1}")).unwrap_or("-".into()));
            }
        }
        fed = end;
    }
}
