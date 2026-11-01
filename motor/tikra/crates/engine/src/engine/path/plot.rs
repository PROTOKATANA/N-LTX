use crate::canvas::Item;
use crate::scene::{Point, Segment};
use crate::tex::{Cat, Token};
use crate::Error;

use super::super::coords::{read_coordinate_tokens, Coord};
use super::super::nodes::OnPath;
use super::super::{macro_from, tikra_tokens, Engine};
use super::*;

/// `plot[options] coordinates {…}` and `plot[options] (expression)`.
/// The domain, sample count and variable a plot expression is drawn over.
pub(super) struct PlotSpan {
    start: f64,
    end: f64,
    samples: usize,
    var: String,
}

/// What a plot hands back to the path once its group ends.
pub(super) struct PlotDrawn {
    points: Vec<Point>,
    segs: Vec<Segment>,
    marks: Vec<Item>,
    only_marks: bool,
    continues: bool,
}

pub(super) fn plot_bracket(e: &mut Engine, depth: Option<usize>) -> Result<(), Error> {
    e.process_keys(tikra_tokens("every plot/.try"), Some("/tikz"))?;
    if let Some(t) = peek(e, depth)? {
        if t.is_char('[') {
            let _ = next(e, depth)?;
            let o = e.tex.read_until_char(']', false)?;
            e.process_keys(o, Some("/tikz"))?;
        }
    }
    Ok(())
}

/// `plot coordinates { (a) (b) … }`.
pub(super) fn plot_coordinates(e: &mut Engine, depth: Option<usize>) -> Result<Vec<Point>, Error> {
    let w = word(e, 'c');
    if w != "coordinates" {
        return Err(Error::Figure(format!("plot {w} not supported")));
    }
    let list = match next(e, depth)? {
        Some(Token::Char(_, Cat::Begin)) => e.tex.read_group_body()?,
        _ => return Err(Error::Figure("plot coordinates without a list".into())),
    };
    let depth0 = e.tex.depth();
    e.tex.push_frame(list);
    let mut points = Vec::new();
    while let Some(t) = e.tex.get_x_token_until(depth0)? {
        if t.is_char('(') {
            let r = coordinate(e)?;
            points.push(r.p);
        }
    }
    Ok(points)
}

/// `plot file {name}` and `plot function {…}` (the table gnuplot wrote).
pub(super) fn plot_table(e: &mut Engine) -> Result<Vec<Point>, Error> {
    let w = word(e, 'f');
    let arg = e.argument()?;
    let name = match w.as_str() {
        "file" => e.tex.expand_to_string(arg)?.trim().to_string(),
        "function" => {
            let key = |e: &mut Engine, k: &str| crate::keys::value(&e.tex, k).map(|v| e.tex.expand_to_string(v).unwrap_or_default()).unwrap_or_default();
            let (prefix, id) = (key(e, "/tikz/prefix"), key(e, "/tikz/id"));
            format!("{prefix}{}.table", if id.is_empty() { "plot" } else { id.trim() })
        }
        _ => return Err(Error::Figure(format!("plot {w} not supported"))),
    };
    let text = crate::files::read_file(&name, &[]).map_err(|m| Error::Figure(format!("plot {w}: {m}")))?;
    let mut points = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('%') {
            continue;
        }
        let f: Vec<&str> = line.split_whitespace().collect();
        let (Some(x), Some(y)) = (f.first().and_then(|v| v.parse::<f64>().ok()), f.get(1).and_then(|v| v.parse::<f64>().ok())) else { continue };
        // `u`: undefined, the line breaks there
        if f.get(2) == Some(&"u") {
            points.push(Point::new(f64::NAN, f64::NAN));
            continue;
        }
        let r = e.coordinate(&format!("{},{}", super::super::pgf_number(x), super::super::pgf_number(y)))?;
        points.push(r.p);
    }
    Ok(points)
}

