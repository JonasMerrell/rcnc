//! Listening to the cut (the `listen` crate): the microphone's input, what it says about the running
//! line, the cut log written next to the program, the live indicator, and the tap test at a tool change.
//! Nothing here touches the machine's motion.

use super::palette::*;
use eframe::egui::{self, RichText};
use gcode::Spindle;
use listen::{Analyzer, Context, CutLog, Frame, Guard, GuardSettings, Limiter, LimiterSettings, Source};
use machine::{Command, ControllerHandle, Mode, Snapshot};
use std::sync::Arc;

/// A WAV file to play in place of a microphone (`--sim-audio=file.wav`).
pub static SIM_AUDIO: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
/// The data folder (the listening settings go there).
pub static DATA: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

/// Which input to listen on, saved in the data folder.
#[derive(Default, serde::Serialize, serde::Deserialize)]
struct EarsSettings {
    on: bool,
    device: Option<String>,
    /// Hold the feed when the spindle bogs (heard droop over its no-load droop, for a time).
    #[serde(default)]
    guard: GuardSettings,
    /// The adaptive feed's tuning; a program turns it on with `(RCNC ADAPTIVE_FEED ...)`.
    #[serde(default)]
    limiter: LimiterSettings,
}

pub struct Ears {
    settings: EarsSettings,
    file: std::path::PathBuf,
    source: Option<Source>,
    analyzer: Option<Analyzer>,
    /// Why there's no input, if there isn't.
    problem: Option<String>,
    /// What the running program sounds like, and where it goes when the program ends; when it started.
    log: Option<(std::path::PathBuf, CutLog)>,
    log_start: std::time::Instant,
    guard: Guard,
    /// The adaptive feed (stage 1: reduce-only, on the spindle's sag), and the share last sent to the machine.
    limiter: Limiter,
    adaptive_sent: f64,
    /// Per program: its adaptive feed settings (None: not asked for) and, per line, whether it's on there.
    adaptive: (usize, Option<LimiterSettings>, Vec<bool>),
    /// Why the guard last held the feed, shown until the program moves on again.
    held: Option<String>,
    /// The speed (rpm / 250) the unheard-spindle hold last fired at, so it fires once per speed.
    unheard_held: Option<i64>,
    /// Since the listening started (the guard's clock).
    clock: std::time::Instant,
    /// A program with `(RCNC RECORD_AUDIO)` in it: the whole run's sound, written as a WAV beside it.
    /// The run's recording, written to disk as it comes (`(RCNC RECORD_AUDIO)`).
    recording: Option<listen::WavRecorder>,
    /// When the running log was last written out, and the write in flight: a crash keeps all but the last
    /// few seconds of the run's log.
    log_saved: std::time::Instant,
    log_saving: Option<std::thread::JoinHandle<()>>,
    pub last: Option<Frame>,
    /// Recent chatter verdicts, to steady the indicator.
    chatter_recent: std::collections::VecDeque<bool>,
    tap: Tap,
    /// The operation each program line belongs to (its last "(Name - Kind)" comment), per program.
    ops: (usize, Vec<String>),
}

enum Tap {
    Idle,
    /// Recording for a tap test of this tool until then.
    Recording { tool: u32, until: std::time::Instant, samples: Vec<f32> },
    Done { tool: u32, result: Result<listen::tap::Ring, String> },
}

impl Ears {
    pub fn new() -> Ears {
        let file = DATA.get().cloned().unwrap_or_default().join("listen.json");
        let settings = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let mut e = Ears {
            settings,
            file,
            source: None,
            analyzer: None,
            problem: None,
            log: None,
            log_start: std::time::Instant::now(),
            guard: Guard::default(),
            limiter: Limiter::new(LimiterSettings::default()),
            adaptive_sent: 1.0,
            adaptive: (0, None, vec![]),
            held: None,
            unheard_held: None,
            clock: std::time::Instant::now(),
            recording: None,
            log_saved: std::time::Instant::now(),
            log_saving: None,
            last: None,
            chatter_recent: Default::default(),
            tap: Tap::Idle,
            ops: (0, vec![]),
        };
        e.guard = Guard::new(e.settings.guard);
        e.limiter = Limiter::new(e.settings.limiter);
        e.reopen();
        e
    }

