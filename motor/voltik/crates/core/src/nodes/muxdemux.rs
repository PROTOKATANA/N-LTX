//! Multiplexers and de-multiplexers (manual 4.20): `muxdemux`, a trapezoid `Lh` high on the left,
//! `Rh` on the right and `w` wide, with an optional inset on its left side, and pins on its four
//! sides, all set by `muxdemux def` (`multipoles/muxdemux/…`).
//!
//! Measured on what CircuiTikZ 1.8.7 draws (`tools/oracle/`): every length is in
//! `multipoles/muxdemux/base len` (0.4 of `bipoles/length`) times `muxdemuxes/scale`; the pins of a
//! side are evenly spread along it, the left ones on the parts above and below the inset when there
//! is one (an odd one at its tip); the bottom and top pins stick out straight down and up, from the
//! slanted side or, with `square pins=1`, to the same line. `muxdemux label` adds texts by the pins
//! (inside, `L1`, or outside, `LU1`), by the borders (`N`, `Ni`) and the clock wedges (`cL1`),
//! negation circles (`nL1`, nodes of their own, `M-nL1`) and wedges (`wiL1`, `woL1`); `bgpicture`
//! in `muxdemux def` draws pgf's code on it.

use tikra::extension::{Context, Error, NodeShape, PathSegment, Point, ShapeBorder, ShapeInput, ShapePaint, ShapePath, ShapeText, TextAnchor, Tokens};

use super::chips::drawn_pins;
use crate::bipole::{number, setting};

