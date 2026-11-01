//! Reading arrow specifications and the names of the tips.

use std::collections::HashMap;

use crate::scene::Segment;

pub fn parse_arrows(spec: &str, user: &UserTips) -> Option<(Option<String>, Option<String>)> {
    let c: Vec<char> = spec.trim().chars().collect();
    let mut level = 0;
    let mut cut = None;
    for (i, ch) in c.iter().enumerate() {
        match ch {
            '{' | '[' => level += 1,
            '}' | ']' => level -= 1,
            '-' if level == 0 => {
                cut = Some(i);
                break;
            }
            _ => {}
        }
    }
    let i = cut?;
    let a: String = c[..i].iter().collect();
    let b: String = c[i + 1..].iter().collect();
    let side = |s: &str, start: bool| -> Option<Option<String>> {
        let s = s.trim();
        if s.is_empty() {
            return Some(None);
        }
        sequence(s, start, user, 0).map(|_| Some(s.to_string()))
    };
    let start = side(&a, true)?;
    let end = side(&b, false)?;
    if start.is_none() && end.is_none() && !spec.trim().starts_with('-') {
        return None;
    }
    Some((start, end))
}

/// Arrow tips declared in the document (`name/.tip=…`, `>=…`): what each end name means, and the end
/// name each start name stands for. Kept in the graphic state, so a group ends them, as in pgf.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UserTips {
    pub means: HashMap<String, String>,
    pub starts: HashMap<String, String>,
}

impl UserTips {
    /// `\pgfdeclarearrow{name=…, means=…}`: `name` is `start-end`, or one name for both ends.
    pub fn declare(&mut self, name: &str, means: &str) {
        let name = name.trim();
        let (start, end) = match name.split_once('-') {
            Some((a, b)) => (a.trim(), b.trim()),
            None => (name, name),
        };
        self.starts.insert(start.to_string(), end.to_string());
        self.means.insert(end.to_string(), strip_group(means.trim()).to_string());
    }
}

/// When tips are drawn (`tips=`): pgf's `\pgf@tips@mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TipsMode {
    Never,
    /// on stroked paths (the default)
    #[default]
    OnDraw,
    /// also on paths that are neither stroked nor filled
    True,
    /// on stroked paths that do not stay at one point
    OnProperDraw,
    Proper,
}

impl TipsMode {
    pub fn from_name(s: &str) -> Option<TipsMode> {
        Some(match s.trim() {
            "" | "true" => TipsMode::True,
            "on draw" => TipsMode::OnDraw,
            "never" | "false" => TipsMode::Never,
            "on proper draw" => TipsMode::OnProperDraw,
            "proper" => TipsMode::Proper,
            _ => return None,
        })
    }
}

// ------------------------------------------------------------------------------------------ names

/// A kind of tip with drawing code: those of `arrows.meta`, and the older ones.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    StraightBarb,
    Hooks,
    ArcBarb,
    TeeBarb,
    ClassicalRightarrow,
    CmRightarrow,
    Implies,
    Latex,
    Stealth,
    Kite,
    Square,
    Circle,
    RoundCap,
    ButtCap,
    TriangleCap,
    FastTriangle,
    FastRound,
    Rays,
    /// CircuiTikZ's `Jack Tap`: a step then half a triangle
    JackTap,
    /// `_`: only space
    Underscore,
    Old(Old),
}

/// The tips declared with `\pgfarrowsdeclare` (pgf's core and the `arrows` library).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Old {
    Stealth,
    To,
    ToReversed,
    Latex,
    Bar,
    Space,
    SquareBracket,
    Parenthesis,
    Angle(u8),
    Dot,
    Ring,
    Diamond,
    OpenDiamond,
    Square,
    OpenSquare,
    Triangle(u8),
    OpenTriangle(u8),
    OpenTriangleReversed(u8),
    LatexPrime,
    StealthPrime,
    LeftTo,
    RightTo,
    LeftToReversed,
    RightToReversed,
    LeftHook,
    RightHook,
    Hooks,
    SerifCm,
    RoundCap,
    ButtCap,
    Triangle90Cap,
    Triangle90CapReversed,
    FastCap,
    FastCapReversed,
    Implies,
    /// the circuits' `direction ee`, as long as its `length` (`\pgfsetarrowoptions`)
    DirectionEe,
    /// one the document declared with `\pgfarrowsdeclare`: its place among them
    User(u16),
}

