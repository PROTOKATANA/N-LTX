//! Seven segment displays (manual 4.22): `bare7seg`, its segments `a`…`g` on or off by
//! `seven seg/bits`, a decimal dot (`seven seg/dot`: `none`, `empty`, `on`, `off`) and a box
//! (`seven seg/box`); `seven segment val=A dot off box on` and `seven segment bits=…` set them.
//!
//! Measured on what CircuiTikZ 1.8.7 draws (`tools/oracle/`): the digit is `seven seg/width` wide
//! and twice as high, in `bipoles/length` times `displays/scale`; each segment is a line
//! `seven seg/thickness` wide between pointed ends, `segment sep` short of the corners, in
//! `color on` or `color off`; the thickness and the gaps do not scale.

use tikra::extension::{Context, Error, NodeShape, PathSegment, Point, ShapeBorder, ShapeInput, ShapePaint, ShapePath, Tokens};

use crate::bipole::{number, setting};

/// The segments lit for each value of `seven segment val`, `a` to `g`.
const VALUES: &[(&str, &str)] = &[
    ("0", "1111110"),
    ("1", "0110000"),
    ("2", "1101101"),
    ("3", "1111001"),
    ("4", "0110011"),
    ("5", "1011011"),
    ("6", "1011111"),
    ("7", "1110000"),
    ("8", "1111111"),
    ("9", "1111011"),
    ("10", "1110111"),
    ("11", "0011111"),
    ("12", "1001110"),
    ("13", "0111101"),
    ("14", "1001111"),
    ("15", "1000111"),
    ("a", "1110111"),
    ("b", "0011111"),
    ("c", "1001110"),
    ("d", "0111101"),
    ("e", "1001111"),
    ("f", "1000111"),
    ("-", "0000001"),
];

/// `seven segment val={A dot off box on}` and `seven segment bits={1001001 dot empty box on}`: a
/// `bare7seg` with those segments, dot and box (`val` a digit, a hexadecimal one or `-`).
pub fn k_seven_segment(cx: &mut Context, v: Option<Tokens>, bits: bool) -> Result<(), Error> {
    let spec = cx.expand(v.unwrap_or_default())?;
    // the value runs up to `dot` (`\i dot on` loses its space after `\i`)
    let (first, rest) = match spec.get(1..).and_then(|s| s.find("dot")) {
        Some(at) => (spec[..at + 1].trim(), &spec[at + 1..]),
        None => (spec.trim(), ""),
    };
    let first = if first.is_empty() { "0" } else { first };
    let words: Vec<&str> = std::iter::once(first).chain(rest.split_whitespace()).collect();
    let lit = if bits {
        first.to_string()
    } else {
        match VALUES.iter().find(|(v, _)| *v == first.to_ascii_lowercase()) {
            Some((_, b)) => b.to_string(),
            None => {
                cx.warn(format!("voltik: seven segment val={first} is not a digit; drawn dark"));
                "0000000".into()
            }
        }
    };
    let mut keys = format!("shape=bare7seg, voltik/class fill=displays, /tikz/circuitikz/seven seg/bits={lit}");
    for pair in words[1..].chunks(2) {
        if let [key @ ("dot" | "box"), value] = pair {
            keys.push_str(&format!(", /tikz/circuitikz/seven seg/{key}={value}"));
        }
    }
    cx.keys(Tokens::text(&keys))
}

pub fn k_seven_segment_val(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    k_seven_segment(cx, v, false)
}

pub fn k_seven_segment_bits(cx: &mut Context, v: Option<Tokens>) -> Result<(), Error> {
    k_seven_segment(cx, v, true)
}

