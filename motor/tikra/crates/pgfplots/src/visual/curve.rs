use tikra_engine::scene::{Point, Segment, Transform};
use tikra_engine::tex::{self, Token};
use tikra_engine::{Error, PT};

use super::super::draw::Frame;
use super::super::keys;
use super::super::plot::Plot;
use super::super::{toks, Engine};

use super::nodes::{near_nodes, post_nodes};
use super::stamps::error_bars;
use super::{canvas_points, define, rect, runs, smooth, unit, Marks, NearQuery, PlacedNodes, PlotDraw};

/// What the marks of a plot are assembled from. `mark_saved` is the plot mark from before the
/// line was stroked; None reads it after the nodes are built.
pub(super) struct MarkAsk<'a> {
    plot: &'a Plot,
    pts: &'a [[f64; 3]],
    frame: &'a Frame,
    base: &'a [f64],
    handler: &'a str,
    shift: f64,
    mark_saved: Option<bool>,
    canvas: Option<&'a [Option<Point>]>,
}

/// The canvas points a line is drawn through, with the plot's empty-line jumps.
pub(super) struct JumpLine<'a> {
    plot: &'a Plot,
    frame: &'a Frame,
    points: &'a [Option<Point>],
}

/// Bars and combs: where they stand, and how wide a bar is.
pub(super) struct BarGeom<'a> {
    pts: &'a [[f64; 3]],
    base: &'a [f64],
    frame: &'a Frame,
    stack: Option<char>,
    vertical: bool,
    width: f64,
    shift: f64,
}

impl BarGeom<'_> {
    fn level(&self, i: usize, axis: usize) -> f64 {
        if self.stack == Some(['x', 'y'][axis]) {
            self.base.get(i).copied().unwrap_or(0.0)
        } else {
            self.frame.zero[axis]
        }
    }

    fn at(&self, x: f64, y: f64) -> Point {
        self.frame.at([x, y, self.frame.min[2]])
    }
}

/// One quiver arrow plot.
pub(super) struct QuiverPlot<'a> {
    plot: &'a Plot,
    pts: &'a [[f64; 3]],
    frame: &'a Frame,
}

/// `\closedcycle`: the points, the stacked base and the handler the way back is stepped with.
pub(super) struct CycleClose<'a> {
    plot: &'a Plot,
    pts: &'a [[f64; 3]],
    base: &'a [f64],
    frame: &'a Frame,
    handler: &'a str,
}

/// A smooth plot handler, or a polyline.
pub(super) struct Curve {
    tension: f64,
    smooth: bool,
}

/// Draws a plot; returns its marks, for later.
pub fn draw_plot(e: &mut Engine, draw: &PlotDraw<'_>) -> Result<Option<Marks>, Error> {
    e.begin_group();
    let r = draw_plot_in_group(e, draw);
    e.end_group();
    r
}

pub(super) fn apply_plot_style(e: &mut Engine, draw: &PlotDraw<'_>) -> Result<Vec<Token>, Error> {
    let p = draw.plot;
    define(e, "numplotsofactualtype", &draw.of_type.to_string());
    define(e, "plotnumofactualtype", &draw.index_of_type.to_string());
    define(e, "plotnum", &p.index.to_string());
    e.st.mode = Default::default();
    e.st.pending = Transform::default();
    // the areas of `fill between` are filled, not drawn (`\tikzfillbetween`)
    let mut style = toks(if p.fill_between.is_some() { "/pgfplots/every fill between plot," } else { "/tikz/draw," });
    style.extend(p.style.clone());
    e.process_keys(style.clone(), Some("/pgfplots"))?;
    e.st.apply_pending();
    // `only marks` or `scatter` without a mark: `mark=*` (`\pgfplots@markers@check@for@default@plot@mark`)
    if (e.st.plot.only_marks || p.scatter) && e.st.plot.plot_mark.as_deref().is_none_or(|m| m.trim().is_empty() || m.trim() == "none") {
        e.process_keys(toks("/tikz/mark=*"), Some("/tikz"))?;
        style.extend(toks(",/tikz/mark=*"));
    }
    if let Some(code) = keys::value_tokens(e, "/pgfplots/execute at begin plot visualization") {
        e.run_tokens(code)?;
    }
    Ok(style)
}

