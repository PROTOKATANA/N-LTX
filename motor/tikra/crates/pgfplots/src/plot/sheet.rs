use tikra_engine::tex::{self, Cat, Token};
use tikra_engine::Error;

use super::super::keys;
use super::super::table::{self, Table};
use super::super::Engine;
use super::rows::table_coords;
use super::Coord;

pub(super) fn search_path(e: &mut Engine) -> Vec<String> {
    let s = keys::value(e, "/pgfplots/table/@search path");
    s.split(',').map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
}

/// `[options]{file or data}` with line ends as characters (rows), and the data as text.
pub(super) fn table_source(e: &mut Engine) -> Result<(Option<Vec<Token>>, String), Error> {
    let saved = (e.tex.catcode('\n'), e.tex.catcode('\r'));
    e.tex.set_catcode('\n', Cat::Other);
    e.tex.set_catcode('\r', Cat::Other);
    let r = (|| -> Result<(Option<Vec<Token>>, Vec<Token>), Error> {
        // line ends before the options or the data are spaces still
        let next = |e: &mut Engine| loop {
            match e.tex.next_nonspace() {
                Some(Token::Char('\n' | '\r', Cat::Other)) => continue,
                t => break t,
            }
        };
        if let Some(t) = next(e) {
            e.tex.back(t);
        }
        let opts = e.optional()?;
        let data = match next(e) {
            Some(Token::Char(_, Cat::Begin)) => e.tex.read_group_body()?,
            // `table [options] from \macro`: a table read with \pgfplotstableread
            Some(Token::Char('f', c)) => {
                let mut taken = vec![Token::Char('f', c)];
                for want in ['r', 'o', 'm'] {
                    match e.tex.get_token() {
                        Some(t @ Token::Char(x, _)) if x == want => taken.push(t),
                        Some(t) => {
                            taken.push(t);
                            break;
                        }
                        None => break,
                    }
                }
                if taken.len() == 4 {
                    match next(e) {
                        Some(Token::Char(_, Cat::Begin)) => e.tex.read_group_body()?,
                        Some(t) => vec![t],
                        None => Vec::new(),
                    }
                } else {
                    for t in taken.into_iter().skip(1).rev() {
                        e.tex.back(t);
                    }
                    vec![Token::Char('f', c)]
                }
            }
            Some(t) => vec![t],
            None => Vec::new(),
        };
        Ok((opts, data))
    })();
    e.tex.set_catcode('\n', saved.0);
    e.tex.set_catcode('\r', saved.1);
    let (opts, mut data) = r?;
    // a file name with macros (`plotdata/experiment\n.dat` in a \foreach) is expanded; a table
    // kept by \pgfplotstableread (one macro) and inline data are not
    let significant: Vec<&Token> = data.iter().filter(|t| !t.is_space()).collect();
    let one_line = !data.iter().any(|t| matches!(t, Token::Char('\n' | '\r', _)) || t.is_cs("\\"));
    if one_line && significant.len() > 1 && significant.iter().any(|t| matches!(t, Token::Cs(_))) {
        data = super::super::toks(&e.tex.expand_to_string(data)?);
    }
    let mut text = String::new();
    for t in &data {
        match t {
            Token::Char(c, _) => text.push(*c),
            // `\\` kept as written: a row's end with `row sep=\\`
            Token::Cs(n) if &**n == "\\" => text.push_str("\\\\"),
            Token::Cs(n) => {
                text.push('\\');
                text.push_str(n);
                text.push(' ');
            }
        }
    }
    Ok((opts, text))
}

/// `\pgfplotstableread[options]{file or data}\macro`: the table, kept for `table {\macro}`.
pub fn table_read(e: &mut Engine) -> Result<(), Error> {
    let (opts, text) = table_source(e)?;
    let name = match e.tex.next_nonspace() {
        Some(Token::Cs(n)) => n.to_string(),
        Some(Token::Char(_, Cat::Begin)) => {
            let body = e.tex.read_group_body()?;
            match body.iter().find(|t| matches!(t, Token::Cs(_))) {
                Some(Token::Cs(n)) => n.to_string(),
                _ => String::new(),
            }
        }
        _ => String::new(),
    };
    if name.is_empty() {
        e.warn("\\pgfplotstableread without a macro to keep the table: ignored");
        return Ok(());
    }
    super::super::fig(e).tables.insert(name, (text, opts.unwrap_or_default()));
    Ok(())
}

/// `table[options]{file or data}`.
pub(super) fn read_table(e: &mut Engine, three_d: bool) -> Result<(Vec<Coord>, Vec<usize>), Error> {
    let (opts, text) = table_source(e)?;
    let Some(table) = load_table(e, opts, text, "\\addplot table")? else { return Ok((Vec::new(), Vec::new())) };
    Ok((table_coords(e, &table, three_d)?, table.scanlines.clone()))
}

