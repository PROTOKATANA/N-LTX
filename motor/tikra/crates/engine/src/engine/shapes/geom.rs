//! Points, borders and arcs the shapes share.

use crate::scene::{Point, Segment};

// ------------------------------------------------------------------ geometry helpers

/// `\pgfqpointpolar`: the point at `angle` degrees and distance `r`.
pub fn polar(angle: f64, r: f64) -> Point {
    let (s, c) = angle.to_radians().sin_cos();
    Point::new(r * c, r * s)
}

/// `\pgfmathrotatepointaround`: `p` rotated by `angle` degrees around `c`.
pub fn rotate_around(p: Point, c: Point, angle: f64) -> Point {
    if angle == 0.0 {
        return p;
    }
    let (s, co) = angle.to_radians().sin_cos();
    let d = p.minus(c);
    c.plus(Point::new(d.x * co - d.y * s, d.x * s + d.y * co))
}

/// `\pgfmathanglebetweenpoints`: the angle of `q` seen from `p`, in [0, 360).
pub fn angle_between(p: Point, q: Point) -> f64 {
    let a = (q.y - p.y).atan2(q.x - p.x).to_degrees();
    if a < 0.0 {
        a + 360.0
    } else {
        a
    }
}

/// `\pgfpointintersectionoflines`: where the line through `a` and `b` meets the line through `c`
/// and `d`.
pub fn intersect_lines(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let r = b.minus(a);
    let s = d.minus(c);
    let den = r.x * s.y - r.y * s.x;
    if den.abs() < 1e-15 {
        return None;
    }
    let t = ((c.x - a.x) * s.y - (c.y - a.y) * s.x) / den;
    Some(a.plus(r.scale(t)))
}

/// `\pgfpointborderrectangle`: the point where the ray in direction `d` leaves the rectangle of
/// half sizes `h` centered at the origin.
pub fn border_rectangle(d: Point, h: Point) -> Point {
    let tx = if d.x.abs() > 1e-12 { h.x / d.x.abs() } else { f64::INFINITY };
    let ty = if d.y.abs() > 1e-12 { h.y / d.y.abs() } else { f64::INFINITY };
    d.scale(tx.min(ty))
}

/// `\pgfpointborderellipse`: the point where the ray in direction `d` meets the ellipse of radii
/// `r` centered at the origin.
pub fn border_ellipse(d: Point, r: Point) -> Point {
    let (a, b) = (r.x.max(1e-12), r.y.max(1e-12));
    let k = 1.0 / ((d.x / a).powi(2) + (d.y / b).powi(2)).sqrt();
    d.scale(k)
}

/// Where the ray from `o` in direction `d` leaves the closed outline (its farthest crossing).
pub fn ray_exit(o: Point, d: Point, outline: &[Point]) -> Option<Point> {
    let n = outline.len();
    let mut best: Option<f64> = None;
    for k in 0..n {
        let a = outline[k];
        let b = outline[(k + 1) % n];
        let s = b.minus(a);
        let den = d.x * s.y - d.y * s.x;
        if den.abs() < 1e-15 {
            continue;
        }
        // o + t d = a + u s
        let w = a.minus(o);
        let t = (w.x * s.y - w.y * s.x) / den;
        let u = (w.x * d.y - w.y * d.x) / den;
        if t > 1e-12 && (-1e-9..=1.0 + 1e-9).contains(&u) && best.is_none_or(|bt| t > bt) {
            best = Some(t);
        }
    }
    best.map(|t| o.plus(d.scale(t)))
}

/// The border anchor from a reference point: where the ray from `o` toward `d` leaves the outline,
/// or `o` if it does not.
pub fn ray(o: Point, d: Point, outline: &[Point]) -> Point {
    ray_exit(o, d, outline).unwrap_or(o)
}

/// The polygon with each side moved out by `d` (the corners by the miter length), as the shapes
/// compute their anchor borders from `outer sep`.
pub fn miter_offset(poly: &[Point], d: f64) -> Vec<Point> {
    let n = poly.len();
    if n < 3 || d == 0.0 {
        return poly.to_vec();
    }
    // the outward normal side depends on the orientation
    let area: f64 = (0..n).map(|k| poly[k].x * poly[(k + 1) % n].y - poly[(k + 1) % n].x * poly[k].y).sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let shifted = |a: Point, b: Point| -> (Point, Point) {
        let e = b.minus(a);
        let l = e.norm().max(1e-15);
        // right normal of a counterclockwise polygon points outside
        let nrm = Point::new(e.y / l, -e.x / l).scale(sign * d);
        (a.plus(nrm), b.plus(nrm))
    };
    (0..n)
        .map(|k| {
            let (p0, p1) = shifted(poly[(k + n - 1) % n], poly[k]);
            let (q0, q1) = shifted(poly[k], poly[(k + 1) % n]);
            intersect_lines(p0, p1, q0, q1).unwrap_or(p1)
        })
        .collect()
}

