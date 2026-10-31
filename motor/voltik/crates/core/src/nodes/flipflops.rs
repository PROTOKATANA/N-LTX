//! Flip-flops (manual 4.19): `flipflop`, a box `multipoles/flipflop/width` wide with three pins on
//! each side (`pin 1`…`pin 6`, as a DIP chip's) and one above and below (`up`, `down`), and what
//! `flipflop def` puts on each: a text (`t1`, `tu`), a clock wedge (`c2`) and a negation circle
//! (`n4`, a node of its own, `FF-N4`). Only the pins with one of them stick out.
//!
//! Measured on what CircuiTikZ 1.8.7 draws (`tools/oracle/`): the pins are
//! `multipoles/flipflop/pin spacing` apart and the box one and a half spacings high on each side of
//! them, in `bipoles/length` times `flipflops/scale`; the texts, in `multipoles/flipflop/font`
//! (`fontud` above and below), stay upright when it turns unless `rotated numbers`.

use tikra::extension::{Context, Error, NodeShape, PathSegment, Point, ShapeBorder, ShapeInput, ShapePaint, ShapePath, ShapeText, TextAnchor, Tokens};

use crate::bipole::{number, setting};

/// Where a pin's text goes from the pin on the border (cm, in `\small` on the sides and `\tiny`
/// above and below): inside, by its baseline.
const SIDE_IN: f64 = 0.1084;
const SIDE_DOWN: f64 = 0.0976;
const ABOVE_BOTTOM: f64 = 0.0956;
const BELOW_TOP: f64 = 0.2392;

/// The pins: their name in `flipflop def` (`1`, `u`) and in the anchors, by side.
const PINS: &[(&str, &str)] = &[("1", "pin 1"), ("2", "pin 2"), ("3", "pin 3"), ("4", "pin 4"), ("5", "pin 5"), ("6", "pin 6"), ("u", "up"), ("d", "down")];

/// A setting that is TeX (`\small`), unexpanded.
fn tex(cx: &mut Context, key: &str) -> Result<String, Error> {
    cx.expand(Tokens::text(&format!("\\unexpanded\\expandafter{{\\romannumeral-`0\\pgfkeysvalueof{{/tikz/circuitikz/{key}}}}}")))
}

/// The measured box of a flip-flop.
struct FlipflopBox {
    x: f64,
    h: f64,
    s: f64,
    thick: f64,
    thin: f64,
    lead: f64,
    clock: f64,
    radius: f64,
    fonts: [String; 2],
    upright: bool,
    turned: Option<(f64, f64)>,
}

/// The sizes `flipflop` is drawn at.
fn flipflop_box(cx: &mut Context, input: &ShapeInput) -> Result<FlipflopBox, Error> {
    let length = setting(cx, "bipoles/length")?;
    let u = cx.length(Tokens::text(&length)).unwrap_or(1.4) * number(cx, "flipflops/scale", 1.0);
    let s = number(cx, "multipoles/flipflop/pin spacing", 0.6) * u;
    let x = number(cx, "multipoles/flipflop/width", 1.2) * u / 2.0;
    let h = 1.5 * s;
    let lw = input.line_width;
    let thick = number(cx, "flipflops/thickness", 2.0) * lw;
    let thin = number(cx, "multipoles/external pins thickness", 1.0) * lw;
    let lead = number(cx, "multipoles/external pins width", 0.2) * u;
    let clock = number(cx, "multipoles/flipflop/clock wedge size", 0.2) * s;
    let radius = number(cx, "nodes width", 0.04) * u;
    let fonts = [tex(cx, "multipoles/flipflop/font")?, tex(cx, "multipoles/flipflop/fontud")?];
    let upright = setting(cx, "multipoles/straight numbers")? != "false";
    // how the node turns, when it does: the sine and cosine
    let t = &input.transform;
    let turned = (t.b.abs() > 1e-9 || t.c.abs() > 1e-9).then(|| {
        let scale = (t.a * t.a + t.b * t.b).sqrt().max(1e-9);
        (t.b / scale, t.a / scale)
    });
    Ok(FlipflopBox { x, h, s, thick, thin, lead, clock, radius, fonts, upright, turned })
}

/// `flipflop def`, the later of a key winning.
fn flipflop_def(cx: &mut Context) -> Vec<(String, String)> {
    let mut def: Vec<(String, String)> = Vec::new();
    for (k, v) in cx.macro_body("voltik@ffd").key_list() {
        let v = v.map(|t| t.trim().to_text()).unwrap_or_default();
        def.retain(|(old, _)| *old != k);
        def.push((k, v));
    }
    def
}

