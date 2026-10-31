//! Voltages, currents and flows: every variant draws, in every style, where the manual puts it.

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

/// The numbers of a path, in order.
fn numbers(d: &str) -> Vec<f64> {
    d.split(|c: char| c.is_ascii_alphabetic() || c == ' ').filter_map(|t| t.parse().ok()).collect()
}

#[test]
fn every_variant_draws_in_every_style() {
    for kind in ["v", "i", "f"] {
        for variant in ["", "^", "_", ">", "<", "^>", "^<", "_>", "_<", ">^", ">_", "<^", "<_"] {
            for style in ["european voltages", "straight voltages", "american voltages", "raised voltages"] {
                for dir in ["RP", "EF"] {
                    quiet(&format!("\\draw[{style}, voltage dir={dir}] (0,0) to[R, {kind}{variant}=$x$] (2,0) to[open, {kind}{variant}=$y$] (2,2);"));
                }
            }
        }
    }
}

#[test]
fn a_current_is_an_arrow_head_on_a_lead() {
    // the label and the head: a filled triangle after the component, or before it with `>_`
    let tip_x = |opts: &str| {
        let svg = quiet(&format!(r"\draw (0,0) to[R, {opts}=$i$] (4,0);")).svg;
        let heads: Vec<Vec<f64>> = paths(&svg).iter().filter(|d| d.matches('L').count() == 2 && d.ends_with('Z')).map(|d| numbers(d)).collect();
        assert_eq!(heads.len(), 1, "{opts}: {svg}");
        heads[0][2]
    };
    let end = tip_x("i");
    let start = tip_x("i>_");
    // 4 cm is 113.4 bp: the end lead is past the middle, the start lead before it
    assert!(end > 80.0 && start < 40.0, "{end} {start}");
}

#[test]
fn the_voltage_direction_follows_the_convention() {
    // the head of a European voltage is at the start with rising-potential voltages, at the end with
    // electric-field ones (the package's default)
    let head = |dir: &str| {
        let svg = quiet(&format!(r"\draw[voltage dir={dir}] (0,0) to[R, v=$v$] (4,0);")).svg;
        let heads: Vec<Vec<f64>> = paths(&svg).iter().filter(|d| d.matches('L').count() == 2 && d.ends_with('Z')).map(|d| numbers(d)).collect();
        heads[0][2]
    };
    assert!(head("RP") < 56.0, "{}", head("RP"));
    assert!(head("EF") > 56.0, "{}", head("EF"));
    // and the document's options choose it
    let doc = |opts: &str| {
        voltik::convert(&format!(
            "\\documentclass{{standalone}}\\usepackage[{opts}]{{circuitikz}}\\begin{{document}}\\begin{{circuitikz}}\\draw (0,0) to[R, v=$v$] (4,0);\\end{{circuitikz}}\\end{{document}}"
        ))
        .unwrap()
        .svg
    };
    assert_ne!(paths(&doc("RPvoltages")), paths(&doc("EFvoltages")));
}

#[test]
fn american_voltages_are_signs() {
    let r = quiet(r"\draw[american voltages] (0,0) to[R, v=$v$] (2,0);");
    // three texts (+, - and the label), no arrow
    assert_eq!(r.svg.matches("<g transform=").count(), 3, "{}", r.svg);
    assert!(!paths(&r.svg).iter().any(|d| d.contains(" C")), "{}", r.svg);
}

#[test]
fn anchors_label_nodes_and_symbols() {
    // the anchors of a named component's voltage, current and flow, and its labels' nodes
    quiet(
        r"\draw (0,0) to[R, l=$R$, v=$v$, i=$i$, f>_=$f$, name=R] (3,0);
          \draw[red] (R-Vfrom) .. controls (R-Vcont1) and (R-Vcont2) .. (R-Vto) (R-Vlab) -- (R-Ipos) -- (R-Ifrom) -- (R-Ito)
            (R-Fpos) -- (R-Ffrom) -- (R-Fto) (Rlabel.north) -- (Rvoltage.south) -- (Rcurrent.center) -- (Rflow.center);
          \node[anchor=\ctikzgetanchor{R}{Vlab}] at (R-Vlab) {x};",
    );
    // without a value only the anchors; without symbols only the label
    let bare = quiet(r"\draw (0,0) to[R, v, name=R] (3,0); \draw (R-Vfrom) -- (R-Vto);");
    assert_eq!(bare.svg.matches("<g transform=").count(), 0);
    let only = quiet(r"\draw (0,0) to[R, v=$v$, no v symbols] (3,0);");
    assert_eq!(only.svg.matches("<g transform=").count(), 1);
    assert!(!paths(&only.svg).iter().any(|d| d.contains(" C")), "{}", only.svg);
}

