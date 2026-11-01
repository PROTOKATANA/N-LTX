use tikra_engine::tex::{self, Cat, Token};
use tikra_engine::Error;

use super::super::keys;
use super::super::table::Table;
use super::super::Engine;
use super::rows::replace_thisrow;
use super::sample::fpu_vars;
use super::{add_label, Coord, Plot, NO_LABEL};

/// The code of a key (`x filter/.code`), if it has some: its macro.
pub(super) fn filter_code(e: &Engine, path: &str) -> Option<Token> {
    let name = tikra_engine::keys::macro_name(&format!("{path}/.@cmd"));
    match e.tex.meaning_of(&name) {
        tex::Meaning::Macro(m) if !m.body.is_empty() => Some(Token::cs(&name)),
        _ => None,
    }
}

/// pgfplots' coordinate filters (`\pgfplots@coord@filter`): `x filter`, `y filter`, `z filter` as
/// code (`\pgfmathresult` in and out, empty to drop the point) and as expressions (`x`, `y`,
/// `rawx`…, `\coordindex`, `\thisrow`), then `restrict expr to domain`.
pub(super) struct FilterSet {
    codes: Vec<Option<Token>>,
    exprs: Vec<Vec<Vec<Token>>>,
    restrict: Option<(Vec<Token>, String)>,
    conditions: Vec<Vec<Token>>,
    from: Option<Vec<Token>>,
    classes: Vec<(String, Vec<Token>)>,
    deps: Vec<(Vec<Token>, bool, String)>,
    dims: usize,
}

impl FilterSet {
    fn idle(&self) -> bool {
        self.codes.iter().all(Option::is_none)
            && self.exprs.iter().all(Vec::is_empty)
            && self.restrict.is_none()
            && self.conditions.is_empty()
            && self.from.is_none()
            && self.classes.is_empty()
            && self.deps.is_empty()
    }
}

/// `visualization depends on`: (expression, verbatim, macro).
pub(super) fn visualization_deps(e: &Engine) -> Vec<(Vec<Token>, bool, String)> {
    split_groups(&tikra_engine::keys::value(&e.tex, "/pgfplots/@visualization depends on").unwrap_or_default())
        .into_iter()
        .filter_map(|d| {
            let at = d.iter().position(|t| t.is_cs("as"))?;
            let name = d[at + 1..].iter().find_map(|t| if let Token::Cs(n) = t { Some(n.to_string()) } else { None })?;
            let mut expr = tikra_engine::keys::trim(&d[..at]).to_vec();
            let text = tex::tokens_to_string(&expr);
            let verbatim = text.trim_start().starts_with("value");
            if verbatim {
                expr = tex::strip_outer_braces(tikra_engine::keys::trim(&super::super::toks(text.trim_start().trim_start_matches("value"))).to_vec());
            }
            Some((expr, verbatim, name))
        })
        .collect()
}

pub(super) fn load_filters(e: &mut Engine, plot: &Plot) -> FilterSet {
    let dims = if plot.three_d { 3 } else { 2 };
    let mut codes: Vec<Option<Token>> = Vec::new();
    let mut exprs: Vec<Vec<Vec<Token>>> = Vec::new();
    for c in ['x', 'y', 'z'].iter().take(dims) {
        let path = format!("/pgfplots/{c} filter");
        codes.push(filter_code(e, &path));
        let list = tikra_engine::keys::value(&e.tex, &format!("{path}/@expressions")).unwrap_or_default();
        exprs.push(split_groups(&list));
    }
    FilterSet {
        codes,
        exprs,
        restrict: keys::value_tokens(e, "/pgfplots/@restrict expr").zip(Some(keys::value(e, "/pgfplots/@restrict expr domain"))),
        conditions: split_groups(&tikra_engine::keys::value(&e.tex, "/pgfplots/coordinate style/@conditions").unwrap_or_default()),
        from: keys::value_tokens(e, "/pgfplots/coordinate style/@from"),
        classes: if plot.scatter { scatter_classes(e) } else { Vec::new() },
        deps: visualization_deps(e),
        dims,
    }
}

/// One point under the filters: its raw value, its table row, and the filters themselves.
pub(super) struct PointEnv<'a> {
    set: &'a FilterSet,
    row: Option<(&'a Table, usize)>,
    index: usize,
    raw: [f64; 3],
    label: u32,
}

