use std::rc::Rc;

use tikra_engine::engine::nodes::NodeInfo;
use tikra_engine::engine::shapes::{Border, Geometry};
use tikra_engine::scene::{Point, Transform};

use crate::Engine;

use super::frame::*;

/// pgfplots' anchors of an axis: the box's compass anchors, `outer …` ones of the bounding box,
/// `origin`, `left of origin`… and `right of north east`…
/// The axis box and the picture box a named anchor is measured against.
pub(in crate::draw) struct AnchorPlace<'a> {
    pub(in crate::draw) lo: Point,
    pub(in crate::draw) hi: Point,
    pub(in crate::draw) outer_lo: Point,
    pub(in crate::draw) outer_hi: Point,
    pub(in crate::draw) frame: &'a Frame,
}

pub(in crate::draw) fn named_anchor(name: &str, place: &AnchorPlace<'_>) -> Option<Point> {
    let &AnchorPlace { lo, hi, outer_lo: blo, outer_hi: bhi, frame: f } = place;
    let compass = |a: &str, lo: Point, hi: Point| -> Option<Point> {
        let (cx, cy) = ((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
        Some(match a {
            "south west" => lo,
            "south east" => Point::new(hi.x, lo.y),
            "north west" => Point::new(lo.x, hi.y),
            "north east" => hi,
            "south" => Point::new(cx, lo.y),
            "north" => Point::new(cx, hi.y),
            "west" => Point::new(lo.x, cy),
            "east" => Point::new(hi.x, cy),
            "center" => Point::new(cx, cy),
            _ => return None,
        })
    };
    let n = name.trim();
    if let Some(rest) = n.strip_prefix("outer ") {
        return compass(rest, blo, bhi);
    }
    if let Some(p) = compass(n, lo, hi) {
        return Some(p);
    }
    let origin = f.at([f.zero[0], f.zero[1], f.zero[2]]);
    // where the line through the origin along an axis meets an edge of the box (a 3d axis'
    // lines are slanted)
    let at_y = |dir: Point, y: f64| if dir.y.abs() > 1e-12 { origin.plus(dir.scale((y - origin.y) / dir.y)) } else { Point::new(origin.x, y) };
    let at_x = |dir: Point, x: f64| if dir.x.abs() > 1e-12 { origin.plus(dir.scale((x - origin.x) / dir.x)) } else { Point::new(x, origin.y) };
    Some(match n {
        "origin" => origin,
        "left of origin" => at_x(f.e[0], lo.x),
        "right of origin" => at_x(f.e[0], hi.x),
        "above origin" => at_y(f.e[1], hi.y),
        "below origin" => at_y(f.e[1], lo.y),
        "right of north east" => Point::new(bhi.x, hi.y),
        "right of north west" => Point::new(blo.x.max(lo.x), hi.y),
        "right of south east" => Point::new(bhi.x, lo.y),
        "left of north west" => Point::new(blo.x, hi.y),
        "left of south west" => Point::new(blo.x, lo.y),
        "left of north east" => Point::new(lo.x, hi.y),
        "above north west" => Point::new(lo.x, bhi.y),
        "above north east" => Point::new(hi.x, bhi.y),
        "above south west" => Point::new(lo.x, bhi.y),
        "below south west" => Point::new(lo.x, blo.y),
        "below south east" => Point::new(hi.x, blo.y),
        "below north west" => Point::new(lo.x, blo.y),
        _ => return None,
    })
}

pub(in crate::draw) fn register_axis_node(e: &mut Engine, name: &str, place: &AnchorPlace<'_>) {
    let &AnchorPlace { lo, hi, .. } = place;
    let c = Point::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
    let (hx, hy) = ((hi.x - lo.x) / 2.0, (hi.y - lo.y) / 2.0);
    let mut g = Geometry::new(Point::default(), Border::Rectangle(Point::new(hx, hy)));
    for (a, x, y) in [
        ("center", 0.0, 0.0),
        ("north", 0.0, hy),
        ("south", 0.0, -hy),
        ("east", hx, 0.0),
        ("west", -hx, 0.0),
        ("north east", hx, hy),
        ("north west", -hx, hy),
        ("south east", hx, -hy),
        ("south west", -hx, -hy),
        ("mid", 0.0, 0.0),
        ("mid east", hx, 0.0),
        ("mid west", -hx, 0.0),
        ("base", 0.0, 0.0),
        ("base east", hx, 0.0),
        ("base west", -hx, 0.0),
    ] {
        g.anchor_at(a, Point::new(x, y));
    }
    let info = NodeInfo { shape: "rectangle".into(), geometry: Rc::new(g), transform: Transform::translation(c.x, c.y) };
    e.nodes.insert(name.to_string(), info);
    for a in [
        "origin",
        "left of origin",
        "right of origin",
        "above origin",
        "below origin",
        "outer north east",
        "outer north west",
        "outer south east",
        "outer south west",
        "outer north",
        "outer south",
        "outer east",
        "outer west",
        "outer center",
        "right of north east",
        "right of south east",
        "left of north west",
        "left of south west",
        "above north west",
        "above north east",
        "below south west",
        "below south east",
    ] {
        if let Some(p) = named_anchor(a, place) {
            let mut g = Geometry::new(Point::default(), Border::Point);
            g.anchor_at("center", Point::default());
            let info = NodeInfo { shape: "coordinate".into(), geometry: Rc::new(g), transform: Transform::translation(p.x, p.y) };
            e.nodes.insert(format!("{name}@{a}"), info);
        }
    }
}
