//! Cutter radius compensation (G41 left / G42 right) in the XY plane.
//!
//! Each programmed XY move is offset by the tool radius and held until the
//! next one arrives, so the corner between them can be resolved: outside
//! corners get an arc around the programmed corner, inside corners are
//! trimmed to where the two offset paths cross. The first move after G41/G42
//! is the lead-in (a line from the uncompensated position to the offset
//! path); the first move after G40 is the lead-out.

use crate::interp::{Axes, Op, Plane};

type P2 = [f64; 2];

fn sub(a: P2, b: P2) -> P2 {
    [a[0] - b[0], a[1] - b[1]]
}
fn add(a: P2, b: P2) -> P2 {
    [a[0] + b[0], a[1] + b[1]]
}
fn scale(a: P2, s: f64) -> P2 {
    [a[0] * s, a[1] * s]
}
fn dot(a: P2, b: P2) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: P2, b: P2) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn len(a: P2) -> f64 {
    dot(a, a).sqrt()
}
fn unit(a: P2) -> P2 {
    let l = len(a);
    if l < 1e-12 {
        [0.0, 0.0]
    } else {
        scale(a, 1.0 / l)
    }
}
/// Left-hand normal of a direction.
fn left(d: P2) -> P2 {
    [-d[1], d[0]]
}
fn xy(a: &Axes) -> P2 {
    [a[0], a[1]]
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    Line,
    Arc { center: P2, cw: bool, turns: u32 },
}

/// A compensated move waiting for its successor.
#[derive(Debug, Clone)]
struct Piece {
    shape: Shape,
    /// Offset start and end.
    start: P2,
    end: P2,
    /// Programmed end, the corner with the next move.
    corner: P2,
    /// Programmed direction at the end.
    end_dir: P2,
    op: Op,
}

impl Piece {
    fn emit(&self) -> Op {
        let mut op = self.op.clone();
        match &mut op {
            Op::Rapid { to } | Op::Linear { to, .. } | Op::Arc { to, .. } => {
                to[0] = self.end[0];
                to[1] = self.end[1];
            }
            _ => {}
        }
        op
    }
}

/// Tangent direction of an arc at point `p`.
fn arc_tangent(center: P2, p: P2, cw: bool) -> P2 {
    let u = unit(sub(p, center));
    if cw {
        [u[1], -u[0]]
    } else {
        [-u[1], u[0]]
    }
}

#[derive(Debug, Clone)]
pub struct Comp {
    /// +1 for G41 (tool left of the path), -1 for G42.
    side: f64,
    radius: f64,
    pending: Option<Piece>,
    /// Ops after the pending move that don't move in XY (Z moves, spindle, ...).
    held: Vec<Op>,
    /// Programmed XY position (uncompensated).
    programmed: P2,
    /// Where the tool actually is (or will be once pending ops run).
    actual: Axes,
}

impl Comp {
    pub fn new(left_side: bool, radius: f64, programmed: &Axes, actual: &Axes) -> Self {
        Comp {
            side: if left_side { 1.0 } else { -1.0 },
            radius,
            pending: None,
            held: Vec::new(),
            programmed: xy(programmed),
            actual: *actual,
        }
    }

    /// Where the tool will be after everything emitted so far.
    pub fn actual(&self) -> Axes {
        self.actual
    }

    fn offset_at(&self, p: P2, dir: P2) -> P2 {
        add(p, scale(left(dir), self.side * self.radius))
    }

