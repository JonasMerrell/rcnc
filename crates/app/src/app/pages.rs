//! The right-hand tabs besides the program listing: tool table, work
//! offsets, probing and machine settings.

use eframe::egui::{self, Button, DragValue, RichText, ScrollArea, Vec2};
use gcode::{Tool, AXIS_LETTERS};
use machine::{mach3, store, Command, MachineConfig, Mode, ProbeCalShape, ProbeCalTarget, ProbeCalibration, ProbeJob};

use super::palette::*;
use super::{section_title, App};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Tab {
    Program,
    Tools,
    Offsets,
    Probe,
    Settings,
}

impl Tab {
    pub(super) const ALL: [(Tab, &'static str); 5] = [
        (Tab::Program, "Program"),
        (Tab::Tools, "Tools"),
        (Tab::Offsets, "Offsets"),
        (Tab::Probe, "Probe"),
        (Tab::Settings, "Settings"),
    ];
}

/// Edits to the machine settings, applied together (they may reconnect the board).
#[derive(Default)]
pub(super) struct SettingsDraft {
    pub config: Option<MachineConfig>,
    /// The settings the draft started from: Apply changes only what was
    /// edited here, so changes made meanwhile elsewhere (probe page, tool
    /// change, jog speed) survive.
    pub base: Option<MachineConfig>,
    pub import_notes: Vec<String>,
    pub new_tool: u32,
    /// Tool number for "Change tool".
    pub change_to: u32,
}

fn num(ui: &mut egui::Ui, v: &mut f64, speed: f64, suffix: &str) -> bool {
    ui.add(DragValue::new(v).speed(speed).max_decimals(4).suffix(suffix)).changed()
}

fn labelled(ui: &mut egui::Ui, label: &str, v: &mut f64, speed: f64, suffix: &str) -> bool {
    ui.label(RichText::new(label).color(MUTED));
    let changed = num(ui, v, speed, suffix);
    ui.end_row();
    changed
}

impl App {
    pub(super) fn tools_page(&mut self, ui: &mut egui::Ui) {
        let s = self.snap.clone();
        section_title(ui, "Tool table");
        ui.label(
            RichText::new("Lengths are used by G43 H<n>. Diameters by cutter compensation (G41/G42 D<n>).")
                .size(12.0)
                .color(MUTED),
        );
        ui.add_space(6.0);
        // The tool table is locked while the machine moves: a new length would move Z mid-cut.
        let editable = matches!(s.mode, Mode::Idle | Mode::EStop | Mode::Disconnected | Mode::Alarm)
            || (s.mode == Mode::Running && s.paused.is_some());
        if !editable {
            ui.label(RichText::new("Locked while the machine runs").color(AMBER).size(12.0));
        }
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_enabled_ui(editable, |ui| {
                // The description gets its own line, so the table fits a narrow panel.
                let (tw, nw) = (40.0, 86.0);
                ui.horizontal(|ui| {
                    for (h, w) in [("Tool", tw), ("Diameter", nw), ("Length", nw)] {
                        ui.add_sized([w, 18.0], egui::Label::new(RichText::new(h).color(MUTED).strong()));
                    }
                });
                for (n, tool) in &s.tools {
                    let mut t = tool.clone();
                    let mut changed = false;
                    let mut remove = false;
                    let current = *n == s.tool;
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [tw, 22.0],
                            egui::Label::new(RichText::new(format!("T{n}")).strong().color(if current { GREEN } else { TEXT })),
                        );
                        for v in [&mut t.diameter, &mut t.length] {
                            changed |= ui
                                .add_sized([nw, 22.0], DragValue::new(v).speed(0.01).max_decimals(4).suffix(" mm"))
                                .changed();
                        }
                        remove = ui.small_button("Remove").clicked();
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(tw + ui.spacing().item_spacing.x);
                        changed |= ui
                            .add(
                                egui::TextEdit::singleline(&mut t.description)
                                    .hint_text("Description")
                                    .desired_width(ui.available_width()),
                            )
                            .changed();
                    });
                    // What the tool change window draws, or the photo it shows instead.
                    ui.horizontal(|ui| {
                        ui.add_space(tw + ui.spacing().item_spacing.x);
                        egui::ComboBox::from_id_salt(("kind", *n))
                            .selected_text(t.kind.name())
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                for k in gcode::ToolKind::ALL {
                                    changed |= ui.selectable_value(&mut t.kind, k, k.name()).changed();
                                }
                            });
                        ui.label(RichText::new("Flutes").color(MUTED));
                        changed |= ui.add(DragValue::new(&mut t.flute_length).speed(0.1).range(0.0..=500.0).suffix(" mm")).changed();
                        ui.label(RichText::new("Stick out").color(MUTED));
                        changed |= ui.add(DragValue::new(&mut t.stickout).speed(0.1).range(0.0..=500.0).suffix(" mm")).changed();
                        // the tool just measured, with its stick-out known for sure, calibrates the stick-out check
                        if self.snap.last_measured_tool == Some(*n) && t.stickout > 0.0 {
                            let text = if self.snap.config.tool_change.nut_face_z.is_some() { "Recalibrate stick-out check" } else { "Calibrate stick-out check" };
                            if ui
                                .small_button(text)
                                .on_hover_text(format!("T{n} was just measured on the setter and sticks out exactly {} mm from the collet nut: from it, rcnc works out every measured tool's stick-out", t.stickout))
                                .clicked()
                            {
                                self.machine.send(Command::CalibrateStickout { stickout: t.stickout });
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.add_space(tw + ui.spacing().item_spacing.x);
                        let photo = if t.image.is_empty() { format!("Photo: tools/T{n}.png if there is one") } else { format!("Photo: {}", t.image) };
                        ui.label(RichText::new(photo).size(12.0).color(MUTED));
                        if ui.add_enabled(self.can_open_dialogs(), egui::Button::new("Pick…").small()).clicked() {
                            self.pick(super::Picking::ToolPhoto(*n));
                        }
                        if !t.image.is_empty() && ui.small_button("Clear").clicked() {
                            t.image.clear();
                            changed = true;
                        }
                    });
                    ui.add_space(4.0);
                    if remove {
                        self.machine.send(Command::SetTool(*n, None));
                    } else if changed {
                        self.machine.send(Command::SetTool(*n, Some(t)));
                    }
                }
                ui.horizontal(|ui| {
                    let next = s.tools.keys().max().map(|m| m + 1).unwrap_or(1);
                    if self.draft.new_tool == 0 || s.tools.contains_key(&self.draft.new_tool) {
                        self.draft.new_tool = next;
                    }
                    ui.label("T");
                    ui.add(DragValue::new(&mut self.draft.new_tool).range(1..=999));
                    if ui.button("Add tool").clicked() {
                        self.machine.send(Command::SetTool(self.draft.new_tool, Some(Tool::default())));
                    }
                });
            });
            ui.add_space(12.0);
            section_title(ui, "Z zero");
            if let Some(job) = z_reference_buttons(ui, &s) {
                self.machine.send(Command::Probe(job));
            }
            ui.add_space(12.0);
            self.tool_change_settings(ui);
            ui.add_space(12.0);
            self.wash_down_settings(ui);
        });
    }

    fn wash_down_settings(&mut self, ui: &mut egui::Ui) {
        let s = self.snap.clone();
        let idle = s.mode == Mode::Idle;
        section_title(ui, "Coolant wash-down");
        let area = s.config.wash_area();
        let homed = s.homed[..3].iter().all(|h| *h);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(idle && homed && area.is_ok(), Button::new("Wash down now"))
                .on_hover_text("Stop the spindle, raise Z, turn the flood coolant on and sweep the area, then coolant off and back. A program can ask for it with (RCNC WASH_DOWN)")
                .clicked()
            {
                self.machine.send(Command::WashDown);
            }
            let text = match &area {
                Ok((x, y)) => RichText::new(format!("Sweeps machine X{:.1} to X{:.1}, Y{:.1} to Y{:.1}", x[0], x[1], y[0], y[1])).color(TEXT),
                Err(e) => RichText::new(format!("Can't run: {e}")).color(AMBER),
            };
            ui.label(text);
        });
        let mut c = s.config.clone();
        let mut changed = false;
        let w = &mut c.wash_down;
        changed |= ui
            .checkbox(&mut w.around_vise, "Around the saved vise location (Find vise location, Probe page)")
            .changed();
        egui::Grid::new("wash-down").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            if w.around_vise {
                changed |= labelled(ui, "Vise size X", &mut w.vise_size[0], 1.0, " mm");
                changed |= labelled(ui, "Vise size Y", &mut w.vise_size[1], 1.0, " mm");
                changed |= labelled(ui, "Past the vise by", &mut w.margin, 1.0, " mm");
            } else {
                changed |= labelled(ui, "From X", &mut w.x[0], 1.0, " mm");
                changed |= labelled(ui, "To X", &mut w.x[1], 1.0, " mm");
                changed |= labelled(ui, "From Y", &mut w.y[0], 1.0, " mm");
                changed |= labelled(ui, "To Y", &mut w.y[1], 1.0, " mm");
            }
            changed |= labelled(ui, "Lowest Z", &mut w.z, 0.5, " mm");
            changed |= labelled(ui, "Row spacing", &mut w.spacing, 1.0, " mm");
            changed |= labelled(ui, "Sweep feed", &mut w.feed, 50.0, " mm/min");
            ui.label(RichText::new("Passes").color(MUTED));
            changed |= ui.add(DragValue::new(&mut w.passes).range(1..=20)).changed();
            ui.end_row();
            changed |= labelled(ui, "Coolant on before moving", &mut w.soak, 0.5, " s");
        });
        if !w.around_vise {
            if ui
                .button("Corner here")
                .on_hover_text("Jog to one corner of the area, press this, then jog to the opposite corner and press it again")
                .clicked()
            {
                // the first press sets both corners here, the second the far one
                if (w.x[0] - w.x[1]).abs() < 1e-6 && (w.y[0] - w.y[1]).abs() < 1e-6 {
                    w.x = [s.machine[0]; 2];
                    w.y = [s.machine[1]; 2];
                } else {
                    w.x[1] = s.machine[0];
                    w.y[1] = s.machine[1];
                }
                changed = true;
            }
        }
        ui.label(
            RichText::new(
                "Machine coordinates. Z only ever rises to the lowest Z. Each pass laps the edge, then rows across it;                  every other pass runs crosswise. Stop switches the coolant off.",
            )
            .size(12.0)
            .color(MUTED),
        );
        if changed && idle {
            self.machine.send(Command::SetConfig(Box::new(c)));
        }
    }

    fn tool_change_settings(&mut self, ui: &mut egui::Ui) {
        let s = self.snap.clone();
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        let idle = s.mode == Mode::Idle;
        section_title(ui, "Tool change (M6)");
        ui.horizontal(|ui| {
            if self.draft.change_to == 0 {
                self.draft.change_to = s.tools.keys().copied().find(|&n| n != s.tool).unwrap_or(1);
            }
            ui.label("Load T");
            ui.add(DragValue::new(&mut self.draft.change_to).range(1..=999));
            if ui
                .add_enabled(idle, Button::new("Change tool"))
                .on_hover_text("Runs the tool change now, as M6 would in a program")
                .clicked()
            {
                self.machine.send(Command::Mdi(format!("T{} M6", self.draft.change_to)));
            }
        });
        ui.add_space(4.0);
        let mut c = s.config.clone();
        let mut changed = false;
        let mut y = c.change_y();
        let t = &mut c.tool_change;
        changed |= ui
            .checkbox(&mut t.guided, "Guided: raise Z, move the bed back, measure, come back")
            .changed();
        ui.add_enabled_ui(t.guided, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
            egui::Grid::new("tool-change").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                if labelled(ui, "Change Y (bed back)", &mut y, 0.5, " mm") {
                    t.y = Some(y);
                    changed = true;
                }
                changed |= labelled(ui, "Safe Z", &mut t.z, 0.5, " mm");
                ui.label("X to the middle");
                changed |= ui.checkbox(&mut t.centre_x, "").changed();
                ui.end_row();
                if t.centre_x {
                    changed |= labelled(ui, "X travel", &mut t.x_travel, 1.0, " mm");
                } else {
                    ui.label("Move X too");
                    changed |= ui.checkbox(&mut t.move_x, "").changed();
                    ui.end_row();
                    if t.move_x {
                        changed |= labelled(ui, "Change X", &mut t.x, 0.5, " mm");
                    }
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .button("Use the current position")
                    .on_hover_text("Jog to where the spindle is easiest to reach, then press this")
                    .clicked()
                {
                    t.x = s.machine[0];
                    t.y = Some(s.machine[1]);
                    changed = true;
                }
                if ui
                    .add_enabled(t.y.is_some(), Button::new("Bed all the way back"))
                    .on_hover_text("Y just short of its home switch, the back end of travel")
                    .clicked()
                {
                    t.y = None;
                    changed = true;
                }
            });
            changed |= ui.checkbox(&mut t.return_to_start, "Go back to where the change started").changed();
        });
        changed |= ui.checkbox(&mut c.tool_change_touch_off, "Measure the new tool on the tool setter").changed();
        ui.horizontal(|ui| {
            ui.label("It's measured twice; the readings may differ by");
            changed |= ui
                .add(DragValue::new(&mut c.probing.setter.repeat_tolerance).speed(0.001).range(0.001..=1.0).max_decimals(3).suffix(" mm"))
                .changed();
        });
        ui.label(
            RichText::new("Positions are machine coordinates. The spindle stops during the change and restarts on the way back.")
                .size(12.0)
                .color(MUTED),
        );
        if changed && idle {
            self.machine.send(Command::SetConfig(Box::new(c)));
        }
    }

    pub(super) fn offsets_page(&mut self, ui: &mut egui::Ui) {
        let s = self.snap.clone();
        let editable = !matches!(s.mode, Mode::Running | Mode::Probing);
        section_title(ui, "Work offsets (machine position of each work zero)");
        ui.add_space(6.0);
        egui::Grid::new("offsets").num_columns(5).striped(true).spacing([10.0, 6.0]).show(ui, |ui| {
            for h in ["", "X", "Y", "Z", ""] {
                ui.label(RichText::new(h).color(MUTED).strong());
            }
            ui.end_row();
            for (i, offset) in s.work_offsets.iter().enumerate() {
                let active = i == s.coord_system;
                ui.label(RichText::new(format!("G{}", 54 + i)).strong().color(if active { GREEN } else { TEXT }));
                let mut o = *offset;
                let mut changed = false;
                for v in o.iter_mut().take(3) {
                    changed |= ui.add_enabled(editable, DragValue::new(v).speed(0.01).max_decimals(4)).changed();
                }
                if changed {
                    self.machine.send(Command::SetWorkOffset { system: i, offset: o });
                }
                if active {
                    ui.label(RichText::new("active").color(GREEN).size(12.0));
                } else if ui.add_enabled(s.mode == Mode::Idle, Button::new("Use").small()).clicked() {
                    self.machine.send(Command::Mdi(format!("G{}", 54 + i)));
                }
                ui.end_row();
            }
        });
        ui.add_space(10.0);
        let g92 = s.g92;
        if g92.iter().any(|v| *v != 0.0) {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("G92 shift X{:.3} Y{:.3} Z{:.3}", g92[0], g92[1], g92[2])).color(AMBER));
                if ui.add_enabled(s.mode == Mode::Idle, Button::new("Clear (G92.1)")).clicked() {
                    self.machine.send(Command::Mdi("G92.1".into()));
                }
            });
        }
        ui.label(
            RichText::new(format!("Tool length offset {:.3} mm (T{})", s.tool_length_offset, s.tool))
                .size(12.0)
                .color(MUTED),
        );
    }

    pub(super) fn probe_page(&mut self, ui: &mut egui::Ui) {
        let s = self.snap.clone();
        let paused = s.mode == Mode::Running && s.paused.is_some();
        let can = (s.mode == Mode::Idle || paused) && !s.probe_active;
        if s.probe_active {
            ui.label(RichText::new("Probe is touching: check the probe before probing").color(RED));
        }
        let run = |ui: &mut egui::Ui, label: &str, job: ProbeJob, size: Vec2| {
            if ui.add_enabled(can, Button::new(label).min_size(size)).clicked() {
                self.machine.send(Command::Probe(job));
            }
        };
        let mut c = s.config.clone();
        let p = &mut c.probing;
        let mut changed = false;
        // One scroll area for the page so the setter settings are reachable on small screens.
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            section_title(ui, "Tool");
            if let Some(job) = z_reference_buttons(ui, &s) {
                self.machine.send(Command::Probe(job));
            }
            run(ui, "Touch off tool on setter", ProbeJob::ToolTouchOff, Vec2::new(220.0, 36.0));
            ui.add_space(8.0);
            section_title(ui, "Work");
            let b = Vec2::new(72.0, 30.0);
            ui.horizontal(|ui| {
                run(ui, "Z surface", ProbeJob::ZSurface, Vec2::new(100.0, 30.0));
                run(ui, "Bore", ProbeJob::BoreCentre, b);
                run(ui, "Boss", ProbeJob::BossCentre, b);
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Edge").color(MUTED));
                run(ui, "X+", ProbeJob::Edge { axis: 0, positive: true }, b);
                run(ui, "X−", ProbeJob::Edge { axis: 0, positive: false }, b);
                run(ui, "Y+", ProbeJob::Edge { axis: 1, positive: true }, b);
                run(ui, "Y−", ProbeJob::Edge { axis: 1, positive: false }, b);
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Corner").color(MUTED));
                // Named by which corner of the part it is (front = -Y). The probe starts
                // diagonally outside it, so it probes toward the part.
                let b = Vec2::new(88.0, 30.0);
                run(ui, "Front left", ProbeJob::Corner { x_positive: true, y_positive: true }, b);
                run(ui, "Front right", ProbeJob::Corner { x_positive: false, y_positive: true }, b);
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Corner").color(PANEL));
                let b = Vec2::new(88.0, 30.0);
                run(ui, "Back left", ProbeJob::Corner { x_positive: true, y_positive: false }, b);
                run(ui, "Back right", ProbeJob::Corner { x_positive: false, y_positive: false }, b);
            });
            ui.label(
                RichText::new(
                    "Edge: start beside the part, above it, within the clearance. Corner: start diagonally off it. \
                     Bore: start inside at probing height. Boss: start above its middle.",
                )
                .size(12.0)
                .color(MUTED),
            );
            ui.add_space(8.0);
            section_title(ui, "Find stock");
            ui.horizontal(|ui| {
                ui.label(RichText::new("Size X/Y/Z").color(MUTED));
                for v in self.fs_size.iter_mut() {
                    num(ui, v, 0.1, " mm");
                }
            });
            let probe_in = s.loaded_tool == p.probe_tool && s.tool_measured;
            if ui
                .add_enabled(can && probe_in, Button::new("Find stock (zero on the top centre)").min_size(Vec2::new(260.0, 32.0)))
                .clicked()
            {
                self.machine.send(Command::FindStock(gcode::StockRequest {
                    size: self.fs_size,
                    top_centre: [0.0; 3],
                    tolerance: None,
                    depth: None,
                    keep_z: false,
                    jaw: None,
                    jaw_at: None,
                    vise: self.fs_vise.filter(|_| p.vise.location.is_some()),
                }));
            }
            if p.vise.location.is_some() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("X from").color(MUTED));
                    ui.selectable_value(&mut self.fs_vise, None, "the stock's sides");
                    ui.selectable_value(&mut self.fs_vise, Some(-1.0), "right station's fixed jaw");
                    ui.selectable_value(&mut self.fs_vise, Some(1.0), "left station's fixed jaw");
                });
            }
            if !probe_in {
                ui.label(RichText::new(format!("Load and measure the probe (T{} M6) first", p.probe_tool)).size(12.0).color(MUTED));
            }
            egui::Grid::new("find-stock-settings").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                let f = &mut p.find_stock;
                changed |= labelled(ui, "Rough position ±", &mut f.clearance, 0.5, " mm");
                changed |= labelled(ui, "Search down", &mut f.z_search, 0.5, " mm");
                changed |= labelled(ui, "Above the top", &mut f.z_clearance, 0.5, " mm");
                changed |= labelled(ui, "Sides below the top", &mut f.depth, 0.1, " mm");
                changed |= labelled(ui, "Size tolerance", &mut f.tolerance, 0.05, " mm");
                ui.label("Probe tool");
                let mut t = p.probe_tool as f64;
                if num(ui, &mut t, 1.0, "") {
                    p.probe_tool = t.max(0.0).round() as u32;
                    changed = true;
                }
                ui.end_row();
            });
            ui.add_space(8.0);
            section_title(ui, "Vise");
            match &p.vise.location {
                Some(v) => {
                    ui.label(
                        RichText::new(format!(
                            "Fixed jaws at machine X{:.3} (left station) / X{:.3} (right station), {:.3} apart",
                            v.faces[0], v.faces[1], v.gap
                        ))
                        .color(TEXT),
                    );
                    let len = v.length.map(|l| format!(", jaws {l:.3} long")).unwrap_or_default();
                    ui.label(
                        RichText::new(format!(
                            "Middle Y{:.3}{len}; turned {:.3}°; jaw top {:.3} below the setter top, {:.3} above the ledges",
                            v.y, v.angle, v.setter_above_top, v.web_below_top
                        ))
                        .color(TEXT),
                    );
                }
                None => {
                    ui.label(RichText::new("Vise location not found yet: programs with FIND_STOCK VISE= need it").color(MUTED));
                }
            }
            if ui
                .add_enabled(s.mode == Mode::Idle && probe_in, Button::new("Find vise location").min_size(Vec2::new(260.0, 32.0)))
                .on_hover_text("Empty the vise, jog the probe over the middle of the fixed centre block, between the jaw bars: rcnc touches the web, a jaw bar's top, the bars' Y ends and both fixed jaw faces, and keeps where they are")
                .clicked()
            {
                self.machine.send(Command::FindVise);
            }
            egui::Grid::new("vise-settings").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                let v = &mut p.vise;
                changed |= labelled(ui, "Between the fixed jaw faces", &mut v.gap, 0.1, " mm");
                changed |= labelled(ui, "Jaw bar thickness", &mut v.jaw_thickness, 0.1, " mm");
                changed |= labelled(ui, "Jaw length", &mut v.jaw_length, 0.1, " mm");
                ui.label(RichText::new("Touch the jaws' Y ends").color(MUTED));
                changed |= ui.checkbox(&mut v.ends, "").changed();
                ui.end_row();
                changed |= labelled(ui, "Faces below the jaw top", &mut v.face_depth, 0.1, " mm");
                changed |= labelled(ui, "Faces touched ± along Y", &mut v.spread, 1.0, " mm");
                changed |= labelled(ui, "Gap / length tolerance", &mut v.tolerance, 0.05, " mm");
            });
            ui.add_space(8.0);
            section_title(ui, "Calibrate probe");
            {
                let cal = &p.calibration;
                let ball = if cal.diameter.iter().all(|d| *d > 0.0) {
                    format!("Ball X {:.3} / Y {:.3} mm, on the {}", cal.diameter[0], cal.diameter[1], cal.diameter_on)
                } else {
                    format!("Ball not calibrated: the Ø{:.3} stylus diameter is used", p.stylus_diameter)
                };
                ui.label(RichText::new(ball).color(TEXT));
                let z = if cal.z_offset_on.is_empty() {
                    "Z offset not calibrated (0)".to_string()
                } else {
                    format!("Z offset {:+.3} mm, on the {} reference", cal.z_offset, cal.z_offset_on)
                };
                ui.label(RichText::new(z).color(TEXT));
                if let Some(r) = s.probe_runout {
                    ui.label(RichText::new(format!("Ball off centre X {:+.3} Y {:+.3} mm (until the probe comes out)", r[0], r[1])).color(TEXT));
                }
            }
            ui.horizontal(|ui| {
                for shape in ProbeCalShape::ALL {
                    if ui.selectable_label(self.pc_shape == shape, shape.name()).clicked() {
                        self.pc_shape = shape;
                    }
                }
            });
            let shape = self.pc_shape;
            ui.horizontal(|ui| {
                if shape.round() {
                    ui.label(RichText::new("Diameter").color(MUTED));
                    num(ui, &mut self.pc_size[0], 0.001, " mm");
                } else {
                    ui.label(RichText::new("Size X / Y").color(MUTED));
                    num(ui, &mut self.pc_size[0], 0.001, " mm");
                    num(ui, &mut self.pc_size[1], 0.001, " mm");
                }
                if !shape.inside() {
                    ui.label(RichText::new("sides").color(MUTED));
                    num(ui, &mut self.pc_depth, 0.1, " mm");
                    ui.label(RichText::new("below the top").color(MUTED));
                }
            });
            let idle = s.mode == Mode::Idle;
            let size = if shape.round() { [self.pc_size[0]; 2] } else { self.pc_size };
            let hint = "Effective ball size in X and Y. Then you can turn the probe half a turn and measure again to find how far it's off centre";
            if ui
                .add_enabled(idle && probe_in, Button::new(format!("Calibrate ball on the {}", shape.name().to_lowercase())).min_size(Vec2::new(260.0, 32.0)))
                .on_hover_text(hint)
                .clicked()
            {
                self.machine.send(Command::CalibrateProbe(ProbeCalTarget::Shape { shape, size, depth: self.pc_depth }));
            }
            ui.horizontal(|ui| {
                let refs = &p.setter.references;
                self.pc_zref = self.pc_zref.min(refs.len().saturating_sub(1));
                let name = refs.get(self.pc_zref).map_or("—".to_string(), |r| r.name.clone());
                egui::ComboBox::from_id_salt("probe-cal-zref").selected_text(name).show_ui(ui, |ui| {
                    for (i, r) in refs.iter().enumerate() {
                        ui.selectable_value(&mut self.pc_zref, i, &r.name);
                    }
                });
                let go = ui
                    .add_enabled(idle && probe_in && !refs.is_empty(), Button::new("Calibrate Z offset"))
                    .on_hover_text("Touch a surface whose Z reference you trust (calibrated with a gauge block and a real tool)");
                if go.clicked() {
                    self.machine.send(Command::CalibrateProbe(ProbeCalTarget::ZReference(self.pc_zref)));
                }
            });
            if p.calibration != ProbeCalibration::default() && ui.add_enabled(idle, Button::new("Forget the probe calibration")).clicked() {
                p.calibration = ProbeCalibration::default();
                changed = true;
            }
            ui.add_space(8.0);
            changed |= ui.checkbox(&mut p.auto_zero, "Set work zero at the result").changed();
            egui::Grid::new("probe-settings").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                changed |= labelled(ui, "Search feed", &mut p.fast_feed, 1.0, " mm/min");
                changed |= labelled(ui, "Latch feed", &mut p.slow_feed, 0.5, " mm/min");
                changed |= labelled(ui, "Search distance", &mut p.max_distance, 0.5, " mm");
                changed |= labelled(ui, "Back-off", &mut p.latch, 0.05, " mm");
                changed |= labelled(ui, "Side clearance", &mut p.xy_clearance, 0.1, " mm");
                changed |= labelled(ui, "Edge depth", &mut p.z_depth, 0.1, " mm");
                changed |= labelled(ui, "Stylus diameter (nominal)", &mut p.stylus_diameter, 0.01, " mm");
                changed |= labelled(ui, "Touch plate", &mut p.plate_thickness, 0.01, " mm");
                changed |= labelled(ui, "Boss size", &mut p.boss_size, 0.5, " mm");
            });
            ui.add_space(6.0);
            section_title(ui, "Tool setter (machine coordinates)");
            let t = &mut p.setter;
            egui::Grid::new("setter").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                changed |= labelled(ui, "X", &mut t.x, 0.1, " mm");
                changed |= labelled(ui, "Y", &mut t.y, 0.1, " mm");
                changed |= labelled(ui, "Travel height Z", &mut t.travel_z, 0.1, " mm");
                changed |= labelled(ui, "Search distance", &mut t.max_distance, 0.5, " mm");
                if t.active().is_none() {
                    changed |= labelled(ui, "Setter top (work Z)", &mut t.offset, 0.001, " mm");
                }
            });
            ui.add_space(4.0);
            ui.label(RichText::new("Z zero references: the setter top's height above each surface").color(MUTED));
            let idle = s.mode == Mode::Idle;
            let mut calibrate = None;
            egui::Grid::new("z-references").num_columns(4).spacing([12.0, 4.0]).show(ui, |ui| {
                for (i, r) in t.references.iter_mut().enumerate() {
                    changed |= ui.add(egui::TextEdit::singleline(&mut r.name).desired_width(90.0)).changed();
                    changed |= num(ui, &mut r.setter_top, 0.001, " mm");
                    if ui.add_enabled(idle, Button::new("Calibrate (probe)")).on_hover_text("Touch the surface with the measured touch probe").clicked() {
                        calibrate = Some((i, None));
                    }
                    if ui.add_enabled(idle, Button::new("Calibrate (gauge)")).on_hover_text("Jog a measured tool onto a gauge block standing on the surface").clicked() {
                        calibrate = Some((i, Some(self.cal_gauge)));
                    }
                    ui.end_row();
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Gauge block").color(MUTED));
                num(ui, &mut self.cal_gauge, 0.01, " mm");
            });
            if let Some((reference, gauge)) = calibrate {
                self.machine.send(Command::CalibrateZ { reference, gauge });
            }
            if ui.button("Setter is under the spindle now").clicked() {
                t.x = s.machine[0];
                t.y = s.machine[1];
                changed = true;
            }
        });
        if changed && s.mode == Mode::Idle {
            self.machine.send(Command::SetConfig(Box::new(c)));
        }
    }

    pub(super) fn settings_page(&mut self, ui: &mut egui::Ui) {
        self.ears.settings_ui(ui);
        ui.separator();
        self.vfd.settings_ui(ui);
        ui.separator();
        let s = self.snap.clone();
        let idle = matches!(s.mode, Mode::Idle | Mode::EStop | Mode::Disconnected | Mode::Alarm);
        if self.draft.config.is_none() {
            self.draft.base = Some(s.config.clone());
        }
        let draft = self.draft.config.get_or_insert_with(|| s.config.clone());
        // Edited here, not changed elsewhere since (that would make every draft look dirty).
        let dirty = Some(&*draft) != self.draft.base.as_ref();
        let (mut apply, mut revert, mut import, mut dismiss) = (false, false, false, false);
        // Buttons first, so they can't be pushed off the bottom by the import summary.
        section_title(ui, "Machine settings");
        ui.horizontal_wrapped(|ui| {
            import = ui.add_enabled(idle, Button::new("Import Mach3 profile…")).clicked();
            apply = ui.add_enabled(dirty && idle, Button::new("Apply")).clicked();
            revert = ui.add_enabled(dirty, Button::new("Revert")).clicked();
            if dirty && !idle {
                ui.label(RichText::new("Stop the machine to apply").color(MUTED));
            }
        });
        ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if !self.draft.import_notes.is_empty() {
                egui::Frame::new().fill(RAISED).inner_margin(8).corner_radius(6).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (text, color) = if dirty {
                            ("Imported: check the values below, then Apply", AMBER)
                        } else {
                            ("Imported: matches the current settings, nothing to apply", GREEN)
                        };
                        ui.label(RichText::new(text).color(color).strong());
                        dismiss = ui.small_button("Dismiss").clicked();
                    });
                    for n in &self.draft.import_notes {
                        ui.label(RichText::new(n).size(12.0));
                    }
                });
                ui.add_space(6.0);
            }
            // Narrow fields so the table fits the panel; scrolls sideways if it still can't.
            ScrollArea::horizontal().id_salt("axes-scroll").show(ui, |ui| {
                ui.spacing_mut().interact_size.x = 48.0;
                egui::Grid::new("axes").num_columns(7).striped(true).spacing([4.0, 4.0]).show(ui, |ui| {
                    for h in ["", "Steps/mm", "mm/s", "mm/s²", "Min", "Max", "Home"] {
                        ui.label(RichText::new(h).color(MUTED).strong());
                    }
                    ui.end_row();
                    for (i, a) in draft.axes.iter_mut().enumerate().filter(|(_, a)| a.enabled) {
                        ui.label(RichText::new(AXIS_LETTERS[i].to_string()).strong());
                        num(ui, &mut a.steps_per_unit, 0.1, "");
                        num(ui, &mut a.max_velocity, 0.5, "");
                        num(ui, &mut a.acceleration, 10.0, "");
                        num(ui, &mut a.soft_min, 0.5, "");
                        num(ui, &mut a.soft_max, 0.5, "");
                        num(ui, &mut a.home_position, 0.1, "");
                        ui.end_row();
                    }
                });
            });
            ui.add_space(6.0);
            ui.checkbox(&mut draft.soft_limits, "Soft limits");
            ui.checkbox(&mut draft.constant_velocity, "Constant velocity (blend corners)");
            ui.checkbox(&mut draft.spindle_reversible, "Spindle can run in reverse (M4, G84)");
            egui::Grid::new("spindle").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                labelled(ui, "Max spindle speed", &mut draft.spindle_max_rpm, 100.0, " rpm");
                labelled(ui, "Spin-up wait", &mut draft.spindle_spin_up, 0.1, " s");
                labelled(ui, "Spin-down wait", &mut draft.spindle_spin_down, 0.1, " s");
                labelled(ui, "Corner tolerance", &mut draft.junction_deviation, 0.001, " mm");
                labelled(ui, "Home pull-off", &mut draft.home_pulloff, 0.1, " mm");
                labelled(ui, "Slow home speed", &mut draft.home_latch_speed, 0.01, " × seek");
            });
            ui.add_space(10.0);
            if let Some(cmd) = home_test_section(ui, &s, &mut self.ht_axis, &mut self.ht_cycles, &mut self.ht_away) {
                self.machine.send(cmd);
            }
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!(
                    "Settings, tools and offsets are saved in {}. Macros: put M<n>.nc files in its macros folder.",
                    store::data_dir().display()
                ))
                .size(12.0)
                .color(MUTED),
            );
        });
        if import {
            self.import_profile(&s.config);
        }
        if dismiss {
            self.draft.import_notes.clear();
        }
        if apply {
            if let Some(c) = &self.draft.config {
                let base = self.draft.base.clone().unwrap_or_else(|| s.config.clone());
                let merged = merge_edits(&base, c, &s.config);
                self.machine.send(Command::SetConfig(Box::new(merged)));
            }
            self.draft.import_notes.clear();
        }
        if revert {
            self.draft.config = None;
            self.draft.base = None;
            self.draft.import_notes.clear();
        }
        if !dirty && self.draft.import_notes.is_empty() {
            // Track changes made elsewhere (jog speed, probe page).
            self.draft.config = None;
        }
    }

    fn import_profile(&mut self, _current: &MachineConfig) {
        self.pick(super::Picking::Profile);
    }

    pub(super) fn import_profile_from(&mut self, path: &std::path::Path, current: &MachineConfig) {
        let result = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| mach3::import(&String::from_utf8_lossy(&b), current));
        match result {
            Ok(import) => {
                self.draft.config = Some(import.config);
                self.draft.import_notes = import.notes;
            }
            Err(e) => self.draft.import_notes = vec![format!("Could not import {}: {e}", path.display())],
        }
    }
}

