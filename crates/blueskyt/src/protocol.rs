//! Wire format of the BlueSkyT "BL-UsbMach" USB motion card (VID 0x3698,
//! PID 0x0107), as used by the Shariff DMC2's stock controller.
//!
//! Every message is a 64-byte HID report (plus report ID 0). Byte 63 is the
//! command. Payloads are XOR-scrambled with a fixed table. See
//! `docs/dmc2-protocol/protocol-notes.md` for how this was worked out.
//!
//! `P[n]` below is byte n of an outgoing payload, `Q[n]` of an incoming one,
//! both after descrambling.

use machine::config::Input;

pub const VENDOR_ID: u16 = 0x3698;
pub const PRODUCT_ID: u16 = 0x0107;

/// One motion segment lasts 4 ms.
pub const SEGMENT_PERIOD: f64 = 0.004;
/// Segments the board can queue.
pub const QUEUE_CAPACITY: usize = 250;
pub const SEGMENTS_PER_FRAME: usize = 6;

pub mod cmd {
    pub const SET_POSITION: u8 = 0x02;
    pub const STATUS: u8 = 0x03;
    pub const MOTION: u8 = 0x04;
    pub const MOTION_END: u8 = 0x09;
    pub const AXIS_MOVE: u8 = 0x0B;
    pub const AXIS_STOP: u8 = 0x0C;
    pub const CONFIG_AXES: u8 = 0x0E;
    pub const CONFIG_PINS: u8 = 0x0F;
    pub const CONFIG_LIMITS: u8 = 0x10;
    pub const HANDSHAKE: u8 = 0x62;
    pub const ABORT: u8 = 0x63;
    pub const DISCONNECT: u8 = 0x64;
}

/// Scrambling table from the Mach3 plugin.
pub const KEY: [u8; 65] = [
    0x21, 0xF2, 0xA1, 0x94, 0x45, 0xDF, 0x3F, 0x99, 0xAA, 0xBE, 0xF6, 0xD2, 0x3A, 0x77, 0x23, 0x67, 0xC2, 0x3C,
    0xEE, 0xE9, 0x62, 0x38, 0x71, 0xE2, 0xD2, 0xD8, 0x9D, 0xFD, 0xFE, 0xAF, 0x3D, 0x5C, 0x9D, 0xB2, 0x35, 0x26,
    0x88, 0x1A, 0x5C, 0x5A, 0x27, 0x83, 0x81, 0xF9, 0x2C, 0xAE, 0xDF, 0xFA, 0xFC, 0xCE, 0x26, 0x33, 0xDC, 0xCC,
    0xAA, 0xF4, 0xD5, 0xA9, 0x86, 0x2E, 0xC1, 0x70, 0xAE, 0xFE, 0xA7,
];

pub type Payload = [u8; 64];

/// Scramble an outgoing payload.
pub fn scramble_out(p: &Payload) -> Payload {
    std::array::from_fn(|i| p[i] ^ KEY[i])
}

/// Descramble an outgoing payload (for decoding captures).
pub fn unscramble_out(w: &Payload) -> Payload {
    scramble_out(w)
}

/// Descramble an incoming report: the table is read backwards, offset by one.
pub fn unscramble_in(w: &Payload) -> Payload {
    std::array::from_fn(|i| w[i] ^ KEY[64 - i])
}

/// Scramble an incoming report (for test boards).
pub fn scramble_in(q: &Payload) -> Payload {
    unscramble_in(q)
}

/// The handshake's last three wire bytes are sent unscrambled.
pub const HANDSHAKE_SIGNATURE: [u8; 3] = [0x1F, 0xBA, 0x62];

/// Wire bytes of a handshake payload.
pub fn handshake_wire(p: &Payload) -> Payload {
    let mut w = scramble_out(p);
    w[61..64].copy_from_slice(&HANDSHAKE_SIGNATURE);
    w
}

