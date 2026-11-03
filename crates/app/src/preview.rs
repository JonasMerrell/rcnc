//! Toolpath preview: runs the program through the interpreter and draws the
//! path with an orbiting orthographic camera.

use eframe::egui::{self, Color32, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use gcode::{Axes, Interpreter, Op};
use machine::planner::op_to_moves;

#[derive(Debug, Clone, Copy)]
pub struct PathSeg {
    pub a: [f64; 3],
    pub b: [f64; 3],
    pub rapid: bool,
    pub line: usize,
}

#[derive(Default)]
pub struct Toolpath {
    pub segs: Vec<PathSeg>,
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub error: Option<String>,
    pub feed_length: f64,
    pub rapid_length: f64,
}

/// What the controller knows that changes how a program runs: tools,
/// offsets, macros and where the machine is. Without it G43 H and G41 D stop
/// the preview, and G53 moves are drawn in the wrong place.
#[derive(Clone, Default)]
pub struct Setup {
    pub tools: std::collections::BTreeMap<u32, gcode::Tool>,
    pub params: std::collections::BTreeMap<u32, f64>,
    pub coord_system: usize,
    pub tool: u32,
    pub tool_length_offset: f64,
    pub macros: std::collections::HashMap<u32, String>,
    pub machine: Axes,
    pub spindle_reversible: bool,
}

impl Setup {
    pub fn from_snapshot(s: &machine::Snapshot) -> Self {
        Setup {
            tools: s.tools.clone(),
            params: s.params.clone(),
            coord_system: s.coord_system,
            tool: s.tool,
            tool_length_offset: s.tool_length_offset,
            macros: (*s.macros).clone(),
            machine: s.machine,
            spindle_reversible: s.spindle_reversible,
        }
    }
}

/// Lines the preview reads at most, so a program that loops forever (a big
/// M98 L count) can't keep its thread busy for good.
const MAX_STEPS: usize = 5_000_000;

impl Toolpath {
    /// Build on a thread of its own; the result comes on the receiver.
    pub fn build_in_background(text: std::sync::Arc<String>, setup: Setup, ctx: egui::Context) -> std::sync::mpsc::Receiver<Toolpath> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("preview".into())
            .spawn(move || {
                let path = Toolpath::build(&text, &setup);
                let _ = tx.send(path);
                ctx.request_repaint();
            })
            .expect("spawn preview thread");
        rx
    }

    /// The path in work coordinates (the active offsets at each move), as
    /// the controller would run it from `setup`.
    pub fn build(text: &str, setup: &Setup) -> Self {
        let mut interp = Interpreter::new(text);
        interp.tools = setup.tools.clone();
        for (&k, &v) in &setup.params {
            interp.params.set(k, v);
        }
        interp.state.coord_system = setup.coord_system;
        interp.state.tool = setup.tool;
        interp.state.tool_length_offset = setup.tool_length_offset;
        interp.macros = setup.macros.clone();
        interp.spindle_reversible = setup.spindle_reversible;
        interp.position = setup.machine;
        let mut path = Toolpath {
            min: [f64::MAX; 3],
            max: [f64::MIN; 3],
            ..Default::default()
        };
        let mut pos: Axes = interp.position;
        let mut steps = 0usize;
        loop {
            let step = match interp.next_step() {
                Ok(Some(s)) => s,
                Ok(None) => break,
                Err(e) => {
                    path.error = Some(e.to_string());
                    break;
                }
            };
            steps += 1;
            if steps > MAX_STEPS {
                path.error = Some("program too long to preview, or it loops forever".into());
                break;
            }
            // Work coordinates of the offsets in force after this line.
            let off = interp.program_offset();
            let work = |p: &Axes| [p[0] - off[0], p[1] - off[1], p[2] - off[2]];
            for op in &step.ops {
                if let Op::Probe { to, .. } = op {
                    interp.probe_result(*to);
                }
                let rapid = matches!(op, Op::Rapid { .. });
                let moves = match op {
                    Op::Probe { to, .. } => vec![machine::planner::Move {
                        to: *to,
                        speed: 1.0,
                        rapid: false,
                        line: step.line,
                        continues: false,
                    }],
                    _ => op_to_moves(op, &pos, step.line, 0.01),
                };
                for m in moves {
                    let a = work(&pos);
                    let b = work(&m.to);
                    let len =
                        ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2))
                            .sqrt();
                    if rapid {
                        path.rapid_length += len;
                    } else {
                        path.feed_length += len;
                    }
                    path.segs.push(PathSeg {
                        a,
                        b,
                        rapid,
                        line: step.line,
                    });
                    pos = m.to;
                }
            }
            if path.segs.len() > 2_000_000 {
                path.error = Some("program too large to preview".into());
                break;
            }
        }
        for s in &path.segs {
            for p in [s.a, s.b] {
                for i in 0..3 {
                    path.min[i] = path.min[i].min(p[i]);
                    path.max[i] = path.max[i].max(p[i]);
                }
            }
        }
        if path.segs.is_empty() {
            path.min = [0.0; 3];
            path.max = [0.0; 3];
        }
        path
    }
}

