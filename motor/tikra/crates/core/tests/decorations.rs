//! Decorations as pgf's automata draw them, the `decorate` path operation, decorations declared in
//! the document, the basic layer's `\pgfdecoration`, and `preaction` / `postaction`.

use tikra::canvas::{Item, Paint};
use tikra::engine::run_document;
use tikra::scene::{Point, Segment};
use tikra::PT;

fn picture(body: &str) -> (Vec<Item>, Vec<String>) {
    // the baseline at y = 0 keeps the picture's coordinates, and a fixed box starting at x = 0
    // (strokes would widen it by half their width)
    let (c, w) = run_document(&format!("\\begin{{tikzpicture}}[baseline=0pt]\\useasboundingbox (0,-5) rectangle (20,5);{body}\\end{{tikzpicture}}"))
        .unwrap_or_else(|e| panic!("{e}\n{body}"));
    (c.items, w)
}

/// The paths drawn, in order.
fn paths(items: &[Item]) -> Vec<Vec<Segment>> {
    items.iter().filter_map(|i| if let Item::Path { segments, .. } = i { Some(segments.clone()) } else { None }).collect()
}

fn ends(segs: &[Segment]) -> Vec<Point> {
    segs.iter()
        .filter_map(|s| match s {
            Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => Some(*p),
            Segment::Close => None,
        })
        .collect()
}

fn near(p: Point, x: f64, y: f64) -> bool {
    (p.x - x).abs() < 1e-4 && (p.y - y).abs() < 1e-4
}

#[test]
fn zigzag_is_pgfs_automaton() {
    let (items, w) = picture(r"\draw[decorate, decoration=zigzag] (0,0) -- (3,0);");
    assert!(w.is_empty(), "{w:?}");
    let p = ends(&paths(&items)[0]);
    // up from center by a quarter segment, then down and up every half segment
    assert!(near(p[0], 0.0, 0.0), "{p:?}");
    assert!(near(p[1], 2.5 * PT, 2.5 * PT), "{p:?}");
    assert!(near(p[2], 7.5 * PT, -2.5 * PT), "{p:?}");
    assert!(near(p[3], 12.5 * PT, 2.5 * PT), "{p:?}");
    // the end: back to the line, then a line to the end of the path
    assert!(near(*p.last().unwrap(), 3.0, 0.0), "{p:?}");
    let before = p[p.len() - 2];
    assert!(before.y.abs() < 1e-9, "center finish on the line: {p:?}");
}

#[test]
fn pre_and_post_lengths() {
    let (items, _) = picture(r"\draw[decorate, decoration={zigzag, pre length=1cm, post length=1cm}] (0,0) -- (3,0);");
    let p = ends(&paths(&items)[0]);
    assert!(near(p[1], 1.0, 0.0), "a line for the pre length: {p:?}");
    assert!(near(p[2], 1.0 + 2.5 * PT, 2.5 * PT), "{p:?}");
    assert!(near(*p.last().unwrap(), 3.0, 0.0), "{p:?}");
    // pre=moveto leaves a gap
    let (items, _) = picture(r"\draw[decorate, decoration={zigzag, pre=moveto, pre length=1cm}] (0,0) -- (3,0);");
    let s = &paths(&items)[0];
    assert!(matches!(s[1], Segment::MoveTo(q) if near(q, 1.0, 0.0)), "{s:?}");
}

#[test]
fn raise_and_mirror() {
    let (items, _) = picture(r"\draw[decorate, decoration={brace, amplitude=10pt, mirror, raise=5pt}] (0,0) -- (3,0);");
    let p = ends(&paths(&items)[0]);
    // after the moveto of the path, all of it below
    assert!(p[1..].iter().all(|q| q.y < -4.0 * PT), "below the path: {p:?}");
    assert!(p.iter().any(|q| (q.y + 15.0 * PT).abs() < 1e-4), "the tip 10pt further: {p:?}");
}

