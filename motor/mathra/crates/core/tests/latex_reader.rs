//! LaTeX reader (input/latex.rs and input/symbols.rs): every command and environment the reader
//! knows, the recovery paths of tolerant mode and, at the end, problem inputs that
//! must not crash or hang the reader.

use mathra::input::latex::{
    read_with, undo_doubled_backslashes, BOLD_END, BOLD_START, ITALIC_CORRECTED_END, ITALIC_END, ITALIC_START, MONO_END, MONO_START, NESTING_LIMIT, SMALL_CAPS_END,
    SMALL_CAPS_START,
};
use mathra::input::symbols;
use mathra::rules::Registry;
use mathra::{convert, Error, Locale, Node, Options};

fn read_in(tex: &str, locale: Locale, strict: bool) -> (Result<Node, Error>, Registry) {
    let mut registry = Registry::default();
    let r = read_with(tex, locale, &mut registry, strict);
    (r, registry)
}

/// Tolerant mode's tree (as Debug, to search for pieces) and the warnings.
fn tolerant(tex: &str) -> (String, Vec<String>) {
    let (r, registry) = read_in(tex, Locale::PtBr, false);
    let node = r.unwrap_or_else(|e| panic!("{tex}: {e}"));
    (format!("{node:?}"), registry.warnings.iter().map(|a| format!("{}: {}", a.rule, a.description)).collect())
}

/// Tree of a valid input: read the same in both modes, with no warnings.
fn tree(tex: &str) -> String {
    let (strict, _) = read_in(tex, Locale::PtBr, true);
    let strict = strict.unwrap_or_else(|e| panic!("{tex}: {e}"));
    let (t, warnings) = tolerant(tex);
    assert!(warnings.is_empty(), "{tex}: {warnings:?}");
    assert_eq!(format!("{strict:?}"), t, "{tex}");
    t
}

/// Error message in strict mode.
fn strict_error(tex: &str) -> String {
    match read_in(tex, Locale::PtBr, true).0 {
        Ok(node) => panic!("{tex}: strict mode accepted: {node:?}"),
        Err(e) => e.to_string(),
    }
}

/// Error message, which also happens in tolerant mode.
fn always_error(tex: &str) -> String {
    let (r, _) = read_in(tex, Locale::PtBr, false);
    match r {
        Ok(node) => panic!("{tex}: tolerant mode accepted: {node:?}"),
        Err(e) => e.to_string(),
    }
}

/// Invalid: strict mode refuses, tolerant mode draws with a warning from the expected rule.
fn recovers(tex: &str, rule: &str) -> String {
    strict_error(tex);
    let (t, a) = tolerant(tex);
    assert!(a.iter().any(|a| a.starts_with(rule)), "{tex}: expected {rule}, got {a:?}");
    t
}

fn has(t: &str, snippets: &[&str]) {
    for snippet in snippets {
        assert!(t.contains(snippet), "missing {snippet:?} in {t}");
    }
}

// ---------- input preparation ----------