    /// Feed ops through compensation; returns what can run now.
    pub fn process(&mut self, ops: Vec<Op>) -> Result<Vec<Op>, String> {
        let mut out = Vec::new();
        for op in ops {
            match &op {
                Op::Rapid { to } | Op::Linear { to, .. } | Op::Arc { to, .. }
                    if (to[0] - self.programmed[0]).abs() > 1e-9 || (to[1] - self.programmed[1]).abs() > 1e-9 =>
                {
                    self.add_move(op, &mut out)?;
                }
                Op::Rapid { to } | Op::Linear { to, .. } | Op::Arc { to, .. } => {
                    // No XY change: keep the tool where compensation put it.
                    let mut op = op.clone();
                    let mut target = *to;
                    if let Op::Rapid { to } | Op::Linear { to, .. } | Op::Arc { to, .. } = &mut op {
                        let end = self.pending.as_ref().map(|p| p.end).unwrap_or(xy(&self.actual));
                        to[0] = end[0];
                        to[1] = end[1];
                        target = *to;
                    }
                    self.actual[2..].copy_from_slice(&target[2..]);
                    self.hold_or_emit(op, &mut out);
                }
                Op::Probe { .. } => return Err("probing (G31) is not allowed with cutter compensation on".into()),
                Op::ProgramEnd => {
                    out.extend(self.flush());
                    out.push(op);
                }
                _ => self.hold_or_emit(op, &mut out),
            }
        }
        Ok(out)
    }

    fn hold_or_emit(&mut self, op: Op, out: &mut Vec<Op>) {
        if self.pending.is_some() {
            self.held.push(op);
        } else {
            out.push(op);
        }
    }

    fn add_move(&mut self, op: Op, out: &mut Vec<Op>) -> Result<(), String> {
        let from = self.programmed;
        let (to, shape) = match &op {
            Op::Arc { to, center, plane, clockwise, turns, .. } => {
                if *plane != Plane::XY {
                    return Err("cutter compensation only works in the XY plane (G17)".into());
                }
                (xy(to), Shape::Arc { center: xy(center), cw: *clockwise, turns: *turns })
            }
            Op::Rapid { to } | Op::Linear { to, .. } => (xy(to), Shape::Line),
            _ => unreachable!(),
        };
        let (start_dir, end_dir) = match shape {
            Shape::Line => {
                let d = unit(sub(to, from));
                (d, d)
            }
            Shape::Arc { center, cw, .. } => (arc_tangent(center, from, cw), arc_tangent(center, to, cw)),
        };
        let mut piece = Piece {
            shape,
            start: self.offset_at(from, start_dir),
            end: self.offset_at(to, end_dir),
            corner: to,
            end_dir,
            op,
        };
        if let Shape::Arc { center, .. } = shape {
            // The offset arc must keep a positive radius on the same side of the centre.
            let r0 = len(sub(from, center));
            let r1 = len(sub(piece.end, center));
            if dot(sub(piece.end, center), sub(to, center)) <= 0.0 || r1 < 1e-6 || r0 < 1e-6 {
                return Err(format!("tool radius {} is too big for an arc of radius {r0:.3}", self.radius));
            }
        }
        match self.pending.take() {
            None => {
                // Lead-in: a straight move from where the tool is onto the offset path.
                if !matches!(shape, Shape::Line) {
                    return Err("the first move after G41/G42 must be a straight line".into());
                }
                piece.start = xy(&self.actual);
            }
            Some(mut prev) => {
                let mut corner_arc = None;
                if len(sub(prev.end, piece.start)) > 1e-7 {
                    let turn = cross(prev.end_dir, start_dir);
                    let outside = turn * self.side < 0.0 || (turn.abs() < 1e-9 && dot(prev.end_dir, start_dir) < 0.0);
                    if outside {
                        let mut arc = prev.op.clone();
                        let mut to6 = self.actual;
                        to6[0] = piece.start[0];
                        to6[1] = piece.start[1];
                        if let Op::Rapid { to } | Op::Linear { to, .. } | Op::Arc { to, .. } = &prev.op {
                            to6[2..].copy_from_slice(&to[2..]);
                        }
                        let feed = match &prev.op {
                            Op::Linear { feed, .. } | Op::Arc { feed, .. } => *feed,
                            _ => 0.0,
                        };
                        arc = if matches!(arc, Op::Rapid { .. }) {
                            Op::Rapid { to: to6 }
                        } else {
                            let mut center = to6;
                            center[0] = prev.corner[0];
                            center[1] = prev.corner[1];
                            Op::Arc { to: to6, center, plane: Plane::XY, clockwise: self.side > 0.0, turns: 0, feed }
                        };
                        corner_arc = Some(arc);
                    } else {
                        let x = self.intersect(&prev, &piece)?;
                        prev.end = x;
                        piece.start = x;
                    }
                }
                out.push(prev.emit());
                out.append(&mut self.held);
                out.extend(corner_arc);
            }
        }
        self.programmed = to;
        let emitted_to = piece.end;
        self.actual[0] = emitted_to[0];
        self.actual[1] = emitted_to[1];
        if let Op::Rapid { to } | Op::Linear { to, .. } | Op::Arc { to, .. } = &piece.op {
            self.actual[2..].copy_from_slice(&to[2..]);
        }
        self.pending = Some(piece);
        Ok(())
    }