/// One dimensional patches of one color (`supports global path`), closed down to the zero level.
pub(super) fn close_global_path(e: &mut Engine, draw: &PlotDraw<'_>) -> Option<Vec<Segment>> {
    let p = draw.plot;
    let pts = draw.pts;
    let f = draw.frame;
    let mut segs = super::super::mesh::global_path(e, p, pts, f)?;
    let ends = (pts.iter().find(|l| l[0].is_finite()), pts.iter().rev().find(|l| l[0].is_finite()));
    if let (true, (Some(first), Some(last))) = (p.post.iter().any(|t| t.is_cs("closedcycle")), ends) {
        segs.push(Segment::LineTo(f.at([last[0], f.zero[1], f.min[2]])));
        segs.push(Segment::LineTo(f.at([first[0], f.zero[1], f.min[2]])));
        segs.push(Segment::Close);
    }
    Some(segs)
}

pub(super) fn assembled_marks(e: &mut Engine, ask: &MarkAsk<'_>, style: Vec<Token>) -> Result<Option<Marks>, Error> {
    let (node_text, nodes) = near_nodes(e, &NearQuery { plot: ask.plot, pts: ask.pts, base: ask.base, frame: ask.frame, handler: ask.handler, shift: ask.shift })?;
    let present = match ask.mark_saved {
        Some(saved) => saved,
        None => e.st.plot.plot_mark.is_some(),
    };
    if !present && nodes.is_empty() {
        return Ok(None);
    }
    let owned;
    let cp = match ask.canvas {
        Some(c) => c,
        None => {
            owned = canvas_points(ask.frame, ask.pts);
            &owned
        }
    };
    let (points, styled, deps) = mark_points(ask.plot, cp);
    Ok(Some(Marks { style, points, scatter: ask.plot.scatter, node_text, nodes, styled, deps }))
}

pub(super) fn bare_marks<'a>(draw: &PlotDraw<'a>) -> MarkAsk<'a> {
    MarkAsk { plot: draw.plot, pts: draw.pts, frame: draw.frame, base: &[], handler: "", shift: 0.0, mark_saved: None, canvas: None }
}

/// An empty line in the input interrupts a 2d plot (`empty line=jump`, or `auto` before compat 1.4).
pub(super) fn with_empty_jumps(e: &mut Engine, line: &JumpLine<'_>) -> Vec<Option<Point>> {
    let empty_line = keys::value(e, "/pgfplots/empty line");
    let jumps =
        !line.frame.three_d && !line.plot.scanlines.is_empty() && (empty_line == "jump" || (matches!(empty_line.as_str(), "" | "auto") && !(1..104).contains(&keys::compat(e))));
    if !jumps {
        return line.points.to_vec();
    }
    let mut out = Vec::with_capacity(line.points.len() + line.plot.scanlines.len());
    for (i, c) in line.points.iter().enumerate() {
        if i > 0 && line.plot.scanlines.contains(&i) {
            out.push(None);
        }
        out.push(*c);
    }
    out
}

pub(super) fn draw_bars(segs: &mut Vec<Segment>, g: &BarGeom<'_>) {
    let (ua, _) = if g.vertical { (unit(g.frame.e[0]), unit(g.frame.e[1])) } else { (unit(g.frame.e[1]), unit(g.frame.e[0])) };
    for (i, l) in g.pts.iter().enumerate() {
        if !(l[0].is_finite() && l[1].is_finite()) {
            continue;
        }
        let (from, to) = if g.vertical { (g.at(l[0], g.level(i, 1)), g.at(l[0], l[1])) } else { (g.at(g.level(i, 0), l[1]), g.at(l[0], l[1])) };
        let c0 = from.plus(ua.scale(g.shift - g.width / 2.0));
        let c1 = to.plus(ua.scale(g.shift + g.width / 2.0));
        rect(segs, c0, c1);
    }
}

