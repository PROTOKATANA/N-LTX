//! Path operations: lines, elbows, curves, relatives, shapes, arcs, grids, plots,
//! decorations and post-processing (corners, shortening, clipping).

mod common;
use common::*;
use tikra::canvas::Item;
use tikra::scene::{Point, Segment};

#[test]
fn line_elbows_and_closing() {
    let t = first_path(r"\draw (0,0) -- (2,0) -| (3,2) |- (0,3) -- cycle;");
    let p = points(&t.segments);
    assert!(close_to(p[2], (3.0, 0.0)), "the -| corner in {p:?}");
    assert!(close_to(p[4], (3.0, 3.0)), "the |- corner in {p:?}");
    assert!(matches!(t.segments.last(), Some(Segment::Close)));
    // cycle with no operation before it does not close
    let t = first_path(r"\draw (0,0) -- (1,0) cycle;");
    assert!(!t.segments.contains(&Segment::Close));
}

#[test]
fn relative_coordinates() {
    // ++ accumulates, + does not move the base
    let t = first_path(r"\draw (1,1) -- ++(1,0) -- ++(0,1) -- +(1,1) -- +(2,0);");
    let p = points(&t.segments);
    assert!(close_to(p[2], (2.0, 2.0)), "{p:?}");
    assert!(close_to(p[3], (3.0, 3.0)), "{p:?}");
    assert!(close_to(p[4], (4.0, 2.0)), "{p:?}");
}

#[test]
fn current_point_is_local() {
    // after the group, ++ counts from where it began, and the line goes on from where it ended
    let t = first_path(r"\draw (2,0) -- ++(1,0) { [current point is local] -- ++(0,1) } -- ++(-1,0);");
    assert_eq!(t.segments.iter().filter(|s| matches!(s, Segment::MoveTo(_))).count(), 1, "{:?}", t.segments);
    let p = points(&t.segments);
    assert!(close_to(p[2], (3.0, 1.0)) && close_to(p[3], (2.0, 0.0)), "{p:?}");
    // also to a point given absolutely: one line from the group's last point
    let t = first_path(r"\draw (0,-1) -- (1,-1) { [current point is local] (0,1) -- (1,1) } -- (2,2);");
    assert!(matches!(t.segments.last(), Some(Segment::LineTo(_))), "{:?}", t.segments);
    let p = points(&t.segments);
    assert!(close_to(p[p.len() - 2], (1.0, 1.0)) && close_to(p[p.len() - 1], (2.0, 2.0)), "{p:?}");
}

#[test]
fn polar_coordinates_with_and_without_unit() {
    let t = first_path(r"\draw (0,0) -- (90:2) -- (0:1cm);");
    let p = points(&t.segments);
    assert!(close_to(p[1], (0.0, 2.0)), "{p:?}");
    assert!(close_to(p[2], (1.0, 0.0)), "{p:?}");
    // without a unit it follows the scope's unit vector; with a unit, it does not
    let c = canvas(r"\begin{scope}[x=2cm] \draw (0,0) -- (0:1) -- (0:1cm); \end{scope}");
    let p = points(&paths(&c)[0].segments);
    assert!(close_to(p[1], (2.0, 0.0)) && close_to(p[2], (1.0, 0.0)), "{p:?}");
    // the angle may be a braced expression, as in a \foreach over the vertices of a polygon
    let t = first_path(r"\def\k{1} \draw ({90*\k}:2) -- ({45+45}:{1+1});");
    let p = points(&t.segments);
    assert!(close_to(p[0], (0.0, 2.0)) && close_to(p[1], (0.0, 2.0)), "{p:?}");
}

#[test]
fn explicit_length_in_coordinates() {
    let c = canvas(r"\begin{scope}[x=2cm, y=3cm] \draw (1cm, 1cm) -- (1,1); \end{scope}");
    let p = points(&paths(&c)[0].segments);
    assert!(close_to(p[0], (1.0, 1.0)) && close_to(p[1], (2.0, 3.0)), "{p:?}");
}

#[test]
fn names_anchors_and_angles() {
    let c = canvas(
        r"\node[draw, minimum size=2cm] (q) at (0,0) {};
          \coordinate (o) at (5,5);
          \draw (q.north) -- (q.30) -- (o);",
    );
    // the node's border is the first path; its anchors lie outer sep (half the line width) outside it
    let p = points(&paths(&c)[1].segments);
    let outer = 0.2 * tikra::PT;
    assert!(close_to(p[0], (0.0, 1.0 + outer)), "{p:?}");
    assert!(close(p[1].0, 1.0 + outer), "the square's border at 30°: {p:?}");
    assert!(close_to(p[2], (5.0, 5.0)));
}

#[test]
fn unknown_coordinate_and_unknown_anchor_become_warnings() {
    let (_, w) = canvas_and_warnings(r"\draw (0,0) -- (ghost);");
    assert!(w.iter().any(|a| a.contains("unknown coordinate")), "{w:?}");
    let (_, w) = canvas_and_warnings(r"\node (a) {x}; \draw (a.side) -- (1,1);");
    assert!(w.iter().any(|a| a.contains("unknown anchor")), "{w:?}");
}

