//! AsciiMath tokenizer. Symbols are a table; the scan that walks the source is separate.

use super::{Kind, Locale, Token};

/// (AsciiMath, LaTeX) for every symbol. The tokenizer takes the longest match.
pub(super) const SYMBOLS: &[(&str, &str)] = &[
    // operations
    ("+", "+"),
    ("-", "-"),
    ("*", "\\cdot"),
    ("**", "\\ast"),
    ("***", "\\star"),
    ("//", "/"),
    ("\\\\", "\\backslash"),
    ("setminus", "\\setminus"),
    ("xx", "\\times"),
    ("|><", "\\ltimes"),
    ("><|", "\\rtimes"),
    ("|><|", "\\bowtie"),
    ("-:", "\\div"),
    ("divide", "\\div"),
    ("@", "\\circ"),
    ("o+", "\\oplus"),
    ("ox", "\\otimes"),
    ("o.", "\\odot"),
    ("sum", "\\sum"),
    ("prod", "\\prod"),
    ("^^", "\\wedge"),
    ("^^^", "\\bigwedge"),
    ("vv", "\\vee"),
    ("vvv", "\\bigvee"),
    ("nn", "\\cap"),
    ("nnn", "\\bigcap"),
    ("uu", "\\cup"),
    ("uuu", "\\bigcup"),
    // relations
    ("=", "="),
    ("!=", "\\ne"),
    ("<", "<"),
    (">", ">"),
    ("<=", "\\le"),
    (">=", "\\ge"),
    ("lt", "<"),
    ("gt", ">"),
    ("le", "\\le"),
    ("ge", "\\ge"),
    ("mlt", "\\ll"),
    ("mgt", "\\gg"),
    ("-<", "\\prec"),
    ("-<=", "\\preceq"),
    (">-", "\\succ"),
    (">-=", "\\succeq"),
    ("in", "\\in"),
    ("!in", "\\notin"),
    ("sub", "\\subset"),
    ("sup", "\\supset"),
    ("sube", "\\subseteq"),
    ("supe", "\\supseteq"),
    ("-=", "\\equiv"),
    ("~=", "\\cong"),
    ("~~", "\\approx"),
    ("~", "\\sim"),
    ("prop", "\\propto"),
    // logic
    ("and", "\\text{ and }"),
    ("or", "\\text{ or }"),
    ("not", "\\neg"),
    ("=>", "\\implies"),
    ("if", "\\text{ if }"),
    ("iff", "\\iff"),
    ("AA", "\\forall"),
    ("EE", "\\exists"),
    ("_|_", "\\bot"),
    ("TT", "\\top"),
    ("|--", "\\vdash"),
    ("|==", "\\models"),
    // miscellaneous
    ("int", "\\int"),
    ("oint", "\\oint"),
    ("del", "\\partial"),
    ("grad", "\\nabla"),
    ("+-", "\\pm"),
    ("-+", "\\mp"),
    ("O/", "\\emptyset"),
    ("oo", "\\infty"),
    ("aleph", "\\aleph"),
    (":.", "\\therefore"),
    (":'", "\\because"),
    ("/_", "\\angle"),
    ("/_\\", "\\triangle"),
    ("'", "'"),
    // the LaTeX for "\\ " is the same space, written so that trimming cannot cut it
    ("\\ ", "~"),
    ("frown", "\\frown"),
    ("quad", "\\quad"),
    ("qquad", "\\qquad"),
    ("cdots", "\\cdots"),
    ("vdots", "\\vdots"),
    ("ddots", "\\ddots"),
    ("ldots", "\\ldots"),
    ("...", "\\ldots"),
    ("diamond", "\\diamond"),
    ("square", "\\square"),
    ("|__", "\\lfloor"),
    ("__|", "\\rfloor"),
    ("|~", "\\lceil"),
    ("~|", "\\rceil"),
    ("CC", "\\mathbb{C}"),
    ("NN", "\\mathbb{N}"),
    ("QQ", "\\mathbb{Q}"),
    ("RR", "\\mathbb{R}"),
    ("ZZ", "\\mathbb{Z}"),
    ("dx", "d x"),
    ("dy", "d y"),
    ("dz", "d z"),
    ("dt", "d t"),
    ("hbar", "\\hbar"),
    ("!", "!"),
    (",", ","),
    (";", ";"),
    (":", ":"),
    (".", "."),
    // functions
    ("sin", "\\sin"),
    ("cos", "\\cos"),
    ("tan", "\\tan"),
    ("sec", "\\sec"),
    ("csc", "\\csc"),
    ("cot", "\\cot"),
    ("arcsin", "\\arcsin"),
    ("arccos", "\\arccos"),
    ("arctan", "\\arctan"),
    ("sinh", "\\sinh"),
    ("cosh", "\\cosh"),
    ("tanh", "\\tanh"),
    ("sech", "\\sech"),
    ("csch", "\\csch"),
    ("coth", "\\coth"),
    ("exp", "\\exp"),
    ("log", "\\log"),
    ("ln", "\\ln"),
    ("det", "\\det"),
    ("dim", "\\dim"),
    ("mod", "\\operatorname{mod}"),
    ("gcd", "\\gcd"),
    ("lcm", "\\operatorname{lcm}"),
    ("lub", "\\operatorname{lub}"),
    ("glb", "\\operatorname{glb}"),
    ("min", "\\min"),
    ("max", "\\max"),
    ("lim", "\\lim"),
    ("Lim", "\\operatorname*{Lim}"),
    // arrows
    ("uarr", "\\uparrow"),
    ("darr", "\\downarrow"),
    ("rarr", "\\rightarrow"),
    ("->", "\\to"),
    (">->", "\\rightarrowtail"),
    ("->>", "\\twoheadrightarrow"),
    (">->>", "\\twoheadrightarrow"),
    ("|->", "\\mapsto"),
    ("larr", "\\leftarrow"),
    ("harr", "\\leftrightarrow"),
    ("rArr", "\\Rightarrow"),
    ("lArr", "\\Leftarrow"),
    ("hArr", "\\Leftrightarrow"),
    // Greek
    ("alpha", "\\alpha"),
    ("beta", "\\beta"),
    ("gamma", "\\gamma"),
    ("Gamma", "\\Gamma"),
    ("delta", "\\delta"),
    ("Delta", "\\Delta"),
    ("epsilon", "\\epsilon"),
    ("epsi", "\\epsilon"),
    ("varepsilon", "\\varepsilon"),
    ("zeta", "\\zeta"),
    ("eta", "\\eta"),
    ("theta", "\\theta"),
    ("Theta", "\\Theta"),
    ("vartheta", "\\vartheta"),
    ("iota", "\\iota"),
    ("kappa", "\\kappa"),
    ("lambda", "\\lambda"),
    ("Lambda", "\\Lambda"),
    ("mu", "\\mu"),
    ("nu", "\\nu"),
    ("xi", "\\xi"),
    ("Xi", "\\Xi"),
    ("pi", "\\pi"),
    ("Pi", "\\Pi"),
    ("rho", "\\rho"),
    ("sigma", "\\sigma"),
    ("Sigma", "\\Sigma"),
    ("tau", "\\tau"),
    ("upsilon", "\\upsilon"),
    ("phi", "\\phi"),
    ("Phi", "\\Phi"),
    ("varphi", "\\varphi"),
    ("chi", "\\chi"),
    ("psi", "\\psi"),
    ("Psi", "\\Psi"),
    ("omega", "\\omega"),
    ("Omega", "\\Omega"),
    // the TeX names asciimath.org accepts too
    ("<=>", "\\iff"),
    ("cdot", "\\cdot"),
    ("ast", "\\ast"),
    ("star", "\\star"),
    ("backslash", "\\backslash"),
    ("times", "\\times"),
    ("div", "\\div"),
    ("ltimes", "\\ltimes"),
    ("rtimes", "\\rtimes"),
    ("bowtie", "\\bowtie"),
    ("circ", "\\circ"),
    ("oplus", "\\oplus"),
    ("otimes", "\\otimes"),
    ("odot", "\\odot"),
    ("wedge", "\\wedge"),
    ("bigwedge", "\\bigwedge"),
    ("vee", "\\vee"),
    ("bigvee", "\\bigvee"),
    ("cap", "\\cap"),
    ("bigcap", "\\bigcap"),
    ("cup", "\\cup"),
    ("bigcup", "\\bigcup"),
    ("partial", "\\partial"),
    ("nabla", "\\nabla"),
    ("pm", "\\pm"),
    ("mp", "\\mp"),
    ("emptyset", "\\emptyset"),
    ("infty", "\\infty"),
    ("therefore", "\\therefore"),
    ("because", "\\because"),
    ("angle", "\\angle"),
    ("triangle", "\\triangle"),
    ("lfloor", "\\lfloor"),
    ("rfloor", "\\rfloor"),
    ("lceiling", "\\lceil"),
    ("rceiling", "\\rceil"),
    ("ne", "\\ne"),
    ("ll", "\\ll"),
    ("gg", "\\gg"),
    ("prec", "\\prec"),
    ("preceq", "\\preceq"),
    ("succ", "\\succ"),
    ("succeq", "\\succeq"),
    ("notin", "\\notin"),
    ("subset", "\\subset"),
    ("supset", "\\supset"),
    ("subseteq", "\\subseteq"),
    ("supseteq", "\\supseteq"),
    ("equiv", "\\equiv"),
    ("cong", "\\cong"),
    ("approx", "\\approx"),
    ("propto", "\\propto"),
    ("neg", "\\neg"),
    ("implies", "\\implies"),
    ("forall", "\\forall"),
    ("exists", "\\exists"),
    ("bot", "\\bot"),
    ("top", "\\top"),
    ("vdash", "\\vdash"),
    ("models", "\\models"),
    ("uparrow", "\\uparrow"),
    ("downarrow", "\\downarrow"),
    ("rightarrow", "\\rightarrow"),
    ("to", "\\to"),
    ("rightarrowtail", "\\rightarrowtail"),
    ("twoheadrightarrow", "\\twoheadrightarrow"),
    ("twoheadrightarrowtail", "\\twoheadrightarrow"),
    ("mapsto", "\\mapsto"),
    ("leftarrow", "\\leftarrow"),
    ("leftrightarrow", "\\leftrightarrow"),
    ("Rightarrow", "\\Rightarrow"),
    ("Leftarrow", "\\Leftarrow"),
    ("Leftrightarrow", "\\Leftrightarrow"),
];

