//! Axes, ticks, grids and axis labels, drawn after the data, and the tick placement strategies: `step=`,
//! `about=` (`few`, `some`, `many`) with the about strategies, linear and exponential steps, and
//! `minor steps between steps`.
//!
//! Positions come from the axes' objects, as pgf computes them: an attribute is set (`goto`), the
//! data point is mapped and transformed, and the point (pt, the picture's transformation not
//! applied) is where the line, tick or label goes. A tick's direction is that of its direction
//! axis between its minimum and its maximum; the anchor of its text is the one TikZ's `auto` would
//! give for that direction.

use tikra_engine::keys::macro_name;
use tikra_engine::scene::{Point, Transform};
use tikra_engine::tex::{self, Cat, Meaning, Token};
use tikra_engine::{Error, PT};

use super::objects::Step;
use super::{group, key_text, macro_text, num, parse_value, set_attribute, PathToken, Signal};
use tikra_engine::engine::{macro_tokens, tikra_tokens, Engine};

/// A point as TikZ coordinates in pt (the picture's transformation applies to it).
pub fn pt(p: Point) -> String {
    format!("({}pt,{}pt)", num(p.x), num(p.y))
}

/// The anchors TikZ's `auto` gives a node beside a direction: on its left, and on its right.
fn auto_anchors(d: Point) -> (&'static str, &'static str) {
    let (x, y) = (d.x, d.y);
    let left = if x > 0.05 {
        if y > 0.05 {
            "south east"
        } else if y < -0.05 {
            "south west"
        } else {
            "south"
        }
    } else if x < -0.05 {
        if y > 0.05 {
            "north east"
        } else if y < -0.05 {
            "north west"
        } else {
            "north"
        }
    } else if y > 0.0 {
        "east"
    } else {
        "west"
    };
    let right = if x > 0.05 {
        if y > 0.05 {
            "north west"
        } else if y < -0.05 {
            "north east"
        } else {
            "north"
        }
    } else if x < -0.05 {
        if y > 0.05 {
            "south west"
        } else if y < -0.05 {
            "south east"
        } else {
            "south"
        }
    } else if y > 0.0 {
        "west"
    } else {
        "east"
    };
    (left, right)
}

fn normalized(p: Point) -> Point {
    let n = (p.x * p.x + p.y * p.y).sqrt();
    if n < 1e-12 {
        Point::new(0.0, 0.0)
    } else {
        p.scale(1.0 / n)
    }
}

/// `before creation` of an axis' scaling mapper: `scaling` (or its default) with its `at`
/// values evaluated.
pub fn prepare_scaling(e: &mut Engine, axis: &str) -> Result<(), Error> {
    let mut spec = key_text(e, &format!("{axis}/scaling")).unwrap_or_default();
    if spec.is_empty() {
        spec = key_text(e, &format!("{axis}/scaling/default")).unwrap_or_default();
    }
    let (first, second) = spec.split_once(" and ").unwrap_or((&spec, ""));
    let (a, c) = first.split_once(" at").unwrap_or((first, "0"));
    let (b, d) = second.split_once(" at").unwrap_or((second, "1"));
    let eval = |e: &mut Engine, s: &str| -> String {
        let s = s.trim();
        let s = s.strip_prefix('{').and_then(|r| r.strip_suffix('}')).unwrap_or(s);
        match tikra_engine::expr::eval_length(&e.math_string(tex::string_tokens(s)).unwrap_or_default(), &e.vars()) {
            Ok((v, true)) => num(v / PT),
            Ok((v, false)) => num(v),
            Err(_) => s.to_string(),
        }
    };
    let (c, d) = (eval(e, c), eval(e, d));
    let v = format!("{} at {c} and {} at {d}", a.trim(), b.trim());
    e.tex.define_string(&macro_name(&format!("{axis}/scaling")), &v, false);
    Ok(())
}

/// `goto=value` for an axis: its attribute set to the value, `min`, `max`, `padded min` or
/// `padded max` (with the padding as an offset of the scaled attribute).
pub fn goto(e: &mut Engine, axis: &str, value: &str) -> Result<(), Error> {
    let Some(mapper) = crate::object_of(e, &format!("{axis}/scaling mapper")) else { return Ok(()) };
    match value.trim() {
        v @ ("padded min" | "padded max") => {
            let end = &v[7..];
            crate::objects::set_in_to(e, mapper, end);
            let out = mapper_output(e, mapper);
            let padding = key_text(e, &format!("{axis}/padding {end}")).unwrap_or_default();
            let p = if padding.trim().is_empty() { 0.0 } else { e.length(&padding)? / PT };
            set_attribute(e, &format!("{out}/offset"), &num(p));
        }
        v => crate::objects::set_in_to(e, mapper, v),
    }
    Ok(())
}

