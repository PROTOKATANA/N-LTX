use crate::keys;
use crate::scene::{Point, Segment};
use crate::tex::{self, Cat, Token};
use crate::Error;

use super::super::coords::{read_coordinate_tokens, Coord};
use super::super::nodes::OnPath;
use super::super::Engine;
use super::*;

/// A rectangle from the current point to the corner `r`.
pub(super) fn rectangle_to(e: &mut Engine, b: &mut Builder, r: Coord) {
    b.from_node = None;
    let a = b.point();
    let t = e.st.transform;
    let inv = t.inverse();
    let (la, lb) = (inv.apply(a), inv.apply(r.p));
    b.segs.push(Segment::MoveTo(a));
    // in the order of `\tikz@rectB`: first to (a.x, b.y)
    for k in [Point::new(la.x, lb.y), lb, Point::new(lb.x, la.y)] {
        b.segs.push(Segment::LineTo(t.apply(k)));
    }
    b.segs.push(Segment::Close);
    b.subpath_start = Some(a);
    b.last = Some(OnPath::Segment(a, None, r.p));
    b.current = Some(Coord { p: r.p, node: None });
}

/// A line to `r`, stopping at the border of the nodes on both ends.
pub(super) fn line_to(e: &mut Engine, b: &mut Builder, r: Coord) {
    let Some(a) = b.current.clone() else {
        b.move_to(r);
        return;
    };
    let mut pa = a.p;
    let mut pb = r.p;
    if let Some(n) = a.node.as_ref().and_then(|n| e.nodes.get(n)) {
        let target = r.node.as_ref().and_then(|m| e.nodes.get(m)).map_or(r.p, |m| m.center());
        pa = n.border_toward(target);
        // the move to a node waits for the border it leaves from: after a line that ended on
        // the node, a new subpath starts there (a coordinate is a point, its path goes on)
        if n.shape == "coordinate" {
            if let Some(Segment::MoveTo(m)) = b.segs.last_mut() {
                *m = pa;
            }
        } else {
            match b.segs.last_mut() {
                Some(Segment::MoveTo(m)) => *m = pa,
                _ => b.segs.push(Segment::MoveTo(pa)),
            }
        }
    }
    if let Some(n) = r.node.as_ref().and_then(|n| e.nodes.get(n)) {
        let from = a.node.as_ref().and_then(|m| e.nodes.get(m)).map_or(a.p, |m| m.center());
        pb = n.border_toward(from);
    }
    if !matches!(b.segs.last(), Some(Segment::MoveTo(_)) | Some(Segment::LineTo(_)) | Some(Segment::CurveTo(..))) {
        b.segs.push(Segment::MoveTo(pa));
    }
    b.segs.push(Segment::LineTo(pb));
    b.last = Some(OnPath::Segment(pa, None, pb));
    note_link(e, b, &r);
    b.current = Some(r);
}

/// `snake=…`: each straight line the path got from `start` on (a closing one too) becomes that
/// decoration of it, as the snakes library does with each line.
pub(super) fn snake(e: &mut Engine, b: &mut Builder, start: usize) -> Result<(), Error> {
    let Some(name) = e.st.snake.clone() else { return Ok(()) };
    let added = b.segs.split_off(start.min(b.segs.len()));
    let end = |s: &Segment| match s {
        Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => Some(*p),
        Segment::Close => None,
    };
    let mut first = b.segs.iter().rev().find_map(|s| if let Segment::MoveTo(p) = s { Some(*p) } else { None });
    let mut at = b.segs.last().and_then(end).or(first);
    e.begin_group();
    e.st.snake = None;
    e.st.decoration.name = Some(name);
    let r = (|| {
        for s in added {
            let target = match s {
                Segment::LineTo(p) => p,
                Segment::Close => first.unwrap_or_default(),
                other => {
                    if let Segment::MoveTo(p) = other {
                        first = Some(p);
                    }
                    at = end(&other).or(at);
                    b.segs.push(other);
                    continue;
                }
            };
            let from = at.unwrap_or(target);
            let decorated = e.decorate_segments(&[Segment::MoveTo(from), Segment::LineTo(target)], None)?;
            // the decoration goes on from where the path is, in the same subpath
            b.segs.extend(decorated.into_iter().skip_while(|d| matches!(d, Segment::MoveTo(p) if p.minus(from).norm() < 1e-9)));
            if matches!(s, Segment::Close) {
                b.segs.push(Segment::Close);
            }
            at = Some(target);
        }
        Ok(())
    })();
    e.end_group();
    r
}