pub struct Camera {
    yaw: f32,
    pitch: f32,
    zoom: f32,
    pan: Vec2,
    center: [f32; 3],
    fit_pending: bool,
    /// The path as last drawn, and what for.
    mesh: Option<(MeshKey, std::sync::Arc<egui::Mesh>)>,
}

impl Default for Camera {
    fn default() -> Self {
        Camera {
            yaw: -0.6,
            pitch: 0.9,
            zoom: 4.0,
            pan: Vec2::ZERO,
            center: [0.0; 3],
            fit_pending: true,
            mesh: None,
        }
    }
}

impl Camera {
    pub fn top(&mut self) {
        self.yaw = 0.0;
        self.pitch = 0.0;
        self.fit_pending = true;
    }

    pub fn front(&mut self) {
        self.yaw = 0.0;
        self.pitch = std::f32::consts::FRAC_PI_2;
        self.fit_pending = true;
    }

    pub fn iso(&mut self) {
        self.yaw = -0.6;
        self.pitch = 0.9;
        self.fit_pending = true;
    }

    pub fn fit(&mut self) {
        self.fit_pending = true;
    }

    /// World (X right, Y away, Z up) to view space: x right, y down on screen, depth.
    fn view(&self, p: [f64; 3]) -> [f32; 3] {
        let x = p[0] as f32 - self.center[0];
        let y = p[1] as f32 - self.center[1];
        let z = p[2] as f32 - self.center[2];
        let (sy, cy) = self.yaw.sin_cos();
        let x1 = x * cy - y * sy;
        let y1 = x * sy + y * cy;
        let (sp, cp) = self.pitch.sin_cos();
        // Pitch 0 looks straight down (top view); pitch pi/2 looks from the front.
        let up = y1 * cp + z * sp;
        let depth = -y1 * sp + z * cp;
        [x1, -up, depth]
    }

    fn project(&self, rect: Rect, p: [f64; 3]) -> Pos2 {
        let v = self.view(p);
        rect.center() + self.pan + Vec2::new(v[0], v[1]) * self.zoom
    }

    /// `project` for many points: the sines and cosines worked out once.
    fn projector(&self, rect: Rect) -> impl Fn([f64; 3]) -> Pos2 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        let (c, zoom, origin) = (self.center, self.zoom, rect.center() + self.pan);
        move |p| {
            let x = p[0] as f32 - c[0];
            let y = p[1] as f32 - c[1];
            let z = p[2] as f32 - c[2];
            let x1 = x * cy - y * sy;
            let y1 = x * sy + y * cy;
            let up = y1 * cp + z * sp;
            origin + Vec2::new(x1, -up) * zoom
        }
    }
}

pub struct Colors {
    pub feed: Color32,
    pub rapid: Color32,
    pub active: Color32,
    pub tool: Color32,
    pub envelope: Color32,
    pub background: Color32,
}

/// Points of a drawn path closer together than this are merged.
const MIN_PIXELS: f32 = 1.5;

/// What the cached path mesh was drawn for.
#[derive(Clone, Copy, PartialEq)]
struct MeshKey {
    path: (usize, usize),
    view: [f32; 8],
    rect: Rect,
    pixels_per_point: f32,
    colors: [Color32; 2],
}

/// Rapids first so cuts draw on top. Moves that join up become one
/// polyline, leaving out points within a pixel of the last one kept.
fn path_mesh(cam: &Camera, rect: Rect, path: &Toolpath, colors: &Colors, pixels_per_point: f32) -> egui::Mesh {
    let mut tess = egui::epaint::Tessellator::new(pixels_per_point, Default::default(), [1, 1], Vec::new());
    tess.set_clip_rect(rect);
    let mut mesh = egui::Mesh::default();
    let project = cam.projector(rect);
    let visible = rect.expand(50.0);
    for rapid in [true, false] {
        let stroke = if rapid {
            Stroke::new(0.8_f32, colors.rapid)
        } else {
            Stroke::new(1.3_f32, colors.feed)
        };
        let mut line: Vec<Pos2> = Vec::new();
        let mut flush = |line: &mut Vec<Pos2>, mesh: &mut egui::Mesh| {
            if line.len() >= 2 {
                if rapid {
                    for shape in Shape::dashed_line(line, stroke, 4.0, 4.0) {
                        tess.tessellate_shape(shape, mesh);
                    }
                } else {
                    tess.tessellate_shape(Shape::line(std::mem::take(line), stroke), mesh);
                }
            }
            line.clear();
        };
        for s in path.segs.iter().filter(|s| s.rapid == rapid) {
            let a = project(s.a);
            let b = project(s.b);
            if !visible.contains(a) && !visible.contains(b) {
                flush(&mut line, &mut mesh);
                continue;
            }
            match line.last() {
                Some(last) if last.distance(a) <= MIN_PIXELS => {}
                _ => {
                    flush(&mut line, &mut mesh);
                    line.push(a);
                }
            }
            if line.last().is_some_and(|last| last.distance(b) > MIN_PIXELS) {
                line.push(b);
            }
        }
        flush(&mut line, &mut mesh);
    }
    mesh
}

