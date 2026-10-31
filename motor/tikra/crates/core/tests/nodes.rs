//! Nodes: shapes, anchors, positioning, extra labels, text and size.

mod common;
use common::*;
use tikra::scene::{Label, Point, Segment, Transform};

/// The border a node draws (the first path of the body).
fn border(body: &str) -> Vec<Segment> {
    first_path(body).segments
}

/// Width and height of the border a node draws.
fn border_size(body: &str) -> (f64, f64) {
    let (a, b) = bounds(&border(body));
    (b.x - a.x, b.y - a.y)
}

/// The first label of a body and where it is placed.
fn label(body: &str) -> (Label, Transform) {
    texts(&canvas(body)).into_iter().next().unwrap_or_else(|| panic!("no text in {body}"))
}

fn curves(segs: &[Segment]) -> usize {
    segs.iter().filter(|s| matches!(s, Segment::CurveTo(..))).count()
}

fn lines(segs: &[Segment]) -> usize {
    segs.iter().filter(|s| matches!(s, Segment::LineTo(..))).count()
}

/// The shape a border path has: "rectangle" (straight, its corners off the axes), "diamond"
/// (straight, its corners on the axes through the center), "circle" or "ellipse" (curves).
fn shape_of(segs: &[Segment]) -> &'static str {
    let (a, b) = bounds(segs);
    let c = a.lerp(b, 0.5);
    if curves(segs) > 0 {
        return if close(b.x - a.x, b.y - a.y) { "circle" } else { "ellipse" };
    }
    let on_axis = points(segs).iter().all(|&(x, y)| (x - c.x).abs() < 1e-6 || (y - c.y).abs() < 1e-6);
    if on_axis {
        "diamond"
    } else {
        "rectangle"
    }
}

#[test]
fn shapes_by_option_and_by_shape_key() {
    for (op, shape) in [
        ("circle", "circle"),
        ("ellipse", "ellipse"),
        ("rectangle", "rectangle"),
        ("diamond", "diamond"),
        ("shape=circle", "circle"),
        ("shape=ellipse", "ellipse"),
        ("shape=diamond", "diamond"),
    ] {
        // a wide text, so that an ellipse is not a circle
        assert_eq!(shape_of(&border(&format!(r"\node[{op}, draw] {{ABCD}};"))), shape, "{op}");
    }
    // a shape with no definition still draws the node
    assert!(!paths(&canvas(r"\node[shape=star, draw] {A};")).is_empty());
    // a coordinate draws nothing
    let c = canvas(r"\node[shape=coordinate] (p) at (1,1) {};  \node[coordinate] (q) at (2,2) {};");
    assert!(paths(&c).is_empty() && texts(&c).is_empty());
}

#[test]
fn minimum_sizes_separation_and_aspect() {
    let (w, h) = border_size(r"\node[draw, minimum width=3cm, minimum height=2cm] {};");
    assert!(close(w, 3.0) && close(h, 2.0), "{w} {h}");
    let (w, _) = border_size(r"\node[draw, circle, minimum size=2cm] {};");
    assert!(close(w, 2.0), "{w}");
    let (w, _) = border_size(r"\node[draw, inner sep=0pt] {};");
    // an empty node with no inner sep: TikZ's minimum width of 1pt
    assert!(close(w, tikra::PT), "{w}");
    let (w, _) = border_size(r"\node[draw, inner sep=0pt, minimum size=0pt] {};");
    assert!(close(w, 0.0), "{w}");
    let (w, h) = border_size(r"\node[draw, inner xsep=1cm, inner ysep=2cm] {};");
    assert!(close(w, 2.0) && close(h, 4.0), "{w} {h}");
    let (narrow, _) = border_size(r"\node[diamond, draw, aspect=3] {AB};");
    let (wide, _) = border_size(r"\node[diamond, draw] {AB};");
    assert!(narrow > wide, "{narrow} {wide}");
    let (w, _) = border_size(r"\node[ellipse, draw, minimum width=4cm, minimum height=1cm] {x};");
    assert!(close(w, 4.0), "{w}");
    // bad values: the node is still drawn
    let (c, _) = canvas_and_warnings(r"\node[draw, inner sep=zz, inner xsep=zz, inner ysep=zz, minimum width=zz, minimum height=zz, minimum size=zz, aspect=zz] {x};");
    assert!(!texts(&c).is_empty());
}

#[test]
fn named_and_angle_anchors() {
    // outer sep=0pt: the anchors on the border itself (TikZ puts them half a line width outside)
    let n = r"\node[draw, outer sep=0pt, minimum width=2cm, minimum height=2cm] (n) at (0,0) {};";
    for (a, p) in [
        ("north", (0.0, 1.0)),
        ("south", (0.0, -1.0)),
        ("east", (1.0, 0.0)),
        ("west", (-1.0, 0.0)),
        ("north east", (1.0, 1.0)),
        ("south west", (-1.0, -1.0)),
        ("center", (0.0, 0.0)),
        ("base", (0.0, 0.0)),
        // mid: half an ex (2.15pt) above the baseline, as in TikZ
        ("mid", (0.0, 2.15 * tikra::PT)),
        ("text", (0.0, 0.0)),
        ("0", (1.0, 0.0)),
    ] {
        let q = point(n, &format!("n.{a}"));
        assert!(close_to((q.x, q.y), p), "{a}: {q:?}");
    }
    let q = point(n, "n");
    assert!(close_to((q.x, q.y), (0.0, 0.0)), "the node's name alone is its center: {q:?}");
    // the default outer sep: half the line width outside the border
    let q = point(r"\node[draw, minimum width=2cm, minimum height=2cm] (n) at (0,0) {};", "n.north");
    assert!(close(q.y, 1.0 + 0.2 * tikra::PT), "{q:?}");
    // an anchor that does not exist
    let (_, w) = canvas_and_warnings(&format!(r"{n} \draw (n.nothing) -- (1,1);"));
    assert!(!w.is_empty());
    let c = r"\node[circle, draw, outer sep=0pt, minimum size=2cm] (c) at (0,0) {};";
    let q = point(c, "c.north east");
    assert!(close(q.x.hypot(q.y), 1.0), "the circle's corner is on its border: {q:?}");
    // a coordinate's anchors are its point
    let q = point(r"\coordinate (p) at (1,2);", "p.east");
    assert!(close_to((q.x, q.y), (1.0, 2.0)), "{q:?}");
    let l = r"\node[diamond, draw, outer sep=0pt, minimum size=2cm] (l) at (0,0) {};";
    assert!(point(l, "l.45").x > 0.0);
    // a line toward a node stops at its border, also in a purely vertical or horizontal direction
    let p = paths(&canvas(&format!(r"{n} \draw (0,5) -- (n);")));
    let q = points(&p.last().unwrap().segments);
    assert!(close(q[1].1, 1.0), "{q:?}");
    let p = paths(&canvas(&format!(r"{n} \draw (5,0) -- (n);")));
    let q = points(&p.last().unwrap().segments);
    assert!(close(q[1].0, 1.0), "{q:?}");
}

