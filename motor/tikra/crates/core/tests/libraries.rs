//! The TikZ libraries made of styles and small additions: automata, petri, er, shadows, through,
//! spy, 3d, perspective and the matrix library's delimiters.

use tikra::canvas::{Item, Paint};
use tikra::scene::{Point, Segment};
use tikra::PT;

/// What the engine draws for a picture body, without warnings.
fn items(body: &str) -> Vec<Item> {
    let (canvas, warnings) = tikra::engine::run_document(&format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}")).unwrap();
    assert!(warnings.is_empty(), "{body}: {warnings:?}");
    canvas.items
}

/// What the engine draws for a picture with options.
fn picture(options: &str, body: &str) -> Vec<Item> {
    let (canvas, warnings) = tikra::engine::run_document(&format!("\\begin{{tikzpicture}}{options}{body}\\end{{tikzpicture}}")).unwrap();
    assert!(warnings.is_empty(), "{body}: {warnings:?}");
    canvas.items
}

fn paths(body: &str) -> Vec<Vec<Segment>> {
    items(body).into_iter().filter_map(|i| if let Item::Path { segments, .. } = i { Some(segments) } else { None }).collect()
}

fn near(p: Point, x: f64, y: f64) -> bool {
    (p.x - x).abs() < 1e-3 && (p.y - y).abs() < 1e-3
}

fn point(s: &Segment) -> Point {
    match *s {
        Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => p,
        Segment::Close => panic!("close"),
    }
}

fn first_point(s: &[Segment]) -> Point {
    point(&s[0])
}

#[test]
fn an_initial_state_has_an_arrow_from_the_left() {
    // the arrow starts 3ex left of the state's west side, and ends at its border
    let p = paths(r"\node[state,initial] (q) {$q$};");
    let arrow = p.iter().find(|s| matches!(s.last(), Some(Segment::LineTo(_)))).expect("the initial arrow");
    let (a, b) = (point(&arrow[arrow.len() - 2]), point(&arrow[arrow.len() - 1]));
    assert!(a.x < b.x && (a.y - b.y).abs() < 1e-9, "{arrow:?}");
}

#[test]
fn an_accepting_state_is_drawn_double() {
    let strokes: Vec<f64> =
        items(r"\node[state,accepting] {$q$};").into_iter().filter_map(|i| if let Item::Path { stroke: Some(s), .. } = i { Some(s.width / PT) } else { None }).collect();
    // the outer line, 2 × 0.4pt + 0.6pt, then the white core
    assert!(strokes.iter().any(|w| (w - 1.4).abs() < 1e-6) && strokes.iter().any(|w| (w - 0.6).abs() < 1e-6), "{strokes:?}");
}

#[test]
fn on_grid_places_centers_at_the_node_distance() {
    let p = paths(r"\node[draw] (a) {A}; \node[draw,on grid,right=2cm of a] (b) {Longer}; \draw (a.center) -- (b.center);");
    let line = p.last().unwrap();
    assert!(near(point(&line[1]).minus(point(&line[0])), 2.0, 0.0), "{line:?}");
}

#[test]
fn tokens_are_placed_around_the_center_of_a_place() {
    // the place, then two tokens at ±.5 × 1.5ex (by their rightmost points)
    let p = paths(r"\node[place,tokens=2] at (0,0) {};");
    let ex = 4.3 * PT;
    let centers: Vec<f64> = p[1..].iter().map(|s| first_point(s).x - first_point(&p[0]).x).collect();
    assert_eq!(centers.len(), 2, "{p:?}");
    assert!(centers[0] < 0.0 && (centers[1] - centers[0] - 1.5 * ex).abs() < 1e-3, "{centers:?}");
}

#[test]
fn er_shapes_are_styles() {
    assert_eq!(paths(r"\node[entity] {E}; \node[relationship] at (2,0) {R}; \node[attribute] at (4,0) {A};").len(), 3);
}

/// The items as drawn: those inside a `transform canvas` group moved by it.
fn drawn(items: Vec<Item>) -> Vec<Item> {
    // what each open group does to what is inside (clips, fadings, groups: nothing)
    let mut stack: Vec<tikra::scene::Transform> = Vec::new();
    let mut out = Vec::new();
    for mut i in items {
        let outer = stack.last().copied().unwrap_or_default();
        match &i {
            Item::Transform { transform } => {
                stack.push(outer.compose(transform));
                continue;
            }
            Item::Clip { .. } | Item::Group { .. } | Item::Fade { .. } => stack.push(outer),
            Item::End => {
                stack.pop();
            }
            _ => {}
        }
        i.transform(&outer);
        out.push(i);
    }
    out
}

