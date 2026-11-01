use tikra_engine::scene::{Point, Transform};
use tikra_engine::tex::{self, Token};
use tikra_engine::{Error, PT};

use super::super::keys;
use super::super::{toks, Engine};
use tikra_engine::engine::nodes::{NodeSpec, OnPath};

use super::{define, unit, Marks, NearNode, NearQuery, PlacedNodes};

/// The nodes after a plot (`\addplot … node[pos=0.3] {…};`): at the last point, or with `pos` at
/// that fraction of the plot (`\pgfplotspointplotattime`), on the segment there.
pub(super) fn post_nodes(e: &mut Engine, placed: &PlacedNodes<'_>) -> Result<(), Error> {
    let PlacedNodes { plot: p, pts, canvas: cp, frame: f, rest } = *placed;
    let (prefix, items) = split_nodes(rest);
    let mut at_end: Vec<Token> = prefix.clone();
    let size = |e: &mut Engine, a: usize| keys::length(e, if a == 0 { "/pgfplots/width" } else { "/pgfplots/height" }).unwrap_or(if a == 0 { 240.0 * PT } else { 207.0 * PT }) / PT;
    let scale = [0, 1, 2].map(|a| {
        let [lo, hi] = f.data_range[a];
        if lo.is_finite() && hi.is_finite() {
            super::super::ticks::data_scale(lo, hi, size(e, a))
        } else {
            1.0
        }
    });
    let mut points: Vec<Option<[f64; 3]>> = pts.iter().map(|l| l.iter().take(if f.three_d { 3 } else { 2 }).all(|v| v.is_finite()).then_some(*l)).collect();
    // `pos segment=k`: positions along the k-th piece of the plot (between empty lines) only
    let prefix_text = tex::tokens_to_string(&prefix);
    if let Some(k) = prefix_text
        .split_once("pos segment")
        .and_then(|(_, r)| r.trim_start().strip_prefix('='))
        .and_then(|r| r.trim_start().split(|c: char| !c.is_ascii_digit()).next()?.parse::<usize>().ok())
    {
        let mut bounds = vec![0];
        bounds.extend(p.scanlines.iter().copied());
        bounds.push(points.len());
        if let (Some(&lo), Some(&hi)) = (bounds.get(k), bounds.get(k + 1)) {
            for (i, q) in points.iter_mut().enumerate() {
                if i < lo || i >= hi {
                    *q = None;
                }
            }
        }
    }
    let tl = super::super::attime::Timeline { points, scale, log: f.log, time: 1.0 };
    let mut placed = Vec::new();
    // the path goes on at the last point when no node has a `pos` (options, `--` and others too)
    let mut ends = items.is_empty();
    for item in items {
        match node_time(e, &item).and_then(|t| tl.at(t).map(|r| (t, r))) {
            Some((t, (_, a, b, frac))) => placed.push((item, t, a, b, frac)),
            None => {
                at_end.extend(item);
                ends = true;
            }
        }
    }
    super::super::attime::set(e, Some(tl));
    let r = (|| -> Result<(), Error> {
        let z = |l: [f64; 3]| if f.three_d { l } else { [l[0], l[1], f.min[2]] };
        // a scatter plot has no segments: a node goes to the point nearest its `pos`
        let snapped: Vec<[f64; 3]> =
            if e.st.plot.only_marks { pts.iter().copied().filter(|l| l.iter().take(if f.three_d { 3 } else { 2 }).all(|v| v.is_finite())).collect() } else { Vec::new() };
        for (item, t, a, b, frac) in placed {
            super::super::attime::set_time(e, t);
            let (a, b, frac) = match snapped.len() {
                0 => (a, b, frac),
                n => {
                    let k = ((t * (n - 1) as f64).round().max(0.0) as usize).min(n - 1);
                    (snapped[k], snapped[k], 0.0)
                }
            };
            let (pa, pb) = (f.at(z(a)), f.at(z(b)));
            let mut code = toks("\\path[draw=none,fill=none]");
            code.extend(prefix.clone());
            code.extend(toks(&format!(" (tikra cs:{}pt,{}pt) -- (tikra cs:{}pt,{}pt) ", pa.x / PT, pa.y / PT, pb.x / PT, pb.y / PT)));
            code.extend(with_options(&with_pos(&item, frac), &prefix));
            code.push(Token::other(';'));
            e.run_tokens(code)?;
        }
        if ends {
            super::super::attime::set_time(e, 1.0);
            // a box plot without outliers starts its path at the origin
            let origin = (p.boxplot.is_some()).then(|| f.at(f.min));
            if let Some(last) = cp.iter().rev().flatten().next().copied().or(origin).as_ref() {
                let mut code = toks(&format!("\\path[draw=none,fill=none] (tikra cs:{}pt,{}pt) ", last.x / PT, last.y / PT));
                code.extend(at_end);
                code.push(Token::other(';'));
                e.run_tokens(code)?;
            }
        }
        Ok(())
    })();
    super::super::attime::set(e, None);
    r
}

