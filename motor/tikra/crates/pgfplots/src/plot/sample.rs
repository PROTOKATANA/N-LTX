use tikra_engine::tex::{self, Cat, Token};
use tikra_engine::Error;

use super::super::keys;
use super::super::table;
use super::super::Engine;
use super::{Coord, Plot};

/// Splits tokens at top-level commas.
pub fn split_commas(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut out = vec![Vec::new()];
    let mut depth = 0i32;
    for t in tokens {
        match t {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => depth -= 1,
            Token::Char('(', _) => depth += 1,
            Token::Char(')', _) => depth -= 1,
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
    out.into_iter().map(|p| tex::strip_outer_braces(tikra_engine::keys::trim(&p).to_vec())).collect()
}

/// The domain and samples of an expression plot.
pub(super) fn sample_points(e: &mut Engine, domain_key: &str, samples_key: &str, log: bool) -> Result<Vec<f64>, Error> {
    let at = tikra_engine::keys::value(&e.tex, "/pgfplots/samples at").filter(|v| !tikra_engine::keys::trim(v).is_empty());
    if let Some(list) = at.filter(|_| domain_key == "/pgfplots/domain") {
        let list = e.tex.expand_fully(list)?;
        let mut v = Vec::new();
        for item in tikra_engine::engine::foreach_items(&list) {
            v.push(e.number_of(item)?);
        }
        return Ok(v);
    }
    let d = keys::value(e, domain_key);
    let d = if d.is_empty() { keys::value(e, "/pgfplots/domain") } else { d };
    let (a, b) = if d.is_empty() {
        e.st.plot.domain
    } else {
        let (a, b) = d.split_once(':').ok_or_else(|| Error::Figure(format!("domain '{d}' is not a:b")))?;
        (e.number(a)?, e.number(b)?)
    };
    let n = keys::number(e, samples_key).or_else(|| keys::number(e, "/pgfplots/samples")).map_or(e.st.plot.samples, |s| s.max(1.0) as usize).clamp(1, 10_000);
    if n == 1 {
        return Ok(vec![a]);
    }
    if !log {
        if let Some(v) = tex_samples(a, b, n) {
            return Ok(v);
        }
    } else if a > 0.0 && b > 0.0 {
        // on a log axis, the same loop between the fpu's logarithms of the ends: 1:100000 in 25
        // samples ends at ln 11.51257, not 11.51291
        if let Some(v) = tex_samples(super::super::fpu::ln(a), super::super::fpu::ln(b), n) {
            return Ok(v.into_iter().map(f64::exp).collect());
        }
    }
    let mut v = Vec::with_capacity(n);
    for k in 0..n {
        let t = k as f64 / (n - 1) as f64;
        v.push(if log && a > 0.0 && b > 0.0 { (a.ln() + t * (b.ln() - a.ln())).exp() } else { a + t * (b - a) });
    }
    Ok(v)
}

/// The samples of `domain=a:b` as pgfplots loops over them, in pgf's floating point unit
/// (`\pgfplots@domain@to@foreach` and the `a,a+h,...,b` loop of `\pgfplotsforeachungrouped`):
/// `a + (b - a)/(n - 1)` is the second sample, `h` its distance to `a`, and the loop adds `h`
/// while the sample is at most `b + h/2`. The steps keep few digits, so the last sample can fall a
/// little short of `b`, which moves the data limits and, sometimes, the ticks.
pub(super) fn tex_samples(a: f64, b: f64, n: usize) -> Option<Vec<f64>> {
    use super::super::fpu::Num;
    if n < 2 || a == b {
        return None;
    }
    let (fa, fb) = (Num::from_f64(a)?, Num::from_f64(b)?);
    let q = fb.plus(fa.negated()).over(Num::from_f64((n - 1) as f64)?);
    let h = fa.plus(q).plus(fa.negated());
    if h.is_zero() {
        return None;
    }
    let end = b + 0.5 * h.value();
    let up = h.value() > 0.0;
    let mut v = Vec::with_capacity(n + 1);
    let mut x = fa;
    while (up && x.value() <= end) || (!up && x.value() >= end) {
        v.push(x.value());
        x = x.plus(h);
        if v.len() > 2 * n + 2 {
            return None;
        }
    }
    Some(v)
}

/// Where the points of one patch of a type lie in the unit interval or square (`sample in unit
/// cube` of the patch classes), in their order; None for a type without them.
pub(super) fn patch_unit_points(kind: &str) -> Option<Vec<(f64, f64)>> {
    let (t1, t2) = (1.0 / 3.0, 2.0 / 3.0);
    Some(match kind.trim() {
        "line" => vec![(0.0, 0.0), (1.0, 0.0)],
        "quadratic spline" => vec![(0.0, 0.0), (1.0, 0.0), (0.5, 0.0)],
        "cubic spline" => vec![(0.0, 0.0), (1.0, 0.0), (t1, 0.0), (t2, 0.0)],
        "rectangle" | "bilinear" => vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)],
        "triangle quadr" => vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (0.5, 0.0), (0.5, 0.5), (0.0, 0.5), (0.0, 1.0), (1.0, 0.0), (1.0, 1.0), (0.5, 0.5), (1.0, 0.5), (0.5, 1.0)],
        "biquadratic" => vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5), (0.5, 0.5)],
        "bicubic" => (0..16).map(|k| ([0.0, t1, t2, 1.0][k % 4], [0.0, t1, t2, 1.0][k / 4])).collect(),
        _ => return None,
    })
}

