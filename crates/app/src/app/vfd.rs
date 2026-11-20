//! The spindle VFD's own readings over RS-485 (the `vfd` crate): output frequency, current and voltage in the
//! status bar, a per-run log beside the program (`<program>.vfd.csv`), the raw registers on the Settings page
//! to check the register map against the drive's display, and (opt-in) a feed hold on the output current.
//! Read-only: rcnc never writes to the drive. With no adapter plugged in this shows nothing and costs nothing.

use super::palette::*;
use eframe::egui::{self, RichText};
use gcode::Spindle;
use machine::{Command, ControllerHandle, Mode, Snapshot};
use vfd::{AmpsHold, Line, Reading, Status, VfdLink, VfdSettings};

pub struct SpindleVfd {
    settings: VfdSettings,
    file: std::path::PathBuf,
    link: Option<VfdLink>,
    hold: AmpsHold,
    /// The latest reading (decoded) and its raw registers.
    pub last: Option<Reading>,
    raw: Vec<u16>,
    /// Why the current hold last held the feed, shown until the program moves on again.
    held: Option<String>,
    /// The running program's readings and where they go when it ends.
    log: Option<(std::path::PathBuf, String)>,
    /// The highest current in this run (shown on hover).
    peak: f64,
    /// A port typed on the Settings page.
    port_text: String,
}

impl SpindleVfd {
    pub fn new() -> SpindleVfd {
        let file = super::ears::DATA.get().cloned().unwrap_or_default().join("vfd.json");
        let settings: VfdSettings = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let mut v = SpindleVfd {
            hold: AmpsHold::new(settings.hold),
            port_text: settings.port.clone().unwrap_or_default(),
            settings,
            file,
            link: None,
            last: None,
            raw: vec![],
            held: None,
            log: None,
            peak: 0.0,
        };
        v.restart();
        v
    }

    fn save(&self) {
        if let Ok(s) = serde_json::to_string_pretty(&self.settings) {
            let _ = std::fs::write(&self.file, s);
        }
    }

    fn restart(&mut self) {
        self.link = None; // stops the old thread
        self.last = None;
        self.raw.clear();
        if self.settings.on {
            self.link = Some(VfdLink::start(self.settings.clone()));
        }
    }

    fn status(&self) -> Option<Status> {
        self.link.as_ref().and_then(|l| l.shared.lock().ok().and_then(|s| s.status.clone()))
    }

    pub fn connected(&self) -> bool {
        matches!(self.status(), Some(Status::Connected { .. }))
    }

    /// Each frame: take the new readings, judge the current, log them per line.
    pub fn poll(&mut self, s: &Snapshot, program: Option<&std::path::Path>, machine: &ControllerHandle) {
        let Some(link) = &self.link else { return };
        let fresh = match link.shared.lock() {
            Ok(mut g) => std::mem::take(&mut g.fresh),
            Err(_) => return,
        };
        // the hold is over once the program moves again (Cycle Start) or stops
        if self.held.is_some() && (!s.hold || !matches!(s.mode, Mode::Running)) && s.velocity > 0.0 {
            self.held = None;
        }
        let running = matches!(s.mode, Mode::Running) && s.program_name.is_some();
        let cutting = running && !s.hold && s.spindle != Spindle::Off && s.velocity > 0.0;
        if running && self.log.is_none() {
            if let Some(p) = program {
                let mut path = p.as_os_str().to_owned();
                path.push(if s.air_cut.is_some() { ".air.vfd.csv" } else { ".vfd.csv" });
                self.log = Some((path.into(), String::from(CSV_HEADER)));
                self.peak = 0.0;
            }
        }
        let poles = self.poles();
        for (t, regs) in fresh {
            let r = self.settings.map.decode(&regs);
            if let Some(why) = self.hold.update(t, &r, cutting) {
                if matches!(s.mode, Mode::Running) && !s.hold {
                    machine.send(Command::FeedHold);
                    machine.send(Command::Note(why.clone()));
                    self.held = Some(why);
                }
            }
            if let Some((_, csv)) = &mut self.log {
                let f = |v: Option<f64>, d: usize| v.map(|v| format!("{v:.d$}")).unwrap_or_default();
                csv.push_str(&format!(
                    "{t:.3},{},{},{:.0},{:.0},{:.0},{},{},{},{},{},{},{},{},{}\n",
                    s.line.map(|l| l + 1).unwrap_or(0),
                    s.tool,
                    if s.spindle == Spindle::Off { 0.0 } else { s.rpm },
                    s.feed_rate,
                    s.velocity,
                    f(r.out_hz, 1),
                    f(r.set_hz, 1),
                    f(r.amps, 2),
                    f(r.rpm_from_hz(poles), 0),
                    f(r.ac_volts, 1),
                    f(r.dc_volts, 1),
                    f(r.temp_c, 1),
                    f(r.apparent_va(), 0),
                    regs.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "),
                ));
            }
            self.peak = self.peak.max(r.amps.unwrap_or(0.0));
            self.last = Some(r);
            self.raw = regs;
        }
        if !running && !matches!(s.mode, Mode::Probing) && s.paused.is_none() {
            if let Some((path, csv)) = self.log.take() {
                if csv.len() > CSV_HEADER.len() {
                    let _ = std::fs::write(path, csv);
                }
            }
        }
    }

