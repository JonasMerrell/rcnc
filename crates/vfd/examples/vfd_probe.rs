//! Find the VFD and print what it says, without rcnc: `cargo run -p cnc-vfd --features serial --example
//! vfd_probe [PORT] [ADDRESS]`. Prints the line it answered on, its F parameters, then the raw live registers
//! twice a second for 20 s (run the spindle and compare them with the drive's display). Read-only.

fn main() {
    let mut args = std::env::args().skip(1);
    let port = args.next().filter(|p| p != "auto");
    let address = args.next().and_then(|a| a.parse().ok()).unwrap_or(1);
    let link = vfd::VfdLink::start(vfd::VfdSettings { port, address, rate_hz: 2.0, ..Default::default() });
    let mut last = None;
    let end = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut connected_at = None;
    while std::time::Instant::now() < end {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let mut s = link.shared.lock().unwrap();
        if s.status != last {
            println!("{:?}", s.status);
            last = s.status.clone();
            if matches!(s.status, Some(vfd::Status::Connected { .. })) {
                connected_at.get_or_insert(std::time::Instant::now());
                for (n, v) in &s.params {
                    println!("  F{n:03} = {}", v.map(|v| v.to_string()).unwrap_or("?".into()));
                }
            }
        }
        for (t, regs) in s.fresh.drain(..) {
            let r = vfd::RegisterMap::default().decode(&regs);
            let f = |v: Option<f64>| v.map(|v| format!("{v:.1}")).unwrap_or("-".into());
            println!("{t:7.2} s  raw {regs:?}  ->  out {} Hz, set {} Hz, current {} A (unverified)", f(r.out_hz), f(r.set_hz), f(r.amps));
        }
        if connected_at.is_some_and(|c| c.elapsed().as_secs() >= 20) {
            break;
        }
    }
}
