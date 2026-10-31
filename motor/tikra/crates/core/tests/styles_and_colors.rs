//! Path options, xcolor colors and user styles (`\tikzset`, `\tikzstyle`).

mod common;
use common::*;
use std::collections::HashMap;
use tikra::canvas::{Paint, Stroke};
use tikra::color::{self, Color};

fn stroke(d: &Drawn) -> &Stroke {
    d.stroke.as_ref().unwrap_or_else(|| panic!("not stroked: {d:?}"))
}

fn fill_color(d: &Drawn) -> Option<String> {
    match &d.fill.as_ref()?.paint {
        Paint::Solid(c) => Some(c.hex()),
        _ => None,
    }
}

/// The arrow tips of `\draw[op] (0,0) -- (1,0);`: the paths drawn after the line, split by the end
/// they sit at (start, end), and the warnings.
fn tips(op: &str) -> (Vec<Drawn>, Vec<Drawn>, Vec<String>) {
    let (c, w) = canvas_and_warnings(&format!(r"\draw[{op}] (0,0) -- (1,0);"));
    let (start, end) = paths(&c).into_iter().skip(1).partition(|d| {
        let (a, b) = bounds(&d.segments);
        a.x + b.x < 1.0
    });
    (start, end, w)
}

/// The geometry of the tips of `op`, to compare two of them.
fn tip_shapes(op: &str) -> String {
    let (s, e, _) = tips(op);
    format!("{:?}", s.iter().chain(&e).map(|d| d.segments.clone()).collect::<Vec<_>>())
}

#[test]
fn dashes_widths_opacities() {
    for (op, n) in [
        ("dashed", 2),
        ("densely dashed", 2),
        ("loosely dashed", 2),
        ("dotted", 2),
        ("densely dotted", 2),
        ("loosely dotted", 2),
        ("dash dot", 4),
        ("dash pattern=on 2pt off 1pt on 1pt off 1pt", 4),
    ] {
        let t = first_path(&format!(r"\draw[{op}] (0,0) -- (1,0);"));
        assert_eq!(stroke(&t).dash.as_ref().map(|d| d.0.len()), Some(n), "{op}");
    }
    assert!(stroke(&first_path(r"\draw[dashed, solid] (0,0) -- (1,0);")).dash.is_none());
    // no space needed after `on` and `off`
    let d = stroke(&first_path(r"\draw[dash pattern=on5pt off3pt on 1pt  off2pt] (0,0) -- (1,0);")).dash.clone().unwrap();
    assert_eq!(d.0.iter().map(|x| (x / tikra::PT).round()).collect::<Vec<_>>(), [5.0, 3.0, 1.0, 2.0]);
    // a dash that is not a length: TeX stops on it; the statement is dropped, with a warning
    let (c, w) = canvas_and_warnings(r"\draw[dash pattern=on zz] (0,0) -- (1,0);");
    assert!(paths(&c).is_empty() && !w.is_empty(), "{w:?}");
    for (op, w) in [("ultra thin", 0.1), ("very thin", 0.2), ("thin", 0.4), ("semithick", 0.6), ("thick", 0.8), ("very thick", 1.2), ("ultra thick", 1.6), ("line width=2pt", 2.0)]
    {
        let t = first_path(&format!(r"\draw[{op}] (0,0) -- (1,0);"));
        assert!(close(stroke(&t).width, w * tikra::PT), "{op}");
    }
    let t = first_path(r"\filldraw[opacity=0.5] (0,0) rectangle (1,1);");
    assert!(close(stroke(&t).opacity, 0.5) && close(t.fill.unwrap().opacity, 0.5));
    let t = first_path(r"\filldraw[draw opacity=0.2, fill opacity=0.3] (0,0) rectangle (1,1);");
    assert!(close(stroke(&t).opacity, 0.2) && close(t.fill.unwrap().opacity, 0.3));
    // values that are not numbers: dropped with the statement, with a warning, as TeX stops on them
    let (c, w) = canvas_and_warnings(r"\filldraw[opacity=zz, draw opacity=zz, fill opacity=zz, line width=zz] (0,0) rectangle (1,1);");
    assert!(paths(&c).is_empty() && !w.is_empty(), "{w:?}");
}