/// A node with the options given before it on the path (`[yshift=8pt] node[pos=0] {…}`): they
/// move the node too.
pub(super) fn with_options(item: &[Token], prefix: &[Token]) -> Vec<Token> {
    let opts: Vec<Token> = {
        let t = tikra_engine::keys::trim(prefix);
        match (t.first(), t.last()) {
            (Some(a), Some(b)) if a.is_char('[') && b.is_char(']') && t.len() > 2 => t[1..t.len() - 1].to_vec(),
            _ => return item.to_vec(),
        }
    };
    let Some(open) = item.iter().position(|t| t.is_char('[')) else {
        let mut out = item.to_vec();
        let at = out.iter().position(|t| matches!(t, Token::Char(c, _) if !c.is_alphabetic() && !c.is_whitespace())).unwrap_or(out.len());
        let mut group = vec![Token::other('[')];
        group.extend(opts);
        group.push(Token::other(']'));
        out.splice(at..at, group);
        return out;
    };
    let mut out = item[..=open].to_vec();
    out.extend(opts);
    out.push(Token::other(','));
    out.extend(item[open + 1..].iter().cloned());
    out
}

/// The path's options before its first node, and each `node …` or `coordinate …` (with what
/// follows it up to the next one).
pub(super) fn split_nodes(rest: &[Token]) -> (Vec<Token>, Vec<Vec<Token>>) {
    let word = |i: usize| -> Option<usize> {
        for w in ["coordinate", "node"] {
            let n = w.len();
            if i + n <= rest.len()
                && rest[i..i + n].iter().zip(w.chars()).all(|(t, c)| t.is_char(c))
                && (i == 0 || !matches!(rest[i - 1], Token::Char(ch, tex::Cat::Letter) if ch.is_alphabetic()))
                && !matches!(rest.get(i + n), Some(Token::Char(ch, tex::Cat::Letter)) if ch.is_alphabetic())
            {
                return Some(n);
            }
        }
        None
    };
    let mut prefix = Vec::new();
    let mut items: Vec<Vec<Token>> = Vec::new();
    let (mut braces, mut brackets) = (0i32, 0i32);
    let mut i = 0;
    while i < rest.len() {
        let t = &rest[i];
        if braces == 0 && brackets == 0 {
            if let Some(n) = word(i) {
                items.push(rest[i..i + n].to_vec());
                i += n;
                continue;
            }
        }
        match t {
            Token::Char(_, tex::Cat::Begin) => braces += 1,
            Token::Char(_, tex::Cat::End) => braces -= 1,
            Token::Char('[', _) if braces == 0 => brackets += 1,
            Token::Char(']', _) if braces == 0 => brackets -= 1,
            _ => {}
        }
        match items.last_mut() {
            Some(item) => item.push(t.clone()),
            None => prefix.push(t.clone()),
        }
        i += 1;
    }
    (prefix, items)
}