pub(crate) fn mapper_output(e: &mut Engine, mapper: usize) -> String {
    match crate::running(e).last().and_then(|d| d.objects.get(mapper)) {
        Some(super::Object::ScalingMapper(m)) => m.output.clone(),
        _ => String::new(),
    }
}

/// `\tikzpointandanchordirection`: the direction of the axis (from its minimum to its maximum)
/// at the current data point, and the anchors for texts below (min) and above (max) it.
pub(crate) fn direction(e: &mut Engine, mapper: usize) -> Result<(Point, &'static str, &'static str), Error> {
    let mut ends = [Point::new(0.0, 0.0); 2];
    for (i, end) in ["min", "max"].iter().enumerate() {
        e.begin_group();
        crate::objects::set_in_to(e, mapper, end);
        let token = if i == 0 { PathToken::DirectionFrom } else { PathToken::DirectionTo };
        let r = crate::dv_generic(e, Signal::Direction, token);
        e.end_group();
        if let Step::Point(p) = r? {
            ends[i] = p;
        }
    }
    let d = match crate::dv_generic(e, Signal::Direction, PathToken::DirectionAt)? {
        Step::Direction(d) => d,
        _ => normalized(ends[1].minus(ends[0])),
    };
    // the anchors look at the direction turned by -90 degrees
    let (max, min) = auto_anchors(Point::new(d.y, -d.x));
    e.tex.define_string("tikra@dv@maxanchor", max, true);
    e.tex.define_string("tikra@dv@minanchor", min, true);
    Ok((d, max, min))
}

/// `anchor at min`, `anchor at max`: the node anchor for a text at that end of the axis.
pub fn anchor_at(e: &mut Engine, axis: &str, which: &str) -> Result<(), Error> {
    let Some(mapper) = crate::object_of(e, &format!("{axis}/scaling mapper")) else { return Ok(()) };
    let (_, max, min) = direction(e, mapper)?;
    let a = if which == "min" { min } else { max };
    e.keys_command(tikra_tokens(&format!("/tikz/anchor={a}")), None, false)
}

/// A path step of a data visualization path (`\pgfpathdvmoveto`, `\pgfpathdvlineto`).
pub(crate) fn path_step(e: &mut Engine, token: PathToken) -> Result<Step, Error> {
    crate::dv_generic(e, Signal::Path, token)
}

/// Steps as a TikZ path (pt): moves, lines, and the arcs of polar axes as Bézier curves.
fn path_text(steps: &[(PathToken, Step)]) -> String {
    let mut out = String::new();
    let mut current = Point::new(0.0, 0.0);
    for (token, step) in steps {
        match step {
            Step::Point(p) | Step::Direction(p) => {
                if *token == PathToken::LineTo && !out.is_empty() {
                    out.push_str(" -- ");
                } else {
                    out.push(' ');
                }
                out.push_str(&pt(*p));
                current = *p;
            }
            Step::Arc { from, to, a, b } => {
                let e = |t: f64| {
                    let (s, c) = t.to_radians().sin_cos();
                    a.scale(c).plus(b.scale(s))
                };
                let d = |t: f64| {
                    let (s, c) = t.to_radians().sin_cos();
                    b.scale(c).minus(a.scale(s))
                };
                let center = current.minus(e(*from));
                let n = ((to - from).abs() / 90.0).ceil().max(1.0) as usize;
                let delta = (to - from) / n as f64;
                for k in 0..n {
                    let t1 = from + delta * k as f64;
                    let t2 = t1 + delta;
                    let kk = 4.0 / 3.0 * (delta.to_radians() / 4.0).tan();
                    let c1 = center.plus(e(t1)).plus(d(t1).scale(kk));
                    let c2 = center.plus(e(t2)).minus(d(t2).scale(kk));
                    let p2 = center.plus(e(t2));
                    out.push_str(&format!(" .. controls {} and {} .. {}", pt(c1), pt(c2), pt(p2)));
                    current = p2;
                }
            }
        }
    }
    out
}

/// The low and high ends along the axis: a move and a line, each `goto` in a group.
pub(crate) fn axis_steps(e: &mut Engine, axis: &str, low: &str, high: &str) -> Result<Vec<(PathToken, Step)>, Error> {
    let mut steps = Vec::new();
    for (token, value) in [(PathToken::MoveTo, low), (PathToken::LineTo, high)] {
        e.begin_group();
        let r = goto(e, axis, value).and_then(|_| path_step(e, token));
        e.end_group();
        steps.push((token, r?));
    }
    Ok(steps)
}

