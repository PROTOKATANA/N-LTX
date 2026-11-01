use tikra_engine::tex::{self, Cat, Token};
use tikra_engine::Error;

use super::super::keys;
use super::super::table::{self, Table};
use super::super::Engine;
use super::adjust::{coordinate_filters, data_cs, filter, histogram, lattice_from_scanlines, point_meta, quiver_vectors};
use super::coords::coordinates;
use super::rows::table_coords;
use super::sample::{sample, split_commas};
use super::sheet::{read_table, search_path};
use super::{word, Plot};

/// Reads the plot's input and trailing path, with its keys set (inside the plot's group).
pub(super) fn survey(e: &mut Engine, style: &[Token], three_d: bool) -> Result<Plot, Error> {
    e.st.mode = Default::default();
    e.process_keys(style.to_vec(), Some("/pgfplots"))?;
    if let Some(code) = keys::value_tokens(e, "/pgfplots/execute at begin plot") {
        e.run_tokens(code)?;
    }
    let mut plot = Plot { three_d, ..Default::default() };
    super::super::fig(e).labels.clear();
    let r = survey_input(e, &mut plot, three_d);
    plot.labels = std::mem::take(&mut super::super::fig(e).labels);
    r?;
    Ok(plot)
}

/// Copies the plot's keys (`forget plot`, mesh, scatter, stack, the prepared contour's meta).
pub(super) fn read_plot_keys(e: &mut Engine, plot: &mut Plot, three_d: bool) {
    plot.forget = keys::flag(e, "/pgfplots/forget plot");
    plot.keep_limits = !keys::flag(e, "/pgfplots/update limits");
    plot.mesh = match keys::value(e, "/pgfplots/@mesh").as_str() {
        "m" => Some('m'),
        "s" => Some('s'),
        _ => None,
    };
    plot.patch = keys::flag(e, "/pgfplots/@patch");
    plot.image = plot.mesh.is_some() && keys::value(e, "/pgfplots/mesh input") == "image";
    plot.late = keys::value_tokens(e, "/pgfplots/late options").unwrap_or_default();
    plot.scatter = keys::flag(e, "/pgfplots/scatter");
    plot.handler = keys::value(e, "/tikz/@plot handler");
    let axis_options = super::super::fig(e).axis.as_ref().map(|a| tex::tokens_to_string(&a.options)).unwrap_or_default();
    plot.bar_shift_given = axis_options.contains("bar shift");
    // a prepared contour: its level is z in 3d, the third column (point meta=explicit) in 2d
    if plot.handler == "contour prepared" && keys::value(e, "/pgfplots/point meta").is_empty() {
        if three_d {
            keys::set(e, "/pgfplots/point meta", "z");
        } else if keys::value(e, "/pgfplots/@contour format") != "matlab" {
            keys::set(e, "/pgfplots/table/meta index", "2");
            keys::set(e, "/pgfplots/point meta", "explicit");
        }
    }
    plot.stack = match keys::value(e, "/pgfplots/stack plots").as_str() {
        "x" => Some('x'),
        "y" => Some('y'),
        _ => None,
    };
    plot.stack_minus = keys::value(e, "/pgfplots/stack dir") == "minus";
}

/// A formula sampled as `(x, y[, z])` or, in 3d, `(x, y, z)`.
pub(super) struct Sampled {
    three_d: bool,
    body: Vec<Token>,
}

pub(super) fn sample_formula(e: &mut Engine, plot: &mut Plot, sampled: Sampled) -> Result<(), Error> {
    plot.expression = tex::tokens_to_string(&sampled.body);
    let parts = if sampled.three_d { vec![super::super::toks("x"), super::super::toks("y"), sampled.body] } else { vec![super::super::toks("x"), sampled.body] };
    plot.coords = sample(e, &parts, plot)?;
    Ok(())
}