pub(super) fn curve_to(e: &mut Engine, b: &mut Builder, end: CurveEnd) {
    let CurveEnd { c1, c2, target: r } = end;
    let a = b.current.clone().map_or(r.p, |a| a.p);
    if b.current.is_none() {
        b.segs.push(Segment::MoveTo(a));
    }
    if let Some(n) = b.current.as_ref().and_then(|c| c.node.as_ref()).and_then(|n| e.nodes.get(n)) {
        // from the node's border toward the first control: a new piece if the path came to the
        // node (`(a) to (b) .. controls … .. (b)`)
        let pa = n.border_toward(c1);
        match b.segs.last_mut() {
            Some(Segment::MoveTo(m)) => *m = pa,
            _ => b.segs.push(Segment::MoveTo(pa)),
        }
    }
    let mut pb = r.p;
    if let Some(n) = r.node.as_ref().and_then(|n| e.nodes.get(n)) {
        pb = n.border_toward(c2);
    }
    let start = match b.segs.last() {
        Some(Segment::MoveTo(q) | Segment::LineTo(q) | Segment::CurveTo(_, _, q)) => *q,
        _ => a,
    };
    b.segs.push(Segment::CurveTo(c1, c2, pb));
    b.last = Some(OnPath::Segment(start, Some((c1, c2)), pb));
    note_link(e, b, &r);
    b.current = Some(r);
}

pub(super) fn polar(angle: f64, d: f64) -> Point {
    let (s, c) = angle.to_radians().sin_cos();
    Point::new(d * c, d * s)
}

/// `circle (r)`, `circle (rx and ry)`, `circle[radius=…]`, `ellipse[x radius=…, y radius=…]`.
pub(super) fn circle(e: &mut Engine, b: &mut Builder, depth: Option<usize>) -> Result<(), Error> {
    e.begin_group();
    let res = (|| -> Result<(f64, f64, Option<Point>, crate::scene::Transform), Error> {
        // `circle[options]`, or the old `circle (radius)`: a point after the options is the next one
        let mut options = false;
        e.st.node.at = None;
        let before = e.st.pending;
        if let Some(t) = peek(e, depth)? {
            if t.is_char('[') {
                let _ = next(e, depth)?;
                let o = e.tex.read_until_char(']', false)?;
                e.process_keys(o, Some("/tikz"))?;
                options = true;
            }
        }
        let (mut rx, mut ry) = radii(e)?;
        if let Some(t) = peek(e, depth)?.filter(|_| !options) {
            if t.is_char('(') {
                let _ = next(e, depth)?;
                let tokens = read_coordinate_tokens(e)?;
                let text = e.math_string(tokens)?;
                if let Some((a, bb)) = text.split_once(" and ") {
                    rx = e.radius_length(a)?;
                    ry = e.radius_length(bb)?;
                } else {
                    rx = e.radius_length(&text)?;
                    ry = rx;
                }
            }
        }
        Ok((rx, ry, e.st.node.at.take(), before.inverse().compose(&e.st.pending)))
    })();
    e.end_group();
    let (rx, ry, at, turn) = res?;
    // `at=(c)` centers it elsewhere; the options' transformations (`rotate=30`) turn it about its
    // center
    let t = e.st.transform.compose(&turn);
    let center = at.unwrap_or_else(|| b.point());
    let mut v = arc_segments(EllipseArc { center, rx, ry, start: 0.0, end: 360.0 }, &t);
    v.insert(0, Segment::MoveTo(center.plus(t.linear(Point::new(rx, 0.0)))));
    v.push(Segment::Close);
    b.segs.extend(v);
    b.subpath_start = Some(center);
    Ok(())
}

/// `radius`, `x radius`, `y radius` from the keys.
pub(super) fn radii(e: &mut Engine) -> Result<(f64, f64), Error> {
    let get = |e: &mut Engine, k: &str| -> Result<Option<f64>, Error> {
        match keys::value(&e.tex, k) {
            Some(v) if !keys::trim(&v).is_empty() => {
                let s = e.math_string(v)?;
                Ok(Some(e.radius_length(&s)?))
            }
            _ => Ok(None),
        }
    };
    // (`radius` sets both)
    let rx = get(e, "/tikz/x radius")?.unwrap_or(0.0);
    let ry = get(e, "/tikz/y radius")?.unwrap_or(0.0);
    Ok((rx, ry))
}

