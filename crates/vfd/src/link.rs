//! The link to the drive: a thread that finds the adapter and the drive's line settings, then polls the live
//! readings a few times a second. Nothing here blocks the app; with no adapter it says so and looks again
//! every few seconds (so plugging it in later just works).

use crate::modbus::{self, Read};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Parity {
    None,
    Even,
    Odd,
}

/// A serial line setting: baud, parity, stop bits (8 data bits always).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Line {
    pub baud: u32,
    pub parity: Parity,
    pub stop_bits: u8,
}

impl std::fmt::Display for Line {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let p = match self.parity {
            Parity::None => 'N',
            Parity::Even => 'E',
            Parity::Odd => 'O',
        };
        write!(f, "{} 8{}{}", self.baud, p, self.stop_bits)
    }
}

impl Line {
    /// What to try when the line isn't set: the usual H100 settings first.
    pub fn scan() -> Vec<Line> {
        let mut v = vec![];
        for baud in [9600, 19200, 38400, 4800] {
            for (parity, stop_bits) in [(Parity::None, 1), (Parity::Even, 1), (Parity::Odd, 1), (Parity::None, 2)] {
                v.push(Line { baud, parity, stop_bits });
            }
        }
        v
    }
}

/// Saved in the data folder (vfd.json).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct VfdSettings {
    /// Look for the drive at all.
    pub on: bool,
    /// The adapter's port (COM5, /dev/ttyUSB0); None: any USB serial port that answers.
    pub port: Option<String>,
    /// The drive's address (F163).
    pub address: u8,
    /// The line (F164/F165); None: try the usual ones.
    pub line: Option<Line>,
    /// Polls a second.
    pub rate_hz: f64,
    pub map: crate::RegisterMap,
    pub hold: crate::AmpsHoldSettings,
}

impl Default for VfdSettings {
    fn default() -> Self {
        VfdSettings { on: true, port: None, address: 1, line: None, rate_hz: 10.0, map: Default::default(), hold: Default::default() }
    }
}

/// A serial port, as far as Modbus needs one.
pub trait Port: Send {
    /// Send a request and collect the answer: until `expect` bytes, an exception answer, or the time is up.
    fn exchange(&mut self, request: &[u8], expect: usize) -> Vec<u8>;
}

/// The ports there are and how to open one (the real serial ports, or a simulated drive in tests).
pub trait Bus: Send {
    /// (name, description) of each candidate port, USB-RS485 adapters first.
    fn ports(&self) -> Vec<(String, String)>;
    fn open(&self, name: &str, line: Line) -> Result<Box<dyn Port>, String>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
    Off,
    /// No serial port at all (the adapter isn't plugged in).
    NoAdapter,
    /// Trying ports and line settings.
    Searching(String),
    /// Ports there, but no drive answered on any (wiring, address, the drive off).
    NoAnswer(String),
    Connected { port: String, line: Line },
    /// Was connected; answers stopped. Looking again.
    Lost(String),
}

/// What the link thread shares with the app.
#[derive(Debug, Default)]
pub struct Shared {
    pub status: Option<Status>,
    /// The drive's F parameters read on connecting: (number, raw value).
    pub params: Vec<(u16, Option<u16>)>,
    /// The latest raw input registers from 0, and when (s since the link started).
    pub latest: Option<(f64, Vec<u16>)>,
    /// Polls not yet taken by the app (time, raw registers).
    pub fresh: Vec<(f64, Vec<u16>)>,
    pub polls: u64,
    pub errors: u64,
}

/// The F parameters read once on connecting, to show on the Settings page (run/frequency source, max
/// frequency, motor nameplate, comms).
pub const PARAMS: [u16; 12] = [1, 2, 4, 5, 141, 142, 143, 144, 163, 164, 165, 169];

pub struct VfdLink {
    pub shared: Arc<Mutex<Shared>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    pub start: Instant,
}

impl Drop for VfdLink {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl VfdLink {
    /// Start looking for the drive with the real serial ports (needs the `serial` feature).
    #[cfg(feature = "serial")]
    pub fn start(settings: VfdSettings) -> VfdLink {
        Self::start_on(Box::new(crate::serial::SerialBus), settings)
    }

