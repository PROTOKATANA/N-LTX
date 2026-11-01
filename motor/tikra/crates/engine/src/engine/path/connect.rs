use crate::describe::Link;
use crate::scene::{Point, Segment};
use crate::tex::{self, Cat, Meaning, Token};
use crate::Error;

use super::super::coords::Coord;
use super::super::nodes::{NodeSpec, OnPath};
use super::super::state::Mode;
use super::super::{tikra_tokens, Engine};
use super::*;

pub(super) fn edge_path(e: &mut Engine, segs: Vec<Segment>, options: Vec<Token>, links: Vec<Link>) -> Result<(), Error> {
    e.begin_group();
    let outer = e.st.clone();
    e.st.mode = Mode::default();
    reset_path_settings(e);
    let r = (|| {
        e.process_keys(tikra_tokens("every edge/.try"), Some("/tikz"))?;
        e.process_keys(options, Some("/tikz"))?;
        e.note_links(links);
        e.finish_path(segs, &outer, "edge")
    })();
    e.end_group();
    r
}

pub(super) fn apply_operation(e: &mut Engine, b: &mut Builder, applied: AppliedOp) -> Result<(), Error> {
    let AppliedOp { op, to_options: to_opts, target: r } = applied;
    match op.as_deref() {
        Some("--") => {
            let start = b.segs.len();
            line_to(e, b, r);
            snake(e, b, start)?;
            place_mid_nodes(e, b, Some(0.5))?;
        }
        Some("-|") | Some("|-") => {
            let a = b.point();
            let corner = if op.as_deref() == Some("-|") { Point::new(r.p.x, a.y) } else { Point::new(a.x, r.p.y) };
            let first = b.segs.len();
            let touched = e.facts.touched.take();
            line_to(e, b, Coord { p: corner, node: None });
            e.facts.touched = touched;
            // the legs from and to the nodes' borders
            let start = match b.last {
                Some(OnPath::Segment(p, _, _)) => p,
                _ => a,
            };
            line_to(e, b, r.clone());
            let end = match b.last {
                Some(OnPath::Segment(_, _, p)) => p,
                _ => b.point(),
            };
            // pos runs along both legs: 0.5, the default, is the corner
            b.last = Some(OnPath::Corner(start, corner, end));
            snake(e, b, first)?;
            place_mid_nodes(e, b, Some(0.5))?;
        }
        Some("to") => to(e, b, to_opts, r)?,
        Some("rectangle") => {
            let start = b.segs.len();
            rectangle_to(e, b, r);
            snake(e, b, start)?;
            place_mid_nodes(e, b, Some(0.5))?;
        }
        _ => {
            b.move_to(r);
            if b.from_node.is_none() {
                b.from_node = e.facts.touched.take();
            }
            place_mid_nodes(e, b, Some(0.5))?;
        }
    }
    Ok(())
}

/// `\tikz@to@compute@distance@main`: the out and in distances, from the looseness and clamped by
/// `min distance` and `max distance`.
pub(super) fn to_distances(e: &Engine, a: Point, b: Point) -> (f64, f64) {
    let t = &e.st.to;
    let d = 0.3915 * b.minus(a).norm();
    let clamp = |x: f64, lo: f64, hi: f64| {
        let x = if x < lo { lo } else { x };
        if x > hi {
            hi
        } else {
            x
        }
    };
    (clamp(t.out_looseness * d, t.out_min, t.out_max), clamp(t.in_looseness * d, t.in_min, t.in_max))
}