/// An axis line, from `low` to `high`.
pub fn show_axis(e: &mut Engine, axis: &str, options: Vec<Token>) -> Result<(), Error> {
    let mut open = tikra_tokens("\\begin{scope}[data visualization/axis layer,/tikz/data visualization/.cd,low=min,high=max,every axis/.try,");
    open.extend(options);
    open.extend(tikra_tokens("]"));
    e.run_tokens(open)?;
    let r = (|| {
        if let Some(mapper) = crate::object_of(e, &format!("{axis}/scaling mapper")) {
            direction(e, mapper)?;
        }
        let low = key_text(e, "/tikz/data visualization/low").unwrap_or_default();
        let high = key_text(e, "/tikz/data visualization/high").unwrap_or_default();
        let steps = axis_steps(e, axis, &low, &high)?;
        let path = path_text(&steps);
        e.run_tokens(tikra_tokens(&format!("\\path[draw,/tikz/data visualization/styling]{path};")))
    })();
    let close = e.run_tokens(tikra_tokens("\\end{scope}"));
    r.and(close)
}

/// The positions of an axis' major, minor and
/// subminor ticks (or grid lines), from the placement strategy, in `axis/default … at`.
pub fn default_action(e: &mut Engine, what: &str, axis: &str) -> Result<(), Error> {
    let mut marshal = Vec::new();
    e.begin_group();
    let r = (|| {
        e.tex.set_meaning(&Token::cs("tikra@dv@ticks"), macro_tokens(Vec::new()), false);
        let default_at = tikra_engine::keys::value(&e.tex, &format!("{axis}/ticks at")).unwrap_or_default();
        e.tex.set_meaning(&Token::cs("tikra@dv@defaultticks"), macro_tokens(default_at), false);
        let settings = tikra_engine::keys::value(&e.tex, &format!("{axis}/{what} at")).unwrap_or_default();
        if let Some(v) = tikra_engine::keys::value(&e.tex, &format!("{axis}/scaling mapper")) {
            e.tex.set_meaning(&Token::cs("tikra@dv@mapper"), macro_tokens(v), false);
        }
        // the settings' own `major=`… do not count: only what the strategy computes
        crate::dv(e)?.setters_off = true;
        let r = dv_keys(e, settings);
        crate::dv(e)?.setters_off = false;
        r?;
        let step = tikra_engine::keys::value(&e.tex, "/tikz/data visualization/@step").unwrap_or_default();
        if !tikra_engine::keys::trim(&step).is_empty() {
            // linear or exponential steps, or the code of one's own strategy
            let strategy = tikra_engine::keys::value(&e.tex, &format!("{axis}/tick placement strategy")).unwrap_or_default();
            match tex::tokens_to_string(&strategy).trim() {
                "linear" | "" => compute_at(e, false)?,
                "exponential" => compute_at(e, true)?,
                _ => e.run_tokens(strategy)?,
            }
            for k in ["major", "minor", "subminor"] {
                if !marshal.is_empty() {
                    marshal.push(Token::other(','));
                }
                marshal.extend(tikra_tokens(&format!("{k}=")));
                marshal.extend(group(tikra_engine::keys::value(&e.tex, &format!("/tikz/data visualization/{k}")).unwrap_or_default()));
            }
        }
        Ok(())
    })();
    e.end_group();
    e.tex.set_meaning(&Token::cs(&macro_name(&format!("{axis}/default {what} at"))), macro_tokens(marshal), false);
    r
}

/// Keys in `/tikz/data visualization` (`\tikzdatavisualizationset`).
pub(crate) fn dv_keys(e: &mut Engine, v: Vec<Token>) -> Result<(), Error> {
    e.keys_command(v, Some("/tikz/data visualization"), true)
}

/// `\tikra@dv@pi{x}`: the text of an angle in radians as a multiple of π, to the nearest twelfth
/// (`0`, `\pi`, `-\pi`, or the fraction then `\pi`).
pub fn pi_typesetter(e: &mut Engine) -> Result<(), Error> {
    let x = e.argument_string()?;
    let x = parse_value(e, &x).unwrap_or(0.0);
    let k = (x * 12.0 / std::f64::consts::PI).round() / 12.0;
    let text = if k.abs() < 1e-4 {
        "$0$".to_string()
    } else if (k - 1.0).abs() < 1e-4 {
        "$\\pi$".to_string()
    } else if (k + 1.0).abs() < 1e-4 {
        "$-\\pi$".to_string()
    } else {
        format!("\\pgfmathprintnumber{{{}}}$\\pi$", num(k))
    };
    e.tex.push_tokens(tikra_tokens(&text));
    Ok(())
}