#[test]
fn anchor_and_relative_placement() {
    let base = r"\node[draw, minimum size=1cm, inner sep=0pt, outer sep=0pt] (n) at (0,0) {};";
    let center = |options: &str| point(&base.replace("draw,", &format!("draw, {options},")), "n.center");
    assert!(close(center("anchor=west").x, 0.5), "{:?}", center("anchor=west"));
    for (pos, sign) in [
        ("above", (0.0, 1.0)),
        ("below", (0.0, -1.0)),
        ("left", (-1.0, 0.0)),
        ("right", (1.0, 0.0)),
        ("above left", (-1.0, 1.0)),
        ("above right", (1.0, 1.0)),
        ("below left", (-1.0, -1.0)),
        ("below right", (1.0, -1.0)),
    ] {
        let c = center(pos);
        assert!(c.x * sign.0 >= 0.0 && c.y * sign.1 >= 0.0, "{pos}: {c:?}");
        let c = center(&format!("{pos}=1cm"));
        assert!(c.x.abs() + c.y.abs() > 0.9, "{pos}=1cm: {c:?}");
    }
    assert!(close(center("xshift=1cm, yshift=-1cm").x, 1.0));
}

#[test]
fn positioning_of_with_distances() {
    let body = |op: &str| {
        format!(
            r"\node[draw, minimum size=1cm, inner sep=0pt, outer sep=0pt] (a) at (0,0) {{}}; \coordinate (p) at (5,5); \node[draw, minimum size=1cm, inner sep=0pt, outer sep=0pt, {op}] (b) {{}};"
        )
    };
    let b = |op: &str| point(&body(op), "b.center");
    assert!(close(b("right=of a").x, 2.0), "1cm default distance between borders");
    assert!(close(b("below=2cm of a").y, -3.0));
    let q = b("below left=1cm and 2cm of a");
    assert!(close(q.x, -3.0) && close(q.y, -2.0), "{q:?}");
    assert!(close(b("above=of a.north").y, 2.0));
    assert!(b("above right=of p").x > 5.0, "a coordinate as the target");
    for op in ["left=of a", "above left=of a", "above right=of a", "below right=of a", "above=1cm and 2cm of a"] {
        b(op);
    }
    let c = canvas(r"\begin{tikzpicture}[node distance=2cm and 3cm] \end{tikzpicture}");
    assert!(paths(&c).is_empty() && texts(&c).is_empty());
    let (_, w) = canvas_and_warnings(r"\node[right=of nobody] {x};");
    assert!(w.iter().any(|a| a.contains("nobody")), "{w:?}");
}

#[test]
fn figure_node_distance() {
    let r = no_warnings(
        r"\begin{tikzpicture}[node distance=2cm and 3cm]
            \node[draw] (a) {A}; \node[draw, below=of a] (b) {B}; \node[draw, right=of a] (c) {C};
          \end{tikzpicture}",
    );
    assert!(r.svg.contains("<path"));
    // an invalid node distance: a warning
    let r = figure(r"\begin{tikzpicture}[node distance=zz] \node (a) {A}; \node[below=of a] {B}; \end{tikzpicture}");
    assert!(!r.warnings.is_empty());
}

#[test]
fn position_along_the_path() {
    for (op, x) in
        [("midway", 1.0), ("near start", 0.5), ("near end", 1.5), ("at start", 0.0), ("at end", 2.0), ("very near start", 0.25), ("very near end", 1.75), ("pos=0.1", 0.2)]
    {
        let q = point(&format!(r"\draw (0,0) -- (2,0) node[{op}, inner sep=0pt] (n) {{}};"), "n.center");
        assert!(close(q.x, x), "{op}: {q:?}");
    }
    // with no previous segment, it stays at the point
    let q = point(r"\path (3,3) node[midway] (n) {x};", "n.center");
    assert!(close(q.x, 3.0), "{q:?}");
}

#[test]
fn extra_labels_by_side_and_by_angle() {
    for l in [
        "label=above:A",
        "label=below:B",
        "label=left:C",
        "label=right:D",
        "label=north:E",
        "label=south:F",
        "label=west:G",
        "label=east:H",
        "label=45:I",
        "label=-90:J",
        "label=90:K",
        "label=180:L",
        "label=0:M",
        "label=270:N",
        "label=north east:O",
        "label={[red]above:P}",
        "label=Q",
    ] {
        let c = canvas(&format!(r"\node[draw, {l}] at (0,0) {{x}};"));
        assert_eq!(texts(&c).len(), 2, "{l}");
    }
    // the label goes on its side: above is higher than the node's text, left further left
    let t = texts(&canvas(r"\node[label=above:A] at (0,0) {x};"));
    assert!(t[1].1.f > t[0].1.f, "{:?} {:?}", t[0].1, t[1].1);
    let t = texts(&canvas(r"\node[label=left:A] at (0,0) {x};"));
    assert!(t[1].1.e < t[0].1.e, "{:?} {:?}", t[0].1, t[1].1);
    let t = texts(&canvas(r"\node[label={[red]above:P}] at (0,0) {x};"));
    assert_eq!(t[1].0.color.hex(), "#ff0000");
}