/// `\addplot expression[domain=…] {…}`.
pub(super) fn read_expression_word(e: &mut Engine, plot: &mut Plot, three_d: bool) -> Result<(), Error> {
    if let Some(o) = e.optional()? {
        e.process_keys(o, Some("/pgfplots"))?;
    }
    let body = match e.tex.next_nonspace() {
        Some(Token::Char(_, Cat::Begin)) => e.tex.read_group_body()?,
        _ => return Err(Error::Figure("expression without a formula".into())),
    };
    sample_formula(e, plot, Sampled { three_d, body })
}

/// `\addplot file {name}`: a table of spaces.
pub(super) fn read_file_plot(e: &mut Engine, plot: &mut Plot, three_d: bool) -> Result<(), Error> {
    let _ = e.optional()?;
    let name = e.argument_string()?;
    let search = search_path(e);
    match tikra_engine::files::read_file(&name, &search) {
        Ok(text) => {
            let t = Table::parse(&text, table::ColSep::Space, "false", 0, "");
            plot.coords = table_coords(e, &t, three_d)?;
            plot.scanlines = t.scanlines.clone();
        }
        Err(m) => e.warn(format!("\\addplot file: {m}")),
    }
    Ok(())
}

/// The input after the options: `(…)`, `{…}`, `coordinates`, `table`, `expression`, `file`, …
pub(super) fn read_plot_source(e: &mut Engine, plot: &mut Plot, three_d: bool) -> Result<(), Error> {
    let t = e.tex.next_nonspace().ok_or_else(|| Error::Figure("\\addplot without input".into()))?;
    match &t {
        Token::Char('(', _) => {
            let tokens = tikra_engine::engine::coords::read_coordinate_tokens(e)?;
            let parts = split_commas(&tokens);
            plot.coords = sample(e, &parts, plot)?;
        }
        Token::Char(_, Cat::Begin) => {
            let body = e.tex.read_group_body()?;
            sample_formula(e, plot, Sampled { three_d, body })?;
        }
        Token::Char(_, Cat::Letter) => {
            e.tex.back(t.clone());
            let w = word(e);
            match w.as_str() {
                "coordinates" => {
                    let list = match e.tex.next_nonspace() {
                        Some(Token::Char(_, Cat::Begin)) => e.tex.read_group_body()?,
                        _ => return Err(Error::Figure("coordinates without a list".into())),
                    };
                    (plot.coords, plot.scanlines) = coordinates(e, list)?;
                }
                "table" => {
                    (plot.coords, plot.scanlines) = read_table(e, three_d)?;
                }
                // `\addplot expression[domain=0:3] {…}`: options for the sampling
                "expression" => read_expression_word(e, plot, three_d)?,
                "file" => read_file_plot(e, plot, three_d)?,
                "fill" => {
                    let between = word_after_space(e);
                    if between != "between" {
                        return Err(Error::Figure(format!("\\addplot fill {between} not supported")));
                    }
                    plot.fill_between = Some(e.optional()?.unwrap_or_default());
                }
                "graphics" => super::super::graphics::survey(e, plot, three_d)?,
                "gnuplot" | "shell" | "function" => {
                    let _ = e.optional()?;
                    let _ = e.argument()?;
                    e.warn(format!("\\addplot {w} not supported: it needs an external program or file"));
                }
                other => return Err(Error::Figure(format!("\\addplot input '{other}' not supported"))),
            }
        }
        other => return Err(Error::Figure(format!("\\addplot input '{}' not supported", tex::show_token(other)))),
    }
    Ok(())
}

/// The trailing path, up to the `;`.
pub(super) fn trailing_path(e: &mut Engine) -> Vec<Token> {
    let mut post = Vec::new();
    let mut depth = 0usize;
    while let Some(x) = e.tex.get_token() {
        match &x {
            Token::Char(_, Cat::Begin) => depth += 1,
            Token::Char(_, Cat::End) => {
                if depth == 0 {
                    e.tex.back(x);
                    break;
                }
                depth -= 1;
            }
            _ => {}
        }
        if depth == 0 && x.is_char(';') {
            break;
        }
        post.push(x);
        if post.len() > tex::MAX_TOKENS {
            break;
        }
    }
    post
}

