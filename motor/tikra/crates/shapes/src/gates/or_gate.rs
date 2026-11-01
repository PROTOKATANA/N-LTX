//! The US or, nor, xor and xnor gates.

use crate::scene::{Point, Segment};

use super::super::basic::ellipse_path;
use super::super::{arc_to, polar, Border, EllipseSpan, Geometry, Input};
use super::common::*;

/// The or gate's front: two arcs of radius `2·halfside` from the top to the tip and back down.
pub(super) fn or_front(v: &mut Vec<Segment>, c: Point, xc: f64, yc: f64) {
    let start = c.plus(Point::new(-0.16666 * xc, yc));
    v.push(Segment::MoveTo(start));
    let p = arc_to(v, start, EllipseSpan { a0: 90.0, a1: 30.0, rx: 2.0 * yc, ry: 2.0 * yc });
    arc_to(v, p, EllipseSpan { a0: -30.0, a1: -90.0, rx: 2.0 * yc, ry: 2.0 * yc });
}

/// The front of the or, xor and CDH and gates' borders: the arcs from the top and the bottom to the
/// tip, from the center `c` along `d` (absolute).
pub(super) fn or_front_border(c: Point, d: Point, u: &Us) -> Point {
    let (h, w, hh) = (u.h, u.w, u.hh);
    if d.y.abs() < 1e-12 {
        // and gate US's east anchor, with these dimensions
        return Point::new(c.x + (w - u.ox) / 3.0 + w, c.y);
    }
    if d.y < 0.0 {
        ray_arc(c, d, EllipticArc { center: c.plus(Point::new(-0.166666 * h, h)), span: EllipseSpan { a0: 270.0, a1: 330.0, rx: hh + h, ry: hh + h } })
    } else {
        ray_arc(c, d, EllipticArc { center: c.plus(Point::new(-0.166666 * h, -h)), span: EllipseSpan { a0: 30.0, a1: 90.0, rx: w + h, ry: hh + h } })
    }
}

/// The or gate's `north east` and `south east` without the bounding box.
pub(super) fn or_corner(c: Point, u: &Us, s: f64) -> Point {
    Point::new(c.x - 0.166666 * u.h + 0.5 * (u.w + u.h), c.y + s * (-u.h + 0.866025 * (u.hh + u.h)))
}

/// The x of an or gate's input at height `y`, on its back arc.
fn or_input_x(u: &Us, lw: f64, y: f64) -> f64 {
    let (h, yb) = (u.h, 2.0 * u.h - 0.5 * lw);
    let cosv = (y / yb).clamp(-1.0, 1.0).asin().cos();
    -1.166666 * h - 0.866025 * 2.0 * h + cosv * yb
}

/// `or gate US`, `nor gate US`, and the xor and xnor gates with their second back line.
/// An or, nor, xor or xnor gate after its keys have been read.
struct OrGate {
    xor: bool,
    inverted: bool,
    cm: Common,
    u: Us,
    bb: bool,
    c: Point,
    h: f64,
    w: f64,
    hh: f64,
    ox: f64,
    lw: f64,
    east: Point,
}

fn or_kind(input: &mut Input, kind: &str) -> OrGate {
    let xor = kind.starts_with('x');
    let inverted = kind == "nor" || kind == "xnor";
    let cm = common(input, if xor { 2 } else { 1024 });
    let u = us_dimensions(input, &cm, false);
    let c = cm.c;
    let (h, w, hh, ox, lw) = (u.h, u.w, u.hh, u.ox, cm.lw);
    let bb = input.keys.flag("tikra@gatebox");
    let east = c.plus(u.tip);
    OrGate { xor, inverted, cm, u, bb, c, h, w, hh, ox, lw, east }
}

/// A back corner. `bounding` selects the xor gate's bounding-box corner; an or gate ignores it.
fn or_back(gate: &OrGate, bounding: bool, s: f64) -> Point {
    if gate.xor {
        let xb = 2.0 * gate.h - 0.5 * gate.lw;
        Point::new(gate.c.x - 3.232051 * gate.h + 0.866025 * xb, gate.c.y + s * if bounding { gate.hh } else { 0.5 * xb })
    } else {
        Point::new(gate.c.x - 1.16666 * gate.h - 1.732050 * gate.ox, gate.c.y + s * gate.hh)
    }
}

/// The back of the border, computed once: the 720 samples only test the ray against it.
struct OrBorderFrame {
    nw_bb: Point,
    sw_bb: Point,
    north: Point,
    south: Point,
    top: f64,
    arc_center: Point,
    arc_r: f64,
}

fn or_border_frame(gate: &OrGate) -> OrBorderFrame {
    let nw_bb = or_back(gate, true, 1.0);
    let sw_bb = or_back(gate, true, -1.0);
    let north = Point::new(gate.c.x, gate.c.y + gate.hh);
    let south = Point::new(gate.c.x, gate.c.y - gate.hh);
    let shift = if gate.xor { 0.166666 * 2.0 * gate.h } else { 0.0 };
    let top_at = if gate.xor { nw_bb } else { or_back(gate, false, 1.0) };
    let top = angle(gate.c, top_at);
    let arc_center = Point::new(gate.c.x - 1.166666 * gate.h - 0.866025 * 2.0 * gate.h - gate.ox - shift, gate.c.y);
    let arc_r = gate.hh + gate.h - gate.ox;
    OrBorderFrame { nw_bb, sw_bb, north, south, top, arc_center, arc_r }
}

