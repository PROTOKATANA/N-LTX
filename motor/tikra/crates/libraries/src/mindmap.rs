//! The mindmap library (`tikzlibrarymindmap`): concepts are circles filled with the `concept
//! color`, grown as a tree with `grow cyclic`; `mindmap` sets the sizes, distances and fonts of
//! each level (`root concept`, `level 1 concept`…), `small`, `large` and `huge mindmap` other
//! ones; `extra concept`, `concept connection` and `annotation` are styles.
//!
//! A child's edge is a `circle connection bar`: a bar from the parent's circle to the child's with
//! a widening cap at each end, filled with the concept color and not stroked. When the child has a
//! `concept color` of its own, the bar is shaded from the parent's color to the child's between the
//! two circles (`circle connection bar switch color=from (a) to (b)`). A bar on a path that draws
//! or fills (`\draw[circle connection bar] (a) edge (b)`) is painted as that path says. The bar
//! is also a decoration (`decoration=circle connection bar`, with `start radius`, `end radius`,
//! `amplitude`, `angle`).

use tikra_engine::canvas::{Fill, Item, Paint, Shading};
use tikra_engine::color::Color;
use tikra_engine::describe::{Arrow, Link};
use tikra_engine::engine::mindmap::bar;
use tikra_engine::engine::nodes::NodeInfo;
use tikra_engine::engine::piece::{CommandFn, Piece};
use tikra_engine::engine::Engine;
use tikra_engine::scene::{Point, Segment, Transform};
use tikra_engine::tex::Token;
use tikra_engine::{Error, PT};

/// The mindmap library.
pub static PIECE: Piece = Piece { name: "mindmap", commands: COMMANDS, definitions: || DEFINITIONS.to_string(), ..Piece::EMPTY };

const COMMANDS: &[(&str, CommandFn)] = &[("tikra@circlebar", circle_bar)];

