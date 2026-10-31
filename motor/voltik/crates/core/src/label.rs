//! Labels (`l`) and annotations (`a`) of path-style components.
//!
//! A label goes on one side of the component (above it when the path goes left to right; `l_` puts it
//! on the other side), an annotation on the other side by default. `label/align` says how the text
//! turns with the component: `straight` never, `rotate` always (along the line), `smart` (the default)
//! only when the line is far from horizontal and vertical.

use tikra::extension::{Context, Error, Point, Tokens};

use crate::bipole::{setting, Frame, Placed};

/// The space between the component's band and its label (`.75ex` of the 10 pt font, in cm).
const LABEL_SPACE: f64 = 0.75 * 4.305_56 * tikra::extension::PT;
/// How far out of the component's outline its labels start (the "border margin").
const MARGIN: f64 = 1.1;
/// Straight labels, from the band: the baseline of a label above, of a label below, and the text's
/// near side at the left and right (measured on TeX's drawings, in cm).
const ABOVE: f64 = 2.83 * tikra::extension::PT;
const BELOW: f64 = 10.03 * tikra::extension::PT;
const SIDE: f64 = 1.35 * tikra::extension::PT;
/// TikZ's default inner sep of a node, `.3333em` of the 10 pt font.
const INNER_SEP: f64 = 3.333 * tikra::extension::PT;

/// Records a label (`which` = `l`) or an annotation (`a`) on side `side` (1 above, -1 below, 0 the
/// default of its kind).
pub fn set(cx: &mut Context, which: &str, side: i32, text: Tokens) {
    cx.define(&format!("voltik@{which}"), text);
    cx.define(&format!("voltik@{which}side"), Tokens::text(&side.to_string()));
}

/// `voltik/label={which}{side}{text}`.
pub fn k_label(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    let a = v.unwrap_or_default().arguments(3);
    let which = cx.expand(a[0].clone())?;
    let side = cx.expand(a[1].clone())?.parse().unwrap_or(0);
    // `l2=A and B`, `a2=…`: a label of two lines
    let (which, lines) = match which.strip_suffix('2') {
        Some(w) => (w.to_string(), "2"),
        None => (which, "1"),
    };
    set(cx, &which, side, a[2].clone());
    cx.define_text(&format!("voltik@{which}lines"), lines, false);
    Ok(())
}