#[test]
fn curves_are_followed() {
    let (items, w) = picture(r"\draw[decorate, decoration={zigzag, amplitude=1mm}] (0,0) arc (180:0:1);");
    assert!(w.is_empty(), "{w:?}");
    let p = ends(&paths(&items)[0]);
    assert!(p.len() > 15, "{}", p.len());
    // the picture starts where the zigzag goes furthest left: measure from the path's start
    let o = p[0];
    for q in &p[1..p.len() - 1] {
        let r = (q.x - o.x - 1.0).hypot(q.y - o.y);
        assert!((r - 1.0).abs() < 0.11, "{q:?} at {r} from the center");
    }
}

#[test]
fn decorate_operation_and_nesting() {
    let (items, w) = picture(r"\draw (0,0) -- (1,0) decorate[decoration=zigzag] { -- (3,0) } -- (3,1);");
    assert!(w.is_empty(), "{w:?}");
    let p = ends(&paths(&items)[0]);
    assert!(near(p[1], 1.0, 0.0), "{p:?}");
    assert!(p.len() > 10 && near(*p.last().unwrap(), 3.0, 1.0), "{p:?}");
    // Koch curves inside Koch curves: 5, then 25 segments
    let (items, _) = picture(r"\draw[decoration=Koch curve type 1] decorate{ decorate{ (0,0) -- (3,0) }};");
    assert_eq!(ends(&paths(&items)[0]).len(), 26);
}

#[test]
fn markings_with_arrows_and_nodes() {
    let (items, w) = picture(r"\draw[decoration={markings, mark=at position .5 with {\arrow{>}}, mark=at position 1cm with {\node {a};}}, postaction={decorate}] (0,0) -- (2,0);");
    assert!(w.is_empty(), "{w:?}");
    let all = paths(&items);
    // the path, then the arrow tip at the middle, pointing right
    assert!(near(ends(&all[0])[1], 2.0, 0.0));
    let tip = ends(&all[1]);
    // (the stroked Computer Modern tip reaches the middle with its line width)
    assert!(tip.iter().any(|q| (q.x - 1.0).abs() < 0.01 && q.y.abs() < 1e-6), "{tip:?}");
    assert!(tip.iter().all(|q| q.x <= 1.0 + 1e-6), "{tip:?}");
    // the node is at 1cm
    let text = items.iter().find_map(|i| if let Item::Text { place, .. } = i { Some(*place) } else { None }).expect("a node");
    assert!((text.e - 1.0).abs() < 0.1, "{text:?}");
}

#[test]
fn mark_distance_is_written_as_tex_writes_a_dimension() {
    // `120.0pt`, as `\the` writes it: a mark is drawn only if the text is that
    let (items, w) = picture(
        r"\def\want{120.0pt}
          \path[decoration={markings, mark=at position 120pt with {\edef\d{\pgfkeysvalueof{/pgf/decoration/mark info/distance from start}}
            \edef\d{\detokenize\expandafter{\d}}\edef\w{\detokenize\expandafter{\want}}\ifx\d\w \fill (0,0) circle (1pt);\fi}}, decorate] (0,0) -- (10,0);",
    );
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(paths(&items).len(), 1);
}

#[test]
fn marks_between_positions() {
    let (items, _) = picture(r"\path[decoration={markings, mark=between positions 0 and 1 step 0.25 with {\fill (0,0) circle (1pt);}}, decorate] (0,0) -- (4,0);");
    assert_eq!(paths(&items).len(), 5);
}

