//! Nemeth signs: one symbol to its dot numbers. The tables follow the BANA code (operations,
//! comparisons, grouping, punctuation, and the signs that are none of those).

use super::Kind;
use crate::tree::Variant;

/// The enlarged form of a grouping sign (§19.6): dot 6 after its prefix.
pub(super) fn enlarged(c: char) -> Option<&'static str> {
    Some(match c {
        '(' => "6 12356",
        ')' => "6 23456",
        '[' => "4 6 12356",
        ']' => "4 6 23456",
        '{' => "46 6 12356",
        '}' => "46 6 23456",
        '|' => "6 1256",
        '‖' => "6 1256 6 1256",
        _ => return None,
    })
}

pub(super) fn digit(d: char) -> &'static str {
    match d {
        '1' => "2",
        '2' => "23",
        '3' => "25",
        '4' => "256",
        '5' => "26",
        '6' => "235",
        '7' => "2356",
        '8' => "236",
        '9' => "35",
        _ => "356",
    }
}

/// Greek letters (§6.1.4, §6.1.5): (dots of the letter, capital, alternative form).
pub(super) fn greek(c: char) -> Option<(&'static str, bool, bool)> {
    let lower = c.to_lowercase().next()?;
    let capital = c != lower;
    // capitals have no alternative forms: Φ lowercases to φ, which is the alternative phi
    let (dots, alternative) = match lower {
        'α' => ("1", false),
        'β' => ("12", false),
        'γ' => ("1245", false),
        'δ' => ("145", false),
        'ϵ' | 'ε' => ("15", false),
        'ζ' => ("1356", false),
        'η' => ("156", false),
        'θ' => ("1456", false),
        'ϑ' => ("1456", true),
        'ι' => ("24", false),
        'κ' => ("13", false),
        'λ' => ("123", false),
        'μ' => ("134", false),
        'ν' => ("1345", false),
        'ξ' => ("1346", false),
        'ο' => ("135", false),
        'π' => ("1234", false),
        'ρ' => ("1235", false),
        'σ' => ("234", false),
        'ς' => ("234", true),
        'τ' => ("2345", false),
        'υ' => ("136", false),
        'ϕ' => ("124", false),
        'φ' => ("124", true),
        'χ' => ("12346", false),
        'ψ' => ("13456", false),
        'ω' => ("2456", false),
        _ => return None,
    };
    Some((dots, capital, alternative && !capital))
}

/// Typeform and alphabetic indicators before a letter (§7, §6.2): bold ⠸, double-struck ⠠⠸, script ⠈ and
/// sans-serif ⠠⠨ are followed by the English-letter indicator ⠰, or by the Greek one, which the letter's
/// own cells already carry; German (Fraktur) letters take the German-letter indicator ⠸ (Ex. 6-2).
pub(super) fn typeform(c: char, variant: Variant) -> &'static str {
    let greek = !c.is_ascii_alphabetic();
    match (variant, greek) {
        (Variant::Bold | Variant::BoldItalic, false) => "⠸⠰",
        (Variant::Bold | Variant::BoldItalic, true) => "⠸",
        (Variant::DoubleStruck, false) => "⠠⠸⠰",
        (Variant::DoubleStruck, true) => "⠠⠸",
        (Variant::Calligraphic, false) => "⠈⠰",
        (Variant::Calligraphic, true) => "⠈",
        (Variant::SansSerif, false) => "⠠⠨⠰",
        (Variant::SansSerif, true) => "⠠⠨",
        (Variant::Fraktur, false) => "⠸",
        _ => "",
    }
}

/// The sign of one symbol: operations, comparisons, grouping, punctuation, shapes, or the rest.
pub(super) fn sign(c: char) -> Option<(&'static str, Kind)> {
    operation(c).or_else(|| comparison(c)).or_else(|| grouping(c)).or_else(|| punctuation(c)).or_else(|| shape(c)).or_else(|| other_sign(c))
}

