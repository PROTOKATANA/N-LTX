//! Data visualizations (`\datavisualization`): the data is read, scaled by the axes and drawn where
//! pgf draws it; the axis systems, ticks, style sheets and legends come out without warnings.

use tikra::canvas::Item;
use tikra::scene::{Point, Segment};
use tikra::PT;

/// What the engine draws for a picture body, without warnings.
fn items(body: &str) -> Vec<Item> {
    let (canvas, warnings) = tikra::engine::run_document(&format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}")).unwrap();
    assert!(warnings.is_empty(), "{body}: {warnings:?}");
    canvas.items
}

fn paths(body: &str) -> Vec<Vec<Segment>> {
    items(body).into_iter().filter_map(|i| if let Item::Path { segments, .. } = i { Some(segments) } else { None }).collect()
}

fn texts(body: &str) -> usize {
    items(body).iter().filter(|i| matches!(i, Item::Text { .. })).count()
}

fn end(s: &Segment) -> Option<Point> {
    match s {
        Segment::MoveTo(p) | Segment::LineTo(p) | Segment::CurveTo(_, _, p) => Some(*p),
        Segment::Close => None,
    }
}

fn near(p: Point, x: f64, y: f64) -> bool {
    (p.x - x).abs() < 1e-3 && (p.y - y).abs() < 1e-3
}

#[test]
fn xy_cartesian_maps_one_unit_to_one_centimeter() {
    let p = paths(r"\datavisualization [xy Cartesian, visualize as line] data point [x=0, y=0] data point [x=2, y=1];");
    let line = &p[0];
    let d = end(&line[1]).unwrap().minus(end(&line[0]).unwrap());
    assert!(near(d, 2.0, 1.0), "{line:?}");
}

#[test]
fn tables_and_functions_give_data_points() {
    // a table: a head line, then one data point per line
    let p = paths("\\datavisualization [xy Cartesian, visualize as line]\n  data {\n    x, y\n    0, 0\n    1, 2\n    3, 1\n  };");
    assert_eq!(p[0].len(), 3);
    // a function: 5 samples from 0 to 2
    let p = paths(
        "\\datavisualization [xy Cartesian, visualize as line]\n  data [format=function] {\n    var x : interval [0:2] samples 5;\n    func y = \\value x * \\value x;\n  };",
    );
    assert_eq!(p[0].len(), 5);
    assert!(near(end(&p[0][4]).unwrap().minus(end(&p[0][0]).unwrap()), 2.0, 4.0), "{:?}", p[0]);
}

#[test]
fn scientific_axes_fit_the_data_in_their_width() {
    let p = paths(r"\datavisualization [scientific axes, visualize as line] data point [x=10, y=1] data point [x=20, y=3];");
    // the data goes from 0 to the width, 5cm, and from 0 to the height, 0.618 × 5cm
    let fits = |l: &Vec<Segment>| l.len() == 2 && near(end(&l[1]).unwrap().minus(end(&l[0]).unwrap()), 5.0, 0.618 * 5.0);
    assert!(p.iter().any(fits), "{p:?}");
}

#[test]
fn ticks_have_texts() {
    let n = texts(r"\datavisualization [scientific axes, x axis={ticks={step=5}}, visualize as line] data point [x=0, y=0] data point [x=20, y=1];");
    // 0, 5, 10, 15, 20 on x, and the y axis' own
    assert!(n >= 5, "{n} texts");
}

#[test]
fn school_book_axes_have_arrows_through_the_origin() {
    let p = paths(r"\datavisualization [school book axes, visualize as smooth line] data [format=function] { var x : interval [-1:1]; func y = \value x*\value x; };");
    // the x axis: from -1 - padding to 1 + padding, horizontal (a bit shorter for its arrow tip)
    let pad = 7.5 * PT;
    let axis = |l: &Vec<Segment>| {
        let d = end(&l[l.len() - 1]).unwrap().minus(end(&l[0]).unwrap());
        l.len() == 2 && d.y.abs() < 1e-6 && (d.x - 2.0 - 2.0 * pad).abs() < 0.05
    };
    assert!(p.iter().any(axis), "{p:?}");
}

#[test]
fn style_sheets_legends_and_polar_axes_run_without_warnings() {
    items(
        r"\datavisualization [scientific axes=clean, visualize as line/.list={a,b}, style sheet=strong colors, style sheet=vary dashing,
          a={label in legend={text=first}}, b={label in legend={text=second}}]
          data [set=a] { x, y
          0, 0
          1, 1 }
          data point [x=0, y=1, set=b] data point [x=1, y=0, set=b];",
    );
    items(r"\datavisualization [scientific polar axes, visualize as smooth line] data [format=function] { var angle : interval [0:90]; func radius = \value{angle}; };");
}

#[test]
fn a_tick_placement_strategy_of_ones_own() {
    let n = texts(
        r"\def\primes{\tikzdatavisualizationset{major={at={2,3,5,7}}}}
          \datavisualization [scientific axes, visualize as scatter, x axis={tick placement strategy=\primes}]
          data point [x=0, y=0] data point [x=10, y=10];",
    );
    // 2, 3, 5, 7 on x, and the y axis' own
    assert!(n >= 4 + 3, "{n} texts");
}

#[test]
fn a_new_axis_system_from_its_parts() {
    let p = paths(
        r"\tikzdatavisualizationset{new axis system={my axes}{xy Cartesian}{}{x axis={visualize axis={y axis={goto=0}}}}}
          \datavisualization [my axes, visualize as line] data point [x=0, y=0] data point [x=3, y=1];",
    );
    // the x axis, from 0 to 3
    let horizontal = |l: &Vec<Segment>| l.len() == 2 && near(end(&l[1]).unwrap().minus(end(&l[0]).unwrap()), 3.0, 0.0);
    assert!(p.iter().any(horizontal), "{p:?}");
}

#[test]
fn about_divides_as_the_fpu_does() {
    // a sine sampled to TeX's precision spans 1.99998: about 5 ticks is a step of 0.5 (pgf's FPU
    // rounds 1.99998/5 to 0.4), not 0.25; so -1 to 1 by 0.5 and 0 to 8 by 2
    let n = texts(r"\datavisualization [scientific axes=clean, visualize as smooth line] data [format=function] { var x : interval[0:3*pi]; func y = cos(\value x r); };");
    assert_eq!(n, 10);
}
