//! The `Backend` for the BlueSkyT USB card.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use machine::backend::{AxisMove, Backend, BackendError, BoardInfo, Outputs, Result, Status};
use machine::planner::Segment;
use machine::MachineConfig;

use crate::protocol::{self as proto, cmd, BoardIdentity, BoardSetup, HostState, Payload};

/// Raw 64-byte report exchange with the board.
pub trait Transport: Send {
    fn write(&mut self, wire: &Payload) -> Result<()>;
    /// Read one report, or `None` on timeout.
    fn read(&mut self, timeout: Duration) -> Result<Option<Payload>>;
}

/// USB HID transport via hidapi.
pub struct HidTransport {
    device: hidapi::HidDevice,
}

impl HidTransport {
    pub fn open() -> Result<Self> {
        let api = hidapi::HidApi::new().map_err(|e| BackendError::Io(e.to_string()))?;
        let device = api.open(proto::VENDOR_ID, proto::PRODUCT_ID).map_err(|_| BackendError::NotFound)?;
        Ok(HidTransport { device })
    }
}

impl Transport for HidTransport {
    fn write(&mut self, wire: &Payload) -> Result<()> {
        let mut buf = [0u8; 65];
        buf[1..].copy_from_slice(wire);
        self.device.write(&buf).map_err(|e| BackendError::Io(e.to_string()))?;
        Ok(())
    }

    fn read(&mut self, timeout: Duration) -> Result<Option<Payload>> {
        let mut buf = [0u8; 65];
        let n = self
            .device
            .read_timeout(&mut buf, timeout.as_millis() as i32)
            .map_err(|e| BackendError::Io(e.to_string()))?;
        match n {
            0 => Ok(None),
            64 => Ok(Some(buf[..64].try_into().unwrap())),
            65 => Ok(Some(buf[1..].try_into().unwrap())),
            n => Err(BackendError::Io(format!("short report ({n} bytes)"))),
        }
    }
}

const REPLY_TIMEOUT: Duration = Duration::from_millis(100);

pub struct BlueSkyBoard {
    transport: Option<Box<dyn Transport>>,
    open: Box<dyn FnMut() -> Result<Box<dyn Transport>> + Send>,
    setup: BoardSetup,
    config: Option<MachineConfig>,
    identity: Option<BoardIdentity>,
    encoder: proto::SegmentEncoder,
    last_status: Payload,
    queue_free: usize,
    /// Lines of segments sent but not yet played, oldest first: (segment count, line).
    in_flight: VecDeque<(usize, usize)>,
    line: usize,
    /// Last real status exchange, reused by polls inside `POLL_INTERVAL`.
    cached: Option<(Instant, Outputs, Status)>,
}

/// Shortest time between status exchanges. The controller polls every few
/// milliseconds; asking the board that often floods the USB bus (Mach3 polls
/// at about 9 Hz). 25 ms still reacts to E-stop and probe inputs quickly.
const POLL_INTERVAL: Duration = Duration::from_millis(25);

impl BlueSkyBoard {
    pub fn new(setup: BoardSetup) -> Self {
        Self::with_transport(setup, || Ok(Box::new(HidTransport::open()?) as Box<dyn Transport>))
    }

    pub fn with_transport(
        setup: BoardSetup,
        open: impl FnMut() -> Result<Box<dyn Transport>> + Send + 'static,
    ) -> Self {
        BlueSkyBoard {
            transport: None,
            open: Box::new(open),
            setup,
            config: None,
            identity: None,
            encoder: proto::SegmentEncoder::default(),
            last_status: [0; 64],
            queue_free: proto::QUEUE_CAPACITY,
            in_flight: VecDeque::new(),
            line: 0,
            cached: None,
        }
    }

    pub fn identity(&self) -> Option<&BoardIdentity> {
        self.identity.as_ref()
    }

    fn transport(&mut self) -> Result<&mut Box<dyn Transport>> {
        self.transport.as_mut().ok_or(BackendError::NotConnected)
    }

    fn send(&mut self, payload: &Payload) -> Result<()> {
        let wire = proto::scramble_out(payload);
        self.transport()?.write(&wire)
    }