/// The options of a node item (`node [options] …`): where they are.
pub(super) fn node_options(item: &[Token]) -> Option<(usize, usize)> {
    let mut j = item.iter().position(|t| !matches!(t, Token::Char(_, tex::Cat::Letter)))?;
    while j < item.len() && item[j].is_space() {
        j += 1;
    }
    if !item.get(j)?.is_char('[') {
        return None;
    }
    let mut depth = 0;
    for (k, t) in item.iter().enumerate().skip(j) {
        match t {
            Token::Char(_, tex::Cat::Begin) => depth += 1,
            Token::Char(_, tex::Cat::End) => depth -= 1,
            Token::Char(']', _) if depth == 0 => return Some((j, k)),
            _ => {}
        }
    }
    None
}

/// The `pos` a node item asks for (`pos=…`, `midway`, `near end` and their friends).
pub(super) fn node_time(e: &mut Engine, item: &[Token]) -> Option<f64> {
    let (a, b) = node_options(item)?;
    let mut time = None;
    for it in tikra_engine::keys::split_list(&item[a + 1..b]) {
        let t = match it.key.trim() {
            "pos" => it.value.and_then(|v| e.number_of(v).ok()),
            "at start" => Some(0.0),
            "very near start" => Some(0.125),
            "near start" => Some(0.25),
            "midway" => Some(0.5),
            "near end" => Some(0.75),
            "very near end" => Some(0.875),
            "at end" => Some(1.0),
            _ => None,
        };
        if t.is_some() {
            time = t;
        }
    }
    time
}

/// A node item with its `pos` set to `frac` (of the segment it is placed on).
pub(super) fn with_pos(item: &[Token], frac: f64) -> Vec<Token> {
    let Some((_, b)) = node_options(item) else { return item.to_vec() };
    let mut out = item[..b].to_vec();
    out.extend(toks(&format!(",pos={}", tikra_engine::engine::pgf_number(frac))));
    out.extend(item[b..].iter().cloned());
    out
}

/// The anchors `nodes near coords align` may name.
pub(super) const ANCHORS: &[&str] =
    &["north", "south", "east", "west", "north east", "north west", "south east", "south west", "center", "base", "base east", "base west", "mid", "mid east", "mid west"];

/// `nodes near coords`: one node per point (`scatter/@post marker code`), aligned by
/// `nodes near coords/@@eval alignment`; bars move them by their bar shift, stacked bars (since
/// compat 1.9) put them in the middle of their bar.
pub(super) fn near_nodes(e: &mut Engine, ask: &NearQuery<'_>) -> Result<(Vec<Token>, Vec<NearNode>), Error> {
    let NearQuery { plot: p, pts, base, frame: f, handler, shift: bar_shift } = *ask;
    let Some(text) = keys::value_tokens(e, "/pgfplots/@nodes near coords") else { return Ok((Vec::new(), Vec::new())) };
    if tikra_engine::keys::trim(&text).is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let source = keys::value(e, "/pgfplots/point meta");
    let symbolic = source == "explicit symbolic";
    let align = keys::value(e, "/pgfplots/nodes near coords align");
    let align = if align == "auto" {
        if source == "x" {
            "horizontal".to_string()
        } else {
            "vertical".to_string()
        }
    } else {
        align
    };
    let compat = keys::compat(e);
    let stacked = p.stack.is_some() && compat >= 109 && matches!(handler, "ybar" | "xbar");
    let offset = keys::number(e, "/pgfplots/nodes near coords bar offset").unwrap_or(0.5);
    let shift = match handler {
        // (the nodes of stacked bars before compat 1.9 stay where the stacked point is)
        _ if p.stack.is_some() && !stacked => Point::default(),
        "ybar" => unit(f.e[0]).scale(bar_shift),
        "xbar" => unit(f.e[1]).scale(bar_shift),
        _ => Point::default(),
    };
    let mut nodes = Vec::new();
    for (i, l) in pts.iter().enumerate() {
        if !l.iter().take(if f.three_d { 3 } else { 2 }).all(|v| v.is_finite()) {
            continue;
        }
        let point = f.at(*l);
        let c = p.coords.get(i).copied().unwrap_or_else(|| super::super::plot::Coord::new(f64::NAN, f64::NAN, f64::NAN));
        let label = (c.label != super::super::plot::NO_LABEL).then(|| p.labels.get(c.label as usize).cloned()).flatten();
        // before compat 1.9 the nodes of stacked plots show the stacked value (f(x) after stacking)
        let meta = match p.stack {
            // f(x) is y (2d): x stacked bars keep it
            Some(_) if !stacked && matches!(source.as_str(), "f(x)" | "y" | "x") => {
                if source == "x" {
                    l[0]
                } else {
                    l[1]
                }
            }
            _ => c.meta,
        };
        let at = if stacked {
            let b = base.get(i).copied().unwrap_or(0.0);
            if handler == "ybar" {
                f.at([l[0], offset * l[1] + (1.0 - offset) * b, l[2]])
            } else {
                f.at([offset * l[0] + (1.0 - offset) * b, l[1], l[2]])
            }
        } else {
            point
        };
        let a = if stacked {
            "anchor=center".to_string()
        } else {
            match align.as_str() {
                "horizontal" if symbolic => "above".into(),
                "horizontal" => (if meta < 0.0 { "left" } else { "right" }).into(),
                "vertical" if symbolic => "above".into(),
                "vertical" => (if meta < 0.0 { "below" } else { "above" }).into(),
                other if ANCHORS.contains(&other) => format!("anchor={other}"),
                other => other.to_string(),
            }
        };
        let style = p.coord_styles.get(c.style as usize).cloned().unwrap_or_default();
        let deps = p.deps.get(c.deps as usize).cloned().unwrap_or_default();
        nodes.push(NearNode { point, at: at.plus(shift), align: a, meta, label, index: i, data: [c.x, c.y, c.z], style, deps });
    }
    Ok((text, nodes))
}