/// The library's styles. `\tikra@conceptcolor` is the concept color; `concept color` also makes
/// the edge to the parent switch from the parent's color to the new one.
const DEFINITIONS: &str = r"
\def\tikra@conceptcolor{black}\def\tikra@barfrom{}\def\tikra@barto{}\def\tikra@barconcept{}
\tikzset{
  concept/.style={circle,fill=\tikra@conceptcolor,draw=\tikra@conceptcolor,every concept},
  every concept/.style={},
  concept color/.code={\edef\tikra@temp{\noexpand\tikzset{edge from parent path={(\noexpand\tikzparentnode)
    to[tikra@concept bar,circle connection bar switch color=from (\tikra@conceptcolor) to (#1)] (\noexpand\tikzchildnode)}}}\tikra@temp
    \def\tikra@conceptcolor{#1}},
  tikz@concept@setting/.style={edge from parent path={(\tikzparentnode) to[tikra@concept bar,circle connection bar] (\tikzchildnode)}},
  tikra@concept bar/.code={\def\tikra@barconcept{1}},
  tikz@concept@color@set/.style={},
  extra concept/.style={concept color=black!50,level 2 concept,concept,every extra concept},
  every extra concept/.style={},
  concept connection/.style={line width=1mm,shorten <=2mm,shorten >=2mm,cap=round,draw=black!50},
  circle connection bar/.code={\tikzset{to path={\pgfextra{\tikra@circlebar} (\tikztotarget) \tikztonodes}}\def\tikra@barfrom{}},
  circle connection bar switch color/.code={\tikra@barswitchcolor#1\relax},
  every circle connection bar/.style={},
  mindmap/.style={fill,draw,very thick,outer sep=0pt,inner sep=1pt,
    every child/.append style={style=tikz@concept@setting,style=tikz@concept@color@set},
    root concept, level 1/.append style={level 1 concept}, level 2/.append style={level 2 concept},
    level 3/.append style={level 3 concept}, level 4/.append style={level 4 concept},
    text centered, segment angle=20, style=every mindmap},
  every mindmap/.style={},
  root concept/.style={minimum size=4cm,text width=3.5cm,font=\large},
  level 1 concept/.style={minimum size=2.25cm,level distance=5cm,text width=2cm,sibling angle=60,font=\small},
  level 2 concept/.style={minimum size=1.75cm,level distance=2.9cm,text width=1.5cm,sibling angle=60,font=\footnotesize},
  level 3 concept/.style={minimum size=1.15cm,text width=1cm,level distance=2.4cm,sibling angle=30,font=\tiny},
  level 4 concept/.style={minimum size=0.9cm,text width=0.7cm,level distance=1.85cm,sibling angle=30,font=\tiny},
  small mindmap/.style={
    root concept/.style={minimum size=2.3cm,text width=2.1cm,font=\footnotesize},
    level 1 concept/.style={minimum size=1.5cm,level distance=2.8cm,text width=1.4cm,sibling angle=75,font=\scriptsize},
    level 2 concept/.style={minimum size=1.1cm,level distance=2.2cm,text width=1.1cm,sibling angle=60,font=\tiny},
    level 3 concept/.style={level 2 concept,sibling angle=30,font=\tiny},
    level 4 concept/.style={level 3 concept},
    mindmap, line width=2pt},
  large mindmap/.style={
    root concept/.style={minimum size=5.6cm,text width=4.5cm,font=\Large},
    level 1 concept/.style={minimum size=3.2cm,level distance=7cm,text width=2.8cm,sibling angle=60,font=},
    level 2 concept/.style={minimum size=2.45cm,level distance=4cm,text width=2.2cm,sibling angle=60,font=\small},
    level 3 concept/.style={minimum size=1.63cm,text width=1.4cm,level distance=3.38cm,sibling angle=30,font=\scriptsize},
    level 4 concept/.style={minimum size=1.27cm,text width=1cm,level distance=2.60cm,sibling angle=30,font=\tiny},
    mindmap, line width=2pt},
  huge mindmap/.style={
    root concept/.style={minimum size=8cm,text width=7cm,font=\huge},
    level 1 concept/.style={minimum size=4.5cm,level distance=10cm,text width=4cm,sibling angle=60,font=\large},
    level 2 concept/.style={minimum size=3.5cm,level distance=5.8cm,text width=3cm,sibling angle=60,font=},
    level 3 concept/.style={minimum size=2.3cm,text width=2cm,level distance=4.8cm,sibling angle=30,font=\footnotesize},
    level 4 concept/.style={minimum size=1.7cm,text width=1.4cm,level distance=3.7cm,sibling angle=30,font=\scriptsize},
    mindmap, line width=3pt},
  annotation/.style={shape=rectangle,minimum size=0pt,text width=3.5cm,outer sep=1.5mm,inner sep=1mm,
    text badly ragged,rounded corners,font=\tiny,every annotation},
  every annotation/.style={},
}
\def\tikra@barswitchcolor from (#1) to (#2)\relax{\tikzset{circle connection bar}\def\tikra@barfrom{#1}\def\tikra@barto{#2}}
";

fn macro_text(e: &mut Engine, name: &str) -> Result<String, Error> {
    e.tex.expand_to_string(vec![Token::cs(name)]).map(|s| s.trim().to_string())
}

fn macro_color(e: &mut Engine, name: &str) -> Result<Option<Color>, Error> {
    let s = e.tex.expand_to_string(vec![Token::cs(name)])?;
    let s = s.trim();
    if s.is_empty() {
        return Ok(None);
    }
    e.color(s).map(Some).ok_or_else(|| Error::Figure(format!("unknown color '{s}'")))
}

/// From the center to the west anchor, on the canvas: how pgf measures a circle's radius.
fn west_radius(n: &NodeInfo) -> f64 {
    n.center().x - n.anchor("west").unwrap_or(n.center()).x
}

/// The bar placed from `start_node` to `end_node`, or nothing when the two centers coincide.
struct PlacedBar {
    segments: Vec<Segment>,
    /// The start circle's border, in pt.
    start: Point,
    /// Unit vector from the start center to the end center.
    ux: f64,
    uy: f64,
    /// Distance between the centers, in pt.
    dist: f64,
    /// The end circle's radius, in pt.
    r2: f64,
    /// The start circle's center, in cm.
    ca: Point,
}

fn place_bar(start_node: &NodeInfo, end_node: &NodeInfo, angle: f64) -> Option<PlacedBar> {
    let (ca, cb) = (start_node.center(), end_node.center());
    let (r1, r2) = (west_radius(start_node) / PT, west_radius(end_node) / PT);
    let amplitude = 0.175 * r1.min(r2);
    let d = cb.minus(ca);
    let dist = (d.x * d.x + d.y * d.y).sqrt() / PT;
    if dist < 1e-9 {
        return None;
    }
    let (ux, uy) = (d.x / PT / dist, d.y / PT / dist);
    // the frame of the decoration: x along the line, from the start circle's border, in pt
    let start = Point::new(ca.x / PT + r1 * ux, ca.y / PT + r1 * uy);
    let frame = Transform { a: ux * PT, b: uy * PT, c: -uy * PT, d: ux * PT, e: start.x * PT, f: start.y * PT };
    let segments = bar(r1, r2, amplitude, angle, dist - r1 - r2, &frame);
    Some(PlacedBar { segments, start, ux, uy, dist, r2, ca })
}

fn switched_paint(e: &mut Engine, placed: &PlacedBar) -> Result<Paint, Error> {
    match (macro_color(e, "tikra@barfrom")?, macro_color(e, "tikra@barto")?) {
        (Some(from_color), Some(to_color)) => {
            // from the start circle's border to the end circle's, along y of the shading's box
            let s0 = Point::new(placed.start.x * PT, placed.start.y * PT);
            let end = Point::new((placed.ca.x / PT + (placed.dist - placed.r2) * placed.ux) * PT, (placed.ca.y / PT + (placed.dist - placed.r2) * placed.uy) * PT);
            let (vx, vy) = ((end.x - s0.x) / 100.0, (end.y - s0.y) / 100.0);
            let mid = s0.lerp(end, 0.5);
            let transform = Transform { a: vy, b: -vx, c: vx, d: vy, e: mid.x, f: mid.y };
            Ok(Paint::Shading(Shading::Axial { transform, stops: vec![(0.0, from_color), (1.0, to_color)] }))
        }
        _ => Ok(Paint::Solid(macro_color(e, "tikra@conceptcolor")?.unwrap_or(Color::BLACK))),
    }
}

fn paint_bar(e: &mut Engine, segments: Vec<Segment>, paint: Paint) -> Result<(), Error> {
    if e.counts_for_size() {
        e.canvas.include_path(&segments);
    }
    // a child's edge, or a bar on a path that neither fills nor draws, is filled with the
    // concept colors; on a path that does, the bar is filled and drawn as that path is
    // (`\draw[circle connection bar] (a) edge (b)` outlines it)
    let tree = e.tex.expand_to_string(vec![Token::cs("tikra@barconcept")])?.trim() == "1";
    let (fill, stroke) = if tree || !(e.st.mode.fill || e.st.mode.draw) {
        (Some(Fill { paint, opacity: e.st.paint.fill_opacity, even_odd: false }), None)
    } else {
        let fill = e.st.mode.fill.then(|| Fill { paint: Paint::Solid(e.st.fill_paint_color()), opacity: e.st.paint.fill_opacity, even_odd: false });
        (fill, e.st.mode.draw.then(|| e.stroke()))
    };
    e.canvas.push(Item::Path { segments, fill, stroke });
    Ok(())
}

fn note_link(e: &mut Engine, from: String, to: String) {
    if e.facts.mute == 0 {
        e.facts.links.push(Link { from, to, arrow: Arrow::None, labels: Vec::new() });
    }
}

/// `\tikra@circlebar`, in a `to path`: the bar from `\tikztostart` to `\tikztotarget` (two
/// circles), filled with the concept color, or shaded when the colors switch.
fn circle_bar(e: &mut Engine, _: &Token) -> Result<(), Error> {
    let (from, to) = (macro_text(e, "tikztostart")?, macro_text(e, "tikztotarget")?);
    let (Some(a), Some(b)) = (e.nodes.get(&from).cloned(), e.nodes.get(&to).cloned()) else {
        return Err(Error::Figure("a circle connection bar connects two nodes".into()));
    };
    if a.shape == "coordinate" || b.shape == "coordinate" {
        return Err(Error::Figure("a circle connection bar cannot connect a node of shape 'coordinate'".into()));
    }
    note_link(e, from, to);
    let Some(placed) = place_bar(&a, &b, e.st.decoration.angle) else { return Ok(()) };
    let paint = switched_paint(e, &placed)?;
    paint_bar(e, placed.segments, paint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tikra_engine::canvas::Canvas;
    use tikra_engine::engine::piece::Parts;

    fn run(code: &str) -> Result<(Canvas, Vec<String>), Error> {
        let parts = Parts { pieces: &[&PIECE], libraries: &[] };
        tikra_engine::engine::run_document_in(code, parts).map(|(canvas, warnings, _)| (canvas, warnings))
    }

    #[test]
    fn a_mindmap_grows() {
        let (canvas, warnings) = run(
            r"\tikz[mindmap,concept color=red!50] \node[concept] {Root} child[grow=right] {node[concept] {Child}} child[grow=left,concept color=blue] {node[concept] {Other}};",
        )
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert!(canvas.items.iter().any(|i| matches!(i, Item::Path { fill: Some(Fill { paint: Paint::Shading(_), .. }), .. })));
    }

    #[test]
    fn a_bar_of_ones_own_is_painted_as_its_path() {
        let bars = |path: &str| {
            let (canvas, warnings) = run(&format!(
                r"\tikz[mindmap,concept color=red] {{\node[concept] (a) at (0,0) {{}}; \node[concept] (b) at (6,0) {{}}; {path}[circle connection bar] (a) edge (b);}}"
            ))
            .unwrap();
            assert!(warnings.is_empty(), "{warnings:?}");
            // the two concepts, then the bar
            canvas.items.into_iter().filter(|i| matches!(i, Item::Path { .. })).skip(2).collect::<Vec<_>>()
        };
        // `\draw` outlines it, unfilled; `\fill` fills it in its own color
        assert!(matches!(&bars(r"\draw")[..], [Item::Path { fill: None, stroke: Some(_), .. }]), "{:?}", bars(r"\draw"));
        let blue = Color::rgb(0.0, 0.0, 1.0);
        assert!(matches!(&bars(r"\fill[blue]")[..], [Item::Path { fill: Some(Fill { paint: Paint::Solid(c), .. }), .. }] if *c == blue), "{:?}", bars(r"\fill[blue]"));
        // on a path that neither fills nor draws, it is filled with the concept color
        let (canvas, _) = run(r"\tikz {\node[circle,draw] (a) at (0,0) {}; \node[circle,draw] (b) at (6,0) {}; \path (a) to[circle connection bar] (b);}").unwrap();
        assert!(matches!(canvas.items.last(), Some(Item::Path { fill: Some(_), stroke: None, .. })), "{:?}", canvas.items.last());
        // a child's edge is still filled with the concept color
        let (canvas, _) = run(r"\tikz[mindmap,concept color=red] \node[concept] {Root} child {node[concept] {Child}};").unwrap();
        let red = Color::rgb(1.0, 0.0, 0.0);
        assert!(canvas.items.iter().any(|i| matches!(i, Item::Path { fill: Some(Fill { paint: Paint::Solid(c), .. }), stroke: None, segments } if *c == red && segments.iter().filter(|s| matches!(s, Segment::Close)).count() == 3)));
    }
}
