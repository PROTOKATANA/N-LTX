//! The input forms `convert` accepts and the repairs made before reading.

mod common;
use common::*;

#[test]
fn standalone_document_with_border() {
    let r = no_warnings(
        r"\documentclass[tikz,border=1cm]{standalone}
          \usepackage{amsmath}
          \usetikzlibrary{arrows.meta}
          \pgfplotsset{compat=1.18}
          \begin{document}
          \begin{tikzpicture} \draw (0,0) -- (1,0); \end{tikzpicture}
          \end{document}",
    );
    // 1 cm of line + 2 cm of margin = 3 cm ≈ 85 pt
    assert!(r.svg.contains(r#"width="85."#), "{}", &r.svg[..200]);
}

#[test]
fn standalone_without_border_uses_default_margin() {
    // documentclass with options but no border: a 2pt margin
    let r = no_warnings(r"\documentclass[tikz]{standalone}\begin{document}\begin{tikzpicture}\draw (0,0)--(1,0);\end{tikzpicture}\end{document}");
    assert!(r.svg.contains(r#"width="32."#), "{}", &r.svg[..200]);
    // a border whose value is not a length: default margin
    no_warnings(r"\documentclass[border={abc}]{standalone}\begin{document}\begin{tikzpicture}\draw (0,0)--(1,0);\end{tikzpicture}\end{document}");
    // unclosed documentclass[
    no_warnings(r"\documentclass[border=3pt \begin{tikzpicture}\draw (0,0)--(1,0);\end{tikzpicture}");
}

#[test]
fn preview_package_setup_is_neither_text_nor_a_warning() {
    // the preamble of many TeXample figures: nothing of it is drawn
    let plain = no_warnings(r"\begin{tikzpicture}\draw (0,0)--(1,0);\end{tikzpicture}");
    let r = no_warnings(
        r"\documentclass{article}
          \usepackage{tikz}
          \usepackage[active,tightpage]{preview}
          \PreviewEnvironment{tikzpicture}
          \setlength\PreviewBorder{5pt}%
          \begin{document}
          \begin{tikzpicture}\draw (0,0)--(1,0);\end{tikzpicture}
          \end{document}",
    );
    let drawing = |svg: &str| (svg[..svg.find(" role=").unwrap()].to_string(), svg[svg.find("</metadata>").unwrap()..].to_string());
    assert_eq!(drawing(&r.svg), drawing(&plain.svg));
}

#[test]
fn tikzpicture_with_options_and_spaces() {
    let r = no_warnings("\\begin{tikzpicture}  \n [scale=2]\n \\draw (0,0) -- (1,0);\n\\end{tikzpicture}");
    assert!(r.svg.contains("<path"));
}

#[test]
fn more_than_one_tikzpicture() {
    // as TeX sets them: side by side, a space between them; a blank line starts a new line
    let one = no_warnings(r"\begin{tikzpicture}\draw (0,0)--(1,0);\end{tikzpicture}");
    let two = no_warnings(r"\begin{tikzpicture}\draw (0,0)--(1,0);\end{tikzpicture} \begin{tikzpicture}\draw (0,0)--(2,0);\end{tikzpicture}");
    assert_eq!(two.svg.matches("<path").count(), 2);
    let width = |svg: &str| -> f64 { svg.split("width=\"").nth(1).unwrap().split("pt").next().unwrap().parse().unwrap() };
    // 1 cm + 3.33 pt + 2 cm (each picture half a line width wider at both ends), and the margins
    assert!((width(&two.svg) - width(&one.svg) - (2.0 * 28.3465 + (3.333 + 0.4) * 72.0 / 72.27)).abs() < 0.1, "{}", width(&two.svg));
    let lines = no_warnings("\\tikz \\draw (0,0)--(1,0);\n\n\\tikz \\draw (0,0)--(1,0);");
    assert!(width(&lines.svg) < width(&one.svg) + 0.1);
}

#[test]
fn scalebox_scales_the_pictures_in_it() {
    // the drawing, its lines and its size, by each factor (the second one up)
    let width = |svg: &str| -> f64 { svg.split("width=\"").nth(1).unwrap().split("pt").next().unwrap().parse().unwrap() };
    let one = no_warnings(r"\tikz \draw[line width=2pt] (0,0) -- (4,0);");
    let half = no_warnings(r"\scalebox{0.5}{\tikz \draw[line width=2pt] (0,0) -- (4,0);}");
    assert!((width(&one.svg) - width(&half.svg) - 2.0 * 28.3465 - 1.0).abs() < 0.1, "{}", width(&half.svg));
    assert!(half.svg.contains("stroke-width=\"1\"") || half.svg.contains("stroke-width=\"0.996\""), "{}", half.svg);
    let wide = no_warnings(r"\scalebox{2}[1]{\tikz \draw (0,0) circle (1);}");
    assert!(width(&wide.svg) > 4.0 * 28.3465, "{}", wide.svg);
}

#[test]
fn preview_takes_the_pictures_out_of_their_boxes() {
    // with `\PreviewEnvironment{tikzpicture}` of the active preview package, each picture is a page
    // of its own: the box around it neither scales nor frames it (TeX's page is 114.21 by 57.31 pt)
    let size = |doc: &str| -> (f64, f64) {
        let (c, w) = tikra::engine::run_document(doc).unwrap();
        assert!(w.is_empty(), "{w:?}");
        let (lo, hi) = (c.min.unwrap(), c.max.unwrap());
        ((hi.x - lo.x) / tikra::PT, (hi.y - lo.y) / tikra::PT)
    };
    let pic = r"\begin{tikzpicture}\draw (0,0) rectangle (4,2);\end{tikzpicture}";
    let preview = r"\documentclass{article}\usepackage{tikz}\usepackage[active,tightpage]{preview}\PreviewEnvironment{tikzpicture}\begin{document}";
    for boxed in [
        format!(r"\scalebox{{0.5}}{{{pic}}}"),
        format!(r"\scalebox{{0.5}}[2]{{{pic}}}"),
        format!(r"\resizebox{{2cm}}{{!}}{{{pic}}}"),
        format!(r"\resizebox*{{2cm}}{{1cm}}{{{pic}}}"),
        format!(r"\fbox{{{pic}}}"),
    ] {
        let (w, h) = size(&format!(r"{preview}{boxed}\end{{document}}"));
        assert!((w - 114.21).abs() < 0.05 && (h - 57.31).abs() < 0.05, "{boxed}: {w} {h}");
    }
    // other environments previewed, or the package not active: the box is there
    for setup in [r"\usepackage[active]{preview}\PreviewEnvironment{center}", r"\usepackage{preview}\PreviewEnvironment{tikzpicture}", r"\usepackage[active]{xcolor}"] {
        let (w, _) = size(&format!(r"\documentclass{{article}}\usepackage{{tikz}}{setup}\begin{{document}}\scalebox{{0.5}}{{{pic}}}\end{{document}}"));
        assert!((w - 57.11).abs() < 0.05, "{setup}: {w}");
    }
}

#[test]
fn saved_boxes_are_drawn_where_they_are_used() {
    // the page's box (pt): the picture's, from its baseline
    let bbox = |doc: &str| -> [f64; 4] {
        let (c, w) = tikra::engine::run_document(doc).unwrap();
        assert!(w.is_empty(), "{w:?}");
        let (lo, hi) = (c.min.unwrap(), c.max.unwrap());
        [lo.x, lo.y, hi.x, hi.y].map(|v| v / tikra::PT)
    };
    let close = |a: [f64; 4], b: [f64; 4]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 0.05);
    let preamble = r"\documentclass{article}\usepackage{tikz}
        \newsavebox{\boxa}\newsavebox\boxb\newsavebox{\boxc}
        \savebox{\boxa}{\begin{tikzpicture}\draw (0,0) rectangle (2,1);\end{tikzpicture}}
        \sbox\boxb{\begin{tikzpicture}[overlay]\draw (0,0) -- (1,1);\end{tikzpicture}}
        \begin{lrbox}{\boxc}\begin{tabular}{c}\begin{tikzpicture}\draw (0,0) rectangle (1,1);\end{tikzpicture}\\ a b\end{tabular}\end{lrbox}
        \begin{document}";
    // a node's text: the picture saved, its box the node's; one with nothing but `overlay` drawing
    // is a box of no size
    let b = bbox(&format!(
        r"{preamble}\begin{{tikzpicture}}\node[draw,inner sep=0pt] at (3,4) {{\usebox{{\boxa}}}}; \node[draw] at (0,0) {{\usebox\boxb}};\end{{tikzpicture}}\end{{document}}"
    ));
    // (TeX's box: from -3.533 to 114.211 and 128.437)
    assert!(close(b, [0.0, 0.0, 117.744, 131.970]), "{b:?}");
    // a box of text and pictures: on its baseline (a `tabular`'s middle, on the math axis)
    let b = bbox(&format!(r"{preamble}\begin{{tikzpicture}}[baseline=0pt]\node[draw,inner sep=0pt,anchor=base west] {{\usebox{{\boxc}}}};\end{{tikzpicture}}\end{{document}}"));
    assert!(close(b, [0.0, -19.926, 41.253, 24.926]), "{b:?}");
    // in the document, the box where it is used; the pictures saved are not drawn where they were made
    let b = bbox(&format!(r"{preamble}\usebox{{\boxa}}\end{{document}}"));
    assert!(close(b, [0.0, 0.0, 57.305, 28.853]), "{b:?}");
    // a box never saved in is empty
    no_warnings(r"\newsavebox{\boxd}\tikz \node {\usebox{\boxd}};");
    // what is not understood
    with_warning(r"\tikz \node {\usebox{\nobox}};", r"\usebox: no box '\nobox'");
    with_warning(r"\usebox{\nobox} \tikz \draw (0,0) -- (1,0);", r"\usebox: no box '\nobox'");
    with_warning(r"\newsavebox{} \tikz \draw (0,0) -- (1,0);", r"\newsavebox: no box name");
    with_warning(r"\sbox{}{x} \tikz \draw (0,0) -- (1,0);", r"\savebox: no box name");
    with_warning(r"\newsavebox{\boxa}\savebox{\boxa}[3cm][l]{\tikz \draw (0,0) -- (1,0);}\usebox{\boxa}", r"\savebox: the width is ignored");
    // in a picture outside a node's text, nothing (as TikZ ignores text there)
    let r = no_warnings(r"\newsavebox{\boxa}\sbox{\boxa}{\tikz \draw (0,0) -- (5,0);}\begin{tikzpicture}\usebox{\boxa}\draw (0,0) -- (1,0);\end{tikzpicture}");
    assert_eq!(r.svg.matches("<path").count(), 1, "{}", r.svg);
}

#[test]
fn spaces_between_the_words_of_a_tabular_cell() {
    // TeX's interword space between them, none at the cell's ends
    let width = |cell: &str| -> f64 {
        let (c, _) = tikra::engine::run_document(&format!(r"\begin{{tabular}}{{c}}{cell}\\ \tikz \draw (0,0) -- (0.1,0);\end{{tabular}}")).unwrap();
        (c.max.unwrap().x - c.min.unwrap().x) / tikra::PT
    };
    assert!((width("a b") - width("ab") - 3.333).abs() < 0.01, "{} {}", width("a b"), width("ab"));
    assert!((width(" ab ") - width("ab")).abs() < 0.01, "{} {}", width(" ab "), width("ab"));
}

#[test]
fn inline_tikz_with_and_without_braces() {
    no_warnings(r"\tikz \draw (0,0) -- (1,1);");
    no_warnings(r"\tikz[thick] {\draw (0,0) -- (1,1); \fill (0,0) circle (2pt);}");
    no_warnings(r"\tikz  [thick]  \draw (0,0) -- (1,1);");
    // \tikzset in the preamble does not count as \tikz
    no_warnings(r"\tikzset{a/.style={red}} \tikz \draw[a] (0,0) -- (1,1);");
}

#[test]
fn center_lines_and_the_space_around_it() {
    // the height of the document's drawing, in pt
    let height = |doc: &str| -> f64 {
        let (c, _) = tikra::engine::run_document(doc).unwrap();
        (c.max.unwrap().y - c.min.unwrap().y) / tikra::PT
    };
    let pic = r"\begin{tikzpicture}\draw (0,0) rectangle (3,1);\end{tikzpicture}";
    // `\\` in `center` ends the paragraph: the blank line after it adds no empty line, so the text's
    // baseline is a \baselineskip (12pt) below the picture (28.85pt high), as TeX has it
    let h = height(&format!("\\begin{{center}}\n{pic}\n\\\\\n\nxx\n\\end{{center}}"));
    assert!((h - 40.85).abs() < 0.3, "{h}");
    // \topsep above and below the list (the space below it comes before the next line)
    let h = height(&format!("{pic}\n\\begin{{center}}\n{pic}\n\\end{{center}}\nxx"));
    assert!((h - 86.7).abs() < 0.3, "{h}");
    // in a \vbox the space below the list stays at its end, \partopsep more as it began a paragraph
    // (TeX's \vbox is 50.85pt high)
    let h = height(&format!("\\vbox\\bgroup\\hsize=12cm\\relax\n\\begin{{center}}\n{pic}\n\\\\\n\nxx\n\\end{{center}}\n\\egroup"));
    assert!((h - 50.85).abs() < 0.3, "{h}");
}

#[test]
fn bare_axis_and_loose_statements() {
    no_warnings(r"\begin{axis}[xmin=0,xmax=1,ymin=0,ymax=1] \addplot coordinates {(0,0) (1,1)}; \end{axis}");
    no_warnings(r"\draw (0,0) -- (1,1);");
    no_warnings(r"\coordinate (A) at (1,1); \fill (A) circle (1pt);");
}

#[test]
fn no_figure_is_an_error() {
    let e = tikra::convert("just some text, no figure").unwrap_err();
    assert!(e.to_string().contains("no tikzpicture"), "{e}");
    assert!(tikra::convert("").is_err());
}

#[test]
fn errors_have_a_message_per_kind() {
    assert_eq!(tikra::Error::Expression("x".into()).to_string(), "expression: x");
    assert_eq!(tikra::Error::Figure("y".into()).to_string(), "figure: y");
    assert_eq!(tikra::Error::Png("z".into()).to_string(), "png: z");
    let e: Box<dyn std::error::Error> = Box::new(tikra::Error::Figure("w".into()));
    assert!(e.to_string().contains('w'));
}

#[test]
fn doubled_backslashes_are_undone() {
    let r = with_warning(r"\\begin{tikzpicture} \\draw (0,0) -- (1,1); \\end{tikzpicture}", "doubled backslashes");
    assert!(r.svg.contains("<path"));
}

#[test]
fn markdown_fences_commands_and_brackets() {
    // a code fence with escaped \[ \]
    with_warning("```latex\n\\draw\\[red\\] (0,0) -- (1,1);\n```", "Markdown");
    // one command with a doubled backslash among normal ones
    let r = with_warning("\\begin{tikzpicture} \\\\draw (0,0) -- (1,1); \\node at (0,0) {a}; \\end{tikzpicture}", "Markdown");
    assert!(r.svg.contains("<path"));
    // \\[2pt] (a line break with spacing) is left alone
    no_warnings(r"\begin{tikzpicture} \node[align=center] at (0,0) {a \\[2pt] b}; \end{tikzpicture}");
    // \[ at the start of the text
    with_warning("\\[\\draw (0,0) -- (1,1);", "Markdown");
    // TeX's display math, and a line break before `\end`, are no Markdown
    let r = no_warnings(r"\begin{tikzpicture} \node[text width=3cm] {so \[ x^2 \] and \[ \begin{array}{c} a \end{array} \]}; \end{tikzpicture}");
    assert!(r.svg.contains("<use"));
    no_warnings(r"\begin{tikzpicture} \node {\begin{tabular}{c} a \\\end{tabular}}; \end{tikzpicture}");
    // the option list of an environment, escaped
    with_warning("\\begin{tikzpicture}\\[thick\\] \\draw (0,0) -- (1,1); \\end{tikzpicture}", "Markdown");
}

#[test]
fn preamble_operators_and_page_setup() {
    // `\DeclareMathOperator` (and starred) defines a command a label uses; the geometry
    // package's margins are no text
    let r = no_warnings(
        r"\documentclass{article}
          \usepackage{amsmath}
          \usepackage{geometry}
          \geometry{hmargin=1cm,vmargin=1cm}
          \DeclareMathOperator{\p}{p}
          \DeclareMathOperator*{\am}{arg\,max}
          \begin{document}
          \begin{tikzpicture} \node {$\p(a) = \am_x f$}; \end{tikzpicture}
          \end{document}",
    );
    assert!(!r.description.contains("margin"), "{}", r.description);
}

#[test]
fn resizebox_scales_the_picture() {
    let width = |body: &str| {
        let r = figure(&format!("\\documentclass{{article}}\\begin{{document}}{body}\\end{{document}}"));
        let at = r.svg.find("viewBox=\"").unwrap() + 9;
        r.svg[at..].split(' ').nth(2).unwrap().parse::<f64>().unwrap()
    };
    let picture = r"\begin{tikzpicture}\draw (0,0) rectangle (2,1);\end{tikzpicture}";
    let plain = width(picture);
    // to 4cm wide (twice), to 3cm high (thrice), to its height and depth, or as it is with `!` twice
    let wide = width(&format!(r"\resizebox{{4cm}}{{!}}{{{picture}}}"));
    let high = width(&format!(r"\resizebox{{!}}{{3cm}}{{{picture}}}"));
    let starred = width(&format!(r"\resizebox*{{!}}{{3cm}}{{{picture}}}"));
    let same = width(&format!(r"\resizebox{{!}}{{!}}{{{picture}}}"));
    // (nothing to resize)
    assert!((width(&format!(r"\resizebox{{1cm}}{{!}}{{}}{picture}")) - plain).abs() < 1e-6);
    assert!(wide > 1.9 * plain && high > 2.8 * plain && (high - starred).abs() < 1e-6 && (same - plain).abs() < 1e-6, "{plain} {wide} {high} {starred} {same}");
    // a size tikra cannot read leaves the picture as it is, with a warning
    let r = with_warning(&format!(r"\documentclass{{article}}\begin{{document}}\resizebox{{0.5\width}}{{!}}{{{picture}}}\end{{document}}"), "resizebox");
    assert!(!r.description.contains("width"));
}

#[test]
fn preamble_defines_styles_colors_and_macros() {
    let r = no_warnings(
        r"\documentclass{standalone}
          \usepackage{tikz}
          \definecolor{brand}{RGB}{200,30,30}
          \tikzset{arrow/.style={->, thick, brand}}
          \def\L{3}
          \begin{document}
          \begin{tikzpicture} \draw[arrow] (0,0) -- (\L,0); \end{tikzpicture}
          \end{document}",
    );
    assert!(r.svg.contains("#c81e1e"), "color from the preamble");
}

#[test]
fn comments_are_removed_but_escaped_percent_stays() {
    let r = no_warnings("\\begin{tikzpicture} % a comment with \\draw (9,9)\n \\draw (0,0) -- (1,0); \\node at (0,1) {50\\%};\n\\end{tikzpicture}");
    assert!(r.svg.contains("<use"), "the 50% label is drawn");
}

#[test]
fn svg_has_title_and_escaped_source() {
    let r = no_warnings(r#"\begin{tikzpicture} \draw (0,0) -- (1,0) node[right] {"a" & <b>}; \end{tikzpicture}"#);
    // the ids of the words end in the figure's own suffix
    let svg = r.svg.replace(&r.svg[r.svg.find("tr-title-").unwrap() + 8..r.svg.find("tr-title-").unwrap() + 17], "");
    assert!(svg.contains(r#"aria-labelledby="tr-title" aria-describedby="tr-desc""#));
    assert!(svg.contains("<title id=\"tr-title\">Drawing of one shape with the text “&quot;a&quot; &amp; &lt;b&gt;”</title>"));
    assert!(svg.contains("<desc id=\"tr-desc\">Text in the figure: &quot;a&quot; &amp; &lt;b&gt;.</desc>"));
    assert!(r.svg.contains("<tikra:source type=\"tikz\">"));
    assert!(r.svg.contains("&quot;a&quot; &amp; &lt;b&gt;"));
}

#[test]
fn version_and_unit() {
    assert!(!tikra::VERSION.is_empty());
    assert!((tikra::PT * 72.27 - 2.54).abs() < 1e-12);
}

#[test]
fn errors_that_bring_the_figure_down() {
    // tikzpicture and preamble options with an invalid computation
    assert!(tikra::convert(r"\begin{tikzpicture}[scale=zz] \draw (0,0) -- (1,0); \end{tikzpicture}").is_err());
    assert!(tikra::convert(r"\begin{scope}[scale=zz]\end{scope} \begin{tikzpicture} \draw (0,0) -- (1,0); \end{tikzpicture}").is_err());
    // an empty border: default margin
    no_warnings(r"\documentclass[border={}]{standalone}\begin{document}\begin{tikzpicture}\draw (0,0)--(1,0);\end{tikzpicture}\end{document}");
}

#[test]
fn pagecolor_paints_the_page_behind_the_figure() {
    // the whole page, margin included, under the figure; its argument is not text
    let doc = |pre: &str| {
        format!(r"\documentclass[border=5pt]{{standalone}}{pre}\begin{{document}}\begin{{tikzpicture}}\fill[blue] (0,0) circle (1);\end{{tikzpicture}}\end{{document}}")
    };
    let r = no_warnings(&doc(r"\pagecolor{olive!50!yellow!50!white}"));
    let view = &r.svg[r.svg.find("viewBox=\"").unwrap() + 9..];
    let view = &view[..view.find('"').unwrap()];
    let [x, y, w, h]: [&str; 4] = view.split(' ').collect::<Vec<_>>().try_into().unwrap();
    let body = &r.svg[r.svg.find("</defs>").unwrap()..];
    // olive and yellow are cmyk: TeX's PDF shows (230,220,138)
    assert!(body.starts_with(&format!(r##"</defs><rect x="{x}" y="{y}" width="{w}" height="{h}" fill="#e6dc8a"/>"##)), "{body}");
    assert!(!r.svg.contains("<use"), "no text: {}", r.svg);
    // a color model, and \nopagecolor taking it away
    assert!(no_warnings(&doc(r"\pagecolor[RGB]{200,30,30}")).svg.contains(r##"fill="#c81e1e"/>"##));
    assert!(!no_warnings(&doc(r"\pagecolor{red}\nopagecolor")).svg.contains("<rect"));
    let r = with_warning(&doc(r"\pagecolor{nocolor}"), "\\pagecolor: unknown color 'nocolor'");
    assert!(!r.svg.contains("<rect"));
}

#[test]
fn xcolor_named_sets_from_the_preamble() {
    // `Gray` is cmyk in the dvipsnames and rgb in the svgnames: each option defines its colors over
    // those before it, the class's options first, as xcolor takes them
    let gray = |pre: &str| {
        let r = no_warnings(&format!(r"{pre}\begin{{document}}\begin{{tikzpicture}}\fill[Gray] (0,0) circle (1);\end{{tikzpicture}}\end{{document}}"));
        let at = r.svg.find("fill=\"#").unwrap() + 6;
        r.svg[at..at + 7].to_string()
    };
    let (dvips, svg) = ("#939598", "#808080");
    assert_eq!(gray(r"\documentclass{article}\usepackage{tikz}"), dvips);
    assert_eq!(gray(r"\documentclass[svgnames]{article}\usepackage{tikz}"), svg);
    assert_eq!(gray(r"\documentclass{article}\usepackage[dvipsnames,svgnames]{xcolor}"), svg);
    assert_eq!(gray(r"\documentclass{article}\usepackage[svgnames,dvipsnames]{xcolor}"), dvips);
    assert_eq!(gray(r"\documentclass[dvipsnames]{article}\usepackage[svgnames]{xcolor}"), svg);
    // options of another package are not xcolor's
    assert_eq!(gray(r"\documentclass{article}\usepackage[svgnames]{other}"), dvips);
    assert_eq!(gray(r"\PassOptionsToPackage{svgnames}{xcolor}\documentclass{article}\usepackage{tikz}"), svg);
    assert_eq!(gray(r"\PassOptionsToPackage{svgnames}{other}\documentclass{article}\usepackage{tikz}"), dvips);
    // \xdefinecolor defines a color as \definecolor does
    assert_eq!(gray(r"\documentclass{article}\usepackage{tikz}\xdefinecolor{Gray}{RGB}{175,193,36}"), "#afc124");
}

#[test]
fn nothing_after_end_document_and_no_title_without_maketitle() {
    // TeX reads no further than \end{document}, and \title, \author and \date only keep their text
    let drawing = |svg: &str| svg[svg.find("</metadata>").unwrap()..].to_string();
    let plain = no_warnings(r"\documentclass{article}\usepackage{tikz}\begin{document}\begin{tikzpicture}\draw (0,0)--(1,0);\end{tikzpicture}\end{document}");
    let r = no_warnings(
        r"\documentclass{article}\usepackage{tikz}
          \title{A title}\author{Someone}\date{Today}
          \begin{document}\begin{tikzpicture}\draw (0,0)--(1,0);\end{tikzpicture}\end{document}
          Notes after the end. \begin{tikzpicture}\draw (0,0) circle (1);\end{tikzpicture}",
    );
    assert_eq!(drawing(&r.svg), drawing(&plain.svg));
}
