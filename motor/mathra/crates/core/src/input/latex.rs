//! Reader for math LaTeX: the commands and environments listed in the README, plus mathra's extensions
//! (`\divchave`, `\contaarmada`).

use crate::locale::Locale;
use crate::rules::{self, Registry};
use crate::tree::*;
use crate::Error;

use super::symbols::{self, Symbol};

#[cfg(feature = "chem")]
mod chemistry;
mod diagram;

mod accent;
mod atom;
mod boxes;
mod command;
mod define;
mod formula;
mod package;
mod quantity;
mod repair;
mod style;
mod table;
mod text;
mod unit;

use repair::{repair_controls, undo_doubled_backslashes as undo_one_escape};

pub fn read(tex: &str, locale: Locale, registry: &mut Registry) -> Result<Node, Error> {
    read_with(tex, locale, registry, true)
}

/// `strict`: refuse invalid LaTeX like TeX does. Without it, recover what is possible and record warnings.
pub fn read_with(tex: &str, locale: Locale, registry: &mut Registry, strict: bool) -> Result<Node, Error> {
    let clean = prepare(tex, registry, strict);
    let mut macros = Macros { max_size: clean.chars().count() + MAX_GROWTH, ..Macros::default() };
    let mut reader = Reader { c: clean.chars().collect(), pos: 0, locale, registry, macros: &mut macros, variant: Variant::Italic, strict };
    let body = reader.top()?;
    Ok(body)
}

/// Substitution macros defined in the equation itself (`\newcommand`, `\def`, `\let`…).
/// There are no conditionals and no `\catcode`: mathra expands macros, it does not run TeX programs.
#[derive(Default)]
struct Macros {
    commands: std::collections::HashMap<String, Macro>,
    environments: std::collections::HashMap<String, (String, String)>,
    expansions: usize,
    /// How much the text may grow through expansions, counting the original size.
    max_size: usize,
    /// The too-many-expansions warning was already given (there is one, not one per discarded expansion).
    warned_excess: bool,
    /// Groups, commands and scripts open right now: protects the stack from inputs like `{{{{…`.
    depth: usize,
}

#[derive(Clone)]
struct Macro {
    arguments: usize,
    default_arg: Option<String>,
    body: String,
    /// `\DeclarePairedDelimiter`: opening and closing delimiters.
    paired: Option<(String, String)>,
}

/// More than this is almost certainly a macro that calls itself.
const EXPANSION_LIMIT: usize = 2000;
/// Characters the expansions may add to the original text.
const MAX_GROWTH: usize = 200_000;
/// Group, command and script levels open at the same time. Each level costs ~10 KB of stack unoptimized, so 64 fit in 1 MB.
pub const NESTING_LIMIT: usize = 64;
/// Columns of an `array`, even with a repeated `*{n}{…}`.
const MAX_COLUMNS: usize = 1000;

/// Bold markers inside the text of `\text{…}` (Unicode private use area).
pub const BOLD_START: char = '\u{E000}';
pub const BOLD_END: char = '\u{E001}';
/// Small capitals markers (`\textsc{…}`).
pub const SMALL_CAPS_START: char = '\u{E002}';
pub const SMALL_CAPS_END: char = '\u{E003}';
/// Typewriter markers (`\texttt{…}`, `\url{…}`).
pub const MONO_START: char = '\u{E004}';
pub const MONO_END: char = '\u{E005}';
/// Italic markers (`\itshape`); `\textit{…}` and `\emph{…}` close with the second end, after which
/// LaTeX adds the italic correction of the last letter (none before a period or a comma).
pub const ITALIC_START: char = '\u{E006}';
pub const ITALIC_END: char = '\u{E007}';
pub const ITALIC_CORRECTED_END: char = '\u{E008}';
/// Every marker, for the outputs that drop them.
pub const MARKERS: [char; 9] = [BOLD_START, BOLD_END, SMALL_CAPS_START, SMALL_CAPS_END, MONO_START, MONO_END, ITALIC_START, ITALIC_END, ITALIC_CORRECTED_END];

