//! pgfplots: the axis' size, its ticks and labels, plots by expression, coordinates and table,
//! the cycle list and marks, bars, legends, log axes and TikZ statements inside an axis.

mod common;
use common::*;
use tikra::canvas::{Canvas, Item, Paint};
use tikra::scene::Segment;

const PT: f64 = tikra::PT;

fn canvas(body: &str) -> (Canvas, Vec<String>) {
    tikra::engine::run_document(&format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}")).unwrap_or_else(|e| panic!("{e}\n{body}"))
}

fn axis(options: &str, body: &str) -> (Canvas, Vec<String>) {
    canvas(&format!("\\begin{{axis}}[{options}] {body} \\end{{axis}}"))
}

/// Width and height of the drawing, in pt.
fn size(c: &Canvas) -> (f64, f64) {
    let (a, b) = (c.min.unwrap(), c.max.unwrap());
    ((b.x - a.x) / PT, (b.y - a.y) / PT)
}

fn texts(c: &Canvas) -> usize {
    c.items.iter().filter(|i| matches!(i, Item::Text { .. })).count()
}

fn filled(c: &Canvas) -> Vec<&Vec<Segment>> {
    c.items.iter().filter_map(|i| if let Item::Path { segments, fill: Some(_), .. } = i { Some(segments) } else { None }).collect()
}

#[test]
fn default_size_is_pgfplots() {
    // 240pt × 207pt with the labels: TeX draws this one 233.6pt × 182.9pt (empty labels included)
    let (c, w) = axis("", r"\addplot+[sharp plot] coordinates {(0,0) (1,2) (2,3)};");
    assert!(w.is_empty(), "{w:?}");
    let (wd, ht) = size(&c);
    assert!((wd - 233.6).abs() < 2.0 && (ht - 182.9).abs() < 2.0, "{wd} × {ht}");
    // scale only axis: the box is exactly the size
    let (c, _) = axis("scale only axis, width=5cm, height=3cm, hide axis", r"\addplot coordinates {(0,0) (1,1)};");
    let (wd, ht) = size(&c);
    assert!(wd >= 5.0 / 2.54 * 72.27 - 1.0, "{wd} {ht}");
}

#[test]
fn description_cs_is_the_bounding_box_of_a_skewed_axis() {
    // the labels (empty here) are at `axis description cs` points: of the axis' bounding box, not
    // of its slanted sides. TeX draws this one 225.8pt × 154.1pt
    let (c, _) = axis("x={(5pt,1pt)}, y={(-4pt,4pt)}", r"\addplot {1-x^2};");
    let (wd, ht) = size(&c);
    assert!((wd - 225.8).abs() < 1.0 && (ht - 154.1).abs() < 1.0, "{wd} × {ht}");
}

#[test]
fn axis_line_styles() {
    for lines in ["left", "middle", "center", "none", "box", "right"] {
        let (c, w) = axis(&format!("xmin=-1, xmax=1, ymin=-1, ymax=1, axis lines={lines}"), r"\addplot {x};");
        assert!(w.is_empty(), "{lines}: {w:?}");
        assert!(!c.items.is_empty(), "{lines}");
    }
}

#[test]
fn ticks_follow_the_algorithm() {
    // x from -6 to 6 (domain -5:5 enlarged by 10%): ticks every 2, seven labels; y: 0, 10, 20 (as TeX draws it)
    let (c, w) = axis("", r"\addplot[samples=3] {x^2};");
    assert!(w.is_empty(), "{w:?}");
    // 7 x labels, 3 y labels, and the two empty label nodes draw nothing
    assert_eq!(texts(&c), 10, "{}", texts(&c));
    // explicit ticks and labels
    let (c, w) = axis("xtick={0,1,2}, xticklabels={a,b,c}, ytick=\\empty", r"\addplot coordinates {(0,0) (2,1)};");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(texts(&c), 3);
}