/// `plot (expression)` sampled across the domain.
pub(super) fn plot_expression(e: &mut Engine, span: &PlotSpan) -> Result<Vec<Point>, Error> {
    let tokens = read_coordinate_tokens(e)?;
    let name = Token::cs(&span.var);
    let n = span.samples;
    let (d0, d1) = (span.start, span.end);
    let values = e.st.plot.samples_at.clone().unwrap_or_else(|| (0..n).map(|k| d0 + (d1 - d0) * k as f64 / (n - 1) as f64).collect());
    let mut points = Vec::new();
    for v in values {
        e.tex.set_meaning(&name, macro_from(&super::super::pgf_number(v)), false);
        let text = e.math_string(tokens.clone())?;
        match e.coordinate(&text) {
            Ok(r) if r.p.x.is_finite() && r.p.y.is_finite() => points.push(r.p),
            Ok(_) => {}
            Err(err) => {
                e.warn(format!("plot: {err}"));
                break;
            }
        }
    }
    Ok(points)
}

/// The points of `plot`, from coordinates, a file, or an expression.
pub(super) fn read_plot_points(e: &mut Engine, depth: Option<usize>, span: &PlotSpan) -> Result<Vec<Point>, Error> {
    match next(e, depth)? {
        Some(Token::Char('c', Cat::Letter)) => plot_coordinates(e, depth),
        Some(Token::Char('f', Cat::Letter)) => plot_table(e),
        Some(t) if t.is_char('(') => plot_expression(e, span),
        _ => Err(Error::Figure("plot without an expression".into())),
    }
}

/// Appends a finished plot to the path being built.
pub(super) fn attach_plot(b: &mut Builder, drawn: PlotDrawn) -> Result<(), Error> {
    let PlotDrawn { points, mut segs, marks, only_marks, continues } = drawn;
    b.node_items.extend(marks);
    if points.is_empty() || only_marks {
        if let Some(p) = points.last() {
            b.current = Some(Coord { p: *p, node: None });
        }
        return Ok(());
    }
    // `--plot` continues the path: the plot's first move is a line
    if continues && b.current.is_some() {
        if let Some(Segment::MoveTo(p)) = segs.first().copied() {
            segs[0] = Segment::LineTo(p);
        }
    }
    if let Some(Segment::MoveTo(p)) = segs.first() {
        b.subpath_start = Some(*p);
    }
    b.segs.extend(segs);
    let points: Vec<Point> = points.into_iter().filter(|p| p.x.is_finite()).collect();
    let n = points.len();
    if n == 0 {
        return Ok(());
    }
    b.last = Some(OnPath::Segment(points[n.saturating_sub(2)], None, points[n - 1]));
    b.current = Some(Coord { p: points[n - 1], node: None });
    Ok(())
}

pub(super) fn plot(e: &mut Engine, b: &mut Builder, depth: Option<usize>, continues: bool) -> Result<(), Error> {
    e.begin_group();
    let res = (|| -> Result<PlotDrawn, Error> {
        plot_bracket(e, depth)?;
        let (d0, d1) = e.st.plot.domain;
        let n = e.st.plot.samples.max(2);
        let var = e.st.plot.variable.clone();
        let span = PlotSpan { start: d0, end: d1, samples: n, var };
        let points = read_plot_points(e, depth, &span)?;
        let marks = e.plot_marks(&points)?;
        let segs = plot_segments(e, &points)?;
        Ok(PlotDrawn { points, segs, marks, only_marks: e.st.plot.only_marks, continues })
    })();
    e.end_group();
    attach_plot(b, res?)
}

/// The points of one plot run, and the segments the handler appends.
pub(super) struct PlotRun<'a> {
    q: &'a [Point],
    segs: &'a mut Vec<Segment>,
}

/// `\pgfplothandlercurveto`: each inner point's supports along its neighbours' chord.
pub(super) fn smooth_plot(run: &mut PlotRun<'_>, tension: f64) {
    let q = run.q;
    let n = q.len();
    run.segs.push(Segment::MoveTo(q[0]));
    let mut support = q[0];
    for i in 1..n - 1 {
        let d = q[i + 1].minus(q[i - 1]).scale(tension);
        run.segs.push(Segment::CurveTo(support, q[i].minus(d), q[i]));
        support = q[i].plus(d);
    }
    run.segs.push(Segment::CurveTo(support, q[n - 1], q[n - 1]));
}