#[test]
fn calc_midpoint_sum_factors_and_rotation() {
    let c = canvas(
        r"\coordinate (A) at (0,0); \coordinate (B) at (4,0);
          \draw ($(A)!0.5!(B)$) -- ($(A)+(1,2)$) -- ($0.5*(B) - (1,0)$) -- ($(A)!1cm!(B)$) -- ($(A)!0.5!90:(B)$) -- ($(A)!0.5! (B)$);",
    );
    let p = points(&paths(&c)[0].segments);
    assert!(close_to(p[0], (2.0, 0.0)), "{p:?}");
    assert!(close_to(p[1], (1.0, 2.0)), "{p:?}");
    assert!(close_to(p[2], (1.0, 0.0)), "{p:?}");
    assert!(close_to(p[3], (1.0, 0.0)), "{p:?}");
    assert!(close_to(p[4], (0.0, 2.0)), "{p:?}");
}

#[test]
fn calc_distance_is_measured_before_the_transformation() {
    let at = |options: &str, b: &str| {
        let body = format!(r"\begin{{scope}}[{options}] \coordinate (A) at (0,0); \coordinate (B) at {b}; \coordinate (C) at ($(A)!1cm!(B)$); \end{{scope}}");
        let q = point(&body, "C");
        (q.x, q.y)
    };
    assert!(close_to(at("scale=3", "(2,0)"), (3.0, 0.0)));
    assert!(close_to(at("xscale=3", "(0,2)"), (0.0, 1.0)));
    let s = 0.5f64.sqrt();
    assert!(close_to(at("xscale=3", "(2,2)"), (3.0 * s, s)));
    assert!(close_to(at("shift={(1,1)}, rotate=90", "(2,0)"), (1.0, 2.0)));
}

#[test]
fn let_gives_points_in_the_scopes_coordinates() {
    // a point read in a shifted scope is what it was given there, so `(\x1,0)` comes back below it
    let body = r"\coordinate (A) at (1,0);
        \begin{scope}[xshift=6cm] \coordinate (B) at (1,2); \path let \p1 = (B), \p2 = (A) in (\x1,0) coordinate (C) (\x2,\y2) coordinate (D); \end{scope}";
    for (name, x) in [("C", 7.0), ("D", 1.0)] {
        let q = point(body, name);
        assert!(close_to((q.x, q.y), (x, 0.0)), "{name}: {q:?}");
    }
    let q = point(r"\begin{scope}[scale=2] \coordinate (B) at (1,1); \path let \p1 = (B) in (\x1,0) coordinate (C); \end{scope}", "C");
    assert!(close_to((q.x, q.y), (2.0, 0.0)), "{q:?}");
    // a name in braces is expanded
    let q = point(r"\def\nm{a} \path let \p{\nm} = (1,2) in (\p{a}) coordinate (C);", "C");
    assert!(close_to((q.x, q.y), (1.0, 2.0)), "{q:?}");
}

#[test]
fn a_unit_vector_from_a_node_is_in_the_scopes_coordinates() {
    // x=(m) in a shifted and scaled scope is the vector to m from the scope's origin
    let body = r"\coordinate (o) at (1,0);
        \begin{scope}[shift={(3,0)}, scale=2] \coordinate (m) at (1,0);
          \begin{scope}[x={(m)}, y={(0,1)}] \coordinate (a) at (1,0); \end{scope}
          \begin{scope}[x={(o)}, y={(0,1)}] \coordinate (b) at (1,0); \end{scope}
          \begin{scope}[x={(1,0)}, y={(0,1)}] \coordinate (c) at (1,0); \end{scope}
        \end{scope}";
    for (name, x) in [("a", 5.0), ("b", 1.0), ("c", 5.0)] {
        let q = point(body, name);
        assert!(close_to((q.x, q.y), (x, 0.0)), "{name}: {q:?}");
    }
}

#[test]
fn calc_errors_become_warnings() {
    for bad in [r"($(A)!0.5$)", r"($(A)!0.5!$)", r"($(A)!0.5!(B$)", r"($(A)!xx!(B)$)", r"($(A$)", r"($(Z)!0.5!(A)$)"] {
        let (_, w) = canvas_and_warnings(&format!(r"\coordinate (A) at (0,0); \coordinate (B) at (1,0); \draw {bad} -- (1,1);"));
        assert!(!w.is_empty(), "{bad}");
    }
}

#[test]
fn bezier_curves_with_one_and_two_controls() {
    let t = first_path(r"\draw (0,0) .. controls (1,1) and (2,1) .. (3,0) .. controls (4,-1) .. (5,0);");
    assert_eq!(t.segments.iter().filter(|s| matches!(s, Segment::CurveTo(..))).count(), 2);
    // .. without controls: ignored without hanging
    first_path(r"\draw (0,0) -- (1,0) .. (2,2);");
    // a curve that starts the path
    let t = first_path(r"\draw .. controls (1,1) and (2,1) .. (3,0);");
    assert!(matches!(t.segments[0], Segment::MoveTo(_)));
}

