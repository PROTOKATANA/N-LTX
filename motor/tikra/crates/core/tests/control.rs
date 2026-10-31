//! The statement reader: `\foreach`, `scope` and transforms, macros, environments and commands
//! that are not drawing.

mod common;
use common::*;
use tikra::canvas::Paint;
use tikra::scene::{Point, Transform};

/// The x of the start of each path drawn by `\foreach \x in {list}`.
fn foreach_values(list: &str) -> Vec<f64> {
    let c = canvas(&format!(r"\foreach \x in {{{list}}} {{ \draw (\x,0) -- (\x,1); }}"));
    paths(&c).iter().map(|d| points(&d.segments)[0].0).collect()
}

/// The texts `\foreach \x in {list}` puts in nodes, as the number of them.
fn foreach_texts(list: &str) -> usize {
    texts(&canvas(&format!(r"\foreach \x in {{{list}}} {{ \node {{\x}}; }}"))).len()
}

#[test]
fn foreach_lists_ellipsis_and_counter() {
    assert_eq!(foreach_values("0,1,...,4"), [0.0, 1.0, 2.0, 3.0, 4.0]);
    assert_eq!(foreach_values("0,0.5,...,2"), [0.0, 0.5, 1.0, 1.5, 2.0]);
    assert_eq!(foreach_values("5,4,...,1"), [5.0, 4.0, 3.0, 2.0, 1.0]);
    assert_eq!(foreach_texts("a,b,c"), 3);
    let c = canvas(r"\foreach \x [count=\i] in {a,b,c} { \node (n\i) at (\i,0) {\x}; }");
    assert_eq!(texts(&c).len(), 3);
    let p = point(r"\foreach \x [count=\i] in {a,b,c} { \node (n\i) at (\i,0) {\x}; }", "n3.center");
    assert!(close_to((p.x, p.y), (3.0, 0.0)), "{p:?}");
    // slash variables, a body without braces and a glued in
    let c = canvas(r"\foreach \p/\q in{1/2, 3} \draw (\p,0) -- (\q,1);");
    assert_eq!(paths(&c).len(), 2);
    // loop variables do not leak
    let (_, w) = canvas_and_warnings(r"\foreach \k in {1} {} \draw (\k,0) -- (1,1);");
    assert!(!w.is_empty());
}

#[test]
fn nested_foreach() {
    let c = canvas(r"\foreach \x in {0,1,2} \foreach \y in {0,1} \fill (\x,\y) circle (1pt);");
    assert_eq!(paths(&c).len(), 6);
    let c = canvas(r"\foreach \x in {0,1,2} {\foreach \y in {0,1} { \fill (\x,\y) circle (1pt); }}");
    assert_eq!(paths(&c).len(), 6);
}

#[test]
fn nested_foreach_with_a_braced_inner_body() {
    let c = canvas(r"\foreach \x in {0,1,2} \foreach \y in {0,1} { \fill (\x,\y) circle (1pt); }");
    assert_eq!(paths(&c).len(), 6);
}

#[test]
fn scopes_and_transforms() {
    let c = canvas(
        r"\coordinate (A) at (1,1);
          \begin{scope}[xshift=1cm, yshift=2cm, xscale=2, yscale=3, thick]
            \draw (0,0) -- (1,1);
          \end{scope}
          \begin{scope}[shift={(A)}, scale=2] \draw (0,0) -- (1,0); \end{scope}
          \begin{scope}[x={(1cm,1cm)}, y={(0cm,2cm)}] \draw (0,0) -- (1,1); \end{scope}
          \begin{scope}[x=2cm, y=3cm] \draw (0,0) -- (1,1); \end{scope}",
    );
    let t = paths(&c);
    assert!(close_to(points(&t[0].segments)[1], (3.0, 5.0)), "{:?}", points(&t[0].segments));
    assert!(close(t[0].stroke.as_ref().unwrap().width, 0.8 * tikra::PT), "option inherited from the scope");
    assert!(close_to(points(&t[1].segments)[0], (1.0, 1.0)) && close_to(points(&t[1].segments)[1], (3.0, 1.0)));
    assert!(close_to(points(&t[2].segments)[1], (1.0, 3.0)));
    assert!(close_to(points(&t[3].segments)[1], (2.0, 3.0)));
}

