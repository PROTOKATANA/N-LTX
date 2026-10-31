//! Whole figures, made up for these tests, that combine several features at once.

mod common;
use common::*;
use tikra::scene::Segment;

#[test]
fn number_line_with_two_variable_foreach() {
    let r = no_warnings(
        r"\begin{tikzpicture}
        \draw[->,thick] (0,0) -- (10,0);
        \foreach \pos/\name in {1/start, 3.5/middle, 6/end}{
          \draw[thick] (\pos,-0.15) -- (\pos,0.15);
          \node[above] at (\pos,0.15) {\name};
        }
      \end{tikzpicture}",
    );
    assert!(r.svg.contains("<svg"));
    let c = canvas(r"\foreach \x/\n in {1/a, 2/b, 3/c} { \node at (\x,0) {\n}; }");
    assert_eq!(texts(&c).len(), 3);
}

#[test]
fn folding_fan_with_def_rotated_scope_and_arc() {
    no_warnings(
        r"\begin{tikzpicture}
        \def\opening{40} \def\radius{5}
        \coordinate (O) at (0,0);
        \coordinate (T) at ({\radius*cos(\opening)}, {\radius*sin(\opening)});
        \draw[thick, fill=orange!20] (O) -- (\radius,0) -- (T) -- cycle;
        \draw (1.2,0) arc (0:\opening:1.2);
        \begin{scope}[rotate=\opening, shift={(O)}]
          \draw[thick, fill=teal!50] (2,0) rectangle (2.8,0.5);
          \draw[decoration={zigzag, amplitude=2pt, segment length=4pt}, decorate] (2.8,0.25) -- (4.2,0.25);
        \end{scope}
      \end{tikzpicture}",
    );
}

#[test]
fn rectangle_rotates_with_the_scope() {
    let t = first_path(r"\begin{scope}[rotate=90] \draw (0,0) rectangle (2,1); \end{scope}");
    // rotated 90°: the corner (2,0) goes to (0,2)
    assert!(t.segments.iter().any(|s| matches!(s, Segment::LineTo(p) if (p.x).abs() < 1e-9 && (p.y - 2.0).abs() < 1e-9)), "{:?}", t.segments);
}