/// A tip the document declared with `\pgfarrowsdeclare{start}{end}{extents}{code}`: its names and
/// what its code drew (pt, the tip pointing right with its origin where the line ends).
#[derive(Debug, Clone)]
pub struct DeclaredTip {
    pub names: (String, String),
    pub left: f64,
    pub right: f64,
    /// each path, whether it is filled and stroked, and its line width
    pub paths: Vec<(Vec<Segment>, bool, bool, f64)>,
}

thread_local! {
    /// The tips the document declared, for the figure being drawn.
    pub(crate) static DECLARED: std::cell::RefCell<Vec<DeclaredTip>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Forgets the tips declared for another figure.
pub fn clear_declared() {
    DECLARED.with(|d| d.borrow_mut().clear());
}

/// Keeps a tip the document declared (a later one of a name wins).
pub fn declare_tip(tip: DeclaredTip) {
    DECLARED.with(|d| d.borrow_mut().push(tip));
}

fn declared(name: &str) -> Option<Kind> {
    DECLARED.with(|d| d.borrow().iter().rposition(|t| t.names.0 == name || t.names.1 == name)).map(|i| Kind::Old(Old::User(i as u16)))
}

fn code(name: &str) -> Option<Kind> {
    use Kind::*;
    let angle = |s: &str| match s {
        "90" => Some(90),
        "60" => Some(60),
        "45" => Some(45),
        _ => None,
    };
    Some(match name {
        "Straight Barb" => StraightBarb,
        "Hooks" => Hooks,
        "Arc Barb" => ArcBarb,
        "Tee Barb" => TeeBarb,
        "Classical TikZ Rightarrow" => ClassicalRightarrow,
        "Computer Modern Rightarrow" => CmRightarrow,
        "Implies" => Implies,
        "Latex" => Latex,
        "Stealth" => Stealth,
        "Kite" => Kite,
        "Square" => Square,
        "Circle" => Circle,
        "Round Cap" => RoundCap,
        "Butt Cap" => ButtCap,
        "Triangle Cap" => TriangleCap,
        "Fast Triangle" => FastTriangle,
        "Fast Round" => FastRound,
        "Rays" => Rays,
        "Jack Tap" => JackTap,
        "_" => Underscore,
        name if declared(name).is_some() => return declared(name),
        "stealth" => Old(self::Old::Stealth),
        "to" => Old(self::Old::To),
        "to reversed" => Old(self::Old::ToReversed),
        "latex" => Old(self::Old::Latex),
        "@bar" => Old(self::Old::Bar),
        "space" => Old(self::Old::Space),
        "square bracket" => Old(self::Old::SquareBracket),
        ")" => Old(self::Old::Parenthesis),
        "*" => Old(self::Old::Dot),
        "o" => Old(self::Old::Ring),
        "diamond" => Old(self::Old::Diamond),
        "open diamond" => Old(self::Old::OpenDiamond),
        "square" => Old(self::Old::Square),
        "open square" => Old(self::Old::OpenSquare),
        "latex'" => Old(self::Old::LatexPrime),
        "stealth'" => Old(self::Old::StealthPrime),
        "left to" => Old(self::Old::LeftTo),
        "right to" => Old(self::Old::RightTo),
        "left to reversed" => Old(self::Old::LeftToReversed),
        "right to reversed" => Old(self::Old::RightToReversed),
        "left hook" => Old(self::Old::LeftHook),
        "right hook" => Old(self::Old::RightHook),
        "hooks" => Old(self::Old::Hooks),
        "serif cm" => Old(self::Old::SerifCm),
        "round cap" => Old(self::Old::RoundCap),
        "butt cap" => Old(self::Old::ButtCap),
        "triangle 90 cap" => Old(self::Old::Triangle90Cap),
        "triangle 90 cap reversed" => Old(self::Old::Triangle90CapReversed),
        "fast cap" => Old(self::Old::FastCap),
        "fast cap reversed" => Old(self::Old::FastCapReversed),
        "implies" => Old(self::Old::Implies),
        "tikra direction ee" => Old(self::Old::DirectionEe),
        _ => {
            if let Some(a) = name.strip_prefix("angle ").and_then(angle) {
                Old(self::Old::Angle(a))
            } else if let Some(a) = name.strip_prefix("triangle ").and_then(angle) {
                Old(self::Old::Triangle(a))
            } else {
                let r = name.strip_prefix("open triangle ")?;
                match r.strip_suffix(" reversed") {
                    Some(a) => Old(self::Old::OpenTriangleReversed(angle(a)?)),
                    None => Old(self::Old::OpenTriangle(angle(r)?)),
                }
            }
        }
    })
}

/// Tips whose reversed form is declared with `\pgfarrowsdeclarereversed` (`stealth reversed`…).
const REVERSIBLE: [&str; 13] =
    ["stealth", "latex", "angle 90", "angle 60", "angle 45", "triangle 90", "triangle 60", "triangle 45", "latex'", "stealth'", "left hook", "right hook", "hooks"];

enum Def {
    Code(Kind),
    Means(String),
}

/// What an end name stands for: drawing code, or a shorthand for other tips.
fn definition(name: &str, user: &UserTips, depth: usize) -> Option<Def> {
    if let Some(m) = user.means.get(name) {
        return Some(Def::Means(m.clone()));
    }
    if let Some(k) = code(name) {
        return Some(Def::Code(k));
    }
    let m = match name {
        // TikZ's `>` (redefined by `>=`) and pgf's own shorthands
        ">" => "To",
        "<" => ">[reversed]",
        "|" => "Bar",
        ">|" => ">[sep=0pt].|",
        // arrows.meta's
        "To" => "Computer Modern Rightarrow",
        "Bar" => "Tee Barb[length=+0pt]",
        "LaTeX" => "Latex",
        "Triangle" => "Stealth[inset=+0pt, angle=+60:+2.7pt +3.6]",
        "Rectangle" => "Square[length=+3pt +4.5,width'=+0pt +.5]",
        "Ellipse" => "Circle[length=+3.3pt +4.95, width'=+0pt +0.5]",
        "Diamond" => "Kite[inset'=+0pt .5]",
        "Turned Square" => "Kite[length=+3pt 4,width'= +0pt 1,inset'= +0pt 0.5]",
        "Bracket" => "Tee Barb[inset'=+0pt +1,length=+0.75pt +1]",
        "Parenthesis" => "Arc Barb[arc=+120,length=+1.725pt +2.3]",
        "direction ee" => "tikra direction ee[length=4pt]",
        // the arrows library's brackets
        "]" => "square bracket",
        "[" => "][reversed]",
        "(" => ")[reversed]",
        "spaced [" => "square bracket[reversed].space",
        "spaced ]" => "square bracket[].space",
        _ => {
            if let Some(base) = name.strip_suffix(" reversed").filter(|b| REVERSIBLE.contains(b)) {
                return Some(Def::Means(format!("{base}[reversed]")));
            }
            // arrows.spaced: the tip, then a space
            if let Some(base) = name.strip_prefix("spaced ") {
                if depth < 4 && definition(base, user, depth + 1).is_some() {
                    return Some(Def::Means(format!("{base}[sep=0pt].space")));
                }
            }
            return None;
        }
    };
    Some(Def::Means(m.to_string()))
}

/// The end name a start name stands for (`<` is the start of `>`).
fn start_to_end(name: &str, user: &UserTips) -> Option<String> {
    if let Some(e) = user.starts.get(name) {
        return Some(e.clone());
    }
    let e = match name {
        "<" => ">",
        ">" => "<",
        "(" => ")",
        ")" => "(",
        "[" => "]",
        "]" => "[",
        "|<" => ">|",
        "spaced (" => "spaced )",
        "spaced )" => "spaced (",
        "spaced [" => "spaced ]",
        "spaced ]" => "spaced [",
        ">|" => return None,
        _ => return definition(name, user, 0).map(|_| name.to_string()),
    };
    Some(e.to_string())
}

// ---------------------------------------------------------------------------------------- parsing

/// An entry of a tip sequence: a tip with its option lists (its own, then those of the shorthands
/// it came from), or `.`, where the line ends.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Raw {
    Tip { kind: Kind, opts: Vec<String> },
    Dot,
}