#[test]
fn expressions_tables_and_coordinates() {
    let (_, w) =
        axis("domain=0:2, samples=5, xlabel={$x$}, ylabel={$y$}, title={Title}", r"\addplot[red, thick] {x^2}; \addplot+[domain=-1:1, samples=3] {1/x}; \addplot (x, {sqrt(x)});");
    assert!(w.is_empty(), "{w:?}");
    let (c, w) = axis("", "\\addplot table {\nx y\n0 1\n1 3\n2 2\n};");
    assert!(w.is_empty(), "{w:?}");
    // three marks (filled circles) of the first cycle list entry
    assert!(!filled(&c).is_empty());
    let (_, w) = axis("", r"\addplot table[x=b, y expr=\thisrow{a}*2, row sep=\\] {a b \\ 1 2 \\ 3 4 \\};");
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn cycle_list_and_marks() {
    let (c, _) = axis("", r"\addplot coordinates {(0,0) (1,1)}; \addplot coordinates {(0,1) (1,0)};");
    let svg = tikra::driver::svg(&c, "", &Default::default());
    // blue line, red line; marks filled blue!80!black and red!80!black
    assert!(svg.contains("#0000ff") && svg.contains("#ff0000"), "{svg}");
    assert!(svg.contains("#0000cc") && svg.contains("#cc0000"), "{svg}");
    // a plot with its own options does not take marks from the cycle list
    let (c, _) = axis("", r"\addplot[blue] coordinates {(0,0) (1,1)};");
    assert!(filled(&c).is_empty());
    let (c, _) = axis("", r"\addplot[only marks, mark=square*] coordinates {(0,0) (1,1) (2,0)};");
    assert!(!filled(&c).is_empty());
}

#[test]
fn bars_and_closed_cycles() {
    let (c, w) = axis("ybar, ymin=0", r"\addplot coordinates {(0,3) (1,2) (2,4)};");
    assert!(w.is_empty(), "{w:?}");
    let bars = filled(&c);
    assert_eq!(bars.len(), 1, "the bars are one path");
    assert_eq!(bars[0].iter().filter(|s| matches!(s, Segment::Close)).count(), 3);
    let (c, w) = axis("", r"\addplot[fill=blue] {x^2} \closedcycle;");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(filled(&c).len(), 1);
}

#[test]
fn legend_frame_and_positions() {
    let legend_box = |opts: &str| {
        let (c, w) = axis(&format!("width=10cm, height=6cm, xmin=0, xmax=1, ymin=0, ymax=1, {opts}"), r"\addplot coordinates {(0,0) (1,1)}; \addlegendentry{data}");
        assert!(w.is_empty(), "{opts}: {w:?}");
        let frame = c
            .items
            .iter()
            .find_map(|i| match i {
                Item::Path { segments, fill: Some(f), .. } if matches!(f.paint, Paint::Solid(col) if col == tikra::color::Color::WHITE) => Some(points(segments)),
                _ => None,
            })
            .expect("a white legend frame");
        let xs: Vec<f64> = frame.iter().map(|p| p.0).collect();
        let ys: Vec<f64> = frame.iter().map(|p| p.1).collect();
        (
            xs.iter().cloned().fold(f64::INFINITY, f64::min),
            ys.iter().cloned().fold(f64::INFINITY, f64::min),
            xs.iter().cloned().fold(f64::MIN, f64::max),
            ys.iter().cloned().fold(f64::MIN, f64::max),
        )
    };
    let (_, _, right, top) = legend_box("");
    let (left_nw, _, _, _) = legend_box("legend pos=north west");
    let (_, bottom_se, right_se, _) = legend_box("legend pos=south east");
    let (left_out, _, _, _) = legend_box("legend pos=outer north east");
    assert!(left_nw < right / 2.0, "{left_nw}");
    assert!(bottom_se < top / 2.0 && right_se < right, "{bottom_se} {right_se}");
    assert!(left_out > right, "outer north east is outside the box: {left_out} {right}");
    // \legend and legend entries
    let (_, w) = axis("legend entries={a,b}", r"\addplot {x}; \addplot {2*x};");
    assert!(w.is_empty(), "{w:?}");
    let (_, w) = axis("", r"\addplot {x}; \addplot {2*x}; \legend{$x$,$2x$}");
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn log_axes() {
    for env in ["semilogyaxis", "semilogxaxis", "loglogaxis"] {
        let (c, w) = canvas(&format!(r"\begin{{{env}}} \addplot coordinates {{(1,1) (10,100) (100,1000)}}; \end{{{env}}}"));
        assert!(w.is_empty(), "{env}: {w:?}");
        assert!(texts(&c) >= 3, "{env}");
    }
    let (_, w) = axis("ymode=log, xmin=1, xmax=10", r"\addplot[domain=1:10] {exp(x)};");
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn tikz_statements_inside_an_axis() {
    let (c, w) = axis("xmin=0, xmax=10, ymin=0, ymax=10", r"\draw[red] (axis cs:0,0) -- (axis cs:10,10); \node at (rel axis cs:0.5,0.5) {mid};");
    assert!(w.is_empty(), "{w:?}");
    assert!(texts(&c) >= 1);
    // compat=1.11 on: plain coordinates are axis coordinates
    let (_, w) = axis("compat=1.18, xmin=0, xmax=1, ymin=0, ymax=1", r"\draw (0,0) -- (1,1); \coordinate (a) at (0.5,0.5);");
    assert!(w.is_empty(), "{w:?}");
    // named axes and their anchors
    let (_, w) = canvas(r"\begin{axis}[name=a] \addplot {x}; \end{axis} \draw (a.outer north east) -- (a.origin) -- (a.south west);");
    assert!(w.is_empty(), "{w:?}");
}

#[test]
fn degenerate_input_does_not_fail() {
    let (_, w) = axis("", "");
    assert!(w.iter().any(|x| x.contains("without data")), "{w:?}");
    let (_, w) = axis("xmin=2, xmax=2, ymin=1, ymax=1", r"\addplot coordinates {(2,1)};");
    assert!(w.is_empty(), "{w:?}");
    // the step vanishes in precision or the range is huge: no hang
    axis("xmin=1e17, xmax=1.0000000000000001e17, ymin=0, ymax=1", r"\addplot coordinates {(1e17,0)};");
    axis("xmin=0, xmax=1e-300, ymin=0, ymax=1", "");
    axis("ymin=-1e308, ymax=1e308", "");
    let (_, w) = axis("", r"\addplot coordinates;");
    assert!(!w.is_empty());
    let (_, w) = axis("", r"\addplot table {no-such-file.dat};");
    assert!(w.iter().any(|x| x.contains("not found")), "{w:?}");
    let (_, w) = axis("", r"\addplot table {../../etc/passwd};");
    assert!(w.iter().any(|x| x.contains("refused")), "{w:?}");
    // unknown keys are reported
    let (_, w) = axis("no such key=1", r"\addplot {x};");
    assert!(w.iter().any(|x| x.contains("no such key")), "{w:?}");
    // an axis outside tikzpicture is a picture of its own
    let r = tikra::convert(r"\begin{axis} \addplot {x}; \end{axis}").unwrap();
    assert!(r.svg.contains("<path"));
}

#[test]
fn three_d_axis_and_surface() {
    // view {25}{30}: x and y not enlarged, three ticks wanted at least (`every 3d description`)
    let (c, w) = axis("", r"\addplot3[surf,shader=interp] {x*y};");
    assert!(w.is_empty(), "{w:?}");
    // as TeX: x -4, -2, 0, 2, 4 (the longer axis on paper); y -5, 0, 5; z -20, 0, 20
    assert_eq!(texts(&c), 11);
    // 25 × 25 samples: 24 × 24 patches, each shaded as two triangles
    assert_eq!(filled(&c).len(), 2 * 24 * 24);
    // 3d box=complete draws the edges in front too: more lines than the background box alone
    let strokes = |c: &Canvas| c.items.iter().filter(|i| matches!(i, Item::Path { fill: None, stroke: Some(_), .. })).count();
    let (c2, w) = axis("3d box=complete", r"\addplot3[surf,shader=interp] {x*y};");
    assert!(w.is_empty(), "{w:?}");
    assert!(strokes(&c2) > strokes(&c), "{} {}", strokes(&c2), strokes(&c));
}

/// The gray of an axial shading at canvas point `p` (its stops interpolated, padded at the ends).
fn shading_gray(paint: &Paint, p: tikra::scene::Point) -> f64 {
    let Paint::Shading(tikra::canvas::Shading::Axial { transform, stops }) = paint else { panic!("{paint:?}") };
    let t = (transform.inverse().apply(p).y + 50.0) / 100.0;
    let at = |k: usize| (stops[k].0, stops[k].1.r);
    if t <= at(0).0 {
        return at(0).1;
    }
    for k in 1..stops.len() {
        let ((a, ca), (b, cb)) = (at(k - 1), at(k));
        if t <= b {
            return ca + (cb - ca) * (t - a) / (b - a).max(1e-12);
        }
    }
    at(stops.len() - 1).1
}

#[test]
fn interp_shades_each_triangle_smoothly() {
    // a cell of a surface is shaded as two triangles (a lattice shading), its color linear in the
    // point meta between the corners: a gradient on each triangle, however small, not one color
    let body = r"\addplot3[surf,shader=interp,samples=3,domain=0:1,y domain=0:1] {x+2*y};";
    let (c, w) = axis("colormap/blackwhite", body);
    assert!(w.is_empty(), "{w:?}");
    let tris: Vec<(&Vec<Segment>, &Paint)> =
        c.items.iter().filter_map(|i| if let Item::Path { segments, fill: Some(f), .. } = i { Some((segments, &f.paint)) } else { None }).collect();
    assert_eq!(tris.len(), 2 * 2 * 2);
    // at each corner, the gray of its meta: z = x + 2y in 0, 0.5, … 3, grays in steps of 1/6
    let mut grays = Vec::new();
    for (segs, paint) in &tris {
        assert!(matches!(paint, Paint::Shading(_)), "{paint:?}");
        for s in segs.iter() {
            if let Segment::MoveTo(p) | Segment::LineTo(p) = s {
                grays.push(shading_gray(paint, *p));
            }
        }
    }
    for g in &grays {
        assert!((g * 6.0 - (g * 6.0).round()).abs() < 1e-6, "{g} {grays:?}");
    }
    assert!(grays.iter().any(|g| g.abs() < 1e-6) && grays.iter().any(|g| (g - 1.0).abs() < 1e-6), "{grays:?}");
    // drawn without antialiasing, as viewers draw shadings: no seams between the triangles
    assert!(c.items.iter().any(|i| matches!(i, Item::Crisp)));
    let svg = tikra::convert(&format!("\\begin{{tikzpicture}}\\begin{{axis}}[colormap/blackwhite] {body} \\end{{axis}}\\end{{tikzpicture}}")).unwrap().svg;
    assert!(svg.contains(r#"shape-rendering="crispEdges""#));
    // `faceted interp`: the same shading, each patch's edges on top (antialiased, out of the group)
    let (c, _) = axis("colormap/blackwhite", &body.replace("shader=interp", "shader=faceted interp"));
    let kinds: Vec<char> = c
        .items
        .iter()
        .filter_map(|i| match i {
            Item::Path { fill: Some(_), .. } => Some('s'),
            Item::Path { fill: None, stroke: Some(_), segments } if segments.len() == 5 => Some('e'),
            _ => None,
        })
        .collect();
    let s: String = kinds.into_iter().filter(|k| *k == 's' || *k == 'e').collect();
    assert!(s.contains("sse") && s.matches('e').count() >= 4, "{s}");
}

#[test]
fn interp_image_cells_shade_between_their_corners() {
    // `matrix plot, shader=interp`: the corners' metas as pgfplots finds them sweeping the
    // scanlines (pgfplots manual, the matrix with a nan in its middle)
    let metas = [0.0, 1.0, 2.0, 3.0, f64::NAN, 5.0, 6.0, 7.0, 8.0];
    let cells = tikra::engine::pgfplots::mesh::image_interp_metas(&metas, 3, 3, 0.0);
    // the first corner its own; the inner corner after a nan the mean of the four around
    // (nan as the lowest meta); the first row's from its centers; the last column extrapolated
    assert_eq!(cells[0], [0.0, 1.0, 1.0, 3.0]);
    assert_eq!(cells[2], [2.0, -1.0, 2.0, 5.0]);
    assert_eq!(cells[5], [5.0, 2.0, 5.0, 8.0]);
    assert_eq!(cells[6], [6.0, 7.0, 6.0, 5.0]);
    assert_eq!(cells[8], [8.0, 5.0, 8.0, 7.0]);
    // drawn as two shaded triangles each, the nan cell left out
    let (c, w) = axis(
        "colorbar=false",
        r"\addplot [matrix plot, shader=interp, point meta=explicit] coordinates {
        (0,0) [0] (1,0) [1] (2,0) [2]

        (0,1) [3] (1,1) [nan] (2,1) [5]

        (0,2) [6] (1,2) [7] (2,2) [8]
    };",
    );
    assert!(w.is_empty(), "{w:?}");
    let shaded = c.items.iter().filter(|i| matches!(i, Item::Path { fill: Some(f), .. } if matches!(f.paint, Paint::Shading(_)))).count();
    assert_eq!(shaded, 8 * 2);
}

#[test]
fn group_plots() {
    // 2 by 2: four axes side by side and below each other, 1cm apart
    let (c, w) = canvas(
        r"\begin{groupplot}[group style={group size=2 by 2, x descriptions at=edge bottom}, width=4cm, height=4cm, xtick=\empty, ytick=\empty, xlabel=t]
        \nextgroupplot \addplot coordinates {(0,0) (1,1)};
        \nextgroupplot \addplot coordinates {(0,0) (1,1)};
        \nextgroupplot \addplot coordinates {(0,0) (1,1)};
        \nextgroupplot \addplot coordinates {(0,0) (1,1)};
        \end{groupplot}",
    );
    assert!(w.is_empty(), "{w:?}");
    // the x labels of the bottom row only
    assert_eq!(texts(&c), 2);
    let (wd, ht) = size(&c);
    // two axes of 4cm less 45pt each and 1cm between them
    let box_pt = 4.0 / 2.54 * 72.27 - 45.0;
    // (with the empty y label node 35pt left of the first axis, as TeX draws it)
    assert!(wd > 2.0 * box_pt + 28.0 && wd < 2.0 * box_pt + 80.0, "{wd}");
    assert!(ht > 2.0 * box_pt + 28.0, "{ht}");
}

#[test]
fn nodes_near_coords() {
    // one node per point with its y value, above the points and below for negative ones
    let (c, w) = axis("xtick=\\empty, ytick=\\empty, nodes near coords", r"\addplot coordinates {(0,1) (1,-1) (2,0.5)};");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(texts(&c), 3);
    // symbolic point meta is shown as written
    let (c, w) = axis("xtick=\\empty, ytick=\\empty", r"\addplot[nodes near coords, point meta=explicit symbolic] coordinates {(0,1) [a] (1,2) [b]};");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(texts(&c), 2);
    // and the text can be anything, with \coordindex
    let (c, w) = axis("xtick=\\empty, ytick=\\empty", r"\addplot+[ybar, nodes near coords={(\coordindex)}] coordinates {(0,1) (1,2)};");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(texts(&c), 2);
}

#[test]
fn statistics_and_colorbar() {
    // hist: 3 bins of 12 values as ybar intervals
    let (c, w) = axis("xtick=\\empty, ytick=\\empty", r"\addplot+[hist={bins=3}] coordinates {(0,1) (0,2) (0,1) (0,5) (0,4) (0,10) (0,7) (0,10) (0,9) (0,8) (0,9) (0,9)};");
    assert!(w.is_empty(), "{w:?}");
    assert!(!filled(&c).is_empty());
    // a prepared box plot: a box, two whiskers, a median
    let (c, w) =
        axis("xtick=\\empty, ytick=\\empty", r"\addplot[boxplot prepared={lower whisker=1, lower quartile=2, median=3, upper quartile=4, upper whisker=5}] coordinates {};");
    assert!(w.is_empty(), "{w:?}");
    let strokes = c.items.iter().filter(|i| matches!(i, Item::Path { stroke: Some(_), .. })).count();
    assert!(strokes >= 4, "{strokes}");
    // a colorbar is an axis of its own next to the plot
    let (c, w) = axis("colorbar", r"\addplot[mesh] {x};");
    assert!(w.is_empty(), "{w:?}");
    let (wd, _) = size(&c);
    assert!(wd > 260.0, "{wd}");
}

#[test]
fn a_scaled_picture_scales_its_axis_text_and_all() {
    let plain = size(&axis("title=T", r"\addplot coordinates {(0,0) (1,1)};").0);
    let (c, w) = tikra::engine::run_document(r"\begin{tikzpicture}[scale=0.5]\begin{axis}[title=T] \addplot coordinates {(0,0) (1,1)}; \end{axis}\end{tikzpicture}").unwrap();
    assert!(w.is_empty(), "{w:?}");
    let half = size(&c);
    assert!((half.0 - plain.0 / 2.0).abs() < 0.5 && (half.1 - plain.1 / 2.0).abs() < 0.5, "{plain:?} {half:?}");
    // the title's text too
    let scale = |c: &Canvas| c.items.iter().find_map(|i| if let Item::Text { place, .. } = i { Some(place.d) } else { None }).unwrap();
    assert!((scale(&c) - scale(&axis("title=T", r"\addplot coordinates {(0,0) (1,1)};").0) / 2.0).abs() < 1e-9);
}

/// A PNG of `w` × `h` pixels (72 dpi: as many pt) in one color, in a folder of its own that tikra
/// reads files from.
fn png_file(name: &str, w: u32, h: u32, color: &str) -> std::path::PathBuf {
    let svg = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}pt" height="{h}pt" viewBox="0 0 {w} {h}"><rect width="{w}" height="{h}" fill="{color}"/></svg>"#);
    let png = tikra::png(&svg, 72.0, false).unwrap();
    let dir = std::env::temp_dir().join(format!("tikra-graphics-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(name), png).unwrap();
    // `.` too: other tests read (or refuse) files there
    std::env::set_var("TIKRA_INPUT_DIR", format!("{}:.", dir.display()));
    dir
}

fn image(c: &Canvas) -> tikra::scene::Transform {
    let places: Vec<_> = c.items.iter().filter_map(|i| if let Item::Image { place, .. } = i { Some(*place) } else { None }).collect();
    assert_eq!(places.len(), 1);
    places[0]
}

#[test]
fn addplot_graphics_places_a_png() {
    png_file("g2.png", 40, 20, "red");
    // 2d: the image fills xmin..xmax × ymin..ymax, which are the limits
    let (c, w) = axis("scale only axis, width=4cm, height=2cm, enlargelimits=false, hide axis", r"\addplot graphics[xmin=0, xmax=2, ymin=0, ymax=1] {g2.png};");
    assert!(w.is_empty(), "{w:?}");
    let t = image(&c);
    assert!((t.a - 4.0).abs() < 1e-6 && (t.d - 2.0).abs() < 1e-6 && t.b == 0.0 && t.c == 0.0, "{t:?}");
    // the SVG embeds it, and the PNG shows it
    let r = tikra::convert(r"\begin{tikzpicture}\begin{axis}[hide axis] \addplot graphics[xmin=0, xmax=2, ymin=0, ymax=1] {g2.png}; \end{axis}\end{tikzpicture}").unwrap();
    assert!(r.svg.contains(r#"href="data:image/png;base64,iVBORw0KGgo"#), "{}", r.svg);
    let png = resvg::tiny_skia::Pixmap::decode_png(&tikra::png(&r.svg, 72.0, true).unwrap()).unwrap();
    let red = png.pixels().iter().filter(|p| p.red() > 200 && p.green() < 50 && p.blue() < 50).count();
    assert!(red > png.pixels().len() / 2, "{red} of {}", png.pixels().len());

    // 3d: four points with their image coordinates (pt) set the view; the image is placed by the
    // first two, and the others land where the image has them: (0,0,1) at (10,60)
    png_file("g3.png", 100, 100, "blue");
    let points = "points={(0,0,0) => (10,10) (1,0,0) => (70,25) (0,1,0) => (-10,35) (0,0,1) => (10,60)}";
    let (c, w) = axis("hide axis", &format!(r"\addplot3 graphics[{points}] {{g3.png}}; \addplot3[only marks, mark=*] coordinates {{(0,0,1)}};"));
    assert!(w.is_empty(), "{w:?}");
    let t = image(&c);
    let marks = filled(&c);
    assert_eq!(marks.len(), 1);
    let mut b = Canvas::default();
    b.include_path(marks[0]);
    let center = tikra::scene::Point::new((b.min.unwrap().x + b.max.unwrap().x) / 2.0, (b.min.unwrap().y + b.max.unwrap().y) / 2.0);
    // the image is 100 px at 72 dpi: 100.375pt
    let n = 100.0 * 72.27 / 72.0;
    let want = t.apply(tikra::scene::Point::new(10.0 / n, 60.0 / n));
    assert!((want.x - center.x).abs() < 0.01 && (want.y - center.y).abs() < 0.01, "{want:?} {center:?}");

    // a missing file and a PDF are reported
    let (_, w) = axis("", r"\addplot graphics[xmin=0, xmax=1, ymin=0, ymax=1] {no-such-image.png};");
    assert!(w.iter().any(|x| x.contains("not found")), "{w:?}");
    let dir = png_file("g2.png", 40, 20, "red");
    std::fs::write(dir.join("doc.pdf"), "%PDF-1.4").unwrap();
    let (_, w) = axis("", r"\addplot graphics[xmin=0, xmax=1, ymin=0, ymax=1] {doc};");
    assert!(w.iter().any(|x| x.contains("not a PNG")), "{w:?}");
}

#[test]
fn fpu_square_root_decides_the_ticks() {
    // the fpu's sqrt(1) is 0.999985, so x stays below 1 and one tick per unit would leave one
    // label in range: pgfplots asks for four ticks instead and labels x at -0.5, 0, 0.5, 1 as TeX
    let (c, w) = axis("view={60}{30}", r"\addplot3[surf, samples=30, domain=-1:0, y domain=0:2*pi] ({sqrt(1-x^2) * cos(deg(y))}, {sqrt(1-x^2) * sin(deg(y))}, x);");
    assert!(w.is_empty(), "{w:?}");
    // x: -0.5, 0, 0.5, 1; y: -0.5, 0, 0.5; z: -1, -0.5, 0
    assert_eq!(texts(&c), 10);
}

#[test]
fn smith_chart_minor_lines_count_after_the_major_ones() {
    // pgfplots numbers a `minor xtick` list after the whole xtick list, so x = 1, 1.5, 2 are the
    // first, second and third by magnitude: with `xgrid each nth passes y={1:2}` x = 1 and x = 2
    // stop at y = 1 and only x = 1.5 goes round
    let (c, w) = canvas(
        r"\begin{smithchart}[xtick={1,2}, minor xtick={1.5}, ytick={0,1,-1}, xminorgrids, xgrid each nth passes y={1:2}, ygrid each nth passes x={}] \addplot coordinates {(0.5,0.5)}; \end{smithchart}",
    );
    assert!(w.is_empty(), "{w:?}");
    let count = |s: &[Segment], f: fn(&Segment) -> bool| s.iter().filter(|x| f(x)).count();
    let shapes: Vec<(usize, usize)> = c
        .items
        .iter()
        .filter_map(|i| if let Item::Path { segments, stroke: Some(_), .. } = i { Some(segments) } else { None })
        .map(|s| (count(s, |x| matches!(x, Segment::MoveTo(_))), count(s, |x| matches!(x, Segment::CurveTo(..)))))
        .collect();
    // the major x grid: two arcs of 106° and 74° (two and one Bézier pieces), not a full circle
    assert!(shapes.contains(&(2, 3)), "{shapes:?}");
    // the minor x grid: the whole circle x = 1.5
    assert!(shapes.contains(&(1, 4)), "{shapes:?}");
}

#[test]
fn expressions_use_the_exp_of_pgfs_fpu() {
    // the sample at -0.00008 (25 samples of -5:5) has exp 1.00014 in pgf's fpu, not 0.99992:
    // it stays in [1, 15], and the plot has 7 points (x 0 to 2.5), not 6
    let (c, w) = canvas(r"\begin{semilogyaxis}[hide axis] \addplot[mark=none, restrict expr to domain={rawy}{1e0:1.5e1}] {exp(x)}; \end{semilogyaxis}");
    assert!(w.is_empty(), "{w:?}");
    let n: Vec<usize> = c.items.iter().filter_map(|i| if let Item::Path { segments, stroke: Some(_), .. } = i { Some(points(segments).len()) } else { None }).collect();
    assert_eq!(n, vec![7]);
}

#[test]
fn log_ticks_as_pgfplots_computes_them() {
    // 1:100000 in 25 samples ends at ln 11.51257 (pgfplots loops in the fpu's logarithms), so y
    // goes down to ln 1.0007e-10; the ticks are computed in TeX dimensions with 1/ln 10 = 0.4343:
    // the y limits, -25.3277 to 2.3026, are 3 decades apart per tick from -10.9997 decades, so the
    // ticks are 10^-10, 10^-7, 10^-4, 10^-1 (four labels), not 10^-11 … 10^1 (five)
    let (c, w) = canvas(r"\begin{loglogaxis}[small, enlarge x limits={abs=11}, xtick=\empty] \addplot+[domain=1:100000] {x^-2}; \end{loglogaxis}");
    assert!(w.is_empty(), "{w:?}");
    assert_eq!(texts(&c), 4);
}

#[test]
fn transformed_coordinates_in_basic_layer_points() {
    // `\pgfplotstransformcoordinatex` gives the units `\pgfqpointxy` takes in the axis (without
    // `pgfpoint substitution`): the circle is at (1,1) of the axis, as `axis cs:1,1` is
    let (c, w) = axis(
        "xmin=0.5, xmax=2, ymin=0.5, ymax=5",
        r"\pgfplotsset{compat/pgfpoint substitution=1.3}
        \pgfplotsextra{\pgfplotstransformcoordinatex{1}\let\xcoord=\pgfmathresult
            \pgfplotstransformcoordinatey{1}\let\ycoord=\pgfmathresult
            \pgfpathcircle{\pgfqpointxy{\xcoord}{\ycoord}}{5pt}\pgfusepath{fill}}
        \fill (axis cs:1,1) circle[radius=5pt];",
    );
    assert!(w.is_empty(), "{w:?}");
    let f = filled(&c);
    assert_eq!(f.len(), 2);
    let (a, b) = (bounds(f[0]), bounds(f[1]));
    assert!(close_to((a.0.x, a.0.y), (b.0.x, b.0.y)) && close_to((a.1.x, a.1.y), (b.1.x, b.1.y)), "{a:?} {b:?}");
}

#[test]
fn a_declared_function_is_plotted_and_a_failed_plot_says_why() {
    // \pgfmathdeclarefunction's #1 is the argument
    let (c, w) = canvas(r"\pgfmathdeclarefunction{sq}{1}{\pgfmathparse{(#1)^2}} \begin{axis} \addplot[domain=0:2] {sq(x)}; \end{axis}");
    assert!(w.is_empty(), "{w:?}");
    assert!(c.items.iter().any(|i| matches!(i, Item::Path { segments, .. } if segments.len() > 20)));
    // a plot none of whose points can be computed is not dropped in silence
    let (_, w) = axis("", r"\addplot[domain=0:2] {nosuchfunction(x)};");
    assert!(w.iter().any(|w| w.contains("no point of the plot")), "{w:?}");
}