    fn save(&self) {
        if let Ok(s) = serde_json::to_string_pretty(&self.settings) {
            let _ = std::fs::write(&self.file, s);
        }
    }

    fn reopen(&mut self) {
        self.source = None;
        self.analyzer = None;
        self.problem = None;
        if let Some(wav) = SIM_AUDIO.get() {
            match Source::wav(wav) {
                Ok(s) => self.source = Some(s),
                Err(e) => self.problem = Some(e),
            }
        } else if self.settings.on {
            match listen::open(self.settings.device.as_deref()) {
                Ok(s) => self.source = Some(s),
                Err(e) => self.problem = Some(e),
            }
        }
        if let Some(s) = &self.source {
            self.analyzer = Some(Analyzer::new(s.rate()));
        }
    }

    /// Each frame: take what came in, analyse it against what the machine is doing, log it per line.
    pub fn poll(&mut self, s: &Snapshot, program: Option<&std::path::Path>, machine: &ControllerHandle) {
        if self.source.is_none() {
            return;
        }
        let ctx = self.context(s);
        let Some(src) = &mut self.source else { return };
        let samples = src.take();
        if let Tap::Recording { tool, until, samples: rec } = &mut self.tap {
            rec.extend_from_slice(&samples);
            if std::time::Instant::now() >= *until {
                let (tool, rec) = (*tool, std::mem::take(rec));
                let rate = src.rate();
                self.tap = Tap::Done { tool, result: listen::tap::analyse(&rec, rate, 500.0, 0.45 * rate as f64) };
            }
            return;
        }
        // the hold is over once the program moves again (Cycle Start) or stops
        if self.held.is_some() && (!s.hold || !matches!(s.mode, Mode::Running)) && s.velocity > 0.0 {
            self.held = None;
        }
        let Some(a) = &mut self.analyzer else { return };
        let frames = a.push(&samples, &ctx);
        // a program running: log it, and write the log beside the program when it stops
        let running = matches!(s.mode, Mode::Running) && s.program_name.is_some();
        if running && self.log.is_none() {
            if let Some(p) = program {
                // an air cut's log goes beside the real one, not over it
                let air = s.air_cut.is_some();
                self.log = Some((CutLog::path_for_run(p, air), CutLog::new(&p.display().to_string(), &src.name())));
                self.log_start = std::time::Instant::now();
                self.log_saved = std::time::Instant::now();
                // at most an hour of it (about 330 MB at 48 kHz)
                self.recording = records_audio(&s.program)
                    .then(|| listen::WavRecorder::create(&CutLog::wav_for_run(p, air), src.rate(), 3600.0))
                    .and_then(|r| r.map_err(|e| eprintln!("recording: {e}")).ok());
            }
        }
        // windows come in batches a few times a second: spread them back over the time they took
        let now = self.log_start.elapsed().as_secs_f64();
        let step = samples_per_frame_s(src.rate());
        let n = frames.len();
        // the bog guard: hold the feed (the spindle stays on) when the spindle can't keep its speed
        let gt = self.clock.elapsed().as_secs_f64();
        // the adaptive feed, where the program asks for it: only with the guard armed behind it
        if self.adaptive.0 != Arc::as_ptr(&s.program) as usize {
            let (set, on) = adaptive_lines(&s.program, self.settings.limiter);
            self.adaptive = (Arc::as_ptr(&s.program) as usize, set, on);
            self.limiter = Limiter::new(set.unwrap_or(self.settings.limiter));
        }
        let mut asked = vec![None; n];
        let mut over = vec![None; n];
        for (i, f) in frames.iter().enumerate() {
            let t = gt - (n - 1 - i) as f64 * step;
            let held = self.guard.update(t, f);
            over[i] = self.guard.excess(f);
            if let Some(why) = held {
                if matches!(s.mode, Mode::Running) && !s.hold {
                    machine.send(Command::FeedHold);
                    machine.send(Command::Note(why.clone()));
                    self.held = Some(why);
                }
            }
            let line_on = self.adaptive.1.is_some() && f.ctx.line.checked_sub(1).and_then(|l| self.adaptive.2.get(l)).copied().unwrap_or(false);
            if running && line_on && self.settings.guard.on {
                asked[i] = Some(self.limiter.update(t, f, self.guard.excess(f)));
            } else {
                self.limiter.reset();
            }
        }
        // a program that needs the hold mustn't move with the spindle on before the hold has heard it and
        // learned its no-load speed there: hold the feed once per speed instead (Cycle Start carries on)
        if running {
            let key = (ctx.rpm / 250.0).round() as i64;
            let learned = ctx.rpm > 0.0 && self.guard.air_droop(ctx.rpm).is_some();
            if holds_unheard(requires_guard(&s.program), ctx.cutting, ctx.rpm, learned, self.unheard_held == Some(key), self.settings.guard.on) && !s.hold {
                let why = format!(
                    "Feed hold: the listening hold hasn't heard the spindle at S{:.0} yet (no speed found in its sound, so no no-load speed learned) and this program needs it (RCNC REQUIRE_GUARD). Check the transducer and the level; Cycle Start carries on without the hold at this speed",
                    ctx.rpm
                );
                machine.send(Command::FeedHold);
                machine.send(Command::Note(why.clone()));
                self.held = Some(why);
                self.unheard_held = Some(key);
            }
        } else {
            self.unheard_held = None;
        }
        let want = asked.iter().rev().flatten().next().copied().unwrap_or(1.0);
        if (want - self.adaptive_sent).abs() > 0.004 || (want == 1.0 && self.adaptive_sent != 1.0) {
            machine.send(Command::SetAdaptiveFeed(want));
            self.adaptive_sent = want;
        }
        if let Some((_, log)) = &mut self.log {
            if s.rotation != 0.0 {
                log.rotation_deg = Some(s.rotation);
            }
            for (i, f) in frames.iter().enumerate() {
                log.add_at(now - (n - 1 - i) as f64 * step, f);
                if let Some(m) = log.timeline.last_mut() {
                    m.adaptive_feed = asked[i];
                    m.over_no_load = over[i];
                }
            }
        }
        if let Some(rec) = &mut self.recording {
            if let Err(e) = rec.push(&samples) {
                eprintln!("recording: {e}");
                self.recording = None;
            }
        }
        // the log so far, every few seconds, on a thread of its own (it's a few MB by the end of a long run)
        if let Some((path, log)) = &self.log {
            let idle = self.log_saving.as_ref().is_none_or(|h| h.is_finished());
            if idle && self.log_saved.elapsed().as_secs_f64() >= LOG_SAVE_S && !log.lines.is_empty() {
                self.log_saved = std::time::Instant::now();
                let (path, log) = (path.clone(), log.clone());
                self.log_saving = std::thread::Builder::new()
                    .name("cut log".into())
                    .spawn(move || {
                        if let Err(e) = log.save(&path) {
                            eprintln!("cut log: {e}");
                        }
                    })
                    .ok();
            }
        }
        if !running && !matches!(s.mode, Mode::Probing) && s.paused.is_none() {
            if let Some((path, mut log)) = self.log.take() {
                if let Some(h) = self.log_saving.take() {
                    let _ = h.join();
                }
                if !log.lines.is_empty() {
                    // where it chattered: a speed to try next time (in the log for rCAD; never changed here)
                    log.suggest(s.config.spindle_max_rpm.max(1.0));
                    let _ = log.save(&path);
                }
            }
            if let Some(rec) = self.recording.take() {
                if let Err(e) = rec.finish() {
                    eprintln!("recording: {e}");
                }
            }
        }
        if let Some(f) = frames.last() {
            self.chatter_recent.push_back(f.chatter.is_some());
            if self.chatter_recent.len() > 6 {
                self.chatter_recent.pop_front();
            }
            self.last = Some(f.clone());
        }
    }

