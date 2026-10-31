//! Libraries from outside tikra: their keys and definitions, used through the public API only.

use tikra::extension::{Context, Error, Library, Tokens};
use tikra::Options;

/// `twice={options}`: the options, then `line width` doubled.
fn twice(cx: &mut Context, value: Option<Tokens>) -> Result<(), Error> {
    cx.keys(value.unwrap_or_default())?;
    let w = cx.line_width();
    cx.keys(Tokens::text(&format!("line width={}cm", 2.0 * w)))
}

/// `midpoint={A}{B}{name}`: a coordinate halfway between two others.
fn midpoint(cx: &mut Context, value: Option<Tokens>) -> Result<(), Error> {
    let a = value.unwrap_or_default().arguments(3);
    let (p, q, name) = (cx.expand(a[0].clone())?, cx.expand(a[1].clone())?, cx.expand(a[2].clone())?);
    let at = cx.point(&p)?.plus(cx.point(&q)?).scale(0.5);
    cx.run(Tokens::text(&format!("\\coordinate ({name}) at ({}cm,{}cm);", at.x, at.y)))
}

/// `complain`: a warning, and nothing drawn.
fn complain(cx: &mut Context, _value: Option<Tokens>) -> Result<(), Error> {
    cx.warn("complain: nothing to do");
    Ok(())
}

fn definitions() -> String {
    r"\tikzset{road/.style={twice={draw=gray}}}\def\tikramid{midpoint}".into()
}

static ROADS: Library = Library { name: "roads", definitions, keys: &[("/tikz/twice", twice), ("/tikz/midpoint", midpoint), ("/tikz/complain", complain)] };