#[test]
fn curve_errors() {
    for bad in [r"(0,0) .. controls", r"(0,0) .. controls (1,1) and", r"(0,0) .. controls (1,1) and (2,2) .."] {
        let (_, w) = canvas_and_warnings(&format!(r"\draw {bad};"));
        assert!(!w.is_empty(), "{bad}");
    }
}

#[test]
fn to_with_angles_bends_and_looseness() {
    for op in ["to[out=90, in=180]", "to[bend left]", "to[bend right=45]", "to[bend left=20, looseness=2]", "edge[bend right]"] {
        let t = first_path(&format!(r"\draw (0,0) {op} (2,0);"));
        assert!(t.segments.iter().any(|s| matches!(s, Segment::CurveTo(..))), "{op}");
    }
    // to without options is a straight line
    let t = first_path(r"\draw (0,0) to (2,0);");
    assert!(matches!(t.segments[1], Segment::LineTo(_)));
    // to a node stops at its border (curve)
    let c = canvas(r"\node[draw, circle, minimum size=1cm] (b) at (3,0) {}; \draw (0,0) to[bend left] (b);");
    assert!(points(&paths(&c)[1].segments)[1].0 < 3.0);
}

#[test]
fn rectangle_circle_ellipse() {
    let t = first_path(r"\draw (0,0) rectangle (2,1);");
    assert_eq!(points(&t.segments).len(), 5, "the start point and the four corners");
    for shape in ["circle (1)", "circle [radius=1]", "circle [x radius=2, y radius=1]", "ellipse (2 and 1)", "circle[radius=5mm] ", "ellipse [x radius=1cm, y radius=5mm]"] {
        let t = first_path(&format!(r"\draw (1,1) {shape};"));
        assert_eq!(t.segments.iter().filter(|s| matches!(s, Segment::CurveTo(..))).count(), 4, "{shape}");
    }
}

#[test]
fn shape_errors() {
    for bad in [
        r"(0,0) rectangle",
        r"(0,0) circle (1",
        r"(0,0) circle [radius=zz]",
        r"(0,0) circle [x radius=zz]",
        r"(0,0) circle [y radius=zz]",
        r"(0,0) ellipse (zz and 1)",
        r"(0,0) ellipse (1 and zz)",
        r"(0,0) circle (zz)",
    ] {
        let (_, w) = canvas_and_warnings(&format!(r"\draw {bad};"));
        assert!(!w.is_empty(), "{bad}");
    }
}

#[test]
fn arcs_by_parentheses_and_by_options() {
    let t = first_path(r"\draw (1,0) arc (0:90:1);");
    let p = points(&t.segments);
    assert!(close_to(*p.last().unwrap(), (0.0, 1.0)), "{p:?}");
    let t = first_path(r"\draw (2,0) arc (0:180:2 and 1);");
    assert!(close_to(*points(&t.segments).last().unwrap(), (-2.0, 0.0)));
    let t = first_path(r"\draw (1,0) arc [start angle=0, end angle=270, radius=1];");
    assert_eq!(t.segments.iter().filter(|s| matches!(s, Segment::CurveTo(..))).count(), 3);
    let t = first_path(r"\draw (1,0) arc [start angle=0, delta angle=-90, x radius=1, y radius=2];");
    assert!(close_to(*points(&t.segments).last().unwrap(), (0.0, -2.0)));
    // an arc that starts the path, an arc with no parameters and with incomplete parameters
    first_path(r"\draw arc (0:90:1);");
    first_path(r"\draw (0,0) arc;");
    first_path(r"\draw (0,0) arc (0:90);");
    // unknown options on the arc
    first_path(r"\draw (1,0) arc [radius=1, nothing=2];");
}

#[test]
fn arc_errors() {
    for bad in [
        r"(0,0) arc (0:90:1",
        r"(0,0) arc (zz:90:1)",
        r"(0,0) arc (0:zz:1)",
        r"(0,0) arc (0:90:zz)",
        r"(0,0) arc (0:90:zz and 1)",
        r"(0,0) arc (0:90:1 and zz)",
        r"(0,0) arc [start angle=zz]",
        r"(0,0) arc [end angle=zz]",
        r"(0,0) arc [delta angle=zz]",
        r"(0,0) arc [radius=zz]",
        r"(0,0) arc [x radius=zz]",
        r"(0,0) arc [y radius=zz]",
    ] {
        let (_, w) = canvas_and_warnings(&format!(r"\draw {bad};"));
        assert!(!w.is_empty(), "{bad}");
    }
}