/// The clock wedge at a pin, pointing out of the box.
fn clock_wedge(at: Point, out: Point, clock: f64) -> Vec<PathSegment> {
    let turn = Point::new(-out.y, out.x);
    let q = |a: f64, t: f64| Point::new(at.x + out.x * a + turn.x * t, at.y + out.y * a + turn.y * t);
    vec![PathSegment::MoveTo(q(0.0, clock)), PathSegment::LineTo(q(-clock, 0.0)), PathSegment::LineTo(q(0.0, -clock))]
}

/// A negation circle on a pin, and the pole node drawn after the shape.
struct NegationPole {
    center: Point,
    anchor: String,
    segments: Vec<PathSegment>,
    node: String,
}

fn negation_pole(at: Point, out: Point, radius: f64, k: &str) -> NegationPole {
    let center = Point::new(at.x + out.x * radius, at.y + out.y * radius);
    let anchor = format!("voltik N{k}");
    let node = format!("(\\voltik@tn.{anchor}) node[shape=voltik pole, voltik/pole shape=ocirc, name=\\voltik@tn-N{k}] {{}} ");
    NegationPole { center, anchor, segments: circle(center, radius), node }
}

/// A pin's text: upright when the flip-flop is turned, otherwise on its own side.
struct PinText<'a> {
    key: &'a str,
    at: Point,
    out: Point,
    inset: f64,
    text: String,
    fonts: &'a [String; 2],
    upright: bool,
    turned: Option<(f64, f64)>,
}

fn pin_text(pin: PinText) -> ShapeText {
    let PinText { key, at, out, inset: c, text, fonts, upright, turned } = pin;
    if upright && turned.is_some() {
        // upright in a turned flip-flop: by the side of its box that faces the pin
        let (sin, cos) = turned.unwrap_or_default();
        let face = Point::new(out.x * cos - out.y * sin, out.x * sin + out.y * cos);
        let anchor = if face.x.abs() > face.y.abs() {
            if face.x > 0.0 {
                TextAnchor::East
            } else {
                TextAnchor::West
            }
        } else if face.y > 0.0 {
            TextAnchor::North
        } else {
            TextAnchor::South
        };
        let font = if key.len() == 1 && key.as_bytes()[0].is_ascii_digit() { &fonts[0] } else { &fonts[1] };
        let point = Point::new(at.x - out.x * (SIDE_IN + c), at.y - out.y * (SIDE_IN + c));
        ShapeText { text, at: point, anchor, font: Some(font.clone()), upright, ..Default::default() }
    } else {
        let (point, anchor, font) = match key {
            "1" | "2" | "3" => (Point::new(at.x + SIDE_IN + c, at.y - SIDE_DOWN), TextAnchor::BaseWest, &fonts[0]),
            "4" | "5" | "6" => (Point::new(at.x - SIDE_IN - c, at.y - SIDE_DOWN), TextAnchor::BaseEast, &fonts[0]),
            "u" => (Point::new(at.x, at.y - BELOW_TOP - c), TextAnchor::Base, &fonts[1]),
            _ => (Point::new(at.x, at.y + ABOVE_BOTTOM + c), TextAnchor::Base, &fonts[1]),
        };
        ShapeText { text, at: point, anchor, font: Some(font.clone()), upright, ..Default::default() }
    }
}