/// `muxdemux`.
pub fn muxdemux_shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let length = setting(cx, "bipoles/length")?;
    let unit = cx.length(Tokens::text(&length)).unwrap_or(1.4) * number(cx, "muxdemuxes/scale", 1.0);
    let b = number(cx, "multipoles/muxdemux/base len", 0.4) * unit;
    let def = |cx: &mut Context, key: &str, default: f64| number(cx, &format!("multipoles/muxdemux/{key}"), default);
    let count = |cx: &mut Context, key: &str, default: f64| def(cx, key, default).round().max(0.0) as usize;
    let x = def(cx, "w", 3.0) * b / 2.0;
    let left = def(cx, "Lh", 8.0) * b / 2.0;
    let right = def(cx, "Rh", 6.0) * b / 2.0;
    let iw = def(cx, "inset w", 0.0) * b;
    let il = def(cx, "inset Lh", 0.0) * b / 2.0;
    let ir = def(cx, "inset Rh", 0.0) * b / 2.0;
    let (nl, nr, nb, nt) = (count(cx, "NL", 8.0), count(cx, "NR", 1.0), count(cx, "NB", 3.0), count(cx, "NT", 0.0));
    let square = def(cx, "square pins", 0.0) != 0.0;
    let lw = input.line_width;
    let thick = number(cx, "muxdemuxes/thickness", 2.0) * lw;
    let thin = number(cx, "multipoles/external pins thickness", 1.0) * lw;
    let lead = number(cx, "multipoles/external pins width", 0.2) * unit;
    let leads = setting(cx, "tr/no input leads")? != "true";
    let p = Point::new;

    // the top and bottom sides, as lines from the left to the right
    let top_at = |px: f64| left + (right - left) * (px + x) / (2.0 * x);
    let reach = left.max(right);

    let mut anchors = super::compass(-x, x, -reach, reach);
    for (name, at) in [
        ("center", p(0.0, 0.0)),
        ("text", p(iw / 2.0, 0.0)),
        ("top left", p(-x, left)),
        ("top", p(0.0, (left + right) / 2.0)),
        ("top right", p(x, right)),
        ("bottom left", p(-x, -left)),
        ("bottom", p(0.0, -(left + right) / 2.0)),
        ("bottom right", p(x, -right)),
        ("left", p(-x, 0.0)),
        ("right", p(x, 0.0)),
        ("center up", p(0.0, ((left + right) / 2.0 + ir) / 2.0)),
        ("center down", p(0.0, -((left + right) / 2.0 + ir) / 2.0)),
        ("inset top left", p(-x, il)),
        ("inset top", p(-x + iw / 2.0, (il + ir) / 2.0)),
        ("inset top right", p(-x + iw, ir)),
        ("inset bottom left", p(-x, -il)),
        ("inset bottom", p(-x + iw / 2.0, -(il + ir) / 2.0)),
        ("inset bottom right", p(-x + iw, -ir)),
        ("inset left", p(-x, 0.0)),
        ("inset right", p(-x + iw, 0.0)),
        ("inset center", p(-x + iw / 2.0, 0.0)),
        ("narrow center", p(iw / 2.0, 0.0)),
    ] {
        anchors.push((name.to_string(), at));
    }

    // the pins of each side: where on the border, and which way out
    let mut sides: Vec<(&str, Vec<Point>, Point)> = Vec::new();
    let mut lefts = Vec::with_capacity(nl);
    if il > 0.0 {
        let k = nl / 2;
        let step = if k > 0 { (left - il) / k as f64 } else { 0.0 };
        for j in 0..k {
            lefts.push(p(-x, left - (j as f64 + 0.5) * step));
        }
        if nl % 2 == 1 {
            lefts.push(p(-x + iw, 0.0));
        }
        for j in 0..k {
            lefts.push(p(-x, -il - (j as f64 + 0.5) * step));
        }
    } else {
        let step = 2.0 * left / nl.max(1) as f64;
        lefts.extend((0..nl).map(|j| p(-x, left - (j as f64 + 0.5) * step)));
    }
    sides.push(("l", lefts, p(-1.0, 0.0)));
    let step = 2.0 * right / nr.max(1) as f64;
    sides.push(("r", (0..nr).map(|j| p(x, right - (j as f64 + 0.5) * step)).collect(), p(1.0, 0.0)));
    let across = |n: usize| (0..n).map(move |j| -x + (j as f64 + 0.5) * 2.0 * x / n as f64);
    sides.push(("b", across(nb).map(|px| p(px, -top_at(px))).collect(), p(0.0, -1.0)));
    sides.push(("t", across(nt).map(|px| p(px, top_at(px))).collect(), p(0.0, 1.0)));

    let mut pins = Vec::new();
    let mut borders: Vec<(char, Vec<Point>, Point)> = Vec::new();
    for (side, points, out) in sides {
        borders.push((side.chars().next().unwrap_or('l'), points.clone(), out));
        let name = match side {
            "l" => "left",
            "r" => "right",
            "b" => "bottom",
            _ => "top",
        };
        let only = drawn_pins(&setting(cx, &format!("multipoles/draw only {name} pins"))?);
        for (i, at) in points.into_iter().enumerate() {
            let k = i + 1;
            // straight out by the lead, or down (up) to the same line with square pins
            let end = if square && out.x == 0.0 { p(at.x, out.y * (reach + lead)) } else { p(at.x + out.x * lead, at.y + out.y * lead) };
            let drawn = leads && lead > 0.0 && only.as_ref().is_none_or(|set| set.contains(&k));
            let pin = if drawn || !leads { end } else { at };
            anchors.push((format!("{side}pin {k}"), pin));
            anchors.push((format!("b{side}pin {k}"), at));
            if drawn {
                pins.extend([PathSegment::MoveTo(at), PathSegment::LineTo(end)]);
            }
        }
    }

    let mut outline = vec![PathSegment::MoveTo(p(-x, left)), PathSegment::LineTo(p(x, right)), PathSegment::LineTo(p(x, -right)), PathSegment::LineTo(p(-x, -left))];
    if il > 0.0 || iw > 0.0 {
        outline.extend([PathSegment::LineTo(p(-x, -il)), PathSegment::LineTo(p(-x + iw, -ir)), PathSegment::LineTo(p(-x + iw, ir)), PathSegment::LineTo(p(-x, il))]);
    }
    outline.push(PathSegment::Close);
    let mut paths = vec![
        ShapePath { segments: outline.clone(), paint: ShapePaint::Fill, ..Default::default() },
        ShapePath { segments: outline, paint: ShapePaint::Stroke(thick), ..Default::default() },
    ];
    if !pins.is_empty() {
        paths.push(ShapePath { segments: pins, paint: ShapePaint::Stroke(thin), ..Default::default() });
    }
    let t = input.transform;
    let angle = if t.a * t.d - t.b * t.c > 0.0 { t.b.atan2(t.a) } else { 0.0 };
    let mut texts = labels(cx, &borders, x, reach, b, unit, thin, angle, &mut paths, &mut anchors)?;
    for text in texts.iter_mut() {
        // a color among the font commands (`\tiny\color{blue}`) is the text's
        if let Some(font) = &text.font {
            let (color, font) = super::font_color(font);
            text.text = format!("{color}{}", text.text);
            text.font = Some(font);
        }
    }
    Ok(NodeShape {
        center: p(0.0, 0.0),
        anchors,
        border: ShapeBorder::Rectangle(p(x, reach)),
        text: p(iw / 2.0 - input.width / 2.0, -(input.height - input.depth) / 2.0),
        paths,
        texts,
        ..Default::default()
    })
}

