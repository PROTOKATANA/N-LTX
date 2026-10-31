//! Sources, instruments, mechanical analogies and the other bipoles: each draws, and a source's
//! value is its voltage or its current, drawn the way the manual draws a source's.

fn convert(body: &str) -> voltik::Output {
    voltik::convert(&format!("\\begin{{circuitikz}}{body}\\end{{circuitikz}}")).unwrap()
}

fn quiet(body: &str) -> voltik::Output {
    let r = convert(body);
    assert!(r.warnings.is_empty(), "{body}: {:?}", r.warnings);
    r
}

/// The `d` of every path of the SVG.
fn paths(svg: &str) -> Vec<String> {
    svg.split("<path d=\"").skip(1).map(|p| p[..p.find('"').unwrap()].to_string()).collect()
}

/// The arrow heads: filled triangles, as their numbers.
fn heads(svg: &str) -> Vec<Vec<f64>> {
    paths(svg)
        .iter()
        .filter(|d| d.matches('L').count() == 2 && d.ends_with('Z') && !d.contains(" C"))
        .map(|d| d.split(|c: char| c.is_ascii_alphabetic() || c == ' ').filter_map(|t| t.parse().ok()).collect())
        .collect()
}

#[test]
fn every_key_draws_with_its_voltage_and_current() {
    for (key, _) in voltik::components::aliases() {
        for dir in ["RP", "EF"] {
            quiet(&format!("\\draw[voltage dir={dir}] (0,0) to[{key}, v=$v$, i=$i$, fill=yellow] (2,0);"));
        }
    }
}

#[test]
fn a_sources_value_is_its_voltage_or_its_current() {
    // `V=` draws what `V, v=` does, not a label; `I=` what `I, i=` does
    assert_eq!(paths(&quiet(r"\draw (0,0) to[V=$u$] (2,0);").svg), paths(&quiet(r"\draw (0,0) to[V, v=$u$] (2,0);").svg));
    assert_eq!(paths(&quiet(r"\draw (0,0) to[I=$a$] (2,0);").svg), paths(&quiet(r"\draw (0,0) to[I, i=$a$] (2,0);").svg));
    // with a variant of the key: `I<=` points back
    assert_ne!(heads(&quiet(r"\draw (0,0) to[I=$a$] (2,0);").svg), heads(&quiet(r"\draw (0,0) to[I<=$a$] (2,0);").svg));
    // and a resistor's value stays its label: no arrow
    assert!(heads(&quiet(r"\draw (0,0) to[R=$r$] (2,0);").svg).is_empty());
}

#[test]
fn a_voltage_sources_voltage_is_a_straight_arrow_to_its_end() {
    // in both conventions, above the source, its head at the end (2 cm is 56.7 bp)
    for dir in ["RP", "EF"] {
        let svg = quiet(&format!(r"\draw[voltage dir={dir}] (0,0) to[V, v=$u$] (2,0);")).svg;
        let h = heads(&svg);
        assert_eq!(h.len(), 1, "{dir}: {svg}");
        assert!(h[0][2] > 28.3, "{dir}: {h:?}");
        assert!(!paths(&svg).iter().any(|d| d.contains(" C") && !d.ends_with('Z')), "{dir}: {svg}");
    }
    // unless it is told it is not a source
    let passive = quiet(r"\draw (0,0) to[V, bipole/is voltage=false, v=$u$] (2,0);").svg;
    assert!(paths(&passive).iter().any(|d| d.contains(" C") && !d.ends_with('Z')), "{passive}");
}

#[test]
fn instruments_text_and_symbols() {
    let plain = quiet(r"\draw (0,0) to[rmeter] (2,0);").svg;
    let text = quiet(r"\draw (0,0) to[rmeter, t=A] (2,0);").svg;
    assert_eq!(text.matches("<g transform=").count(), plain.matches("<g transform=").count() + 1, "{text}");
    // (the dc symbol's lower line on its own, for its `relative thickness`)
    for (symbol, n) in [("dc symbol", 2), ("ac symbol", 1)] {
        assert_eq!(paths(&quiet(&format!(r"\draw (0,0) node[{symbol}] {{}};")).svg).len(), n, "{symbol}");
    }
}
