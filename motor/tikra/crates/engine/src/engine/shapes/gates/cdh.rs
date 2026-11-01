//! The CDH and and nand gates.

use crate::scene::{Point, Segment};

use super::super::basic::ellipse_path;
use super::super::{Geometry, Input};
use super::common::*;
use super::or_gate::{or_corner, or_front, or_front_border};

/// `and gate CDH` and `nand gate CDH`: the or gate's front on the and gate's square back.
pub fn and_gate_cdh(input: &mut Input, nand: bool) -> Geometry {
    let cm = common(input, 1024);
    let u = us_dimensions(input, &cm, false);
    let c = cm.c;
    let (h, w, hh, ox, oy) = (u.h, u.w, u.hh, u.ox, u.oy);
    let bb = input.keys.flag("tikra@gatebox");
    let east = c.plus(u.tip);
    let border = sampled(|d| if d.x < 0.0 { c.plus(super::super::border_rectangle(d, Point::new(0.166666 * h + w, hh))) } else { or_front_border(c, d, &u) });
    let mut g = Geometry::new(c, border);
    text_anchors(&mut g, input);
    let x_back = c.x - 1.16666 * h - ox;
    for (name, p) in [
        ("east", east),
        ("north east", if bb { Point::new(east.x, c.y + hh) } else { or_corner(c, &u, 1.0) }),
        ("south east", if bb { Point::new(east.x, c.y - hh) } else { or_corner(c, &u, -1.0) }),
        ("north", Point::new(c.x, c.y + if bb { hh } else { 0.993043 * hh })),
        ("south", Point::new(c.x, c.y - if bb { hh } else { 0.993043 * hh })),
        ("north west", Point::new(x_back, c.y + hh)),
        ("south west", Point::new(x_back, c.y - hh)),
        ("west", Point::new(x_back, c.y)),
    ] {
        g.anchor_at(name, p);
    }
    side_anchors(&mut g, input, "east", east.x);
    side_anchors(&mut g, input, "west", x_back);
    let output = if nand { Point::new(c.x - 0.166666 * h + 0.866025 * 2.0 * h + cm.r + cm.outer_r + ox, c.y) } else { east };
    g.anchor_at("output", output);
    input_anchors(&mut g, &cm, InputSites { single: false, half: h, at_x: |_, _| -0.166666 * h - w });
    let (xc, yc) = (w - ox, hh - oy);
    let mut v = Vec::new();
    or_front(&mut v, c, xc, yc);
    v.push(Segment::LineTo(c.plus(Point::new(-1.16666 * xc, -yc))));
    v.push(Segment::LineTo(c.plus(Point::new(-1.16666 * xc, yc))));
    v.push(Segment::Close);
    if nand {
        v.extend(ellipse_path(c.plus(Point::new(-0.166666 * xc + 0.866025 * 2.0 * yc + cm.outer_r, 0.0)), cm.r, cm.r));
    }
    input_circles(&mut v, &cm, yc, |_| -1.16666 * h - 0.5 * cm.lw - cm.r);
    g.background(v);
    g
}