    fn context(&mut self, s: &Snapshot) -> Context {
        let line = s.line.map(|l| l + 1).unwrap_or(0);
        if self.ops.0 != Arc::as_ptr(&s.program) as usize {
            let mut cur = String::new();
            self.ops = (
                Arc::as_ptr(&s.program) as usize,
                s.program
                    .lines()
                    .map(|l| {
                        let t = l.trim();
                        if let Some(name) = t.strip_prefix('(').and_then(|r| r.strip_suffix(')')).and_then(|r| r.rsplit_once(" - ").map(|x| x.0)) {
                            if !name.starts_with('T') && !name.contains("made by") {
                                cur = name.to_string();
                            }
                        }
                        cur.clone()
                    })
                    .collect(),
            );
        }
        let tool = s.tools.get(&s.tool);
        Context {
            line,
            op: line.checked_sub(1).and_then(|i| self.ops.1.get(i)).cloned().unwrap_or_default(),
            tool: s.tool,
            teeth: tool.map(|t| t.flutes).filter(|f| *f > 0).unwrap_or(2),
            rpm: if s.spindle == Spindle::Off { 0.0 } else { s.rpm },
            feed: s.feed_rate,
            pos: [s.work[0], s.work[1], s.work[2]],
            velocity: s.velocity,
            cutting: s.spindle != Spindle::Off && matches!(s.mode, Mode::Running) && s.velocity > 0.0,
        }
    }