/// L-ENT-1 and L-ENT-2: HTML entities and outer delimiters that arrive with the equation.
fn prepare(tex: &str, registry: &mut Registry, strict: bool) -> String {
    // the repair comes before the trim: a \frac eaten by JSON starts with a form feed, which trim would delete
    let mut s = tex.to_string();
    if !strict {
        if let Some(repaired) = repair_controls(&s) {
            registry.warn(rules::E_REC_2, "JSON control character in place of the backslash (\\frac stored unescaped): command rebuilt");
            s = repaired;
        }
    }
    s = s.trim().to_string();
    if !strict {
        if let Some(undone) = undo_doubled_backslashes(&s) {
            registry.warn(rules::E_REC_2, "doubled backslashes (LaTeX escaped twice): one level undone");
            s = undone;
        }
        // a lone backslash at the end is left over from cut text
        let trailing_backslashes = s.chars().rev().take_while(|c| *c == '\\').count();
        if trailing_backslashes % 2 == 1 {
            registry.warn(rules::E_REC_9, "lone '\\' at the end: ignored");
            s.pop();
        }
    }
    for (open, close) in [("$$", "$$"), ("\\[", "\\]"), ("\\(", "\\)"), ("$", "$")] {
        if s.len() >= open.len() + close.len() && s.starts_with(open) && s.ends_with(close) {
            s = s[open.len()..s.len() - close.len()].trim().to_string();
            registry.note(rules::L_ENT_2, format!("delimitadores externos {open}…{close} removidos"));
            break;
        }
    }
    s = s.replace("\\begingroup", "{").replace("\\endgroup", "}");
    let entities = [("&lt;", "<"), ("&gt;", ">"), ("&amp;", "&"), ("&nbsp;", "~"), ("&quot;", "\""), ("&#39;", "'")];
    for (ent, swap) in entities {
        if s.contains(ent) {
            s = s.replace(ent, swap);
            registry.note(rules::L_ENT_1, format!("HTML entity {ent} read as '{swap}'"));
        }
    }
    s
}

#[derive(Debug, Clone, PartialEq)]
enum Stop {
    Eof,
    CloseBrace,
    E,
    NewRow,
    End(String),
    Right(Option<char>),
    Middle(Option<char>),
    /// `\hline`, or `\hdashline` when dashed.
    Hline(bool),
    /// `\cline{a-b}`, or `\cdashline{a-b}` when dashed.
    Cline(usize, usize, bool),
    /// `\intertext{…}` inside an alignment.
    Intertext(Node),
}

/// A command the dispatcher hands to one group, together with the atoms already read.
struct CommandCall<'a> {
    name: &'a str,
    /// Column of the backslash, for errors and warnings.
    column: usize,
    /// Index of the backslash. An expansion replaces the input from here.
    start: usize,
    starred: bool,
    items: &'a mut Vec<Node>,
}

struct Reader<'a> {
    c: Vec<char>,
    pos: usize,
    locale: Locale,
    registry: &'a mut Registry,
    macros: &'a mut Macros,
    /// Current variant, changed by `\bf`, `\rm`, `\mathrm{…}`.
    variant: Variant,
    strict: bool,
}

type R<T> = Result<T, Error>;

/// Neighbor letters of one variant as one identifier: a word, as a text font sets it.
fn words(node: Node) -> Node {
    let Node::Row { children } = node else { return node };
    let mut merged: Vec<Node> = Vec::with_capacity(children.len());
    for child in children {
        if let (Some(Node::Identifier { name, variant }), Node::Identifier { name: next, variant: v }) = (merged.last_mut(), &child) {
            if variant == v && name.chars().chain(next.chars()).all(char::is_alphabetic) {
                name.push_str(next);
                continue;
            }
        }
        merged.push(child);
    }
    Node::row(merged).simplify()
}

