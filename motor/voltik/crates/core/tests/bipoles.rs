//! Path-style components: each one draws, with its leads, labels and poles, and follows the settings
//! of the manual.

fn convert(body: &str) -> voltik::Output {
    voltik::convert(&format!("\\begin{{circuitikz}}{body}\\end{{circuitikz}}")).unwrap()
}

/// The `d` of every stroked path of the SVG.
fn paths(svg: &str) -> Vec<String> {
    svg.split("<path d=\"").skip(1).map(|p| p[..p.find('"').unwrap()].to_string()).collect()
}

fn quiet(body: &str) -> voltik::Output {
    let r = convert(body);
    assert!(r.warnings.is_empty(), "{body}: {:?}", r.warnings);
    r
}

#[test]
fn every_component_draws_without_warnings() {
    for (key, _) in voltik::components::ALIASES {
        for style in ["american", "european", "cute inductors"] {
            quiet(&format!("\\draw[{style}] (0,0) to[{key}=$x$, fill=yellow] (2,0);"));
        }
    }
}

#[test]
fn the_style_chooses_the_variable_and_sensing_components() {
    let american = paths(&quiet(r"\draw[american] (0,0) to[vR] (2,0) to[sL] (4,0);").svg);
    let european = paths(&quiet(r"\draw[european] (0,0) to[vR] (2,0) to[sL] (4,0);").svg);
    assert_ne!(american, european);
}

#[test]
fn fill_fills_the_component_and_not_its_strokes() {
    let filled = quiet(r"\draw (0,0) to[generic, fill=red] (2,0) to[C, fill=blue] (4,0);").svg;
    assert!(filled.contains("fill=\"#ff0000\""), "{filled}");
    assert!(filled.contains("fill=\"#0000ff\""), "{filled}");
    // the American resistor is not fillable, the solid parts keep the stroke's color
    let plain = quiet(r"\draw (0,0) to[R, fill=red] (2,0) to[ageneric] (4,0);").svg;
    assert!(!plain.contains("fill=\"#ff0000\""), "{plain}");
    assert!(plain.contains("fill=\"#000000\""), "{plain}");
}

#[test]
fn named_components_have_anchors() {
    let r = quiet(r"\draw (0,0) to[pR, name=P] (2,0); \draw[red] (P.wiper) -- (P.tip);");
    // the wiper is straight above its tip, in the middle of the potentiometer
    let d = r.svg.split("<path d=\"").find(|p| p.contains("stroke=\"#ff0000\"")).expect("the wire between the anchors");
    let n: Vec<f64> = d[..d.find('"').unwrap()].split(['M', 'L', ' ']).filter_map(|t| t.parse().ok()).collect();
    assert_eq!(n.len(), 4, "{d}");
    assert!((n[0] - n[2]).abs() < 1e-3 && n[1] < n[3], "{n:?}");
    // the others answer too, and a component without a name has none
    quiet(r"\draw (0,0) to[sL, name=S] (2,0); \draw (S.label) -- (S.midtap) -- (S.core west) -- (S.ur dot) -- (S.north east);");
    assert!(!convert(r"\draw (0,0) to[pR] (2,0); \draw (P.wiper) -- (0,1);").warnings.is_empty());
}

#[test]
fn poles_by_shorthand_and_bipole_nodes() {
    for poles in ["d-d", "o-d", "*-d", "bipole nodes={squarepole}{osquarepole}", "bipole nodes={odiamondpole, fill=red}{circ}"] {
        let r = quiet(&format!(r"\draw (0,0) to[R, {poles}] (2,0);"));
        assert!(r.svg.matches("<path").count() >= 4, "{poles}: {}", r.svg);
    }
}