    /// Whether the bog guard can hold the feed: Ok(true) armed (the spindle heard and its no-load speed
    /// learned at this speed), Ok(false) ready (a live input and the hold on: it arms once the spindle runs),
    /// Err(why) not.
    pub fn arm_state(&self) -> Result<bool, String> {
        let input = match &self.source {
            None => Err(self.problem.clone().map(|p| format!("there is no input ({p})")).unwrap_or("listening is off".into())),
            Some(_) => Ok(()),
        };
        let rpm = self.last.as_ref().map(|f| f.ctx.rpm).unwrap_or(0.0);
        arm(input, self.settings.guard.on, rpm, rpm > 0.0 && self.guard.air_droop(rpm).is_some())
    }

    /// Whether a program that needs the hold may start: listening, with an input open, and the hold on (the
    /// spindle is off then; whether it hears the spindle is checked at the program's first move with it on).
    pub fn guard_ready(&self) -> Result<(), String> {
        self.arm_state().map(|_| ())
    }

    pub fn recording(&self) -> bool {
        self.recording.is_some()
    }

    pub fn listening(&self) -> bool {
        self.source.is_some()
    }

    /// The small live readout in the status bar: what the microphone hears.
    pub fn indicator(&self, ui: &mut egui::Ui) {
        if self.source.is_none() {
            if self.settings.on {
                ui.label(RichText::new("mic: none").size(12.0).color(MUTED)).on_hover_text(self.problem.clone().unwrap_or_default());
            }
            return;
        }
        let chatter = self.chatter_recent.iter().filter(|c| **c).count() >= 3;
        let (text, color) = match &self.last {
            Some(f) if chatter => (format!("mic: CHATTER {:.0} Hz", f.chatter.map(|c| c.0).unwrap_or(0.0)), RED),
            Some(Frame { droop: Some(d), .. }) => (format!("mic: rpm −{:.1}%", d * 100.0), if *d > 0.06 { AMBER } else { GREEN }),
            Some(f) if f.ctx.rpm > 0.0 => ("mic: listening".to_string(), MUTED),
            _ => ("mic: on".to_string(), MUTED),
        };
        ui.label(RichText::new(text).size(13.0).color(color).strong()).on_hover_text("From the microphone: the spindle's speed under load (droop) and chatter. Logged per line next to the program as .cutlog.json.");
        if let Some(why) = &self.held {
            let banner = if why.contains("hasn't heard the spindle") { "FEED HELD: spindle not heard" } else { "FEED HELD: spindle bogging" };
            ui.label(RichText::new(banner).size(13.0).color(RED).strong()).on_hover_text(why);
        }
        if let Some(set) = self.adaptive.1 {
            let (t, c) = if self.adaptive_sent < 0.995 {
                (format!("feed {:.0}% (adaptive)", self.adaptive_sent * 100.0), AMBER)
            } else if self.adaptive_sent > 1.005 {
                (format!("feed {:.0}% (air)", self.adaptive_sent * 100.0), GREEN)
            } else {
                ("adaptive feed".to_string(), MUTED)
            };
            ui.label(RichText::new(t).size(12.0).color(c)).on_hover_text(format!(
                "The program asked for the adaptive feed: it lowers the feed (to {:.0}% at the least) to keep the spindle's sag near {:.1}% over its no-load sag; in air it goes up to {:.0}%",
                set.min * 100.0,
                set.target * 100.0,
                set.max * 100.0
            ));
        }
        let g = self.settings.guard;
        let what = format!(
            "Feed hold when the spindle bogs: over {:.0} rpm of slip (or {:.0}%) for {:.1} s, or still slowing down past {:.0} rpm a second into a cut (Settings -> Listening)",
            g.slip_rpm,
            g.cap * 100.0,
            g.time,
            g.rising_rpm
        );
        let (gtext, gcolor, why) = match self.arm_state() {
            Ok(true) => ("hold armed", GREEN, "the spindle is heard and its no-load speed learned".to_string()),
            Ok(false) => ("hold ready", TEXT, format!("listening ({:.0} dB now); it arms once it has heard the spindle run a few seconds in air", self.last.as_ref().map(|f| f.loud_db).unwrap_or(f64::NEG_INFINITY))),
            Err(_) if !self.settings.guard.on => ("hold off", MUTED, "switched off".to_string()),
            Err(why) => ("hold NOT armed", AMBER, why),
        };
        ui.label(RichText::new(gtext).size(12.0).color(gcolor)).on_hover_text(format!("{what}: {why}"));
        if self.recording.is_some() {
            ui.label(RichText::new("● REC").size(13.0).color(RED).strong()).on_hover_text("The program asked for (RCNC RECORD_AUDIO): its sound is saved as a .wav beside it when it ends");
        }
    }

