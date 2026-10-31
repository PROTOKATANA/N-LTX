//! Transistors and tubes: each node draws with its anchors and its variants by its options, on a path too,
//! and a depletion channel is dark unless filled.

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

fn viewbox(svg: &str) -> Vec<f64> {
    svg.split("viewBox=\"").nth(1).unwrap().split('"').next().unwrap().split(' ').map(|v| v.parse().unwrap()).collect()
}

#[test]
fn every_transistor_draws_with_its_terminals() {
    let shapes: Vec<&str> = voltik::nodes::transistor_shapes().collect();
    assert!(shapes.len() > 25, "{shapes:?}");
    for shape in shapes {
        let (a, b) = if shape.contains("jt") || shape.contains("pn") { ("B", "C") } else { ("G", "D") };
        quiet(&format!(r"\draw (0,0) node[{shape}] (Q) {{$Q$}} (Q.{a}) -- (Q.{b}) -- (Q.text);"));
    }
}

#[test]
fn options_choose_the_variant() {
    let draw = |opts: &str| paths(&quiet(&format!(r"\draw (0,0) node[{opts}] {{}};")).svg);
    for (base, opts) in [("npn", "npn, bodydiode"), ("npn", "npn, tr circle"), ("nmos", "nmos, bulk"), ("nigfete", "nigfete, doublegate"), ("bjtnpn", "bjtnpn, emitters=2")] {
        assert_ne!(draw(opts), draw(base), "{opts}");
    }
    // one not measured is drawn plain, with a warning
    let r = convert(r"\draw (0,0) node[npn, bulk, fdsoi] {};");
    assert_eq!(r.warnings.len(), 1, "{:?}", r.warnings);
}

#[test]
fn the_text_is_right_of_it_and_does_not_count() {
    let plain = viewbox(&quiet(r"\draw (0,0) node[npn] {};").svg);
    let text = viewbox(&quiet(r"\draw (0,0) node[npn] {$Q_1$};").svg);
    assert_eq!(plain, text);
}

#[test]
fn a_depletion_channel_is_dark_unless_filled() {
    let dark = quiet(r"\draw (0,0) node[nmosd] {};").svg;
    assert!(dark.contains("fill=\"#000000\""), "{dark}");
    let red = quiet(r"\draw (0,0) node[nmosd, fill=red] {};").svg;
    assert!(red.contains("fill=\"#ff0000\""), "{red}");
}

#[test]
fn transistors_on_a_path() {
    quiet(r"\draw (0,0) to[Tnmos=$M_1$, n=M] (0,-2) (M.G) -- ++(-1,0) (M.D) -- (M.S);");
    quiet(r"\draw (0,0) to[Tnpn, n=Q] (2,0) (Q.B) -- ++(0,1);");
    // inverted, its gate goes to the other side
    assert_ne!(paths(&quiet(r"\draw (0,0) to[Tnmos] (0,-2);").svg), paths(&quiet(r"\draw (0,0) to[Tnmos, invert] (0,-2);").svg));
}

#[test]
fn tubes_draw_with_their_anchors_and_options() {
    quiet(r"\draw (0,0) node[triode] (T) {} (T.anode) -- (T.control) -- (T.cathode);");
    quiet(r"\draw (0,0) node[pentode] (T) {} (T.screen) -- (T.suppressor);");
    quiet(r"\draw (0,0) node[magnetron] (M) {} (M.cathode1) -- (M.cathode2) -- (M.anode);");
    quiet(r"\draw (0,0) node[dynode] (D) {} (D.arc) -- (D.top right);");
    let draw = |opts: &str| paths(&quiet(&format!(r"\draw (0,0) node[{opts}] {{}};")).svg);
    for (base, opts) in [("diodetube", "diodetube, filament"), ("diodetube", "diodetube, fullcathode"), ("matube", "matube, nixieanode, anodedot, nogrid")] {
        assert_ne!(draw(opts), draw(base), "{opts}");
    }
}

#[test]
fn transistor_texts_and_options() {
    let text = |opts: &str| quiet(&format!(r"\draw (0,0) node[npn{opts}] {{$Q$}};")).svg;
    let plain = text("");
    for opts in [", component text=up", ", component text=down", ", legacy transistors text"] {
        assert_ne!(text(opts), plain, "{opts}");
    }
    // the manual's text helpers
    quiet(r"\draw (0,0) node[npn, xscale=-1] {\ctikzflipx{T1}} (2,0) node[nmos] {\killdepth{q1}} (4,0) node {\rotatebox{90}{x} \raisebox{1pt}[2pt][0pt]{y}};");
    let draw = |pre: &str, node: &str| paths(&quiet(&format!(r"{pre} \draw (0,0) node[{node}] {{}};")).svg);
    assert_ne!(draw("", "nmos, ferroel gate"), draw("", "nmos"));
    assert_eq!(draw(r"\ctikzset{ferroel gate}", "npn"), draw("", "npn"), "an option a shape does not have changes nothing");
    assert_ne!(draw(r"\ctikzset{transistors/arrow pos=end}", "npn"), draw("", "npn"));
    assert_ne!(draw(r"\ctikzset{transistors/jfet gate height=0.5}", "njfet"), draw("", "njfet"));
    assert_ne!(draw(r"\ctikzset{transistor circle/scale circle radius=1.4}", "npn, tr circle"), draw("", "npn, tr circle"));
    assert_ne!(draw(r"\ctikzset{tripoles/mos style/arrows}", "nmos"), draw("", "nmos"));
}

#[test]
fn tubes_sizes_and_anodes() {
    let draw = |pre: &str, node: &str| paths(&quiet(&format!(r"{pre} \draw (0,0) node[{node}] (T) {{}};")).svg);
    let triode = draw("", "triode");
    for pre in [r"\ctikzset{tubes/width=1.2}", r"\ctikzset{tubes/width=2, tubes/height=1}", r"\ctikzset{tubes/thickness=4}"] {
        assert_ne!(draw(pre, "triode"), triode, "{pre}");
    }
    assert_ne!(draw(r"\ctikzset{tubes/filament angle=40, tubes/filament distance=0.2}", "triode, filament"), draw("", "triode, filament"));
    quiet(r"\ctikzset{tubes/anodes=10, tubes/width=3.6, tubes/height=1} \draw (0,0) node[matube, filament, nocathode] (A) {} (A.anode 10) -- (A.anode 1);");
    assert_ne!(draw(r"\ctikzset{monopoles/dynode/arc pos=0.7, monopoles/dynode/top width=0.5}", "dynode"), draw("", "dynode"));
}
