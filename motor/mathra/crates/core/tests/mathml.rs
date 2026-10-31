//! MathML input (input/mathml.rs): presentation elements, fences, tables, scripts, decorations,
//! entities, tolerant recovery and the recognition of MathML pasted as LaTeX (L-ENT-4).

use mathra::tree::{Cancel, Frame, TableKind, Variant};
use mathra::{convert, Input, Locale, Node, Options};

fn options(strict: bool) -> Options {
    Options { input: Input::MathMl, strict, ..Options::default() }
}

/// Valid MathML: the same in both modes, with no warnings.
fn mml(src: &str) -> mathra::Output {
    let m = if mathra::input::mathml::looks_like_mathml(src) { src.to_string() } else { format!("<math>{src}</math>") };
    let strict = convert(&m, &options(true)).unwrap_or_else(|e| panic!("{m}: {e}"));
    let tolerant = convert(&m, &options(false)).unwrap();
    assert!(tolerant.warnings.is_empty(), "{m}: {:?}", tolerant.warnings);
    assert_eq!(strict.svg, tolerant.svg, "{m}");
    strict
}

fn first(src: &str) -> Node {
    mml(src).tree.children()[0].clone()
}

fn latex(tex: &str) -> mathra::Output {
    convert(tex, &Options { strict: true, ..Options::default() }).unwrap_or_else(|e| panic!("{tex}: {e}"))
}

/// The same equation in MathML and in LaTeX: the same speech and the same linear text.
#[test]
fn same_meaning_as_latex() {
    for (m, tex) in [
        (
            "<mi>x</mi><mo>=</mo><mfrac><mrow><mo>−</mo><mi>b</mi><mo>±</mo><msqrt><msup><mi>b</mi><mn>2</mn></msup><mo>−</mo><mn>4</mn><mi>a</mi><mi>c</mi></msqrt></mrow><mrow><mn>2</mn><mi>a</mi></mrow></mfrac>",
            r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
        ),
        ("<munderover><mo>∑</mo><mrow><mi>n</mi><mo>=</mo><mn>1</mn></mrow><mi>∞</mi></munderover><mfrac><mn>1</mn><msup><mi>n</mi><mn>2</mn></msup></mfrac>", r"\sum_{n=1}^{\infty} \frac{1}{n^2}"),
        ("<msubsup><mo>∫</mo><mn>0</mn><mn>1</mn></msubsup><msup><mi>x</mi><mn>2</mn></msup><mi mathvariant=\"normal\">d</mi><mi>x</mi>", r"\int_0^1 x^2 \mathrm{d}x"),
        ("<msup><mi>e</mi><mrow><mi>i</mi><mi>π</mi></mrow></msup><mo>+</mo><mn>1</mn><mo>=</mo><mn>0</mn>", r"e^{i\pi} + 1 = 0"),
        ("<mi>sin</mi><mo>&ApplyFunction;</mo><mi>θ</mi>", r"\sin\theta"),
        ("<munder><mi>lim</mi><mrow><mi>x</mi><mo>→</mo><mn>0</mn></mrow></munder><mi>x</mi>", r"\lim_{x \to 0} x"),
        ("<mroot><mn>8</mn><mn>3</mn></mroot><mo>=</mo><mn>2</mn>", r"\sqrt[3]{8} = 2"),
        ("<mi>a</mi><mo>≰</mo><mi>b</mi><mo>⇒</mo><mi>c</mi><mo>∈</mo><mi mathvariant=\"double-struck\">R</mi>", r"a \nleq b \Rightarrow c \in \mathbb{R}"),
        ("<mover><mi>v</mi><mo>→</mo></mover><mo>⋅</mo><mover><mi>x</mi><mo>^</mo></mover>", r"\vec{v} \cdot \hat{x}"),
        ("<mrow><mo>|</mo><mi>x</mi><mo>|</mo></mrow>", r"\left| x \right|"),
    ] {
        let a = mml(m);
        let b = latex(tex);
        assert_eq!(a.speech, b.speech, "{m}");
        assert_eq!(a.text, b.text, "{m}");
    }
}