/// `bare7seg`.
pub fn seven_segment_shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let length = setting(cx, "bipoles/length")?;
    let w = cx.length(Tokens::text(&length)).unwrap_or(1.4) * number(cx, "displays/scale", 1.0) * number(cx, "seven seg/width", 0.4);
    let len = |cx: &mut Context, key: &str, default: f64| {
        let v = setting(cx, key).unwrap_or_default();
        cx.length(Tokens::text(&v)).unwrap_or(default)
    };
    let t = len(cx, "seven seg/thickness", 0.1406) / 2.0;
    let sep = len(cx, "seven seg/segment sep", 0.007);
    let gap = len(cx, "seven seg/box sep", 0.0351);
    let bits: Vec<bool> = setting(cx, "seven seg/bits")?.chars().map(|c| c == '1').collect();
    let dot = setting(cx, "seven seg/dot")?;
    let boxed = setting(cx, "seven seg/box")? == "on";
    let colors = [setting(cx, "seven seg/color off")?, setting(cx, "seven seg/color on")?];
    let thick = number(cx, "multipoles/thickness", 2.0) * input.line_width;
    let p = Point::new;
    let hw = w / 2.0;

    let (left, top) = (-(hw + t + gap), w + t + gap);
    let right = if dot == "none" { hw + t + gap } else { hw + 3.0 * t + gap };
    let rect =
        vec![PathSegment::MoveTo(p(left, -top)), PathSegment::LineTo(p(right, -top)), PathSegment::LineTo(p(right, top)), PathSegment::LineTo(p(left, top)), PathSegment::Close];
    let mut paths = vec![ShapePath { segments: rect.clone(), paint: ShapePaint::Fill, ..Default::default() }];
    if boxed {
        paths.push(ShapePath { segments: rect, paint: ShapePaint::Stroke(thick), ..Default::default() });
    }
    let segments = [
        ("a", p(-hw, w), p(hw, w)),
        ("b", p(hw, w), p(hw, 0.0)),
        ("c", p(hw, 0.0), p(hw, -w)),
        ("d", p(hw, -w), p(-hw, -w)),
        ("e", p(-hw, -w), p(-hw, 0.0)),
        ("f", p(-hw, 0.0), p(-hw, w)),
        ("g", p(-hw, 0.0), p(hw, 0.0)),
    ];
    let mut anchors = super::compass(left, right, -top, top);
    anchors.push(("center".into(), p(0.0, 0.0)));
    anchors.push(("text".into(), p(0.0, 0.0)));
    for (i, (name, a, b)) in segments.iter().enumerate() {
        let color = colors[usize::from(bits.get(i).copied().unwrap_or(false))].clone();
        let len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        let d = p((b.x - a.x) / len, (b.y - a.y) / len);
        let n = p(-d.y, d.x);
        let at = |from: Point, along: f64, across: f64| p(from.x + d.x * along + n.x * across, from.y + d.y * along + n.y * across);
        paths.push(ShapePath {
            segments: vec![PathSegment::MoveTo(at(*a, sep + 2.0 * t, 0.0)), PathSegment::LineTo(at(*b, -sep - 2.0 * t, 0.0))],
            paint: ShapePaint::Stroke(2.0 * t),
            color: Some(color.clone()),
            ..Default::default()
        });
        // the pointed ends
        for (end, k) in [(*a, 1.0), (*b, -1.0)] {
            let tip = [at(end, k * (sep + 2.5 * t), t), at(end, k * (sep + t), t), at(end, k * sep, 0.0), at(end, k * (sep + t), -t), at(end, k * (sep + 2.5 * t), -t)];
            let mut segs = vec![PathSegment::MoveTo(tip[0])];
            segs.extend(tip[1..].iter().map(|q| PathSegment::LineTo(*q)));
            segs.push(PathSegment::Close);
            paths.push(ShapePath { segments: segs, paint: ShapePaint::Solid, color: Some(color.clone()), ..Default::default() });
        }
        anchors.push((name.to_string(), p((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)));
    }
    let center = if dot == "none" { p(hw, -w) } else { p(hw + 2.0 * t, -w) };
    anchors.push(("dot".into(), center));
    if dot == "on" || dot == "off" {
        let color = colors[usize::from(dot == "on")].clone();
        let k = 0.5523 * t;
        let c = center;
        let circle = vec![
            PathSegment::MoveTo(p(c.x + t, c.y)),
            PathSegment::CurveTo(p(c.x + t, c.y + k), p(c.x + k, c.y + t), p(c.x, c.y + t)),
            PathSegment::CurveTo(p(c.x - k, c.y + t), p(c.x - t, c.y + k), p(c.x - t, c.y)),
            PathSegment::CurveTo(p(c.x - t, c.y - k), p(c.x - k, c.y - t), p(c.x, c.y - t)),
            PathSegment::CurveTo(p(c.x + k, c.y - t), p(c.x + t, c.y - k), p(c.x + t, c.y)),
            PathSegment::Close,
        ];
        paths.push(ShapePath { segments: circle.clone(), paint: ShapePaint::Solid, color: Some(color.clone()), ..Default::default() });
        paths.push(ShapePath { segments: circle, paint: ShapePaint::Stroke(input.line_width), color: Some(color), ..Default::default() });
    }
    Ok(NodeShape {
        center: p(0.0, 0.0),
        anchors,
        border: ShapeBorder::Rectangle(p(right.max(-left), top)),
        text: p(-input.width / 2.0, -(input.height - input.depth) / 2.0),
        paths,
        ..Default::default()
    })
}
