//! Text mode: \text and the commands read inside it.

use super::*;

struct TextScan<'a> {
    c: &'a [char],
    i: usize,
    level: usize,
    open_until: Vec<(usize, char)>,
    bold_math: Option<usize>,
    current: String,
    parts: Vec<TextPart>,
    column: usize,
}

impl TextScan<'_> {
    /// Bold math follows `\boldmath` until its group ends. Computed before a piece changes that state.
    fn math_variant(&self) -> Variant {
        if self.bold_math.is_some() {
            Variant::BoldItalic
        } else {
            Variant::Italic
        }
    }

    fn close_group(&mut self) {
        while let Some(&(_, end)) = self.open_until.last().filter(|(at, _)| *at == self.level) {
            self.open_until.pop();
            self.current.push(end);
        }
        if self.bold_math == Some(self.level) {
            self.bold_math = None;
        }
        self.level = self.level.saturating_sub(1);
        self.i += 1;
    }
}

impl Reader<'_> {
    /// Content of `\text{…}`: running text, old-style accents and `$…$` snippets.
    pub(super) fn text(&mut self) -> R<Node> {
        let column = self.column();
        let raw = self.raw_text()?;
        let c: Vec<char> = raw.chars().collect();
        let mut scan = TextScan {
            c: &c,
            i: 0,
            level: 0,
            open_until: Vec::new(),
            // bold and small capitals: \textbf{…} closes at the group's brace; \bf and \bfseries last until the
            // end of the current group. Each entry is the group level and the marker that closes it.
            // L-TXT-2: \boldmath sets the math that follows in bold, until the end of its group (and
            // into what a bold reader reads, as the text of `\rotatebox`)
            bold_math: (self.variant == Variant::BoldItalic).then_some(0),
            current: String::new(),
            parts: Vec::new(),
            column,
        };
        while scan.i < scan.c.len() {
            self.text_piece(&mut scan)?;
        }
        for (_, end) in scan.open_until.into_iter().rev() {
            scan.current.push(end);
        }
        if !scan.current.is_empty() {
            scan.parts.push(TextPart::Text(scan.current));
        }
        Ok(Node::Text { parts: scan.parts })
    }

    fn text_piece(&mut self, scan: &mut TextScan<'_>) -> R<()> {
        match scan.c[scan.i] {
            '$' => self.text_dollar(scan),
            '~' => {
                scan.current.push('\u{00A0}');
                scan.i += 1;
                Ok(())
            }
            '{' => {
                scan.level += 1;
                scan.i += 1;
                Ok(())
            }
            '}' => {
                scan.close_group();
                Ok(())
            }
            '\\' if matches!(scan.c.get(scan.i + 1), Some('(') | Some('[')) => self.text_delimited(scan),
            '\\' => self.text_backslash(scan),
            q @ ('`' | '\'') => self.text_quote(scan, q),
            other => {
                scan.current.push(other);
                scan.i += 1;
                Ok(())
            }
        }
    }

    fn text_dollar(&mut self, scan: &mut TextScan<'_>) -> R<()> {
        let end = scan.c[scan.i + 1..].iter().position(|&x| x == '$').map(|p| p + scan.i + 1);
        let Some(end) = end else {
            self.tolerate(rules::E_REC_9, scan.column, "unpaired '$' inside \\text", "written as a dollar sign")?;
            scan.current.push('$');
            scan.i += 1;
            return Ok(());
        };
        let snippet: String = scan.c[scan.i + 1..end].iter().collect();
        self.text_as_math(scan, snippet)?;
        scan.i = end + 1;
        Ok(())
    }

    /// `\(…\)` and `\[…\]` inside text, like `$…$`.
    fn text_delimited(&mut self, scan: &mut TextScan<'_>) -> R<()> {
        let close = if scan.c[scan.i + 1] == '(' { ')' } else { ']' };
        let end = (scan.i + 2..scan.c.len().saturating_sub(1)).find(|&k| scan.c[k] == '\\' && scan.c[k + 1] == close);
        let Some(end) = end else {
            self.tolerate(rules::E_REC_9, scan.column, format!("unpaired \\{} inside \\text", scan.c[scan.i + 1]), "written as is")?;
            scan.current.push(scan.c[scan.i + 1]);
            scan.i += 2;
            return Ok(());
        };
        let snippet: String = scan.c[scan.i + 2..end].iter().collect();
        self.text_as_math(scan, snippet)?;
        scan.i = end + 2;
        Ok(())
    }

    fn text_backslash(&mut self, scan: &mut TextScan<'_>) -> R<()> {
        scan.i += 1;
        let mark = scan.c.get(scan.i).copied().unwrap_or(' ');
        if "'`^~\"=.".contains(mark) || (matches!(mark, 'c' | 'u' | 'v' | 'H' | 'r' | 'k' | 'b' | 'd' | 't') && scan.c.get(scan.i + 1).is_some_and(|x| *x == '{' || *x == ' ')) {
            return self.text_accent(scan, mark);
        }
        let mut name = String::new();
        while let Some(x) = scan.c.get(scan.i).filter(|x| x.is_ascii_alphabetic()) {
            name.push(*x);
            scan.i += 1;
        }
        if name.is_empty() {
            scan.i += 1;
            return self.text_char_command(scan, mark);
        }
        self.text_named(scan, &name)
    }

    fn text_accent(&mut self, scan: &mut TextScan<'_>, mark: char) -> R<()> {
        scan.i += 1;
        while scan.c.get(scan.i) == Some(&' ') || scan.c.get(scan.i) == Some(&'{') {
            scan.i += 1;
        }
        let letter = scan.c.get(scan.i).copied().unwrap_or(' ');
        scan.i += 1;
        if scan.c.get(scan.i) == Some(&'}') {
            scan.i += 1;
        }
        match symbols::text_accent(mark, letter) {
            Some(composed) => scan.current.push(composed),
            // no precomposed form: the letter followed by the combining accent
            None => scan.current.extend([letter].into_iter().chain(symbols::text_combining(mark))),
        }
        Ok(())
    }

    fn text_char_command(&mut self, scan: &mut TextScan<'_>, mark: char) -> R<()> {
        match mark {
            '%' | '$' | '&' | '#' | '_' | '{' | '}' => scan.current.push(mark),
            '|' => scan.current.push('‖'),
            ',' | ';' | ':' | ' ' | '!' => scan.current.push(if mark == '!' { '\u{200B}' } else { ' ' }),
            '\\' => scan.current.push(' '),
            other => {
                self.tolerate(rules::E_REC_1, scan.column, format!("unsupported text command: \\{other}"), "written as is")?;
                scan.current.push(other);
            }
        }
        Ok(())
    }

    fn text_named(&mut self, scan: &mut TextScan<'_>, name: &str) -> R<()> {
        while scan.c.get(scan.i) == Some(&' ') {
            scan.i += 1;
        }
        if name == "begin" {
            if let Some(end) = text_diagram_end(scan.c, scan.i) {
                let tex = format!("\\begin{}", scan.c[scan.i..end].iter().collect::<String>());
                scan.i = end;
                return self.text_as_math(scan, tex);
            }
        }
        if let Some((open, at, end)) = text_font(name, scan.level) {
            scan.current.push(open);
            scan.open_until.push((at, end));
            return Ok(());
        }
        if let Some(glyph) = text_glyph(name) {
            scan.current.push_str(glyph);
            return Ok(());
        }
        match name {
            "textrm" | "textsf" | "text" | "mbox" | "textup" | "textnormal" | "rm" | "sf" | "small" | "scriptsize" | "footnotesize" | "tiny" | "large" | "normalsize"
            | "mdseries" | "selectfont" => {}
            // in text there is no height to hide: only its optional [t] or [b] goes
            "smash" => {
                if scan.c.get(scan.i) == Some(&'[') {
                    while scan.i < scan.c.len() && scan.c[scan.i] != ']' {
                        scan.i += 1;
                    }
                    scan.i += 1;
                }
            }
            // L-ENT-3: the size comes from whoever uses the SVG
            "fontsize" => {
                text_arguments(scan.c, &mut scan.i, "{{");
            }
            "boldmath" => scan.bold_math = Some(scan.bold_math.map_or(scan.level, |at| at.min(scan.level))),
            "unboldmath" => scan.bold_math = None,
            // L-TXT-2: commands that TeX sets as math inside text
            "underline" => {
                let tex = format!("\\underline{{\\text{}}}", text_arguments(scan.c, &mut scan.i, "{"));
                self.text_as_math(scan, tex)?;
            }
            "textsubscript" | "textsuperscript" | "ensuremath" => {
                let tex = format!("\\{name}{}", text_arguments(scan.c, &mut scan.i, "{"));
                self.text_as_math(scan, tex)?;
            }
            // L-CHE-1: mhchem typesets its formulas and quantities in text as in math
            "ce" | "pu" => {
                let tex = format!("\\{name}{}", text_arguments(scan.c, &mut scan.i, "{"));
                self.text_as_math(scan, tex)?;
            }
            // L-TXT-4: \textcolor colors its argument, \color the rest of its group; the
            // colored text keeps the bold, small capitals and monospace around it
            "textcolor" | "color" => self.text_color(scan, name)?,
            // L-TXT-3: a text is a box of its natural width, where the fills take no room; a
            // line break, which one line of text cannot hold, is a space, as `\\` is
            "hfill" | "hfil" | "hss" => {}
            "newline" | "linebreak" => {
                text_arguments(scan.c, &mut scan.i, "[");
                scan.current.push(' ');
            }
            // L-TXT-2: an environment that starts math, read as math up to its \end
            "begin" if math_environment(&environment_name(scan.c, scan.i)).is_some() => self.text_math_environment(scan)?,
            // L-UNI-4: siunitx typesets its quantities in text as in math; L-BOX-1: graphicx's boxes
            other if siunitx_in_text(other).or(graphicx_box(other)).is_some() => {
                let tex = format!("\\{other}{}", text_arguments(scan.c, &mut scan.i, siunitx_in_text(other).or(graphicx_box(other)).unwrap_or_default()));
                self.text_as_math(scan, tex)?;
            }
            // E-REC-1: a math command in text is an error in TeX; read as math
            other if math_in_text(other).is_some() => {
                self.tolerate(rules::E_REC_1, scan.column, format!("math command in text: \\{other}"), "read as math")?;
                let tex = format!("\\{other}{}", text_arguments(scan.c, &mut scan.i, math_in_text(other).unwrap_or_default()));
                self.text_as_math(scan, tex)?;
            }
            // L-TXT-3: space inside text, set by the math reader, which knows lengths and phantoms
            "hspace" => {
                let tex = format!("\\hspace{}", text_arguments(scan.c, &mut scan.i, "*{"));
                self.text_as_math(scan, tex)?;
            }
            "phantom" | "hphantom" | "vphantom" => self.text_phantom(scan, name)?,
            other => self.text_symbol(scan, other)?,
        }
        Ok(())
    }

    fn text_as_math(&mut self, scan: &mut TextScan<'_>, tex: String) -> R<()> {
        let math_variant = scan.math_variant();
        split_text(&mut scan.parts, &mut scan.current, &scan.open_until);
        scan.parts.push(TextPart::Math(self.sub_reader(&tex, math_variant)?));
        Ok(())
    }

    fn text_color(&mut self, scan: &mut TextScan<'_>, name: &str) -> R<()> {
        let color = text_arguments(scan.c, &mut scan.i, "[{");
        let body = if name == "textcolor" {
            let arg = text_arguments(scan.c, &mut scan.i, "{");
            arg[1..arg.len() - 1].to_string()
        } else {
            let end = group_end(scan.c, scan.i);
            let rest: String = scan.c[scan.i..end].iter().collect();
            scan.i = end;
            rest
        };
        let mut styles: String = scan
            .open_until
            .iter()
            .map(|&(_, end)| match end {
                BOLD_END => "\\bfseries ",
                SMALL_CAPS_END => "\\scshape ",
                ITALIC_END | ITALIC_CORRECTED_END => "\\itshape ",
                _ => "\\ttfamily ",
            })
            .collect();
        if scan.bold_math.is_some() {
            styles.push_str("\\boldmath ");
        }
        let math_variant = scan.math_variant();
        split_text(&mut scan.parts, &mut scan.current, &scan.open_until);
        scan.parts.push(TextPart::Math(self.sub_reader(&format!("\\textcolor{color}{{\\text{{{styles}{body}}}}}"), math_variant)?));
        Ok(())
    }

    fn text_math_environment(&mut self, scan: &mut TextScan<'_>) -> R<()> {
        let env = environment_name(scan.c, scan.i);
        if math_environment(&env) == Some(false) {
            self.tolerate(rules::E_REC_1, scan.column, format!("math environment in text: {env}"), "read as math")?;
        }
        let end = environment_end(scan.c, scan.i, &env);
        let mut tex = format!("\\begin{}", scan.c[scan.i..end.unwrap_or(scan.c.len())].iter().collect::<String>());
        if end.is_none() {
            self.tolerate(rules::E_REC_5, scan.column, format!("\\begin{{{env}}} without \\end{{{env}}}"), "closed at the end")?;
            tex.push_str(&format!("\\end{{{env}}}"));
        }
        scan.i = end.unwrap_or(scan.c.len());
        self.text_as_math(scan, tex)
    }

    fn text_phantom(&mut self, scan: &mut TextScan<'_>, name: &str) -> R<()> {
        // the argument stays text: the room it takes is the text's
        let arg = text_arguments(scan.c, &mut scan.i, "{");
        let arg = if arg.starts_with('{') { arg } else { format!("{{{arg}}}") };
        self.text_as_math(scan, format!("\\{name}{{\\text{arg}}}"))
    }

    fn text_symbol(&mut self, scan: &mut TextScan<'_>, name: &str) -> R<()> {
        if let Some(symbols::Symbol { symbol, .. }) = symbols::command(name) {
            scan.current.push(symbol);
        } else if let Some(g) = symbols::greek(name) {
            scan.current.push(g);
        } else {
            self.tolerate(rules::E_REC_1, scan.column, format!("unsupported text command: \\{name}"), "written as is")?;
            scan.current.push('\\');
            scan.current.push_str(name);
        }
        Ok(())
    }

    /// L-TXT-1: TeX's quote ligatures in text: `` '' ` ' become “ ” ‘ ’.
    fn text_quote(&mut self, scan: &mut TextScan<'_>, q: char) -> R<()> {
        let double = scan.c.get(scan.i + 1) == Some(&q);
        let typographic = match (q, double) {
            ('`', true) => '“',
            ('`', false) => '‘',
            (_, true) => '”',
            _ => '’',
        };
        self.registry.note(rules::L_TXT_1, format!("{} written as {typographic}", if double { format!("{q}{q}") } else { q.to_string() }));
        scan.current.push(typographic);
        scan.i += if double { 2 } else { 1 };
        Ok(())
    }
}

