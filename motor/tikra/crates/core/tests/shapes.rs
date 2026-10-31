//! Node shapes of the shape libraries: drawn without warnings, with their named anchors usable in
//! coordinates.

use tikra::canvas::Item;
use tikra::scene::{Point, Segment};
use tikra::PT;

fn no_warnings(body: &str) -> String {
    let r = tikra::convert(&format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}")).unwrap();
    assert!(r.warnings.is_empty(), "{body}: {:?}", r.warnings);
    r.svg
}

#[test]
fn geometric_shapes_and_their_anchors() {
    for (shape, anchors) in [
        ("regular polygon, regular polygon sides=6", "corner 1,side 6"),
        ("star, star points=7, star point ratio=2", "outer point 7,inner point 1"),
        ("trapezium, trapezium left angle=70", "bottom left corner,top side"),
        ("semicircle", "apex,arc start,chord center"),
        ("isosceles triangle, shape border rotate=90", "apex,left corner,lower side"),
        ("kite, kite vertex angles=100 and 50", "upper vertex,lower left side"),
        ("dart", "tip,tail center,left tail"),
        ("circular sector, circular sector angle=90", "arc start,sector center,arc center"),
        ("cylinder, aspect=0.5", "top,before top,after bottom,shape center"),
    ] {
        let mut body = format!("\\node[draw, {shape}] (s) {{text}};");
        for a in anchors.split(',') {
            body.push_str(&format!("\\draw (0,2) -- (s.{a}) -- (s.30) -- (s.mid);"));
        }
        let svg = no_warnings(&body);
        assert!(svg.matches("<path").count() >= 2, "{shape}");
    }
}

/// The paths the engine draws for a picture body, in cm.
fn paths(body: &str) -> Vec<Vec<Segment>> {
    let (canvas, warnings) = tikra::engine::run_document(&format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}")).unwrap();
    assert!(warnings.is_empty(), "{body}: {warnings:?}");
    canvas.items.into_iter().filter_map(|i| if let Item::Path { segments, .. } = i { Some(segments) } else { None }).collect()
}

fn near(p: Point, x: f64, y: f64) -> bool {
    (p.x - x).abs() < 1e-3 && (p.y - y).abs() < 1e-3
}

#[test]
fn a_relative_second_control_is_relative_to_the_end() {
    let p = paths(r"\draw (0,0) .. controls +(1,1) and +(-1,1) .. (2,0);");
    match (p[0][0], p[0][1]) {
        (Segment::MoveTo(o), Segment::CurveTo(a, b, _)) => assert!(near(a.minus(o), 1.0, 1.0) && near(b.minus(o), 1.0, 1.0), "{a:?} {b:?}"),
        ref s => panic!("{s:?}"),
    }
}

#[test]
fn lines_stop_at_the_border_with_outer_sep() {
    // an empty circle of radius √2 · 3.333pt, plus .2pt of outer sep
    let r = (2.0f64.sqrt() * 3.333 + 0.2) * PT;
    let p = paths(r"\node[circle,draw] (a) {}; \draw (a) -- (1,0);");
    match (p[1][0], p[1][1]) {
        (Segment::MoveTo(q), Segment::LineTo(e)) => assert!(near(e.minus(q), 1.0 - r, 0.0), "{q:?} {e:?} {r}"),
        ref s => panic!("{s:?}"),
    }
    // a diamond: from its center toward its east corner
    let p = paths(r"\node[diamond,draw] (a) {}; \draw (a) -- (1,0);");
    match (p[1][0], p[1][1]) {
        (Segment::MoveTo(q), Segment::LineTo(e)) => assert!(near(e.minus(q), 1.0 - (2.0 * 3.333 + 0.2) * PT, 0.0), "{q:?} {e:?}"),
        ref s => panic!("{s:?}"),
    }
}

#[test]
fn split_parts_are_placed_and_named() {
    let p = paths(r"\node[circle split,draw] (s) {a \nodepart{lower} b}; \draw (s.lower) -- (s.center);");
    // the circle, the split line and the line between the anchors
    assert!(p.len() >= 3, "{p:?}");
    let p = paths(r"\node[rectangle split,rectangle split parts=3,draw] (s) {a \nodepart{two} b \nodepart{three} c}; \draw (s.two split) -- (s.text east);");
    assert!(p.len() >= 2, "{p:?}");
}

