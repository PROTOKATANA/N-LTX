use tikra_engine::keys as pk;
use tikra_engine::tex::{self, Token};
use tikra_engine::{Error, PT};

use crate::keys;
use crate::ticks::{self, TickSettings};
use crate::{Axis, Engine, Entry};

use super::frame::*;
use super::util::*;

/// Ticks kept by `xtickmin` and `xtickmax`.
pub(in crate::draw) struct TickWindow<'a> {
    pub(in crate::draw) frame: &'a Frame,
    pub(in crate::draw) axis: usize,
    pub(in crate::draw) plot: &'a Axis,
}

pub(in crate::draw) fn clipped_ticks(e: &mut Engine, window: &TickWindow<'_>) -> Result<(Vec<f64>, f64), Error> {
    let frame = window.frame;
    let a = window.axis;
    let plot = window.plot;
    let (mut t, d) = tick_list(e, a, frame, plot)?;
    let c = axis_char(a);
    let tol = 1e-9 * (frame.max[a] - frame.min[a]).abs().max(1e-300);
    if let Some(lo) = keys::number(e, &format!("/pgfplots/{c}tickmin")) {
        let lo = frame.logical(a, lo);
        t.retain(|v| *v >= lo - tol);
    }
    if let Some(hi) = keys::number(e, &format!("/pgfplots/{c}tickmax")) {
        let hi = frame.logical(a, hi);
        t.retain(|v| *v <= hi + tol);
    }
    Ok((t, d))
}

/// `hide x axis`, and an axis of no length on paper.
pub(in crate::draw) fn hidden_axes(e: &mut Engine, frame: &Frame, dims: usize) -> [bool; 3] {
    let mut hide = [keys::flag(e, "/pgfplots/hide x axis"), keys::flag(e, "/pgfplots/hide y axis"), keys::flag(e, "/pgfplots/hide z axis")];
    // an axis of no length on paper (z at view {0}{90}) is hidden (`\pgfplots@hide@ztrue`)
    for (a, h) in hide.iter_mut().enumerate() {
        if a < dims && frame.e[a].norm() * (frame.max[a] - frame.min[a]).abs() < 1e-9 {
            *h = true;
        }
    }
    hide
}
/// The styles that pgfplots' `descriptions` family holds, and the keys that only set them.
pub(in crate::draw) const DESCRIPTION_KEYS: &[&str] = &[
    "every axis label",
    "every axis x label",
    "every axis y label",
    "every axis z label",
    "every axis title",
    "every x tick scale label",
    "every y tick scale label",
    "every z tick scale label",
    "label style",
    "xlabel style",
    "ylabel style",
    "zlabel style",
    "title style",
    "x tick scale label style",
    "y tick scale label style",
    "z tick scale label style",
    "xlabel near ticks",
    "ylabel near ticks",
    "xlabel absolute",
    "ylabel absolute",
    // their label styles
    "axis lines",
    "axis x line",
    "axis y line",
    "axis z line",
];

/// `\pgfplots@BEGIN@prepare@description@styles`: once an axis is known to be 3d it gets
/// `every 3d description` (and `every 3d view {az}{el}`); the description styles of `every axis`
/// and of the axis' options are then set again, so that they still win.
pub(in crate::draw) fn three_d_descriptions(e: &mut Engine, axis: &Axis) -> Result<(), Error> {
    let az = pv(e, "view/az");
    let el = pv(e, "view/el");
    let saved = e.st.key_path.clone();
    e.st.key_path = "/pgfplots".into();
    let r = (|| {
        keys::run(e, "every 3d description")?;
        keys::run(e, &format!("every 3d view {{{az}}}{{{el}}}/.try"))?;
        let mut lists = Vec::new();
        if let Some(Meaning(body)) = style_code(e, "/pgfplots/every axis") {
            lists.push(body);
        }
        lists.push(axis.options.clone());
        for list in lists {
            let kept = description_items(&list);
            if !kept.is_empty() {
                e.process_keys(kept, Some("/pgfplots"))?;
            }
        }
        Ok(())
    })();
    e.st.key_path = saved;
    r
}