#[test]
fn grid_with_steps() {
    let t = first_path(r"\draw (0,0) grid (2,2);");
    assert_eq!(t.segments.len(), 13, "the start point, 3 vertical and 3 horizontal lines");
    let t = first_path(r"\draw[step=0.5] (0,0) grid (1,1);");
    assert_eq!(t.segments.len(), 13, "the path's step applies to the grid");
    // a bad step is an error in TeX: the statement is dropped, with a warning
    let (c, w) = canvas_and_warnings(r"\draw[xstep=2, ystep=zz] (0,0) grid (2,2);");
    assert!(paths(&c).is_empty() && !w.is_empty(), "{w:?}");
    let t = first_path(r"\draw (0,0) grid [step=0.5] (1,1);");
    assert_eq!(t.segments.len(), 13);
    let t = first_path(r"\draw (0,0) grid [xstep=1, ystep=0.5, other=1] (2,1);");
    assert_eq!(t.segments.len(), 13, "the start point, 3 vertical and 3 horizontal lines");
    for bad in [r"(0,0) grid", r"(0,0) grid [step=zz] (1,1)", r"(0,0) grid [xstep=zz] (1,1)", r"(0,0) grid [ystep=zz] (1,1)"] {
        let (_, w) = canvas_and_warnings(&format!(r"\draw {bad};"));
        assert!(!w.is_empty(), "{bad}");
    }
}

#[test]
fn plot_of_expression_coordinates_and_smooth() {
    let t = first_path(r"\draw[domain=0:1, samples=11] plot (\x, {\x*\x});");
    assert_eq!(points(&t.segments).len(), 11);
    let t = first_path(r"\draw plot[smooth] coordinates {(0,0) (1,1) (2,0) (3,1)};");
    assert_eq!(t.segments.iter().filter(|s| matches!(s, Segment::CurveTo(..))).count(), 3);
    let t = first_path(r"\draw plot[domain=0:2, samples=3, variable=\t] (\t, 1);");
    assert_eq!(points(&t.segments).len(), 3);
    // a plot that continues a path
    let t = first_path(r"\draw (0,5) -- plot coordinates {(1,1) (2,2)};");
    assert!(matches!(t.segments[1], Segment::LineTo(_)));
    // the variable does not leak and a macro with the same name comes back afterwards
    let c = canvas(r"\def\x{7} \draw[domain=0:1, samples=2, variable=\x] plot (\x, 0); \draw (\x, 0) -- (8,0);");
    assert!(close(points(&paths(&c)[1].segments)[0].0, 7.0));
    // a saved numeric variable value comes back
    let c = canvas(r"\foreach \x in {3} { \draw[domain=0:1, samples=2] plot (\x, 0); \draw (\x,0) -- (9,9); }");
    assert!(close(points(&paths(&c)[1].segments)[0].0, 3.0));
    // non-finite points are skipped
    let t = first_path(r"\draw[domain=-1:1, samples=3] plot (\x, {1/\x});");
    assert_eq!(points(&t.segments).len(), 2);
}

#[test]
fn plot_samples_at_a_list() {
    let xs = |body: &str| points(&first_path(body).segments).iter().map(|p| p.0).collect::<Vec<_>>();
    // the values of a `\foreach` list, `...` included, in their order
    assert_eq!(xs(r"\draw plot[samples at={0,1,3}] (\x, {\x*\x});"), [0.0, 1.0, 3.0]);
    assert_eq!(xs(r"\draw plot[samples at={0,0.5,...,2}] (\x, 0);"), [0.0, 0.5, 1.0, 1.5, 2.0]);
    assert_eq!(xs(r"\draw[variable=\t, samples at={2,1}] plot (\t, 0);"), [2.0, 1.0]);
    // `samples` or `domain` after it take over again, as in TeX; before it, they do not
    assert_eq!(xs(r"\draw plot[samples at={0,1,3}, samples=3] (\x, 0);"), [-5.0, 0.0, 5.0]);
    assert_eq!(xs(r"\draw plot[samples at={0,1,3}, domain=0:4] (\x, 0);").len(), 25);
    assert_eq!(xs(r"\draw plot[samples=4, samples at={0,1,3}] (\x, 0);"), [0.0, 1.0, 3.0]);
    let (_, w) = canvas_and_warnings(r"\draw plot[samples at={0,zz}] (\x, 0);");
    assert!(!w.is_empty());
}

#[test]
fn plot_errors() {
    let (_, w) = canvas_and_warnings(r"\draw plot (\x, {\doesnotexist});");
    assert!(w.iter().any(|a| a.contains("plot:")), "{w:?}");
    for bad in [r"plot coordinates", r"plot", r"plot[domain=zz:1] (\x,0)", r"plot[domain=0:zz] (\x,0)", r"plot[samples=zz] (\x,0)", r"plot coordinates {(zz,0)}"] {
        let (_, w) = canvas_and_warnings(&format!(r"\draw {bad};"));
        assert!(!w.is_empty(), "{bad}");
    }
    // a domain without ':' and unknown options: ignored
    first_path(r"\draw[domain=1] plot[domain=2, mark=*] (\x, 0);");
}