    /// The speed the drive turns the motor's field at (output frequency x 120 / poles), rpm, while connected.
    pub fn rpm(&self) -> Option<f64> {
        if !self.connected() {
            return None;
        }
        self.last.as_ref()?.rpm_from_hz(self.poles())
    }

    /// Motor poles from the drive (F143), else 2 (the DMC2's spindle).
    fn poles(&self) -> f64 {
        self.link
            .as_ref()
            .and_then(|l| l.shared.lock().ok().and_then(|s| s.params.iter().find(|p| p.0 == 143).and_then(|p| p.1)))
            .filter(|p| *p >= 2)
            .unwrap_or(2) as f64
    }

    /// The status bar: what the drive says it's doing.
    pub fn indicator(&self, ui: &mut egui::Ui) {
        let Some(status) = self.status() else { return };
        match status {
            Status::Connected { .. } => {
                let Some(r) = &self.last else { return };
                let mut text = format!("VFD {:.1} Hz", r.out_hz.unwrap_or(0.0));
                if let Some(a) = r.amps {
                    text.push_str(&format!(" {a:.1} A"));
                }
                if let Some(v) = r.ac_volts {
                    text.push_str(&format!(" {v:.0} V"));
                }
                let over = r.amps.is_some_and(|a| a > self.settings.hold.limit);
                let color = if over { AMBER } else { GREEN };
                ui.label(RichText::new(text).size(13.0).color(color).strong()).on_hover_text(format!(
                    "From the spindle VFD over RS-485: output frequency {} ({:.0} rpm at {} poles), set {}; current, output voltage{}. Peak this run {:.1} A. Logged per line beside the program as .vfd.csv.",
                    r.out_hz.map(|v| format!("{v:.1} Hz")).unwrap_or("-".into()),
                    r.rpm_from_hz(self.poles()).unwrap_or(0.0),
                    self.poles(),
                    r.set_hz.map(|v| format!("{v:.1} Hz")).unwrap_or("-".into()),
                    r.dc_volts.map(|v| format!(", DC bus {v:.0} V")).unwrap_or_default(),
                    self.peak
                ));
                if self.settings.hold.on {
                    ui.label(RichText::new(format!("amps hold {:.1} A", self.settings.hold.limit)).size(12.0).color(GREEN))
                        .on_hover_text("Feed hold when the spindle draws more than this for the set time while cutting (Settings -> Spindle VFD)");
                }
            }
            Status::Lost(why) => {
                ui.label(RichText::new("VFD: lost").size(12.0).color(AMBER)).on_hover_text(why);
            }
            Status::NoAnswer(why) => {
                ui.label(RichText::new("VFD: no answer").size(12.0).color(MUTED)).on_hover_text(why);
            }
            // no adapter, still looking, or off: nothing to say in the status bar
            _ => {}
        }
        if let Some(why) = &self.held {
            ui.label(RichText::new("FEED HELD: spindle current").size(13.0).color(RED).strong()).on_hover_text(why);
        }
    }

