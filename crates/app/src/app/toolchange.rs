//! The guided tool change window, shown while an M6 runs. It fills most of
//! the screen so it reads from across the machine: a big picture of the tool
//! to load (its photo, or a drawing from the tool table), its number and
//! size, where the change is up to, and one big Continue button.

use std::path::PathBuf;

use eframe::egui::{
    self, pos2, Align2, Button, Color32, CornerRadius, FontId, Margin, Pos2, Rect, RichText, Sense, Shape, Stroke, Vec2,
};
use gcode::{Tool, ToolKind};
use machine::{Command, ToolChangeStage, ToolChangeView};

use super::palette::*;
use super::App;

impl App {
    pub(super) fn tool_change_window(&mut self, ctx: &egui::Context) {
        let Some(tc) = self.snap.tool_change.clone() else {
            self.tc_measure = None;
            self.tc_photo = None;
            return;
        };
        let measure = *self.tc_measure.get_or_insert(tc.measure);
        let screen = ctx.content_rect();
        let size = Vec2::new((screen.width() * 0.92).min(1500.0), (screen.height() * 0.88).min(1000.0));
        egui::Window::new("Tool change")
            .title_bar(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .collapsible(false)
            .resizable(false)
            .fixed_size(size - Vec2::splat(48.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(PANEL)
                    .stroke(Stroke::new(2.0_f32, if tc.stage == ToolChangeStage::Mismatch { RED } else { ACCENT }))
                    .corner_radius(CornerRadius::same(14))
                    .inner_margin(Margin::same(24)),
            )
            .show(ctx, |ui| self.tool_change_body(ui, &tc, measure));
    }

    fn tool_change_body(&mut self, ui: &mut egui::Ui, tc: &ToolChangeView, measure: bool) {
        let tool = self.snap.tools.get(&tc.to).cloned();
        let photo = self.tool_photo(tc.to, tool.as_ref());
        let full = ui.available_size();
        ui.horizontal_top(|ui| {
            // The picture: as big as the window allows.
            let pic = Vec2::new((full.x * 0.48).max(200.0), full.y);
            let (rect, _) = ui.allocate_exact_size(pic, Sense::hover());
            ui.painter().rect_filled(rect, CornerRadius::same(10), RAISED);
            let inner = rect.shrink(16.0);
            match &photo {
                Some(path) => {
                    let uri = format!("file://{}", path.display());
                    ui.put(inner, egui::Image::new(uri).fit_to_exact_size(inner.size()).maintain_aspect_ratio(true));
                }
                None => draw_tool(ui.painter(), inner, tool.as_ref()),
            }
            ui.add_space(24.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width());
                self.tool_change_panel(ui, tc, tool.as_ref(), measure)
            });
        });
    }