/// One level of braces around the whole text, as TeX strips them from an argument.
pub(crate) fn strip_group(s: &str) -> &str {
    if s.starts_with('{') && s.ends_with('}') {
        let mut level = 0;
        for (i, ch) in s.char_indices() {
            match ch {
                '{' => level += 1,
                '}' => {
                    level -= 1;
                    if level == 0 && i != s.len() - 1 {
                        return s;
                    }
                }
                _ => {}
            }
        }
        return s[1..s.len() - 1].trim();
    }
    s
}

/// The text inside the brackets that open at `c[i]`, and the index after them.
fn bracket(c: &[char], i: usize) -> Option<(String, usize)> {
    let mut level = 0;
    for (j, &ch) in c.iter().enumerate().skip(i) {
        match ch {
            '[' | '{' => level += 1,
            ']' | '}' => {
                level -= 1;
                if level == 0 {
                    return Some((c[i + 1..j].iter().collect(), j + 1));
                }
            }
            _ => {}
        }
    }
    None
}

/// The tips of a specification, from the back of the arrow to its point, as pgf's parser reads them
/// (`\pgfarrows@parser`): a name up to the options is a tip or a shorthand; otherwise its first
/// character is. At the start of a path names are those of starts, and each tip goes before the
/// ones already read. None if a name is unknown.
pub(crate) fn sequence(spec: &str, start: bool, user: &UserTips, depth: usize) -> Option<Vec<Raw>> {
    if depth > 12 {
        return None;
    }
    let s = strip_group(spec.trim());
    // brackets alone cannot be read as names before options: pgf has them directly
    if matches!(s, "[" | "]" | "spaced [" | "spaced ]") {
        let end = if start { start_to_end(s, user)? } else { s.to_string() };
        return expand(definition(&end, user, 0)?, Vec::new(), user, depth);
    }
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut initial = None;
    if c.first() == Some(&'[') {
        let (o, j) = bracket(&c, 0)?;
        initial = Some(o).filter(|o| !o.trim().is_empty());
        i = j;
    }
    let resolve = |name: &str| -> Option<Def> {
        let end = if start { start_to_end(name, user)? } else { name.to_string() };
        definition(&end, user, 0)
    };
    let mut out: Vec<Raw> = Vec::new();
    let push = |out: &mut Vec<Raw>, block: Vec<Raw>| {
        if start {
            out.splice(0..0, block);
        } else {
            out.extend(block);
        }
    };
    while i < c.len() {
        if c[i].is_whitespace() {
            i += 1;
            continue;
        }
        let mut j = i;
        let mut level = 0;
        while j < c.len() && !(c[j] == '[' && level == 0) {
            match c[j] {
                '{' => level += 1,
                '}' => level -= 1,
                _ => {}
            }
            j += 1;
        }
        let name: String = c[i..j].iter().collect::<String>().trim().to_string();
        let (local, next) = if j < c.len() { bracket(&c, j)? } else { (String::new(), c.len()) };
        let mut opts: Vec<String> = initial.iter().cloned().collect();
        if name == "." {
            push(&mut out, vec![Raw::Dot]);
            i = next;
            continue;
        }
        if name.is_empty() {
            i = next;
            continue;
        }
        if let Some(def) = resolve(strip_group(&name)) {
            if !local.trim().is_empty() {
                opts.push(local);
            }
            push(&mut out, expand(def, opts, user, depth)?);
            i = next;
            continue;
        }
        // a one-character tip; the rest is read again
        let first = name.chars().next()?.to_string();
        if first == "." {
            push(&mut out, vec![Raw::Dot]);
        } else {
            push(&mut out, expand(resolve(&first)?, opts, user, depth)?);
        }
        i += first.len().max(1);
    }
    Some(out)
}

/// The tips of a definition with options: a shorthand's tips get them after their own.
fn expand(def: Def, opts: Vec<String>, user: &UserTips, depth: usize) -> Option<Vec<Raw>> {
    match def {
        Def::Code(kind) => Some(vec![Raw::Tip { kind, opts }]),
        Def::Means(m) => {
            let mut seq = sequence(&m, false, user, depth + 1)?;
            for r in &mut seq {
                if let Raw::Tip { opts: o, .. } = r {
                    o.extend(opts.iter().cloned());
                }
            }
            Some(seq)
        }
    }
}
