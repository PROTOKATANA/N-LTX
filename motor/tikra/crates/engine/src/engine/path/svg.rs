use crate::scene::{Point, Segment};
use crate::tex::{Cat, Token};
use crate::Error;

use super::super::coords::Coord;
use super::super::nodes::OnPath;
use super::super::Engine;
use super::*;

/// `svg[options] {M 0 0 L 20 20 h 10 a 10 10 0 0 0 -20 0}` (or `svg "…"`): SVG's path data, its unit the pt and its
/// y axis up (pgf's `\pgfpathsvg`), in the coordinate system the options make; relative commands
/// start from the path's current point.
pub(super) fn svg_operation(e: &mut Engine, b: &mut Builder, depth: Option<usize>) -> Result<(), Error> {
    e.begin_group();
    let r = (|| -> Result<(), Error> {
        if let Some(t) = peek(e, depth)? {
            if t.is_char('[') {
                let _ = next(e, depth)?;
                let o = e.tex.read_until_char(']', false)?;
                e.process_keys(o, Some("/tikz"))?;
                e.st.apply_pending();
            }
        }
        let data = match next(e, depth)? {
            Some(Token::Char(_, Cat::Begin)) => e.tex.read_group_body()?,
            // or in quotes, as the svg.path library also takes it
            Some(t) if t.is_char('"') => e.tex.read_until_char('"', false)?,
            _ => return Err(Error::Figure("svg without a {path} or \"path\"".into())),
        };
        let text = e.tex.expand_to_string(data)?;
        svg_data(e, b, &text);
        Ok(())
    })();
    e.end_group();
    r
}

/// The numbers and command letters of SVG path data, in order.
pub(super) fn svg_tokens(text: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        let starts_number = c == '-' && !cur.is_empty() && !cur.ends_with(['e', 'E']);
        if c.is_ascii_alphabetic() && c != 'e' && c != 'E' || c.is_whitespace() || c == ',' || starts_number {
            if !cur.is_empty() {
                tokens.push(std::mem::take(&mut cur));
            }
            if c.is_ascii_alphabetic() {
                tokens.push(c.to_string());
            } else if c == '-' {
                cur.push(c);
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        tokens.push(cur);
    }
    tokens
}

/// Where the SVG pen is, in the data's own unit.
pub(super) struct SvgPen {
    last: Point,
    start: Point,
    prev_control: Option<Point>,
    prev_quad: Option<Point>,
    cmd: char,
}

/// The data still to read.
pub(super) struct SvgSrc<'a> {
    tokens: &'a [String],
    i: usize,
}

pub(super) fn svg_num(src: &mut SvgSrc<'_>) -> Option<f64> {
    let v = src.tokens.get(src.i)?.parse().ok()?;
    src.i += 1;
    Some(v)
}

pub(super) fn svg_pt(src: &mut SvgSrc<'_>, base: Point) -> Option<Point> {
    Some(Point::new(svg_num(src)?, svg_num(src)?).plus(base))
}

pub(super) fn svg_canvas(t: &crate::scene::Transform, p: Point) -> Point {
    t.apply(p.scale(crate::PT))
}

pub(super) fn svg_local(t: &crate::scene::Transform, c: Point) -> Point {
    t.inverse().apply(c).scale(1.0 / crate::PT)
}

/// The pen and the data it is reading.
pub(super) struct SvgCursor<'a> {
    pen: SvgPen,
    src: SvgSrc<'a>,
}

/// `C` and `S`. False when the numbers run out.
pub(super) fn svg_cubic(b: &mut Builder, cur: &mut SvgCursor<'_>, t: &crate::scene::Transform) -> bool {
    let rel = cur.pen.cmd.is_ascii_lowercase();
    let base = if rel { cur.pen.last } else { Point::default() };
    let c1 = if cur.pen.cmd.eq_ignore_ascii_case(&'S') {
        cur.pen.prev_control.map_or(cur.pen.last, |c| cur.pen.last.scale(2.0).minus(c))
    } else {
        let Some(c) = svg_pt(&mut cur.src, base) else { return false };
        c
    };
    let Some(c2) = svg_pt(&mut cur.src, base) else { return false };
    let Some(p) = svg_pt(&mut cur.src, base) else { return false };
    b.segs.push(Segment::CurveTo(svg_canvas(t, c1), svg_canvas(t, c2), svg_canvas(t, p)));
    cur.pen.prev_control = Some(c2);
    cur.pen.last = p;
    true
}