#[test]
fn decorations() {
    for d in ["zigzag", "snake", "random steps", "coil"] {
        let t = first_path(&format!(r"\draw[decorate, decoration={{{d}, amplitude=1mm, segment length=2mm}}] (0,0) -- (2,0) arc (0:90:1) -- cycle;"));
        assert!(points(&t.segments).len() > 5, "{d}");
    }
    // decorate without a decoration leaves the path as it is (pgf has no default decoration);
    // decoration without decorate does not decorate
    assert_eq!(points(&first_path(r"\draw[decorate] (0,0) -- (2,0);").segments).len(), 2);
    assert_eq!(points(&first_path(r"\draw[decoration=zigzag] (0,0) -- (2,0);").segments).len(), 2);
    // a zero-length segment stays as it is
    first_path(r"\draw[decorate] (0,0) -- (0,0) -- (1,0);");
    // bad values are an error in TeX: the statement is dropped, with a warning
    let (c, w) = canvas_and_warnings(r"\draw[decorate, decoration={zigzag, amplitude=zz, segment length=zz, x=1}] (0,0) -- (1,0);");
    assert!(paths(&c).is_empty() && !w.is_empty(), "{w:?}");
}

#[test]
fn rounded_corners() {
    let t = first_path(r"\draw[rounded corners] (0,0) -- (2,0) -- (2,2);");
    assert!(t.segments.iter().any(|s| matches!(s, Segment::CurveTo(..))));
    let t = first_path(r"\draw[rounded corners=2mm] (0,0) -- (2,0) -- (2,2) -- cycle;");
    assert!(matches!(t.segments.last(), Some(Segment::Close)));
    // closed, repeating the first point
    first_path(r"\draw[rounded corners] (0,0) -- (2,0) -- (2,2) -- (0,0) -- cycle;");
    // few points: no corner to round
    let t = first_path(r"\draw[rounded corners] (0,0) -- (2,0);");
    assert_eq!(t.segments.len(), 2);
    let t = first_path(r"\draw[rounded corners] (0,0) -- (2,0) -- cycle;");
    assert!(matches!(t.segments.last(), Some(Segment::Close)));
    // with a curve on the path: the corner where the arc starts is rounded too, its inside is not
    let t = first_path(r"\draw[rounded corners] (0,0) -- (1,0) arc (0:90:1);");
    assert!(t.segments.iter().any(|s| matches!(s, Segment::CurveTo(..))));
    // sharp corners turns it off
    let t = first_path(r"\draw[rounded corners, sharp corners] (0,0) -- (2,0) -- (2,2);");
    assert!(!t.segments.iter().any(|s| matches!(s, Segment::CurveTo(..))));
    // a bad radius is an error in TeX: the statement is dropped, with a warning
    let (c, w) = canvas_and_warnings(r"\draw[rounded corners=zz] (0,0) rectangle (2,1);");
    assert!(paths(&c).is_empty() && !w.is_empty(), "{w:?}");
}

#[test]
fn shorten_ends() {
    let t = first_path(r"\draw[shorten >=1cm, shorten <=5mm] (0,0) -- (3,0);");
    let p = points(&t.segments);
    assert!(close_to(p[0], (0.5, 0.0)) && close_to(p[1], (2.0, 0.0)), "{p:?}");
    // a closed end and a curve at the end: not shortened
    first_path(r"\draw[shorten >=1mm] (0,0) -- (1,0) -- cycle;");
    first_path(r"\draw[shorten >=1mm, shorten <=1mm] (0,0) arc (0:90:1);");
    // shortening by more than the segment: pgf moves the ends anyway, past each other (the end first,
    // then the start toward the moved end)
    let t = first_path(r"\draw[shorten >=5cm, shorten <=5cm] (0,0) -- (1,0);");
    assert!(close_to(points(&t.segments)[1], (-4.0, 0.0)), "{:?}", t.segments);
    // the last segment after a curve
    first_path(r"\draw[shorten >=1mm] (0,0) arc (0:90:1) -- (0,3);");
    // a one-segment path: nothing to shorten
    let c = canvas(r"\draw[shorten >=1mm] (0,0) circle (1);");
    assert_eq!(paths(&c).len(), 1);
}

#[test]
fn clip_and_end_clip_in_the_scope() {
    let c = canvas(r"\begin{scope} \clip (0,0) rectangle (1,1); \draw (0,0) -- (2,2); \end{scope}");
    assert!(matches!(c.items[0], Item::Clip { .. }), "{:?}", c.items);
    assert!(matches!(c.items.last(), Some(Item::End)), "{:?}", c.items);
}

#[test]
fn options_and_braces_mid_path_and_junk() {
    first_path(r"\draw (0,0) -- (1,0) [red] -- (2,0) {text} -- (3,0);");
    let (_, w) = canvas_and_warnings(r"\draw (0,0) -- (1,0) ??? -- (2,0);");
    assert!(w.iter().any(|a| a.contains("unrecognized")), "{w:?}");
    // an unmatched bracket in the statement's options
    let (_, w) = canvas_and_warnings(r"\draw[red (0,0) -- (1,0);");
    assert!(!w.is_empty());
    // an unmatched parenthesis
    let (_, w) = canvas_and_warnings(r"\draw (0,0 -- (1,0);");
    assert!(!w.is_empty());
    // options in several brackets
    first_path(r"\draw[red][thick] (0,0) -- (1,0);");
}

