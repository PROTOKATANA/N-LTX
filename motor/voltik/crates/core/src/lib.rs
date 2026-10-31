//! voltik: CircuiTikZ circuits to SVG and PNG, with no TeX install.
//!
//! voltik is a library of [tikra](https://docs.rs/tikra) (`tikra::extension`): the CircuiTikZ keys
//! and components on top of tikra's TikZ. What it draws follows the CircuiTikZ manual.
//!
//! ```
//! let r = voltik::convert(r"\begin{circuitikz} \draw (0,0) to[R=$R_1$, o-*] (2,0); \end{circuitikz}").unwrap();
//! assert!(r.svg.starts_with("<svg"));
//! ```

pub mod bipole;
pub mod components;
pub mod label;
pub mod nodes;
pub mod vif;

use crate::bipole::setting;
use tikra::extension::{Context, KeyFn, Library, Point, Tokens};
pub use tikra::{png, Error, Locale, Options, Output, MAX_PIXELS};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The CircuiTikZ library, to give tikra next to other libraries.
pub static LIBRARY: Library = Library { name: "voltik", definitions, keys: KEYS };

const KEYS: &[(&str, KeyFn)] = &[
    ("/tikz/voltik/component", bipole::k_component),
    ("/tikz/voltik/place", bipole::k_place),
    ("/tikz/voltik/pole", bipole::k_pole),
    ("/tikz/voltik/flag", bipole::k_flag),
    ("/tikz/voltik/label", label::k_label),
    ("/tikz/voltik/ctikzset", k_ctikzset),
    ("/tikz/voltik/vif", vif::k_vif),
    ("/tikz/voltik/shapes", nodes::k_shapes),
    ("/tikz/voltik/class fill", nodes::k_class_fill),
    ("/tikz/voltik/rotary", nodes::k_rotary),
    ("/tikz/voltik/logic port", nodes::k_logic_port),
    ("/tikz/voltik/subcircuit", k_subcircuit),
    ("/tikz/voltik/tunable arrow", k_tunable_arrow),
    ("/tikz/muxdemux def", nodes::k_muxdemux_def),
    ("/tikz/seven segment val", nodes::k_seven_segment_val),
    ("/tikz/seven segment bits", nodes::k_seven_segment_bits),
];

/// `voltik/subcircuit={divider}{in}`: where a subcircuit starts for its anchor `in` to be at the
/// current point (`\voltik@scx`, `\voltik@scy`), found by measuring it drawn from the origin,
/// without drawing it.
fn k_subcircuit(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let a = v.unwrap_or_default().arguments(2);
    let name = cx.expand(a[0].clone())?;
    let anchor = cx.expand(a[1].clone())?;
    let anchor = anchor.trim();
    let (mut x, mut y) = (0.0, 0.0);
    if !anchor.is_empty() {
        let t = cx.transform();
        cx.set_transform(tikra::extension::Transform::default());
        let probe = format!("\\path (0,0) \\csname voltik@sc@{name}\\endcsname{{voltik@probe}};");
        let measured = cx.measure(Tokens::text(&probe), |cx| cx.point(&format!("voltik@probe-{anchor}")));
        cx.set_transform(t);
        if let Ok((Ok(at), _)) = measured {
            (x, y) = (-at.x, -at.y);
        }
    }
    cx.define_text("voltik@scx", &format!("{x}cm"), true);
    cx.define_text("voltik@scy", &format!("{y}cm"), true);
    Ok(())
}

/// `voltik/tunable arrow={options}{thickness}{length}{angle}{name}`: an arrow through the middle
/// of the component or node `name`, `length` times its diagonal long, turned `angle` from its
/// direction, `thickness` times the line width, with the tips of `tunable start arrow` and
/// `tunable end arrow` (none and `latexslim` by default).
fn k_tunable_arrow(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let a = v.unwrap_or_default().arguments(5);
    let options = a[0].to_text();
    let thickness = cx.number(a[1].clone()).unwrap_or(1.0);
    let length = cx.number(a[2].clone()).unwrap_or(1.0);
    let angle = cx.number(a[3].clone()).unwrap_or(0.0);
    let name = cx.expand(a[4].clone())?;
    let name = name.trim();
    let (Ok(c), Ok(ne), Ok(sw)) = (cx.point(&format!("{name}.center")), cx.point(&format!("{name}.north east")), cx.point(&format!("{name}.south west"))) else {
        cx.warn(format!("voltik: \\ctikztunablearrow: unknown component {name}; ignored"));
        return Ok(());
    };
    // (the points are on the canvas: back to the picture's coordinates)
    let t = cx.transform();
    let det = t.a * t.d - t.b * t.c;
    let back = |p: Point| -> Point {
        let (x, y) = (p.x - t.e, p.y - t.f);
        if det.abs() < 1e-12 {
            p
        } else {
            Point::new((t.d * x - t.c * y) / det, (-t.b * x + t.a * y) / det)
        }
    };
    let (c, ne, sw) = (back(c), back(ne), back(sw));
    let diagonal = ((ne.x - sw.x).powi(2) + (ne.y - sw.y).powi(2)).sqrt();
    let direction = cx.macro_text(&format!("voltik@direction@{name}")).trim().parse::<f64>().unwrap_or(0.0);
    let turn = (direction + angle).to_radians();
    let half = length * diagonal / 2.0;
    let (dx, dy) = (half * turn.cos(), half * turn.sin());
    let tip = |cx: &mut Context, key: &str, default: &str| -> String {
        match setting(cx, key).unwrap_or_default().trim() {
            "" | "default" => default.to_string(),
            t => t.to_string(),
        }
    };
    let (start, end) = (tip(cx, "tunable start arrow", ""), tip(cx, "tunable end arrow", "latexslim"));
    let width = thickness * cx.line_width();
    cx.run(Tokens::text(&format!("\\draw[{{{start}}}-{{{end}}}, line width={width}cm, {options}] ({}cm,{}cm) -- ({}cm,{}cm);", c.x - dx, c.y - dy, c.x + dx, c.y + dy)))
}

/// The settings each class of components has (`resistors/scale`), with their initial values.
const CLASS_SETTINGS: &[(&str, &str)] = &[("scale", "1"), ("width", ""), ("height", ""), ("thickness", "none"), ("modifier thickness", "1"), ("fill", "")];

/// `\ctikzset{…}`: the keys under `/tikz/circuitikz`; a style defined there (`tallR/.style={…}`) is
/// also a style of TikZ, to be used in a `to[…]` as the manual does.
fn k_ctikzset(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let list = v.unwrap_or_default();
    cx.keys_at("/tikz/circuitikz", list.clone())?;
    let keys = list.key_list();
    if keys.iter().any(|(k, _)| k.ends_with(".cd")) {
        return Ok(());
    }
    for (key, value) in keys {
        let handler = [".style", ".append style", ".prefix style", ".code", ".style 2 args", ".code 2 args", ".style args", ".code args", ".default"]
            .iter()
            .any(|h| key.ends_with(&format!("/{h}")))
            || key.contains("/.style n args")
            || key.contains("/.code n args");
        // (but the labels' styles, which a `to` sets by its own keys of their names)
        let own = key.starts_with("bipole ") && key.split('/').next().is_some_and(|k| k.trim().ends_with(" style"));
        if handler && !own && key.matches('/').count() == 1 {
            cx.keys(Tokens::text(&format!("{key}=")).and(value.unwrap_or_default().braced()))?;
        }
    }
    Ok(())
}

/// The poles a `to` can end in, by their shorthand: none, `o` (open circle), `*` (filled circle) and
/// `d` (filled diamond).
const POLES: &[(&str, &str)] = &[("", "none"), ("o", "ocirc"), ("*", "circ"), ("d", "diamondpole")];

