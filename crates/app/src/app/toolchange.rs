//! The guided tool change window, shown while an M6 runs: which tool comes
//! out and which goes in, where the change is up to, and Continue once the
//! new tool is in.

use eframe::egui::{self, Align2, Button, Color32, CornerRadius, FontId, Margin, RichText, Sense, Stroke, Vec2};
use machine::{Command, ToolChangeStage, ToolChangeView};

use super::palette::*;
use super::{big_button, App};

impl App {
    pub(super) fn tool_change_window(&mut self, ctx: &egui::Context) {
        let Some(tc) = self.snap.tool_change.clone() else {
            self.tc_measure = None;
            return;
        };
        let measure = *self.tc_measure.get_or_insert(tc.measure);
        egui::Window::new("Tool change")
            .title_bar(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(440.0, 0.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(PANEL)
                    .stroke(Stroke::new(1.5_f32, ACCENT))
                    .corner_radius(CornerRadius::same(12))
                    .inner_margin(Margin::same(20)),
            )
            .show(ctx, |ui| self.tool_change_body(ui, &tc, measure));
    }

    fn tool_change_body(&mut self, ui: &mut egui::Ui, tc: &ToolChangeView, measure: bool) {
        let loading = tc.stage == ToolChangeStage::Loading;
        ui.label(RichText::new("TOOL CHANGE").size(13.0).color(MUTED).strong());
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            let from = if tc.from == 0 { "T?".to_string() } else { format!("T{}", tc.from) };
            ui.label(RichText::new(from).font(FontId::monospace(28.0)).color(MUTED));
            // The UI font has no arrow glyph.
            let (rect, _) = ui.allocate_exact_size(Vec2::new(36.0, 28.0), Sense::hover());
            let y = rect.center().y + 2.0;
            ui.painter().arrow(
                egui::pos2(rect.left() + 6.0, y),
                Vec2::new(rect.width() - 12.0, 0.0),
                Stroke::new(2.0_f32, MUTED),
            );
            ui.label(RichText::new(format!("T{}", tc.to)).font(FontId::monospace(40.0)).color(ACCENT).strong());
        });
        let tool = self.snap.tools.get(&tc.to);
        let description = tool.map(|t| t.description.trim()).filter(|d| !d.is_empty()).unwrap_or("Not in the tool table");
        ui.label(RichText::new(description).size(17.0).color(TEXT));
        if let Some(t) = tool.filter(|t| t.diameter > 0.0) {
            ui.label(RichText::new(format!("Ø {:.3} mm", t.diameter)).size(14.0).color(MUTED));
        }
        ui.add_space(12.0);

        let order = |s: ToolChangeStage| match s {
            ToolChangeStage::Moving => 0,
            ToolChangeStage::Loading => 1,
            ToolChangeStage::Measuring => 2,
            ToolChangeStage::Returning => 3,
        };
        let now = order(tc.stage);
        let measure_now = if loading { measure } else { tc.measure };
        let mut steps = vec![
            (0, "Stop the spindle and move to the change position".to_string(), true),
            (1, format!("Load T{}", tc.to), true),
            (2, "Measure its length on the tool setter".to_string(), measure_now),
        ];
        if tc.return_to_start {
            steps.push((3, "Go back and carry on with the program".to_string(), true));
        }
        for (i, text, on) in steps {
            step_row(ui, &text, i.cmp(&now), on);
        }
        ui.add_space(12.0);

        if loading {
            egui::Frame::new()
                .fill(AMBER.linear_multiply(0.15))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(
                        RichText::new(format!("Put T{} in the spindle, then press Continue.", tc.to))
                            .color(AMBER)
                            .strong(),
                    );
                    ui.label(RichText::new("The machine moves as soon as you do, so keep your hands clear.").size(13.0));
                });
            ui.add_space(8.0);
            let mut m = measure;
            if ui.checkbox(&mut m, "Measure the tool on the setter").changed() {
                self.tc_measure = Some(m);
            }
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add(big_button("Tool loaded, continue", GREEN).min_size(Vec2::new(260.0, 52.0)))
                    .clicked()
                {
                    self.machine.send(Command::ToolChangeContinue { measure: Some(m) });
                }
                if ui.add(big_button("Cancel", RED).min_size(Vec2::new(120.0, 52.0))).clicked() {
                    self.machine.send(Command::Stop);
                }
            });
        } else {
            ui.horizontal(|ui| {
                ui.spinner();
                let text = match tc.stage {
                    ToolChangeStage::Moving => "Moving to the change position…",
                    ToolChangeStage::Measuring => "Measuring the tool…",
                    _ => "Going back…",
                };
                ui.label(RichText::new(text).size(15.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(Button::new(RichText::new("Stop").color(RED))).clicked() {
                        self.machine.send(Command::Stop);
                    }
                });
            });
        }
    }
}

/// One step of the change: a dot (done, current or to come) and its text.
fn step_row(ui: &mut egui::Ui, text: &str, when: std::cmp::Ordering, on: bool) {
    use std::cmp::Ordering::*;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
        let c = rect.center();
        let p = ui.painter();
        let color = match (on, when) {
            (false, _) => MUTED.linear_multiply(0.5),
            (_, Less) => GREEN,
            (_, Equal) => ACCENT,
            (_, Greater) => MUTED,
        };
        match when {
            Less if on => {
                p.circle_filled(c, 7.0, color);
                p.line_segment([c + Vec2::new(-3.5, 0.0), c + Vec2::new(-1.0, 2.5)], Stroke::new(2.0_f32, Color32::BLACK));
                p.line_segment([c + Vec2::new(-1.0, 2.5), c + Vec2::new(3.5, -2.5)], Stroke::new(2.0_f32, Color32::BLACK));
            }
            Equal if on => {
                p.circle_filled(c, 7.0, color);
            }
            _ => {
                p.circle_stroke(c, 6.0, Stroke::new(1.5_f32, color));
            }
        }
        let mut t = RichText::new(text).size(15.0).color(if when == Equal && on { TEXT } else { color });
        if !on {
            t = t.strikethrough();
        }
        ui.label(t);
    });
}
