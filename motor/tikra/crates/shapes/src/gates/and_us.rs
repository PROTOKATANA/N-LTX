//! The US and and nand gates.

use crate::scene::{Point, Segment};

use super::super::basic::ellipse_path;
use super::super::{arc_to, EllipseSpan, Geometry, Input};
use super::common::*;

// pgf's constants as it writes them (`.707106`), not the exact ones
#[allow(clippy::approx_constant)]
pub fn and_gate_us(input: &mut Input, nand: bool) -> Geometry {
    let cm = common(input, 1024);
    let u = us_dimensions(input, &cm, true);
    let c = cm.c;
    let (h, w, hh, ox) = (u.h, u.w, u.hh, u.ox);
    let bb = input.keys.flag("tikra@gatebox");
    let east = Point::new(c.x + (w - ox) / 3.0 + w, c.y);
    let nw = Point::new(c.x - 1.16666 * h - ox, c.y + hh);
    let sw = Point::new(nw.x, c.y - hh);
    let arc_top = c.plus(Point::new((w - ox) / 3.0, hh));
    let arc_bottom = c.plus(Point::new((w - ox) / 3.0, -hh));
    let border = sampled(|d| {
        let q = c.plus(d);
        let a = angle(c, q);
        let m = c.plus(Point::new(h / 3.0, 0.0));
        if a < angle(c, nw) {
            if a < angle(c, arc_top) {
                ray_arc(c, d, EllipticArc { center: m, span: EllipseSpan { a0: 0.0, a1: 90.0, rx: w, ry: hh } })
            } else {
                ray_line(c, d, arc_top, nw)
            }
        } else if a > angle(c, arc_bottom) {
            ray_arc(c, d, EllipticArc { center: m, span: EllipseSpan { a0: 270.0, a1: 360.0, rx: w, ry: hh } })
        } else if a > angle(c, sw) {
            ray_line(c, d, arc_bottom, sw)
        } else {
            ray_line(c, d, nw, sw)
        }
    });
    let mut g = Geometry::new(c, border);
    text_anchors(&mut g, input);
    let corner = |s: f64| {
        if bb {
            Point::new(east.x, c.y + s * hh)
        } else {
            Point::new(c.x + (w - ox) / 3.0 + 0.707106 * w, c.y + s * 0.707106 * hh)
        }
    };
    for (name, p) in [
        ("east", east),
        ("north east", corner(1.0)),
        ("south east", corner(-1.0)),
        ("north", Point::new(c.x, c.y + hh)),
        ("south", Point::new(c.x, c.y - hh)),
        ("north west", nw),
        ("south west", sw),
        ("west", Point::new(nw.x, c.y)),
    ] {
        g.anchor_at(name, p);
    }
    side_anchors(&mut g, input, "east", east.x);
    side_anchors(&mut g, input, "west", nw.x);
    let output = if nand { Point::new(c.x + 1.33333 * h + cm.r + cm.outer_r + ox, c.y) } else { east };
    g.anchor_at("output", output);
    input_anchors(&mut g, &cm, InputSites { single: false, half: h, at_x: |_, _| -0.166666 * h - w });
    // the background: a half circle in front of a square
    let (xc, yc) = (w - ox, hh - u.oy);
    let mut v = Vec::new();
    let start = c.plus(Point::new(xc / 3.0, yc));
    v.push(Segment::MoveTo(start));
    arc_to(&mut v, start, EllipseSpan { a0: 90.0, a1: -90.0, rx: yc, ry: yc });
    v.push(Segment::LineTo(c.plus(Point::new(-1.16666 * xc, -yc))));
    v.push(Segment::LineTo(c.plus(Point::new(-1.16666 * xc, yc))));
    v.push(Segment::Close);
    input_circles(&mut v, &cm, yc, |_| -1.16666 * h - 0.5 * cm.lw - cm.r);
    if nand {
        v.extend(ellipse_path(c.plus(Point::new(1.33333 * xc + cm.outer_r, 0.0)), cm.r, cm.r));
    }
    g.background(v);
    g
}
