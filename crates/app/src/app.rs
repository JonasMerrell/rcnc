//! The main window: status bar, DRO and jog, toolpath preview, program
//! listing, cycle controls, overrides and MDI.

use eframe::egui::{
    self, Align, Button, Color32, CornerRadius, FontId, Key, Layout, Margin, PointerButton, RichText,
    ScrollArea, Stroke, Vec2,
};
use gcode::{Spindle, AXIS_LETTERS};
use machine::{Command, ControllerHandle, Mode, Snapshot};
use std::sync::Arc;

use crate::preview::{self, Camera, Toolpath};
use crate::sheet::{self, Sheets, SheetView};

mod pages;
mod toolchange;
use pages::{SettingsDraft, Tab};

mod palette {
    use eframe::egui::Color32;
    pub const BG: Color32 = Color32::from_rgb(0x15, 0x17, 0x1c);
    pub const PANEL: Color32 = Color32::from_rgb(0x1d, 0x20, 0x27);
    pub const RAISED: Color32 = Color32::from_rgb(0x27, 0x2b, 0x34);
    pub const BORDER: Color32 = Color32::from_rgb(0x34, 0x39, 0x44);
    pub const TEXT: Color32 = Color32::from_rgb(0xe6, 0xe8, 0xec);
    pub const MUTED: Color32 = Color32::from_rgb(0x8b, 0x92, 0xa0);
    pub const ACCENT: Color32 = Color32::from_rgb(0x4d, 0x9c, 0xf8);
    pub const GREEN: Color32 = Color32::from_rgb(0x3f, 0xb9, 0x6b);
    pub const AMBER: Color32 = Color32::from_rgb(0xf0, 0xa8, 0x30);
    pub const RED: Color32 = Color32::from_rgb(0xe5, 0x48, 0x4d);
}
use palette::*;

/// What the big centre area shows.
#[derive(Clone, Copy, PartialEq)]
enum View {
    Toolpath,
    Sheet,
}

#[derive(Clone, Copy, PartialEq)]
enum JogStep {
    Continuous,
    Step(f64),
}

pub struct App {
    machine: ControllerHandle,
    snap: Arc<Snapshot>,
    toolpath: Toolpath,
    /// The program and the tools/offsets the preview was built for.
    toolpath_for: (usize, u64),
    /// A preview being built in the background.
    toolpath_next: Option<std::sync::mpsc::Receiver<Toolpath>>,
    /// The program changed: fit the view and look for its sheet once it's built.
    toolpath_new_program: bool,
    /// When the Reset/E-stop button last changed what it does.
    reset_label: (&'static str, std::time::Instant),
    /// The controller clock last seen moving, and when (wall clock).
    alive: (f64, std::time::Instant),
    camera: Camera,
    jog_step: JogStep,
    jog_held: Option<(usize, bool)>,
    /// The held jog came from a key, so the on-screen pad must not end it.
    jog_by_key: bool,
    mdi: String,
    mdi_history: Vec<String>,
    mdi_cursor: Option<usize>,
    follow_line: bool,
    tab: Tab,
    draft: SettingsDraft,
    /// The opened file and its modification time, to reload it when the CAM rewrites it.
    program_file: Option<(std::path::PathBuf, Option<std::time::SystemTime>)>,
    file_checked: f64,
    /// The tool change window's "measure" checkbox.
    tc_measure: Option<bool>,
    view: View,
    sheets: Arc<Sheets>,
    /// The loaded program's setup sheet and its modification time.
    sheet: Option<(std::path::PathBuf, Option<std::time::SystemTime>)>,
    sheet_view: SheetView,
    /// Where the sheet goes this frame, if it's on show.
    sheet_rect: Option<egui::Rect>,
    /// Start of each line of the loaded program, worked out once per
    /// program rather than every frame (a CAM file can be a million lines).
    program_lines: (usize, Vec<usize>),
    /// The window's close button was pressed while the machine was busy.
    confirm_quit: bool,
    /// Quit was confirmed: let the window close.
    quitting: bool,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        machine: ControllerHandle,
        sheets: Arc<Sheets>,
        web_data: std::path::PathBuf,
    ) -> Self {
        apply_theme(&cc.egui_ctx);
        machine.send(Command::Connect);
        let snap = machine.snapshot();
        App {
            machine,
            snap,
            toolpath: Toolpath::default(),
            toolpath_for: (0, 0),
            toolpath_next: None,
            toolpath_new_program: false,
            reset_label: ("", std::time::Instant::now()),
            alive: (f64::NAN, std::time::Instant::now()),
            camera: Camera::default(),
            jog_step: JogStep::Continuous,
            jog_held: None,
            jog_by_key: false,
            mdi: String::new(),
            mdi_history: Vec::new(),
            mdi_cursor: None,
            follow_line: true,
            tab: Tab::Program,
            draft: SettingsDraft::default(),
            program_file: None,
            file_checked: 0.0,
            tc_measure: None,
            view: View::Toolpath,
            sheets,
            sheet: None,
            sheet_view: SheetView::new(web_data),
            sheet_rect: None,
            program_lines: (0, Vec::new()),
            confirm_quit: false,
            quitting: false,
        }
    }