/// `about=n`: a step that gives about n ticks over the range, its first
/// digit chosen by the about strategy (or 1, 3, 6 for exponential steps).
pub fn compute_about(e: &mut Engine, n: &str) -> Result<(), Error> {
    let Some(mapper) = crate::object_in(e, "tikra@dv@mapper") else { return Ok(()) };
    let (Some(mut lo), Some(mut hi)) = crate::objects::range_of(e, mapper) else { return Ok(()) };
    let log = crate::dv(e)?.log_steps;
    if log {
        lo = lo.log10();
        hi = hi.log10();
    }
    let about = parse_value(e, n).unwrap_or(5.0);
    let around = fpu_divide(hi - lo, about);
    if around == 0.0 || !around.is_finite() {
        e.tex.define_string("tikra@dv@step", "0", false);
        return Ok(());
    }
    let exp = around.abs().log10().floor() as i32;
    let m = around.abs() / 10f64.powi(exp);
    // TeX's dimensions keep five decimals of the mantissa
    let m = (m * 65536.0).round() / 65536.0;
    let step = if log {
        if exp < 0 {
            1.0
        } else if m < 2.5 {
            10f64.powi(exp)
        } else if m < 5.1 {
            3.0 * 10f64.powi(exp)
        } else {
            6.0 * 10f64.powi(exp)
        }
    } else {
        let strategy = key_text(e, "/tikz/data visualization/about strategy").unwrap_or_default();
        let mut x = m;
        for pair in strategy.split(',') {
            if let Some((t, r)) = pair.split_once('/') {
                if let (Ok(t), Ok(r)) = (t.trim().parse::<f64>(), r.trim().parse::<f64>()) {
                    if x < t {
                        x = r;
                        break;
                    }
                }
            }
        }
        if x < 10.0 {
            x * 10f64.powi(exp)
        } else {
            x / 10.0 * 10f64.powi(exp + 1)
        }
    };
    e.tex.define_string("tikra@dv@step", &num(step), false);
    Ok(())
}

/// Linear and exponential steps: the major positions from the step and the
/// phase, and the minor ones between them.
pub fn compute_at(e: &mut Engine, log: bool) -> Result<(), Error> {
    crate::dv(e)?.log_steps = log;
    e.tex.set_meaning(&Token::cs("tikra@dv@step"), Meaning::Prim(tex::Prim::Relax), false);
    let code = tikra_engine::keys::value(&e.tex, "/tikz/data visualization/@step").unwrap_or_default();
    e.run_tokens(code)?;
    if matches!(e.tex.meaning_of("tikra@dv@step"), Meaning::Prim(tex::Prim::Relax) | Meaning::Undefined) {
        return Ok(());
    }
    let step_text = macro_text(e, "tikra@dv@step");
    let Some(mapper) = crate::object_in(e, "tikra@dv@mapper") else { return Ok(()) };
    let (Some(lo), Some(hi)) = crate::objects::range_of(e, mapper) else { return Ok(()) };
    let phase = key_text(e, "/tikz/data visualization/phase").unwrap_or_default();
    let phase = parse_value(e, &phase).unwrap_or(0.0);
    let step = parse_value(e, &step_text).unwrap_or(0.0);
    let substeps = key_text(e, "/tikz/data visualization/minor steps between steps").filter(|s| !s.is_empty());
    let substeps = match substeps {
        Some(s) => Some(e.number(&s)?.trunc() as i64 + 1),
        None => None,
    };
    let mut majors = Vec::new();
    let mut minors = Vec::new();
    let linear_function =
        !matches!(crate::running(e).last().and_then(|d| d.objects.get(mapper)), Some(super::Object::ScalingMapper(m)) if m.function != super::objects::Function::Identity);
    if !log {
        if step <= 0.0 {
            dv_keys(e, tikra_tokens(&format!("major/.expanded={{at={{{}}}}}", num(lo))))?;
            return Ok(());
        }
        let mut current = ((lo - phase) / step).floor() * step + phase;
        let epsilon = if linear_function { (hi - lo) * 0.001 } else { 0.0 };
        let (min_eps, max_eps) = (lo - epsilon, hi + epsilon);
        let mut n = 0;
        loop {
            if let Some(k) = substeps {
                let sub = step / k as f64;
                let mut s = current;
                for _ in 0..k {
                    s += sub;
                    if s >= lo && s < hi {
                        minors.push(s);
                    }
                }
            }
            if current >= min_eps {
                majors.push(current);
            }
            current += step;
            n += 1;
            if current >= max_eps || n > 1000 {
                break;
            }
        }
    } else {
        let (lo_l, hi_l) = (lo.log10(), hi.log10());
        if step <= 0.0 || !lo_l.is_finite() || !hi_l.is_finite() {
            return Ok(());
        }
        let mut current = ((lo_l - phase) / step).floor() * step + phase;
        let epsilon = (hi_l - lo_l) * 0.001;
        let (min_eps, max_eps) = (lo_l - epsilon, hi_l + epsilon);
        let mut n = 0;
        loop {
            if let Some(k) = substeps {
                let a = pow10(current);
                let b = pow10(current + step);
                let sub = (b - a) / k as f64;
                let mut s = a;
                for _ in 0..k {
                    s += sub;
                    if s >= lo && s < hi {
                        minors.push(s);
                    }
                }
            }
            if current >= min_eps {
                majors.push(pow10(current));
            }
            current += step;
            n += 1;
            if current >= max_eps || n > 1000 {
                break;
            }
        }
    }
    let list = |v: &[f64]| v.iter().map(|x| format!(",{}", num(*x))).collect::<String>();
    let keys = format!("major/.expanded={{at={{{}}}}},minor/.expanded={{at={{{}}}}}", list(&majors), list(&minors));
    dv_keys(e, tikra_tokens(&keys))
}