#[test]
fn errors_in_scope_options_are_figure_errors() {
    for op in ["scale=zz", "xscale=zz", "yscale=zz", "rotate=zz", "xshift=zz", "yshift=zz", "shift={(zz)}", "x=zz", "y={(zz)}"] {
        let r = tikra::engine::run_document(&format!(r"\begin{{tikzpicture}}\begin{{scope}}[{op}] \end{{scope}}\end{{tikzpicture}}"));
        assert!(r.is_err(), "{op}");
    }
}

#[test]
fn ends_of_scopes_with_no_begin_do_no_harm() {
    let c = canvas(r"\end{scope} \end{scope} \draw (0,0) -- (1,0);");
    assert_eq!(paths(&c).len(), 1);
}

#[test]
fn macros_def_newcommand_and_pgfmath() {
    let c = canvas(
        r"\def\a{1} \gdef\b{2} \edef\cc{3} \xdef\dd{4}
          \newcommand{\ee}{5} \renewcommand{\ff}[1]{6} \providecommand\gg{7}
          \pgfmathsetmacro{\total}{\a+\b+\cc+\dd+\ee+\ff+\gg}
          \pgfmathsetlengthmacro{\len}{2cm}
          \pgfmathparse{1+1}
          \draw (0,0) -- (\total,\len);",
    );
    // \ff takes one argument: as in TeX it eats the `+` after it, so the sum is 1+2+3+4+5+67
    let end = points(&paths(&c)[0].segments)[1];
    assert!(close(end.0, 82.0) && (end.1 - 2.0).abs() < 1e-6, "{end:?}");
    let (_, w) = canvas_and_warnings(r"\pgfmathsetmacro{\x}{\doesnotexist}");
    assert!(w.iter().any(|a| a.contains("pgfmathsetmacro")), "{w:?}");
}

/// The start of the first path of a body, with no warnings.
fn start(body: &str) -> (f64, f64) {
    let (c, w) = canvas_and_warnings(body);
    assert!(w.is_empty(), "{w:?}\n{body}");
    points(&paths(&c)[0].segments)[0]
}