#[test]
fn token_elements() {
    // one letter in italic, uppercase Greek upright, several letters a function or an upright word
    assert_eq!(first("<mi>x</mi>"), Node::Identifier { name: "x".into(), variant: Variant::Italic });
    assert_eq!(first("<mi>Γ</mi>"), Node::Identifier { name: "Γ".into(), variant: Variant::Upright });
    assert_eq!(first("<mi>sin</mi>"), Node::Function { name: "sin".into(), limits: false });
    assert_eq!(first("<mi>speed</mi>"), Node::Function { name: "speed".into(), limits: false });
    assert_eq!(first("<mi mathvariant=\"bold\">AB</mi>"), Node::Identifier { name: "AB".into(), variant: Variant::Bold });
    assert_eq!(first("<mi mathvariant=\"bold\">v</mi>"), Node::Identifier { name: "v".into(), variant: Variant::Bold });
    assert_eq!(first("<mi> x </mi>"), Node::Identifier { name: "x".into(), variant: Variant::Italic });
    assert_eq!(first("<mn>3.14</mn>"), Node::Number { value: "3.14".into(), group: false, thousands: None });
    assert!(matches!(first("<mo>∑</mo>"), Node::LargeOperator { symbol: '∑', .. }));
    assert!(matches!(first("<mo>lim</mo>"), Node::Function { limits: true, .. }));
    assert_eq!(mml("<mtext>if </mtext><ms>s</ms>").text, "if\"s\"");
    assert!(matches!(first("<mspace width=\"1em\"/>"), Node::Space { em } if (em - 1.0).abs() < 1e-6));
    assert!(matches!(first("<mspace width=\"thickmathspace\"/>"), Node::Space { em } if (em - 5.0 / 18.0).abs() < 1e-6));
}

#[test]
fn numbers_follow_the_locale() {
    let pt = |m: &str| convert(&format!("<math>{m}</math>"), &Options { input: Input::MathMl, locale: Locale::PtBr, ..Options::default() }).unwrap();
    // a decimal comma in pt-BR (L-NUM-1), thousands with a dot (L-NUM-3)
    assert_eq!(pt("<mn>3,14</mn>").tree.children()[0], Node::Number { value: "3.14".into(), group: false, thousands: None });
    assert_eq!(pt("<mn>1.000</mn>").tree.children()[0], Node::Number { value: "1000".into(), group: false, thousands: Some('.') });
    assert_eq!(first("<mn>1,000</mn>"), Node::Number { value: "1000".into(), group: false, thousands: Some(',') });
    assert_eq!(pt("<mn>3.14</mn>").text, "3,14");
}

#[test]
fn operators_take_the_class_of_their_character() {
    use mathra::Class;
    for (mo, class) in [
        ("+", Class::Bin),
        ("=", Class::Rel),
        ("≰", Class::Rel),
        ("⊗", Class::Bin),
        ("(", Class::Open),
        ("{", Class::Open),
        ("]", Class::Close),
        (",", Class::Punct),
        ("∞", Class::Ord),
        ("→", Class::Rel),
    ] {
        let tree = mml(&format!("<mi>a</mi><mo>{mo}</mo><mi>b</mi>")).tree;
        assert!(tree.children().iter().any(|c| matches!(c, Node::Operator { class: k, .. } if *k == class)), "{mo}: {tree:?}");
    }
    // invisible operators draw nothing
    assert_eq!(mml("<mi>a</mi><mo>&InvisibleTimes;</mo><mi>b</mi><mo>&#x2061;</mo>").tree.children().len(), 2);
}