/// Font declared inside text: the marker that opens it, the group level that closes it, and the marker that closes it.
fn text_font(name: &str, level: usize) -> Option<(char, usize, char)> {
    Some(match name {
        "textbf" | "mathbf" => (BOLD_START, level + 1, BOLD_END),
        "bf" | "bfseries" => (BOLD_START, level, BOLD_END),
        "textsc" => (SMALL_CAPS_START, level + 1, SMALL_CAPS_END),
        "sc" | "scshape" => (SMALL_CAPS_START, level, SMALL_CAPS_END),
        "texttt" => (MONO_START, level + 1, MONO_END),
        "tt" | "ttfamily" => (MONO_START, level, MONO_END),
        // L-TXT-5: italic text
        "textit" | "emph" => (ITALIC_START, level + 1, ITALIC_CORRECTED_END),
        "it" | "itshape" | "em" => (ITALIC_START, level, ITALIC_END),
        _ => return None,
    })
}

/// A command that is one glyph in text. A table: one name, the characters it writes.
fn text_glyph(name: &str) -> Option<&'static str> {
    Some(match name {
        "quad" => "\u{2003}",
        "qquad" => "\u{2003}\u{2003}",
        "ldots" | "dots" => "…",
        "textdegree" | "degree" => "°",
        "o" => "ø",
        "ss" => "ß",
        "aa" => "å",
        "AA" => "Å",
        "ae" => "æ",
        "AE" => "Æ",
        "oe" => "œ",
        "OE" => "Œ",
        "i" => "ı",
        "j" => "ȷ",
        "L" => "Ł",
        "textdollar" => "$",
        "textasciicircum" => "^",
        "textunderscore" => "_",
        "textbar" => "|",
        "textbardbl" => "‖",
        "textbraceleft" => "{",
        "textbraceright" => "}",
        "textdagger" => "†",
        "textdaggerdbl" => "‡",
        "textellipsis" => "…",
        "textbullet" => "•",
        "textperiodcentered" => "·",
        "textparagraph" => "¶",
        "textsection" => "§",
        "textquoteleft" => "‘",
        "textquoteright" => "’",
        "textquotedblleft" => "“",
        "textquotedblright" => "”",
        "textendash" => "–",
        "textemdash" => "—",
        "textasciitilde" => "~",
        "textbackslash" => "\\",
        "times" => "×",
        "LaTeX" => "LaTeX",
        "TeX" => "TeX",
        _ => return None,
    })
}