/// The settings of `\ctikzset`, the styles and the component keys, as TeX.
fn definitions() -> String {
    let mut s = String::new();
    s.push_str("\\def\\ctikzset#1{\\pgfkeys{/tikz/voltik/ctikzset={#1}}}\n");
    s.push_str("\\def\\ctikzvalof#1{\\pgfkeysvalueof{/tikz/circuitikz/#1}}\n");
    // the release of CircuiTikZ that voltik draws as
    s.push_str("\\def\\pgfcircversion{1.8.7}\n");
    // the anchors that labels were placed by, and their rotation
    s.push_str("\\def\\ctikzgetanchor#1#2{\\csname voltik@anchor@#1@#2\\endcsname}\n");
    s.push_str("\\def\\ctikzgetdirection#1{\\csname voltik@direction@#1\\endcsname}\n");
    // `\\ctikzloadstyle{romano}`: the style files of the distribution, as `romano circuit style`…
    // (`example` as the manual lists it; `romano` and `legacy` as they set the settings voltik has,
    // measured on TeX)
    s.push_str("\\def\\circuitikzbasekey{/tikz/circuitikz}\\def\\ctikzloadstyle#1{\\csname voltik@style@#1\\endcsname}\n");
    for (name, settings) in [
        ("example", "amplifiers/fill=blue!20!white"),
        ("romano", "amplifiers/thickness=3.0, batteries/thickness=2.0, bipoles/cuteswitch/thickness=0.5, capacitors/scale=0.7, capacitors/thickness=1.0, chips/thickness=3.0, csources/scale=0.9, csources/thickness=1.0, diodes/scale=0.6, diodes/thickness=1.0, flipflops/thickness=2.0, grounds/thickness=1.0, inductors/scale=0.8, inductors/thickness=1.0, logic ports/thickness=2.0, resistors/scale=0.8, resistors/thickness=1.0, sources/scale=0.9, sources/thickness=1.0, transistors/scale=1.3, transistors/thickness=3.0"),
        ("legacy", "amplifiers/thickness=none, batteries/thickness=none, bipoles/cuteswitch/thickness=1, capacitors/scale=1.0, capacitors/thickness=none, chips/thickness=none, csources/scale=1.0, csources/thickness=none, diodes/scale=1.0, diodes/thickness=none, flipflops/thickness=none, grounds/thickness=none, inductors/scale=1.0, inductors/thickness=none, logic ports/thickness=none, resistors/scale=1.0, resistors/thickness=none, sources/scale=1.0, sources/thickness=none, transistors/scale=1.0, transistors/thickness=none"),
    ] {
        s.push_str(&format!("\\def\\voltik@style@{name}{{\\tikzset{{{name} circuit style/.style={{/tikz/circuitikz/.cd, {settings}}}}}}}\n"));
    }
    s.push_str("\\def\\voltik@vqadd#1#2#3#4{\\expandafter\\def\\expandafter\\voltik@vq\\expandafter{\\voltik@vq\\csname #1\\endcsname{\\voltik@qn}{#2}{#3}{#4}}}\n");
    s.push_str(
        "\\def\\voltik@queue{}\\def\\ctikzprocessvif{\\voltik@queue\\gdef\\voltik@queue{}}\\tikzset{every picture/.append style={execute at end picture=\\ctikzprocessvif}}\n",
    );
    // `\ctikzactivatevoltagedirections{vpms}`: `vpms^>={…}` is `vpms={…}` with its `v` a `v^>`
    for (kind, what) in [("v", "voltage"), ("i", "current"), ("f", "flow")] {
        s.push_str(&format!("\\def\\ctikzactivate{what}directions#1{{"));
        for variant in ["^", "_", ">", "<", "^>", "^<", "_>", "_<", ">^", ">_", "<^", "<_"] {
            s.push_str(&format!("\\tikzset{{#1{variant}/.code={{\\def\\voltik@{kind}var{{{variant}}}\\pgfkeysalso{{#1={{##1}}}}}}}}"));
        }
        s.push_str("}\n");
    }
    s.push_str(
        "\\pgfkeys{/tikz/circuitikz/.cd,\n\
         bipoles/length/.initial=1.4cm, bipoles/thickness/.initial=2,\n\
         resistors/zigs/.initial=3, inductors/coils/.initial=, wiper pos/.initial=0.5, quadpoles style/.initial=, nodes width/.initial=0.04,\n\
         bipoles/inductors/core distance/.initial=2pt, bipoles/inductors/dot x distance/.initial=4pt, bipoles/inductors/dot y distance/.initial=1pt,\n\
         resistor/.initial=american, inductor/.initial=cute, diode/.initial=empty, thyristor style/.initial=legacy, led arrows/.initial=anode, pd arrows/.initial=anode, diode whiskers/.initial=sloped, tjumper connections/.initial=00, tr/no input leads/.initial=false, tr/no output leads/.initial=false, tr/no leads/.initial=false, tr/number inputs/.initial=,\n\
         logic ports/.initial=american, american ports/.style={logic ports=american}, european ports/.style={logic ports=european}, ieee ports/.style={logic ports=ieee}, tubes/anodes/.initial=3, transistor circle/scale circle radius/.initial=1, transistor circle/relative thickness/.initial=1, tr/ferroel gate/.initial=false, transistors/arrow pos/.initial=mid, tr/legacy text/.initial=false, legacy transistors text/.style={tr/legacy text=true}, bipoles/cuteswitch/shape/.initial=ocirc, bipoles/cuteswitch/height/.initial=.6, bipoles/cuteswitch/thickness/.initial=1, bipoles/jumper/shape/.initial=ocirc, multipoles/rotary/channels/.initial=5, multipoles/rotary/angle/.initial=60, multipoles/rotary/wiper/.initial=20,\n\
         multipoles/rotary/arrow/.initial=none, multipoles/rotary/thickness/.initial=1, multipoles/rotary/shape/.initial=ocirc, multipoles/num pins/.initial=8, multipoles/dipchip/width/.initial=1.2, multipoles/dipchip/pin spacing/.initial=0.4, multipoles/qfpchip/pin spacing/.initial=0.4, multipoles/thickness/.initial=2, multipoles/external pins thickness/.initial=1, multipoles/external pins width/.initial=0.2, multipoles/external pad fraction/.initial=0, multipoles/topmark/.initial=true, multipoles/numbers/.initial=true, multipoles/draw only pins/.initial=all, multipoles/straight numbers/.initial=true, multipoles/font/.initial=\\tiny,
         multipoles/muxdemux/Lh/.initial=8, multipoles/muxdemux/Rh/.initial=6, multipoles/muxdemux/w/.initial=3, multipoles/muxdemux/NL/.initial=8, multipoles/muxdemux/NR/.initial=1, multipoles/muxdemux/NB/.initial=3, multipoles/muxdemux/NT/.initial=0, multipoles/muxdemux/inset w/.initial=0, multipoles/muxdemux/inset Lh/.initial=0, multipoles/muxdemux/inset Rh/.initial=0, multipoles/muxdemux/square pins/.initial=0, multipoles/muxdemux/base len/.initial=0.4,
         multipoles/draw only left pins/.initial=all, multipoles/draw only right pins/.initial=all, multipoles/draw only bottom pins/.initial=all, multipoles/draw only top pins/.initial=all,
         muxdemux/inner label font/.initial=\\tiny, muxdemux/outer label font/.initial=\\tiny, muxdemux/border label font/.initial=\\tiny, muxdemux/inner label xsep/.initial=2pt, muxdemux/inner label ysep/.initial=2pt, muxdemux/outer label xsep/.initial=2pt, muxdemux/outer label ysep/.initial=2pt, muxdemux/border label xsep/.initial=2pt, muxdemux/border label ysep/.initial=2pt,
         muxdemux/inner label sep/.style={muxdemux/inner label xsep=#1, muxdemux/inner label ysep=#1}, muxdemux/outer label sep/.style={muxdemux/outer label xsep=#1, muxdemux/outer label ysep=#1}, muxdemux/border label sep/.style={muxdemux/border label xsep=#1, muxdemux/border label ysep=#1}, muxdemux/clock wedge size/.initial=0.2, wedge inversion mark/width/.initial=0.2, wedge inversion mark/height/.initial=0.1, tripoles/spdt/width/.initial=.85, double bipole L/.initial=, double bipole R/.initial=, db/L invert/.initial=false, db/R invert/.initial=false,\n\
         double bipole L invert/.code={\\pgfkeys{/tikz/circuitikz/db/L invert=true}}, double bipole R invert/.code={\\pgfkeys{/tikz/circuitikz/db/R invert=true}},\n\
         every double bipole L/.style={}, every double bipole R/.style={}, transformer L1/.style={}, transformer L2/.style={}, en amp text/.initial=, component text/.initial=center, left text distance/.initial=.3em, tr/noinv input up/.initial=false, tr/noinv input down/.initial=false, tr/noinv output up/.initial=false,\n\
         tr/noinv output down/.initial=false, tr/filament/.initial=false, tr/nocathode/.initial=false, tr/fullcathode/.initial=false, tr/nixieanode/.initial=false,\n\
         tr/anodedot/.initial=false, tr/nogrid/.initial=false, tr/schottky base/.initial=false, tr/bodydiode/.initial=false, tr/photo/.initial=false, tr/tr circle/.initial=false,\n\
         tr/nobase/.initial=false, tr/bulk/.initial=false, tr/emptycircle/.initial=false, tr/nocircle/.initial=false, tr/arrowmos/.initial=false, tr/solderdot/.initial=false,\n\
         tr/fdsoi/.initial=false, tr/fdsoidot/.initial=false, tr/doublegate/.initial=false, tr/collectors/.initial=, tr/emitters/.initial=, tr/bjt pins width/.initial=, tr/bjt multi height/.initial=,\n\
         led arrows from cathode/.style={led arrows=cathode}, led arrows from anode/.style={led arrows=anode}, pd arrows to cathode/.style={pd arrows=cathode},\n\
         pd arrows to anode/.style={pd arrows=anode}, diode straight whiskers/.style={diode whiskers=straight}, diode sloped whiskers/.style={diode whiskers=sloped}, voltage/.initial=european, current/.initial=european, voltage dir/.initial=noold,\n\
         voltage/distance from node/.initial=, voltage/bump b/.initial=, l2 halign/.initial=l, l2 valign/.initial=c, a2 halign/.initial=r, a2 valign/.initial=c, bipole/is voltage/.initial=true, bipole/is current/.initial=true, bipole/override source vif/.initial=false,\n\
         voltage shift/.initial=0, current/distance/.initial=, flow/distance/.initial=, v symbols/.initial=true, i symbols/.initial=true, f symbols/.initial=true,\n\
         bipole label style/.style={}, bipole annotation style/.style={}, bipole voltage style/.style={}, bipole current style/.style={}, bipole flow style/.style={},\n\
         no v symbols/.style={v symbols=false}, no i symbols/.style={i symbols=false}, no f symbols/.style={f symbols=false},\n\
         voltage/american label distance/.initial=1.4, voltage/american font/.initial=, voltage/american plus/.initial=$+$, voltage/american minus/.initial=$\\vphantom{+}-$,\n\
         label/align/.initial=smart, label distance/.initial=0pt, annotation distance/.initial=0pt,\n\
         american/.style={/tikz/american}, european/.style={/tikz/european},\n\
         american resistors/.style={/tikz/american resistors}, european resistors/.style={/tikz/european resistors},\n\
         american inductors/.style={/tikz/american inductors}, european inductors/.style={/tikz/european inductors},\n\
         cute inductors/.style={/tikz/cute inductors}, .search also={/tikz},\n\
         straightlabels/.style={label/align=straight}, rotatelabels/.style={label/align=rotate}, smartlabels/.style={label/align=smart},\n\
         }\n",
    );
    // the blocks' settings: their boxed and ideal variants, a bare transmission line
    s.push_str(
        "\\pgfkeys{/tikz/circuitikz/.cd, blocks/boxed/.initial=false, blocks/box only/.initial=false, blocks/ideal filter/.initial=false, bipoles/tline/bare/.initial=false, RF/scale/.initial=1, RF/thickness/.initial=none, RF/fill/.initial=, blocks/inner color/.initial=default, blocks/inner thickness/.initial=1,block left anchors pos/.initial=0.5, block right anchors pos/.initial=0.5, block lateral anchors pos/.style={block left anchors pos=#1, block right anchors pos=#1}}\n",
    );
    // the flip-flops' and the seven segment displays' settings
    s.push_str(
        "\\pgfkeys{/tikz/circuitikz/.cd, multipoles/flipflop/width/.initial=1.2, multipoles/flipflop/pin spacing/.initial=0.6, multipoles/flipflop/clock wedge size/.initial=0.2,\n\
         multipoles/flipflop/font/.initial=\\small, multipoles/flipflop/fontud/.initial=\\tiny, seven seg/width/.initial=0.4, seven seg/thickness/.initial=4pt,\n\
         seven seg/segment sep/.initial=0.2pt, seven seg/box sep/.initial=1pt, seven seg/color on/.initial=red, seven seg/color off/.initial=gray!20!white, seven seg/bits/.initial=0000000,\n\
         seven seg/dot/.initial=none, seven seg/box/.initial=off}\n",
    );
    // the logic ports' symbols and origin
    s.push_str(
        "\\pgfkeys{/tikz/circuitikz/.cd, tripoles/european not symbol/.initial=triangle, european xnor style/.initial=default, european ports font/.initial=,\n\
         logic ports origin/.initial=legacy, tgate scale/.initial=0.7, tr/inner inputs/.initial=, ieeestd ports/height/.initial=2, ieeestd ports/pin length/.initial=0.7}\n",
    );
    // the American ports' sizes that voltik has measured some of (`tripoles/american or port/height`)
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd");
    for kind in ["and", "or", "nand", "nor", "xor", "xnor"] {
        // (`input height` changes none of the ports measured, with 2 or 3 inputs)
        s.push_str(&format!(", tripoles/american {kind} port/height/.initial=.8, tripoles/american {kind} port/input height/.initial=.5"));
        if kind.ends_with("or") {
            s.push_str(&format!(", tripoles/american {kind} port/inner/.initial=0.3, tripoles/american {kind} port/angle/.initial=70"));
        }
    }
    s.push_str(", tripoles/thickness/.initial=2}\n");
    // `\ctikztextnot{Q}`: a text with a bar over it
    s.push_str("\\def\\ctikztextnot#1{\\ensuremath{\\overline{\\mbox{#1}}}}\n");
    let mut keys: Vec<String> = vec![
        "american resistors/.style={/tikz/circuitikz/resistor=american}".into(),
        "european resistors/.style={/tikz/circuitikz/resistor=european}".into(),
        "american inductors/.style={/tikz/circuitikz/inductor=american}".into(),
        "european inductors/.style={/tikz/circuitikz/inductor=european}".into(),
        "cute inductors/.style={/tikz/circuitikz/inductor=cute}".into(),
        "empty diodes/.style={/tikz/circuitikz/diode=empty}".into(),
        "full diodes/.style={/tikz/circuitikz/diode=full}".into(),
        "stroke diodes/.style={/tikz/circuitikz/diode=stroke}".into(),
        "led arrows from cathode/.style={/tikz/circuitikz/led arrows=cathode}".into(),
        "led arrows from anode/.style={/tikz/circuitikz/led arrows=anode}".into(),
        "pd arrows to cathode/.style={/tikz/circuitikz/pd arrows=cathode}".into(),
        "pd arrows to anode/.style={/tikz/circuitikz/pd arrows=anode}".into(),
        "diode straight whiskers/.style={/tikz/circuitikz/diode whiskers=straight}".into(),
        "diode sloped whiskers/.style={/tikz/circuitikz/diode whiskers=sloped}".into(),
        "voltage dir/.style={/tikz/circuitikz/voltage dir=#1}".into(),
        "american voltages/.style={/tikz/circuitikz/voltage=american}".into(),
        "american currents/.style={/tikz/circuitikz/current=american}".into(),
        "european currents/.style={/tikz/circuitikz/current=european}".into(),
        "european voltages/.style={/tikz/circuitikz/voltage=european}".into(),
        "straight voltages/.style={/tikz/circuitikz/voltage=straight}".into(),
        "raised voltages/.style={/tikz/circuitikz/voltage=raised}".into(),
        "american/.style={american resistors, american inductors, american voltages, american currents, american ports}".into(),
        "european/.style={european resistors, european inductors, european voltages, european currents, european ports}".into(),
        "straight labels/.style={/tikz/circuitikz/label/align=straight}".into(),
        "rotate labels/.style={/tikz/circuitikz/label/align=rotate}".into(),
        "smart labels/.style={/tikz/circuitikz/label/align=smart}".into(),
        "mirror/.style={voltik/flag=mirror}".into(),
        "invert/.style={voltik/flag=invert}".into(),
        "label distance/.style={/tikz/circuitikz/label distance={#1}}".into(),
        "annotation distance/.style={/tikz/circuitikz/annotation distance={#1}}".into(),
        "wiper pos/.style={/tikz/circuitikz/wiper pos={#1}}".into(),
        "no v symbols/.style={/tikz/circuitikz/v symbols=false}".into(),
        // `to[bipole label style={fill=cyan}]`: the style of its label, instead of the circuit's
        "bipole label style/.style={/tikz/circuitikz/bipole label style/.style={#1}}".into(),
        "bipole annotation style/.style={/tikz/circuitikz/bipole annotation style/.style={#1}}".into(),
        "bipole voltage style/.style={/tikz/circuitikz/bipole voltage style/.style={#1}}".into(),
        "bipole current style/.style={/tikz/circuitikz/bipole current style/.style={#1}}".into(),
        "bipole flow style/.style={/tikz/circuitikz/bipole flow style/.style={#1}}".into(),
        "bipole label append style/.style={/tikz/circuitikz/bipole label style/.append style={#1}}".into(),
        "bipole annotation append style/.style={/tikz/circuitikz/bipole annotation style/.append style={#1}}".into(),
        "bipole voltage append style/.style={/tikz/circuitikz/bipole voltage style/.append style={#1}}".into(),
        "bipole current append style/.style={/tikz/circuitikz/bipole current style/.append style={#1}}".into(),
        "bipole flow append style/.style={/tikz/circuitikz/bipole flow style/.append style={#1}}".into(),
        "no i symbols/.style={/tikz/circuitikz/i symbols=false}".into(),
        "no f symbols/.style={/tikz/circuitikz/f symbols=false}".into(),
        "v symbols/.style={/tikz/circuitikz/v symbols=true}".into(),
        "i symbols/.style={/tikz/circuitikz/i symbols=true}".into(),
        "f symbols/.style={/tikz/circuitikz/f symbols=true}".into(),
    ];
    // the node-style components, shapes of their own
    for name in nodes::NODES.iter().map(|n| n.name).chain(nodes::TERMINALS.iter().copied()).chain(nodes::variant_shapes()) {
        // and its class's `fill` (`\ctikzset{amplifiers/fill=cyan}`)
        match nodes::find(name).and_then(|n| n.class()) {
            Some(class) if name.starts_with("cute spdt") => {
                keys.push(format!("{name}/.style={{shape={name}, voltik/class fill={class}, append after command={{\\voltik@na\\voltik@cs}}}}"))
            }
            // (arrows of a look of their own are drawn after it)
            Some(class) if class == "transistors" => keys.push(format!("{name}/.style={{shape={name}, voltik/class fill={class}, append after command={{\\voltik@na}}}}")),
            // a supply's arrow, when it is one of TikZ's, is drawn after it
            Some(class) if name == "vcc" || name == "vee" => keys.push(format!("{name}/.style={{shape={name}, voltik/class fill={class}, append after command={{\\voltik@sa}}}}")),
            Some(class) => keys.push(format!("{name}/.style={{shape={name}, voltik/class fill={class}}}")),
            // a double bipole draws the bipoles of `double bipole L` and `R` after it
            None if name == "double bipole" => keys.push(format!("{name}/.style={{shape={name}, append after command={{\\voltik@db}}}}")),
            // a transformer's coils are nodes of their own (`T1-L1`), and drawn by `transformer L1`
            None if name.starts_with("transformer") => keys.push(format!("{name}/.style={{shape={name}, append after command={{\\voltik@tr}}}}")),
            None => keys.push(format!("{name}/.style={{shape={name}}}")),
        }
    }
    for (key, target) in components::aliases() {
        keys.push(format!("{key}/.style={{voltik/component={{{key}}}{{#1}}{{}}}}"));
        // a source's key takes the variants of its voltage or current (`I<=1A`, `V_>=…`)
        if let components::Target::Fixed(n) | components::Target::Voltage(n, _) | components::Target::Current(n, _) = target {
            if components::find(n).is_some_and(|c| c.source() != components::Source::Not) {
                for variant in ["^", "_", ">", "<", "^>", "^<", "_>", "_<", ">^", ">_", "<^", "<_"] {
                    keys.push(format!("{key}{variant}/.style={{voltik/component={{{key}}}{{#1}}{{{variant}}}}}"));
                }
            }
        }
    }
    for (key, which, side) in [
        ("l", "l", 0),
        ("l^", "l", 1),
        ("l_", "l", -1),
        ("a", "a", 0),
        ("a^", "a", 1),
        ("a_", "a", -1),
        ("l2", "l2", 0),
        ("l2^", "l2", 1),
        ("l2_", "l2", -1),
        ("a2", "a2", 0),
        ("a2^", "a2", 1),
        ("a2_", "a2", -1),
    ] {
        keys.push(format!("{key}/.style={{voltik/label={{{which}}}{{{side}}}{{#1}}}}"));
    }
    // the transistors', tubes' and amplifiers' options (`node[npn, bodydiode]`, `node[op amp, noinv input up]`)
    for flag in nodes::VARIANT_FLAGS {
        keys.push(format!("{flag}/.style={{/tikz/circuitikz/tr/{flag}=#1}}, {flag}/.default=true"));
    }
    for key in nodes::VARIANT_VALUES {
        keys.push(format!("{key}/.style={{/tikz/circuitikz/tr/{key}=#1}}"));
    }
    // `en amp text={…}`: the text of an `en amp` instead of its ▷∞ (drawn by its shape)
    keys.push("en amp text/.code={\\pgfkeys{/tikz/circuitikz/en amp text={#1}}}".into());
    // a bipole as a node (`node[resistorshape]`), and inside a double bipole
    keys.push("voltik/fit/.code={\\def\\voltik@fit{1}}".into());
    keys.push("voltik/up/.code={\\def\\voltik@up{#1}}".into());
    keys.push("voltik/bipole node/.code={\\def\\voltik@bn{#1}}".into());
    keys.push("voltik/coil/.code={\\def\\voltik@coil{#1}}".into());
    keys.push("voltik/pole shape/.code={\\def\\voltik@pole{#1}}".into());
    for (shape, key) in components::bipole_shapes() {
        keys.push(format!("{shape}/.style={{shape=voltik bipole, voltik/bipole node={{{key}}}, append after command={{\\voltik@bp}}}}"));
    }
    // the rotary switch: bare, and by its styles
    keys.push("rotaryswitch/.style={shape=rotaryswitch, voltik/class fill=switches, append after command={\\voltik@rs}}".into());
    for (style, arrow) in [("rotary switch", ""), ("rotary switch -", "none"), ("rotary switch <-", "ccw"), ("rotary switch ->", "cw"), ("rotary switch <->", "both")] {
        keys.push(format!("{style}/.style={{voltik/rotary={{{arrow}}}{{#1}}}}, {style}/.default={{}}"));
    }
    // the blocks' own sizes (`bipoles/amp/width`), and the RF lines' (`bipoles/tline/width`)
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd");
    for name in components::block_names() {
        for key in ["width", "height"] {
            s.push_str(&format!(", bipoles/{name}/{key}/.initial="));
            keys.push(format!("bipoles/{name}/{key}/.style={{/tikz/circuitikz/bipoles/{name}/{key}={{#1}}}}"));
        }
    }
    s.push_str("}\n");
    // the blocks drawn as nodes
    for (name, class) in nodes::block_shapes() {
        keys.push(format!("{name}/.style={{shape={name}, voltik/class fill={class}}}"));
    }
    // the blocks' variants
    for (key, setting) in [
        ("boxed", "blocks/boxed=true"),
        ("box", "blocks/boxed=true"),
        ("box only", "blocks/boxed=true, /tikz/circuitikz/blocks/box only=true"),
        ("boxed only", "blocks/boxed=true, /tikz/circuitikz/blocks/box only=true"),
        ("ideal filter", "blocks/ideal filter=true"),
    ] {
        keys.push(format!("{key}/.style={{/tikz/circuitikz/{setting}}}"));
    }
    keys.push("bipoles/tline/bare/.style={/tikz/circuitikz/bipoles/tline/bare=#1}, bipoles/tline/bare/.default=true".into());
    keys.push("mstlinelen/.style={/tikz/circuitikz/bipoles/mstline/width=#1}".into());
    for key in ["blocks/inner color", "blocks/inner thickness", "block left anchors pos", "block right anchors pos", "block lateral anchors pos"] {
        keys.push(format!("{key}/.style={{/tikz/circuitikz/{key}={{#1}}}}"));
    }
    // a split two-port's texts
    keys.push("t1/.code={\\def\\voltik@ta{#1}}, t2/.code={\\def\\voltik@tb{#1}}".into());
    keys.push("border text distance/.style={/tikz/circuitikz/border text distance=#1}".into());
    s.push_str("\\def\\voltik@ta{}\\def\\voltik@tb{}\\pgfkeys{/tikz/circuitikz/border text distance/.initial=2pt}\n");
    // the flip-flops, by `flipflop def` (the later of a key winning), and the ones the package defines
    keys.push("flipflop/.style={shape=flipflop, voltik/class fill=flipflops, append after command={\\voltik@ff}}".into());
    keys.push("flipflop def/.code={\\expandafter\\def\\expandafter\\voltik@ffd\\expandafter{\\voltik@ffd,#1}}".into());
    keys.push("clockwedge/.style={shape=clockwedge}".into());
    for (name, def) in [
        ("latch", "t1=D, t6=Q, t3=CLK, t4=\\ctikztextnot{Q}"),
        ("flipflop SR", "t1=S, t3=R, t6=Q, t4=\\ctikztextnot{Q}"),
        ("flipflop D", "t1=D, t6=Q, c3=1, t4=\\ctikztextnot{Q}"),
        ("flipflop T", "t1=T, t6=Q, c3=1, t4=\\ctikztextnot{Q}"),
        ("flipflop JK", "t1=J, t3=K, c2=1, t6=Q, t4=\\ctikztextnot{Q}"),
    ] {
        keys.push(format!("{name}/.style={{flipflop, flipflop def={{{def}}}}}"));
    }
    keys.push("add async SR/.style={flipflop def={tu={\\ctikztextnot{SET}}, td={\\ctikztextnot{RST}}}}".into());
    keys.push("dot on notQ/.style={flipflop def={t4={Q}, n4=1}}".into());
    // the ports that have one style only, by their short names
    for (short, port) in [("blank port", "european blank port"), ("blank not port", "european blank not port"), ("tgate", "ieee tgate"), ("double tgate", "ieee double tgate")] {
        keys.push(format!("{short}/.style={{{port}}}"));
    }
    // the seven segment display, bare and by its value or segments
    keys.push("bare7seg/.style={shape=bare7seg, voltik/class fill=displays}".into());
    // the multiplexers, and the ones the package defines
    keys.push("muxdemux/.style={shape=muxdemux, voltik/class fill=muxdemuxes, append after command={\\voltik@md}}".into());
    keys.push("muxdemux label/.code={\\expandafter\\def\\expandafter\\voltik@mdl\\expandafter{\\voltik@mdl,#1}}".into());
    keys.push("wedgeinv/.style={shape=wedgeinv}".into());
    keys.push("circleinv/.style={shape=circleinv, voltik/class fill=muxdemuxes}".into());
    for (name, def) in [
        ("demux", "Lh=4, Rh=8, NL=1, NB=3, NR=8"),
        ("one bit adder", "Lh=4, NL=2, Rh=2, NR=1, NB=1, w=1.5, inset w=0.5, inset Lh=2, inset Rh=1.5"),
        ("ALU", "Lh=5, NL=2, Rh=2, NR=1, NB=2, NT=1, w=2, inset w=1, inset Lh=2, inset Rh=0, square pins=1"),
    ] {
        keys.push(format!("{name}/.style={{muxdemux, muxdemux def={{{def}}}}}"));
    }
    // the instruments: an oscilloscope's trace and size, and screens kept upright or turned
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, bipoles/oscope/waveform/.initial=ramps, bipoles/oscope/width/.initial=, bipoles/oscope/height/.initial=, bipoles/oosource/width/.initial=, bipoles/oosource/height/.initial=, instruments/rotated/.initial=false, rotated instruments/.style={/tikz/circuitikz/instruments/rotated=true}, straight instruments/.style={/tikz/circuitikz/instruments/rotated=false}}\n");
    keys.push("rotated instruments/.style={/tikz/circuitikz/instruments/rotated=true}".into());
    keys.push("straight instruments/.style={/tikz/circuitikz/instruments/rotated=false}".into());
    // the transformer sources' windings: their three-phase symbols, sizes and extra winding
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, ootrans size/.initial=small, bipoles/oosource/prim/.initial=, bipoles/oosource/sec/.initial=, bipoles/oosource/tert/.initial=, bipoles/oosource/upper/.initial=, bipoles/oosourcetrans/upper/.initial=, bipoles/oosourcetrans/upper circle size default/.initial=0.7, sources/symbol/thickness/.initial=1");
    for kind in ["delta", "wye", "eyw", "xdelta", "zig"] {
        s.push_str(&format!(", sources/symbol/{kind} scale/.initial=1, sources/symbol/{kind} rot/.initial=0"));
    }
    for winding in ["prim", "sec", "tert", "upper"] {
        s.push_str(&format!(", sources/symbol/{winding} rot/.initial=0"));
        keys.push(format!("{winding}/.style={{/tikz/circuitikz/bipoles/oosource/{winding}=#1}}"));
    }
    s.push_str("}\n");
    keys.push("oo upper winding/.style={/tikz/circuitikz/bipoles/oosourcetrans/upper=#1}, oo upper winding/.default=default".into());
    // the partial borders, the body diode's look and the open poles' fill
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, transistor circle/partial borders/.initial=none, transistor circle/partial border dash/.initial={{2pt}{2pt}}, tubes/partial borders/.initial=none, tubes/partial border dash/.initial={{2pt}{2pt}}, transistor bodydiode/scale/.initial=0.3, transistor bodydiode/dot scale/.initial=0.7, transistor bodydiode/relative thickness/.initial=1, transistor bodydiode/color/.initial=default, transistor bodydiode/dash/.initial=default, tripoles/igbt/outer base height/.initial=, tripoles/igbt/outer base thickness/.initial=1, open poles fill/.initial=white, poles/open fill opacity/.initial=1}\n");
    keys.push("open poles opacity/.style={/tikz/circuitikz/poles/open fill opacity=#1}".into());
    // the amplifiers' signs, the supplies' arrows and the sizes of what is drawn at one size
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, amplifiers/plus/.initial=default, amplifiers/minus/.initial=default, amplifiers/symbol font/.initial=, monopoles/vcc/arrow/.initial=legacy, monopoles/vee/arrow/.initial=legacy, tripoles/schottky base size/.initial=0.05, monopoles/msrstub/radius/.initial=0.125, proximeter/hlines position/.initial=0.3, proximeter/hlines thickness/.initial=0.5, current arrow scale/.initial=16, blocks/gridnode/gridlines/.initial=7, logic ports draw input leads/.code={\\def\\voltik@a{#1}\\def\\voltik@b{false}\\ifx\\voltik@a\\voltik@b\\pgfkeys{/tikz/circuitikz/tr/no input leads=true}\\else\\pgfkeys{/tikz/circuitikz/tr/no input leads=false}\\fi}}\n");
    keys.push("amp plus/.style={/tikz/circuitikz/amplifiers/plus={#1}}".into());
    keys.push("amp minus/.style={/tikz/circuitikz/amplifiers/minus={#1}}".into());
    keys.push("amp symbol font/.style={/tikz/circuitikz/amplifiers/symbol font={#1}}".into());
    keys.push("no schottky base/.style={/tikz/circuitikz/tr/schottky base=false}".into());
    keys.push("gridlines/.style={/tikz/circuitikz/blocks/gridnode/gridlines=#1}".into());
    keys.push("current arrow scale/.style={/tikz/circuitikz/current arrow scale=#1}".into());
    // `to[amp, >]`: an arrow into the block
    keys.push("bare arrow/.code={\\def\\voltik@gt{1}}".into());
    // a cute choke's core: its thickness and one or two lines
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, bipoles/cutechoke/cthick/.initial=1, bipoles/cutechoke/lines/.initial=1}\n");
    for (name, lines) in [("onelinechoke", 1), ("twolineschoke", 2)] {
        keys.push(format!("{name}/.style={{/tikz/circuitikz/bipoles/cutechoke/lines={lines}}}"));
        s.push_str(&format!("\\pgfkeys{{/tikz/circuitikz/{name}/.style={{/tikz/circuitikz/bipoles/cutechoke/lines={lines}}}}}\n"));
    }
    // the miscellaneous bipoles' own settings
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, bipoles/mic/bar thickness/.initial=1, sparkgap end arrow/.initial=default, sparkgap/gap/.initial=0.05, bipoles/noise sources/fillcolor/.initial=, bipoles/currtap/dot size/.initial=0.5, bipoles/currtap/fill/.initial=default, bipoles/currtap/color/.initial=default, bipoles/currtap/thickness/.initial=default, bipoles/currtap/dash/.initial=default, transistor circle/color/.initial=default, transistor circle/dash/.initial=default, sources/symbol/rotate/.initial=90, csources/symbol/rotate/.initial=90, csources/symbol/thickness/.initial=1, bipoles/wfuse/dots/.initial=true, bipoles/wfuse/shape/.initial=ocirc}\n");
    // settings of components that a `to[…]` or a style can give too (`bipoles/oscope/width=1`)
    for key in [
        "bipoles/cutechoke/cthick",
        "bipoles/oscope/waveform",
        "bipoles/oscope/width",
        "bipoles/oscope/height",
        "bipoles/oosource/width",
        "bipoles/oosource/height",
        "bipoles/mic/bar thickness",
        "bipoles/wfuse/dots",
        "sparkgap end arrow",
        "sparkgap/gap",
        "nodes width",
        "dashed blocks pattern",
        "blocks dc segments",
        "blocks dc in segments",
        "blocks dc out segments",
        "blocks/filter grid/color",
        "blocks/filter grid/dash",
        "blocks/filter grid/thickness",
        "blocks/filter grid/opacity",
        "blocks/filter plot/relative thickness",
        "blocks/filter plot/rounding",
        "ideal filter number low",
        "ideal filter number high",
        "bipoles/dcisource/angle",
        "bipoles/isourcesin/angle",
        "bipoles/smeter/index position",
        "bipoles/qmeter/index position",
        "sources/symbol/sign rotation",
        "bipoles/vsourceam/inner plus",
        "bipoles/vsourceam/inner minus",
        "bipoles/vsourceam/margin",
        "bipoles/cvsourceam/inner plus",
        "bipoles/cvsourceam/inner minus",
        "bipoles/cvsourceam/margin",
        "opto start arrow",
        "opto end arrow",
        "switch start arrow",
        "switch end arrow",
        "tunable start arrow",
        "tunable end arrow",
        "wiper start arrow",
        "wiper end arrow",
        "gto gate start arrow",
        "gto gate end arrow",
        "opto arrows/color",
        "opto arrows/dash",
        "opto arrows/relative thickness",
        "switch arrows/color",
        "switch arrows/dash",
        "switch arrows/relative thickness",
        "bipoles/noise sources/fillcolor",
        "bipoles/currtap/dot size",
        "bipoles/currtap/fill",
        "bipoles/currtap/color",
        "bipoles/currtap/thickness",
        "bipoles/currtap/dash",
        "bipoles/wfuse/shape",
        "sources/symbol/thickness",
        "sources/symbol/rotate",
        "csources/symbol/thickness",
        "csources/symbol/rotate",
        "sources/symbol/prim rot",
        "sources/symbol/sec rot",
        "sources/symbol/tert rot",
        "sources/symbol/upper rot",
        "sources/symbol/delta rot",
        "sources/symbol/wye rot",
        "sources/symbol/eyw rot",
        "sources/symbol/xdelta rot",
        "sources/symbol/zig rot",
        "sources/symbol/delta scale",
        "sources/symbol/wye scale",
        "sources/symbol/eyw scale",
        "sources/symbol/xdelta scale",
        "sources/symbol/zig scale",
        "ootrans size",
    ] {
        keys.push(format!("{key}/.style={{/tikz/circuitikz/{key}={{#1}}}}"));
    }
    // subcircuits: a path with named anchors, used by one of them (`\optovishay{one}{in 1}`)
    s.push_str("\\def\\ctikzsubcircuitdef#1#2#3{\\expandafter\\gdef\\csname voltik@sc@#1\\endcsname##1{#3}}\n");
    s.push_str("\\def\\ctikzsubcircuitactivate#1{\\expandafter\\gdef\\csname #1\\endcsname##1##2{\\pgfextra{\\pgfkeys{/tikz/voltik/subcircuit={#1}{##2}}} ++(\\voltik@scx,\\voltik@scy) \\csname voltik@sc@#1\\endcsname{##1}}}\n");
    // `\ctikztunablearrow[options]{thickness}{length}{angle}{name}`: an arrow across a component
    s.push_str("\\newcommand{\\ctikztunablearrow}[5][]{\\pgfkeys{/tikz/voltik/tunable arrow={#1}{#2}{#3}{#4}{#5}}}\n");
    // the poles by `bipole/nodes/left` and `right`, as `bipole nodes`
    s.push_str("\\pgfkeys{/tikz/circuitikz/bipole/nodes/left/.code={\\def\\voltik@ps{#1}}, /tikz/circuitikz/bipole/nodes/right/.code={\\def\\voltik@pe{#1}}}\n");
    // what was left of the transistors, crossings and transformer cores
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, transistor solderdot scale/.initial=0.7, tripoles/nmosd/depletion color/.initial=default, tripoles/pmosd/depletion color/.initial=default, tr gap fill/.initial=none, tripoles/hemt/split gate/.initial=, crossing vertical/color/.initial=default, crossing vertical/dash/.initial=default, crossing vertical/relative thickness/.initial=1, transformer core/color/.initial=default, transformer core/dash/.initial=default, transformer core/relative thickness/.initial=1}\n");
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, american or shape/.initial=pointy, ieeestd ports/not radius/.initial=0.154, ieeestd ports/xor bar distance/.initial=0.192, ieeestd ports/xor leads in/.initial=0.192}\n");
    // the blocks: their dashes, the dc lines, and the plots of the filters
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, blocks/inner dashed/.initial=false, dashed blocks pattern/.initial={{1mm}{1mm}}, blocks dc segments/.initial=1, blocks dc in segments/.initial=, blocks dc out segments/.initial=, blocks/filter grid/color/.initial=default, blocks/filter grid/dash/.initial={{0.04cm}{0.04cm}}, blocks/filter grid/thickness/.initial=0.5, blocks/filter grid/opacity/.initial=0.3, blocks/filter plot/relative thickness/.initial=2, blocks/filter plot/rounding/.initial=0.15, blocks/ideal numbers/.initial=false, ideal filter number low/.initial=0, ideal filter number high/.initial=1, inner blocks dashed/.style={/tikz/circuitikz/blocks/inner dashed=#1}, inner blocks dashed/.default=true, ideal filter numbers/.style={/tikz/circuitikz/blocks/ideal numbers=#1}, ideal filter numbers/.default=true}\n");
    keys.push("inner blocks dashed/.style={/tikz/circuitikz/blocks/inner dashed=#1}, inner blocks dashed/.default=true".into());
    keys.push("ideal filter numbers/.style={/tikz/circuitikz/blocks/ideal numbers=#1}, ideal filter numbers/.default=true".into());
    keys.push("tr gap fill/.style={/tikz/circuitikz/tr gap fill={#1}}".into());
    keys.push("en amp text A/.style={/tikz/circuitikz/en amp text={$\\mathstrut{\\triangleright}\\,\\mathrm{A}$}}".into());
    // the sources' and meters' own settings
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, bipoles/dcisource/angle/.initial=80, bipoles/isourcesin/angle/.initial=90, bipoles/smeter/index position/.initial=80, bipoles/qmeter/index position/.initial=80, sources/symbol/sign rotation/.initial=default");
    for source in ["vsourceam", "cvsourceam"] {
        s.push_str(&format!(", bipoles/{source}/inner plus/.initial=, bipoles/{source}/inner minus/.initial=, bipoles/{source}/margin/.initial=0.7"));
    }
    s.push_str("}\n");
    // the arrows' tips and looks (`opto end arrow`, `switch arrows/color`…)
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd");
    for group in ["opto", "switch", "wiper", "gto gate"] {
        s.push_str(&format!(", {group} start arrow/.initial=default, {group} end arrow/.initial=default"));
    }
    for style in ["opto arrows", "switch arrows"] {
        s.push_str(&format!(", {style}/color/.initial=default, {style}/dash/.initial=default, {style}/relative thickness/.initial=1"));
    }
    s.push_str("}\n");
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, transistors/gate thickness/.initial=1, transistors/rounded caps/.initial=false}\n");
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, tunable start arrow/.initial=default, tunable end arrow/.initial=default, bipoles/fix tunable direction/.initial=true, voltage shift sources adjust/.initial=0.5}\n");
    // the dc and ac symbols, with their sizes and text above them
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd, dc symbol/width/.initial=0.3, dc symbol/height/.initial=0.15, dc symbol/text offset/.initial=6pt, dc symbol/segments/.initial=2, dc symbol/relative thickness/.initial=1, ac symbol/width/.initial=0.3, ac symbol/height/.initial=0.15, ac symbol/text offset/.initial=6pt}\n");
    for symbol in ["dc symbol", "ac symbol"] {
        keys.push(format!("{symbol}/.style={{shape={symbol}}}"));
    }
    keys.push("dc symbol segments/.style={/tikz/circuitikz/dc symbol/segments=#1}".into());
    // the chips, and their options
    keys.push("dipchip/.style={shape=dipchip, voltik/class fill=chips}".into());
    keys.push("qfpchip/.style={shape=qfpchip, voltik/class fill=chips}".into());
    for key in
        ["num pins", "external pins width", "external pad fraction", "draw only pins", "draw only left pins", "draw only right pins", "draw only bottom pins", "draw only top pins"]
    {
        keys.push(format!("{key}/.style={{/tikz/circuitikz/multipoles/{key}={{#1}}}}"));
    }
    for (key, setting) in [("hide numbers", "numbers=false"), ("show numbers", "numbers=true"), ("no topmark", "topmark=false"), ("topmark", "topmark=true")] {
        keys.push(format!("{key}/.style={{/tikz/circuitikz/multipoles/{setting}}}"));
    }
    keys.push("straight numbers/.style={/tikz/circuitikz/multipoles/straight numbers=true}".into());
    keys.push("rotated numbers/.style={/tikz/circuitikz/multipoles/straight numbers=false}".into());
    keys.push("legacy transistors text/.style={/tikz/circuitikz/tr/legacy text=true}".into());
    keys.push("no ferroel gate/.style={/tikz/circuitikz/tr/ferroel gate=false}".into());
    // `tripoles/mos style/arrows`: every MOSFET with its arrow (`arrowmos`)
    keys.push("/tikz/circuitikz/tripoles/mos style/arrows/.style={/tikz/circuitikz/tr/arrowmos=true}".into());
    keys.push("/tikz/circuitikz/tripoles/mos style/no arrows/.style={/tikz/circuitikz/tr/arrowmos=false}".into());
    keys.push("/tikz/circuitikz/tripoles/mos style/.code={\\pgfkeys{/tikz/circuitikz/tripoles/mos style/#1}}".into());
    // `tripoles/pmos style/emptycircle`: a PMOS's circle empty, or none (`nocircle`)
    for (style, empty, none) in [("emptycircle", "true", "false"), ("nocircle", "false", "true"), ("fullcircle", "false", "false")] {
        keys.push(format!("/tikz/circuitikz/tripoles/pmos style/{style}/.style={{/tikz/circuitikz/tr/emptycircle={empty}, /tikz/circuitikz/tr/nocircle={none}}}"));
    }
    // (for the double precision of TeX's arithmetic: nothing here)
    keys.push("use fpu reciprocal/.code={}".into());
    // the logic ports by the `logic ports` style (`and port`: American, European or IEEE), and their leads
    for kind in ["and", "or", "nand", "nor", "xor", "xnor", "buffer", "not"] {
        keys.push(format!("{kind} port/.style={{voltik/logic port={kind}}}"));
    }
    for style in ["american", "european", "ieee"] {
        keys.push(format!("{style} ports/.style={{/tikz/circuitikz/logic ports={style}}}"));
    }
    keys.push("input leads/.style={/tikz/circuitikz/tr/no input leads=false, /tikz/circuitikz/tr/no leads=false}".into());
    keys.push("output leads/.style={/tikz/circuitikz/tr/no output leads=false, /tikz/circuitikz/tr/no leads=false}".into());
    keys.push("all leads/.style={/tikz/circuitikz/tr/no input leads=false, /tikz/circuitikz/tr/no output leads=false, /tikz/circuitikz/tr/no leads=false}".into());
    // `n=R1`: a component's name, as `name=R1`
    keys.push("n/.style={name={#1}}".into());
    // `t=…`: the text inside an instrument
    keys.push("t/.code={\\def\\voltik@t{#1}}".into());
    for (a, start) in POLES {
        for (b, end) in POLES {
            if a.is_empty() && b.is_empty() {
                continue;
            }
            keys.push(format!("{a}-{b}/.style={{voltik/pole={{{start}}}{{{end}}}}}"));
        }
    }
    keys.push("bipole nodes/.style 2 args={voltik/pole={#1}{#2}}".into());
    // the manual's own voltages, currents and flows (its "advanced voltages"): a macro queued for the
    // component (`vif queue add={macro}{a}{b}{c}`), run with its name once it is drawn
    keys.push("vif queue add/.code={\\voltik@vqadd#1}".into());
    keys.push("use auto vif/.code={}".into());
    // voltages, currents and flows, with the side (`^`, `_`) and the direction (`>`, `<`)
    for kind in ["v", "i", "f"] {
        for variant in ["", "^", "_", ">", "<", "^>", "^<", "_>", "_<", ">^", ">_", "<^", "<_"] {
            keys.push(format!("{kind}{variant}/.style={{voltik/vif={{{kind}}}{{{variant}}}{{#1}}}}"));
        }
    }
    // the settings every class has, and the settings a `to[…]` can give too
    s.push_str("\\pgfkeys{/tikz/circuitikz/.cd");
    let mut settings = vec![
        "bipoles/length".to_string(),
        "bipoles/thickness".into(),
        "resistors/zigs".into(),
        "inductors/coils".into(),
        "voltage".into(),
        "voltage shift".into(),
        "voltage/bump b".into(),
        "voltage/distance from node".into(),
        "current/distance".into(),
        "flow/distance".into(),
        "l2 halign".into(),
        "l2 valign".into(),
        "a2 halign".into(),
        "a2 valign".into(),
        "bipole/is voltage".into(),
        "bipole/is current".into(),
        "bipole/override source vif".into(),
        "thyristor style".into(),
        "led arrows".into(),
        "pd arrows".into(),
        "diode whiskers".into(),
        "tjumper connections".into(),
        "component text".into(),
        "logic ports".into(),
        "bipoles/jumper/shape".into(),
        "bipoles/cuteswitch/shape".into(),
        "left text distance".into(),
    ];
    // every class of the manual has them, even before voltik draws its components, so that
    // `\ctikzset{diodes/scale=…}` is a setting and not TikZ's `scale`
    for class in [
        "resistors",
        "capacitors",
        "inductors",
        "grounds",
        "power supplies",
        "connectors",
        "diodes",
        "batteries",
        "sources",
        "csources",
        "instruments",
        "mechanicals",
        "misc",
        "blocks",
        "tubes",
        "amplifiers",
        "switches",
        "logic ports",
        "transistors",
        "flipflops",
        "muxdemuxes",
        "chips",
        "displays",
        "default",
    ] {
        for (key, initial) in CLASS_SETTINGS {
            s.push_str(&format!(",\n{class}/{key}/.initial={initial}"));
            settings.push(format!("{class}/{key}"));
        }
    }
    s.push_str("}\n");
    for key in settings {
        keys.push(format!("{key}/.style={{/tikz/circuitikz/{key}={{#1}}}}"));
    }
    // the `fill` of a `to` (or of its path) fills its component too: TikZ's own `fill`, kept under
    // another name, after noting its value
    s.push_str("\\expandafter\\let\\csname pgfk@/tikz/voltik@fill/.@cmd\\expandafter\\endcsname\\csname pgfk@/tikz/fill/.@cmd\\endcsname\n");
    s.push_str("\\tikzset{fill/.code={\\def\\voltik@fill{#1}\\pgfkeys{/tikz/voltik@fill={#1}}}}\n");
    // and its `name` names the component, whose anchors are then coordinates (`R1.north`)
    s.push_str("\\expandafter\\let\\csname pgfk@/tikz/voltik@name/.@cmd\\expandafter\\endcsname\\csname pgfk@/tikz/name/.@cmd\\endcsname\n");
    s.push_str("\\tikzset{name/.code={\\def\\voltik@name{#1}\\pgfkeys{/tikz/voltik@name={#1}}}}\n");
    // the manual's text helpers: a text flipped back in a flipped node (`\\ctikzflipx`), without its
    // depth (`\\killdepth`): the text itself
    s.push_str("\\def\\ctikzflipx#1{\\scalebox{-1}[1]{#1}}\\def\\ctikzflipy#1{\\scalebox{1}[-1]{#1}}\\def\\ctikzflipxy#1{\\scalebox{-1}[-1]{#1}}\\def\\killdepth#1{#1}\n");
    // siunitx's unit macros, for a unit written outside `\SI` (which is rewritten before, in `convert`)
    for (name, symbol) in label::UNITS {
        s.push_str(&format!("\\def\\{name}{{{symbol}}}"));
    }
    s.push('\n');
    // and `transform shape` scales the components on paths too, as it does nodes
    s.push_str("\\def\\voltik@ts{0}\\expandafter\\let\\csname pgfk@/tikz/voltik@ts/.@cmd\\expandafter\\endcsname\\csname pgfk@/tikz/transform shape/.@cmd\\endcsname\n");
    s.push_str("\\tikzset{transform shape/.code={\\def\\voltik@ts{1}\\pgfkeys{/tikz/voltik@ts}}}\n");
    s.push_str(
        "\\def\\voltik@sa{}\\def\\voltik@na{}\\def\\voltik@cs{}\\def\\voltik@rs{}\\def\\voltik@md{}\\def\\voltik@mdl{}\\def\\voltik@mdbg{}\\def\\voltik@bp{}\\def\\voltik@db{}\\def\\voltik@tr{}\\def\\voltik@bn{}\\def\\voltik@coil{-0}\\def\\voltik@prefix{}\\def\\voltik@named{}\n",
    );
    s.push_str("\\def\\voltik@ff{}\\def\\voltik@ffd{}\n");
    s.push_str("\\pgfkeys{/tikz/voltik/shapes}\n");
    // the node components' sizes (`tripoles/op amp/width`), at their defaults
    // (the defaults of the measured sizes, not of a tube's wide measures)
    let mut sizes: Vec<&nodes::Param> = nodes::params().filter(|p| !p.name.contains('~')).collect();
    sizes.dedup_by_key(|p| p.setting);
    for p in sizes {
        s.push_str(&format!("\\pgfkeys{{/tikz/circuitikz/{}/.initial={}}}\n", p.setting, p.default));
    }
    s.push_str("\\tikzset{\n");
    for k in keys {
        s.push_str(&k);
        s.push_str(",\n");
    }
    s.push_str("}\n");
    // the circuit's own defaults, reset at each `to`
    s.push_str(
        "\\def\\voltik@gt{0}\\def\\voltik@kind{short}\\def\\voltik@fit{0}\\def\\voltik@up{}\\def\\voltik@ps{}\\def\\voltik@pe{}\\def\\voltik@mirror{0}\\def\\voltik@invert{0}\\def\\voltik@l{}\\def\\voltik@t{}\\def\\voltik@a{}\\def\\voltik@fill{}\\def\\voltik@name{}\\def\\voltik@v{}\\def\\voltik@i{}\\def\\voltik@f{}\\def\\voltik@llines{1}\\def\\voltik@alines{1}\\def\\voltik@von{0}\\def\\voltik@ion{0}\\def\\voltik@vpassive{}\\def\\voltik@ipassive{}\\def\\voltik@voverride{}\\def\\voltik@ioverride{}\\def\\voltik@fon{0}\\def\\voltik@vq{}\\def\\voltik@vvar{}\\def\\voltik@ivar{}\\def\\voltik@fvar{}\n",
    );
    s
}

