//! AsciiMath input (input/asciimath.rs): the grammar of asciimath.org, its symbols, brackets, matrices,
//! fractions without their brackets, and the recovery of unpaired brackets.

use mathra::tree::{FractionStyle, TableKind, Variant};
use mathra::{convert, Input, Locale, Node, Options};

fn options(strict: bool) -> Options {
    Options { input: Input::AsciiMath, strict, ..Options::default() }
}

/// Valid AsciiMath: the same in both modes, with no warnings.
fn am(src: &str) -> mathra::Output {
    let strict = convert(src, &options(true)).unwrap_or_else(|e| panic!("{src}: {e}"));
    let tolerant = convert(src, &options(false)).unwrap();
    assert!(tolerant.warnings.is_empty(), "{src}: {:?}", tolerant.warnings);
    assert_eq!(strict.svg, tolerant.svg, "{src}");
    strict
}

fn first(src: &str) -> Node {
    am(src).tree.children()[0].clone()
}

fn latex(tex: &str) -> mathra::Output {
    convert(tex, &Options { strict: true, ..Options::default() }).unwrap_or_else(|e| panic!("{tex}: {e}"))
}

/// The same equation in AsciiMath and in LaTeX: the same speech and the same linear text.
#[test]
fn same_meaning_as_latex() {
    for (src, tex) in [
        ("x = (-b +- sqrt(b^2-4ac))/(2a)", r"x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}"),
        ("sum_(i=1)^n i^2 = (n(n+1)(2n+1))/6", r"\sum_{i=1}^n i^2 = \frac{n(n+1)(2n+1)}{6}"),
        ("int_0^1 x^2 dx", r"\int_0^1 x^2 d x"),
        ("lim_(x->oo) (1+1/x)^x = e", r"\lim_{x\to\infty} \left(1+\frac{1}{x}\right)^x = e"),
        ("e^(i pi) + 1 = 0", r"e^{i\pi} + 1 = 0"),
        ("sin^2 theta + cos^2 theta = 1", r"\sin^2\theta + \cos^2\theta = 1"),
        ("a in A, b !in B, A sube B", r"a \in A, b \notin B, A \subseteq B"),
        ("AA x EE y: x != y", r"\forall x \exists y: x \ne y"),
        ("root(3)(8) = 2", r"\sqrt[3]{8} = 2"),
        ("|x| + abs(y) + norm(v)", r"\left|x\right| + \left|y\right| + \left\|v\right\|"),
        ("floor(x) + ceil(y)", r"\left\lfloor x \right\rfloor + \left\lceil y \right\rceil"),
        ("hat x + vec v + bar x + ul y + dot x + ddot y + tilde z", r"\hat{x} + \vec{v} + \overline{x} + \underline{y} + \dot{x} + \ddot{y} + \tilde{z}"),
        ("RR, NN, ZZ, QQ, CC", r"\mathbb{R}, \mathbb{N}, \mathbb{Z}, \mathbb{Q}, \mathbb{C}"),
        ("x xx y -: z ** w", r"x \times y \div z \ast w"),
        ("a <= b >= c ~~ d -= e prop f", r"a \le b \ge c \approx d \equiv e \propto f"),
        ("A => B iff C", r"A \implies B \iff C"),
        ("f: A -> B, x |-> x^2", r"f: A \to B, x \mapsto x^2"),
        ("del f // del x + grad g", r"\partial f / \partial x + \nabla g"),
        ("alpha beta Gamma Delta varphi omega", r"\alpha \beta \Gamma \Delta \varphi \omega"),
        ("[[a,b],[c,d]]", r"\begin{bmatrix} a & b \\ c & d \end{bmatrix}"),
        ("((1,2),(3,4))", r"\begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix}"),
        ("text(speed) = d/t", r"\text{speed} = \frac{d}{t}"),
    ] {
        let a = am(src);
        let b = latex(tex);
        assert_eq!(a.speech, b.speech, "{src}");
        assert_eq!(a.text, b.text, "{src}");
    }
}

#[test]
fn fractions_drop_the_brackets_of_their_parts() {
    let Node::Fraction { num, den, style: FractionStyle::Auto, rule: true, .. } = first("(a+b)/(c+d)") else { panic!() };
    assert!(!format!("{num:?}{den:?}").contains("Delimited"));
    // brackets of scripts go too, but not of a function's argument
    assert!(!format!("{:?}", am("x^(2n)").tree).contains("Delimited"));
    assert!(format!("{:?}", am("f(x)").tree).contains("Delimited"));
    assert!(matches!(first("frac(a)(b)"), Node::Fraction { .. }));
    // a/b/c: the first slash makes a fraction, the second is drawn
    assert_eq!(am("a/b/c").text, "a/b/c");
    assert!(matches!(first("a/b/c"), Node::Fraction { .. }));
}

#[test]
fn brackets_and_intervals() {
    // any closing bracket closes: half-open intervals
    let Node::Delimited { open: Some('('), close: Some(']'), .. } = first("(a,b]") else { panic!() };
    assert!(matches!(first("(:a, b:)"), Node::Delimited { open: Some('⟨'), close: Some('⟩'), .. }));
    assert!(matches!(first("<<a, b>>"), Node::Delimited { open: Some('⟨'), close: Some('⟩'), .. }));
    // invisible brackets group without drawing
    let r = am("{:a+b:}^2");
    assert_eq!(r.tree, latex("{a+b}^2").tree);
    assert!(!r.text.contains('{'));
    // a lone bar stays a bar
    assert_eq!(am("a | b").text, latex("a | b").text);
    assert!(matches!(first("|x|"), Node::Delimited { open: Some('|'), close: Some('|'), .. }));
}