#[test]
fn path_without_drawing_does_not_become_a_path() {
    let c = canvas(r"\path (0,0) -- (1,1); \draw (0,0); \fill[draw=none, fill=none] (0,0) -- (1,0);");
    assert!(paths(&c).is_empty());
}

#[test]
fn arrows_without_stroke_vanish() {
    // pgf adds the tips only to a stroked path: each tip is a path of its own
    let c = canvas(r"\fill[->] (0,0) -- (1,0) -- (1,1);");
    assert_eq!(paths(&c).len(), 1);
    let c = canvas(r"\draw[<->] (0,0) -- (1,0);");
    assert_eq!(paths(&c).len(), 3, "the line and its two tips");
}

#[test]
fn nodes_on_a_path_inherit_its_color() {
    let c = canvas(r"\tikzset{f/.style={color=blue}} \draw[f] (0,0) -- (1,0) node[right] {a}; \draw[red] (0,0) -- (1,0) node {b}; \draw (0,0) -- (1,1) coordinate (end);");
    assert_eq!(texts(&c).len(), 2);
    // the text takes the path's color
    let colors: Vec<String> = texts(&c).iter().map(|(l, _)| l.color.hex()).collect();
    assert_eq!(colors, ["#0000ff", "#ff0000"]);
}

#[test]
fn error_inside_a_path_node_becomes_a_warning() {
    let (_, w) = canvas_and_warnings(r"\draw (0,0) -- (1,0) node[below=of nobody] {x};");
    assert!(w.iter().any(|a| a.contains("nobody")), "{w:?}");
}

#[test]
fn builder_edge_cases() {
    // a line with no start point: becomes the start point
    let t = first_path(r"\draw -- (1,0) -- (2,0);");
    assert!(matches!(t.segments[0], Segment::MoveTo(_)));
    // a line after closing: restarts from the start point
    let t = first_path(r"\draw (0,0) -- (1,0) -- (1,1) -- cycle -- (2,2);");
    assert!(matches!(t.segments[4], Segment::MoveTo(_)), "{:?}", t.segments);
    // space before the semicolon
    first_path("\\draw (0,0) -- (1,0)   \n ;");
    // an unknown option on to and on circle
    first_path(r"\draw (0,0) to[out=0, in=180, red] (2,0);");
    first_path(r"\draw (0,0) circle [radius=1, red];");
    // a node with two option blocks
    let c = canvas(r"\draw (0,0) -- (1,0) node[red][above] {x};");
    assert_eq!(texts(&c).len(), 1);
    // shortening a one-point path
    let c = canvas(r"\draw[shorten >=1mm] (0,0);");
    assert!(paths(&c).is_empty());
    // arrows with braces on both sides
    let (c, w) = canvas_and_warnings(r"\draw[{Latex[length=2mm]}-{Stealth[width=1mm]}] (0,0) -- (1,0);");
    assert!(w.is_empty(), "{w:?}");
    let p = paths(&c);
    assert_eq!(p.len(), 3, "the line and its two tips");
    // the line stops short of the 2mm Latex tip at its start
    assert!(points(&p[0].segments)[0].0 > 0.1, "{:?}", p[0].segments);
}

#[test]
fn coordinate_errors_in_each_operation() {
    for bad in [
        r"(zz:1) -- (1,1)",
        r"(0:zz) -- (1,1)",
        r"(0,0) .. controls (zz) .. (1,1)",
        r"(0,0) .. controls (1,1) and (zz) .. (2,2)",
        r"(0,0) .. controls (1,1) and (2,2) .. (zz)",
        r"(0,0) rectangle (zz)",
        r"(0,0) grid (zz)",
        r"(0,0) -- ($(zz)$)",
        r"(0,0) -- ($(0,0)!0.5!(zz)$)",
        r"(0,0) node at (zz) {x}",
        r"(0,0) -- (zz) -- (1,1)",
        r"(0,0) -- +(zz)",
        r"(0,0) plot coordinates {(zz,1)}",
    ] {
        let (_, w) = canvas_and_warnings(&format!(r"\draw {bad};"));
        assert!(!w.is_empty(), "{bad}");
    }
}