/// `current` with the settings that differ between `base` and `edited`
/// taken from `edited`, field by field.
fn merge_edits(base: &MachineConfig, edited: &MachineConfig, current: &MachineConfig) -> MachineConfig {
    fn merge(base: &serde_json::Value, edited: &serde_json::Value, current: &mut serde_json::Value) {
        use serde_json::Value;
        match (base, edited, current) {
            (Value::Object(b), Value::Object(e), Value::Object(c)) => {
                for (k, ev) in e {
                    match (b.get(k), c.get_mut(k)) {
                        (Some(bv), Some(cv)) => merge(bv, ev, cv),
                        (_, cv) => {
                            if b.get(k) != Some(ev) {
                                match cv {
                                    Some(cv) => *cv = ev.clone(),
                                    None => {
                                        c.insert(k.clone(), ev.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            (Value::Array(b), Value::Array(e), Value::Array(c)) if b.len() == e.len() && e.len() == c.len() => {
                for ((bv, ev), cv) in b.iter().zip(e).zip(c.iter_mut()) {
                    merge(bv, ev, cv);
                }
            }
            (b, e, c) => {
                if b != e {
                    *c = e.clone();
                }
            }
        }
    }
    let (Ok(b), Ok(e), Ok(mut c)) = (serde_json::to_value(base), serde_json::to_value(edited), serde_json::to_value(current)) else {
        return edited.clone();
    };
    merge(&b, &e, &mut c);
    serde_json::from_value(c).unwrap_or_else(|_| edited.clone())
}

/// The Z zero references (table, vise): which one is in use, and a big
/// button for each that touches the tool off on the setter and sets Z zero
/// there. Returns the job to run.
fn z_reference_buttons(ui: &mut egui::Ui, s: &machine::Snapshot) -> Option<ProbeJob> {
    let setter = &s.config.probing.setter;
    let in_use = match setter.active() {
        Some(r) => format!("Z zero: {} (setter top at Z{:.3})", r.name, r.setter_top),
        None => format!("Z zero: custom (setter top at Z{:.3})", setter.offset),
    };
    ui.label(RichText::new(in_use).size(16.0).color(GREEN).strong());
    let ready = matches!(s.mode, Mode::Idle) || (s.mode == Mode::Running && s.paused.is_some());
    let mut job = None;
    ui.horizontal_wrapped(|ui| {
        for (i, r) in setter.references.iter().enumerate() {
            // Table and vise get the big buttons; any others are smaller.
            let (size, min) = if i < 2 { (20.0, Vec2::new(170.0, 56.0)) } else { (15.0, Vec2::new(0.0, 36.0)) };
            let button = Button::new(RichText::new(format!("Set Z: {}", r.name.to_lowercase())).size(size).strong())
                .min_size(min);
            if ui
                .add_enabled(ready, button)
                .on_hover_text(format!(
                    "Measure the tool on the setter and set Z zero on the {} ({:.3} below the setter top). Tool changes keep it.",
                    r.name.to_lowercase(),
                    r.setter_top
                ))
                .clicked()
            {
                job = Some(ProbeJob::SetZ(i));
            }
        }
    });
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_keeps_changes_made_elsewhere() {
        let base = MachineConfig::dmc2();
        let mut edited = base.clone();
        edited.axes[0].max_velocity = 100.0;
        let mut current = base.clone();
        current.jog_speed = 0.8;
        current.probing.fast_feed = 321.0;
        let merged = merge_edits(&base, &edited, &current);
        assert_eq!(merged.axes[0].max_velocity, 100.0);
        assert_eq!(merged.jog_speed, 0.8);
        assert_eq!(merged.probing.fast_feed, 321.0);
    }
}

/// Homing test: re-home one axis many times and show how far each home
/// landed from the one before.
fn home_test_section(
    ui: &mut egui::Ui,
    s: &machine::Snapshot,
    axis: &mut usize,
    cycles: &mut usize,
    away: &mut f64,
) -> Option<Command> {
    let mut cmd = None;
    section_title(ui, "Homing test");
    ui.label(
        RichText::new(
            "Homes the axis, moves it away and homes it again. Each re-home shows how far the \
             switch landed from the last one (switch scatter plus any lost steps); 1 step = 5 µm.",
        )
        .size(12.0)
        .color(MUTED),
    );
    let running = s.home_test.as_ref().is_some_and(|t| t.running) && s.mode == Mode::Homing;
    ui.horizontal_wrapped(|ui| {
        for (i, _) in s.config.axes.iter().enumerate().filter(|(_, a)| a.enabled && a.home_input.is_some()) {
            ui.selectable_value(axis, i, AXIS_LETTERS[i].to_string());
        }
        ui.label(RichText::new("Re-homes").color(MUTED));
        ui.add(DragValue::new(cycles).range(1..=200));
        ui.label(RichText::new("Away").color(MUTED));
        ui.add(DragValue::new(away).speed(1.0).range(1.0..=400.0).suffix(" mm"));
        if ui.add_enabled(s.mode == Mode::Idle, Button::new("Run homing test")).clicked() {
            cmd = Some(Command::HomeTest { axis: *axis, cycles: *cycles, away: *away });
        }
    });
    let Some(t) = &s.home_test else { return cmd };
    let state = if running {
        format!("Running: {} of {}", t.shifts.len(), t.cycles)
    } else if t.running {
        format!("Stopped after {} of {}", t.shifts.len(), t.cycles)
    } else {
        "Finished".to_string()
    };
    ui.label(RichText::new(format!("{} axis · {state}", AXIS_LETTERS[t.axis])).strong());
    if let Some((m, sd, r)) = machine::home_test_stats(&t.shifts) {
        ui.label(format!(
            "Shift: mean {:+.1} µm, std dev {:.1} µm, range {:.1} µm",
            m * 1000.0,
            sd * 1000.0,
            r * 1000.0
        ));
    }
    if let Some((m, sd, _)) = machine::home_test_stats(&t.hysteresis) {
        ui.label(format!("Switch hysteresis: {:.3} mm ± {:.1} µm", m, sd * 1000.0));
    }
    if !t.shifts.is_empty() {
        let list: Vec<String> = t.shifts.iter().map(|d| format!("{:+.0}", d * 1000.0)).collect();
        ui.label(RichText::new(format!("Each re-home, µm: {}", list.join(" "))).size(12.0).color(MUTED));
    }
    if let Some(p) = &t.saved {
        ui.label(RichText::new(format!("Saved to {p}")).size(12.0).color(MUTED));
    }
    cmd
}
