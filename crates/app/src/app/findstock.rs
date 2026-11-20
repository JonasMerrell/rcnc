//! The Find stock wizard window. It sits at the top right, clear of the
//! position readout, with its own jog buttons: the operator taps the stylus,
//! jogs over the middle of the stock and presses Continue; rcnc does the rest.

use eframe::egui::{self, pos2, Align2, Button, Color32, CornerRadius, FontId, Margin, Rect, RichText, Sense, Stroke, Vec2};
use machine::{CalibrateStage, CalibrateView, Command, FindStockStage, FindStockView, FindViseStage, FindViseView, Mode, ProbeCalShape, ProbeCalStage, ProbeCalTarget, ProbeCalView, ProbeRetryView};

use super::palette::*;
use super::{big_button, App};

const STEPS: [f64; 4] = [0.1, 1.0, 5.0, 10.0];

impl App {
    pub(super) fn find_stock_window(&mut self, ctx: &egui::Context) {
        let Some(f) = self.snap.find_stock.clone() else { return };
        let border = match f.stage {
            FindStockStage::Mismatch => RED,
            _ => ACCENT,
        };
        egui::Window::new("Find stock")
            .title_bar(false)
            .anchor(Align2::RIGHT_TOP, Vec2::new(-16.0, 64.0))
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(460.0, 0.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(PANEL)
                    .stroke(Stroke::new(2.0_f32, border))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| self.find_stock_body(ui, &f));
    }

    /// A probing step in the program failed: what it measured and why, and what to do. Nothing was set.
    pub(super) fn probe_retry_window(&mut self, ctx: &egui::Context) {
        let Some(r) = self.snap.probe_retry.clone() else {
            self.accept_confirm = false;
            return;
        };
        egui::Window::new("Probe again")
            .title_bar(false)
            .anchor(Align2::RIGHT_TOP, Vec2::new(-16.0, 64.0))
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(500.0, 0.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(PANEL)
                    .stroke(Stroke::new(2.0_f32, RED))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| self.probe_retry_body(ui, &r));
    }