#[test]
fn fences_stretch_and_wrap_tables() {
    assert!(matches!(first("<mrow><mo>(</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo>)</mo></mrow>"), Node::Delimited { open: Some('('), close: Some(')'), .. }));
    // not balanced inside: ( a ) + ( b ) keeps its plain parentheses
    let t = mml("<mrow><mo>(</mo><mi>a</mi><mo>)</mo><mo>+</mo><mo>(</mo><mi>b</mi><mo>)</mo></mrow>").tree;
    assert_eq!(t.children().len(), 7, "{t:?}");
    assert!(!format!("{t:?}").contains("Delimited"), "{t:?}");
    let table = "<mtable><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr><mtr><mtd><mi>c</mi></mtd><mtd><mi>d</mi></mtd></mtr></mtable>";
    for (open, close, shape) in
        [("(", ")", TableKind::PMatrix), ("[", "]", TableKind::BMatrix), ("|", "|", TableKind::VMatrix), ("‖", "‖", TableKind::VVMatrix), ("{", "}", TableKind::BraceMatrix)]
    {
        let n = first(&format!("<mrow><mo>{open}</mo>{table}<mo>{close}</mo></mrow>"));
        assert!(matches!(n, Node::Table { shape: s, .. } if s == shape), "{open}{close}: {n:?}");
    }
    assert!(matches!(first(&format!("<mrow><mo>{{</mo>{table}</mrow>")), Node::Table { shape: TableKind::Cases, .. }));
    assert!(matches!(first(&format!("<mrow>{table}<mo>}}</mo></mrow>")), Node::Table { shape: TableKind::RCases, .. }));
    assert!(matches!(first(&format!("<mfenced open=\"[\" close=\"]\">{table}</mfenced>")), Node::Table { shape: TableKind::BMatrix, .. }));
    assert_eq!(mml("<mfenced><mi>a</mi><mi>b</mi></mfenced>").text, "(a, b)");
    assert_eq!(mml("<mfenced open=\"[\" close=\")\" separators=\";\"><mi>a</mi><mi>b</mi></mfenced>").text, "[a; b)");
}

#[test]
fn tables_with_alignment_lines_and_labels() {
    let n = first("<mtable columnalign=\"right left\" columnlines=\"solid\" rowlines=\"dashed\"><mtr><mtd><mi>a</mi></mtd><mtd><mi>b</mi></mtd></mtr><mtr><mtd><mi>c</mi></mtd><mtd><mi>d</mi></mtd></mtr></mtable>");
    let Node::Table { shape: TableKind::Array, columns, verticals, horizontals, .. } = n else { panic!("{n:?}") };
    assert_eq!(columns, vec!['r', 'l']);
    assert_eq!(verticals, vec![1]);
    assert_eq!(horizontals.len(), 1);
    // the label of mlabeledtr goes to the end of the row, like \tag
    let r = mml("<mtable><mlabeledtr><mtd><mtext>(1)</mtext></mtd><mtd><mi>x</mi></mtd></mlabeledtr></mtable>");
    assert!(r.text.contains("x (1)"), "{}", r.text);
}

#[test]
fn scripts_limits_and_multiscripts() {
    assert!(matches!(first("<msub><mi>x</mi><mn>1</mn></msub>"), Node::Scripts { sub: Some(_), sup: None, .. }));
    assert!(matches!(first("<msubsup><mi>x</mi><mn>1</mn><mn>2</mn></msubsup>"), Node::Scripts { sub: Some(_), sup: Some(_), .. }));
    // munderover over a large operator: limits, also for an integral MathML stacks on purpose
    let Node::Scripts { base, .. } = first("<munderover><mo>∫</mo><mn>0</mn><mn>1</mn></munderover>") else { panic!() };
    assert!(matches!(*base, Node::LargeOperator { symbol: '∫', limits: Some(true) }));
    // isotope: prescripts on an empty base
    assert_eq!(mml("<mmultiscripts><mi>C</mi><mprescripts/><mn>6</mn><mn>14</mn></mmultiscripts>").text, "_6^14C");
    assert_eq!(mml("<mmultiscripts><mi>R</mi><mi>i</mi><none/><none/><mi>j</mi></mmultiscripts>").text, "R_i^j");
    // stacking that is neither an accent nor a limit
    assert!(matches!(first("<mover><mo>=</mo><mtext>def</mtext></mover>"), Node::ForcedClass { class: mathra::Class::Rel, .. }));
}