/// A closed polygon as a path.
pub fn polygon_path(points: &[Point]) -> Vec<Segment> {
    let mut v = Vec::with_capacity(points.len() + 1);
    for (k, p) in points.iter().enumerate() {
        v.push(if k == 0 { Segment::MoveTo(*p) } else { Segment::LineTo(*p) });
    }
    v.push(Segment::Close);
    v
}

/// Angles (degrees) and radii of one elliptical arc. What [`arc_to`], [`arc_to_rotated`] and
/// [`arc_points`] describe together.
#[derive(Debug, Clone, Copy)]
pub struct EllipseSpan {
    pub a0: f64,
    pub a1: f64,
    pub rx: f64,
    pub ry: f64,
}

/// `\pgfpatharc` from the current point: an elliptical arc whose start is `from` at angle `a0`.
/// Returns the end point.
pub fn arc_to(v: &mut Vec<Segment>, from: Point, span: EllipseSpan) -> Point {
    let EllipseSpan { a0, a1, rx, ry } = span;
    let c = from.minus(Point::new(rx * a0.to_radians().cos(), ry * a0.to_radians().sin()));
    v.extend(super::super::path::arc_segments(super::super::path::EllipseArc { center: c, rx, ry, start: a0, end: a1 }, &crate::scene::Transform::default()));
    c.plus(Point::new(rx * a1.to_radians().cos(), ry * a1.to_radians().sin()))
}

/// An [`arc_to`] rotated by `rotate` degrees around the arc's start (as `\pgftransformrotate`
/// before `\pgfpatharc`).
#[derive(Debug, Clone, Copy)]
pub struct RotatedArc {
    pub from: Point,
    pub span: EllipseSpan,
    pub rotate: f64,
}

/// Like [`arc_to`], rotated around the arc's start.
pub fn arc_to_rotated(v: &mut Vec<Segment>, arc: RotatedArc) -> Point {
    let RotatedArc { from, span, rotate } = arc;
    let mut local = Vec::new();
    let end = arc_to(&mut local, Point::new(0.0, 0.0), span);
    let t = crate::scene::Transform::translation(from.x, from.y).compose(&crate::scene::Transform::rotation(rotate));
    v.extend(crate::canvas::transform_segments(&local, &t));
    t.apply(end)
}

/// Points along an elliptical arc around `c` (for outlines), every degree or so.
pub fn arc_points(c: Point, span: EllipseSpan) -> Vec<Point> {
    let EllipseSpan { a0, a1, rx, ry } = span;
    let n = ((a1 - a0).abs().ceil() as usize).clamp(2, 3600);
    (0..=n)
        .map(|k| {
            let a = (a0 + (a1 - a0) * k as f64 / n as f64).to_radians();
            c.plus(Point::new(rx * a.cos(), ry * a.sin()))
        })
        .collect()
}

/// Points along a path's segments (curves flattened), for outlines; closes are dropped.
pub fn flatten(segs: &[Segment]) -> Vec<Point> {
    let mut out: Vec<Point> = Vec::new();
    let mut cur = Point::new(0.0, 0.0);
    for s in segs {
        match *s {
            Segment::MoveTo(p) | Segment::LineTo(p) => {
                out.push(p);
                cur = p;
            }
            Segment::CurveTo(a, b, p) => {
                for k in 1..=24 {
                    let t = k as f64 / 24.0;
                    let u = 1.0 - t;
                    out.push(cur.scale(u * u * u).plus(a.scale(3.0 * u * u * t)).plus(b.scale(3.0 * u * t * t)).plus(p.scale(t * t * t)));
                }
                cur = p;
            }
            Segment::Close => {}
        }
    }
    out
}

/// pgf's trigonometry in degrees.
pub fn sin(a: f64) -> f64 {
    a.to_radians().sin()
}

pub fn cos(a: f64) -> f64 {
    a.to_radians().cos()
}

pub fn tan(a: f64) -> f64 {
    a.to_radians().tan()
}

pub fn cot(a: f64) -> f64 {
    1.0 / tan(a)
}

/// The angle between two lines (`\pgfmathanglebetweenlines`): from direction `p1 → p2` to
/// direction `q1 → q2`, in [0, 360).
pub fn angle_between_lines(p1: Point, p2: Point, q1: Point, q2: Point) -> f64 {
    let a = angle_between(p1, p2);
    let b = angle_between(q1, q2);
    let d = b - a;
    if d < 0.0 {
        d + 360.0
    } else {
        d
    }
}
