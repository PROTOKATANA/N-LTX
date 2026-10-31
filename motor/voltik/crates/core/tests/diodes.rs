//! Diodes and such: every key draws, the short keys follow the `diode` style, and the thyristors,
//! LEDs, photodiodes and whiskered diodes take their other shapes from their settings.

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
fn every_diode_draws_in_every_style() {
    let keys: Vec<&str> = voltik::components::aliases().map(|(k, _)| *k).filter(|k| k.contains('D') || k.contains("diode") || k.contains("led")).collect();
    assert!(keys.len() > 60, "{keys:?}");
    for key in keys {
        for style in ["empty diodes", "full diodes", "stroke diodes"] {
            quiet(&format!("\\draw[{style}] (0,0) to[{key}, v=$v$, l=$x$, fill=yellow] (2,0);"));
        }
    }
}

#[test]
fn the_short_keys_follow_the_diode_style() {
    let draw = |opts: &str, key: &str| paths(&quiet(&format!(r"\draw[{opts}] (0,0) to[{key}] (2,0);")).svg);
    assert_eq!(draw("", "D"), draw("", "Do"));
    assert_eq!(draw("full diodes", "D"), draw("", "D*"));
    assert_eq!(draw("stroke diodes", "D"), draw("", "D-"));
    // and the document's options choose it
    let doc =
        voltik::convert(r"\documentclass{standalone}\usepackage[fulldiode]{circuitikz}\begin{document}\begin{circuitikz}\draw (0,0) to[D] (2,0);\end{circuitikz}\end{document}")
            .unwrap();
    assert_eq!(paths(&doc.svg), draw("", "D*"));
}

#[test]
fn settings_choose_the_other_shapes() {
    let draw = |set: &str, key: &str| paths(&quiet(&format!(r"\ctikzset{{{set}}} \draw (0,0) to[{key}] (2,0);")).svg);
    for (set, key) in [("thyristor style=compact", "Ty"), ("led arrows from cathode", "leD"), ("pd arrows to cathode", "pD"), ("diode straight whiskers", "zzD")] {
        assert_ne!(draw(set, key), draw("", key), "{set}");
    }
    // a laser's arrows stay
    assert_eq!(draw("led arrows from cathode", "lasD"), draw("", "lasD"));
    // in a `to` too
    assert_eq!(paths(&quiet(r"\draw (0,0) to[leD, led arrows from cathode] (2,0);").svg), draw("led arrows from cathode", "leD"));
}

#[test]
fn thyristors_have_a_gate_and_leds_their_room() {
    quiet(r"\draw (0,0) to[Ty, name=T] (2,0) (T.G) -- (T.gate) -- (T.anode) -- (T.cathode);");
    // a voltage above an LED goes over its arrows
    let below = viewbox(&quiet(r"\draw (0,0) to[leD, v=$v$] (2,0);").svg);
    let above = viewbox(&quiet(r"\draw (0,0) to[leD, v^=$v$] (2,0);").svg);
    assert!(above[3] > below[3], "{below:?} {above:?}");
}