    /// Where the offset paths of `a` (ending) and `b` (starting) cross, nearest the corner.
    fn intersect(&self, a: &Piece, b: &Piece) -> Result<P2, String> {
        let mut candidates = Vec::new();
        match (a.shape, b.shape) {
            (Shape::Line, Shape::Line) => {
                let da = sub(a.end, a.start);
                let db = sub(b.end, b.start);
                let den = cross(da, db);
                if den.abs() > 1e-12 {
                    let t = cross(sub(b.start, a.start), db) / den;
                    candidates.push(add(a.start, scale(da, t)));
                }
            }
            (Shape::Line, Shape::Arc { center, .. }) => {
                candidates.extend(line_circle(a.start, a.end, center, len(sub(b.start, center))));
            }
            (Shape::Arc { center, .. }, Shape::Line) => {
                candidates.extend(line_circle(b.start, b.end, center, len(sub(a.end, center))));
            }
            (Shape::Arc { center: c1, .. }, Shape::Arc { center: c2, .. }) => {
                candidates.extend(circle_circle(c1, len(sub(a.end, c1)), c2, len(sub(b.start, c2))));
            }
        }
        let best = candidates
            .into_iter()
            .min_by(|p, q| len(sub(*p, a.corner)).total_cmp(&len(sub(*q, a.corner))))
            .ok_or_else(|| format!("tool radius {} is too big for an inside corner", self.radius))?;
        // Trimming a line back past its start would gouge the previous feature.
        if a.shape == Shape::Line && dot(sub(best, a.start), sub(a.end, a.start)) < -1e-9
            || b.shape == Shape::Line && dot(sub(b.end, best), sub(b.end, b.start)) < -1e-9
        {
            return Err(format!("tool radius {} is too big for an inside corner", self.radius));
        }
        Ok(best)
    }

    /// End compensation (G40, program end): send everything still held.
    pub fn flush(&mut self) -> Vec<Op> {
        let mut out = Vec::new();
        if let Some(p) = self.pending.take() {
            out.push(p.emit());
        }
        out.append(&mut self.held);
        out
    }
}

fn line_circle(p0: P2, p1: P2, c: P2, r: f64) -> Vec<P2> {
    let d = sub(p1, p0);
    let f = sub(p0, c);
    let a = dot(d, d);
    if a < 1e-18 {
        return vec![];
    }
    let b = 2.0 * dot(f, d);
    let cc = dot(f, f) - r * r;
    let disc = b * b - 4.0 * a * cc;
    if disc < 0.0 {
        return vec![];
    }
    let s = disc.sqrt();
    [(-b - s) / (2.0 * a), (-b + s) / (2.0 * a)].iter().map(|t| add(p0, scale(d, *t))).collect()
}

