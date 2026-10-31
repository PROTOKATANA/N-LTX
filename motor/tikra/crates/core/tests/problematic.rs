//! Input crafted to break the interpreter. Each must end quickly with a clear error or a
//! warning: never a panic, an infinite loop or memory that blows up.

mod common;
use std::sync::mpsc;
use std::time::Duration;

/// Runs `tikz` on a thread with a 5 s deadline: an infinite loop fails the test instead of hanging the suite.
fn convert_with_deadline(tikz: &str) -> Result<tikra::Output, tikra::Error> {
    let (tx, rx) = mpsc::channel();
    let t = tikz.to_string();
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let _ = tx.send(tikra::convert(&t));
        })
        .unwrap();
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(r) => r,
        Err(mpsc::RecvTimeoutError::Timeout) => panic!("hung (more than 5 s): {}", &tikz[..tikz.len().min(200)]),
        Err(mpsc::RecvTimeoutError::Disconnected) => panic!("panic: {}", &tikz[..tikz.len().min(200)]),
    }
}

fn figure(body: &str) -> String {
    format!("\\begin{{tikzpicture}}\n{body}\n\\end{{tikzpicture}}")
}

/// Ends with an SVG and a warning containing `fragment`.
fn warns(body: &str, fragment: &str) -> tikra::Output {
    let r = convert_with_deadline(&figure(body)).unwrap_or_else(|e| panic!("expected a warning, got error {e}: {body}"));
    assert!(r.warnings.iter().any(|a| a.contains(fragment)), "expected a warning with '{fragment}', got {:?}: {body}", r.warnings);
    assert!(r.svg.starts_with("<svg"));
    assert!(!r.svg.contains("NaN") && !r.svg.contains("inf"), "invalid number in the SVG: {body}");
    r
}

/// Ends with an error containing `fragment`.
fn fails(input: &str, fragment: &str) {
    match convert_with_deadline(input) {
        Err(e) => assert!(e.to_string().contains(fragment), "expected an error with '{fragment}', got '{e}'"),
        Ok(r) => panic!("expected an error, got an SVG with warnings {:?}", r.warnings),
    }
}

/// Ends, with or without a warning, and the SVG has no invalid number.
fn finishes(body: &str) -> tikra::Output {
    let r = convert_with_deadline(&figure(body)).unwrap_or_else(|e| panic!("{e}: {body}"));
    assert!(!r.svg.contains("NaN") && !r.svg.contains("inf"), "invalid number in the SVG: {body}");
    r
}

#[test]
fn empty_input_or_only_comments() {
    fails("", "no tikzpicture");
    fails("% just a comment\n%% and another", "no tikzpicture");
    let r = finishes("% nothing\n");
    assert!(r.svg.starts_with("<svg"), "an empty figure is a minimal SVG");
    fails("   \n\t  ", "no tikzpicture");
}

#[test]
fn unbalanced_braces_brackets_and_parentheses() {
    warns(r"\draw[red (0,0) -- (1,1);", "unmatched bracket");
    warns(r"\draw (0,0 -- (1,1);", "unmatched parenthesis");
    warns(r"\node {unclosed;", "unmatched brace");
    finishes(r"\draw (0,0) -- (1,1); }}}}");
    finishes(r"\foreach \x in {1,2 { \draw (\x,0) -- (1,1); }");
    finishes(r"\begin{scope} \draw (0,0) -- (1,1);");
    finishes(r"\end{scope} \draw (0,0) -- (1,1);");
    finishes(r"\begin{tikzpicture} \begin{tikzpicture}");
}

#[test]
fn huge_and_nested_foreach() {
    // 10 thousand × 10 thousand iterations: the statement cap cuts it off
    fails(&figure(r"\foreach \i in {1,...,10000} { \foreach \j in {1,...,10000} { \draw (\i,\j) -- (0,0); } }"), "too large");
    // an ellipsis with a tiny step (ten billion items): a cap cuts it off, quickly and with an error
    fails(&figure(r"\foreach \x in {0,0.0000001,...,1000} { \draw (\x,0) -- (0,0); }"), "too large");
    // deep \foreach nesting
    let deep = "\\foreach \\x in {1} {".repeat(200) + &"}".repeat(200);
    fails(&figure(&deep), "nesting");
}