#[test]
fn a_drop_shadow_is_a_shifted_copy_behind() {
    let it = drawn(items(r"\node[fill=white,drop shadow] {A};"));
    let fills: Vec<(Point, f64)> = it
        .iter()
        .filter_map(|i| match i {
            Item::Path { segments, fill: Some(f), .. } => Some((first_point(segments), f.opacity)),
            _ => None,
        })
        .collect();
    // first the shadow, .5ex right and down, half transparent
    let (shadow, node) = (fills[0], fills[1]);
    let d = shadow.0.minus(node.0);
    assert!(near(d, 0.5 * 4.3 * PT, -0.5 * 4.3 * PT) && shadow.1 == 0.5, "{fills:?}");
}

#[test]
fn a_copy_shadow_has_the_colors_of_the_node() {
    let it = drawn(items(r"\node[copy shadow,fill=red,draw=blue] {A};"));
    match &it[0] {
        Item::Path { fill: Some(f), stroke: Some(s), .. } => {
            assert_eq!(f.paint, Paint::Solid(tikra::color::Color::rgb(1.0, 0.0, 0.0)));
            assert_eq!(s.color, tikra::color::Color::rgb(0.0, 0.0, 1.0));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn options_of_a_coordinate_move_a_node_anchor() {
    let p = paths(r"\node (a) at (1,1) {}; \draw (a.center) -- ([shift={(1,0)}]a.center) -- ([xshift=1cm,yshift=1cm]a.center);");
    let (a, b, c) = (point(&p[0][0]), point(&p[0][1]), point(&p[0][2]));
    assert!(near(b.minus(a), 1.0, 0.0) && near(c.minus(a), 1.0, 1.0), "{p:?}");
}

#[test]
fn a_plane_at_z_becomes_the_canvas() {
    // with z=(1,1)cm, the xy plane at z=1 is the xy plane moved by (1,1)
    let p = paths(r"\draw (0,0) -- (1,0); \begin{scope}[z={(1cm,1cm)},canvas is xy plane at z=1] \draw (0,0) -- (2,0); \end{scope}");
    let (o, a, b) = (first_point(&p[0]), first_point(&p[1]), point(&p[1][1]));
    assert!(near(a.minus(o), 1.0, 1.0) && near(b.minus(a), 2.0, 0.0), "{p:?}");
}

#[test]
fn cylindrical_and_spherical_coordinates() {
    let p = paths(r"\draw (0,0,0) -- (xyz cylindrical cs:radius=2,angle=90,z=1) -- (xyz spherical cs:radius=1,longitude=90);");
    let o = first_point(&p[0]);
    // z is (-3.85mm, -3.85mm) by default; longitude 90 is along x
    let z = -0.385;
    assert!(near(point(&p[0][1]).minus(o), z, 2.0 + z) && near(point(&p[0][2]).minus(o), 1.0, 0.0), "{p:?}");
}

#[test]
fn three_point_perspective() {
    // with a vanishing point at x=10, x=5 is halfway there
    let p = paths(r"\begin{scope}[perspective={p={(10,0,0)}}] \draw (tpp cs:x=0) -- (tpp cs:x=5) -- (tpp cs:x=5,y=1); \end{scope}");
    let o = first_point(&p[0]);
    assert!(near(point(&p[0][1]).minus(o), 10.0 / 3.0, 0.0) && near(point(&p[0][2]).minus(o), 10.0 / 3.0, 2.0 / 3.0), "{p:?}");
}

#[test]
fn a_3d_view_sets_the_unit_vectors() {
    let p = paths(r"\draw[3d view={90}{0}] (0,0,0) -- (1,0,0) -- (1,1,0) -- (1,1,1);");
    let o = first_point(&p[0]);
    // azimuth 90: x points down the page, y right; elevation 0: z up
    assert!(near(point(&p[0][1]).minus(o), 0.0, 0.0) && near(point(&p[0][2]).minus(o), 1.0, 0.0) && near(point(&p[0][3]).minus(o), 1.0, 1.0), "{p:?}");
}

#[test]
fn a_circle_through_a_point() {
    // radius 1: the circle's rightmost point is 1 right of the center
    let p = paths(r"\coordinate (a) at (1,0); \node[draw,circle through=(a)] (c) at (0,0) {}; \draw (c.center) -- (c.east);");
    let line = p.last().unwrap();
    assert!(near(point(&line[1]).minus(point(&line[0])), 1.0, 0.0), "{line:?}");
}

#[test]
fn delimiters_beside_a_matrix() {
    let r = tikra::convert(r"\begin{tikzpicture}\matrix[left delimiter=(,right delimiter={]}] (m) {\node{a}; & \node{b};\\ \node{c}; & \node{d};\\};\end{tikzpicture}").unwrap();
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
    // the two delimiters are drawn as text, left and right of the four cells
    assert_eq!(r.svg.matches("<g transform").count(), 6, "{}", r.svg);
}

#[test]
fn the_cells_style_is_not_the_delimiters() {
    // `nodes={…}` is for the cells: a delimiter is not a 1 cm circle, which would put it half a
    // centimeter further out
    let it = items(r"\matrix[matrix of nodes, nodes={draw,circle,minimum size=1cm}, left delimiter=(] {a & b\\};");
    let cell = it.iter().find_map(|i| if let Item::Path { segments, stroke: Some(_), .. } = i { Some(segments.clone()) } else { None }).unwrap();
    let left = cell.iter().filter(|s| !matches!(s, Segment::Close)).map(|s| point(s).x).fold(f64::MAX, f64::min);
    // the leftmost text is the delimiter's
    let delimiter = it.iter().filter_map(|i| if let Item::Text { place, .. } = i { Some(place.e) } else { None }).fold(f64::MAX, f64::min);
    assert!(delimiter < left && left - delimiter < 0.6, "the delimiter at {delimiter}, the cells from {left}");
}

#[test]
fn pgfmath_results_with_units_are_in_points() {
    let p = paths(r"\pgfmathsetmacro\l{1cm} \draw (0,0) -- (\l pt,0);");
    assert!(near(point(&p[0][1]).minus(point(&p[0][0])), 1.0, 0.0), "{p:?}");
}

#[test]
fn a_spy_shows_the_picture_magnified_and_clipped() {
    let it = picture("[spy using outlines={circle,magnification=2,size=1cm,connect spies}]", r"\draw (0,0) -- (1,0); \spy on (0.5,0) in node at (3,0);");
    // the line, the node on (0.5,0), the clip of the node at (3,0) with the line twice as long and
    // twice as thick in it, that node's border, the connection
    let lines: Vec<(Point, Point, f64)> = it
        .iter()
        .filter_map(|i| match i {
            Item::Path { segments, stroke: Some(s), .. } if segments.len() == 2 => Some((first_point(segments), point(&segments[1]), s.width)),
            _ => None,
        })
        .collect();
    let (a0, a1, w) = lines[0];
    let (b0, b1, w2) = lines[1];
    assert!(near(a1.minus(a0), 1.0, 0.0) && near(b1.minus(b0), 2.0, 0.0) && (w2 - 2.0 * w).abs() < 1e-9, "{lines:?}");
    // the magnified line is centered on the node at (3,0): 2.5 right of the spied point
    assert!(near(b0.lerp(b1, 0.5).minus(a0.lerp(a1, 0.5)), 2.5, 0.0), "{lines:?}");
    assert!(it.iter().any(|i| matches!(i, Item::Clip { .. })));
}

#[test]
fn a_spy_reads_its_point_when_the_scope_ends() {
    // with the macros it names as they are then
    let it =
        picture("[spy using outlines={magnification=2,size=1cm}]", r"\def\k{-1} \fill (0,-1) circle (1pt); \fill (0,1) circle (1pt); \spy on (0,\k) in node at (3,0); \gdef\k{1}");
    // the node on the point: a square of 0.5 cm around (0,1)
    let on = it
        .iter()
        .find_map(|i| match i {
            Item::Path { segments, stroke: Some(_), .. } if segments.len() == 5 => Some(segments.clone()),
            _ => None,
        })
        .unwrap();
    let mid = first_point(&on).lerp(point(&on[2]), 0.5);
    // 2 cm above the first dot, at (0,-1)
    let dot = it.iter().find_map(|i| if let Item::Path { segments, fill: Some(_), .. } = i { Some(segments.clone()) } else { None }).unwrap();
    let (lo, hi) = dot.iter().fold((Point::new(f64::MAX, f64::MAX), Point::new(f64::MIN, f64::MIN)), |(lo, hi), s| match s {
        Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => (Point::new(lo.x.min(p.x), lo.y.min(p.y)), Point::new(hi.x.max(p.x), hi.y.max(p.y))),
        Segment::Close => (lo, hi),
    });
    assert!(near(mid.minus(lo.lerp(hi, 0.5)), 0.0, 2.0), "{on:?}");
}

#[test]
fn a_functional_shading_is_sampled_under_its_path_only() {
    // the path shows the middle of the shading's box: its 64 rows are there, not over the whole
    // box, and each row's cells reach over the whole next row, which draws on them: no seam shows
    // between rows thinner than a pixel
    for shade in [r"\shade[shading=color wheel] (0,0) rectangle (5,5);", r"\shade[upper left=red,upper right=green,lower left=blue,lower right=yellow] (0,0) rectangle (5,5);"] {
        let cells: Vec<(f64, f64)> = items(shade)
            .iter()
            .filter_map(|i| match i {
                Item::Path { segments, fill: Some(f), .. } if matches!(f.paint, Paint::Solid(_)) => Some((first_point(segments).y, point(&segments[2]).y)),
                _ => None,
            })
            .collect();
        let mut starts: Vec<f64> = cells.iter().map(|c| c.0).collect();
        starts.sort_by(f64::total_cmp);
        starts.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        let row = 5.0 / 64.0;
        assert!(starts.windows(2).all(|w| (w[1] - w[0] - row).abs() < 1e-6), "{shade}: {starts:?}");
        let top = starts.last().unwrap() + row;
        assert!(cells.iter().all(|&(y0, y1)| y1 - y0 > 2.0 * row - 1e-6 || (y1 - top).abs() < 1e-6), "{shade}: {cells:?}");
    }
}

#[test]
fn chainin_continues_with_after_node_path() {
    // a join of one's own, as `after node path`: the edge from the previous node on the chain;
    // given no value, its `#1` (`\pgfkeysnovalue`) is nothing
    let it = items(
        r"\tikzset{link/.code={\tikzset{after node path={(\tikzchainprevious) edge[->] #1 (\tikzchaincurrent)}}}}
          \node (a) at (0,0) {A}; \node (b) at (2,0) {B}; \node (c) at (4,0) {C};
          { [start chain] \chainin (a); \chainin (b) [link]; \chainin (c) [link={node[above] {x}}]; }",
    );
    let edges = it.iter().filter(|i| matches!(i, Item::Path { segments, stroke: Some(_), fill: None } if segments.len() == 2)).count();
    assert_eq!(edges, 2, "{it:?}");
}

#[test]
fn a_spy_outside_a_spy_scope_is_reported() {
    let (_, warnings) = tikra::engine::run_document(r"\begin{tikzpicture}\spy on (0,0) in node at (1,1);\end{tikzpicture}").unwrap();
    assert!(warnings.iter().any(|w| w.contains("outside a spy scope")), "{warnings:?}");
}

#[test]
fn every_picture_keys_are_tikz_keys() {
    let (_, warnings) =
        tikra::engine::run_document(r"\tikzset{every picture/.style={on grid,node distance=2cm}}\begin{tikzpicture}\node (a) {}; \node[right=of a] {};\end{tikzpicture}").unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn insert_path_puts_operations_where_the_options_are() {
    let p = paths(r"\draw[insert path={-- (1,0)}] (0,0) [insert path={-- (1,1)}];");
    assert_eq!(p[0].len(), 3, "{p:?}");
}

#[test]
fn scope_fading_of_an_empty_path_is_fitted_to_that_path() {
    // pgf fits the fading to the path that gives it: the circle drawn next is faded as the
    // rectangle would be
    let mask = |body: &str| {
        items(&format!(r"\usetikzlibrary{{fadings}}{body}"))
            .into_iter()
            .find_map(|i| match i {
                Item::Fade { mask: tikra::canvas::Shading::Axial { transform, .. } } => Some(transform),
                _ => None,
            })
            .unwrap()
    };
    let carried = mask(r"\path[scope fading=south] (-2,-2) rectangle (2,2); \fill (0,1) circle (1);");
    let own = mask(r"\fill[scope fading=south] (-2,-2) rectangle (2,2); \path (0,1) circle (1);");
    assert_eq!(carried, own);
}

#[test]
fn system_layer_animation_snapshot() {
    // the timeline of \pgfsysanimkeytime/\pgfsysanimval… at the snapshot time, given to the node
    // the id refers to; without a snapshot, the node keeps its base state
    let fill_opacity = |snapshot: &str| {
        let (canvas, warnings) = tikra::engine::run_document(&format!(
            r"{snapshot}\tikz {{
              \pgfidrefnextuse{{\objid}}{{node}}\pgfsysanimkeywhom{{\objid}}{{}}
              \pgfsysanimkeytime{{0}}{{1}}{{1}}{{0}}{{0}}\pgfsysanimvalscalar{{1}}
              \pgfsysanimkeytime{{2}}{{1}}{{1}}{{0}}{{0}}\pgfsysanimvalscalar{{0}}
              \pgfsysanimate{{opacity}}
              \node (node) [fill=blue!20, circle] {{Hi}};
            }}"
        ))
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        canvas.items.iter().find_map(|i| if let Item::Path { fill: Some(f), .. } = i { Some(f.opacity) } else { None }).unwrap()
    };
    assert!((fill_opacity(r"\pgfsysanimsnapshot{0.5}") - 0.75).abs() < 1e-6);
    assert!((fill_opacity(r"\pgfsysanimsnapshot{2}") - 0.0).abs() < 1e-6);
    assert!((fill_opacity("") - 1.0).abs() < 1e-6);
}