#[test]
fn text_color_font_width_alignment() {
    let (r, _) = label(r"\node[text=blue, font=\bfseries\large] {Title};");
    assert_eq!(r.color.hex(), "#0000ff");
    let wrapped = r"\node[draw, text width=1cm, align=left] {a sentence with several words};";
    assert!(label(wrapped).0.lines.len() > 1);
    // in another family too (each word in it, not the line as one formula)
    assert!(label(r"\node[text width=1cm, font=\sffamily] {a sentence with several words};").0.lines.len() > 1);
    // the text width and the inner sep (.3333em) on both sides
    let (w, _) = border_size(wrapped);
    assert!((w - (1.0 + 2.0 * 0.3333 * 10.0 * tikra::PT)).abs() < 1e-4, "{w}");
    for a in ["right", "flush right", "flush left", "center", "justify"] {
        label(&format!(r"\node[text width=2cm, align={a}] {{word another word}};"));
    }
    label(r"\node[text centered] {center};");
    // a bare color and color=
    assert_eq!(label(r"\node[red] {reddish};").0.color.hex(), "#ff0000");
    assert_eq!(label(r"\node[color=green] {greenish};").0.color.hex(), "#00ff00");
    // a bad text width: an error in TikZ; the statement is left out, with a warning
    let (c, w) = canvas_and_warnings(r"\node[text width=zz] {no wrapping};");
    assert!(texts(&c).is_empty() && w.iter().any(|a| a.contains("zz")), "{w:?}");
}

#[test]
fn node_rotation_and_scale() {
    let (rotated, place) = label(r"\node[rotate=30, scale=2] {rotated};");
    let (plain, plain_place) = label(r"\node {rotated};");
    assert!(close(place.b.atan2(place.a).to_degrees(), 30.0), "{place:?}");
    // the text grows with the node, not only the box around it
    let grown = rotated.width() * place.factor() / (plain.width() * plain_place.factor());
    assert!(close(grown, 2.0), "{grown}");
    let (w, _) = border_size(r"\node[draw, scale=2] {rotated};");
    let (v, _) = border_size(r"\node[draw] {rotated};");
    assert!(close(w, 2.0 * v), "{w} {v}");
    canvas_and_warnings(r"\node[rotate=zz, scale=zz, pos=zz] {bad};");
    let r = no_warnings(r"\begin{tikzpicture}\node[draw, rotate=45, dashed] {x};\end{tikzpicture}");
    // the text turns with the node
    assert!(r.svg.contains("matrix(0.70711 -0.70711 0.70711 0.70711"), "{}", r.svg);
}

#[test]
fn node_syntax_errors() {
    for bad in [r"\node[draw {x};", r"\node (a {x};", r"\node at {x};", r"\node {x;"] {
        // an error or a warning, never a silent drawing
        if let Ok((_, w)) = tikra::engine::run_document(&format!("\\begin{{tikzpicture}}{bad}\\end{{tikzpicture}}")) {
            assert!(!w.is_empty(), "{bad}");
        }
    }
}

#[test]
fn draw_none_and_unnamed_coordinate() {
    let c = canvas(r"\node[draw=none] {x};");
    assert!(paths(&c).iter().all(|p| p.stroke.is_none()));
    let p = first_path(r"\node[draw=red] {x};");
    assert_eq!(p.stroke.unwrap().color.hex(), "#ff0000");
    // an unnamed coordinate draws nothing, and is no error
    let (c, w) = canvas_and_warnings(r"\path (1,1) coordinate;");
    assert!(w.is_empty(), "{w:?}");
    assert!(paths(&c).is_empty() && texts(&c).is_empty());
}

#[test]
fn text_with_sizes_circled_and_grouped_words() {
    let c = canvas(r"\node {\small small \Large large};");
    assert!(!texts(&c).is_empty());
    let r = no_warnings(r"\begin{tikzpicture} \node {$\textcircled{a}$}; \end{tikzpicture}");
    assert!(r.svg.contains("<ellipse"));
    // with text width, $…$ and {…} are not split
    let (l, _) = label(r"\node[text width=1cm] {$a + b$ {c d} e};");
    assert!(l.lines.len() >= 2);
}

#[test]
fn rows_of_math_in_text_are_not_lines_of_the_node() {
    // an array's `\\` is its own: one line of text, and the `\\` after it a line of the node
    let (l, _) = label(r"\node[align=center] {x $\begin{array}{c} a \\ b \end{array}$ y \\ z};");
    assert_eq!(l.lines.len(), 2);
    // and so are those of a display and of an alignment (mathra sets them as a whole)
    for text in [r"\[ a \\ b \]", r"\begin{align*} a &= b \\ c &= d \end{align*}"] {
        assert_eq!(label(&format!(r"\node[align=center] {{{text}}};")).0.lines.len(), 1, "{text}");
    }
    // a color inside math is a color of math, not a formula of its own
    no_warnings(r"\begin{tikzpicture} \node {$\begin{matrix} a \\ \textcolor{red}{b} \end{matrix}$}; \end{tikzpicture}");
}

#[test]
fn newline_breaks_a_paragraphs_line() {
    // in a text width, as `\\`; out of one TeX sets the words together
    let (l, _) = label(r"\node[text width=3cm] {one\newline two};");
    assert_eq!(l.lines.len(), 2);
    let (l, _) = label(r"\node[align=center] {one\newline two};");
    assert_eq!(l.lines.len(), 1);
}