#[test]
fn shape_names_alone_are_shapes() {
    no_warnings("\\node[trapezium, draw] {a}; \\node[regular polygon, draw] at (2,0) {b};");
}

#[test]
fn a_part_named_again_starts_its_box_anew() {
    // `\nodepart{text}` at the start restarts the text box: the text part holds `text`, not nothing
    let ends = |text: &str| {
        let p = paths(&format!(r"\node[rectangle split,rectangle split parts=2,draw] (s) {{{text}}}; \draw (0,0) -- (s.text split) -- (s.text);"));
        match p.last().map(|s| s.as_slice()) {
            Some([Segment::MoveTo(_), Segment::LineTo(a), Segment::LineTo(b)]) => (*a, *b),
            s => panic!("{s:?}"),
        }
    };
    let (a, b) = ends(r"\nodepart{text}text\nodepart{two}two");
    let (c, d) = ends(r"text\nodepart{two}two");
    assert!(near(a, c.x, c.y) && near(b, d.x, d.y), "{a:?} {b:?} / {c:?} {d:?}");
}

#[test]
fn an_ellipse_callout_goes_around_through_east() {
    // pgf draws the ellipse in two arcs, to 360° and from 0°: no control point lies past the
    // ellipse's east and west, so the picture is as wide as the ellipse
    let p = paths(r"\node[ellipse callout, draw] (h) {Hallo!}; \draw (h.east) -- (h.west);");
    let (east, west) = match p[1][..] {
        [Segment::MoveTo(e), Segment::LineTo(w)] => (e.x - 0.2 * PT, w.x + 0.2 * PT),
        ref s => panic!("{s:?}"),
    };
    for s in &p[0] {
        let pts = match *s {
            Segment::MoveTo(a) | Segment::LineTo(a) => vec![a],
            Segment::CurveTo(a, b, c) => vec![a, b, c],
            _ => vec![],
        };
        for q in pts {
            assert!(q.x <= east + 1e-6 && q.x >= west - 1e-6, "{q:?} outside {west}..{east}");
        }
    }
}

#[test]
fn a_rectangle_callout_smaller_than_its_pointer_is_drawn() {
    // the text is narrower than the pointer's width: there is no room to keep its base away from
    // the corners, which must not stop the drawing
    for at in [90, 270, 180, 0] {
        let p = paths(&format!(r"\node[rectangle callout, draw, inner sep=0pt, callout relative pointer={{({at}:1cm)}}] {{\tiny .}};"));
        assert!(!p.is_empty(), "{at}");
    }
}

/// The items the engine draws for a picture body after a preamble (`@` a letter), with no
/// warnings, in the picture's coordinates (cm): drawn in a fixed box moved back after pgf's shift.
fn items(preamble: &str, body: &str) -> Vec<Item> {
    let doc = format!("\\makeatletter{preamble}\\begin{{tikzpicture}}\\useasboundingbox (-50,-50) rectangle (50,50);{body}\\end{{tikzpicture}}");
    let (canvas, warnings) = tikra::engine::run_document(&doc).unwrap();
    assert!(warnings.is_empty(), "{body}: {warnings:?}");
    let mut items = canvas.items;
    for i in &mut items {
        i.translate(Point::new(-50.0, -50.0));
    }
    items
}