    pub fn with_program(mut self, program: Option<std::path::PathBuf>) -> Self {
        if let Some(p) = program {
            self.load_path(p);
        }
        self
    }

    fn load_path(&mut self, path: std::path::PathBuf) {
        match std::fs::read(&path) {
            Ok(bytes) => {
                let text = String::from_utf8_lossy(&bytes).into_owned();
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.machine.send(Command::Load { name, text });
                let modified = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
                self.program_file = Some((path, modified));
            }
            Err(e) => eprintln!("could not read {}: {e}", path.display()),
        }
    }

    /// Reload the opened file when it changes on disk (the CAM posted it
    /// again), unless the machine is busy; then it waits until it's idle.
    fn watch_file(&mut self, now: f64) {
        if now - self.file_checked < 1.0 || !matches!(self.snap.mode, Mode::Idle | Mode::EStop | Mode::Disconnected) {
            return;
        }
        self.file_checked = now;
        if let Some((path, seen)) = &mut self.sheet {
            let modified = std::fs::metadata(&*path).and_then(|m| m.modified()).ok();
            if modified.is_some() && modified != *seen {
                *seen = modified;
                self.sheet_view.reload();
            }
        }
        let Some((path, seen)) = &self.program_file else { return };
        let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        if modified.is_some() && modified != *seen {
            let path = path.clone();
            self.load_path(path);
        }
    }

    /// Find the setup sheet for a newly loaded program: the one sent with
    /// it, or one saved next to its file.
    fn find_sheet(&mut self) {
        let name = self.snap.program_name.clone().unwrap_or_default();
        let path = match self.sheets.take(&name) {
            Some(sent) => sent,
            None => self
                .program_file
                .as_ref()
                .filter(|(p, _)| p.file_name().is_some_and(|n| n.to_string_lossy() == name))
                .and_then(|(p, _)| sheet::beside(p)),
        };
        if path.is_some() && path.as_ref() != self.sheet.as_ref().map(|(p, _)| p) {
            self.view = View::Sheet;
        }
        self.set_sheet(path);
    }

    fn set_sheet(&mut self, path: Option<std::path::PathBuf>) {
        self.sheet = path.map(|p| {
            let modified = std::fs::metadata(&p).and_then(|m| m.modified()).ok();
            (p, modified)
        });
    }

    /// File dialogs block the window until they close: Stop, Esc and jog
    /// key-ups would go nowhere. Only while nothing moves.
    fn can_open_dialogs(&self) -> bool {
        matches!(self.snap.mode, Mode::Idle | Mode::EStop | Mode::Disconnected | Mode::Alarm)
    }

    fn open_sheet(&mut self) {
        if !self.can_open_dialogs() {
            return;
        }
        let mut dialog = rfd::FileDialog::new().add_filter("Web page", &["html", "htm"]);
        if let Some(dir) = self.program_file.as_ref().and_then(|(p, _)| p.parent()) {
            dialog = dialog.set_directory(dir);
        }
        if let Some(path) = dialog.pick_file() {
            self.set_sheet(Some(path));
            self.view = View::Sheet;
        }
    }

    fn open_file(&mut self) {
        if !self.can_open_dialogs() {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "G-code",
                &["nc", "tap", "ngc", "gcode", "txt", "cnc", "nc1"],
            )
            .add_filter("All files", &["*"])
            .pick_file()
        else {
            return;
        };
        self.load_path(path);
    }

    fn keyboard(&mut self, ctx: &egui::Context) {
        let input = ctx.input(|i| i.clone());
        let moving = matches!(self.snap.mode, Mode::Running | Mode::Jog | Mode::Homing | Mode::Probing);
        // Esc and Space work even while a text box has the keyboard.
        if input.key_pressed(Key::Escape) {
            self.machine.send(Command::Stop);
        }
        if moving && input.key_pressed(Key::Space) {
            self.machine.send(Command::FeedHold);
            // Not typed into the text box as well.
            ctx.input_mut(|i| {
                i.events.retain(|e| !matches!(e, egui::Event::Text(t) if t == " "));
                i.consume_key(egui::Modifiers::NONE, Key::Space);
            });
        }
        // Mach3-style jog keys: arrows for X/Y, Page Up/Down for Z.
        let keys = [
            (Key::ArrowRight, 0, true),
            (Key::ArrowLeft, 0, false),
            (Key::ArrowUp, 1, true),
            (Key::ArrowDown, 1, false),
            (Key::PageUp, 2, true),
            (Key::PageDown, 2, false),
        ];
        // A key jog ends when its key is up, even if the release never
        // arrived (Alt-Tab, a dialog, a click in the sheet).
        if self.jog_by_key {
            let held = keys
                .iter()
                .find(|(_, a, p)| self.jog_held == Some((*a, *p)))
                .is_some_and(|(k, _, _)| input.key_down(*k));
            if !held || !input.focused {
                self.stop_jog();
            }
        }
        // Tell the controller the jog is still held; it stops it otherwise.
        if self.jog_held.is_some() {
            self.machine.send(Command::JogAlive);
        }
        if ctx.wants_keyboard_input() {
            return;
        }
        for (key, axis, positive) in keys {
            // Ignore OS key repeat: a held key is one continuous jog, or one step.
            let first_press = input.events.iter().any(|e| {
                matches!(e, egui::Event::Key { key: k, pressed: true, repeat: false, .. } if *k == key)
            });
            if first_press && self.jog_held.is_none() {
                self.start_jog(axis, positive);
                self.jog_by_key = self.jog_held.is_some();
            }
            if input.key_released(key) && self.jog_held == Some((axis, positive)) {
                self.stop_jog();
            }
        }
        if input.modifiers.alt && input.key_pressed(Key::R) {
            self.machine.send(Command::CycleStart);
        }
    }

