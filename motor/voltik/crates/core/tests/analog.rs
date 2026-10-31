//! The analog components' own settings: transformer sources' windings, waveforms and sizes,
//! oscilloscopes, instruments kept upright, the dc and ac symbols, wiggly fuses, microphones,
//! spark gaps, current taps, transistor circles, tunable arrows and subcircuits.

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
fn transformer_sources_draw_their_windings() {
    let plain = paths(&quiet(r"\draw (0,0) to[oosourcetrans] (2,0);").svg).len();
    for kind in ["delta", "wye", "eyw", "xdelta", "zig"] {
        let r = quiet(&format!(r"\draw (0,0) to[oosourcetrans, prim={kind}, sec={kind}, name=T] (2,0) (T.symbolprim) -- (T.symbolsec);"));
        assert!(paths(&r.svg).len() > plain, "{kind}");
        quiet(&format!(r"\draw (0,0) to[ooosource, prim={kind}, sec={kind}, tert={kind}, name=T] (2,0) (T.symboltert) -- (T.centertert);"));
        quiet(&format!(r"\draw (0,0) to[oosourceatrans, prim={kind}] (2,0);"));
    }
    // turned and scaled
    let delta = |opts: &str| paths(&quiet(&format!(r"\draw (0,0) to[oosourcetrans, prim=delta, {opts}] (2,0);")).svg);
    assert_ne!(delta(""), delta("sources/symbol/delta rot=180"));
    assert_ne!(delta(""), delta("sources/symbol/prim rot=90"));
    assert_ne!(delta(""), delta(r"sources/symbol/delta scale=2"));
    // the extra winding, its anchors, and a bigger source
    quiet(r"\draw (0,0) to[oosourcetrans, oo upper winding, upper=delta, name=a] (2,0) (a.uppertop) -- (a.topleft) -- (a.topright) -- (a.symbolupper) -- (a.centerupper);");
    quiet(r"\draw (0,0) to[oosourceatrans, oo upper winding=1, upper=wye] (2,0);");
    let large = viewbox(&quiet(r"\ctikzset{ootrans size=large} \draw (0,0) to[ooosource] (2,0);").svg);
    let small = viewbox(&quiet(r"\draw (0,0) to[ooosource] (2,0);").svg);
    assert!(large[3] > small[3]);
}

#[test]
fn sources_change_their_symbols_and_sizes() {
    let sq = |opts: &str| quiet(&format!(r"\ctikzset{{{opts}}} \draw (0,0) to[sqV] (0,2);")).svg;
    assert_ne!(sq(""), sq("sources/symbol/rotate=auto"));
    assert_ne!(sq(""), sq("sources/symbol/thickness=3"));
    // round circles as high as the source, touching its ends
    let oo = |opts: &str| paths(&quiet(&format!(r"\draw (0,0) to[voosource{opts}] (2,0);")).svg);
    assert_ne!(oo(""), oo(", bipoles/oosource/width=0.6, bipoles/oosource/height=0.3"));
    // a noise source's fill, or its hatching
    let noise = |color: &str| quiet(&format!(r"\ctikzset{{bipoles/noise sources/fillcolor={color}}} \draw (0,0) to[nV] (2,0);")).svg;
    assert!(noise("red").contains("#ff0000"));
    assert!(noise("dashed").matches(" M").count() > noise("red").matches(" M").count());
}

#[test]
fn oscilloscopes_and_upright_instruments() {
    let scope = |wave: &str| paths(&quiet(&format!(r"\ctikzset{{bipoles/oscope/waveform={wave}}} \draw (0,0) node[oscopeshape] {{}};")).svg);
    let waves: Vec<_> = ["ramps", "sin", "square", "triangle", "lissajous", "zero"].iter().map(|w| scope(w)).collect();
    for (i, a) in waves.iter().enumerate() {
        for b in &waves[i + 1..] {
            assert_ne!(a, b);
        }
    }
    assert!(scope("none").len() < waves[0].len());
    let r = convert(r"\ctikzset{bipoles/oscope/waveform=mine} \draw (0,0) node[oscopeshape] {};");
    assert!(r.warnings.iter().any(|w| w.contains("waveform mine")), "{:?}", r.warnings);
    // the grid is half transparent
    assert!(quiet(r"\draw (0,0) to[oscope] (2,0);").svg.contains("opacity"));
    // its own size, its text above it as a node, and its inputs where its size puts them
    let wide = viewbox(&quiet(r"\ctikzset{bipoles/oscope/width=1.6} \draw (0,0) node[oscopeshape, fill=green] (O) {$C$} (O.in 1) -- (O.in 2);").svg);
    let plain = viewbox(&quiet(r"\draw (0,0) node[oscopeshape] (O) {} (O.in 1) -- (O.in 2);").svg);
    assert!(wide[2] > plain[2] * 2.0);
    // turned, the screen stays upright unless `rotated instruments`
    for meter in ["oscope", "rmeterwa", "smeter"] {
        let turned = |opts: &str| paths(&quiet(&format!(r"\draw (0,0) to[{meter}{opts}] (0,2);")).svg);
        assert_ne!(turned(""), turned(", rotated instruments"), "{meter}");
    }
}

