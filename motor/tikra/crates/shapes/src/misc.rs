//! The shapes of `pgflibraryshapes.misc.code.tex`: cross out, strike out, rounded rectangle and
//! chamfered rectangle.

use crate::scene::{Point, Segment};

use super::basic::{box_anchors, rectangle_corners};
use super::{arc_points, arc_to, cos, cot, miter_offset, polygon_path, ray, sin, tan, Border, EllipseSpan, Geometry, Input, Layer, Usage};

/// A rectangle's anchors and border, with nothing in the background: what `cross out` and `strike
/// out` inherit.
fn rectangle_without_background(input: &Input) -> (Geometry, Point, Point) {
    let (sw, ne) = rectangle_corners(input);
    let mut g = Geometry::new(sw.lerp(ne, 0.5), Border::Rectangle(ne.minus(sw).scale(0.5)));
    box_anchors(&mut g, sw, ne, input.ex);
    (g, sw, ne)
}

/// Two lines across the rectangle's corners (outer sep included), in the foreground.
pub fn cross_out(input: &mut Input) -> Geometry {
    let (mut g, sw, ne) = rectangle_without_background(input);
    let v = vec![Segment::MoveTo(sw), Segment::LineTo(ne), Segment::MoveTo(Point::new(sw.x, ne.y)), Segment::LineTo(Point::new(ne.x, sw.y))];
    g.path(Layer::Foreground, v, Usage::Node);
    g
}

/// One line from the south west corner to the north east one.
pub fn strike_out(input: &mut Input) -> Geometry {
    let (mut g, sw, ne) = rectangle_without_background(input);
    g.path(Layer::Foreground, vec![Segment::MoveTo(sw), Segment::LineTo(ne)], Usage::Node);
    g
}

#[derive(Clone, Copy, PartialEq)]
enum Arc {
    Convex,
    Concave,
    None,
}

fn arc_kind(s: &str) -> Arc {
    match s.trim() {
        "convex" => Arc::Convex,
        "concave" => Arc::Concave,
        _ => Arc::None,
    }
}

/// The sizes a rounded rectangle reads before it draws: arcs, text, separations and center.
#[derive(Clone, Copy)]
struct RoundedMetrics {
    west: Arc,
    east: Arc,
    a: f64,
    hh: f64,
    r: f64,
    arc_w: f64,
    chord_w: f64,
    hw: f64,
    ox: f64,
    oy: f64,
    concave_shift: f64,
    convex_shift: f64,
    c: Point,
}

/// Corners, outline and the west/east anchor x, built from the metrics.
struct RoundedShape {
    m: RoundedMetrics,
    ne: Point,
    se: Point,
    nw: Point,
    sw: Point,
    outline: Vec<Point>,
    wx: f64,
    ex: f64,
    top: f64,
    bottom: f64,
}

fn rounded_metrics(input: &mut Input) -> RoundedMetrics {
    let west = arc_kind(&input.keys.text("/pgf/rounded rectangle west arc"));
    let east = arc_kind(&input.keys.text("/pgf/rounded rectangle east arc"));
    let a = input.number("/pgf/rounded rectangle arc length", 180.0) / 2.0;
    let t = input.text();
    let (half_text_w, half_text_h) = (t.wd / 2.0, (t.ht + t.dp) / 2.0);
    let x = input.inner_xsep + half_text_w;
    let hh = (input.inner_ysep + half_text_h).max(input.minimum_height / 2.0);
    let r = hh / sin(a);
    // the width of an arc, and of the chord when the text goes into a convex arc
    let arc_w = r - r * cos(a);
    let chord_w = r - r * cos((half_text_h / r).clamp(-1.0, 1.0).asin().to_degrees());
    let extra = |k: Arc| match k {
        Arc::Concave => arc_w,
        Arc::Convex => chord_w,
        Arc::None => 0.0,
    };
    let width = 2.0 * x + extra(west) + extra(east);
    let mut x_offset = if width < input.minimum_width { (input.minimum_width - width) / 2.0 } else { input.inner_xsep };
    // wider if the convex arcs overlap
    if r - chord_w > half_text_w + x_offset {
        x_offset = r - chord_w - half_text_w;
    }
    let hw = half_text_w + x_offset;
    let (ox, oy) = (input.outer_xsep, input.outer_ysep);
    let (concave_shift, convex_shift) = if a == 90.0 { (0.0, 0.0) } else { (ox * cot((90.0 - a) / 2.0), ox * cot((90.0 + a) / 2.0)) };
    let c = input.center_point();
    RoundedMetrics { west, east, a, hh, r, arc_w, chord_w, hw, ox, oy, concave_shift, convex_shift, c }
}

