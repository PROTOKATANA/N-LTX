//! The circuit libraries: the logic gates and electrical shapes, and the symbols that `circuit ee
//! IEC`, `circuit logic US`… put on nodes and on `to` paths, with their labels and annotations.

use tikra::canvas::Item;
use tikra::scene::Segment;
use tikra::PT;

/// What the engine draws for a picture body; no warnings.
fn items(options: &str, body: &str) -> Vec<Item> {
    let (canvas, warnings) = tikra::engine::run_document(&format!("\\begin{{tikzpicture}}[{options}]{body}\\end{{tikzpicture}}")).unwrap();
    assert!(warnings.is_empty(), "{body}: {warnings:?}");
    canvas.items
}

fn paths(options: &str, body: &str) -> Vec<Vec<Segment>> {
    items(options, body).into_iter().filter_map(|i| if let Item::Path { segments, .. } = i { Some(segments) } else { None }).collect()
}

fn texts(options: &str, body: &str) -> usize {
    items(options, body).iter().filter(|i| matches!(i, Item::Text { .. })).count()
}

#[test]
fn gates_have_their_inputs_and_output() {
    for shape in ["and gate US", "nand gate US", "or gate US", "nor gate US", "and gate CDH", "nand gate CDH", "and gate IEC", "nand gate IEC", "or gate IEC", "nor gate IEC"] {
        let body =
            format!("\\node[shape={shape}, draw, logic gate inputs=nin] (g) {{}}; \\draw (g.input 1) -- (g.input 3) -- (g.output) -- (g.north east) -- (g.30) -- (g.mid west);");
        assert!(paths("", &body).len() >= 2, "{shape}");
    }
    for shape in ["xor gate US", "xnor gate US", "xor gate IEC", "xnor gate IEC"] {
        paths("", &format!("\\node[shape={shape}, draw] (g) {{}}; \\draw (g.input 1) -- (g.input 2) -- (g.output) -- (g.south west);"));
    }
    for shape in ["not gate US", "buffer gate US", "not gate IEC", "buffer gate IEC"] {
        paths("", &format!("\\node[shape={shape}, draw] (g) {{}}; \\draw (g.input) -- (g.output) -- (g.north);"));
    }
}

#[test]
fn an_and_gate_is_as_pgf_draws_it() {
    // an empty and gate US with two inputs: its half side is 3 · (2pt + .4pt), its outer sep .2pt
    let p = paths("", r"\node[shape=and gate US, draw] (g) {}; \draw (g.output) -- (g.input 1);");
    let h = 3.0 * 2.4 * PT;
    match (p[1][0], p[1][1]) {
        (Segment::MoveTo(o), Segment::LineTo(i)) => {
            let d = o.minus(i);
            assert!((d.x - (h / 3.0 + h + 0.2 * PT + h / 6.0 + h + 0.2 * PT)).abs() < 1e-6 && (d.y + h / 3.0).abs() < 1e-6, "{d:?}");
        }
        ref s => panic!("{s:?}"),
    }
    // an IEC gate writes its symbol
    assert_eq!(texts("", r"\node[and gate IEC, draw] {};"), 1);
}

#[test]
fn electrical_shapes_end_at_their_input_and_output() {
    for shape in [
        "rectangle ee",
        "circle ee",
        "var resistor IEC",
        "inductor IEC",
        "capacitor IEC",
        "ground IEC",
        "battery IEC",
        "breakdown diode IEC",
        "generic circle IEC",
        "generic diode IEC",
        "make contact IEC",
        "var make contact IEC",
        "break contact IEC",
        "direction ee",
    ] {
        let p = paths("", &format!("\\node[shape={shape}, draw, minimum width=1cm, minimum height=5mm] (s) {{}}; \\draw (s.input) -- (s.output);"));
        let Some(&[Segment::MoveTo(a), Segment::LineTo(b)]) = p.last().map(|v| v.as_slice()) else { panic!("{shape}: {p:?}") };
        assert!(b.x > a.x && (a.y - b.y).abs() < 1e-9, "{shape}: {a:?} {b:?}");
    }
}

#[test]
fn a_symbol_on_a_to_cuts_the_line() {
    let p = paths("circuit ee IEC", r"\draw (0,0) to [resistor] (3,0);");
    // the resistor, then the line from the start to its west and from its east to the end
    let line = p.last().unwrap();
    let w = 4.0 * 7.0 * PT / 2.0 + 0.2 * PT;
    match line.as_slice() {
        [Segment::MoveTo(a), Segment::LineTo(b), Segment::MoveTo(c), Segment::LineTo(d)] => {
            assert!((d.x - a.x - 3.0).abs() < 1e-9, "{line:?}");
            assert!((b.x - a.x - (1.5 - w)).abs() < 1e-5 && (c.x - a.x - (1.5 + w)).abs() < 1e-5, "{line:?}");
        }
        _ => panic!("{line:?}"),
    }
    // two symbols on one line, a symbol turned along it
    let p = paths("circuit ee IEC", r"\draw (0,0) to [capacitor={near start}, inductor={near end}] (3,3);");
    assert_eq!(p.last().unwrap().iter().filter(|s| matches!(s, Segment::MoveTo(_))).count(), 3);
}

#[test]
fn symbols_on_nodes_with_labels_and_annotations() {
    assert_eq!(texts("circuit ee IEC", r"\node[resistor, ohm=5] {};"), 1);
    assert_eq!(texts("circuit ee IEC", r"\draw (0,0) to [resistor={info=$R$, info'=$5\Omega$}] (2,0);"), 2);
    assert_eq!(texts("circuit ee IEC", r"\node [amperemeter] {};"), 1);
    // light emitting: two arrows next to the diode, with their tips
    let n = paths("circuit ee IEC", r"\node[diode] {};").len();
    let m = paths("circuit ee IEC", r"\node[diode, light emitting] {};").len();
    assert!(m >= n + 4, "{n} {m}");
    // a symbol of one's own
    paths("circuit declare symbol=foo, set foo graphic={draw,shape=rectangle,minimum size=5mm}", r"\node [foo] {}; \node [foo={red}] at (1,0) {};");
}

#[test]
fn logic_circuits_size_their_gates() {
    // circuit logic US: as high as 2.5 circuit symbol units
    let p = paths("circuit logic US", r"\node [and gate] (a) {}; \draw (a.north) -- (a.south);");
    match p.last().unwrap().as_slice() {
        [Segment::MoveTo(a), Segment::LineTo(b)] => assert!((a.y - b.y - (2.5 * 7.0 + 0.4) * PT).abs() < 1e-6, "{a:?} {b:?}"),
        s => panic!("{s:?}"),
    }
    paths("circuit logic IEC", r"\node [nor gate, inputs=nni] (a) {}; \draw (a.input 3) -- (a.output);");
    paths("circuit logic CDH, tiny circuit symbols", r"\node [nand gate] (a) {};");
}
