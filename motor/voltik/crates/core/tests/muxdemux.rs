//! Multiplexers: `muxdemux` draws by `muxdemux def` with its pins and anchors, the package's
//! shapes, the labels, clocks and negations of `muxdemux label`, and the inversion marks.

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

/// Where `M.<anchor>` is from the node's point, in hundredths of a cm, by a red line between them.
fn at(body: &str, anchor: &str) -> (i64, i64) {
    let svg = quiet(&format!(r"{body} \draw[red] (0,0) -- (M.{anchor});")).svg;
    let d = svg.split("<path d=\"").find(|p| p.contains("#ff0000")).expect("a red path");
    let n: Vec<f64> = d[..d.find('"').unwrap()].split([' ', 'M', 'L']).filter_map(|t| t.parse().ok()).collect();
    let cm = 100.0 / 28.3465;
    (((n[2] - n[0]) * cm).round() as i64, ((n[1] - n[3]) * cm).round() as i64)
}

#[test]
fn every_muxdemux_draws_with_its_anchors() {
    let anchors = "top left, top, top right, bottom left, bottom, bottom right, left, right, center up, center down, inset top left, inset top, inset top right, inset bottom left, inset bottom, inset bottom right, inset left, inset right, inset center, narrow center, nw, se";
    for node in [
        "muxdemux",
        "demux",
        "one bit adder",
        "ALU",
        "muxdemux, muxdemux def={NL=7, NR=8, NT=4, inset w=1.0, inset Lh=4.0, inset Rh=0.0}",
        "muxdemux, muxdemux def={Lh=4, NL=4, Rh=3, NB=2, w=2, square pins=1}",
        "muxdemux, no input leads",
        "muxdemux, external pins width=0",
        "muxdemux, draw only left pins={2-5}, draw only bottom pins={3}",
    ] {
        let mut body = format!(r"\draw (0,0) node[{node}] (M) {{X}};");
        for a in anchors.split(", ") {
            body.push_str(&format!(r" \draw (M.{a}) -- (M.lpin 1) -- (M.blpin 1) -- (M.bpin 1) -- (M.bbpin 1);"));
        }
        quiet(&body);
    }
    for node in ["wedgeinv", "circleinv"] {
        quiet(&format!(r"\draw (0,0) node[{node}] (M) {{}} (M.base) -- (M.apex) -- (M.mid) -- (M.top);"));
    }
}

#[test]
fn the_def_sets_the_shape_and_the_pins() {
    let m = r"\draw (0,0) node[muxdemux] (M) {};";
    // 8 left pins over 8 base lengths, the first on top; the bottom ones left to right
    assert_eq!(at(m, "blpin 1"), (-84, 196));
    assert_eq!(at(m, "blpin 8"), (-84, -196));
    assert!(at(m, "bbpin 1").0 < at(m, "bbpin 3").0);
    // a lead sticks out, and the pin stays put without leads
    assert_eq!(at(m, "lpin 1"), (-112, 196));
    assert_eq!(at(r"\draw (0,0) node[muxdemux, no input leads] (M) {};", "lpin 1"), (-112, 196));
    assert_eq!(at(r"\draw (0,0) node[muxdemux, draw only left pins={2}] (M) {};", "lpin 1"), (-84, 196));
    // an odd pin of an inset is at its tip
    let inset = r"\draw (0,0) node[muxdemux, muxdemux def={NL=7, inset w=1.0, inset Lh=4.0}] (M) {};";
    assert_eq!(at(inset, "blpin 4"), (-28, 0));
    // square pins end on one line
    let square = r"\draw (0,0) node[muxdemux, muxdemux def={square pins=1}] (M) {};";
    assert_eq!(at(square, "bpin 1").1, at(square, "bpin 3").1);
    assert_ne!(at(m, "bpin 1").1, at(m, "bpin 3").1);
    // a def changes only what it says
    assert_ne!(paths(&quiet(m).svg), paths(&quiet(r"\draw (0,0) node[muxdemux, muxdemux def={Rh=2}] (M) {};").svg));
}

#[test]
fn labels_clocks_and_negations() {
    let base = r"\tikzset{chip/.style={muxdemux, muxdemux def={Lh=4, Rh=4, w=4, NL=2, NR=1, NB=1, NT=1}}}";
    let plain = quiet(&format!(r"{base} \draw (0,0) node[chip] (M) {{}};")).svg;
    let labelled = quiet(&format!(r"{base} \draw (0,0) node[chip, muxdemux label={{L1=in, R1=out, B1=b, T1=t, LU1=1, RD1=3, BL1=x, TR1=y, N=top, Si=s}}] (M) {{}};")).svg;
    assert!(labelled.matches("<use").count() > plain.matches("<use").count() + 10);
    // a local label replaces one of the style, and `0` takes a symbol away
    let symbols = quiet(&format!(r"{base} \draw (0,0) node[chip, muxdemux label={{cL1=1, nL2=1, woR1=1, wiB1=-1}}] (M) {{}} (M-nL2.west) -- ++(-0.5,0);")).svg;
    let fewer = quiet(&format!(r"{base} \draw (0,0) node[chip, muxdemux label={{cL1=1, nL2=1, woR1=1, wiB1=-1}}, muxdemux label={{cL1=0}}] (M) {{}};")).svg;
    assert!(paths(&symbols).len() > paths(&fewer).len());
    // the border labels along the sides, turned a quarter
    let side = quiet(&format!(r"{base} \draw (0,0) node[chip, muxdemux label={{E=e, Wi=w}}] {{}};")).svg;
    assert_eq!(side.matches("matrix(0 -1 1 0").count(), 2, "{side}");
    // pin labels stay upright on a turned multiplexer, unless `rotated numbers`
    let turned = |extra: &str| quiet(&format!(r"{base} \draw (0,0) node[chip, rotate=90, {extra} muxdemux label={{L1=a}}] {{}};")).svg;
    assert!(!turned("").contains("matrix(0 -1 1 0"));
    assert!(turned("rotated numbers,").contains("matrix(0 -1 1 0"));
    // a drawing on it, in pgf's commands
    let picture = r"\pgfpathmoveto{\pgfpoint{-1cm}{0cm}}\pgfpathlineto{\pgfpoint{1cm}{0cm}}\pgfusepath{draw}";
    let drawn = quiet(&format!(r"\draw (0,0) node[muxdemux, muxdemux def={{bgpicture/.code={{{picture}}}}}] {{}};")).svg;
    assert_eq!(paths(&drawn).len(), paths(&quiet(r"\draw (0,0) node[muxdemux] {};").svg).len() + 1);
}