#[test]
fn preaction_and_postaction() {
    let (items, w) = picture(r"\draw[preaction={draw, line width=4pt}, postaction={draw, blue, line width=1pt}, red, line width=2pt] (0,0) -- (1,0);");
    assert!(w.is_empty(), "{w:?}");
    let strokes: Vec<(f64, tikra::color::Color)> = items.iter().filter_map(|i| if let Item::Path { stroke: Some(s), .. } = i { Some((s.width, s.color)) } else { None }).collect();
    assert_eq!(strokes.len(), 3);
    // the preaction draws first, with the scope's color; the postaction after, on top
    assert!((strokes[0].0 - 4.0 * PT).abs() < 1e-9);
    assert_eq!(strokes[0].1, tikra::color::Color::BLACK);
    assert!((strokes[1].0 - 2.0 * PT).abs() < 1e-9);
    assert!((strokes[2].0 - 1.0 * PT).abs() < 1e-9);
    // decorate only in the postaction
    let (items, _) = picture(r"\draw[decoration=zigzag, postaction={decorate, draw, red}] (0,0) -- (3,0);");
    let all = paths(&items);
    assert_eq!(ends(&all[0]).len(), 2);
    assert!(ends(&all[1]).len() > 10);
}

#[test]
fn declared_decorations_run_as_tex() {
    let (items, w) = picture(
        r"\pgfdeclaredecoration{example}{initial}{
            \state{initial}[width=10pt]{\pgfpathlineto{\pgfpoint{0pt}{5pt}}\pgfpathlineto{\pgfpoint{5pt}{5pt}}
              \pgfpathlineto{\pgfpoint{5pt}{-5pt}}\pgfpathlineto{\pgfpoint{10pt}{-5pt}}\pgfpathlineto{\pgfpoint{10pt}{0pt}}}
            \state{final}{\pgfpathlineto{\pgfpointdecoratedpathlast}}}
          \draw[decorate, decoration=example] (0,0) -- (3,0);",
    );
    assert!(w.is_empty(), "{w:?}");
    let p = ends(&paths(&items)[0]);
    assert!(near(p[1], 0.0, 5.0 * PT) && near(p[3], 5.0 * PT, -5.0 * PT), "{p:?}");
    assert!(near(*p.last().unwrap(), 3.0, 0.0), "{p:?}");
}

#[test]
fn basic_layer_decoration_environment() {
    let (items, w) = picture(
        r"\begin{pgfdecoration}{{lineto}{\pgfdecoratedpathlength/3},{zigzag}{\pgfdecoratedpathlength/3},{lineto}{\pgfdecoratedremainingdistance}}
          \pgfpathmoveto{\pgfpointorigin}\pgfpathlineto{\pgfpoint{3cm}{0cm}}
          \end{pgfdecoration}\pgfusepath{stroke}",
    );
    assert!(w.is_empty(), "{w:?}");
    let p = ends(&paths(&items)[0]);
    assert!(near(p[1], 1.0, 0.0), "{p:?}");
    assert!(p.iter().any(|q| q.y > 2.0 * PT), "{p:?}");
}

#[test]
fn text_along_path() {
    let (items, w) = picture(r"\path[decorate, decoration={text along path, text={abc}}] (0,0) -- (3,0);");
    assert!(w.is_empty(), "{w:?}");
    let places: Vec<f64> = items.iter().filter_map(|i| if let Item::Text { place, .. } = i { Some(place.e) } else { None }).collect();
    assert_eq!(places.len(), 3);
    assert!(places[0] < 0.05 && places[0] < places[1] && places[1] < places[2], "{places:?}");
}

#[test]
fn footprints_and_shapes() {
    let (items, w) = picture(r"\fill[decorate, decoration={footprints, foot length=20pt}] (0,0) -- (3,0);");
    assert!(w.is_empty(), "{w:?}");
    assert!(paths(&items)[0].iter().filter(|s| matches!(s, Segment::Close)).count() >= 6 * 3);
    let (items, _) = picture(r"\draw[decorate, decoration=crosses] (0,0) -- (1,0);");
    assert!(paths(&items)[0].iter().filter(|s| matches!(s, Segment::MoveTo(_))).count() > 6);
    let (items, _) = picture(r"\fill[decorate, decoration={shape backgrounds, shape=circle, shape size=2mm}] (0,0) -- (2,0);");
    let s = &paths(&items)[0];
    assert!(s.iter().filter(|s| matches!(s, Segment::Close)).count() >= 8, "{s:?}");
    assert!(items.iter().any(|i| matches!(i, Item::Path { fill: Some(f), .. } if matches!(f.paint, Paint::Solid(_)))));
}