fn side_x(m: &RoundedMetrics, k: Arc) -> f64 {
    match k {
        Arc::Concave => m.hw + m.arc_w + m.concave_shift,
        Arc::Convex => m.hw + m.chord_w + m.ox,
        Arc::None => m.hw + m.ox,
    }
}

fn corner_x(m: &RoundedMetrics, k: Arc) -> f64 {
    if k == Arc::Convex {
        m.hw + m.chord_w - m.arc_w + m.convex_shift
    } else {
        side_x(m, k)
    }
}

fn rounded_border(m: RoundedMetrics) -> RoundedShape {
    let p = |dx: f64, dy: f64| m.c.plus(Point::new(dx, dy));
    let (top, bottom) = (m.hh + m.oy, -m.hh - m.oy);
    let (ne, se) = (p(corner_x(&m, m.east), top), p(corner_x(&m, m.east), bottom));
    let (nw, sw) = (p(-corner_x(&m, m.west), top), p(-corner_x(&m, m.west), bottom));
    // the border: the sides, and the convex arcs grown by the outer seps
    let mut outline = vec![nw, ne];
    if m.east == Arc::Convex {
        outline.extend(arc_points(p(m.hw + m.chord_w - m.r, 0.0), EllipseSpan { a0: m.a, a1: -m.a, rx: m.r + m.ox, ry: m.r + m.oy }));
    }
    outline.push(se);
    outline.push(sw);
    if m.west == Arc::Convex {
        outline.extend(arc_points(p(-m.hw - m.chord_w + m.r, 0.0), EllipseSpan { a0: 180.0 + m.a, a1: 180.0 - m.a, rx: m.r + m.ox, ry: m.r + m.oy }));
    }
    let (wx, ex) = (p(-side_x(&m, m.west), 0.0).x, p(side_x(&m, m.east), 0.0).x);
    RoundedShape { m, ne, se, nw, sw, outline, wx, ex, top, bottom }
}

pub fn rounded_rectangle(input: &mut Input) -> Geometry {
    let mut shape = rounded_border(rounded_metrics(input));
    let outline = std::mem::take(&mut shape.outline);
    let mut g = Geometry::new(shape.m.c, Border::Outline(outline));
    place_rounded_anchors(&mut g, input, &shape);
    rounded_background(&mut g, &shape);
    g
}

fn place_rounded_anchors(g: &mut Geometry, input: &Input, shape: &RoundedShape) {
    let c = shape.m.c;
    let p = |dx: f64, dy: f64| c.plus(Point::new(dx, dy));
    let (top, bottom) = (shape.top, shape.bottom);
    let (wx, ex) = (shape.wx, shape.ex);
    let (nw, sw, ne, se) = (shape.nw, shape.sw, shape.ne, shape.se);
    let m = 0.5 * input.ex;
    for (name, q) in [
        ("center", c),
        ("mid", input.mid_point()),
        ("base", input.base_point()),
        ("north", p(0.0, top)),
        ("south", p(0.0, bottom)),
        ("west", Point::new(wx, c.y)),
        ("east", Point::new(ex, c.y)),
        ("mid west", Point::new(wx, m)),
        ("mid east", Point::new(ex, m)),
        ("base west", Point::new(wx, 0.0)),
        ("base east", Point::new(ex, 0.0)),
        ("north west", nw),
        ("south west", sw),
        ("north east", ne),
        ("south east", se),
    ] {
        g.anchor_at(name, q);
    }
}

fn rounded_background(g: &mut Geometry, shape: &RoundedShape) {
    let m = &shape.m;
    let p = |dx: f64, dy: f64| m.c.plus(Point::new(dx, dy));
    let mut v = vec![Segment::MoveTo(p(0.0, m.hh))];
    match m.east {
        Arc::Concave => {
            let q = p(m.hw + m.arc_w, m.hh);
            v.push(Segment::LineTo(q));
            arc_to(&mut v, q, EllipseSpan { a0: 180.0 - m.a, a1: 180.0 + m.a, rx: m.r, ry: m.r });
        }
        Arc::Convex => {
            let q = p(m.hw + m.chord_w - m.arc_w, m.hh);
            v.push(Segment::LineTo(q));
            arc_to(&mut v, q, EllipseSpan { a0: m.a, a1: -m.a, rx: m.r, ry: m.r });
        }
        Arc::None => {
            v.push(Segment::LineTo(p(m.hw, m.hh)));
            v.push(Segment::LineTo(p(m.hw, -m.hh)));
        }
    }
    match m.west {
        Arc::Concave => {
            let q = p(-m.hw - m.arc_w, -m.hh);
            v.push(Segment::LineTo(q));
            arc_to(&mut v, q, EllipseSpan { a0: -m.a, a1: m.a, rx: m.r, ry: m.r });
        }
        Arc::Convex => {
            let q = p(-m.hw - m.chord_w + m.arc_w, -m.hh);
            v.push(Segment::LineTo(q));
            arc_to(&mut v, q, EllipseSpan { a0: 180.0 + m.a, a1: 180.0 - m.a, rx: m.r, ry: m.r });
        }
        Arc::None => {
            v.push(Segment::LineTo(p(-m.hw, -m.hh)));
            v.push(Segment::LineTo(p(-m.hw, m.hh)));
        }
    }
    v.push(Segment::Close);
    g.background(v);
}

