//! Logic gates: every port draws with its anchors in its three styles, with more inputs and
//! without leads; the short keys follow `logic ports`; the gates on a path too.

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

#[test]
fn every_port_draws_with_its_inputs() {
    for style in ["american", "european", "ieeestd"] {
        for gate in ["and", "or", "nand", "nor", "xor", "xnor"] {
            for n in 2..=8 {
                quiet(&format!(r"\draw (0,0) node[{style} {gate} port, number inputs={n}] (G) {{}} (G.in 1) -- (G.in {n}) -- (G.out);"));
            }
            quiet(&format!(r"\draw (0,0) node[{style} {gate} port, no input leads] (G) {{}} (G.in 1) -- (G.out);"));
        }
        quiet(&format!(r"\draw (0,0) node[{style} not port] (G) {{}} (G.in) -- (G.out);"));
    }
    for node in ["schmitt", "invschmitt", "ieee tgate", "ieee double tgate", "notcirc", "schmitt symbol", "european blank port"] {
        quiet(&format!(r"\draw (0,0) node[{node}] {{}};"));
    }
}

#[test]
fn the_short_keys_follow_the_style() {
    let draw = |style: &str| paths(&quiet(&format!(r"\ctikzset{{logic ports={style}}} \draw (0,0) node[nand port] {{}};")).svg);
    assert_eq!(draw("american"), paths(&quiet(r"\draw (0,0) node[american nand port] {};").svg));
    assert_eq!(draw("ieee"), paths(&quiet(r"\draw (0,0) node[ieeestd nand port] {};").svg));
    assert_ne!(draw("european"), draw("american"));
}

#[test]
fn gates_on_a_path_follow_the_style() {
    for key in ["inline not", "inline buffer", "inline schmitt", "inline invschmitt", "inline tgate", "inline double tgate"] {
        quiet(&format!(r"\draw (0,0) to[{key}, n=G] (2,0) (G.in) -- (G.out);"));
    }
    let american = paths(&quiet(r"\draw (0,0) to[inline not] (2,0);").svg);
    let ieee = paths(&quiet(r"\ctikzset{logic ports=ieee} \draw (0,0) to[inline not] (2,0);").svg);
    assert_ne!(american, ieee);
}

#[test]
fn the_ports_options() {
    let draw = |body: &str| paths(&quiet(body).svg);
    // the European negation symbols and xnor, measured; `european` is European ports too
    let nand = |symbol: &str| draw(&format!(r"\ctikzset{{tripoles/european not symbol={symbol}}} \draw (0,0) node[european nand port, number inputs=3] {{}};"));
    assert_ne!(nand("triangle"), nand("circle"));
    assert_ne!(nand("circle"), nand("ieee circle"));
    assert_ne!(draw(r"\draw (0,0) node[european xnor port] {};"), draw(r"\ctikzset{european xnor style=direct} \draw (0,0) node[european xnor port] {};"));
    assert_eq!(draw(r"\begin{scope}[european] \draw (0,0) node[nand port] {}; \end{scope}"), draw(r"\draw (0,0) node[european nand port] {};"));
    // the sizes and inputs measured, and the ones not yet, with a warning
    for body in [
        r"\ctikzset{tripoles/american or port/height=1.6} \draw (0,0) node[american or port, number inputs=5] (G) {} (G.in 5) -- (G.out);",
        r"\draw (0,0) node[american xnor port, circuitikz/tripoles/american xnor port/inner=.7, circuitikz/tripoles/american xnor port/angle=40] {};",
        r"\draw (0,0) node[ieeestd or port, number inputs=4, inner inputs=2] (G) {} (G.in 4) -- (G.out);",
        r"\draw (0,0) node[ieeestd or port, number inputs=4, circuitikz/ieeestd ports/height=4] {};",
        r"\draw (0,0) node[ieeestd nor port, number inputs=32, inner inputs=2] (G) {} (G.in 32) -- (G.out);",
        r"\draw (0,0) node[ieeestd nand port, number inputs=5, no input leads] (G) {} (G.in 5) -- (G.bin 5);",
        r"\draw (0,0) node[tgate, anchor=in] {} (2,0) node[double tgate] {} (4,0) node[blank port] {} (6,0) node[blank not port] {};",
    ] {
        quiet(body);
    }
    let r = convert(r"\ctikzset{tripoles/american or port/height=1.3} \draw (0,0) node[american or port] {};");
    assert!(r.warnings.iter().any(|w| w.contains("height=1.3")), "{:?}", r.warnings);
    // `logic ports origin=center`: the same port, moved so that its point is in its middle
    let legacy = draw(r"\draw (0,0) node[and port] {} (-2,-1) -- (2,1);");
    let centered = draw(r"\ctikzset{logic ports origin=center} \draw (0,0) node[and port] {} (-2,-1) -- (2,1);");
    assert_eq!(legacy.len(), centered.len());
    assert_ne!(legacy, centered);
    // `component text=left` on a port: the text starts at its body's left
    let text = |ct: &str| quiet(&format!(r"\ctikzset{{logic ports=ieee, component text={ct}}} \draw (0,0) node[nand port] {{A1}};")).svg;
    assert_ne!(text("left"), text("center"));
}
