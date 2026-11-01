use tikra_engine::engine::nodes::{NodeSpec, OnPath};
use tikra_engine::engine::number::NumberFormat;
use tikra_engine::keys as pk;
use tikra_engine::scene::{Point, Segment};
use tikra_engine::tex::{self, Token};
use tikra_engine::{Error, PT};

use crate::keys;
use crate::ticks::{self};
use crate::{toks, Engine};

use super::frame::*;
use super::prepare::*;
use super::util::*;

/// The axes of the libraries that draw their own.
pub(in crate::draw) fn other_axis(e: &mut Engine, s: &Setup) -> Result<(), Error> {
    if s.frame.smith {
        crate::smith::draw_axis(e, s)
    } else {
        crate::ternary::draw_axis(e, s)
    }
}

/// The 2d axis: grids, ticks, lines, tick labels and extra ticks, x and y in turn.
pub(in crate::draw) fn draw_axis_2d(e: &mut Engine, s: &Setup) -> Result<(), Error> {
    let f = s.frame.clone();
    for a in 0..2 {
        if !s.hide[a] {
            grid(e, s, AxisMarks { axis: a, values: &s.ticks[a], extra: "" })?;
        }
    }
    for a in 0..2 {
        if !s.hide[a] {
            tick_lines(e, s, AxisMarks { axis: a, values: &s.ticks[a], extra: "" })?;
        }
    }
    axis_lines(e, s)?;
    for a in 0..2 {
        if !s.hide[a] {
            tick_labels(e, s, TickLabels { axis: a, values: &s.ticks[a], extra: false })?;
        }
    }
    for a in 0..2 {
        if s.hide[a] {
            continue;
        }
        let c = axis_char(a);
        let extra = pk::value(&e.tex, &format!("/pgfplots/extra {c} ticks")).unwrap_or_default();
        if pk::trim(&extra).is_empty() {
            continue;
        }
        let v: Vec<f64> = number_list(e, extra)?.into_iter().map(|x| f.logical(a, x)).collect();
        e.begin_group();
        let r = (|| {
            e.st.key_path = "/pgfplots".into();
            keys::run(e, &format!("every extra {c} tick"))?;
            grid(e, s, AxisMarks { axis: a, values: &v, extra: "extra" })?;
            tick_lines(e, s, AxisMarks { axis: a, values: &v, extra: "extra" })?;
            tick_labels(e, s, TickLabels { axis: a, values: &v, extra: true })
        })();
        e.end_group();
        r?;
    }
    Ok(())
}

/// Ticks of one axis. `extra` is the style suffix, empty for the ordinary ticks.
pub(in crate::draw) struct AxisMarks<'a> {
    axis: usize,
    values: &'a [f64],
    extra: &'a str,
}

/// Grid lines at the ticks of one axis.
pub(in crate::draw) fn grid(e: &mut Engine, s: &Setup, marks: AxisMarks<'_>) -> Result<(), Error> {
    let AxisMarks { axis: a, values: list, extra } = marks;
    let f = &s.frame;
    let c = axis_char(a);
    let b = 1 - a;
    let major = keys::flag(e, &format!("/pgfplots/{c}majorgrids"));
    let minor = keys::flag(e, &format!("/pgfplots/{c}minorgrids")) && extra.is_empty();
    let line = |t: f64| vec![Segment::MoveTo(on_axis(f, a, t, f.min[b])), Segment::LineTo(on_axis(f, a, t, f.max[b]))];
    if minor {
        let segs: Vec<Segment> = minor_ticks(e, s, a, list).into_iter().flat_map(line).collect();
        draw_with(e, &format!("/pgfplots/every axis grid,/pgfplots/every minor grid,/pgfplots/every axis {c} grid,/pgfplots/every minor {c} grid"), segs)?;
    }
    if major {
        let segs: Vec<Segment> = visible(f, a, list).into_iter().flat_map(|(_, t)| line(t)).collect();
        draw_with(e, &format!("/pgfplots/every axis grid,/pgfplots/every major grid,/pgfplots/every axis {c} grid,/pgfplots/every major {c} grid"), segs)?;
    }
    Ok(())
}

