//! The generated SVG (bounds, labels, clips, opacities) and PNG rasterization.

mod common;
use common::*;
use tikra::canvas::{Canvas, Item};
use tikra::color::Color;
use tikra::scene::{Label, Transform};

fn png_width(png: &[u8]) -> u32 {
    u32::from_be_bytes([png[16], png[17], png[18], png[19]])
}

#[test]
fn svg_with_opacities_dashes_and_drawn_nodes() {
    let r = no_warnings(
        r"\begin{tikzpicture}
            \filldraw[fill opacity=0.4, draw opacity=0.6, dashed] (0,0) rectangle (1,1);
            \node[draw, fill=yellow, dashed, rounded corners] at (3,0) {box};
            \node[draw, circle, fill=red] at (5,0) {c};
            \node[draw, diamond] at (7,0) {d};
            \node[draw, rounded corners=1mm, minimum size=1cm] at (9,0) {};
          \end{tikzpicture}",
    );
    for fragment in ["fill-opacity=\"0.4\"", "stroke-opacity=\"0.6\"", "stroke-dasharray", "#fff200", "<use href=\"#tr-g"] {
        assert!(r.svg.contains(fragment), "{fragment}");
    }
}

#[test]
fn multiline_label_and_alignments() {
    for a in ["left", "right", "center"] {
        let r = no_warnings(&format!(r"\begin{{tikzpicture}} \node[align={a}] {{line one\\ two}}; \end{{tikzpicture}}"));
        assert!(r.svg.contains("<g transform="), "{a}");
    }
    // math with a fraction and a root: mathra's rules and lines
    let r = no_warnings(r"\begin{tikzpicture} \node {$\frac{a}{b} + \sqrt{x} + \overline{y}$}; \end{tikzpicture}");
    assert!(r.svg.contains("<rect"));
}

#[test]
fn mathra_items_with_color_and_ellipse() {
    let r = figure(
        r"\begin{tikzpicture} \node {$\textcolor{red}{x} + \cancel{y} + \boxed{z} + \fcolorbox{red}{yellow}{w}$}; \node at (0,1) {$\bigcirc$}; \node at (0,2) {$\textcolor{red}{\frac{a}{b}}$}; \end{tikzpicture}",
    );
    assert!(r.svg.contains("<svg"));
}

#[test]
fn empty_canvas_has_minimum_size() {
    let svg = tikra::driver::svg(&Canvas::default(), "", &Default::default());
    assert!(svg.contains("viewBox"));
    // a text whose label has no lines
    let mut c = Canvas::default();
    let label = Label { lines: Vec::new(), scale: 1.0, color: Color::BLACK, align: 'c', line_height: 0.0, stretch: 1.0, skips: Vec::new(), font: tikra::label::FONT, lift: 0.0 };
    assert_eq!(label.depth(), 0.0);
    assert_eq!(label.height(), 0.0);
    c.push(Item::Text { label, place: Transform::default(), opacity: 1.0 });
    assert!(tikra::driver::svg(&c, "", &Default::default()).contains("viewBox"));
}

#[test]
fn nested_clips() {
    let r = no_warnings(r"\begin{tikzpicture} \begin{scope} \clip (0,0) circle (1); \clip (0,0) rectangle (2,2); \fill (0,0) rectangle (3,3); \end{scope} \end{tikzpicture}");
    assert!(r.svg.contains("tr-c2"));
    assert_eq!(r.svg.matches("<g clip-path").count(), 2);
}

#[test]
fn png_with_and_without_background_and_dpi() {
    let r = no_warnings(r"\begin{tikzpicture} \draw (0,0) -- (2.54,0); \end{tikzpicture}");
    let a = tikra::png(&r.svg, 96.0, true).unwrap();
    let b = tikra::png(&r.svg, 192.0, false).unwrap();
    assert!(png_width(&b) > png_width(&a) * 19 / 10);
    assert_ne!(a, b);
}

#[test]
fn png_rejects_invalid_svg_and_bad_dpi() {
    assert!(matches!(tikra::png("this is not svg", 300.0, true), Err(tikra::Error::Png(_))));
    let r = no_warnings(r"\begin{tikzpicture} \draw (0,0) -- (1,0); \end{tikzpicture}");
    for dpi in [0.0, -10.0, f32::NAN, f32::INFINITY] {
        let e = tikra::png(&r.svg, dpi, true).unwrap_err();
        assert!(e.to_string().contains("invalid dpi"), "{dpi}: {e}");
    }
    // a tiny dpi: the image vanishes
    assert!(tikra::png(&r.svg, 0.001, true).is_err());
    // a huge dpi: goes past the pixel cap instead of allocating gigabytes
    let e = tikra::png(&r.svg, 1e6, true).unwrap_err();
    assert!(e.to_string().contains("too large"), "{e}");
}

#[test]
fn a_striped_pattern_has_no_seams_between_its_tiles() {
    // horizontal stripes: along a row every pixel has the stripe's color, none lighter where tiles meet
    let r = no_warnings(r"\tikz \fill[pattern=horizontal lines light blue] (0,0) rectangle (2,1);");
    let png = tikra::png(&r.svg, 100.0, true).unwrap();
    let image = resvg::tiny_skia::Pixmap::decode_png(&png).unwrap();
    let row: Vec<u8> = (10..60).map(|x| image.pixel(x, 20).unwrap().red()).collect();
    assert!(row.iter().all(|&v| v == row[0]), "{row:?}");
}