    fn tool_change_panel(&mut self, ui: &mut egui::Ui, tc: &ToolChangeView, tool: Option<&Tool>, measure: bool) {
        let loading = tc.stage == ToolChangeStage::Loading;
        let from = if tc.from == 0 { "T?".to_string() } else { format!("T{}", tc.from) };
        ui.label(RichText::new(format!("TOOL CHANGE   ·   out: {from}")).size(16.0).color(MUTED).strong());
        ui.label(RichText::new(format!("T{}", tc.to)).font(FontId::monospace(110.0)).color(ACCENT).strong());
        let description = tool.map(|t| t.description.trim()).filter(|d| !d.is_empty()).unwrap_or("Not in the tool table");
        ui.label(RichText::new(description).size(30.0).color(TEXT).strong());
        if let Some(t) = tool.filter(|t| t.diameter > 0.0) {
            let mut size = format!("Ø {} mm", trim(t.diameter));
            if t.flute_length > 0.0 {
                size += &format!("   ·   flutes {} mm", trim(t.flute_length));
            }
            ui.label(RichText::new(size).size(24.0).color(TEXT));
        }
        // the stick-out the program (and its simulation) assumed: set it when loading the tool
        let stick = tool.map(|t| t.stickout).filter(|s| *s > 0.0);
        let (text, color) = match stick {
            Some(s) => (format!("Stick-out {} mm", trim(s)), AMBER),
            None => ("Stick-out not given".to_string(), MUTED),
        };
        ui.label(RichText::new(text).size(30.0).color(color).strong());
        if let Some(m) = tc.measured_stickout {
            let ok = stick.is_none_or(|s| (m - s).abs() <= self.snap.config.tool_change.stickout_tolerance);
            ui.label(RichText::new(format!("measured {:.1} mm", m)).size(20.0).color(if ok { GREEN } else { RED }));
        } else if stick.is_some() {
            ui.label(RichText::new("from the collet nut's face to the tip").size(15.0).color(MUTED));
        }
        ui.add_space(16.0);

        let order = |s: ToolChangeStage| match s {
            ToolChangeStage::Moving => 0,
            ToolChangeStage::Loading => 1,
            ToolChangeStage::Measuring | ToolChangeStage::Mismatch => 2,
            ToolChangeStage::Returning => 3,
        };
        let now = order(tc.stage);
        let measure_now = if loading { measure } else { tc.measure };
        let mut steps = vec![
            (0, "Stop the spindle, raise Z, move the bed back".to_string(), true),
            (1, format!("Load T{}", tc.to), true),
            (2, "Measure it twice on the tool setter".to_string(), measure_now),
        ];
        if tc.return_to_start {
            steps.push((3, "Go back and carry on with the program".to_string(), true));
        }
        for (i, text, on) in steps {
            step_row(ui, &text, i.cmp(&now), on);
        }
        ui.add_space(16.0);

        let wide = Vec2::new(ui.available_width(), 120.0);
        match tc.stage {
            ToolChangeStage::Loading => {
                notice(ui, AMBER, &format!("Put T{} in the spindle and tighten it, then press Continue.", tc.to),
                    "The machine moves to the tool setter as soon as you do: keep your hands and the wrenches clear.");
                ui.add_space(10.0);
                let mut m = measure;
                if ui.checkbox(&mut m, RichText::new("Measure the tool on the setter").size(18.0)).changed() {
                    self.tc_measure = Some(m);
                }
                ui.add_space(10.0);
                if ui.add(huge_button("Continue", GREEN).min_size(wide)).clicked() {
                    self.machine.send(Command::ToolChangeContinue { measure: Some(m) });
                }
                ui.label(RichText::new("Cycle Start does the same.").size(14.0).color(MUTED));
                ui.add_space(8.0);
                if ui.add(cancel_button().min_size(Vec2::new(180.0, 44.0))).clicked() {
                    self.machine.send(Command::Stop);
                }
            }
            ToolChangeStage::Mismatch => {
                let problem = tc.problem.as_deref().unwrap_or("The two tool length readings disagree.");
                notice(ui, RED, problem, "The new length hasn't been used. Fix what's wrong, then measure again.");
                ui.add_space(14.0);
                if ui.add(huge_button("Measure again", AMBER).min_size(wide)).clicked() {
                    self.machine.send(Command::ToolChangeContinue { measure: None });
                }
                ui.label(RichText::new("Cycle Start does the same.").size(14.0).color(MUTED));
                ui.add_space(8.0);
                if ui.add(cancel_button().min_size(Vec2::new(180.0, 44.0))).clicked() {
                    self.machine.send(Command::Stop);
                }
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(28.0));
                    let text = match tc.stage {
                        ToolChangeStage::Moving => "Raising Z and moving the bed back…",
                        ToolChangeStage::Measuring => "Measuring the tool…",
                        _ => "Going back…",
                    };
                    ui.label(RichText::new(text).size(24.0));
                });
                ui.add_space(12.0);
                if ui.add(cancel_button().min_size(Vec2::new(180.0, 52.0))).clicked() {
                    self.machine.send(Command::Stop);
                }
            }
        }
    }

    /// The photo to show for tool `n`: its own, or `tools/T<n>.png` (or .jpg)
    /// in the settings folder. Looked up once per tool change.
    fn tool_photo(&mut self, n: u32, tool: Option<&Tool>) -> Option<PathBuf> {
        let wanted = tool.map(|t| t.image.clone()).unwrap_or_default();
        if let Some((for_tool, for_image, path)) = &self.tc_photo {
            if *for_tool == n && *for_image == wanted {
                return path.clone();
            }
        }
        let dir = machine::store::data_dir();
        let path = if !wanted.trim().is_empty() {
            Some(dir.join(wanted.trim())).filter(|p| p.is_file())
        } else {
            ["png", "jpg", "jpeg"].iter().map(|e| dir.join("tools").join(format!("T{n}.{e}"))).find(|p| p.is_file())
        };
        self.tc_photo = Some((n, wanted, path.clone()));
        path
    }
}

fn trim(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn huge_button(text: &str, color: Color32) -> Button<'static> {
    Button::new(RichText::new(text.to_owned()).size(40.0).strong().color(Color32::WHITE))
        .fill(color.linear_multiply(0.85))
        .stroke(Stroke::new(2.0_f32, color))
        .corner_radius(CornerRadius::same(12))
}

fn cancel_button() -> Button<'static> {
    Button::new(RichText::new("Cancel").size(18.0).color(RED))
        .fill(RAISED)
        .stroke(Stroke::new(1.0_f32, RED))
        .corner_radius(CornerRadius::same(8))
}

fn notice(ui: &mut egui::Ui, color: Color32, title: &str, detail: &str) {
    egui::Frame::new()
        .fill(color.linear_multiply(0.15))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(16, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).size(22.0).color(color).strong());
            ui.label(RichText::new(detail).size(16.0));
        });
}