/// The nodes of the label and the annotation, if any, as path operations.
pub fn nodes(cx: &mut Context, placed: &Placed, frame: &Frame, unit: f64) -> Result<String, Error> {
    let _ = unit;
    let align = setting(cx, "label/align")?;
    let name = cx.macro_text("voltik@name");
    let mut s = String::new();
    for (which, default_side, distance_key) in [("l", 1, "label distance"), ("a", -1, "annotation distance")] {
        let node = if which == "l" { "label" } else { "annotation" };
        // the node is `R1label` for a component named `R1`, styled by `bipole label style`
        let mut options = format!("/tikz/circuitikz/bipole {node} style,");
        if !name.is_empty() {
            options = format!("name={{{name}{node}}},{options}");
        }
        let text = cx.macro_body(&format!("voltik@{which}"));
        if text.trim().is_empty() {
            continue;
        }
        let side = match cx.macro_text(&format!("voltik@{which}side")).parse::<i32>().unwrap_or(0) {
            0 => default_side,
            n => n.signum(),
        };
        // the label stays on its side when the component is mirrored or inverted
        let side = side as f64;
        let extra = match setting(cx, distance_key)? {
            d if d.is_empty() => 0.0,
            d => cx.length(Tokens::text(&d)).unwrap_or(0.0),
        };
        let stacked = cx.macro_text(&format!("voltik@{which}lines")) == "2";
        // two lines: a `tabular` as the manual's, its middle on the math axis
        let text = match text.to_text().split_once(" and ") {
            Some((first, second)) if stacked => {
                let column = match setting(cx, &format!("{which}2 halign"))?.as_str() {
                    "c" => 'c',
                    "r" => 'r',
                    _ => 'l',
                };
                format!("\\begin{{tabular}}{{@{{}}{column}@{{}}}}{}\\\\{}\\end{{tabular}}", units(first), units(second))
            }
            _ => units(&text.to_text()),
        };
        // the direction from the component to the label, on the canvas
        let outward = placed.angle + 90.0 * side;
        // turned in the picture's own coordinates: `transform shape` turns it the rest of the way
        let local = frame.angle(placed.along);
        let rotate = match align.as_str() {
            "straight" => false,
            "rotate" => true,
            _ => {
                let m = local.rem_euclid(90.0);
                m > 20.0 && m < 70.0
            }
        };
        if stacked && !rotate {
            // two lines, as the manual's `tabular` (`l2 halign`, l, c or r, aligns them): `l2 valign`
            // (b, c, t) puts the first one's baseline above, around or at the label's (measured)
            let valign = setting(cx, &format!("{which}2 valign"))?;
            // (below the line, TeX sets them the same whatever `valign` says)
            let below = (225.0..315.0).contains(&outward.rem_euclid(360.0));
            // (from the `tabular`'s baseline, half a line under the first one's)
            let lift = match valign.as_str() {
                _ if below => 0.0,
                "b" => 5.9,
                "t" => -6.1,
                _ => 0.0,
            } * tikra::extension::PT;
            let reach = placed.reach(side) * MARGIN + extra;
            let (anchor, at) = match outward.rem_euclid(360.0) {
                a if (45.0..135.0).contains(&a) => ("base", placed.mid.plus(Point::new(0.0, reach + 3.2 * tikra::extension::PT + lift))),
                a if (225.0..315.0).contains(&a) => ("base", placed.mid.plus(Point::new(0.0, -reach - 16.1 * tikra::extension::PT + lift))),
                a if (135.0..225.0).contains(&a) => ("base east", placed.mid.plus(Point::new(-reach - 3.6 * tikra::extension::PT + INNER_SEP, lift - 2.1 * tikra::extension::PT))),
                _ => ("base west", placed.mid.plus(Point::new(reach + 3.6 * tikra::extension::PT - INNER_SEP, lift - 2.1 * tikra::extension::PT))),
            };
            s.push_str(&format!(" node[{options}anchor={anchor}] at {} {{{text}}}", frame.coordinate(at)));
            record(cx, &name, node, &tex_anchor(local, side), 0.0);
            continue;
        }
        if rotate {
            // along the line, read left to right; `mid` sits on the line through the label's middle,
            // further out below the line than above it
            // from 0 to 360, upside down (and turned round) only strictly between 90 and 270
            let a = local.rem_euclid(360.0);
            let flipped = a > 90.5 && a < 269.5;
            let turn = if flipped { a - 180.0 } else { a };
            let below = (side < 0.0) != flipped;
            let space = if below { 2.25 } else { 1.5 } * LABEL_SPACE;
            let reach = placed.reach(side) * MARGIN + space + extra;
            let at = placed.mid.plus(placed.normal.scale(side * reach));
            s.push_str(&format!(" node[{options}anchor=mid,rotate={}] at {} {{{text}}}", round(turn), frame.coordinate(at)));
            record(cx, &name, node, "mid", turn);
        } else {
            // the text keeps its baseline (above and below) or the middle of its x-height (at the
            // sides) at a fixed distance from the component's band, whatever its letters
            let border = placed.mid.plus(placed.normal.scale(side * (placed.reach(side) * MARGIN + extra)));
            // the node keeps TikZ's inner sep, as CircuiTikZ's do (it counts in the picture's size):
            // at the sides, its anchor is that much nearer, so that the text stays where it was
            let level = (local / 90.0 - (local / 90.0).round()).abs() < 1e-4;
            // (under `transform shape` the text turns with the picture: its own frame decides)
            let shaped = cx.macro_text("voltik@ts") == "1";
            let side_angle = if shaped { local + 90.0 * side } else { outward };
            let (anchor, shift) = match side_angle.rem_euclid(360.0) {
                // a slanted line: the label's border point that faces the line, as TeX anchors it
                _ if !level => (tex_anchor(local, side), Point::new(0.0, 0.0)),
                a if (45.0..135.0).contains(&a) => ("base".into(), Point::new(0.0, ABOVE)),
                a if (225.0..315.0).contains(&a) => ("base".into(), Point::new(0.0, -BELOW)),
                a if (135.0..225.0).contains(&a) => ("mid east".into(), Point::new(-SIDE + INNER_SEP, 0.0)),
                _ => ("mid west".into(), Point::new(SIDE - INNER_SEP, 0.0)),
            };
            // (turned with the picture under `rotate` and `transform shape`, as the text is)
            let turn = if shaped { (placed.angle - local).to_radians() } else { 0.0 };
            let at = border.plus(Point::new(shift.x * turn.cos() - shift.y * turn.sin(), shift.x * turn.sin() + shift.y * turn.cos()));
            s.push_str(&format!(" node[{options}anchor={anchor}] at {} {{{text}}}", frame.coordinate(at)));
            record(cx, &name, node, &tex_anchor(local, side), 0.0);
        }
    }
    Ok(s)
}