/// `\pgfmathresult` and `\coordindex` replaced, and `\thisrow` when the point came from a table.
pub(super) fn substitute(env: &PointEnv<'_>, tokens: &[Token], result: f64) -> Vec<Token> {
    let tokens = match env.row {
        Some((t, r)) => replace_thisrow(tokens, t, r),
        None => tokens.to_vec(),
    };
    let mut out = Vec::with_capacity(tokens.len());
    for t in tokens {
        if t.is_cs("pgfmathresult") {
            out.extend(super::super::toks(&format!("({})", tikra_engine::engine::pgf_number(result))));
        } else if t.is_cs("coordindex") {
            out.extend(super::super::toks(&env.index.to_string()));
        } else {
            out.push(t);
        }
    }
    out
}

/// The point as the filters see it: `cur` after earlier filters, `raw` as read.
pub(super) struct AxisNow {
    cur: [f64; 3],
    raw: [f64; 3],
}

pub(super) fn eval_filter(e: &mut Engine, tokens: Vec<Token>, now: &AxisNow) -> f64 {
    let mut vars = fpu_vars(e);
    for (i, n) in ["x", "y", "z"].iter().enumerate() {
        vars.insert((*n).into(), now.cur[i]);
        vars.insert(format!("raw{n}"), now.raw[i]);
    }
    e.math_string(tokens).and_then(|s| tikra_engine::expr::eval(&s, &vars)).unwrap_or(f64::NAN)
}

/// `x filter`, `y filter`, `z filter`. True when a code leaves `\pgfmathresult` empty: the point is dropped.
pub(super) fn apply_axis_filters(e: &mut Engine, env: &PointEnv<'_>, cur: &mut [f64; 3]) -> Result<bool, Error> {
    for a in 0..env.set.dims {
        if let Some(code) = &env.set.codes[a] {
            e.tex.define_string("pgfmathresult", &tikra_engine::engine::pgf_number(cur[a]), false);
            e.tex.define_string("coordindex", &env.index.to_string(), false);
            // the code's argument is the value too
            let mut call = vec![code.clone(), Token::Char('{', Cat::Begin)];
            call.extend(super::super::toks(&tikra_engine::engine::pgf_number(cur[a])));
            call.push(Token::Char('}', Cat::End));
            e.run_tokens(call)?;
            let out = e.tex.expand_to_string(vec![Token::cs("pgfmathresult")])?;
            let out = out.trim();
            if out.is_empty() {
                return Ok(true);
            }
            cur[a] = e.number(out).unwrap_or(f64::NAN);
        }
        for x in env.set.exprs[a].clone() {
            let tokens = substitute(env, &x, cur[a]);
            cur[a] = eval_filter(e, tokens, &AxisNow { cur: *cur, raw: env.raw });
        }
    }
    Ok(false)
}

/// `restrict expr to domain`: a value outside it makes x unbounded.
pub(super) fn apply_restrict(e: &mut Engine, env: &PointEnv<'_>, cur: &mut [f64; 3]) -> Result<(), Error> {
    let Some((expr, domain)) = &env.set.restrict else { return Ok(()) };
    if let Some((lo, hi)) = domain.split_once(':') {
        let (lo, hi) = (e.number(lo)?, e.number(hi)?);
        let v = eval_filter(e, substitute(env, expr, cur[0]), &AxisNow { cur: *cur, raw: env.raw });
        if !(lo..=hi).contains(&v) {
            cur[0] = f64::INFINITY;
        }
    }
    Ok(())
}

/// The coordinate's own style: its `scatter/classes` class, `.from`, then the conditions that hold.
pub(super) fn coordinate_style(e: &mut Engine, env: &PointEnv<'_>, cur: [f64; 3]) -> Result<Vec<Token>, Error> {
    let mut style = Vec::new();
    // (the labels are still being collected)
    let label = (env.label != NO_LABEL).then(|| super::super::fig(e).labels.get(env.label as usize).cloned()).flatten();
    if let Some(l) = label {
        let name = tex::tokens_to_string(&l);
        if let Some((_, s)) = env.set.classes.iter().find(|(n, _)| *n == name.trim()) {
            style.extend(s.clone());
        }
    }
    if let Some(from) = &env.set.from {
        // `\thisrow{style}`: the cell's text
        let text = tex::tokens_to_string(from);
        let cell = text.trim().strip_prefix("\\thisrow").map(|n| n.trim().trim_start_matches('{').trim_end_matches('}').trim().to_string());
        let text = match (&cell, env.row) {
            (Some(name), Some((t, r))) => t.column(name).and_then(|c| t.cell(r, c)).unwrap_or("").to_string(),
            _ => e.tex.expand_to_string(from.clone())?,
        };
        let text = text.trim();
        let text = text.strip_prefix('{').and_then(|t| t.strip_suffix('}')).unwrap_or(text);
        style.extend(super::super::toks(text.trim()));
    }
    for pair in env.set.conditions.chunks(2) {
        let [test, s] = pair else { break };
        let v = eval_filter(e, substitute(env, test, cur[0]), &AxisNow { cur, raw: env.raw });
        if v != 0.0 && !v.is_nan() {
            if !style.is_empty() {
                style.push(Token::other(','));
            }
            style.extend(s.clone());
        }
    }
    Ok(style)
}