#[test]
fn scopes_nested_too_deep() {
    let deep = "\\begin{scope}".repeat(5_000) + &"\\end{scope}".repeat(5_000);
    fails(&figure(&deep), "nesting");
}

#[test]
fn styles_that_call_each_other() {
    finishes(r"\tikzset{a/.style={b}, b/.style={a}} \draw[a] (0,0) -- (1,1);");
    finishes(r"\tikzset{a/.style={a, red}} \draw[a] (0,0) -- (1,1);");
    // each level multiplies by 4: with no cap there would be 4^20 options
    let mut defs = String::new();
    for k in 0..20 {
        defs.push_str(&format!("s{k}/.style={{s{0},s{0},s{0},s{0}}},", k + 1));
    }
    let r = finishes(&format!(r"\tikzset{{{defs}}} \draw[s0] (0,0) -- (1,1);"));
    assert!(r.svg.contains("<path"));
}

#[test]
fn recursive_macros() {
    // \a becomes \a\a: it doubles on each pass; the expansion cap holds it
    warns(r"\def\a{\a\a} \draw (\a,0) -- (1,1);", "statement ignored");
    warns(r"\def\a{\a\a\a\a\a\a\a\a} \draw (\a,0) -- (1,1);", "statement ignored");
    warns(r"\def\a{\b} \def\b{\a} \draw (\a,0) -- (1,1);", "statement ignored");
    // \def at the end of the text, with no name
    finishes(r"\draw (0,0) -- (1,1); \def");
    finishes(r"\draw (0,0) -- (1,1); \newcommand");
}

#[test]
fn recursive_functions_and_deep_expressions() {
    warns(r"\begin{scope}[declare function={f(\x)=f(\x)+1;}] \draw (0,0) -- ({f(1)},0); \end{scope}", "nesting");
    let parens = "(".repeat(10_000) + "1" + &")".repeat(10_000);
    warns(&format!(r"\draw (0,0) -- ({{{parens}}},0);"), "nesting");
    let signs = "-".repeat(10_000) + "1";
    warns(&format!(r"\draw (0,0) -- ({{{signs}}},0);"), "nesting");
    // a long power is read in a loop, without deepening the stack
    let power = vec!["1"; 10_000].join("^");
    finishes(&format!(r"\draw (0,0) -- ({{{power}}},0);"));
    let color = "-".repeat(100_000) + "red";
    finishes(&format!(r"\draw[{color}] (0,0) -- (1,1);"));
}

#[test]
fn division_by_zero_nan_and_infinity() {
    warns(r"\draw (0,0) -- ({1/0},0);", "not finite");
    warns(r"\draw (0,0) -- ({0/0},0);", "not finite");
    warns(r"\draw (0,0) -- ({ln(0)},1);", "not finite");
    warns(r"\draw (0,0) -- ({sqrt(-1)},1);", "not finite");
    warns(r"\node at ({1/0},0) {x};", "not finite");
    warns(r"\draw (0,0) circle ({1/0});", "not finite");
    warns(r"\draw (0,0) arc (0:{0/0}:1);", "not finite");
    warns(r"\pgfmathsetmacro{\z}{1/0} \draw (0,0) -- (\z,1);", "non-finite result");
    warns(r"\clip (0,0) -- ({1/0},0) -- (1,1); \draw (0,0) -- (1,1);", "not finite");
    finishes(r"\draw[scale=0] (0,0) -- (1,1);");
}