/// One step of the change: a dot (done, current or to come) and its text.
fn step_row(ui: &mut egui::Ui, text: &str, when: std::cmp::Ordering, on: bool) {
    use std::cmp::Ordering::*;
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(26.0), Sense::hover());
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
                p.circle_filled(c, 10.0, color);
                p.line_segment([c + Vec2::new(-5.0, 0.0), c + Vec2::new(-1.5, 3.5)], Stroke::new(2.5_f32, Color32::BLACK));
                p.line_segment([c + Vec2::new(-1.5, 3.5), c + Vec2::new(5.0, -3.5)], Stroke::new(2.5_f32, Color32::BLACK));
            }
            Equal if on => {
                p.circle_filled(c, 10.0, color);
            }
            _ => {
                p.circle_stroke(c, 9.0, Stroke::new(2.0_f32, color));
            }
        }
        let mut t = RichText::new(text).size(20.0).color(if when == Equal && on { TEXT } else { color });
        if !on {
            t = t.strikethrough();
        }
        ui.label(t);
    });
}

const STEEL: Color32 = Color32::from_rgb(0xb4, 0xba, 0xc4);
const STEEL_DARK: Color32 = Color32::from_rgb(0x7c, 0x83, 0x8f);
const NUT: Color32 = Color32::from_rgb(0x5a, 0x60, 0x6b);

/// Draw the tool to scale, tip down, hanging from a collet nut, with its
/// diameter marked. Unknown sizes get typical proportions.
fn draw_tool(p: &egui::Painter, rect: Rect, tool: Option<&Tool>) {
    let Some(tool) = tool else {
        p.text(rect.center(), Align2::CENTER_CENTER, "?", FontId::proportional(160.0), MUTED);
        return;
    };
    let kind = tool.shape();
    let d = if tool.diameter > 0.0 { tool.diameter } else { 6.0 };
    let shank = if kind == ToolKind::Probe { 6.0 } else if d < 3.0 { 3.175 } else { d };
    let flutes = if tool.flute_length > 0.0 {
        tool.flute_length
    } else {
        match kind {
            ToolKind::Drill => 6.0 * d,
            ToolKind::Chamfer => d / 2.0,
            ToolKind::Probe => 20.0,
            _ => 3.0 * d,
        }
    };
    let stickout = if tool.stickout > flutes { tool.stickout } else { flutes + (1.5 * shank).max(8.0) };
    // An ER16-sized nut above the tool, with a bit of spindle over it.
    let (nut_d, nut_h, spindle_h) = (28.0, 14.0, 10.0);
    let total = spindle_h + nut_h + stickout;
    let label_room = 70.0;
    let scale = ((rect.height() - label_room) / total as f32).min(rect.width() * 0.7 / nut_d as f32);
    let mm = |v: f64| v as f32 * scale;
    let cx = rect.center().x;
    let top = rect.top() + (rect.height() - label_room - mm(total)) / 2.0;

    // Spindle and nut.
    let spindle = Rect::from_min_max(pos2(cx - mm(nut_d * 0.42), top), pos2(cx + mm(nut_d * 0.42), top + mm(spindle_h)));
    p.rect_filled(spindle, CornerRadius::ZERO, NUT.linear_multiply(0.6));
    let nut_top = top + mm(spindle_h);
    let nut = Rect::from_min_max(pos2(cx - mm(nut_d / 2.0), nut_top), pos2(cx + mm(nut_d / 2.0), nut_top + mm(nut_h)));
    p.rect_filled(nut, CornerRadius::same(4), NUT);
    for f in [-0.25, 0.25] {
        let x = cx + mm(nut_d * f);
        p.line_segment([pos2(x, nut.top() + 3.0), pos2(x, nut.bottom() - 3.0)], Stroke::new(2.0_f32, NUT.linear_multiply(0.6)));
    }

    // Shank, from the nut to the flutes.
    let shank_top = nut.bottom();
    let flute_top = shank_top + mm(stickout - flutes);
    let tip = shank_top + mm(stickout);
    let r = mm(d / 2.0);
    p.rect_filled(Rect::from_min_max(pos2(cx - mm(shank / 2.0), shank_top), pos2(cx + mm(shank / 2.0), flute_top)), CornerRadius::ZERO, STEEL);
    if shank > d {
        // A neck down to the cutting diameter.
        p.add(Shape::convex_polygon(
            vec![pos2(cx - mm(shank / 2.0), flute_top), pos2(cx + mm(shank / 2.0), flute_top), pos2(cx + r, flute_top + mm(shank - d)), pos2(cx - r, flute_top + mm(shank - d))],
            STEEL,
            Stroke::NONE,
        ));
    }

    // Cutting end outline, tip at `tip`.
    let outline = cutter_outline(kind, cx, flute_top, tip, r);
    p.add(Shape::convex_polygon(outline.clone(), STEEL, Stroke::new(1.5_f32, STEEL_DARK)));
    // Helix lines across the straight part of the flutes.
    let straight_end = tip - mm(tip_height(kind, d));
    if straight_end > flute_top && kind != ToolKind::Probe {
        let clip = p.with_clip_rect(Rect::from_min_max(pos2(cx - r, flute_top), pos2(cx + r, straight_end)).intersect(rect));
        let pitch = (2.0 * r).max(6.0) * 0.7;
        let mut y = flute_top - 2.0 * r;
        while y < straight_end {
            clip.line_segment([pos2(cx - r, y + r * 0.9), pos2(cx + r, y)], Stroke::new(2.0_f32, STEEL_DARK));
            y += pitch;
        }
    }
    p.add(Shape::closed_line(outline, Stroke::new(1.5_f32, STEEL_DARK)));

    // Diameter, marked under the tip.
    let y = tip + 22.0;
    let (l, rr) = (pos2(cx - r.max(4.0), y), pos2(cx + r.max(4.0), y));
    let s = Stroke::new(2.0_f32, ACCENT);
    p.line_segment([pos2(l.x, tip + 6.0), pos2(l.x, y + 6.0)], Stroke::new(1.0_f32, ACCENT));
    p.line_segment([pos2(rr.x, tip + 6.0), pos2(rr.x, y + 6.0)], Stroke::new(1.0_f32, ACCENT));
    p.line_segment([l, rr], s);
    p.text(pos2(cx, y + 10.0), Align2::CENTER_TOP, format!("Ø {}", trim(d)), FontId::proportional(30.0), ACCENT);
    // The stick-out, from the nut's face to the tip, when the program gives it.
    if tool.stickout > 0.0 {
        let x = cx + mm(nut_d / 2.0) + 14.0;
        let st = Stroke::new(2.0_f32, AMBER);
        p.line_segment([pos2(x, shank_top), pos2(x, tip)], st);
        for yy in [shank_top, tip] {
            p.line_segment([pos2(x - 7.0, yy), pos2(x + 7.0, yy)], st);
        }
        p.line_segment([pos2(cx + r + 3.0, tip), pos2(x - 7.0, tip)], Stroke::new(1.0_f32, AMBER));
        p.text(pos2(x + 10.0, (shank_top + tip) / 2.0), Align2::LEFT_CENTER, format!("{} mm", trim(tool.stickout)), FontId::proportional(26.0), AMBER);
    }
    p.text(rect.left_top(), Align2::LEFT_TOP, kind.name(), FontId::proportional(16.0), MUTED);
}