#[test]
fn lone_trailing_backslash_is_ignored() {
    let t = recovers(r"x + 1\", "E-REC-9");
    has(&t, &["\"x\"", "\"1\""]);
}

#[test]
fn json_control_characters_become_commands() {
    // \v, \a, \r and \n stored unescaped become VT, BEL, CR and LF
    for (tex, expected) in [("\u{0B}ec{v}", "Accent"), ("\u{07}lpha", "α"), ("\\left( x \right)", "Delimited"), ("a \neq b", "≠")] {
        let (t, a) = tolerant(tex);
        assert!(t.contains(expected), "{tex:?}: {t}");
        assert!(a.iter().any(|a| a.starts_with("E-REC-2")), "{tex:?}: {a:?}");
    }
}

// ---------- raw arguments ----------

#[test]
fn raw_argument_single_char_or_unclosed() {
    has(&tree(r"\mathrm x"), &["\"x\"", "Upright"]);
    let t = recovers(r"\text{abc", "E-REC-4");
    has(&t, &["abc"]);
    recovers(r"\text", "E-REC-7");
    recovers(r"\mathrm", "E-REC-7");
}

#[test]
fn optional_with_braces_and_unclosed() {
    has(&tree(r"\sqrt[\frac{1}{2}]{x}"), &["Root", "index: Some(Fraction"]);
    // '[' without ']' is not an optional argument: the bracket becomes the radicand
    has(&tree(r"\sqrt[3 x"), &["Root", "index: None", "'['"]);
}

#[test]
fn sub_reader_stopping_before_the_end() {
    let t = recovers(r"\mathrm{a & b}", "E-REC-5");
    has(&t, &["\"a\""]);
}

// ---------- top-level structure ----------

#[test]
fn top_level_ampersand_and_break_become_alignment() {
    has(&tree("a &= b"), &["Table", "Aligned"]);
    has(&tree(r"a \\ b"), &["Table", "Centered"]);
    // a break at the end does not create an empty row
    let t = tree(r"a \\ b \\");
    assert_eq!(t.matches("Row { children: [Identifier").count(), 0, "{t}");
    has(&tree(r"a \hline b \cline{1-1} c"), &["\"c\""]);
    assert!(always_error(r"a \cline{x-y} b").contains("the form"));
    has(&tree(r"a \\ \intertext{and also} b"), &["and also"]);
}

#[test]
fn closings_without_opening_at_top() {
    recovers(r"x \end{foo}", "E-REC-5");
    has(&recovers(r"x \right)", "E-REC-5"), &["')'", "Close"]);
    recovers(r"x \right.", "E-REC-5");
    recovers(r"a \middle| b", "E-REC-5");
    recovers("a } b", "E-REC-4");
}

// ---------- atoms ----------

#[test]
fn special_atoms() {
    has(&tree("a~b"), &["Space"]);
    // a dollar sign in the middle: a leftover delimiter
    has(&tree("a$b"), &["\"a\"", "\"b\""]);
    has(&recovers("a # b", "E-REC-9"), &["'#'"]);
    has(&tree("a*b"), &["'∗'"]);
}

#[test]
fn group_ending_in_something_else() {
    recovers(r"{a \end{x}}", "E-REC-4");
    recovers("{a & b}", "E-REC-4");
}

#[test]
fn thousands_followed_by_short_group_is_not_thousands() {
    // 1.000.00: the last group has two digits, so it is not a thousands separator
    let t = tree("1.000.00");
    assert!(!t.contains("thousands: Some"), "{t}");
}

#[test]
fn missing_arguments() {
    recovers(r"{\frac}", "E-REC-7");
    recovers(r"\frac\\ b", "E-REC-7");
}

// ---------- scripts ----------

#[test]
fn brace_with_label_on_the_other_side() {
    // \overbrace takes its label in ^; a _ becomes an ordinary subscript
    has(&tree(r"\overbrace{x}_{a}"), &["Scripts", "Brace"]);
    has(&tree(r"\underbrace{x}^{a}"), &["Scripts", "Brace"]);
    has(&tree(r"\underbrace{x}_{a}"), &["label: Some"]);
}

#[test]
fn scripts_continuing_after_empty_command() {
    // \nolimits leaves no node: the following ^ attaches to the previous script
    has(&tree(r"\sum_{i}\nolimits^{n}"), &["sup: Some", "sub: Some"]);
    // a prime after a superscript: it comes before it
    has(&tree(r"x^a\relax'"), &["'′'", "\"a\""]);
    has(&tree(r"f'^{2}"), &["'′'", "\"2\""]);
    has(&tree("f''''"), &["'‴'"]);
}

// ---------- simple commands ----------

#[test]
fn tex_primitives() {
    has(&tree(r"\bgroup a \egroup"), &["\"a\""]);
    has(&tree(r"x\sp 2 \sb 3"), &["sup: Some", "sub: Some"]);
    has(&tree(r"a \hfill b \hss c"), &["Space { em: 1.0 }"]);
    has(&tree(r"a \vrule b"), &["Rule"]);
}

#[test]
fn starred_commands() {
    has(&tree(r"\operatorname*{argmax}_x"), &["argmax", "limits: true"]);
    has(&tree(r"x \tag*{A}"), &["\"A\""]);
    has(&tree(r"x \tag{1}"), &["\"(1)\""]);
    has(&tree(r"\smash*{x}"), &["Smashed"]);
}

#[test]
fn measured_spaces() {
    for tex in [
        r"a\hspace{1em}b",
        r"a\hspace*{2pt}b",
        r"a\kern 3pt b",
        r"a\kern-1em b",
        r"a\mkern 2mu b",
        r"a\hskip 1.5ex b",
        r"a\mskip3mu b",
        r"a\kern 1,5cm b",
        r"a\kern2in b",
        r"a\kern3mm b",
        r"a\mspace{4mu}b",
        r"a\hspace{2px}b",
    ] {
        has(&tree(tex), &["Space"]);
    }
    assert!(always_error(r"a\hspace{3qq}b").contains("unrecognized measure"));
    // a space before the unit ends the measure
    assert!(always_error(r"a\kern 3 pt b").contains("unrecognized measure"));
}

#[test]
fn styles_until_end_of_group() {
    for (tex, level) in [(r"\displaystyle x", 0), (r"\textstyle x", 1), (r"\scriptstyle x", 2), (r"\scriptscriptstyle x", 3)] {
        has(&tree(tex), &[&format!("Style {{ level: {level}")]);
    }
    has(&tree(r"{\color{red} x} y"), &["Colored", "red"]);
}

#[test]
fn plain_tex_fractions() {
    has(&tree(r"a \over b"), &["Fraction", "rule: true"]);
    has(&tree(r"{n \choose k}"), &["Some('(')", "rule: false"]);
    has(&tree(r"a \atop b"), &["rule: false"]);
    has(&tree(r"a \brack b"), &["Some('[')"]);
    has(&tree(r"a \brace b"), &["Some('{')"]);
}

#[test]
fn font_switches() {
    for (tex, variant) in [
        (r"\bf x", "Bold"),
        (r"\boldmath x", "Bold"),
        (r"\rm x", "Upright"),
        (r"\normalfont x", "Upright"),
        (r"\sf x", "SansSerif"),
        (r"\tt x", "Monospace"),
        (r"\cal X", "Calligraphic"),
        (r"\bf a \it x", "Italic"),
        (r"\bf a \mit x", "Italic"),
        (r"\bf a \unboldmath x", "Italic"),
    ] {
        let t = tree(tex);
        let last_row = t.rsplit("variant: ").next().unwrap();
        assert!(last_row.starts_with(variant), "{tex}: {t}");
    }
    has(&tree(r"\large x \Huge y"), &["\"y\""]);
}

#[test]
fn explicit_limits() {
    has(&tree(r"\sum\limits_{i}"), &["limits: Some(true)"]);
    has(&tree(r"\sum\nolimits_{i}"), &["limits: Some(false)"]);
    has(&tree(r"\sum\displaylimits_{i}"), &["limits: None"]);
    has(&tree(r"\lim\nolimits_{x}"), &["limits: false"]);
    has(&tree(r"\sum_{i}\limits"), &["limits: Some(true)"]);
    has(&tree(r"\mathop{\sum}\limits_{i}"), &["limits: Some(true)"]);
    // \limits with no operator before it: nothing to change
    has(&tree(r"\limits x"), &["\"x\""]);
    has(&tree(r"x\limits"), &["\"x\""]);
}

#[test]
fn labels_and_references() {
    has(&tree(r"x \label{eq:1}"), &["\"x\""]);
    has(&tree(r"\ref{eq:1}"), &["\"eq:1\""]);
    has(&tree(r"\eqref{eq:1}"), &["\"(eq:1)\""]);
    has(&tree(r"\strut x \mathstrut"), &["Phantom"]);
}

#[test]
fn fractions_roots_and_boxes() {
    has(&tree(r"\cfrac[l]{1}{2}"), &["Display"]);
    has(&tree(r"\fbox{x}"), &["Framed", "Rectangle"]);
    has(&tree(r"\framebox{y}"), &["Framed"]);
}

#[test]
fn math_alphabets() {
    for (cmd, variant) in [
        ("mathrm", "Upright"),
        ("mathup", "Upright"),
        ("symrm", "Upright"),
        ("mathit", "Italic"),
        ("mathnormal", "Italic"),
        ("symit", "Italic"),
        ("mathbf", "Bold"),
        ("symbf", "Bold"),
        ("boldsymbol", "BoldItalic"),
        ("bm", "BoldItalic"),
        ("pmb", "BoldItalic"),
        ("mathbfit", "BoldItalic"),
        ("mathsf", "SansSerif"),
        ("mathtt", "Monospace"),
        ("mathbb", "DoubleStruck"),
        ("mathds", "DoubleStruck"),
        ("mathbbm", "DoubleStruck"),
        ("Bbb", "DoubleStruck"),
        ("mathcal", "Calligraphic"),
        ("mathscr", "Calligraphic"),
        ("mathfrak", "Fraktur"),
    ] {
        has(&tree(&format!(r"\{cmd}{{R}}")), &[&format!("variant: {variant}")]);
    }
}

#[test]
fn forced_classes() {
    for (cmd, class) in
        [("mathop", "Op"), ("mathrel", "Rel"), ("mathbin", "Bin"), ("mathopen", "Open"), ("mathclose", "Close"), ("mathpunct", "Punct"), ("mathinner", "Inner"), ("mathord", "Ord")]
    {
        has(&tree(&format!(r"\{cmd}{{x}}")), &[&format!("ForcedClass {{ class: {class}")]);
    }
}

#[test]
fn stacked_and_extensible_arrows() {
    has(&tree(r"\overset{a}{=}"), &["class: Rel", "Stacked"]);
    has(&tree(r"\stackrel{def}{=}"), &["class: Rel"]);
    has(&tree(r"\underset{n}{\lim}"), &["class: Op"]);
    has(&tree(r"\overset{a}{x}"), &["class: Ord"]);
    has(&tree(r"\overset{a}="), &["class: Rel"]);
    for (cmd, arrow) in [
        ("xrightarrow", '→'),
        ("xleftarrow", '←'),
        ("xRightarrow", '⇒'),
        ("xLeftarrow", '⇐'),
        ("xleftrightarrow", '↔'),
        ("xLeftrightarrow", '⇔'),
        ("xrightleftharpoons", '⇌'),
        ("xleftrightharpoons", '⇋'),
        ("xmapsto", '↦'),
        ("xhookrightarrow", '↪'),
        ("xlongequal", '='),
    ] {
        has(&tree(&format!(r"\{cmd}[b]{{a}}")), &[&format!("symbol: '{arrow}'"), "below: Some"]);
    }
    has(&tree(r"\xrightarrow{}"), &["above: None"]);
}

#[test]
fn cancels_frames_and_phantoms() {
    has(&tree(r"\cancelto{0}{x}"), &["Canceled", "sup: Some"]);
    has(&tree(r"\bcancel{x} \xcancel{y} \sout{z}"), &["Backslash", "X }"]);
    has(&tree(r"\circled{1}"), &["Circle"]);
    // amssymb's \circledcirc is the operator ⊚, not a frame
    has(&tree(r"a \circledcirc b"), &["'⊚'", "Bin"]);
    has(&tree(r"\phantom{x}"), &["width: true, height: true"]);
    has(&tree(r"\hphantom{x}"), &["width: true, height: false"]);
    has(&tree(r"\vphantom{x}"), &["width: false, height: true"]);
    has(&tree(r"\textcolor{blue}{x}"), &["blue"]);
    has(&tree(r"\colorbox{yellow}{nota}"), &["yellow", "nota"]);
}

#[test]
fn not_negates_or_strikes() {
    for (tex, negated) in [
        ("=", '≠'),
        (r"\in", '∉'),
        (r"\subset", '⊄'),
        (r"\supset", '⊅'),
        (r"\subseteq", '⊈'),
        (r"\supseteq", '⊉'),
        (r"\equiv", '≢'),
        ("<", '≮'),
        (">", '≯'),
        (r"\leq", '≰'),
        (r"\geq", '≱'),
        (r"\sim", '≁'),
        (r"\approx", '≉'),
        (r"\exists", '∄'),
        (r"\mid", '∤'),
        (r"\parallel", '∦'),
        (r"\to", '↛'),
    ] {
        has(&tree(&format!(r"a \not{tex} b")), &[&format!("'{negated}'")]);
    }
    has(&tree(r"\not+"), &["Canceled", "'+'"]);
    has(&tree(r"\not x"), &["Canceled", "\"x\""]);
    assert!(always_error(r"\not").contains("with no symbol"));
    assert!(always_error(r"\not\\").contains("\\not before"));
}

#[test]
fn fixed_size_delimiters() {
    for (tex, class, size) in [(r"\bigl(", "Open", 1), (r"\Bigr)", "Close", 2), (r"\biggm|", "Rel", 3), (r"\Bigg\{", "Ord", 4), (r"\big(", "Open", 1), (r"\big|", "Ord", 1)] {
        has(&tree(tex), &[&format!("class: {class}, size: {size}")]);
    }
    recovers(r"x \big", "E-REC-7");
}

// ---------- units ----------

#[test]
fn siunitx_units() {
    has(&tree(r"\si{\meter}"), &["\"m\""]);
    has(&tree(r"\unit{\kilo\gram}"), &["\"kg\""]);
    has(&tree(r"\SI{3}{\meter\tothe{3}}"), &["value: \"3\""]);
    has(&tree(r"\si{\per\second\raiseto{2}}"), &["\"1\"", "\"s\"", "\"2\""]);
    // the exponent applies to the prefixed unit: km²
    has(&tree(r"\si{\square\kilo\meter}"), &["\"km\"", "\"2\""]);
    has(&tree(r"\si{\kilo\cubic\meter}"), &["\"km\"", "\"3\""]);
    has(&tree(r"\si{\newton\meter}"), &["\"N\"", "\"m\"", "Space"]);
    has(&tree(r"\si{\meter\per\second\per\kilogram}"), &["'('", "'·'"]);
    has(&tree(r"\si{m/s}"), &["\"m\""]);
    // \squared with no unit before it: there is nothing to raise
    has(&tree(r"\si{\squared\per\cubed}"), &["Quantity"]);
    has(&tree(r"\SI{-5}{\meter}"), &["value: None", "'−'"]);
    // an unknown unit is written by its name, a value that is not a number as written
    assert!(strict_error(r"\si{\foo}").contains("unsupported unit"));
    has(&recovers(r"\si{\foo}", "E-REC-1"), &["\"foo\""]);
    assert!(strict_error(r"\SI{abc}{\meter}").contains("is not a number"));
    has(&recovers(r"\SI{abc}{\meter}", "E-REC-9"), &["\"m\""]);
    has(&tree(r"\unit[3]{m}"), &["\"3\"", "\"m\""]);
}

#[test]
fn every_unit_has_a_symbol() {
    for name in [
        "watt",
        "pascal",
        "hertz",
        "coulomb",
        "volt",
        "ohm",
        "farad",
        "tesla",
        "weber",
        "henry",
        "siemens",
        "liter",
        "litre",
        "hour",
        "minute",
        "day",
        "degreeCelsius",
        "celsius",
        "degree",
        "electronvolt",
        "calorie",
        "bar",
        "atm",
        "atmosphere",
        "percent",
        "mmHg",
        "hectare",
        "tonne",
    ] {
        assert!(symbols::unit(name).is_some(), "{name}");
        has(&tree(&format!(r"\si{{\{name}}}")), &["Quantity"]);
    }
    assert!(symbols::unit("parsec").is_none());
}

#[test]
fn siunitx_numbers() {
    has(&tree(r"\num{1e5}"), &["'×'", "\"10\"", "\"5\""]);
    has(&tree(r"\num{-3}"), &["'−'"]);
    has(&tree(r"\num{+3}"), &["'+'"]);
    has(&tree(r"\num{1,5e-3}"), &["\"1.5\"", "'−'"]);
    has(&tree(r"\num{2E+4}"), &["\"4\""]);
    // not a number: strict mode refuses, tolerant mode draws what was written (E-REC-9)
    for tex in [r"\num{1.2.3}", r"\num{3e}", r"\num{3e2x}", r"\ang{x}"] {
        assert!(strict_error(tex).contains("is not a number"), "{tex}");
        recovers(tex, "E-REC-9");
    }
    has(&tree(r"\ang{30}"), &["'°'"]);
}

// ---------- arithmetic and other commands ----------

#[test]
fn column_arithmetic_and_unicode() {
    for op in ["+", "-", "x", r"\times", "×", "*"] {
        has(&tree(&format!(r"\contaarmada{{{op}}}{{12}}{{3}}")), &["ColumnArithmetic"]);
    }
    // invalid ones: strict mode refuses; tolerant mode writes the calculation inline
    has(&recovers(r"\contaarmada{/}{1}{2}", "E-REC-9"), &["'/'", "\"1\"", "\"2\""]);
    has(&recovers(r"\contaarmada{+}{a}{2}", "E-REC-9"), &["'+'", "\"a\""]);
    has(&recovers(r"\divchave{10}{0}", "E-REC-9"), &["'÷'", "\"10\"", "\"0\""]);
    has(&recovers(&format!(r"\divchave{{{}}}{{7}}", "9".repeat(31)), "E-REC-9"), &["'÷'"]);
    assert!(strict_error(r"\divchave{10}{0}").contains("nonzero divisor"));
    has(&tree(r"\unicode{x221E}"), &["'∞'"]);
    has(&tree(r"\unicode{U+00B2}"), &["'²'"]);
    assert!(always_error(r"\unicode{zz}").contains("invalid code point"));
    has(&tree(r"\LaTeX \TeX"), &["\"LaTeX\"", "\"TeX\""]);
}

#[test]
fn genfrac_in_all_forms() {
    has(&tree(r"\genfrac{}{}{0pt}{1}{a}{b}"), &["style: Row", "rule: false"]);
    has(&tree(r"\genfrac{}{}{}{0}{a}{b}"), &["style: Display", "rule: true"]);
    has(&tree(r"\genfrac{\langle}{\rangle}{1pt}{}{a}{b}"), &["Some('⟨')", "Some('⟩')", "style: Auto"]);
    has(&tree(r"\genfrac(]{}{}{a}{b}"), &["Some('(')", "Some(']')"]);
}

#[test]
fn boxes_and_rules() {
    has(&tree(r"\mathllap{x} \mathrlap{y} \mathclap{z}"), &["side: 'l'", "side: 'r'", "side: 'c'"]);
    has(&tree(r"\llap{a} \rlap{b} \clap{c}"), &["Overlapped"]);
    has(&tree(r"\raisebox{2pt}[1pt][1pt]{a}"), &["Raised"]);
    assert!(always_error(r"\raisebox{x}{a}").contains("unrecognized measure"));
    has(&tree(r"\scalebox{2}[1]{a}"), &["Scaled"]);
    assert!(always_error(r"\scalebox{dobro}{a}").contains("takes a number"));
    // L-BOX-1: graphicx's lengths, factors and angles
    has(&tree(r"\raisebox{0pt}[\height][0pt]{a}"), &["height: Some(BoxLength { em: 0.0, width: 0.0, height: 1.0", "depth: Some(BoxLength { em: 0.0,"]);
    has(&tree(r"\raisebox{0pt}[-.5\totalheight]{a}"), &["height: -0.5, depth: -0.5"]);
    has(&recovers(r"\raisebox{0pt}[alto]{a}", "E-REC-9"), &["height: None"]);
    has(&tree(r"\scalebox{-1}[1]{a}"), &["factor: -1.0, vertical: Some(1.0)"]);
    has(&tree(r"\scalebox{2}[2]{a}"), &["factor: 2.0, vertical: None"]);
    assert!(always_error(r"\scalebox{1}[x]{a}").contains("takes numbers"));
    assert!(always_error(r"\scalebox{inf}{a}").contains("takes a number"));
    has(&tree(r"\resizebox{1cm}{!}{a}"), &["Resized", "height: None", "total: false"]);
    has(&tree(r"\resizebox*{!}{\totalheight}{a}"), &["width: None", "total: true"]);
    assert!(always_error(r"\resizebox{largo}{!}{a}").contains("unrecognized measure"));
    has(&tree(r"\rotatebox{90}{a}"), &["Rotated { degrees: 90.0, origin: \"\""]);
    has(&tree(r"\rotatebox[origin=br, units=360]{-30}{a}"), &["degrees: -30.0, origin: \"br\""]);
    has(&tree(r"\rotatebox[origin]{30}{a}"), &["origin: \"c\""]);
    assert!(always_error(r"\rotatebox{noventa}{a}").contains("takes a number"));
    has(&tree(r"\rule[1pt]{2em}{1ex}"), &["Rule"]);
    assert!(always_error(r"\rule{a}{b}").contains("unrecognized measures"));
    has(&tree(r"\smash[t]{x} \smash[b]{y}"), &["height: true, depth: false", "height: false, depth: true"]);
}

#[test]
fn sideset_and_text_scripts() {
    has(&tree(r"\sideset{_a}{^b}\sum_{i}^{n}"), &["LargeOperator", "\"a\"", "\"b\""]);
    has(&tree(r"\sideset{}{'}\sum"), &["LargeOperator"]);
    has(&tree(r"\textsuperscript{a}"), &["sup: Some"]);
    has(&tree(r"\textsubscript{x}"), &["sub: Some"]);
}

#[test]
fn verbatim() {
    has(&tree(r"\verb|a b-c|"), &["Monospace", "Space { em: 0.525 }", "'-'"]);
    has(&tree(r"\verb*+x+"), &["Monospace"]);
    assert!(always_error(r"\verb|abc").contains("without the closing '|'"));
    assert!(always_error(r"\verb").contains("without a delimiter"));
}

#[test]
fn multiple_columns_and_dots() {
    has(&tree(r"\multicolumn{2}{|r|}{x}"), &["columns: 2", "alignment: 'r'"]);
    assert!(always_error(r"\multicolumn{n}{c}{x}").contains("takes a number"));
    has(&tree(r"\hdotsfor{3}"), &["columns: 3"]);
    has(&tree(r"\dotsfor[2]{x}"), &["columns: 1"]);
}

#[test]
fn modular_arithmetic_and_misc() {
    has(&tree(r"a \bmod b"), &["class: Bin", "mod"]);
    has(&tree(r"a \pmod{3}"), &["'('", "mod"]);
    has(&tree(r"a \mod{4}"), &["mod"]);
    has(&tree(r"a \pod{2}"), &["'('"]);
    has(&tree(r"\imath \jmath"), &["ı", "ȷ"]);
    has(&tree(r"a \lt b \gt c"), &["'<'", "'>'"]);
}

// ---------- colors and delimiters ----------

#[test]
fn colored_boxes_paint_the_background_not_the_text() {
    let t = tree(r"\colorbox{yellow}{x}");
    has(&t, &["Framed", "shape: Fill", "background: Some(\"yellow\")", "border: None"]);
    assert!(!t.contains("Colored"), "{t}");
    has(&tree(r"\fcolorbox{red}{yellow}{x}"), &["shape: Rectangle", "background: Some(\"yellow\")", "border: Some(\"red\")"]);
    has(&tree(r"\fcolorbox[HTML]{FF0000}{yellow}{x}"), &["border: Some(\"#FF0000\")"]);
    // an invalid color in tolerant mode: the box stays, without that color
    has(&recovers(r"\fcolorbox{re;d}{yellow}{x}", "E-REC-9"), &["border: None", "background: Some(\"yellow\")"]);
}

#[test]
fn square_brackets_and_arrows_under_and_over() {
    has(&tree(r"\overbracket{abc}"), &["Brace", "above: true", "bracket: true"]);
    has(&tree(r"\underbracket{abc}_{n}"), &["above: false", "label: Some(Identifier { name: \"n\"", "bracket: true"]);
    // mathtools' optional thickness and height are read and ignored
    has(&tree(r"\overbracket[1pt][2pt]{abc}"), &["bracket: true"]);
    for (name, arrow) in [("underrightarrow", '→'), ("underleftarrow", '←'), ("underleftrightarrow", '↔')] {
        has(&tree(&format!(r"\{name}{{AB}}")), &[&format!("accent: '{arrow}', stretchy: true, below: true")]);
    }
    has(&tree(r"\arc{AB}"), &["accent: '⏜', stretchy: true, below: false"]);
}

#[test]
fn color_models() {
    has(&tree(r"\color[HTML]{FF0000} x"), &["#FF0000"]);
    has(&tree(r"\textcolor[rgb]{1,0,0}{x}"), &["rgb(1,0,0)"]);
    // invalid color: strict mode refuses; tolerant mode draws without color
    for tex in [r"\color{re;d} x", r#"\textcolor{red" onload="x}{a}"#, r"\colorbox{}{a}", r"\color[HTML]{<script>} x"] {
        let t = recovers(tex, "E-REC-9");
        assert!(!t.contains("Colored"), "{tex}: {t}");
    }
}

#[test]
fn invalid_delimiters() {
    recovers(r"\left\foo x \right)", "E-REC-9");
    has(&recovers(r"\left a x \right)", "E-REC-9"), &["Some('a')"]);
    recovers(r"\left", "E-REC-7");
    has(&recovers(r"\left( a }", "E-REC-5"), &["close: None"]);
    recovers(r"\left( a", "E-REC-5");
    has(&tree(r"\left( a \middle| b \middle. c \right)"), &["middles: ['|', '|']"]);
}

#[test]
fn all_delimiters_by_name() {
    for (name, expected) in [
        ("|", '‖'),
        ("Vert", '‖'),
        ("lVert", '‖'),
        ("rVert", '‖'),
        ("lbrack", '['),
        ("rbrack", ']'),
        ("llbracket", '⟦'),
        ("lBrack", '⟦'),
        ("rrbracket", '⟧'),
        ("rBrack", '⟧'),
        ("uparrow", '↑'),
        ("downarrow", '↓'),
        ("updownarrow", '↕'),
        ("backslash", '\\'),
    ] {
        assert_eq!(symbols::delimiter_command(name), Some(Some(expected)), "{name}");
    }
    assert_eq!(symbols::delimiter_command("foo"), None);
    assert_eq!(symbols::delimiter_char('<'), Some(Some('⟨')));
    assert_eq!(symbols::delimiter_char('>'), Some(Some('⟩')));
    assert_eq!(symbols::delimiter_char('a'), None);
}

// ---------- environments ----------

#[test]
fn single_line_environments() {
    has(&tree(r"\begin{subequations} a \end{subequations}"), &["\"a\""]);
    for env in ["equation", "equation*", "displaymath", "math"] {
        has(&tree(&format!(r"\begin{{{env}}} a \end{{{env}}}")), &["\"a\""]);
        assert!(always_error(&format!(r"\begin{{{env}}} a")).contains("without \\end"));
    }
    assert!(always_error(r"\begin{subequations} a").contains("without \\end"));
}

#[test]
fn environments_with_options() {
    has(&tree(r"\begin{alignat}{2} a &= b \end{alignat}"), &["Aligned"]);
    has(&tree(r"\begin{pmatrix*}[r] 1 & -2 \\ 3 & 4 \end{pmatrix*}"), &["columns: ['r', 'r']"]);
    has(&tree(r"\begin{bmatrix*} 1 \end{bmatrix*}"), &["BMatrix"]);
    has(&tree(r"\begin{subarray}{l} a \\ b \end{subarray}"), &["Substack"]);
}

#[test]
fn badly_closed_table_body() {
    has(&recovers(r"\begin{matrix} a & b \end{pmatrix}", "E-REC-5"), &["\"b\""]);
    has(&recovers(r"\begin{matrix} a & b", "E-REC-5"), &["\"b\""]);
    recovers(r"\begin{matrix} a } b \end{matrix}", "E-REC-4");
    recovers(r"\begin{matrix} a \middle| b \end{matrix}", "E-REC-5");
    recovers(r"\begin{foo} a \end{foo}", "E-REC-6");
    has(&tree(r"\begin{matrix} a \\ \intertext{ou} b \end{matrix}"), &["intertexts: [1]"]);
}

#[test]
fn argument_tables() {
    for cmd in ["substack", "cases", "matrix", "pmatrix", "bordermatrix", "eqalign", "eqalignno", "displaylines"] {
        has(&tree(&format!(r"\{cmd}{{a & b \cr c & d}}")), &["Table"]);
    }
}

#[test]
fn array_column_specs() {
    has(&tree(r"\begin{array}{p{2cm} m{1cm} b X c} a \end{array}"), &["columns: ['l', 'l', 'l', 'l', 'c']"]);
    has(&tree(r"\begin{array}{c@{\hspace{1em}}c!{,}c} a \end{array}"), &["between: [(1,", "(2,"]);
    has(&tree(r"\begin{array}{*{3}{c}|r} a \end{array}"), &["columns: ['c', 'c', 'c', 'r']", "verticals: [3]"]);
    has(&tree(r"\begin{tabular}[t]{l c} a \end{tabular}"), &["Array"]);
    recovers(r"\begin{array}{cq} a \end{array}", "E-REC-9");
}

#[test]
fn rcases_puts_the_brace_on_the_right() {
    for env in ["rcases", "drcases"] {
        has(&tree(&format!(r"\begin{{{env}}} a & x > 0 \\ b & x \le 0 \end{{{env}}}")), &["shape: RCases"]);
    }
    let r = convert(r"\begin{rcases} a \\ b \end{rcases} \Rightarrow c", &Options::default()).unwrap();
    assert_eq!(r.text, "a; b} ⇒ c");
    assert!(r.speech.starts_with("cases: case 1: a; case 2: b"), "{}", r.speech);
}

#[test]
fn commutative_diagram() {
    let t = tree(r"\begin{CD} A @>{f_1}>> B \\ @VgVV @AAhA \\ C @<<k< D \end{CD}");
    has(&t, &["Diagram", "'→'", "'↓'", "'↑'", "'←'"]);
    has(&tree(r"\begin{CD} A @= B \\ @| @. \end{CD}"), &["'='", "'‖'"]);
    // a label without its closing: it runs to the end of the row
    has(&tree(r"\begin{CD} A @>f \end{CD}"), &["'→'"]);
    assert!(always_error(r"\begin{CD} A @x B \end{CD}").contains("unknown diagram arrow"));
    has(&recovers(r"\begin{CD} A @>>> B", "E-REC-5"), &["Diagram"]);
}

#[test]
fn arrow_diagrams_recover() {
    has(&recovers(r"\begin{tikzcd} A \arrow[r, wobbly] & B \end{tikzcd}", "E-REC-1"), &["ArrowDiagram"]);
    has(&recovers(r#"\begin{tikzcd} A \arrow["f"] & B \end{tikzcd}"#, "E-REC-7"), &["arrows: []"]);
    has(&recovers(r"\begin{tikzcd} A \arrow[rr] & B \end{tikzcd}", "E-REC-7"), &["arrows: []"]);
    has(&recovers(r"\begin{tikzcd} A \arrow[u] \end{tikzcd}", "E-REC-7"), &["arrows: []"]);
    has(&recovers(r#"\begin{tikzcd} A \arrow[r, "f] & B \end{tikzcd}"#, "E-REC-4"), &["ArrowDiagram"]);
    has(&recovers(r"\begin{tikzcd} A \arrow[to=x] & B \end{tikzcd}", "E-REC-1"), &["ArrowDiagram"]);
    has(&recovers(r"\begin{tikzcd} A \arrow[r] & B", "E-REC-5"), &["ArrowDiagram"]);
    has(&recovers(r"\xymatrix{ A \ar & B }", "E-REC-7"), &["arrows: []"]);
    has(&recovers(r"\xymatrix{ A \ar[x] & B }", "E-REC-1"), &["arrows: []"]);
    has(&recovers(r#"\xymatrix{ A \ar"0,1" & B }"#, "E-REC-1"), &["arrows: []"]);
    has(&recovers(r"\xymatrix{ A \ar[l] & B }", "E-REC-7"), &["arrows: []"]);
    has(&recovers(r#"\xymatrix{ A \ar"1,1" & B }"#, "E-REC-7"), &["arrows: []"]);
    // a cell can hold \arccos and other commands that start like \ar
    has(&tree(r"\begin{tikzcd} \arccos x \arrow[r] & \arg z \end{tikzcd}"), &["arccos", "arg"]);
    has(&tree(r"\xymatrix{ \arccos x \ar[r] & \{ y \} }"), &["arccos"]);
}

// ---------- text ----------

#[test]
fn small_capitals() {
    has(&tree(r"\textsc{Smith}"), &["Text(\"\\u{e002}Smith\\u{e003}\")"]);
    // inside \text, \textsc closes at its brace and \scshape at the end of the group
    has(&tree(r"\text{a \textsc{bc} d}"), &["\"a \\u{e002}bc\\u{e003} d\""]);
    has(&tree(r"\text{a {\scshape bc} d}"), &["\"a \\u{e002}bc\\u{e003} d\""]);
    // the markers are not spoken nor written in the linear text
    let r = convert(r"\textsc{Smith} + \text{\textsc{Lee}}", &Options::default()).unwrap();
    assert_eq!(r.speech, "Smith plus Lee");
    assert_eq!(r.text, "Smith + Lee");
}

#[test]
fn math_inside_text() {
    has(&tree(r"\text{a $x^2$ b}"), &["Math(", "\"a \""]);
    has(&tree(r"\text{$x$}"), &["Math("]);
    has(&tree(r"\text{a \(x\) b \[y\]}"), &["Math("]);
    has(&recovers(r"\text{a $ b}", "E-REC-9"), &["$"]);
    has(&recovers(r"\text{a \( b}", "E-REC-9"), &["("]);
}

#[test]
fn bold_and_groups_in_text() {
    has(&tree(r"\text{a {b} c}"), &["\"a b c\""]);
    has(&tree(r"\text{\textbf{neg} x}"), &["\\u{e000}neg\\u{e001} x"]);
    has(&tree(r"\text{{\bf a} b}"), &["\\u{e000}a\\u{e001} b"]);
    // \bfseries until the end of the text
    has(&tree(r"\text{\bfseries abc}"), &["\\u{e000}abc\\u{e001}"]);
    has(&tree(r"\text{} \text{\}}"), &["\"}\""]);
}

#[test]
fn text_accents() {
    has(&tree(r#"\text{\'e \`a \^o \~n \"u \=a \.z}"#), &["é à ô ñ ü ā ż"]);
    has(&tree(r"\text{\c{c} \u{a} \v{s} \c c}"), &["ç ă š ç"]);
    // an accent with no known composition: the letter stays
    has(&tree(r#"\text{\"q}"#), &["q\\u{308}"]);
    has(&tree(r"\text{\'{\i}}"), &["Text"]);
}

#[test]
fn text_commands() {
    has(&tree(r"\text{\%\$\&\#\_\{\}}"), &["%$&#_{}"]);
    has(&tree(r"\text{a\|b}"), &["a‖b"]);
    has(&tree(r"\text{a\,b\;c\:d\ e\!f\\g}"), &["a b c d e\\u{200b}f g"]);
    for (cmd, output) in [
        ("quad", "\\u{2003}"),
        ("qquad", "\\u{2003}\\u{2003}"),
        ("ldots", "…"),
        ("dots", "…"),
        ("textdegree", "°"),
        ("degree", "°"),
        ("o", "ø"),
        ("ss", "ß"),
        ("aa", "å"),
        ("AA", "Å"),
        ("ae", "æ"),
        ("textendash", "–"),
        ("textemdash", "—"),
        ("textasciitilde", "~"),
        ("textbackslash", "\\\\"),
        ("times", "×"),
        ("LaTeX", "LaTeX"),
        ("TeX", "TeX"),
        ("alpha", "α"),
        ("infty", "∞"),
        ("textit", "x"),
    ] {
        has(&tree(&format!(r"\text{{\{cmd} x}}")), &[output]);
    }
    has(&recovers(r"\text{a \@ b}", "E-REC-1"), &["@"]);
    has(&recovers(r"\text{a \foo b}", "E-REC-1"), &["\\\\foo"]);
}

#[test]
fn l_txt_2_scripts_and_math_inside_text() {
    // H₂O: the script of the text before it, as \textsubscript outside text
    let t = tree(r"\text{H\textsubscript{2}O}");
    has(&t, &["Text(\"H\")", "Math(Scripts", "sub: Some(Text", "Text(\"O\")"]);
    has(&tree(r"\text{x\textsuperscript{th}}"), &["sup: Some(Text { parts: [Text(\"th\")]"]);
    has(&tree(r"\text{n\textsuperscript 2}"), &["sup: Some"]);
    // \ensuremath: math, whether in text or not
    has(&tree(r"\text{a \ensuremath{x^2} b}"), &["Math(Scripts", "Italic"]);
    assert_eq!(tree(r"\ensuremath{x}"), tree("x"));
}

#[test]
fn l_txt_2_boldmath_until_the_end_of_its_group() {
    let t = tree(r"\text{{\boldmath $x$} $y$}");
    assert!(t.find("Bold").unwrap() < t.find("Italic").unwrap(), "{t}");
    has(&tree(r"\text{\boldmath \ensuremath{x}}"), &["Bold"]);
    assert!(!tree(r"\text{\boldmath\unboldmath $x$}").contains("Bold"));
    // CircuiTikZ's op amp font: the size is the page's, the math bold
    has(&tree(r"\text{\fontsize{10}{12}\selectfont\boldmath $V$}"), &["Bold"]);
    assert!(!tree(r"\text{\fontsize{10}{12}\selectfont a}").contains("fontsize"));
}

#[test]
fn l_txt_2_math_commands_in_text_read_as_math() {
    has(&recovers(r"\text{\mathrm{V}}", "E-REC-1"), &["Math(Identifier", "Upright"]);
    has(&recovers(r"\text{a \frac{1}{2} b}", "E-REC-1"), &["Math(Fraction"]);
    has(&recovers(r"\text{\sqrt[3]{2}}", "E-REC-1"), &["Math(Root", "index: Some"]);
    has(&recovers(r"\text{\operatorname*{argmax}}", "E-REC-1"), &["argmax"]);
    has(&recovers(r"\text{\mathbb R}", "E-REC-1"), &["DoubleStruck"]);
}

#[test]
fn l_txt_3_space_inside_text() {
    // \hspace: 1cm is 2.845em of 10pt, between the two letters
    let t = tree(r"\text{a\hspace{1cm}b}");
    has(&t, &["Text(\"a\")", "Math(Space { em: 2.845", "Text(\"b\")"]);
    assert!(t.find("Text(\"a\")").unwrap() < t.find("Space").unwrap() && t.find("Space").unwrap() < t.find("Text(\"b\")").unwrap(), "{t}");
    has(&tree(r"\text{M \hspace*{5pt}.}"), &["Math(Space { em: 0.5 }"]);
    // \phantom: the room of its argument, read as text
    has(&tree(r"\text{a\phantom{xy}b}"), &["Math(Phantom", "Text { parts: [Text(\"xy\")]", "width: true, height: true"]);
    has(&tree(r"\text{\hphantom{x}}"), &["Math(Phantom", "width: true, height: false"]);
    has(&tree(r"\text{\vphantom{x}}"), &["Math(Phantom", "width: false, height: true"]);
    has(&tree(r"\text{\phantom x}"), &["Math(Phantom", "Text(\"x\")"]);
}

#[test]
fn l_txt_3_fills_and_line_breaks_inside_text() {
    // the box has its natural width: the fill takes no room, the spaces around it stay one
    has(&tree(r"\text{1 \hfill IA}"), &["Text(\"1 IA\")"]);
    has(&tree(r"\text{a\hfil b\hss}"), &["Text(\"ab\")"]);
    // a line break is a space, as \\ is
    has(&tree(r"\text{a\newline b}"), &["Text(\"a b\")"]);
    has(&tree(r"\text{a\linebreak[3]b\linebreak c}"), &["Text(\"a b c\")"]);
}

#[test]
fn l_txt_4_color_inside_text() {
    // \color: the rest of its group, math included; the text after the group keeps the page color
    let t = tree(r"\text{a {\color{red}b $x$} c}");
    has(&t, &["Text(\"a \")", "Math(Colored { color: \"red\", base: Text { parts: [Text(\"b \"), Math(Identifier", "Text(\" c\")"]);
    assert!(t.find("Colored").unwrap() < t.find("Text(\" c\")").unwrap(), "{t}");
    // to the end of the text, with a color model
    has(&tree(r"\text{a \color[rgb]{0,0,1}b}"), &["Colored { color: \"rgb(0,0,1)\"", "Text(\"b\")"]);
    // \textcolor: its argument
    has(&tree(r"\text{\textcolor{blue}{x}y}"), &["Math(Colored { color: \"blue\", base: Text { parts: [Text(\"x\")]", "Text(\"y\")"]);
    // the colored text stays bold, and its math bold after \boldmath
    let bold = format!("{BOLD_START}b{BOLD_END}");
    has(&tree(r"\text{\textbf{a \color{red}b}}"), &["Colored", &format!("Text({bold:?})")]);
    has(&tree(r"\text{\boldmath\textcolor{red}{$x$}}"), &["Colored", "Bold"]);
    has(
        &tree(r"\text{\textsc{\color{red}a} \texttt{\color{red}b}}"),
        &[&format!("Text({:?})", format!("{SMALL_CAPS_START}a{SMALL_CAPS_END}")), &format!("Text({:?})", format!("{MONO_START}b{MONO_END}"))],
    );
    // the group ends at its own brace, past escaped braces and the groups in it
    has(&tree(r"\text{{\color{red}\{a\} {b}} c}"), &["Text(\"{a} b\")", "Text(\" c\")"]);
}

#[test]
fn l_txt_5_italic_inside_text() {
    // \textit and \emph to their brace, with LaTeX's correction after it; \itshape, \it and \em to the
    // end of their group, without it
    let corrected = format!("{ITALIC_START}b{ITALIC_CORRECTED_END}");
    has(&tree(r"\text{a \textit{b} c}"), &[&format!("Text({:?})", format!("a {corrected} c"))]);
    has(&tree(r"\text{a \emph{b}}"), &[&format!("Text({:?})", format!("a {corrected}"))]);
    for shape in ["itshape", "it", "em"] {
        has(&tree(&format!(r"\text{{a {{\{shape} b}} c}}")), &[&format!("Text({:?})", format!("a {ITALIC_START}b{ITALIC_END} c"))]);
    }
    // in math, \textit is text in italic
    has(&tree(r"\textit{b}"), &[&format!("Text({corrected:?})")]);
    // italic goes on after math in its group, and keeps a color's text italic
    let t = tree(r"\text{\textit{a $x$ b}}");
    has(&t, &[&format!("Text({:?})", format!("{ITALIC_START}a {ITALIC_CORRECTED_END}")), &format!("Text({:?})", format!("{ITALIC_START} b{ITALIC_CORRECTED_END}"))]);
    has(&tree(r"\text{\itshape \color{red}b}"), &["Colored", &format!("Text({:?})", format!("{ITALIC_START}b{ITALIC_END}"))]);
}

#[test]
fn l_txt_2_math_environments_inside_text() {
    // a display: math up to its \end, the text around it kept
    let t = tree(r"\text{so \begin{align*}a&=1\\b&=2\end{align*} done}");
    has(&t, &["Text(\"so \")", "Math(Table { shape: Aligned", "Text(\" done\")"]);
    has(&tree(r"\text{\begin{equation*}\begin{equation*}x\end{equation*}\end{equation*}}"), &["Math(Identifier"]);
    // a math-only environment is an error in TeX's text: read as math, with a warning
    has(&recovers(r"\text{\begin{pmatrix}1\end{pmatrix}}", "E-REC-1"), &["Math(Table { shape: PMatrix"]);
    // without its \end: closed at the end of the text
    has(&recovers(r"\text{\begin{equation}x}", "E-REC-5"), &["Math(Identifier"]);
    // an environment that does not start math is still written as is
    recovers(r"\text{\begin{itemize}\end{itemize}}", "E-REC-1");
    recovers(r"\text{\begin x}", "E-REC-1");
}

#[test]
fn all_text_accents_compose() {
    for (mark, letter, expected) in [('`', 'a', 'à'), ('^', 'o', 'ô'), ('"', 'u', 'ü'), ('=', 'a', 'ā'), ('.', 'z', 'ż'), ('u', 'a', 'ă'), ('v', 's', 'š')] {
        assert_eq!(symbols::text_accent(mark, letter), Some(expected), "{mark}{letter}");
    }
    assert_eq!(symbols::text_accent('?', 'a'), None);
}

// ---------- chemistry ----------

#[test]
fn chemical_formulas() {
    has(&tree(r"\ce{2H2O -> 2H2 + O2}"), &["\"2\"", "'→'", "'+'"]);
    has(&tree(r"\ce{NaCl(aq) <- Na+ + Cl-}"), &["\"Na\"", "\"(aq)\"", "'←'"]);
    has(&tree(r"\ce{SO4^{2-}}"), &["\"2−\""]);
    has(&tree(r"\ce{Fe^3+}"), &["\"3+\""]);
    has(&tree(r"\ce{H2O(l) <=> H2O(g)}"), &["'⇌'", "\"(l)\"", "\"(g)\""]);
    has(&tree(r"\ce{NaCl(s) = A ^ v}"), &["\"(s)\"", "'↑'", "'↓'"]);
    has(&tree(r"\ce{[Cu(NH3)4]}"), &["'['", "'('", "class: Close"]);
    has(&tree(r"\ce{CuSO4.5H2O}"), &["'·'"]);
    // the bond is text: not a minus, not read as "negative"
    has(&tree(r"\ce{A*B H-Cl}"), &["'·'", "Text(\"−\")"]);
    has(&tree(r"\ce{^{2+}}"), &["\"2+\""]);
    has(&tree(r"\pu{3 mol}"), &["\"3\""]);
    has(&recovers(r"\ce{H☃}", "E-REC-9"), &["'☃'"]);
    assert!(strict_error(r"\ce{H☃}").contains("unsupported character in \\ce"));
}

// ---------- macros ----------

#[test]
fn paired_delimiter() {
    let def = r"\DeclarePairedDelimiter\norma{\lVert}{\rVert}";
    has(&tree(&format!(r"{def} \norma{{x}}")), &["ForcedClass { class: Open"]);
    has(&tree(&format!(r"{def} \norma*{{x}}")), &["Delimited"]);
    has(&tree(&format!(r"{def} \norma[\big]{{x}}")), &["FixedDelimiter"]);
}

#[test]
fn macro_arguments() {
    // '#' without a digit, at the end of the body, #0 and #2 with no second argument: only the nonexistent disappears
    has(&tree(r"\newcommand\a[1]{#1 + \#} \a{x}"), &["\"x\"", "'#'"]);
    has(&recovers(r"\newcommand\b[1]{#1#} \b{y}", "E-REC-9"), &["'#'"]);
    has(&tree(r"\newcommand\c[1]{[#0]} \c{z}"), &["'['"]);
    has(&tree(r"\newcommand\d[1]{(#2)} \d{z}"), &["'('"]);
    has(&tree(r"\newcommand\e[2][1]{#1/#2} \e{a} \e[b]{c}"), &["\"b\"", "\"c\""]);
    has(&tree(r"\newcommand*\f{x} \f"), &["\"x\""]);
    has(&tree(r"\newcommand{\g}{x} \providecommand{\g}{y} \g"), &["\"x\""]);
    has(&tree(r"\providecommand{\h}{y} \h"), &["\"y\""]);
    has(&recovers(r"\newcommand\h{#a} \h", "E-REC-9"), &["'#'"]);
    assert!(always_error(r"\newcommand x").contains("missing macro name"));
    assert!(always_error(r"\newcommand{}{x}").contains("empty macro name"));
    assert!(always_error(r"\newcommand\a[x]{y}").contains("invalid number of arguments"));
}

#[test]
fn def_and_let() {
    has(&tree(r"\def\a #1#2{#2#1} \a xy"), &["\"y\""]);
    assert!(always_error(r"\def\a").contains("without a body"));
    assert!(always_error(r"\def\a.{x}").contains("delimited"));
    has(&tree(r"\let\a = \alpha \a"), &["α"]);
    has(&tree(r"\def\b{x} \let\a\b \a"), &["\"x\""]);
    has(&tree(r"\let\a x \a"), &["\"x\""]);
    assert!(always_error(r"\let\a").contains("without a target"));
}

#[test]
fn defined_operators_and_environments() {
    has(&tree(r"\DeclareMathOperator*\argmax{arg\,max} \argmax_x"), &["limits: true"]);
    has(&tree(r"\DeclareMathOperator\sen{sen} \sen x"), &["\"sen\""]);
    assert!(always_error(r"\newenvironment{foo}[1]{a}{b}").contains("with arguments"));
    let t = tree(r"\newenvironment{e}[0]{(}{)} \begin{e}\begin{e}x\end{e}\end{e}");
    assert_eq!(t.matches("'('").count(), 2, "{t}");
    has(&recovers(r"\newenvironment{e}{(}{)} \begin{e} x", "E-REC-5"), &["\"x\""]);
}

#[test]
fn discarded_commands() {
    has(&tree(r"\aftergroup\x a"), &["\"a\""]);
    has(&tree(r"\aftergroup x a"), &["\"a\""]);
    has(&tree(r"a \aftergroup"), &["\"a\""]);
    has(&tree(r"\usepackage[x]{amsmath} \makeatletter \bfseries a"), &["\"a\""]);
}

#[test]
fn package_commands() {
    has(&tree(r"\dv{f}"), &["Fraction"]);
    has(&tree(r"\dv[2]{f}"), &["\"2\""]);
    has(&tree(r"\dv{f}{x}"), &["\"x\""]);
    has(&tree(r"\dv[2]{f}{x}"), &["\"2\""]);
    has(&tree(r"\pdv{f}{x}{y}"), &["'∂'", "\"y\""]);
    has(&tree(r"\Set{x | x>0}"), &["middles: ['|']"]);
    has(&tree(r"\set{a,b}"), &["Some('{')"]);
    has(&tree(r"\bra{a}\ket{b}\braket{a|b}"), &["Some('⟨')"]);
    has(&tree(r"\abs{x} \norm{y}"), &["Some('|')", "Some('‖')"]);
    has(&tree(r"\sfrac{1}{2} \unitfrac[3]{m}{s}"), &["\"3\""]);
}

// ---------- unknown commands ----------

#[test]
fn json_escapes_and_backslashes_before_symbols() {
    has(&recovers(r"x\u00b2", "E-REC-2"), &["'²'"]);
    // \u without four hex digits is not an escape
    has(&recovers(r"\u12", "E-REC-1"), &["\\\\u"]);
    has(&recovers(r#"\"a"#, "E-REC-1"), &["\"a\""]);
    has(&recovers(r"\5", "E-REC-1"), &["\"5\""]);
    // JSON \t only when the name is short: \tx is tab + x, \txyz is an unknown command
    has(&recovers(r"a\tx", "E-REC-2"), &["\"x\""]);
    has(&recovers(r"a\txyz", "E-REC-1"), &["\\\\txyz"]);
    assert!(strict_error(r"\foobar").contains("unsupported command"));
}

#[test]
fn doubled_backslashes() {
    assert_eq!(undo_doubled_backslashes(r"\\frac{1}{2}").as_deref(), Some(r"\frac{1}{2}"));
    // a single backslash proves the doubles are real line breaks
    assert_eq!(undo_doubled_backslashes(r"a \\ \frac{1}{2}"), None);
    assert_eq!(undo_doubled_backslashes("a + b"), None);
    assert_eq!(undo_doubled_backslashes(r"a \\ b"), None);
}

/// Negations starting with \n used to be read as a JSON \n followed by the relation: \nleq came out as ≤,
/// the opposite of what was written.
#[test]
fn amssymb_negations_are_not_json_line_breaks() {
    for (name, symbol) in [
        ("nless", '≮'),
        ("ngtr", '≯'),
        ("nleq", '≰'),
        ("ngeq", '≱'),
        ("nleqslant", '≰'),
        ("ngeqslant", '≱'),
        ("nleqq", '≰'),
        ("ngeqq", '≱'),
        ("nsim", '≁'),
        ("ncong", '≇'),
        ("nprec", '⊀'),
        ("nsucc", '⊁'),
        ("npreceq", '⋠'),
        ("nsucceq", '⋡'),
        ("nvdash", '⊬'),
        ("nvDash", '⊭'),
        ("nVdash", '⊮'),
        ("nVDash", '⊯'),
        ("ntriangleleft", '⋪'),
        ("ntriangleright", '⋫'),
        ("ntrianglelefteq", '⋬'),
        ("ntrianglerighteq", '⋭'),
        ("nsupseteq", '⊉'),
        ("nsubseteqq", '⊈'),
        ("nsupseteqq", '⊉'),
        ("nsubset", '⊄'),
        ("nsupset", '⊅'),
        ("nni", '∌'),
        ("nequiv", '≢'),
        ("napprox", '≉'),
        ("nsimeq", '≄'),
        ("nasymp", '≭'),
        ("nshortmid", '∤'),
        ("nshortparallel", '∦'),
        ("nleftarrow", '↚'),
        ("nrightarrow", '↛'),
        ("nleftrightarrow", '↮'),
        ("nLeftarrow", '⇍'),
        ("nRightarrow", '⇏'),
        ("nLeftrightarrow", '⇎'),
        ("lneq", '⪇'),
        ("gneq", '⪈'),
        ("lneqq", '≨'),
        ("gneqq", '≩'),
        ("lnsim", '⋦'),
        ("gnsim", '⋧'),
        ("lnapprox", '⪉'),
        ("gnapprox", '⪊'),
        ("precneqq", '⪵'),
        ("succneqq", '⪶'),
        ("precnsim", '⋨'),
        ("succnsim", '⋩'),
        ("precnapprox", '⪹'),
        ("succnapprox", '⪺'),
    ] {
        let t = tree(&format!(r"a \{name} b"));
        has(&t, &[&format!("Operator {{ symbol: '{symbol}', class: Rel }}")]);
    }
    // the speech no longer spells the letters of the relation
    let speech = convert(r"a \nleq b", &Options::default()).unwrap().speech;
    assert_eq!(speech, "a ≰ b");
}

#[test]
fn unknown_negation_of_a_known_relation_reads_as_not() {
    // \nlesssim does not exist, but \lesssim does: the author meant its negation, not a line break
    let t = recovers(r"a \nlesssim b", "E-REC-1");
    has(&t, &["Canceled { base: Operator { symbol: '≲'"]);
    let (_, warnings) = tolerant(r"a \nlesssim b");
    assert!(warnings.iter().any(|w| w.contains(r"read as \not\lesssim")), "{warnings:?}");
    // a \n before something that is not a relation is still a JSON line break
    has(&recovers(r"x\nb) y", "E-REC-2"), &["\"b\""]);
    has(&recovers(r"\nfoo", "E-REC-2"), &["\"f\""]);
}

#[test]
fn amssymb_and_stmaryrd_symbols() {
    for (name, symbol, class) in [
        ("dashv", '⊣', "Rel"),
        ("Join", '⨝', "Rel"),
        ("lessapprox", '⪅', "Rel"),
        ("gtrapprox", '⪆', "Rel"),
        ("lesseqgtr", '⋚', "Rel"),
        ("gtreqless", '⋛', "Rel"),
        ("gtrless", '≷', "Rel"),
        ("leqq", '≦', "Rel"),
        ("geqq", '≧', "Rel"),
        ("lll", '⋘', "Rel"),
        ("ggg", '⋙', "Rel"),
        ("divides", '∣', "Rel"),
        ("vartriangleleft", '⊲', "Rel"),
        ("trianglerighteq", '⊵', "Rel"),
        ("preccurlyeq", '≼', "Rel"),
        ("succsim", '≿', "Rel"),
        ("curlyeqprec", '⋞', "Rel"),
        ("eqslantless", '⪕', "Rel"),
        ("lessdot", '⋖', "Rel"),
        ("Vvdash", '⊪', "Rel"),
        ("VDash", '⊫', "Rel"),
        ("Subset", '⋐', "Rel"),
        ("eqcirc", '≖', "Rel"),
        ("bumpeq", '≏', "Rel"),
        ("approxeq", '≊', "Rel"),
        ("backsim", '∽', "Rel"),
        ("eqsim", '≂', "Rel"),
        ("fallingdotseq", '≒', "Rel"),
        ("doteqdot", '≑', "Rel"),
        ("between", '≬', "Rel"),
        ("pitchfork", '⋔', "Rel"),
        ("varsubsetneq", '⊊', "Rel"),
        ("supsetneq", '⊋', "Rel"),
        ("varsupsetneq", '⊋', "Rel"),
        ("subsetneqq", '⫋', "Rel"),
        ("supseteqq", '⫆', "Rel"),
        ("upharpoonright", '↾', "Rel"),
        ("upharpoonleft", '↿', "Rel"),
        ("downharpoonright", '⇂', "Rel"),
        ("downharpoonleft", '⇃', "Rel"),
        ("rightharpoondown", '⇁', "Rel"),
        ("leftharpoondown", '↽', "Rel"),
        ("leftrightarrows", '⇆', "Rel"),
        ("Lsh", '↰', "Rel"),
        ("looparrowright", '↬', "Rel"),
        ("rightarrowtail", '↣', "Rel"),
        ("upuparrows", '⇈', "Rel"),
        ("Rrightarrow", '⇛', "Rel"),
        ("leftrightsquigarrow", '↭', "Rel"),
        ("Updownarrow", '⇕', "Rel"),
        ("oslash", '⊘', "Bin"),
        ("ltimes", '⋉', "Bin"),
        ("rtimes", '⋊', "Bin"),
        ("leftthreetimes", '⋋', "Bin"),
        ("barwedge", '⊼', "Bin"),
        ("veebar", '⊻', "Bin"),
        ("curlywedge", '⋏', "Bin"),
        ("Cap", '⋒', "Bin"),
        ("Cup", '⋓', "Bin"),
        ("dotplus", '∔', "Bin"),
        ("divideontimes", '⋇', "Bin"),
        ("lhd", '⊲', "Bin"),
        ("unrhd", '⊵', "Bin"),
        ("sphericalangle", '∢', "Ord"),
        ("Diamond", '◇', "Ord"),
        ("surd", '√', "Ord"),
        ("hslash", 'ℏ', "Ord"),
        ("triangledown", '▽', "Ord"),
        ("backprime", '‵', "Ord"),
        ("Finv", 'Ⅎ', "Ord"),
        ("circledS", 'Ⓢ', "Ord"),
        ("standardstate", '⦵', "Ord"),
        ("hdots", '…', "Inner"),
    ] {
        has(&tree(&format!(r"a \{name} b")), &[&format!("Operator {{ symbol: '{symbol}', class: {class} }}")]);
    }
}

#[test]
fn every_new_symbol_is_in_the_font() {
    // strict mode turns a missing glyph into an error instead of a box
    let names = "nleq ngeq nless ngtr nsim ncong nprec nsucc npreceq nsucceq nvdash nvDash nVdash nVDash ntriangleleft ntriangleright \
                 ntrianglelefteq ntrianglerighteq nsupseteq nsubset nsupset nni nequiv napprox nsimeq nasymp nleftarrow nrightarrow \
                 nleftrightarrow nLeftarrow nRightarrow nLeftrightarrow lneq gneq lneqq gneqq lnsim gnsim lnapprox gnapprox precneqq \
                 succneqq precnsim succnsim precnapprox succnapprox dashv Join lessapprox gtrapprox lesseqgtr gtreqless gtrless leqq \
                 geqq lll ggg divides vartriangleleft vartriangleright trianglelefteq trianglerighteq preccurlyeq succcurlyeq precsim \
                 succsim precapprox succapprox curlyeqprec curlyeqsucc eqslantless eqslantgtr lessdot gtrdot Vvdash VDash Subset Supset \
                 eqcirc circeq bumpeq Bumpeq approxeq backsim backsimeq eqsim risingdotseq fallingdotseq doteqdot between pitchfork \
                 smallsmile smallfrown supsetneq varsubsetneq subsetneqq supsetneqq subseteqq supseteqq leftrightsquigarrow \
                 leftrightarrows Lsh Rsh looparrowleft looparrowright rightarrowtail leftarrowtail upuparrows downdownarrows Lleftarrow \
                 Rrightarrow Updownarrow upharpoonright upharpoonleft downharpoonright downharpoonleft rightharpoondown leftharpoondown \
                 oslash ltimes rtimes leftthreetimes rightthreetimes barwedge veebar curlywedge curlyvee Cap Cup dotplus divideontimes \
                 centerdot lhd rhd unlhd unrhd doublebarwedge sphericalangle Diamond surd hslash triangledown diagup diagdown backprime \
                 Finv Bbbk circledS standardstate hdots digamma Digamma varkappa iiiint varoiint intclockwise varointclockwise \
                 ointctrclockwise fint sqint idotsint i j";
    for name in names.split_whitespace() {
        let tex = format!(r"a \{name} b");
        let r = convert(&tex, &Options { strict: true, ..Options::default() });
        assert!(r.is_ok(), "{tex}: {:?}", r.err());
    }
}

#[test]
fn greek_variants_dotless_letters_and_arg_max() {
    has(&tree(r"\digamma + \Digamma + \varkappa"), &["\"ϝ\"", "\"Ϝ\"", "\"ϰ\""]);
    has(&tree(r"\i + \j"), &["\"ı\"", "\"ȷ\""]);
    has(&tree(r"\argmax_x f + \argmin_y g"), &["Function { name: \"arg max\", limits: true }", "Function { name: \"arg min\", limits: true }"]);
}

#[test]
fn more_integrals_keep_their_limits_on_the_side() {
    for (name, symbol) in [("iiiint", '⨌'), ("varoiint", '∯'), ("intclockwise", '∱'), ("varointclockwise", '∲'), ("ointctrclockwise", '∳'), ("fint", '⨏'), ("sqint", '⨖')]
    {
        has(&tree(&format!(r"\{name}_V f")), &[&format!("LargeOperator {{ symbol: '{symbol}'")]);
        let speech = convert(&format!(r"\{name}_V f"), &Options::default()).unwrap().speech;
        assert!(speech.starts_with("integral over V"), "{name}: {speech}");
    }
    // \idotsint is ∫⋯∫, and the subscript goes on the last integral
    has(&tree(r"\idotsint_V f"), &["LargeOperator { symbol: '∫'", "'⋯'", "sub: Some(Identifier { name: \"V\""]);
}

#[test]
fn english_locale_reads_thousands_comma() {
    let (r, _) = read_in("12,345.67", Locale::En, true);
    assert!(format!("{:?}", r.unwrap()).contains("thousands: Some(',')"));
    let (r, _) = read_in("3,5", Locale::En, true);
    assert!(!format!("{:?}", r.unwrap()).contains("\"3.5\""));
}

// ---------- problem cases ----------
//
// Each input below has crashed, hung or fooled some LaTeX reader. The rule is the same for
// all of them: never a panic or a hang; strict mode returns an error with a message, tolerant mode draws
// with a warning (or, when there is nothing to recover, also returns an error).

/// Both modes finish: no panic. Returns (strict mode error, tolerant mode warnings or error).
fn finishes(tex: &str) -> (Option<String>, Result<Vec<String>, String>) {
    let (strict, _) = read_in(tex, Locale::PtBr, true);
    let (tol, registry) = read_in(tex, Locale::PtBr, false);
    let tol = tol.map(|_| registry.warnings.iter().map(|a| a.rule.to_string()).collect()).map_err(|e| e.to_string());
    (strict.err().map(|e| e.to_string()), tol)
}

#[test]
fn p_unbalanced_braces() {
    for tex in ["{{{a", "a}}}", "}{", r"\frac{a}{b", r"x^{2"] {
        let (e, t) = finishes(tex);
        assert!(e.is_some(), "{tex}");
        assert!(t.as_ref().is_ok_and(|a| a.iter().any(|r| r == "E-REC-4")), "{tex}: {t:?}");
    }
}

#[test]
fn p_left_without_right_and_vice_versa() {
    for tex in [r"\left( x", r"x \right)", r"\left( \left[ x \right)", r"\right\} \left\{"] {
        let (e, t) = finishes(tex);
        assert!(e.is_some() && t.is_ok(), "{tex}: {e:?} {t:?}");
    }
}

#[test]
fn p_alignment_outside_environment() {
    // & and \\ at the top level are an implicit alignment: valid in both modes
    for tex in ["a & b", r"a \\ b", r"& \\ &", r"\\\\\\", "&&&&"] {
        let (e, t) = finishes(tex);
        assert!(e.is_none() && t.is_ok(), "{tex}: {e:?} {t:?}");
    }
}

#[test]
fn p_unpaired_begin_and_end() {
    for tex in
        [r"\begin{pmatrix} 1 & 2", r"1 & 2 \end{pmatrix}", r"\begin{pmatrix} 1 \end{bmatrix}", r"\begin{cases} a \begin{matrix} b \end{cases}", r"\begin", r"\end", r"\begin{"]
    {
        let (e, t) = finishes(tex);
        assert!(e.is_some() && t.is_ok(), "{tex}: {e:?} {t:?}");
    }
}

#[test]
fn p_recursive_macro() {
    for tex in [r"\newcommand\a{\a} \a", r"\def\a{\a\a} \a", r"\def\a{x\a} \a", r"\newcommand\a[1]{\a{#1#1}} \a{x}", r"\newcommand\a{\abs{\a}} \a"] {
        let (e, t) = finishes(tex);
        let e = e.unwrap_or_else(|| panic!("{tex}: strict mode accepted"));
        // \abs{\a} also nests: the depth limit arrives before the expansion limit
        if e.contains("nesting") {
            assert!(t.is_err_and(|t| t.contains("nesting")), "{tex}");
            continue;
        }
        assert!(e.contains("too many macros"), "{tex}: {e}");
        let a = t.unwrap_or_else(|e| panic!("{tex}: {e}"));
        // a single warning for the cause, not one per discarded expansion
        assert_eq!(a.iter().filter(|r| *r == "E-REC-1").count(), 1, "{tex}: {a:?}");
    }
}

#[test]
fn p_exponential_expansion() {
    // each level doubles: 2^30 copies if nobody stops it
    let mut tex = String::from(r"\def\a{xx}");
    for (i, name) in "bcdefghijklmnopqrstuvwxyz".chars().enumerate() {
        let previous = if i == 0 { 'a' } else { "bcdefghijklmnopqrstuvwxyz".chars().nth(i - 1).unwrap() };
        tex.push_str(&format!(r"\def\{name}{{\{previous}\{previous}}}"));
    }
    tex.push_str(r"\z");
    let start = std::time::Instant::now();
    let (e, t) = finishes(&tex);
    assert!(e.is_some() && t.is_ok(), "{e:?} {t:?}");
    assert!(start.elapsed().as_secs() < 10, "{:?}", start.elapsed());
}

#[test]
fn p_deep_nesting_does_not_overflow_the_stack() {
    let n = 10_000;
    for tex in [
        format!("{}x{}", "{".repeat(n), "}".repeat(n)),
        format!("{}x{}", r"\frac{".repeat(n), "}{y}".repeat(n)),
        format!("{}x", r"\frac".repeat(n)),
        format!("{}x{}", r"\sqrt{".repeat(n), "}".repeat(n)),
        format!("{}x{}", r"\left(".repeat(n), r"\right)".repeat(n)),
        format!("x{}", "^{a".repeat(n)),
        format!("x{}", "^a".repeat(n)),
        format!("{}x", r"\not".repeat(n)),
        format!("{}x{}", r"\text{$".repeat(n), "$}".repeat(n)),
        format!(r"\begin{{array}}{{{}c{}}} a \end{{array}}", "*{1}{".repeat(n), "}".repeat(n)),
    ] {
        // runs on a thread with the default test stack (2 MiB)
        let tex2 = tex.clone();
        let r = std::thread::spawn(move || finishes(&tex2)).join();
        let (e, t) = r.unwrap_or_else(|_| panic!("panic with {}…", &tex[..40]));
        if let Some(e) = &e {
            assert!(e.contains("nesting") || e.contains("without") || e.contains("double"), "{}…: {e}", &tex[..40]);
        }
        let _ = t;
    }
    // at the limit it still reads; one more level is a clear error
    let inside = format!("{}x{}", "{".repeat(NESTING_LIMIT - 2), "}".repeat(NESTING_LIMIT - 2));
    assert!(finishes(&inside).0.is_none());
    let outside = format!("{}x{}", "{".repeat(NESTING_LIMIT + 1), "}".repeat(NESTING_LIMIT + 1));
    assert!(always_error(&outside).contains("nesting"));
}

#[test]
fn p_nesting_at_the_limit_still_draws() {
    // the reader accepts; drawing the tree must also fit on the stack
    let bottom = (NESTING_LIMIT / 2) - 2;
    for tex in [format!("{}x{}", r"\frac{".repeat(bottom), "}{y}".repeat(bottom)), format!("{}x{}", "{".repeat(NESTING_LIMIT - 2), "}".repeat(NESTING_LIMIT - 2))] {
        let r = std::thread::spawn(move || convert(&tex, &Options::default()).map(|r| r.svg.len())).join();
        assert!(r.is_ok_and(|r| r.is_ok()));
    }
}

#[test]
fn p_one_megabyte_input() {
    let tex = "x + ".repeat(250_000) + "1";
    let start = std::time::Instant::now();
    let (e, t) = finishes(&tex);
    assert!(e.is_none() && t.is_ok());
    // groups and macros too: expansion must not go quadratic or hit the growth cap
    let groups = r"\bgroup a \egroup ".repeat(60_000);
    let (e, _) = finishes(&groups);
    assert!(e.is_none(), "{e:?}");
    // expansions that grow the text have a cap: past it strict mode refuses and tolerant mode discards
    let (e, t) = finishes(&r"\abs{x}".repeat(140_000));
    assert!(e.is_some_and(|e| e.contains("too many macros")) && t.is_ok_and(|a| a.iter().any(|r| r == "E-REC-1")));
    let (e, _) = finishes(&r"\begin{CD} A @>>> B \end{CD}".repeat(30_000));
    assert!(e.is_none(), "{e:?}");
    assert!(start.elapsed().as_secs() < 30, "{:?}", start.elapsed());
}

#[test]
fn p_empty_and_only_spaces() {
    for tex in ["", " ", "   \t  ", "\u{00A0}\u{2003}", "$$", "$ $", r"\[\]", "{}"] {
        let (e, t) = finishes(tex);
        assert!(e.is_none() && t.is_ok(), "{tex:?}: {e:?} {t:?}");
    }
}

#[test]
fn p_control_characters_and_nul() {
    for tex in ["a\u{0}b", "a\u{1}b", "\u{7F}", "x\u{1B}[31m"] {
        let (e, t) = finishes(tex);
        assert!(e.is_some(), "{tex:?}: strict mode accepted");
        let a = t.unwrap_or_else(|e| panic!("{tex:?}: {e}"));
        assert!(!a.is_empty(), "{tex:?}");
    }
    // and the drawing does not carry the control character into the SVG
    let svg = convert("a\u{0}b", &Options::default()).unwrap().svg;
    assert!(!svg.contains('\u{0}'));
}

#[test]
fn p_emoji_combining_marks_and_rtl() {
    // the reader accepts any character; whatever has no glyph becomes a box in the drawing (E-REC-8)
    for tex in ["🙂 + 1", "e\u{0301}", "\u{0301}", "שלום + 1", "مرحبا", "x\u{200D}y", "\u{FEFF}x", "👩‍👩‍👧"] {
        let (e, t) = finishes(tex);
        assert!(e.is_none() && t.is_ok(), "{tex:?}: {e:?} {t:?}");
        let r = convert(tex, &Options::default());
        assert!(r.is_ok(), "{tex:?}: {r:?}");
    }
    // JSON escape of half a surrogate pair: it does not become an invalid character
    let (e, t) = finishes(r"\ud83d\ude42");
    assert!(e.is_some() && t.is_ok(), "{e:?} {t:?}");
}

#[test]
fn p_repeated_backslashes() {
    for n in 1..=12 {
        let tex = "\\".repeat(n);
        let (_, t) = finishes(&tex);
        assert!(t.is_ok(), "{n} barras: {t:?}");
        let (_, t) = finishes(&format!("a{}b", "\\".repeat(n)));
        assert!(t.is_ok(), "a{n}b: {t:?}");
    }
}

#[test]
fn p_scripts_without_base_and_repeated() {
    for tex in ["^^", "^", "_", "^_^_", "x^^2", "x__2", "'", "x^", "x_", "^{}_{}", "x^1^2^3^4_1_2_3"] {
        let (_, t) = finishes(tex);
        assert!(t.is_ok(), "{tex}: {t:?}");
    }
    assert!(finishes("^_^_").0.is_some());
}

#[test]
fn p_unclosed_optionals_and_arguments() {
    for tex in [r"\sqrt[", r"\sqrt[3", r"\sqrt[3]", r"\sqrt{", r"\frac", r"\frac{", r"\binom{1}", r"\left", r"\text{", r"\begin{array}{", r"\cfrac[", r"\xrightarrow["] {
        let (_, t) = finishes(tex);
        assert!(t.is_ok(), "{tex}: {t:?}");
    }
}

#[test]
fn p_absurd_numbers_and_measures() {
    for tex in [
        r"\hdotsfor{99999999999999999999}",
        r"\multicolumn{99999}{c}{x}",
        r"\scalebox{1e30}{x}",
        r"\scalebox{NaN}{x}",
        r"\hspace{1e30em}",
        r"\rule{-5em}{-1em}",
        r"\begin{array}{*{50}{*{50}{*{50}{c}}}} a \end{array}",
        r"\cline{0-99999}",
        "9".repeat(10_000).as_str(),
    ] {
        let (_, t) = finishes(tex);
        if let Ok(a) = &t {
            let _ = a;
        }
        // the drawing also finishes and does not blow up memory
        let tex = tex.to_string();
        let r = std::thread::spawn(move || convert(&tex, &Options::default()).map(|r| r.svg.len())).join();
        assert!(r.is_ok(), "panic");
        if let Ok(Ok(size)) = r {
            assert!(size < 20_000_000, "SVG of {size} bytes");
        }
    }
}

#[test]
fn p_reading_fits_in_a_one_megabyte_stack() {
    // wasm and worker threads have short stacks: reading, even unoptimized, fits in 1 MiB
    let cases = [
        format!("{}x{}", r"\frac{".repeat(100), "}{y}".repeat(100)),
        format!("{}x{}", "{".repeat(10_000), "}".repeat(10_000)),
        format!("{}x{}", r"\left(".repeat(1_000), r"\right)".repeat(1_000)),
        format!("x{}", "_1".repeat(10_000)),
        format!("{}x{}", r"\begin{matrix}".repeat(1_000), r"\end{matrix}".repeat(1_000)),
    ];
    for tex in cases {
        let r = std::thread::Builder::new()
            .stack_size(1 << 20)
            .spawn(move || {
                let (strict, _) = read_in(&tex, Locale::PtBr, true);
                let (tol, _) = read_in(&tex, Locale::PtBr, false);
                (strict.map(|_| ()), tol.map(|_| ()))
            })
            .unwrap()
            .join()
            .expect("reading must not overflow the stack");
        // either it reads, or it refuses with a clear message
        for side in [r.0, r.1] {
            if let Err(e) = side {
                let e = e.to_string();
                assert!(e.contains("nesting") || e.contains("double"), "{e}");
            }
        }
    }
}

#[test]
fn mathtools_colon_equals_and_centernot() {
    has(&tree(r"A \coloneqq B"), &["'≔'", "Rel"]);
    has(&tree(r"A \coloneq B \eqcolon C \eqqcolon D \Coloneqq E"), &["'≔'", "'≕'", "'⩴'"]);
    // \centernot reads like \not: a negated relation when one exists, a slash otherwise
    assert_eq!(tree(r"a \centernot= b"), tree(r"a \not= b"));
    assert_eq!(tree(r"a \centernot\implies b"), tree(r"a \not\implies b"));
}

#[test]
fn unbraced_command_as_an_argument() {
    // found by the braille review: \boldsymbol\alpha read only the backslash as its argument
    assert_eq!(tree(r"\boldsymbol\alpha"), tree(r"\boldsymbol{\alpha}"));
    assert_eq!(tree(r"\mathbf\alpha + \text\o"), tree(r"\mathbf{\alpha} + \text{\o}"));
    assert_eq!(tree(r"\frac\alpha\beta"), tree(r"\frac{\alpha}{\beta}"));
}

#[test]
fn boldmath_is_bold_italic_and_reaches_a_rotatebox() {
    has(&tree(r"\text{{\boldmath $x$}}"), &["BoldItalic"]);
    has(&tree(r"\text{{\boldmath \rotatebox{90}{$x$}}}"), &["BoldItalic"]);
}

#[test]
fn a_quantitys_value_as_siunitx_prints_it() {
    assert_eq!(tree(r"\SI{+10}{V}"), tree(r"\SI{10}{V}"));
    assert_eq!(tree(r"\SI{.25}{V}"), tree(r"\SI{0.25}{V}"));
    assert_eq!(tree(r"\SI{-.5}{V}"), tree(r"\SI{-0.5}{V}"));
}