#[test]
fn class_settings_in_a_to_and_ctikzset_styles() {
    let wide = paths(&quiet(r"\draw (0,0) to[R, resistors/width=1.6] (4,0);").svg);
    let set = paths(&quiet(r"\ctikzset{resistors/width=1.6} \draw (0,0) to[R] (4,0);").svg);
    let styled = paths(&quiet(r"\ctikzset{wide/.style={resistors/width=1.6}} \draw (0,0) to[R, wide] (4,0);").svg);
    assert_eq!(wide, set);
    assert_eq!(wide, styled);
    assert_ne!(wide, paths(&quiet(r"\draw (0,0) to[R] (4,0);").svg));
}

#[test]
fn a_component_is_its_outline_and_two_leads() {
    let r = quiet(r"\draw (0,0) to[R] (2,0);");
    let p = paths(&r.svg);
    assert_eq!(p.len(), 2, "{p:?}");
    // the zig-zag: a start, six teeth and an end
    assert!(p.iter().any(|d| d.matches(" L").count() == 7), "{p:?}");
    // the leads: two segments
    assert!(p.iter().any(|d| d.matches('M').count() == 2 && d.matches('L').count() == 2), "{p:?}");
}

#[test]
fn short_and_open_have_no_outline() {
    assert_eq!(paths(&quiet(r"\draw (0,0) to[short] (2,0);").svg).len(), 1);
    // an open draws nothing at all but its poles
    let open = quiet(r"\draw (0,0) to[open, o-o] (2,0);");
    assert_eq!(paths(&open.svg).iter().filter(|d| d.contains(" L")).count(), 0);
}

#[test]
fn the_style_chooses_the_resistor() {
    let american = paths(&quiet(r"\draw (0,0) to[R] (2,0);").svg);
    let european = paths(&quiet(r"\ctikzset{resistor=european} \draw (0,0) to[R] (2,0);").svg);
    let styled = paths(&convert(r"\draw[european] (0,0) to[R] (2,0);").svg);
    assert_ne!(american, european);
    assert_eq!(european, styled);
    assert!(european.iter().any(|d| d.ends_with('Z')), "{european:?}");
}

#[test]
fn settings_change_the_shape() {
    let teeth = |body: &str| paths(&quiet(body).svg).iter().map(|d| d.matches(" L").count()).max().unwrap();
    assert_eq!(teeth(r"\draw (0,0) to[R] (2,0);"), 7);
    assert_eq!(teeth(r"\ctikzset{resistors/zigs=6} \draw (0,0) to[R] (2,0);"), 13);
}

#[test]
fn labels_and_units() {
    let r = quiet(r"\draw (0,0) to[R=$R_1$] (2,0) to[C, l_=$C$, a=1<\micro\farad>] (2,-2);");
    // three labels: R_1, C and the annotation
    assert_eq!(r.svg.matches("<g transform=").count(), 3, "{}", r.svg);
}

#[test]
fn poles_are_circles_over_the_wire() {
    let r = quiet(r"\draw (0,0) to[R, o-*] (2,0);");
    let p = paths(&r.svg);
    assert_eq!(p.iter().filter(|d| d.contains(" C")).count(), 2, "{p:?}");
    assert!(r.svg.contains("fill=\"#ffffff\""));
}

#[test]
fn mirror_and_invert_keep_the_leads_outside() {
    for opts in ["", ", mirror", ", invert", ", mirror, invert"] {
        let r = quiet(&format!(r"\draw (0,0) to[R{opts}] (0,2);"));
        let leads = paths(&r.svg).into_iter().find(|d| d.matches('M').count() == 2).unwrap();
        // two vertical segments that do not overlap: the first ends below where the second starts
        let ys: Vec<f64> = leads.split(['M', 'L']).filter_map(|s| s.split_whitespace().nth(1)?.parse().ok()).collect();
        assert!(ys[1] > ys[2], "{opts}: {leads}");
    }
}

#[test]
fn components_work_in_a_plain_tikzpicture() {
    let r = voltik::convert(r"\begin{tikzpicture}\draw (0,0) to[L=$L$] (2,0);\end{tikzpicture}").unwrap();
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
}