pub(super) fn depend_values(e: &mut Engine, env: &PointEnv<'_>, cur: [f64; 3]) -> Result<Vec<(String, String)>, Error> {
    let mut values = Vec::new();
    for (expr, verbatim, name) in &env.set.deps {
        let tokens = substitute(env, expr, cur[0]);
        let v =
            if *verbatim { e.tex.expand_to_string(tokens)?.trim().to_string() } else { tikra_engine::engine::pgf_number(eval_filter(e, tokens, &AxisNow { cur, raw: env.raw })) };
        values.push((name.clone(), v));
    }
    Ok(values)
}

/// Filters one point. True when a filter code asked to drop it.
pub(super) fn filter_point(e: &mut Engine, plot: &mut Plot, env: &PointEnv<'_>) -> Result<bool, Error> {
    let mut cur = env.raw;
    let dropped = apply_axis_filters(e, env, &mut cur)?;
    apply_restrict(e, env, &mut cur)?;
    let style = coordinate_style(e, env, cur)?;
    let values = depend_values(e, env, cur)?;
    if !values.is_empty() {
        plot.coords[env.index].deps = plot.deps.len() as u32;
        plot.deps.push(values);
    }
    let c = &mut plot.coords[env.index];
    (c.x, c.y, c.z) = (cur[0], cur[1], cur[2]);
    if style.iter().any(|t| !t.is_space()) {
        c.style = plot.coord_styles.len() as u32;
        plot.coord_styles.push(style);
    }
    Ok(dropped)
}

pub(super) fn coordinate_filters(e: &mut Engine, plot: &mut Plot) -> Result<(), Error> {
    let rows = super::super::fig(e).filter_rows.take().filter(|(_, r)| r.len() == plot.coords.len());
    let set = load_filters(e, plot);
    if set.idle() {
        return Ok(());
    }
    let mut drop = vec![false; plot.coords.len()];
    for k in 0..plot.coords.len() {
        let raw = plot.coords[k];
        let row = rows.as_ref().map(|(t, r)| (t, r[k]));
        drop[k] = filter_point(e, plot, &PointEnv { set: &set, row, index: k, raw: [raw.x, raw.y, raw.z], label: raw.label })?;
    }
    if drop.iter().any(|d| *d) {
        let mut k = 0;
        plot.coords.retain(|_| {
            k += 1;
            !drop[k - 1]
        });
    }
    Ok(())
}

/// `scatter/classes={a={mark=square*,blue},b={…}}`: each class' name and style.
pub fn scatter_classes(e: &mut Engine) -> Vec<(String, Vec<Token>)> {
    let Some(v) = keys::value_tokens(e, "/pgfplots/scatter/classes") else { return Vec::new() };
    tikra_engine::keys::split_list(&v).into_iter().map(|i| (i.key.trim().to_string(), i.value.map(tex::strip_outer_braces).unwrap_or_default())).collect()
}

/// The brace groups of a list (`{a}{b}`).
pub(super) fn split_groups(list: &[Token]) -> Vec<Vec<Token>> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut cur = Vec::new();
    for t in list {
        match t {
            Token::Char(_, Cat::Begin) => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
            }
            Token::Char(_, Cat::End) => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    out.push(std::mem::take(&mut cur));
                    continue;
                }
            }
            _ => {}
        }
        if depth > 0 {
            cur.push(t.clone());
        }
    }
    out
}