pub(super) fn draw_bar_intervals(e: &mut Engine, segs: &mut Vec<Segment>, g: &BarGeom<'_>) {
    let w = keys::number(e, "/pgf/bar interval width").unwrap_or(1.0);
    let sh = keys::number(e, "/pgf/bar interval shift").unwrap_or(0.5);
    for i in 0..g.pts.len().saturating_sub(1) {
        let (l, n) = (g.pts[i], g.pts[i + 1]);
        if !(l[0].is_finite() && l[1].is_finite() && n[0].is_finite() && n[1].is_finite()) {
            continue;
        }
        if g.vertical {
            let d = n[0] - l[0];
            let x0 = l[0] + d * (sh - w / 2.0);
            rect(segs, g.at(x0, g.level(i, 1)), g.at(x0 + d * w, l[1]));
        } else {
            let d = n[1] - l[1];
            let y0 = l[1] + d * (sh - w / 2.0);
            rect(segs, g.at(g.level(i, 0), y0), g.at(l[0], y0 + d * w));
        }
    }
}

pub(super) fn draw_steps(segs: &mut Vec<Segment>, line: &[Option<Point>], handler: &str) {
    for run in runs(line) {
        segs.push(Segment::MoveTo(run[0]));
        for w in run.windows(2) {
            let (a, b) = (w[0], w[1]);
            match handler {
                "const" => {
                    segs.push(Segment::LineTo(Point::new(b.x, a.y)));
                    segs.push(Segment::LineTo(b));
                }
                "const right" => {
                    segs.push(Segment::LineTo(Point::new(a.x, b.y)));
                    segs.push(Segment::LineTo(b));
                }
                "const mid" => {
                    let m = (a.x + b.x) / 2.0;
                    segs.push(Segment::LineTo(Point::new(m, a.y)));
                    segs.push(Segment::LineTo(Point::new(m, b.y)));
                    segs.push(Segment::LineTo(b));
                }
                "jump left" => {
                    segs.push(Segment::LineTo(Point::new(b.x, a.y)));
                    segs.push(Segment::MoveTo(b));
                }
                "jump right" => {
                    segs.push(Segment::MoveTo(Point::new(a.x, b.y)));
                    segs.push(Segment::LineTo(b));
                }
                _ => {
                    let m = (a.x + b.x) / 2.0;
                    segs.push(Segment::LineTo(Point::new(m, a.y)));
                    segs.push(Segment::MoveTo(Point::new(m, b.y)));
                    segs.push(Segment::LineTo(b));
                }
            }
        }
    }
}

pub(super) fn draw_curve(segs: &mut Vec<Segment>, line: &[Option<Point>], curve: &Curve) {
    for run in runs(line) {
        segs.push(Segment::MoveTo(run[0]));
        if curve.smooth {
            smooth(&run, curve.tension, segs);
        } else {
            for q in &run[1..] {
                segs.push(Segment::LineTo(*q));
            }
        }
    }
}

/// Rows give a point and its tie's other end: the curve through all of them, then every nth tie.
pub(super) fn draw_tielines(e: &mut Engine, cp: &[Option<Point>], segs: &mut Vec<Segment>) -> Result<(), Error> {
    let ends: Vec<Point> = cp.iter().step_by(2).flatten().copied().collect();
    let others: Vec<Point> = cp.iter().skip(1).step_by(2).flatten().copied().collect();
    let curve: Vec<Point> = ends.iter().chain(others.iter().rev()).copied().collect();
    if let Some((first, rest)) = curve.split_first() {
        let mut path = vec![Segment::MoveTo(*first)];
        path.extend(rest.iter().map(|q| Segment::LineTo(*q)));
        e.begin_group();
        let r = (|| {
            e.process_keys(toks("/pgfplots/tieline/every curve"), Some("/pgfplots"))?;
            e.st.apply_pending();
            e.use_path(path, "addplot")
        })();
        e.end_group();
        r?;
    }
    e.process_keys(toks("/pgfplots/tieline/every tieline"), Some("/pgfplots"))?;
    e.st.apply_pending();
    let nth = keys::number(e, "/pgfplots/tieline/each nth tie").unwrap_or(1.0).max(1.0) as usize;
    if keys::value(e, "/tikz/@plot handler") == "tieline" {
        for (i, pair) in cp.chunks(2).enumerate() {
            if let ([Some(a), Some(b)], true) = (pair, (i + 1) % nth == 0) {
                segs.push(Segment::MoveTo(*b));
                segs.push(Segment::LineTo(*a));
            }
        }
    }
    Ok(())
}