/// The text so far as a part of its own, before math inside it: each part is set apart, so the bold,
/// small capitals and monospace still open (`open`, by their closing markers) are closed at its end
/// and opened again in the text that follows (`\textbf{a $x$ b}`: b is bold too). Text of markers
/// alone stays, to go on after the math.
pub(super) fn split_text(parts: &mut Vec<TextPart>, current: &mut String, open: &[(usize, char)]) {
    if current.chars().all(|c| MARKERS.contains(&c)) {
        return;
    }
    let mut done = std::mem::take(current);
    done.extend(open.iter().rev().map(|&(_, end)| end));
    parts.push(TextPart::Text(done));
    current.extend(open.iter().map(|&(_, end)| match end {
        BOLD_END => BOLD_START,
        SMALL_CAPS_END => SMALL_CAPS_START,
        ITALIC_END | ITALIC_CORRECTED_END => ITALIC_START,
        _ => MONO_START,
    }));
}

/// The arguments of a command inside text, as written, to read them again with the command: `spec` has
/// `{` for each mandatory argument, `[` for each optional one and `*` for an optional star, in order. A
/// mandatory argument without braces is one character or one command, as in TeX.
/// Where the `tikzcd` whose `\begin` is just behind `i` ends, if that is what comes: a diagram is
/// math, also in the middle of a text like that of a TikZ node.
pub(super) fn text_diagram_end(c: &[char], i: usize) -> Option<usize> {
    let open: Vec<char> = "{tikzcd}".chars().collect();
    let close: Vec<char> = "\\end{tikzcd}".chars().collect();
    let rest = c.get(i..)?;
    if !rest.starts_with(&open) {
        return None;
    }
    rest.windows(close.len()).position(|w| w == close).map(|p| i + p + close.len())
}

