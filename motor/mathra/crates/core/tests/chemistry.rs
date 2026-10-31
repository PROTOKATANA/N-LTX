//! mhchem's `\ce` and `\pu` (L-CHE-1): what each construction becomes in the tree, in the speech and in
//! the linear text. Every case is valid mhchem, so it must read the same in both modes, with no warnings.

use mathra::tree::{Class, FractionStyle, TextPart};
use mathra::{convert, Locale, Node, Options};

fn read(tex: &str, locale: Locale) -> mathra::Output {
    let strict = convert(tex, &Options { locale, strict: true, ..Options::default() }).unwrap_or_else(|e| panic!("{tex}: {e}"));
    let tolerant = convert(tex, &Options { locale, ..Options::default() }).unwrap();
    assert!(tolerant.warnings.is_empty(), "{tex}: {:?}", tolerant.warnings);
    assert_eq!(strict.svg, tolerant.svg, "{tex}");
    strict
}

fn en(tex: &str) -> mathra::Output {
    read(tex, Locale::En)
}

fn tree(tex: &str) -> String {
    format!("{:?}", en(tex).tree)
}

fn has(t: &str, snippets: &[&str]) {
    for snippet in snippets {
        assert!(t.contains(snippet), "missing {snippet:?} in {t}");
    }
}

/// Every node of the tree, depth first.
fn nodes(node: &Node) -> Vec<&Node> {
    let mut out = vec![node];
    let children: Vec<&Node> = match node {
        Node::Row { children } => children.iter().collect(),
        Node::Scripts { base, sup, sub } => std::iter::once(base.as_ref()).chain(sup.as_deref()).chain(sub.as_deref()).collect(),
        Node::Fraction { num, den, .. } => vec![num, den],
        Node::Arrow { above, below, .. } => above.as_deref().into_iter().chain(below.as_deref()).collect(),
        Node::Text { parts } => parts.iter().filter_map(|p| if let TextPart::Math(m) = p { Some(m) } else { None }).collect(),
        Node::Delimited { parts, .. } => parts.iter().collect(),
        Node::Quantity { unit, .. } => vec![unit],
        _ => vec![],
    };
    for c in children {
        out.extend(nodes(c));
    }
    out
}

fn arrows(tex: &str) -> Vec<Node> {
    let r = en(tex);
    nodes(&r.tree).into_iter().filter(|n| matches!(n, Node::Arrow { .. })).cloned().collect()
}

#[test]
fn l_che_1_arrow_with_a_label_above() {
    let a = arrows(r"\ce{A ->[\Delta] B}");
    let [Node::Arrow { symbol: '→', above: Some(above), below: None }] = a.as_slice() else { panic!("{a:?}") };
    assert_eq!(**above, Node::Identifier { name: "Δ".into(), variant: mathra::tree::Variant::Upright });
    assert!(en(r"\ce{A ->[\Delta] B}").applied.iter().any(|a| a.rule == "L-CHE-1"));
}

#[test]
fn l_che_1_arrow_with_labels_above_and_below() {
    let a = arrows(r"\ce{A ->[catalyst][heat] B}");
    let [Node::Arrow { above: Some(above), below: Some(below), .. }] = a.as_slice() else { panic!("{a:?}") };
    // words in labels are upright text, not a product of italic letters
    has(&format!("{above:?}"), &["Text(\"catalyst\")"]);
    has(&format!("{below:?}"), &["Text(\"heat\")"]);
    assert_eq!(en(r"\ce{A ->[catalyst][heat] B}").speech, "A right arrow (catalyst) (heat) B");
}

#[test]
fn l_che_1_equilibrium_with_a_formula_as_label() {
    let a = arrows(r"\ce{H2O <=>[K_w] H+ + OH-}");
    let [Node::Arrow { symbol: '⇌', above: Some(above), .. }] = a.as_slice() else { panic!("{a:?}") };
    has(&format!("{above:?}"), &["\"K\"", "Identifier { name: \"w\""]);
    let speech = en(r"\ce{H2O <=>[K_w] H+ + OH-}").speech;
    assert!(speech.contains("in equilibrium with (K sub w)"), "{speech}");
}

