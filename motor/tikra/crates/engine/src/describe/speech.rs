use mathra::locale::Locale;

pub fn num(v: f64) -> String {
    num_in(v, v)
}

/// A number with four significant digits of `span` (a range it is in): TeX's arithmetic leaves
/// 0.000000000069 where 0 was meant, and in a range of 4 that is 0.
pub fn num_in(v: f64, span: f64) -> String {
    if !v.is_finite() {
        return v.to_string();
    }
    let scale = if span.is_finite() && span.abs() > 0.0 { span.abs() } else { v.abs() };
    if v == 0.0 || scale == 0.0 {
        return "0".into();
    }
    let mag = scale.log10().floor() as i32;
    let decimals = (3 - mag).clamp(0, 12) as usize;
    let mut s = format!("{v:.decimals$}");
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if s == "-0" {
        s = "0".into();
    }
    s
}

/// A tick label: a number, perhaps signed, in math or with a unit sign (`$-1$`, `50\%`), in any
/// language, read from its TeX.
pub(in crate::describe) fn is_number(tex: &str) -> bool {
    let mut letters = false;
    let mut digits = false;
    let mut command = false;
    for c in tex.chars() {
        match c {
            '\\' => command = true,
            c if c.is_alphabetic() => letters |= !command,
            c => {
                command = false;
                digits |= c.is_ascii_digit();
            }
        }
    }
    digits && !letters
}

/// A node's TeX as it is read: the math spoken by mathra in `locale`, the commands gone.
pub fn spoken(tex: &str, locale: Locale) -> String {
    let c: Vec<char> = tex.chars().collect();
    let mut out = String::new();
    let mut text = String::new();
    let mut i = 0;
    while i < c.len() {
        // `$…$`, `$$…$$`, `\(…\)`, `\[…\]`
        let open = match (c[i], c.get(i + 1)) {
            ('$', Some('$')) => Some(("$$", 2)),
            ('$', _) => Some(("$", 1)),
            ('\\', Some('(')) => Some(("\\)", 2)),
            ('\\', Some('[')) => Some(("\\]", 2)),
            ('\\', Some(_)) => {
                text.push(c[i]);
                text.push(c[i + 1]);
                i += 2;
                continue;
            }
            _ => None,
        };
        let Some((close, skip)) = open else {
            text.push(c[i]);
            i += 1;
            continue;
        };
        let start = i + skip;
        let rest: String = c[start..].iter().collect();
        let Some(end) = find_close(&rest, close) else {
            text.extend(&c[i..]);
            break;
        };
        out.push_str(&plain(&text));
        text.clear();
        let math = &rest[..end];
        out.push(' ');
        out.push_str(&speak(math, locale));
        out.push(' ');
        i = start + rest[..end].chars().count() + close.chars().count();
    }
    out.push_str(&plain(&text));
    // math spoken next to brackets and punctuation: `($^\circ$C)` is "(degrees C)", `$x$?` "x?"
    let mut out = out.split_whitespace().collect::<Vec<_>>().join(" ").replace("( ", "(");
    for p in [")", ",", ".", ";", ":", "?", "!"] {
        out = out.replace(&format!(" {p}"), p);
    }
    out
}

/// Where `close` ends the math, skipping escaped characters.
fn find_close(s: &str, close: &str) -> Option<usize> {
    let mut i = 0;
    let b = s.as_bytes();
    while i < s.len() {
        if s[i..].starts_with(close) {
            return Some(i);
        }
        if b[i] == b'\\' {
            i += 1;
        }
        i += s[i..].chars().next().map_or(1, char::len_utf8);
    }
    None
}

/// Math as mathra speaks it in `locale`, read as the figure draws it (English conventions: `3.14`).
fn speak(math: &str, locale: Locale) -> String {
    let mut registry = mathra::rules::Registry::default();
    match mathra::input::latex::read(math, Locale::En, &mut registry) {
        Ok(tree) => mathra::output::speech::speak(&tree, locale),
        Err(_) => plain(math),
    }
}

/// Text mode without its commands: `\textbf{a}` is `a`, `\\` a space, `\%` a percent sign.
fn plain(tex: &str) -> String {
    // commands whose first argument is not text
    const DROP_ARG: &[&str] = &["color", "textcolor", "colorbox", "hspace", "vspace", "rule", "includegraphics", "fontsize", "label", "ref", "setlength"];
    const WORDS: &[(&str, &str)] = &[("ldots", "…"), ("dots", "…"), ("textbackslash", "\\"), ("LaTeX", "LaTeX"), ("TeX", "TeX"), ("textdegree", "°"), ("euro", "€")];
    let c: Vec<char> = tex.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '\\' => {
                i += 1;
                if i >= c.len() {
                    break;
                }
                if !c[i].is_alphabetic() {
                    // `\\`, `\,` and friends are space; `\%` and friends the character
                    out.push(if matches!(c[i], '%' | '&' | '$' | '#' | '_' | '{' | '}') { c[i] } else { ' ' });
                    i += 1;
                    if c[i - 1] == '\\' && c.get(i) == Some(&'[') {
                        // `\\[2pt]`
                        while i < c.len() && c[i] != ']' {
                            i += 1;
                        }
                        i += 1;
                    }
                    continue;
                }
                let start = i;
                while i < c.len() && c[i].is_alphabetic() {
                    i += 1;
                }
                let name: String = c[start..i].iter().collect();
                if c.get(i) == Some(&'*') {
                    i += 1;
                }
                if let Some((_, w)) = WORDS.iter().find(|(n, _)| *n == name) {
                    out.push_str(w);
                } else if DROP_ARG.contains(&name.as_str()) {
                    while c.get(i).is_some_and(|ch| ch.is_whitespace()) {
                        i += 1;
                    }
                    if c.get(i) == Some(&'[') {
                        while i < c.len() && c[i] != ']' {
                            i += 1;
                        }
                        i += 1;
                    }
                    if c.get(i) == Some(&'{') {
                        let mut depth = 0;
                        while i < c.len() {
                            match c[i] {
                                '{' => depth += 1,
                                '}' => {
                                    depth -= 1;
                                    if depth == 0 {
                                        i += 1;
                                        break;
                                    }
                                }
                                _ => {}
                            }
                            i += 1;
                        }
                    }
                } else {
                    out.push(' ');
                }
            }
            '{' | '}' => i += 1,
            '~' => {
                out.push(' ');
                i += 1;
            }
            '`' | '\'' if c.get(i + 1) == Some(&c[i]) => {
                out.push(if c[i] == '`' { '\u{201c}' } else { '\u{201d}' });
                i += 2;
            }
            '-' if c.get(i + 1) == Some(&'-') => {
                let three = c.get(i + 2) == Some(&'-');
                out.push(if three { '—' } else { '–' });
                i += if three { 3 } else { 2 };
            }
            ch => {
                out.push(ch);
                i += 1;
            }
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
