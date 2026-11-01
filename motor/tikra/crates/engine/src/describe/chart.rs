use super::words::Words;
use super::{AxisFact, Facts, NodeFact, Series, LIST_POINTS};

pub(in crate::describe) fn visible_nodes(f: &Facts) -> Vec<&NodeFact> {
    let mut out: Vec<&NodeFact> = Vec::new();
    for n in &f.nodes {
        if n.on_link || n.text.trim().is_empty() {
            continue;
        }
        // a node drawn again (measured, then placed) is told once
        if let Some(i) = out.iter().position(|m| (n.named && m.key == n.key) || (m.text == n.text && m.at == n.at)) {
            out[i] = n;
            continue;
        }
        out.push(n);
    }
    out
}

/// The axis in words: its short name and the whole of it.
pub(in crate::describe) fn describe_axis(w: &Words, a: &AxisFact) -> (String, String) {
    let kind = w.t(&format!("chart.{}", chart_kind(a)), &[]);
    let title = w.spoken(&a.title);
    let labels: Vec<String> = a.labels.iter().map(|l| w.spoken(l)).collect();
    let short = if !title.is_empty() {
        w.t("axis.short_title", &[&kind, &title])
    } else if !labels[0].is_empty() && !labels[1].is_empty() && !a.three_d {
        w.t("axis.against", &[&kind, &labels[1], &labels[0]])
    } else {
        kind.clone()
    };
    let mut s = Vec::new();
    s.push(w.sentence(&if title.is_empty() { kind.clone() } else { w.t("axis.titled", &[&kind, &w.quote(&title)]) }));
    let dims = if a.three_d { 3 } else { 2 };
    for (k, label) in labels.iter().enumerate().take(dims) {
        if let Some(t) = axis_sentence(w, a, k, label) {
            s.push(w.sentence(&t));
        }
    }
    if let Some([lo, hi]) = a.colors {
        s.push(w.sentence(&w.t("colors", &[&w.num(lo, hi - lo), &w.num(hi, hi - lo)])));
    }
    let series: Vec<&Series> = a.series.iter().collect();
    let same_kind = series.iter().all(|p| p.kind == series[0].kind);
    match series.len() {
        0 => s.push(w.sentence(w.phrase("no_data"))),
        1 => s.push(w.sentence(&series_text(w, &SeriesTell { axis: a, series: series[0], number: 0, with_kind: false }))),
        n => {
            s.push(w.sentence(&w.count("series", n)));
            for (i, p) in series.iter().enumerate() {
                s.push(w.sentence(&series_text(w, &SeriesTell { axis: a, series: p, number: i + 1, with_kind: !same_kind })));
            }
        }
    }
    (short, s.join(w.phrase("sep.sentence")))
}

/// The chart's kind, as the key of its name.
fn chart_kind(a: &AxisFact) -> &'static str {
    match a.kind {
        "polar" | "ternary" | "smith" => return a.kind,
        _ => {}
    }
    let kinds: Vec<&str> = a.series.iter().map(|p| p.kind).collect();
    let all = |k: &str| !kinds.is_empty() && kinds.iter().all(|x| *x == k);
    let named = [
        ("bar", "bar"),
        ("horizontal bar", "hbar"),
        ("histogram", "histogram"),
        ("scatter", "scatter"),
        ("area", "area"),
        ("surface", "surface"),
        ("mesh", "surface"),
        ("contour", "contour"),
        ("box plot", "box"),
        ("stem", "stem"),
        ("vector field", "field"),
    ];
    if let Some((_, k)) = named.iter().find(|(series, _)| all(series)) {
        k
    } else if all("line") {
        if a.three_d {
            "line3d"
        } else {
            "line"
        }
    } else if a.three_d {
        "plot3d"
    } else {
        "chart"
    }
}

fn axis_sentence(w: &Words, a: &AxisFact, k: usize, label: &str) -> Option<String> {
    if a.kind == "ternary" || a.kind == "smith" {
        return None;
    }
    let name = match (a.kind, a.three_d, k) {
        ("polar", _, 0) => "axis.angle",
        ("polar", _, _) => "axis.radius",
        (_, true, 0) => "axis.x",
        (_, true, 1) => "axis.y",
        (_, true, _) => "axis.z",
        (_, false, 0) => "axis.horizontal",
        _ => "axis.vertical",
    };
    let mut s = w.t(name, &[]);
    if !label.is_empty() {
        s = w.t("axis.label", &[&s, &w.quote(label)]);
    }
    if !a.symbolic[k].is_empty() {
        s = w.t("axis.categories", &[&s, &a.symbolic[k].join(w.phrase("sep.items"))]);
    } else if let Some([lo, hi]) = a.range[k] {
        let (lo, hi) = (value(w, a, k, lo), value(w, a, k, hi));
        s = w.t(if a.kind == "polar" && k == 0 { "axis.range_degrees" } else { "axis.range" }, &[&s, &lo, &hi]);
        if a.log[k] {
            s = w.t("axis.log", &[&s]);
        }
    } else if label.is_empty() {
        return None;
    }
    Some(s)
}

/// Where a series sits in the telling: the axis, the series, its number among several
/// (0 when it is alone) and whether its kind is part of the sentence.
struct SeriesTell<'a> {
    axis: &'a AxisFact,
    series: &'a Series,
    number: usize,
    with_kind: bool,
}