fn convert(body: &str, libraries: &[&'static Library]) -> tikra::Output {
    tikra::convert_with_libraries(&format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}"), &Options::default(), libraries).unwrap()
}

#[test]
fn a_library_key_runs_and_its_styles_are_defined() {
    let with = convert(r"\draw[road] (0,0) -- (2,0);", &[&ROADS]);
    assert!(with.warnings.is_empty(), "{:?}", with.warnings);
    assert!(with.svg.contains("stroke-width=\"0.797\""), "{}", with.svg);
}

#[test]
fn without_the_library_its_keys_are_unknown() {
    let without = convert(r"\draw[road] (0,0) -- (2,0);", &[]);
    assert!(!without.warnings.is_empty());
    // and the library used before does not stay in the engine
    convert(r"\draw[road] (0,0) -- (2,0);", &[&ROADS]);
    assert!(!convert(r"\draw[twice] (0,0) -- (1,0);", &[]).warnings.is_empty());
}

#[test]
fn a_library_key_reads_points_and_runs_code() {
    let r = convert(r"\path[midpoint={0,0}{4,2}{M}]; \fill (M) circle (2pt);", &[&ROADS]);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
}

#[test]
fn a_library_key_warns() {
    let r = convert(r"\draw[complain] (0,0) -- (1,0);", &[&ROADS]);
    assert!(r.warnings.iter().any(|w| w.contains("complain: nothing to do")), "{:?}", r.warnings);
}

#[test]
fn tokens_split_as_tex_does() {
    let a = Tokens::text("{a}{b c}").arguments(3);
    assert_eq!(a.iter().map(Tokens::to_text).collect::<Vec<_>>(), ["a", "b c", ""]);
    let (opts, rest) = Tokens::text(" [red, thick] $x$").leading_options();
    assert_eq!((opts.to_text().as_str(), rest.to_text().trim()), ("red, thick", "$x$"));
    let keys: Vec<String> = Tokens::text("pos=.3, near end").key_list().into_iter().map(|(k, _)| k).collect();
    assert_eq!(keys, ["pos", "near end"]);
}

use tikra::extension::{LineCap, NodeShape, PathSegment, Point, ShapeBorder, ShapeInput, ShapePaint, ShapePath, ShapeText, TextAnchor};

/// `flag`: a pole from the node's point up 1 cm with a triangle at its top, filled with the node's
/// fill and outlined twice as wide as the lines; a solid dot at the foot. Its anchors are `foot`
/// and `top`, its border an outline, the text right of the pole.
fn flag(cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    let w = 2.0 * input.line_width;
    let p = Point::new;
    let triangle = vec![PathSegment::MoveTo(p(0.0, 1.0)), PathSegment::LineTo(p(0.6, 0.85)), PathSegment::LineTo(p(0.0, 0.7)), PathSegment::Close];
    // a key of the library, read while the shape is made
    let size = cx.number(Tokens::text("\\flagdot"))?;
    let k = 0.55 * size;
    let dot = vec![
        PathSegment::MoveTo(p(size, 0.0)),
        PathSegment::CurveTo(p(size, k), p(k, size), p(0.0, size)),
        PathSegment::CurveTo(p(-k, size), p(-size, k), p(-size, 0.0)),
        PathSegment::CurveTo(p(-size, -k), p(-k, -size), p(0.0, -size)),
        PathSegment::CurveTo(p(k, -size), p(size, -k), p(size, 0.0)),
        PathSegment::Close,
    ];
    Ok(NodeShape {
        center: p(0.0, 0.5),
        anchors: vec![("foot".into(), p(0.0, 0.0)), ("top".into(), p(0.0, 1.0))],
        border: ShapeBorder::Outline(vec![p(-0.1, 0.0), p(0.6, 0.0), p(0.6, 1.0), p(-0.1, 1.0)]),
        text: p(0.7, 0.5 - input.height / 2.0),
        paths: vec![
            ShapePath { segments: triangle.clone(), paint: ShapePaint::Fill, ..Default::default() },
            ShapePath { segments: vec![PathSegment::MoveTo(p(0.0, 0.0)), PathSegment::LineTo(p(0.0, 1.0))], paint: ShapePaint::Stroke(input.line_width), ..Default::default() },
            ShapePath { segments: triangle, paint: ShapePaint::Stroke(w), ..Default::default() },
            ShapePath { segments: dot, paint: ShapePaint::Solid, ..Default::default() },
        ],
        extent: Vec::new(),
        texts: Vec::new(),
    })
}

/// `post` and `mark`: the other borders, a rectangle and a point.
fn post(_cx: &mut Context, _input: &ShapeInput) -> Result<NodeShape, Error> {
    let p = Point::new;
    let square = vec![PathSegment::MoveTo(p(-0.25, -0.25)), PathSegment::LineTo(p(0.25, -0.25)), PathSegment::LineTo(p(0.25, 0.25)), PathSegment::Close];
    Ok(NodeShape {
        border: ShapeBorder::Rectangle(p(0.5, 0.25)),
        paths: vec![
            ShapePath { segments: square, paint: ShapePaint::Background, ..Default::default() },
            ShapePath { segments: vec![PathSegment::MoveTo(p(-0.5, -0.25)), PathSegment::LineTo(p(0.5, 0.25))], paint: ShapePaint::Stroke(0.05), ..Default::default() },
        ],
        ..NodeShape::default()
    })
}

fn mark(_cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    // one function may draw several shapes, told apart by name
    assert!(input.name == "mark" || input.name == "other mark", "{}", input.name);
    // nothing drawn, but room made around it
    Ok(NodeShape { extent: vec![Point::new(-1.0, -1.0), Point::new(1.0, 1.0)], ..NodeShape::default() })
}

/// `declare flags`: the shapes, declared from the library's definitions.
fn declare(cx: &mut Context, _value: Option<Tokens>) -> Result<(), Error> {
    cx.declare_shape("flag", flag);
    cx.declare_shape(" post ", post);
    cx.declare_shape("mark", mark);
    cx.declare_shape("other mark", mark);
    Ok(())
}

fn flag_definitions() -> String {
    r"\def\flagdot{0.05}\pgfkeys{/tikz/declare flags}\tikzset{flag/.style={shape=flag}}".into()
}

static FLAGS: Library = Library { name: "flags", definitions: flag_definitions, keys: &[("/tikz/declare flags", declare)] };

/// The numbers of the `d` of the paths of an SVG, one list per path.
fn path_numbers(svg: &str) -> Vec<Vec<f64>> {
    svg.split("<path d=\"").skip(1).map(|p| p[..p.find('"').unwrap()].split([' ', 'M', 'L', 'C', 'Z']).filter_map(|t| t.parse().ok()).collect()).collect()
}

#[test]
fn a_library_shape_draws_its_paths_as_it_paints_them() {
    let plain = convert(r"\node[flag] {};", &[&FLAGS]);
    assert!(plain.warnings.is_empty(), "{:?}", plain.warnings);
    // the pole, the outline twice as wide, the dot in the line color; no fill without `fill`
    assert!(plain.svg.contains("stroke-width=\"0.399\"") && plain.svg.contains("stroke-width=\"0.797\""), "{}", plain.svg);
    assert!(plain.svg.contains("fill=\"#000000\""), "{}", plain.svg);
    assert_eq!(plain.svg.matches("<path").count(), 3, "{}", plain.svg);
    let filled = convert(r"\node[flag, fill=red, draw=blue] {};", &[&FLAGS]);
    assert!(filled.svg.contains("fill=\"#ff0000\"") && filled.svg.contains("stroke=\"#0000ff\""), "{}", filled.svg);
    assert_eq!(filled.svg.matches("<path").count(), 4, "{}", filled.svg);
}

#[test]
fn a_library_shape_has_its_anchors_and_is_placed_by_them() {
    // the top of a flag placed by its foot at (1,1) is at (1,2): a line from there to (3,2) is flat
    let r = convert(r"\node[flag, anchor=foot] (F) at (1,1) {A}; \draw[red] (F.top) -- (3,2) (F.foot) -- (F.center) (F.30) -- (F.text);", &[&FLAGS]);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    let red = r.svg.split("<path d=\"").find(|p| p.contains("#ff0000")).unwrap();
    let n: Vec<f64> = red[..red.find('"').unwrap()].split([' ', 'M', 'L']).filter_map(|t| t.parse().ok()).collect();
    assert!((n[1] - n[3]).abs() < 1e-6 && (n[2] - n[0] - 56.693).abs() < 0.01, "{n:?}");
    // the text is drawn
    assert!(r.svg.contains("<g transform="), "{}", r.svg);
}

#[test]
fn a_library_shape_turns_with_its_node_and_stops_lines_at_its_border() {
    let straight = path_numbers(&convert(r"\node[flag] {};", &[&FLAGS]).svg);
    let turned = path_numbers(&convert(r"\node[flag, rotate=90] {};", &[&FLAGS]).svg);
    assert_ne!(straight, turned);
    // a line to the flag stops at its outline, one to a post at its rectangle, one to a mark at it
    for (shape, x) in [("flag", 3.0 - 0.1), ("post", 3.0 - 0.5), ("mark", 3.0), ("other mark", 3.0)] {
        let r = convert(&format!(r"\node[shape={shape}] (N) at (3,0.5) {{}}; \draw[red] (0,0.5) -- (N);"), &[&FLAGS]);
        assert!(r.warnings.is_empty(), "{shape}: {:?}", r.warnings);
        let red = r.svg.split("<path d=\"").find(|p| p.contains("#ff0000")).unwrap();
        let n: Vec<f64> = red[..red.find('"').unwrap()].split([' ', 'M', 'L']).filter_map(|t| t.parse().ok()).collect();
        assert!((n[2] - n[0] - x * 28.3465).abs() < 0.05, "{shape}: {n:?}");
    }
}

#[test]
fn a_library_shape_has_a_background_white_or_its_fill() {
    assert!(convert(r"\node[shape=post] {};", &[&FLAGS]).svg.contains("fill=\"#ffffff\""));
    let filled = convert(r"\node[shape=post, fill=red] {};", &[&FLAGS]).svg;
    assert!(filled.contains("fill=\"#ff0000\"") && !filled.contains("fill=\"#ffffff\""), "{filled}");
}

#[test]
fn a_library_shape_makes_room_for_its_extent() {
    let r = convert(r"\node[shape=mark] {};", &[&FLAGS]);
    // 2 cm square, plus 2 bp of margin on each side
    let viewbox: Vec<f64> = r.svg.split("viewBox=\"").nth(1).unwrap().split('"').next().unwrap().split(' ').map(|v| v.parse().unwrap()).collect();
    assert!((viewbox[2] - 2.0 * 28.3465 - 3.986).abs() < 0.01 && (viewbox[3] - 2.0 * 28.3465 - 3.986).abs() < 0.01, "{viewbox:?}");
}

#[test]
fn a_library_shape_is_known_only_with_its_library() {
    assert!(!convert(r"\node[shape=flag] {};", &[]).warnings.is_empty());
}

thread_local! {
    /// What each `badge` was made from, in this test's thread.
    static SEEN: std::cell::RefCell<Vec<ShapeInput>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// `badge`: a square outlined red, dashed and with round caps; filled blue when the node is
/// filled; a green background, a solid dot in an unknown color; `+` left of its center and `CLK`
/// in `\tiny` ending at its right side on the baseline.
fn badge(_cx: &mut Context, input: &ShapeInput) -> Result<NodeShape, Error> {
    SEEN.with(|s| s.borrow_mut().push(input.clone()));
    let p = Point::new;
    let square =
        vec![PathSegment::MoveTo(p(-0.5, -0.5)), PathSegment::LineTo(p(0.5, -0.5)), PathSegment::LineTo(p(0.5, 0.5)), PathSegment::LineTo(p(-0.5, 0.5)), PathSegment::Close];
    let dot = vec![PathSegment::MoveTo(p(0.0, 0.0)), PathSegment::LineTo(p(0.1, 0.0)), PathSegment::LineTo(p(0.1, 0.1)), PathSegment::Close];
    Ok(NodeShape {
        border: ShapeBorder::Rectangle(p(0.5, 0.5)),
        paths: vec![
            ShapePath { segments: square.clone(), paint: ShapePaint::Background, color: Some("green".into()), ..Default::default() },
            ShapePath { segments: square.clone(), paint: ShapePaint::Fill, color: Some("blue".into()), ..Default::default() },
            ShapePath {
                segments: square,
                paint: ShapePaint::Stroke(0.02),
                color: Some("red!50!black".into()),
                dash: Some(vec![0.1, 0.05]),
                cap: Some(LineCap::Round),
                ..Default::default()
            },
            ShapePath { segments: dot, paint: ShapePaint::Solid, color: Some("no such color".into()), ..Default::default() },
        ],
        texts: vec![
            ShapeText { text: "$+$".into(), at: p(0.0, 0.0), anchor: TextAnchor::East, ..Default::default() },
            ShapeText { text: "CLK".into(), at: p(2.0, 0.0), anchor: TextAnchor::BaseEast, font: Some("\\tiny".into()), ..Default::default() },
        ],
        ..NodeShape::default()
    })
}

/// `tag`: `A` turned a quarter, and `B` kept upright.
fn tag(_cx: &mut Context, _input: &ShapeInput) -> Result<NodeShape, Error> {
    let p = Point::new;
    Ok(NodeShape {
        texts: vec![
            ShapeText { text: "A".into(), at: p(1.0, 0.0), anchor: TextAnchor::North, turn: 90.0, ..Default::default() },
            ShapeText { text: "B".into(), at: p(-1.0, 0.0), anchor: TextAnchor::Center, upright: true, ..Default::default() },
        ],
        ..NodeShape::default()
    })
}

/// `remember`: `\tikramemo` is `\tikzlastnode`, as tokens, everywhere; `\tikrashape` says whether
/// `transform shape` is on.
fn remember(cx: &mut Context, _value: Option<Tokens>) -> Result<(), Error> {
    cx.define_tex("tikramemo", "\\tikzlastnode", true);
    let on = cx.transform_shape();
    cx.define_text("tikrashape", if on { "on" } else { "off" }, true);
    cx.declare_shape("badge", badge);
    cx.declare_shape("compass", compass);
    cx.declare_shape("tag", tag);
    Ok(())
}

static BADGES: Library = Library { name: "badges", definitions: || String::new(), keys: &[("/tikz/remember", remember)] };

#[test]
fn a_library_shape_path_has_its_own_color_dash_and_caps() {
    let r = convert(r"\node[remember, shape=badge, fill=yellow] {};", &[&BADGES]);
    // the stroke in its color, dashed and round; the fill and background in theirs
    assert!(r.svg.contains(r##"stroke="#800000""##) && r.svg.contains("stroke-dasharray=\"2.835 1.417\"") && r.svg.contains(r#"stroke-linecap="round""#), "{}", r.svg);
    assert!(r.svg.contains(r##"fill="#0000ff""##) && r.svg.contains(r##"fill="#00ff00""##), "{}", r.svg);
    // an unknown color: the node's, with a warning
    assert!(r.warnings.iter().any(|w| w.contains("no such color")), "{:?}", r.warnings);
    // not filled: no fill in its color, the background still in its own
    let plain = convert(r"\node[remember, shape=badge] {};", &[&BADGES]);
    assert!(!plain.svg.contains(r##"fill="#0000ff""##) && plain.svg.contains(r##"fill="#00ff00""##), "{}", plain.svg);
    // butt and rect caps, and a solid in the line color
    assert!(!plain.svg.contains("stroke-linecap=\"square\""));
}

#[test]
fn a_library_shape_draws_texts_turned_with_the_node() {
    let r = convert(r"\node[remember, shape=badge] {};", &[&BADGES]);
    // two texts besides the node's empty one, in the figure's words
    assert_eq!(r.svg.matches("<g transform=").count(), 2, "{}", r.svg);
    assert!(r.svg.contains("CLK"), "{}", r.svg);
    // `CLK` past the square makes room for itself: the picture is wider than the square
    let width = |svg: &str| -> f64 { svg.split("viewBox=\"").nth(1).unwrap().split(' ').nth(2).unwrap().parse().unwrap() };
    assert!(width(&r.svg) > 2.0 * 28.3465, "{}", width(&r.svg));
    // mirrored with the node: the texts' placement turns too
    let mirrored = convert(r"\node[remember, shape=badge, xscale=-1] {};", &[&BADGES]);
    let places = |svg: &str| svg.match_indices("<g transform=\"").map(|(i, _)| svg[i + 14..].split('"').next().unwrap().to_string()).collect::<Vec<_>>();
    assert_ne!(places(&r.svg), places(&mirrored.svg));
    assert!(places(&mirrored.svg).iter().all(|p| p.starts_with("matrix(-1")), "{:?}", places(&mirrored.svg));
}

#[test]
fn a_library_shape_text_turns_or_stays_upright() {
    let places = |body: &str| {
        let svg = convert(body, &[&BADGES]).svg;
        svg.match_indices("<g transform=\"").map(|(i, _)| svg[i + 14..].split('"').next().unwrap().to_string()).collect::<Vec<_>>()
    };
    // `A` along a side, `B` as it is
    let plain = places(r"\node[remember, shape=tag] {};");
    assert!(plain[0].starts_with("matrix(0 -1 1 0"), "{plain:?}");
    assert!(plain[1].starts_with("translate("), "{plain:?}");
    // the node turned: `A` turns with it, `B` stays upright but moves
    let turned = places(r"\node[remember, shape=tag, rotate=90] {};");
    assert!(turned[0].starts_with("matrix(-1 0 0 -1"), "{turned:?}");
    assert!(turned[1].starts_with("translate("), "{turned:?}");
    assert_ne!(plain[1], turned[1]);
}

#[test]
fn a_library_shape_knows_its_node() {
    SEEN.with(|s| s.borrow_mut().clear());
    let r =
        convert(r"\begin{scope}[name prefix=S-] \node[remember, shape=badge, fill=red!50, draw=blue, xscale=-1] (T1) {}; \end{scope} \node[remember, shape=badge] {};", &[&BADGES]);
    assert!(r.warnings.iter().all(|w| w.contains("no such color")), "{:?}", r.warnings);
    let seen = SEEN.with(|s| s.borrow().clone());
    assert_eq!(seen[0].node.as_deref(), Some("S-T1"));
    assert_eq!(seen[0].fill.as_deref(), Some("rgb,1:red,1;green,0.5;blue,0.5"));
    assert_eq!(seen[0].draw.as_deref(), Some("rgb,1:red,0;green,0;blue,1"));
    assert!(seen[0].transform.a < 0.0, "{:?}", seen[0].transform);
    assert_eq!((seen[1].node.as_deref(), seen[1].fill.as_deref(), seen[1].draw.as_deref()), (None, None, None));
    // the colors work as colors again
    let again = convert(&format!(r"\fill[{{{}}}] (0,0) circle (1);", seen[0].fill.as_deref().unwrap()), &[]);
    assert!(again.svg.contains(r##"fill="#ff8080""##) && again.warnings.is_empty(), "{:?}", again.warnings);
    // `transform shape` scales the shape with the picture
    SEEN.with(|s| s.borrow_mut().clear());
    convert(r"\begin{scope}[scale=2, transform shape] \node[remember, shape=badge] {}; \end{scope}", &[&BADGES]);
    assert_eq!(SEEN.with(|s| s.borrow()[0].transform.a), 2.0);
}

#[test]
fn a_library_defines_tex_and_knows_transform_shape() {
    // \tikramemo is \tikzlastnode itself: it names the last node
    let r = convert(r"\node[remember] (A) at (1,0) {}; \draw[red] (0,0) -- (\tikramemo);", &[&BADGES]);
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    // read where the key runs: on in the first node, off in the second
    let r = convert(r"\node[transform shape, remember] {\tikrashape}; \node[remember] at (2,0) {\tikrashape};", &[&BADGES]);
    let words = r.svg.split("<desc").nth(1).unwrap().split("</desc>").next().unwrap().to_string();
    assert!(words.contains("on") && words.contains("off"), "{words}");
}

/// Every text anchor, in this order, for `gx` at the origin; strokes with butt and rect caps.
const ANCHORS: [TextAnchor; 15] = [
    TextAnchor::Center,
    TextAnchor::North,
    TextAnchor::South,
    TextAnchor::East,
    TextAnchor::West,
    TextAnchor::NorthEast,
    TextAnchor::NorthWest,
    TextAnchor::SouthEast,
    TextAnchor::SouthWest,
    TextAnchor::Base,
    TextAnchor::BaseEast,
    TextAnchor::BaseWest,
    TextAnchor::Mid,
    TextAnchor::MidEast,
    TextAnchor::MidWest,
];

fn compass(_cx: &mut Context, _input: &ShapeInput) -> Result<NodeShape, Error> {
    let line = vec![PathSegment::MoveTo(Point::new(-1.0, 0.0)), PathSegment::LineTo(Point::new(1.0, 0.0))];
    Ok(NodeShape {
        paths: [LineCap::Butt, LineCap::Rect]
            .into_iter()
            .map(|c| ShapePath { segments: line.clone(), paint: ShapePaint::Stroke(0.1), cap: Some(c), ..Default::default() })
            .collect(),
        texts: ANCHORS.iter().map(|&anchor| ShapeText { text: "gx".into(), anchor, ..Default::default() }).collect(),
        ..NodeShape::default()
    })
}

#[test]
fn a_library_shape_text_goes_where_its_anchor_says() {
    let r = convert(r"\node[remember, shape=compass] {};", &[&BADGES]);
    assert!(r.svg.contains(r#"stroke-linecap="square""#), "{}", r.svg);
    // where each `gx` starts (the left end of its baseline), in SVG units (y down), all for one point
    let at: Vec<(f64, f64)> = r
        .svg
        .match_indices("<g transform=\"translate(")
        .map(|(i, _)| {
            let v: Vec<f64> = r.svg[i + 24..].split(')').next().unwrap().split(' ').map(|n| n.parse().unwrap()).collect();
            (v[0], v[1])
        })
        .collect();
    assert_eq!(at.len(), 15, "{}", r.svg);
    let a = |k: TextAnchor| at[ANCHORS.iter().position(|&x| x == k).unwrap()];
    use TextAnchor::*;
    // west of the point, the box starts there; east, it ends there; centered between
    let x0 = a(West).0;
    for (w, c, e) in [(West, Center, East), (NorthWest, North, NorthEast), (SouthWest, South, SouthEast), (BaseWest, Base, BaseEast), (MidWest, Mid, MidEast)] {
        assert!(a(w).0 == x0 && a(e).0 < a(c).0 && a(c).0 < x0 && ((a(c).0 - x0) * 2.0 - (a(e).0 - x0)).abs() < 2e-3, "{w:?} {c:?} {e:?}: {at:?}");
        assert!(a(w).1 == a(c).1 && a(c).1 == a(e).1, "{at:?}");
    }
    // the baseline below the point by the height for north, half of height less depth for center,
    // half an ex for mid, not at all for base, above it by the depth for south (y down)
    assert!(a(North).1 > a(Center).1 && a(North).1 > a(Mid).1 && a(Mid).1 > a(Base).1 && a(Center).1 > a(Base).1 && a(Base).1 > a(South).1, "{at:?}");
    assert!(a(Mid).1 != a(Center).1, "{at:?}");
}

/// `probe={code}{point}`: `code` measured unseen; a coordinate `at` where its `point` was, and
/// `lo` and `hi` at the corners of what it drew.
fn probe(cx: &mut Context, value: Option<Tokens>) -> Result<(), Error> {
    let a = value.unwrap_or_default().arguments(2);
    let point = cx.expand(a[1].clone())?;
    let (at, bounds) = cx.measure(a[0].clone(), |cx| cx.point(&point))?;
    let (at, (lo, hi)) = (at?, bounds.expect("the code drew"));
    for (name, p) in [("at", at), ("lo", lo), ("hi", hi)] {
        cx.run(Tokens::text(&format!("\\coordinate ({name}) at ({}cm,{}cm);", p.x, p.y)))?;
    }
    Ok(())
}

static PROBES: Library = Library { name: "probes", definitions: String::new, keys: &[("/tikz/probe", probe)] };

#[test]
fn a_library_measures_code_without_drawing_it() {
    let code = r"\draw[red] (1,1) rectangle (3,2); \node[draw] (n) at (5,0) {x};";
    let probed = convert(&format!(r"\path[probe={{{code}}}{{n.center}}]; \fill (at) circle (1pt) (lo) circle (1pt) (hi) circle (1pt);"), &[&PROBES]);
    assert!(probed.warnings.is_empty(), "{:?}", probed.warnings);
    // only the three dots: neither the rectangle nor the node is drawn
    let body = probed.svg.split("</metadata>").nth(1).unwrap();
    assert!(body.matches("<path").count() == 1 && !body.contains("<text") && !body.contains("#ff0000"), "{body}");
    // the dots where the node's center and the corners of the rectangle and the node are, as drawn
    // apart: the picture as large
    let dots = convert(r"\fill (5,0) circle (1pt) (1,-0.2) circle (1pt) (5.2,2) circle (1pt);", &[]);
    let size = |svg: &str| -> Vec<f64> {
        let view = svg.split("viewBox=\"").nth(1).unwrap().split('"').next().unwrap();
        view.split(' ').skip(2).map(|v| v.parse().unwrap()).collect()
    };
    let (p, d) = (size(&probed.svg), size(&dots.svg));
    assert!(p.iter().zip(&d).all(|(a, b)| (a - b).abs() < 1.0), "{p:?} for about {d:?}");
    // the node it named is forgotten
    let after = convert(&format!(r"\path[probe={{{code}}}{{n.center}}]; \draw (n) -- (0,0);"), &[&PROBES]);
    assert!(!after.warnings.is_empty(), "the node is unknown after");
}