/// `Q` and `T`. False when the numbers run out.
pub(super) fn svg_quad(b: &mut Builder, cur: &mut SvgCursor<'_>, t: &crate::scene::Transform) -> bool {
    let rel = cur.pen.cmd.is_ascii_lowercase();
    let base = if rel { cur.pen.last } else { Point::default() };
    let q = if cur.pen.cmd.eq_ignore_ascii_case(&'T') {
        cur.pen.prev_quad.map_or(cur.pen.last, |c| cur.pen.last.scale(2.0).minus(c))
    } else {
        let Some(c) = svg_pt(&mut cur.src, base) else { return false };
        c
    };
    let Some(p) = svg_pt(&mut cur.src, base) else { return false };
    b.segs.push(Segment::CurveTo(svg_canvas(t, cur.pen.last.lerp(q, 2.0 / 3.0)), svg_canvas(t, p.lerp(q, 2.0 / 3.0)), svg_canvas(t, p)));
    cur.pen.prev_quad = Some(q);
    cur.pen.last = p;
    true
}

/// `A`. False when the numbers run out.
pub(super) fn svg_arc_cmd(b: &mut Builder, cur: &mut SvgCursor<'_>, t: &crate::scene::Transform) -> bool {
    let rel = cur.pen.cmd.is_ascii_lowercase();
    let base = if rel { cur.pen.last } else { Point::default() };
    let Some(rx) = svg_num(&mut cur.src) else { return false };
    let Some(ry) = svg_num(&mut cur.src) else { return false };
    let Some(phi) = svg_num(&mut cur.src) else { return false };
    let Some(large) = svg_num(&mut cur.src) else { return false };
    let Some(sweep) = svg_num(&mut cur.src) else { return false };
    let Some(p) = svg_pt(&mut cur.src, base) else { return false };
    // with y up, pgf turns the other way for the same sweep flag
    svg_arc(b, SvgArc { from: cur.pen.last, to: p, rx: rx.abs(), ry: ry.abs(), phi, large: large != 0.0, sweep: sweep == 0.0 }, t);
    cur.pen.last = p;
    true
}

/// The path data of SVG (`M 0 0 l 20 0 …`, in pt) onto the path being built.
pub(super) fn svg_data(e: &Engine, b: &mut Builder, text: &str) {
    let t = e.st.transform;
    let tokens = svg_tokens(text);
    let src = SvgSrc { tokens: &tokens, i: 0 };
    let mut pen = SvgPen { last: svg_local(&t, b.point()), start: Point::default(), prev_control: None, prev_quad: None, cmd: 'M' };
    pen.start = pen.last;
    let mut cur = SvgCursor { pen, src };
    while cur.src.i < cur.src.tokens.len() {
        if let Some(c) = cur.src.tokens[cur.src.i].chars().next().filter(|c| c.is_ascii_alphabetic()) {
            cur.pen.cmd = c;
            cur.src.i += 1;
            if c == 'Z' || c == 'z' {
                b.segs.push(Segment::Close);
                cur.pen.last = cur.pen.start;
                b.current = Some(Coord { p: svg_canvas(&t, cur.pen.last), node: None });
                continue;
            }
        }
        let rel = cur.pen.cmd.is_ascii_lowercase();
        let base = if rel { cur.pen.last } else { Point::default() };
        let before = cur.src.i;
        match cur.pen.cmd.to_ascii_uppercase() {
            'M' => {
                let Some(p) = svg_pt(&mut cur.src, base) else { break };
                b.move_to(Coord { p: svg_canvas(&t, p), node: None });
                (cur.pen.last, cur.pen.start) = (p, p);
                // more pairs after a moveto are lines
                cur.pen.cmd = if rel { 'l' } else { 'L' };
            }
            'L' => {
                let Some(p) = svg_pt(&mut cur.src, base) else { break };
                b.segs.push(Segment::LineTo(svg_canvas(&t, p)));
                cur.pen.last = p;
            }
            'H' => {
                let Some(x) = svg_num(&mut cur.src) else { break };
                let last = cur.pen.last;
                cur.pen.last = Point::new(if rel { last.x + x } else { x }, last.y);
                b.segs.push(Segment::LineTo(svg_canvas(&t, cur.pen.last)));
            }
            'V' => {
                let Some(y) = svg_num(&mut cur.src) else { break };
                let last = cur.pen.last;
                cur.pen.last = Point::new(last.x, if rel { last.y + y } else { y });
                b.segs.push(Segment::LineTo(svg_canvas(&t, cur.pen.last)));
            }
            'C' | 'S' => {
                if !svg_cubic(b, &mut cur, &t) {
                    break;
                }
            }
            'Q' | 'T' => {
                if !svg_quad(b, &mut cur, &t) {
                    break;
                }
            }
            'A' => {
                if !svg_arc_cmd(b, &mut cur, &t) {
                    break;
                }
            }
            _ => break,
        }
        if !matches!(cur.pen.cmd.to_ascii_uppercase(), 'C' | 'S') {
            cur.pen.prev_control = None;
        }
        if !matches!(cur.pen.cmd.to_ascii_uppercase(), 'Q' | 'T') {
            cur.pen.prev_quad = None;
        }
        if cur.src.i == before {
            break;
        }
        b.current = Some(Coord { p: svg_canvas(&t, cur.pen.last), node: None });
        b.last = Some(OnPath::Point(svg_canvas(&t, cur.pen.last)));
    }
}