#[test]
fn symbols_and_miscellaneous_bipoles() {
    // the dc symbol's dashes, and the text above the symbols, not counted in the picture's size
    let dc = |opts: &str| quiet(&format!(r"\draw (0,0) node[dc symbol{opts}] (S) {{}} (S.left) -- (S.right);")).svg.matches('M').count();
    assert!(dc(", dc symbol segments=4") > dc(""));
    let with = viewbox(&quiet(r"\draw (0,0) node[ac symbol] {text};").svg);
    let without = viewbox(&quiet(r"\draw (0,0) node[ac symbol] {};").svg);
    assert_eq!(with, without);
    quiet(r"\ctikzset{dc symbol/width=0.2, dc symbol/height=0.1, dc symbol/relative thickness=.5} \draw (0,0) node[dc symbol] {};");
    // a wiggly fuse without poles, or with other ones as nodes of their own
    let fuse = |opts: &str| paths(&quiet(&format!(r"\draw (0,0) to[wfuse, name=F{opts}] (2,0);")).svg).len();
    assert!(fuse(", bipoles/wfuse/dots=false") < fuse(""));
    quiet(r"\ctikzset{bipoles/wfuse/shape=osquarepole} \draw (0,0) to[wfuse, name=F] (2,0) (F-left.-135) -- (F-right);");
    // a microphone's bar, a barrier's cross, a spark gap's arrows
    let mic = |k: &str| quiet(&format!(r"\ctikzset{{bipoles/mic/bar thickness={k}}} \draw (0,0) to[mic] (2,0);")).svg;
    assert_ne!(mic("1"), mic("3"));
    assert!(paths(&quiet(r"\draw (0,0) to[barrier] (2,0);").svg).iter().any(|d| d.matches('M').count() >= 2));
    let spark = |opts: &str| paths(&quiet(&format!(r"\draw (0,0) to[sparkgap{opts}] (2,0);")).svg);
    assert_ne!(spark(""), spark(", sparkgap/gap=0.15"));
    assert_ne!(spark(""), spark(", sparkgap end arrow={Kite[scale=1.5]}"));
    // a current tap's dot (xcolor's yellow is cmyk, drawn as Ghostscript draws it)
    let tap = |opts: &str| quiet(&format!(r"\draw (0,0) to[currtap, name=C{opts}] (2,0) (C.tap) -- ++(0,-1);")).svg;
    assert!(tap(", bipoles/currtap/fill=yellow, bipoles/currtap/color=red, bipoles/currtap/thickness=3").contains("#fff200"));
    assert_ne!(tap(""), tap(", bipoles/currtap/dot size=0.3"));
}

#[test]
fn transistor_circles_arrows_and_subcircuits() {
    let circle = |opts: &str| quiet(&format!(r"\ctikzset{{{opts}}} \draw (0,0) node[npn, tr circle] {{}};")).svg;
    assert!(circle("transistor circle/color=red").contains("#ff0000"));
    assert!(circle("transistor circle/dash={{4pt}{4pt}{1pt}{4pt}}").contains("dasharray"));
    // an arrow across a component or a node
    let plain = paths(&quiet(r"\draw (0,0) to[sV, name=A] (0,3) node[op amp, anchor=+] (B) {};").svg).len();
    let arrows = quiet(r"\draw (0,0) to[sV, name=A] (0,3) node[op amp, anchor=+] (B) {}; \ctikztunablearrow{1}{1.2}{30}{A} \ctikztunablearrow[red]{6}{0.8}{110}{B}");
    assert!(paths(&arrows.svg).len() >= plain + 4);
    // a subcircuit, placed by one of its anchors
    let def =
        r"\ctikzsubcircuitdef{divider}{in, out}{coordinate (#1-in) to[R, -*] ++(2,0) coordinate(#1-tmp) to[R] ++(0,-2) node[ground]{} (#1-tmp) -- ++(0.5,0) coordinate(#1-out)}";
    let r = voltik::convert(&format!(r"{def}\begin{{circuitikz}}\ctikzsubcircuitactivate{{divider}} \draw (0,0) \divider{{a}}{{}}; \draw (a-out) \divider{{b}}{{in}}; \draw (b-out) -- ++(1,0);\end{{circuitikz}}")).unwrap();
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    assert!(viewbox(&r.svg)[2] > 5.0 * 28.0);
}