    fn probe_retry_body(&mut self, ui: &mut egui::Ui, r: &ProbeRetryView) {
        let s = self.snap.clone();
        ui.label(RichText::new("PROBING FAILED · PROGRAM HELD").size(16.0).color(MUTED).strong());
        ui.label(RichText::new(&r.name).size(28.0).color(TEXT).strong());
        if let Some(line) = s.line {
            ui.label(RichText::new(format!("Line {}: nothing was set, and the probe went back up", line + 1)).size(15.0).color(MUTED));
        }
        ui.add_space(8.0);
        // "what it measured: why. Nothing was set. hint": one paragraph each
        for (i, part) in r.reason.split(". ").filter(|p| !p.trim().is_empty() && !p.starts_with("Nothing was set")).enumerate() {
            let text = part.trim_end_matches('.');
            let (size, color) = if i == 0 { (17.0, TEXT) } else { (16.0, AMBER) };
            ui.label(RichText::new(format!("{text}.")).size(size).color(color));
        }
        ui.add_space(12.0);
        let ready = s.mode == Mode::Running && s.velocity == 0.0;
        if !ready {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Lifting the probe…").size(16.0).color(ACCENT));
            });
        }
        ui.horizontal(|ui| {
            if ui.add_enabled(ready, big_button("Probe again", GREEN).min_size(Vec2::new(220.0, 56.0))).on_hover_text("Run the same probing again from the top (after cleaning the bores, or moving the probe)").clicked() {
                self.accept_confirm = false;
                self.machine.send(Command::ProbeAgain);
            }
            if ui.add(big_button("Stop program", RED).min_size(Vec2::new(180.0, 56.0))).clicked() {
                self.accept_confirm = false;
                self.machine.send(Command::Stop);
            }
        });
        if let Some(off) = &r.accept {
            ui.add_space(10.0);
            ui.separator();
            if !self.accept_confirm {
                if ui.add_enabled(ready, Button::new(RichText::new("Accept anyway…").size(15.0))).on_hover_text("Use what it measured although it's outside the tolerance").clicked() {
                    self.accept_confirm = true;
                }
            } else {
                ui.label(RichText::new(format!("Out of tolerance: {off}.")).size(16.0).color(AMBER).strong());
                ui.label(
                    RichText::new("If something was in the way, the work offset will be wrong by about that much and the part may not clean up. Accept only if you know why it reads off.")
                        .size(14.0)
                        .color(TEXT),
                );
                ui.horizontal(|ui| {
                    if ui.add_enabled(ready, big_button("Accept and carry on", AMBER).min_size(Vec2::new(220.0, 48.0))).clicked() {
                        self.accept_confirm = false;
                        self.machine.send(Command::ProbeAccept);
                    }
                    if ui.add(Button::new(RichText::new("Back").size(15.0)).min_size(Vec2::new(90.0, 48.0))).clicked() {
                        self.accept_confirm = false;
                    }
                });
            }
        }
    }

    /// The Z reference calibration window, in the same place and style.
    pub(super) fn calibrate_window(&mut self, ctx: &egui::Context) {
        let Some(k) = self.snap.calibrate.clone() else { return };
        egui::Window::new("Calibrate Z reference")
            .title_bar(false)
            .anchor(Align2::RIGHT_TOP, Vec2::new(-16.0, 64.0))
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(460.0, 0.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(PANEL)
                    .stroke(Stroke::new(2.0_f32, ACCENT))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| self.calibrate_body(ui, &k));
    }

    fn calibrate_body(&mut self, ui: &mut egui::Ui, k: &CalibrateView) {
        let s = self.snap.clone();
        ui.label(RichText::new("CALIBRATE Z REFERENCE").size(16.0).color(MUTED).strong());
        ui.label(RichText::new(&k.reference).size(28.0).color(TEXT).strong());
        ui.label(RichText::new(format!("Stored: the setter's top is {:.3} mm above it", k.old)).size(15.0).color(MUTED));
        ui.add_space(8.0);
        let check = |ui: &mut egui::Ui, done: bool, text: &str| {
            let (mark, color) = if done { ("✔", GREEN) } else { ("○", MUTED) };
            ui.horizontal(|ui| {
                ui.label(RichText::new(mark).size(20.0).color(color));
                ui.label(RichText::new(text).size(17.0).color(TEXT));
            });
        };
        let stop = |ui: &mut egui::Ui, app: &mut Self| {
            if ui.add(big_button("Stop", RED).min_size(Vec2::new(150.0, 56.0))).clicked() {
                app.machine.send(Command::Stop);
            }
        };
        match (k.stage, k.gauge) {
            (CalibrateStage::Locate, None) => {
                check(ui, k.tapped, "Tap the stylus with a finger");
                let (text, color) = if s.probe_active { ("ON", AMBER) } else { ("off", MUTED) };
                ui.label(RichText::new(format!("      Probe input: {text}")).size(15.0).color(color).strong());
                check(ui, false, "Jog it over the surface, 2–10 mm above it");
                ui.add_space(6.0);
                self.find_stock_jog(ui);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(k.tapped && !s.probe_active, big_button("Continue", GREEN).min_size(Vec2::new(250.0, 56.0))).clicked() {
                        self.machine.send(Command::CalibrateContinue);
                    }
                    stop(ui, self);
                });
            }
            (CalibrateStage::Locate, Some(g)) => {
                check(ui, false, &format!("Stand the {g} mm gauge block on the surface"));
                check(ui, false, "Jog the tool down in small steps until it just touches the block");
                ui.add_space(6.0);
                self.find_stock_jog(ui);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.add(big_button("It touches", GREEN).min_size(Vec2::new(250.0, 56.0))).clicked() {
                        self.machine.send(Command::CalibrateContinue);
                    }
                    stop(ui, self);
                });
            }
            (CalibrateStage::Probing, _) => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("Touching the surface twice…").size(17.0).color(ACCENT));
                });
                ui.add_space(10.0);
                stop(ui, self);
            }
            (CalibrateStage::Result, _) => {
                let new = k.new.unwrap_or(k.old);
                ui.label(RichText::new(format!("Measured: {new:.3} mm")).size(26.0).color(GREEN).strong());
                ui.label(RichText::new(format!("{:+.3} mm from the stored {:.3}", new - k.old, k.old)).size(17.0).color(TEXT));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.add(big_button("Save", GREEN).min_size(Vec2::new(250.0, 56.0))).clicked() {
                        self.machine.send(Command::CalibrateContinue);
                    }
                    if ui.add(big_button("Cancel", RED).min_size(Vec2::new(150.0, 56.0))).clicked() {
                        self.machine.send(Command::Stop);
                    }
                });
            }
        }
    }

    /// The probe calibration window, in the same place and style.
    pub(super) fn probe_cal_window(&mut self, ctx: &egui::Context) {
        let Some(k) = self.snap.probe_cal.clone() else { return };
        egui::Window::new("Calibrate probe")
            .title_bar(false)
            .anchor(Align2::RIGHT_TOP, Vec2::new(-16.0, 64.0))
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(460.0, 0.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(PANEL)
                    .stroke(Stroke::new(2.0_f32, ACCENT))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| self.probe_cal_body(ui, &k));
    }

    fn probe_cal_body(&mut self, ui: &mut egui::Ui, k: &ProbeCalView) {
        let s = self.snap.clone();
        let shape = match k.target {
            ProbeCalTarget::Shape { shape, .. } => Some(shape),
            ProbeCalTarget::ZReference(_) => None,
        };
        ui.label(RichText::new("CALIBRATE PROBE").size(16.0).color(MUTED).strong());
        ui.label(RichText::new(&k.on).size(26.0).color(TEXT).strong());
        let old = &k.old;
        let stored = match shape {
            Some(_) if old.diameter.iter().all(|d| *d > 0.0) => {
                format!("Stored: ball X {:.3} / Y {:.3} mm (Ø{:.3} nominal)", old.diameter[0], old.diameter[1], k.nominal)
            }
            Some(_) => format!("Stored: not calibrated, the Ø{:.3} nominal ball is used", k.nominal),
            None => format!("Stored Z offset: {:+.3} mm", old.z_offset),
        };
        ui.label(RichText::new(stored).size(15.0).color(MUTED));
        if let Some(shape) = shape {
            ui.add_space(4.0);
            shape_sketch(ui, shape);
        }
        ui.add_space(8.0);
        let check = |ui: &mut egui::Ui, done: bool, text: &str| {
            let (mark, color) = if done { ("✔", GREEN) } else { ("○", MUTED) };
            ui.horizontal(|ui| {
                ui.label(RichText::new(mark).size(20.0).color(color));
                ui.label(RichText::new(text).size(17.0).color(TEXT));
            });
        };
        let stop = |ui: &mut egui::Ui, app: &mut Self, label: &str| {
            if ui.add(big_button(label, RED).min_size(Vec2::new(150.0, 56.0))).clicked() {
                app.machine.send(Command::Stop);
            }
        };
        match k.stage {
            ProbeCalStage::Locate => {
                check(ui, k.tapped, "Tap the stylus with a finger");
                let (text, color) = if s.probe_active { ("ON", AMBER) } else { ("off", MUTED) };
                ui.label(RichText::new(format!("      Probe input: {text}")).size(15.0).color(color).strong());
                let place = match shape {
                    Some(sh) if sh.inside() => "Jog the ball into the middle, about 3 mm below the top",
                    Some(_) => "Jog it over the middle, 2–10 mm above the top",
                    None => "Jog it over the surface, 2–10 mm above it",
                };
                check(ui, false, place);
                if shape.is_none() {
                    ui.label(RichText::new("Use a reference you trust: one calibrated with a gauge block and a real tool.").size(13.0).color(MUTED));
                }
                ui.add_space(6.0);
                self.find_stock_jog(ui);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(k.tapped && !s.probe_active, big_button("Continue", GREEN).min_size(Vec2::new(250.0, 56.0))).clicked() {
                        self.machine.send(Command::CalibrateProbeContinue);
                    }
                    stop(ui, self, "Stop");
                });
            }
            ProbeCalStage::Turn => {
                check(ui, true, "First measurement done");
                check(ui, false, "Spindle off: turn the probe half a turn in the spindle by hand");
                check(ui, false, "Leave the machine where it is");
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!s.probe_active, big_button("Measure again", GREEN).min_size(Vec2::new(250.0, 56.0))).clicked() {
                        self.machine.send(Command::CalibrateProbeContinue);
                    }
                    stop(ui, self, "Cancel");
                });
            }
            ProbeCalStage::Probing => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    let what = if shape.is_some() { "Centring, then touching X and Y both ways twice…" } else { "Touching the surface twice…" };
                    ui.label(RichText::new(what).size(17.0).color(ACCENT));
                });
                ui.add_space(10.0);
                stop(ui, self, "Stop");
            }
            ProbeCalStage::Result => {
                let new = k.new.clone().unwrap_or_default();
                let row = |ui: &mut egui::Ui, name: &str, old: String, new: String, delta: String| {
                    ui.label(RichText::new(name).size(15.0).color(MUTED));
                    ui.label(RichText::new(old).size(17.0).color(TEXT));
                    ui.label(RichText::new(new).size(19.0).color(GREEN).strong());
                    ui.label(RichText::new(delta).size(15.0).color(MUTED));
                    ui.end_row();
                };
                egui::Grid::new("probe-cal-result").num_columns(4).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("");
                    ui.label(RichText::new("stored").size(13.0).color(MUTED));
                    ui.label(RichText::new("measured").size(13.0).color(MUTED));
                    ui.label(RichText::new("change").size(13.0).color(MUTED));
                    ui.end_row();
                    match shape {
                        Some(_) => {
                            for (i, axis) in ["Ball X", "Ball Y"].iter().enumerate() {
                                let was = if old.diameter[i] > 0.0 { old.diameter[i] } else { k.nominal };
                                let tag = if old.diameter[i] > 0.0 { "" } else { " nom." };
                                row(ui, axis, format!("{was:.3}{tag}"), format!("{:.3}", new.diameter[i]), format!("{:+.3}", new.diameter[i] - was));
                            }
                            if let Some(r) = k.runout {
                                let was = k.old_runout.map_or("—".to_string(), |o| format!("{:+.3} / {:+.3}", o[0], o[1]));
                                row(ui, "Off centre X / Y", was, format!("{:+.3} / {:+.3}", r[0], r[1]), format!("TIR {:.3}", 2.0 * r[0].hypot(r[1])));
                            }
                        }
                        None => row(ui, "Z offset", format!("{:+.3}", old.z_offset), format!("{:+.3}", new.z_offset), format!("{:+.3}", new.z_offset - old.z_offset)),
                    }
                });
                if let Some(r) = k.repeat {
                    let color = if r <= s.config.probing.setter.repeat_tolerance { MUTED } else { AMBER };
                    ui.label(RichText::new(format!("Repeat: the same span twice was {r:.3} mm apart")).size(14.0).color(color));
                }
                if k.runout.is_some() {
                    ui.label(RichText::new("The off-centre offset is used until the probe comes out of the spindle.").size(13.0).color(MUTED));
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.add(big_button("Save", GREEN).min_size(Vec2::new(170.0, 56.0))).clicked() {
                        self.machine.send(Command::CalibrateProbeContinue);
                    }
                    if shape.is_some() && k.centres.len() < 2 {
                        let turn = ui.add(big_button("Turn probe", ACCENT).min_size(Vec2::new(120.0, 56.0)));
                        if turn.on_hover_text("Turn the probe half a turn and measure again, to find how far its ball is off centre").clicked() {
                            self.machine.send(Command::CalibrateProbeTurn);
                        }
                    }
                    stop(ui, self, "Cancel");
                });
            }
        }
    }

    /// The Find vise location window, in the same place and style.
    pub(super) fn find_vise_window(&mut self, ctx: &egui::Context) {
        let Some(v) = self.snap.find_vise.clone() else { return };
        egui::Window::new("Find vise location")
            .title_bar(false)
            .anchor(Align2::RIGHT_TOP, Vec2::new(-16.0, 64.0))
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(500.0, 0.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(PANEL)
                    .stroke(Stroke::new(2.0_f32, ACCENT))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| self.find_vise_body(ui, &v));
    }

    fn find_vise_body(&mut self, ui: &mut egui::Ui, v: &FindViseView) {
        let s = self.snap.clone();
        ui.label(RichText::new("FIND VISE LOCATION").size(16.0).color(MUTED).strong());
        ui.label(RichText::new("Fixed centre block").size(26.0).color(TEXT).strong());
        let stored = match &v.old {
            Some(o) => format!("Stored: jaws at X{:.3} / X{:.3}, middle Y{:.3}", o.faces[0], o.faces[1], o.y),
            None => "Stored: none yet".to_string(),
        };
        ui.label(RichText::new(stored).size(15.0).color(MUTED));
        ui.add_space(8.0);
        let check = |ui: &mut egui::Ui, done: bool, text: &str| {
            let (mark, color) = if done { ("✔", GREEN) } else { ("○", MUTED) };
            ui.horizontal(|ui| {
                ui.label(RichText::new(mark).size(20.0).color(color));
                ui.label(RichText::new(text).size(17.0).color(TEXT));
            });
        };
        let stop = |ui: &mut egui::Ui, app: &mut Self, label: &str| {
            if ui.add(big_button(label, RED).min_size(Vec2::new(150.0, 56.0))).clicked() {
                app.machine.send(Command::Stop);
            }
        };
        match v.stage {
            FindViseStage::Locate => {
                check(ui, false, "Empty the vise (or open both stations wide) and take the work stops off");
                check(ui, v.tapped, "Tap the stylus with a finger");
                let (text, color) = if s.probe_active { ("ON", AMBER) } else { ("off", MUTED) };
                ui.label(RichText::new(format!("      Probe input: {text}")).size(15.0).color(color).strong());
                check(ui, false, "Jog it over the middle of the centre block, between the two jaw bars, 2–10 mm above the web");
                ui.add_space(6.0);
                self.find_stock_jog(ui);
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(v.tapped && !s.probe_active, big_button("Continue", GREEN).min_size(Vec2::new(250.0, 56.0))).clicked() {
                        self.machine.send(Command::FindViseContinue);
                    }
                    stop(ui, self, "Stop");
                });
            }
            FindViseStage::Probing => {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("Touching the web, a jaw top, the jaw ends, then both jaw faces twice…").size(17.0).color(ACCENT));
                });
                ui.add_space(10.0);
                stop(ui, self, "Stop");
            }
            FindViseStage::Result => {
                let new = v.new.unwrap_or(machine::ViseLocation { faces: [0.0; 2], y: 0.0, angle: 0.0, setter_above_top: 0.0, gap: 0.0, length: None, web_below_top: 0.0 });
                let row = |ui: &mut egui::Ui, name: &str, old: String, new: String| {
                    ui.label(RichText::new(name).size(15.0).color(MUTED));
                    ui.label(RichText::new(old).size(17.0).color(TEXT));
                    ui.label(RichText::new(new).size(19.0).color(GREEN).strong());
                    ui.end_row();
                };
                let was = |f: &dyn Fn(&machine::ViseLocation) -> f64| v.old.as_ref().map_or("—".to_string(), |o| format!("{:.3}", f(o)));
                egui::Grid::new("find-vise-result").num_columns(3).spacing([14.0, 6.0]).show(ui, |ui| {
                    ui.label("");
                    ui.label(RichText::new("stored").size(13.0).color(MUTED));
                    ui.label(RichText::new("measured").size(13.0).color(MUTED));
                    ui.end_row();
                    row(ui, "Left station jaw X", was(&|o| o.faces[0]), format!("{:.3}", new.faces[0]));
                    row(ui, "Right station jaw X", was(&|o| o.faces[1]), format!("{:.3}", new.faces[1]));
                    row(ui, "Between the faces", was(&|o| o.gap), format!("{:.3}", new.gap));
                    row(ui, "Middle Y", was(&|o| o.y), format!("{:.3}", new.y));
                    row(ui, "Turned, °", was(&|o| o.angle), format!("{:.4}", new.angle));
                    row(ui, "Jaw top below setter", was(&|o| o.setter_above_top), format!("{:.3}", new.setter_above_top));
                    row(ui, "Jaw top above ledges", was(&|o| o.web_below_top), format!("{:.3}", new.web_below_top));
                });
                for w in &v.warnings {
                    ui.label(RichText::new(w).size(14.0).color(AMBER));
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.add(big_button("Save", GREEN).min_size(Vec2::new(250.0, 56.0))).clicked() {
                        self.machine.send(Command::FindViseContinue);
                    }
                    stop(ui, self, "Cancel");
                });
            }
        }
    }

    fn find_stock_body(&mut self, ui: &mut egui::Ui, f: &FindStockView) {
        let s = self.snap.clone();
        let [sx, sy, sz] = f.request.size;
        ui.label(RichText::new("FIND STOCK").size(16.0).color(MUTED).strong());
        ui.label(RichText::new(format!("{} × {} × {} mm", trim(sx), trim(sy), trim(sz))).size(28.0).color(TEXT).strong());
        ui.add_space(6.0);
        stock_sketch(ui, f);
        ui.add_space(8.0);
        let check = |ui: &mut egui::Ui, done: bool, text: &str| {
            let (mark, color) = if done { ("✔", GREEN) } else { ("○", MUTED) };
            ui.horizontal(|ui| {
                ui.label(RichText::new(mark).size(20.0).color(color));
                ui.label(RichText::new(text).size(17.0).color(TEXT));
            });
        };
        match f.stage {
            FindStockStage::Locate | FindStockStage::Mismatch => {
                if let Some(p) = &f.problem {
                    ui.label(RichText::new(p).size(16.0).color(RED));
                    ui.add_space(6.0);
                }
                check(ui, f.tapped, "Tap the stylus with a finger");
                // Always shown, so the buttons below never move under a finger.
                let (text, color) = if s.probe_active { ("ON", AMBER) } else { ("off", MUTED) };
                ui.label(RichText::new(format!("      Probe input: {text}")).size(15.0).color(color).strong());
                let from_vise = f.request.vise.is_some() && s.config.probing.vise.location.is_some();
                if from_vise && !f.positioned {
                    check(ui, false, "Press Continue: rcnc moves over the stock from the saved vise location and probes it");
                } else {
                    check(ui, f.positioned, "Jog it over the middle of the stock, 2–10 mm above the top");
                }
                if let (Some(side), true) = (f.request.vise, from_vise) {
                    let station = if side < 0.0 { "right" } else { "left" };
                    ui.horizontal(|ui| {
                        let go = ui.add_enabled(f.stage == FindStockStage::Locate && !s.probe_active, Button::new(RichText::new("Over the stock").size(16.0)).min_size(Vec2::new(160.0, 40.0)));
                        if go.on_hover_text("Move over where the stock should be, from the saved vise location, and stop there to fine-tune before Continue").clicked() {
                            self.machine.send(Command::FindStockOverStock);
                        }
                        ui.label(RichText::new(format!("X from the {station} station's fixed jaw (saved)")).size(14.0).color(MUTED));
                    });
                }
                ui.add_space(6.0);
                self.find_stock_jog(ui);
                ui.add_space(8.0);
                // from the program; changed here, they hold for this Find stock only
                let (mut depth, mut tolerance) = (f.depth, f.tolerance);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Touch the sides").size(15.0).color(TEXT));
                    let d = ui.add(egui::DragValue::new(&mut depth).range(0.5..=60.0).speed(0.1).fixed_decimals(1).suffix(" mm"));
                    ui.label(RichText::new("below the top, size within ±").size(15.0).color(TEXT));
                    let t = ui.add(egui::DragValue::new(&mut tolerance).range(0.1..=20.0).speed(0.05).fixed_decimals(1).suffix(" mm"));
                    if d.changed() || t.changed() {
                        self.machine.send(Command::FindStockSettings { depth, tolerance });
                    }
                });
                ui.add_space(8.0);
                let ready = f.tapped && !s.probe_active;
                let label = if f.stage == FindStockStage::Mismatch { "Probe again" } else { "Continue" };
                ui.horizontal(|ui| {
                    let go = ui.add_enabled(ready, big_button(label, GREEN).min_size(Vec2::new(250.0, 56.0)));
                    if go.clicked() {
                        self.machine.send(Command::FindStockContinue);
                    }
                    if ui.add(big_button("Stop", RED).min_size(Vec2::new(150.0, 56.0))).clicked() {
                        self.machine.send(Command::Stop);
                    }
                });
                if !f.tapped {
                    ui.label(RichText::new("Continue unlocks once rcnc has seen the probe input switch on.").size(13.0).color(MUTED));
                }
            }
            FindStockStage::Probing => {
                check(ui, true, "Probe works");
                check(ui, true, "Over the stock");
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("Probing the top, then X and Y sides…").size(17.0).color(ACCENT));
                });
                ui.add_space(10.0);
                if ui.add(big_button("Stop", RED).min_size(Vec2::new(150.0, 48.0))).clicked() {
                    self.machine.send(Command::Stop);
                }
            }
        }
    }

    fn find_stock_jog(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Step").color(MUTED));
            for st in STEPS {
                let on = (self.fs_step - st).abs() < 1e-9;
                if ui.selectable_label(on, RichText::new(format!("{} mm", trim(st))).size(15.0)).clicked() {
                    self.fs_step = st;
                }
            }
        });
        let b = Vec2::new(64.0, 44.0);
        let step = self.fs_step;
        let jog = |ui: &mut egui::Ui, label: &str, axis: usize, positive: bool| {
            if ui.add(Button::new(RichText::new(label).size(18.0).strong()).min_size(b)).clicked() {
                self.machine.send(Command::Jog { axis, positive, step: Some(step) });
            }
        };
        egui::Grid::new("fs-jog").spacing([6.0, 6.0]).show(ui, |ui| {
            ui.label("");
            jog(ui, "Y+", 1, true);
            ui.label("");
            ui.add_space(12.0);
            jog(ui, "Z+", 2, true);
            ui.end_row();
            jog(ui, "X−", 0, false);
            ui.label("");
            jog(ui, "X+", 0, true);
            ui.add_space(12.0);
            ui.label("");
            ui.end_row();
            ui.label("");
            jog(ui, "Y−", 1, false);
            ui.label("");
            ui.add_space(12.0);
            jog(ui, "Z−", 2, false);
            ui.end_row();
        });
        ui.label(RichText::new("A jog that touches the stock stops on the probe.").size(13.0).color(MUTED));
    }
}