/// An arrow per point, each a path of its own (its tips at its end).
pub(super) fn draw_quiver(e: &mut Engine, q: &QuiverPlot<'_>) -> Result<(), Error> {
    let color = keys::value(e, "/pgfplots/quiver/arrow color");
    let f = q.frame;
    for (i, l) in q.pts.iter().enumerate() {
        let Some(c) = q.plot.coords.get(i) else { continue };
        if !l[0].is_finite() || !l[1].is_finite() || !c.uvw.iter().take(if f.three_d { 3 } else { 2 }).all(|v| v.is_finite()) {
            continue;
        }
        let end = [f.logical(0, c.x + c.uvw[0]), f.logical(1, c.y + c.uvw[1]), if f.three_d { f.logical(2, c.z + c.uvw[2]) } else { f.min[2] }];
        let segs = vec![Segment::MoveTo(f.at(*l)), Segment::LineTo(f.at(end))];
        e.begin_group();
        let r = (|| -> Result<(), Error> {
            if c.meta.is_finite() {
                let t = (c.meta - f.meta[0]) / (f.meta[1] - f.meta[0]) * 1000.0;
                define(e, "pgfplotspointmetatransformed", &tikra_engine::engine::pgf_number(if t.is_finite() { t } else { 0.0 }));
                if f.colormap.is_some() {
                    e.colors.insert("mapped color".into(), super::super::mesh::map(f, c.meta));
                }
            }
            e.st.pending = Transform::default();
            let mut style = Vec::new();
            if !color.is_empty() {
                style.extend(toks(&format!("/tikz/color={color},")));
            }
            style.extend(toks("/pgfplots/quiver/every arrow"));
            e.process_keys(style, Some("/pgfplots"))?;
            e.st.apply_pending();
            e.use_path(segs, "addplot")
        })();
        e.colors.insert("mapped color".into(), tikra_engine::color::Color::BLACK);
        e.end_group();
        r?;
    }
    Ok(())
}

/// Down to the zero level (or the previous stacked plot) and back.
pub(super) fn close_cycle(segs: &mut Vec<Segment>, c: &CycleClose<'_>) {
    let f = c.frame;
    let at = |x: f64, y: f64| f.at([x, y, f.min[2]]);
    let Some(first) = c.pts.iter().find(|l| l[0].is_finite()) else { return };
    let Some(last) = c.pts.iter().rev().find(|l| l[0].is_finite()) else { return };
    if c.plot.stack == Some('y') {
        // the previous plot backwards, with its plot handler: a step that holds to the
        // right holds to the left when walked back
        let back: Vec<Point> = c.pts.iter().enumerate().rev().filter(|(_, l)| l[0].is_finite()).map(|(i, l)| at(l[0], c.base.get(i).copied().unwrap_or(f.zero[1]))).collect();
        let step = match c.handler {
            "const" => "const right",
            "const right" => "const",
            "const mid" => "const mid",
            _ => "",
        };
        for (k, b) in back.iter().enumerate() {
            if k > 0 {
                let a = back[k - 1];
                match step {
                    "const" => segs.push(Segment::LineTo(Point::new(b.x, a.y))),
                    "const right" => segs.push(Segment::LineTo(Point::new(a.x, b.y))),
                    "const mid" => {
                        let m = (a.x + b.x) / 2.0;
                        segs.push(Segment::LineTo(Point::new(m, a.y)));
                        segs.push(Segment::LineTo(Point::new(m, b.y)));
                    }
                    _ => {}
                }
            }
            segs.push(Segment::LineTo(*b));
        }
    } else if c.plot.three_d {
        // down to the zero level of z, below the last point and the first
        segs.push(Segment::LineTo(f.at([last[0], last[1], f.zero[2]])));
        segs.push(Segment::LineTo(f.at([first[0], first[1], f.zero[2]])));
    } else {
        segs.push(Segment::LineTo(at(last[0], f.zero[1])));
        segs.push(Segment::LineTo(at(first[0], f.zero[1])));
    }
    segs.push(Segment::Close);
}