/// Draws the nodes of `nodes near coords`, in the plot's style.
pub fn draw_near_nodes(e: &mut Engine, m: &Marks) -> Result<(), Error> {
    if m.nodes.is_empty() {
        return Ok(());
    }
    e.begin_group();
    let r = (|| -> Result<(), Error> {
        e.st.mode = Default::default();
        e.st.pending = Transform::default();
        e.process_keys(m.style.clone(), Some("/pgfplots"))?;
        e.st.apply_pending();
        let default = tex::tokens_to_string(&m.node_text).replace(' ', "") == "\\pgfmathprintnumber\\pgfplotspointmeta";
        for n in &m.nodes {
            e.begin_group();
            let r = (|| -> Result<(), Error> {
                let meta = match &n.label {
                    Some(l) if !n.meta.is_finite() => tex::tokens_to_string(l),
                    _ => tikra_engine::engine::pgf_number(n.meta),
                };
                define(e, "pgfplotspointmeta", &meta);
                define(e, "coordindex", &n.index.to_string());
                for (name, value) in &n.deps {
                    define(e, name, value);
                }
                for (k, c) in ['x', 'y', 'z'].iter().enumerate() {
                    super::super::keys::set(e, &format!("/data point/{c}"), &tikra_engine::engine::pgf_number(n.data[k]));
                }
                // symbolic point meta is shown as written
                let text = match (&n.label, default && !n.meta.is_finite()) {
                    (Some(l), true) => l.clone(),
                    _ => {
                        // `\pgfmathprintnumber\pgfplotspointmeta` takes the macro, then the rest
                        let t = super::super::print_numbers(e, &m.node_text)?;
                        e.tex.expand_fully(t)?
                    }
                };
                let mut options = toks(&format!("{},/pgfplots/every node near coord", n.align));
                if !n.style.is_empty() {
                    options.push(Token::other(','));
                    options.extend(n.style.clone());
                }
                let items = e.node(NodeSpec { options, name: None, at: Some(n.at), text, place: OnPath::Point(n.at), point: n.at, coordinate: false, default_pos: None })?;
                e.canvas.items.extend(items);
                Ok(())
            })();
            e.end_group();
            r?;
        }
        Ok(())
    })();
    e.end_group();
    r
}
