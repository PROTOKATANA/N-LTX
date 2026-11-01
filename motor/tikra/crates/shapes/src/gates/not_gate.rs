//! The US not and buffer gates.

use crate::scene::{Point, Segment};

use super::super::basic::ellipse_path;
use super::super::{Geometry, Input};
use super::common::*;

/// `not gate US` and `buffer gate US`: a triangle, with a circle at the not gate's tip.
pub fn not_gate_us(input: &mut Input, not: bool) -> Geometry {
    let cm = common(input, 1);
    let u = us_dimensions(input, &cm, false);
    let c = cm.c;
    let (h, w, hh, ox, oy) = (u.h, u.w, u.hh, u.ox, u.oy);
    let bb = input.keys.flag("tikra@gatebox");
    let tip = c.plus(u.tip);
    let far_tip = tip.plus(Point::new(2.350943 * ox, 0.0));
    let x_back = c.x - 0.833333 * h - ox;
    let nw = Point::new(x_back, c.y + 1.166666 * h + 1.565 * oy);
    let sw = Point::new(x_back, c.y - 1.166666 * h - 1.565 * oy);
    let east = if not { tip } else { far_tip };
    let output = if not { Point::new(c.x - 0.166666 * h + 0.866025 * 2.0 * h + cm.r + cm.outer_r + ox, c.y) } else { east };
    let in_x = c.x - 0.166666 * h - w + h / 3.0 - if cm.inputs.first() == Some(&true) { cm.r + cm.outer_r } else { 0.0 };
    let input_point = Point::new(in_x, c.y);
    let border = sampled(|d| {
        if d.y.abs() < 1e-12 {
            return if d.x > 0.0 { output } else { input_point };
        }
        let a = angle(c, c.plus(d));
        if a < angle(c, nw) {
            ray_line(c, d, nw, far_tip)
        } else if a < angle(c, sw) {
            ray_line(c, d, nw, sw)
        } else {
            ray_line(c, d, sw, far_tip)
        }
    });
    let mut g = Geometry::new(c, border);
    text_anchors(&mut g, input);
    let on_side = |dir: Point, corner: Point| ray_line(c, dir, corner, far_tip);
    for (name, p) in [
        ("east", east),
        ("output", output),
        ("north", if bb { Point::new(c.x, nw.y) } else { on_side(Point::new(0.0, 1.0), nw) }),
        ("south", if bb { Point::new(c.x, sw.y) } else { on_side(Point::new(0.0, -1.0), sw) }),
        ("north east", if bb { Point::new(tip.x, nw.y) } else { on_side(Point::new(1.0, 1.0), nw) }),
        ("south east", if bb { Point::new(tip.x, sw.y) } else { on_side(Point::new(1.0, -1.0), sw) }),
        ("north west", nw),
        ("south west", sw),
        ("west", Point::new(x_back, c.y)),
        ("input", input_point),
    ] {
        g.anchor_at(name, p);
    }
    side_anchors(&mut g, input, "east", east.x);
    side_anchors(&mut g, input, "west", x_back);
    let (xc, yc) = (w - ox, hh - oy);
    let mut v = vec![Segment::MoveTo(tip), Segment::LineTo(c.plus(Point::new(-0.833333 * xc, 1.166666 * yc))), Segment::LineTo(c.plus(Point::new(-0.833333 * xc, -1.166666 * yc)))];
    if not {
        // the not gate goes back to its tip instead of closing
        v.push(Segment::LineTo(tip));
        v.extend(ellipse_path(c.plus(Point::new(-0.166666 * xc + 0.866025 * 2.0 * yc + cm.outer_r, 0.0)), cm.r, cm.r));
    } else {
        v.push(Segment::Close);
    }
    if cm.inputs.first() == Some(&true) {
        v.extend(ellipse_path(c.plus(Point::new(-0.833333 * h - 0.5 * cm.lw - cm.r, 0.0)), cm.r, cm.r));
    }
    g.background(v);
    g
}