#[test]
fn giant_coordinates() {
    warns(r"\draw (0,0) -- (1e300,1e300);", "beyond");
    warns(r"\draw (0,0) -- (200000,0);", "beyond");
    warns(r"\node[minimum width=1e9cm] {x};", "beyond");
    // at the limit it still draws
    finishes(r"\draw (0,0) -- (99999,0);");
}

#[test]
fn odd_radii_and_angles() {
    finishes(r"\draw (0,0) circle (-1);");
    finishes(r"\draw (0,0) circle (0);");
    finishes(r"\draw (1,0) arc (0:0:1);");
    finishes(r"\draw (1,0) arc (0:90:0);");
    finishes(r"\draw (1,0) arc (0:90:-1);");
    // a billion degrees: one turn plus the rest is kept, without allocating millions of curves
    let r = finishes(r"\draw (1,0) arc (0:1000000000:1);");
    assert!(r.svg.len() < 10_000, "{}", r.svg.len());
    finishes(r"\node[circle, draw, inner sep=0pt] (a) {}; \draw (a) -- (1,1);");
}

#[test]
fn grid_and_decoration_with_zero_step() {
    warns(r"\draw[step=0] (0,0) grid (1,1);", "grid");
    warns(r"\draw (0,0) grid [step=-1] (1,1);", "grid");
    warns(r"\draw (0,0) grid [step=0.000001] (100,100);", "grid");
    finishes(r"\draw[decorate, decoration={zigzag, segment length=0pt}] (0,0) -- (1,0);");
    finishes(r"\draw[decorate, decoration={zigzag, segment length=-1pt}] (0,0) -- (1,0);");
    let r = finishes(r"\draw[decorate, decoration={zigzag, segment length=0.0000001pt}] (0,0) -- (1,0);");
    assert!(r.svg.len() < 2_000_000, "{}", r.svg.len());
}

#[test]
fn textless_nodes_missing_names_and_unicode() {
    finishes(r"\node {}; \node[draw] {}; \node[circle, draw] at (1,1) {};");
    warns(r"\draw (a) -- (b);", "unknown coordinate");
    warns(r"\node[below=of zz] {x};", "zz");
    warns(r"\draw ($(a)!0.5!(b)$) -- (1,1);", "unknown coordinate");
    let r = finishes(r"\node {Olá, ção — ∑ 😀 中文}; \node[draw] (é) at (2,0) {é}; \draw (é) -- (0,0);");
    assert!(r.svg.contains("<use"));
    finishes(r"\begin{environment-ção} \draw (0,0) -- (1,1); \end{environment-ção}");
    finishes(r"\end{ção} \draw (0,0) -- (1,1);");
}

#[test]
fn odd_samples_and_domains() {
    finishes(r"\draw[domain=0:1, samples=0] plot (\x, \x);");
    finishes(r"\draw[domain=0:1, samples=1] plot (\x, \x);");
    finishes(r"\draw[domain=0:1, samples=-5] plot (\x, \x);");
    finishes(r"\draw[domain=1:0] plot (\x, \x);");
    finishes(r"\draw[domain=0:0] plot (\x, \x);");
    let r = finishes(r"\draw plot[domain=0:1, samples=1000000000] (\x, \x);");
    assert!(r.svg.len() < 200_000, "the 2000-sample cap also applies to the plot options: {}", r.svg.len());
    finishes(r"\draw[domain=0:1, samples=1000000000] plot (\x, \x);");
    finishes(r"\begin{axis}[samples=0] \addplot {x}; \end{axis}");
    finishes(r"\begin{axis}[samples=1000000000] \addplot {x}; \end{axis}");
    finishes(r"\begin{axis}[domain=1:0] \addplot {x}; \end{axis}");
}

#[test]
fn degenerate_pgfplots_windows() {
    finishes(r"\begin{axis}[xmin=1, xmax=1] \addplot coordinates {(1,1)}; \end{axis}");
    finishes(r"\begin{axis}[xmin=2, xmax=1, ymin=5, ymax=-5] \addplot {x}; \end{axis}");
    finishes(r"\begin{axis}[ymin=1e17, ymax=100000000000000016] \addplot {x}; \end{axis}");
    finishes(r"\begin{axis} \addplot coordinates {(1,1)}; \end{axis}");
    finishes(r"\begin{axis}[xmin={1/0}] \addplot {x}; \end{axis}");
}