/// Mach3 input signal numbers used in the handshake's input map.
pub mod input_signal {
    pub const X_HOME: usize = 2;
    pub const Y_HOME: usize = 5;
    pub const Z_HOME: usize = 8;
    pub const A_HOME: usize = 11;
    pub const B_HOME: usize = 14;
    pub const PROBE: usize = 22;
    pub const ESTOP: usize = 25;
    pub const OEM_TRIG1: usize = 29;
    pub const HOME: [usize; 5] = [X_HOME, Y_HOME, Z_HOME, A_HOME, B_HOME];
}

/// A Mach3 output signal routed to a board pin. Signal 1..=6 are Enable1..6,
/// 7.. are Output#1...
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputPin {
    pub signal: u8,
    pub pin: u8,
    pub active_low: bool,
    pub enabled: bool,
}

pub const ENABLE1: u8 = 1;
pub const OUTPUT1: u8 = 7;

/// Board wiring and plugin settings that live in the Mach3 profile but are
/// not part of the generic machine configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardSetup {
    /// Step and direction pin per axis X..B (board pin numbers, 1-based).
    pub step_pins: [u8; 5],
    pub dir_pins: [u8; 5],
    pub outputs: Vec<OutputPin>,
    /// Extra inputs by Mach3 signal number (OEM triggers and the like).
    pub extra_inputs: Vec<(usize, Input)>,
    pub enable_signal: u8,
    pub spindle_signal: u8,
    pub flood_signal: u8,
    pub mist_signal: u8,
    /// PWM base frequency × 10 as the plugin sends it (5000 Hz → 10000, capped).
    pub pwm_base: u16,
    /// Handshake P[54..=60] as Mach3 sends them for this profile.
    pub handshake_flags: [u8; 7],
    /// Config frame 0x0E, P[50..=52].
    pub axis_flags: [u8; 3],
}

impl BoardSetup {
    /// Wiring of the Shariff DMC2, from its Mach3 profile and USB captures.
    pub fn dmc2() -> Self {
        let out = |signal, pin, active_low, enabled| OutputPin { signal, pin, active_low, enabled };
        BoardSetup {
            step_pins: [1, 3, 5, 7, 9],
            dir_pins: [2, 4, 6, 8, 10],
            outputs: vec![
                out(ENABLE1, 14, false, true),
                out(OUTPUT1, 11, true, true),
                out(OUTPUT1 + 1, 12, false, true),
                out(OUTPUT1 + 2, 13, false, true),
                out(OUTPUT1 + 3, 14, false, false),
            ],
            // OEM Trig 4..13 on pins 16..25, active low.
            extra_inputs: (0..10).map(|i| (input_signal::OEM_TRIG1 + 3 + i, Input::new(16 + i as u8, true))).collect(),
            enable_signal: ENABLE1,
            spindle_signal: OUTPUT1,
            flood_signal: OUTPUT1 + 2,
            mist_signal: OUTPUT1 + 2,
            pwm_base: 10000,
            handshake_flags: [0x07, 0x00, 0x00, 0x10, 0x0F, 0x02, 0x00],
            axis_flags: [0x00, 0x00, 0x02],
        }
    }

    fn output_bits(&self, active: impl Fn(u8) -> bool) -> u32 {
        self.outputs
            .iter()
            .filter(|o| o.enabled)
            .filter(|o| active(o.signal) != o.active_low)
            .fold(0, |bits, o| bits | 1 << o.signal)
    }
}

