//! Block diagrams and RF: every block draws on a path and as a node, with its variants (`boxed`,
//! `ideal filter`, `bare`), its anchors, texts and sizes.

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

/// Where `B.<anchor>` is from the picture's origin, in hundredths of a cm, by a red line to it.
fn at(body: &str, anchor: &str) -> (i64, i64) {
    let svg = quiet(&format!(r"{body} \draw[red] (0,0) -- (B.{anchor});")).svg;
    let d = svg.split("<path d=\"").find(|p| p.contains("#ff0000")).expect("a red path");
    let n: Vec<f64> = d[..d.find('"').unwrap()].split([' ', 'M', 'L']).filter_map(|t| t.parse().ok()).collect();
    let cm = 100.0 / 28.3465;
    (((n[2] - n[0]) * cm).round() as i64, ((n[1] - n[3]) * cm).round() as i64)
}

const PATH_BLOCKS: &[&str] = &[
    "twoport",
    "twoportsplit",
    "vco",
    "bandpass",
    "bandstop",
    "highpass",
    "lowpass",
    "highpass2",
    "lowpass2",
    "allpass",
    "vallpass",
    "bgenerator",
    "qgenerator",
    "cgenerator",
    "ngenerator",
    "adc",
    "dac",
    "dsp",
    "fft",
    "amp",
    "iamp",
    "vamp",
    "piattenuator",
    "vpiattenuator",
    "tattenuator",
    "vtattenuator",
    "phaseshifter",
    "vphaseshifter",
    "biast",
    "sinetable",
    "register",
    "detector",
    "saturation",
    "sigmoid",
    "allornothing",
    "fiber",
    "sdcdc",
    "sacdc",
    "sdcac",
    "sacac",
    "tacdc",
    "tdcac",
    "tacac",
    "lowpassp",
    "highpassp",
    "bandpassp",
    "bandstopp",
    "resonantp",
    "notchp",
    "camera",
    "tvset",
    "trx",
    "swr",
    "power",
    "mstline",
    "TL",
];

const NODE_BLOCKS: &[&str] = &[
    "mixer",
    "adder",
    "oscillator",
    "circulator",
    "wilkinson",
    "splitter",
    "genericsplitter",
    "mzm",
    "gridnode",
    "fourport",
    "coupler",
    "coupler2",
    "bareantenna",
    "dinantenna",
    "bareTXantenna",
    "bareRXantenna",
    "waves",
    "harmonics",
    "mslstub",
    "msport",
    "msrstub",
    "antenna",
    "rxantenna",
    "txantenna",
    "tlinestub",
    "match",
    "inputarrow",
];

#[test]
fn every_block_draws_on_a_path_and_as_a_node() {
    for block in PATH_BLOCKS {
        quiet(&format!(r"\draw (0,0) to[{block}, name=B, fill=yellow] (2,0) (B.north) -- (B.south east);"));
    }
    for block in ["amp", "iamp", "vamp", "vco"] {
        let plain = paths(&quiet(&format!(r"\draw (0,0) to[{block}] (2,0);")).svg);
        assert_ne!(plain, paths(&quiet(&format!(r"\draw (0,0) to[{block}, boxed] (2,0);")).svg), "{block}");
    }
    for filter in ["lowpassp", "highpassp", "bandpassp", "bandstopp", "resonantp", "notchp"] {
        let plain = paths(&quiet(&format!(r"\draw (0,0) to[{filter}] (2,0);")).svg);
        assert_ne!(plain, paths(&quiet(&format!(r"\draw (0,0) to[{filter}, ideal filter] (2,0);")).svg), "{filter}");
    }
    assert_ne!(paths(&quiet(r"\draw (0,0) to[TL] (2,0);").svg), paths(&quiet(r"\draw (0,0) to[TL, bipoles/tline/bare=true] (2,0);").svg));
    for node in NODE_BLOCKS {
        quiet(&format!(r"\draw (0,0) node[{node}, fill=yellow] (B) {{x}} (B.center) -- (B.north east);"));
    }
    for node in ["mixer", "adder", "oscillator", "circulator"] {
        for variant in ["boxed", "box only"] {
            assert_ne!(paths(&quiet(&format!(r"\node[{node}] {{}};")).svg), paths(&quiet(&format!(r"\node[{node}, {variant}] {{}};")).svg));
        }
    }
    quiet(r"\draw (0,0) node[sacdcshape] (A) {} (3,0) node[twoportsplitshape, t1=in, t2=out] (B) {} (A.right up) -- (B.left up);");
}

#[test]
fn the_anchors_of_blocks() {
    let bp = r"\draw (0,0) to[bandpass, name=B] (2,0);";
    assert_eq!(at(bp, "left up"), (51, 25));
    assert_eq!(at(&format!(r"\ctikzset{{block left anchors pos=0.8}} {bp}"), "left up"), (51, 39));
    let mixer = r"\draw (1,1) node[mixer] (B) {};";
    assert_eq!(at(mixer, "w"), (51, 100));
    assert_eq!(at(mixer, "geocenter"), (100, 100));
    let osc = r"\draw (1,1) node[oscillator] (B) {};";
    assert_eq!(at(osc, "geocenter"), (51, 100));
    assert_eq!(at(r"\draw (1,1) node[wilkinson] (B) {};", "out2"), (191, 145));
    assert_eq!(at(r"\draw (1,1) node[coupler2] (B) {};", "port1"), (9, 55));
    // lines to a round block stop at its circle
    let r = quiet(r"\draw (0,0) node[adder] (B) {}; \draw[red] (-2,0) -- (B);").svg;
    let d = r.split("<path d=\"").find(|p| p.contains("#ff0000")).unwrap();
    let n: Vec<f64> = d[..d.find('"').unwrap()].split([' ', 'M', 'L']).filter_map(|t| t.parse().ok()).collect();
    assert!(((n[2] - n[0]) / 28.3465 - 1.51).abs() < 0.02, "{n:?}");
}

#[test]
fn texts_and_sizes() {
    let plain = quiet(r"\draw (0,0) to[twoportsplit] (2,0);").svg;
    let texts = quiet(r"\draw (0,0) to[twoportsplit, t1=in, t2=out] (2,0);").svg;
    let border = quiet(r"\draw (0,0) to[twoportsplit, t1=in, t2=out, component text=border] (2,0);").svg;
    assert!(texts.matches("<use").count() > plain.matches("<use").count());
    assert_ne!(texts, border);
    let amp = |opt: &str| paths(&quiet(&format!(r"\draw (0,0) to[amp, t=LNA{opt}] (3,0);")).svg);
    assert_ne!(amp(""), amp(", bipoles/amp/width=1"));
    let wide = r"\draw (0,0) to[twoport, bipoles/twoport/width=1.5, name=B] (3,0);";
    assert_eq!(at(wide, "north east"), (255, 49));
    assert_ne!(paths(&quiet(r"\node[wilkinson] {};").svg), paths(&quiet(r"\ctikzset{tripoles/wilkinson/width=1} \node[wilkinson] {};").svg));
    assert!(quiet(r"\draw (0,0) to[lowpass, blocks/inner color=red] (2,0);").svg.contains("#ff0000"));
}
