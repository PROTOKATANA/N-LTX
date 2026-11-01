use tikra_engine::canvas::{Fill, Item, Paint};
use tikra_engine::keys as pk;
use tikra_engine::scene::{Point, Segment, Transform};
use tikra_engine::tex::{self, Token};
use tikra_engine::{Error, PT};

use crate::keys;
use crate::ticks::{self};
use crate::{toks, Engine};

use super::frame::*;

/// Items and the bounding box they grew.
pub(in crate::draw) type Captured<T> = (T, Vec<Item>, Option<(Point, Point)>);

pub(crate) fn axis_char(a: usize) -> char {
    ['x', 'y', 'z'][a]
}

/// `enlarge x limits=…`: (lower, upper, relative amount, absolute amount).
pub(in crate::draw) fn enlarge_setting(v: &str, auto_min: bool, auto_max: bool) -> (bool, bool, f64, Option<f64>) {
    let v = v.trim();
    let mut rel = 0.1;
    let mut abs = None;
    let (mut lo, mut hi) = (false, false);
    for part in v.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        match part {
            "auto" => {
                lo = auto_min;
                hi = auto_max;
            }
            "true" => {
                lo = true;
                hi = true;
            }
            "false" => {
                lo = false;
                hi = false;
            }
            "upper" => {
                lo = false;
                hi = true;
            }
            "lower" => {
                lo = true;
                hi = false;
            }
            other => {
                let (k, val) = other.split_once('=').map(|(a, b)| (a.trim(), b.trim())).unwrap_or(("value", other));
                match k {
                    "abs" | "abs value" => {
                        abs = val.parse().ok();
                        if k == "abs" {
                            lo = true;
                            hi = true;
                        }
                    }
                    "rel" | "value" => {
                        rel = val.parse().unwrap_or(0.1);
                        if k == "rel" || !v.contains("upper") && !v.contains("lower") {
                            lo = true;
                            hi = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    (lo, hi, rel, abs)
}

/// A value of the axis (`/pgfplots/<key>`), expanded.
pub(crate) fn pv(e: &mut Engine, key: &str) -> String {
    keys::value(e, &format!("/pgfplots/{key}"))
}

/// Items pushed and the bounding box grown while `f` runs, taken out of the canvas.
pub(in crate::draw) fn capture<T>(e: &mut Engine, f: impl FnOnce(&mut Engine) -> Result<T, Error>) -> Result<Captured<T>, Error> {
    let start = e.canvas.items.len();
    let (min, max) = (e.canvas.min.take(), e.canvas.max.take());
    let r = f(e);
    let bbox = e.canvas.min.zip(e.canvas.max);
    e.canvas.min = min;
    e.canvas.max = max;
    let items: Vec<Item> = e.canvas.items.drain(start..).collect();
    Ok((r?, items, bbox))
}

/// Runs a key list on a path or node, with `/pgfplots` styles reachable.
pub(crate) fn keys_tikz(e: &mut Engine, list: &str) -> Result<(), Error> {
    e.process_keys(toks(list), Some("/tikz"))
}

/// Draws a path with the options `list` (as `\draw[list] …;`).
pub(crate) fn draw_with(e: &mut Engine, list: &str, segs: Vec<Segment>) -> Result<(), Error> {
    if segs.is_empty() {
        return Ok(());
    }
    e.begin_group();
    e.st.mode = Default::default();
    e.st.pending = Transform::default();
    let r = (|| {
        e.st.mode.draw = true;
        keys_tikz(e, list)?;
        e.st.apply_pending();
        e.use_path(segs, "axis")
    })();
    e.end_group();
    r
}

pub(in crate::draw) fn bbox_size(points: &[Point]) -> (f64, f64) {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
    for p in points {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    (x1 - x0, y1 - y0)
}

/// A list of tick values (`1,2,...,10`, `{0,0.5}`), in data units.
pub(crate) fn number_list(e: &mut Engine, tokens: Vec<Token>) -> Result<Vec<f64>, Error> {
    let list = e.tex.expand_fully(tokens)?;
    let mut v = Vec::new();
    for item in tikra_engine::engine::foreach_items(&list) {
        if let Ok(x) = e.number_of(item) {
            if x.is_finite() {
                v.push(x);
            }
        }
    }
    Ok(v)
}

/// The view's azimuth and elevation, in degrees (`trig format=rad` gives them in radians).
pub(crate) fn view_angles(e: &mut Engine) -> (f64, f64) {
    let az = keys::number(e, "/pgfplots/view/az").unwrap_or(25.0);
    let el = keys::number(e, "/pgfplots/view/el").unwrap_or(30.0);
    if e.st.trig_rad {
        (az.to_degrees(), el.to_degrees())
    } else {
        (az, el)
    }
}

/// Whether a coordinate's part has a unit (`1cm`, `-2pt`).
pub(in crate::draw) fn has_unit(part: &str) -> bool {
    let t = part.trim().trim_end_matches('}');
    ["cm", "mm", "pt", "in", "em", "ex", "bp", "pc"].iter().any(|u| t.ends_with(u))
}

pub(in crate::draw) fn text_is_empty(s: &str) -> bool {
    s.trim().is_empty()
}

/// The ticks of axis `a` that are drawn: (index in the list, logical value).
pub(crate) fn visible(f: &Frame, a: usize, list: &[f64]) -> Vec<(usize, f64)> {
    let (lo, hi) = (f.min[a].min(f.max[a]), f.min[a].max(f.max[a]));
    let tol = 0.05 * PT / f.e[a].norm().max(f64::MIN_POSITIVE);
    list.iter().copied().enumerate().filter(|(_, t)| *t >= lo - tol && *t <= hi + tol).collect()
}

/// The point of logical value `t` on axis `a`, with the other 2d axis at `b_value`.
pub(in crate::draw) fn on_axis(f: &Frame, a: usize, t: f64, b_value: f64) -> Point {
    let b = 1 - a;
    let mut l = f.min;
    l[a] = t;
    l[b] = b_value;
    f.at(l)
}

/// A style's code, if it does something.
pub(in crate::draw) struct Meaning(pub(in crate::draw) Vec<Token>);

pub(in crate::draw) fn style_code(e: &Engine, path: &str) -> Option<Meaning> {
    match e.tex.meaning_of(&pk::macro_name(&format!("{path}/.@cmd"))) {
        tikra_engine::tex::Meaning::Macro(m) => {
            let body = tex::substitute(&m.body, &[vec![]]);
            let text = tex::tokens_to_string(&body);
            let inner = text.replace("\\pgfkeysalso", "").replace(['{', '}', ' '], "");
            if inner.is_empty() {
                None
            } else {
                Some(Meaning(body))
            }
        }
        _ => None,
    }
}

pub(in crate::draw) fn box_path(f: &Frame) -> Vec<Segment> {
    let p = |x: f64, y: f64| f.at([x, y, f.min[2]]);
    vec![
        Segment::MoveTo(p(f.min[0], f.min[1])),
        Segment::LineTo(p(f.max[0], f.min[1])),
        Segment::LineTo(p(f.max[0], f.max[1])),
        Segment::LineTo(p(f.min[0], f.max[1])),
        Segment::Close,
    ]
}

/// The outline of a 3d box's projection (the clip path).
pub(in crate::draw) fn hull_path(f: &Frame) -> Vec<Segment> {
    let mut pts = Vec::new();
    for i in 0..8 {
        let l = [if i & 1 == 0 { f.min[0] } else { f.max[0] }, if i & 2 == 0 { f.min[1] } else { f.max[1] }, if i & 4 == 0 { f.min[2] } else { f.max[2] }];
        pts.push(f.at(l));
    }
    let hull = convex_hull(pts);
    let mut v = Vec::new();
    for (i, p) in hull.iter().enumerate() {
        v.push(if i == 0 { Segment::MoveTo(*p) } else { Segment::LineTo(*p) });
    }
    v.push(Segment::Close);
    v
}

pub(in crate::draw) fn convex_hull(mut pts: Vec<Point>) -> Vec<Point> {
    pts.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
    let cross = |o: Point, a: Point, b: Point| (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x);
    let mut lower: Vec<Point> = Vec::new();
    for p in &pts {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 1e-12 {
            lower.pop();
        }
        lower.push(*p);
    }
    let mut upper: Vec<Point> = Vec::new();
    for p in pts.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 1e-12 {
            upper.pop();
        }
        upper.push(*p);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

/// The data scale transformation of axis `a` (`\pgfplots@set@optimal@datatrafo@for@`): `10^E` and
/// the smallest data value, which goes to 0; (1, 0) for a logarithmic axis or `disabledatascaling`.
/// Before compat 1.11, plain coordinates in an axis are in its units.
pub(crate) fn data_scale_trafo(e: &mut Engine, f: &Frame, a: usize) -> (f64, f64) {
    let [lo, hi] = f.data_range[a];
    if f.log[a] || keys::flag(e, "/pgfplots/@disabledatascaling") || !(lo.is_finite() && hi.is_finite()) {
        return (1.0, 0.0);
    }
    let size = keys::length(e, if a == 0 { "/pgfplots/width" } else { "/pgfplots/height" }).unwrap_or(if a == 0 { 240.0 * PT } else { 207.0 * PT }) / PT;
    (ticks::data_scale(lo, hi, size), lo)
}

/// An axis line from `p0` to `p1`, with `axis x discontinuity` near its start: a zigzag of 12pt
/// (`crunch`) or a 4pt gap between two bars (`parallel`), 4pt from the start.
pub(in crate::draw) fn discontinuous(p0: Point, p1: Point, kind: &str) -> Vec<Segment> {
    let u = unit(p1.minus(p0));
    let n = Point::new(-u.y, u.x);
    let at = |s: f64, h: f64| p0.plus(u.scale(s * PT)).plus(n.scale(h * PT));
    match kind {
        "crunch" => vec![
            Segment::MoveTo(p0),
            Segment::LineTo(at(4.0, 0.0)),
            Segment::LineTo(at(7.0, 4.0)),
            Segment::LineTo(at(13.0, -4.0)),
            Segment::LineTo(at(16.0, 0.0)),
            Segment::LineTo(p1),
        ],
        "parallel" => vec![
            Segment::MoveTo(p0),
            Segment::LineTo(at(4.0, 0.0)),
            Segment::MoveTo(at(4.0, -8.0)),
            Segment::LineTo(at(4.0, 8.0)),
            Segment::MoveTo(at(8.0, -8.0)),
            Segment::LineTo(at(8.0, 8.0)),
            Segment::MoveTo(at(8.0, 0.0)),
            Segment::LineTo(p1),
        ],
        _ => vec![Segment::MoveTo(p0), Segment::LineTo(p1)],
    }
}

/// `axis x line shift=10pt`: how far axis `a`'s line, ticks and tick labels move outwards; a
/// number without a unit is in units of the other axis (`\pgfplots@init@axis@shift`).
pub(crate) fn line_shift(e: &mut Engine, f: &Frame, a: usize) -> f64 {
    let v = keys::value(e, &format!("/pgfplots/axis {} line shift", axis_char(a)));
    let v = v.trim();
    if v.is_empty() {
        return 0.0;
    }
    if let Ok(n) = v.parse::<f64>() {
        return if f.three_d { 0.0 } else { n * f.e[1 - a].norm() };
    }
    e.length(v).unwrap_or(0.0)
}

/// How far tick labels move out for the tick lines (`tick align`).
pub(crate) fn tick_offset(e: &mut Engine, a: usize) -> f64 {
    let align = pv(e, &format!("{}tick align", axis_char(a)));
    let len = keys::length(e, "/pgfplots/major tick length").unwrap_or(0.15);
    match align.as_str() {
        "outside" => len,
        "center" => len / 2.0,
        _ => 0.0,
    }
}

/// A filled or stroked path for a solid color (used by legend frames).
pub fn solid(color: tikra_engine::color::Color, opacity: f64) -> Fill {
    Fill { paint: Paint::Solid(color), opacity, even_odd: false }
}

/// Runs `f` with TikZ's unit vectors those of the data scale transformation (so that a radius of
/// 1 is one of its units), when plain coordinates are its coordinates.
pub(in crate::draw) fn in_plain_units<T>(e: &mut Engine, f: &Frame, run: impl FnOnce(&mut Engine) -> Result<T, Error>) -> Result<T, Error> {
    let units = (e.st.x, e.st.y, e.st.z);
    if f.cs.get() == Cs::Plain && !f.polar && !f.smith && !f.ternary {
        let inverse = e.st.transform.inverse();
        let v: Vec<Point> = (0..3).map(|a| inverse.linear(f.e[a].scale(1.0 / data_scale_trafo(e, f, a).0))).collect();
        (e.st.x, e.st.y) = (v[0], v[1]);
        if f.three_d {
            e.st.z = v[2];
        }
    }
    let r = run(e);
    (e.st.x, e.st.y, e.st.z) = units;
    r
}