/// Height of the tip's own shape below the straight flutes, mm.
fn tip_height(kind: ToolKind, d: f64) -> f64 {
    match kind {
        ToolKind::Ball | ToolKind::Probe => d / 2.0,
        ToolKind::Bull => d / 6.0,
        ToolKind::Chamfer => d / 2.0,
        // 118° point.
        ToolKind::Drill => d / 2.0 / (59.0_f64).to_radians().tan(),
        ToolKind::Flat | ToolKind::Auto => 0.0,
    }
}

/// Convex outline of the cutting end from `top` down to the tip at `tip`,
/// clockwise from the top left. A chamfer's cone starts at the full diameter.
fn cutter_outline(kind: ToolKind, cx: f32, top: f32, tip: f32, r: f32) -> Vec<Pos2> {
    let arc = |centre: Pos2, radius: f32, from: f32, to: f32| -> Vec<Pos2> {
        (0..=12)
            .map(|i| {
                let a = from + (to - from) * i as f32 / 12.0;
                centre + Vec2::new(a.cos(), a.sin()) * radius
            })
            .collect()
    };
    use std::f32::consts::PI;
    let mut pts = vec![pos2(cx - r, top), pos2(cx + r, top)];
    match kind {
        ToolKind::Ball | ToolKind::Probe => pts.extend(arc(pos2(cx, tip - r), r, 0.0, PI)),
        ToolKind::Bull => {
            let cr = r / 3.0;
            pts.extend(arc(pos2(cx + r - cr, tip - cr), cr, 0.0, PI / 2.0));
            pts.extend(arc(pos2(cx - r + cr, tip - cr), cr, PI / 2.0, PI));
        }
        ToolKind::Chamfer => pts.push(pos2(cx, tip)),
        ToolKind::Drill => {
            let h = r / (59.0_f32).to_radians().tan();
            pts.extend([pos2(cx + r, tip - h), pos2(cx, tip), pos2(cx - r, tip - h)]);
        }
        ToolKind::Flat | ToolKind::Auto => pts.extend([pos2(cx + r, tip), pos2(cx - r, tip)]),
    }
    pts
}
