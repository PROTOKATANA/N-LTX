use tikra_engine::tex::{self, Cat, Token};
use tikra_engine::Error;

use super::super::keys;
use super::super::table::{self, Table};
use super::super::Engine;
use super::adjust::{symbolic_label, symbolic_meta};
use super::sample::fpu_vars;
use super::{add_label, Coord};

/// Where one coordinate of a table row comes from.
#[derive(Clone)]
pub(super) enum Src {
    Col(Option<usize>),
    Expr(Vec<Token>),
    /// A column made for the plot (`create col/linear regression`).
    Values(Vec<f64>),
    None,
}

/// The columns a table plot reads.
pub(super) struct TableCols {
    x: Src,
    y: Src,
    z: Src,
    meta: Src,
    tie: Option<[Src; 3]>,
    ey: Src,
    ex: Src,
    eyp: Src,
    eym: Src,
    exp: Src,
    exm: Src,
    quiver: bool,
    symbolic: Option<(Vec<Token>, bool)>,
    box_filter: Option<Vec<tex::Body>>,
}

/// One axis key (`x`, `y expr`, `x index`, `create col/linear regression`).
pub(super) struct ColumnPick<'a> {
    table: &'a Table,
    axis: &'a str,
    default: Option<usize>,
}

pub(super) fn column_source(e: &mut Engine, pick: &ColumnPick<'_>) -> Src {
    let t = pick.table;
    let axis = pick.axis;
    if let Some(expr) = keys::value_tokens(e, &format!("/pgfplots/table/{axis} expr")) {
        return Src::Expr(expr);
    }
    let name = keys::value(e, &format!("/pgfplots/table/{axis}"));
    let created = name.trim().trim_start_matches('{').trim_start();
    if let Some(options) = created.strip_prefix("create col/linear regression") {
        // `={y=…}`: the options, up to their closing brace
        let options = options.trim().trim_start_matches('=').trim();
        let options = match options.strip_prefix('{') {
            Some(inner) => {
                let mut depth = 1;
                let end = inner
                    .char_indices()
                    .find(|(_, c)| {
                        match c {
                            '{' => depth += 1,
                            '}' => depth -= 1,
                            _ => {}
                        }
                        depth == 0
                    })
                    .map_or(inner.len(), |(i, _)| i);
                &inner[..end]
            }
            None => options.trim_end_matches('}'),
        };
        return Src::Values(linear_regression(e, t, options));
    }
    if !name.is_empty() {
        return Src::Col(t.column(&name));
    }
    if let Some(i) = keys::number(e, &format!("/pgfplots/table/{axis} index")) {
        return Src::Col(Some(i as usize));
    }
    match pick.default {
        Some(d) => Src::Col(Some(d)),
        None => Src::None,
    }
}

/// `tie end x` (or its index), else the column three after the axis' own.
pub(super) fn tie_ends(e: &mut Engine, t: &Table, own: &[Src; 3]) -> Option<[Src; 3]> {
    if keys::value(e, "/tikz/@plot handler") != "tieline" {
        return None;
    }
    let mut ends = [Src::None, Src::None, Src::None];
    for (k, c) in ["x", "y", "z"].iter().enumerate() {
        let name = keys::value(e, &format!("/pgfplots/table/tie end {c}"));
        ends[k] = if !name.is_empty() {
            Src::Col(t.column(&name))
        } else if let Some(i) = keys::number(e, &format!("/pgfplots/table/tie end {c} index")) {
            Src::Col(Some(i as usize))
        } else {
            match &own[k] {
                Src::Col(Some(i)) if i + 3 < t.columns() => Src::Col(Some(i + 3)),
                _ => Src::None,
            }
        };
    }
    Some(ends)
}