/// `flipflop`.
pub fn flipflop_shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let FlipflopBox { x, h, s, thick, thin, lead, clock, radius, fonts, upright, turned } = flipflop_box(cx, input)?;
    let def = flipflop_def(cx);
    let get = |key: &str| def.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone()).unwrap_or_default();
    let on = |key: &str| {
        let v = get(key);
        !v.is_empty() && v.trim() != "0"
    };
    let p = Point::new;

    let mut anchors = super::compass(super::Edges { left: -x, right: x, bottom: -h, top: h });
    anchors.push(("center".into(), p(0.0, 0.0)));
    anchors.push(("text".into(), p(0.0, 0.0)));
    let mut leads = Vec::new();
    let mut wedges = Vec::new();
    let mut circles = Vec::new();
    let mut texts = Vec::new();
    let mut nodes = String::new();
    for (k, name) in PINS {
        // where the pin meets the box, and which way it leaves
        let (at, out) = match *k {
            "1" => (p(-x, s), p(-1.0, 0.0)),
            "2" => (p(-x, 0.0), p(-1.0, 0.0)),
            "3" => (p(-x, -s), p(-1.0, 0.0)),
            "4" => (p(x, -s), p(1.0, 0.0)),
            "5" => (p(x, 0.0), p(1.0, 0.0)),
            "6" => (p(x, s), p(1.0, 0.0)),
            "u" => (p(0.0, h), p(0.0, 1.0)),
            _ => (p(0.0, -h), p(0.0, -1.0)),
        };
        let end = p(at.x + out.x * lead, at.y + out.y * lead);
        anchors.push((name.to_string(), end));
        anchors.push((format!("b{name}"), at));
        let text = get(&format!("t{k}"));
        let (clocked, negated) = (on(&format!("c{k}")), on(&format!("n{k}")));
        if text.is_empty() && !clocked && !negated {
            continue;
        }
        if lead > 0.0 {
            leads.extend([PathSegment::MoveTo(at), PathSegment::LineTo(end)]);
        }
        let c = if clocked { clock } else { 0.0 };
        if clocked {
            wedges.extend(clock_wedge(at, out, clock));
        }
        if negated {
            let pole = negation_pole(at, out, radius, k);
            anchors.push((pole.anchor.clone(), pole.center));
            circles.extend(pole.segments);
            nodes.push_str(&pole.node);
        }
        if !text.is_empty() {
            texts.push(pin_text(PinText { key: k, at, out, inset: c, text, fonts: &fonts, upright, turned }));
        }
    }

    let outline = vec![PathSegment::MoveTo(p(-x, -h)), PathSegment::LineTo(p(x, -h)), PathSegment::LineTo(p(x, h)), PathSegment::LineTo(p(-x, h)), PathSegment::Close];
    let mut paths = vec![
        ShapePath { segments: outline.clone(), paint: ShapePaint::Fill, ..Default::default() },
        ShapePath { segments: outline, paint: ShapePaint::Stroke(thick), ..Default::default() },
    ];
    for segments in [wedges, leads] {
        if !segments.is_empty() {
            paths.push(ShapePath { segments, paint: ShapePaint::Stroke(thin), ..Default::default() });
        }
    }
    if !circles.is_empty() {
        paths.push(ShapePath { segments: circles.clone(), paint: ShapePaint::Background, ..Default::default() });
        paths.push(ShapePath { segments: circles, paint: ShapePaint::Stroke(thin), ..Default::default() });
    }
    let code = if nodes.is_empty() { String::new() } else { format!("\\pgfextra{{\\xdef\\voltik@tn{{\\tikzlastnode}}}} {nodes}") };
    cx.run(Tokens::text(&format!("\\gdef\\voltik@ff{{{code}}}")))?;
    Ok(NodeShape {
        center: p(0.0, 0.0),
        anchors,
        border: ShapeBorder::Rectangle(p(x, h)),
        text: p(-input.width / 2.0, -(input.height - input.depth) / 2.0),
        paths,
        texts,
        extent: Vec::new(),
    })
}

/// A circle around `c`.
fn circle(c: Point, r: f64) -> Vec<PathSegment> {
    let k = 0.5523 * r;
    let p = Point::new;
    vec![
        PathSegment::MoveTo(p(c.x + r, c.y)),
        PathSegment::CurveTo(p(c.x + r, c.y + k), p(c.x + k, c.y + r), p(c.x, c.y + r)),
        PathSegment::CurveTo(p(c.x - k, c.y + r), p(c.x - r, c.y + k), p(c.x - r, c.y)),
        PathSegment::CurveTo(p(c.x - r, c.y - k), p(c.x - k, c.y - r), p(c.x, c.y - r)),
        PathSegment::CurveTo(p(c.x + k, c.y - r), p(c.x + r, c.y - k), p(c.x + r, c.y)),
        PathSegment::Close,
    ]
}

/// `clockwedge`: a flip-flop's clock wedge on its own, its text after it.
pub fn clockwedge_shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let length = setting(cx, "bipoles/length")?;
    let u = cx.length(Tokens::text(&length)).unwrap_or(1.4) * number(cx, "flipflops/scale", 1.0);
    let c = number(cx, "multipoles/flipflop/clock wedge size", 0.2) * number(cx, "multipoles/flipflop/pin spacing", 0.6) * u;
    let thin = number(cx, "multipoles/external pins thickness", 1.0) * input.line_width;
    let p = Point::new;
    // the text two points after the wedge
    let text = p(c + 0.0703, 0.0);
    let mut anchors = vec![("center".to_string(), p(0.0, 0.0)), ("nw".into(), p(0.0, c)), ("sw".into(), p(0.0, -c)), ("right".into(), p(c, 0.0)), ("text".into(), text)];
    for (name, at) in [("north", p(c / 2.0, c)), ("south", p(c / 2.0, -c)), ("east", p(c, 0.0)), ("west", p(0.0, 0.0)), ("left", p(0.0, 0.0))] {
        anchors.push((name.into(), at));
    }
    Ok(NodeShape {
        center: p(0.0, 0.0),
        anchors,
        border: ShapeBorder::Point,
        text: p(text.x, -(input.height - input.depth) / 2.0),
        paths: vec![ShapePath {
            segments: vec![PathSegment::MoveTo(p(0.0, c)), PathSegment::LineTo(p(c, 0.0)), PathSegment::LineTo(p(0.0, -c))],
            paint: ShapePaint::Stroke(thin),
            ..Default::default()
        }],
        ..Default::default()
    })
}