/// Signs and symbols of operation (§20, §23), including the spaced dash of a range.
fn operation(c: char) -> Option<(&'static str, Kind)> {
    use Kind::*;
    Some(match c {
        '+' => ("346", Other),
        '−' | '-' => ("36", Minus),
        '×' => ("4 16", Other),
        '·' | '⋅' => ("16", Other),
        '÷' => ("46 34", Other),
        '±' => ("346 36", Other),
        '∓' => ("36 346", Other),
        '!' => ("12346", Other),
        '%' => ("4 356", Other),
        '∪' | '⋃' => ("46 346", Other),
        '∩' | '⋂' => ("46 146", Other),
        // Signs and Symbols of Operation (Rule 20 table)
        '∗' => ("4 3456", Other),
        '⋆' => ("1246 234", Other),
        '∖' | '\\' => ("456 16", Other),
        // the short dash of ranges (Rule 8 table), spaced like an operation next to a dash (§20.1.1)
        '–' => ("36 36", Spaced),
        '∘' => ("46 16", Other),
        '∧' | '⋀' => ("4 146", Other),
        '∨' | '⋁' => ("4 346", Other),
        '⊙' => ("1246 14 456 1246 16 12456", Other),
        '⊕' => ("1246 14 456 1246 346 12456", Other),
        '⊖' => ("1246 14 456 1246 36 12456", Other),
        '†' => ("456 12456", Other),
        '‡' => ("456 456 12456", Other),
        '⊗' => ("1246 14 456 1246 4 16 12456", Other),
        '/' => ("456 34", Other),
        _ => return None,
    })
}

/// Signs and symbols of comparison (§21, §22), arrows included.
fn comparison(c: char) -> Option<(&'static str, Kind)> {
    use Kind::*;
    Some(match c {
        // Signs and Symbols of Comparison (Rule 21 tables)
        '≺' => ("46 5 13", Comparison),
        '≻' => ("46 46 2", Comparison),
        '⊃' => ("456 46 2", Comparison),
        '⊇' => ("456 46 2 156", Comparison),
        '∋' => ("4 26", Comparison),
        '∥' => ("1246 123", Comparison),
        '∦' => ("34 1246 123", Comparison),
        '⊥' => ("1246 1234", Comparison),
        '∣' => ("1256", Comparison),
        '⪯' | '≼' => ("46 5 13 156", Comparison),
        '⪰' | '≽' => ("46 46 2 156", Comparison),
        '⇐' | '⟸' => ("1246 246 2356 2356", Comparison),
        // more negations, by the rule of §21.1: / before the sign
        '≇' => ("34 4 156 46 13", Comparison),
        '≄' => ("34 4 156 156", Comparison),
        '⇏' => ("34 1246 2356 2356 135", Comparison),
        '⇍' => ("34 1246 246 2356 2356", Comparison),
        '⇎' => ("34 1246 246 2356 2356 135", Comparison),
        '↚' => ("34 1246 246 25 25", Comparison),
        '↮' => ("34 1246 246 25 25 135", Comparison),
        '⊅' => ("34 456 46 2", Comparison),
        '⊉' => ("34 456 46 2 156", Comparison),
        '∌' => ("34 4 26", Comparison),
        '⊀' => ("34 46 5 13", Comparison),
        '⊁' => ("34 46 46 2", Comparison),
        '∤' => ("34 1256", Comparison),
        '≪' => ("5 13 4 5 13 12456", Comparison),
        '≫' => ("46 2 4 46 2 12456", Comparison),
        '≅' => ("4 156 46 13", Comparison),
        '≃' => ("4 156 156", Comparison),
        '≦' => ("5 13 46 13", Comparison),
        '≧' => ("46 2 46 13", Comparison),
        '↑' => ("1246 126 25 25 135", Comparison),
        '↓' => ("1246 146 25 25 135", Comparison),
        '↕' => ("1246 126 246 25 25 135", Comparison),
        // arrows compounded vertically: right over left (equilibrium), long over short, short over long
        '⇌' => ("1246 25 25 135 1246 246 25 25", Comparison),
        '⥂' => ("1246 25 25 25 135 1246 246 25", Comparison),
        '⥄' => ("1246 25 135 1246 246 25 25 25", Comparison),
        '=' => ("46 13", Comparison),
        '≠' => ("34 46 13", Comparison),
        '<' => ("5 13", Comparison),
        '>' => ("46 2", Comparison),
        '≤' => ("5 13 156", Comparison),
        '≥' => ("46 2 156", Comparison),
        '≡' => ("456 123", Comparison),
        '∼' => ("4 156", Comparison),
        '≈' => ("4 156 4 156", Comparison),
        '∝' => ("456 123456", Comparison),
        '∈' => ("4 15", Comparison),
        '∉' => ("34 4 15", Comparison),
        '⊂' => ("456 5 13", Comparison),
        '⊆' => ("456 5 13 156", Comparison),
        '→' => ("1246 135", Comparison),
        '←' => ("1246 246 25 25", Comparison),
        '↔' => ("1246 246 25 25 135", Comparison),
        // long arrows: three shaft cells (§22.5.3)
        '⟶' => ("1246 25 25 25 135", Comparison),
        '⟵' => ("1246 246 25 25 25", Comparison),
        '⟷' => ("1246 246 25 25 25 135", Comparison),
        '⇒' | '⟹' => ("1246 2356 2356 135", Comparison),
        '⇔' | '⟺' => ("1246 246 2356 2356 135", Comparison),
        // negation: / before the comparison sign (§21.1)
        '≮' => ("34 5 13", Comparison),
        '≯' => ("34 46 2", Comparison),
        '≰' => ("34 5 13 156", Comparison),
        '≱' => ("34 46 2 156", Comparison),
        '≢' => ("34 456 123", Comparison),
        '≁' => ("34 4 156", Comparison),
        '≉' => ("34 4 156 4 156", Comparison),
        '⊄' => ("34 456 5 13", Comparison),
        '⊈' => ("34 456 5 13 156", Comparison),
        '↛' => ("34 1246 135", Comparison),
        _ => return None,
    })
}

