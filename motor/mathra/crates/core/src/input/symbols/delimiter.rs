//! Delimiters after \left, \right and \big.

/// Delimiter after `\left`, `\right`, `\big`: a command or a character. `.` is the null one.
pub fn delimiter_command(name: &str) -> Option<Option<char>> {
    Some(Some(match name {
        "{" | "lbrace" => '{',
        "}" | "rbrace" => '}',
        "langle" => '⟨',
        "rangle" => '⟩',
        "lfloor" => '⌊',
        "rfloor" => '⌋',
        "lceil" => '⌈',
        "rceil" => '⌉',
        "vert" | "lvert" | "rvert" | "mid" => '|',
        "|" | "Vert" | "lVert" | "rVert" => '‖',
        "lbrack" => '[',
        "rbrack" => ']',
        "llbracket" | "lBrack" => '⟦',
        "rrbracket" | "rBrack" => '⟧',
        "uparrow" => '↑',
        "downarrow" => '↓',
        "updownarrow" => '↕',
        "Uparrow" => '⇑',
        "Downarrow" => '⇓',
        "Updownarrow" => '⇕',
        "lgroup" => '⟮',
        "lparen" => '(',
        "rparen" => ')',
        "lBrace" => '⦃',
        "rBrace" => '⦄',
        "rgroup" => '⟯',
        "lmoustache" => '⎰',
        "rmoustache" => '⎱',
        "arrowvert" => '⏐',
        "Arrowvert" => '‖',
        "bracevert" => '⎪',
        "backslash" => '\\',
        _ => return None,
    }))
}

pub fn delimiter_char(c: char) -> Option<Option<char>> {
    match c {
        '.' => Some(None),
        '(' | ')' | '[' | ']' | '|' | '/' | '<' | '>' | '⟨' | '⟩' | '‖' => Some(Some(match c {
            '<' => '⟨',
            '>' => '⟩',
            other => other,
        })),
        _ => None,
    }
}