    fn start_jog(&mut self, axis: usize, positive: bool) {
        let step = match self.jog_step {
            JogStep::Continuous => None,
            JogStep::Step(d) => Some(d),
        };
        self.machine.send(Command::Jog {
            axis,
            positive,
            step,
        });
        if step.is_none() {
            self.jog_held = Some((axis, positive));
        }
    }

    fn stop_jog(&mut self) {
        self.jog_by_key = false;
        if self.jog_held.take().is_some() {
            self.machine.send(Command::JogStop);
        }
    }
}

impl eframe::App for App {
    /// No Tab key for egui. Tab moves the keyboard focus onto buttons, and a
    /// focused button is pressed by Space or Enter: Tab then Space would press
    /// Reset (or Cycle Start) instead of feed hold, and the jog keys would
    /// move the focus instead of the machine. Clicks still focus text boxes.
    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        raw_input.events.retain(|e| !matches!(e, egui::Event::Key { key: Key::Tab, .. }));
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.snap = self.machine.snapshot();
        if self.snap.time != self.alive.0 {
            self.alive = (self.snap.time, std::time::Instant::now());
        }
        self.watch_file(ctx.input(|i| i.time));
        self.update_preview(ctx);
        self.keyboard(ctx);
        self.guard_close(ctx);
        if self.alive.1.elapsed() > std::time::Duration::from_millis(500) {
            egui::TopBottomPanel::top("dead")
                .frame(egui::Frame::new().fill(RED).inner_margin(Margin::symmetric(14, 8)))
                .show(ctx, |ui| {
                    ui.label(
                        RichText::new(
                            "CONTROLLER NOT RESPONDING: the screen is not live and its buttons do nothing. \
                             Use the machine's E-stop, then restart rcnc.",
                        )
                        .color(Color32::WHITE)
                        .strong(),
                    );
                });
        }

        egui::TopBottomPanel::top("status")
            .frame(panel_frame().inner_margin(Margin::symmetric(14, 10)))
            .show(ctx, |ui| self.status_bar(ui));
        egui::TopBottomPanel::bottom("controls")
            .frame(panel_frame().inner_margin(Margin::symmetric(14, 12)))
            .show(ctx, |ui| self.controls(ui));
        egui::SidePanel::left("dro")
            .exact_width(360.0)
            .resizable(false)
            .frame(panel_frame().inner_margin(Margin::same(14)))
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    self.dro(ui);
                    ui.add_space(8.0);
                    self.jog(ui);
                });
            });
        // Capped so the toolpath preview keeps some room on small screens.
        let right_max = (ctx.content_rect().width() * 0.36).max(280.0);
        egui::SidePanel::right("program")
            .default_width(380.0_f32.min(right_max))
            .max_width(right_max)
            .min_width(280.0)
            .frame(panel_frame().inner_margin(Margin::same(14)))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (tab, label) in Tab::ALL {
                        if ui.selectable_label(self.tab == tab, label).clicked() {
                            self.tab = tab;
                        }
                    }
                });
                ui.separator();
                match self.tab {
                    Tab::Program => self.program(ui),
                    Tab::Tools => self.tools_page(ui),
                    Tab::Offsets => self.offsets_page(ui),
                    Tab::Probe => self.probe_page(ui),
                    Tab::Settings => self.settings_page(ui),
                }
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(Margin::same(10)))
            .show(ctx, |ui| match self.view {
                View::Toolpath => self.preview(ui),
                View::Sheet => self.sheet_page(ui),
            });
        self.tool_change_window(ctx);

        // The sheet is a native window over the centre area, drawn above
        // anything egui puts there: keep it out of the way of the tool change
        // window and of open menus.
        let covered = self.snap.tool_change.is_some() || egui::Popup::is_any_open(ctx);
        match (self.sheet_rect.take(), &self.sheet) {
            (Some(rect), Some((path, _))) if !covered => {
                self.sheet_view.show(frame, ctx, rect, path, &self.machine)
            }
            _ => self.sheet_view.hide(),
        }
        if self.sheet_view.keep_keyboard(frame) {
            // A click in the sheet is a click away from any text box, as it
            // would be on the rest of the window: Space is feed hold again, not
            // a space typed into MDI.
            ctx.memory_mut(|m| {
                if let Some(id) = m.focused() {
                    m.surrender_focus(id);
                }
            });
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }
}