    pub fn start_on(bus: Box<dyn Bus>, settings: VfdSettings) -> VfdLink {
        let shared = Arc::new(Mutex::new(Shared::default()));
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let start = Instant::now();
        let thread = {
            let (shared, stop) = (shared.clone(), stop.clone());
            std::thread::Builder::new().name("vfd".into()).spawn(move || run(bus, settings, shared, stop, start)).ok()
        };
        VfdLink { shared, stop, thread, start }
    }
}

fn set_status(shared: &Mutex<Shared>, s: Status) {
    if let Ok(mut g) = shared.lock() {
        g.status = Some(s);
    }
}

/// Sleep in small steps so stopping is quick; false when asked to stop.
fn nap(stop: &std::sync::atomic::AtomicBool, d: Duration) -> bool {
    let end = Instant::now() + d;
    while Instant::now() < end {
        if stop.load(std::sync::atomic::Ordering::Relaxed) {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20).min(end - Instant::now()));
    }
    !stop.load(std::sync::atomic::Ordering::Relaxed)
}

fn read(port: &mut dyn Port, addr: u8, what: Read, start: u16, count: u16) -> Result<Vec<u16>, String> {
    let req = modbus::request(addr, what, start, count);
    let ans = port.exchange(&req, modbus::reply_len(count));
    modbus::parse(addr, what, count, &ans)
}

fn run(bus: Box<dyn Bus>, set: VfdSettings, shared: Arc<Mutex<Shared>>, stop: Arc<std::sync::atomic::AtomicBool>, start: Instant) {
    if !set.on {
        set_status(&shared, Status::Off);
        return;
    }
    let lines = set.line.map(|l| vec![l]).unwrap_or_else(Line::scan);
    let period = Duration::from_secs_f64(1.0 / set.rate_hz.clamp(1.0, 30.0));
    loop {
        // find the drive
        let ports: Vec<(String, String)> = match &set.port {
            Some(p) => vec![(p.clone(), String::new())],
            None => bus.ports(),
        };
        if ports.is_empty() {
            set_status(&shared, Status::NoAdapter);
            if !nap(&stop, Duration::from_secs(3)) {
                return;
            }
            continue;
        }
        let mut found = None;
        let mut why = String::new();
        'search: for (name, _) in &ports {
            for line in &lines {
                if stop.load(std::sync::atomic::Ordering::Relaxed) {
                    return;
                }
                set_status(&shared, Status::Searching(format!("{name} at {line}")));
                let mut port = match bus.open(name, *line) {
                    Ok(p) => p,
                    Err(e) => {
                        why = format!("{name}: {e}");
                        break; // the port itself won't open: next port
                    }
                };
                // F143 (motor poles) answers 1..=16 from a real H100
                match read(port.as_mut(), set.address, Read::Holding, 143, 1) {
                    Ok(v) if (1..=16).contains(&v[0]) => {
                        found = Some((name.clone(), *line, port));
                        break 'search;
                    }
                    Ok(v) => why = format!("{name} at {line}: F143 read {} (not a motor's poles)", v[0]),
                    Err(e) => why = format!("{name} at {line}: {e}"),
                }
            }
        }
        let Some((name, line, mut port)) = found else {
            set_status(&shared, Status::NoAnswer(format!("no drive answered at address {} on {} port(s); last: {why}", set.address, ports.len())));
            if !nap(&stop, Duration::from_secs(5)) {
                return;
            }
            continue;
        };
        // what the drive is set to
        let params: Vec<(u16, Option<u16>)> =
            PARAMS.iter().map(|&n| (n, read(port.as_mut(), set.address, Read::Holding, n, 1).ok().map(|v| v[0]))).collect();
        // as many live registers as the map wants, or as many as the drive will give
        let mut count = set.map.count();
        while count > 2 && read(port.as_mut(), set.address, Read::Input, 0, count).is_err() {
            count = (count / 2).max(2);
        }
        if let Ok(mut g) = shared.lock() {
            g.params = params;
            g.status = Some(Status::Connected { port: name.clone(), line });
        }
        // poll
        let mut fails = 0;
        let mut next = Instant::now();
        loop {
            if stop.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            match read(port.as_mut(), set.address, Read::Input, 0, count) {
                Ok(regs) => {
                    fails = 0;
                    let t = start.elapsed().as_secs_f64();
                    if let Ok(mut g) = shared.lock() {
                        g.polls += 1;
                        g.latest = Some((t, regs.clone()));
                        g.fresh.push((t, regs));
                        // nobody taking them (the app minimised for long): keep the last minute
                        let n = g.fresh.len();
                        if n > 1200 {
                            g.fresh.drain(..n - 1200);
                        }
                        if !matches!(g.status, Some(Status::Connected { .. })) {
                            g.status = Some(Status::Connected { port: name.clone(), line });
                        }
                    }
                }
                Err(e) => {
                    fails += 1;
                    if let Ok(mut g) = shared.lock() {
                        g.errors += 1;
                    }
                    if fails >= 10 {
                        set_status(&shared, Status::Lost(format!("{name}: {e}")));
                        break;
                    }
                }
            }
            next += period;
            let now = Instant::now();
            if next > now {
                if !nap(&stop, next - now) {
                    return;
                }
            } else {
                next = now;
            }
        }
        drop(port);
        if !nap(&stop, Duration::from_secs(2)) {
            return;
        }
    }
}

