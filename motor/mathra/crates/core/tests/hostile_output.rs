//! Hostile input versus the output: the SVG must stay valid XML that is safe to embed,
//! whatever comes in the source.

use mathra::{convert, Options};

fn valid_svg(tex: &str) -> String {
    let r = convert(tex, &Options::default()).unwrap_or_else(|e| panic!("{tex:?}: {e}"));
    if let Err(e) = roxmltree::Document::parse(&r.svg) {
        panic!("{tex:?} produced invalid XML: {e}");
    }
    r.svg
}

#[test]
fn markup_in_source_does_not_become_markup() {
    for tex in [
        r"\text{</title><script>alert(1)</script>}",
        r"\text{]]>}",
        r"a --> b <!-- c",
        r#"\text{&amp; "aspas" 'simples'}"#,
        r"\text{<?xml version='1.0'?>}",
        r#"\color{red" onload="x}{a}"#,
    ] {
        let svg = valid_svg(tex);
        let doc = roxmltree::Document::parse(&svg).unwrap();
        assert!(doc.descendants().all(|n| n.tag_name().name() != "script"), "{tex}");
        assert!(doc.descendants().all(|n| n.attributes().all(|a| !a.name().starts_with("on"))), "{tex}");
    }
}

#[test]
fn characters_forbidden_in_xml() {
    // XML 1.0 accepts no control characters (except tab, LF, CR), nor U+FFFE/U+FFFF, not even escaped
    for tex in ["a\u{0}b", "a\u{1}b", "x\u{8}y", "x\u{1b}[31my", "a\u{7f}b", "a\u{FFFE}b", "a\u{FFFF}b", "a\rb", "a\tb"] {
        valid_svg(tex);
    }
}

#[test]
fn assorted_unicode_in_source() {
    for tex in ["😀", "x + 😀", "\u{202E}abc", "a\u{0301}", "\u{200B}x", "مرحبا", "日本語", "\u{1F600}^{\u{1F600}}"] {
        valid_svg(tex);
    }
}

#[test]
fn long_source_stays_valid() {
    let tex = "x+".repeat(20_000) + "x";
    let svg = valid_svg(&tex);
    assert!(svg.len() > tex.len());
}

#[test]
fn unique_ids_across_different_equations() {
    let a = valid_svg("x");
    let b = valid_svg("y");
    let id = |s: &str| s.split("id=\"").nth(1).unwrap().split('"').next().unwrap().to_string();
    assert_ne!(id(&a), id(&b));
    assert_eq!(id(&a), id(&valid_svg("x")), "same input, same id");
}

fn valid_svg_from(source: &str, input: mathra::Input) -> Option<String> {
    let r = convert(source, &Options { input, ..Options::default() }).ok()?;
    if let Err(e) = roxmltree::Document::parse(&r.svg) {
        panic!("{source:?} produced invalid XML: {e}");
    }
    Some(r.svg)
}

#[test]
fn markup_in_mathml_and_asciimath_does_not_become_markup() {
    use mathra::Input;
    for (source, input) in [
        ("<math><mtext>&lt;/title&gt;&lt;script&gt;alert(1)&lt;/script&gt;</mtext></math>", Input::MathMl),
        (r#"<math><mi mathcolor="red&quot; onload=&quot;x">a</mi></math>"#, Input::MathMl),
        (r#"<math><mstyle mathbackground="url(javascript:x)"><mi>a</mi></mstyle></math>"#, Input::MathMl),
        ("<math><mtext><![CDATA[</title><script>x</script>]]></mtext></math>", Input::MathMl),
        (r#""</title><script>alert(1)</script>""#, Input::AsciiMath),
        (r#"color(red" onload="x)(a)"#, Input::AsciiMath),
    ] {
        let svg = valid_svg_from(source, input).unwrap_or_else(|| panic!("{source}"));
        let doc = roxmltree::Document::parse(&svg).unwrap();
        assert!(doc.descendants().all(|n| n.tag_name().name() != "script"), "{source}");
        assert!(doc.descendants().all(|n| n.attributes().all(|a| !a.name().starts_with("on"))), "{source}");
        // the source keeps the text, escaped, in <metadata>; no attribute may carry it
        assert!(doc.descendants().all(|n| n.attributes().all(|a| !a.value().contains("javascript"))), "{source}");
    }
}

#[test]
fn mathml_entity_bombs_stay_bounded() {
    // the "billion laughs": the XML parser refuses it instead of expanding it
    let bomb = r#"<!DOCTYPE math [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;"><!ENTITY c "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;"><!ENTITY d "&c;&c;&c;&c;&c;&c;&c;&c;&c;&c;"><!ENTITY e "&d;&d;&d;&d;&d;&d;&d;&d;&d;&d;"><!ENTITY f "&e;&e;&e;&e;&e;&e;&e;&e;&e;&e;"><!ENTITY g "&f;&f;&f;&f;&f;&f;&f;&f;&f;&f;">]><math><mi>&g;</mi></math>"#;
    let start = std::time::Instant::now();
    let _ = valid_svg_from(bomb, mathra::Input::MathMl);
    assert!(start.elapsed().as_secs() < 5, "took {:?}", start.elapsed());
}