/// A compass anchor of a text turned by `angle` (radians), to the nearest of the eight.
fn turned(anchor: TextAnchor, angle: f64) -> TextAnchor {
    use TextAnchor::*;
    const RING: [TextAnchor; 8] = [East, NorthEast, North, NorthWest, West, SouthWest, South, SouthEast];
    let Some(i) = RING.iter().position(|a| *a == anchor) else { return anchor };
    let steps = (angle / std::f64::consts::FRAC_PI_4).round() as i64;
    RING[(i as i64 + steps).rem_euclid(8) as usize]
}

/// A setting that is TeX (`\tiny\color{blue}`), unexpanded.
fn tex(cx: &mut Context, key: &str) -> Result<String, Error> {
    cx.expand(Tokens::text(&format!("\\unexpanded\\expandafter{{\\romannumeral-`0\\pgfkeysvalueof{{/tikz/circuitikz/{key}}}}}")))
}

/// A length setting (`2pt`), in cm.
fn sep(cx: &mut Context, key: &str) -> f64 {
    let v = setting(cx, key).unwrap_or_default();
    cx.length(Tokens::text(&v)).unwrap_or(0.0703)
}

/// The texts, clock wedges, negation circles and wedges of `muxdemux label` (`\voltik@mdl`, the
/// later of a key winning), drawn by the pins of `borders` and by the box `width` by `reach` (halves); the
/// circles' nodes are made after the shape, by `\voltik@md`.
#[allow(clippy::too_many_arguments)]
fn labels(
    cx: &mut Context,
    borders: &[(char, Vec<Point>, Point)],
    width: f64,
    reach: f64,
    b: f64,
    unit: f64,
    thin: f64,
    angle: f64,
    paths: &mut Vec<ShapePath>,
    anchors: &mut Vec<(String, Point)>,
) -> Result<Vec<ShapeText>, Error> {
    let mut keys: Vec<(String, String)> = Vec::new();
    for (k, v) in cx.macro_body("voltik@mdl").key_list() {
        let v = v.map(|t| t.trim().to_text()).unwrap_or_default();
        keys.retain(|(old, _)| *old != k);
        keys.push((k, v));
    }
    let mut circles = String::new();
    let mut texts = Vec::new();
    let p = Point::new;
    let pin = |side: char, i: usize| -> Option<(Point, Point)> {
        let (_, points, out) = borders.iter().find(|(s, _, _)| *s == side)?;
        Some((*points.get(i.checked_sub(1)?)?, *out))
    };
    // where a key points: its kind (`L`, `LU`, `c`…), side and pin
    let parse = |key: &str| -> Option<(String, char, usize)> {
        let digits = key.find(|c: char| c.is_ascii_digit())?;
        let (head, n) = key.split_at(digits);
        let n: usize = n.parse().ok()?;
        let side = head.chars().find(|c| "LRBT".contains(*c))?;
        let at = head.find(side)?;
        Some((format!("{}{}", &head[..at], &head[at + 1..]), side.to_ascii_lowercase(), n))
    };
    let upright = setting(cx, "multipoles/straight numbers")? != "false";
    let clock = number(cx, "muxdemux/clock wedge size", 0.2) * b;
    let wedge = (number(cx, "wedge inversion mark/width", 0.2) * unit, number(cx, "wedge inversion mark/height", 0.1) * unit);
    let radius = number(cx, "nodes width", 0.04) * unit;
    let fonts = [tex(cx, "muxdemux/inner label font")?, tex(cx, "muxdemux/outer label font")?, tex(cx, "muxdemux/border label font")?];
    let seps = [
        (sep(cx, "muxdemux/inner label xsep"), sep(cx, "muxdemux/inner label ysep")),
        (sep(cx, "muxdemux/outer label xsep"), sep(cx, "muxdemux/outer label ysep")),
        (sep(cx, "muxdemux/border label xsep"), sep(cx, "muxdemux/border label ysep")),
    ];
    let clocked = |side: char, n: usize| keys.iter().any(|(k, v)| parse(k) == Some(("c".into(), side, n)) && v.trim() != "0");
    let mut wedges = Vec::new();
    for (key, value) in &keys {
        // the border labels
        let border = match key.as_str() {
            "N" => Some((p(0.0, reach + seps[2].1), TextAnchor::South)),
            "Ni" => Some((p(0.0, reach - seps[2].1), TextAnchor::North)),
            "S" => Some((p(0.0, -reach - seps[2].1), TextAnchor::North)),
            "Si" => Some((p(0.0, -reach + seps[2].1), TextAnchor::South)),
            _ => None,
        };
        // along the sides, turned a quarter
        let side = match key.as_str() {
            "E" => Some((p(width + seps[2].0, 0.0), TextAnchor::North)),
            "Ei" => Some((p(width - seps[2].0, 0.0), TextAnchor::South)),
            "W" => Some((p(-width - seps[2].0, 0.0), TextAnchor::South)),
            "Wi" => Some((p(-width + seps[2].0, 0.0), TextAnchor::North)),
            _ => None,
        };
        if let Some((at, anchor)) = border {
            texts.push(ShapeText { text: value.clone(), at, anchor, font: Some(fonts[2].clone()), ..Default::default() });
            continue;
        }
        if let Some((at, anchor)) = side {
            texts.push(ShapeText { text: value.clone(), at, anchor, font: Some(fonts[2].clone()), turn: 90.0, ..Default::default() });
            continue;
        }
        let Some((kind, side, n)) = parse(key) else { continue };
        let Some((at, out)) = pin(side, n) else { continue };
        let on = value.trim() != "0";
        // across the pin, a quarter turn from its way out
        let turn = p(-out.y, out.x);
        match kind.as_str() {
            "" => {
                let (sx, sy) = seps[0];
                let c = if clocked(side, n) { clock } else { 0.0 };
                let (at, anchor) = match side {
                    'l' => (p(at.x + sx + c, at.y), TextAnchor::West),
                    'r' => (p(at.x - sx - c, at.y), TextAnchor::East),
                    'b' => (p(at.x, at.y + sy + c), TextAnchor::South),
                    _ => (p(at.x, at.y - sy - c), TextAnchor::North),
                };
                // upright in a turned box, the side of the text at the point turns with it
                let anchor = if upright { turned(anchor, angle) } else { anchor };
                texts.push(ShapeText { text: value.clone(), at, anchor, font: Some(fonts[0].clone()), upright, ..Default::default() });
            }
            "U" | "D" | "L" | "R" if upright && angle.abs() > 1e-9 => {
                // upright in a turned box, as on the page: the first (`U`, `L`) above a pin that
                // goes out sideways and left of one that goes up or down, the other below or right
                let (sx, sy) = seps[1];
                let (sin, cos) = angle.sin_cos();
                let o = p(out.x * cos - out.y * sin, out.x * sin + out.y * cos);
                let first = if matches!(kind.as_str(), "U" | "L") { 1.0 } else { -1.0 };
                let (dx, dy) = if o.x.abs() > o.y.abs() { (o.x.signum(), first) } else { (-first, o.y.signum()) };
                let anchor = match (dx > 0.0, dy > 0.0) {
                    (true, true) => TextAnchor::SouthWest,
                    (true, false) => TextAnchor::NorthWest,
                    (false, true) => TextAnchor::SouthEast,
                    (false, false) => TextAnchor::NorthEast,
                };
                // the point, back in the box's own turn
                let (px, py) = (dx * sx, dy * sy);
                let at = p(at.x + px * cos + py * sin, at.y - px * sin + py * cos);
                texts.push(ShapeText { text: value.clone(), at, anchor, font: Some(fonts[1].clone()), upright, ..Default::default() });
            }
            "U" | "D" | "L" | "R" => {
                let (sx, sy) = seps[1];
                let (at, anchor) = match (side, kind.as_str()) {
                    ('l', "U") => (p(at.x - sx, at.y + sy), TextAnchor::SouthEast),
                    ('l', _) => (p(at.x - sx, at.y - sy), TextAnchor::NorthEast),
                    ('r', "U") => (p(at.x + sx, at.y + sy), TextAnchor::SouthWest),
                    ('r', _) => (p(at.x + sx, at.y - sy), TextAnchor::NorthWest),
                    ('b', "L") => (p(at.x - sx, at.y - sy), TextAnchor::NorthEast),
                    ('b', _) => (p(at.x + sx, at.y - sy), TextAnchor::NorthWest),
                    ('t', "L") => (p(at.x - sx, at.y + sy), TextAnchor::SouthEast),
                    _ => (p(at.x + sx, at.y + sy), TextAnchor::SouthWest),
                };
                texts.push(ShapeText { text: value.clone(), at, anchor, font: Some(fonts[1].clone()), upright, ..Default::default() });
            }
            "c" if on => {
                let q = |a: f64, t: f64| p(at.x + out.x * a + turn.x * t, at.y + out.y * a + turn.y * t);
                wedges.extend([PathSegment::MoveTo(q(0.0, clock)), PathSegment::LineTo(q(-clock, 0.0)), PathSegment::LineTo(q(0.0, -clock))]);
            }
            "n" if on => {
                let c = p(at.x + out.x * radius, at.y + out.y * radius);
                let name = format!("voltik n{}{n}", side.to_ascii_uppercase());
                anchors.push((name.clone(), c));
                let k = 0.5523 * radius;
                let circle = vec![
                    PathSegment::MoveTo(p(c.x + radius, c.y)),
                    PathSegment::CurveTo(p(c.x + radius, c.y + k), p(c.x + k, c.y + radius), p(c.x, c.y + radius)),
                    PathSegment::CurveTo(p(c.x - k, c.y + radius), p(c.x - radius, c.y + k), p(c.x - radius, c.y)),
                    PathSegment::CurveTo(p(c.x - radius, c.y - k), p(c.x - k, c.y - radius), p(c.x, c.y - radius)),
                    PathSegment::CurveTo(p(c.x + k, c.y - radius), p(c.x + radius, c.y - k), p(c.x + radius, c.y)),
                    PathSegment::Close,
                ];
                paths.push(ShapePath { segments: circle.clone(), paint: ShapePaint::Background, ..Default::default() });
                paths.push(ShapePath { segments: circle, paint: ShapePaint::Stroke(thin), ..Default::default() });
                circles.push_str(&format!("(\\voltik@tn.{name}) node[shape=voltik pole, voltik/pole shape=ocirc, name=\\voltik@tn-{key}] {{}} "));
            }
            "wi" | "wo" if on => {
                let sign: f64 = if value.trim().starts_with('-') { -1.0 } else { 1.0 };
                // up from the pin: the y axis on the left and right, turned with the pin below and above
                let up = if out.x != 0.0 { p(0.0, 1.0) } else { p(-out.y, 0.0) };
                let (w, h) = (wedge.0, wedge.1 * sign);
                let q = |a: f64, t: f64| p(at.x + out.x * a + up.x * t, at.y + out.y * a + up.y * t);
                let tri = if kind == "wo" { [q(0.0, 0.0), q(0.0, h), q(w, 0.0)] } else { [q(w, 0.0), q(w, h), q(0.0, 0.0)] };
                paths.push(ShapePath {
                    segments: vec![PathSegment::MoveTo(tri[0]), PathSegment::LineTo(tri[1]), PathSegment::LineTo(tri[2]), PathSegment::Close],
                    paint: ShapePaint::Stroke(thin),
                    ..Default::default()
                });
            }
            _ => {}
        }
    }
    if !wedges.is_empty() {
        paths.push(ShapePath { segments: wedges, paint: ShapePaint::Stroke(thin), ..Default::default() });
    }
    // `bgpicture`: pgf's code, its x axis from -1 cm to 1 cm across the shape
    let picture = cx.macro_body("voltik@mdbg").to_text();
    if !picture.trim().is_empty() {
        circles.push_str(&format!(
            "\\pgfextra{{\\begin{{pgfscope}}\\pgftransformshift{{\\pgfpointanchor{{\\voltik@tn}}{{center}}}}\\pgftransformscale{{{width}}}{picture}\\end{{pgfscope}}}} "
        ));
    }
    let code = if circles.is_empty() { String::new() } else { format!("\\pgfextra{{\\xdef\\voltik@tn{{\\tikzlastnode}}}} {circles}") };
    cx.run(Tokens::text(&format!("\\gdef\\voltik@md{{{code}}}")))?;
    Ok(texts)
}

