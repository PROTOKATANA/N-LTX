//! Amplifiers, transformers and motors: each node draws with its anchors, an amplifier with its `+`
//! and `-` (`noinv input up` swaps them), a transformer with the inductors' style.

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
fn every_amplifier_draws_with_its_anchors() {
    for amp in ["op amp", "fd op amp", "en amp", "gm amp", "inst amp", "inst amp ra", "fd inst amp", "plain amp"] {
        quiet(&format!(r"\draw (0,0) node[{amp}] (A) {{$A$}} (A.+) -- (A.-) -- (A.out) -- (A.up) -- (A.down);"));
    }
    for amp in ["plain mono amp", "plain gm mono amp", "buffer"] {
        quiet(&format!(r"\draw (0,0) node[{amp}] (A) {{}} (A.in) -- (A.out) -- (A.bin) -- (A.bout);"));
    }
    quiet(r"\draw (0,0) node[fd op amp] (A) {} (A.out +) -- (A.out -);");
    quiet(r"\draw (0,0) node[inst amp ra] (A) {} (A.ra+) -- (A.ra-) -- (A.refv up);");
}

#[test]
fn the_signs_are_drawn_and_swap() {
    let plain = paths(&quiet(r"\draw (0,0) node[op amp] {};").svg);
    let swapped = paths(&quiet(r"\draw (0,0) node[op amp, noinv input up] {};").svg);
    // the triangle and its leads, the minus and the plus
    assert!(plain.len() >= 4, "{plain:?}");
    assert_ne!(plain, swapped);
    // the anchors swap with them
    let at = |opts: &str| paths(&quiet(&format!(r"\draw (0,0) node[{opts}] (A) {{}}; \draw[red] (A.+) -- (0,0);")).svg).last().cloned();
    assert_ne!(at("op amp"), at("op amp, noinv input up"));
}

#[test]
fn transformers_follow_the_inductors_and_quadpoles_style() {
    let draw = |pre: &str| paths(&quiet(&format!(r"{pre} \draw (0,0) node[transformer] (T) {{}} (T.A1) -- (T.B2) (T.inner dot A1) -- (T.outer dot B2);")).svg);
    let cute = draw("");
    assert_ne!(draw(r"\ctikzset{inductor=american}"), cute);
    assert_ne!(draw(r"\ctikzset{inductor=european}"), cute);
    assert_ne!(draw(r"\ctikzset{quadpoles style=inline}"), cute);
    for node in ["transformer core", "gyrator", "double bipole"] {
        quiet(&format!(r"\draw (0,0) node[{node}] (T) {{}} (T.A1) -- (T.A2) -- (T.B1) -- (T.B2);"));
    }
}

#[test]
fn electro_mechanical_devices() {
    // its circle is white unless filled, over its brushes
    assert!(quiet(r"\draw (0,0) node[elmech] (M) {M} (M.top) -- (M.bottom);").svg.contains("fill=\"#ffffff\""));
    quiet(r"\draw (0,0) to[Telmech=M, n=motor] (2,0) (motor.top) -- ++(0,1);");
}