impl Reader<'_> {
    // ---------- utilities ----------

    fn current(&self) -> Option<char> {
        self.c.get(self.pos).copied()
    }

    fn em(&self, d: usize) -> Option<char> {
        self.c.get(self.pos + d).copied()
    }

    fn error<T>(&self, column: usize, message: impl Into<String>) -> R<T> {
        Err(Error::Syntax { column, message: message.into() })
    }

    /// In strict mode, an error; in tolerant mode, a warning, and the caller's recovery goes on.
    fn tolerate(&mut self, rule: &'static str, column: usize, problem: impl Into<String>, recovery: &str) -> R<()> {
        let problem = problem.into();
        if self.strict {
            return Err(Error::Syntax { column, message: problem });
        }
        self.registry.warn(rule, format!("column {column}: {problem}; {recovery}"));
        Ok(())
    }

    fn column(&self) -> usize {
        self.pos + 1
    }

    /// Runs `f` one level deeper. Going past the limit is an error in both modes: there is nothing to recover.
    fn nested<T>(&mut self, f: impl FnOnce(&mut Self) -> R<T>) -> R<T> {
        if self.macros.depth >= NESTING_LIMIT {
            return self.error(self.column(), format!("nesting deeper than {NESTING_LIMIT} levels"));
        }
        self.macros.depth += 1;
        let r = f(self);
        self.macros.depth -= 1;
        r
    }

    fn skip_spaces(&mut self) {
        while self.current().is_some_and(char::is_whitespace) {
            self.pos += 1;
        }
    }

    fn read_command_name(&mut self) -> String {
        // we are already past the backslash
        let mut name = String::new();
        while let Some(c) = self.current().filter(|c| c.is_ascii_alphabetic()) {
            name.push(c);
            self.pos += 1;
        }
        if name.is_empty() {
            if let Some(c) = self.current() {
                name.push(c);
                self.pos += 1;
            }
        }
        name
    }

    /// Raw content between braces, honoring inner braces. Accepts a single character without braces.
    fn raw_text(&mut self) -> R<String> {
        self.skip_spaces();
        let col = self.column();
        match self.current() {
            Some('{') => {
                self.pos += 1;
                let mut level = 1;
                let mut s = String::new();
                while let Some(c) = self.current() {
                    self.pos += 1;
                    match c {
                        '\\' => {
                            s.push(c);
                            if let Some(d) = self.current() {
                                s.push(d);
                                self.pos += 1;
                            }
                            continue;
                        }
                        '{' => level += 1,
                        '}' => {
                            level -= 1;
                            if level == 0 {
                                return Ok(s);
                            }
                        }
                        _ => {}
                    }
                    s.push(c);
                }
                self.tolerate(rules::E_REC_4, col, "'{' without a matching '}'", "closed at the end")?;
                Ok(s)
            }
            // an unbraced command is a whole argument, as in TeX: \boldsymbol\alpha, \mathbb\R
            Some('\\') => {
                let start = self.pos;
                self.pos += 1;
                self.read_command_name();
                Ok(self.c[start..self.pos].iter().collect())
            }
            Some(c) => {
                self.pos += 1;
                Ok(c.to_string())
            }
            None => {
                self.tolerate(rules::E_REC_7, col, "missing argument", "left empty")?;
                Ok(String::new())
            }
        }
    }

    fn raw_optional(&mut self) -> Option<String> {
        let saved = self.pos;
        self.skip_spaces();
        if self.current() != Some('[') {
            self.pos = saved;
            return None;
        }
        self.pos += 1;
        let mut level = 0;
        let mut s = String::new();
        while let Some(c) = self.current() {
            self.pos += 1;
            match c {
                '{' => level += 1,
                '}' => level -= 1,
                ']' if level == 0 => return Some(s),
                _ => {}
            }
            s.push(c);
        }
        self.pos = saved;
        None
    }

    /// Reads a raw snippet as math, with the current variant.
    fn sub_reader(&mut self, tex: &str, variant: Variant) -> R<Node> {
        let mut sub = Reader { c: tex.chars().collect(), pos: 0, locale: self.locale, registry: self.registry, macros: self.macros, variant, strict: self.strict };
        let (items, stop) = sub.list()?;
        match stop {
            Stop::Eof => Ok(Node::row(items).simplify()),
            other => {
                let col = sub.column();
                sub.tolerate(rules::E_REC_5, col, format!("unexpected closing: {other:?}"), "ignored")?;
                Ok(Node::row(items).simplify())
            }
        }
    }

    // ---------- structure ----------

    /// Whole body. Loose `\\` and `&` become rows and columns of an alignment.
    fn top(&mut self) -> R<Node> {
        let mut rows: Vec<Vec<Node>> = Vec::new();
        let mut cells: Vec<Node> = Vec::new();
        let mut had_e = false;
        loop {
            let (items, stop) = self.list()?;
            cells.push(Node::row(items).simplify());
            match stop {
                Stop::Eof => break,
                Stop::E => had_e = true,
                Stop::NewRow => rows.push(std::mem::take(&mut cells)),
                Stop::CloseBrace => self.tolerate(rules::E_REC_4, self.pos, "'}' without a matching '{'", "ignored")?,
                Stop::End(n) => self.tolerate(rules::E_REC_5, self.pos, format!("\\end{{{n}}} without a matching \\begin"), "ignored")?,
                Stop::Right(d) => {
                    self.tolerate(rules::E_REC_5, self.pos, "\\right without a matching \\left", "delimiter drawn alone")?;
                    if let Some(d) = d {
                        cells.push(Node::Operator { symbol: d, class: Class::Close });
                    }
                }
                Stop::Middle(_) => self.tolerate(rules::E_REC_5, self.pos, "\\middle outside \\left…\\right", "ignored")?,
                Stop::Hline(_) | Stop::Cline(..) => {}
                Stop::Intertext(t) => cells.push(t),
            }
        }
        if rows.is_empty() && !had_e {
            return Ok(Node::row(cells.into_iter().flat_map(undo_line).collect()));
        }
        if !cells.iter().all(empty) || rows.is_empty() {
            rows.push(cells);
        }
        let kind = if had_e { TableKind::Aligned } else { TableKind::Centered };
        Ok(Node::Table { shape: kind, rows, columns: vec![], verticals: vec![], dashed: vec![], horizontals: vec![], between: vec![], intertexts: vec![], stretch: 1.0 })
    }

    /// Reads atoms up to a stop (end, `}`, `&`, `\\`, `\end`, `\right`…).
    fn list(&mut self) -> R<(Vec<Node>, Stop)> {
        self.nested(Self::list_unguarded)
    }

    fn list_unguarded(&mut self) -> R<(Vec<Node>, Stop)> {
        let mut items: Vec<Node> = Vec::new();
        loop {
            self.skip_spaces();
            let Some(c) = self.current() else {
                return Ok((items, Stop::Eof));
            };
            match c {
                '}' => {
                    self.pos += 1;
                    return Ok((items, Stop::CloseBrace));
                }
                '&' => {
                    self.pos += 1;
                    return Ok((items, Stop::E));
                }
                '^' | '_' | '\'' => self.scripts(&mut items)?,
                '\\' => {
                    if let Some(p) = self.structural_command(&mut items)? {
                        return Ok((items, p));
                    }
                }
                _ => {
                    let node = self.atom()?;
                    items.extend(node);
                }
            }
        }
    }
}