#[test]
fn a_paragraph_box_in_text_wraps_to_its_width() {
    let text = "several words that do not fit in two centimeters";
    for body in [format!(r"\parbox[t]{{2cm}}{{\centering {text}}}"), format!(r"\begin{{minipage}}{{2cm}}{text}\end{{minipage}}"), format!(r"\parbox{{2cm}}{{\raggedleft {text}}}")]
    {
        let node = format!(r"\node[draw] {{{body}}};");
        assert!(label(&node).0.lines.len() >= 3, "{body}");
        // as wide as the box and the inner sep (.3333em) on both sides
        let (w, _) = border_size(&node);
        assert!((w - (2.0 + 2.0 * 0.3333 * 10.0 * tikra::PT)).abs() < 1e-4, "{body}: {w}");
    }
    // a width tikra does not know is left as it is
    let (_, w) = canvas_and_warnings(r"\node {\parbox{zz}{a b}};");
    assert!(w.iter().any(|a| a.contains("parbox")), "{w:?}");
}

#[test]
fn picture_environment_in_text() {
    // the manual of CircuiTikZ's \fourdots: a cross and four dots, 4 units of 0.8mm square
    let doc = r"\newcommand{\fourdots}{\begingroup\setlength{\unitlength}{0.8mm}%
        \begin{picture}(4,4)(-2,-2)
        \put(-2,0){\line(1,0){4}} \put(0,-2){\line(0,1){4}}
        \multiput(-1,1)(2,0){2}{\circle*{1}} \multiput(-1,-1)(2,0){2}{\circle*{1}}
        \end{picture}\endgroup}
        \begin{tikzpicture} \node[inner sep=0pt] {a\fourdots b}; \end{tikzpicture}";
    let r = no_warnings(doc);
    assert_eq!(r.svg.matches("<ellipse").count(), 4, "{}", r.svg);
    let (l, _) = label(r"\node {\setlength{\unitlength}{1cm}\begin{picture}(2,1)\put(0,0){\line(1,0){2}}\end{picture}};");
    assert!((l.width() - 2.0).abs() < 1e-3 && (l.height() - 1.0).abs() < 1e-3 && l.depth().abs() < 1e-3, "{} {} {}", l.width(), l.height(), l.depth());
}