    /// Read until a report answering `command` arrives.
    fn reply(&mut self, command: u8) -> Result<Payload> {
        let deadline = Instant::now() + REPLY_TIMEOUT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.transport()?.read(left)? {
                Some(wire) => {
                    let q = proto::unscramble_in(&wire);
                    if q[63] == command {
                        return Ok(q);
                    }
                }
                None => return Err(BackendError::Io(format!("no reply to command {command:#04x}"))),
            }
        }
    }

    fn exchange_status(&mut self, outputs: &Outputs, reset: bool) -> Result<proto::BoardStatus> {
        let config = self.config.as_ref().ok_or(BackendError::NotConnected)?;
        let state = HostState {
            reset,
            outputs: *outputs,
            max_rpm: config.spindle_max_rpm,
            z_steps_per_unit: config.axes[2].steps_per_unit,
        };
        let request = proto::status_request(&self.setup, &state);
        self.send(&request)?;
        let q = self.reply(cmd::STATUS)?;
        let status = proto::parse_status(&q).ok_or_else(|| BackendError::Io("bad status report".into()))?;
        self.last_status = q;
        Ok(status)
    }

    fn track_played(&mut self, board_free: usize) {
        let queued = proto::QUEUE_CAPACITY.saturating_sub(board_free);
        let mut in_flight: usize = self.in_flight.iter().map(|(n, _)| n).sum();
        while in_flight > queued {
            let Some(front) = self.in_flight.front_mut() else { break };
            let played = (in_flight - queued).min(front.0);
            front.0 -= played;
            in_flight -= played;
            self.line = front.1;
            if front.0 == 0 {
                self.in_flight.pop_front();
            }
        }
        if let Some((_, line)) = self.in_flight.front() {
            self.line = *line;
        }
    }

    /// After sending motion, the cached status no longer says idle.
    fn mark_busy(&mut self) {
        if let Some((_, _, status)) = &mut self.cached {
            status.idle = false;
        }
    }

    fn send_frames(&mut self, frames: Vec<Payload>) -> Result<()> {
        self.mark_busy();
        for f in frames {
            self.send(&f)?;
            self.queue_free = self.queue_free.saturating_sub(f[62] as usize);
        }
        Ok(())
    }
}

impl Backend for BlueSkyBoard {
    fn name(&self) -> &str {
        "BlueSkyT USB (DMC2 stock board)"
    }

    fn connect(&mut self, config: &MachineConfig) -> Result<BoardInfo> {
        if self.transport.is_none() {
            self.transport = Some((self.open)()?);
        }
        self.config = Some(config.clone());
        self.cached = None;
        // Same order as the Mach3 plugin: abort, handshake, positions, configuration, status.
        self.send(&proto::simple(cmd::ABORT))?;
        let hello = proto::handshake(config, &self.setup);
        let wire = proto::handshake_wire(&hello);
        self.transport()?.write(&wire)?;
        let reply = self.reply(cmd::HANDSHAKE)?;
        let identity = proto::parse_handshake_reply(&reply).ok_or(BackendError::NotFound)?;
        if !identity_uses_scrambling(identity.firmware) {
            return Err(BackendError::Other(format!(
                "firmware {} is not supported yet (only scrambled firmware like V3.25 has been tested)",
                identity.firmware_text()
            )));
        }
        // Keep the board's own position counters across reconnects.
        let status = self.exchange_status(&Outputs::default(), true)?;
        self.send(&proto::set_position(&self.last_status, status.position))?;
        self.send(&proto::config_axes(config, &self.setup))?;
        self.send(&proto::config_pins(&self.setup))?;
        self.send(&proto::config_limits(config))?;
        self.exchange_status(&Outputs::default(), true)?;
        self.encoder.clear();
        self.in_flight.clear();
        let info = BoardInfo { name: "BlueSkyT USB card".into(), firmware: identity.firmware_text() };
        self.identity = Some(identity);
        Ok(info)
    }

    fn disconnect(&mut self) {
        if self.transport.is_some() {
            let _ = self.send(&proto::simple(cmd::ABORT));
            let _ = self.exchange_status(&Outputs::default(), true);
            let _ = self.send(&proto::simple(cmd::DISCONNECT));
        }
        self.transport = None;
        self.cached = None;
        self.config = None;
    }

    fn segment_period(&self) -> f64 {
        proto::SEGMENT_PERIOD
    }

    fn queue_capacity(&self) -> usize {
        proto::QUEUE_CAPACITY
    }

    fn poll(&mut self, outputs: &Outputs) -> Result<Status> {
        if let Some((at, sent, status)) = &self.cached {
            // Position and feed only feed the pendant display; they can wait for the next exchange.
            let same = Outputs { work_position: sent.work_position, feed_rate: sent.feed_rate, ..*outputs } == *sent;
            if at.elapsed() < POLL_INTERVAL && same {
                return Ok(Status { queue_free: self.queue_free, line: self.line, ..status.clone() });
            }
        }
        let status = self.exchange_status(outputs, !outputs.enable)?;
        let board_free = status.queue_free as usize;
        self.track_played(board_free);
        // Segments still waiting in the encoder count as queued.
        self.queue_free = board_free.saturating_sub(self.encoder.pending());
        let config = self.config.as_ref().ok_or(BackendError::NotConnected)?;
        let mut position = [0i64; 6];
        for i in 0..5 {
            position[i] = status.position[i] as i64;
        }
        let _ = config;
        let status = Status {
            position,
            inputs: status.inputs,
            queue_free: self.queue_free,
            idle: status.idle && self.encoder.pending() == 0,
            line: self.line,
        };
        self.cached = Some((Instant::now(), *outputs, status.clone()));
        Ok(status)
    }