/// The items of a key list (or of a style's `\pgfkeysalso{…}`s) that set description styles.
pub(in crate::draw) fn description_items(list: &[Token]) -> Vec<Token> {
    let mut lists = Vec::new();
    let mut i = 0;
    let mut plain = Vec::new();
    while i < list.len() {
        if list[i].is_cs("pgfkeysalso") {
            let (arg, end) = crate::group_at(list, i + 1);
            lists.push(arg);
            i = end;
            continue;
        }
        plain.push(list[i].clone());
        i += 1;
    }
    lists.push(plain);
    let mut out = Vec::new();
    for l in lists {
        for item in pk::split_list(&l) {
            let path = item.key.trim_start_matches("/pgfplots/");
            let name = pk::handler(path).map_or(path, |(base, _)| base);
            if !DESCRIPTION_KEYS.contains(&name.trim()) {
                continue;
            }
            if !out.is_empty() {
                out.extend(tex::string_tokens(","));
            }
            out.extend(tex::string_tokens(&item.key));
            if let Some(v) = item.value {
                out.extend(tex::string_tokens("="));
                out.push(Token::Char('{', tex::Cat::Begin));
                out.extend(v);
                out.push(Token::Char('}', tex::Cat::End));
            }
        }
    }
    out
}

/// The point meta range of the axis' plots (`point meta min/max` if given).
pub(in crate::draw) fn meta_range(e: &mut Engine, axis: &Axis) -> [f64; 2] {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for en in &axis.entries {
        if let Entry::Plot(p) = en {
            if !p.has_meta {
                continue;
            }
            for c in &p.coords {
                if c.meta.is_finite() {
                    lo = lo.min(c.meta);
                    hi = hi.max(c.meta);
                }
            }
        }
    }
    if let Some(v) = keys::number(e, "/pgfplots/point meta min") {
        lo = v;
    }
    if let Some(v) = keys::number(e, "/pgfplots/point meta max") {
        hi = v;
    }
    if !(lo.is_finite() && hi.is_finite()) {
        return [0.0, 1.0];
    }
    if hi - lo < 1e-12 {
        return [lo - 0.5, hi + 0.5];
    }
    [lo, hi]
}

/// The current colormap (`colormap name`).
pub fn colormap(e: &mut Engine) -> Option<crate::colormap::Colormap> {
    let name = keys::value(e, "/pgfplots/colormap name");
    let m = colormap_named(e, &name);
    if m.is_none() {
        e.warn(format!("colormap '{name}' not defined"));
    }
    m
}

/// A colormap by name, with the current `colormap access`.
pub fn colormap_named(e: &mut Engine, name: &str) -> Option<crate::colormap::Colormap> {
    let spec = keys::value(e, &format!("/pgfplots/@colormaps/{name}"));
    let mut m = crate::colormap::Colormap::parse(&spec, &e.colors);
    if let Some(m) = m.as_mut() {
        m.access = match keys::value(e, "/pgfplots/colormap access").trim() {
            "piecewise constant" | "const" => crate::colormap::Access::Const,
            "direct" => crate::colormap::Access::Direct,
            _ => crate::colormap::Access::Map,
        };
    }
    m
}