/// Minor tick positions: `minor x tick num` between the ticks (9 per decade on log axes), or
/// `minor xtick`.
pub(crate) fn minor_ticks(e: &mut Engine, s: &Setup, a: usize, list: &[f64]) -> Vec<f64> {
    let f = &s.frame;
    let c = axis_char(a);
    if let Some(v) = keys::value_tokens(e, &format!("/pgfplots/minor {c}tick")) {
        return number_list(e, v).unwrap_or_default().into_iter().map(|x| f.logical(a, x)).filter(|t| visible(f, a, &[*t]).len() == 1).collect();
    }
    let mut out = Vec::new();
    if f.log[a] {
        let uniform = list.windows(2).all(|w| ((w[1] - w[0]) - std::f64::consts::LN_10).abs() < 1e-6);
        if !uniform || list.len() < 2 {
            return out;
        }
        for t in &list[..list.len() - 1] {
            for i in 2..=9 {
                out.push(t + (i as f64).ln());
            }
        }
    } else {
        let n = keys::number(e, &format!("/pgfplots/minor {c} tick num")).unwrap_or(0.0) as usize;
        if n == 0 || list.len() < 2 {
            return out;
        }
        let d = s.tick_distance[a];
        let d = if d > 0.0 { d } else { list[1] - list[0] };
        for t in &list[..list.len() - 1] {
            for i in 1..=n {
                out.push(t + i as f64 * d / (n + 1) as f64);
            }
        }
    }
    out.into_iter().filter(|t| visible(f, a, &[*t]).len() == 1).collect()
}

/// The tick marks of axis `a`.
pub(in crate::draw) fn tick_lines(e: &mut Engine, s: &Setup, marks: AxisMarks<'_>) -> Result<(), Error> {
    let AxisMarks { axis: a, values: list, extra } = marks;
    let f = &s.frame;
    let c = axis_char(a);
    let b = 1 - a;
    let pos = pv(e, &format!("{c}tick pos"));
    let align = pv(e, &format!("{c}tick align"));
    let (lower, upper, center) = match pos.as_str() {
        "left" | "bottom" => (true, false, false),
        "right" | "top" => (false, true, false),
        "center" => (true, false, true),
        "none" => (false, false, false),
        _ => (true, true, false),
    };
    let reversed_b = (f.max[b] - f.min[b]) * if f.e[b].x + f.e[b].y < 0.0 { -1.0 } else { 1.0 } < 0.0;
    let (lower, upper) = if reversed_b && !center { (upper, lower) } else { (lower, upper) };
    let ub = unit(f.e[b]).scale(if f.max[b] >= f.min[b] { 1.0 } else { -1.0 });
    let sh = line_shift(e, f, a);
    let mut kinds = Vec::new();
    if keys::flag(e, &format!("/pgfplots/{c}minorticks")) && extra.is_empty() {
        let m = minor_ticks(e, s, a, list);
        if !m.is_empty() {
            let len = keys::length(e, "/pgfplots/minor tick length").unwrap_or(0.1);
            kinds.push(("minor", m, len));
        }
    }
    if keys::flag(e, &format!("/pgfplots/{c}majorticks")) {
        let m: Vec<f64> = visible(f, a, list).into_iter().map(|(_, t)| t).filter(|t| !obscured(e, s, a, *t)).collect();
        let len = keys::length(e, "/pgfplots/major tick length").unwrap_or(0.15);
        kinds.push(("major", m, len));
    }
    for (kind, ticks, len) in kinds {
        let off = match align.as_str() {
            "outside" => len,
            "center" => len / 2.0,
            _ => 0.0,
        };
        let mut segs = Vec::new();
        for t in ticks {
            if lower {
                let bv = if center { f.zero[b] } else { f.min[b] };
                let out = if center { f.normal(a) } else { ub.scale(-1.0) };
                let p = on_axis(f, a, t, bv).plus(out.scale(sh));
                segs.push(Segment::MoveTo(p.plus(ub.scale(-off))));
                segs.push(Segment::LineTo(p.plus(ub.scale(len - off))));
            }
            if upper {
                let p = on_axis(f, a, t, f.max[b]).plus(ub.scale(sh));
                segs.push(Segment::MoveTo(p.plus(ub.scale(off))));
                segs.push(Segment::LineTo(p.plus(ub.scale(off - len))));
            }
        }
        draw_with(e, &format!("/pgfplots/every tick,/pgfplots/every {kind} tick,/pgfplots/every {c} tick,/pgfplots/every {kind} {c} tick"), segs)?;
    }
    Ok(())
}