pub(super) fn text_arguments(c: &[char], i: &mut usize, spec: &str) -> String {
    let mut out = String::new();
    for kind in spec.chars() {
        let mut k = *i;
        while c.get(k) == Some(&' ') {
            k += 1;
        }
        match kind {
            '*' => {
                if c.get(k) == Some(&'*') {
                    out.push('*');
                    *i = k + 1;
                }
            }
            '[' => {
                if c.get(k) != Some(&'[') {
                    continue;
                }
                let mut depth = 0usize;
                let end = (k + 1..c.len()).find(|&j| match c[j] {
                    '{' => {
                        depth += 1;
                        false
                    }
                    '}' => {
                        depth = depth.saturating_sub(1);
                        false
                    }
                    ']' => depth == 0,
                    _ => false,
                });
                if let Some(end) = end {
                    out.extend(&c[k..=end]);
                    *i = end + 1;
                }
            }
            _ => match c.get(k) {
                Some('{') => {
                    let mut depth = 0usize;
                    let mut j = k;
                    while j < c.len() {
                        match c[j] {
                            '\\' => j += 1,
                            '{' => depth += 1,
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        j += 1;
                    }
                    let end = j.min(c.len().saturating_sub(1));
                    out.extend(&c[k..=end]);
                    // an unclosed group is closed at the end, as raw_text does
                    if j >= c.len() {
                        out.push('}');
                    }
                    *i = end + 1;
                }
                Some('\\') => {
                    let mut j = k + 1;
                    while c.get(j).is_some_and(char::is_ascii_alphabetic) {
                        j += 1;
                    }
                    let end = j.max(k + 2).min(c.len());
                    out.push('{');
                    out.extend(&c[k..end]);
                    out.push('}');
                    *i = end;
                }
                Some(&x) => {
                    out.push('{');
                    out.push(x);
                    out.push('}');
                    *i = k + 1;
                }
                None => {}
            },
        }
    }
    out
}

/// The siunitx commands, which work in text as in math, with their arguments (as for `text_arguments`).
pub(super) fn siunitx_in_text(name: &str) -> Option<&'static str> {
    Some(match name {
        "SI" | "qty" | "SIlist" | "qtylist" | "SIproduct" | "qtyproduct" => "[{{",
        "si" | "unit" | "ang" | "numlist" | "anglist" | "numproduct" => "[{",
        "num" => "[{[",
        "SIrange" | "qtyrange" => "[{{{",
        "numrange" | "angrange" => "[{{",
        _ => return None,
    })
}