/// A curve of `to` as the topaths library computes it, from `from` to `r`: an explicit start (the
/// relative curves start on the border of the start node), the controls and an explicit end.
pub(super) fn to_curve(e: &mut Engine, from: &Coord, r: &Coord) -> Result<(Option<Point>, Point, Point, Option<Point>), Error> {
    let node = |e: &Engine, c: &Coord| c.node.as_ref().and_then(|n| e.nodes.get(n)).cloned();
    let (na, nb) = (node(e, from), node(e, r));
    let (out, inn) = (e.st.to.out, e.st.to.inn);
    if !e.st.to.relative {
        // the ends are the nodes' borders at the angles out and in
        let a = na.as_ref().and_then(|n| n.anchor(&out.to_string())).unwrap_or(from.p);
        let b = nb.as_ref().and_then(|n| n.anchor(&inn.to_string())).unwrap_or(r.p);
        // the angles and the distance are in the picture's coordinate system: under `xscale` or a
        // slant, the controls go where the transformation takes them
        let t = e.st.transform;
        let inverse = t.inverse();
        let (ko, ki) = to_distances(e, inverse.apply(a), inverse.apply(b));
        // `(p)`, or `+(d)` from the end it belongs to
        let control = |e: &mut Engine, c: Option<Vec<Token>>, base: Point| -> Result<Option<Point>, Error> {
            let Some(v) = c else { return Ok(None) };
            let mut v = crate::keys::trim(&v).to_vec();
            let plus = v.iter().take_while(|t| t.is_char('+')).count();
            v.drain(..plus);
            let v = if v.first().is_some_and(|t| t.is_char('(')) && v.last().is_some_and(|t| t.is_char(')')) { v[1..v.len() - 1].to_vec() } else { v };
            let text = e.math_string(v)?;
            let p = e.coordinate(&text)?.p;
            Ok(Some(if plus > 0 { base.plus(p.minus(e.st.transform.apply(Point::default()))) } else { p }))
        };
        let c1 = control(e, e.st.to.out_control.clone(), from.p)?.unwrap_or_else(|| a.plus(t.linear(polar(out, ko))));
        let c2 = control(e, e.st.to.in_control.clone(), r.p)?.unwrap_or_else(|| b.plus(t.linear(polar(inn, ki))));
        return Ok((None, c1, c2, None));
    }
    // relative: the angles are measured from the line between the ends
    let (a0, b0) = (na.as_ref().map_or(from.p, |n| n.center()), nb.as_ref().map_or(r.p, |n| n.center()));
    let rotate = |p: Point, about: Point, angle: f64| {
        let (s, c) = angle.to_radians().sin_cos();
        let d = p.minus(about);
        about.plus(Point::new(c * d.x - s * d.y, s * d.x + c * d.y))
    };
    let first = na.as_ref().map_or(a0, |n| n.border_toward(rotate(b0, a0, out)));
    let second = nb.as_ref().map_or(b0, |n| n.border_toward(rotate(a0, b0, 180.0 + inn)));
    // the controls in the picture's coordinate system, as for absolute angles
    let t = e.st.transform;
    let (fl, sl) = (t.inverse().apply(first), t.inverse().apply(second));
    let (ko, ki) = to_distances(e, fl, sl);
    let d = sl.minus(fl);
    let u = if d.norm() > 1e-12 { d.scale(1.0 / d.norm()) } else { Point::new(1.0, 0.0) };
    let turn = |v: Point| Point::new(u.x * v.x - u.y * v.y, u.y * v.x + u.x * v.y);
    let c1 = t.apply(fl.plus(turn(polar(out, ko))));
    let c2 = t.apply(sl.plus(turn(polar(inn, ki))));
    Ok((na.map(|_| first), c1, c2, Some(second)))
}

/// The end of a `to` as `\tikztostart` and `\tikztotarget` name it: a node, or a point on the canvas.
pub(super) fn to_name(c: &Coord) -> String {
    match &c.node {
        Some(n) => n.clone(),
        None => format!("tikra cs:{}pt,{}pt", c.p.x / crate::PT, c.p.y / crate::PT),
    }
}

/// `\tikztonodes`: the nodes given between `to` and its target, labels at the middle by default.
pub(super) fn to_nodes(mids: Vec<MidNode>) -> Vec<Token> {
    let mut v = Vec::new();
    for m in mids {
        v.extend(tikra_tokens(if m.coordinate { "coordinate[pos=0.5," } else { "node[pos=0.5," }));
        v.extend(m.options);
        v.extend(tikra_tokens("]"));
        if let Some(n) = m.name {
            v.extend(tikra_tokens(&format!("({n})")));
        }
        if !m.coordinate {
            v.push(Token::Char('{', Cat::Begin));
            v.extend(m.text);
            v.push(Token::Char('}', Cat::End));
        }
    }
    v
}