#[cfg(test)]
pub(crate) mod sim {
    //! A simulated H100 on a simulated bus, answering the reads like the real one.
    use super::*;
    use crate::modbus::crc16;

    #[derive(Clone)]
    pub struct SimDrive {
        pub address: u8,
        pub line: Line,
        pub params: std::collections::HashMap<u16, u16>,
        pub inputs: Arc<Mutex<Vec<u16>>>,
        /// Answer nothing (unplugged cable).
        pub dead: Arc<std::sync::atomic::AtomicBool>,
        /// Every request seen (to check nothing but reads is ever sent).
        pub seen: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl SimDrive {
        pub fn new(line: Line) -> SimDrive {
            SimDrive {
                address: 1,
                line,
                params: [(1, 1), (2, 1), (4, 1500), (5, 4000), (141, 110), (142, 110), (143, 2), (144, 24000), (163, 1), (164, 1), (165, 3), (169, 0)].into_iter().collect(),
                inputs: Arc::new(Mutex::new(vec![1500, 1500, 27, 9000, 155, 63, 312])),
                dead: Default::default(),
                seen: Default::default(),
            }
        }

        fn answer(&self, req: &[u8]) -> Vec<u8> {
            self.seen.lock().unwrap().push(req.to_vec());
            if self.dead.load(std::sync::atomic::Ordering::Relaxed) || req.len() != 8 || req[0] != self.address || crc16(req) != 0 {
                return vec![];
            }
            let start = u16::from_be_bytes([req[2], req[3]]) as usize;
            let count = u16::from_be_bytes([req[4], req[5]]) as usize;
            let vals: Option<Vec<u16>> = match req[1] {
                0x03 => (start..start + count).map(|n| self.params.get(&(n as u16)).copied()).collect(),
                0x04 => {
                    let i = self.inputs.lock().unwrap();
                    (start + count <= i.len()).then(|| i[start..start + count].to_vec())
                }
                _ => None,
            };
            let mut out = match vals {
                Some(v) => {
                    let mut o = vec![self.address, req[1], (2 * v.len()) as u8];
                    for x in v {
                        o.extend_from_slice(&x.to_be_bytes());
                    }
                    o
                }
                None => vec![self.address, req[1] | 0x80, 2],
            };
            let c = crc16(&out);
            out.extend_from_slice(&c.to_le_bytes());
            out
        }
    }

    pub struct SimBus {
        pub ports: Vec<String>,
        pub drive: Option<SimDrive>,
    }

    struct SimPort {
        drive: Option<SimDrive>,
        line: Line,
    }

    impl Port for SimPort {
        fn exchange(&mut self, request: &[u8], _expect: usize) -> Vec<u8> {
            match &self.drive {
                Some(d) if d.line == self.line => d.answer(request),
                Some(d) => {
                    d.seen.lock().unwrap().push(request.to_vec());
                    vec![]
                }
                None => vec![],
            }
        }
    }

