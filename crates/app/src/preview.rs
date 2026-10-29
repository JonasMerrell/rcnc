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

impl Toolpath {
    /// Path in work coordinates of the program as written (no offsets applied).
    pub fn build(text: &str) -> Self {
        let mut interp = Interpreter::new(text);
        let mut path = Toolpath {
            min: [f64::MAX; 3],
            max: [f64::MIN; 3],
            ..Default::default()
        };
        let mut pos: Axes = interp.position;
        loop {
            let step = match interp.next_step() {
                Ok(Some(s)) => s,
                Ok(None) => break,
                Err(e) => {
                    path.error = Some(e.to_string());
                    break;
                }
            };
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
                    let a = [pos[0], pos[1], pos[2]];
                    let b = [m.to[0], m.to[1], m.to[2]];
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
}

pub struct Colors {
    pub feed: Color32,
    pub rapid: Color32,
    pub active: Color32,
    pub tool: Color32,
    pub envelope: Color32,
    pub background: Color32,
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

    // Path: rapids first so cuts draw on top.
    for rapid in [true, false] {
        for s in path.segs.iter().filter(|s| s.rapid == rapid) {
            let a = cam.project(rect, s.a);
            let b = cam.project(rect, s.b);
            if !rect.expand(50.0).contains(a) && !rect.expand(50.0).contains(b) {
                continue;
            }
            let active = active_line == Some(s.line);
            let color = if active {
                colors.active
            } else if rapid {
                colors.rapid
            } else {
                colors.feed
            };
            let width: f32 = if active {
                2.5
            } else if rapid {
                0.8
            } else {
                1.3
            };
            if rapid && !active {
                painter.extend(Shape::dashed_line(
                    &[a, b],
                    Stroke::new(width, color),
                    4.0,
                    4.0,
                ));
            } else {
                painter.line_segment([a, b], Stroke::new(width, color));
            }
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