#[test]
fn l_che_1_label_with_math_between_dollars() {
    let a = arrows(r"\ce{A + B ->[$\Delta H < 0$] C}");
    let [Node::Arrow { above: Some(above), .. }] = a.as_slice() else { panic!("{a:?}") };
    has(&format!("{above:?}"), &["\"Δ\"", "'<'", "\"0\""]);
    assert!(!format!("{above:?}").contains("'$'"));
}

#[test]
fn l_che_1_every_arrow() {
    for (tex, symbol) in [("->", '→'), ("<-", '←'), ("<->", '↔'), ("<-->", '⇄'), ("<=>", '⇌'), ("<=>>", '⥂'), ("<<=>", '⥄')] {
        let r = en(&format!(r"\ce{{A {tex} B}}"));
        assert!(nodes(&r.tree).iter().any(|n| matches!(n, Node::Operator { symbol: s, class: Class::Rel } if *s == symbol)), "{tex}: {:?}", r.tree);
        // the arrow alone in \ce does not take the next token as a label
        assert_eq!(r.text.matches('[').count(), 0, "{tex}: {}", r.text);
    }
}

#[test]
fn l_che_1_hydrate_coefficient_is_not_a_subscript() {
    let t = tree(r"\ce{CuSO4.5H2O}");
    // the 5 after the dot is a coefficient, beside the dot; the 4 before it stays a subscript of O
    has(&t, &["Operator { symbol: '·', class: Ord }, Number { value: \"5\"", "sub: Some(Number { value: \"4\""]);
    assert!(!t.contains("base: Operator { symbol: '·'"), "{t}");
    assert_eq!(en(r"\ce{CuSO4.5H2O}").text, "CuSO_4·5 H_2O");
    assert_eq!(en(r"\ce{CuSO4*5H2O}").text, "CuSO_4·5 H_2O");
}

#[test]
fn l_che_1_decimal_and_fraction_coefficients() {
    let r = en(r"\ce{0.5 H2}");
    assert_eq!(r.tree.children()[0].clone().simplify().children()[0], Node::Number { value: "0.5".into(), group: false, thousands: None });
    assert_eq!(r.speech, "0.5 H sub 2");
    let r = en(r"\ce{1/2 O2 -> O}");
    let first = r.tree.children()[0].children()[0].clone();
    let Node::Fraction { style: FractionStyle::Row, rule: true, .. } = first else { panic!("{first:?}") };
    assert!(r.speech.starts_with("1 over 2 O sub 2"), "{}", r.speech);
    // in pt-BR the decimal coefficient is written with a comma
    assert_eq!(read(r"\ce{0.5 H2}", Locale::PtBr).text, "0,5 H_2");
}

#[test]
fn l_che_1_math_coefficient_between_dollars() {
    let r = en(r"\ce{$x$ H2O}");
    assert!(!r.text.contains('$'), "{}", r.text);
    assert!(nodes(&r.tree).iter().any(|n| matches!(n, Node::Identifier { name, variant: mathra::tree::Variant::Italic } if name == "x")));
    assert_eq!(r.text, "x H_2O");
    // a lone lowercase letter is a variable coefficient too
    assert_eq!(en(r"\ce{n H2O}").text, "n H_2O");
}

#[test]
fn l_che_1_bonds() {
    for (tex, bond) in [
        (r"\ce{CH3-CH3}", "−"),
        (r"\ce{CH2=CH2}", "="),
        (r"\ce{HC#CH}", "≡"),
        (r"\ce{A\bond{-}B}", "−"),
        (r"\ce{A\bond{=}B}", "="),
        (r"\ce{A\bond{#}B}", "≡"),
        (r"\ce{A\bond{3}B}", "≡"),
        (r"\ce{A\bond{~}B}", "∼"),
    ] {
        has(&tree(tex), &[&format!("Text(\"{bond}\")")]);
    }
    // a bond is not a minus, nor a congruence
    for tex in [r"\ce{HC#CH}", r"\ce{CH3-CH3}"] {
        let speech = en(tex).speech;
        assert!(!speech.contains("congruent") && !speech.contains("negative") && !speech.contains("minus"), "{tex}: {speech}");
    }
}