/// A series in words.
fn series_text(w: &Words, tell: &SeriesTell<'_>) -> String {
    let (a, p, number, with_kind) = (tell.axis, tell.series, tell.number, tell.with_kind);
    let name = w.spoken(&p.name);
    let mut head = String::new();
    if !name.is_empty() {
        head = w.quote(&name);
    } else if number > 0 {
        head = w.t("series.number", &[&number.to_string()]);
    }
    if with_kind {
        head = w.t("series.kind", &[&head, &w.t(&format!("kind.{}", p.kind), &[])]);
    }
    let data = series_data(w, a, p);
    match (head.is_empty(), data.is_empty()) {
        (true, true) => w.t("no_data", &[]),
        (true, false) => data,
        (false, true) => head,
        (false, false) => w.t("series.data", &[&head, &data]),
    }
}

fn series_data(w: &Words, a: &AxisFact, p: &Series) -> String {
    let n = |v: f64| w.num(v, v);
    if let Some([lw, lq, m, uq, uw]) = p.boxplot {
        let mut v = Vec::new();
        if let Some(m) = m {
            v.push(w.t("box.median", &[&n(m)]));
        }
        if let (Some(lq), Some(uq)) = (lq, uq) {
            v.push(w.t("box.quartiles", &[&n(lq), &n(uq)]));
        }
        if let (Some(lw), Some(uw)) = (lw, uw) {
            v.push(w.t("box.whiskers", &[&n(lw), &n(uw)]));
        }
        if !p.points.is_empty() {
            v.push(w.count("outlier", p.points.len()));
        }
        return v.join(w.phrase("sep.items"));
    }
    let pts: Vec<[f64; 3]> = p.points.iter().copied().filter(|c| c.iter().take(if a.three_d { 3 } else { 2 }).all(|v| v.is_finite())).collect();
    if pts.is_empty() {
        return String::new();
    }
    let x = |v: f64| value(w, a, 0, v);
    let y = |v: f64| value(w, a, 1, v);
    let z = |v: f64| value(w, a, 2, v);
    if !p.expression.is_empty() && pts.len() > LIST_POINTS {
        let formula = format!("{} = {}", if a.three_d { "z" } else { "y" }, p.expression.trim());
        let (x0, x1) = extent(&pts, 0);
        let s = if a.three_d {
            let (y0, y1) = extent(&pts, 1);
            w.t("formula.xy", &[&formula, &x(x0), &x(x1), &y(y0), &y(y1)])
        } else {
            w.t("formula.x", &[&formula, &x(x0), &x(x1)])
        };
        return format!("{s}{}{}", w.phrase("sep.list"), extremes(w, a, &pts));
    }
    if p.kind == "histogram" && pts.len() >= 2 && pts.len() <= LIST_POINTS + 1 {
        let bins: Vec<String> = pts.windows(2).map(|b| w.t("bin", &[&x(b[0][0]), &x(b[1][0]), &y(b[0][1])])).collect();
        return w.t("counted", &[&w.count("bin", bins.len()), &bins.join(w.phrase("sep.list"))]);
    }
    if pts.len() <= LIST_POINTS {
        let between = w.between();
        let list: Vec<String> = pts
            .iter()
            .map(|c| match p.kind {
                "bar" if !a.three_d => format!("{}: {}", x(c[0]), y(c[1])),
                "horizontal bar" if !a.three_d => format!("{}: {}", y(c[1]), x(c[0])),
                _ if a.three_d => format!("({}{between}{}{between}{})", x(c[0]), y(c[1]), z(c[2])),
                _ => format!("({}{between}{})", x(c[0]), y(c[1])),
            })
            .collect();
        let what = if matches!(p.kind, "bar" | "horizontal bar") { "bar" } else { "point" };
        return w.t("counted", &[&w.count(what, pts.len()), &list.join(between)]);
    }
    let (x0, x1) = extent(&pts, 0);
    let s = w.t("points.range", &[&w.count("point", pts.len()), &x(x0), &x(x1)]);
    format!("{s}{}{}", w.phrase("sep.list"), extremes(w, a, &pts))
}

/// Where the values (y, or z in 3d) are lowest and highest.
fn extremes(w: &Words, a: &AxisFact, pts: &[[f64; 3]]) -> String {
    let k = if a.three_d { 2 } else { 1 };
    let var = if a.three_d { "z" } else { "y" };
    // the first of equal extremes, as one reads from the left
    let lo = pts.iter().fold(pts[0], |m, p| if p[k] < m[k] { *p } else { m });
    let hi = pts.iter().fold(pts[0], |m, p| if p[k] > m[k] { *p } else { m });
    let at = |c: [f64; 3]| {
        if a.three_d {
            format!("x = {}{}y = {}", value(w, a, 0, c[0]), w.between(), value(w, a, 1, c[1]))
        } else {
            format!("x = {}", value(w, a, 0, c[0]))
        }
    };
    if (hi[k] - lo[k]).abs() <= 1e-12 * hi[k].abs().max(1.0) {
        return w.t("extremes.flat", &[var, &value(w, a, k, lo[k])]);
    }
    w.t("extremes", &[var, &value(w, a, k, lo[k]), &at(lo), &value(w, a, k, hi[k]), &at(hi)])
}

fn extent(pts: &[[f64; 3]], k: usize) -> (f64, f64) {
    pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| (lo.min(p[k]), hi.max(p[k])))
}

/// A coordinate as the axis shows it: a category's name, or the number, as precise as the axis'
/// range calls for.
fn value(w: &Words, a: &AxisFact, k: usize, v: f64) -> String {
    let names = &a.symbolic[k];
    if !names.is_empty() && (v - v.round()).abs() < 1e-9 && v >= 0.0 && (v as usize) < names.len() {
        return names[v as usize].clone();
    }
    match a.range[k] {
        Some([lo, hi]) if !a.log[k] => w.num(v, hi - lo),
        _ => w.num(v, v),
    }
}