#[test]
fn matrices_and_cases() {
    for (src, shape) in [
        ("[[a,b],[c,d]]", TableKind::BMatrix),
        ("((a,b),(c,d))", TableKind::PMatrix),
        ("{(a,b),(c,d)}", TableKind::BraceMatrix),
        ("{:(a,b),(c,d):}", TableKind::Matrix),
        ("{(x, x>0),(0, x<=0):}", TableKind::Cases),
    ] {
        let n = first(src);
        assert!(matches!(n, Node::Table { shape: s, .. } if s == shape), "{src}: {n:?}");
    }
    let Node::Table { rows, .. } = first("[[1,2,3],[4,5,6]]") else { panic!() };
    assert_eq!((rows.len(), rows[0].len()), (2, 3));
    // rows of different length, or a single row, are not a matrix
    assert!(matches!(first("[[a,b],[c]]"), Node::Delimited { .. }));
    assert!(matches!(first("[(a,b)]"), Node::Delimited { .. }));
    let r = am("f(x) = {(x^2, if x >= 0),(-x, if x < 0):}");
    assert!(r.speech.contains("cases: case 1: x squared, if x is greater than or equal to 0"), "{}", r.speech);
}

#[test]
fn fonts_text_color_and_braces() {
    assert!(matches!(first("bb v"), Node::Identifier { variant: Variant::Bold, .. }));
    assert!(matches!(first("bbb N"), Node::Identifier { variant: Variant::DoubleStruck, .. }));
    assert!(matches!(first("cc L"), Node::Identifier { variant: Variant::Calligraphic, .. }));
    assert!(matches!(first("fr g"), Node::Identifier { variant: Variant::Fraktur, .. }));
    assert!(matches!(first("tt x"), Node::Identifier { variant: Variant::Monospace, .. }));
    assert!(matches!(first("sf x"), Node::Identifier { variant: Variant::SansSerif, .. }));
    assert_eq!(am(r#""hello world" + text(a b)"#).text, "hello world + a b");
    assert!(matches!(first("color(red)(x)"), Node::Colored { ref color, .. } if color == "red"));
    assert!(matches!(first("ubrace(1+2)_(3)"), Node::Brace { above: false, label: Some(_), .. }));
    assert!(matches!(first("obrace(1+2)^(3)"), Node::Brace { above: true, label: Some(_), .. }));
    assert!(matches!(first("cancel(x)"), Node::Canceled { .. }));
    assert!(matches!(first("stackrel(def)(=)"), Node::ForcedClass { class: mathra::Class::Rel, .. }));
    assert!(matches!(first("underset(x)(min)"), Node::ForcedClass { .. }));
}

#[test]
fn letters_numbers_and_symbols() {
    // a word that is not a symbol is read letter by letter, as asciimath.org does
    assert_eq!(am("speed").speech, "s p e e d");
    assert!(matches!(first("sin x"), Node::Function { .. }));
    assert_eq!(first("3.14"), Node::Number { value: "3.14".into(), group: false, thousands: None });
    // in pt-BR a comma between digits is decimal (L-NUM-1); with a space it separates
    let pt = |s: &str| convert(s, &Options { input: Input::AsciiMath, locale: Locale::PtBr, ..Options::default() }).unwrap();
    assert_eq!(pt("3,14").tree.children()[0], Node::Number { value: "3.14".into(), group: false, thousands: None });
    assert_eq!(pt("f(1, 5)").text, "f(1, 5)");
    assert_eq!(am("x_1^2").speech, "x sub 1 squared");
    assert_eq!(am("^14_6 C").text, "^14_6C");
    assert!(am("x").svg.contains(r#"<mathra:source type="asciimath">x</mathra:source>"#));
}

#[test]
fn e_rec_5_unpaired_brackets() {
    for (src, text) in [("(a+b", "(a + b"), ("a+b)", "a + b)"), ("a :} b", "ab")] {
        let r = convert(src, &options(false)).unwrap();
        assert_eq!(r.text, text, "{src}");
        assert_eq!(r.warnings.iter().map(|a| a.rule).collect::<Vec<_>>(), vec!["E-REC-5"], "{src}");
        assert!(convert(src, &options(true)).is_err(), "{src}");
    }
    assert!(convert("color(re;d)(x)", &options(false)).unwrap().warnings.iter().any(|a| a.rule == "E-REC-9"));
    let deep = format!("{}x{}", "(".repeat(200), ")".repeat(200));
    assert!(convert(&deep, &options(false)).unwrap_err().to_string().contains("nesting"));
}

#[test]
fn empty_and_odd_inputs_do_not_panic() {
    for src in ["", "   ", "/", "^", "_", "((", "))", "[[", "]]", "|", "||", "\"unclosed", "text(", "color(red)", "frac", "root(3)", "{:", "a/", "sqrt"] {
        let _ = convert(src, &options(false));
        let _ = convert(src, &options(true));
    }
}