#[test]
fn decorations_over_and_under() {
    assert!(matches!(first("<mover><mi>v</mi><mo>→</mo></mover>"), Node::Accent { accent: '\u{20D7}', .. }));
    assert!(matches!(first("<mover accent=\"true\"><mi>x</mi><mo>^</mo></mover>"), Node::Accent { accent: '\u{0302}', stretchy: false, .. }));
    assert!(matches!(first("<mover><mi>x</mi><mo>˙</mo></mover>"), Node::Accent { accent: '\u{0307}', .. }));
    assert!(matches!(first("<mover><mrow><mi>A</mi><mi>B</mi></mrow><mo>¯</mo></mover>"), Node::Bar { above: true, .. }));
    assert!(matches!(first("<munder><mi>x</mi><mo>_</mo></munder>"), Node::Bar { above: false, .. }));
    assert!(matches!(first("<munder><mrow><mi>A</mi><mi>B</mi></mrow><mo>→</mo></munder>"), Node::Accent { accent: '→', below: true, .. }));
    assert!(matches!(first("<mover><mrow><mi>A</mi><mi>B</mi></mrow><mo>⏜</mo></mover>"), Node::Accent { accent: '⏜', .. }));
    // a brace and its label, as MathJax writes them
    let n = first("<munder><munder><mrow><mi>a</mi><mo>+</mo><mi>b</mi></mrow><mo>⏟</mo></munder><mi>n</mi></munder>");
    assert!(matches!(n, Node::Brace { above: false, label: Some(_), bracket: false, .. }), "{n:?}");
    assert!(matches!(first("<mover><mi>x</mi><mo>⎴</mo></mover>"), Node::Brace { above: true, bracket: true, .. }));
}

#[test]
fn enclose_style_and_wrappers() {
    assert!(matches!(first("<menclose notation=\"box\"><mi>x</mi></menclose>"), Node::Framed { shape: Frame::Rectangle, .. }));
    assert!(matches!(first("<menclose notation=\"circle\"><mi>x</mi></menclose>"), Node::Framed { shape: Frame::Circle, .. }));
    assert!(matches!(first("<menclose notation=\"updiagonalstrike downdiagonalstrike\"><mi>x</mi></menclose>"), Node::Canceled { shape: Cancel::X, .. }));
    assert!(matches!(first("<menclose notation=\"radical\"><mi>x</mi></menclose>"), Node::Root { .. }));
    assert!(matches!(first("<mstyle mathcolor=\"red\"><mi>x</mi></mstyle>"), Node::Colored { .. }));
    assert!(matches!(first("<mi mathcolor=\"#00f\">x</mi>"), Node::Colored { .. }));
    assert!(matches!(first("<mstyle mathbackground=\"yellow\"><mi>x</mi></mstyle>"), Node::Framed { shape: Frame::Fill, .. }));
    assert!(matches!(first("<mstyle displaystyle=\"true\"><mi>x</mi></mstyle>"), Node::Style { level: 0, .. }));
    assert!(matches!(first("<mphantom><mi>x</mi></mphantom>"), Node::Phantom { .. }));
    // mathvariant on mstyle reaches the letters inside
    assert!(matches!(first("<mstyle mathvariant=\"bold\"><mi>x</mi></mstyle>"), Node::Identifier { variant: Variant::Bold, .. }));
    // semantics: the presentation, not the annotation; maction: the selected child
    assert_eq!(mml("<semantics><mi>x</mi><annotation encoding=\"application/x-tex\">y</annotation></semantics>").text, "x");
    assert_eq!(mml("<maction actiontype=\"toggle\" selection=\"2\"><mi>a</mi><mi>b</mi></maction>").text, "b");
}

