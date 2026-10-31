//! Input that arrived escaped twice or with JSON control characters in place of the backslash.

use super::command::known;

/// `\\frac{\\sqrt{3}}{2}`: every command comes with a doubled backslash. Undoes one level of escaping,
/// as long as there is no command with a single backslash (then the double backslashes are real line breaks).
pub(super) fn undo_doubled_backslashes(s: &str) -> Option<String> {
    let c: Vec<char> = s.chars().collect();
    let mut doubles = 0;
    let mut i = 0;
    while i < c.len() {
        if c[i] == '\\' {
            let mut n = 0;
            while i < c.len() && c[i] == '\\' {
                n += 1;
                i += 1;
            }
            if c.get(i).is_some_and(|x| x.is_ascii_alphabetic()) {
                if n % 2 == 1 {
                    return None;
                }
                doubles += 1;
            }
        } else {
            i += 1;
        }
    }
    if doubles == 0 {
        return None;
    }
    let mut output = String::with_capacity(s.len());
    let mut i = 0;
    while i < c.len() {
        if c[i] == '\\' {
            let mut n: usize = 0;
            while i < c.len() && c[i] == '\\' {
                n += 1;
                i += 1;
            }
            output.extend(std::iter::repeat_n('\\', n.div_ceil(2)));
        } else {
            output.push(c[i]);
            i += 1;
        }
    }
    Some(output)
}

/// `\frac` stored in JSON without escaping the backslash becomes form feed + "rac"; `\begin` becomes backspace + "egin";
/// `\times`, `\right`, `\neq` become tab, CR and line feed. Rebuilds the command when the following
/// letters form a known command; otherwise the control character becomes a space.
pub(super) fn repair_controls(s: &str) -> Option<String> {
    let letter_of = |c: char| match c {
        '\u{0C}' => Some('f'),
        '\u{08}' => Some('b'),
        '\u{0B}' => Some('v'),
        '\u{07}' => Some('a'),
        '\t' => Some('t'),
        '\r' => Some('r'),
        '\n' => Some('n'),
        _ => None,
    };
    if !s.chars().any(|c| letter_of(c).is_some()) {
        return None;
    }
    let lines = s.contains("\\\\") || s.contains("\\begin");
    let c: Vec<char> = s.chars().collect();
    let mut output = String::with_capacity(s.len() + 8);
    let mut changed = false;
    for (i, ch) in c.iter().enumerate() {
        let Some(letter) = letter_of(*ch) else {
            output.push(*ch);
            continue;
        };
        let following: String = c[i + 1..].iter().take_while(|x| x.is_ascii_alphabetic()).collect();
        let name = format!("{letter}{following}");
        // a line break or a tab is also what a formula of several lines has: a line `e^x` is not
        // `\ne^x`. There only a long name is unmistakable; a form feed never belongs in a formula.
        let glued = i > 0 && !c[i - 1].is_whitespace();
        let shortest = if matches!(*ch, '\n' | '\r' | '\t') && (lines || glued) { 4 } else { 2 };
        let rebuild = (shortest..=name.len()).rev().any(|n| known(&name[..n]));
        if rebuild {
            output.push('\\');
            output.push(letter);
            changed = true;
        } else {
            output.push(' ');
        }
    }
    changed.then_some(output)
}