/// `restrict x to domain`, `restrict y to domain`, `unbounded coords`.
pub(super) fn filter(e: &mut Engine, plot: &mut Plot) -> Result<(), Error> {
    // a restriction makes the coordinates outside unbounded, and unbounded coordinates jump
    let mut restricted = false;
    for (axis, key) in [('x', "/pgfplots/restrict x to domain"), ('y', "/pgfplots/restrict y to domain")] {
        let d = keys::value(e, key);
        if let Some((a, b)) = d.split_once(':') {
            restricted = true;
            let (a, b) = (e.number(a)?, e.number(b)?);
            for c in &mut plot.coords {
                let v = if axis == 'x' { &mut c.x } else { &mut c.y };
                if *v < a || *v > b {
                    *v = f64::NAN;
                }
            }
        }
    }
    let jump = restricted || keys::value(e, "/pgfplots/unbounded coords") == "jump";
    let log = (keys::value(e, "/pgfplots/xmode") == "log", keys::value(e, "/pgfplots/ymode") == "log");
    for c in &mut plot.coords {
        if (log.0 && c.x <= 0.0) || (log.1 && c.y <= 0.0) {
            c.x = f64::NAN;
        }
        if !(c.x.is_finite() && c.y.is_finite() && (!plot.three_d || c.z.is_finite())) {
            c.x = f64::NAN;
        }
    }
    if !jump && plot.lattice.is_none() {
        plot.coords.retain(|c| c.x.is_finite());
    }
    Ok(())
}

/// An expression with the parentheses still open at its end closed: pgfmath reads
/// `(sqrt((1)^2 + (2*x)^2)` (the manual's quiver example) as if they were.
pub(super) fn balanced(text: &str) -> String {
    let open = text.chars().filter(|c| *c == '(').count();
    let close = text.chars().filter(|c| *c == ')').count();
    format!("{text}{}", ")".repeat(open.saturating_sub(close)))
}

/// `quiver`: each point's (u, v, w), from `quiver/u` and its friends (expressions of x, y, z and
/// the sampling variable), times `scale arrows`.
pub(super) fn quiver_vectors(e: &mut Engine, plot: &mut Plot) {
    let var = e.st.plot.variable.clone();
    let var_y = keys::value(e, "/pgfplots/variable y").trim_start_matches('\\').to_string();
    let texts: Vec<Option<String>> = ["u", "v", "w"].iter().map(|n| keys::value_tokens(e, &format!("/pgfplots/quiver/{n}")).and_then(|t| e.math_string(t).ok())).collect();
    let scale = keys::number(e, "/pgfplots/quiver/scale arrows").unwrap_or(1.0);
    let mut vars = fpu_vars(e);
    for c in plot.coords.iter_mut() {
        vars.insert("x".into(), c.x);
        vars.insert("y".into(), c.y);
        vars.insert("z".into(), c.z);
        if c.param[0].is_finite() {
            vars.insert(var.clone(), c.param[0]);
        }
        if c.param[1].is_finite() && !var_y.is_empty() {
            vars.insert(var_y.clone(), c.param[1]);
        }
        for (u, text) in c.uvw.iter_mut().zip(&texts) {
            if !u.is_finite() {
                *u = match text {
                    Some(t) if !t.trim().is_empty() => tikra_engine::expr::eval(&balanced(t), &vars).unwrap_or(f64::NAN),
                    _ => 0.0,
                };
            }
            *u *= scale;
        }
    }
}

/// `data cs`: the input's coordinate system (`cart`, `polar` in degrees, `polarrad`) turned into
/// the axis' (`polar` for a polar axis).
pub(super) fn data_cs(e: &mut Engine, plot: &mut Plot) {
    let source = keys::value(e, "/pgfplots/data cs");
    let target = keys::value(e, "/pgfplots/@axis type");
    if source == target || source.is_empty() {
        return;
    }
    for c in plot.coords.iter_mut() {
        if source == "polarrad" {
            c.x = c.x.to_degrees();
        }
        let (x, y) = (c.x, c.y);
        match (source.as_str(), target.as_str()) {
            ("polar" | "polarrad", "cart") => {
                let (s, co) = super::super::polar::sin_cos(x);
                (c.x, c.y) = (y * co, y * s);
            }
            ("cart", "polar") => {
                (c.x, c.y) = (y.atan2(x).to_degrees().rem_euclid(360.0), x.hypot(y));
            }
            _ => {}
        }
    }
}