/// Ticks of a centered axis line that would cross the other axis line are hidden
/// (`hide obscured x ticks`); none when that line is shifted.
pub(in crate::draw) fn obscured(e: &mut Engine, s: &Setup, a: usize, t: f64) -> bool {
    let f = &s.frame;
    if s.line[a] != "center" || f.three_d || line_shift(e, f, 1 - a) != 0.0 {
        return false;
    }
    let tol = 0.05 * PT / f.e[a].norm().max(f64::MIN_POSITIVE);
    let near = |v: f64| (t - v).abs() <= tol;
    let (lo, hi) = (f.min[a].min(f.max[a]), f.min[a].max(f.max[a]));
    match s.line[1 - a].as_str() {
        "box" => near(lo) || near(hi),
        "low" => near(lo),
        "center" => near(f.zero[a]),
        "high" => near(hi),
        _ => false,
    }
}

/// The axis lines: a closed box, or separate lines (with arrows) for `axis lines=…`.
pub(in crate::draw) fn axis_lines(e: &mut Engine, s: &Setup) -> Result<(), Error> {
    let f = &s.frame;
    let shift = [line_shift(e, f, 0), line_shift(e, f, 1)];
    if s.line[0] == "box" && s.line[1] == "box" && !keys::flag(e, "/pgfplots/separate axis lines") && shift == [0.0, 0.0] {
        if s.hide[0] && s.hide[1] {
            return Ok(());
        }
        let p = |x: f64, y: f64| f.at([x, y, f.min[2]]);
        let (x0, x1, y0, y1) = (f.min[0], f.max[0], f.min[1], f.max[1]);
        let mut segs = vec![Segment::MoveTo(p(x0, y0))];
        let mut step = |hidden: bool, q: Point| segs.push(if hidden { Segment::MoveTo(q) } else { Segment::LineTo(q) });
        step(s.hide[1], p(x0, y1));
        step(s.hide[0], p(x1, y1));
        step(s.hide[1], p(x1, y0));
        step(s.hide[0], p(x0, y0));
        if !s.hide[0] && !s.hide[1] {
            segs.push(Segment::Close);
        }
        return draw_with(e, "/pgfplots/every outer x axis line,/pgfplots/every outer y axis line", segs);
    }
    for (a, &sa) in shift.iter().enumerate() {
        if s.hide[a] {
            continue;
        }
        let c = axis_char(a);
        let b = 1 - a;
        let discontinuity = pv(e, &format!("axis {c} discontinuity"));
        // shifted outwards: away from the other axis' middle, along the normal for a centered line
        let ub = unit(f.e[b]).scale(if f.max[b] >= f.min[b] { 1.0 } else { -1.0 });
        let line = |bv: f64| {
            let out = if bv == f.max[b] {
                ub
            } else if bv == f.min[b] {
                ub.scale(-1.0)
            } else {
                f.normal(a)
            };
            let d = out.scale(sa);
            let (p0, p1) = (on_axis(f, a, f.min[a], bv).plus(d), on_axis(f, a, f.max[a], bv).plus(d));
            discontinuous(p0, p1, &discontinuity)
        };
        match s.line[a].as_str() {
            "center" => draw_with(e, &format!("/pgfplots/every inner {c} axis line"), line(f.zero[b]))?,
            "box" => {
                draw_with(e, &format!("/pgfplots/every outer {c} axis line"), line(f.min[b]))?;
                draw_with(e, &format!("/pgfplots/every outer {c} axis line"), line(f.max[b]))?;
            }
            "low" => draw_with(e, &format!("/pgfplots/every outer {c} axis line"), line(if f.label_side[a] > 0 { f.max[b] } else { f.min[b] }))?,
            "high" => draw_with(e, &format!("/pgfplots/every outer {c} axis line"), line(f.max[b]))?,
            _ => {}
        }
    }
    Ok(())
}

/// The text of tick `t` of axis `a`, the `k`-th.
pub(crate) fn tick_text_for(e: &mut Engine, a: usize, t: f64, k: usize) -> Result<Vec<Token>, Error> {
    tick_text(e, &TickText { axis: a, value: t, next: None, index: k, factor: 1.0, extra: false })
}

/// One tick whose label is being typeset.
pub(in crate::draw) struct TickText {
    axis: usize,
    value: f64,
    next: Option<f64>,
    index: usize,
    factor: f64,
    extra: bool,
}