pub(super) fn table_columns(e: &mut Engine, t: &Table, three_d: bool) -> TableCols {
    let x = column_source(e, &ColumnPick { table: t, axis: "x", default: Some(0) });
    let y = column_source(e, &ColumnPick { table: t, axis: "y", default: Some(1) });
    let z = column_source(e, &ColumnPick { table: t, axis: "z", default: if three_d { Some(2) } else { None } });
    let symbolic = symbolic_meta(e);
    let meta = match tikra_engine::keys::value(&e.tex, "/pgfplots/point meta") {
        _ if symbolic.is_some() => Src::None,
        Some(v) if tex::tokens_to_string(&v).contains("\\thisrow") => Src::Expr(v),
        _ => column_source(e, &ColumnPick { table: t, axis: "meta", default: None }),
    };
    // a tieline plot reads the tie's other end of each row too
    let tie = tie_ends(e, t, &[x.clone(), y.clone(), z.clone()]);
    if tie.as_ref().is_some_and(|t| t.iter().any(|s| matches!(s, Src::None | Src::Col(None)))) {
        e.warn("tieline: the tie ends' columns are not found (give 'tie end x' and its friends): ties not drawn");
    }
    let box_filter = match e.tex.meaning_of(&tikra_engine::keys::macro_name("/pgfplots/boxplot/data filter/.@cmd")) {
        tex::Meaning::Macro(m) if !m.body.is_empty() && !keys::value(e, "/pgfplots/@boxplot").is_empty() => Some(m.body.clone()),
        _ => None,
    };
    TableCols {
        x,
        y,
        z,
        meta,
        tie,
        ey: column_source(e, &ColumnPick { table: t, axis: "y error", default: None }),
        ex: column_source(e, &ColumnPick { table: t, axis: "x error", default: None }),
        eyp: column_source(e, &ColumnPick { table: t, axis: "y error plus", default: None }),
        eym: column_source(e, &ColumnPick { table: t, axis: "y error minus", default: None }),
        exp: column_source(e, &ColumnPick { table: t, axis: "x error plus", default: None }),
        exm: column_source(e, &ColumnPick { table: t, axis: "x error minus", default: None }),
        quiver: keys::flag(e, "/pgfplots/@quiver"),
        symbolic,
        box_filter,
    }
}

/// One row of the table, with the columns chosen for the plot.
pub(super) struct RowSrc<'a> {
    table: &'a Table,
    cols: &'a TableCols,
    row: usize,
}

impl RowSrc<'_> {
    /// The number a source gives for this row. An expression sees `known` (`x`, `y` so far).
    fn number(&self, e: &mut Engine, s: &Src, known: &[(&str, f64)]) -> f64 {
        match s {
            Src::Col(Some(c)) => self.table.cell(self.row, *c).and_then(table::parse_number).unwrap_or(f64::NAN),
            Src::Values(v) => v.get(self.row).copied().unwrap_or(f64::NAN),
            Src::Col(None) | Src::None => f64::NAN,
            Src::Expr(tokens) => {
                let replaced = replace_thisrow(tokens, self.table, self.row);
                let mut vars = fpu_vars(e);
                for (n, v) in known {
                    vars.insert((*n).into(), *v);
                }
                e.math_string(replaced).and_then(|s| tikra_engine::expr::eval(&s, &vars)).unwrap_or(f64::NAN)
            }
        }
    }

    /// A cell of a symbolic axis (or of a histogram's symbolic data) is a name.
    fn named(&self, e: &mut Engine, s: &Src, c: char, v: f64) -> f64 {
        match s {
            Src::Col(Some(col)) if !v.is_finite() => {
                self.table.cell(self.row, *col).and_then(|cell| super::super::symbolic::index(e, c, cell).or_else(|| super::super::symbolic::hist_index(e, cell))).unwrap_or(v)
            }
            _ => v,
        }
    }
}

/// `x` and `y` already read, for an expression later in the same row.
pub(super) struct RowKnown<'a> {
    src: &'a RowSrc<'a>,
    known: &'a [(&'a str, f64)],
}