/// Commands with one argument.
const UNARY: &[&str] = &[
    "sqrt",
    "abs",
    "floor",
    "ceil",
    "norm",
    "hat",
    "bar",
    "overline",
    "ul",
    "underline",
    "vec",
    "tilde",
    "dot",
    "ddot",
    "overarc",
    "overparen",
    "underparen",
    "ubrace",
    "underbrace",
    "obrace",
    "overbrace",
    "cancel",
    "bb",
    "mathbf",
    "bbb",
    "mathbb",
    "cc",
    "mathcal",
    "tt",
    "mathtt",
    "fr",
    "mathfrak",
    "sf",
    "mathsf",
    "rm",
    "mathrm",
];

/// Commands with two arguments.
const BINARY: &[&str] = &["frac", "root", "stackrel", "overset", "underset"];

/// Brackets: (text, opening?, drawn delimiter).
const BRACKETS: &[(&str, bool, Option<char>)] = &[
    ("(:", true, Some('⟨')),
    (":)", false, Some('⟩')),
    ("<<", true, Some('⟨')),
    (">>", false, Some('⟩')),
    ("langle", true, Some('⟨')),
    ("rangle", false, Some('⟩')),
    ("{:", true, None),
    (":}", false, None),
    ("(", true, Some('(')),
    (")", false, Some(')')),
    ("[", true, Some('[')),
    ("]", false, Some(']')),
    ("{", true, Some('{')),
    ("}", false, Some('}')),
];