/// Top view of the stock with where the probe touches, and the measured size once known.
fn stock_sketch(ui: &mut egui::Ui, f: &FindStockView) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 130.0), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, CornerRadius::same(8), RAISED);
    let [sx, sy, _] = f.request.size;
    let scale = ((rect.width() - 90.0) / sx as f32).min((rect.height() - 40.0) / sy as f32);
    let half = Vec2::new(sx as f32 * scale / 2.0, sy as f32 * scale / 2.0);
    let c = rect.center();
    let stock = Rect::from_center_size(c, half * 2.0);
    p.rect_filled(stock, CornerRadius::same(2), Color32::from_rgb(0x9a, 0xa3, 0xad));
    p.line_segment([c - Vec2::new(8.0, 0.0), c + Vec2::new(8.0, 0.0)], Stroke::new(2.0_f32, ACCENT));
    p.line_segment([c - Vec2::new(0.0, 8.0), c + Vec2::new(0.0, 8.0)], Stroke::new(2.0_f32, ACCENT));
    let arrow = Stroke::new(2.0_f32, AMBER);
    for (from, to) in [
        (pos2(stock.left() - 26.0, c.y), pos2(stock.left() - 4.0, c.y)),
        (pos2(stock.right() + 26.0, c.y), pos2(stock.right() + 4.0, c.y)),
        (pos2(c.x, stock.top() - 18.0), pos2(c.x, stock.top() - 3.0)),
        (pos2(c.x, stock.bottom() + 18.0), pos2(c.x, stock.bottom() + 3.0)),
    ] {
        p.arrow(from, to - from, arrow);
    }
    if let Some(found) = f.found {
        let ok = f.problem.is_none();
        let text = format!("measured {} × {}", trim3(found.size[0]), trim3(found.size[1]));
        p.text(rect.left_bottom() + Vec2::new(8.0, -6.0), Align2::LEFT_BOTTOM, text, FontId::proportional(15.0), if ok { GREEN } else { RED });
    }
    p.text(rect.left_top() + Vec2::new(8.0, 6.0), Align2::LEFT_TOP, "top view, +X right, +Y up", FontId::proportional(12.0), MUTED);
}

