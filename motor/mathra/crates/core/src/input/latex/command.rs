//! Dispatch of a LaTeX command, and the groups that are only a match.

use super::*;

impl Reader<'_> {
    /// Reads a command. Returns a stop when the command ends the list (`\\`, `\end`, `\right`…).
    pub(super) fn structural_command(&mut self, items: &mut Vec<Node>) -> R<Option<Stop>> {
        self.nested(|this| this.command_unguarded(items))
    }

    pub(super) fn command_unguarded(&mut self, items: &mut Vec<Node>) -> R<Option<Stop>> {
        let col = self.column();
        let start = self.pos;
        self.pos += 1;
        let name = self.read_command_name();
        if let Some(m) = self.macros.commands.get(&name).cloned() {
            self.expand(start, &m, col)?;
            return Ok(None);
        }
        let primitive = match name.as_str() {
            "bgroup" => Some("{"),
            "egroup" => Some("}"),
            "sp" => Some("^"),
            "sb" => Some("_"),
            "hfill" | "hss" | "quad" if name == "hfill" || name == "hss" => Some("\\quad "),
            "vrule" => Some("\\rule{0.04em}{1em}"),
            _ => None,
        };
        if let Some(text) = primitive {
            self.inject(start, text, col)?;
            return Ok(None);
        }
        if let Some(expansion) = self.package_expansion(&name, col)? {
            self.inject(start, &expansion, col)?;
            return Ok(None);
        }
        if self.definition(&name, col)? {
            return Ok(None);
        }
        let star = if self.current() == Some('*') && matches!(name.as_str(), "operatorname" | "tag" | "smash" | "resizebox") {
            self.pos += 1;
            true
        } else {
            false
        };
        // each group of commands lives in its own function: in debug builds, a single match with every
        // arm reserved ~80 KB of stack per level of nested \frac{…}.
        // Each group returns None when the name is not its own; the last one handles any name.
        // Function pointers, not direct calls: a direct call would let the compiler inline every group
        // back into this one, and a debug build would again reserve the stack of all of them at once.
        #[allow(clippy::type_complexity)]
        let groups: [fn(&mut Self, CommandCall<'_>) -> R<Option<Option<Stop>>>; 11] = [
            Self::structure_command,
            Self::space_command,
            Self::style_command,
            Self::fraction_command,
            Self::text_command,
            Self::accent_command,
            Self::unit_command,
            Self::unicode_and_table_command,
            Self::box_command,
            Self::verbatim_command,
            Self::mod_command,
        ];
        for group in groups {
            if let Some(stop) = group(self, CommandCall { name: &name, column: col, start, starred: star, items })? {
                return Ok(stop);
            }
        }
        self.symbol_command(CommandCall { name: &name, column: col, start, starred: star, items })
    }

    pub(super) fn structure_command(&mut self, call: CommandCall<'_>) -> R<Option<Option<Stop>>> {
        let node: Node = match call.name {
            "\\" | "cr" | "newline" => {
                self.raw_optional();
                return Ok(Some(Some(Stop::NewRow)));
            }
            "end" => return Ok(Some(Some(Stop::End(self.raw_text()?.trim().to_string())))),
            "right" => return Ok(Some(Some(Stop::Right(self.delimiter()?)))),
            "middle" => return Ok(Some(Some(Stop::Middle(self.delimiter()?)))),
            "hline" | "midrule" | "toprule" | "bottomrule" => return Ok(Some(Some(Stop::Hline(false)))),
            // arydshln
            "hdashline" => {
                self.raw_optional();
                return Ok(Some(Some(Stop::Hline(true))));
            }
            "cline" | "cdashline" => {
                let range = self.raw_text()?;
                let (a, b) = range.split_once('-').unwrap_or((&range, &range));
                let n = |t: &str| t.trim().parse::<usize>().map(|v| v.saturating_sub(1));
                return match (n(a), n(b)) {
                    (Ok(a), Ok(b)) => Ok(Some(Some(Stop::Cline(a, b, call.name == "cdashline")))),
                    _ => self.error(call.column, format!("\\cline{{{range}}}: use the form \\cline{{1-2}}")),
                };
            }
            "left" => self.delimited(call.column)?,
            "begin" => {
                let saved = self.pos;
                let env_name = self.raw_text()?.trim().to_string();
                if let Some((open, close)) = self.macros.environments.get(&env_name).cloned() {
                    let content = self.environment_body(&env_name, call.column)?;
                    self.inject(call.start, &format!("{{{open}{content}{close}}}"), call.column)?;
                    return Ok(Some(None));
                }
                self.pos = saved;
                self.environment(call.column)?
            }
            "intertext" | "shortintertext" => return Ok(Some(Some(Stop::Intertext(self.text()?)))),
            _ => return Ok(None),
        };
        call.items.push(node);
        Ok(Some(None))
    }

    pub(super) fn space_command(&mut self, call: CommandCall<'_>) -> R<Option<Option<Stop>>> {
        let mu = |m: f32| Node::Space { em: m / 18.0 };
        let node: Node = match call.name {
            // spaces
            "," | "thinspace" => mu(3.0),
            ":" | ">" | "medspace" => mu(4.0),
            ";" | "thickspace" => mu(5.0),
            "!" | "negthinspace" => mu(-3.0),
            "negmedspace" => mu(-4.0),
            "negthickspace" => mu(-5.0),
            " " => Node::Space { em: 0.333 },
            "quad" => Node::Space { em: 1.0 },
            "qquad" => Node::Space { em: 2.0 },
            "enspace" | "enskip" => Node::Space { em: 0.5 },
            "hspace" | "mspace" | "kern" | "mkern" | "hskip" | "mskip" => {
                if self.current() == Some('*') {
                    self.pos += 1;
                }
                let measure =
                    if self.c.get(self.pos).is_some_and(|c| *c == '{') || call.name.starts_with('h') && call.name != "hskip" { self.raw_text()? } else { self.raw_measure() };
                Node::Space { em: measure_em(&measure).ok_or(Error::Syntax { column: call.column, message: format!("unrecognized measure: {measure}") })? }
            }
            _ => return Ok(None),
        };
        call.items.push(node);
        Ok(Some(None))
    }

    pub(super) fn fraction_command(&mut self, call: CommandCall<'_>) -> R<Option<Option<Stop>>> {
        let node: Node = match call.name {
            // fractions, roots, binomials
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                if call.name == "cfrac" {
                    self.raw_optional();
                }
                let num = self.argument()?;
                let den = self.argument()?;
                let style = match call.name {
                    "dfrac" | "cfrac" => FractionStyle::Display,
                    "tfrac" => FractionStyle::Row,
                    _ => FractionStyle::Auto,
                };
                Node::Fraction { num: Box::new(num), den: Box::new(den), style, rule: true, thickness: None }
            }
            "binom" | "dbinom" | "tbinom" => {
                let num = self.argument()?;
                let den = self.argument()?;
                let style = match call.name {
                    "dbinom" => FractionStyle::Display,
                    "tbinom" => FractionStyle::Row,
                    _ => FractionStyle::Auto,
                };
                let f = Node::Fraction { num: Box::new(num), den: Box::new(den), style, rule: false, thickness: None };
                Node::Delimited { open: Some('('), close: Some(')'), parts: vec![f], middles: vec![] }
            }
            "sqrt" | "root" => {
                let index = self.optional_argument()?;
                let radicand = self.argument()?;
                Node::Root { radicand: Box::new(radicand), index: index.map(Box::new) }
            }
            _ => return Ok(None),
        };
        call.items.push(node);
        Ok(Some(None))
    }

    pub(super) fn text_command(&mut self, call: CommandCall<'_>) -> R<Option<Option<Stop>>> {
        let node: Node = match call.name {
            // text and fonts
            "text" | "textrm" | "textnormal" | "textup" | "mbox" | "hbox" | "textsf" | "textmd" | "textsl" | "fbox" | "framebox" => {
                let text = self.text()?;
                if matches!(call.name, "fbox" | "framebox") {
                    Node::Framed { base: Box::new(text), shape: Frame::Rectangle, background: None, border: None }
                } else {
                    text
                }
            }
            "textsc" | "textbf" | "texttt" | "textit" | "emph" => {
                let (open, close) = match call.name {
                    "textsc" => (SMALL_CAPS_START, SMALL_CAPS_END),
                    "textbf" => (BOLD_START, BOLD_END),
                    "textit" | "emph" => (ITALIC_START, ITALIC_CORRECTED_END),
                    _ => (MONO_START, MONO_END),
                };
                match self.text()? {
                    Node::Text { parts } => Node::Text {
                        parts: parts
                            .into_iter()
                            .map(|p| match p {
                                TextPart::Text(t) => TextPart::Text(format!("{open}{t}{close}")),
                                math => math,
                            })
                            .collect(),
                    },
                    other => other,
                }
            }
            // hyperref: the address as written, in typewriter type (_ % # ~ are its own characters)
            "url" => {
                let address = self.raw_text()?;
                let address: String = address.chars().filter(|c| !matches!(c, '\\')).collect();
                Node::Text { parts: vec![TextPart::Text(format!("{MONO_START}{address}{MONO_END}"))] }
            }
            // graphicx: \scalebox{-1}[1]{…}, its argument in text mode
            "reflectbox" => Node::Reflected { base: Box::new(self.text()?) },
            "mathrm" | "mathup" | "mathit" | "mathbf" | "mathsf" | "mathtt" | "mathbb" | "mathcal" | "mathscr" | "mathfrak" | "boldsymbol" | "bm" | "pmb" | "mathbfit"
            | "mathnormal" | "symbf" | "symrm" | "symit" | "mathds" | "mathbbm" | "Bbb" => {
                let variant = match call.name {
                    "mathrm" | "mathup" | "symrm" => Variant::Upright,
                    "mathit" | "mathnormal" | "symit" => Variant::Italic,
                    "mathbf" | "symbf" => Variant::Bold,
                    "boldsymbol" | "bm" | "pmb" | "mathbfit" => Variant::BoldItalic,
                    "mathsf" => Variant::SansSerif,
                    "mathtt" => Variant::Monospace,
                    "mathbb" | "mathds" | "mathbbm" | "Bbb" => Variant::DoubleStruck,
                    "mathcal" | "mathscr" => Variant::Calligraphic,
                    _ => Variant::Fraktur,
                };
                let raw = self.raw_text()?;
                let node = self.sub_reader(&raw, variant)?;
                // \mathit's font is a text font (cmti, cmbxti): its letters make words, with no
                // italic correction between them (TeXbook, Appendix G, rule 17)
                if matches!(call.name, "mathit" | "mathbfit") {
                    words(node)
                } else {
                    node
                }
            }
            "operatorname" => {
                let raw = self.raw_text()?;
                // in math mode TeX ignores spaces: \operatorname{ l i m } is "lim"
                // \, stays a space, as in \operatorname*{inj\,lim}
                let function_name: String = raw.replace("\\,", "\u{2009}").chars().filter(|c| c.is_alphanumeric() || matches!(c, '-' | '\u{2009}')).collect();
                let function_name = function_name.replace('\u{2009}', " ");
                Node::Function { name: function_name.trim().to_string(), limits: call.starred }
            }
            "mathop" | "mathrel" | "mathbin" | "mathord" | "mathopen" | "mathclose" | "mathpunct" | "mathinner" => {
                let class = match call.name {
                    "mathop" => Class::Op,
                    "mathrel" => Class::Rel,
                    "mathbin" => Class::Bin,
                    "mathopen" => Class::Open,
                    "mathclose" => Class::Close,
                    "mathpunct" => Class::Punct,
                    "mathinner" => Class::Inner,
                    _ => Class::Ord,
                };
                Node::ForcedClass { class, base: Box::new(self.argument()?) }
            }
            _ => return Ok(None),
        };
        call.items.push(node);
        Ok(Some(None))
    }

    pub(super) fn unicode_and_table_command(&mut self, call: CommandCall<'_>) -> R<Option<Option<Stop>>> {
        let node: Node = match call.name {
            "unicode" => {
                let raw = self.raw_text()?;
                let code = raw.trim().trim_start_matches(['x', 'X', '#', 'U', '+']);
                let c = u32::from_str_radix(code, 16).ok().and_then(char::from_u32);
                match c {
                    Some(c) => Node::Operator { symbol: c, class: Class::Ord },
                    None => return self.error(call.column, format!("\\unicode{{{raw}}}: invalid code point")),
                }
            }
            "LaTeX" | "TeX" => Node::Text { parts: vec![TextPart::Text(call.name.to_string())] },
            "substack" => self.argument_table(TableKind::Substack, call.column)?,
            "cases" => self.argument_table(TableKind::Cases, call.column)?,
            "matrix" => self.argument_table(TableKind::Matrix, call.column)?,
            "pmatrix" => self.argument_table(TableKind::PMatrix, call.column)?,
            "bordermatrix" => self.argument_table(TableKind::BorderMatrix, call.column)?,
            "eqalign" | "eqalignno" | "displaylines" => self.argument_table(TableKind::Aligned, call.column)?,
            _ => return Ok(None),
        };
        call.items.push(node);
        Ok(Some(None))
    }

    pub(super) fn verbatim_command(&mut self, call: CommandCall<'_>) -> R<Option<Option<Stop>>> {
        let node: Node = match call.name {
            "verb" => {
                if self.current() == Some('*') {
                    self.pos += 1;
                }
                let delim = self.current().ok_or(Error::Syntax { column: call.column, message: "\\verb without a delimiter".into() })?;
                self.pos += 1;
                let mut children = Vec::new();
                loop {
                    match self.current() {
                        None => return self.error(call.column, format!("\\verb{delim}… without the closing '{delim}'")),
                        Some(x) if x == delim => {
                            self.pos += 1;
                            break;
                        }
                        Some(x) => {
                            self.pos += 1;
                            children.push(if x.is_ascii_alphanumeric() {
                                Node::Identifier { name: x.to_string(), variant: Variant::Monospace }
                            } else if x == ' ' {
                                Node::Space { em: 0.525 }
                            } else {
                                Node::Operator { symbol: if x == '-' { '-' } else { x }, class: Class::Ord }
                            });
                        }
                    }
                }
                Node::row(children)
            }
            "multicolumn" => {
                let n: usize = self.raw_text()?.trim().parse().map_err(|_| Error::Syntax { column: call.column, message: "\\multicolumn{n}{…}{…} takes a number".into() })?;
                let spec = self.raw_text()?;
                let alignment = spec.chars().find(|c| matches!(c, 'l' | 'c' | 'r')).unwrap_or('c');
                Node::Multicolumn { columns: n.clamp(1, 50), alignment, base: Box::new(self.argument()?) }
            }
            "hdotsfor" | "dotsfor" => {
                self.raw_optional();
                let n: usize = self.raw_text()?.trim().parse().unwrap_or(1);
                let points = (0..n.clamp(1, 50) * 3).flat_map(|_| [Node::Operator { symbol: '.', class: Class::Ord }, Node::Space { em: 0.35 }]).collect();
                Node::Multicolumn { columns: n.clamp(1, 50), alignment: 'c', base: Box::new(Node::row(points)) }
            }
            _ => return Ok(None),
        };
        call.items.push(node);
        Ok(Some(None))
    }

    pub(super) fn mod_command(&mut self, call: CommandCall<'_>) -> R<Option<Option<Stop>>> {
        let mu = |m: f32| Node::Space { em: m / 18.0 };
        let node: Node = match call.name {
            // modular arithmetic, with amsmath's spacing
            "pmod" | "mod" | "pod" | "bmod" => {
                let text_mod = || Node::Text { parts: vec![TextPart::Text("mod".into())] };
                if call.name == "bmod" {
                    Node::ForcedClass { class: Class::Bin, base: Box::new(text_mod()) }
                } else {
                    let arg = self.argument()?;
                    let mut children = vec![mu(if call.name == "mod" { 12.0 } else { 18.0 })];
                    if call.name != "mod" {
                        children.push(Node::Operator { symbol: '(', class: Class::Open });
                    }
                    if call.name != "pod" {
                        children.push(text_mod());
                        children.push(mu(6.0));
                    }
                    children.push(arg);
                    if call.name != "mod" {
                        children.push(Node::Operator { symbol: ')', class: Class::Close });
                    }
                    Node::row(children)
                }
            }
            "imath" | "jmath" | "i" | "j" => Node::Identifier { name: if matches!(call.name, "imath" | "i") { "ı" } else { "ȷ" }.into(), variant: self.variant },
            "lt" => Node::Operator { symbol: '<', class: Class::Rel },
            "gt" => Node::Operator { symbol: '>', class: Class::Rel },
            _ => return Ok(None),
        };
        call.items.push(node);
        Ok(Some(None))
    }

    /// Functions, large operators, letters and symbols; whatever is left is an unknown command.
    pub(super) fn symbol_command(&mut self, call: CommandCall<'_>) -> R<Option<Stop>> {
        let node = if let Some((canonical, limits)) = symbols::function(call.name) {
            if canonical != call.name {
                self.registry.note(rules::L_FUN_1, format!("\\{} recognized as the function {canonical}", call.name));
            }
            Node::Function { name: canonical.to_string(), limits }
        } else if let Some((symbol, _)) = symbols::large_operator(call.name) {
            let operator = Node::LargeOperator { symbol, limits: None };
            // plain TeX's \intop is \mathop without \nolimits: limits above and below in display
            if matches!(call.name, "intop" | "ointop") {
                Node::ForcedClass { class: Class::Op, base: Box::new(operator) }
            } else {
                operator
            }
        } else if let Some(letter) = call.name.strip_prefix("var").and_then(symbols::greek).filter(|g| g.is_uppercase()) {
            // amsmath's \varGamma…\varOmega: the capital Greek letters in italic
            Node::Identifier { name: letter.to_string(), variant: Variant::Italic }
        } else if let Some(letter) = symbols::greek(call.name) {
            let variant = if symbols::greek_upright(call.name) && self.variant == Variant::Italic { Variant::Upright } else { self.variant };
            Node::Identifier { name: letter.to_string(), variant }
        } else if let Some((accent, stretchy)) = symbols::accent(call.name) {
            Node::Accent { base: Box::new(self.argument()?), accent, stretchy, below: false }
        } else if let Some(Symbol { symbol, class }) = symbols::command(call.name) {
            Node::Operator { symbol, class }
        } else {
            return self.unknown_command(call);
        };
        call.items.push(node);
        Ok(None)
    }

    /// A command mathra does not know. In tolerant mode: a `\n` that came from JSON becomes a line break,
    /// `²` becomes the character, `\5` becomes the digit, and the rest shows up in red.
    pub(super) fn unknown_command(&mut self, call: CommandCall<'_>) -> R<Option<Stop>> {
        if self.strict {
            return self.error(call.column, format!("unsupported command: \\{}", call.name));
        }
        // \nfoo where \foo is a relation: an amssymb-style negation mathra lacks, not a JSON \n. Reading it as a
        // line break would draw \foo itself, the opposite of what was written
        if let Some(remainder) = call.name.strip_prefix('n').filter(|r| symbols::command(r).is_some_and(|s| s.class == Class::Rel)) {
            self.tolerate(rules::E_REC_1, call.column, format!("unsupported command: \\{}", call.name), &format!("read as \\not\\{remainder}"))?;
            self.inject(call.start, &format!("\\not\\{remainder} "), call.column)?;
            return Ok(None);
        }
        if let Some(remainder) = call.name.strip_prefix('n').or_else(|| call.name.strip_prefix('r')).or_else(|| call.name.strip_prefix('t').filter(|_| call.name.len() <= 2)) {
            self.tolerate(rules::E_REC_2, call.column, format!("\\{}", call.name), "JSON '\\n' read as a line break")?;
            self.inject(call.start, &format!(" {remainder}"), call.column)?;
            return Ok(None);
        }
        if call.name == "u" {
            let hex: String = self.c[self.pos..].iter().take(4).collect();
            if let Some(c) = u32::from_str_radix(&hex, 16).ok().filter(|_| hex.len() == 4).and_then(char::from_u32) {
                self.tolerate(rules::E_REC_2, call.column, format!("\\u{hex} is a JSON escape"), &format!("read as '{c}'"))?;
                self.pos += 4;
                self.inject(call.start, &c.to_string(), call.column)?;
                return Ok(None);
            }
        }
        // \vertA, \alphax: a space forgotten after a known command
        // ASCII names only: a character like '∠' after the backslash has no prefix to cut
        for cut in (2..if call.name.is_ascii() { call.name.len() } else { 0 }).rev() {
            let (prefix, remainder) = call.name.split_at(cut);
            if known(prefix) {
                self.tolerate(rules::E_REC_1, call.column, format!("unsupported command: \\{}", call.name), &format!("read as \\{prefix} {remainder}"))?;
                self.inject(call.start, &format!("\\{prefix} {remainder}"), call.column)?;
                return Ok(None);
            }
        }
        if call.name.chars().count() == 1 && !call.name.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) {
            self.tolerate(rules::E_REC_1, call.column, format!("'\\{}'", call.name), "backslash before a symbol ignored")?;
            self.inject(call.start, call.name, call.column)?;
            if matches!(call.name, "\"") {
                self.pos += 1;
            }
            return Ok(None);
        }
        self.tolerate(rules::E_REC_1, call.column, format!("unsupported command: \\{}", call.name), "drawn in red")?;
        call.items.push(Node::Colored { color: "#c0392b".into(), base: Box::new(Node::Text { parts: vec![TextPart::Text(format!("\\{}", call.name))] }) });
        Ok(None)
    }
}