pub(super) fn apply_row_quiver(e: &mut Engine, src: &RowSrc<'_>, c: &mut Coord) {
    if !src.cols.quiver {
        return;
    }
    for (k, name) in ["u", "v", "w"].iter().enumerate() {
        if let Some(toks) = keys::value_tokens(e, &format!("/pgfplots/quiver/{name}")) {
            let replaced = replace_thisrow(&toks, src.table, src.row);
            let mut vars = fpu_vars(e);
            vars.insert("x".into(), c.x);
            vars.insert("y".into(), c.y);
            vars.insert("z".into(), c.z);
            c.uvw[k] = e.math_string(replaced).and_then(|s| tikra_engine::expr::eval(&s, &vars)).unwrap_or(f64::NAN);
        }
    }
}

pub(super) fn apply_symbolic_meta(e: &mut Engine, src: &RowSrc<'_>, c: &mut Coord) {
    let Some((tokens, mathparse)) = &src.cols.symbolic else { return };
    let replaced = replace_thisrow(tokens, src.table, src.row);
    let text = e.tex.expand_to_string(replaced).unwrap_or_default();
    let l = symbolic_label(e, &text, *mathparse, [c.x, c.y, c.z]);
    c.label = add_label(e, tex::string_tokens(&l));
}

pub(super) fn apply_row_errors(e: &mut Engine, row: &RowKnown<'_>, c: &mut Coord) {
    let src = row.src;
    let ey = src.number(e, &src.cols.ey, row.known);
    let ex = src.number(e, &src.cols.ex, row.known);
    c.err = [ex, ex, ey, ey];
    let (p, m) = (src.number(e, &src.cols.eyp, row.known), src.number(e, &src.cols.eym, row.known));
    if p.is_finite() {
        c.err[3] = p;
    }
    if m.is_finite() {
        c.err[2] = m;
    }
    let (p, m) = (src.number(e, &src.cols.exp, row.known), src.number(e, &src.cols.exm, row.known));
    if p.is_finite() {
        c.err[1] = p;
    }
    if m.is_finite() {
        c.err[0] = m;
    }
}

pub(super) fn tie_point(e: &mut Engine, row: &RowKnown<'_>, c: &Coord) -> Option<Coord> {
    let ends = row.src.cols.tie.as_ref()?;
    let mut b = *c;
    b.x = row.src.number(e, &ends[0], row.known);
    b.y = row.src.number(e, &ends[1], row.known);
    b.z = row.src.number(e, &ends[2], row.known);
    Some(b)
}

/// `boxplot/data filter/.code`: the row's value in `\pgfmathresult` (empty: left out).
pub(super) fn apply_box_filter(e: &mut Engine, src: &RowSrc<'_>, c: &mut Coord) -> Result<(), Error> {
    let Some(body) = &src.cols.box_filter else { return Ok(()) };
    let data = keys::value(e, "/pgfplots/boxplot/data");
    let v = match data.as_str() {
        "x" => &mut c.x,
        "z" => &mut c.z,
        _ => &mut c.y,
    };
    if v.is_finite() {
        let text = tikra_engine::engine::pgf_number(*v);
        let code = replace_thisrow_as(&tex::substitute(body, &[tex::string_tokens(&text)]), src.table, src.row, true);
        e.begin_group();
        e.tex.define_string("pgfmathresult", &text, false);
        let res = e.run_tokens(code).and_then(|_| e.tex.expand_to_string(vec![Token::cs("pgfmathresult")]));
        e.end_group();
        *v = table::parse_number(&res?).unwrap_or(f64::NAN);
    }
    Ok(())
}

/// One table row: its point, and the tie's other end when the plot is a tieline.
pub(super) fn read_row(e: &mut Engine, src: &RowSrc<'_>) -> Result<Vec<Coord>, Error> {
    // an expression may use the coordinates read so far: `z expr=x^2-y^2`
    let mut known: Vec<(&str, f64)> = Vec::new();
    let x = src.number(e, &src.cols.x, &known);
    let x = src.named(e, &src.cols.x, 'x', x);
    known.push(("x", x));
    let y = src.number(e, &src.cols.y, &known);
    let y = src.named(e, &src.cols.y, 'y', y);
    known.push(("y", y));
    let z = if matches!(src.cols.z, Src::None) { 0.0 } else { src.number(e, &src.cols.z, &known) };
    let mut c = Coord::new(x, y, z);
    c.meta = src.number(e, &src.cols.meta, &known);
    if let Src::Col(Some(col)) = &src.cols.meta {
        if let Some(cell) = src.table.cell(src.row, *col) {
            c.label = add_label(e, tex::string_tokens(cell));
        }
    }
    apply_row_quiver(e, src, &mut c);
    apply_symbolic_meta(e, src, &mut c);
    let row = RowKnown { src, known: &known };
    apply_row_errors(e, &row, &mut c);
    let other = tie_point(e, &row, &c);
    apply_box_filter(e, src, &mut c)?;
    let mut out = vec![c];
    out.extend(other);
    Ok(out)
}