fn or_border_at(gate: &OrGate, frame: &OrBorderFrame, d: Point) -> Point {
    if d.x >= 0.0 {
        return or_front_border(gate.c, d, &gate.u);
    }
    let a = angle(gate.c, gate.c.plus(d));
    if a < frame.top {
        ray_line(gate.c, d, frame.north, frame.nw_bb)
    } else if a > 360.0 - frame.top {
        ray_line(gate.c, d, frame.south, frame.sw_bb)
    } else if d.y > 0.0 {
        ray_arc(gate.c, d, EllipticArc { center: frame.arc_center, span: EllipseSpan { a0: 0.0, a1: 90.0, rx: frame.arc_r, ry: frame.arc_r } })
    } else {
        ray_arc(gate.c, d, EllipticArc { center: frame.arc_center, span: EllipseSpan { a0: 270.0, a1: 360.0, rx: frame.arc_r, ry: frame.arc_r } })
    }
}

fn or_border(gate: &OrGate) -> Border {
    let frame = or_border_frame(gate);
    sampled(|d| or_border_at(gate, &frame, d))
}

fn or_west(gate: &OrGate) -> f64 {
    if gate.xor {
        let xb = 2.0 * gate.h - 0.5 * gate.lw;
        let x = gate.c.x - 1.166666 * gate.h - 0.866025 * 2.0 * gate.h + xb + 0.5 * gate.lw - gate.ox - gate.h / 3.0;
        if gate.bb {
            x - 2.0 * 0.133974 * gate.h
        } else {
            x
        }
    } else if gate.bb {
        gate.c.x - 1.16666 * gate.h - gate.ox
    } else {
        gate.c.x - 1.16666 * gate.h - gate.ox + 0.267949 * gate.h
    }
}

fn place_or_anchors(g: &mut Geometry, input: &Input, gate: &OrGate) {
    text_anchors(g, input);
    let west = or_west(gate);
    let (c, hh, bb, east) = (gate.c, gate.hh, gate.bb, gate.east);
    for (name, p) in [
        ("east", east),
        ("north east", if bb { Point::new(east.x, c.y + hh) } else { or_corner(c, &gate.u, 1.0) }),
        ("south east", if bb { Point::new(east.x, c.y - hh) } else { or_corner(c, &gate.u, -1.0) }),
        ("north", Point::new(c.x, c.y + if bb { hh } else { 0.993043 * hh })),
        ("south", Point::new(c.x, c.y - if bb { hh } else { 0.993043 * hh })),
        ("north west", or_back(gate, bb, 1.0)),
        ("south west", or_back(gate, bb, -1.0)),
        ("west", Point::new(west, c.y)),
    ] {
        g.anchor_at(name, p);
    }
    side_anchors(g, input, "east", east.x);
    side_anchors(g, input, "west", or_back(gate, bb, 1.0).x);
    let output = if gate.inverted { Point::new(c.x - 0.166666 * gate.h + 0.866025 * 2.0 * gate.h + gate.cm.outer_r + gate.cm.r + gate.ox, c.y) } else { east };
    g.anchor_at("output", output);
    let (h, lw, ox, xor) = (gate.h, gate.lw, gate.ox, gate.xor);
    let u = &gate.u;
    input_anchors(g, &gate.cm, InputSites { single: false, half: h, at_x: |_, y| or_input_x(u, lw, y) + 0.5 * lw - ox - if xor { h / 3.0 } else { 0.0 } });
}

fn or_background(g: &mut Geometry, gate: &OrGate) {
    let (xc, yc) = (gate.w - gate.ox, gate.hh - gate.u.oy);
    let mut v = Vec::new();
    or_front(&mut v, gate.c, xc, yc);
    let p = gate.c.plus(Point::new(-1.16666 * xc, -yc));
    v.push(Segment::LineTo(p));
    if gate.xor {
        let q = arc_to(&mut v, p, EllipseSpan { a0: -22.0, a1: 0.0, rx: 1.166666 * 2.66666 * yc, ry: 2.66666 * yc });
        arc_to(&mut v, q, EllipseSpan { a0: 0.0, a1: 22.0, rx: 1.166666 * 2.66666 * yc, ry: 2.66666 * yc });
    } else {
        let q = arc_to(&mut v, p, EllipseSpan { a0: -30.0, a1: 0.0, rx: 2.0 * yc, ry: 2.0 * yc });
        arc_to(&mut v, q, EllipseSpan { a0: 0.0, a1: 30.0, rx: 2.0 * yc, ry: 2.0 * yc });
    }
    v.push(Segment::Close);
    if gate.inverted {
        v.extend(ellipse_path(gate.c.plus(Point::new(-0.166666 * xc + 0.866025 * 2.0 * yc + gate.cm.outer_r, 0.0)), gate.cm.r, gate.cm.r));
    }
    let back_shift = if gate.xor { -0.333334 * gate.h } else { 0.0 };
    input_circles(&mut v, &gate.cm, yc, |y| or_input_x(&gate.u, gate.lw, y) + back_shift - gate.cm.r);
    if gate.xor {
        // the second line at the back, as short lines (so that it is not filled)
        xor_back_line(&mut v, gate.c, gate.h);
    }
    g.background(v);
}

fn xor_back_line(v: &mut Vec<Segment>, c: Point, h: f64) {
    let m = c.plus(Point::new(-3.232051 * h, 0.0));
    v.push(Segment::MoveTo(c.plus(Point::new(-1.5 * h, -h))));
    for k in 1..61 {
        let p = m.plus(polar(k as f64 - 30.0, 2.0 * h));
        v.push(Segment::LineTo(p));
        v.push(Segment::MoveTo(p));
    }
}

/// `or gate US`, `nor gate US`, and the xor and xnor gates with their second back line.
pub fn or_gate_us(input: &mut Input, kind: &str) -> Geometry {
    let gate = or_kind(input, kind);
    let mut g = Geometry::new(gate.c, or_border(&gate));
    place_or_anchors(&mut g, input, &gate);
    or_background(&mut g, &gate);
    g
}