#[test]
fn unknown_names_and_keys_are_reported() {
    let (_, w) = picture(r"\draw[decorate, decoration={zigzag, wiggle=3}] (0,0) -- (1,0);");
    assert!(w.iter().any(|w| w.contains("wiggle")), "{w:?}");
    let (items, w) = picture(r"\draw[decorate, decoration={name=no such thing}] (0,0) -- (1,0);");
    assert!(w.iter().any(|w| w.contains("no such thing")), "{w:?}");
    assert_eq!(ends(&paths(&items)[0]).len(), 2);
    // decorate on a scope does nothing: each path resets it
    let (items, _) = picture(r"\begin{scope}[decorate, decoration=zigzag]\draw (0,0) -- (1,0);\end{scope}");
    assert_eq!(ends(&paths(&items)[0]).len(), 2);
}

#[test]
fn text_effects_make_a_node_per_character() {
    let (items, w) = picture(
        r"\path[decoration={text effects along path, text={ab-c}, text effects/.cd, word separator=-,
            every letter/.style={fill=blue}, text along path}, decorate] (0,0) -- (3,0);",
    );
    assert!(w.is_empty(), "{w:?}");
    let texts = items.iter().filter(|i| matches!(i, Item::Text { .. })).count();
    assert_eq!(texts, 4);
    // the three letters are filled, the separator is not
    let fills = items.iter().filter(|i| matches!(i, Item::Path { fill: Some(_), .. })).count();
    assert_eq!(fills, 3);
    // path from text: a lone point becomes a line as long as the text
    let (items, _) = picture(r"\path[decoration={text effects along path, text={abc}, text effects/.cd, path from text}, decorate] (0,0);");
    assert_eq!(items.iter().filter(|i| matches!(i, Item::Text { .. })).count(), 3);
}

#[test]
fn replaced_characters_and_relative_controls() {
    // the second control of `.. controls +(…) and +(…) ..` is relative to the end
    let (items, _) = picture(r"\draw (0,0) .. controls +(1,0) and +(-1,0) .. (3,2);");
    let s = &paths(&items)[0];
    assert!(matches!(s[1], Segment::CurveTo(a, b, _) if near(a, 1.0, 0.0) && near(b, 2.0, 2.0)), "{s:?}");
    // a character drawn by code instead of a node
    let (items, w) =
        picture(r"\path[decoration={text effects along path, text={a0a}, text effects/.cd, replace characters=0 with {\fill circle [radius=2pt];}}, decorate] (0,0) -- (3,0);");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(items.iter().filter(|i| matches!(i, Item::Text { .. })).count(), 2);
    assert_eq!(paths(&items).len(), 1);
}

#[test]
fn the_mindmap_library_sets_the_decoration_angle_to_20() {
    // as pgf's mindmap library does when it loads: each wave starts at 20 degrees, not 45
    let (items, _) = picture(r"\usetikzlibrary{mindmap}\draw[decorate,decoration={waves,radius=4mm}] (0,0) -- (3,0);");
    let top = ends(&paths(&items)[0]).iter().map(|p| p.y).fold(f64::MIN, f64::max);
    assert!((top - 0.4 * 20f64.to_radians().sin()).abs() < 1e-3, "{top}");
}

