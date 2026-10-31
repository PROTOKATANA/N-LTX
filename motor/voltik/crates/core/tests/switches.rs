//! Switches, buttons and jumpers: every key draws, the poles are white unless filled, the jumpers
//! take their connections, and the node-style switches have their anchors.

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
fn every_switch_draws_with_its_anchors() {
    let keys: Vec<&str> = voltik::components::aliases()
        .map(|(k, _)| *k)
        .filter(|k| k.contains("switch") || k.contains("jumper") || k.contains("button") || k.contains("sw") || *k == "reed")
        .collect();
    assert!(keys.len() > 30, "{keys:?}");
    for key in keys {
        quiet(&format!("\\draw (0,0) to[{key}, l=$S$, n=S, fill=yellow] (2,0); \\draw (S.left) -- (S.right) (S.north) -- (S.south);"));
    }
    quiet(r"\draw (0,0) to[toggle switch, n=S] (2,0) (S.out 1) -- (S.out 2) -- (S.in) -- (S.mid);");
    quiet(r"\draw (0,0) to[open double solder jumper, n=J] (2,0) (J.tap) -- (J.tap down) -- (J.in);");
}

#[test]
fn poles_are_white_unless_filled() {
    let plain = quiet(r"\draw (0,0) to[cute open switch] (2,0);").svg;
    assert!(plain.contains("fill=\"#ffffff\""), "{plain}");
    let filled = quiet(r"\draw (0,0) to[cute open switch, fill=red] (2,0);").svg;
    assert!(filled.contains("fill=\"#ff0000\"") && !filled.contains("fill=\"#ffffff\""), "{filled}");
}

#[test]
fn a_three_pins_jumper_takes_its_connections() {
    let draw = |c: &str| paths(&quiet(&format!(r"\draw (0,0) to[three-pins jumper, tjumper connections={c}] (2,0);")).svg);
    assert_eq!(draw("00"), paths(&quiet(r"\draw (0,0) to[three-pins jumper] (2,0);").svg));
    assert_ne!(draw("12"), draw("00"));
    assert_ne!(draw("S1"), draw("12"));
}

#[test]
fn node_switches_have_their_anchors() {
    for node in ["spdt", "cute spdt up", "cute spdt mid", "cute spdt down", "cute spdt up arrow", "cute spdt mid arrow", "cute spdt down arrow"] {
        quiet(&format!(r"\draw (0,0) node[{node}] (S) {{}} (S.in) -- (S.out 1) -- (S.out 2) -- (S.mid);"));
    }
    quiet(r"\draw (0,0) node[proximeter] (P) {} (P.hlines nw) -- (P.hlines se);");
}

#[test]
fn rotary_switches() {
    // channels on a circle around the input, the wiper, the arrows, and the poles as nodes
    quiet(r"\draw (0,0) node[rotary switch <-=8 in 120 wiper 40] (A) {} (A.in) -- (A.out 4) (A.aout 2) -- (A.sqout 2) (A.ext north) -- (A.cout 8) (A-out 2.n) -- (A-in.s);");
    let plain = paths(&quiet(r"\draw (0,0) node[rotaryswitch] {};").svg);
    let arrows = paths(&quiet(r"\draw (0,0) node[rotary switch <->] {};").svg);
    assert!(arrows.len() > plain.len());
    let more = paths(&quiet(r"\draw (0,0) node[rotary switch=9 in 90 wiper 10] {};").svg);
    assert!(more.len() > plain.len());
    // `rotary switch` keeps the arrow in force, `rotary switch -` takes it off
    let kept = paths(&quiet(r"\ctikzset{multipoles/rotary/arrow=both} \draw (0,0) node[rotary switch] {};").svg);
    let none = paths(&quiet(r"\ctikzset{multipoles/rotary/arrow=both} \draw (0,0) node[rotary switch -] {};").svg);
    assert!(kept.len() > none.len());
}

#[test]
fn cute_switches_and_jumpers_have_their_poles_as_nodes() {
    quiet(r"\draw (0,0) node[cute spdt up] (S) {} (S-in.s) -- (S-out 1.n) -- (S-out 2.e);");
    quiet(r"\draw (0,0) to[closed jumper, name=J] (2,0) (J-in.-135) -- (J-out.45);");
    quiet(r"\draw (0,0) to[three-pins jumper, name=J] (2,0) (J-tap.s) -- ++(0,-1);");
    // their shape, height and lever thickness
    let ocirc = paths(&quiet(r"\draw (0,0) to[closed jumper] (2,0);").svg);
    let diamond = paths(&quiet(r"\draw (0,0) to[closed jumper, bipoles/jumper/shape=odiamondpole] (2,0);").svg);
    assert_ne!(ocirc, diamond);
    let low = paths(&quiet(r"\draw (0,0) node[cute spdt up] {};").svg);
    assert_ne!(low, paths(&quiet(r"\ctikzset{bipoles/cuteswitch/height=0.8} \draw (0,0) node[cute spdt up] {};").svg));
    assert_ne!(quiet(r"\draw (0,0) node[cute spdt up] {};").svg, quiet(r"\ctikzset{bipoles/cuteswitch/thickness=0.5} \draw (0,0) node[cute spdt up] {};").svg);
}
