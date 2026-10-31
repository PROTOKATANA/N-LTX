//! Arrow tips on paths, as pgf draws them: where the line ends, what the tips are made of.

use tikra::canvas::{Item, Paint};
use tikra::scene::{Point, Segment};
use tikra::PT;

/// A path of the canvas: segments, filled, stroked.
type Drawn = (Vec<Segment>, bool, bool);

/// The paths of a figure and its warnings.
fn draw(body: &str) -> (Vec<Drawn>, Vec<String>) {
    let code = format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}");
    let (canvas, warnings) = tikra::engine::run_document(&code).unwrap();
    let paths = canvas
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Path { segments, fill, stroke } => Some((segments.clone(), fill.is_some(), stroke.is_some())),
            _ => None,
        })
        .collect();
    (paths, warnings)
}

fn points(segs: &[Segment]) -> Vec<Point> {
    segs.iter()
        .filter_map(|s| match s {
            Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => Some(*p),
            Segment::Close => None,
        })
        .collect()
}

/// Length of the main line, the first path.
fn line_length(body: &str) -> f64 {
    let (p, w) = draw(body);
    assert!(w.is_empty(), "{w:?}");
    let q = points(&p[0].0);
    q.last().unwrap().minus(q[0]).norm()
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn the_line_ends_inside_the_tip() {
    // Computer Modern Rightarrow (TikZ's `>`): the line stops half a line width before the point
    let lw = 0.4;
    assert!(near(line_length(r"\draw[->] (0,0) -- (1,0);"), 1.0 - lw * PT));
    // the old `to`: at the tip's back
    assert!(near(line_length(r"\draw[-to] (0,0) -- (1,0);"), 1.0 - (0.21 + 0.625 * lw) * PT));
    // both ends
    assert!(near(line_length(r"\draw[<->] (0,0) -- (1,0);"), 1.0 - 2.0 * lw * PT));
    // a Latex tip: the line stops at its back, half a line width short of the point by the miter
    let l = 3.0 + 4.5 * lw - lw / 2.0;
    assert!(near(line_length(r"\draw[-Latex] (0,0) -- (1,0);"), 1.0 - l * PT));
    // shorten adds to it, and works without tips
    assert!(near(line_length(r"\draw[shorten >=1mm] (0,0) -- (1,0);"), 0.9));
    assert!(near(line_length(r"\draw[-Latex, shorten >=1mm] (0,0) -- (1,0);"), 0.9 - l * PT));
    // `>=` changes `>`
    assert!(near(line_length(r"\draw[>=Latex, ->] (0,0) -- (1,0);"), 1.0 - l * PT));
}

#[test]
fn only_the_last_subpath_has_tips() {
    // pgf splits off the last subpath: both tips are on it, the first one is left alone
    let (p, _) = draw(r"\draw[<->] (0,0) -- (1,0) (2,0) -- (3,0);");
    let q = points(&p[0].0);
    assert!(near(q[1].minus(q[0]).norm(), 1.0) && near(q[3].minus(q[2]).norm(), 1.0 - 0.8 * PT));
    // a lone point at the end still gets its tips, pointing up
    let (p, _) = draw(r"\draw[->] (0,0) -- (1,0) (2,0);");
    assert_eq!(p.len(), 2);
}

#[test]
fn tips_follow_the_path() {
    // the tip of `-Stealth` (lw 0) has its point at the end, pointing up
    let (p, _) = draw(r"\draw[line width=0pt, -Stealth] (0,0) -- (0,1);");
    assert_eq!(p.len(), 2);
    let base = points(&p[0].0)[0];
    let tip = points(&p[1].0);
    let top = tip.iter().map(|q| q.y - base.y).fold(f64::MIN, f64::max);
    assert!(near(top, 1.0), "{top}");
    assert!(tip.iter().all(|q| (q.x - base.x).abs() <= 1.5 * 0.75 * 3.0 * PT));
    // a curve: aimed from its last control point
    let (p, _) = draw(r"\draw[-Stealth] (0,0) .. controls (1,0) and (1,1) .. (1,2);");
    let tip = points(&p[1].0);
    let base = points(&p[0].0)[0];
    assert!(tip.iter().all(|q| (q.x - base.x - 1.0).abs() < 0.1));
}

#[test]
fn what_tips_are_made_of() {
    // Latex: filled and stroked; open: stroked only; Straight Barb: stroked
    let (p, _) = draw(r"\draw[-Latex] (0,0) -- (1,0); \draw[-{Latex[open]}] (0,1) -- (1,1); \draw[-Straight Barb] (0,2) -- (1,2);");
    assert_eq!((p[1].1, p[1].2), (true, true));
    assert_eq!((p[3].1, p[3].2), (false, true));
    assert_eq!((p[5].1, p[5].2), (false, true));
    // several tips, and `.`
    let (p, w) = draw(r"\draw[->>] (0,0) -- (1,0); \draw[<<.<->|] (0,1) -- (1,1);");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(p.len(), 3 + 6);
    // the tip's color
    let code = r"\begin{tikzpicture}\draw[-{Stealth[red]}] (0,0) -- (1,0);\end{tikzpicture}";
    let (c, _) = tikra::engine::run_document(code).unwrap();
    let Item::Path { fill: Some(f), .. } = &c.items[1] else { panic!() };
    assert!(matches!(f.paint, Paint::Solid(c) if c.r > 0.9 && c.g < 0.1));
}

#[test]
fn where_there_are_no_tips() {
    // closed paths, fills, and tips=never
    for body in [r"\draw[->] (0,0) -- (1,0) -- (1,1) -- cycle;", r"\fill[->] (0,0) -- (1,0) -- (1,1);", r"\draw[->, tips=never] (0,0) -- (1,0);"] {
        let (p, w) = draw(body);
        assert_eq!(p.len(), 1, "{body} {w:?}");
    }
    // tips=true: tips on a path that is not drawn
    let (p, _) = draw(r"\path[->, tips] (0,0) -- (1,0);");
    assert_eq!(p.len(), 1);
}

#[test]
fn tips_defined_in_the_document() {
    let (p, w) = draw(r"\tikzset{foo/.tip={Stealth[sep] Latex[]}} \draw[-foo] (0,0) -- (1,0);");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(p.len(), 3);
    let (_, w) = draw(r"\draw[-{Latex[nonsense]}] (0,0) -- (1,0);");
    assert!(w.iter().any(|w| w.contains("nonsense")), "{w:?}");
}