/// The path of a `to` (or an `edge`) with its options already set, from the current point to `r`.
pub(super) fn to_path(e: &mut Engine, b: &mut Builder, r: Coord) -> Result<(), Error> {
    let from = b.current.clone().unwrap_or(Coord { p: b.point(), node: None });
    let r = if e.st.to.is_loop {
        // [looseness=8,min distance=5mm] after the options, and back to the start
        let t = &mut e.st.to;
        (t.out_looseness, t.in_looseness) = (8.0, 8.0);
        (t.out_min, t.in_min) = (0.5, 0.5);
        (t.out_control, t.in_control) = (None, None);
        from.clone()
    } else {
        r
    };
    if let Some(path) = e.st.to.path.clone() {
        // the user's to path, run as path operations with the three macros set
        e.tex.define_string("tikztostart", &to_name(&from), false);
        e.tex.define_string("tikztotarget", &to_name(&r), false);
        e.tex.set_meaning(&Token::cs("tikztonodes"), super::super::macro_tokens(to_nodes(std::mem::take(&mut b.mid_nodes))), false);
        let mut v = path;
        v.push(Token::other(';'));
        e.tex.push_tokens(v);
        return operations(e, b, None);
    }
    if !e.st.to.curve {
        let start = b.segs.len();
        line_to(e, b, r);
        return snake(e, b, start);
    }
    e.process_keys(tikra_tokens("every curve to/.try"), Some("/tikz"))?;
    let (start, c1, c2, end) = to_curve(e, &from, &r)?;
    if let Some(s) = start {
        match b.segs.last_mut() {
            Some(Segment::MoveTo(m)) => *m = s,
            _ => b.segs.push(Segment::MoveTo(s)),
        }
        b.current = Some(Coord { p: s, node: None });
        b.subpath_start.get_or_insert(s);
    }
    match end {
        Some(p) => {
            let a = b.point();
            if b.current.is_none() {
                b.segs.push(Segment::MoveTo(a));
            }
            b.segs.push(Segment::CurveTo(c1, c2, p));
            b.last = Some(OnPath::Segment(a, Some((c1, c2)), p));
            note_link(e, b, &r);
            b.current = Some(Coord { p, node: r.node.clone() });
        }
        None => curve_to(e, b, CurveEnd { c1, c2, target: r }),
    }
    Ok(())
}

/// `execute at begin to` and `execute at end to`: the code they collect, run in the `to`'s group.
pub(super) fn run_hook(e: &mut Engine, name: &str) -> Result<(), Error> {
    match e.tex.meaning_of(name) {
        Meaning::Macro(m) if !m.body.is_empty() => {
            let code = tex::substitute(&m.body, &[]);
            e.run_tokens(code)
        }
        _ => Ok(()),
    }
}

pub(super) fn to(e: &mut Engine, b: &mut Builder, opts: Vec<Token>, r: Coord) -> Result<(), Error> {
    if let Some(s) = b.to_start.take() {
        b.move_to(s);
    }
    // what the target set for a `+(…)` after it: a user's to path, whatever its points, does not change it
    let relative_base = b.relative_base;
    e.begin_group();
    e.st.quotes_on_path = true;
    e.st.quoted.clear();
    let res = (|| {
        run_hook(e, "tikz@atbegin@to")?;
        e.process_keys(tikra_tokens("every to/.try"), Some("/tikz"))?;
        e.process_keys(opts, Some("/tikz"))?;
        for (text, options) in std::mem::take(&mut e.st.quoted) {
            b.mid_nodes.push(MidNode { options, name: None, text, coordinate: false, pic: false });
        }
        to_path(e, b, r)?;
        place_mid_nodes(e, b, Some(0.5))?;
        run_hook(e, "tikz@atend@to")
    })();
    e.end_group();
    b.relative_base = relative_base;
    res
}