/// `\pgfplothandlerclosedcurve`: from the second point round to it again.
pub(super) fn smooth_cycle(run: &mut PlotRun<'_>, tension: f64) {
    let q = run.q;
    let n = q.len();
    let d = |i: usize| q[(i + 1) % n].minus(q[(i + n - 1) % n]).scale(tension);
    run.segs.push(Segment::MoveTo(q[1]));
    for step in 1..=n {
        let (i, j) = (step % n, (step + 1) % n);
        run.segs.push(Segment::CurveTo(q[i].plus(d(i)), q[j].minus(d(j)), q[j]));
    }
    run.segs.push(Segment::Close);
}

/// A comb from the axis (or the origin) to each point.
pub(super) fn comb_plot(run: &mut PlotRun<'_>, handler: &str) {
    let p = Point::new;
    for a in run.q {
        let base = match handler {
            "xcomb" => p(0.0, a.y),
            "ycomb" => p(a.x, 0.0),
            _ => p(0.0, 0.0),
        };
        run.segs.extend([Segment::MoveTo(base), Segment::LineTo(*a)]);
    }
}

/// A bar from the zero level to each point, `bar width` across.
pub(super) fn bar_plot(e: &mut Engine, run: &mut PlotRun<'_>, ybar: bool) -> Result<(), Error> {
    let p = Point::new;
    let (w, sh) = (e.key_text("/pgf/bar width"), e.key_text("/pgf/bar shift"));
    let (width, shift) = (e.length(&w)?, e.length(&sh)?);
    for a in run.q {
        let (lo, size) = if ybar { (p(a.x - width / 2.0 + shift, 0.0), p(width, a.y)) } else { (p(0.0, a.y - width / 2.0 + shift), p(a.x, width)) };
        run.segs.extend([
            Segment::MoveTo(lo),
            Segment::LineTo(p(lo.x + size.x, lo.y)),
            Segment::LineTo(p(lo.x + size.x, lo.y + size.y)),
            Segment::LineTo(p(lo.x, lo.y + size.y)),
            Segment::Close,
        ]);
    }
    Ok(())
}

/// A bar for each interval between two points, its height the first's.
pub(super) fn bar_interval(e: &mut Engine, run: &mut PlotRun<'_>, ybar: bool) -> Result<(), Error> {
    let p = Point::new;
    let num = |e: &mut Engine, k: &str, d: f64| -> f64 {
        let t = e.key_text(k);
        if t.trim().is_empty() {
            d
        } else {
            e.number(&t).unwrap_or(d)
        }
    };
    let (width, shift) = (num(e, "/pgf/bar interval width", 1.0), num(e, "/pgf/bar interval shift", 0.5));
    for w in run.q.windows(2) {
        let (a, b) = (w[0], w[1]);
        let (lo, size) = if ybar {
            let d = b.x - a.x;
            (p(a.x + shift * d - 0.5 * width * d, 0.0), p(width * d, a.y))
        } else {
            let d = b.y - a.y;
            (p(0.0, a.y + shift * d - 0.5 * width * d), p(a.x, width * d))
        };
        run.segs.extend([
            Segment::MoveTo(lo),
            Segment::LineTo(p(lo.x + size.x, lo.y)),
            Segment::LineTo(p(lo.x + size.x, lo.y + size.y)),
            Segment::LineTo(p(lo.x, lo.y + size.y)),
            Segment::Close,
        ]);
    }
    Ok(())
}

