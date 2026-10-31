//! Node-style components: each draws, has its anchors, is placed by them, and follows its class.

fn convert(body: &str) -> voltik::Output {
    voltik::convert(&format!("\\begin{{circuitikz}}{body}\\end{{circuitikz}}")).unwrap()
}

fn quiet(body: &str) -> voltik::Output {
    let r = convert(body);
    assert!(r.warnings.is_empty(), "{body}: {:?}", r.warnings);
    r
}

/// The numbers of the path drawn in red.
fn red(svg: &str) -> Vec<f64> {
    let d = svg.split("<path d=\"").find(|p| p.contains("#ff0000")).expect("a red path");
    d[..d.find('"').unwrap()].split([' ', 'M', 'L']).filter_map(|t| t.parse().ok()).collect()
}

fn viewbox(svg: &str) -> Vec<f64> {
    svg.split("viewBox=\"").nth(1).unwrap().split('"').next().unwrap().split(' ').map(|v| v.parse().unwrap()).collect()
}

#[test]
fn every_node_component_draws_with_its_anchors() {
    let names = voltik::nodes::NODES.iter().map(|n| n.name).chain(voltik::nodes::TERMINALS.iter().copied());
    for name in names {
        let r = quiet(&format!(
            r"\draw (0,0) node[{name}, fill=yellow] (N) {{x}};
              \draw (N.north) -- (N.south) -- (N.east) -- (N.west) -- (N.north east) -- (N.sw) -- (N.left) -- (N.right) -- (N.center);"
        ));
        assert!(r.svg.matches("<path").count() >= 2, "{name}: {}", r.svg);
    }
}

#[test]
fn a_node_component_is_placed_by_its_anchors() {
    // a BNC placed by its zero: its shield is straight below that point, 0.1 of 1.4 cm away
    let r = quiet(r"\node[bnc, anchor=zero] (B) at (1,1) {}; \draw[red] (B.shield) -- (1,1);");
    let n = red(&r.svg);
    assert!((n[0] - n[2]).abs() < 1e-3 && (n[1] - n[3] - 0.14 * 28.3465).abs() < 0.01, "{n:?}");
    // a ground hangs from its point; a line to it stops there
    let g = quiet(r"\draw (0,0) node[ground] (G) {}; \draw[red] (2,0) -- (G);");
    let n = red(&g.svg);
    assert!((n[2] - n[0] + 2.0 * 28.3465).abs() < 0.01, "{n:?}");
}

#[test]
fn terminals_are_filled_or_white() {
    let open = quiet(r"\draw (0,0) node[ocirc] {};").svg;
    assert!(open.contains("fill=\"#ffffff\""), "{open}");
    let filled = quiet(r"\draw (0,0) node[ocirc, fill=red] {};").svg;
    assert!(filled.contains("fill=\"#ff0000\"") && !filled.contains("fill=\"#ffffff\""), "{filled}");
    let full = quiet(r"\draw (0,0) node[circ] {};").svg;
    assert!(full.contains("fill=\"#000000\""), "{full}");
    // `nodes width` sizes them
    let small = viewbox(&quiet(r"\draw (0,0) node[circ] {};").svg);
    let big = viewbox(&quiet(r"\ctikzset{nodes width=0.1} \draw (0,0) node[circ] {};").svg);
    // (less the picture's margin, 2 bp each side)
    assert!(big[2] - 3.986 > (small[2] - 3.986) * 2.0, "{small:?} {big:?}");
}

#[test]
fn classes_scale_their_components() {
    for (class, node) in [("grounds", "ground"), ("power supplies", "vcc"), ("connectors", "bnc")] {
        let plain = viewbox(&quiet(&format!(r"\draw (0,0) node[{node}] {{}};")).svg);
        let scaled = viewbox(&quiet(&format!(r"\ctikzset{{{class}/scale=2}} \draw (0,0) node[{node}] {{}};")).svg);
        assert!(scaled[3] > plain[3] * 1.5, "{class}: {plain:?} {scaled:?}");
    }
    // and a class's scale is a setting of its own, not the picture's
    let r = quiet(r"\ctikzset{diodes/scale=3} \draw (0,0) to[R] (2,0);");
    assert!(viewbox(&r.svg)[2] < 70.0, "{:?}", viewbox(&r.svg));
}

#[test]
fn the_text_of_supplies_counts_and_of_connectors_does_not() {
    let empty = viewbox(&quiet(r"\draw (0,0) node[vcc] {};").svg);
    let text = viewbox(&quiet(r"\draw (0,0) node[vcc] {5V};").svg);
    assert!(text[3] > empty[3], "{empty:?} {text:?}");
    let plug = viewbox(&quiet(r"\draw (0,0) node[iecplugR] {};").svg);
    let named = viewbox(&quiet(r"\draw (0,0) node[iecplugR] {name};").svg);
    assert_eq!(plug, named);
}

#[test]
fn buses_crossings_and_connectors_on_paths() {
    for key in ["multiwire", "bmultiwire", "tmultiwire", "crossing", "iec connector"] {
        quiet(&format!(r"\draw (0,0) to[{key}, name=X] (2,0); \draw (X.nw) -- (X.se);"));
    }
}