    /// The Settings page's part.
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Listening (microphone)").size(16.0).strong());
        if SIM_AUDIO.get().is_some() {
            ui.label(RichText::new(format!("Playing {} in place of a microphone (--sim-audio)", SIM_AUDIO.get().unwrap().display())).color(MUTED));
            return;
        }
        let mut on = self.settings.on;
        if ui.checkbox(&mut on, "Listen to the cut: spindle speed under load, chatter, logged per line").changed() {
            self.settings.on = on;
            self.save();
            self.reopen();
        }
        let inputs = listen::inputs();
        let current = self.settings.device.clone().unwrap_or_else(|| "Default input".into());
        let mut picked = None;
        egui::ComboBox::from_label("Input").selected_text(&current).show_ui(ui, |ui| {
            if ui.selectable_label(self.settings.device.is_none(), "Default input").clicked() {
                picked = Some(None);
            }
            for name in &inputs {
                if ui.selectable_label(self.settings.device.as_deref() == Some(name), name).clicked() {
                    picked = Some(Some(name.clone()));
                }
            }
        });
        if let Some(p) = picked {
            self.settings.device = p;
            self.save();
            self.reopen();
        }
        match (&self.source, &self.problem) {
            (Some(s), _) => {
                ui.label(RichText::new(format!("Listening on {} at {} Hz", s.name(), s.rate())).color(GREEN));
            }
            (None, Some(p)) => {
                ui.label(RichText::new(p).color(AMBER));
            }
            _ => {}
        }
        if inputs.is_empty() {
            ui.label(RichText::new("No input devices found (plug the microphone in, then open Settings again).").size(12.0).color(MUTED));
        }
        ui.add_space(6.0);
        let mut g = self.settings.guard;
        ui.checkbox(&mut g.on, "Feed hold when the spindle bogs").on_hover_text(
            "While cutting: when the heard speed is more than the limit under its no-load speed for longer than the time, hold the feed (the spindle stays on; Cycle Start carries on)",
        );
        ui.horizontal(|ui| {
            ui.label("over");
            ui.add(egui::DragValue::new(&mut g.slip_rpm).range(200.0..=4000.0).speed(10.0).suffix(" rpm of slip"));
            ui.label("or");
            let mut pct = g.cap * 100.0;
            ui.add(egui::DragValue::new(&mut pct).range(2.0..=20.0).speed(0.1).suffix(" %"));
            ui.label("for");
            ui.add(egui::DragValue::new(&mut g.time).range(0.1..=2.0).speed(0.01).suffix(" s"));
            g.cap = pct / 100.0;
        });
        if g != self.settings.guard {
            self.settings.guard = g;
            self.guard.settings = g;
            self.save();
        }
    }

    /// The tool change window's tap test: record a second while the operator taps the tool, then show what
    /// it rings at and keep it in the tool table.
    pub fn tap_ui(&mut self, ui: &mut egui::Ui, tool: u32, machine: &ControllerHandle) {
        if self.source.is_none() {
            return;
        }
        ui.add_space(6.0);
        match &self.tap {
            Tap::Idle => {
                if ui.button(RichText::new("Tap test").size(16.0)).on_hover_text("Press, then within a second tap the tool near its tip with something hard (a wrench). Measures how it rings for the chatter model.").clicked() {
                    self.tap = Tap::Recording { tool, until: std::time::Instant::now() + std::time::Duration::from_millis(1500), samples: vec![] };
                }
            }
            Tap::Recording { .. } => {
                ui.label(RichText::new("Tap the tool now…").size(18.0).color(AMBER).strong());
                ui.ctx().request_repaint();
            }
            Tap::Done { tool: t, result } => {
                let t = *t;
                match result.clone() {
                    Ok(r) => {
                        ui.label(RichText::new(format!("T{t} rings at {:.0} Hz, damping {:.3} ({:.0} dB over the noise)", r.hz, r.zeta, r.snr_db)).size(16.0).color(GREEN));
                        ui.horizontal(|ui| {
                            if ui.button("Keep it").clicked() {
                                machine.send(Command::SetToolMode { tool: t, hz: r.hz, zeta: r.zeta });
                                self.tap = Tap::Idle;
                            }
                            if ui.button("Again").clicked() {
                                self.tap = Tap::Idle;
                            }
                        });
                    }
                    Err(e) => {
                        ui.label(RichText::new(e).color(AMBER));
                        if ui.button("Again").clicked() {
                            self.tap = Tap::Idle;
                        }
                    }
                }
            }
        }
    }
}