/// Samples `(x(var), y(var)[, z(var, y)])`.
pub(super) fn sample(e: &mut Engine, parts: &[Vec<Token>], plot: &mut Plot) -> Result<Vec<Coord>, Error> {
    let var = e.st.plot.variable.clone();
    let var_y = keys::value(e, "/pgfplots/variable y").trim_start_matches('\\').to_string();
    // the variables read as names in the expressions
    for (name, value) in [(var.as_str(), var.clone()), (var_y.as_str(), var_y.clone())] {
        e.tex.set_meaning(&Token::cs(name), tikra_engine::engine::macro_from(&value), false);
    }
    let texts: Vec<String> = parts.iter().map(|p| e.math_string(p.clone())).collect::<Result<_, _>>()?;
    let xlog = keys::value(e, "/pgfplots/xmode") == "log";
    let uniform_x = texts.first().is_some_and(|t| t.trim() == var);
    let xs = sample_points(e, "/pgfplots/domain", "/pgfplots/samples", xlog && uniform_x)?;
    let mut vars = fpu_vars(e);
    // `trig format plots`: the plot's expressions in radians (or degrees), whatever pgf's is
    match keys::value(e, "/pgfplots/trig format plots").trim() {
        "rad" => vars.trig_rad = true,
        "deg" => vars.trig_rad = false,
        _ => {}
    }
    let mut out = Vec::new();
    // a point pgfplots cannot compute is unbounded (NaN); the first error is kept, to tell it when
    // no point at all could be
    let failed: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    let eval = |text: &str, vars: &tikra_engine::expr::Variables| {
        tikra_engine::expr::eval(text, vars).unwrap_or_else(|err| {
            failed.borrow_mut().get_or_insert_with(|| err.to_string());
            f64::NAN
        })
    };
    // `patch type sampling`: each interval (or cell) gives the points of one patch, at the places
    // of the patch type's points in the unit interval (square)
    let unit = if keys::flag(e, "/pgfplots/patch type sampling") { patch_unit_points(&keys::value(e, "/pgfplots/patch type")) } else { None };
    if let Some(unit) = unit {
        let at = |v: &[f64], i: usize, t: f64| v[i] + t * (v[i + 1] - v[i]);
        let two = plot.three_d && texts.len() >= 3 && unit.iter().any(|(_, v)| *v != 0.0);
        let ys = if two { sample_points(e, "/pgfplots/y domain", "/pgfplots/samples y", false)? } else { vec![0.0, 0.0] };
        for j in 0..ys.len().saturating_sub(1) {
            for i in 0..xs.len().saturating_sub(1) {
                for &(u, v) in &unit {
                    let (x, y) = (at(&xs, i, u), at(&ys, j, v));
                    vars.insert(var.clone(), x);
                    vars.insert(var_y.clone(), y);
                    let mut c = Coord::new(eval(&texts[0], &vars), texts.get(1).map_or(f64::NAN, |t| eval(t, &vars)), texts.get(2).map_or(0.0, |t| eval(t, &vars)));
                    c.param = [x, y];
                    out.push(c);
                }
            }
        }
        plot.patch = plot.mesh.is_some();
        return Ok(out);
    }
    if plot.three_d && texts.len() >= 3 {
        let ys = sample_points(e, "/pgfplots/y domain", "/pgfplots/samples y", false)?;
        let line = ys.len() < 2;
        if line {
            for &x in &xs {
                vars.insert(var.clone(), x);
                vars.insert(var_y.clone(), 0.0);
                out.push(Coord::new(eval(&texts[0], &vars), eval(&texts[1], &vars), eval(&texts[2], &vars)));
            }
        } else {
            for &y in &ys {
                for &x in &xs {
                    vars.insert(var.clone(), x);
                    vars.insert(var_y.clone(), y);
                    let mut c = Coord::new(eval(&texts[0], &vars), eval(&texts[1], &vars), eval(&texts[2], &vars));
                    c.param = [x, y];
                    out.push(c);
                }
            }
            plot.lattice = Some((ys.len(), xs.len()));
        }
    } else {
        for &x in &xs {
            vars.insert(var.clone(), x);
            let cx = eval(&texts[0], &vars);
            let cy = texts.get(1).map_or(f64::NAN, |t| eval(t, &vars));
            let cz = texts.get(2).map_or(0.0, |t| eval(t, &vars));
            let mut c = Coord::new(cx, cy, cz);
            c.param = [x, f64::NAN];
            out.push(c);
        }
    }
    if let Some(err) = failed.into_inner().filter(|_| out.iter().all(|c| !(c.x.is_finite() && c.y.is_finite()))) {
        e.warn(format!("\\addplot: {err}: no point of the plot could be computed"));
    }
    Ok(out)
}

/// A value of `coordinates`: a number, or an expression.
/// A coordinate of axis `c`: a name of `symbolic c coords`, or a number.
pub(super) fn axis_value(e: &mut Engine, c: char, tokens: &[Token]) -> f64 {
    if let Some(i) = super::super::symbolic::index(e, c, &tex::tokens_to_string(tokens)) {
        return i;
    }
    value(e, tokens)
}

/// pgfmath's variables for pgfplots' math, which runs with the fpu on.
pub(super) fn fpu_vars(e: &Engine) -> tikra_engine::expr::Variables {
    tikra_engine::expr::Variables { fpu: true, ..e.vars() }
}

pub(super) fn value(e: &mut Engine, tokens: &[Token]) -> f64 {
    let s = tex::tokens_to_string(tokens);
    if let Some(v) = table::parse_number(&s) {
        return v;
    }
    let tokens = tokens.to_vec();
    e.math_string(tokens).and_then(|t| tikra_engine::expr::eval(&t, &fpu_vars(e))).unwrap_or(f64::NAN)
}