/// The basic layer's `\pgfpathsvg{data}`: the segments of SVG path data (in pt, transformed as the
/// basic layer's points are), from `from` (the canvas point the path is at, if any).
pub fn svg_path(e: &Engine, text: &str, from: Option<Point>) -> Vec<Segment> {
    let mut b = Builder {
        segs: Vec::new(),
        current: from.map(|p| Coord { p, node: None }),
        subpath_start: from,
        last: None,
        node_items: Vec::new(),
        mid_nodes: Vec::new(),
        edges: Vec::new(),
        relative_base: None,
        origin: e.st.transform.apply(Point::default()),
        close_after: false,
        to_start: None,
        groups: Vec::new(),
        links: Vec::new(),
        from_node: None,
        fresh_link: false,
    };
    svg_data(e, &mut b, text);
    b.segs
}

/// An SVG elliptical arc from `a` to `p` (local pt), as curves on the canvas (SVG's appendix F.6.5).
pub(super) fn svg_arc(b: &mut Builder, arc: SvgArc, t: &crate::scene::Transform) {
    b.segs.extend(svg_arc_segments(arc, t));
}

/// The curves of an SVG elliptical arc from `a` to `p` (local pt) on the canvas; `sweep` turns it
/// counterclockwise. A radius of zero or no distance makes it a line.
pub fn svg_arc_segments(arc: SvgArc, t: &crate::scene::Transform) -> Vec<Segment> {
    let SvgArc { from: a, to: p, phi, large, sweep, .. } = arc;
    let (mut rx, mut ry) = (arc.rx, arc.ry);
    if rx < 1e-12 || ry < 1e-12 || a.minus(p).norm() < 1e-12 {
        return vec![Segment::LineTo(t.apply(p.scale(crate::PT)))];
    }
    let (s, c) = phi.to_radians().sin_cos();
    let d = a.minus(p).scale(0.5);
    let x1 = Point::new(c * d.x + s * d.y, -s * d.x + c * d.y);
    let lambda = (x1.x * x1.x) / (rx * rx) + (x1.y * x1.y) / (ry * ry);
    if lambda > 1.0 {
        (rx, ry) = (rx * lambda.sqrt(), ry * lambda.sqrt());
    }
    let num = (rx * rx * ry * ry - rx * rx * x1.y * x1.y - ry * ry * x1.x * x1.x).max(0.0);
    let den = rx * rx * x1.y * x1.y + ry * ry * x1.x * x1.x;
    let mut k = (num / den).sqrt();
    if large == sweep {
        k = -k;
    }
    let cp = Point::new(k * rx * x1.y / ry, -k * ry * x1.x / rx);
    let m = a.lerp(p, 0.5);
    let center = Point::new(c * cp.x - s * cp.y + m.x, s * cp.x + c * cp.y + m.y);
    let angle = |u: Point| u.y.atan2(u.x).to_degrees();
    let u = Point::new((x1.x - cp.x) / rx, (x1.y - cp.y) / ry);
    let v = Point::new((-x1.x - cp.x) / rx, (-x1.y - cp.y) / ry);
    let theta = angle(u);
    let mut delta = angle(v) - theta;
    if sweep && delta < 0.0 {
        delta += 360.0;
    } else if !sweep && delta > 0.0 {
        delta -= 360.0;
    }
    let local = arc_segments(EllipseArc { center: Point::default(), rx, ry, start: theta, end: theta + delta }, &crate::scene::Transform::default());
    let m = t
        .compose(&crate::scene::Transform::scaling(crate::PT, crate::PT))
        .compose(&crate::scene::Transform::translation(center.x, center.y))
        .compose(&crate::scene::Transform::rotation(phi));
    crate::canvas::transform_segments(&local, &m)
}