/// The text of one tick.
pub(in crate::draw) fn tick_text(e: &mut Engine, tick: &TickText) -> Result<Vec<Token>, Error> {
    let &TickText { axis: a, value: t, next, index: k, factor, extra } = tick;
    let f = crate::fig(e).frame.clone().expect("drawing an axis");
    let c = axis_char(a);
    // groupplots' inner plots: the labels are there, empty
    if !extra && keys::flag(e, &format!("/pgfplots/@empty {c}ticklabels")) {
        return Ok(Vec::new());
    }
    let labels_key = if extra { format!("/pgfplots/extra {c} tick labels") } else { format!("/pgfplots/{c}ticklabels") };
    if let Some(list) = pk::value(&e.tex, &labels_key).filter(|l| !pk::trim(l).is_empty()) {
        let items = split_labels(&list);
        return Ok(items.get(k).cloned().unwrap_or_default());
    }
    let shown = |v: f64| if f.log[a] { v } else { v * factor };
    let label_key = if extra { format!("/pgfplots/extra {c} tick label") } else { format!("/pgfplots/{c}ticklabel") };
    if let Some(code) = keys::value_tokens(&*e, &label_key) {
        e.begin_group();
        let value = if f.log[a] { t } else { shown(t) };
        let tick = tikra_engine::engine::pgf_number(value);
        e.tex.set_meaning(&Token::cs("tick"), tikra_engine::engine::macro_from(&tick), false);
        e.tex.set_meaning(&Token::cs("ticknum"), tikra_engine::engine::macro_from(&k.to_string()), false);
        if crate::dateplot::is_date(e, c) {
            crate::dateplot::define_tick(e, value);
        }
        if let Some(n) = next {
            e.tex.set_meaning(&Token::cs("nexttick"), tikra_engine::engine::macro_from(&tikra_engine::engine::pgf_number(shown(n))), false);
        }
        let r = e.tex.expand_fully(code).and_then(|x| crate::print_numbers(e, &x));
        e.end_group();
        return r;
    }
    if f.log[a] {
        let basis = keys::number(e, &format!("/pgfplots/log basis {c}"));
        let (base_text, exp) = match basis {
            Some(b) if b > 0.0 && b != 10.0 => (tikra_engine::engine::pgf_number(b), t / b.ln()),
            _ => ("10".to_string(), t / std::f64::consts::LN_10),
        };
        let fmt = NumberFormat { style: tikra_engine::engine::number::Style::Fixed, precision: 2, ..e.st.number_format.clone() };
        // `log identify minor tick positions` (extra ticks): 5·10^-6 rather than 10^-5.3
        if base_text == "10" && keys::flag(e, "/pgfplots/log identify minor tick positions") {
            let j = exp.floor();
            let m = 10f64.powf(exp - j);
            let i = m.round();
            if (exp - exp.round()).abs() > 1e-4 && (2.0..=9.0).contains(&i) && (m - i).abs() < 1e-3 * i {
                let sci = NumberFormat { style: tikra_engine::engine::number::Style::Sci, ..e.st.number_format.clone() };
                return Ok(tex::string_tokens(&format!("${}$", sci.print(i * 10f64.powf(j)))));
            }
        }
        if keys::flag(e, "/pgfplots/@log fixed point") {
            let v = (exp * (base_text.parse::<f64>().unwrap_or(10.0)).ln()).exp();
            let ff = NumberFormat { style: tikra_engine::engine::number::Style::FixedRelative, precision: 3, ..e.st.number_format.clone() };
            return Ok(tex::string_tokens(&format!("${}$", ff.print(v))));
        }
        return Ok(tex::string_tokens(&format!("${base_text}^{{{}}}$", fmt.print(exp))));
    }
    if let Some(name) = crate::symbolic::label(e, c, t) {
        return Ok(tikra_engine::engine::tikra_tokens(&name));
    }
    // `scaled y ticks=manual:{label}{code}`: the code maps the tick (#1) to \pgfmathresult
    if let Some((_, code)) = manual_scale(e, c) {
        let body = tex::substitute(&tex::parse_body(code), &[toks(&tikra_engine::engine::pgf_number(t))]);
        e.begin_group();
        let r = e.run_tokens(body).and_then(|_| e.tex.expand_to_string(vec![Token::cs("pgfmathresult")]));
        let v = r.ok().and_then(|s| e.number(s.trim()).ok());
        e.end_group();
        if let Some(v) = v {
            return Ok(tex::string_tokens(&format!("${}$", e.st.number_format.print(v))));
        }
    }
    Ok(tex::string_tokens(&format!("${}$", e.st.number_format.print(shown(t)))))
}