/// `hist`: the points become the bins' counts, `(start of bin k, count k)`, and a last point at
/// the end of the data range (`\pgfplotsplothandlersurveyend@hist@`).
pub(super) fn histogram(e: &mut Engine, plot: &mut Plot) -> Result<(), Error> {
    let data = keys::value(e, "/pgfplots/hist/data");
    let value_expr = keys::value(e, "/pgfplots/hist/data value");
    let mut vars = fpu_vars(e);
    let mut values = Vec::new();
    for c in &plot.coords {
        let v = if !value_expr.is_empty() {
            vars.insert("x".into(), c.x);
            vars.insert("y".into(), c.y);
            vars.insert("z".into(), c.z);
            tikra_engine::expr::eval(&value_expr, &vars).unwrap_or(f64::NAN)
        } else {
            match data.as_str() {
                "x" => c.x,
                "z" => c.z,
                "y" | "" => c.y,
                other => {
                    vars.insert("x".into(), c.x);
                    vars.insert("y".into(), c.y);
                    vars.insert("z".into(), c.z);
                    tikra_engine::expr::eval(other, &vars).unwrap_or(f64::NAN)
                }
            }
        };
        if v.is_finite() {
            values.push(v);
        }
    }
    let bins = keys::number(e, "/pgfplots/hist/bins").unwrap_or(10.0).max(1.0) as usize;
    // `data min` defaults to `xmin`
    let limit = |e: &mut Engine, k: &str, axis: &str| keys::number(e, &format!("/pgfplots/hist/{k}")).or_else(|| keys::number(e, &format!("/pgfplots/{axis}")));
    let lo = limit(e, "data min", "xmin").unwrap_or_else(|| values.iter().copied().fold(f64::INFINITY, f64::min));
    let hi = limit(e, "data max", "xmax").unwrap_or_else(|| values.iter().copied().fold(f64::NEG_INFINITY, f64::max));
    if values.is_empty() || !(lo.is_finite() && hi.is_finite()) {
        plot.coords.clear();
        return Ok(());
    }
    let h = (hi - lo) / bins as f64;
    let mut counts = vec![0.0f64; bins];
    for v in &values {
        let k = if h != 0.0 { ((v - lo) / h + 1e-4).trunc() } else { 0.0 };
        let k = if k < 0.0 { 0 } else { (k as usize).min(bins - 1) };
        counts[k] += 1.0;
    }
    let total: f64 = counts.iter().sum();
    let cumulative = keys::flag(e, "/pgfplots/hist/cumulative");
    let density = keys::flag(e, "/pgfplots/hist/density");
    if cumulative {
        let mut acc = 0.0;
        for c in counts.iter_mut() {
            acc += *c;
            *c = if density { acc / total } else { acc };
        }
    } else if density && total > 0.0 && h != 0.0 {
        for c in counts.iter_mut() {
            *c /= total * h;
        }
    }
    let mut coords: Vec<Coord> = counts.iter().enumerate().map(|(k, c)| Coord::new(lo + h * k as f64, *c, 0.0)).collect();
    if keys::flag(e, "/pgfplots/hist/intervals") {
        coords.push(Coord::new(hi, counts[bins - 1], 0.0));
    }
    plot.coords = coords;
    plot.three_d = false;
    plot.scanlines.clear();
    plot.handler = keys::value(e, "/tikz/@plot handler");
    Ok(())
}

/// `point meta={symbolic={R,G,B}}`: the text of each point's meta, and whether
/// `mesh/color input=explicit mathparse` evaluates it.
pub(super) fn symbolic_meta(e: &mut Engine) -> Option<(Vec<Token>, bool)> {
    let raw = tikra_engine::keys::value(&e.tex, "/pgfplots/point meta")?;
    let items = tikra_engine::keys::split_list(tikra_engine::keys::trim(&raw));
    let item = items.first().filter(|i| i.key == "symbolic" && items.len() == 1)?;
    let mathparse = keys::value(e, "/pgfplots/mesh/color input") == "explicit mathparse";
    Some((item.value.clone().unwrap_or_default(), mathparse))
}

