//! Replay a recording through the analysis and the bog guard the way the app hears it: same windows, the
//! spindle's programmed speed from `--on`, "cutting" (a feed move with the spindle on) from `--move`, and the
//! passes (only for the printout) from `--cuts a-b,c-d`.
//!
//!     cargo run --release -p cnc-listen --example replay -- run.wav --on 129 --move 140 --rpm 24000 \
//!         --cuts 144.6-147.7,150.7-153.9,156.9-160.0,163.0-165 [--limit 0.06 --time 0.3]

use cnc_listen::{Analyzer, Context, Guard, GuardSettings};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let wav = args.first().expect("a .wav");
    let (x, rate) = cnc_listen::read_wav(std::path::Path::new(wav)).expect("read the wav");
    let on: f64 = get("--on").map(|v| v.parse().unwrap()).unwrap_or(0.0);
    let mv: f64 = get("--move").map(|v| v.parse().unwrap()).unwrap_or(on);
    let rpm: f64 = get("--rpm").map(|v| v.parse().unwrap()).unwrap_or(24_000.0);
    let cuts: Vec<(f64, f64)> = get("--cuts")
        .map(|c| c.split(',').map(|p| { let (a, b) = p.split_once('-').unwrap(); (a.parse().unwrap(), b.parse().unwrap()) }).collect())
        .unwrap_or_default();
    let mut settings = GuardSettings::default();
    if let Some(l) = get("--limit") { settings.limit = l.parse().unwrap(); }
    if let Some(t) = get("--time") { settings.time = t.parse().unwrap(); }
    let mut a = Analyzer::new(rate);
    let mut g = Guard::new(settings);
    // feed it in 0.1 s chunks, as the app polls; a window's time is when its last sample came in
    let chunk = rate as usize / 10;
    let n = if rate >= 32_000 { 8192 } else { 4096 };
    let mut fed = 0usize;
    let mut fired = vec![];
    let mut last_pass = usize::MAX;
    while fed < x.len() {
        let end = (fed + chunk).min(x.len());
        let t_chunk = end as f64 / rate as f64;
        let ctx = Context {
            rpm: if t_chunk >= on { rpm } else { 0.0 },
            teeth: 3,
            cutting: t_chunk >= mv,
            line: cuts.iter().position(|(s, e)| t_chunk >= *s && t_chunk <= *e).map(|p| p + 1).unwrap_or(0),
            ..Default::default()
        };
        let frames = a.push(&x[fed..end], &ctx);
        let k = frames.len();
        for (i, f) in frames.iter().enumerate() {
            // the window's end time: the chunk's end less the hops still in it
            let t = t_chunk - (k - 1 - i) as f64 * (n / 2) as f64 / rate as f64;
            let pass = cuts.iter().position(|(s, e)| t >= *s && t <= *e);
            if let Some(p) = pass {
                if p != last_pass {
                    println!("--- pass {} ({:.1}-{:.1} s)", p + 1, cuts[p].0, cuts[p].1);
                    last_pass = p;
                }
                println!(
                    "{t:8.2}  droop {:>6}  air {:>6}  chatter {:>8}",
                    f.droop.map(|d| format!("{:.2}%", d * 100.0)).unwrap_or("-".into()),
                    g.air_droop(rpm).map(|d| format!("{:.2}%", d * 100.0)).unwrap_or("-".into()),
                    f.chatter.map(|c| format!("{:.0}", c.0)).unwrap_or("-".into())
                );
            }
            if let Some(why) = g.update(t, f) {
                let where_ = pass.map(|p| format!("in pass {}", p + 1)).unwrap_or_else(|| "OUTSIDE A PASS (false trigger?)".into());
                println!(">>> HOLD at {t:.2} s {where_}: {why}");
                fired.push((t, pass));
            }
        }
        fed = end;
    }
    println!("\nholds: {:?}", fired);
}
