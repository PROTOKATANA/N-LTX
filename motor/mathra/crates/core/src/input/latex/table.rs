//! Environments, arrays, delimiters and amscd diagrams.

use super::*;

/// Columns of an array preamble, and the rules and material that sit between them.
pub(super) struct ArrayPreamble {
    columns: Vec<char>,
    verticals: Vec<usize>,
    dashed: Vec<usize>,
    between: Vec<(usize, Node)>,
}

/// One row of an amscd diagram, before its cells are read as math.
struct CdRow {
    objects: Vec<String>,
    arrows: Vec<Node>,
    vertical: bool,
}

impl Reader<'_> {
    pub(super) fn raw_measure(&mut self) -> String {
        self.skip_spaces();
        let mut s = String::new();
        while let Some(c) = self.current() {
            if c.is_ascii_digit() || c == '.' || c == '-' || c == ',' || (c.is_ascii_alphabetic() && s.len() < 12) {
                s.push(c);
                self.pos += 1;
                if s.ends_with("em") || s.ends_with("ex") || s.ends_with("pt") || s.ends_with("mu") || s.ends_with("cm") || s.ends_with("mm") || s.ends_with("in") {
                    break;
                }
            } else {
                break;
            }
        }
        s
    }

    /// A valid color, or None in tolerant mode when it is unusable (the content comes out uncolored).
    pub(super) fn color(&mut self) -> R<Option<String>> {
        let col = self.column();
        let model = self.raw_optional();
        let raw = self.raw_text()?.trim().to_string();
        let color = match model.as_deref() {
            Some("HTML") => format!("#{raw}"),
            Some("rgb") => format!("rgb({raw})"),
            _ => raw,
        };
        let valid = color.chars().all(|c| c.is_ascii_alphanumeric() || "#(),. ".contains(c)) && !color.is_empty() && color.len() < 40;
        if valid {
            return Ok(Some(color));
        }
        self.tolerate(rules::E_REC_9, col, format!("invalid color: {color}"), "drawn without color")?;
        Ok(None)
    }

    pub(super) fn delimiter(&mut self) -> R<Option<char>> {
        self.skip_spaces();
        let col = self.column();
        match self.current() {
            Some('\\') => {
                self.pos += 1;
                let name = self.read_command_name();
                match symbols::delimiter_command(&name) {
                    Some(d) => Ok(d),
                    None => {
                        self.tolerate(rules::E_REC_9, col, format!("\\{name} is not a delimiter"), "used the empty one")?;
                        Ok(None)
                    }
                }
            }
            Some(c) => {
                self.pos += 1;
                match symbols::delimiter_char(c) {
                    Some(d) => Ok(d),
                    None => {
                        self.tolerate(rules::E_REC_9, col, format!("'{c}' is not a delimiter"), "drawn as is")?;
                        Ok(Some(c))
                    }
                }
            }
            None => {
                self.tolerate(rules::E_REC_7, col, "missing delimiter", "used the empty one")?;
                Ok(None)
            }
        }
    }

    pub(super) fn delimited(&mut self, col: usize) -> R<Node> {
        let open = self.delimiter()?;
        let variant = self.variant;
        let mut parts = Vec::new();
        let mut middles = Vec::new();
        loop {
            let (items, stop) = self.list()?;
            parts.push(Node::row(items));
            match stop {
                Stop::Right(close) => {
                    self.variant = variant;
                    return Ok(Node::Delimited { open, close, parts, middles });
                }
                Stop::Middle(m) => middles.push(m.unwrap_or('|')),
                Stop::Eof => {
                    self.tolerate(rules::E_REC_5, col, "\\left without \\right", "closed with an empty delimiter")?;
                    self.variant = variant;
                    return Ok(Node::Delimited { open, close: None, parts, middles });
                }
                other => {
                    self.tolerate(rules::E_REC_5, col, format!("\\left opened here ended in {other:?}"), "closed there")?;
                    self.variant = variant;
                    return Ok(Node::Delimited { open, close: None, parts, middles });
                }
            }
        }
    }

    pub(super) fn environment(&mut self, col: usize) -> R<Node> {
        let name = self.raw_text()?.trim().to_string();
        let base = name.trim_end_matches('*');
        let mut preamble = ArrayPreamble { columns: Vec::new(), verticals: Vec::new(), dashed: Vec::new(), between: Vec::new() };
        let kind = match base {
            "matrix" => TableKind::Matrix,
            "pmatrix" => TableKind::PMatrix,
            "bmatrix" => TableKind::BMatrix,
            "Bmatrix" => TableKind::BraceMatrix,
            "vmatrix" => TableKind::VMatrix,
            "Vmatrix" => TableKind::VVMatrix,
            "smallmatrix" => TableKind::SmallMatrix,
            "cases" | "dcases" | "numcases" => TableKind::Cases,
            "rcases" | "drcases" => TableKind::RCases,
            "aligned" | "align" | "alignat" | "alignedat" | "split" | "eqnarray" | "flalign" | "flalignat" => TableKind::Aligned,
            "gathered" | "gather" | "multline" => TableKind::Centered,
            "array" | "tabular" => TableKind::Array,
            "subarray" => TableKind::Substack,
            "CD" => {
                let raw = self.environment_body("CD", col)?;
                return self.cd_diagram(&raw, col);
            }
            "tikzcd" => {
                let raw = self.environment_body("tikzcd", col)?;
                return self.tikzcd(&raw, col);
            }
            "subequations" => {
                let (items, stop) = self.list()?;
                return match stop {
                    Stop::End(end) if end == name => Ok(Node::row(items)),
                    _ => self.error(col, format!("\\begin{{{name}}} without \\end{{{name}}}")),
                };
            }
            "equation" | "displaymath" | "math" => {
                let (items, stop) = self.list()?;
                return match stop {
                    Stop::End(end) if end == name => Ok(Node::row(items)),
                    _ => self.error(col, format!("\\begin{{{name}}} without \\end{{{name}}}")),
                };
            }
            _ => {
                self.tolerate(rules::E_REC_6, col, format!("unsupported environment: {name}"), "read as a table")?;
                TableKind::Centered
            }
        };
        if matches!(base, "alignat" | "alignedat" | "flalignat") {
            self.raw_text()?;
        }
        if matches!(base, "pmatrix" | "bmatrix" | "matrix" | "vmatrix" | "Bmatrix" | "Vmatrix") && name.ends_with('*') {
            if let Some(a) = self.raw_optional() {
                preamble.columns = vec![a.trim().chars().next().unwrap_or('c'); 1];
            }
        }
        if matches!(kind, TableKind::Array | TableKind::Substack) {
            self.raw_optional();
            let spec = self.raw_text()?;
            self.array_columns(&spec, &mut preamble, col)?;
        }

        let (rows, horizontals, intertexts) = self.table_body(Some(&name), col)?;
        if !preamble.columns.is_empty() && kind != TableKind::Array {
            // pmatrix*[r]: the same alignment in every column
            let a = preamble.columns[0];
            let n = rows.iter().map(Vec::len).max().unwrap_or(1);
            preamble.columns = vec![a; n];
        }
        Ok(Node::Table {
            shape: kind,
            rows,
            columns: preamble.columns,
            verticals: preamble.verticals,
            dashed: preamble.dashed,
            horizontals,
            between: preamble.between,
            intertexts,
            stretch: self.array_stretch(),
        })
    }

    /// `\arraystretch` as the input defined it (`\def`, `\renewcommand`), or 1.
    pub(super) fn array_stretch(&self) -> f32 {
        self.macros.commands.get("arraystretch").and_then(|m| m.body.trim().parse::<f32>().ok()).filter(|v| v.is_finite() && *v > 0.0).map_or(1.0, |v| v.min(10.0))
    }

    /// Rows and cells up to `\end{name}` (or up to the end of the text, when `name` is None).
    #[allow(clippy::type_complexity)]
    pub(super) fn table_body(&mut self, name: Option<&str>, col: usize) -> R<(Vec<Vec<Node>>, Vec<TableRule>, Vec<usize>)> {
        let label = name.unwrap_or("tabela");
        let mut rows: Vec<Vec<Node>> = Vec::new();
        let mut row: Vec<Node> = Vec::new();
        let mut cell: Vec<Node> = Vec::new();
        let mut horizontals = Vec::new();
        let mut intertexts = Vec::new();
        loop {
            let (items, stop) = self.list()?;
            cell.extend(items);
            let body_end = match (&stop, name) {
                (Stop::End(end), Some(n)) if end == n => true,
                (Stop::Eof, None) => true,
                _ => false,
            };
            if body_end {
                if !cell.is_empty() || !row.is_empty() {
                    row.push(Node::row(std::mem::take(&mut cell)).simplify());
                    rows.push(row);
                }
                break;
            }
            match stop {
                Stop::E => row.push(Node::row(std::mem::take(&mut cell)).simplify()),
                Stop::NewRow => {
                    row.push(Node::row(std::mem::take(&mut cell)).simplify());
                    rows.push(std::mem::take(&mut row));
                }
                Stop::Hline(dashed) => horizontals.push(TableRule { row: rows.len(), from: 0, to: usize::MAX, dashed }),
                Stop::Cline(from, to, dashed) => horizontals.push(TableRule { row: rows.len(), from, to, dashed }),
                Stop::Intertext(t) => {
                    if !cell.is_empty() || !row.is_empty() {
                        row.push(Node::row(std::mem::take(&mut cell)).simplify());
                        rows.push(std::mem::take(&mut row));
                    }
                    intertexts.push(rows.len());
                    rows.push(vec![t]);
                }
                Stop::End(other) => {
                    self.tolerate(rules::E_REC_5, col, format!("\\begin{{{label}}} closed with \\end{{{other}}}"), "closed there")?;
                    if !cell.is_empty() || !row.is_empty() {
                        row.push(Node::row(std::mem::take(&mut cell)).simplify());
                        rows.push(std::mem::take(&mut row));
                    }
                    break;
                }
                Stop::Eof => {
                    self.tolerate(rules::E_REC_5, col, format!("\\begin{{{label}}} without \\end{{{label}}}"), "closed at the end")?;
                    if !cell.is_empty() || !row.is_empty() {
                        row.push(Node::row(std::mem::take(&mut cell)).simplify());
                        rows.push(std::mem::take(&mut row));
                    }
                    break;
                }
                Stop::CloseBrace => self.tolerate(rules::E_REC_4, col, format!("stray '}}' inside {label}"), "ignored")?,
                other => self.tolerate(rules::E_REC_5, col, format!("{other:?} inside {label}"), "ignored")?,
            }
        }
        Ok((rows, horizontals, intertexts))
    }

    /// Table read from a braced argument (`\substack{…}`, plain TeX's `\cases{…}`).
    pub(super) fn argument_table(&mut self, shape: TableKind, col: usize) -> R<Node> {
        let raw = self.raw_text()?;
        let mut sub = Reader { c: raw.chars().collect(), pos: 0, locale: self.locale, registry: self.registry, macros: self.macros, variant: self.variant, strict: self.strict };
        let (rows, horizontals, intertexts) = sub.table_body(None, col)?;
        Ok(Node::Table { shape, rows, columns: vec![], verticals: vec![], dashed: vec![], horizontals, between: vec![], intertexts, stretch: 1.0 })
    }

    /// amscd commutative diagram: `A @>f>> B \\ @VgVV @VVhV \\ C @>>k> D`.
    pub(super) fn cd_diagram(&mut self, raw: &str, col: usize) -> R<Node> {
        let mut rows = Vec::new();
        for row in cd_raw_rows(raw).into_iter().filter(|line| !line.trim().is_empty()) {
            rows.push(self.cd_row(&row, col)?);
        }
        Ok(Node::Table {
            shape: TableKind::Diagram,
            rows,
            columns: vec![],
            verticals: vec![],
            dashed: vec![],
            horizontals: vec![],
            between: vec![],
            intertexts: vec![],
            stretch: 1.0,
        })
    }

    /// One row: the objects and the arrows between them, or a row made only of vertical arrows.
    fn cd_row(&mut self, row: &str, col: usize) -> R<Vec<Node>> {
        let c: Vec<char> = row.chars().collect();
        let mut objects: Vec<String> = vec![String::new()];
        let mut arrows: Vec<Node> = Vec::new();
        let mut vertical = false;
        let mut i = 0;
        while i < c.len() {
            if c[i] == '@' && i + 1 < c.len() {
                let kind = c[i + 1];
                i += 2;
                let arrow = match kind {
                    '>' | '<' => {
                        let above = cd_until(&c, &mut i, kind);
                        let below = cd_until(&c, &mut i, kind);
                        Node::Arrow { symbol: if kind == '>' { '→' } else { '←' }, above: self.cd_label(above)?, below: self.cd_label(below)? }
                    }
                    'V' | 'A' => {
                        vertical = true;
                        let left = cd_until(&c, &mut i, kind);
                        let right = cd_until(&c, &mut i, kind);
                        Node::VerticalArrow { symbol: if kind == 'V' { '↓' } else { '↑' }, left: self.cd_label(left)?, right: self.cd_label(right)? }
                    }
                    '=' => Node::Arrow { symbol: '=', above: None, below: None },
                    '|' => {
                        vertical = true;
                        Node::VerticalArrow { symbol: '‖', left: None, right: None }
                    }
                    '.' => Node::row(vec![]),
                    other => return self.error(col, format!("unknown diagram arrow: @{other}")),
                };
                arrows.push(arrow);
                objects.push(String::new());
            } else {
                objects.last_mut().unwrap().push(c[i]);
                i += 1;
            }
        }
        self.cd_cells(CdRow { objects, arrows, vertical })
    }

    /// A label of an amscd arrow. Blank text is no label.
    fn cd_label(&mut self, text: String) -> R<Option<Box<Node>>> {
        if text.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(Box::new(self.sub_reader(&text, Variant::Italic)?)))
        }
    }

    /// Vertical arrows stand in the object columns, with an empty column between them.
    fn cd_cells(&mut self, parsed: CdRow) -> R<Vec<Node>> {
        let CdRow { objects, arrows, vertical } = parsed;
        let mut cells = Vec::new();
        if vertical {
            for (k, arrow) in arrows.into_iter().enumerate() {
                if k > 0 {
                    cells.push(Node::row(vec![]));
                }
                cells.push(arrow);
            }
        } else {
            let mut arrows = arrows.into_iter();
            for (k, object) in objects.iter().enumerate() {
                if k > 0 {
                    cells.push(arrows.next().unwrap_or(Node::row(vec![])));
                }
                cells.push(self.sub_reader(object, Variant::Italic)?);
            }
        }
        Ok(cells)
    }

    pub(super) fn array_columns(&mut self, spec: &str, preamble: &mut ArrayPreamble, col: usize) -> R<()> {
        let c: Vec<char> = spec.chars().collect();
        let mut i = 0;
        let group = |i: &mut usize| -> String {
            // content of {…} starting at c[*i] == '{'
            let mut level = 0;
            let mut s = String::new();
            while *i < c.len() {
                let ch = c[*i];
                *i += 1;
                match ch {
                    '{' => {
                        level += 1;
                        if level == 1 {
                            continue;
                        }
                    }
                    '}' => {
                        level -= 1;
                        if level == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                s.push(ch);
            }
            s
        };
        while i < c.len() {
            match c[i] {
                'l' | 'c' | 'r' => {
                    preamble.columns.push(c[i]);
                    i += 1;
                }
                'p' | 'm' | 'b' | 'X' => {
                    i += 1;
                    if c.get(i) == Some(&'{') {
                        group(&mut i);
                    }
                    preamble.columns.push('l');
                }
                '|' => {
                    preamble.verticals.push(preamble.columns.len());
                    i += 1;
                }
                // arydshln: a dashed rule
                ':' => {
                    preamble.dashed.push(preamble.columns.len());
                    i += 1;
                }
                '@' | '!' => {
                    i += 1;
                    let content = group(&mut i);
                    preamble.between.push((preamble.columns.len(), self.sub_reader(&content, Variant::Italic)?));
                }
                '*' => {
                    i += 1;
                    let n: usize = group(&mut i).trim().parse().unwrap_or(1);
                    let repeat_n_times = group(&mut i);
                    // *{50}{*{50}{…}} multiplies: the total column count is capped too
                    for _ in 0..n.min(50) {
                        if preamble.columns.len() >= MAX_COLUMNS {
                            break;
                        }
                        self.nested(|this| this.array_columns(&repeat_n_times, preamble, col))?;
                    }
                }
                ' ' => i += 1,
                other => {
                    self.tolerate(rules::E_REC_9, col, format!("unsupported array column: '{other}'"), "ignored")?;
                    i += 1;
                }
            }
        }
        Ok(())
    }
}

/// Rows of an amscd diagram, split at `\\` that is not inside braces.
fn cd_raw_rows(raw: &str) -> Vec<String> {
    let c: Vec<char> = raw.chars().collect();
    let mut raw_rows: Vec<String> = vec![String::new()];
    let mut level = 0;
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '{' => level += 1,
            '}' => level -= 1,
            '\\' if level == 0 && c.get(i + 1) == Some(&'\\') => {
                raw_rows.push(String::new());
                i += 2;
                continue;
            }
            _ => {}
        }
        raw_rows.last_mut().unwrap().push(c[i]);
        i += 1;
    }
    raw_rows
}

/// Text up to `end` at brace level zero. The character that ends it is consumed and not included.
fn cd_until(c: &[char], i: &mut usize, end: char) -> String {
    let mut level = 0;
    let mut s = String::new();
    while *i < c.len() {
        let ch = c[*i];
        *i += 1;
        match ch {
            '{' => level += 1,
            '}' => level -= 1,
            x if x == end && level == 0 => return s,
            _ => {}
        }
        s.push(ch);
    }
    s
}