/// The path goes on after the plot: `-- cycle`, `|- (0,0) -- cycle`.
pub(super) fn append_path_ops(e: &mut Engine, segs: &mut Vec<Segment>, rest: &mut Vec<Token>) {
    let mut text = tex::tokens_to_string(rest);
    let mut moved = false;
    loop {
        let t = text.trim_start();
        let Some(op) = ["--", "|-", "-|"].into_iter().find(|op| t.starts_with(op)) else { break };
        let after = t[2..].trim_start();
        if let Some(more) = after.strip_prefix("cycle") {
            segs.push(Segment::Close);
            text = more.to_string();
            moved = true;
            break;
        }
        let Some(inner) = after.strip_prefix('(').and_then(|a| a.find(')').map(|i| (a[..i].to_string(), a[i + 1..].to_string()))) else { break };
        let Ok(q) = e.coordinate(&inner.0) else { break };
        let q = q.p;
        let cur = segs.iter().rev().find_map(|s| match s {
            Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => Some(*p),
            Segment::Close => None,
        });
        if let Some(c) = cur {
            match op {
                "|-" => segs.push(Segment::LineTo(Point::new(c.x, q.y))),
                "-|" => segs.push(Segment::LineTo(Point::new(q.x, c.y))),
                _ => {}
            }
        }
        segs.push(Segment::LineTo(q));
        text = inner.1;
        moved = true;
    }
    if moved {
        *rest = super::super::toks(&text);
    }
}

/// With the path's decoration, preactions and postactions (`\tikz@finish`).
pub(super) fn stroke_plot(e: &mut Engine, segs: &[Segment], name: Option<&str>) -> Result<(), Error> {
    // pgfplots' `every path` gives them the plot's draw and fill modes back, and its name
    let mut modes = format!("{}{}", if e.st.mode.draw { "draw," } else { "" }, if e.st.mode.fill { "fill," } else { "" });
    if let Some(name) = name {
        modes.push_str(&format!("name path={{{name}}},"));
    }
    let st = &mut *e.st;
    for actions in [&mut st.preactions, &mut st.postactions] {
        if !actions.is_empty() {
            for a in std::rc::Rc::make_mut(actions).iter_mut() {
                let mut o = super::super::toks(&modes);
                o.append(a);
                *a = o;
            }
        }
    }
    let outer = e.st.clone();
    e.finish_path(segs.to_vec(), &outer, "addplot")
}

pub(super) fn draw_handler(e: &mut Engine, draw: &PlotDraw<'_>, style: Vec<Token>) -> Result<Option<Marks>, Error> {
    let p = draw.plot;
    let pts = draw.pts;
    let f = draw.frame;
    let handler = keys::value(e, "/tikz/@plot handler");
    let cp = canvas_points(f, pts);
    let line_cp = with_empty_jumps(e, &JumpLine { plot: p, frame: f, points: &cp });
    let mut segs: Vec<Segment> = Vec::new();
    let bar_width = keys::length(e, "/pgf/bar width").unwrap_or(10.0 * PT);
    define(e, "pgfplotbarwidth", &format!("{}pt", bar_width / PT));
    // stacked bars stand on each other: no shift
    let given = p.bar_shift_given || tex::tokens_to_string(&p.style).contains("bar shift");
    let bar_shift = if p.stack.is_some() && !given { 0.0 } else { keys::length(e, "/pgf/bar shift").unwrap_or(0.0) };
    define(e, "pgfplotbarshift", &format!("{}pt", bar_shift / PT));
    let geom = |vertical: bool| BarGeom { pts, base: draw.base, frame: f, stack: p.stack, vertical, width: bar_width, shift: bar_shift };
    match handler.as_str() {
        // box plots draw their box; the points are the outliers' marks
        "boxplot" => super::super::boxplot::draw(e, p, f, draw.index_of_type)?,
        "contour prepared" => super::super::contour::draw(e, p, pts, f)?,
        "tieline" => draw_tielines(e, &cp, &mut segs)?,
        "quiver" => draw_quiver(e, &QuiverPlot { plot: p, pts, frame: f })?,
        // rays from the center
        "polar comb" => {
            for l in pts {
                if l[0].is_finite() && l[1].is_finite() {
                    segs.push(Segment::MoveTo(f.at([l[0], 0.0, 0.0])));
                    segs.push(Segment::LineTo(f.at(*l)));
                }
            }
        }
        "ybar" | "xbar" => draw_bars(&mut segs, &geom(handler == "ybar")),
        "ybar interval" | "xbar interval" => draw_bar_intervals(e, &mut segs, &geom(handler == "ybar interval")),
        "ycomb" | "xcomb" => {
            let g = geom(handler == "ycomb");
            for (i, l) in pts.iter().enumerate() {
                if !(l[0].is_finite() && l[1].is_finite()) {
                    continue;
                }
                let from = if g.vertical { g.at(l[0], g.level(i, 1)) } else { g.at(g.level(i, 0), l[1]) };
                segs.push(Segment::MoveTo(from));
                segs.push(Segment::LineTo(g.at(l[0], l[1])));
            }
        }
        h if h.starts_with("const") || h.starts_with("jump") => draw_steps(&mut segs, &line_cp, h),
        _ => draw_curve(&mut segs, &line_cp, &Curve { tension: e.st.plot.tension, smooth: e.st.plot.plot_handler == "smooth" }),
    }
    let closed = p.post.iter().any(|t| t.is_cs("closedcycle"));
    if closed {
        close_cycle(&mut segs, &CycleClose { plot: p, pts, base: draw.base, frame: f, handler: &handler });
    }
    let mut rest: Vec<Token> = p.post.iter().filter(|t| !t.is_cs("closedcycle")).cloned().collect();
    if !closed && !segs.is_empty() {
        append_path_ops(e, &mut segs, &mut rest);
    }
    let name_path = e.st.name_path.take();
    if let Some(name) = &name_path {
        e.named_paths.insert(name.clone(), segs.clone());
    }
    let only_marks = e.st.plot.only_marks;
    let mark = e.st.plot.plot_mark.clone();
    // a mark-only path does not need the line
    if !only_marks && !segs.is_empty() {
        stroke_plot(e, &segs, name_path.as_deref())?;
    }
    if rest.iter().any(|t| !t.is_space()) {
        post_nodes(e, &PlacedNodes { plot: p, pts, canvas: &cp, frame: f, rest: &rest })?;
    }
    error_bars(e, p, pts, f)?;
    let ask = MarkAsk { plot: p, pts, frame: f, base: draw.base, handler: &handler, shift: bar_shift, mark_saved: Some(mark.is_some()), canvas: Some(&cp) };
    assembled_marks(e, &ask, style)
}