#[test]
fn minimal_paths() {
    finishes(r"\draw (0,0);");
    finishes(r"\draw[->] (0,0);");
    finishes(r"\draw[->] (0,0) -- (0,0);");
    finishes(r"\draw -- cycle;");
    finishes(r"\draw cycle;");
    finishes(r"\draw[rounded corners] (0,0) -- cycle;");
    finishes(r"\draw;");
    finishes(r"\draw -- -- --;");
    finishes(r"\draw rectangle (1,1);");
    finishes(r"\fill circle (1);");
    finishes(r"\draw (0,0) to;");
    finishes(r";;;;");
}

#[test]
fn invalid_text_inside_a_label() {
    // mathra gets the text inside \text{…}: braces that close early become label warnings
    let r = finishes(r"\node {a}b{c};");
    assert!(r.svg.starts_with("<svg"));
    finishes(r"\node {$\frac{1}{$};");
    finishes(r"\node {\\\\\\};");
    // mathra refuses nesting past its limit even in tolerant mode: the label goes away, with a warning
    let deep = format!("\\node {{${}x{}$}};", "{".repeat(100), "}".repeat(100));
    warns(&deep, "not typeset");
    // an impossible division mathra now writes inline, with its own warning
    warns(r"\node {$\divchave{10}{0}$};", "E-REC-9");
}

#[test]
fn png_of_extreme_figures() {
    let r = finishes(r"\draw (0,0) -- (99999,99999);");
    let e = tikra::png(&r.svg, 300.0, true).unwrap_err();
    assert!(e.to_string().contains("too large"), "{e}");
    let r = finishes(r"\draw (0,0) -- (0.0001,0);");
    assert!(tikra::png(&r.svg, 300.0, true).is_ok());
}

#[test]
fn unclosed_groups_never_hang() {
    // a group that never closes, at each place a group is read; found by tools/conformance
    for body in [
        r"\begin{axis}\addplot {x}; \addlegendentry{a\end{axis}",
        r"\begin{axis}\addplot {x}; \addlegendentry\end{axis}",
        r"\begin{axis}\addplot[red {x};\end{axis}",
        r"\begin{axis}\addplot {x^2;\end{axis}",
        r"\begin{axis}\addplot coordinates {(1,2) (3,4};\end{axis}",
        r"\draw[red (0,0) -- (1,1);",
        r"\draw (0,0) -- (1,1) node[red {a};",
        r"\draw (0,0) to[bend left (1,1);",
        r"\draw (0,0) .. controls (1,1 .. (2,0);",
        r"\draw plot[smooth coordinates {(0,0) (1,1)};",
        r"\draw plot coordinates {(0,0) (1,1};",
        r"\draw (0,0) arc[start angle=0, end angle=90, radius=1;",
        r"\node {a;",
    ] {
        let _ = convert_with_deadline(&figure(body));
    }
}

#[test]
fn expanded_legend_entries() {
    // pgfplots manual: \addlegendentryexpanded inside a \foreach once looped forever; it draws as \addlegendentry
    let drawing = |legend: &str| {
        let r = convert_with_deadline(&figure(&format!(r"\begin{{axis}}\addplot {{x^2}}; {legend}{{$x^2$}}\end{{axis}}"))).unwrap();
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        // the drawing: not the words' ids (they come from the source) nor the source
        let (head, tail) = (r.svg.find(" aria-labelledby").unwrap(), r.svg.find("</metadata>").unwrap());
        format!("{}{}", &r.svg[..head], &r.svg[tail..])
    };
    assert_eq!(drawing(r"\addlegendentryexpanded"), drawing(r"\addlegendentry"));
}