#[test]
fn pgfmath_as_tex_reads_it() {
    // functions without parentheses, lengths counted in pt, a number glued to a macro's digits
    assert!(close_to(start(r"\def\a{30} \draw (sin \a, cos 60*2) -- (1,1);"), (0.5, 1.0)));
    assert!(close_to(start(r"\draw (3pt*2cm, 1/10pt) -- (1,1);"), (6.0, 0.1 * tikra::PT)));
    assert!(close_to(start(r"\def\d{6} \draw (18\d pt, 0) -- (1,1);"), (186.0 * tikra::PT, 0.0)));
    let (c, _) = canvas_and_warnings(r"\draw (0,0) arc (0:90:1/10pt);");
    let end = *points(&paths(&c)[0].segments).last().unwrap();
    assert!(close_to(end, (-0.1 * tikra::PT, 0.1 * tikra::PT)), "{end:?}");
    // an array of colors, through \pgfmathparse and \pgfmathsetmacro
    for set in [r"\pgfmathparse{\list[1]}\let\c\pgfmathresult", r"\pgfmathsetmacro\c{\list[1]}"] {
        let (c, w) = canvas_and_warnings(&format!(r#"\def\list{{{{"orange","blue","red"}}}} {set} \fill[\c] (0,0) rectangle (1,1);"#));
        assert!(w.is_empty(), "{w:?}");
        assert!(matches!(&paths(&c)[0].fill.as_ref().unwrap().paint, Paint::Solid(k) if k.hex() == "#0000ff"));
    }
}

#[test]
fn pgfmath_inner_commands_and_random_numbers() {
    // `\pgfmathadd@` and the other inner forms
    assert!(close_to(
        start(r"\makeatletter\def\x{0} \pgfmathadd@{\x}{0.25}\let\x=\pgfmathresult \pgfmathmultiply@{\x}{4}\let\x=\pgfmathresult \draw (\x,0) -- (1,1);"),
        (1.0, 0.0)
    ));
    // \pgfmathrand before the point it computes, as TeX's first number after the seed 1
    assert!(close_to(
        start(r"\pgfmathsetseed{1} \pgfpathmoveto{\pgfmathrand\pgfpoint{\pgfmathresult cm}{0cm}} \pgfpathlineto{\pgfpointorigin} \pgfusepath{stroke}"),
        (-0.30379, 0.0)
    ));
    // an item of a random list (the second, for the seed 1), then a random integer
    let p = start(r"\pgfmathsetseed{1} \pgfmathdeclarerandomlist{L}{{1}{{2}}3{4}} \pgfmathrandomitem{\m}{L} \pgfmathrandominteger{\n}{0}{3} \draw (\m,\n) -- (5,5);");
    assert!(close_to(p, (2.0, 3.0)), "{p:?}");
    assert!(close_to(start(r"\pgfmathsetseed{1} \pgfmathsetmacro\r{random(3,7)} \pgfmathrnd \draw (\r,\pgfmathresult) -- (5,5);"), (4.0, 0.10826)));
}

#[test]
fn join_options_in_braces() {
    // `join=with a by {style=value,…}`: the braces only group the options
    let (c, w) = canvas_and_warnings(
        r"\tikzset{down/.style={to path={-- ++(0,#1) -| (\tikztotarget)}}}
          \begin{scope}[start chain] \node[on chain] (a) {a}; \node[on chain] (b) {b}; \chainin (a) [join=with b by {down=-1cm,ultra thick}]; \end{scope}",
    );
    assert!(w.is_empty(), "{w:?}");
    let join = paths(&c).into_iter().find(|d| d.stroke.as_ref().is_some_and(|s| close(s.width, 1.6 * tikra::PT))).expect("the join");
    assert_eq!(points(&join.segments).len(), 4, "{:?}", join.segments);
}

#[test]
fn decoration_aspect_is_a_macro() {
    assert!(close_to(start(r"\draw (\pgfdecorationsegmentaspect,0) -- (1,1);"), (0.5, 0.0)));
    assert!(close_to(start(r"\pgfkeys{/pgf/decoration/aspect=0.3} \draw (\pgfdecorationsegmentaspect,0) -- (1,1);"), (0.3, 0.0)));
}

#[test]
fn latex_counters_and_pgfmath() {
    // \the<counter> is the counter's value, in an expression, a color and a node's text
    let (c, w) = canvas_and_warnings(
        r"\newcounter{d} \setcounter{d}{30}
          \draw (0,0) -- (\thed/10,0);
          \fill[red!\thed] (0,0) rectangle (1,1); \fill[red!30] (0,0) rectangle (1,1);
          \node {\thed}; \node {30};",
    );
    assert!(w.is_empty(), "{w:?}");
    let p = paths(&c);
    assert!(close(points(&p[0].segments)[1].0, 3.0), "{:?}", p[0].segments);
    assert_eq!(format!("{:?}", p[1].fill), format!("{:?}", p[2].fill));
    let t = texts(&c);
    assert!(close(t[0].0.width(), t[1].0.width()) && t[0].0.width() > 0.0);
    // pgfmath truncates toward zero; its counters are set locally (a \foreach body is a group), which
    // \setcounter{n}{\then} makes global
    let x = |body: &str| {
        let c = canvas(&format!(r"\newcounter{{n}} {body} \draw (0,0) -- (\then,0);"));
        points(&paths(&c)[0].segments)[1].0
    };
    assert!(close(x(r"\pgfmathsetcounter{n}{2.7*2}"), 5.0));
    assert!(close(x(r"\pgfmathsetcounter{n}{-2.7}"), -2.0));
    assert!(close(x(r"\pgfmathsetcounter{n}{1cm}"), 28.0));
    assert!(close(x(r"\pgfmathsetcounter{n}{5} \pgfmathaddtocounter{n}{-1.9}"), 4.0));
    assert!(close(x(r"\begingroup \pgfmathsetcounter{n}{9} \endgroup"), 0.0));
    assert!(close(x(r"\foreach \i in {1,2} {\pgfmathaddtocounter{n}{10}}"), 0.0));
    assert!(close(x(r"\foreach \i in {1,2} {\pgfmathaddtocounter{n}{10} \setcounter{n}{\then}}"), 20.0));
    // a loop on a counter that pgfmath moves ends
    let c = canvas(r"\newcounter{k} \whiledo{\value{k}<3}{\pgfmathsetcounter{k}{\value{k}+1} \draw (\thek,0) -- (\thek,1);}");
    assert_eq!(paths(&c).len(), 3);
    // lengths: a plain number is in points
    let c = canvas(r"\newlength{\l} \pgfmathsetlength{\l}{2cm+3} \pgfmathaddtolength{\l}{1pt*2} \draw (0,0) -- (\l,0);");
    assert!((points(&paths(&c)[0].segments)[1].0 - (2.0 + 5.0 / 72.27 * 2.54)).abs() < 1e-4);
    let (_, w) = canvas_and_warnings(r"\pgfmathsetcounter{nope}{1}");
    assert!(w.iter().any(|a| a.contains("no register or counter 'nope'")), "{w:?}");
}

#[test]
fn environments_and_end_without_begin() {
    // layers are drawn now, with no warning
    let (c, w) = canvas_and_warnings("\\begin{pgfonlayer}{background}\n\\draw (0,0) -- (1,0); \\end{pgfonlayer}");
    assert_eq!(paths(&c).len(), 1);
    assert!(w.is_empty(), "{w:?}");
    let (_, w) = canvas_and_warnings(r"\begin{scope} \end{other} \draw (0,0) -- (1,0); \end{scope}");
    assert!(w.iter().any(|a| a.contains("without a matching \\begin")), "{w:?}");
    let (_, w) = canvas_and_warnings(r"\begin{scope} \end{ \draw (0,0) -- (1,0);");
    assert!(w.iter().any(|a| a.contains("never closes")), "{w:?}");
}

#[test]
fn ignored_and_unknown_commands() {
    let (c, w) = canvas_and_warnings(
        r"\small \bfseries \centering \usetikzlibrary{calc} \usepackage[x]{y} \pgfplotsset{compat=1.18} \usepgfplotslibrary{fillbetween} \draw (0,0) -- (1,0);",
    );
    assert_eq!(paths(&c).len(), 1);
    assert!(w.is_empty(), "{w:?}");
    // \pic and \matrix are drawn now: what is left unknown is the pic type `x`
    let (_, w) = canvas_and_warnings(r"\shadow (0,0);  \pic {x}; \matrix {a};");
    assert!(w.iter().any(|a| a.contains("shadow")), "{w:?}");
    assert!(w.iter().any(|a| a.contains("'x'")), "{w:?}");
    let (_, w) = canvas_and_warnings("loose text here\n\\draw (0,0) -- (1,0);\n;\n");
    assert!(w.iter().any(|a| a.contains("text outside")), "{w:?}");
    // the same warning is not repeated
    let (_, w) = canvas_and_warnings(r"\shadow; \shadow;");
    assert_eq!(w.len(), 1);
}

#[test]
fn statement_without_semicolon_runs_to_the_end() {
    let c = canvas(r"\draw (0,0) -- (1,0)");
    assert_eq!(paths(&c).len(), 1);
}

#[test]
fn declare_function_and_globals() {
    let c = canvas(r"\begin{scope}[declare function={f(\x)=2*\x; g(\a,\b)=\a+\b; bad; noparens=1}] \draw (0,0) -- ({f(2)},{g(1,2)}); \end{scope}");
    assert!(close_to(points(&paths(&c)[0].segments)[1], (4.0, 3.0)));
    // a macro in the body is read when the function is used
    let c = canvas(r"\tikzset{total/.store in=\total, declare function={f(\q)=\q*\total;}, total=2} \draw (0,0) -- (f(1),0);");
    assert!(close_to(points(&paths(&c)[0].segments)[1], (2.0, 0.0)));
}

#[test]
fn foreach_items_in_braces_are_not_expanded() {
    // `{\a,\b}` is two items, whatever commas `\a` holds; the ends of a range are read
    let c = canvas(r"\def\a{(0,0) -- (1,0)}\def\b{(2,0) -- (3,0)} \foreach \p in {\a,\b} { \draw \p; } \foreach \p/\q in {\a/\b} { \draw \p \q; }");
    assert_eq!(paths(&c).len(), 3);
    for (list, n) in [(r"1,...,\e", 7), (r"1,\s,...,\e", 4), (r"\s,4,...,\e", 5)] {
        let c = canvas(&format!(r"\def\s{{3}}\def\e{{7}} \foreach \x in {{{list}}} {{ \draw (\x,0) -- (\x,1); }}"));
        assert_eq!(paths(&c).len(), n, "{list}");
    }
}

#[test]
fn line_widths_and_transforms() {
    assert!(close(first_path(r"\draw[thick] (0,0) -- (1,0);").stroke.unwrap().width, 0.8 * tikra::PT));
    let (_, w) = canvas_and_warnings(r"\draw[chunky] (0,0) -- (1,0);");
    assert!(w.iter().any(|a| a.contains("chunky")), "{w:?}");
    // singular matrix: identity
    assert_eq!(Transform::scaling(0.0, 1.0).inverse(), Transform::default());
    let t = Transform::rotation(30.0).compose(&Transform::translation(1.0, 2.0));
    let p = t.inverse().apply(t.apply(Point::new(3.0, 4.0)));
    assert!(close_to((p.x, p.y), (3.0, 4.0)));
    assert!(close(Transform::scaling(2.0, 8.0).factor(), 4.0));
}

#[test]
fn groups_and_comments() {
    // parentheses inside braces do not end a coordinate
    let d = first_path(r"\draw (0,0) -- ({(1)+1}, {max(1,2)});");
    assert!(close_to(points(&d.segments)[1], (2.0, 2.0)), "{:?}", d.segments);
    // a comment runs to the end of the line; \% is a percent sign
    let c = canvas("\\draw (0,0) -- (1,0); % \\draw (5,5) -- (6,6);\n\\node {50\\% off};");
    assert_eq!(paths(&c).len(), 1);
    assert_eq!(texts(&c).len(), 1);
    assert!(tikra::convert("% \\begin{tikzpicture} \\draw (0,0) -- (1,0); \\end{tikzpicture}").is_err(), "a picture in a comment is no picture");
}

#[test]
fn errors_inside_environments_and_loops_propagate() {
    for t in [r"\begin{whatever} \begin{scope}[scale=zz] \end{scope} \end{whatever}", r"\foreach \x in {1} { \begin{scope}[scale=zz] \end{scope} }"] {
        assert!(tikra::engine::run_document(&format!(r"\begin{{tikzpicture}}{t}\end{{tikzpicture}}")).is_err(), "{t}");
    }
}

#[test]
fn numexpr_and_dimexpr_in_expressions() {
    // e-TeX's expressions where pgfmath reads a number: in a coordinate, in \pgfmathsetmacro
    let d = first_path(r"\def\n{3}\newlength\u\setlength\u{1cm}\draw (0,0) -- (\numexpr\n+1\relax*\u, \the\numexpr 7/2\relax);");
    let end = points(&d.segments)[1];
    assert!((end.0 - 4.0).abs() < 1e-4 && end.1 == 4.0, "{:?}", d.segments);
    let d = first_path(r"\pgfmathsetmacro\v{2*\numexpr 1+1\relax}\draw (0,0) -- (\v, \dimexpr 1cm+10mm\relax);");
    assert!(close_to(points(&d.segments)[1], (4.0, 2.0)), "{:?}", d.segments);
}

#[test]
fn last_point_of_the_path() {
    // \pgfgetlastxy: the last point on the canvas, in pt; it outlives the scope that made it
    let end = |body: &str| points(&paths(&canvas(&format!(r"{body} \pgfgetlastxy{{\x}}{{\y}} \draw (5,5) -- (\x,\y);"))).last().unwrap().segments)[1];
    assert!(close_to(end(r"\path (2,1);"), (2.0, 1.0)));
    assert!(close_to(end(r"\begin{scope}[xshift=1cm] \path (1,1); \end{scope}"), (2.0, 1.0)));
    assert!(close_to(end(r"\path[scale=2] (1,1);"), (2.0, 2.0)));
    assert!(close_to(end(r"\draw (1,1) -- (1,2) -- cycle;"), (1.0, 1.0)));
    // the basic layer's path, used
    assert!(close_to(end(r"\pgfpathmoveto{\pgfpoint{1cm}{3cm}}\pgfusepath{discard}"), (1.0, 3.0)));
    // a new picture starts at the origin
    let (c, _) = tikra::engine::run_document(r"\tikz\path (2,1); \tikz{\pgfgetlastxy{\x}{\y}\draw (1,1) -- (\x,\y);}").unwrap();
    let (a, b) = bounds(&paths(&c).last().unwrap().segments);
    assert!(close(b.x - a.x, 1.0) && close(b.y - a.y, 1.0), "{a:?} {b:?}");
}

#[test]
fn tikz_checks_for_a_unit() {
    // \tikz@checkunit: a value with a unit, or a plain number (then a document may use \pgf@yy)
    let x = |v: &str| {
        let d = first_path(&format!(r"\makeatletter\tikz@checkunit{{{v}}}\iftikz@isdimension\def\u{{1}}\else\def\u{{2}}\fi\draw (0,0) -- (\u,0);"));
        points(&d.segments)[1].0
    };
    assert_eq!([x("1"), x("1cm"), x("2*3pt"), x(r"\pgflinewidth"), x("1+2")], [2.0, 1.0, 1.0, 1.0, 2.0]);
    let d = first_path(r"\makeatletter\begin{scope}[y=2cm]\pgfmathsetmacro\v{\pgf@yy/2+\pgf@yx}\draw (0,0) -- (\v pt,0);\end{scope}");
    assert!(close_to(points(&d.segments)[1], (1.0, 0.0)), "{:?}", d.segments);
}

#[test]
fn key_families() {
    // `/K/.is family`: the key `/K` goes to that path, for the keys after it
    let (c, w) = canvas_and_warnings(r"\pgfkeys{/K/.is family, /K, len/.initial=2, x/.store in=\x} \pgfkeys{/K, x=3} \draw (0,0) -- (\pgfkeysvalueof{/K/len},\x);");
    assert!(w.is_empty(), "{w:?}");
    assert!(close_to(points(&paths(&c)[0].segments)[1], (2.0, 3.0)));
}

#[test]
fn page_lengths_of_latex() {
    // the paper's size from the class options, in a coordinate; layout lengths set with no warning
    let size = |class: &str| {
        let doc = format!(
            r"\documentclass{class}\setlength\oddsidemargin{{0pt}}\setlength\parskip{{1ex}}\begin{{document}}\begin{{tikzpicture}}\draw (0,0) -- (.1\paperwidth,.1\paperheight);\end{{tikzpicture}}\end{{document}}"
        );
        let (c, w) = tikra::engine::run_document(&doc).unwrap();
        assert!(w.is_empty(), "{w:?}");
        let (a, b) = (c.min.unwrap(), c.max.unwrap());
        ((b.x - a.x) / tikra::PT, (b.y - a.y) / tikra::PT)
    };
    let (w, h) = size("{article}");
    assert!((w - 61.43).abs() < 0.5 && (h - 79.5).abs() < 0.5, "{w} {h}");
    let (w, h) = size("[a4paper,landscape]{article}");
    assert!((w - 84.5).abs() < 0.5 && (h - 59.75).abs() < 0.5, "{w} {h}");
    let (w, _) = size(r"{article}\usepackage[paper=a5paper]{geometry}");
    assert!((w - 42.11).abs() < 0.5, "{w}");
}