/// The coordinates of a table: the `x`, `y`, `z` columns (by name, index or expression).
pub(super) fn table_coords(e: &mut Engine, t: &Table, three_d: bool) -> Result<Vec<Coord>, Error> {
    let cols = table_columns(e, t, three_d);
    let mut out = Vec::with_capacity(t.rows.len());
    let mut rows = Vec::with_capacity(t.rows.len());
    for r in 0..t.rows.len() {
        let points = read_row(e, &RowSrc { table: t, cols: &cols, row: r })?;
        for _ in &points {
            rows.push(r);
        }
        out.extend(points);
    }
    super::super::fig(e).filter_rows = Some((t.clone(), rows));
    Ok(out)
}

/// `create col/linear regression={y=…}`: the least squares line through the table's points (in
/// logarithms on log axes, weighted by `variance` or `variance list`), at every row's x; its slope
/// and intercept are kept in `\pgfplotstableregressiona` and `\pgfplotstableregressionb`.
pub(super) fn linear_regression(e: &mut Engine, t: &Table, options: &str) -> Vec<f64> {
    let mut opt = std::collections::HashMap::new();
    for part in tikra_engine::keys::split_top_level(options) {
        if let Some((k, v)) = part.split_once('=') {
            opt.insert(k.trim().to_string(), v.trim().trim_start_matches('{').trim_end_matches('}').trim().to_string());
        }
    }
    let column = |name: Option<&String>, fallback: &str, index: usize| -> Option<usize> {
        let n = name.cloned().unwrap_or_else(|| fallback.to_string());
        if n.is_empty() {
            Some(index)
        } else {
            t.column(&n)
        }
    };
    let plot_x = keys::value(e, "/pgfplots/table/x");
    let xs = column(opt.get("x"), &plot_x, 0);
    let ys = column(opt.get("y"), "", 1);
    let (Some(xc), Some(yc)) = (xs, ys) else {
        e.warn("create col/linear regression: column not found");
        return vec![f64::NAN; t.rows.len()];
    };
    // the modes: given, else the axis'
    let mode = |e: &mut Engine, c: char| -> Option<f64> {
        let m = opt.get(&format!("{c}mode")).cloned().unwrap_or_default();
        let log = match m.as_str() {
            "log" => true,
            "linear" => false,
            _ => keys::value(e, &format!("/pgfplots/{c}mode")) == "log",
        };
        // the logarithm's base: `log basis`, else e
        log.then(|| keys::number(e, &format!("/pgfplots/log basis {c}")).filter(|b| *b > 0.0 && *b != 1.0).unwrap_or(std::f64::consts::E))
    };
    let (xlog, ylog) = (mode(e, 'x'), mode(e, 'y'));
    let linear_variance = opt.get("variance format").cloned().unwrap_or_else(|| keys::value(e, "/pgfplots/table/create col/linear regression/variance format")) == "linear";
    let mut variances: std::collections::VecDeque<f64> = match opt.get("variance list") {
        Some(list) => list.split(',').filter_map(|v| e.number(v.trim()).ok()).collect(),
        None => match opt.get("variance").and_then(|n| t.column(n)) {
            Some(c) => (0..t.rows.len()).map(|r| t.cell(r, c).and_then(table::parse_number).unwrap_or(1.0)).collect(),
            None => Default::default(),
        },
    };
    let log_of = |v: f64, base: Option<f64>| match base {
        Some(b) => v.ln() / b.ln(),
        None => v,
    };
    let (mut s, mut sx, mut sxx, mut sy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut parsed = Vec::with_capacity(t.rows.len());
    for r in 0..t.rows.len() {
        let x = t.cell(r, xc).and_then(table::parse_number).unwrap_or(f64::NAN);
        let y = t.cell(r, yc).and_then(table::parse_number).unwrap_or(f64::NAN);
        if !(x.is_finite() && y.is_finite()) {
            parsed.push(f64::NAN);
            continue;
        }
        let (px, py) = (log_of(x, xlog), log_of(y, ylog));
        parsed.push(px);
        let var = variances.pop_front().unwrap_or(1.0);
        let mut w = 1.0 / (var * var);
        // a variance of y given for y itself: its logarithm's is dy / (y ln b)
        if let (Some(b), true) = (ylog, linear_variance) {
            w *= (y * b.ln()).powi(2);
        }
        s += w;
        sx += px * w;
        sxx += px * px * w;
        sy += py * w;
        sxy += px * py * w;
    }
    let delta = s * sxx - sx * sx;
    let a = (s * sxy - sx * sy) / delta;
    let b = (sxx * sy - sx * sxy) / delta;
    for (name, v) in [("pgfplotstableregressiona", a), ("pgfplotstableregressionb", b)] {
        e.tex.define_string(name, &format!("{v:e}"), true);
    }
    parsed
        .into_iter()
        .map(|px| {
            let v = a * px + b;
            match ylog {
                Some(base) => base.powf(v),
                None => v,
            }
        })
        .collect()
}

/// `\thisrow{name}`, `\thisrowno{i}`, `\coordindex`, `\lineno` replaced by the row's values.
pub(super) fn replace_thisrow(tokens: &[Token], t: &Table, row: usize) -> Vec<Token> {
    replace_thisrow_as(tokens, t, row, false)
}

/// The same, the cells as their text (`raw`) or as numbers in parentheses.
pub(super) fn replace_thisrow_as(tokens: &[Token], t: &Table, row: usize, raw: bool) -> Vec<Token> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let tk = &tokens[i];
        let (is_row, is_no) = (tk.is_cs("thisrow"), tk.is_cs("thisrowno"));
        if is_row || is_no {
            let mut j = i + 1;
            while j < tokens.len() && tokens[j].is_space() {
                j += 1;
            }
            let (arg, end) = if j < tokens.len() && matches!(tokens[j], Token::Char(_, Cat::Begin)) {
                let mut depth = 0;
                let mut k = j;
                loop {
                    match &tokens[k] {
                        Token::Char(_, Cat::Begin) => depth += 1,
                        Token::Char(_, Cat::End) => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    k += 1;
                    if k >= tokens.len() {
                        break;
                    }
                }
                (tex::tokens_to_string(&tokens[j + 1..k.min(tokens.len())]), k + 1)
            } else {
                (tokens.get(j).map(|t| tex::tokens_to_string(std::slice::from_ref(t))).unwrap_or_default(), j + 1)
            };
            let col = if is_no { arg.trim().parse().ok() } else { t.column(arg.trim()) };
            if raw {
                let cell = col.and_then(|c| t.cell(row, c)).unwrap_or("");
                out.extend(cell.chars().map(|c| Token::Char(c, if c.is_alphabetic() { Cat::Letter } else { Cat::Other })));
                i = end;
                continue;
            }
            let v = col.and_then(|c| t.cell(row, c)).and_then(table::parse_number).unwrap_or(f64::NAN);
            out.extend(tex::string_tokens(&format!("({})", if v.is_nan() { "nan".to_string() } else { format!("{v}") })));
            i = end;
            continue;
        }
        if tk.is_cs("coordindex") || tk.is_cs("lineno") {
            out.extend(tex::string_tokens(&row.to_string()));
            i += 1;
            continue;
        }
        out.push(tk.clone());
        i += 1;
    }
    out
}
