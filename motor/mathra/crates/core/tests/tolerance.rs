//! Tolerant mode (spec/tolerance.md): invalid LaTeX becomes a drawing with a warning; strict mode refuses.

use mathra::{convert, Locale, Options};

fn tolerant(tex: &str) -> mathra::Output {
    convert(tex, &Options { locale: Locale::PtBr, ..Default::default() }).unwrap_or_else(|e| panic!("{tex}: {e}"))
}

fn warning_rules(tex: &str) -> Vec<&'static str> {
    tolerant(tex).warnings.iter().map(|a| a.rule).collect()
}

fn strict(tex: &str) -> Result<mathra::Output, mathra::Error> {
    convert(tex, &Options { locale: Locale::PtBr, strict: true, ..Default::default() })
}

#[test]
fn t_tol_valid_equation_same_in_both_modes() {
    for tex in [r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}", r"\begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}", r"\SI{9,8}{\meter}"] {
        let a = tolerant(tex);
        let b = strict(tex).unwrap();
        assert_eq!(a.svg, b.svg, "{tex}");
        assert!(a.warnings.is_empty(), "{tex}: {:?}", a.warnings);
    }
}

#[test]
fn e_rec_1_unknown_command() {
    assert_eq!(tolerant(r"\vertA\vert = 1").text, "|A| = 1");
    assert!(tolerant(r"\foobar + 1").svg.contains(r##"fill="#c0392b""##));
    assert!(strict(r"\foobar").is_err());
}

#[test]
fn e_rec_2_json_escapes() {
    assert_eq!(tolerant(r"a²+b").text, "a² + b");
    assert_eq!(warning_rules(r"x\nb) y"), vec!["E-REC-2"]);
}

#[test]
fn e_rec_3_double_script_in_chemistry() {
    let r = tolerant(r"\text{HSO}_4^-_{(aq)}");
    assert_eq!(warning_rules(r"\text{HSO}_4^-_{(aq)}"), vec!["E-REC-3"]);
    assert!(r.text.contains("HSO_4^−"), "{}", r.text);
}

#[test]
fn e_rec_4_and_5_unpaired_braces_and_delimiters() {
    assert_eq!(tolerant(r"x = \frac{1}{2").text, "x = 1/2");
    assert_eq!(tolerant(r"a } + b").text, "a + b");
    assert_eq!(tolerant(r"\left( x + 1").text, "(x + 1");
    assert_eq!(warning_rules(r"\left( x"), vec!["E-REC-5"]);
}

#[test]
fn e_rec_6_unknown_environment() {
    assert_eq!(tolerant(r"\begin{foo} a & b \end{foo}").text, "a, b");
}

#[test]
fn e_rec_7_and_9_missing_argument_and_dollar_in_text() {
    assert_eq!(tolerant(r"\xrightleftharpoons").text, "⇌");
    assert_eq!(tolerant(r"15 + 15 = \text{R$ } 30,00").text, "15 + 15 = R$ 30,00");
}

#[test]
fn bordermatrix() {
    let r = tolerant(r"\bordermatrix{ & c_1 & c_2 \cr r_1 & a & b \cr r_2 & c & d}");
    assert!(r.warnings.is_empty());
    assert!(r.speech.starts_with("matriz 3 por 3"), "{}", r.speech);
}

#[test]
fn backslash_before_unicode_char_does_not_panic() {
    // regression: the prefix cut of \vertA used to cut '∠' in the middle of its bytes
    for tex in [r"\∠ABC", r"\é", r"\π r^2", r"\—"] {
        tolerant(tex);
    }
}

#[test]
fn e_rec_2_command_eaten_by_json() {
    // "\frac" in JSON without escaping becomes form feed + "rac"
    assert_eq!(tolerant("\u{0C}rac{1}{2} \times 3").text, "1/2 × 3");
    assert_eq!(tolerant("\u{08}egin{pmatrix} 1 \\\\ 2 \\end{pmatrix}").text, "([1], [2])");
    // a real tab is still a space
    assert_eq!(tolerant("a\tb").text, "ab");
}

#[test]
fn e_rec_2_doubled_backslashes() {
    assert_eq!(tolerant(r"y = 17,9 \\cdot 2,5").text, "y = 17,9 · 2,5");
    // with any single-backslash command, the doubles are real line breaks
    assert!(tolerant(r"\begin{cases} a \\ b \end{cases}").warnings.is_empty());
}

/// These inputs used to stop tolerant mode with an error and no SVG at all, against "draw what it can".
#[test]
fn units_angles_and_physics_qty_never_stop_the_drawing() {
    for tex in [r"\qty(\frac{a}{b})", r"\SI{1}{\angstrom}", r"\ang{12;30;15}", r"\qty[x]", r"\qty|x|"] {
        let r = tolerant(tex);
        assert!(r.warnings.is_empty(), "{tex}: {:?}", r.warnings);
        assert!(strict(tex).is_ok(), "{tex}");
        assert!(r.svg.contains("<use"), "{tex}: nothing drawn");
    }
    // what is still unknown is drawn as written, with a warning, instead of refused
    let r = tolerant(r"\SI{3}{\furlong}");
    assert_eq!(r.text, "3 furlong");
    assert_eq!(warning_rules(r"\SI{3}{\furlong}"), vec!["E-REC-1"]);
    assert!(strict(r"\SI{3}{\furlong}").is_err());
    assert_eq!(tolerant(r"\SI{x}{\meter}").text, "x m");
    assert_eq!(warning_rules(r"\SI{x}{\meter}"), vec!["E-REC-9"]);
    assert_eq!(warning_rules(r"\ang{1;2;3;4}"), vec!["E-REC-9"]);
    assert_eq!(tolerant(r"\num{1.2.3}").text, "1,2,3");
}

#[test]
fn e_rec_1_negation_of_a_known_relation() {
    // amssymb has no \nlesssim, but \lesssim exists: a crossed ≲, not a line break and "lesssim"
    let r = tolerant(r"a \nlesssim b");
    assert_eq!(warning_rules(r"a \nlesssim b"), vec!["E-REC-1"]);
    assert!(r.speech.contains("≲") && !r.speech.contains("l e s s"), "{}", r.speech);
    assert!(strict(r"a \nlesssim b").is_err());
    // the existing negations are not recovered: they are known, with no warning
    assert!(tolerant(r"a \nleq b \ngeq c \nsim d").warnings.is_empty());
}

#[test]
fn e_rec_9_unknown_character_and_bond_in_ce() {
    assert_eq!(warning_rules(r"\ce{H☃}"), vec!["E-REC-9"]);
    assert_eq!(warning_rules(r"\ce{A \bond{?} B}"), vec!["E-REC-9"]);
    assert!(strict(r"\ce{A \bond{?} B}").is_err());
}

#[test]
fn e_rec_10_unknown_mathml() {
    let mathml = |src: &str, strict: bool| convert(src, &Options { input: mathra::Input::MathMl, strict, ..Default::default() });
    let r = mathml("<math><mi>x</mi><mfoo><mn>1</mn></mfoo></math>", false).unwrap();
    assert_eq!(r.text, "x1");
    assert_eq!(r.warnings.iter().map(|a| a.rule).collect::<Vec<_>>(), vec!["E-REC-10"]);
    assert!(mathml("<math><mi>x</mi><mfoo><mn>1</mn></mfoo></math>", true).is_err());
    // the XML itself must be well formed in both modes: there is no tree to recover from
    assert!(mathml("<math><mi>x</math>", false).is_err());
}