pub(super) fn tokenize(source: &str, locale: Locale) -> Vec<Token> {
    let c: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let col = i + 1;
        if c[i].is_whitespace() {
            i += 1;
            continue;
        }
        // "quoted text"
        if c[i] == '"' {
            tokens.push(Token { kind: Kind::Text(take_quoted(&c, &mut i)), col });
            continue;
        }
        // numbers: digits with a decimal point; a decimal comma in the languages that write it (L-NUM-1)
        if c[i].is_ascii_digit() || (c[i] == '.' && c.get(i + 1).is_some_and(char::is_ascii_digit)) {
            tokens.push(Token { kind: Kind::Number(take_number(&c, &mut i, locale)), col });
            continue;
        }
        // text(…), mbox(…) and color(…): the argument is raw
        if let Some((kind, next)) = raw_command(&c, i) {
            tokens.push(Token { kind, col });
            i = next;
            continue;
        }
        match longest_token(&c, i) {
            Some((len, kind)) => {
                tokens.push(Token { kind, col });
                i += len;
            }
            // nothing matched: one character, a letter or a symbol ("speed" is s p e e d)
            None => {
                tokens.push(Token { kind: Kind::Letter(c[i]), col });
                i += 1;
            }
        }
    }
    tokens
}

fn starts_at(c: &[char], i: usize, text: &str) -> bool {
    c[i..].iter().take(text.chars().count()).copied().eq(text.chars())
}