/// An elliptical arc from `a0` to `a1` degrees as cubic curves, cut the way `\pgfpatharc` cuts it:
/// 90° pieces while more than 115° are left, 60° pieces while more than 90° are, then the rest.
pub fn arc_segments(arc: EllipseArc, t: &crate::scene::Transform) -> Vec<Segment> {
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

/// `arc (start:end:radius)`, `arc (start:end:rx and ry)`, `arc[start angle=…, end angle=…, radius=…]`.
pub(super) fn arc(e: &mut Engine, b: &mut Builder, depth: Option<usize>) -> Result<(), Error> {
    e.begin_group();
    let res = (|| -> Result<(f64, f64, f64, f64), Error> {
        let (mut a0, mut a1, mut rx, mut ry) = (0.0, 90.0, 0.0, 0.0);
        match peek(e, depth)? {
            Some(t) if t.is_char('(') => {
                let _ = next(e, depth)?;
                let tokens = read_coordinate_tokens(e)?;
                let text = e.math_string(tokens)?;
                let p: Vec<&str> = text.split(':').collect();
                if p.len() == 3 {
                    a0 = e.number(p[0])?;
                    a1 = e.number(p[1])?;
                    if let Some((x, y)) = p[2].split_once(" and ") {
                        rx = e.radius_length(x)?;
                        ry = e.radius_length(y)?;
                    } else {
                        rx = e.radius_length(p[2])?;
                        ry = rx;
                    }
                }
            }
            Some(t) if t.is_char('[') => {
                let _ = next(e, depth)?;
                let o = e.tex.read_until_char(']', false)?;
                e.process_keys(o, Some("/tikz"))?;
                let (x, y) = radii(e)?;
                rx = x;
                ry = y;
                let get = |e: &mut Engine, k: &str| -> Result<Option<f64>, Error> {
                    match keys::value(&e.tex, k) {
                        Some(v) if !keys::trim(&v).is_empty() => Ok(Some(e.number_of(v)?)),
                        _ => Ok(None),
                    }
                };
                let s = get(e, "/tikz/start angle")?;
                let en = get(e, "/tikz/end angle")?;
                let d = get(e, "/tikz/delta angle")?;
                match (s, en, d) {
                    (Some(s), Some(en), _) => {
                        a0 = s;
                        a1 = en;
                    }
                    (Some(s), None, Some(d)) => {
                        a0 = s;
                        a1 = s + d;
                    }
                    (None, Some(en), Some(d)) => {
                        a0 = en - d;
                        a1 = en;
                    }
                    (Some(s), None, None) => a0 = s,
                    _ => {}
                }
            }
            _ => {
                let (x, y) = radii(e)?;
                rx = x;
                ry = y;
            }
        }
        Ok((a0, a1, rx, ry))
    })();
    e.end_group();
    let (a0, a1, rx, ry) = res?;
    let t = e.st.transform;
    let p = b.point();
    if b.current.is_none() {
        b.segs.push(Segment::MoveTo(p));
    }
    let (s0, c0) = a0.to_radians().sin_cos();
    let center = p.minus(t.linear(Point::new(rx * c0, ry * s0)));
    let segs = arc_segments(EllipseArc { center, rx, ry, start: a0, end: a1 }, &t);
    let (s1, c1) = a1.to_radians().sin_cos();
    let end = center.plus(t.linear(Point::new(rx * c1, ry * s1)));
    if !segs.is_empty() {
        b.last = Some(OnPath::Arc { center, x: t.linear(Point::new(rx, 0.0)), y: t.linear(Point::new(0.0, ry)), from: a0, to: a1 });
    }
    b.segs.extend(segs);
    b.current = Some(Coord { p: end, node: None });
    // a `+(…)` after it is from the arc's end
    b.relative_base = None;
    Ok(())
}

/// Multiples of `step` between `start` and `end`. None if the step is unusable.
pub(super) fn ticks(start: f64, end: f64, step: f64) -> Option<Vec<f64>> {
    // a step of zero: no lines that way (`ystep=0`)
    if step == 0.0 {
        return Some(Vec::new());
    }
    if !(step > 0.0 && step.is_finite() && start.is_finite() && end.is_finite()) {
        return None;
    }
    let first = (start / step - 1e-9).ceil();
    let count = ((end + 1e-9) / step).floor() - first + 1.0;
    if count > MAX_GRID_LINES as f64 {
        return None;
    }
    Some((0..count.max(0.0) as usize).map(|k| (first + k as f64) * step).collect())
}

pub(super) fn grid(e: &mut Engine, b: &mut Builder, depth: Option<usize>) -> Result<(), Error> {
    e.begin_group();
    let res = (|| -> Result<(f64, f64), Error> {
        if let Some(t) = peek(e, depth)? {
            if t.is_char('[') {
                let _ = next(e, depth)?;
                let o = e.tex.read_until_char(']', false)?;
                e.process_keys(o, Some("/tikz"))?;
            }
        }
        Ok(e.st.plot.grid_step)
    })();
    e.end_group();
    let step = res?;
    if step == (0.0, 0.0) {
        e.warn("grid: step 0 × 0 draws no lines");
    }
    let r = target(e, b, depth, "grid")?;
    let t = e.st.transform;
    let inv = t.inverse();
    let (la, lb) = (inv.apply(b.point()), inv.apply(r.p));
    let (x0, x1) = (la.x.min(lb.x), la.x.max(lb.x));
    let (y0, y1) = (la.y.min(lb.y), la.y.max(lb.y));
    match (ticks(x0, x1, step.0), ticks(y0, y1, step.1)) {
        (Some(xs), Some(ys)) => {
            for x in xs {
                b.segs.push(Segment::MoveTo(t.apply(Point::new(x, y0))));
                b.segs.push(Segment::LineTo(t.apply(Point::new(x, y1))));
            }
            for y in ys {
                b.segs.push(Segment::MoveTo(t.apply(Point::new(x0, y))));
                b.segs.push(Segment::LineTo(t.apply(Point::new(x1, y))));
            }
        }
        _ => e.warn(format!("grid: step {} × {} is invalid or too fine (more than {MAX_GRID_LINES} lines): grid ignored", step.0, step.1)),
    }
    b.last = Some(OnPath::Point(r.p));
    b.current = Some(r);
    Ok(())
}

/// `parabola (b)`, `parabola bend (c) (b)`, `parabola[bend at end] (b)`.
pub(super) fn parabola(e: &mut Engine, b: &mut Builder, depth: Option<usize>) -> Result<(), Error> {
    // `bend pos=f`: a relative bend is from the point that far from the start to the end (the
    // start by default); `parabola height=h`: the bend is h above the middle; `bend at end`
    let (mut factor, mut height, mut bend_at_end) = (0.0, None, false);
    if let Some(t) = peek(e, depth)? {
        if t.is_char('[') {
            let _ = next(e, depth)?;
            let o = e.tex.read_until_char(']', false)?;
            for item in crate::keys::split_list(&o) {
                let value = item.value.map(|v| tex::tokens_to_string(&v)).unwrap_or_default();
                match item.key.as_str() {
                    "bend at end" => bend_at_end = true,
                    "bend at start" => factor = 0.0,
                    "bend pos" => factor = e.number(&value)?,
                    "parabola height" => {
                        factor = 0.5;
                        height = Some(e.length(&value)?);
                    }
                    _ => {}
                }
            }
        }
    }
    let mut bend = None;
    if let Some(Token::Char('b', Cat::Letter)) = peek(e, depth)? {
        let _ = next(e, depth)?;
        let w = word(e, 'b');
        if w == "bend" {
            let relative = matches!(peek(e, depth)?, Some(t) if t.is_char('+'));
            bend = Some((target(e, b, depth, "parabola bend")?.p, relative));
        }
    }
    let r = target(e, b, depth, "parabola")?;
    let a = b.point();
    if b.current.is_none() {
        b.segs.push(Segment::MoveTo(a));
    }
    let saved = a.lerp(r.p, factor);
    let bend = match bend {
        Some((p, true)) => Some(p.minus(a).plus(saved)),
        Some((p, false)) => Some(p),
        // the height is a y vector of the picture: `scale` and `yscale` stretch it too
        None => height.map(|h| saved.plus(e.st.transform.linear(Point::new(0.0, h)))),
    };
    // \pgfpathparabola: to the bend with its vertex there, then on to the end with its vertex at the bend
    let (bend_vector, end_vector) = match bend {
        Some(v) => (v.minus(a), r.p.minus(v)),
        None if bend_at_end => (r.p.minus(a), Point::default()),
        None => (Point::default(), r.p.minus(a)),
    };
    let mut from = a;
    if bend_vector.norm() > 1e-12 {
        let c = bend_vector;
        b.segs.push(Segment::CurveTo(from.plus(Point::new(0.1125 * c.x, 0.225 * c.y)), from.plus(Point::new(0.5 * c.x, c.y)), from.plus(c)));
        from = from.plus(c);
    }
    if end_vector.norm() > 1e-12 {
        let c = end_vector;
        b.segs.push(Segment::CurveTo(from.plus(Point::new(0.5 * c.x, 0.0)), from.plus(Point::new(0.8875 * c.x, 0.775 * c.y)), from.plus(c)));
    }
    b.current = Some(r);
    Ok(())
}