#[test]
fn settings_move_them() {
    let at = |opts: &str| paths(&quiet(&format!(r"\draw (0,0) to[R, v=$v$, i=$i$, {opts}] (3,0);")).svg);
    let plain = at("");
    for opts in ["voltage shift=1", "voltage/bump b=3", "voltage/distance from node=.2", "current/distance=.2"] {
        assert_ne!(at(opts), plain, "{opts}");
    }
}

#[test]
fn stacked_labels_and_siunitx() {
    let r = quiet(r"\draw (0,0) to[R, l2=$R_1$ and \SI{4.7}{\kilo\ohm}, a=\qty{1}{\milli\ampere}, l2 halign=c] (0,3);");
    // \SI and \qty are written out (`quiet`: no "unsupported text command")
    // two labels, the first of two lines
    assert_eq!(r.svg.matches("<g transform=").count(), 2, "{}", r.svg);
}

#[test]
fn label_anchors_and_directions_as_tex_answers() {
    // what `\ctikzgetanchor` and `\ctikzgetdirection` answer for a label and an annotation, by the
    // line's way and `label/align`, as CircuiTikZ 1.8.7 does
    let cases = [
        ("straight", "(2,0)", "base/0/north/0"),
        ("straight", "(0,-2)", "mid west/0/mid east/0"),
        ("straight", "(-2,0)", "north/0/base/0"),
        ("straight", "(2,1.15)", "-60/0/120/0"),
        ("straight", "(2,-2)", "225/0/45/0"),
        ("straight", "(1,2)", "-27/0/153/0"),
        ("smart", "(0,2)", "mid east/0/mid west/0"),
        ("smart", "(2,1.15)", "mid/30/mid/30"),
        ("smart", "(-2,1)", "mid/-26/mid/-26"),
        ("rotate", "(0,-2)", "mid/270/mid/270"),
        ("rotate", "(-2,0)", "mid/0/mid/0"),
        ("rotate", "(2,-2)", "mid/315/mid/315"),
    ];
    for (align, to, expected) in cases {
        let r = quiet(&format!(
            r"\ctikzset{{label/align={align}}}\draw (0,0) to[R, name=R, l=x, a=y] ++{to};
              \node at (0,-3) {{\ctikzgetanchor{{R}}{{label}}/\ctikzgetdirection{{Rlabel}}/\ctikzgetanchor{{R}}{{annotation}}/\ctikzgetdirection{{Rannotation}}}};"
        ));
        assert!(r.description.contains(expected), "{align} {to}: {}", r.description);
    }
}

#[test]
fn named_components_are_nodes() {
    // `(R)` is the component's center, and a name prefix names it as it names nodes
    let r = quiet(
        r"\begin{scope}[name prefix=in-]\draw (0,0) to[R, name=r, i<] ++(2,1);\end{scope}
          \fill (in-r) circle (1pt) (in-r.north) circle (1pt) (in-r-Ipos) circle (1pt);
          \node at (0,-1) {\ctikzgetdirection{in-r} \ctikzgetdirection{in-r-Iarrow}};",
    );
    assert!(r.description.contains("26.56505 -153"), "{}", r.description);
}