/// graphicx's box commands, with their arguments (as for `text_arguments`); their content is text.
pub(super) fn graphicx_box(name: &str) -> Option<&'static str> {
    Some(match name {
        "scalebox" => "{[{",
        "reflectbox" => "{",
        "resizebox" => "*{{{",
        "rotatebox" => "[{{",
        "raisebox" => "{[[{",
        _ => return None,
    })
}

/// Math commands that authors write in text, with their arguments (as for `text_arguments`).
pub(super) fn math_in_text(name: &str) -> Option<&'static str> {
    Some(match name {
        "mathrm" | "mathit" | "mathsf" | "mathtt" | "mathcal" | "mathbb" | "mathfrak" | "mathscr" | "mathbfit" | "mathnormal" | "boldsymbol" | "bm" => "{",
        "frac" | "dfrac" | "tfrac" => "{{",
        "sqrt" => "[{",
        "operatorname" => "*{",
        _ => return None,
    })
}

/// Environments that start math, read in text: `Some(true)` for the displays, which TeX allows in a
/// paragraph, `Some(false)` for those TeX allows only in math.
pub(super) fn math_environment(name: &str) -> Option<bool> {
    match name.trim_end_matches('*') {
        "equation" | "align" | "alignat" | "gather" | "multline" | "flalign" | "eqnarray" | "displaymath" | "math" => Some(true),
        "array" | "matrix" | "pmatrix" | "bmatrix" | "Bmatrix" | "vmatrix" | "Vmatrix" | "smallmatrix" | "cases" | "dcases" | "rcases" | "aligned" | "alignedat" | "gathered"
        | "split" | "subarray" => Some(false),
        _ => None,
    }
}