/// Byte offset where each line starts (`str::lines` rules: a trailing
/// newline doesn't start another line).
fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = Vec::new();
    if !text.is_empty() {
        starts.push(0);
    }
    starts.extend(text.match_indices('\n').map(|(i, _)| i + 1).filter(|&i| i < text.len()));
    starts
}

impl App {
    /// Closing the window mid-cut would leave the board with the spindle on.
    /// While anything runs, ask first; on Quit the machine is stopped and
    /// switched off (in `main`, once the window has gone).
    fn guard_close(&mut self, ctx: &egui::Context) {
        let s = &self.snap;
        let busy = matches!(s.mode, Mode::Running | Mode::Jog | Mode::Homing | Mode::Probing)
            || s.spindle != Spindle::Off
            || s.flood
            || s.mist;
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            if busy {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.confirm_quit = true;
            } else {
                self.quitting = true;
            }
        }
        if !self.confirm_quit {
            return;
        }
        egui::Window::new("Quit rcnc?")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(if busy {
                    "The machine is running. Quitting stops all motion and switches the spindle, coolant and drives off."
                } else {
                    "The machine has stopped."
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Stop and quit").color(RED).strong()).clicked() {
                        self.confirm_quit = false;
                        self.quitting = true;
                        self.machine.send(Command::Stop);
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    if ui.button("Keep running").clicked() {
                        self.confirm_quit = false;
                    }
                });
            });
    }

    fn program_id(s: &Snapshot) -> usize {
        std::sync::Arc::as_ptr(&s.program) as usize
    }

    /// What, besides the program, changes the preview: tools, offsets, the tool in use.
    fn setup_hash(s: &Snapshot) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for (n, t) in &s.tools {
            (n, t.length.to_bits(), t.diameter.to_bits()).hash(&mut h);
        }
        for (k, v) in &s.params {
            (k, v.to_bits()).hash(&mut h);
        }
        (s.coord_system, s.tool, s.tool_length_offset.to_bits(), s.macros.len()).hash(&mut h);
        h.finish()
    }

    /// Rebuild the preview, on its own thread, when the program or its setup changes.
    fn update_preview(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.toolpath_next {
            match rx.try_recv() {
                Ok(path) => {
                    self.toolpath = path;
                    self.toolpath_next = None;
                    if std::mem::take(&mut self.toolpath_new_program) {
                        self.camera.fit();
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => self.toolpath_next = None,
            }
        }
        let program = Self::program_id(&self.snap);
        let new_program = program != self.toolpath_for.0;
        // Offsets change while a program runs (G10, G92): wait until it's done.
        let busy = matches!(self.snap.mode, Mode::Running | Mode::Probing | Mode::Homing);
        let key = (program, Self::setup_hash(&self.snap));
        if new_program || (key != self.toolpath_for && !busy) {
            self.toolpath_for = key;
            if new_program {
                self.toolpath_new_program = true;
                self.find_sheet();
            }
            self.toolpath_next = Some(Toolpath::build_in_background(
                self.snap.program.clone(),
                preview::Setup::from_snapshot(&self.snap),
                ctx.clone(),
            ));
        }
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let s = &self.snap;
        ui.horizontal(|ui| {
            let (label, color) = match s.mode {
                Mode::Disconnected => ("DISCONNECTED", MUTED),
                Mode::EStop => ("RESET", RED),
                Mode::Idle => ("READY", GREEN),
                Mode::Jog => ("JOG", ACCENT),
                Mode::Homing => ("HOMING", ACCENT),
                _ if s.stopping => ("STOPPING", AMBER),
                Mode::Running if s.paused.is_some() => ("PAUSED", AMBER),
                Mode::Running if s.hold && s.feed_override == 0.0 => ("FEED HOLD (0%)", AMBER),
                Mode::Running if s.hold => ("FEED HOLD", AMBER),
                Mode::Running => ("RUNNING", GREEN),
                Mode::Probing if s.tool_change.is_some() => ("TOOL CHANGE", AMBER),
                Mode::Probing => ("PROBING", ACCENT),
                Mode::Alarm => ("ALARM", RED),
            };
            badge(ui, label, color);
            ui.add_space(8.0);

            // One button that is E-stop or Reset, and a separate Connect: one
            // press can never connect and enable the drives.
            let resettable = s.mode == Mode::EStop || s.mode == Mode::Alarm;
            let label = if resettable { "Reset" } else { "E-stop" };
            if self.reset_label.0 != label {
                self.reset_label = (label, std::time::Instant::now());
            }
            let r = ui.add(
                big_button(label, if resettable { GREEN } else { RED }).min_size(Vec2::new(96.0, 32.0)),
            );
            if resettable {
                // Leaving reset takes a mouse click (not a key on a focused
                // button), and not the second click of a double click that
                // started on the E-stop or Connect button.
                let settled = self.reset_label.1.elapsed() > std::time::Duration::from_millis(750);
                if r.clicked_by(PointerButton::Primary) && settled {
                    self.machine.send(Command::Reset);
                }
            } else if r.clicked() {
                self.machine.send(Command::EStop);
            }
            if s.mode == Mode::Disconnected && ui.button("Connect").clicked() {
                self.reset_label = ("Reset", std::time::Instant::now());
                self.machine.send(Command::Connect);
            }
            ui.add_space(12.0);
            for (i, homed) in s.homed.iter().enumerate().take(3) {
                let c = if *homed { GREEN } else { MUTED };
                ui.label(
                    RichText::new(format!(
                        "{} {}",
                        AXIS_LETTERS[i],
                        if *homed { "homed" } else { "not homed" }
                    ))
                    .color(c)
                    .size(13.0),
                );
            }
            ui.add_space(12.0);
            pill(ui, &format!("G{}", 54 + s.coord_system));
            // During a change the interpreter already has the new tool; show what's in the spindle.
            let tool = s.tool_change.as_ref().map_or(s.tool, |t| t.from);
            pill(ui, &format!("T{tool}"));
            pill(ui, &s.modal);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let board = s
                    .board
                    .as_ref()
                    .map(|b| b.name.as_str())
                    .unwrap_or("No board");
                ui.label(RichText::new(board).color(MUTED).size(13.0));
                if let Some(alarm) = &s.alarm {
                    ui.label(RichText::new(alarm).color(RED).strong());
                } else if let Some(msg) = &s.paused {
                    ui.label(RichText::new(msg).color(AMBER).strong());
                }
            });
        });
    }

    fn dro(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Position");
        let s = self.snap.clone();
        let ready = s.mode == Mode::Idle;
        for axis in 0..3 {
            // One line per axis so the jog pad fits below on a 720p screen.
            egui::Frame::new()
                .fill(RAISED)
                .corner_radius(CornerRadius::same(8))
                .inner_margin(Margin::symmetric(10, 2))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(AXIS_LETTERS[axis].to_string())
                                .font(FontId::monospace(24.0))
                                .color(ACCENT)
                                .strong(),
                        );
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            ui.set_width(150.0);
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.with_layout(Layout::top_down(Align::Max), |ui| {
                                ui.label(
                                    RichText::new(format!("{:.3}", s.work[axis]))
                                        .font(FontId::monospace(24.0))
                                        .color(TEXT),
                                );
                                ui.label(
                                    RichText::new(format!("mach {:.3}", s.machine[axis]))
                                        .font(FontId::monospace(11.0))
                                        .color(MUTED),
                                );
                            });
                        });
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.spacing_mut().button_padding = Vec2::new(8.0, 3.0);
                            if ui.add_enabled(ready, Button::new("Home")).clicked() {
                                self.machine.send(Command::Home(Some(axis)));
                            }
                            if ui.add_enabled(ready, Button::new("Zero")).clicked() {
                                self.machine.send(Command::ZeroAxis(axis));
                            }
                        });
                    });
                });
            ui.add_space(4.0);
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    ready,
                    Button::new("Ref All Home").min_size(Vec2::new(150.0, 28.0)),
                )
                .clicked()
            {
                self.machine.send(Command::Home(None));
            }
            if ui
                .add_enabled(
                    ready,
                    Button::new("Go to Work Zero").min_size(Vec2::new(150.0, 28.0)),
                )
                .clicked()
            {
                // Up to the top first, so the move over can't drag the tool through the part or a clamp.
                self.machine.send(Command::Mdi("G53 G0 Z0\nG0 X0 Y0".into()));
            }
        });
        ui.add_space(6.0);
        let spindle = match s.spindle {
            Spindle::Off => "off".to_string(),
            Spindle::Cw => format!("{:.0} CW", s.rpm),
            Spindle::Ccw => format!("{:.0} CCW", s.rpm),
        };
        let coolant = if s.flood {
            "flood"
        } else if s.mist {
            "mist"
        } else {
            "off"
        };
        ui.columns(4, |cols| {
            stat(&mut cols[0], "Feed", &format!("{:.0}", s.velocity));
            stat(&mut cols[1], "Spindle", &spindle);
            stat(&mut cols[2], "Coolant", coolant);
            stat(
                &mut cols[3],
                "Probe",
                if s.probe_active { "touching" } else { "clear" },
            );
        });
    }

    fn jog(&mut self, ui: &mut egui::Ui) {
        section_title(ui, "Jog");
        let ready = matches!(self.snap.mode, Mode::Idle | Mode::Jog);
        ui.horizontal_wrapped(|ui| {
            for step in [
                JogStep::Continuous,
                JogStep::Step(0.01),
                JogStep::Step(0.1),
                JogStep::Step(1.0),
                JogStep::Step(10.0),
            ] {
                let label = match step {
                    JogStep::Continuous => "Cont".to_string(),
                    JogStep::Step(d) => format!("{d}"),
                };
                if ui
                    .selectable_label(self.jog_step == step, RichText::new(label).size(14.0))
                    .clicked()
                {
                    self.jog_step = step;
                }
            }
        });
        let mut speed = self.snap.jog_speed * 100.0;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Speed").color(MUTED));
            if ui
                .add(
                    egui::Slider::new(&mut speed, 1.0..=100.0)
                        .suffix(" %")
                        .integer(),
                )
                .changed()
            {
                self.machine.send(Command::SetJogSpeed(speed / 100.0));
            }
        });
        ui.add_space(4.0);

        let size = Vec2::new(64.0, 38.0);
        let mut pad = |ui: &mut egui::Ui, label: &str, axis: usize, positive: bool| {
            let r = ui.add_enabled(
                ready,
                Button::new(RichText::new(label).size(17.0).strong()).min_size(size),
            );
            if r.is_pointer_button_down_on() && self.jog_held.is_none() && r.contains_pointer() {
                if self.jog_step == JogStep::Continuous {
                    self.start_jog(axis, positive);
                }
            } else if self.jog_held == Some((axis, positive)) && !self.jog_by_key && !r.is_pointer_button_down_on() {
                self.stop_jog();
            }
            if r.clicked() && self.jog_step != JogStep::Continuous {
                self.start_jog(axis, positive);
            }
        };
        egui::Grid::new("jogpad")
            .spacing(Vec2::new(8.0, 6.0))
            .show(ui, |ui| {
                ui.label("");
                pad(ui, "Y+", 1, true);
                ui.label("");
                pad(ui, "Z+", 2, true);
                ui.end_row();
                pad(ui, "X-", 0, false);
                ui.label("");
                pad(ui, "X+", 0, true);
                ui.label("");
                ui.end_row();
                ui.label("");
                pad(ui, "Y-", 1, false);
                ui.label("");
                pad(ui, "Z-", 2, false);
                ui.end_row();
            });
        ui.add_space(6.0);
        ui.label(
            RichText::new("Keyboard: arrows jog X/Y, Page Up/Down jog Z")
                .size(12.0)
                .color(MUTED),
        );
    }

    fn program(&mut self, ui: &mut egui::Ui) {
        let s = self.snap.clone();
        ui.horizontal(|ui| {
            section_title(ui, s.program_name.as_deref().unwrap_or("No program"));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let can_load = self.can_open_dialogs();
                if ui.add_enabled(can_load, Button::new("Open…")).clicked() {
                    self.open_file();
                }
                ui.checkbox(&mut self.follow_line, "Follow");
            });
        });
        if !self.toolpath.segs.is_empty() {
            ui.label(
                RichText::new(format!(
                    "X {:.1}…{:.1}   Y {:.1}…{:.1}   Z {:.1}…{:.1}",
                    self.toolpath.min[0],
                    self.toolpath.max[0],
                    self.toolpath.min[1],
                    self.toolpath.max[1],
                    self.toolpath.min[2],
                    self.toolpath.max[2]
                ))
                .size(12.0)
                .color(MUTED),
            );
        }
        ui.add_space(6.0);
        let id = Self::program_id(&s);
        if self.program_lines.0 != id {
            self.program_lines = (id, line_starts(&s.program));
        }
        let starts = &self.program_lines.1;
        let line_text = |i: usize| -> &str {
            let end = starts.get(i + 1).map(|e| e - 1).unwrap_or(s.program.len());
            s.program.get(starts[i]..end).unwrap_or("").trim_end_matches('\r')
        };
        let line_count = starts.len();
        let row_h = 18.0;
        egui::Frame::new()
            .fill(BG)
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(6))
            .show(ui, |ui| {
                let mut area = ScrollArea::vertical().auto_shrink([false, false]);
                if let (true, Some(line)) = (self.follow_line, s.line) {
                    area = area.vertical_scroll_offset((line as f32 * row_h - 120.0).max(0.0));
                }
                area.show_rows(ui, row_h, line_count, |ui, range| {
                    for i in range {
                        let active = s.line == Some(i);
                        let text = RichText::new(format!("{:>5}  {}", i + 1, line_text(i)))
                            .font(FontId::monospace(13.0));
                        let text = if active {
                            text.color(Color32::BLACK).background_color(AMBER)
                        } else {
                            text.color(TEXT)
                        };
                        ui.allocate_ui_with_layout(
                            Vec2::new(ui.available_width(), row_h),
                            Layout::left_to_right(Align::Center),
                            |ui| {
                                ui.add(egui::Label::new(text).truncate());
                            },
                        );
                    }
                });
            });
    }

    fn view_switch(&mut self, ui: &mut egui::Ui) {
        for (view, label) in [(View::Toolpath, "Toolpath"), (View::Sheet, "Setup sheet")] {
            if ui.selectable_label(self.view == view, label).clicked() {
                self.view = view;
            }
        }
        ui.separator();
    }

    fn sheet_page(&mut self, ui: &mut egui::Ui) {
        // Wraps rather than widening the panel: the sheet takes the panel's width.
        ui.horizontal_wrapped(|ui| {
            self.view_switch(ui);
            if ui.add_enabled(self.can_open_dialogs(), Button::new("Open sheet…")).clicked() {
                self.open_sheet();
            }
            if let Some((path, _)) = self.sheet.clone() {
                if self.sheet_view.error().is_none() && ui.button("Print").clicked() {
                    self.sheet_view.print();
                }
                if ui.button("Open in browser").clicked() {
                    sheet::open_in_browser(&path);
                }
                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                ui.label(RichText::new(name).size(12.0).color(MUTED));
            }
        });
        ui.add_space(4.0);
        // Kept inside the panel: the sheet is a native window and would cover the panels beside it.
        let rect = ui.available_rect_before_wrap().intersect(ui.clip_rect());
        ui.allocate_rect(rect, egui::Sense::hover());
        let note = |ui: &mut egui::Ui, text: &str| {
            ui.put(rect, egui::Label::new(RichText::new(text).color(MUTED)).wrap());
        };
        if self.sheet.is_none() {
            note(
                ui,
                "No setup sheet for this program.\n\nSend it from the CAM with its setup sheet, save the sheet next to the G-code \
                 (part.html or \"part setup sheet.html\" for part.nc), or open one.",
            );
        } else if let Some(e) = self.sheet_view.error() {
            note(ui, &format!("{e}\n\nUse Open in browser to see it."));
        } else {
            ui.painter().rect_filled(rect, CornerRadius::same(6), Color32::WHITE);
            self.sheet_rect = Some(rect);
        }
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            self.view_switch(ui);
            for (label, f) in [
                ("Top", Camera::top as fn(&mut Camera)),
                ("Front", Camera::front),
                ("Iso", Camera::iso),
                ("Fit", Camera::fit),
            ] {
                if ui.button(label).clicked() {
                    f(&mut self.camera);
                }
            }
            if !self.toolpath.segs.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "Cut {:.0} mm   Rapid {:.0} mm",
                        self.toolpath.feed_length, self.toolpath.rapid_length
                    ))
                    .size(12.0)
                    .color(MUTED),
                );
            }
        });
        let s = &self.snap;
        // The path is in work coordinates; show the envelope in work coordinates too.
        let off: [f64; 3] = std::array::from_fn(|i| s.machine[i] - s.work[i]);
        let envelope = s.config.soft_limits.then(|| {
            let a = &s.config.axes;
            (
                std::array::from_fn(|i| a[i].soft_min - off[i]),
                std::array::from_fn(|i| a[i].soft_max - off[i]),
            )
        });
        let colors = preview::Colors {
            feed: ACCENT,
            rapid: Color32::from_rgb(0x6b, 0x72, 0x80),
            active: AMBER,
            tool: GREEN,
            envelope: BORDER,
            background: PANEL,
        };
        preview::draw(
            ui,
            &mut self.camera,
            &self.toolpath,
            s.work,
            s.line,
            envelope,
            &colors,
        );
    }

    fn controls(&mut self, ui: &mut egui::Ui) {
        let s = self.snap.clone();
        if let (Mode::Running, Some(msg)) = (s.mode, &s.paused) {
            ui.horizontal(|ui| {
                ui.label(RichText::new(msg).color(AMBER).strong().size(15.0));
                if ui
                    .add_enabled(!s.probe_active, Button::new("Touch off tool"))
                    .on_hover_text("Measure the loaded tool on the tool setter and set work Z")
                    .clicked()
                {
                    self.machine.send(Command::Probe(machine::ProbeJob::ToolTouchOff));
                }
            });
            ui.add_space(4.0);
        }
        ui.horizontal(|ui| {
            let can_start = matches!(s.mode, Mode::Idle)
                || (s.mode == Mode::Running && (s.hold || s.paused.is_some()))
                || s.tool_change.as_ref().is_some_and(|t| t.stage == machine::ToolChangeStage::Loading);
            if ui
                .add_enabled(
                    can_start,
                    big_button("Cycle Start", GREEN).min_size(Vec2::new(150.0, 56.0)),
                )
                .clicked()
            {
                self.machine.send(Command::CycleStart);
            }
            if ui
                .add_enabled(
                    matches!(s.mode, Mode::Running | Mode::Homing | Mode::Probing) && !s.hold,
                    big_button("Feed Hold", AMBER).min_size(Vec2::new(130.0, 56.0)),
                )
                .clicked()
            {
                self.machine.send(Command::FeedHold);
            }
            let can_stop = matches!(s.mode, Mode::Running | Mode::Jog | Mode::Homing | Mode::Probing | Mode::Disconnected);
            if ui
                .add_enabled(
                    can_stop,
                    big_button("Stop", RED).min_size(Vec2::new(110.0, 56.0)),
                )
                .clicked()
            {
                self.machine.send(Command::Stop);
            }
            ui.add_space(16.0);

            ui.vertical(|ui| {
                override_slider(ui, "Feed", s.feed_override, 0.0..=2.0, |v| {
                    self.machine.send(Command::SetFeedOverride(v))
                });
                override_slider(ui, "Rapid", s.rapid_override, 0.0..=1.0, |v| {
                    self.machine.send(Command::SetRapidOverride(v))
                });
                override_slider(ui, "Spindle", s.spindle_override, 0.5..=1.5, |v| {
                    self.machine.send(Command::SetSpindleOverride(v))
                });
            });
            ui.add_space(16.0);

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("MDI").color(MUTED).strong());
                    let edit = ui.add(
                        egui::TextEdit::singleline(&mut self.mdi)
                            .font(FontId::monospace(15.0))
                            .hint_text("G0 X0 Y0")
                            .desired_width(ui.available_width() - 70.0),
                    );
                    if edit.has_focus() {
                        let (up, down) = ui.input(|i| {
                            (i.key_pressed(Key::ArrowUp), i.key_pressed(Key::ArrowDown))
                        });
                        if up && !self.mdi_history.is_empty() {
                            let c = self
                                .mdi_cursor
                                .map(|c| c.saturating_sub(1))
                                .unwrap_or(self.mdi_history.len() - 1);
                            self.mdi_cursor = Some(c);
                            self.mdi = self.mdi_history[c].clone();
                        }
                        if down {
                            if let Some(c) = self.mdi_cursor {
                                if c + 1 < self.mdi_history.len() {
                                    self.mdi_cursor = Some(c + 1);
                                    self.mdi = self.mdi_history[c + 1].clone();
                                } else {
                                    self.mdi_cursor = None;
                                    self.mdi.clear();
                                }
                            }
                        }
                    }
                    let submit = edit.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                    if (submit || ui.button("Send").clicked()) && !self.mdi.trim().is_empty() {
                        let line = std::mem::take(&mut self.mdi);
                        self.machine.send(Command::Mdi(line.clone()));
                        self.mdi_history.push(line);
                        self.mdi_cursor = None;
                        edit.request_focus();
                    }
                });
                // Exactly two lines, so a long log can't push the rest of the window around.
                let row = ui.fonts_mut(|f| f.row_height(&FontId::proportional(12.0)));
                let height = 2.0 * row + ui.spacing().item_spacing.y;
                ScrollArea::vertical()
                    .max_height(height)
                    .min_scrolled_height(height)
                    .auto_shrink([false, true])
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for msg in s
                            .log
                            .iter()
                            .rev()
                            .take(30)
                            .collect::<Vec<_>>()
                            .into_iter()
                            .rev()
                        {
                            let color = if msg.starts_with("Alarm") || msg.starts_with("E-stop") {
                                RED
                            } else {
                                MUTED
                            };
                            ui.label(RichText::new(msg).size(12.0).color(color));
                        }
                    });
            });
        });
    }
}

fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.extreme_bg_color = BG;
    v.override_text_color = Some(TEXT);
    v.selection.bg_fill = ACCENT.linear_multiply(0.6);
    v.widgets.inactive.bg_fill = RAISED;
    v.widgets.inactive.weak_bg_fill = RAISED;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x31, 0x36, 0x41);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(0x3a, 0x40, 0x4d);
    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.noninteractive,
    ] {
        w.corner_radius = CornerRadius::same(6);
    }
    style.spacing.button_padding = Vec2::new(12.0, 6.0);
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    // Solid, always-visible scrollbars: thin floating ones are hard to find on a small screen.
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    style.spacing.scroll.bar_width = 12.0;
    style
        .text_styles
        .insert(egui::TextStyle::Body, FontId::proportional(15.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, FontId::proportional(15.0));
    ctx.set_style(style);
}

fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, BORDER))
}

fn section_title(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(13.0).color(MUTED).strong());
}

fn badge(ui: &mut egui::Ui, text: &str, color: Color32) {
    egui::Frame::new()
        .fill(color.linear_multiply(0.18))
        .stroke(Stroke::new(1.5_f32, color))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(color).strong().size(15.0));
        });
}

fn pill(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(RAISED)
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.label(
                RichText::new(text)
                    .font(FontId::monospace(12.5))
                    .color(TEXT),
            );
        });
}

fn stat(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::new()
        .fill(RAISED)
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                ui.label(RichText::new(label).size(11.0).color(MUTED));
                ui.label(RichText::new(value).size(13.0).color(TEXT));
            });
        });
}

fn big_button(text: &str, color: Color32) -> Button<'static> {
    Button::new(
        RichText::new(text.to_owned())
            .size(17.0)
            .strong()
            .color(Color32::WHITE),
    )
    .fill(color.linear_multiply(0.85))
    .stroke(Stroke::new(1.0_f32, color))
    .corner_radius(CornerRadius::same(8))
}

fn override_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: f64,
    range: std::ops::RangeInclusive<f64>,
    mut on_change: impl FnMut(f64),
) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [58.0, 18.0],
            egui::Label::new(RichText::new(label).color(MUTED)),
        );
        let mut pct = value * 100.0;
        let r = ui.add(
            egui::Slider::new(&mut pct, (range.start() * 100.0)..=(range.end() * 100.0))
                .suffix(" %")
                .integer(),
        );
        if r.changed() {
            on_change(pct / 100.0);
        }
        if ui.small_button("100").clicked() {
            on_change(1.0);
        }
    });
}