/// Where the lines `\draw (point) -- +(1,0);` start, in order (cm).
fn starts(items: &[Item]) -> Vec<Point> {
    items
        .iter()
        .filter_map(|i| match i {
            Item::Path { segments, .. } => match segments[..] {
                [Segment::MoveTo(p), Segment::LineTo(q)] if near(q.minus(p), 1.0, 0.0) => Some(p),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn near_pt(p: Point, x: f64, y: f64) -> bool {
    (p.x - x * PT).abs() < 2e-3 && (p.y - y * PT).abs() < 2e-3
}

#[test]
fn a_declared_shape_saves_anchors_from_the_node_and_anchors_from_them() {
    // pdflatex: the saved anchor from the text box ("xy": 10.55562pt wide, 4.30554pt high), the
    // saved dimension from the minimum width, the saved macro, an anchor calling another one
    let shape = r"\pgfdeclareshape{marked}{
      \savedanchor\ne{\pgf@x=.5\wd\pgfnodeparttextbox \pgf@y=\ht\pgfnodeparttextbox
        \advance\pgf@x by 2pt \advance\pgf@y by 3pt}
      \saveddimen\gap{\pgfmathsetlength\pgf@x{\pgfshapeminwidth}\c@pgf@counta=4 \divide\pgf@x by\c@pgf@counta}
      \savedmacro\what{\wd0=7pt \edef\what{\the\wd0}}
      \anchor{center}{\pgfpointorigin}
      \anchor{corner}{\ne}
      \anchor{left}{\ne \pgf@x=-\pgf@x \advance\pgf@y by \gap}
      \anchor{m}{\pgfqpoint{\what}{-\what}}
      \anchor{twice}{\pgf@anchor@marked@corner \pgf@x=2\pgf@x}
      \backgroundpath{\pgfpathmoveto{\pgfpointorigin}\pgfpathlineto{\ne}}}";
    let mut body = String::from(r"\node[marked, minimum width=20pt, draw] (a) at (0,0) {xy};");
    for a in ["corner", "left", "m", "twice"] {
        body.push_str(&format!(r"\draw (a.{a}) -- +(1,0);"));
    }
    let s = starts(&items(shape, &body));
    for (k, (x, y)) in [(7.27781, 7.30554), (-7.27781, 12.30554), (7.0, -7.0), (14.55562, 7.30554)].into_iter().enumerate() {
        assert!(near_pt(s[k], x, y), "{k}: {:?} ({x}, {y})", s[k]);
    }
}

#[test]
fn a_declared_shape_draws_its_layers_and_takes_the_rectangle_border_of_its_corners() {
    // pdflatex: the border is the rectangle of the saved corners around their middle (10pt, 0pt),
    // not around the center anchor; the circle left behind the background is drawn with it, the
    // line left before the background with the foreground
    let shape = r"\pgfdeclareshape{skew}{
      \savedanchor\northeast{\pgf@x=30pt \pgf@y=10pt}
      \savedanchor\southwest{\pgf@x=-10pt \pgf@y=-10pt}
      \anchor{center}{\pgfpointorigin}
      \inheritanchorborder[from=rectangle]
      \behindbackgroundpath{\pgfpathcircle{\pgfpointorigin}{5pt}}
      \backgroundpath{\pgfpathrectanglecorners{\southwest}{\northeast}}
      \beforebackgroundpath{\pgfpathmoveto{\pgfpoint{0pt}{20pt}}\pgfpathlineto{\pgfpoint{10pt}{20pt}}}
      \foregroundpath{\pgfpathmoveto{\pgfpoint{0pt}{30pt}}\pgfpathlineto{\pgfpoint{10pt}{30pt}}}}";
    let mut body = String::from(r"\node[skew, draw] (a) {};");
    for a in ["45", "90", "0", "center"] {
        body.push_str(&format!(r"\draw (a.{a}) -- +(1,0);"));
    }
    let it = items(shape, &body);
    let s = starts(&it);
    for (k, (x, y)) in [(20.00092, 10.0), (10.0, 10.0), (30.0, 0.0), (0.0, 0.0)].into_iter().enumerate() {
        assert!(near_pt(s[k], x, y), "{k}: {:?} ({x}, {y})", s[k]);
    }
    // the circle with the rectangle, then the two lines together
    let moves = |k: usize| match &it[k] {
        Item::Path { segments, stroke: Some(_), .. } => segments.iter().filter(|s| matches!(s, Segment::MoveTo(_))).count(),
        other => panic!("{other:?}"),
    };
    assert_eq!((moves(0), moves(1)), (2, 2));
}

#[test]
fn a_declared_shape_fills_in_its_code_and_inherits_only_the_paths_it_says() {
    // the circle's saved anchors, not its background: the red disk the code fills, then the
    // circle it leaves, drawn as the node is, of the circle's radius, then the line it strokes
    let shape = r"\pgfdeclareshape{ring}{
      \inheritsavedanchors[from=circle]
      \inheritanchor[from=circle]{center}
      \inheritanchorborder[from=circle]
      \behindbackgroundpath{\pgfpathcircle{\centerpoint}{2pt}\pgfsetfillcolor{red}\pgfusepath{fill}\pgfpathcircle{\centerpoint}{\radius}}
      \beforeforegroundpath{\pgfpathmoveto{\centerpoint}\pgfpathlineto{\pgfpoint{0pt}{5pt}}\pgfusepath{stroke}}}";
    let it = items(shape, r"\node[ring, draw] (r) {};\draw (r.east) -- +(1,0);");
    match &it[0] {
        Item::Path { fill: Some(f), stroke: None, .. } => assert!(matches!(&f.paint, tikra::canvas::Paint::Solid(c) if c.r > 0.99 && c.g < 0.01), "{f:?}"),
        other => panic!("{other:?}"),
    }
    let east = starts(&it)[0];
    match &it[1] {
        Item::Path { segments, fill: None, stroke: Some(_) } => assert!(matches!(segments[0], Segment::MoveTo(p) if near(p, east.x, east.y)), "{segments:?} {east:?}"),
        other => panic!("{other:?}"),
    }
    // and the line the code strokes, last
    assert!(matches!(&it[2], Item::Path { segments, fill: None, stroke: Some(_) } if segments.len() == 2), "{:?}", it[2]);
    assert_eq!(it.iter().filter(|i| matches!(i, Item::Path { .. })).count(), 4);
}

#[test]
fn a_declared_shape_with_ports_texts_and_old_names() {
    // `\tikzoption` with a default, `\pgf@process`, `\pgfextractx`, `\pgfclosepath` and a text
    // the shape puts at one of its anchors (pdflatex: `in` at (-42.67912pt, 5pt) with 3cm)
    let shape = r"\tikzoption{wide}[3cm]{\tikzset{minimum width=#1}}
    \pgfdeclareshape{port}{
      \savedanchor\northeast{\pgfmathsetlength\pgf@x{\pgfshapeminwidth}\pgf@x=.5\pgf@x \pgf@y=10pt}
      \anchor{center}{\pgfpointorigin}
      \anchor{in}{\pgf@process{\northeast}\pgf@x=-\pgf@x \pgf@y=.5\pgf@y}
      \backgroundpath{
        \pgfextractx{\pgf@xa}{\northeast}
        \pgfpathmoveto{\pgfqpoint{-\pgf@xa}{0pt}}\pgfpathlineto{\northeast}\pgfpathlineto{\pgfqpoint{\pgf@xa}{-10pt}}\pgfclosepath
        \pgftext[left,at={\pgf@anchor@port@in}]{D}}}";
    let it = items(shape, r"\node[port, wide, draw] (p) {};\draw (p.in) -- +(1,0);");
    assert!(near_pt(starts(&it)[0], -42.67912, 5.0), "{:?}", starts(&it));
    match &it[0] {
        Item::Path { segments, .. } => {
            assert!(matches!(segments[..], [Segment::MoveTo(a), Segment::LineTo(_), Segment::LineTo(_), Segment::Close] if (a.x + 1.5).abs() < 1e-3), "{segments:?}")
        }
        other => panic!("{other:?}"),
    }
    // the text starts at the anchor
    let place = it.iter().find_map(|i| if let Item::Text { place, .. } = i { Some(*place) } else { None }).expect("the shape's text");
    assert!((place.e + 42.67912 * PT).abs() < 1e-3, "{place:?}");
}

#[test]
fn an_anchor_may_be_an_angle_expression() {
    // pgf evaluates an anchor it does not know: `a.200+360/5` is `a.272`
    let s = starts(&items("", r"\node[circle, draw] (a) {}; \draw (a.200+360/5) -- +(1,0); \draw (a.272) -- +(1,0);"));
    assert!(near(s[0], s[1].x, s[1].y) && !near(s[0], 0.0, 0.0), "{s:?}");
}

#[test]
fn the_text_box_of_declared_shapes_leaves_the_part_boxes_alone() {
    // `\pgfnodeparttextbox` is a box register only while a declared shape is computed: before a
    // multipart node, `\setbox` fills the parts' boxes, and both texts are drawn
    let preamble = r"\setbox\pgfnodeparttextbox=\hbox{up}\setbox\pgfnodepartlowerbox=\hbox{down}";
    let it = items(preamble, r"\pgfmultipartnode{circle split}{center}{s}{\pgfusepath{stroke}}");
    assert_eq!(it.iter().filter(|i| matches!(i, Item::Text { .. })).count(), 2, "{it:?}");
}
