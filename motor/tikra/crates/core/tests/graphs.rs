//! `\graph`: where the graphs library puts the nodes, and which edges its operators make.

use tikra::canvas::Item;
use tikra::scene::Point;

/// The origins of the texts of a figure (cm), the number of paths and the warnings.
fn draw(body: &str) -> (Vec<Point>, usize, Vec<String>) {
    let code = format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}");
    let (canvas, warnings) = tikra::engine::run_document(&code).unwrap();
    let mut texts = Vec::new();
    let mut paths = 0;
    for i in &canvas.items {
        match i {
            Item::Text { place, .. } => texts.push(Point::new(place.e, place.f)),
            Item::Path { .. } => paths += 1,
            _ => {}
        }
    }
    (texts, paths, warnings)
}

/// The text origins relative to the first one.
fn offsets(body: &str) -> Vec<(f64, f64)> {
    let (t, _, w) = draw(body);
    assert!(w.is_empty(), "{w:?}");
    t.iter().map(|p| (p.x - t[0].x, p.y - t[0].y)).collect()
}

fn near(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(p, q)| (p.0 - q.0).abs() < 1e-6 && (p.1 - q.1).abs() < 1e-6)
}

#[test]
fn chains_grow_by_the_chain_shift_and_groups_by_the_group_shift() {
    // the same text in every node, so the texts are where the nodes are
    let o = offsets(r"\graph { a [as=x] -> b [as=x] -> c [as=x] };");
    assert!(near(&o, &[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)]), "{o:?}");
    let o = offsets(r"\graph [grow down, branch right=2cm] { a [as=x] -> { b [as=x], c [as=x] } -> d [as=x] };");
    assert!(near(&o, &[(0.0, 0.0), (0.0, -1.0), (2.0, -1.0), (0.0, -2.0)]), "{o:?}");
    // a group with a placement of its own counts for nothing outside
    let o = offsets(r"\graph { a [as=x], { [grow right] b [as=x] } -> c [as=x] };");
    assert!(near(&o, &[(0.0, 0.0), (0.0, -1.0), (0.0, -1.0)]), "{o:?}");
}

#[test]
fn circular_placement_starts_at_the_top_and_turns_clockwise() {
    let o = offsets(r"\graph [clockwise=4, typeset=x] { a, b, c, d };");
    assert!(near(&o, &[(0.0, 0.0), (1.0, -1.0), (0.0, -2.0), (-1.0, -1.0)]), "{o:?}");
}

#[test]
fn operators_make_the_edges() {
    // four nodes, no arrows: a clique has six edges, a cycle four, a path three
    for (graph, edges) in [("subgraph K_n [n=4]", 6), ("subgraph C_n [n=4]", 4), ("subgraph P_n [n=4]", 3), ("subgraph I_n [n=4]", 0)] {
        let (t, p, w) = draw(&format!(r"\graph {{ {graph} }};"));
        assert!(w.is_empty(), "{w:?}");
        assert_eq!((t.len(), p), (4, edges), "{graph}");
    }
    // {a,b} -- {c,d}: matching and star by default, complete bipartite when asked
    assert_eq!(draw(r"\graph { {a, b} -- {c, d} };").1, 2);
    assert_eq!(draw(r"\graph { {a, b} -- [complete bipartite] {c, d} };").1, 4);
    assert_eq!(draw(r"\graph { a -- {b, c, d} };").1, 3);
    assert_eq!(draw(r"\graph { a -!- b };").1, 0);
}

#[test]
fn a_simple_graph_keeps_one_edge_between_two_nodes() {
    let multi = draw(r"\graph { a -- b; a -- [red] b; b -- a };").1;
    let simple = draw(r"\graph [simple] { a -- b; a -- [red] b; b -- a };").1;
    assert_eq!((multi, simple), (3, 1));
    assert_eq!(draw(r"\graph [simple] { subgraph K_n [n=4]; 1 -!- 2 };").1, 5);
}

#[test]
fn names_texts_labels_and_quotes() {
    // a node named once is the same node later on
    let (t, p, w) = draw(r"\graph { a -> b; b -> a; c / {x y} -- a };");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(t.len(), 3);
    assert!(p >= 3);
    // edge labels and quotes are nodes on the edges
    assert_eq!(draw(r#"\graph { a ->["x"] b -> [edge label=y] c };"#).0.len(), 5);
    assert_eq!(draw(r#"\graph { a -> {b [> "u"], c [>"v"']} };"#).0.len(), 5);
    // nodes named in a group with name=… can be used after the graph
    let (_, _, w) = draw(r"\graph { { [name=g] 1, 2 } -- 3 }; \draw (g 1) -- (g 2);");
    assert!(w.is_empty(), "{w:?}");
    // empty nodes have no text, math nodes are typeset in math
    assert_eq!(draw(r"\graph [empty nodes] { a -> b };").0.len(), 0);
}