/// What the survey does once the points exist: coordinate system, quiver, histogram, meta, filters.
pub(super) fn finish_survey(e: &mut Engine, plot: &mut Plot) -> Result<(), Error> {
    data_cs(e, plot);
    // `is smithchart cs`: the input is in the unit circle's plane
    if super::super::fig(e).axis.as_ref().is_some_and(|a| a.kind == "smithchart") && keys::flag(e, "/pgfplots/is smithchart cs") {
        for c in plot.coords.iter_mut() {
            (c.x, c.y) = super::super::smith::impedance(c.x, c.y);
        }
    }
    if plot.handler == "contour prepared" && keys::value(e, "/pgfplots/@contour format") == "matlab" {
        super::super::contour::from_matlab(plot);
        keys::set(e, "/pgfplots/point meta", "explicit");
    }
    if keys::flag(e, "/pgfplots/@quiver") {
        quiver_vectors(e, plot);
    }
    if keys::flag(e, "/pgfplots/@hist") {
        histogram(e, plot)?;
    }
    match keys::value(e, "/pgfplots/@boxplot").as_str() {
        "raw" => super::super::boxplot::survey(e, plot, false),
        "prepared" => super::super::boxplot::survey(e, plot, true),
        _ => {}
    }
    point_meta(e, plot)?;
    if plot.patch {
        patch_table(e, plot);
    }
    lattice_from_scanlines(e, plot);
    coordinate_filters(e, plot)?;
    filter(e, plot)?;
    if let Some(code) = keys::value_tokens(e, "/pgfplots/execute at end plot") {
        e.run_tokens(code)?;
    }
    Ok(())
}

pub(super) fn survey_input(e: &mut Engine, plot: &mut Plot, three_d: bool) -> Result<(), Error> {
    read_plot_keys(e, plot, three_d);
    read_plot_source(e, plot, three_d)?;
    plot.post = trailing_path(e);
    finish_survey(e, plot)
}

/// `patch table={0 1 2\\ 1 2 3\\}` (or a file name): the patches are rows of indices into the
/// points read; `patch table with point meta` adds a color to each row, `… with individual point
/// meta` one to each of its corners. The points become the patches' corners, one after another,
/// each with its color.
pub(super) fn patch_table(e: &mut Engine, plot: &mut Plot) {
    let mut found = None;
    for (key, kind) in [("patch table", 0), ("patch table with point meta", 1), ("patch table with individual point meta", 2)] {
        let v = keys::value(e, &format!("/pgfplots/{key}"));
        if !v.trim().is_empty() {
            found = Some((v, kind));
        }
    }
    let Some((text, kind)) = found else { return };
    let text = text.replace("\\\\", "\n");
    let trimmed = text.trim();
    let text = if !trimmed.contains('\n') && !trimmed.contains(' ') && trimmed.chars().any(|c| c.is_alphabetic()) {
        match tikra_engine::files::read_file(trimmed, &search_path(e)) {
            Ok(t) => t,
            Err(m) => {
                e.warn(format!("patch table: {m}"));
                return;
            }
        }
    } else {
        text
    };
    let t = Table::parse(&text, table::ColSep::Space, "false", 0, "");
    let mut coords = Vec::new();
    for row in &t.rows {
        let v: Vec<f64> = row.iter().filter_map(|c| table::parse_number(c)).collect();
        let n = match kind {
            0 => v.len(),
            1 => v.len().saturating_sub(1),
            _ => v.len() / 2,
        };
        for (i, idx) in v[..n].iter().enumerate() {
            let Some(c) = plot.coords.get(*idx as usize) else { continue };
            let mut c = *c;
            match kind {
                1 => c.meta = v[n],
                2 => c.meta = v.get(n + i).copied().unwrap_or(f64::NAN),
                _ => {}
            }
            coords.push(c);
        }
    }
    plot.coords = coords;
    plot.scanlines.clear();
    if kind > 0 {
        plot.has_meta = true;
    }
}

pub(super) fn word_after_space(e: &mut Engine) -> String {
    if let Some(t) = e.tex.next_nonspace() {
        e.tex.back(t);
    }
    word(e)
}