/// Lines from point to point, stopping `gap around stream point` short of each.
pub(super) fn gap_plot(e: &mut Engine, run: &mut PlotRun<'_>, cycle: bool) -> Result<(), Error> {
    let p = Point::new;
    let g = e.key_text("/pgf/gap around stream point");
    let gap = if g.trim().is_empty() { 1.5 * crate::PT } else { e.length(&g)? };
    let mut pts = run.q.to_vec();
    if cycle && run.q.len() > 1 {
        pts.push(run.q[0]);
    }
    for w in pts.windows(2) {
        let d = w[1].minus(w[0]);
        let u = if d.norm() > 1e-12 { d.scale(1.0 / d.norm()) } else { p(0.0, 0.0) };
        run.segs.extend([Segment::MoveTo(w[0].plus(u.scale(gap))), Segment::LineTo(w[1].minus(u.scale(gap)))]);
    }
    Ok(())
}

/// `const plot` and `jump mark`: the step between two points, on the left, the right or the middle.
pub(super) fn const_plot(run: &mut PlotRun<'_>, handler: &str) {
    let p = Point::new;
    let q = run.q;
    run.segs.push(Segment::MoveTo(q[0]));
    for w in q.windows(2) {
        let (a, b) = (w[0], w[1]);
        let mid = 0.5 * a.x + 0.5 * b.x;
        match handler {
            "const plot mark right" => run.segs.extend([Segment::LineTo(p(a.x, b.y)), Segment::LineTo(b)]),
            "const plot mark mid" => run.segs.extend([Segment::LineTo(p(mid, a.y)), Segment::LineTo(p(mid, b.y)), Segment::LineTo(b)]),
            "jump mark left" => run.segs.extend([Segment::LineTo(p(b.x, a.y)), Segment::MoveTo(b)]),
            "jump mark right" => run.segs.extend([Segment::MoveTo(p(a.x, b.y)), Segment::LineTo(b)]),
            "jump mark mid" => run.segs.extend([Segment::LineTo(p(mid, a.y)), Segment::MoveTo(p(mid, b.y)), Segment::LineTo(b)]),
            _ => run.segs.extend([Segment::LineTo(p(b.x, a.y)), Segment::LineTo(b)]),
        }
    }
}

/// The path of a plot's points (canvas coordinates) as its handler makes it, in the plot's own
/// coordinate system (where the zero level of combs and bars is, and bar widths are measured).
pub fn plot_segments(e: &mut Engine, points: &[Point]) -> Result<Vec<Segment>, Error> {
    // an undefined point breaks the plot: each run is plotted on its own
    if points.iter().any(|p| !p.x.is_finite()) {
        let mut out = Vec::new();
        for run in points.split(|p| !p.x.is_finite()).filter(|r| !r.is_empty()) {
            out.extend(plot_segments(e, run)?);
        }
        return Ok(out);
    }
    let t = e.st.transform;
    let inv = t.inverse();
    let q: Vec<Point> = points.iter().map(|p| inv.apply(*p)).collect();
    let n = q.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    let mut v = Vec::new();
    let handler = e.st.plot.plot_handler.clone();
    let k = 0.2775 * e.st.plot.tension;
    {
        let mut run = PlotRun { q: &q, segs: &mut v };
        match handler.as_str() {
            "smooth" if n > 2 => smooth_plot(&mut run, k),
            "smooth cycle" if n > 2 => smooth_cycle(&mut run, k),
            "xcomb" | "ycomb" | "polar comb" => comb_plot(&mut run, handler.as_str()),
            "xbar" | "ybar" => bar_plot(e, &mut run, handler == "ybar")?,
            "ybar interval" | "xbar interval" => bar_interval(e, &mut run, handler == "ybar interval")?,
            "gap lineto" | "gap cycle" => gap_plot(e, &mut run, handler == "gap cycle")?,
            h if h.starts_with("const plot") || h.starts_with("jump mark") => const_plot(&mut run, h),
            _ => {
                run.segs.push(Segment::MoveTo(q[0]));
                run.segs.extend(q[1..].iter().map(|a| Segment::LineTo(*a)));
                if handler == "sharp cycle" {
                    run.segs.push(Segment::Close);
                }
            }
        }
    }
    Ok(crate::canvas::transform_segments(&v, &t))
}
