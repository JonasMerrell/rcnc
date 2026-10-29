//! The right-hand tabs besides the program listing: tool table, work
//! offsets, probing and machine settings.

use eframe::egui::{self, Button, DragValue, RichText, ScrollArea, Vec2};
use gcode::{Tool, AXIS_LETTERS};
use machine::{mach3, store, Command, MachineConfig, Mode, ProbeJob};

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
    pub import_notes: Vec<String>,
    pub new_tool: u32,
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
        ScrollArea::vertical().auto_shrink([false, true]).max_height(ui.available_height() - 40.0).show(ui, |ui| {
            egui::Grid::new("tools").num_columns(5).striped(true).spacing([10.0, 6.0]).show(ui, |ui| {
                for h in ["Tool", "Diameter", "Length", "Description", ""] {
                    ui.label(RichText::new(h).color(MUTED).strong());
                }
                ui.end_row();
                for (n, tool) in &s.tools {
                    let mut t = tool.clone();
                    let mut changed = false;
                    let current = *n == s.tool;
                    ui.label(RichText::new(format!("T{n}")).strong().color(if current { GREEN } else { TEXT }));
                    changed |= num(ui, &mut t.diameter, 0.01, " mm");
                    changed |= num(ui, &mut t.length, 0.01, " mm");
                    changed |= ui.add(egui::TextEdit::singleline(&mut t.description).desired_width(240.0)).changed();
                    if ui.small_button("Remove").clicked() {
                        self.machine.send(Command::SetTool(*n, None));
                    } else if changed {
                        self.machine.send(Command::SetTool(*n, Some(t)));
                    }
                    ui.end_row();
                }
            });
        });
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
            changed |= ui.checkbox(&mut p.auto_zero, "Set work zero at the result").changed();
            egui::Grid::new("probe-settings").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                changed |= labelled(ui, "Search feed", &mut p.fast_feed, 1.0, " mm/min");
                changed |= labelled(ui, "Latch feed", &mut p.slow_feed, 0.5, " mm/min");
                changed |= labelled(ui, "Search distance", &mut p.max_distance, 0.5, " mm");
                changed |= labelled(ui, "Back-off", &mut p.latch, 0.05, " mm");
                changed |= labelled(ui, "Side clearance", &mut p.xy_clearance, 0.1, " mm");
                changed |= labelled(ui, "Edge depth", &mut p.z_depth, 0.1, " mm");
                changed |= labelled(ui, "Stylus diameter", &mut p.stylus_diameter, 0.01, " mm");
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
                changed |= labelled(ui, "Setter top (work Z)", &mut t.offset, 0.001, " mm");
            });
            if ui.button("Setter is under the spindle now").clicked() {
                t.x = s.machine[0];
                t.y = s.machine[1];
                changed = true;
            }
            changed |= ui
                .checkbox(&mut c.tool_change_touch_off, "At M6, Cycle Start touches the new tool off")
                .changed();
        });
        if changed && s.mode == Mode::Idle {
            self.machine.send(Command::SetConfig(Box::new(c)));
        }
    }

    pub(super) fn settings_page(&mut self, ui: &mut egui::Ui) {
        let s = self.snap.clone();
        let idle = matches!(s.mode, Mode::Idle | Mode::EStop | Mode::Disconnected | Mode::Alarm);
        let draft = self.draft.config.get_or_insert_with(|| s.config.clone());
        let dirty = *draft != s.config;
        let (mut apply, mut revert, mut import, mut dismiss) = (false, false, false, false);
        // Buttons first, so they can't be pushed off the bottom by the import summary.
        ui.horizontal(|ui| {
            section_title(ui, "Machine settings");
            import = ui.button("Import Mach3 profile…").clicked();
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
            egui::Grid::new("axes").num_columns(7).striped(true).spacing([8.0, 4.0]).show(ui, |ui| {
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
            ui.add_space(6.0);
            ui.checkbox(&mut draft.soft_limits, "Soft limits");
            ui.checkbox(&mut draft.constant_velocity, "Constant velocity (blend corners)");
            ui.checkbox(&mut draft.spindle_reversible, "Spindle can run in reverse (M4, G84)");
            egui::Grid::new("spindle").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                labelled(ui, "Max spindle speed", &mut draft.spindle_max_rpm, 100.0, " rpm");
                labelled(ui, "Spin-up wait", &mut draft.spindle_spin_up, 0.1, " s");
                labelled(ui, "Spin-down wait", &mut draft.spindle_spin_down, 0.1, " s");
                labelled(ui, "Corner tolerance", &mut draft.junction_deviation, 0.001, " mm");
            });
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
            if let Some(c) = self.draft.config.clone() {
                self.machine.send(Command::SetConfig(Box::new(c)));
            }
            self.draft.import_notes.clear();
        }
        if revert {
            self.draft.config = None;
            self.draft.import_notes.clear();
        }
        if !dirty && self.draft.import_notes.is_empty() {
            // Track changes made elsewhere (jog speed, probe page).
            self.draft.config = None;
        }
    }

    fn import_profile(&mut self, current: &MachineConfig) {
        let Some(path) = rfd::FileDialog::new().add_filter("Mach3 profile", &["xml"]).pick_file() else {
            return;
        };
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
