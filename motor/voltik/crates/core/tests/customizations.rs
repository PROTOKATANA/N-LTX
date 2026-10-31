//! The settings that change components beyond their style: sizes (`tripoles/op amp/width`), a
//! class's `fill`, `component text`, `en amp text`, `transform shape`, siunitx in any text, bipoles
//! as nodes and inside double bipoles, a transformer's coils as nodes and by `transformer L1`.

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
fn sizes_change_the_shape_in_between() {
    let width = |set: &str| viewbox(&quiet(&format!(r"\ctikzset{{{set}}} \draw (0,0) node[op amp] {{}};")).svg)[2];
    let plain = width("tripoles/op amp/width=1.7");
    assert!(width("tripoles/op amp/width=2.5") > plain + 15.0);
    // with no ports left the leads go, and the width is the body's
    let ports = paths(&quiet(r"\ctikzset{tripoles/op amp/port width=1} \draw (0,0) node[op amp] {};").svg);
    assert!(ports.len() < paths(&quiet(r"\draw (0,0) node[op amp] {};").svg).len(), "{ports:?}");
    let tall = viewbox(&quiet(r"\ctikzset{quadpoles/transformer/height=2} \draw (0,0) node[transformer] {};").svg);
    let plain = viewbox(&quiet(r"\draw (0,0) node[transformer] {};").svg);
    assert!(tall[3] > plain[3] + 10.0, "{tall:?} {plain:?}");
}

#[test]
fn a_class_fills_its_components() {
    let svg = quiet(r"\ctikzset{amplifiers/fill=red, capacitors/fill=blue} \draw (0,0) node[op amp] {} (2,0) to[C] (4,0);").svg;
    assert!(svg.contains("fill=\"#ff0000\"") && svg.contains("fill=\"#0000ff\""), "{svg}");
}

#[test]
fn texts_of_amplifiers() {
    let center = quiet(r"\draw (0,0) node[op amp] {A};").svg;
    let left = quiet(r"\ctikzset{component text=left} \draw (0,0) node[op amp] {A};").svg;
    assert_ne!(center, left);
    let own = paths(&quiet(r"\draw (0,0) node[en amp, en amp text={200}] {};").svg);
    let plain = paths(&quiet(r"\draw (0,0) node[en amp] {};").svg);
    assert!(own.len() < plain.len(), "the ▷∞ is not drawn");
}

#[test]
fn siunitx_and_scalebox_in_any_text() {
    quiet(r"\draw (0,0) node {\SI{12}{\volt}} (2,0) node {$v_1/\si{\volt}$} (4,0) node {\qty{-3}{\deci\bel}} (6,0) node {\scalebox{-1}[1]{200}};");
}

#[test]
fn transform_shape_scales_components_on_paths() {
    let plain = viewbox(&quiet(r"\draw[scale=0.5] (0,0) to[R] (4,0);").svg);
    let shaped = viewbox(&quiet(r"\draw[scale=0.5, transform shape] (0,0) to[R] (4,0);").svg);
    assert!(shaped[3] < plain[3], "{plain:?} {shaped:?}");
}

#[test]
fn bipoles_as_nodes_and_in_double_bipoles() {
    quiet(r"\draw (0,0) node[resistorshape] (R) {} (R.west) -- ++(-1,0) (R.east) -- ++(1,0);");
    quiet(r"\draw (0,0) node[oosourceshape, rotate=90, scale=0.5] {};");
    let plain = paths(&quiet(r"\draw (0,0) node[double bipole] {};").svg);
    let leds = paths(&quiet(r"\ctikzset{double bipole L=fulllediodeshape, double bipole R=emptydiodeshape, double bipole R invert} \draw (0,0) node[double bipole] {};").svg);
    assert_ne!(plain, leds);
}

#[test]
fn transformer_coils_are_nodes() {
    quiet(r"\draw (0,0) node[transformer] (T) {} (T-L1.a) -- (T-L1.b) (T-L2.a) -- (T-L2.b);");
    let plain = paths(&quiet(r"\draw (0,0) node[transformer core] {};").svg);
    let styled = paths(&quiet(r"\ctikzset{transformer L1/.style={inductors/coils=2, inductors/width=0.3}} \draw (0,0) node[transformer core] {};").svg);
    assert_ne!(plain, styled);
}