/// The name in `{name}` at `c[i]` (after spaces), or nothing.
pub(super) fn environment_name(c: &[char], i: usize) -> String {
    let rest = c[i..].iter().skip_while(|&&x| x == ' ');
    let mut rest = rest.peekable();
    if rest.next() != Some(&'{') {
        return String::new();
    }
    rest.take_while(|&&x| x != '}').collect()
}

/// Where the environment `name` that starts at `c[i]` ends: after its `\end{name}`, counting the ones
/// nested in it; nothing if it does not end.
pub(super) fn environment_end(c: &[char], i: usize, name: &str) -> Option<usize> {
    let open: Vec<char> = format!("\\begin{{{name}}}").chars().collect();
    let close: Vec<char> = format!("\\end{{{name}}}").chars().collect();
    let mut level = 0;
    let mut k = i;
    while k < c.len() {
        if c[k..].starts_with(&open) {
            level += 1;
            k += open.len();
        } else if c[k..].starts_with(&close) {
            if level == 0 {
                return Some(k + close.len());
            }
            level -= 1;
            k += close.len();
        } else {
            k += 1;
        }
    }
    None
}

/// Where the group that `c[i]` is in closes: its `}`, or the end of the text.
pub(super) fn group_end(c: &[char], i: usize) -> usize {
    let mut depth = 0usize;
    let mut k = i;
    while k < c.len() {
        match c[k] {
            '\\' => k += 1,
            '{' => depth += 1,
            '}' if depth == 0 => return k,
            '}' => depth -= 1,
            _ => {}
        }
        k += 1;
    }
    c.len()
}
