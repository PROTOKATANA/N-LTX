//! The manual's options that change how a component looks: each is known, and changes what it draws.

fn convert(body: &str) -> voltik::Output {
    let r = voltik::convert(&format!("\\begin{{circuitikz}}{body}\\end{{circuitikz}}")).unwrap();
    assert!(r.warnings.is_empty(), "{body}: {:?}", r.warnings);
    r
}

/// What is drawn: the SVG after its metadata, which holds the source.
fn drawing(body: &str) -> String {
    let svg = convert(body).svg;
    svg[svg.find("</metadata>").unwrap()..].to_string()
}

/// `with` draws something else than `without`, and without warnings.
fn differs(without: &str, with: &str) {
    assert_ne!(drawing(without), drawing(with), "{with}");
}

#[test]
fn the_transistors_styles() {
    let nmos = r"\draw (0,0) node[nmos]{};";
    let pmos = r"\draw (0,0) node[pmos]{};";
    differs(pmos, &format!(r"\ctikzset{{tripoles/mos style=arrows}}{pmos}"));
    differs(pmos, &format!(r"\ctikzset{{tripoles/pmos style/nocircle}}{pmos}"));
    differs(&format!(r"\ctikzset{{tripoles/mos style/arrows}}{pmos}"), &format!(r"\ctikzset{{tripoles/mos style/arrows, tripoles/pmos style/emptycircle}}{pmos}"));
    differs(&format!(r"\ctikzset{{tripoles/mos style/arrows}}{nmos}"), &format!(r"\ctikzset{{tripoles/mos style/arrows, transistors/arrow pos=end}}{nmos}"));
    differs(nmos, &format!(r"\ctikzset{{transistors/gate thickness=2}}{nmos}"));
    differs(nmos, &format!(r"\ctikzset{{transistors/rounded caps=true}}{nmos}"));
    differs(nmos, r"\draw (0,0) node[nmos, tr gap fill=green]{};");
    differs(r"\draw (0,0) node[hemt]{};", r"\draw (0,0) node[hemt, circuitikz/tripoles/hemt/split gate=true]{};");
    differs(r"\draw (0,0) node[GaN hemt]{};", r"\draw (0,0) node[GaN hemt, circuitikz/tripoles/hemt/split gate=false]{};");
    differs(r"\draw (0,0) node[nmosd]{};", r"\ctikzset{tripoles/nmosd/depletion color=gray}\draw (0,0) node[nmosd]{};");
    differs(r"\draw (0,0) node[nigfete, solderdot]{};", r"\ctikzset{transistor solderdot scale=1}\draw (0,0) node[nigfete, solderdot]{};");
}

#[test]
fn a_circle_only_on_what_has_one() {
    // (on an NMOS or a BJT the PMOS's circle changes nothing)
    assert_eq!(drawing(r"\draw (0,0) node[npn]{};"), drawing(r"\ctikzset{tripoles/pmos style/nocircle}\draw (0,0) node[npn]{};"));
}

#[test]
fn the_arrows() {
    differs(r"\draw (0,0) to[leD] (2,0);", r"\ctikzset{opto end arrow={Triangle}}\draw (0,0) to[leD] (2,0);");
    differs(r"\draw (0,0) to[leD] (2,0);", r"\ctikzset{opto arrows/color=red}\draw (0,0) to[leD] (2,0);");
    differs(r"\draw (0,0) to[cogsw] (2,0);", r"\draw (0,0) to[cogsw, switch end arrow={Bar}] (2,0);");
    differs(r"\draw (0,0) to[cogsw] (2,0);", r"\ctikzset{switch arrows/dash={{1pt}{1pt}}}\draw (0,0) to[cogsw] (2,0);");
    differs(r"\draw (0,0) to[vR] (2,0);", r"\draw (0,0) to[vR, tunable start arrow={Bar}] (2,0);");
    differs(r"\draw (0,0) to[pR] (2,0);", r"\ctikzset{wiper end arrow={Kite[open]}}\draw (0,0) to[pR] (2,0);");
    differs(r"\draw (0,0) to[GTO] (2,0);", r"\ctikzset{gto gate end arrow=latexslim}\draw (0,0) to[GTO] (2,0);");
    differs(r"\draw (0,0) node[npn, photo]{};", r"\ctikzset{opto end arrow={Triangle}}\draw (0,0) node[npn, photo]{};");
    differs(r"\draw (0,0) node[cute spdt up arrow]{};", r"\ctikzset{switch end arrow={Triangle}}\draw (0,0) node[cute spdt up arrow]{};");
    differs(r"\draw (0,0) node[rotary switch ->]{};", r"\ctikzset{switch end arrow={Triangle}}\draw (0,0) node[rotary switch ->]{};");
}

#[test]
fn a_styled_arrow_does_not_stay_for_the_next_node() {
    let styled = r"\begin{scope}\ctikzset{opto end arrow={Triangle}}\draw (5,0) node[npn, photo]{};\end{scope}";
    let count = |body: &str| drawing(body).matches("<path").count();
    assert_eq!(count(&format!(r"{styled}\draw (0,0) node[npn]{{}};")), count(styled) + count(r"\draw (0,0) node[npn]{};"));
}