pub fn chamfered_rectangle(input: &mut Input) -> Geometry {
    // which corners are chamfered (pgf reads "top left" as north east, and so on)
    let corners = input.keys.text("/pgf/chamfered rectangle corners");
    let all = corners.contains("chamfer all");
    let listed: Vec<String> = corners.split(',').map(|s| s.split_whitespace().collect::<Vec<_>>().join(" ")).collect();
    let has = |a: &str, b: &str| all || listed.iter().any(|s| s == a || s == b);
    let (ne_c, se_c, sw_c, nw_c) = (has("north east", "top left"), has("south east", "bottom right"), has("south west", "bottom left"), has("north west", "top right"));
    let (mut xa, mut ya) = input.half_with_inner();
    let angle = 90.0 - input.number("/pgf/chamfered rectangle angle", 45.0).clamp(1.0, 89.0);
    let (tan_a, cot_a) = (tan(angle), cot(angle));
    let ex = input.ex;
    let mut xb = input.length("/pgf/chamfered rectangle xsep", 0.666 * ex);
    let mut yc = tan_a * xb;
    if yc > ya {
        yc = ya;
        xb = cot_a * yc;
    }
    let mut yb = input.length("/pgf/chamfered rectangle ysep", 0.666 * ex);
    let mut xc = cot_a * yb;
    if xc > xa {
        xc = xa;
        yb = tan_a * xc;
    }
    if xa + xb < input.minimum_width / 2.0 {
        xa = input.minimum_width / 2.0 - xb;
    }
    if ya + yb < input.minimum_height / 2.0 {
        ya = input.minimum_height / 2.0 - yb;
    }
    let c = input.center_point();
    let p = |dx: f64, dy: f64| c.plus(Point::new(dx, dy));
    let corner = |chamfered: bool, sx: f64, sy: f64| {
        if chamfered {
            p(sx * xa, sy * ya)
        } else {
            p(sx * (xa + xb), sy * (ya + yb))
        }
    };
    let shape = [
        p(xa + xb, ya - yc),
        corner(ne_c, 1.0, 1.0),
        p(xa - xc, ya + yb),
        p(-xa + xc, ya + yb),
        corner(nw_c, -1.0, 1.0),
        p(-xa - xb, ya - yc),
        p(-xa - xb, -ya + yc),
        corner(sw_c, -1.0, -1.0),
        p(-xa + xc, -ya - yb),
        p(xa - xc, -ya - yb),
        corner(se_c, 1.0, -1.0),
        p(xa + xb, -ya + yc),
    ];
    let b = miter_offset(&shape, input.outer_sep());
    let mut g = Geometry::new(c, Border::Outline(b.clone()));
    for (name, q) in [
        ("center", c),
        ("mid", input.mid_point()),
        ("base", input.base_point()),
        ("before north east", b[0]),
        ("north east", b[1]),
        ("after north east", b[2]),
        ("north", Point::new(c.x, b[2].y)),
        ("before north west", b[3]),
        ("north west", b[4]),
        ("after north west", b[5]),
        ("west", Point::new(b[6].x, c.y)),
        ("before south west", b[6]),
        ("south west", b[7]),
        ("after south west", b[8]),
        ("south", Point::new(c.x, b[8].y)),
        ("before south east", b[9]),
        ("south east", b[10]),
        ("after south east", b[11]),
        ("east", Point::new(b[0].x, c.y)),
    ] {
        g.anchor_at(name, q);
    }
    for (name, o, d) in
        [("mid west", input.mid_point(), -1.0), ("mid east", input.mid_point(), 1.0), ("base west", input.base_point(), -1.0), ("base east", input.base_point(), 1.0)]
    {
        g.anchor_at(name, ray(o, Point::new(d, 0.0), &b));
    }
    g.background(polygon_path(&shape));
    g.roundable = true;
    g
}