fn circle_circle(c1: P2, r1: f64, c2: P2, r2: f64) -> Vec<P2> {
    let d = len(sub(c2, c1));
    if d < 1e-12 || d > r1 + r2 || d < (r1 - r2).abs() {
        return vec![];
    }
    let a = (r1 * r1 - r2 * r2 + d * d) / (2.0 * d);
    let h = (r1 * r1 - a * a).max(0.0).sqrt();
    let u = unit(sub(c2, c1));
    let m = add(c1, scale(u, a));
    vec![add(m, scale(left(u), h)), sub(m, scale(left(u), h))]
}

#[cfg(test)]
mod tests {
    use crate::interp::{Interpreter, Op, Tool};

    fn path(program: &str) -> Result<Vec<(f64, f64, bool)>, String> {
        let mut i = Interpreter::new(program);
        i.tools.insert(1, Tool { diameter: 6.0, ..Default::default() });
        let steps = i.run_to_end(10_000).map_err(|e| e.message)?;
        Ok(steps
            .into_iter()
            .flat_map(|s| s.ops)
            .filter_map(|op| match op {
                Op::Linear { to, .. } | Op::Rapid { to } => Some((to[0], to[1], false)),
                Op::Arc { to, .. } => Some((to[0], to[1], true)),
                _ => None,
            })
            .map(|(x, y, a)| ((x * 1000.0).round() / 1000.0, (y * 1000.0).round() / 1000.0, a))
            .collect())
    }

    const SQUARE: &str = "G0 X-10 Y0\nG1 X0 Y0 F100\nG1 X20\nY20\nX0\nY0\nG40 G1 X-10\nM30";

    #[test]
    fn outside_corners_get_arcs() {
        let p = path(&SQUARE.replace("G1 X0 Y0 F100", "G42 D1 G1 X0 Y0 F100")).unwrap();
        assert_eq!(
            p[1..],
            [
                (0.0, -3.0, false),
                (20.0, -3.0, false),
                (23.0, 0.0, true),
                (23.0, 20.0, false),
                (20.0, 23.0, true),
                (0.0, 23.0, false),
                (-3.0, 20.0, true),
                (-3.0, 0.0, false),
                (-10.0, 0.0, false),
            ]
        );
    }

    #[test]
    fn inside_corners_are_trimmed() {
        let p = path(&SQUARE.replace("G1 X0 Y0 F100", "G41 D1 G1 X0 Y0 F100")).unwrap();
        assert_eq!(
            p[1..],
            [(0.0, 3.0, false), (17.0, 3.0, false), (17.0, 17.0, false), (3.0, 17.0, false), (3.0, 0.0, false), (-10.0, 0.0, false)]
        );
    }

    #[test]
    fn arcs_are_offset_and_checked() {
        // Half circle radius 10 around (10,0), clockwise over the top; G42 keeps the
        // tool on the inside, so the tool centre runs on radius 7.
        let p = path("G0 X-10 Y0\nG42 D1 G1 X0 Y0 F100\nG2 X20 Y0 I10 J0\nG40 G1 X30 Y0\nM30").unwrap();
        assert_eq!(p[1..4], [(0.0, -3.0, false), (3.0, 0.0, true), (17.0, 0.0, true)]);
        // A 2 mm radius arc can't be cut on the inside with a 3 mm radius tool.
        let err = path("G0 X-10 Y0\nG42 D1 G1 X0 Y0 F100\nG2 X4 Y0 I2 J0\nG40 G1 X10\nM30").unwrap_err();
        assert!(err.contains("too big"), "{err}");
    }

    #[test]
    fn z_moves_wait_for_the_corner() {
        let p = path("G0 X-10 Y0\nG42 D1 G1 X0 Y0 F100\nG1 X20\nG1 Z-1\nG1 Y20\nG40 G1 X30\nM30").unwrap();
        // The Z move happens at the end of the first edge, before the corner arc.
        assert_eq!(p[2], (20.0, -3.0, false));
        assert_eq!(p[3], (20.0, -3.0, false));
        assert_eq!(p[4], (23.0, 0.0, true));
    }
}