/// Seconds between analysis windows (half a window: 8192 samples at 32 kHz and up, else 4096).
/// How often a running program's cut log is written out, s.
const LOG_SAVE_S: f64 = 10.0;

fn samples_per_frame_s(rate: u32) -> f64 {
    let n = if rate >= 32_000 { 8192 } else { 4096 };
    (n / 2) as f64 / rate.max(1) as f64
}

/// The hold's state from what it knows: the input (Err why), the hold switched on, the spindle speed and
/// whether its no-load speed is learned there. Ok(true) armed, Ok(false) ready. The level isn't judged here:
/// on the DMC2 a connected transducer with the spindle off reads -143 to -172 dB, the same as no signal at
/// all, so the level can't tell a dead input before the spindle runs; a program that needs the hold is held
/// at its first move with the spindle on instead, if the hold hasn't heard the spindle by then.
pub fn arm(input: Result<(), String>, on: bool, rpm: f64, learned: bool) -> Result<bool, String> {
    input?;
    if !on {
        return Err("the feed hold is switched off".into());
    }
    Ok(rpm > 0.0 && learned)
}

/// Whether to hold the feed before a move: the program needs the hold, the hold is on, the machine is moving
/// with the spindle on, no no-load speed is learned at this speed, and it hasn't already held at this speed.
pub fn holds_unheard(requires: bool, moving: bool, rpm: f64, learned: bool, already: bool, on: bool) -> bool {
    requires && on && moving && rpm > 0.0 && !learned && !already
}

/// A program asks for the listening feed hold with `(RCNC REQUIRE_GUARD)`: Cycle Start refuses without it.
pub fn requires_guard(program: &str) -> bool {
    program.lines().any(|l| l.trim().to_ascii_uppercase().starts_with("(RCNC REQUIRE_GUARD"))
}

/// A program turns the adaptive feed on with `(RCNC ADAPTIVE_FEED TARGET=3 MIN=60 MAX=120)` (sag over no-load,
/// %; the lowest feed and, in air, the highest, % of the programmed; all optional, else the Settings page's;
/// MAX defaults to 100: no speed-up), and off and on again for
/// operations with `(RCNC ADAPTIVE_FEED OFF)` / `(RCNC ADAPTIVE_FEED ON)`: on from its first such line. Its
/// settings (None: not asked for) and, per line, whether it's on there.
pub fn adaptive_lines(program: &str, base: LimiterSettings) -> (Option<LimiterSettings>, Vec<bool>) {
    let mut set: Option<LimiterSettings> = None;
    let mut on = false;
    let lines = program
        .lines()
        .map(|l| {
            let u = l.trim().to_ascii_uppercase();
            if let Some(rest) = u.strip_prefix("(RCNC ADAPTIVE_FEED") {
                let rest = rest.trim_end_matches(')');
                let words: Vec<&str> = rest.split_whitespace().collect();
                if words.first() == Some(&"OFF") {
                    on = false;
                } else {
                    on = true;
                    let s = set.get_or_insert(base);
                    for w in words {
                        let num = |k: &str| w.strip_prefix(k).and_then(|v| v.trim_end_matches('%').parse::<f64>().ok());
                        if let Some(v) = num("TARGET=") {
                            s.target = (v / 100.0).clamp(0.005, 0.05);
                        }
                        if let Some(v) = num("MIN=") {
                            s.min = (v / 100.0).clamp(0.3, 1.0);
                        }
                        // speeding up in air: off unless asked, at most 150%
                        if let Some(v) = num("MAX=") {
                            s.max = (v / 100.0).clamp(1.0, 1.5);
                        }
                    }
                }
            }
            on
        })
        .collect();
    (set, lines)
}

