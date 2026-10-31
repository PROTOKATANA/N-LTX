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

mod definitions;

use crate::bipole::setting;
use tikra::extension::{Context, KeyFn, Library, Point, Tokens};
pub use tikra::{png, Error, Locale, Options, Output, MAX_PIXELS};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The CircuiTikZ library, to give tikra next to other libraries.
pub static LIBRARY: Library = Library { name: "voltik", definitions: definitions::definitions, keys: KEYS };

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
pub fn environments(code: &str) -> String {
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
