//! The Find stock wizard window. It sits at the top right, clear of the
//! position readout, with its own jog buttons: the operator taps the stylus,
//! jogs over the middle of the stock and presses Continue; rcnc does the rest.

use eframe::egui::{self, pos2, Align2, Button, Color32, CornerRadius, FontId, Margin, Rect, RichText, Sense, Stroke, Vec2};
use machine::{CalibrateStage, CalibrateView, Command, FindStockStage, FindStockView};

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
                check(ui, false, "Jog it over the middle of the stock, 2–10 mm above the top");
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

fn trim(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn trim3(v: f64) -> String {
    format!("{v:.3}")
}