pub fn draw(
    ui: &mut egui::Ui,
    cam: &mut Camera,
    path: &Toolpath,
    tool: Axes,
    active_line: Option<usize>,
    envelope: Option<([f64; 3], [f64; 3])>,
    colors: &Colors,
) {
    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;
    painter.rect_filled(rect, 8.0, colors.background);

    if cam.fit_pending {
        let (min, max) = if path.segs.is_empty() {
            envelope.unwrap_or(([-50.0; 3], [50.0; 3]))
        } else {
            (path.min, path.max)
        };
        cam.center = std::array::from_fn(|i| ((min[i] + max[i]) / 2.0) as f32);
        cam.pan = Vec2::ZERO;
        // Fit the projected bounding box corners.
        let mut lo = Vec2::splat(f32::MAX);
        let mut hi = Vec2::splat(f32::MIN);
        for k in 0..8 {
            let p = [
                if k & 1 == 0 { min[0] } else { max[0] },
                if k & 2 == 0 { min[1] } else { max[1] },
                if k & 4 == 0 { min[2] } else { max[2] },
            ];
            let v = cam.view(p);
            lo = lo.min(Vec2::new(v[0], v[1]));
            hi = hi.max(Vec2::new(v[0], v[1]));
        }
        let size = (hi - lo).max(Vec2::splat(1.0));
        cam.zoom = ((rect.width() - 60.0) / size.x)
            .min((rect.height() - 60.0) / size.y)
            .clamp(0.01, 500.0);
        cam.fit_pending = false;
    }

    if response.dragged_by(egui::PointerButton::Primary) {
        let d = response.drag_delta();
        cam.yaw -= d.x * 0.008;
        cam.pitch = (cam.pitch - d.y * 0.008).clamp(0.0, std::f32::consts::PI);
    }
    if response.dragged_by(egui::PointerButton::Secondary)
        || response.dragged_by(egui::PointerButton::Middle)
    {
        cam.pan += response.drag_delta();
    }
    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let factor = (scroll * 0.002).exp();
            if let Some(mouse) = response.hover_pos() {
                let anchor = mouse - rect.center() - cam.pan;
                cam.pan -= anchor * (factor - 1.0);
            }
            cam.zoom = (cam.zoom * factor).clamp(0.01, 1000.0);
        }
    }
    if response.double_clicked() {
        cam.fit();
    }

    let painter = painter.with_clip_rect(rect);

    // Machine envelope.
    if let Some((min, max)) = envelope {
        let corners: Vec<[f64; 3]> = (0..8)
            .map(|k| {
                [
                    if k & 1 == 0 { min[0] } else { max[0] },
                    if k & 2 == 0 { min[1] } else { max[1] },
                    if k & 4 == 0 { min[2] } else { max[2] },
                ]
            })
            .collect();
        for (a, b) in [
            (0, 1),
            (1, 3),
            (3, 2),
            (2, 0),
            (4, 5),
            (5, 7),
            (7, 6),
            (6, 4),
            (0, 4),
            (1, 5),
            (2, 6),
            (3, 7),
        ] {
            painter.line_segment(
                [cam.project(rect, corners[a]), cam.project(rect, corners[b])],
                Stroke::new(1.0_f32, colors.envelope),
            );
        }
    }

    // Work origin axes.
    let o = cam.project(rect, [0.0; 3]);
    let axis_len = 20.0 / cam.zoom as f64;
    for (i, c) in [
        Color32::from_rgb(230, 80, 80),
        Color32::from_rgb(90, 200, 110),
        Color32::from_rgb(90, 140, 240),
    ]
    .into_iter()
    .enumerate()
    {
        let mut p = [0.0; 3];
        p[i] = axis_len.max(5.0);
        painter.line_segment([o, cam.project(rect, p)], Stroke::new(2.0_f32, c));
    }

    // The path, as a mesh built once and kept until the view or the path
    // changes. A CAM program of a million tiny moves would otherwise be
    // projected and tessellated every frame (300 ms a frame for a million
    // moves), and the window repaints 30 times a second while a program
    // runs: Stop, the DRO and the jog buttons would all lag.
    let key = MeshKey {
        path: (path.segs.as_ptr() as usize, path.segs.len()),
        view: [cam.yaw, cam.pitch, cam.zoom, cam.pan.x, cam.pan.y, cam.center[0], cam.center[1], cam.center[2]],
        rect,
        pixels_per_point: ui.ctx().pixels_per_point(),
        colors: [colors.feed, colors.rapid],
    };
    let mesh = match &cam.mesh {
        Some((k, mesh)) if *k == key => mesh.clone(),
        _ => {
            let mesh = std::sync::Arc::new(path_mesh(cam, rect, path, colors, key.pixels_per_point));
            cam.mesh = Some((key, mesh.clone()));
            mesh
        }
    };
    painter.add(Shape::mesh(mesh));
    // The line running now, on top.
    if let Some(active) = active_line {
        for s in path.segs.iter().filter(|s| s.line == active) {
            painter.line_segment([cam.project(rect, s.a), cam.project(rect, s.b)], Stroke::new(2.5_f32, colors.active));
        }
    }

    // Tool marker: a short vertical cone at the tool tip.
    let tip = cam.project(rect, [tool[0], tool[1], tool[2]]);
    let top = cam.project(
        rect,
        [tool[0], tool[1], tool[2] + 12.0 / cam.zoom.max(0.05) as f64],
    );
    painter.line_segment([tip, top], Stroke::new(3.0_f32, colors.tool));
    painter.circle_filled(tip, 4.5, colors.tool);

    if let Some(err) = &path.error {
        painter.text(
            rect.left_bottom() + Vec2::new(12.0, -12.0),
            egui::Align2::LEFT_BOTTOM,
            err,
            egui::FontId::proportional(14.0),
            Color32::from_rgb(240, 110, 90),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 3D-finishing-like path: a dense raster of tiny feed moves.
    fn dense_path(n: usize) -> Toolpath {
        let mut segs = Vec::with_capacity(n);
        let mut p = [0.0, 0.0, 0.0];
        for i in 0..n {
            let row = (i / 2000) as f64;
            let x = (i % 2000) as f64 * 0.05;
            let x = if (i / 2000) % 2 == 0 { x } else { 100.0 - x };
            let q = [x, row * 0.2, (x * 0.1).sin() - 2.0];
            segs.push(PathSeg { a: p, b: q, rapid: i % 2000 == 0, line: i });
            p = q;
        }
        Toolpath { segs, min: [0.0, 0.0, -3.0], max: [100.0, n as f64 / 2000.0 * 0.2, 0.0], ..Default::default() }
    }

    fn frame_time(path: &Toolpath) -> (std::time::Duration, usize) {
        let ctx = egui::Context::default();
        let mut cam = Camera::default();
        let colors = Colors {
            feed: Color32::WHITE,
            rapid: Color32::GRAY,
            active: Color32::YELLOW,
            tool: Color32::RED,
            envelope: Color32::DARK_GRAY,
            background: Color32::BLACK,
        };
        let input = || egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 800.0))),
            ..Default::default()
        };
        let frame = |cam: &mut Camera| {
            let out = ctx.run(input(), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| draw(ui, cam, path, [0.0; 6], Some(500), None, &colors));
            });
            ctx.tessellate(out.shapes, out.pixels_per_point)
        };
        let start = std::time::Instant::now();
        frame(&mut cam);
        eprintln!("first frame (builds the mesh): {:?}", start.elapsed());
        let mut best = std::time::Duration::MAX;
        let mut meshes = Vec::new();
        for _ in 0..5 {
            let start = std::time::Instant::now();
            meshes = frame(&mut cam);
            best = best.min(start.elapsed());
        }
        let vertices = meshes
            .iter()
            .map(|m| match &m.primitive {
                egui::epaint::Primitive::Mesh(m) => m.vertices.len(),
                _ => 0,
            })
            .sum();
        (best, vertices)
    }

    /// Drawing a million-move program must not stall the window (it
    /// repaints at 30 Hz while a program runs). Release build: 315 ms a
    /// frame and 8M vertices when every move was its own shape; about 15 ms
    /// with the cached mesh.
    #[test]
    fn big_programs_draw_quickly() {
        let path = dense_path(1_000_000);
        let (t, vertices) = frame_time(&path);
        eprintln!("1M moves: {t:?}, {vertices} vertices");
        assert!(vertices < 2_000_000, "{vertices} vertices");
    }
}