/// The ticks of one kind (`major`…) of an axis, and their texts.
pub fn show_ticks(e: &mut Engine, kind: &str, axis: &str, options: Vec<Token>) -> Result<(), Error> {
    let settings = at_settings(e, axis, "ticks");
    if tikra_engine::keys::trim(&settings).is_empty() {
        return Ok(());
    }
    e.begin_group();
    let r = show_ticks_in_group(e, TickGroup { kind, axis, options, settings });
    e.end_group();
    r
}

/// `axis/default … at` and `axis/… at`, the settings for the ticks or the grid.
pub(crate) fn at_settings(e: &mut Engine, axis: &str, what: &str) -> Vec<Token> {
    let default = tikra_engine::keys::value(&e.tex, &format!("{axis}/default {what} at")).unwrap_or_default();
    let own = tikra_engine::keys::value(&e.tex, &format!("{axis}/{what} at")).unwrap_or_default();
    if tikra_engine::keys::trim(&default).is_empty() {
        own
    } else {
        let mut v = default;
        v.push(Token::other(','));
        v.extend(own);
        v
    }
}

/// One kind of ticks of one axis, with the options of the command and the `at` settings.
struct TickGroup<'a> {
    kind: &'a str,
    axis: &'a str,
    options: Vec<Token>,
    settings: Vec<Token>,
}

fn show_ticks_in_group(e: &mut Engine, group: TickGroup) -> Result<(), Error> {
    let TickGroup { kind, axis, options, settings } = group;
    e.tex.set_meaning(&Token::cs("tikra@dv@at"), macro_tokens(Vec::new()), false);
    e.tex.set_meaning(&Token::cs("tikra@dv@atoptions"), macro_tokens(Vec::new()), false);
    let Some(mapper) = crate::object_of(e, &format!("{axis}/scaling mapper")) else { return Ok(()) };
    let mut open = tikra_tokens(&format!("\\begin{{scope}}[/tikz/data visualization/.cd,every ticks/.try,every {kind} ticks/.try,"));
    open.extend(options);
    open.extend(tikra_tokens("]"));
    e.run_tokens(open)?;
    let r = (|| {
        dv_keys(e, settings)?;
        let common = tikra_engine::keys::value(&e.tex, "/tikz/data visualization/common").unwrap_or_default();
        dv_keys(e, common)?;
        let own = tikra_engine::keys::value(&e.tex, &format!("/tikz/data visualization/{kind}")).unwrap_or_default();
        dv_keys(e, own)?;
        let direction_axis = key_text(e, "/tikz/data visualization/direction axis").unwrap_or_default();
        let dir_mapper = crate::object_of(e, &format!("/tikz/data visualization/{direction_axis}/scaling mapper"));
        let list = at_list(e);
        let styles = style_at_list(e);
        for (count, item) in list.into_iter().enumerate() {
            e.begin_group();
            let r = one_tick(e, TickDraw { mapper, dir_mapper, item: &item, count: count + 1, styles: &styles });
            e.end_group();
            r?;
        }
        Ok(())
    })();
    let close = e.run_tokens(tikra_tokens("\\end{scope}"));
    r.and(close)
}