#[test]
fn node_between_operation_and_target_goes_midway() {
    let body = r"\draw (0,0) -- node (m) {m} (4,0); \draw (0,0) -| node (c) {c} (2,2); \draw (0,0) to[bend left] node[pos=0.25] (q) {q} (4,0); \draw (1,1) -- node (z) {z};";
    let n = |name: &str| point(body, &format!("{name}.center"));
    assert!(close_to((n("m").x, n("m").y), (2.0, 0.0)), "midway on a line: {:?}", n("m"));
    assert!(close_to((n("c").x, n("c").y), (2.0, 0.0)), "at the corner of -|: {:?}", n("c"));
    // pos is the Bézier parameter on the bent curve, as in TikZ: near x = 1, above the chord
    assert!((n("q").x - 1.0).abs() < 0.05 && n("q").y > 0.3, "an explicit pos wins, on the curve: {:?}", n("q"));
    assert!(close_to((n("z").x, n("z").y), (1.0, 1.0)), "no target: stays where the path is: {:?}", n("z"));
}

#[test]
fn bend_given_to_the_whole_statement() {
    let t = first_path(r"\draw[bend left=30] (0,0) to (4,0);");
    assert!(t.segments.iter().any(|s| matches!(s, Segment::CurveTo(..))), "{:?}", t.segments);
}

#[test]
fn node_on_a_bent_path_sits_on_the_curve() {
    // the chord runs along y = 0; bend right pulls the curve below it, and the node follows the curve
    let m = point(r"\coordinate (a) at (0,0); \coordinate (b) at (4,0); \draw (a) to[bend right] node (m) {m} (b);", "m.center");
    assert!(close(m.x, 2.0), "{m:?}");
    assert!(m.y < -0.3, "on the curve, not on the chord: {m:?}");
}

#[test]
fn curve_leaves_from_the_starting_node_border() {
    let c = canvas(r"\node[draw, circle, minimum size=1cm] (t) at (3,0) {}; \node[draw] (s) at (0,0) {}; \draw (t) to[bend left] (s);");
    let t = &paths(&c)[2];
    let Segment::MoveTo(p) = t.segments[0] else { panic!() };
    let r = p.minus(Point::new(3.0, 0.0)).norm();
    // outer sep: half the line width outside the drawn circle
    assert!(close(r, 0.5 + 0.2 * tikra::PT), "starts on the circle (radius 0.5), not at its center: {r}");
}

#[test]
fn translate_moves_every_kind_of_item() {
    let o = Point::new(0.0, 0.0);
    let mut items = vec![Item::Clip { segments: vec![Segment::MoveTo(o), Segment::CurveTo(o, o, o), Segment::Close], even_odd: false }, Item::End];
    for i in &mut items {
        i.translate(Point::new(1.0, 2.0));
    }
    let moved = |p: &Point| p.x == 1.0 && p.y == 2.0;
    assert!(
        matches!(&items[0], Item::Clip { segments: segs, .. } if matches!(segs[0], Segment::MoveTo(p) if moved(&p)) && matches!(segs[1], Segment::CurveTo(a, b, c) if [a, b, c].iter().all(moved)))
    );
}

#[test]
fn corners_arced_as_pgf_rounds_them() {
    // 10mm before each corner, 5mm after it, and the corner into a curve found along its first
    // control point; the curve keeps its control points
    let t = first_path(
        r"\pgfsetcornersarced{\pgfpoint{10mm}{5mm}} \pgfpathmoveto{\pgfpointorigin} \pgfpathlineto{\pgfpoint{0cm}{2cm}}
          \pgfpathlineto{\pgfpoint{3cm}{2cm}} \pgfpathcurveto{\pgfpoint{3cm}{0cm}}{\pgfpoint{2cm}{0cm}}{\pgfpoint{1cm}{0cm}} \pgfusepath{stroke}",
    );
    let p = points(&t.segments);
    let expected = [(0.0, 0.0), (0.0, 1.0), (0.5, 2.0), (2.0, 2.0), (3.0, 1.5), (1.0, 0.0)];
    for q in expected {
        assert!(p.iter().any(|r| close_to(*r, q)), "{q:?} not in {p:?}");
    }
}

#[test]
fn a_slice_overflows_its_box_and_leaves_the_size_alone() {
    // as pgf's pdf output: no clip to the target box, and the picture is as large as the frame
    let body = r"\draw (0,0) rectangle (2,2); \begin{scope}[slice={(0.5,0.5) (2.5,1.5) at (0,0) (2,2)}] \draw (0.5,0.5) rectangle (2.5,1.5); \end{scope}";
    let (w, h) = size(body);
    assert!(close(w, 2.0 + 0.4 * tikra::PT) && close(h, 2.0 + 0.4 * tikra::PT), "{w} {h}");
    assert!(!canvas(body).items.iter().any(|i| matches!(i, Item::Clip { .. })));
}

#[test]
fn transform_canvas_of_a_path_is_for_that_path_only() {
    // the node after it is not turned; a clip, though, stays for the rest of the scope
    let c = canvas(r"\draw[transform canvas={rotate=30}] (0,0) -- (1,0); \node at (0,-1) {A};");
    let ends_before_text = c.items.iter().take_while(|i| !matches!(i, Item::Text { .. })).filter(|i| matches!(i, Item::End)).count();
    let transforms = c.items.iter().filter(|i| matches!(i, Item::Transform { .. })).count();
    assert!(transforms == 1 && ends_before_text >= 1, "{:?}", c.items);
    let c = canvas(r"\clip (0,0) circle (1); \fill (1,0) circle (1);");
    let fill = c.items.iter().position(|i| matches!(i, Item::Path { fill: Some(_), .. })).unwrap();
    assert!(!c.items[..fill].iter().any(|i| matches!(i, Item::End)), "{:?}", c.items);
}