pub(super) fn draw_plot_in_group(e: &mut Engine, draw: &PlotDraw<'_>) -> Result<Option<Marks>, Error> {
    let style = apply_plot_style(e, draw)?;
    let p = draw.plot;
    let pts = draw.pts;
    let f = draw.frame;
    if let Some(spec) = &p.fill_between {
        super::super::fillbetween::draw(e, p, spec)?;
        return Ok(None);
    }
    if p.handler == "contour filled" {
        super::super::contour::draw_filled(e, p, pts, f)?;
        return Ok(None);
    }
    if let Some(g) = &p.graphics {
        super::super::graphics::draw(e, g, pts, f);
        return Ok(None);
    }
    if let Some(segs) = close_global_path(e, draw) {
        e.use_path(segs, "addplot")?;
        return assembled_marks(e, &bare_marks(draw), style);
    }
    if p.mesh.is_some() {
        super::super::mesh::draw(e, &super::super::mesh::MeshDraw { plot: p, pts, frame: f }, &style)?;
        return assembled_marks(e, &bare_marks(draw), style);
    }
    draw_handler(e, draw, style)
}

/// The marks' points with their point meta, and apart those with a `coordinate style`.
#[allow(clippy::type_complexity)]
/// The marks of a plot: plain (with their `visualization depends on` macros), and with a style.
pub(super) type MarkPoints = (Vec<(Point, f64)>, Vec<(Point, f64, Vec<Token>)>, Vec<Vec<(String, String)>>);

pub(super) fn mark_points(p: &Plot, cp: &[Option<Point>]) -> MarkPoints {
    let mut points = Vec::new();
    let mut styled = Vec::new();
    let mut deps = Vec::new();
    for (i, c) in cp.iter().enumerate() {
        let Some(c) = c else { continue };
        let co = p.coords.get(i);
        let meta = co.map_or(f64::NAN, |k| k.meta);
        // only the markers of a scatter plot are drawn one by one, with their style
        match co.and_then(|k| p.coord_styles.get(k.style as usize)).filter(|_| p.scatter) {
            Some(s) => styled.push((*c, meta, s.clone())),
            None => {
                points.push((*c, meta));
                deps.push(co.and_then(|k| p.deps.get(k.deps as usize)).cloned().unwrap_or_default());
            }
        }
    }
    (points, styled, deps)
}
