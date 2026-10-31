//! The figure in words: the SVG's `<title>` (`Output::alt`) and `<desc>` (`Output::description`),
//! for readers who cannot see it.

fn words(body: &str) -> (String, String) {
    let r = tikra::convert(&format!("\\begin{{tikzpicture}}{body}\\end{{tikzpicture}}")).unwrap();
    (r.alt, r.description)
}

/// The SVG with the words' ids without their figure's suffix (`tr-title-1a2b3c4d` is `tr-title`).
fn plain_ids(svg: &str) -> String {
    let mut s = svg.to_string();
    for id in ["tr-title-", "tr-desc-"] {
        while let Some(i) = s.find(id) {
            s.replace_range(i + id.len() - 1..i + id.len() + 8, "");
        }
    }
    s
}

fn axis(options: &str, body: &str) -> (String, String) {
    words(&format!("\\begin{{axis}}[{options}]{body}\\end{{axis}}"))
}

#[test]
fn the_words_are_in_the_svg() {
    let r = tikra::convert(r"\tikz \node {a < b};").unwrap();
    let svg = plain_ids(&r.svg);
    assert!(svg.contains(r#"role="img" aria-labelledby="tr-title" aria-describedby="tr-desc">"#));
    assert!(svg.contains("<title id=\"tr-title\">Drawing with the text “a &lt; b”</title><desc id=\"tr-desc\">Text in the figure: a &lt; b.</desc>"));
}

#[test]
fn each_figure_has_its_own_ids() {
    // many figures on a page: each labeled by its own words, the same figure by the same ids
    let id = |code: &str| {
        let svg = tikra::convert(code).unwrap().svg;
        let i = svg.find("<title id=\"").unwrap() + 11;
        svg[i..i + 17].to_string()
    };
    assert_eq!(id(r"\tikz \node {a};"), id(r"\tikz \node {a};"));
    assert_ne!(id(r"\tikz \node {a};"), id(r"\tikz \node {b};"));
    assert!(id(r"\tikz \node {a};").starts_with("tr-title-"));
}

#[test]
fn drawings_without_text() {
    assert_eq!(words(r"\path (0,0) -- (1,0);").0, "Empty figure");
    assert_eq!(words(r"\draw (0,0) circle (1);"), ("Drawing of one shape, without text".into(), "Drawing of one shape, without text.".into()));
    assert_eq!(words(r"\draw (0,0) circle (1); \fill (2,0) rectangle (3,1);").0, "Drawing of 2 shapes, without text");
}

#[test]
fn text_is_read_as_spoken() {
    let (alt, desc) = words(r"\draw (0,0) -- (1,0); \node at (0,1) {\textbf{Area} $\pi r^2$}; \node at (2,0) {1}; \node at (3,0) {2}; \node at (4,0) {3}; \node at (5,0) {4};");
    // words come before numbers in the alt text
    assert_eq!(alt, "Drawing of one shape with the text “Area pi r squared”, …");
    assert_eq!(desc, "Text in the figure: Area pi r squared; 1; 2; 3; 4.");
    assert_eq!(words(r"\node {1}; \node at (1,0) {2}; \node at (2,0) {3}; \node at (3,0) {4};").0, "Drawing with the text “1”, “2”, “3”, …");
}

#[test]
fn a_flowchart_tells_its_connections_and_their_labels() {
    let (alt, desc) = words(
        r"\node[draw] (s) {Start}; \node[draw] (q) at (0,-1) {$n > 0$?}; \node[draw] (y) at (2,-1) {Print ``yes''};
          \draw[->] (s) -- (q); \draw[->] (q) -- node[above] {yes} (y); \draw[<-] (s.east) -| (y.north);",
    );
    assert_eq!(alt, "Diagram with 3 nodes and 3 connections");
    assert_eq!(
        desc,
        "Diagram with 3 nodes and 3 connections. Nodes: Start; n is greater than 0?; Print “yes”. \
         Connections: from Start to n is greater than 0?; from n is greater than 0? to Print “yes”, labeled “yes”; from Print “yes” to Start."
    );
}

#[test]
fn connections_by_their_ends() {
    // both tips, none, a loop, an edge with a quoted label, through a coordinate
    let (_, desc) = words(
        r#"\node (a) {A}; \node (b) at (2,0) {B}; \coordinate (c) at (1,1);
           \draw[<->] (a) -- (b); \draw (a) to[bend left] (b); \draw[->] (a) to[loop above] (a);
           \path (a) edge[->, "go"] (b); \draw (a) -- (c) -- (b);"#,
    );
    assert_eq!(
        desc,
        "Diagram with 2 nodes and 4 connections. Nodes: A; B. \
         Connections: both ways between A and B; between A and B; from A back to itself; from A to B, labeled “go”."
    );
}

#[test]
fn a_path_that_is_not_drawn_joins_nothing() {
    // its label is a node like any other
    let (alt, desc) = words(r"\node (a) {A}; \node (b) at (2,0) {B}; \path (a) -- node {mid} (b);");
    assert_eq!(alt, "Drawing with the text “A”, “B”, “mid”");
    assert_eq!(desc, "Text in the figure: A; B; mid.");
}

#[test]
fn nodes_without_text_are_told_by_their_names() {
    let (alt, desc) = words(r"\node[circle,fill] (v1) {}; \node[circle,fill] at (1,0) {}; \node[circle,fill] (v2) at (2,0) {}; \draw (v1) -- (v2) (v2) -- (0,1) -- (v1);");
    assert_eq!(alt, "Diagram with 2 nodes and 2 connections");
    assert_eq!(desc, "Diagram with 2 nodes and 2 connections. Connections: between v1 and v2; between v2 and v1.");
}

#[test]
fn trees_and_mindmaps() {
    let (_, desc) = words(r"\node {root} child {node {left}} child {node {right} child {node {leaf}}};");
    assert_eq!(desc, "Diagram with 4 nodes and 3 connections. Nodes: root; left; right; leaf. Connections: between root and left; between right and leaf; between root and right.");
    let (alt, _) = words(r"\path[mindmap, concept color=blue!30] node[concept] {idea} child {node[concept] {one}} child {node[concept] {two}};");
    assert_eq!(alt, "Diagram with 3 nodes and 2 connections");
}

#[test]
fn long_lists_are_cut() {
    let body: String =
        (0..70).map(|i| format!(r"\node (n{i}) at ({i},0) {{n{i}}};")).collect::<String>() + &(1..70).map(|i| format!(r"\draw (n{}) -- (n{i});", i - 1)).collect::<String>();
    let (alt, desc) = words(&body);
    assert_eq!(alt, "Diagram with 70 nodes and 69 connections");
    assert!(desc.contains("n39; and 30 more."), "{desc}");
    assert!(desc.ends_with("between n59 and n60; and 9 more."), "{desc}");
}

#[test]
fn a_bar_chart_with_categories() {
    let (alt, desc) =
        axis("ybar, title={Sales}, symbolic x coords={Mon,Tue,Wed}, xtick=data, ylabel={units}, legend entries={2024}", r"\addplot coordinates {(Mon,3) (Tue,5) (Wed,7.5)};");
    assert_eq!(alt, "Bar chart: Sales");
    assert_eq!(
        desc,
        "Bar chart titled “Sales”. Horizontal axis, with the categories Mon, Tue, Wed. Vertical axis “units”, from 0 to 7.5. “2024”: 3 bars — Mon: 3, Tue: 5, Wed: 7.5."
    );
}

#[test]
fn horizontal_bars_and_histograms() {
    let (alt, desc) = axis("xbar, symbolic y coords={a,b}, ytick=data", r"\addplot coordinates {(4,a) (6,b)};");
    assert_eq!(alt, "Horizontal bar chart");
    assert!(desc.ends_with("2 bars — a: 4, b: 6."), "{desc}");
    let (alt, desc) = axis("ybar interval, xlabel=ms, ylabel=count", r"\addplot coordinates {(0,3) (10,6) (20,1) (30,1)};");
    assert_eq!(alt, "Histogram of count against ms");
    assert!(desc.ends_with("3 bins — 0 to 10: 3; 10 to 20: 6; 20 to 30: 1."), "{desc}");
}

#[test]
fn a_function_plot_tells_its_formula_and_extremes() {
    let (alt, desc) = axis("xlabel=$x$, ylabel=$y$, domain=-2:2", r"\addplot {x^2};");
    assert_eq!(alt, "Line chart of y against x");
    assert_eq!(
        desc,
        "Line chart. Horizontal axis “x”, from -2 to 2. Vertical axis “y”, from 0 to 4. y = x^2, for x from -2 to 2; y is lowest, 0, at x = 0, and highest, 4, at x = -2."
    );
    let (_, desc) = axis("domain=0:1", r"\addplot {3};");
    assert!(desc.ends_with("; y is 3 throughout."), "{desc}");
}

#[test]
fn many_points_are_summed_up() {
    let points: String = (0..20).map(|i| format!("({i},{})", (i * 7) % 11)).collect();
    let (alt, desc) = axis("only marks", &format!(r"\addplot coordinates {{{points}}};"));
    assert_eq!(alt, "Scatter plot");
    assert!(desc.ends_with("20 points, x from 0 to 19; y is lowest, 0, at x = 0, and highest, 10, at x = 3."), "{desc}");
}

#[test]
fn several_series_with_their_names() {
    let (alt, desc) = axis(
        "title=Mixed",
        r"\addplot coordinates {(0,1) (1,2)}; \addlegendentry{up} \addplot+[only marks] coordinates {(0,2) (1,1)}; \addplot coordinates {(0,0) (1,0)} \closedcycle;",
    );
    assert_eq!(alt, "Chart: Mixed");
    assert!(desc.contains("3 series. “up”, line: 2 points — (0, 1), (1, 2). Series 2, scatter: 2 points — (0, 2), (1, 1). Series 3, area: "), "{desc}");
}

#[test]
fn surfaces_tell_their_colors() {
    let (alt, desc) = words(r"\begin{axis}[view={30}{30}, samples=5, domain=0:1] \addplot3[surf] {x*y}; \end{axis}");
    assert_eq!(alt, "3D surface plot");
    assert_eq!(
        desc,
        "3D surface plot. x axis, from 0 to 1. y axis, from 0 to 1. z axis, from 0 to 1. Colors stand for values from 0 to 1. \
         z = x*y, for x from 0 to 1 and y from 0 to 1; z is lowest, 0, at x = 0, y = 0, and highest, 1, at x = 1, y = 1."
    );
    let (alt, desc) = words(r"\begin{axis}[view={30}{30}] \addplot3[mesh] coordinates {(0,0,1) (1,0,2) (0,1,3) (1,1,4)}; \end{axis}");
    assert_eq!(alt, "3D surface plot");
    assert!(desc.ends_with("4 points — (0, 0, 1), (1, 0, 2), (0, 1, 3), (1, 1, 4)."), "{desc}");
    let (alt, _) = words(r"\begin{axis}[view={30}{30}] \addplot3 coordinates {(0,0,1) (1,1,2)}; \end{axis}");
    assert_eq!(alt, "3D line plot");
}

#[test]
fn other_kinds_of_plots() {
    let kind = |options: &str, plot: &str| axis(options, plot).0;
    assert_eq!(kind("", r"\addplot+[ycomb] coordinates {(0,1) (1,2)};"), "Stem plot");
    assert_eq!(kind("", r"\addplot[quiver={u=1,v=1}] coordinates {(0,1) (1,2)};"), "Vector field");
    assert_eq!(kind("", r"\addplot[contour prepared] coordinates {(0,0,1) (1,1,1)};"), "Contour plot");
    assert_eq!(kind("", r"\addplot[fill] coordinates {(0,0) (1,1) (2,0)} \closedcycle;"), "Area chart");
    let (alt, desc) = axis("", r"\addplot[boxplot prepared={median=3, lower quartile=2, upper quartile=4, lower whisker=1, upper whisker=5}] coordinates {(0,7)};");
    assert_eq!(alt, "Box plot");
    assert!(desc.ends_with("median 3, quartiles 2 and 4, whiskers from 1 to 5, one outlier."), "{desc}");
}

#[test]
fn log_polar_ternary_and_smith_axes() {
    let (_, desc) = words(r"\begin{loglogaxis} \addplot coordinates {(1,10) (100,1000)}; \end{loglogaxis}");
    assert!(desc.contains("Horizontal axis, from 1 to 100, on a logarithmic scale."), "{desc}");
    let (alt, desc) = words(r"\begin{polaraxis} \addplot coordinates {(0,1) (90,2)}; \end{polaraxis}");
    assert_eq!(alt, "Polar plot");
    assert!(desc.starts_with("Polar plot. Angle, from 0 to 90 degrees. Radius, from"), "{desc}");
    let (alt, desc) = words(r"\begin{ternaryaxis} \addplot3 coordinates {(0.2,0.3,0.5)}; \end{ternaryaxis}");
    assert_eq!(alt, "Ternary plot");
    assert!(!desc.contains("axis"), "{desc}");
    assert_eq!(words(r"\begin{smithchart} \addplot coordinates {(0.5,0.5)}; \end{smithchart}").0, "Smith chart");
}

#[test]
fn an_axis_without_data_and_text_around_it() {
    let (alt, desc) = words(r"\begin{axis}[title={Empty}] \node at (axis cs:0.5,0.5) {inside}; \end{axis} \node at (8,0) {outside};");
    assert_eq!(alt, "Chart: Empty");
    // the axis' ticks are not the figure's text; what its body and the picture draw is
    assert_eq!(desc, "Chart titled “Empty”. No data. Other text: inside; outside.");
}

#[test]
fn several_axes() {
    let (alt, desc) =
        words(r"\begin{axis}[name=a] \addplot coordinates {(0,1) (1,2)}; \end{axis} \begin{axis}[at={(a.south east)}, xshift=1cm] \addplot coordinates {(0,1) (1,2)}; \end{axis}");
    assert_eq!(alt, "2 plots: Line chart");
    assert!(desc.starts_with("Plot 1 of 2: Line chart.") && desc.contains("Plot 2 of 2: Line chart."), "{desc}");
    let (alt, _) = words(
        r"\begin{axis}[name=a, ybar] \addplot coordinates {(0,1) (1,2)}; \end{axis} \begin{axis}[at={(a.south east)}, xshift=1cm] \addplot coordinates {(0,1) (1,2)}; \end{axis}",
    );
    assert_eq!(alt, "2 plots");
}

#[test]
fn data_visualizations_are_told_by_their_text() {
    let (alt, desc) = words(
        r"\datavisualization [school book axes, visualize as line, x axis={label=time}] data {x, y
        0, 0
        1, 1
    };",
    );
    assert!(alt.starts_with("Chart with the text “time”"), "{alt}");
    assert!(desc.starts_with("A chart, drawn by the data visualization library. Text in the chart: "), "{desc}");
}

#[test]
fn the_authors_text_comes_first() {
    // `alt={…}` on the picture: told as written (its math spoken), with no description of tikra's
    for code in [
        r"\begin{tikzpicture}[alt={A unit circle, radius $r = 1$}] \draw (0,0) circle (1); \node {O}; \end{tikzpicture}",
        r"\tikz[alt={A unit circle, radius $r = 1$}] \draw (0,0) circle (1);",
    ] {
        let r = tikra::convert(code).unwrap();
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(r.alt, "A unit circle, radius r equals 1");
        assert_eq!(r.description, r.alt);
        assert!(plain_ids(&r.svg).contains(r#"role="img" aria-labelledby="tr-title"><title id="tr-title">A unit circle, radius r equals 1</title><metadata>"#), "{}", r.svg);
    }
    // an empty one is no text: tikra writes its own
    assert_eq!(words(r"\begin{scope}[alt={}] \draw (0,0) circle (1); \end{scope}").0, "Drawing of one shape, without text");
}

#[test]
fn the_words_in_other_languages_and_the_same_drawing() {
    let code = r"\begin{tikzpicture}\node (a) {Start}; \node (b) at (2,0) {$x > 1$}; \draw[->] (a) -- node[above] {yes} (b);\end{tikzpicture}";
    let told = |l: tikra::Locale| tikra::convert_with(code, &tikra::Options::locale(l)).unwrap();
    let en = told(tikra::Locale::En);
    assert_eq!(en.description, "Diagram with 2 nodes and one connection. Nodes: Start; x is greater than 1. Connections: from Start to x is greater than 1, labeled “yes”.");
    let pt = told(tikra::Locale::PtBr);
    assert_eq!(pt.alt, "Diagrama com 2 nós e uma ligação");
    assert_eq!(pt.description, "Diagrama com 2 nós e uma ligação. Nós: Start; x é maior que 1. Ligações: de Start para x é maior que 1, com o rótulo “yes”.");
    let ru = told(tikra::Locale::Ru);
    assert_eq!(ru.alt, "Схема с 2 узлами и 1 связью");
    assert_eq!(told(tikra::Locale::Zh).alt, "包含 2 个节点和 1 条连线的图");
    // the drawing does not change with the language
    let drawing = |svg: &str| svg[svg.find("</metadata>").unwrap()..].to_string();
    for l in tikra::Locale::ALL {
        assert_eq!(drawing(&told(l).svg), drawing(&en.svg), "{l:?}");
    }
    // the author's text is told as written, its math in the language
    let r = tikra::convert_with(r"\tikz[alt={Círculo de raio $\frac{1}{2}$}] \draw (0,0) circle (1);", &tikra::Options::locale(tikra::Locale::PtBr)).unwrap();
    assert_eq!(r.alt, "Círculo de raio um meio");
    assert_eq!(tikra::convert(code).unwrap().description, en.description);
}