#[test]
fn voltages_of_the_users_own() {
    // `vif queue add` runs a macro with the component's name once it is drawn (named for it when it
    // has no name), and `\ctikzactivatevoltagedirections` gives a style its variants
    let r = quiet(
        r"\newcommand\myv[4]{\draw[red] (#1-Vfrom) -- (#1-Vto) node[anchor=\ctikzgetanchor{#1}{Vlab}] {#2};}
          \ctikzset{myv/.style 2 args={v, vif queue add={myv}{#1}{#2}{}}}
          \ctikzactivatevoltagedirections{myv}
          \draw (0,0) to[R, myv={$v_1$}{}] (3,0) to[R, myv^>={$v_2$}{}, name=R2] (3,3);",
    );
    assert_eq!(r.svg.matches("stroke=\"#ff0000\"").count(), 2, "{}", r.svg);
    assert_eq!(r.svg.matches("<g transform=").count(), 2, "{}", r.svg);
}

#[test]
fn queued_macros_run_at_the_end_of_the_picture() {
    // the component's nodes are there by then, and a key's `.default` reaches the `to`
    let r = quiet(
        r"\newcommand\sign[4]{\node[#2] at (#1voltage.center) {x};}
          \ctikzset{sign v/.style={vif queue add={sign}{#1}{}{}}, sign v/.default=red}
          \draw (0,0) to[R, v=a, sign v] (3,0) to[R, name=R2, v=b, sign v=blue] (3,3);",
    );
    assert!(r.svg.contains("#ff0000") && r.svg.contains("#0000ff"), "{}", r.svg);
    // `\ctikzprocessvif` runs them earlier, and once
    let r = quiet(
        r"\newcommand\mark[4]{\coordinate (#1-seen) at (#1-Vto);\draw[red] (#1-Vfrom) -- (#1-Vto);}
          \ctikzset{mark v/.style={vif queue add={mark}{}{}{}}}
          \draw (0,0) to[R, name=R1, v=a, mark v] (3,0);
          \ctikzprocessvif
          \draw (R1-seen) -- ++(1,0);",
    );
    assert_eq!(r.svg.matches("stroke=\"#ff0000\"").count(), 1, "{}", r.svg);
}

#[test]
fn an_overridden_american_source_keeps_a_source_label() {
    // TeX: its label at 0.56 cm of the line, where a passive component's would be at 0.7
    let r = quiet(r"\ctikzset{voltage=american, bipole/override source vif=true} \draw (0,0) to[V, name=V1, v=a] (2,0); \draw (0,0) -- (V1-Vlab);");
    let d = paths(&r.svg).pop().unwrap();
    let n = numbers(&d);
    assert!((n[3] - n[1]).abs() < 0.62 * 28.45, "{d}");
}

#[test]
fn an_arrow_into_a_block() {
    let n = |body: &str| paths(&quiet(body).svg).len();
    assert_eq!(n(r"\draw (0,0) to[amp, >] (3,0);"), n(r"\draw (0,0) to[amp] (3,0);") + 1);
    // the next `to` has none, and `>=` still names the arrow tip
    assert_eq!(n(r"\draw (0,0) to[amp, >] (3,0) to[amp] (6,0);"), n(r"\draw (0,0) to[amp] (3,0) to[amp] (6,0);") + 1);
    assert_eq!(n(r"\draw[>=Stealth] (0,0) to[amp] (3,0);"), n(r"\draw (0,0) to[amp] (3,0);"));
}

#[test]
fn a_chokes_core_lines_and_a_coils_tap() {
    let n = |body: &str| paths(&quiet(body).svg).len();
    let one = n(r"\draw (0,0) to[cute choke] (3,0);");
    assert_eq!(n(r"\draw (0,0) to[cute choke, twolineschoke] (3,0);"), one + 1);
    assert_eq!(n(r"\ctikzset{twolineschoke} \draw (0,0) to[cute choke, onelinechoke] (3,0);"), one);
    assert!(quiet(r"\ctikzset{bipoles/cutechoke/cthick=2} \draw (0,0) to[cute choke] (3,0);").svg.contains("stroke-width"));
    // both coils' taps, with the even turns of `inductors/coils=6` too
    quiet(r"\draw (0,0) node[transformer, circuitikz/inductors/coils=6](T){}; \draw (T-L2.midtap) -- ++(1,0) (T-L1.midtap) -- ++(-1,0);");
}

#[test]
fn a_pole_under_transform_shape_is_as_big_as_its_node() {
    let radius = |body: &str| {
        let d = paths(&quiet(body).svg).pop().unwrap();
        let x: Vec<f64> = numbers(&d).into_iter().step_by(2).collect();
        x.iter().copied().fold(f64::MIN, f64::max) - x.iter().copied().fold(f64::MAX, f64::min)
    };
    let pole = radius(r"\begin{scope}[scale=3, transform shape] \path (0,0) to[short, -o] (1,0); \end{scope}");
    let node = radius(r"\begin{scope}[scale=3, transform shape] \path (1,0) node[ocirc]{}; \end{scope}");
    assert!((pole - node).abs() < 0.2, "{pole} {node}");
}

#[test]
fn partial_borders_and_the_body_diode() {
    let n = |body: &str| paths(&quiet(body).svg).len();
    let whole = n(r"\draw (0,0) node[npn, tr circle]{};");
    // each part drawn on its own, the left out ones gone, and the anchors of the circle
    assert_eq!(n(r"\ctikzset{transistor circle/partial borders=2211} \draw (0,0) node[npn, tr circle]{};"), whole + 3);
    assert_eq!(n(r"\ctikzset{transistor circle/partial borders=0011} \draw (0,0) node[npn, tr circle](Q){}; \draw (Q.circle top) -- (Q.circle bottom);"), whole + 2);
    let tube = n(r"\draw (0,0) node[triode]{};");
    assert_eq!(n(r"\ctikzset{tubes/partial borders=012012} \draw (0,0) node[triode]{};"), tube + 3);
    // no dots at `dot scale=0`, and the diode's color
    let diode = |o: &str| quiet(&format!(r"\draw (0,0) node[nigfete, bodydiode, {o}]{{}};")).svg;
    assert!(diode("circuitikz/transistor bodydiode/color=red").contains("#ff0000"));
    assert!(paths(&diode("circuitikz/transistor bodydiode/dot scale=0")).len() < paths(&diode("circuitikz/transistor bodydiode/dot scale=1")).len());
}

#[test]
fn the_sizes_of_what_is_measured_at_one_size() {
    let d = |body: &str| paths(&quiet(body).svg).join(" ");
    for (a, b) in [
        (r"\draw (0,0) node[gridnode]{};", r"\draw (0,0) node[gridnode, gridlines=3]{};"),
        (r"\draw (0,0) node[npn, schottky base]{};", r"\ctikzset{tripoles/schottky base size=0.1} \draw (0,0) node[npn, schottky base]{};"),
        (r"\draw (0,0) node[msrstub]{};", r"\draw (0,0) node[msrstub, circuitikz/monopoles/msrstub/radius=0]{};"),
        (r"\draw (0,0) node[proximeter]{};", r"\draw (0,0) node[proximeter, circuitikz/proximeter/hlines position=0.1]{};"),
        (r"\draw (0,0) to[R, i=f] (2,0);", r"\draw (0,0) to[R, i=f, current arrow scale=8] (2,0);"),
        (r"\draw (0,0) node[nigbt]{};", r"\ctikzset{tripoles/igbt/outer base height=0.3} \draw (0,0) node[nigbt]{};"),
        (r"\draw (0,0) node[vcc]{};", r"\ctikzset{monopoles/vcc/arrow={Latex[blue]}} \draw (0,0) node[vcc]{};"),
        (r"\draw (0,0) node[op amp]{};", r"\ctikzset{amplifiers/plus={}} \draw (0,0) node[op amp]{};"),
        (r"\draw (0,0) node[npn, schottky base]{};", r"\draw (0,0) node[npn, schottky base, no schottky base]{};"),
        (r"\draw (0,0) node[or port]{};", r"\ctikzset{logic ports draw input leads=false} \draw (0,0) node[or port]{};"),
    ] {
        assert_ne!(d(a), d(b), "{b}");
    }
}

#[test]
fn open_poles_fill_and_opacity() {
    let r = quiet(r"\ctikzset{open poles fill=red} \tikzset{open poles opacity=0.5} \draw (0,0) to[short, -o] (1,0) node[ocirc]{};");
    assert_eq!(r.svg.matches("fill=\"#ff0000\"").count(), 2, "{}", r.svg);
    assert_eq!(r.svg.matches("fill-opacity=\"0.5\"").count(), 2, "{}", r.svg);
}

#[test]
fn a_split_two_port_stays_upright() {
    // its diagonal goes from bottom left to top right whichever way the path goes
    let diagonal = |to: &str| {
        let r = quiet(&format!(r"\draw (0,0) to[twoportsplit] {to};"));
        let d = paths(&r.svg).into_iter().find(|d| !d.contains('Z') && numbers(d).len() == 4 && numbers(d)[0] != numbers(d)[2] && numbers(d)[1] != numbers(d)[3]).unwrap();
        let n = numbers(&d);
        (n[2] - n[0]) * (n[3] - n[1]) < 0.0
    };
    assert!(diagonal("(3,0)") && diagonal("(0,-3)") && diagonal("(0,3)") && diagonal("(-3,0)"));
}

#[test]
fn style_files() {
    // `\ctikzloadstyle{example}` gives `example circuit style`
    let plain = quiet(r"\draw (0,0) node[op amp] {};").svg;
    let styled = voltik::convert(r"\ctikzloadstyle{example}\begin{circuitikz}[example circuit style]\draw (0,0) node[op amp] {};\end{circuitikz}").unwrap();
    assert!(styled.warnings.is_empty(), "{:?}", styled.warnings);
    assert_ne!(plain, styled.svg);
    let romano = voltik::convert(r"\ctikzloadstyle{romano}\begin{circuitikz}[romano circuit style]\draw (0,0) to[R] (2,0);\end{circuitikz}").unwrap();
    assert!(romano.warnings.is_empty(), "{:?}", romano.warnings);
}