/// `scaled x ticks=manual:{label}{code}`: the label and the code.
pub(in crate::draw) fn manual_scale(e: &Engine, c: char) -> Option<(Vec<Token>, Vec<Token>)> {
    let v = pk::value(&e.tex, &format!("/pgfplots/scaled {c} ticks"))?;
    let text = tex::tokens_to_string(&v);
    let at = text.find("manual:")?;
    let _ = at;
    // the two groups after `manual:`
    let start = v.iter().position(|t| t.is_char(':'))? + 1;
    let mut groups = Vec::new();
    let mut depth = 0usize;
    let mut cur = Vec::new();
    for t in &v[start..] {
        match t {
            Token::Char(_, tex::Cat::Begin) => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
            }
            Token::Char(_, tex::Cat::End) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    groups.push(std::mem::take(&mut cur));
                    continue;
                }
            }
            _ => {}
        }
        if depth > 0 {
            cur.push(t.clone());
        }
    }
    let mut it = groups.into_iter();
    Some((it.next()?, it.next()?))
}

pub(in crate::draw) fn split_labels(list: &[Token]) -> Vec<Vec<Token>> {
    let mut out = vec![Vec::new()];
    let mut depth = 0usize;
    for t in list {
        match t {
            Token::Char(_, tikra_engine::tex::Cat::Begin) => depth += 1,
            Token::Char(_, tikra_engine::tex::Cat::End) => depth = depth.saturating_sub(1),
            Token::Char(',', _) if depth == 0 => {
                out.push(Vec::new());
                continue;
            }
            _ => {}
        }
        if let Some(l) = out.last_mut() {
            l.push(t.clone());
        }
    }
    out.into_iter().map(|v| tex::strip_outer_braces(pk::trim(&v).to_vec())).collect()
}

/// The tick labels of axis `a`, and the scale label (`\cdot 10^{4}`).
/// Tick labels of one axis. `extra` selects the extra-tick list.
pub(in crate::draw) struct TickLabels<'a> {
    axis: usize,
    values: &'a [f64],
    extra: bool,
}

pub(in crate::draw) fn tick_labels(e: &mut Engine, s: &Setup, labels: TickLabels<'_>) -> Result<(), Error> {
    let TickLabels { axis: a, values: list, extra } = labels;
    let f = s.frame.clone();
    let c = axis_char(a);
    if !keys::flag(e, &format!("/pgfplots/{c}majorticks")) {
        return Ok(());
    }
    let b = 1 - a;
    let off = tick_offset(e, a);
    let n = f.normal(a);
    let side = f.label_side[a];
    let bv = if side > 0 {
        f.max[b]
    } else if side == 0 {
        f.zero[b]
    } else {
        f.min[b]
    };
    let sh = n.scale(line_shift(e, &f, a));
    tick_labels_at(e, s, a, list, extra, &|t| on_axis(&f, a, t, bv).plus(sh), n.scale(off), off)
}

/// The factor a tick label is divided by, and the scale label that says so.
pub(in crate::draw) struct TickScale {
    factor: f64,
    label: Option<String>,
}

pub(in crate::draw) struct ScaleAsk {
    axis: usize,
    extra: bool,
}

/// The custom scale-label code, or the default text.
pub(in crate::draw) struct ScaleCode<'a> {
    custom: bool,
    code: &'a str,
    arg: &'a str,
}

pub(in crate::draw) fn scale_label_text(e: &mut Engine, spec: ScaleCode<'_>, default: String) -> String {
    if !spec.custom {
        return default;
    }
    let mut call = vec![Token::cs(spec.code), Token::Char('{', tex::Cat::Begin)];
    call.extend(toks(spec.arg));
    call.push(Token::Char('}', tex::Cat::End));
    e.tex.expand_fully(call).map(|t| tex::tokens_to_string(&t)).unwrap_or(default)
}