fn empty(n: &Node) -> bool {
    matches!(n, Node::Row { children } if children.is_empty())
}

fn undo_line(n: Node) -> Vec<Node> {
    match n {
        Node::Row { children } => children,
        other => vec![other],
    }
}

/// `\\frac{\\sqrt{3}}{2}`: every command comes with a doubled backslash. Undoes one level of escaping,
/// as long as there is no command with a single backslash (then the double backslashes are real line breaks).
pub fn undo_doubled_backslashes(s: &str) -> Option<String> {
    undo_one_escape(s)
}

pub(super) fn set_limits(node: &mut Node, wanted: Option<bool>) {
    match node {
        Node::LargeOperator { limits, .. } => *limits = wanted,
        Node::Function { limits, .. } => *limits = wanted.unwrap_or(*limits),
        Node::Scripts { base, .. } => set_limits(base, wanted),
        Node::ForcedClass { base, .. } => set_limits(base, wanted),
        _ => {}
    }
}

pub(super) fn natural_class(node: &Node) -> Class {
    match node {
        Node::Operator { class, .. } => *class,
        Node::LargeOperator { .. } | Node::Function { .. } => Class::Op,
        _ => Class::Ord,
    }
}

pub(super) fn negate(c: char) -> Option<char> {
    Some(match c {
        '=' => '≠',
        '∈' => '∉',
        '⊂' => '⊄',
        '⊃' => '⊅',
        '⊆' => '⊈',
        '⊇' => '⊉',
        '≡' => '≢',
        '<' => '≮',
        '>' => '≯',
        '≤' => '≰',
        '≥' => '≱',
        '∼' => '≁',
        '≈' => '≉',
        '∃' => '∄',
        '∣' => '∤',
        '∥' => '∦',
        '→' => '↛',
        _ => return None,
    })
}