#[test]
fn stroke_and_fill_colors() {
    let t = first_path(r"\filldraw[draw=red, fill=blue!20] (0,0) rectangle (1,1);");
    assert_eq!(stroke(&t).color.hex(), "#ff0000");
    assert_eq!(fill_color(&t).unwrap(), "#ccccff");
    let t = first_path(r"\draw[fill] (0,0) rectangle (1,1);");
    assert!(t.fill.is_some());
    let t = first_path(r"\filldraw[fill=none, color=green] (0,0) rectangle (1,1);");
    assert!(t.fill.is_none());
    assert_eq!(stroke(&t).color.hex(), "#00ff00");
    let t = first_path(r"\fill[draw] (0,0) rectangle (1,1);");
    assert!(t.stroke.is_some());
    // color= with an unknown name changes nothing
    let t = first_path(r"\draw[color=noname] (0,0) -- (1,0);");
    assert_eq!(stroke(&t).color.hex(), "#000000");
    // shade and shadedraw fill with a shading
    assert!(matches!(first_path(r"\shade (0,0) rectangle (1,1);").fill.unwrap().paint, Paint::Shading(_)));
    assert!(first_path(r"\shadedraw (0,0) rectangle (1,1);").stroke.is_some());
}

#[test]
fn hatchings() {
    for p in [
        "north east lines",
        "north west lines",
        "horizontal lines",
        "vertical lines",
        "grid",
        "crosshatch",
        "dots",
        "crosshatch dots",
        "fivepointed stars",
        "sixpointed stars",
        "bricks",
    ] {
        let r = no_warnings(&format!(r"\begin{{tikzpicture}} \draw[pattern={p}, pattern color=red] (0,0) rectangle (1,1); \end{{tikzpicture}}"));
        assert!(r.svg.contains("<pattern"), "{p}");
    }
    let t = first_path(r"\path[pattern=dots, pattern color=nothing] (0,0) rectangle (1,1);");
    match t.fill.unwrap().paint {
        Paint::Pattern { color, .. } => assert_eq!(color.hex(), "#000000"),
        p => panic!("{p:?}"),
    }
}

#[test]
fn arrow_tips() {
    // how many tips sit at the start and at the end
    for (op, start, end) in [
        ("->", 0, 1),
        ("<-", 1, 0),
        ("<->", 1, 1),
        ("-latex", 0, 1),
        ("-Latex", 0, 1),
        ("-{Latex[length=3mm]}", 0, 1),
        ("-{Stealth[length=2mm]}", 0, 1),
        ("-Triangle", 0, 1),
        ("-stealth", 0, 1),
        ("-Kite", 0, 1),
        ("|-|", 1, 1),
        ("o-*", 1, 1),
        ("-to", 0, 1),
        ("Bar-Circle", 1, 1),
    ] {
        let (s, e, w) = tips(op);
        assert!(w.is_empty(), "{op}: {w:?}");
        assert_eq!((s.len(), e.len()), (start, end), "{op}");
    }
    // a longer Latex is another tip
    assert_ne!(tip_shapes("-{Latex[length=3mm]}"), tip_shapes("-Latex"));
    // >=Stealth changes what '>' means
    let c = canvas(r"\begin{scope}[>=Stealth] \draw[->] (0,0) -- (1,0); \end{scope}");
    let (_, stealth, _) = tips("-Stealth");
    assert_eq!(format!("{:?}", paths(&c)[1].segments), format!("{:?}", stealth[0].segments));
    // an unknown tip and a lone '-': not an arrow
    for op in ["-Nothing", "-", "Nothing-"] {
        let (s, e, _) = tips(op);
        assert!(s.is_empty() && e.is_empty(), "{op}");
    }
}