/// Converts a CircuiTikZ figure (a `circuitikz` environment, or any TikZ figure using its
/// components) to SVG.
pub fn convert(code: &str) -> Result<Output, Error> {
    convert_with(code, &Options::default())
}

/// [`convert`] with tikra's options (the language of the figure in words, the animation).
pub fn convert_with(code: &str, options: &Options) -> Result<Output, Error> {
    tikra::convert_with_libraries(&environments(code), options, &[&LIBRARY])
}

/// `circuitikz` environments as the `tikzpicture`s they are, after the options the document gave
/// the package (`\usepackage[RPvoltages, american]{circuitikz}`) as styles.
fn environments(code: &str) -> String {
    let code = code.replace("\\begin{circuitikz}", "\\begin{tikzpicture}").replace("\\end{circuitikz}", "\\end{tikzpicture}");
    // siunitx anywhere (`\\SI{12}{\\volt}` in a node's text), in math whether the text is or not
    let code = label::siunitx(&code);
    let styles: Vec<&str> = package_options(&code).iter().filter_map(|o| option_style(o)).collect();
    match (styles.is_empty(), code.find("\\begin{document}")) {
        (false, Some(at)) => {
            let at = at + "\\begin{document}".len();
            format!("{}\\tikzset{{{}}}{}", &code[..at], styles.join(","), &code[at..])
        }
        _ => code,
    }
}