/// Grouping signs (§19), and the vertical bars.
fn grouping(c: char) -> Option<(&'static str, Kind)> {
    use Kind::*;
    Some(match c {
        // half brackets: lower for floor, upper for ceiling (§19.4)
        '⌊' => ("4 56 12356", Open),
        '⌋' => ("4 56 23456", Close),
        '⌈' => ("4 45 12356", Open),
        '⌉' => ("4 45 23456", Close),
        '(' => ("12356", Open),
        ')' => ("23456", Close),
        '[' => ("4 12356", Open),
        ']' => ("4 23456", Close),
        '{' => ("46 12356", Open),
        '}' => ("46 23456", Close),
        '|' => ("1256", Other),
        '‖' => ("1256 1256", Other),
        '⟨' => ("46 46 12356", Open),
        '⟩' => ("46 46 23456", Close),
        _ => return None,
    })
}

/// Punctuation (§8).
fn punctuation(c: char) -> Option<(&'static str, Kind)> {
    use Kind::*;
    Some(match c {
        ',' => ("6", Punctuation),
        ';' => ("456 23", Punctuation),
        ':' => ("456 25", Punctuation),
        '.' => ("456 256", Punctuation),
        _ => return None,
    })
}

/// Signs of shape (Rule 17), and since / therefore (§23.18).
fn shape(c: char) -> Option<(&'static str, Kind)> {
    use Kind::*;
    Some(match c {
        // Since and therefore (§23.18), and signs of shape (Rule 17)
        '∵' => ("4 34", Spaced),
        '∠' => ("1246 246", Shape),
        '◊' | '⋄' => ("1246 145", Shape),
        '□' | '◻' => ("1246 256", Shape),
        '△' => ("1246 2345", Shape),
        '⌢' => ("1246 1", Shape),
        '∴' => ("6 16", Spaced),
        _ => return None,
    })
}

/// Constants, primes and ellipsis.
fn other_sign(c: char) -> Option<(&'static str, Kind)> {
    use Kind::*;
    Some(match c {
        'ℏ' => ("4 125", Other),
        // Hebrew aleph (§6.1)
        'ℵ' => ("6 6 1", Other),
        '∅' => ("456 356", Other),
        '∞' => ("6 123456", Other),
        '∂' => ("4 145", Other),
        '∇' | '𝛁' => ("46 1246", Other),
        '∀' => ("4 12346", Other),
        '∃' => ("4 123456", Other),
        '′' | '\'' => ("3", Other),
        '″' => ("3 3", Other),
        '…' | '⋯' => ("3 3 3", Other),
        _ => return None,
    })
}
