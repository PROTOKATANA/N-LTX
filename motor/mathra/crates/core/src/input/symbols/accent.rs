//! Math accents and the precomposed text accents.

/// Accents: (combining character, stretches with the base).
pub fn accent(name: &str) -> Option<(char, bool)> {
    Some(match name {
        "hat" => ('\u{0302}', false),
        "widehat" => ('\u{0302}', true),
        "check" => ('\u{030C}', false),
        "widecheck" => ('\u{030C}', true),
        "tilde" => ('\u{0303}', false),
        "widetilde" => ('\u{0303}', true),
        "bar" => ('\u{0304}', false),
        "acute" => ('\u{0301}', false),
        "grave" => ('\u{0300}', false),
        "breve" => ('\u{0306}', false),
        "dot" => ('\u{0307}', false),
        "ddot" => ('\u{0308}', false),
        "dddot" => ('\u{20DB}', false),
        "ddddot" => ('\u{20DC}', false),
        "mathring" => ('\u{030A}', false),
        "Tilde" => ('\u{0303}', false),
        "Hat" => ('\u{0302}', false),
        "Bar" => ('\u{0304}', false),
        "Vec" => ('\u{20D7}', false),
        "Dot" => ('\u{0307}', false),
        "Ddot" => ('\u{0308}', false),
        "vec" => ('\u{20D7}', false),
        "overrightarrow" => ('→', true),
        "overleftarrow" => ('←', true),
        "overleftrightarrow" => ('↔', true),
        // harpoon package
        "overleftharpoon" => ('↼', true),
        "overrightharpoon" => ('⇀', true),
        "overarc" | "wideparen" | "overparen" | "arc" => ('⏜', true),
        "underparen" => ('⏝', true),
        _ => return None,
    })
}

/// Text-mode accents (`\'e`, `\~a`, `\c{c}`) composed into one character.
pub fn text_accent(mark: char, letter: char) -> Option<char> {
    let combining = text_combining(mark)?;
    let base = if letter == 'i' { 'ı' } else { letter };
    compose(base, combining).or_else(|| compose(letter, combining))
}

/// The combining accent of each text-mode accent mark.
pub fn text_combining(mark: char) -> Option<char> {
    Some(match mark {
        '\'' => '\u{0301}',
        '`' => '\u{0300}',
        '^' => '\u{0302}',
        '~' => '\u{0303}',
        '"' => '\u{0308}',
        'c' => '\u{0327}',
        '=' => '\u{0304}',
        '.' => '\u{0307}',
        'u' => '\u{0306}',
        'v' => '\u{030C}',
        'H' => '\u{030B}',
        'r' => '\u{030A}',
        'k' => '\u{0328}',
        'b' => '\u{0331}',
        'd' => '\u{0323}',
        't' => '\u{0361}',
        _ => return None,
    })
}

fn compose(base: char, combining: char) -> Option<char> {
    const TABLE: &[(char, char, char)] = &[
        ('a', '\u{0301}', 'á'),
        ('e', '\u{0301}', 'é'),
        ('ı', '\u{0301}', 'í'),
        ('i', '\u{0301}', 'í'),
        ('o', '\u{0301}', 'ó'),
        ('u', '\u{0301}', 'ú'),
        ('A', '\u{0301}', 'Á'),
        ('E', '\u{0301}', 'É'),
        ('I', '\u{0301}', 'Í'),
        ('O', '\u{0301}', 'Ó'),
        ('U', '\u{0301}', 'Ú'),
        ('a', '\u{0300}', 'à'),
        ('A', '\u{0300}', 'À'),
        ('a', '\u{0302}', 'â'),
        ('e', '\u{0302}', 'ê'),
        ('o', '\u{0302}', 'ô'),
        ('A', '\u{0302}', 'Â'),
        ('E', '\u{0302}', 'Ê'),
        ('O', '\u{0302}', 'Ô'),
        ('a', '\u{0303}', 'ã'),
        ('o', '\u{0303}', 'õ'),
        ('n', '\u{0303}', 'ñ'),
        ('A', '\u{0303}', 'Ã'),
        ('O', '\u{0303}', 'Õ'),
        ('u', '\u{0308}', 'ü'),
        ('o', '\u{0308}', 'ö'),
        ('a', '\u{0308}', 'ä'),
        ('U', '\u{0308}', 'Ü'),
        ('c', '\u{0327}', 'ç'),
        ('C', '\u{0327}', 'Ç'),
        ('e', '\u{0300}', 'è'),
        ('ı', '\u{0300}', 'ì'),
        ('o', '\u{0300}', 'ò'),
        ('u', '\u{0300}', 'ù'),
        ('ı', '\u{0302}', 'î'),
        ('u', '\u{0302}', 'û'),
        ('e', '\u{0308}', 'ë'),
        ('ı', '\u{0308}', 'ï'),
        ('a', '\u{0304}', 'ā'),
        ('e', '\u{0304}', 'ē'),
        ('ı', '\u{0304}', 'ī'),
        ('o', '\u{0304}', 'ō'),
        ('u', '\u{0304}', 'ū'),
        ('z', '\u{0307}', 'ż'),
        ('e', '\u{0307}', 'ė'),
        ('a', '\u{0306}', 'ă'),
        ('g', '\u{0306}', 'ğ'),
        ('s', '\u{030C}', 'š'),
        ('c', '\u{030C}', 'č'),
        ('z', '\u{030C}', 'ž'),
        ('e', '\u{030C}', 'ě'),
        ('r', '\u{030C}', 'ř'),
        ('n', '\u{030C}', 'ň'),
        ('S', '\u{030C}', 'Š'),
        ('C', '\u{030C}', 'Č'),
    ];
    TABLE.iter().find(|(b, c, _)| *b == base && *c == combining).map(|(_, _, r)| *r)
}