#[test]
fn the_sources_and_meters() {
    let am = r"\begin{scope}[american]\draw (0,0) to[V] (2,0);\end{scope}";
    differs(r"\draw (0,0) to[sI] (2,0);", r"\ctikzset{bipoles/isourcesin/angle=80}\draw (0,0) to[sI] (2,0);");
    differs(r"\draw (0,0) to[dcisource] (2,0);", r"\ctikzset{bipoles/dcisource/angle=45}\draw (0,0) to[dcisource] (2,0);");
    differs(am, &format!(r"\ctikzset{{bipoles/vsourceam/margin=0.5}}{am}"));
    differs(am, &format!(r"\ctikzset{{bipoles/vsourceam/inner plus={{\tiny $+$}}}}{am}"));
    differs(am, &format!(r"\ctikzset{{sources/symbol/sign rotation=0}}{am}"));
    differs(r"\draw (0,0) to[smeter] (2,0);", r"\ctikzset{bipoles/smeter/index position=50}\draw (0,0) to[smeter] (2,0);");
    differs(r"\draw (0,0) to[qiprobe] (2,0);", r"\ctikzset{bipoles/qmeter/index position=25}\draw (0,0) to[qiprobe] (2,0);");
}

#[test]
fn a_current_source_keeps_its_arrow_when_its_circle_opens() {
    let open = drawing(r"\ctikzset{bipoles/dcisource/angle=45}\draw (0,0) to[dcisource] (2,0);");
    let closed = drawing(r"\draw (0,0) to[dcisource] (2,0);");
    assert_eq!(open.matches("<path").count(), closed.matches("<path").count());
}

#[test]
fn the_blocks() {
    let lowpass = r"\draw[dashed] (0,0) to[lowpass] (2,0);";
    differs(r"\draw (0,0) to[lowpass] (2,0);", lowpass);
    differs(lowpass, &format!(r"\ctikzset{{inner blocks dashed}}{lowpass}"));
    differs(&format!(r"\ctikzset{{inner blocks dashed}}{lowpass}"), &format!(r"\ctikzset{{inner blocks dashed, dashed blocks pattern={{{{1.5pt}}{{1pt}}}}}}{lowpass}"));
    differs(r"\draw (0,0) to[sdcdc] (2,0);", r"\ctikzset{blocks dc segments=3}\draw (0,0) to[sdcdc] (2,0);");
    differs(r"\draw (0,0) to[sdcdc] (2,0);", r"\draw (0,0) to[sdcdc, blocks dc in segments=2] (2,0);");
    let plot = r"\draw (0,0) to[lowpassp] (2,0);";
    for key in ["color=red", "dash=none", "thickness=1", "opacity=0"] {
        differs(plot, &format!(r"\ctikzset{{blocks/filter grid/{key}}}{plot}"));
    }
    differs(plot, &format!(r"\ctikzset{{blocks/filter plot/rounding=0}}{plot}"));
    differs(plot, &format!(r"\ctikzset{{blocks/filter plot/relative thickness=1}}{plot}"));
    differs(r"\draw (0,0) to[lowpassp, ideal filter] (2,0);", r"\draw (0,0) to[lowpassp, ideal filter, ideal filter numbers] (2,0);");
}

#[test]
fn the_logic_ports() {
    let or = r"\begin{scope}[american]\draw (0,0) node[or port]{};\end{scope}";
    differs(or, &format!(r"\ctikzset{{american or shape=roundy}}{or}"));
    let xnor = r"\ctikzset{logic ports=ieee}\draw (0,0) node[xnor port]{};";
    for key in ["not radius=0.1", "xor bar distance=0.3", "xor leads in=0"] {
        differs(xnor, &format!(r"\ctikzset{{ieeestd ports/.cd, {key}}}{xnor}"));
    }
}

#[test]
fn the_rest() {
    differs(r"\draw (0,0) node[jump crossing]{};", r"\ctikzset{crossing vertical/color=red}\draw (0,0) node[jump crossing]{};");
    differs(r"\draw (0,0) node[transformer core]{};", r"\ctikzset{transformer core/color=red}\draw (0,0) node[transformer core]{};");
    differs(r"\draw (0,0) node[en amp]{};", r"\draw (0,0) node[en amp, en amp text A]{};");
    differs(r"\draw (0,0) to[R, o-o] (2,0);", r"\draw (0,0) to[R, o-o, nodes width=0.1] (2,0);");
    // a pole by a macro, and by a style
    differs(r"\draw (0,0) to[R] (2,0);", r"\def\p{ocirc}\draw (0,0) to[R, bipole nodes={none}{\p}] (2,0);");
    differs(r"\draw (0,0) to[R] (2,0);", r"\tikzset{bcirc/.style={shape=ocirc, fill=blue}}\ctikzset{bipole/nodes/left=bcirc}\draw (0,0) to[R] (2,0);");
    // an oscilloscope's own trace
    let scope = r"\draw (0,0) node[oscopeshape]{};";
    differs(
        scope,
        &format!(
            r"\ctikzset{{bipoles/oscope/waveform/mine/.code={{\pgfpathmoveto{{\pgfpoint{{0cm}}{{0cm}}}}\pgfpathlineto{{\pgfpoint{{1cm}}{{1cm}}}}\pgfusepath{{draw}}}}}}\ctikzset{{bipoles/oscope/waveform=mine}}{scope}"
        ),
    );
}