pub(super) fn is_integer(s: &str) -> bool {
    !s.is_empty() && s.len() <= 30 && s.chars().all(|c| c.is_ascii_digit())
}

/// Converts TeX measures to em (1em = 10pt in the 10pt font).
/// A graphicx length: a measure (`1cm`, `0pt`) or a multiple of the box's `\width`, `\height`,
/// `\depth` or `\totalheight` (`2\height`, `-.5\depth`).
pub(super) fn box_length(m: &str) -> Option<BoxLength> {
    let m = m.trim();
    for (name, l) in [
        ("\\totalheight", BoxLength { height: 1.0, depth: 1.0, ..BoxLength::default() }),
        ("\\height", BoxLength { height: 1.0, ..BoxLength::default() }),
        ("\\depth", BoxLength { depth: 1.0, ..BoxLength::default() }),
        ("\\width", BoxLength { width: 1.0, ..BoxLength::default() }),
    ] {
        if let Some(factor) = m.strip_suffix(name) {
            let factor = match factor.trim() {
                "" | "+" => 1.0,
                "-" => -1.0,
                f => f.replace(',', ".").parse::<f32>().ok().filter(|f| f.is_finite())?,
            };
            return Some(BoxLength { em: 0.0, width: l.width * factor, height: l.height * factor, depth: l.depth * factor });
        }
    }
    measure_em(m).filter(|v| v.is_finite()).map(|em| BoxLength { em, ..BoxLength::default() })
}

pub(super) fn measure_em(m: &str) -> Option<f32> {
    let m = m.trim().replace(',', ".");
    let units = [("em", 1.0), ("ex", 0.45), ("pt", 0.1), ("mu", 1.0 / 18.0), ("cm", 2.845), ("mm", 0.2845), ("in", 7.227), ("px", 0.075)];
    for (u, f) in units {
        if let Some(v) = m.strip_suffix(u) {
            return v.trim().parse::<f32>().ok().map(|v| v * f);
        }
    }
    None
}

/// A number in siunitx format: decimal comma or dot, spaces, exponent with `e`.
pub(super) fn siunitx_number(raw: &str, group: bool) -> Option<Node> {
    let clean: String = raw.replace("\\,", "").replace(['{', '}', ' ', '~'], "");
    let (mantissa, exponent) = match clean.split_once(['e', 'E']) {
        Some((m, e)) => (m.to_string(), Some(e.to_string())),
        None => (clean, None),
    };
    let (sign, mantissa) = match mantissa.strip_prefix(['-', '+']) {
        Some(remainder) => (mantissa.chars().next(), remainder.to_string()),
        None => (None, mantissa),
    };
    let value = mantissa.replace(',', ".");
    if value.is_empty() || value.matches('.').count() > 1 || !value.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }
    let mut children = Vec::new();
    if let Some(s) = sign {
        children.push(Node::Operator { symbol: if s == '-' { '−' } else { '+' }, class: Class::Ord });
    }
    children.push(Node::Number { value, group, thousands: None });
    if let Some(e) = exponent {
        let (sign_e, digits) = match e.strip_prefix(['-', '+']) {
            Some(d) => (e.starts_with('-'), d.to_string()),
            None => (false, e.clone()),
        };
        if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let mut exp = Vec::new();
        if sign_e {
            exp.push(Node::Operator { symbol: '−', class: Class::Ord });
        }
        exp.push(Node::Number { value: digits, group: false, thousands: None });
        children.push(Node::Operator { symbol: '×', class: Class::Bin });
        children.push(Node::Scripts {
            base: Box::new(Node::Number { value: "10".into(), group: false, thousands: None }),
            sup: Some(Box::new(Node::row(exp).simplify())),
            sub: None,
        });
    }
    Some(Node::row(children).simplify())
}