    fn push_segments(&mut self, segments: &[Segment], last: bool) -> Result<()> {
        let max = self.identity.as_ref().map(|i| i.max_steps_per_segment()).unwrap_or(410) as i32;
        if let Some(s) = segments.iter().find(|s| s.steps.iter().any(|v| v.abs() > max)) {
            return Err(BackendError::Other(format!("segment too fast for this board ({:?} steps in 4 ms)", s.steps)));
        }
        if let Some(axis) = segments.iter().flat_map(|s| s.steps.iter().enumerate().skip(5)).find(|(_, v)| **v != 0) {
            return Err(BackendError::Other(format!("axis {} is not supported by this board", axis.0)));
        }
        for s in segments {
            match self.in_flight.back_mut() {
                Some((n, line)) if *line == s.line => *n += 1,
                _ => self.in_flight.push_back((1, s.line)),
            }
        }
        if last {
            match self.in_flight.back_mut() {
                Some((n, _)) => *n += 1,
                None => self.in_flight.push_back((1, 0)),
            }
        }
        let input: Vec<([i32; 6], usize)> = segments.iter().map(|s| (s.steps, s.line)).collect();
        let frames = self.encoder.push(&input, last);
        self.send_frames(frames)
    }

    fn axis_move(&mut self, m: &AxisMove) -> Result<()> {
        if m.axis >= 5 {
            return Err(BackendError::Other("axis not supported by this board".into()));
        }
        let (stop_pin, stop_high) = match m.stop_on {
            Some((input, want_active)) => (input.pin, want_active != input.active_low),
            None => (0, false),
        };
        let frame = proto::AxisMoveFrame {
            axis: m.axis as u8,
            positive: m.positive,
            steps: m.steps,
            velocity: m.velocity.round() as u32,
            acceleration: m.acceleration.round() as u32,
            stop_pin,
            stop_high,
        };
        self.mark_busy();
        self.send(&proto::axis_move(&frame))
    }

    fn stop_axis_move(&mut self) -> Result<()> {
        self.send(&proto::simple(cmd::AXIS_STOP))
    }

    fn abort(&mut self) -> Result<()> {
        self.cached = None;
        self.encoder.clear();
        self.in_flight.clear();
        self.send(&proto::simple(cmd::ABORT))
    }

    fn set_position(&mut self, steps: [i64; 6]) -> Result<()> {
        let s: [i32; 5] = std::array::from_fn(|i| steps[i] as i32);
        let p = proto::set_position(&self.last_status, s);
        self.cached = None;
        self.send(&p)
    }
}