    /// The Settings page's part.
    pub fn settings_ui(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Spindle VFD (RS-485)").size(16.0).strong());
        let mut changed = false;
        let mut on = self.settings.on;
        if ui.checkbox(&mut on, "Read the VFD's frequency, current and voltage over a USB-RS485 adapter (read-only)").changed() {
            self.settings.on = on;
            changed = true;
        }
        ui.horizontal(|ui| {
            ui.label("Port");
            let r = ui.add(egui::TextEdit::singleline(&mut self.port_text).hint_text("auto (any USB adapter)").desired_width(140.0));
            if r.lost_focus() {
                let p = self.port_text.trim();
                let p = (!p.is_empty()).then(|| p.to_string());
                if p != self.settings.port {
                    self.settings.port = p;
                    changed = true;
                }
            }
            ui.label("Address (F163)");
            let mut a = self.settings.address as i32;
            if ui.add(egui::DragValue::new(&mut a).range(1..=247)).changed() {
                self.settings.address = a as u8;
                changed = true;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Line");
            let cur = self.settings.line.map(|l| l.to_string()).unwrap_or("auto (try the usual ones)".into());
            let mut pick = None;
            egui::ComboBox::from_id_salt("vfd-line").selected_text(cur).show_ui(ui, |ui| {
                if ui.selectable_label(self.settings.line.is_none(), "auto (try the usual ones)").clicked() {
                    pick = Some(None);
                }
                for l in Line::scan() {
                    if ui.selectable_label(self.settings.line == Some(l), l.to_string()).clicked() {
                        pick = Some(Some(l));
                    }
                }
            });
            if let Some(p) = pick {
                self.settings.line = p;
                changed = true;
            }
            // once found, keep the line so reconnecting is quick
            if let (None, Some(Status::Connected { line, .. })) = (self.settings.line, self.status()) {
                if ui.button(format!("Keep {line}")).on_hover_text("Stop scanning: always use the line the drive answered on").clicked() {
                    self.settings.line = Some(line);
                    changed = true;
                }
            }
        });
        let (text, color) = match self.status() {
            None | Some(Status::Off) => ("Off".to_string(), MUTED),
            Some(Status::NoAdapter) => ("No USB serial adapter found (plug it in; rcnc keeps looking)".into(), MUTED),
            Some(Status::Searching(w)) => (format!("Looking for the drive: {w}"), MUTED),
            Some(Status::NoAnswer(w)) => (format!("No answer: {w}. Check the wiring (swap 485+/485- if unsure), the address (F163) and that the drive is on."), AMBER),
            Some(Status::Lost(w)) => (format!("Lost the drive ({w}); looking again"), AMBER),
            Some(Status::Connected { port, line }) => (format!("Connected on {port} at {line}"), GREEN),
        };
        ui.label(RichText::new(text).color(color));
        if self.connected() {
            let (params, polls, errors) =
                self.link.as_ref().and_then(|l| l.shared.lock().ok().map(|s| (s.params.clone(), s.polls, s.errors))).unwrap_or_default();
            ui.label(
                RichText::new(format!(
                    "Drive parameters (raw): {}   ·   {polls} polls, {errors} errors",
                    params.iter().map(|(n, v)| format!("F{n:03}={}", v.map(|v| v.to_string()).unwrap_or("?".into()))).collect::<Vec<_>>().join("  ")
                ))
                .size(12.0)
                .color(MUTED),
            );
            ui.label(RichText::new("Live registers (check against the drive's display: run the spindle, press the display key to step through Hz / A / rpm / V):").size(12.0));
            let names = self.register_names();
            egui::Grid::new("vfd-regs").striped(true).show(ui, |ui| {
                for (i, v) in self.raw.iter().enumerate() {
                    ui.label(format!("reg {i}"));
                    ui.label(format!("{v}"));
                    ui.label(RichText::new(names.get(i).cloned().unwrap_or_default()).color(MUTED));
                    ui.end_row();
                }
            });
        }
        ui.add_space(4.0);
        let mut h = self.settings.hold;
        ui.checkbox(&mut h.on, "Feed hold on spindle current").on_hover_text(
            "While cutting at speed: when the drive's output current is over the limit for longer than the time, hold the feed (the spindle stays on; Cycle Start carries on). Turn on once the current register is checked against the drive's display.",
        );
        ui.horizontal(|ui| {
            ui.label("over");
            ui.add(egui::DragValue::new(&mut h.limit).range(1.0..=20.0).speed(0.05).suffix(" A"));
            ui.label("for");
            ui.add(egui::DragValue::new(&mut h.time).range(0.1..=2.0).speed(0.01).suffix(" s"));
        });
        if h != self.settings.hold {
            self.settings.hold = h;
            self.hold.settings = h;
            self.save();
        }
        if changed {
            self.save();
            self.restart();
        }
    }

    /// What each raw register is taken as (from the register map in vfd.json).
    fn register_names(&self) -> Vec<String> {
        let m = &self.settings.map;
        let mut names = vec![String::new(); self.raw.len()];
        for (r, what) in [
            (m.out_hz, "output frequency, Hz"),
            (m.set_hz, "set frequency, Hz"),
            (m.amps, "output current, A (unverified)"),
            (m.rpm, "speed, rpm (unverified)"),
            (m.dc_volts, "DC bus, V (unverified)"),
            (m.ac_volts, "output voltage, V (unverified)"),
            (m.temp_c, "temperature, °C (unverified)"),
        ] {
            if let Some(r) = r {
                if let Some(n) = names.get_mut(r.index as usize) {
                    let v = self.raw[r.index as usize] as f64 * r.scale;
                    *n = format!("{what} = {v:.2}");
                }
            }
        }
        names
    }
}

const CSV_HEADER: &str = "t_s,line,tool,prog_rpm,feed_mm_min,velocity_mm_min,out_hz,set_hz,amps,rpm_from_hz,out_volts,dc_volts,temp_c,apparent_va,raw\n";

