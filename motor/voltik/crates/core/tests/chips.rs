//! Chips: DIP and QFP chips draw with their pins, anchors and numbers, by their options.

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

/// Where `C.<anchor>` is from the chip's point, by a red line between them (in the SVG's units).
fn at(body: &str, anchor: &str) -> (i64, i64) {
    let svg = quiet(&format!(r"{body} \draw[red] (0,0) -- (C.{anchor});")).svg;
    let d = svg.split("<path d=\"").find(|p| p.contains("#ff0000")).expect("a red path");
    let n: Vec<f64> = d[..d.find('"').unwrap()].split([' ', 'M', 'L']).filter_map(|t| t.parse().ok()).collect();
    (((n[2] - n[0]) * 100.0).round() as i64, ((n[3] - n[1]) * 100.0).round() as i64)
}

#[test]
fn every_chip_draws_with_its_pins() {
    for chip in ["dipchip", "qfpchip"] {
        for n in [4, 8, 12, 16, 20, 44] {
            quiet(&format!(r"\draw (0,0) node[{chip}, num pins={n}] (C) {{IC}} (C.pin 1) -- (C.pin {n}) -- (C.bpin 1) -- (C.dot) -- (C.nw);"));
        }
        for options in ["hide numbers", "no topmark", "no input leads", "external pins width=0", "external pad fraction=4", "draw only pins={1, 3, 5-8}", "rotated numbers"] {
            quiet(&format!(r"\draw (0,0) node[{chip}, {options}] (C) {{}} (C.pin 1) -- (C.pin 8);"));
        }
    }
}

#[test]
fn the_pins_follow_the_options() {
    // the pin is at the lead's end, on the border without a lead, and stays put without leads
    let pin = |options: &str| at(&format!(r"\draw (0,0) node[dipchip, {options}] (C) {{}};"), "pin 1");
    let bpin = at(r"\draw (0,0) node[dipchip] (C) {};", "bpin 1");
    assert_ne!(pin(""), bpin);
    assert_eq!(pin("external pins width=0"), bpin);
    assert_eq!(pin("no input leads"), pin(""));
    assert_eq!(pin("draw only pins={2-8}"), bpin);
    // more pins, a taller chip; a wider one by its width
    let draw = |body: &str| paths(&quiet(body).svg);
    assert_ne!(draw(r"\draw (0,0) node[dipchip, hide numbers] {};"), draw(r"\draw (0,0) node[dipchip, hide numbers, num pins=12] {};"));
    assert_ne!(draw(r"\draw (0,0) node[dipchip, hide numbers] {};"), draw(r"\ctikzset{multipoles/dipchip/width=2} \draw (0,0) node[dipchip, hide numbers] {};"));
    // the numbers are texts: hidden, fewer paths
    let shown = quiet(r"\draw (0,0) node[qfpchip] {};").svg;
    let hidden = quiet(r"\draw (0,0) node[qfpchip, hide numbers] {};").svg;
    assert!(shown.matches("<use").count() > hidden.matches("<use").count());
}

#[test]
fn a_chip_is_filled_and_scaled() {
    let svg = quiet(r"\draw (0,0) node[dipchip, fill=yellow] {};").svg;
    // xcolor's yellow is cmyk, drawn as Ghostscript draws it
    assert!(svg.contains("fill=\"#fff200\""), "{svg}");
    let big = |s: &str| quiet(&format!(r"\ctikzset{{chips/scale={s}}} \draw (0,0) node[dipchip, hide numbers] {{}};")).svg;
    assert_ne!(paths(&big("1")), paths(&big("2")));
}