#[test]
fn a_filled_and_stroked_path_at_one_opacity_is_one_group() {
    // as PDF paints fill and stroke: the stroke does not darken the fill under it
    let svg = no_warnings(r"\tikz \filldraw[fill=red, opacity=0.5, line width=2mm] (0,0) rectangle (1,1);").svg;
    assert!(svg.contains(r#" opacity="0.5""#) && !svg.contains("fill-opacity") && !svg.contains("stroke-opacity"), "{svg}");
    // different opacities stay apart
    let svg = no_warnings(r"\tikz \filldraw[fill=red, fill opacity=0.5, draw opacity=0.3] (0,0) rectangle (1,1);").svg;
    assert!(svg.contains("fill-opacity") && svg.contains("stroke-opacity"), "{svg}");
}

#[test]
fn a_stroke_that_lets_through_shows_what_is_under_the_path_not_its_fill() {
    // PDF paints fill and stroke as a knockout group: under a translucent stroke, the fill is not there
    let out = no_warnings(r"\tikz \filldraw[fill=yellow, draw opacity=0.5, line width=4mm] (0,0) rectangle (2,2);");
    assert!(out.svg.contains("<mask") && out.svg.contains("stroke-opacity"), "{}", out.svg);
    let image = resvg::tiny_skia::Pixmap::decode_png(&tikra::png(&out.svg, 100.0, true).unwrap()).unwrap();
    // the stroke's inner half (over the fill) is the same gray as its outer half (over the white)
    let (inner, outer) = (image.pixel(10, 40).unwrap(), image.pixel(5, 40).unwrap());
    assert_eq!((inner.red(), inner.green(), inner.blue()), (outer.red(), outer.green(), outer.blue()));
    assert_eq!(inner.blue(), inner.red());
}

#[test]
fn the_lines_of_a_label_take_its_color() {
    // a molecule's bonds are lines in currentColor: the label sets color, not only fill
    let svg = no_warnings(r"\tikz \node[text=red] {$\chemfig{H_3C-OH}$};").svg;
    assert!(svg.contains(r##"fill="#ff0000" color="#ff0000""##) && svg.contains(r#"stroke="currentColor""#), "{svg}");
}

#[test]
fn construction_animation() {
    let code = r"\begin{tikzpicture} \draw[->] (0,0) -- (1,1); \draw[dashed] (0,1) -- (1,0); \fill (2,0) circle (2pt); \node at (3,0) {$x$}; \end{tikzpicture}";
    let still = tikra::convert(code).unwrap().svg;
    assert!(!still.contains("<style>") && !still.contains("class=\"tr-"));
    let svg = tikra::convert_with(code, &tikra::Options::default().animated()).unwrap().svg;
    assert!(svg.contains("<style>@keyframes tr-d"));
    // five steps (the arrow tip is one); the solid line is drawn along its length, the dashed one,
    // the dot and the label fade in
    assert!(svg.contains(r#"data-steps="5""#) && svg.contains(r#"data-step="0" class="tr-d" pathLength="1" style="--i:0""#));
    assert!(!svg.split("stroke-dasharray=").nth(1).unwrap().split("/>").next().unwrap().contains("tr-d"));
    assert!(svg.contains(r#"<g transform="#) && svg.matches(r#"class="tr-f""#).count() >= 3);
    // the drawing is the same: only the attributes of the steps and the style are added
    let svg_only = svg.replace(&svg[svg.find("<style>").unwrap()..svg.find("</style>").unwrap() + 8], "");
    let plain = without_steps(&svg_only);
    assert_eq!(plain, still);
    // a PNG of the animated SVG is the still figure
    assert_eq!(tikra::png(&svg, 72.0, true).unwrap(), tikra::png(&still, 72.0, true).unwrap());
}

#[test]
fn steps_without_the_animation() {
    let code = r"\begin{tikzpicture} \draw (0,0) -- (1,1); \fill (2,0) circle (2pt); \node at (3,0) {$x$}; \end{tikzpicture}";
    let still = tikra::convert(code).unwrap().svg;
    let svg = tikra::convert_with(code, &tikra::Options::default().steps()).unwrap().svg;
    // numbered in the order drawn, counted on the <svg>, and no CSS
    assert!(svg.contains(r#"data-steps="3""#) && svg.contains(r#"data-step="0""#) && svg.contains(r#"data-step="2""#), "{svg}");
    assert!(!svg.contains("<style>") && !svg.contains("class=\"tr-"));
    assert_eq!(without_steps(&svg), still);
}

/// The SVG without the steps' attributes.
fn without_steps(svg: &str) -> String {
    let mut s = svg.replace(r#" class="tr-d" pathLength="1""#, "").replace(r#" class="tr-f""#, "");
    for attr in [r#" style="--i:"#, r#" data-step=""#, r#" data-steps=""#] {
        while let Some(i) = s.find(attr) {
            let end = i + attr.len() + s[i + attr.len()..].find('"').unwrap() + 1;
            s.replace_range(i..end, "");
        }
    }
    s
}

#[test]
fn fills_side_by_side_leave_no_seam_between_them() {
    // two squares meeting inside a pixel: the white under them does not show along their edge
    let out = no_warnings(r"\tikz[x=0.25cm] { \fill[red] (0,0) rectangle (1,1); \fill[red] (1,0) rectangle (2,1); }");
    let image = resvg::tiny_skia::Pixmap::decode_png(&tikra::png(&out.svg, 100.0, true).unwrap()).unwrap();
    let row: Vec<u8> = (3..image.width() - 3).map(|x| image.pixel(x, image.height() / 2).unwrap().green()).collect();
    assert!(row.iter().all(|&g| g < 10), "{row:?}");
    // a fill folded back over another one gets no stroke: it would stick out of both
    let svg = no_warnings(r"\tikz { \fill[red] (0,0) -- (1,0) -- (1,1) -- cycle; \fill[blue] (0,0) -- (1,0) -- (0.5,0.5) -- cycle; }").svg;
    assert!(!svg.contains("<g><path"), "{svg}");
}