/// The tick positions (`at=`), empty ones left out.
pub(crate) fn at_list(e: &mut Engine) -> Vec<Vec<Token>> {
    let v = match e.tex.meaning_of("tikra@dv@at") {
        Meaning::Macro(m) => tex::substitute(&m.body, &[]),
        _ => Vec::new(),
    };
    let v = e.tex.expand_fully(v).unwrap_or_default();
    split_items(&v).into_iter().filter(|i| !tikra_engine::keys::trim(i).is_empty()).collect()
}

/// `options at=value as [options]`: the values and their options.
pub(crate) fn style_at_list(e: &mut Engine) -> Vec<(f64, Vec<Token>)> {
    let v = match e.tex.meaning_of("tikra@dv@atoptions") {
        Meaning::Macro(m) => tex::substitute(&m.body, &[]),
        _ => Vec::new(),
    };
    let mut out = Vec::new();
    for item in split_items(&v) {
        let (pos, opts, _) = parse_tick(&item);
        let s = e.tex.expand_to_string(pos).unwrap_or_default();
        if let Some(x) = parse_value(e, &s) {
            out.push((x, opts));
        }
    }
    out
}

/// One tick: its axis, the axis it is perpendicular to, its `at` item, its index and the `options at` styles.
struct TickDraw<'a> {
    mapper: usize,
    dir_mapper: Option<usize>,
    item: &'a [Token],
    count: usize,
    styles: &'a [(f64, Vec<Token>)],
}

fn one_tick(e: &mut Engine, tick: TickDraw) -> Result<(), Error> {
    let TickDraw { mapper, dir_mapper, item, count, styles } = tick;
    let (pos, opts, text) = parse_tick(item);
    dv_keys(e, opts)?;
    let pos_text = e.tex.expand_to_string(pos)?;
    crate::objects::set_in_to(e, mapper, &pos_text);
    let origin = crate::local_data_point(e)?;
    let (dir, max_anchor, min_anchor) = match dir_mapper {
        Some(m) => direction(e, m)?,
        None => (Point::new(0.0, 1.0), "south", "north"),
    };
    // the options of `options at` for this value
    if let Some(x) = parse_value(e, &pos_text) {
        let mut mine = Vec::new();
        for (v, o) in styles {
            if (v - x).abs() <= 1e-9 * v.abs().max(x.abs()).max(1e-300) || (*v == 0.0 && x.abs() < 1e-12) {
                mine.push(Token::other(','));
                mine.extend(o.clone());
            }
        }
        if !mine.is_empty() {
            dv_keys(e, mine)?;
        }
    }
    let parity = if count % 2 == 1 { "odd" } else { "even" };
    let high = key_text(e, "/tikz/data visualization/high").unwrap_or_default();
    let low = key_text(e, "/tikz/data visualization/low").unwrap_or_default();
    let at_low = matches!(e.tex.meaning_of("iftikra@dv@low"), Meaning::Prim(tex::Prim::Iftrue));
    let at_high = matches!(e.tex.meaning_of("iftikra@dv@high"), Meaning::Prim(tex::Prim::Iftrue));
    let mut max_val = e.length(&high)? / PT;
    let mut min_val = e.length(&low)? / PT;
    if at_high {
        let p = key_text(e, &format!("/tikz/data visualization/tick text high {parity} padding")).unwrap_or_default();
        max_val += e.length(&p)? / PT;
    }
    if at_low {
        let p = key_text(e, &format!("/tikz/data visualization/tick text low {parity} padding")).unwrap_or_default();
        min_val += e.length(&p)? / PT;
    }
    let max_tick = origin.plus(dir.scale(max_val));
    let min_tick = origin.plus(dir.scale(min_val));
    let every = format!("\\tikzdatavisualizationset{{every {parity} tick/.try}}");
    e.run_tokens(tikra_tokens(&format!(
        "\\begin{{scope}}[data visualization/ticks layer]{every}\\draw [/tikz/data visualization/styling] {} -- {};\\end{{scope}}",
        pt(min_tick),
        pt(max_tick)
    )))?;
    let text = match text {
        Some(t) => t,
        None => {
            if at_low || at_high {
                tick_text(e, &pos_text)?
            } else {
                Vec::new()
            }
        }
    };
    if tikra_engine::keys::trim(&text).is_empty() {
        return Ok(());
    }
    for (on, at, anchor) in [(at_low, min_tick, min_anchor), (at_high, max_tick, max_anchor)] {
        if !on {
            continue;
        }
        let mut v = tikra_tokens(&format!(
            "\\begin{{scope}}[data visualization/ticks node layer]{every}\\path [/tikz/data visualization/styling] {} node[anchor={anchor},/tikz/data visualization/node styling]",
            pt(at)
        ));
        v.extend(group(text.clone()));
        v.extend(tikra_tokens(";\\end{scope}"));
        e.run_tokens(v)?;
    }
    Ok(())
}