/// Top view of a calibration shape, with where the ball touches it.
fn shape_sketch(ui: &mut egui::Ui, shape: ProbeCalShape) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 110.0), Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, CornerRadius::same(8), RAISED);
    let c = rect.center();
    let metal = Color32::from_rgb(0x9a, 0xa3, 0xad);
    let (hw, hh) = if shape.round() { (38.0, 38.0) } else { (58.0, 34.0) };
    let inside = shape.inside();
    if inside {
        // a block of metal with the bore or pocket cut in it
        p.rect_filled(Rect::from_center_size(c, Vec2::new(2.0 * hw + 50.0, 2.0 * hh + 24.0)), CornerRadius::same(2), metal);
    }
    let fill = if inside { RAISED } else { metal };
    if shape.round() {
        p.circle_filled(c, hw, fill);
    } else {
        p.rect_filled(Rect::from_center_size(c, Vec2::new(2.0 * hw, 2.0 * hh)), CornerRadius::same(2), fill);
    }
    let arrow = Stroke::new(2.0_f32, AMBER);
    let d = if inside { -1.0 } else { 1.0 };
    for (dir, half) in [(Vec2::new(1.0, 0.0), hw), (Vec2::new(-1.0, 0.0), hw), (Vec2::new(0.0, 1.0), hh), (Vec2::new(0.0, -1.0), hh)] {
        let edge = c + dir * (half + 3.0 * d);
        let from = c + dir * (half + 20.0 * d);
        p.arrow(from, edge - from, arrow);
    }
    p.text(rect.left_top() + Vec2::new(8.0, 6.0), Align2::LEFT_TOP, "top view", FontId::proportional(12.0), MUTED);
}

fn trim(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn trim3(v: f64) -> String {
    format!("{v:.3}")
}
