//! Flip-flops and seven segment displays: the flip-flops of the package and by `flipflop def`
//! with their pins, texts, clocks and negations, and the displays by value and by segments.

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

/// Where `F.<anchor>` is from the node's point, in hundredths of a cm, by a red line between them.
fn at(body: &str, anchor: &str) -> (i64, i64) {
    let svg = quiet(&format!(r"{body} \draw[red] (0,0) -- (F.{anchor});")).svg;
    let d = svg.split("<path d=\"").find(|p| p.contains("#ff0000")).expect("a red path");
    let n: Vec<f64> = d[..d.find('"').unwrap()].split([' ', 'M', 'L']).filter_map(|t| t.parse().ok()).collect();
    let cm = 100.0 / 28.3465;
    (((n[2] - n[0]) * cm).round() as i64, ((n[1] - n[3]) * cm).round() as i64)
}

#[test]
fn every_flipflop_draws_with_its_anchors() {
    let anchors = ["pin 1", "pin 2", "pin 3", "pin 4", "pin 5", "pin 6", "up", "down", "bpin 1", "bpin 6", "bup", "bdown", "nw", "se"];
    for node in [
        "flipflop",
        "latch",
        "flipflop SR",
        "flipflop D",
        "flipflop T",
        "flipflop JK",
        "flipflop JK, dot on notQ",
        "flipflop JK, add async SR",
        "flipflop JK, add async SR, external pins width=0",
        "flipflop JK, add async SR, rotate=90",
        "flipflop JK, add async SR, rotate=90, rotated numbers",
        "flipflop, flipflop def={t1=A, t3=B, t6=Q, t4={\\ctikztextnot{Q}}, td=rst, nd=1, c2=1, n2=1}",
    ] {
        let mut body = format!(r"\draw (0,0) node[{node}] (F) {{FF}};");
        for a in anchors {
            body.push_str(&format!(r" \draw (F.{a}) -- (F.center);"));
        }
        quiet(&body);
    }
    quiet(r"\draw (0,0) node[clockwedge] (W) {CLK} (W.nw) -- (W.right) -- (W.sw);");
}

#[test]
fn the_def_draws_the_pins_it_names() {
    let f = r"\draw (0,0) node[flipflop] (F) {};";
    // the pins are a spacing apart, the first on the left on top; they stick out by a lead
    assert_eq!(at(f, "bpin 1"), (-84, 84));
    assert_eq!(at(f, "bpin 4"), (84, -84));
    assert_eq!(at(f, "pin 1"), (-112, 84));
    assert_eq!(at(f, "bup"), (0, 126));
    assert_eq!(at(r"\draw (0,0) node[flipflop, external pins width=0] (F) {};", "pin 1"), (-84, 84));
    // only the pins with a text, a clock or a negation are drawn
    let blank = paths(&quiet(f).svg).len();
    let jk = paths(&quiet(r"\draw (0,0) node[flipflop JK] (F) {};").svg).len();
    assert!(jk > blank);
    // a negation circle is a node of its own, and a later def wins
    quiet(r"\draw (0,0) node[flipflop JK, flipflop def={n5=1}] (F) {} (F-N5.east) -- ++(1,0);");
    let one = quiet(r"\draw (0,0) node[flipflop JK] {};").svg;
    let off = quiet(r"\draw (0,0) node[flipflop JK, flipflop def={c2=0}] {};").svg;
    assert!(paths(&off).len() < paths(&one).len());
    // a bigger clock wedge, a wider flip-flop
    let wedge = quiet(r"\ctikzset{multipoles/flipflop/clock wedge size=0.4} \draw (0,0) node[flipflop JK] {};").svg;
    assert_ne!(paths(&wedge), paths(&one));
    let wide = r"\ctikzset{multipoles/flipflop/width=2} \draw (0,0) node[flipflop] (F) {};";
    assert_eq!(at(wide, "bpin 1"), (-140, 84));
}

#[test]
fn seven_segments_by_value_and_by_bits() {
    let lit = |spec: &str| quiet(&format!(r"\draw (0,0) node[{spec}] {{}};")).svg.matches("#ff0000").count();
    // an 8 lights every segment, a 1 two of them, and `-` the middle one
    assert!(lit("seven segment val=8 dot none box off") > lit("seven segment val=1 dot none box off"));
    assert_eq!(lit("seven segment val=E dot none box off"), lit("seven segment bits=1001111 dot none box off"));
    assert_eq!(lit("seven segment val=- dot none box off"), lit("seven segment bits=0000001 dot none box off"));
    // a loop's `\i dot` (the space lost after `\i`)
    quiet(r"\foreach \i in {0,...,15} \path (\i,0) node[seven segment val=\i dot on box off]{};");
    // the dot and the box widen it
    let width = |spec: &str| {
        let svg = quiet(&format!(r"\draw (0,0) node[{spec}] {{}};")).svg;
        svg.split("viewBox=\"").nth(1).unwrap().split(' ').nth(2).unwrap().parse::<f64>().unwrap()
    };
    assert!(width("seven segment val=3 dot on box on") > width("seven segment val=3 dot none box on"));
    quiet(r"\draw (0,0) node[bare7seg, fill=yellow] (S) {} (S.a) -- (S.g) -- (S.dot);");
    quiet(r"\ctikzset{seven seg/color on=green, seven seg/color off=yellow!30} \path[color=red] (0,0) node[seven segment val=7 dot none box on, xslant=0.2]{};");
}