/// What `\ctikzgetanchor{R1}{label}` and `\ctikzgetdirection{R1label}` answer.
fn record(cx: &mut Context, name: &str, node: &str, anchor: &str, turn: f64) {
    if !name.is_empty() {
        let name = format!("{}{name}", cx.macro_text("voltik@prefix"));
        cx.define_text(&format!("voltik@anchor@{name}@{node}"), anchor, true);
        // in whole degrees, cut toward zero from pgf's angle, a little over the exact one
        cx.define_text(&format!("voltik@direction@{name}{node}"), &format!("{}", (turn + PGF_ANGLE).trunc() as i64), true);
    }
}

/// How much over the exact angle pgf's own comes out, as `\ctikzgetdirection` shows (30 for a line
/// at 29.9°, 63 at 63.4°).
const PGF_ANGLE: f64 = 0.15;

/// The anchor `\ctikzgetanchor` answers for a label on side `side` of a line at `angle` (degrees,
/// in the picture's coordinates) that is not turned with it: a compass anchor facing the line when
/// it is level or upright, else the angle that faces it (the label's at the line's angle less 90,
/// the other side's half a turn round from that).
fn tex_anchor(angle: f64, side: f64) -> String {
    let a = angle.rem_euclid(360.0);
    if (a / 90.0 - (a / 90.0).round()).abs() < 1e-4 {
        return match ((a + 90.0 * side).rem_euclid(360.0) / 90.0).round() as i64 % 4 {
            1 => "base",
            3 => "north",
            2 => "mid east",
            _ => "mid west",
        }
        .to_string();
    }
    let label = (a - 90.0).round() as i64;
    let anchor = if side > 0.0 {
        label
    } else if label + 180 >= 360 {
        label - 180
    } else {
        label + 180
    };
    anchor.to_string()
}

fn round(x: f64) -> String {
    format!("{}", x.round() as i64)
}

/// The text of a label, a voltage or a current, with siunitx written out: the manual's shorthand
/// `value<unit>` (`R=20<\ohm>`) for the whole text, and `\SI{1}{\kilo\ohm}`, `\qty{…}{…}`,
/// `\si{…}`, `\unit{…}` and `\num{…}` anywhere in it, as math (`$20\,\mathrm{\Omega}$`). Other
/// text is left as it is.
pub fn units(text: &str) -> String {
    let t = text.trim();
    if let (Some(open), true) = (t.rfind('<'), t.ends_with('>')) {
        // a value already in math keeps its own dollars
        let value = t[..open].trim().trim_matches('$');
        let symbols = symbols(&t[open + 1..t.len() - 1]);
        return if value.is_empty() { format!("$\\mathrm{{{symbols}}}$") } else { format!("${value}\\,\\mathrm{{{symbols}}}$") };
    }
    siunitx(text)
}