/// A tick's text: its value with the tick prefix, the typesetter and the suffix.
pub(crate) fn tick_text(e: &mut Engine, pos: &str) -> Result<Vec<Token>, Error> {
    let v = parse_value(e, pos).unwrap_or(0.0);
    let mut code = tikra_engine::keys::value(&e.tex, "/tikz/data visualization/tick prefix").unwrap_or_default();
    // the typesetter's code with the value, run here so that what it prints is kept
    match e.tex.meaning_of(&macro_name("/tikz/data visualization/tick typesetter/.@cmd")) {
        Meaning::Macro(m) => code.extend(tex::substitute(&m.body, &[tex::string_tokens(&num(v))])),
        _ => code.extend(tikra_tokens(&format!("\\pgfmathprintnumber{{{}}}", num(v)))),
    }
    code.extend(tikra_engine::keys::value(&e.tex, "/tikz/data visualization/tick suffix").unwrap_or_default());
    e.tex.define_string("tikra@dv@tickvalue", pos, false);
    // TeX prints the number inside the node: its style (a number format) applies
    e.begin_group();
    let r =
        e.keys_command(tikra_tokens("/tikz/data visualization/styling,/tikz/data visualization/node styling"), Some("/tikz"), true).and_then(|_| crate::numbers::typeset(e, code));
    e.end_group();
    r
}

/// The grid lines of one kind, across the direction axis.
pub fn show_grid(e: &mut Engine, kind: &str, axis: &str, options: Vec<Token>) -> Result<(), Error> {
    let settings = at_settings(e, axis, "grid");
    if tikra_engine::keys::trim(&settings).is_empty() {
        return Ok(());
    }
    e.begin_group();
    let r = (|| {
        e.tex.set_meaning(&Token::cs("tikra@dv@at"), macro_tokens(Vec::new()), false);
        e.tex.set_meaning(&Token::cs("tikra@dv@atoptions"), macro_tokens(Vec::new()), false);
        e.tex.set_meaning(&Token::cs("tikra@dv@defaultticks"), macro_tokens(Vec::new()), false);
        let Some(mapper) = crate::object_of(e, &format!("{axis}/scaling mapper")) else { return Ok(()) };
        let mut open = tikra_tokens(&format!("\\begin{{scope}}[data visualization/grid layer,/tikz/data visualization/.cd,every {kind} grid/.try,every grid,"));
        open.extend(options);
        open.extend(tikra_tokens("]"));
        e.run_tokens(open)?;
        let r = (|| {
            let ticks_at = tikra_engine::keys::value(&e.tex, &format!("{axis}/ticks at")).unwrap_or_default();
            e.tex.set_meaning(&Token::cs("tikra@dv@ticks"), macro_tokens(ticks_at), false);
            dv_keys(e, settings)?;
            let common = tikra_engine::keys::value(&e.tex, "/tikz/data visualization/common").unwrap_or_default();
            dv_keys(e, common)?;
            let own = tikra_engine::keys::value(&e.tex, &format!("/tikz/data visualization/{kind}")).unwrap_or_default();
            dv_keys(e, own)?;
            let direction_axis = key_text(e, "/tikz/data visualization/direction axis").unwrap_or_default();
            let dir_axis = format!("/tikz/data visualization/{direction_axis}");
            for item in at_list(e) {
                e.begin_group();
                let r = (|| {
                    let (pos, opts, _) = parse_tick(&item);
                    dv_keys(e, opts)?;
                    let pos = e.tex.expand_to_string(pos)?;
                    crate::objects::set_in_to(e, mapper, &pos);
                    let low = key_text(e, "/tikz/data visualization/low").unwrap_or_default();
                    let high = key_text(e, "/tikz/data visualization/high").unwrap_or_default();
                    let mut steps = Vec::new();
                    goto(e, &dir_axis, &low)?;
                    steps.push((PathToken::MoveTo, path_step(e, PathToken::MoveTo)?));
                    goto(e, &dir_axis, &high)?;
                    steps.push((PathToken::LineTo, path_step(e, PathToken::LineTo)?));
                    let path = path_text(&steps);
                    e.run_tokens(tikra_tokens(&format!("\\draw[/tikz/data visualization/styling]{path};")))
                })();
                e.end_group();
                r?;
            }
            Ok(())
        })();
        let close = e.run_tokens(tikra_tokens("\\end{scope}"));
        r.and(close)
    })();
    e.end_group();
    r
}