#[test]
fn every_tip_in_the_svg() {
    for op in ["->", "<-", "-latex", "latex-", "-stealth", "stealth-", "|-|", "o-*"] {
        let r = no_warnings(&format!(
            r"\begin{{tikzpicture}} \draw[{op}] (0,0) -- (1,0); \draw[{op}] (0,1) .. controls (1,2) and (2,2) .. (3,1); \draw[{op}] (0,2) .. controls (0,2) and (0,2) .. (1,3); \end{{tikzpicture}}"
        ));
        // a tip: a filled path, or a chevron or bar stroked on its own
        assert!(r.svg.contains(r##"fill="#000000"/>"##) || r.svg.contains(r#"stroke-linecap="round""#) || r.svg.matches("<path").count() > 3, "{op}");
    }
    // an arrow on a closed path
    no_warnings(r"\begin{tikzpicture} \draw[->] (0,0) -- (1,0) -- (1,1) -- cycle; \draw[<-] (0,0) circle (1); \end{tikzpicture}");
    // a curve whose second control point is its end
    no_warnings(r"\begin{tikzpicture} \draw[-latex] (0,0) .. controls (1,1) and (2,0) .. (2,0); \draw[latex-] (0,0) .. controls (0,0) and (1,1) .. (2,0); \end{tikzpicture}");
}

#[test]
fn xcolor_parsing() {
    let d = HashMap::new();
    assert_eq!(color::parse("-red", &d).unwrap().hex(), "#00ffff");
    assert_eq!(color::parse("--red", &d).unwrap().hex(), "#ff0000");
    assert!(color::parse("", &d).is_none());
    assert!(color::parse("-", &d).is_none());
    assert!(color::parse("red!xx", &d).is_none());
    assert!(color::parse("red!50!nothing", &d).is_none());
    for name in [
        "black",
        "white",
        "green",
        "cyan",
        "magenta",
        "yellow",
        "gray",
        "darkgray",
        "lightgray",
        "brown",
        "lime",
        "olive",
        "orange",
        "pink",
        "purple",
        "teal",
        "violet",
        "ForestGreen",
        "NavyBlue",
        "RoyalBlue",
        "Maroon",
        "Goldenrod",
        "Gray",
    ] {
        assert!(color::parse(name, &d).is_some(), "{name}");
    }
    assert_eq!(Color::rgb(2.0, -1.0, 0.5).hex(), "#ff0080", "values outside 0..1 are clamped");
}

#[test]
fn xcolor_named_sets() {
    use color::sets::Set;
    let d = HashMap::new();
    let hex = |e: &str| color::parse(e, &d).unwrap().hex();
    // as \extractcolorspecs gives them: the dvipsnames in cmyk, drawn as Ghostscript draws cmyk
    let set = |s: Set, n: &str| s.color(n).unwrap();
    assert_eq!(set(Set::Dvips, "BrickRed").model, color::Model::Cmyk([0.0, 0.89, 0.94, 0.28]));
    assert_eq!(set(Set::Svg, "DimGray"), Color::rgb(0.41, 0.41, 0.41));
    assert_eq!(set(Set::X11, "Red3"), Color::rgb(0.804, 0.0, 0.0));
    assert_eq!(set(Set::X11, "Gold2"), Color::rgb(0.932, 0.79, 0.0));
    // every color of a set is found by its name
    for (s, n) in [(Set::Dvips, 68), (Set::Svg, 151), (Set::X11, 317)] {
        let colors = s.colors();
        assert_eq!(colors.len(), n);
        for (name, c) in colors {
            assert_eq!(s.color(name), Some(c), "{name}");
        }
    }
    assert_eq!(Set::Svg.color("Red3"), None);
    assert_eq!(Set::of_option(" x11names "), Some(Set::X11));
    assert_eq!(Set::of_option("usenames"), None);
    // in a document that loads no set (where TeX stops), the dvipsnames first, then the others
    assert_eq!(hex("Gray"), "#939598");
    assert_eq!(hex("DimGray"), "#696969");
    assert_eq!(hex("Red3!50"), "#e68080");
    assert!(color::parse("Red5", &d).is_none());
}

#[test]
fn color_definitions() {
    assert_eq!(color::define("rgb", "1,0.5,0").unwrap().hex(), "#ff8000");
    assert_eq!(color::define("RGB", "255,128,0").unwrap().hex(), "#ff8000");
    assert_eq!(color::define("gray", "0.5").unwrap().hex(), "#808080");
    // drawn as Ghostscript draws cmyk, through its ICC profiles
    assert_eq!(color::define("cmyk", "0,1,1,0").unwrap().hex(), "#ed1c24");
    assert_eq!(color::define("HTML", "#00FF00").unwrap().hex(), "#00ff00");
    assert!(color::define("HTML", "12").is_none());
    assert!(color::define("HTML", "ZZZZZZ").is_none());
    assert!(color::define("rgb", "1,2").is_none());
    assert!(color::define("hsb", "1,2,3").is_none());
}

#[test]
fn html_too_short() {
    assert!(color::define("HTML", "1122").is_none());
}

#[test]
fn definecolor_and_colorlet() {
    let c = canvas(
        r"\definecolor{a}{HTML}{112233} \providecolor{b}{rgb}{0,0,1} \colorlet{c}{a!50!b} \colorlet{d}{missing}
          \draw[c] (0,0) -- (1,0);",
    );
    assert_eq!(stroke(&paths(&c)[0]).color.hex(), "#091199");
    let (_, w) = canvas_and_warnings(r"\definecolor{x}{hsb}{1,1,1}");
    assert!(w.iter().any(|a| a.contains("model not supported")), "{w:?}");
}

#[test]
fn user_styles() {
    let c = canvas(
        r"\tikzset{base/.style={thick}, base/.append style={red}, fresh/.append style={dashed}, every path/.style={blue}}
          \tikzstyle{box}=[draw, fill=yellow]
          \tikzstyle{loose}[circle]
          \draw[base] (0,0) -- (1,0);
          \draw[fresh] (0,0) -- (1,0);
          \draw (0,0) -- (1,0);
          \node[box] {x};",
    );
    let t = paths(&c);
    assert_eq!(stroke(&t[0]).color.hex(), "#ff0000");
    assert!(close(stroke(&t[0]).width, 0.8 * tikra::PT));
    assert!(stroke(&t[1]).dash.is_some());
    assert_eq!(stroke(&t[2]).color.hex(), "#0000ff", "every path");
    // the node's border, in xcolor's yellow, a cmyk color
    assert_eq!(fill_color(&t[3]).as_deref(), Some("#fff200"));
}

#[test]
fn tikzstyle_without_braces_and_set_style() {
    let c = canvas(
        r"\tikzstyle my style=[red] \tikzstyle my style+=[dashed]
          \draw[my style] (0,0) -- (1,0);
          \begin{scope}[set style={{help lines}+=[dashed]}, set style={{a}=[blue]}, set style={{a}+=[thick]}]
          \draw[help lines] (0,0) -- (1,0); \draw[a] (0,0) -- (1,0);
          \end{scope}
          \draw[help lines] (0,0) -- (1,0);",
    );
    let t = paths(&c);
    // without braces, the name runs to the `=`: `my style+` is another style
    assert_eq!(stroke(&t[0]).color.hex(), "#ff0000");
    assert!(stroke(&t[0]).dash.is_none());
    // `{name}+=[…]` adds to a style, `{name}=[…]` defines one, for the scope
    assert_eq!(stroke(&t[1]).color.hex(), "#808080");
    assert!(stroke(&t[1]).dash.is_some());
    assert_eq!(stroke(&t[2]).color.hex(), "#0000ff");
    assert!(close(stroke(&t[2]).width, 0.8 * tikra::PT));
    assert!(stroke(&t[3]).dash.is_none());
}

#[test]
fn every_node_applies_to_all() {
    let c = canvas(r"\tikzset{every node/.style={draw, circle}} \node {a}; \node at (1,0) {b};");
    let borders = paths(&c);
    assert_eq!(borders.len(), 2);
    assert!(borders.iter().all(|n| n.stroke.is_some()));
}

#[test]
fn option_lists_and_braces() {
    use tikra::keys::{split_top_level, strip_braces};
    assert_eq!(split_top_level("a, {b, c}, (d, e), [f, g], ,"), ["a", "{b, c}", "(d, e)", "[f, g]"]);
    assert_eq!(strip_braces("{a}"), "a");
    assert_eq!(strip_braces("{a}{b}"), "{a}{b}", "the opening brace closes before the end");
    assert_eq!(strip_braces("{"), "{");
    assert_eq!(strip_braces("x"), "x");
}

#[test]
fn pattern_replaces_the_solid_fill() {
    let t = first_path(r"\fill[pattern=north east lines] (0,0) rectangle (1,1);");
    assert!(matches!(t.fill.unwrap().paint, Paint::Pattern { .. }));
    let c = canvas(r"\filldraw[fill=yellow, pattern=dots] (0,0) rectangle (1,1);");
    let fills: Vec<_> = paths(&c).into_iter().filter_map(|d| d.fill).map(|f| f.paint).collect();
    assert!(fills.iter().any(|p| matches!(p, Paint::Pattern { .. })), "{fills:?}");
}

#[test]
fn double_tip() {
    let (s, e, _) = tips("->>");
    assert_eq!((s.len(), e.len()), (0, 2), "two tips at the end");
    let (s, e, _) = tips("<<-");
    assert_eq!((s.len(), e.len()), (2, 0), "two tips at the start");
}

#[test]
fn radius_option_is_not_the_value_of_the_radius_key() {
    // TikZ's `radius` is code setting `x radius` and `y radius`: a pattern variable
    // `/tikz/radius` keeps the value `.initial` gave it, whatever `radius=` the path has
    let body = r"\pgfdeclarepatternformonly[/tikz/radius]{rings}{\pgfpoint{-10pt}{-10pt}}{\pgfpoint{10pt}{10pt}}{\pgfpoint{20pt}{20pt}}
      {\pgfpathcircle\pgfpointorigin{\pgfkeysvalueof{/tikz/radius}}\pgfusepath{stroke}}
      \tikzset{radius/.initial=4pt}
      \fill[pattern=rings, radius=2pt] (0,0) rectangle (1,1);";
    let t = first_path(body);
    let Paint::Tile { items, .. } = t.fill.unwrap().paint else { panic!("not a tile") };
    let c = tikra::canvas::Canvas { items: items.0.to_vec(), ..Default::default() };
    let ring = &paths(&c)[0];
    let (lo, hi) = bounds(&ring.segments);
    assert!(close(hi.x - lo.x, 8.0 * tikra::PT), "{lo:?} {hi:?}");
    // and a circle's radius is still the option's
    let t = first_path(r"\tikzset{radius/.initial=4pt}\draw (0,0) circle [radius=1];");
    let (lo, hi) = bounds(&t.segments);
    assert!(close(hi.x - lo.x, 2.0), "{lo:?} {hi:?}");
}

#[test]
fn pgfmath_picks_a_quoted_string() {
    // a color chosen by a condition, as in `\pgfmathparse{Mod(\k,2) ? "pink" : "purple!50"}`
    let r = no_warnings(
        r#"\begin{tikzpicture}
            \pgfmathparse{Mod(3,2) ? "red" : "blue!50"}\draw[\pgfmathresult] (0,0) -- (1,0);
            \pgfmathparse{0 ? "red" : 1 == 2 ? "green" : "blue"}\draw[\pgfmathresult] (0,1) -- (1,1);
            \pgfmathsetmacro{\c}{2 > 1 ? "orange" : "black"}\draw[\c] (0,2) -- (1,2);
          \end{tikzpicture}"#,
    );
    for c in ["#ff0000", "#0000ff", "#ff8000"] {
        assert!(r.svg.contains(&format!("stroke=\"{c}\"")), "{c}");
    }
    // a `?` inside the first branch, and parentheses around the condition
    let r = no_warnings(r#"\begin{tikzpicture}\pgfmathparse{(1) ? (0 ? "red" : "green") : "blue"}\draw[\pgfmathresult] (0,0) -- (1,0);\end{tikzpicture}"#);
    assert!(r.svg.contains("stroke=\"#00ff00\""));
    let r = no_warnings(r#"\begin{tikzpicture}\pgfmathparse{1 ? 0 ? "red" : "green" : "blue"}\draw[\pgfmathresult] (0,0) -- (1,0);\end{tikzpicture}"#);
    assert!(r.svg.contains("stroke=\"#00ff00\""));
    // parentheses at both ends that are not one pair
    let r = no_warnings(r#"\begin{tikzpicture}\pgfmathparse{(0) ? "red" : ("blue")}\draw[\pgfmathresult] (0,0) -- (1,0);\end{tikzpicture}"#);
    assert!(r.svg.contains("stroke=\"#0000ff\""));
    // a string in arithmetic is not a number
    with_warning(r#"\begin{tikzpicture}\pgfmathparse{"red" + 1}\end{tikzpicture}"#, "a string where a number is expected");
}

#[test]
fn pgfmath_mod_as_a_command() {
    // `\pgfmathMod{-7}{3}` is 2, as `Mod(-7,3)`
    let at = |x: &str| no_warnings(&format!(r"\begin{{tikzpicture}}{x}\draw (0,0) -- (\pgfmathresult,0);\end{{tikzpicture}}")).svg;
    let path = |svg: String| svg[svg.find("<path").unwrap()..].to_string();
    assert_eq!(path(at(r"\pgfmathMod{-7}{3}")), path(at(r"\pgfmathparse{2}")));
}

#[test]
fn a_shift_reads_length_registers() {
    // as a coordinate on the path does: `\newdimen\R` in `shift={(0,\R)}`
    let shifted = |o: &str| no_warnings(&format!(r"\newdimen\R \R=1cm \begin{{tikzpicture}}\draw[{o}] (0,0) -- (1,0);\end{{tikzpicture}}")).svg;
    let path = |svg: String| svg[svg.find("<path").unwrap()..].to_string();
    assert_eq!(path(shifted(r"shift={(0,\R)}")), path(shifted("shift={(0,1cm)}")));
    assert_eq!(path(shifted(r"shift={(-60:{2*\R})}")), path(shifted("shift={(-60:2cm)}")));
}