/// `\SI`, `\qty`, `\si`, `\unit` and `\num` in a text, in math whether the text is or not there.
pub fn siunitx(text: &str) -> String {
    let mut out = String::new();
    let mut math = false;
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '$' {
            math = !math;
        }
        if c == '\\' {
            let name_end = rest[1..].find(|c: char| !c.is_ascii_alphabetic()).map_or(rest.len(), |e| e + 1);
            let name = &rest[1..name_end];
            let args = match name {
                "SI" | "qty" => 2,
                "si" | "unit" | "num" => 1,
                _ => 0,
            };
            if args > 0 {
                if let Some((values, after)) = arguments(&rest[name_end..], args) {
                    let body = match name {
                        // (siunitx leaves out an explicit plus)
                        "SI" | "qty" => format!("{}\\,\\mathrm{{{}}}", number_value(&values[0]), symbols(&values[1])),
                        "num" => values[0].clone(),
                        _ => format!("\\mathrm{{{}}}", symbols(&values[0])),
                    };
                    out.push_str(&if math { body } else { format!("${body}$") });
                    rest = after;
                    continue;
                }
            }
            out.push_str(&rest[..name_end]);
            rest = &rest[name_end..];
            continue;
        }
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// A quantity's value as siunitx prints it: no explicit plus, a zero before a bare point.
fn number_value(value: &str) -> String {
    let value = value.trim_start().trim_start_matches('+');
    match value.strip_prefix('-') {
        Some(rest) if rest.starts_with('.') => format!("-0{rest}"),
        _ if value.starts_with('.') => format!("0{value}"),
        _ => value.to_string(),
    }
}

/// `n` braced arguments at the start of `text`, and what follows them.
fn arguments(text: &str, n: usize) -> Option<(Vec<String>, &str)> {
    let mut rest = text;
    let mut values = Vec::new();
    for _ in 0..n {
        rest = rest.trim_start().strip_prefix('{')?;
        let mut depth = 1;
        let end = rest.char_indices().find(|&(_, c)| {
            depth += match c {
                '{' => 1,
                '}' => -1,
                _ => 0,
            };
            depth == 0
        })?;
        values.push(rest[..end.0].trim().to_string());
        rest = &rest[end.0 + 1..];
    }
    Some((values, rest))
}

/// The unit macros of siunitx and what they write, in math.
pub const UNITS: &[(&str, &str)] = &[
    ("ohm", "\\Omega"),
    ("farad", "F"),
    ("henry", "H"),
    ("volt", "V"),
    ("ampere", "A"),
    ("siemens", "S"),
    ("watt", "W"),
    ("hertz", "Hz"),
    ("second", "s"),
    ("coulomb", "C"),
    ("joule", "J"),
    ("tesla", "T"),
    ("weber", "Wb"),
    ("meter", "m"),
    ("metre", "m"),
    ("kilo", "k"),
    ("mega", "M"),
    ("giga", "G"),
    ("milli", "m"),
    ("micro", "\\mu "),
    ("nano", "n"),
    ("deci", "d"),
    ("bel", "B"),
    ("decibel", "dB"),
    ("pico", "p"),
    ("femto", "f"),
    ("degree", "{}^\\circ"),
    ("celsius", "{}^\\circ C"),
    ("percent", "\\%"),
    ("per", "/"),
    ("squared", "^2"),
    ("kohm", "k\\Omega"),
    ("Mohm", "M\\Omega"),
    ("mohm", "m\\Omega"),
    ("kV", "kV"),
    ("mV", "mV"),
    ("uV", "\\mu V"),
    ("mA", "mA"),
    ("uA", "\\mu A"),
    ("nA", "nA"),
    ("pF", "pF"),
    ("nF", "nF"),
    ("uF", "\\mu F"),
    ("mF", "mF"),
    ("mH", "mH"),
    ("uH", "\\mu H"),
    ("nH", "nH"),
    ("Hz", "Hz"),
    ("kHz", "kHz"),
    ("MHz", "MHz"),
    ("GHz", "GHz"),
    ("ms", "ms"),
    ("us", "\\mu s"),
    ("ns", "ns"),
    ("mW", "mW"),
    ("kW", "kW"),
];

/// The symbols of a siunitx unit (`\kilo\ohm`, `k\ohm`, `\mA`), for `\mathrm`.
fn symbols(unit: &str) -> String {
    let mut out = String::new();
    let mut rest = unit.trim();
    while let Some(c) = rest.chars().next() {
        if c != '\\' {
            if !c.is_whitespace() {
                out.push(c);
            }
            rest = &rest[c.len_utf8()..];
            continue;
        }
        let end = rest[1..].find(|c: char| !c.is_ascii_alphabetic()).map_or(rest.len(), |e| e + 1);
        // a control symbol (`\%`) is kept as it is
        if end == 1 {
            let next = rest[1..].chars().next().map_or(1, |c| 1 + c.len_utf8());
            out.push_str(&rest[..next]);
            rest = &rest[next..];
            continue;
        }
        let name = &rest[1..end];
        out.push_str(match UNITS.iter().find(|(n, _)| *n == name) {
            Some((_, symbol)) => symbol,
            None => name,
        });
        rest = &rest[end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::units;

    #[test]
    fn siunitx_shorthand() {
        assert_eq!(units(r"20<\ohm>"), r"$20\,\mathrm{\Omega}$");
        assert_eq!(units(r"1<\micro\farad>"), r"$1\,\mathrm{\mu F}$");
        assert_eq!(units(r"$R_1$"), r"$R_1$");
        assert_eq!(units("a < b"), "a < b");
        assert_eq!(units(r"\SI{4.7}{k\ohm}"), r"$4.7\,\mathrm{k\Omega}$");
        assert_eq!(units(r"$i=\SI{1}{\milli\ampere}$"), r"$i=1\,\mathrm{mA}$");
        assert_eq!(units(r"-t\si{\degree}"), r"-t$\mathrm{{}^\circ}$");
        assert_eq!(units(r"+H\si{\%}"), r"+H$\mathrm{\%}$");
        assert_eq!(units(r"\qty{2}{\kohm} and \num{3}"), r"$2\,\mathrm{k\Omega}$ and $3$");
    }
}