/// The options of `\usepackage[…]{circuitikz}`, if the code loads it.
fn package_options(code: &str) -> Vec<String> {
    let Some(end) = code.find("]{circuitikz}") else { return Vec::new() };
    let Some(start) = code[..end].rfind("\\usepackage[") else { return Vec::new() };
    code[start + "\\usepackage[".len()..end].split(',').map(|o| o.trim().to_string()).collect()
}

/// The style a package option stands for.
fn option_style(option: &str) -> Option<&'static str> {
    Some(match option {
        "RPvoltages" => "voltage dir=RP",
        "EFvoltages" => "voltage dir=EF",
        "oldvoltagedirection" => "voltage dir=old",
        "nooldvoltagedirection" => "voltage dir=noold",
        "american" => "american",
        "european" => "european",
        "americanvoltages" | "americanvoltage" => "american voltages",
        "europeanvoltages" | "europeanvoltage" => "european voltages",
        "americancurrents" | "americancurrent" => "american currents",
        "europeancurrents" | "europeancurrent" => "european currents",
        "straightvoltages" => "straight voltages",
        "americanresistors" | "americanresistor" => "american resistors",
        "europeanresistors" | "europeanresistor" => "european resistors",
        "americaninductors" | "americaninductor" => "american inductors",
        "europeaninductors" | "europeaninductor" => "european inductors",
        "cuteinductors" | "cuteinductor" => "cute inductors",
        "emptydiode" | "emptydiodes" => "empty diodes",
        "fulldiode" | "fulldiodes" => "full diodes",
        "strokediode" | "strokediodes" => "stroke diodes",
        "legacytransistorstext" => "legacy transistors text",
        "americanports" => "american ports",
        "europeanports" => "european ports",
        "ieeeports" => "ieee ports",
        _ => return None,
    })
}