#[test]
fn l_che_1_isotopes_share_one_empty_base() {
    let r = en(r"\ce{^{14}_{6}C}");
    let scripts = nodes(&r.tree).into_iter().find(|n| matches!(n, Node::Scripts { .. })).cloned();
    let Some(Node::Scripts { base, sup: Some(sup), sub: Some(sub) }) = scripts else { panic!("{:?}", r.tree) };
    assert_eq!(*base, Node::row(vec![]));
    assert_eq!(*sup, Node::Number { value: "14".into(), group: false, thousands: None });
    assert_eq!(*sub, Node::Number { value: "6".into(), group: false, thousands: None });
    assert_eq!(r.text, "_6^14C");
}

#[test]
fn l_che_1_nuclear_equation() {
    let r = en(r"\ce{^{235}_{92}U + ^{1}_{0}n -> ^{141}_{56}Ba + ^{92}_{36}Kr + 3^{1}_{0}n}");
    assert_eq!(r.text, "_92^235U + _0^1n → _56^141Ba + _36^92Kr + 3 _0^1n");
    // after the coefficient 3, the isotope starts a new base instead of attaching to the space
    let t = format!("{:?}", r.tree);
    assert!(!t.contains("base: Space"), "{t}");
}

#[test]
fn l_che_1_charges_radicals_and_oxidation_states() {
    has(&tree(r"\ce{O2^{.-}}"), &["Text(\"•−\")"]);
    has(&tree(r"\ce{CH3^{.}}"), &["Text(\"•\")"]);
    has(&tree(r"\ce{Mn^{VII}}"), &["Text(\"VII\")"]);
    has(&tree(r"\ce{e-}"), &["Text(\"e\")", "Text(\"−\")"]);
    // the subscript of a charged ion still belongs to the same base
    let t = tree(r"\ce{SO4^{2-}_{(aq)}}");
    has(&t, &["sup: Some(Text { parts: [Text(\"2−\")] })", "sub: Some(Number { value: \"4\""]);
    has(&t, &["Text(\"(aq)\")"]);
}

#[test]
fn l_che_1_fraction_with_formulas_and_charges_inside() {
    let r = en(r"\ce{K_{a} = \frac{[H+][A-]}{[HA]}}");
    let Some(Node::Fraction { num, .. }) = nodes(&r.tree).into_iter().find(|n| matches!(n, Node::Fraction { .. })).cloned() else { panic!("{:?}", r.tree) };
    // H+ and A- inside the brackets are charges, not an addition and a subtraction
    let num = format!("{num:?}");
    has(&num, &["sup: Some(Text { parts: [Text(\"+\")] })", "sup: Some(Text { parts: [Text(\"−\")] })"]);
    assert!(!num.contains("class: Bin"), "{num}");
    assert_eq!(r.text, "K_a = ([H^+][A^−])/([HA])");
}

#[test]
fn l_che_1_subscript_variable_and_states() {
    has(&tree(r"\ce{NO_x}"), &["sub: Some(Identifier { name: \"x\""]);
    has(&tree(r"\ce{CO2 (g) + H2O (l)}"), &["Text(\"(g)\")", "Text(\"(l)\")"]);
}

#[test]
fn l_che_1_physical_units() {
    let r = en(r"\pu{123 kJ mol^-1}");
    let Node::Quantity { value: Some(v), .. } = &r.tree.children()[0] else { panic!("{:?}", r.tree) };
    assert_eq!(v, "123");
    assert_eq!(r.text, "123 kJ mol^(−1)");
    assert_eq!(en(r"\pu{1.5e3 m.s^-2}").text, "1.5 × 10^3 m · s^(−2)");
    assert_eq!(en(r"\pu{kg}").text, "kg");
}

#[test]
fn l_che_1_gas_and_precipitate_marks() {
    let r = en(r"\ce{CO2 ^ + AgCl v}");
    assert_eq!(r.text, "CO_2↑ + AgCl↓");
}

#[test]
fn l_che_1_ce_and_pu_inside_text() {
    // as in math, between the words of the text
    let t = tree(r"\text{water, \ce{H2O}, at \pu{20 mol}}");
    assert_eq!(t, tree(r"\text{water, $\ce{H2O}$, at $\pu{20 mol}$}"));
    has(&t, &["Text(\"water, \")", "Math(Row { children: [Scripts", "Quantity"]);
    assert_eq!(en(r"\text{water, \ce{H2O}}").text, en(r"\text{water, }\ce{H2O}").text);
}