/// The table of a file, of inline data or of a `\pgfplotstableread` macro, read with the options
/// (under `/pgfplots/table`); none (with a warning) if it cannot be read.
pub(super) fn load_table(e: &mut Engine, opts: Option<Vec<Token>>, mut text: String, what: &str) -> Result<Option<Table>, Error> {
    // a table `\pgfplotstableread` kept: its options, then the plot's
    let kept = text.trim().strip_prefix('\\').map(|n| n.trim().to_string()).and_then(|n| super::super::fig(e).tables.get(&n).cloned());
    let mut all_opts = Vec::new();
    if let Some((t, o)) = kept {
        text = t;
        all_opts.extend(o);
        all_opts.extend(super::super::toks(","));
    }
    all_opts.extend(opts.unwrap_or_default());
    if !all_opts.is_empty() {
        let saved = e.st.key_path.clone();
        e.process_keys(all_opts, Some("/pgfplots/table"))?;
        e.st.key_path = saved;
    }
    let row_sep = keys::value(e, "/pgfplots/table/row sep");
    if row_sep.contains("\\\\") {
        // the rows end at `\\`, not at the line ends
        text = text.replace('\n', " ");
    }
    text = text.replace("\\\\", "\n");
    let sep = table::col_sep(&keys::value(e, "/pgfplots/table/col sep"));
    let header = keys::value(e, "/pgfplots/table/header");
    let skip = keys::number(e, "/pgfplots/table/skip first n").unwrap_or(0.0) as usize;
    let comment = keys::value(e, "/pgfplots/table/comment chars");
    let trimmed = text.trim();
    let is_file = !trimmed.contains('\n') && !trimmed.contains(' ') && !trimmed.is_empty() && trimmed.chars().any(|c| c.is_alphabetic());
    // a file the document wrote (`\pgfplotscolormaptodatafile`)
    let written = format!("tikra@file@{trimmed}");
    let table = if is_file && e.tex.is_defined(&written) {
        let content = e.tex.expand_to_string(vec![Token::cs(&written)])?;
        Table::parse(&content, sep, &header, skip, &comment)
    } else if is_file {
        match tikra_engine::files::read_file(trimmed, &search_path(e)) {
            Ok(content) => Table::parse(&content, sep, &header, skip, &comment),
            Err(m) => {
                e.warn(format!("{what}: {m}"));
                return Ok(None);
            }
        }
    } else if trimmed.starts_with('\\') {
        e.warn(format!("{what} {trimmed}: tables read with \\pgfplotstableread are not supported"));
        return Ok(None);
    } else {
        Table::parse(&text, sep, &header, skip, &comment)
    };
    Ok(Some(table))
}

/// `\pgfplotstabletypeset[options]{table}`: the table as text, a `tabular` of its columns
/// (`columns={a,b}`, else all) under their names, the numbers printed as `\pgfmathprintnumber`
/// prints them, each column centered.
pub fn table_typeset(e: &mut Engine) -> Result<(), Error> {
    let (opts, text) = table_source(e)?;
    // `columns` is pgfplotstable's own; the rest is for reading the table
    let mut columns: Option<Vec<String>> = None;
    let mut rest = Vec::new();
    for item in tikra_engine::keys::split_list(&opts.unwrap_or_default()) {
        if item.key.trim() == "columns" {
            let v = item.value.map(|v| tex::tokens_to_string(&v)).unwrap_or_default();
            columns = Some(tikra_engine::keys::split_top_level(&v).iter().map(|c| tikra_engine::keys::strip_braces(c.trim()).trim().to_string()).collect());
            continue;
        }
        rest.extend(super::super::toks(&item.key));
        if let Some(v) = item.value {
            rest.extend(super::super::toks("={"));
            rest.extend(v);
            rest.extend(super::super::toks("}"));
        }
        rest.extend(super::super::toks(","));
    }
    e.begin_group();
    let table = load_table(e, Some(rest), text, "\\pgfplotstabletypeset");
    e.end_group();
    let Some(table) = table? else { return Ok(()) };
    let picked: Vec<usize> = match &columns {
        Some(names) => names.iter().filter_map(|n| table.names.iter().position(|m| m == n)).collect(),
        None => (0..table.names.len()).collect(),
    };
    let mut body = format!("\\begin{{tabular}}{{{}}}", "c".repeat(picked.len()));
    body.push_str(&picked.iter().map(|&k| table.names[k].clone()).collect::<Vec<_>>().join(" & "));
    for row in &table.rows {
        body.push_str("\\\\");
        let cells: Vec<String> = picked
            .iter()
            .map(|&k| {
                let cell = row.get(k).map_or("", |c| c.trim());
                match table::parse_number(cell) {
                    Some(v) => format!("${}$", e.st.number_format.print(v)),
                    None => cell.to_string(),
                }
            })
            .collect();
        body.push_str(&cells.join(" & "));
    }
    body.push_str("\\end{tabular}");
    e.typeset_word(body);
    Ok(())
}