#[test]
fn images_in_text() {
    // a PNG of 40 × 30 px at 72 dpi (40.15 × 30.11pt), in a folder tikra reads files from
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="40pt" height="30pt" viewBox="0 0 40 30"><rect width="40" height="30" fill="blue"/></svg>"#;
    let dir = std::env::temp_dir().join(format!("tikra-node-images-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("pic.png"), tikra::png(svg, 72.0, false).unwrap()).unwrap();
    std::env::set_var("TIKRA_INPUT_DIR", dir.display().to_string());
    let images = |body: &str| {
        let (c, w) = tikra::engine::run_document(&format!(r"\begin{{tikzpicture}}{body}\end{{tikzpicture}}")).unwrap();
        assert!(w.is_empty(), "{w:?}");
        c.items.iter().filter_map(|i| if let tikra::canvas::Item::Image { place, .. } = i { Some(*place) } else { None }).collect::<Vec<_>>()
    };
    // \pgfdeclareimage's width, the height kept in proportion; the box the image's size
    let body = r"\pgfdeclareimage[width=2cm]{p}{pic} \node[inner sep=0pt, outer sep=0pt] (a) {\pgfuseimage{p}};";
    let t = images(body);
    assert!(t.len() == 1 && close(t[0].a, 2.0) && close(t[0].d, 1.5), "{t:?}");
    assert!(close(point(body, "a.east").x, 1.0) && close(point(body, "a.north").y, 0.75));
    // \includegraphics and \pgfimage, at the natural size without options
    let t = images(r"\node {\includegraphics[height=3cm]{pic.png}}; \node at (5,0) {\pgfimage{pic}};");
    assert!(close(t[0].a, 4.0) && close(t[0].d, 3.0), "{t:?}");
    assert!(close(t[1].a, 40.0 * 72.27 / 72.0 * 2.54 / 72.27), "{t:?}");
    // in \pgftext, and a file not found reported
    assert_eq!(images(r"\pgfdeclareimage{p}{pic}\pgftext{\pgfuseimage{p}}").len(), 1);
    let (_, w) = tikra::engine::run_document(r"\begin{tikzpicture}\pgfdeclareimage{q}{none}\node {\pgfuseimage{q}};\end{tikzpicture}").unwrap();
    assert!(w.iter().any(|x| x.contains("not found")), "{w:?}");
}

#[test]
fn baseline_anchors() {
    let body =
        r"\node[draw, anchor=base west] (a) at (1,2) {xg}; \node[anchor=base] (b) at (5,2) {x}; \node[anchor=base east] (c) at (9,2) {x}; \node[anchor=text] (d) at (12,2) {x};";
    for n in ["a", "b", "c", "d"] {
        let q = point(body, &format!("{n}.base"));
        assert!(close(q.y, 2.0), "the baseline sits on the given point: {n} {q:?}");
    }
    assert!(close(point(body, "a.base west").x, 1.0));
    assert!(close(point(body, "b.base").x, 5.0));
    assert!(close(point(body, "c.base east").x, 9.0));
    assert!(close(point(body, "d.text").x, 12.0));
    // the text's baseline starts at the node's text anchor
    let t = texts(&canvas(body));
    let d: Point = point(body, "d.text");
    assert!(close(t[3].1.e, d.x) && close(t[3].1.f, d.y), "{:?}", t[3].1);
    // the "g" goes below the baseline: the center sits above it by less than the half height
    assert!(point(body, "a.center").y > 2.0);
}

#[test]
fn tex_quotes_in_node_text_are_typographic() {
    // mathra turns `` '' into “ ” inside text; the label must not warn or fall back to boxes
    let r = no_warnings(r"\begin{tikzpicture} \node {say ``hi'' and `bye'}; \end{tikzpicture}");
    assert!(r.svg.contains("<path"));
}

#[test]
fn shapes_have_their_border_paths() {
    // a rectangle: four sides; a diamond: four sides too; a circle: four quarter curves
    let r = border(r"\node[draw] {x};");
    assert_eq!((lines(&r), curves(&r)), (3, 0), "{r:?}");
    let d = border(r"\node[draw, diamond] {x};");
    assert_eq!(curves(&d), 0, "{d:?}");
    let c = border(r"\node[draw, circle] {x};");
    assert_eq!(curves(&c), 4, "{c:?}");
}

#[test]
fn a_line_between_nodes_of_a_name_prefix_stops_at_their_borders() {
    let c = canvas(r"\begin{scope}[name prefix=top-] \node (A) at (0,0) {A}; \node (B) at (1,0) {B}; \draw (A) -- (B); \end{scope}");
    let line = paths(&c).pop().unwrap();
    let xs: Vec<f64> = points(&line.segments).into_iter().map(|(x, _)| x).collect();
    // from A's east border to B's west border, not through B to its far side
    assert!(xs[0] > 0.1 && xs[1] < 0.9, "{xs:?}");
}

#[test]
fn a_rectangle_callout_finds_its_angle_anchors_as_pgf_does() {
    // pgf looks for the border toward the direction plus the center (seen from the text's origin):
    // on a wide callout, 60° lands on the east side, not near the top's middle
    let body = r"\node[rectangle callout] (s) {A rather long callout for its height};";
    let (a, east) = (point(body, "s.60"), point(body, "s.east"));
    assert!((a.x - east.x).abs() < 1e-6 && a.y > east.y, "{a:?} {east:?}");
}

#[test]
fn a_snapshot_animates_the_picture_with_its_text() {
    // `myself: = {…}` with several attributes; the rotation turns the canvas, text included
    let c = canvas(
        r#"\begin{scope}[make snapshot of=1s, animate = { myself: = { :rotate = { 0s = "0", 2s = "90" }, :line width = { 0s = "0mm", 4s = "4mm" } }}] \node [draw] {Node}; \end{scope}"#,
    );
    let turned = c.items.iter().any(|i| matches!(i, tikra::canvas::Item::Transform { transform: t } if (t.b.atan2(t.a).to_degrees() - 45.0).abs() < 1e-6));
    assert!(turned && !texts(&c).is_empty());
    let width = paths(&c).iter().find_map(|d| d.stroke.as_ref().map(|s| s.width)).unwrap();
    assert!((width - 0.1).abs() < 1e-6, "{width}");
}

#[test]
fn small_capitals_are_as_wide_as_cmcsc10_sets_them() {
    // a word in \scshape is wider than in the label font (cmcsc10 is an extended design): in 2cm,
    // "Complexity" no longer fits and is hyphenated, as in TeX
    let (l, _) = label(r"\node[text width=2cm, align=center, font=\scshape, execute at begin node=\hskip0pt] {Measuring Complexity};");
    assert_eq!(l.lines.len(), 3, "{}", l.lines.len());
}

#[test]
fn lines_too_tall_for_the_baselines_are_a_lineskip_apart() {
    // a molecule over a word: TeX puts \lineskip (1pt) between the boxes, it does not overlap them
    let (l, _) = label(r"\node[align=center] {$\chemfig{*6(------)}$\\caffeine};");
    let gap = l.baseline(0) - l.baseline(1) - (l.lines[0].depth + l.lines[1].height) as f64 * l.scale;
    assert!((gap - 2.54 / 72.27).abs() < 1e-6, "{gap}");
}

#[test]
fn text_in_times_is_as_wide_as_tex_sets_it() {
    // widths TeX's ptm gives (\settowidth, in pt), against the node's text box: inner and outer
    // sep 0, so its west and east anchors are the text's ends
    let pt = 2.54 / 72.27;
    let width = |font: &str, text: &str| {
        let body = format!(r"\node[inner sep=0, outer sep=0, font=\fontfamily{{ptm}}{font}] (n) {{{text}}};");
        (point(&body, "n.east").x - point(&body, "n.west").x) / pt
    };
    let sentence = "The quick brown fox jumps over the lazy dog ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    for (font, tex) in [("", 358.01787), (r"\bfseries", 378.44699), (r"\itshape", 344.95752), (r"\bfseries\itshape", 360.567)] {
        let w = width(&format!(r"\fontsize{{10}}{{12}}{font}"), sentence);
        assert!((w - tex).abs() < 0.05, "{font}: {w} against TeX's {tex}");
    }
    // bold upright, an italic letter (bold italic, as \emph in \bfseries) and bold again
    let w = width(r"\fontencoding{T1}\fontsize{45}{45}\bfseries", r"Ti\emph{k}Z");
    assert!((w - 94.22916).abs() < 0.05, "{w}");
    // drawn in Termes: its glyphs are defined apart from the label font's
    let svg = no_warnings(r"\tikz \node[font=\fontfamily{ptm}] {Times};").svg;
    assert!(svg.contains(r#"id="tr-gt"#) && !svg.contains(r#"id="tr-g1"#), "{svg}");
}

#[test]
fn bold_text_stays_bold_after_a_formula() {
    let svg = |text: &str| no_warnings(&format!(r"\tikz \node[font=\bfseries] {{{text}}};")).svg;
    let glyphs = |s: &str| s.match_indices("<use href=\"#").map(|(i, _)| s[i + 12..].split('"').next().unwrap().to_string()).collect::<Vec<_>>();
    let g = glyphs(&svg("Z$x$Z"));
    assert_eq!(g[0], g[2], "{g:?}");
    // a formula left open is drawn as mathra takes it, after the bold text
    let out = figure(r"\tikz \node[font=\bfseries] {Z $x};");
    assert!(out.svg.contains("<use"), "{}", out.svg);
}

#[test]
fn times_moves_what_else_a_label_holds_with_its_letters() {
    // a rule, a cancel line and a circled letter in Times: they keep their place among the
    // letters, and a math symbol Termes lacks stays in STIX Two
    let out = no_warnings(r"\tikz \node[font=\fontfamily{ptm}\bfseries] {AV. $\underline{W} \textcircled{c} \cancel{x} \alpha$};");
    assert!(out.svg.contains("<rect") && out.svg.contains("<line") && out.svg.contains("<ellipse"), "{}", out.svg);
    // Termes' glyphs (from 0xC000 on) and STIX Two's (the alpha) side by side
    let ids = |svg: &str| -> Vec<u32> { svg.match_indices(r#"id="tr-gt"#).filter_map(|(i, _)| svg[i + 9..].split('"').next()?.parse().ok()).collect() };
    let both = ids(&out.svg);
    assert!(both.iter().any(|&g| g >= 0xC000) && both.iter().any(|&g| g < 0xC000), "{both:?}");
    // a label of nothing Termes has is mathra's as it is
    let alone = ids(&no_warnings(r"\tikz \node[font=\fontfamily{ptm}] {$\alpha$};").svg);
    assert!(!alone.is_empty() && alone.iter().all(|&g| g < 0xC000), "{alone:?}");
}

#[test]
fn turned_and_scaled_boxes_in_a_label() {
    // graphicx's boxes (mathra 1.7): drawn through their map, in Latin Modern and in Times, and in
    // a line of a size of its own
    for font in ["", r"\fontfamily{ptm}"] {
        let out = no_warnings(&format!(r"\tikz \node[font={font}, align=center] {{\rotatebox{{90}}{{AB}} x\\ \Large \scalebox{{2}}[1]{{Q}} y}};"));
        assert_eq!(out.svg.matches(r#"<g transform="matrix("#).count(), 2, "{font}: {}", out.svg);
    }
    // turned a quarter: the map's first column is (0, 1), (0, -1) in SVG's y down
    let svg = no_warnings(r"\tikz \node {\rotatebox{90}{AB}};").svg;
    assert!(svg.contains(r#"<g transform="matrix(0 -1 1 0 "#), "{svg}");
}

#[test]
fn named_points_in_the_svg() {
    // a coordinate, a node and one named twice (the last place wins), in the SVG's own units
    let out = no_warnings(r"\begin{tikzpicture}[scale=2]\coordinate (a) at (1,2); \node (b) at (-1,0) {b}; \coordinate (a) at (3,1); \fill (a) circle (1pt);\end{tikzpicture}");
    let names: Vec<&str> = out.points.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["a", "b"]);
    let (a, b) = (&out.points[0], &out.points[1]);
    // the circle drawn at (a) starts at its center (a move to it, then the circle around)
    let d = out.svg.split(r#"<path d="M"#).last().unwrap();
    let xy: Vec<f64> = d.split([' ', 'M']).filter_map(|t| t.parse().ok()).take(2).collect();
    assert!((xy[0] - a.x).abs() < 0.01 && (xy[1] - a.y).abs() < 0.01, "{a:?} {xy:?}");
    // from (a) to (b): 8 cm left and 2 cm down, y down in SVG
    let cm = 72.0 / 2.54;
    assert!((b.x - a.x + 8.0 * cm).abs() < 1e-3 && (b.y - a.y - 2.0 * cm).abs() < 1e-3, "{a:?} {b:?}");
    // the origin: (a) is (6 cm, 2 cm) from it
    assert!((a.x - out.origin.0 - 6.0 * cm).abs() < 1e-3 && (a.y - out.origin.1 + 2.0 * cm).abs() < 1e-3, "{a:?} {:?}", out.origin);
    // a figure with no names has none
    assert!(no_warnings(r"\tikz \draw (0,0) -- (1,1);").points.is_empty());
}

#[test]
fn a_matrix_cell_reads_named_points_in_its_own_coordinates() {
    // TeX sets each cell at the origin and moves it afterwards: (A), from outside, is A in the cell;
    // (B), from the first cell, is where it was in that cell. Values from TeX Live 2025, in cm.
    let body = r"\coordinate (A) at (1,0);
        \matrix[column sep=1cm] at (5,5) {
          \draw (A) -- (0,0) node (p) {}; \coordinate (B) at (0,0); &
          \draw (A) -- (B) node (q) {}; \coordinate (C) at (A); \\
        };";
    let near = |p: Point, x: f64, y: f64| (p.x - x).abs() < 1e-3 && (p.y - y).abs() < 1e-3;
    for (name, x, y) in [("A", 1.0, 0.0), ("B", 3.49290, 5.0), ("p", 3.49290, 5.0), ("q", 5.61709, 5.0), ("C", 6.61709, 5.0)] {
        let q = point(body, name);
        assert!(near(q, x, y), "{name}: {q:?}");
    }
}

#[test]
fn text_is_kerned_as_the_documents_fonts_are() {
    let width = |code: &str| -> f64 {
        let svg = tikra::convert(code).unwrap().svg;
        svg.split("viewBox=\"").nth(1).unwrap().split(' ').nth(2).unwrap().parse().unwrap()
    };
    let node = r"\begin{tikzpicture}\node[draw, inner sep=0] {va};\end{tikzpicture}";
    let ot1 = width(node);
    let t1 = width(&format!("\\documentclass{{standalone}}\\usepackage[T1]{{fontenc}}\\usepackage{{tikz}}\\begin{{document}}{node}\\end{{document}}"));
    let spaced = width(r"\begin{tikzpicture}\node[draw, inner sep=0] {v a};\end{tikzpicture}");
    // cmr10 kerns `va` by 0.278pt, ec-lmr10 by 0.556pt
    assert!((ot1 - t1 - 0.278).abs() < 0.02, "{ot1} {t1}");
    assert!(spaced > ot1);
    // the next figure is OT1 again
    assert_eq!(width(node), ot1);
}

#[test]
fn a_placement_offset_without_unit_is_in_points_unless_positioning() {
    let width = |code: &str| -> f64 {
        let svg = tikra::convert(code).unwrap().svg;
        svg.split("viewBox=\"").nth(1).unwrap().split(' ').nth(2).unwrap().parse().unwrap()
    };
    let picture = r"\begin{tikzpicture}\draw (0,0) circle (1pt) node[left=30]{LO};\end{tikzpicture}";
    let points = width(picture);
    let doc = |library: &str| format!("\\documentclass{{standalone}}\\usepackage{{tikz}}{library}\\begin{{document}}{picture}\\end{{document}}");
    assert!((width(&doc("")) - points).abs() < 1e-6);
    // 30 pt across, or 30 cm with the positioning library, as TeX
    assert!(points < 60.0, "{points}");
    assert!(width(&doc(r"\usetikzlibrary{positioning}")) > 800.0);
}

#[test]
fn pin_edge_from_the_pin_options_and_their_styles_the_last_one_winning() {
    let edge = |body: &str| {
        let c = canvas(body);
        let s = paths(&c).into_iter().find_map(|d| d.stroke).unwrap_or_else(|| panic!("no pin edge in {body}"));
        (s.color.hex(), (s.width / tikra::PT * 10.0).round() / 10.0, s.dash.is_some())
    };
    // from a style of the pin, as `\tikzstyle{…}=[pin edge={…}]` is used
    assert_eq!(edge(r"\tikzstyle{p}=[pin edge={red,thick}] \node[pin={[p]above:a}] {x};"), ("#ff0000".into(), 0.8, false));
    // a second `pin edge` replaces the first; what is not given comes from `every pin edge`
    assert_eq!(edge(r"\node[pin={[pin edge={red},pin edge={dashed}]above:a}] {x};"), ("#808080".into(), 0.2, true));
    // `every pin` comes before the pin's own options
    assert_eq!(edge(r"\tikzset{every pin/.style={pin edge={red}}} \node[pin={[pin edge={thick}]above:a}] {x};"), ("#808080".into(), 0.8, false));
    // given to the node itself, it does nothing
    assert_eq!(edge(r"\node[pin edge={red},pin=above:a] {x};"), ("#808080".into(), 0.2, false));
}

#[test]
fn pin_and_label_distance_among_their_own_options() {
    let gap = |body: &str| {
        let t = texts(&canvas(body));
        (t[1].1.e - t[0].1.e).abs()
    };
    assert!(gap(r"\node[pin={[pin distance=1cm]0:a}] {x};") > 1.5 * gap(r"\node[pin={[pin distance=0.1cm]0:a}] {x};"));
    assert!(gap(r"\node[label={[label distance=1cm]0:a}] {x};") > 1.5 * gap(r"\node[label=0:a] {x};"));
}

#[test]
fn quoted_labels_with_a_direction_or_options() {
    let t = texts(&canvas(r#"\usetikzlibrary{quotes} \node["b" below left, "c" text=red] at (0,0) {x};"#));
    // TeX: b 12.09pt left of the node's text and 13.96pt lower, c red and 11.33pt higher
    let (x, b, c) = (t[0].1, t[1].1, t[2].1);
    assert!(((b.e - x.e) / tikra::PT + 12.09).abs() < 0.2 && ((b.f - x.f) / tikra::PT + 13.96).abs() < 0.2, "{x:?} {b:?}");
    assert!(((c.f - x.f) / tikra::PT - 11.33).abs() < 0.2, "{x:?} {c:?}");
    assert_eq!(t[2].0.color.hex(), "#ff0000");
}

#[test]
fn edge_from_parent_of_a_child_that_has_children() {
    // `edge from parent[draw=none]` before the child's own children still hides its edge
    let c = canvas(r"\node {a} child {node {b} edge from parent[draw=none] child {node {c}}} child {node {d} edge from parent[draw=none]};");
    let drawn: Vec<_> = paths(&c).into_iter().filter(|d| d.stroke.is_some()).collect();
    assert_eq!(drawn.len(), 1, "only b to c: {drawn:?}");
}

#[test]
fn right_of_takes_the_node_distance_given_after_it() {
    let at = |options: &str| {
        let p = point(&format!(r"\begin{{scope}}[node distance=2cm] \coordinate (a) at (0,0); \node[{options}] (n) {{}}; \end{{scope}}"), "n.center");
        ((p.x * 1e4).round() / 1e4, (p.y * 1e4).round() / 1e4)
    };
    // the positions TeX gives
    assert_eq!(at("right of=a"), (2.0, 0.0));
    assert_eq!(at("right of=a, node distance=3cm"), (3.0, 0.0));
    assert_eq!(at("above of=a, node distance=1cm, xshift=1cm"), (1.0, 1.0));
    assert_eq!(at("right of=a, xshift=1cm, node distance=0.5cm"), (1.5, 0.0));
    // two of them both shift by the last distance: from (b) at 2cm, 4 + 4
    assert_eq!(at(r"right of=a, node distance=4cm, right of=a"), (8.0, 0.0));
    // in its place among the other transformations
    assert_eq!(at("right of=a, rotate=90, xshift=1cm, node distance=3cm"), (3.0, 1.0));
}

#[test]
fn a_style_that_adds_to_the_text_font_ends() {
    // `\tikz@textfont` is a macro: a key that appends to it, then the font used (as a shape's code
    // does), does not refer to itself forever
    no_warnings(
        r"\makeatletter\tikzset{add font/.code={\expandafter\def\expandafter\tikz@textfont\expandafter{\tikz@textfont#1}}}\begin{tikzpicture}[font=\sffamily]\begingroup\tikzset{add font=\bfseries}\tikz@textfont\endgroup\node[draw] {A};\end{tikzpicture}",
    );
}

#[test]
fn at_a_relative_point_is_from_the_paths_last_point() {
    // alone, from the origin; on a path, from its last point, which `at` does not move
    let q = point(r"\node (n) at +(1,0) {};", "n.center");
    assert!(close_to((q.x, q.y), (1.0, 0.0)), "{q:?}");
    let body = r"\path (4,0) -- (5,5) node (n) at ++(1,0) {} -- +(1,0) coordinate (m);";
    for name in ["n.center", "m"] {
        let q = point(body, name);
        assert!(close_to((q.x, q.y), (6.0, 5.0)), "{name}: {q:?}");
    }
    let q = point(r"\begin{scope}[shift={(1,1)}] \path (2,0) node (n) at +(0,1) {}; \end{scope}", "n.center");
    assert!(close_to((q.x, q.y), (3.0, 2.0)), "{q:?}");
    // a `+` with no point after it
    with_warning(r"\node at +x {};", "at without a point");
}

#[test]
fn a_second_name_replaces_the_first() {
    // `node (a) (b)` is named b; after a coordinate's name, `(…)` is the path's next point
    let q = point(r"\path (2,0) node (a) (b) {A};", "b.center");
    assert!(close_to((q.x, q.y), (2.0, 0.0)), "{q:?}");
    let q = point(r"\path (3,0) node (c) at (1,1) [red] (d) {A};", "d.center");
    assert!(close_to((q.x, q.y), (1.0, 1.0)), "{q:?}");
    let (_, w) = canvas_and_warnings(r"\path (2,0) node (a) (b) {A}; \draw (a) -- (1,1);");
    assert!(w.iter().any(|w| w.contains("unknown coordinate: (a)")), "{w:?}");
    let q = point(r"\path (4,0) coordinate (f) (5,0) coordinate (g);", "g");
    assert!(close_to((q.x, q.y), (5.0, 0.0)), "{q:?}");
}

/// The text a body's nodes show, as the figure tells it.
fn told(body: &str) -> String {
    no_warnings(&format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}")).description
}

#[test]
fn a_text_runs_its_definitions_and_assignments_as_tex_typesets_it() {
    // definitions before a picture, whose `\foreach \number` is a variable and not TeX's \number
    let doc = r"\newcommand\inner[1]{\def\w{#1}\begin{tikzpicture}\foreach \number in {1,2} {\draw (\number,0) -- (\number,\w);}\end{tikzpicture}}
        \begin{tikzpicture} \node {\inner{2}}; \end{tikzpicture}";
    let (c, w) = tikra::engine::run_document(doc).unwrap();
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(c.items.iter().filter(|i| matches!(i, tikra::canvas::Item::Path { .. })).count(), 2, "{:?}", c.items);
    // registers given values, pgfmath's loop, \relax and what is \let to it typeset nothing
    assert!(told(r"\newcount\k \node {\k=3 \multiply\k by 2 \the\k};").contains(": 6."));
    assert!(told(r"\newcount\k \node {\k=0 \pgfmathloop\ifnum\pgfmathcounter<4 \advance\k by \pgfmathcounter\relax\repeatpgfmathloop\the\k};").contains(": 6."));
    assert!(told(r"\let\nothing\relax \node {a\nothing b};").contains(": ab."));
    assert!(told(r"\node {\def\a{x}\a};").contains(": x."));
    // a register that is not given a value is an argument, written as it is
    with_warning(r"\begin{tikzpicture}\newcount\k \node {A\k};\end{tikzpicture}", "\\k");
}

#[test]
fn pics_with_a_text_and_one_for_each_item_at_relative_points() {
    let doc = r"\tikzset{box/.pic={\node (-n) [draw] {\tikzpictext};}}
        \begin{tikzpicture} \path (1,0) pic foreach \i in {1,2} (p\i) [pic text=T\i] at +(\i,0) {box}; \end{tikzpicture}";
    let r = no_warnings(doc);
    assert!(r.description.contains("T1") && r.description.contains("T2"), "{}", r.description);
    let c = canvas(r"\tikzset{box/.pic={\node (-n) {};}} \path (1,0) pic foreach \i in {1,2} (p\i) at +(\i,0) {box}; \draw (p2-n.center) -- (0,0);");
    assert!(close_to(points(&paths(&c)[0].segments)[0], (3.0, 0.0)));
}

#[test]
fn below_of_takes_the_node_distance_the_node_ends_with() {
    let at = |options: &str| {
        let p = point(&format!(r"\node (a) {{A}}; \node[{options}] (b) {{B}};"), "b.center");
        (p.x, p.y)
    };
    // a node distance given after `below of` still counts
    assert!(close_to(at("below of=a, node distance=3cm"), (0.0, -3.0)), "{:?}", at("below of=a, node distance=3cm"));
    // the directions add up, all at the last distance
    assert!(close_to(at("below of=a, right of=a, node distance=2cm"), (2.0, -2.0)));
    assert!(close_to(at("below of=a, node distance=2cm, right of=a, node distance=3cm"), (3.0, -3.0)));
    assert!(close_to(at("below of=a, below of=a"), (0.0, -2.0)));
    // a diagonal one is that far along its angle; `v and h`: the vertical one for below
    let s = 0.5f64.sqrt();
    assert!(close_to(at("below right of=a"), (s, -s)));
    assert!(close_to(at("below of=a, node distance=2cm and 3cm"), (0.0, -2.0)));
}

#[test]
fn a_label_angle_may_come_from_a_macro() {
    // as the expression it expands to, not the default 90
    let (_, by_macro) = label(r"\def\a{100} \node[label=\a+80:x] at (0,0) {};");
    let (_, plain) = label(r"\node[label=180:x] at (0,0) {};");
    assert!(close(by_macro.e, plain.e) && close(by_macro.f, plain.f), "{by_macro:?} {plain:?}");
}

#[test]
fn a_word_too_long_for_the_text_width_goes_to_a_line_of_its_own() {
    // TeX's last pass breaks before it, leaving that line too long, even at the end (a word
    // with a slash is not hyphenated)
    let (l, _) = label(r"\node[text width=3em] {AB CDEFG/HIJKL};");
    assert_eq!(l.lines.len(), 2);
    let (l, _) = label(r"\node[text width=3em] {CDEFG/HIJKL AB};");
    assert_eq!(l.lines.len(), 2);
}

#[test]
fn pic_anchor_moves_a_pic_to_its_coordinate() {
    let pics = r"\tikzset{pics/bar/.style={code={\draw (0,0) coordinate (-a) -- (2,0) coordinate (-b);}}}";
    let at = |body: &str| bounds(&first_path(&format!("{pics}{body}")).segments);
    let (plain, _) = at(r"\path (1,0) pic {bar};");
    let (moved, _) = at(r"\path (1,0) pic[pic anchor=(-b)] {bar};");
    assert!((plain.x - moved.x - 2.0).abs() < 1e-6, "{plain:?} {moved:?}");
}