/// `scaled ticks`: the common factor, and the label that names it.
pub(in crate::draw) fn tick_scale(e: &mut Engine, f: &Frame, ask: ScaleAsk) -> TickScale {
    let mut factor = 1.0;
    let mut scale_label: Option<String> = None;
    let c = axis_char(ask.axis);
    if !f.log[ask.axis] && !ask.extra {
        let v = pv(e, &format!("scaled {c} ticks"));
        let above = keys::number(e, "/pgfplots/scale ticks above exponent").unwrap_or(3.0) as i32;
        let below = keys::number(e, "/pgfplots/scale ticks below exponent").unwrap_or(-1.0) as i32;
        let exponent = match v.as_str() {
            "true" => Some(ticks::scale_exponent(f.min[ask.axis], f.max[ask.axis], above, below)),
            "false" => None,
            other => other.strip_prefix("base 10:").and_then(|n| n.trim().parse().ok()),
        };
        let binop = keys::value_tokens(e, "/pgfplots/tick scale binop").map_or_else(|| "\\cdot".to_string(), |t| tex::tokens_to_string(&t));
        // `xtick scale label code/.code={…}`: the label, given the exponent (or the factor)
        let code = pk::macro_name(&format!("/pgfplots/{c}tick scale label code/.@cmd"));
        let custom = matches!(e.tex.meaning_of(&code), tex::Meaning::Macro(_));
        if let Some((label, _)) = manual_scale(e, c) {
            scale_label = Some(tex::tokens_to_string(&label));
        }
        if let Some(x) = exponent.filter(|x| *x != 0) {
            factor = 10f64.powi(x);
            let arg = (-x).to_string();
            scale_label = Some(scale_label_text(e, ScaleCode { custom, code: &code, arg: &arg }, format!("${binop} 10^{{{}}}$", -x)));
        }
        if let Some(r) = v.strip_prefix("real:") {
            if let Ok(d) = e.number(r) {
                if d != 0.0 {
                    factor = 1.0 / d;
                    let default = format!("${binop} {}$", e.st.number_format.print(d));
                    let arg = tikra_engine::engine::pgf_number(d);
                    scale_label = Some(scale_label_text(e, ScaleCode { custom, code: &code, arg: &arg }, default));
                }
            }
        }
    }
    TickScale { factor, label: scale_label }
}

/// Where one tick label sits, before it is shifted off the axis line.
pub(in crate::draw) struct LabelSite {
    index: usize,
    value: f64,
    next: Option<f64>,
    at: Point,
}

pub(in crate::draw) struct SiteQuery<'a> {
    setup: &'a Setup,
    axis: usize,
    list: &'a [f64],
    interval: bool,
    base: &'a dyn Fn(f64) -> Point,
    visible: &'a [(usize, f64)],
}

/// The tick positions that receive a label (`tick label as interval`, or each visible tick).
pub(in crate::draw) fn label_sites(e: &mut Engine, q: &SiteQuery<'_>) -> Vec<LabelSite> {
    let mut entries = Vec::new();
    if q.interval {
        for w in q.visible.windows(2) {
            let (k, t) = w[0];
            let (_, t2) = w[1];
            entries.push(LabelSite { index: k, value: t, next: Some(t2), at: (q.base)((t + t2) / 2.0) });
        }
    } else {
        for &(k, t) in q.visible {
            if obscured(e, q.setup, q.axis, t) {
                continue;
            }
            let next = q.list.get(k + 1).copied();
            entries.push(LabelSite { index: k, value: t, next, at: (q.base)(t) });
        }
    }
    entries
}

/// One label after the shift off the axis line. `on_line` is where the distance is measured.
pub(in crate::draw) struct PlacedTick {
    axis: usize,
    index: usize,
    value: f64,
    next: Option<f64>,
    at: Point,
    on_line: Point,
    factor: f64,
    extra: bool,
}

pub(in crate::draw) struct LabelPaint<'a> {
    frame: &'a Frame,
    anchor: &'a str,
    styles: &'a str,
    normal: Point,
    dim: &'a mut f64,
}