#[test]
fn svg_path_data_in_quotes() {
    // the svg.path library takes the data in quotes as in braces
    let quoted = first_path(r#"\draw svg "M0,0 L20,10 l5,0";"#).segments;
    assert_eq!(quoted, first_path(r"\draw svg {M0,0 L20,10 l5,0};").segments);
    assert_eq!(quoted.len(), 3);
    let scaled = first_path(r#"\draw svg[scale=2] "M0,0 L20,10";"#).segments;
    assert!(matches!(scaled[1], Segment::LineTo(p) if close(p.x, 40.0 * tikra::PT)), "{scaled:?}");
}

#[test]
fn parabola_height_is_scaled_with_the_picture() {
    // the bend is a point of the picture: twice as high under scale=2
    let t = first_path(r"\begin{scope}[scale=2] \draw (0,0) parabola[parabola height=1cm] +(2,0); \end{scope}");
    let p = points(&t.segments);
    assert!(close_to(p[1], (2.0, 2.0)), "the bend in {p:?}");
}

#[test]
fn let_points_are_in_the_picture_coordinates() {
    // \x1 of (1,1) under scale=2 is 1cm: (\x1,0) is scaled again, to 2cm
    let t = first_path(r"\begin{scope}[scale=2] \draw let \p1=(1,1) in (0,0) -- (\x1,0); \end{scope}");
    assert!(close_to(points(&t.segments)[1], (2.0, 0.0)), "{:?}", points(&t.segments));
    // in a shifted, turned and scaled scope, (\x1,\y1) of a named point is that point again
    let c = canvas(r"\coordinate (a) at (1,1); \begin{scope}[xshift=1cm,rotate=90,scale=2] \draw let \p1=(a) in (0,0) -- (\x1,\y1); \end{scope}");
    let p = points(&paths(&c)[0].segments);
    assert!(close_to(p[1], (1.0, 1.0)), "{p:?}");
}

fn pt(p: Point) -> (f64, f64) {
    (p.x, p.y)
}

#[test]
fn polar_radius_keys_are_read_in_order() {
    // `radius` sets both radii; a later `x radius` sets one of them again, and the other way round
    assert!(close_to(pt(point("", "xyz polar cs:angle=180,radius=1,x radius=2")), (-2.0, 0.0)));
    assert!(close_to(pt(point("", "xyz polar cs:angle=0,x radius=2,radius=1")), (1.0, 0.0)));
    assert!(close_to(pt(point("", "polar cs:angle=90,radius=1,y radius=3")), (0.0, 3.0)));
}

#[test]
fn to_curves_follow_the_transformation() {
    let controls = |body: &str| match first_path(body).segments[1] {
        Segment::CurveTo(c1, c2, _) => (pt(c1), pt(c2)),
        s => panic!("{s:?}"),
    };
    // the distance is measured in the picture's units (3 there, so 0.3915·3 = 1.1745), not on the
    // page, where xscale makes it 6
    let (c1, c2) = controls(r"\begin{scope}[xscale=2] \draw (0,0) to[out=-90,in=-90] (3,0); \end{scope}");
    assert!(close_to(c1, (0.0, -1.1745)) && close_to(c2, (6.0, -1.1745)), "{c1:?} {c2:?}");
    // the angles turn with the picture
    let (c1, _) = controls(r"\begin{scope}[rotate=90] \draw (0,0) to[out=0,in=180] (1,0); \end{scope}");
    assert!(close_to(c1, (0.0, 0.3915)), "{c1:?}");
    // and slant with it
    let (c1, _) = controls(r"\begin{scope}[xslant=1] \draw (0,0) to[out=90,in=90] (1,0); \end{scope}");
    assert!(close_to(c1, (0.3915, 0.3915)), "{c1:?}");
    // so do the bends, measured from the line in the picture's units
    let (c1, _) = controls(r"\begin{scope}[xscale=2] \draw (0,0) to[bend left=30] (3,0); \end{scope}");
    let (s, c) = 30f64.to_radians().sin_cos();
    assert!(close_to(c1, (2.0 * 1.1745 * c, 1.1745 * s)), "{c1:?}");
}

#[test]
fn calc_distance_toward_the_point_itself_goes_up() {
    // a direction of no length is (0,1), turned or not
    let body = r"\coordinate (a) at (1,1);";
    assert!(close_to(pt(point(body, "$(a)!0.5cm!(a)$")), (1.0, 1.5)));
    assert!(close_to(pt(point(body, "$(a)!0.5cm!90:(a)$")), (1.0, 1.5)));
}