/// A symbolic point meta for a point: with `mathparse`, each part (`R,G,B`, `wave=…`) evaluated.
pub(super) fn symbolic_label(e: &mut Engine, text: &str, mathparse: bool, xyz: [f64; 3]) -> String {
    if !mathparse {
        return text.trim().to_string();
    }
    let mut vars = fpu_vars(e);
    vars.insert("x".into(), xyz[0]);
    vars.insert("y".into(), xyz[1]);
    vars.insert("z".into(), xyz[2]);
    let (model, rest) = match text.split_once('=') {
        Some((m, r)) if m.trim().chars().all(|c| c.is_ascii_alphanumeric()) => (format!("{}=", m.trim()), r),
        _ => (String::new(), text),
    };
    let parts: Vec<String> = tikra_engine::keys::split_top_level(rest)
        .iter()
        .map(|p| tikra_engine::expr::eval(p.trim(), &vars).map(|v| format!("{v}")).unwrap_or_else(|_| p.trim().to_string()))
        .collect();
    format!("{model}{}", parts.join(","))
}

/// `point meta`: the value each point maps to the colormap with.
pub(super) fn point_meta(e: &mut Engine, plot: &mut Plot) -> Result<(), Error> {
    if let Some((tokens, mathparse)) = symbolic_meta(e) {
        // tables have set them row by row
        if plot.coords.iter().all(|c| c.label == NO_LABEL) {
            let text = e.tex.expand_to_string(tokens).unwrap_or_default();
            for i in 0..plot.coords.len() {
                let c = plot.coords[i];
                let l = symbolic_label(e, &text, mathparse, [c.x, c.y, c.z]);
                plot.coords[i].label = add_label(e, tex::string_tokens(&l));
            }
        }
        plot.has_meta = false;
        return Ok(());
    }
    let raw = tikra_engine::keys::value(&e.tex, "/pgfplots/point meta").unwrap_or_default();
    let source = tex::tokens_to_string(tikra_engine::keys::trim(&raw)).trim().to_string();
    match source.as_str() {
        "" | "none" => {}
        "x" => plot.coords.iter_mut().for_each(|c| c.meta = c.x),
        "y" => plot.coords.iter_mut().for_each(|c| c.meta = c.y),
        "z" => plot.coords.iter_mut().for_each(|c| c.meta = c.z),
        "f(x)" => {
            let three = plot.three_d;
            plot.coords.iter_mut().for_each(|c| c.meta = if three { c.z } else { c.y })
        }
        "explicit" | "explicit symbolic" => {}
        _ => {
            // \thisrow{…} is evaluated with the table's rows
            if !source.contains("\\thisrow") {
                // `\coordindex` is the point's number
                let per_point = source.contains("\\coordindex");
                let mut text = if per_point { String::new() } else { e.math_string(raw.clone())? };
                let mut vars = fpu_vars(e);
                for (i, c) in plot.coords.iter_mut().enumerate() {
                    if per_point {
                        e.tex.define_string("coordindex", &i.to_string(), false);
                        text = e.math_string(raw.clone())?;
                    }
                    vars.insert("x".into(), c.x);
                    vars.insert("y".into(), c.y);
                    vars.insert("z".into(), c.z);
                    c.meta = tikra_engine::expr::eval(&balanced(&text), &vars).unwrap_or(f64::NAN);
                }
            }
        }
    }
    plot.has_meta = !matches!(source.as_str(), "" | "none" | "explicit symbolic");
    Ok(())
}

/// A lattice from `mesh/rows`, `mesh/cols` or the input's scanlines.
pub(super) fn lattice_from_scanlines(e: &mut Engine, plot: &mut Plot) {
    if plot.lattice.is_some() || plot.mesh.is_none() {
        return;
    }
    let n = plot.coords.len();
    let rows = keys::number(e, "/pgfplots/mesh/rows").map(|v| v as usize);
    let cols = keys::number(e, "/pgfplots/mesh/cols").map(|v| v as usize);
    let lattice = match (rows, cols) {
        (_, Some(c)) if c > 0 => Some((n / c, c)),
        (Some(r), _) if r > 0 => Some((r, n / r)),
        // patches are their own (`mesh input=patches`)
        _ if plot.patch => None,
        _ => plot.scanlines.first().filter(|c| **c > 0 && n.is_multiple_of(**c)).map(|c| (n / c, *c)).or_else(|| {
            // without scanlines a matrix is square (`matrix delinearize uses sqrt`)
            let s = (n as f64).sqrt().round() as usize;
            (plot.image && s * s == n && s > 0).then_some((s, s))
        }),
    };
    if let Some((r, c)) = lattice.filter(|(r, c)| r * c == n && *r > 1 && *c > 1) {
        plot.lattice = Some((r, c));
    }
}