#[test]
fn namespaces_prolog_and_entities() {
    let r = mml("<?xml version=\"1.0\"?><!-- from Word --><m:math xmlns:m=\"http://www.w3.org/1998/Math/MathML\"><m:mi>x</m:mi><m:mo>&le;</m:mo><m:mn>1</m:mn></m:math>");
    assert_eq!(r.text, "x ≤ 1");
    assert_eq!(mml("<mi>&alpha;</mi><mo>&times;</mo><mi>&beta;</mi><mo>&lt;</mo><mi>&infin;</mi>").text, "α × β < ∞");
    assert_eq!(mml("<mi>&#x3B1;</mi><mo>&#43;</mo><mi>b</mi>").text, "α + b");
    // the source keeps its notation in the SVG metadata
    assert!(r.svg.contains(r#"<mathra:source type="mathml">"#));
}

#[test]
fn l_ent_4_mathml_pasted_as_latex() {
    let r = convert("<math><mfrac><mn>1</mn><mn>2</mn></mfrac></math>", &Options::default()).unwrap();
    assert_eq!(r.speech, "1 over 2");
    assert!(r.applied.iter().any(|a| a.rule == "L-ENT-4"));
    assert!(r.svg.contains(r#"type="mathml""#));
    // not MathML: LaTeX that merely contains "<"
    assert!(convert(r"a < b", &Options::default()).unwrap().applied.iter().all(|a| a.rule != "L-ENT-4"));
}

#[test]
fn e_rec_10_unknown_elements_and_entities() {
    let src = "<math><mi>x</mi><mo>&foo;</mo><mblah><mi>y</mi></mblah></math>";
    let r = convert(src, &options(false)).unwrap();
    assert_eq!(r.text, "xy");
    let rules: Vec<_> = r.warnings.iter().map(|a| a.rule).collect();
    assert_eq!(rules, vec!["E-REC-10", "E-REC-10"]);
    assert!(convert(src, &options(true)).unwrap_err().to_string().contains("unknown entity"));
    // content MathML is named as such
    let e = convert("<math><apply><plus/><ci>x</ci></apply></math>", &options(true)).unwrap_err();
    assert!(e.to_string().contains("content MathML"), "{e}");
    // a missing child of mfrac is left empty
    let r = convert("<math><mfrac><mn>1</mn></mfrac></math>", &options(false)).unwrap();
    assert_eq!(r.warnings[0].rule, "E-REC-10");
}

#[test]
fn invalid_xml_and_deep_nesting_are_errors() {
    for src in ["<math><mi>x</mi><mo>+", "<math><mi>x</mo></math>", "<math><mi>x &</mi></math>"] {
        assert!(convert(src, &options(false)).is_err(), "{src}");
    }
    let deep = format!("<math>{}<mi>x</mi>{}</math>", "<mrow>".repeat(200), "</mrow>".repeat(200));
    assert!(convert(&deep, &options(false)).unwrap_err().to_string().contains("nesting"));
}

/// Found by feeding MathJax's MathML to mathra next to MathCAT.
#[test]
fn mathjax_mathml_conventions() {
    // consecutive relations in one <mo>
    let r = mml("<mo>≤≥≠</mo>");
    assert_eq!(r.tree.children().len(), 3, "{:?}", r.tree);
    assert_eq!(r.text, latex(r"\le \ge \ne").text);
    // the no-break space of <mtext> is content
    assert_eq!(mml("<mi>p</mi><mtext>&#xA0;and&#xA0;</mtext><mi>q</mi>").text.split_whitespace().collect::<Vec<_>>(), ["p", "and", "q"]);
    // a name followed by ApplyFunction is a function, even one mathra does not know
    assert!(matches!(first("<mi>rank</mi><mo>&#x2061;</mo><mi>M</mi>"), Node::Function { .. }));
    // cases with column alignment: still cases
    let n = first(r#"<mrow><mo>{</mo><mtable columnalign="left left"><mtr><mtd><mn>1</mn></mtd><mtd><mi>x</mi></mtd></mtr></mtable></mrow>"#);
    assert!(matches!(n, Node::Table { shape: TableKind::Cases, .. }), "{n:?}");
}

#[test]
fn mathjax_norm_bars_are_a_norm() {
    // MathJax writes \| as the parallel sign ∥ (U+2225); between two of them is a norm
    let n = first(r#"<mo data-mjx-texclass="ORD">&#x2225;</mo><mi>x</mi><mo data-mjx-texclass="ORD">&#x2225;</mo>"#);
    assert!(matches!(n, Node::Delimited { open: Some('‖'), close: Some('‖'), .. }), "{n:?}");
    assert_eq!(mml("<mi>a</mi><mo>∥</mo><mi>b</mi>").text, latex(r"a \parallel b").text);
}