/// `edge[options] node{…} (target)`: a separate path, drawn after this one; the current point stays.
pub(super) fn edge(e: &mut Engine, b: &mut Builder, edge: EdgeTo) -> Result<(), Error> {
    let EdgeTo { options: opts, target: r, mids } = edge;
    let saved_current = b.current.clone();
    let mut sub = Builder {
        segs: Vec::new(),
        current: None,
        subpath_start: None,
        last: None,
        node_items: Vec::new(),
        mid_nodes: mids,
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
    if let Some(c) = b.to_start.clone().or_else(|| saved_current.clone()) {
        sub.move_to(c);
    }
    e.begin_group();
    e.st.quotes_on_path = true;
    e.st.quoted.clear();
    let mut opts = opts;
    let res = (|| {
        run_hook(e, "tikz@atbegin@to")?;
        e.process_keys(tikra_tokens("every edge/.try"), Some("/tikz"))?;
        e.process_keys(opts.clone(), Some("/tikz"))?;
        for (text, options) in std::mem::take(&mut e.st.quoted) {
            sub.mid_nodes.push(MidNode { options, name: None, text, coordinate: false, pic: false });
        }
        // options that start the to path are the edge path's own, as TikZ runs it in `\path[…]`
        if let Some(leading) = e.st.to.path.as_deref().and_then(leading_options) {
            opts.push(Token::other(','));
            opts.extend(leading);
        }
        to_path(e, &mut sub, r.clone())?;
        place_mid_nodes(e, &mut sub, Some(0.5))
    })();
    e.end_group();
    res?;
    b.edges.push((sub.segs, opts, sub.node_items, sub.links));
    b.current = saved_current;
    Ok(())
}

/// `[options]` at the start of a path, without the brackets.
pub(super) fn leading_options(path: &[Token]) -> Option<Vec<Token>> {
    let path = crate::keys::trim(path);
    if !path.first()?.is_char('[') {
        return None;
    }
    let mut depth = 0usize;
    for (i, t) in path.iter().enumerate().skip(1) {
        match t {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
            t if depth == 0 && t.is_char(']') => return Some(path[1..i].to_vec()),
            _ => {}
        }
    }
    None
}

/// Nodes between `edge` and its target.
pub(super) fn read_mid_nodes(e: &mut Engine, depth: Option<usize>, base: Point) -> Result<Vec<MidNode>, Error> {
    let mut v = Vec::new();
    while let Some(Token::Char('n' | 'c', Cat::Letter)) = peek(e, depth)? {
        let Some(Token::Char(c, _)) = next(e, depth)? else { break };
        let w = word(e, c);
        if w != "node" && w != "coordinate" {
            e.tex.push_tokens(word_tokens(&w));
            break;
        }
        let coordinate = w == "coordinate";
        let (options, name, _, text) = read_node(e, depth, coordinate, base)?;
        v.push(MidNode { options, name, text, coordinate, pic: false });
    }
    Ok(v)
}

pub(super) fn place_mid_nodes(e: &mut Engine, b: &mut Builder, default_pos: Option<f64>) -> Result<(), Error> {
    // the nodes on a line just made from node to node are its labels
    let link = std::mem::take(&mut b.fresh_link).then(|| b.links.len() - 1);
    let first = e.facts.nodes.len();
    for m in std::mem::take(&mut b.mid_nodes) {
        let place = b.last.unwrap_or(OnPath::Point(b.point()));
        if m.pic {
            e.pic(super::super::pics::PicSpec { options: m.options, name: m.name, at: b.point(), content: m.text, place: Some((place, default_pos)) })?;
            continue;
        }
        let items = e.node(NodeSpec { options: m.options, name: m.name, at: None, text: m.text, place, point: b.point(), coordinate: m.coordinate, default_pos })?;
        b.node_items.extend(items);
    }
    if let Some(l) = link {
        for i in first..e.facts.nodes.len() {
            e.facts.nodes[i].on_link = true;
            b.links[l].labels.push(i);
        }
    }
    Ok(())
}