/// A program asks for its run to be recorded with an `(RCNC RECORD_AUDIO)` comment (a calibration program).
pub fn records_audio(program: &str) -> bool {
    program.lines().any(|l| l.trim().to_ascii_uppercase().starts_with("(RCNC RECORD_AUDIO"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn cycle_start_needs_listening_and_the_hold_not_a_running_spindle() {
        // the gauge programs start with the probe, spindle off (the mill PC read -143 to -172 dB then): ready
        assert_eq!(super::arm(Ok(()), true, 0.0, false), Ok(false));
        assert!(super::arm(Err("listening is off".into()), true, 0.0, false).is_err());
        assert!(super::arm(Ok(()), false, 0.0, false).unwrap_err().contains("switched off"));
        // the spindle heard and its no-load speed learned: armed
        assert_eq!(super::arm(Ok(()), true, 24_000.0, true), Ok(true));
        assert_eq!(super::arm(Ok(()), true, 24_000.0, false), Ok(false));
    }

    #[test]
    fn a_move_with_the_spindle_unheard_is_held_once() {
        use super::holds_unheard;
        // spindle on, moving, nothing learned at this speed: hold
        assert!(holds_unheard(true, true, 24_000.0, false, false, true));
        // the normal case: learned in the dwell before the move
        assert!(!holds_unheard(true, true, 24_000.0, true, false, true));
        // once per speed; not while standing; not with the spindle off (the probe moves); not without REQUIRE_GUARD
        assert!(!holds_unheard(true, true, 24_000.0, false, true, true));
        assert!(!holds_unheard(true, false, 24_000.0, false, false, true));
        assert!(!holds_unheard(true, true, 0.0, false, false, true));
        assert!(!holds_unheard(false, true, 24_000.0, false, false, true));
    }

    #[test]
    fn a_program_turns_the_adaptive_feed_on_for_its_roughing() {
        let base = listen::LimiterSettings::default();
        let (set, on) = super::adaptive_lines("G21\n(RCNC ADAPTIVE_FEED TARGET=1.5 MIN=70)\nG1 X1\n(RCNC ADAPTIVE_FEED OFF)\nG1 X2\n(rcnc adaptive_feed on)\nG1 X3", base);
        let set = set.unwrap();
        assert!((set.target - 0.015).abs() < 1e-12 && (set.min - 0.7).abs() < 1e-12);
        assert_eq!(on, vec![false, true, true, false, false, true, true]);
        assert_eq!(super::adaptive_lines("G21\nG1 X1\nM30", base), (None, vec![false, false, false]));
        // nonsense is held to sane bounds
        let (set, _) = super::adaptive_lines("(RCNC ADAPTIVE_FEED TARGET=40 MIN=5 MAX=300)", base);
        assert_eq!((set.unwrap().target, set.unwrap().min, set.unwrap().max), (0.05, 0.3, 1.5));
        // no MAX: no speed-up in air
        assert_eq!(super::adaptive_lines("(RCNC ADAPTIVE_FEED)", base).0.unwrap().max, 1.0);
    }

    #[test]
    fn a_program_asks_to_be_recorded() {
        assert!(super::records_audio("G21\n(rcnc record_audio)\nM30"));
        assert!(super::records_audio("(RCNC RECORD_AUDIO: calibration passes)"));
        assert!(!super::records_audio("G21\n(RECORD_AUDIO)\nM30"));
        assert!(super::requires_guard("(RCNC REQUIRE_GUARD: the feed hold must be armed)\nM30") && !super::requires_guard("M30"));
    }
}