fn take_quoted(c: &[char], i: &mut usize) -> String {
    let end = c[*i + 1..].iter().position(|&x| x == '"').map_or(c.len(), |p| p + *i + 1);
    let text: String = c[*i + 1..end].iter().collect();
    *i = (end + 1).min(c.len());
    text
}

fn take_number(c: &[char], i: &mut usize, locale: Locale) -> String {
    let mut value = String::new();
    while *i < c.len() {
        let decimal = (c[*i] == '.' || (c[*i] == ',' && locale.comma_is_decimal())) && c.get(*i + 1).is_some_and(char::is_ascii_digit) && !value.contains('.');
        if c[*i].is_ascii_digit() {
            value.push(c[*i]);
        } else if decimal {
            value.push('.');
        } else {
            break;
        }
        *i += 1;
    }
    value
}

fn raw_command(c: &[char], i: usize) -> Option<(Kind, usize)> {
    let raw_commands = ["text", "mbox", "color"];
    let name = raw_commands.iter().find(|n| starts_at(c, i, n))?;
    let mut k = i + name.len();
    while c.get(k).is_some_and(|x| x.is_whitespace()) {
        k += 1;
    }
    let &open = c.get(k).filter(|x| matches!(x, '(' | '[' | '{'))?;
    let close = match open {
        '(' => ')',
        '[' => ']',
        _ => '}',
    };
    let end = c[k + 1..].iter().position(|&x| x == close).map_or(c.len(), |p| p + k + 1);
    let raw: String = c[k + 1..end].iter().collect();
    let kind = if *name == "color" { Kind::Color(raw) } else { Kind::Text(raw) };
    Some((kind, (end + 1).min(c.len())))
}

/// The longest symbol, command or bracket that starts here.
fn longest_token(c: &[char], i: usize) -> Option<(usize, Kind)> {
    let mut best: Option<(usize, Kind)> = None;
    let mut consider = |len: usize, kind: Kind| {
        if best.as_ref().is_none_or(|(l, _)| len > *l) {
            best = Some((len, kind));
        }
    };
    for (text, latex) in SYMBOLS {
        if starts_at(c, i, text) {
            consider(text.chars().count(), Kind::Symbol(latex));
        }
    }
    for name in UNARY {
        if starts_at(c, i, name) {
            consider(name.len(), Kind::Unary(name));
        }
    }
    for name in BINARY {
        if starts_at(c, i, name) {
            consider(name.len(), Kind::Binary(name));
        }
    }
    for (text, open, d) in BRACKETS {
        if starts_at(c, i, text) {
            consider(text.chars().count(), if *open { Kind::Left(*d) } else { Kind::Right(*d) });
        }
    }
    for (text, kind) in [("|", Kind::Bar('|')), ("||", Kind::Bar('‖')), ("/", Kind::Over), ("_", Kind::Sub), ("^", Kind::Sup)] {
        if starts_at(c, i, text) {
            consider(text.len(), kind);
        }
    }
    best
}