#[test]
fn a_control_space_in_text_effects_is_a_space() {
    // `\ ` at the end of the text: a blank, not a backslash that takes the closing brace with it
    let (_, w) = picture(r"\path[decoration={text effects along path, text={ab!\ }}, decorate] (0,0) -- (3,0);");
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn snakes_decorate_each_line_of_the_path() {
    let same = |snake: &str, decorated: &str| {
        let (a, w) = picture(snake);
        assert!(w.is_empty(), "{w:?}");
        let (b, _) = picture(decorated);
        assert_eq!(format!("{:?}", paths(&a)), format!("{:?}", paths(&b)), "{snake}");
    };
    // each line is decorated on its own, from the option on
    same(r"\draw[snake=zigzag] (0,0) -- (3,0) -- (3,2);", r"\draw (0,0) decorate[decoration=zigzag]{-- (3,0)} decorate[decoration=zigzag]{-- (3,2)};");
    same(r"\draw (0,0) -- (1,0) [snake=coil] -- (2,0);", r"\draw (0,0) -- (1,0) decorate[decoration=coil]{-- (2,0)};");
    same(r"\draw[snake=zigzag] (0,0) |- (2,1);", r"\draw (0,0) decorate[decoration=zigzag]{-- (0,1)} decorate[decoration=zigzag]{-- (2,1)};");
    same(r"\draw[snake=zigzag] (0,0) to (2,1);", r"\draw (0,0) decorate[decoration=zigzag]{-- (2,1)};");
    // the older names of the decoration keys
    same(
        r"\draw[snake=zigzag, line before snake=2mm, line after snake=5mm, segment amplitude=4pt] (0,0) -- (3,0);",
        r"\draw[decorate, decoration={zigzag, pre=lineto, pre length=2mm, post=lineto, post length=5mm, amplitude=4pt}] (0,0) -- (3,0);",
    );
    same(
        r"\draw[snake=ticks, segment length=5mm, gap around snake=3mm] (0,0) -- (3,0);",
        r"\draw[decorate, decoration={ticks, segment length=5mm, pre=moveto, pre length=3mm, post=moveto, post length=3mm}] (0,0) -- (3,0);",
    );
    same(
        r"\draw[snake=brace, gap before snake=1mm, gap after snake=2mm, raise snake=2pt, mirror snake] (0,0) -- (3,0);",
        r"\draw[decorate, decoration={brace, pre=moveto, pre length=1mm, post=moveto, post length=2mm, raise=2pt, mirror}] (0,0) -- (3,0);",
    );
    same(r"\draw[snake=zigzag, snake=none] (0,0) -- (3,0);", r"\draw (0,0) -- (3,0);");
    // the sides of a rectangle and the line that closes a path too, then closed
    for body in [r"\draw[snake=zigzag] (0,0) rectangle (3,2);", r"\draw[snake=zigzag] (0,0) -- (3,2) -| cycle;", r"\draw[snake=zigzag] (0,0) -- (3,0) -- (3,2) -- cycle;"] {
        let (items, _) = picture(body);
        let s = &paths(&items)[0];
        assert!(matches!(s.last(), Some(Segment::Close)), "{body}: {s:?}");
        let p = ends(s);
        assert!(p.len() > 30 && near(*p.last().unwrap(), 0.0, 0.0), "{body}: {p:?}");
        assert!(p.iter().all(|q| q.y < 2.0 + 3.0 * PT), "{body}: {p:?}");
    }
    same(r"\draw[snake=zigzag] (0,0) rectangle node{} (3,2);", r"\draw[snake=zigzag] (0,0) rectangle (3,2);");
    // a line after the path was closed starts from where it was closed
    let (items, _) = picture(r"\draw[snake=zigzag] (0,0) -- (1,0) -- cycle -- (2,1);");
    let s = &paths(&items)[0];
    let after = s.iter().position(|x| matches!(x, Segment::Close)).unwrap();
    let p = ends(&s[after + 1..]);
    assert!(near(p[0], 0.0, 0.0) && near(*p.last().unwrap(), 2.0, 1.0) && p.len() > 5, "{s:?}");
}