/// An axis' label: a node at the data point the
/// options go to.
pub fn show_label(e: &mut Engine, axis: &str, options: Vec<Token>) -> Result<(), Error> {
    let label = tikra_engine::keys::value(&e.tex, &format!("{axis}/label")).unwrap_or_default();
    if tikra_engine::keys::trim(&label).is_empty() {
        return Ok(());
    }
    crate::update_bounding_box(e, false);
    let mut open = tikra_tokens("\\begin{scope}[anchor=center,/tikz/data visualization/.cd,every axis label/.try,");
    open.extend(options);
    open.extend(tikra_tokens("]"));
    e.run_tokens(open)?;
    let r = (|| {
        let label = tikra_engine::keys::trim(&label).to_vec();
        let (opts, text) = match label.first() {
            Some(t) if t.is_char('[') => {
                let mut depth = 0usize;
                let mut end = label.len();
                for (i, t) in label.iter().enumerate().skip(1) {
                    match t {
                        Token::Char(_, Cat::Begin) => depth += 1,
                        Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
                        x if depth == 0 && x.is_char(']') => {
                            end = i;
                            break;
                        }
                        _ => {}
                    }
                }
                (label[1..end].to_vec(), label[(end + 1).min(label.len())..].to_vec())
            }
            _ => (Vec::new(), label.clone()),
        };
        dv_keys(e, opts)?;
        let p = crate::local_data_point(e)?;
        e.st.transform = e.st.transform.compose(&Transform::translation(p.x * PT, p.y * PT));
        let mut v = tikra_tokens("\\node [/tikz/data visualization/node styling]");
        v.extend(group(text));
        v.extend(tikra_tokens(";"));
        e.run_tokens(v)
    })();
    let close = e.run_tokens(tikra_tokens("\\end{scope}"));
    r.and(close)
}

fn pow10(x: f64) -> f64 {
    // whole exponents exactly, as \pgfdvmathpowten does
    if (x - x.round()).abs() <= 0.0002 {
        10f64.powi(x.round() as i32)
    } else {
        10f64.powf(x)
    }
}

/// Items at top-level commas, one pair of braces removed from each.
fn split_items(v: &[Token]) -> Vec<Vec<Token>> {
    let mut out = vec![Vec::new()];
    let mut depth = 0usize;
    for t in v {
        match t {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
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
    out.into_iter().map(|i| tex::strip_outer_braces(tikra_engine::keys::trim(&i).to_vec())).collect()
}

/// `value as [options] text`: the value, the options, and the text if given.
fn parse_tick(item: &[Token]) -> (Vec<Token>, Vec<Token>, Option<Vec<Token>>) {
    // `as`, at the top level
    let mut depth = 0usize;
    let mut at = None;
    for i in 0..item.len() {
        match &item[i] {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
            Token::Char('a', _) if depth == 0 && matches!(item.get(i + 1), Some(Token::Char('s', _))) => {
                at = Some(i);
                break;
            }
            _ => {}
        }
    }
    let Some(i) = at else { return (item.to_vec(), Vec::new(), None) };
    let pos = item[..i].to_vec();
    let mut rest = tikra_engine::keys::trim(&item[i + 2..]).to_vec();
    let mut opts = Vec::new();
    if rest.first().is_some_and(|t| t.is_char('[')) {
        let mut depth = 0usize;
        let mut end = rest.len();
        for (k, t) in rest.iter().enumerate().skip(1) {
            match t {
                Token::Char(_, Cat::Begin) => depth += 1,
                Token::Char(_, Cat::End) => depth = depth.saturating_sub(1),
                x if depth == 0 && x.is_char(']') => {
                    end = k;
                    break;
                }
                _ => {}
            }
        }
        // delimited by `]`, TeX drops a pair of braces around all of it
        opts = tex::strip_outer_braces(rest[1..end].to_vec());
        rest = tikra_engine::keys::trim(&rest[(end + 1).min(rest.len())..]).to_vec();
    }
    let text = if rest.is_empty() { None } else { Some(rest) };
    (pos, opts, text)
}

/// `a / b` as pgf's FPU divides: the quotient of the two mantissas kept to five decimals, so
/// 1.99998 / 5 is 0.4 (and a step of 0.5, not 0.25, for a sine sampled to TeX's precision).
fn fpu_divide(a: f64, b: f64) -> f64 {
    if a == 0.0 || b == 0.0 || !a.is_finite() || !b.is_finite() {
        return a / b;
    }
    let (ea, eb) = (a.abs().log10().floor() as i32, b.abs().log10().floor() as i32);
    let q = (a / 10f64.powi(ea)) / (b / 10f64.powi(eb));
    (q * 1e5).round() / 1e5 * 10f64.powi(ea - eb)
}
