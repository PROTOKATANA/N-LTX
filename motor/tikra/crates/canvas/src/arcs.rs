//! Elliptical arcs as cubic curves, as `\pgfpatharc` cuts them.

use crate::scene::{Point, Segment, Transform};

/// An elliptical arc in degrees, before the transform that places it on the canvas.
#[derive(Clone, Copy)]
pub struct EllipseArc {
    pub center: Point,
    pub rx: f64,
    pub ry: f64,
    pub start: f64,
    pub end: f64,
}

/// An elliptical arc from `a0` to `a1` degrees as cubic curves, cut the way `\pgfpatharc` cuts it:
/// 90° pieces while more than 115° are left, 60° pieces while more than 90° are, then the rest.
pub fn arc_segments(arc: EllipseArc, t: &Transform) -> Vec<Segment> {
    let EllipseArc { center: c, rx, ry, start: a0, end: a1 } = arc;
    let mut a = a0;
    let mut b = a1;
    if (b - a).abs() > 720.0 {
        // more than two turns draw the same ellipse: keep one turn and the rest
        b = a + (b - a).signum() * (360.0 + (b - a).abs() % 360.0);
    }
    let piece = |from: f64, to: f64| -> Segment {
        let delta = (to - from).abs();
        let k = if (delta - 90.0).abs() < 1e-12 { 0.55228475 } else { 4.0 / 3.0 * (delta.to_radians() / 4.0).tan() };
        let side = if to > from { 90.0 } else { -90.0 };
        let polar = |angle: f64, sx: f64, sy: f64| {
            let (s, co) = angle.to_radians().sin_cos();
            Point::new(sx * co, sy * s)
        };
        let p0 = polar(from, rx, ry);
        let p3 = polar(to, rx, ry);
        let c1 = p0.plus(polar(from + side, k * rx, k * ry));
        let c2 = p3.plus(polar(to - side, k * rx, k * ry));
        Segment::CurveTo(c.plus(t.linear(c1)), c.plus(t.linear(c2)), c.plus(t.linear(p3)))
    };
    let mut v = Vec::new();
    while (a - b).abs() > 90.0 && v.len() < 100 {
        let step = if (a - b).abs() > 115.0 { 90.0 } else { 60.0 };
        let next = if b > a { a + step } else { a - step };
        v.push(piece(a, next));
        a = next;
    }
    v.push(piece(a, b));
    v
}