/// `wedgeinv`, an open wedge from its `base` to its `apex`, and `circleinv`, a circle from its
/// `base` to its `apex`: inversion marks the size of the `muxdemuxes` class, placed by their left.
pub fn inversion_shape(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let length = setting(cx, "bipoles/length")?;
    let u = cx.length(Tokens::text(&length)).unwrap_or(1.4) * number(cx, "muxdemuxes/scale", 1.0);
    let lw = input.line_width;
    let p = Point::new;
    let (w, h, segments, paths) = if input.name == "wedgeinv" {
        let (w, h) = (number(cx, "wedge inversion mark/width", 0.2) * u, number(cx, "wedge inversion mark/height", 0.1) * u);
        let tri = vec![PathSegment::MoveTo(p(0.0, 0.0)), PathSegment::LineTo(p(0.0, h)), PathSegment::LineTo(p(w, 0.0)), PathSegment::Close];
        let thin = number(cx, "multipoles/external pins thickness", 1.0) * lw;
        (w, h, tri.clone(), vec![ShapePath { segments: tri, paint: ShapePaint::Stroke(thin), ..Default::default() }])
    } else {
        // as wide as a logic port's negation circle
        let r = CIRCLEINV * u;
        let k = 0.5523 * r;
        let circle = vec![
            PathSegment::MoveTo(p(2.0 * r, 0.0)),
            PathSegment::CurveTo(p(2.0 * r, k), p(r + k, r), p(r, r)),
            PathSegment::CurveTo(p(r - k, r), p(0.0, k), p(0.0, 0.0)),
            PathSegment::CurveTo(p(0.0, -k), p(r - k, -r), p(r, -r)),
            PathSegment::CurveTo(p(r + k, -r), p(2.0 * r, -k), p(2.0 * r, 0.0)),
            PathSegment::Close,
        ];
        let thick = number(cx, "muxdemuxes/thickness", 2.0) * lw;
        let paths = vec![
            ShapePath { segments: circle.clone(), paint: ShapePaint::Background, ..Default::default() },
            ShapePath { segments: circle.clone(), paint: ShapePaint::Stroke(thick), ..Default::default() },
        ];
        (2.0 * r, r, circle, paths)
    };
    let wedge = input.name == "wedgeinv";
    let (mid, bottom, text) = if wedge { (p(w / 2.0, h / 2.0), p(0.0, 0.0), p(0.0, 0.0)) } else { (p(w / 2.0, h), p(w / 2.0, -h), p(w / 2.0, 0.0)) };
    let south = if wedge { p(w / 2.0, 0.0) } else { p(w / 2.0, -h) };
    let mut anchors = vec![
        ("center".to_string(), p(0.0, 0.0)),
        ("base".into(), p(0.0, 0.0)),
        ("top".into(), if wedge { p(0.0, h) } else { p(w / 2.0, h) }),
        ("apex".into(), p(w, 0.0)),
        ("mid".into(), mid),
        ("text".into(), text),
        ("north".into(), p(w / 2.0, h)),
        ("south".into(), south),
        ("east".into(), p(w, 0.0)),
        ("west".into(), p(0.0, 0.0)),
        ("left".into(), p(0.0, 0.0)),
        ("right".into(), p(w, 0.0)),
    ];
    if !wedge {
        anchors.push(("bottom".into(), bottom));
    }
    Ok(NodeShape { center: p(0.0, 0.0), anchors, border: ShapeBorder::Outline(super::outline_points(&segments)), text, paths, ..Default::default() })
}

/// A `circleinv`'s radius, in `bipoles/length`.
const CIRCLEINV: f64 = 0.0616;