    impl Bus for SimBus {
        fn ports(&self) -> Vec<(String, String)> {
            self.ports.iter().map(|p| (p.clone(), "sim".into())).collect()
        }
        fn open(&self, name: &str, line: Line) -> Result<Box<dyn Port>, String> {
            if !self.ports.iter().any(|p| p == name) {
                return Err("no such port".into());
            }
            // the drive is on the last port
            let drive = if Some(name) == self.ports.last().map(|s| s.as_str()) { self.drive.clone() } else { None };
            Ok(Box::new(SimPort { drive, line }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sim::*;
    use super::*;

    fn wait_for(link: &VfdLink, what: impl Fn(&Shared) -> bool) -> bool {
        let end = Instant::now() + Duration::from_secs(20);
        while Instant::now() < end {
            if what(&link.shared.lock().unwrap()) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn finds_the_drive_on_an_unusual_line_and_only_ever_reads() {
        let line = Line { baud: 19200, parity: Parity::Even, stop_bits: 1 };
        let drive = SimDrive::new(line);
        let seen = drive.seen.clone();
        let bus = SimBus { ports: vec!["COM3".into(), "COM7".into()], drive: Some(drive.clone()) };
        let link = VfdLink::start_on(Box::new(bus), VfdSettings { rate_hz: 30.0, ..Default::default() });
        assert!(wait_for(&link, |s| s.polls >= 3), "{:?}", link.shared.lock().unwrap().status);
        {
            let s = link.shared.lock().unwrap();
            assert_eq!(s.status, Some(Status::Connected { port: "COM7".into(), line }));
            assert!(s.params.contains(&(143, Some(2))) && s.params.contains(&(5, Some(4000))));
            let (_, regs) = s.latest.clone().unwrap();
            assert_eq!(regs.len(), 7);
            assert_eq!(crate::RegisterMap::default().decode(&regs).amps, Some(2.7));
        }
        drop(link);
        // read requests only, never a write or a run/stop command
        for r in seen.lock().unwrap().iter() {
            assert!(r[1] == 0x03 || r[1] == 0x04, "sent {:02X?}", r);
        }
    }

    #[test]
    fn says_so_without_an_adapter_or_a_drive_and_recovers_from_a_lost_cable() {
        let none = VfdLink::start_on(Box::new(SimBus { ports: vec![], drive: None }), VfdSettings::default());
        assert!(wait_for(&none, |s| s.status == Some(Status::NoAdapter)));
        let silent = VfdLink::start_on(
            Box::new(SimBus { ports: vec!["COM3".into()], drive: None }),
            VfdSettings { line: Some(Line { baud: 9600, parity: Parity::None, stop_bits: 1 }), ..Default::default() },
        );
        assert!(wait_for(&silent, |s| matches!(s.status, Some(Status::NoAnswer(_)))));

        let line = Line { baud: 9600, parity: Parity::None, stop_bits: 1 };
        let drive = SimDrive::new(line);
        let dead = drive.dead.clone();
        let link = VfdLink::start_on(Box::new(SimBus { ports: vec!["COM4".into()], drive: Some(drive) }), VfdSettings { rate_hz: 30.0, ..Default::default() });
        assert!(wait_for(&link, |s| s.polls >= 2));
        dead.store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(wait_for(&link, |s| matches!(s.status, Some(Status::Lost(_)) | Some(Status::NoAnswer(_)))));
        dead.store(false, std::sync::atomic::Ordering::Relaxed);
        let before = link.shared.lock().unwrap().polls;
        assert!(wait_for(&link, |s| s.polls > before + 2 && matches!(s.status, Some(Status::Connected { .. }))));
    }

    #[test]
    fn a_drive_with_fewer_registers_still_gives_the_frequency() {
        let line = Line { baud: 9600, parity: Parity::None, stop_bits: 1 };
        let drive = SimDrive::new(line);
        *drive.inputs.lock().unwrap() = vec![1500, 1500];
        let link = VfdLink::start_on(Box::new(SimBus { ports: vec!["COM4".into()], drive: Some(drive) }), VfdSettings { rate_hz: 30.0, ..Default::default() });
        assert!(wait_for(&link, |s| s.polls >= 2));
        assert_eq!(link.shared.lock().unwrap().latest.clone().unwrap().1, vec![1500, 1500]);
    }
}