/// Firmware versions the plugin talks to with scrambling on.
fn identity_uses_scrambling(fw: u32) -> bool {
    fw == 0 || (222..310).contains(&fw) || (312..320).contains(&fw) || fw >= 322
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A fake board that speaks the protocol well enough for the backend to drive it.
    #[derive(Default)]
    struct FakeBoard {
        sent: Vec<Payload>,
        replies: VecDeque<Payload>,
        position: [i32; 5],
        queued: usize,
    }

    struct FakeTransport(Arc<Mutex<FakeBoard>>);

    impl Transport for FakeTransport {
        fn write(&mut self, wire: &Payload) -> Result<()> {
            let mut b = self.0.lock().unwrap();
            let p = if wire[61..64] == proto::HANDSHAKE_SIGNATURE {
                let mut p = proto::unscramble_out(wire);
                p[63] = cmd::HANDSHAKE;
                p
            } else {
                proto::unscramble_out(wire)
            };
            b.sent.push(p);
            match p[63] {
                cmd::HANDSHAKE => {
                    let mut q = [0u8; 64];
                    q[20..24].copy_from_slice(&325u32.to_le_bytes());
                    q[63] = cmd::HANDSHAKE;
                    b.replies.push_back(proto::scramble_in(&q));
                }
                cmd::STATUS => {
                    let mut q = [0u8; 64];
                    q[0] = 0x6b;
                    q[1] = if b.queued == 0 { 0x80 } else { 0 };
                    for i in 0..5 {
                        q[4 + 4 * i..8 + 4 * i].copy_from_slice(&b.position[i].to_le_bytes());
                    }
                    q[32..36].copy_from_slice(&((250 - b.queued) as u32).to_le_bytes());
                    q[63] = cmd::STATUS;
                    b.replies.push_back(proto::scramble_in(&q));
                }
                cmd::SET_POSITION => {
                    for i in 0..5 {
                        b.position[i] = i32::from_le_bytes(p[4 + 4 * i..8 + 4 * i].try_into().unwrap());
                    }
                }
                cmd::MOTION | cmd::MOTION_END => b.queued += p[62] as usize,
                _ => {}
            }
            Ok(())
        }

        fn read(&mut self, _timeout: Duration) -> Result<Option<Payload>> {
            Ok(self.0.lock().unwrap().replies.pop_front())
        }
    }

    fn board() -> (BlueSkyBoard, Arc<Mutex<FakeBoard>>) {
        let fake = Arc::new(Mutex::new(FakeBoard { position: [-1600, 93073, 1600, 0, 0], ..Default::default() }));
        let f = fake.clone();
        let b = BlueSkyBoard::with_transport(BoardSetup::dmc2(), move || Ok(Box::new(FakeTransport(f.clone())) as Box<dyn Transport>));
        (b, fake)
    }

    #[test]
    fn connect_follows_mach3_sequence() {
        let (mut b, fake) = board();
        let info = b.connect(&MachineConfig::dmc2()).unwrap();
        assert_eq!(info.firmware, "V3.25");
        let cmds: Vec<u8> = fake.lock().unwrap().sent.iter().map(|p| p[63]).collect();
        assert_eq!(cmds, vec![0x63, 0x62, 0x03, 0x02, 0x0E, 0x0F, 0x10, 0x03]);
        // Position counters are kept, not zeroed.
        assert_eq!(fake.lock().unwrap().position, [-1600, 93073, 1600, 0, 0]);
    }

    #[test]
    fn streams_whole_frames_and_tracks_queue() {
        let (mut b, fake) = board();
        b.connect(&MachineConfig::dmc2()).unwrap();
        let segs: Vec<Segment> = (0..14).map(|i| Segment { steps: [4, 0, 0, 0, 0, 0], line: i / 7 }).collect();
        b.push_segments(&segs[..8], false).unwrap();
        // 6 sent, 2 held back until a frame is full.
        assert_eq!(fake.lock().unwrap().queued, 6);
        let s = b.poll(&Outputs { enable: true, ..Default::default() }).unwrap();
        assert_eq!(s.queue_free, 250 - 6 - 2);
        assert!(!s.idle);
        b.push_segments(&segs[8..], true).unwrap();
        // 6 + 6 + (2 + end marker).
        assert_eq!(fake.lock().unwrap().queued, 6 + 6 + 3);
        let last = *fake.lock().unwrap().sent.last().unwrap();
        assert_eq!(last[63], cmd::MOTION_END);
        assert_eq!(last[62], 3);
        assert_eq!(last[20], 0x20);
    }

    #[test]
    fn polls_the_board_at_most_every_25_ms() {
        let (mut b, fake) = board();
        b.connect(&MachineConfig::dmc2()).unwrap();
        let polls = |fake: &Arc<Mutex<FakeBoard>>| fake.lock().unwrap().sent.iter().filter(|p| p[63] == cmd::STATUS).count();
        let out = Outputs { enable: true, ..Default::default() };
        let before = polls(&fake);
        for i in 0..10 {
            b.poll(&Outputs { work_position: [i as f64; 6], ..out }).unwrap();
        }
        assert_eq!(polls(&fake), before + 1);
        // Changed outputs (spindle on) go out at once.
        b.poll(&Outputs { spindle_on: true, ..out }).unwrap();
        assert_eq!(polls(&fake), before + 2);
        // Motion sent between exchanges clears the cached idle flag.
        b.push_segments(&[Segment { steps: [4, 0, 0, 0, 0, 0], line: 0 }], true).unwrap();
        assert!(!b.poll(&Outputs { spindle_on: true, ..out }).unwrap().idle);
    }

    #[test]
    fn rejects_segments_beyond_board_rate() {
        let (mut b, _) = board();
        b.connect(&MachineConfig::dmc2()).unwrap();
        let err = b.push_segments(&[Segment { steps: [411, 0, 0, 0, 0, 0], line: 0 }], true);
        assert!(err.is_err());
    }

    #[test]
    fn stop_level_follows_input_polarity() {
        let (mut b, fake) = board();
        let config = MachineConfig::dmc2();
        b.connect(&config).unwrap();
        let z = config.axes[2].home_input.unwrap();
        b.axis_move(&AxisMove { axis: 2, positive: true, steps: 999_999, velocity: 4000.0, acceleration: 400_000.0, stop_on: Some((z, true)) })
            .unwrap();
        let p = *fake.lock().unwrap().sent.last().unwrap();
        assert_eq!(p[20], 15);
        assert_eq!(p[24], 0, "active-low switch: stop when the pin reads low");
    }
}