/// Does mathra know this command? Used to recover `\vertA` as `\vert A`.
pub(super) fn known(name: &str) -> bool {
    symbols::command(name).is_some()
        || symbols::greek(name).is_some()
        || symbols::function(name).is_some()
        || symbols::large_operator(name).is_some()
        || symbols::accent(name).is_some()
        || matches!(
            name,
            "begin"
                | "end"
                | "binom"
                | "dbinom"
                | "cfrac"
                | "textbf"
                | "textit"
                | "textrm"
                | "mathit"
                | "boldsymbol"
                | "bmatrix"
                | "pmatrix"
                | "vmatrix"
                | "overbrace"
                | "underbrace"
                | "widehat"
                | "tilde"
                | "dot"
                | "ddot"
                | "big"
                | "Big"
                | "bigg"
                | "Bigg"
                | "newline"
                | "not"
                | "centernot"
                | "notin"
                | "neq"
                | "nabla"
                | "frac"
                | "sqrt"
                | "text"
                | "mathrm"
                | "mathbf"
                | "mathbb"
                | "mathcal"
                | "left"
                | "right"
                | "cdot"
                | "times"
                | "quad"
                | "qquad"
                | "vec"
                | "hat"
                | "bar"
                | "cal"
                | "bf"
                | "rm"
                | "it"
                | "tt"
                | "sf"
                | "overline"
                | "underline"
                | "mathsf"
                | "mathtt"
                | "boxed"
                | "cancel"
                | "dfrac"
                | "tfrac"
        )
}