#[test]
fn flowchart_with_positioning_and_elbows() {
    let r = no_warnings(
        r"\begin{tikzpicture}[node distance=1.3cm and 1cm,
        step/.style={rectangle, draw, rounded corners, text centered, text width=3cm, minimum height=1cm},
        arrow/.style={-Stealth, thick}]
        \node[step, fill=blue!10] (a) {Start};
        \node[step, right=2cm of a] (b) {Read the input};
        \node[step, below=of a.south east, xshift=1cm, font=\bfseries] (c) {Check every value twice};
        \node[diamond, draw, aspect=2, below left=1.2cm and -0.2cm of c] (d) {Valid?};
        \draw[arrow] (a.south) |- (c.north west);
        \draw[arrow] (c.south) -| (d.north);
      \end{tikzpicture}",
    );
    // the Stealth tips: closed paths, filled and stroked with the line's width, as pgf does
    assert!(r.svg.contains(r##"Z" fill="#000000" stroke="#000000""##), "the Stealth tip is drawn");
}

#[test]
fn line_between_nodes_stops_at_the_border() {
    let c = canvas(r"\node[draw, minimum width=2cm, minimum height=1cm] (a) at (0,0) {}; \node[draw, circle, minimum size=1cm] (b) at (5,0) {}; \draw (a) -- (b);");
    // the line is the last path, after the two node borders
    let t = paths(&c).pop().unwrap();
    let (Segment::MoveTo(p), Segment::LineTo(q)) = (t.segments[0], t.segments[1]) else { panic!() };
    // the borders are pushed out by the outer sep, half the line width (0.2pt), as in TikZ
    let outer = 0.2 * tikra::PT;
    assert!((p.x - (1.0 + outer)).abs() < 1e-6, "leaves from the rectangle's right border: {p:?}");
    assert!((q.x - (4.5 - outer)).abs() < 1e-6, "arrives at the circle's left border: {q:?}");
}

#[test]
fn stacked_waves_with_declared_function_and_plot() {
    no_warnings(
        r"\begin{tikzpicture}[x=4cm, y=0.7cm, declare function={level(\n) = 2*\n; wave(\n,\x) = sin(\n*360*\x);}]
        \draw[->, thick] (-0.2,0) -- (1.5,0) node[right] {$t$};
        \foreach \n in {1, 2, 3} {
          \pgfmathsetmacro{\height}{level(\n)}
          \draw[blue, dashed] (0, \height) -- (1, \height);
          \node[left=4pt] at (0, \height) {$a_{\n}$};
          \draw[domain=0:1, samples=50, variable=\x, thick, violet] plot (\x, {\height + 0.6*wave(\n,\x)});
        }
      \end{tikzpicture}",
    );
}

#[test]
fn signpost_with_styles_hatching_and_inherited_color() {
    let r = no_warnings(
        r"\documentclass[tikz,border=10pt]{standalone}
      \usetikzlibrary{patterns, arrows.meta}
      \begin{document}
      \begin{tikzpicture}[scale=1.5, >=Latex]
        \tikzset{pointer/.style={->, very thick, color=red}, ground/.style={pattern=north east lines, draw=black, thick}}
        \def\r{0.4}
        \draw[ground] (-2, 0) rectangle (2, -0.2);
        \draw[thick, fill=gray!20] (0,3) circle (\r);
        \draw (-\r, 3) arc (180:360:\r);
        \node[draw, minimum width=1.2cm, minimum height=1cm, fill=blue!10] at (0,1.5) {\textbf{Exit}};
        \draw[pointer] (0.8,1.5) -- ++(1,0) node[right] {$\vec{v}$};
      \end{tikzpicture}
      \end{document}",
    );
    assert!(r.svg.contains("<pattern"), "hatching");
    // the vector's label inherits the arrow's color
    assert!(r.svg.matches(r##"fill="#ff0000""##).count() >= 2, "{}", r.svg.matches("ff0000").count());
}

#[test]
fn pgfplots_plot_with_legend() {
    no_warnings(
        r"\begin{tikzpicture}\begin{axis}[width=11cm, height=6cm, xmin=0, xmax=1, ymin=0, ymax=3, axis lines=left, xlabel={$x$}, ylabel={$f(x)$}, samples=100]
        \addplot[domain=0:1, thick, blue] {1 + x};
        \addlegendentry{Linear}
        \addplot[domain=0:1, thick, red] {2.5*exp(-3*x) + sqrt(x)/(2*pi)};
        \addlegendentry{Decay}
      \end{axis}\end{tikzpicture}",
    );
}

#[test]
fn node_shapes() {
    let c = canvas(r"\node[circle, draw] (a) {A}; \node[ellipse, draw] (b) at (2,0) {B}; \node[diamond, draw] (c) at (4,0) {C}; \coordinate (d) at (6,0);");
    // three borders and nothing for the coordinate
    let borders = paths(&c);
    assert_eq!(borders.len(), 3, "{borders:?}");
    let curves = |d: &Drawn| d.segments.iter().filter(|s| matches!(s, Segment::CurveTo(..))).count();
    let lines = |d: &Drawn| d.segments.iter().filter(|s| matches!(s, Segment::LineTo(..))).count();
    let extent = |d: &Drawn| {
        let (a, b) = bounds(&d.segments);
        (b.x - a.x, b.y - a.y)
    };
    // the circle: curves, as wide as it is high
    let (w, h) = extent(&borders[0]);
    assert!(curves(&borders[0]) >= 4 && (w - h).abs() < 1e-6, "circle: {w} × {h}");
    // the ellipse: curves, fitted to the text box (not a circle: width and height differ)
    let (w, h) = extent(&borders[1]);
    assert!(curves(&borders[1]) >= 4 && (w - h).abs() > 1e-3, "ellipse: {w} × {h}");
    // the diamond: four straight sides
    assert!(curves(&borders[2]) == 0 && lines(&borders[2]) >= 3, "diamond: {:?}", borders[2].segments);
    // the diamond's corners are on its axes: top and bottom at the center's x
    let (a, b) = bounds(&borders[2].segments);
    let top = points(&borders[2].segments).into_iter().find(|p| close(p.1, b.y)).unwrap();
    assert!(close(top.0, (a.x + b.x) / 2.0), "{top:?}");
}

#[test]
fn input_repairs() {
    // escaped for Markdown and with doubled backslashes
    let r = figure("```\n\\\\draw\\[red\\] (0,0) -- (1,1);\n```");
    assert!(r.warnings.iter().any(|a| a.contains("Markdown")), "{:?}", r.warnings);
    // an unknown command becomes a warning, not an error
    let r = figure(r"\begin{tikzpicture} \draw (0,0) -- (1,0); \spy on (0,0) in node at (1,1); \end{tikzpicture}");
    assert!(r.warnings.iter().any(|a| a.contains("spy")), "{:?}", r.warnings);
}

#[test]
fn png_has_the_tex_size() {
    // 10 cm at 110 dpi ≈ 433 px, plus the margin
    let r = no_warnings(r"\begin{tikzpicture}\draw (0,0) -- (10,0);\end{tikzpicture}");
    let png = tikra::png(&r.svg, 110.0, true).unwrap();
    let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    assert!((430..=450).contains(&width), "{width}");
}

#[test]
fn same_figure_same_bytes() {
    let t = r"\begin{tikzpicture}\draw[decoration={random steps, segment length=3pt, amplitude=1pt}, decorate] (0,0) -- (2,2);\end{tikzpicture}";
    assert_eq!(figure(t).svg, figure(t).svg);
}