fn put_u16(p: &mut Payload, at: usize, v: u16) {
    p[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

fn put_i32(p: &mut Payload, at: usize, v: i32) {
    p[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn put_f32(p: &mut Payload, at: usize, v: f32) {
    p[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn get_i32(q: &Payload, at: usize) -> i32 {
    i32::from_le_bytes(q[at..at + 4].try_into().unwrap())
}

fn get_u32(q: &Payload, at: usize) -> u32 {
    u32::from_le_bytes(q[at..at + 4].try_into().unwrap())
}

fn with_cmd(command: u8) -> Payload {
    let mut p = [0u8; 64];
    p[63] = command;
    p
}

/// A payload carrying only a command (abort, jog stop, disconnect).
pub fn simple(command: u8) -> Payload {
    with_cmd(command)
}

/// Input map entry: `(pin << 1) | active_low`, for pins 10..=25 except 14.
fn input_code(input: &Input) -> u8 {
    if (10..=25).contains(&input.pin) && input.pin != 14 {
        (input.pin << 1) | input.active_low as u8
    } else {
        0
    }
}

pub fn handshake(config: &machine::MachineConfig, setup: &BoardSetup) -> Payload {
    let mut p = with_cmd(cmd::HANDSHAKE);
    for (axis, signal) in input_signal::HOME.iter().enumerate() {
        if let Some(input) = config.axes[axis].home_input.filter(|_| config.axes[axis].enabled) {
            p[*signal] = input_code(&input);
        }
    }
    if let Some(probe) = &config.probe {
        p[input_signal::PROBE] = input_code(probe);
    }
    if let Some(estop) = &config.estop {
        p[input_signal::ESTOP] = input_code(estop);
    }
    for (signal, input) in &setup.extra_inputs {
        if *signal <= 52 {
            p[*signal] = input_code(input);
        }
    }
    // The plugin always forces these two bytes.
    p[41] = 0x35;
    p[51] = 0x8E;
    p[54..61].copy_from_slice(&setup.handshake_flags);
    p
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardIdentity {
    pub firmware: u32,
    pub board_type: u32,
    pub date_code: u32,
    pub chip_id: [u8; 12],
}

impl BoardIdentity {
    /// Largest step count one segment may carry on this board.
    pub fn max_steps_per_segment(&self) -> u32 {
        if self.board_type == 0x0006_2F3E {
            900
        } else {
            410
        }
    }

    pub fn firmware_text(&self) -> String {
        format!("V{}.{:02}", self.firmware / 100, self.firmware % 100)
    }
}

pub fn parse_handshake_reply(q: &Payload) -> Option<BoardIdentity> {
    if q[63] != cmd::HANDSHAKE {
        return None;
    }
    Some(BoardIdentity {
        firmware: get_u32(q, 20),
        board_type: get_u32(q, 59),
        date_code: get_u32(q, 4),
        chip_id: q[8..20].try_into().unwrap(),
    })
}

/// 0x0E: steps per unit, max velocity and acceleration per axis.
pub fn config_axes(config: &machine::MachineConfig, setup: &BoardSetup) -> Payload {
    let mut p = with_cmd(cmd::CONFIG_AXES);
    for (i, a) in config.axes.iter().take(5).enumerate() {
        put_u16(&mut p, 2 * i, a.steps_per_unit.round() as u16);
        put_i32(&mut p, 10 + 4 * i, (a.max_velocity * a.steps_per_unit).round() as i32);
        put_i32(&mut p, 30 + 4 * i, (a.acceleration * a.steps_per_unit).round() as i32);
    }
    p[50..53].copy_from_slice(&setup.axis_flags);
    p
}

/// 0x0F: step, direction and output pins.
pub fn config_pins(setup: &BoardSetup) -> Payload {
    let mut p = with_cmd(cmd::CONFIG_PINS);
    for i in 0..5 {
        p[i] = setup.step_pins[i] - 1;
        p[5 + i] = setup.dir_pins[i] - 1;
    }
    let mut enable = 0u32;
    let mut invert = 0u32;
    for o in &setup.outputs {
        p[10 + o.signal as usize] = o.pin - 1;
        if o.enabled {
            enable |= 1 << o.signal;
            if o.active_low {
                invert |= 1 << o.signal;
            }
        }
    }
    p[40..44].copy_from_slice(&enable.to_le_bytes());
    p[44..48].copy_from_slice(&invert.to_le_bytes());
    p
}

/// 0x10: soft limits, home directions and home speeds.
pub fn config_limits(config: &machine::MachineConfig) -> Payload {
    let mut p = with_cmd(cmd::CONFIG_LIMITS);
    let mut home_dirs = 0u8;
    for (i, a) in config.axes.iter().take(5).enumerate() {
        put_i32(&mut p, 4 * i, (a.soft_max * a.steps_per_unit).round() as i32);
        put_i32(&mut p, 20 + 4 * i, (a.soft_min * a.steps_per_unit).round() as i32);
        if a.home_positive {
            home_dirs |= 1 << i;
        }
        p[41 + i] = (a.home_speed * 100.0).round() as u8;
    }
    p[40] = home_dirs;
    p
}

/// Host-side state carried in every status poll.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HostState {
    pub reset: bool,
    pub outputs: machine::Outputs,
    pub max_rpm: f64,
    pub z_steps_per_unit: f64,
}

/// 0x03 poll: outputs, spindle and pendant data.
pub fn status_request(setup: &BoardSetup, s: &HostState) -> Payload {
    let mut p = with_cmd(cmd::STATUS);
    let o = &s.outputs;
    let bits = setup.output_bits(|signal| {
        (signal == setup.enable_signal && !s.reset)
            || (signal == setup.spindle_signal && o.spindle_on)
            || (signal == setup.flood_signal && o.flood)
            || (signal == setup.mist_signal && o.mist)
    });
    p[0..4].copy_from_slice(&bits.to_le_bytes());
    let duty = (o.spindle_duty * 100.0).round().clamp(0.0, 100.0) as u8;
    p[4] = duty;
    p[5] = o.spindle_on as u8;
    put_u16(&mut p, 6, s.z_steps_per_unit.round() as u16);
    p[8] = s.reset as u8;
    put_u16(&mut p, 15, o.feed_rate.round().clamp(0.0, 65535.0) as u16);
    put_u16(&mut p, 17, s.max_rpm.round() as u16);
    p[19] = 1;
    put_f32(&mut p, 20, 0.01);
    put_u16(&mut p, 24, o.spindle_rpm.round().clamp(0.0, 65535.0) as u16);
    put_u16(&mut p, 28, (setup.pwm_base as u32 * duty as u32 / 100) as u16);
    put_u16(&mut p, 30, setup.pwm_base);
    for i in 0..5 {
        put_f32(&mut p, 32 + 4 * i, o.work_position[i] as f32);
    }
    p
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoardStatus {
    /// Bit n = level of board input pin n.
    pub inputs: u32,
    pub idle: bool,
    pub position: [i32; 5],
    pub queue_free: u32,
    /// The raw report, which 0x02 reuses as its template.
    pub raw: Payload,
}

pub fn parse_status(q: &Payload) -> Option<BoardStatus> {
    if q[63] != cmd::STATUS {
        return None;
    }
    let mut inputs = (q[0] as u32) << 10;
    // Q[41] carries pins 17..24; pin 17 is already in Q[0].
    inputs |= ((q[41] as u32) >> 1) << 18;
    inputs |= ((q[1] & 1) as u32) << 25;
    Some(BoardStatus {
        inputs,
        idle: q[1] & 0x80 != 0,
        position: std::array::from_fn(|i| get_i32(q, 4 + 4 * i)),
        queue_free: get_u32(q, 32),
        raw: *q,
    })
}

/// 0x02: set the board's position counters. The rest of the payload is the
/// last status report, as the plugin sends it.
pub fn set_position(last_status: &Payload, steps: [i32; 5]) -> Payload {
    let mut p = *last_status;
    for (i, v) in steps.into_iter().enumerate() {
        put_i32(&mut p, 4 + 4 * i, v);
    }
    p[63] = cmd::SET_POSITION;
    p
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisMoveFrame {
    pub axis: u8,
    pub positive: bool,
    pub steps: u32,
    pub velocity: u32,
    pub acceleration: u32,
    /// Stop pin, 0 for none.
    pub stop_pin: u8,
    /// Stop when the pin reads high (true) or low (false).
    pub stop_high: bool,
}

/// 0x0B: single-axis move, optionally stopping when a pin reaches a level.
pub fn axis_move(m: &AxisMoveFrame) -> Payload {
    let mut p = with_cmd(cmd::AXIS_MOVE);
    put_i32(&mut p, 0, m.axis as i32);
    put_i32(&mut p, 4, m.positive as i32);
    put_i32(&mut p, 8, m.steps as i32);
    put_i32(&mut p, 12, m.velocity as i32);
    put_i32(&mut p, 16, m.acceleration as i32);
    put_i32(&mut p, 20, m.stop_pin as i32);
    put_i32(&mut p, 24, m.stop_high as i32);
    p
}

/// Packs step segments into 0x04/0x09 frames.
#[derive(Debug, Default)]
pub struct SegmentEncoder {
    /// Direction bits of the last segment; an axis that does not move keeps its direction.
    last_dirs: u8,
    pending: Vec<[u8; 10]>,
}

impl SegmentEncoder {
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    pub fn clear(&mut self) {
        self.pending.clear();
    }

    fn encode(&mut self, steps: &[i32; 6], line: usize) -> [u8; 10] {
        let mut s = [0u8; 10];
        let mut dirs = self.last_dirs;
        let mut hi = [0u8; 5];
        for i in 0..5 {
            let v = steps[i];
            if v != 0 {
                // Direction bit set means positive (confirmed by capture).
                if v > 0 {
                    dirs |= 1 << i;
                } else {
                    dirs &= !(1 << i);
                }
            }
            let mag = v.unsigned_abs().min(1023);
            s[1 + i] = (mag & 0xFF) as u8;
            hi[i] = (mag >> 8) as u8;
        }
        self.last_dirs = dirs;
        s[0] = dirs | hi[0] << 6;
        let line = (line as u32).min(0xFF_FFFF);
        s[6..9].copy_from_slice(&line.to_le_bytes()[..3]);
        s[9] = hi[1] | hi[2] << 2 | hi[3] << 4 | hi[4] << 6;
        s
    }

    /// Add segments and return the frames ready to send. With `last`, the
    /// remainder is flushed as a 0x09 frame ending in an end-of-move marker.
    pub fn push(&mut self, segments: &[([i32; 6], usize)], last: bool) -> Vec<Payload> {
        for (steps, line) in segments {
            let s = self.encode(steps, *line);
            self.pending.push(s);
        }
        if last {
            // The plugin's end marker is an empty segment with only the end flag.
            self.pending.push([0x20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        }
        // Without `last`, only whole frames go out. With it, everything goes and
        // the final (possibly short) frame is sent as 0x09.
        let n = self.pending.len();
        let full = if last { (n - 1) / SEGMENTS_PER_FRAME } else { n / SEGMENTS_PER_FRAME } * SEGMENTS_PER_FRAME;
        let mut frames: Vec<Payload> =
            self.pending.drain(..full).collect::<Vec<_>>().chunks(SEGMENTS_PER_FRAME).map(|c| Self::frame(cmd::MOTION, c)).collect();
        if last {
            let rest: Vec<_> = self.pending.drain(..).collect();
            frames.push(Self::frame(cmd::MOTION_END, &rest));
        }
        frames
    }

    fn frame(command: u8, segments: &[[u8; 10]]) -> Payload {
        let mut p = with_cmd(command);
        for (k, s) in segments.iter().enumerate() {
            p[10 * k..10 * k + 10].copy_from_slice(s);
        }
        p[62] = segments.len() as u8;
        p
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use machine::MachineConfig;

    /// Captured payloads (descrambled) from the DMC2 with Mach3, 2026-10-28.
    fn hex(s: &str) -> Payload {
        let v: Vec<u8> = (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect();
        v.try_into().unwrap()
    }

    const CAP_HANDSHAKE_WIRE: &str = "21f2b99445c43f99b5bef6d23a772367c23ceee9623866e2d2cd9dfdfeaf3d5cbc911001a131717516b681f92caedffafcce26bddcccadf4d5b9892cc11fba62";
    const CAP_HANDSHAKE_REPLY_WIRE: &str = "07feae70d648b2a8870bd9ca5a4b7186e4a2fdc969f88183b0985c1a77b734b2675c3daffefd9dd8ec2d713862e9ee3cc26723773ad2f6beaa993fdf4594a190";
    const CAP_CONFIG_AXES: &str = "c800c800c800800280022468000024680000204e0000803e0000803e0000801a0600801a0600801a0600002c0100002c0100000002000000000000000000000e";
    const CAP_CONFIG_PINS: &str = "00020406080103050709000d00000000000a0b0c0d0000000000000000000000000000000000000082030000800000000000000000000000000000000000000f";
    const CAP_CONFIG_LIMITS: &str = "409c0000589800000000000000fa000000fa0000a015ffff00000000709affff0006ffff0006ffff061414141414000000000000000000000000000000000010";
    const CAP_STATUS_IDLE: &str = "6b810000c0f9ffff916b010040060000000000000000000097c20000ff910100fa000000000000003ecf00000000000000000000000000000000000000000003";
    const CAP_STATUS_MOVING: &str = "6b0100002b6e0000916b010040060000000000000000000097c2000000000000e9000000000000003ecf00008048280000000000000000000000000000000003";
    const CAP_JOG_X: &str = "00000000010000007f96980073240000801a0600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000b";
    const CAP_HOME_Z_SEEK: &str = "02000000010000003f420f00a00f0000801a06000f0000000000000000000000000000000000000000000000000000000000000000000000000000000000000b";
    const CAP_HOME_Z_BACKOFF: &str = "02000000000000003f420f0090010000801a06000f0000000100000000000000000000000000000000000000000000000000000000000000000000000000000b";
    const CAP_HOME_X_SEEK: &str = "00000000000000003f420f00d4140000801a06000c0000000100000000000000000000000000000000000000000000000000000000000000000000000000000b";
    const CAP_MOTION_FIRST: &str = "00020000000000000000000400000000000000000004000000000000000000040000000000000000000400000000000000000004000000000000000000000604";
    const CAP_MOTION_END: &str = "00040000000000000000000400000000000000000002000000000000000020000000000000000000000000000000000000000000000000000000000000000409";
    const CAP_POLL_F300: &str = "820000002800c800000000000000002c01a861010ad7233c10270000a00f10277b341343d783304348a1ff420000000000000000000000000000000000000003";

    #[test]
    fn handshake_matches_mach3() {
        let p = handshake(&MachineConfig::dmc2(), &BoardSetup::dmc2());
        assert_eq!(handshake_wire(&p), hex(CAP_HANDSHAKE_WIRE));
    }

    #[test]
    fn handshake_reply_identifies_board() {
        let q = unscramble_in(&hex(CAP_HANDSHAKE_REPLY_WIRE));
        let id = parse_handshake_reply(&q).unwrap();
        assert_eq!(id.firmware, 325);
        assert_eq!(id.firmware_text(), "V3.25");
        assert_eq!(id.board_type, 0);
        assert_eq!(id.max_steps_per_segment(), 410);
        assert_eq!(id.date_code, 20211223);
        assert_eq!(id.chip_id[..4], [0x52, 0xFF, 0x73, 0x06]);
    }

    #[test]
    fn config_frames_match_mach3() {
        let setup = BoardSetup::dmc2();
        let config = MachineConfig::dmc2();
        assert_eq!(config_axes(&config, &setup), hex(CAP_CONFIG_AXES));
        assert_eq!(config_pins(&setup), hex(CAP_CONFIG_PINS));
        assert_eq!(config_limits(&config), hex(CAP_CONFIG_LIMITS));
    }

    #[test]
    fn status_reports_decode() {
        let s = parse_status(&hex(CAP_STATUS_IDLE)).unwrap();
        assert!(s.idle);
        assert_eq!(s.position, [-1600, 93073, 1600, 0, 0]);
        assert_eq!(s.queue_free, 250);
        let config = MachineConfig::dmc2();
        // Nothing active at rest: E-stop, probe and home switches all released.
        assert!(!config.estop.unwrap().is_active(s.inputs));
        assert!(!config.probe.unwrap().is_active(s.inputs));
        for a in &config.axes[..3] {
            assert!(!a.home_input.unwrap().is_active(s.inputs));
        }
        let m = parse_status(&hex(CAP_STATUS_MOVING)).unwrap();
        assert!(!m.idle);
        assert_eq!(m.queue_free, 233);
        assert_eq!(m.position[0], 28203);
    }

    #[test]
    fn axis_moves_match_mach3() {
        let jog = AxisMoveFrame { axis: 0, positive: true, steps: 9_999_999, velocity: 9331, acceleration: 400_000, stop_pin: 0, stop_high: false };
        assert_eq!(axis_move(&jog), hex(CAP_JOG_X));
        let seek = AxisMoveFrame { axis: 2, positive: true, steps: 999_999, velocity: 4000, acceleration: 400_000, stop_pin: 15, stop_high: false };
        assert_eq!(axis_move(&seek), hex(CAP_HOME_Z_SEEK));
        let back = AxisMoveFrame { positive: false, velocity: 400, stop_high: true, ..seek };
        assert_eq!(axis_move(&back), hex(CAP_HOME_Z_BACKOFF));
        let x = AxisMoveFrame { axis: 0, positive: false, steps: 999_999, velocity: 5332, acceleration: 400_000, stop_pin: 12, stop_high: true };
        assert_eq!(axis_move(&x), hex(CAP_HOME_X_SEEK));
    }

    #[test]
    fn segments_match_mach3() {
        let mut enc = SegmentEncoder::default();
        let mut segs = vec![([-2, 0, 0, 0, 0, 0], 0)];
        segs.extend(std::iter::repeat_n(([-4, 0, 0, 0, 0, 0], 0), 5));
        let frames = enc.push(&segs, false);
        assert_eq!(frames, vec![hex(CAP_MOTION_FIRST)]);
        let frames = enc.push(&[([-4, 0, 0, 0, 0, 0], 0), ([-4, 0, 0, 0, 0, 0], 0), ([-2, 0, 0, 0, 0, 0], 0)], true);
        assert_eq!(frames, vec![hex(CAP_MOTION_END)]);
    }

    #[test]
    fn segment_magnitudes_use_ten_bits() {
        let mut enc = SegmentEncoder::default();
        let frames = enc.push(&[([400, -300, 5, 0, 0, 0], 0x123456)], true);
        let s = &frames[0][..10];
        assert_eq!(s[0] & 0x1F, 0b001 | 0b100);
        assert_eq!((s[0] >> 6) as u32 * 256 + s[1] as u32, 400);
        assert_eq!((s[9] & 3) as u32 * 256 + s[2] as u32, 300);
        assert_eq!(&s[6..9], &[0x56, 0x34, 0x12]);
    }

    #[test]
    fn poll_matches_mach3_layout() {
        let setup = BoardSetup::dmc2();
        let cap = hex(CAP_POLL_F300);
        let state = HostState {
            reset: false,
            outputs: machine::Outputs {
                enable: true,
                spindle_on: false,
                spindle_duty: 0.40,
                spindle_rpm: 10000.0,
                feed_rate: 300.0,
                work_position: [cap_f32(&cap, 32), cap_f32(&cap, 36), cap_f32(&cap, 40), 0.0, 0.0, 0.0],
                ..Default::default()
            },
            max_rpm: 25000.0,
            z_steps_per_unit: 200.0,
        };
        assert_eq!(status_request(&setup, &state), cap);
    }

    fn cap_f32(p: &Payload, at: usize) -> f64 {
        f32::from_le_bytes(p[at..at + 4].try_into().unwrap()) as f64
    }

    #[test]
    fn scrambling_round_trips() {
        let p: Payload = std::array::from_fn(|i| i as u8);
        assert_eq!(unscramble_out(&scramble_out(&p)), p);
        assert_eq!(unscramble_in(&scramble_in(&p)), p);
    }
}