/// The tick candidates of axis `a` (logical units) and their distance.
pub(in crate::draw) fn tick_list(e: &mut Engine, a: usize, f: &Frame, axis: &Axis) -> Result<(Vec<f64>, f64), Error> {
    let c = axis_char(a);
    let raw = pk::value(&e.tex, &format!("/pgfplots/{c}tick")).unwrap_or_default();
    let trimmed = pk::trim(&raw).to_vec();
    if trimmed.len() == 1 && trimmed[0].is_cs("empty") {
        return Ok((Vec::new(), 0.0));
    }
    // `xtickten={1,2}` on a log axis: ticks at 10^1 and 10^2
    if f.log[a] && text_is_empty(&e.tex.expand_to_string(trimmed.clone())?) {
        if let Some(list) = pk::value(&e.tex, &format!("/pgfplots/{c}tickten")).filter(|l| !pk::trim(l).is_empty()) {
            let v = number_list(e, list)?;
            return Ok((v.into_iter().map(|k| k * std::f64::consts::LN_10).collect(), 0.0));
        }
    }
    let text = e.tex.expand_to_string(trimmed.clone())?;
    let text = text.trim();
    if text == "data" {
        let first = axis.entries.iter().find_map(|en| if let Entry::Plot(p) = en { Some(p) } else { None });
        let mut v: Vec<f64> = Vec::new();
        if let Some(p) = first {
            for co in &p.coords {
                let x = [co.x, co.y, co.z][a];
                if x.is_finite() && !v.iter().any(|y| (y - x).abs() < 1e-12) {
                    v.push(x);
                }
            }
        }
        return Ok((v.into_iter().map(|x| f.logical(a, x)).collect(), 0.0));
    }
    if !text.is_empty() {
        // names of a symbolic axis
        if !crate::symbolic::names(e, c).is_empty() {
            let mut v = Vec::new();
            for item in tikra_engine::engine::foreach_items(&e.tex.expand_fully(trimmed.clone())?) {
                let s = tex::tokens_to_string(&item);
                match crate::symbolic::index(e, c, &s) {
                    Some(i) => v.push(i),
                    None => {
                        if let Ok(x) = e.number_of(item) {
                            v.push(x);
                        }
                    }
                }
            }
            return Ok((v, 0.0));
        }
        let v = number_list(e, trimmed)?;
        return Ok((v.into_iter().map(|x| f.logical(a, x)).filter(|x| x.is_finite()).collect(), 0.0));
    }
    if f.polar {
        if let Some(t) = crate::polar::ticks(e, a, f) {
            return Ok(t);
        }
    }
    let length_pt = f.e[a].norm() * (f.max[a] - f.min[a]).abs() / PT;
    let max_space = keys::number(e, "/pgfplots/max space between ticks").unwrap_or(35.0);
    let distance = keys::number(e, &format!("/pgfplots/{c}tick distance"));
    let (lo, hi) = (f.min[a].min(f.max[a]), f.min[a].max(f.max[a]));
    if f.log[a] {
        let try_min = keys::number(e, "/pgfplots/try min ticks log").unwrap_or(3.0) as usize;
        // `log basis y=2`: the ticks at powers of the basis (`log basis ticks`), found as powers of
        // ten in logarithms scaled by ln 10 / ln b
        let k = match keys::number(e, &format!("/pgfplots/log basis {c}")) {
            Some(b) if b > 0.0 && b != 1.0 && b != 10.0 => std::f64::consts::LN_10 / b.ln(),
            _ => 1.0,
        };
        let distance = distance.map(|d| d.powf(k));
        let (t, d, _) = ticks::logarithmic(lo * k, hi * k, TickSettings { length_pt, max_space, try_min, distance, scale: None });
        Ok((t.into_iter().map(|v| v / k).collect(), d / k))
    } else {
        let try_min = keys::number(e, "/pgfplots/try min ticks").unwrap_or(4.0) as usize;
        // the data scale transformation: the data range (with given limits) and `width`/`height`
        let size = keys::length(e, if a == 0 { "/pgfplots/width" } else { "/pgfplots/height" }).unwrap_or(if a == 0 { 240.0 * PT } else { 207.0 * PT }) / PT;
        let [dlo, dhi] = f.data_range[a];
        let scale = Some((dlo, dhi, size));
        Ok(ticks::linear(lo, hi, TickSettings { length_pt, max_space, try_min, distance, scale }))
    }
}
