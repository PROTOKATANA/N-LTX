//! Nemeth Braille Code for Mathematics and Science Notation (BANA, 2022). Section numbers (§) refer to
//! that document; `spec/braille.md` lists what is covered and what is not.

mod nodes;
mod signs;

use super::{cells, Braille};
use crate::tree::Node;

pub fn transcribe(node: &Node) -> Braille {
    let mut w = Writer::default();
    w.node(node, &Ctx::default());
    w.finish()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Number,
    /// A letter written alone, which may need the English-letter indicator (§6.3).
    Letter,
    Comparison,
    Function,
    Punctuation,
    Minus,
    Indicator,
    /// Opening and closing grouping signs.
    Open,
    Close,
    /// A sign of shape (∠, ◊): a space follows it (§17, Ex. 6-18).
    Shape,
    /// Since and therefore: spaced from what they apply to (§23.18).
    Spaced,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
enum Item {
    Cells(String, Kind),
    /// A space; `true` after a function name, where a letter takes no English-letter indicator (§6.4.2).
    Space(bool),
    Newline,
}

#[derive(Default)]
struct Writer {
    items: Vec<Item>,
    /// The level of what was written last: "" is the baseline, otherwise its level indicator (§14).
    level: String,
    missing: Vec<char>,
    /// Where the first line of the last system or matrix ends: what follows it goes there (§19.6).
    first_row_end: Option<usize>,
}

#[derive(Debug, Clone, Default)]
struct Ctx {
    /// Level indicator of this part of the expression: "" for the baseline, ⠘ superscript, ⠰ subscript…
    level: String,
    /// Inside an enclosed list (§3.5): no numeric indicator after its spaces, no English-letter indicator.
    list: bool,
    /// Inside a matrix or a system: a numeric indicator on every number (§25.8.1).
    table: bool,
    /// Radicals already open around this one (§16: the inner ones take ⠨ per depth).
    radicals: usize,
}

impl Ctx {
    fn at(&self, level: String) -> Ctx {
        Ctx { level, ..self.clone() }
    }
}

const SUPERSCRIPT: &str = "⠘";
const SUBSCRIPT: &str = "⠰";

impl Writer {
    fn last(&self) -> Option<&Item> {
        self.items.last()
    }

    /// Cells at a level: when the level changes, its indicator first (§14); ⠐ returns to the baseline.
    fn put(&mut self, ctx: &Ctx, text: &str, kind: Kind) {
        if ctx.level != self.level {
            let indicator = if ctx.level.is_empty() { "⠐".to_string() } else { ctx.level.clone() };
            self.items.push(Item::Cells(indicator, Kind::Indicator));
            self.level = ctx.level.clone();
        }
        self.items.push(Item::Cells(text.to_string(), kind));
    }

    fn put_dots(&mut self, ctx: &Ctx, dots: &str, kind: Kind) {
        self.put(ctx, &cells(dots), kind);
    }

    /// A space. After it, what follows is read at `level` with no indicator: the baseline after a
    /// comparison sign (§14.8.7), the level of a function name after the name (§14.8).
    fn space_to(&mut self, level: &str) {
        self.space(level, false);
    }

    fn space(&mut self, level: &str, after_function: bool) {
        if !matches!(self.last(), None | Some(Item::Space(_) | Item::Newline)) {
            self.items.push(Item::Space(after_function));
        }
        self.level = level.to_string();
    }

    fn missing(&mut self, ctx: &Ctx, c: char) {
        if !self.missing.contains(&c) {
            self.missing.push(c);
        }
        self.put(ctx, "⠿", Kind::Other);
    }

    /// Numeric indicator (§3.3): at the start of a line, after a space or a punctuation mark, after a minus
    /// that starts a line or follows a space, and on every number of a matrix.
    fn needs_numeric_indicator(&self, ctx: &Ctx) -> bool {
        if ctx.table {
            return true;
        }
        let n = self.items.len();
        match self.last() {
            None | Some(Item::Newline) => true,
            Some(Item::Space(_)) => !ctx.list,
            Some(Item::Cells(_, Kind::Punctuation)) => true,
            Some(Item::Cells(_, Kind::Minus)) => n < 2 || matches!(self.items[n - 2], Item::Space(_) | Item::Newline),
            _ => false,
        }
    }

    fn number(&mut self, value: &str, ctx: &Ctx) {
        let mut s = String::new();
        if self.needs_numeric_indicator(ctx) && ctx.level == self.level {
            s.push('⠼');
        }
        for c in value.chars() {
            s.push_str(&match c {
                '.' => cells("46"),
                d => cells(signs::digit(d)),
            });
        }
        self.put(ctx, &s, Kind::Number);
    }

    fn finish(mut self) -> Braille {
        self.english_letter_indicators();
        while matches!(self.items.last(), Some(Item::Space(_) | Item::Newline)) {
            self.items.pop();
        }
        let text = self
            .items
            .iter()
            .map(|i| match i {
                Item::Cells(c, _) => c.clone(),
                Item::Space(_) => "⠀".to_string(),
                Item::Newline => "\n".to_string(),
            })
            .collect();
        Braille { text, missing: self.missing }
    }

    /// §6.3: a single letter standing alone takes the English-letter indicator ⠰: a space, a punctuation
    /// mark or the edge on both sides; a grouping sign counts on one side only, as in `(a, 2x, y = z)`
    /// (Ex. 6-30) but not in `(x)`. Never next to a comparison sign (§6.4.7) or after a function name (§6.4.2).
    fn english_letter_indicators(&mut self) {
        let items = self.items.clone();
        let before = |i: usize| -> Option<bool> {
            if i == 0 {
                return Some(false);
            }
            match &items[i - 1] {
                Item::Newline | Item::Cells(_, Kind::Punctuation) => Some(false),
                Item::Space(after_function) => {
                    (!after_function && !matches!(items.get(i.wrapping_sub(2)), Some(Item::Cells(_, Kind::Comparison | Kind::Function)))).then_some(false)
                }
                Item::Cells(_, Kind::Open) => Some(true),
                _ => None,
            }
        };
        let after = |i: usize| -> Option<bool> {
            match items.get(i + 1) {
                None | Some(Item::Newline) | Some(Item::Cells(_, Kind::Punctuation)) => Some(false),
                Some(Item::Space(_)) => (!matches!(items.get(i + 2), Some(Item::Cells(_, Kind::Comparison)))).then_some(false),
                Some(Item::Cells(_, Kind::Close)) => Some(true),
                _ => None,
            }
        };
        for (i, item) in self.items.iter_mut().enumerate() {
            if let Item::Cells(text, Kind::Letter) = item {
                // Some(grouping?) on each side; grouping signs on both sides do not count
                if let (Some(a), Some(b)) = (before(i), after(i)) {
                    if !(a && b) {
                        text.insert(0, '⠰');
                    }
                }
            }
        }
    }
}