/// Typesets one tick label and remembers how far it reaches.
pub(in crate::draw) fn draw_placed_tick(e: &mut Engine, paint: &mut LabelPaint<'_>, placed: PlacedTick) -> Result<(), Error> {
    let f = paint.frame;
    // the number format of the tick label styles applies to the text
    e.begin_group();
    let r = (|| {
        keys_tikz(e, paint.styles)?;
        let (value, next) = if f.ternary {
            (crate::ternary::shown(f, placed.axis, placed.value), placed.next.map(|n| crate::ternary::shown(f, placed.axis, n)))
        } else {
            (placed.value, placed.next)
        };
        tick_text(e, &TickText { axis: placed.axis, value, next, index: placed.index, factor: placed.factor, extra: placed.extra })
    })();
    e.end_group();
    let mut text = r?;
    // `typeset ticklabels with strut`: every label as high and deep as a strut
    if keys::flag(e, "/pgfplots/@ticklabel strut") {
        let mut strut = toks("\\vrule width 0pt height 0.84em depth 0.36em");
        strut.extend(text);
        text = strut;
    }
    let options = toks(&format!("anchor={},{}", paint.anchor, paint.styles));
    // `\ticknum` in the label's options too (`name=tick no \ticknum`)
    e.begin_group();
    e.tex.set_meaning(&Token::cs("ticknum"), tikra_engine::engine::macro_from(&placed.index.to_string()), false);
    let at = placed.at;
    let r = capture(e, |e| e.node(NodeSpec { options, name: None, at: Some(at), text, place: OnPath::Point(at), point: at, coordinate: false, default_pos: None }));
    e.end_group();
    let (items, _, bbox) = r?;
    if let Some((lo, hi)) = bbox {
        let n = paint.normal;
        let p = placed.on_line;
        let corner = Point::new(if n.x > 0.0 { hi.x } else { lo.x }, if n.y > 0.0 { hi.y } else { lo.y });
        let d = (corner.x - p.x) * n.x + (corner.y - p.y) * n.y;
        let dim = (*paint.dim).max(d);
        *paint.dim = dim;
        e.canvas.include(lo);
        e.canvas.include(hi);
    }
    e.canvas.items.extend(items);
    Ok(())
}

pub(in crate::draw) struct ScaleCaption<'a> {
    axis: usize,
    text: &'a str,
}

/// The scale label, in description coordinates, at the origin of that system.
pub(in crate::draw) fn draw_scale_caption(e: &mut Engine, f: &Frame, caption: ScaleCaption<'_>) -> Result<(), Error> {
    let c = axis_char(caption.axis);
    let text = tex::string_tokens(caption.text);
    let options = toks(&format!("/pgfplots/every tick label,/pgfplots/every {c} tick label,/pgfplots/every {c} tick scale label"));
    f.cs.set(Cs::Description);
    let p = f.desc(0.0, 0.0);
    let items = e.node(NodeSpec { options, name: None, at: None, text, place: OnPath::Point(p), point: p, coordinate: false, default_pos: None });
    f.cs.set(Cs::Plain);
    e.canvas.items.extend(items?);
    Ok(())
}

/// The tick labels of axis `a` at `base(t)` moved by `shift`, and the scale label; `off` is how
/// far the labels are from the axis line at least.
#[allow(clippy::too_many_arguments)]
pub(crate) fn tick_labels_at(e: &mut Engine, s: &Setup, a: usize, list: &[f64], extra: bool, base: &dyn Fn(f64) -> Point, shift: Point, off: f64) -> Result<(), Error> {
    let f = s.frame.clone();
    let scale = tick_scale(e, &f, ScaleAsk { axis: a, extra });
    let c = axis_char(a);
    let interval = keys::flag(e, &format!("/pgfplots/{c} tick label as interval"));
    let n = f.normal(a);
    let anchor = format!("near {c}ticklabel");
    let styles = format!("/pgfplots/every tick label,/pgfplots/every {c} tick label");
    let mut dim = 0.0f64;
    let vis = visible(&f, a, list);
    let sites = label_sites(e, &SiteQuery { setup: s, axis: a, list, interval, base, visible: &vis });
    f.label_axis.set(a);
    for site in sites {
        let placed = PlacedTick { axis: a, index: site.index, value: site.value, next: site.next, at: site.at.plus(shift), on_line: site.at, factor: scale.factor, extra };
        draw_placed_tick(e, &mut LabelPaint { frame: &f, anchor: &anchor, styles: &styles, normal: n, dim: &mut dim }, placed)?;
    }
    if !extra {
        let mut td = f.tick_dim.borrow_mut();
        td[a] = td[a].max(dim.max(off));
    }
    if let Some(label) = scale.label.filter(|_| !vis.is_empty() && !keys::flag(e, &format!("/pgfplots/@{c} scale label off"))) {
        draw_scale_caption(e, &f, ScaleCaption { axis: a, text: &label })?;
    }
    Ok(())
}
