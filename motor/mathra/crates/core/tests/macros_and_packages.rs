//! Substitution macros (L-MAC-1), package commands and the boundary of what mathra does not do (spec/boundary.md).

use mathra::tree::{ArrowHead, ArrowLine, ArrowTail, LabelSide, Node, TableKind};
use mathra::{convert, Locale, Options};

fn pt(tex: &str) -> mathra::Output {
    convert(tex, &Options { locale: Locale::PtBr, ..Default::default() }).unwrap_or_else(|e| panic!("{tex}: {e}"))
}

fn error(tex: &str) -> String {
    convert(tex, &Options { strict: true, ..Default::default() }).err().map(|e| e.to_string()).unwrap_or_else(|| panic!("{tex} should fail"))
}

#[test]
fn l_mac_1_newcommand_with_argument_and_default() {
    assert_eq!(pt(r"\newcommand{\vet}[1]{\vec{#1}} \vet{F}").speech, "vetor F");
    assert_eq!(pt(r"\newcommand{\pot}[2][2]{#2^{#1}} \pot{x} + \pot[3]{y}").speech, "x ao quadrado mais y ao cubo");
    assert!(pt(r"\newcommand\um{1} \um").applied.iter().any(|a| a.rule == "L-MAC-1"));
}

#[test]
fn l_mac_1_def_let_and_renewcommand() {
    assert_eq!(pt(r"\def\q#1{#1^2} \q{x}").text, "x^2");
    assert_eq!(pt(r"\let\foo\alpha \foo").text, "α");
    // \renewcommand can even change a built-in command
    assert_eq!(pt(r"\renewcommand{\frac}[2]{#1/#2} \frac{a}{b}").text, "a/b");
}

#[test]
fn l_mac_1_declared_operators_and_delimiters() {
    assert_eq!(pt(r"\DeclareMathOperator{\Ln}{Ln} \Ln x").text, "Ln x");
    let r = pt(r"\DeclarePairedDelimiter{\abs}{\lvert}{\rvert} \abs*{\frac{a}{b}}");
    assert!(matches!(r.tree.children()[0], Node::Delimited { .. }), "{:?}", r.tree);
}

#[test]
fn l_mac_1_new_environment() {
    assert_eq!(pt(r"\newenvironment{par}{\left(}{\right)} \begin{par} x \end{par}").text, "(x)");
}

#[test]
fn l_mac_1_recursive_macro_stops_with_error() {
    assert!(error(r"\newcommand{\a}{\a x} \a").contains("too many macros"));
}

#[test]
fn physics_and_braket_packages() {
    assert_eq!(pt(r"\dv{f}{x}").text, "(df)/(dx)");
    assert_eq!(pt(r"\pdv{f}{x}{y}").text, "(∂^2f)/(∂x ∂y)");
    assert_eq!(pt(r"\ket{\psi}").text, "|ψ⟩");
    assert_eq!(pt(r"\braket{\phi|\psi}").text, "⟨ϕ | ψ⟩");
}

#[test]
fn amscd_commutative_diagram() {
    let r = pt(r"\begin{CD} A @>f>> B \\ @VgVV @VVhV \\ C @>>k> D \end{CD}");
    let Node::Table { shape, rows, .. } = &r.tree.children()[0] else { panic!("{:?}", r.tree) };
    assert_eq!(*shape, TableKind::Diagram);
    assert_eq!(rows.len(), 3);
    assert!(matches!(rows[1][0], Node::VerticalArrow { symbol: '↓', .. }));
    assert!(r.speech.starts_with("diagrama: linha 1: A, seta para a direita (f), B"), "{}", r.speech);
}

#[test]
fn tikzcd_arrows_between_any_two_cells() {
    let r = en(r#"\begin{tikzcd} A \arrow[r, "f"] \arrow[d, "g"'] \arrow[dr, "h" description] & B \arrow[d, "k"] \\ C \arrow[r, "\ell"', dashed] & D \end{tikzcd}"#);
    let Node::ArrowDiagram { cells, arrows } = &r.tree.children()[0] else { panic!("{:?}", r.tree) };
    assert_eq!((cells.len(), cells[0].len()), (2, 2));
    let ends: Vec<_> = arrows.iter().map(|a| (a.from, a.to)).collect();
    assert_eq!(ends, [((0, 0), (0, 1)), ((0, 0), (1, 0)), ((0, 0), (1, 1)), ((0, 1), (1, 1)), ((1, 0), (1, 1))]);
    let sides: Vec<_> = arrows.iter().map(|a| a.labels[0].side).collect();
    assert_eq!(sides, [LabelSide::Left, LabelSide::Right, LabelSide::On, LabelSide::Left, LabelSide::Right]);
    assert_eq!(arrows[4].line, ArrowLine::Dashed);
    assert_eq!(r.text, "A, B; C, D; A →[f] B; A →[g] C; A →[h] D; B →[k] D; C →[ℓ] D");
    assert!(r.speech.ends_with("arrow from A to D (h); arrow from B to D (k); arrow from C to D (ℓ)"), "{}", r.speech);
    assert!(pt(r"\begin{tikzcd} A \arrow[r] & B \end{tikzcd}").speech.ends_with("seta de A para B"));
}

#[test]
fn tikzcd_arrow_styles() {
    let arrow = |options: &str| {
        let r = en(&format!(r"\begin{{tikzcd}} A \arrow[{options}] & B \\ C & D \end{{tikzcd}}"));
        let Node::ArrowDiagram { arrows, .. } = &r.tree.children()[0] else { panic!() };
        arrows[0].clone()
    };
    assert_eq!(arrow("r, hook").tail, ArrowTail::Hook);
    assert_eq!(arrow("r, hook'").tail, ArrowTail::HookRight);
    assert_eq!(arrow("r, mapsto").tail, ArrowTail::Bar);
    assert_eq!(arrow("r, tail").tail, ArrowTail::Tail);
    assert_eq!(arrow("r, two heads").head, ArrowHead::TwoHeads);
    assert_eq!(arrow("r, no head").head, ArrowHead::None);
    assert_eq!(arrow("r, dotted").line, ArrowLine::Dotted);
    assert_eq!(arrow("r, squiggly").line, ArrowLine::Squiggly);
    let equal = arrow("r, equal");
    assert_eq!((equal.line, equal.head), (ArrowLine::Double, ArrowHead::None));
    assert_eq!(arrow("r, Rightarrow").line, ArrowLine::Double);
    assert_eq!(arrow("r, leftrightarrow").tail, ArrowTail::Head);
    assert_eq!(arrow("r, bend left").bend, 30);
    assert_eq!(arrow("r, bend right=50").bend, -50);
    assert!(arrow("r, shift left").shift > 0.0 && arrow("r, shift right=2").shift < 0.0);
    assert_eq!(arrow("dr").to, (1, 1));
    assert_eq!(arrow("to=2-2").to, (1, 1));
    assert_eq!(arrow(r#"r, "f" near start"#).labels[0].position, 0.25);
    assert_eq!(arrow(r#"r, "f", very near end"#).labels[0].position, 0.875);
    let phantom = arrow(r#"dr, phantom, "\lrcorner" very near start"#);
    assert_eq!((phantom.line, phantom.labels[0].side), (ArrowLine::None, LabelSide::On));
    // layout tweaks are read and ignored; the environment's own options too
    en(r#"\begin{tikzcd}[column sep=large, row sep=2em] A \arrow[r, crossing over, red, "{f,g}" swap] & B \end{tikzcd}"#);
}

#[test]
fn xymatrix_arrows_and_labels() {
    let r = en(r"\xymatrix@C=3em@R=2pc{ A \ar[r]^f \ar[d]_g \ar[dr]|h & B \ar[d]^{k} \\ C \ar[r]_\ell & D }");
    let Node::ArrowDiagram { cells, arrows } = &r.tree.children()[0] else { panic!("{:?}", r.tree) };
    assert_eq!(cells.len(), 2);
    let sides: Vec<_> = arrows.iter().map(|a| a.labels[0].side).collect();
    assert_eq!(sides, [LabelSide::Left, LabelSide::Right, LabelSide::On, LabelSide::Left, LabelSide::Right]);
    assert_eq!(r.text, "A, B; C, D; A →[f] B; A →[g] C; A →[h] D; B →[k] D; C →[ℓ] D");
    let r = en(r"\xymatrix{ A \ar@{^{(}->}[r] \ar@/^/[rr]^<{\phi} & B \ar@{->>}[r] & C \ar@{-->}[d] \ar@{=}[r] & E \\ & & D \ar@{|->}[u] }");
    let Node::ArrowDiagram { arrows, .. } = &r.tree.children()[0] else { panic!() };
    assert_eq!(arrows[0].tail, ArrowTail::Hook);
    assert!(arrows[1].bend > 0 && arrows[1].labels[0].position < 0.5);
    assert_eq!(arrows[2].head, ArrowHead::TwoHeads);
    assert_eq!(arrows[3].line, ArrowLine::Dashed);
    assert_eq!((arrows[4].line, arrows[4].head), (ArrowLine::Double, ArrowHead::None));
    assert_eq!((arrows[5].tail, arrows[5].to), (ArrowTail::Bar, (0, 2)));
    assert_eq!(en(r#"\xymatrix{ A \ar"2,2" & \\ & B }"#).text, "A, ; , B; A → B");
}

#[test]
fn substack_multicolumn_and_intertext() {
    pt(r"\sum_{\substack{0 \le i \le m \\ 0 < j < n}} P(i, j)");
    let r = pt(r"\begin{array}{|c|c|} \hline \multicolumn{2}{|c|}{\text{Total}} \\ \hline 1 & 2 \\ \hline \end{array}");
    let Node::Table { rows, .. } = &r.tree.children()[0] else { panic!() };
    assert!(matches!(rows[0][0], Node::Multicolumn { columns: 2, .. }));
    let r = pt(r"\begin{align} a &= b \intertext{logo} c &= d \end{align}");
    let Node::Table { intertexts, .. } = &r.tree.children()[0] else { panic!() };
    assert_eq!(intertexts, &[1]);
}

#[test]
fn plain_tex_cases_and_matrix() {
    assert_eq!(pt(r"\cases{1 & x > 0 \cr 0 & x \le 0}").text, "{1, x > 0; 0, x ≤ 0");
    assert_eq!(pt(r"\matrix{a & b \cr c & d}").text, "[[a, b], [c, d]]");
}

#[test]
fn latex_boxes_and_spaces() {
    for tex in [
        r"\genfrac{(}{)}{0pt}{}{n}{k}",
        r"\sfrac{1}{2} + \unitfrac[3]{m}{s}",
        r"\sideset{_a^b}{_c^d}\prod",
        r"x + \smash{\frac{a}{b}} + \mathclap{\text{c}}",
        r"\raisebox{0.5em}{alto} \scalebox{1.6}{grande} \rule{2em}{0.4pt}",
        r"1\textsuperscript{a} \verb|x^2|",
        r"\llbracket 1, n \rrbracket",
        r"\usepackage{amsmath} \makeatletter x",
    ] {
        pt(tex);
    }
}

#[test]
fn boundary_tex_programming_and_drawings_stay_out() {
    for (tex, reason) in [(r"\def\a{1}\ifx\a\a sim\fi", r"\ifx"), (r"\catcode`\@=11 x", r"\catcode"), (r"\tikz \draw (0,0)--(1,1);", r"\tikz")] {
        assert!(error(tex).contains(reason), "{tex}");
    }
    // structural chemistry comes in with the `chem` feature (L-CHE-2); without it, it stays out
    if cfg!(feature = "chem") {
        convert(r"\chemfig{H-C}", &Options { strict: true, ..Default::default() }).unwrap();
    } else {
        assert!(error(r"\chemfig{H-C}").contains(r"\chemfig"));
    }
}

fn en(tex: &str) -> mathra::Output {
    let r = convert(tex, &Options { strict: true, ..Default::default() }).unwrap_or_else(|e| panic!("{tex}: {e}"));
    let tolerant = convert(tex, &Options::default()).unwrap();
    assert!(tolerant.warnings.is_empty(), "{tex}: {:?}", tolerant.warnings);
    r
}

#[test]
fn physics_brackets_of_quantum_mechanics() {
    assert_eq!(en(r"\expval{A}").text, "⟨A⟩");
    assert_eq!(en(r"\expval{A}{\psi}").text, "⟨ψ | A | ψ⟩");
    assert_eq!(en(r"\ev{H}").text, "⟨H⟩");
    assert_eq!(en(r"\mel{n}{H}{m}").text, "⟨n | H | m⟩");
    assert_eq!(en(r"\braket{a}{b}").text, "⟨a | b⟩");
    assert_eq!(en(r"\innerproduct{a}{b}").text, "⟨a | b⟩");
    assert_eq!(en(r"\ketbra{a}{b}").text, "|a⟩⟨b|");
    assert_eq!(en(r"\dyad{a}").text, "|a⟩⟨a|");
    assert_eq!(en(r"\comm{A}{B}").text, "[A, B]");
    assert_eq!(en(r"\acomm{A}{B}").text, "{A, B}");
    assert_eq!(en(r"\pb{f}{g}").text, "{f, g}");
    // the brackets are \left…\right: they grow with the content
    assert!(matches!(en(r"\comm{\frac{a}{b}}{B}").tree.children()[0], Node::Delimited { open: Some('['), close: Some(']'), .. }));
}

#[test]
fn physics_vector_calculus() {
    assert_eq!(en(r"\grad f").text, "𝛁f");
    assert_eq!(en(r"\grad f").speech, "nabla f");
    assert_eq!(en(r"\divergence \vb{E}").text, "𝛁 · 𝐄");
    assert_eq!(en(r"\curl \vb{B}").text, "𝛁 × 𝐁");
    assert_eq!(en(r"\laplacian \phi").text, "∇^2ϕ");
    assert_eq!(en(r"\vb{x}").text, "𝐱");
    assert_eq!(en(r"\vb*{r}").text, "𝒓");
    assert_eq!(en(r"\vu{x}").speech, "x hat");
    assert_eq!(en(r"\vu*{r}").speech, "r hat");
    assert_eq!(en(r"\va{F}").speech, "vector F");
    assert_eq!(en(r"\vb{a} \cross \vb{b} + \vb{a} \vdot \vb{b}").text, "𝐚 × 𝐛 + 𝐚 · 𝐛");
    // \div keeps the school's ÷: physics' divergence is \divergence
    assert_eq!(en(r"6 \div 3").text, "6 ÷ 3");
}

#[test]
fn physics_derivatives_differentials_and_evaluation() {
    assert_eq!(en(r"\fdv{F}{\phi}").text, "(δF)/(δϕ)");
    assert_eq!(en(r"\fdv{\phi}").text, "δ/(δϕ)");
    assert_eq!(en(r"\int f \dd x").speech, "integral f d x");
    assert_eq!(en(r"\dd[3]{x}").text, "d^3x");
    assert_eq!(en(r"\dd{x}").text, "dx");
    let r = en(r"\eval{x^2}_0^1");
    let Node::Scripts { base, .. } = &r.tree.children()[0] else { panic!("{:?}", r.tree) };
    assert!(matches!(**base, Node::Delimited { open: None, close: Some('|'), .. }), "{base:?}");
    assert!(matches!(en(r"\eval(x^2)|_0^1").tree.children()[0], Node::Scripts { .. }));
}

#[test]
fn physics_operators_and_order() {
    assert_eq!(en(r"\Tr\rho").text, "Tr ρ");
    assert_eq!(en(r"\tr A").text, "tr A");
    assert_eq!(en(r"\rank M").text, "rank M");
    assert_eq!(en(r"\erf x").text, "erf x");
    assert_eq!(en(r"\Res f").text, "Res f");
    assert!(matches!(en(r"\Tr\rho").tree.children()[0], Node::Function { .. }));
    assert_eq!(en(r"\order{n^2}").text, "𝒪(n^2)");
    assert_eq!(en(r"\order(n)").text, "𝒪(n)");
}

/// physics' \qty sizes brackets; siunitx's \qty{value}{unit} is a quantity. Two braced groups decide.
#[test]
fn qty_of_physics_and_of_siunitx() {
    assert_eq!(en(r"\qty(\frac{a}{b})").text, "(a/b)");
    assert_eq!(en(r"\qty[x]").text, "[x]");
    assert_eq!(en(r"\qty|x|").text, "|x|");
    assert_eq!(en(r"\qty{x}").text, "{x}");
    assert!(matches!(en(r"\qty(x)").tree.children()[0], Node::Delimited { open: Some('('), .. }));
    assert!(matches!(en(r"\qty{3}{m}").tree.children()[0], Node::Quantity { .. }));
    assert!(matches!(en(r"\qty {3} {m}").tree.children()[0], Node::Quantity { .. }));
    assert_eq!(en(r"\pqty{x} \bqty{y} \vqty{z} \Bqty{w}").text, "(x)[y]|z|{w}");
}

#[test]
fn siunitx_ranges_lists_and_products() {
    let r = en(r"\SIrange{1}{10}{\kilo\gram}");
    assert_eq!(r.text, "1 kg to 10 kg");
    assert_eq!(r.speech, "1 kilogram to 10 kilograms");
    assert_eq!(en(r"\qtyrange{1}{10}{m}").text, "1 m to 10 m");
    assert_eq!(en(r"\numrange{1}{10}").text, "1 to 10");
    assert_eq!(en(r"\numlist{1;2;3}").text, "1, 2 and 3");
    assert_eq!(en(r"\numlist{1;2}").text, "1 and 2");
    assert_eq!(en(r"\SIlist{1;2;3}{\metre}").text, "1 m, 2 m and 3 m");
    assert_eq!(en(r"\qtylist{1;2}{m}").text, "1 m and 2 m");
    assert_eq!(en(r"\numproduct{2 x 3}").text, "2 × 3");
    assert_eq!(en(r"\qtyproduct{2 x 3}{m}").text, "2 m × 3 m");
    assert_eq!(en(r"\angrange{10}{20}").text, "10° to 20°");
    // the list word and the range word follow the locale
    assert_eq!(pt(r"\numlist{1;2;3}").text, "1, 2 e 3");
    assert_eq!(pt(r"\SIrange{1,5}{2,5}{\meter}").speech, "1,5 metro a 2,5 metros");
    assert_eq!(pt(r"\SIrange{1,5}{2,5}{\meter}").text, "1,5 m a 2,5 m");
}

#[test]
fn siunitx_angles_units_prefixes_and_options() {
    assert_eq!(en(r"\ang{12;30;15}").text, "12°30′15″");
    assert_eq!(en(r"\ang{;;15}").text, "15″");
    assert_eq!(en(r"\ang{1;2;}").text, "1°2′");
    assert_eq!(en(r"\SI{1}{\angstrom}").text, "1 Å");
    for (unit, symbol) in [
        ("radian", "rad"),
        ("steradian", "sr"),
        ("becquerel", "Bq"),
        ("gray", "Gy"),
        ("sievert", "Sv"),
        ("lumen", "lm"),
        ("lux", "lx"),
        ("katal", "kat"),
        ("dalton", "Da"),
        ("astronomicalunit", "au"),
        ("bel", "B"),
        ("neper", "Np"),
        ("barn", "b"),
        ("knot", "kn"),
        ("byte", "B"),
        ("torr", "Torr"),
    ] {
        assert_eq!(en(&format!(r"\si{{\{unit}}}")).text, symbol, "{unit}");
    }
    for (prefix, symbol) in [("yotta", "Y"), ("zetta", "Z"), ("exa", "E"), ("peta", "P"), ("atto", "a"), ("zepto", "z"), ("yocto", "y")] {
        assert_eq!(en(&format!(r"\si{{\{prefix}\meter}}")).text, format!("{symbol}m"), "{prefix}");
    }
    // siunitx options in brackets come before the arguments and are ignored
    assert_eq!(en(r"\SI[mode=text]{3}{\meter}").text, "3 m");
    assert_eq!(en(r"\num[group-digits=false]{12345}").text, "12345");
    assert_eq!(en(r"\si[per-mode=symbol]{\meter\per\second}").text, "m/s");
}

#[test]
fn tensor_indices_and_prescripts() {
    // each change of side starts a new empty base, so the indices stagger
    let r = en(r"\tensor{T}{^\mu_\nu^\rho}");
    assert_eq!(r.text, "T^μ_ν^ρ");
    let Node::Scripts { base, sup: Some(_), sub: None } = &r.tree.children()[0] else { panic!("{:?}", r.tree) };
    assert_eq!(base.clone().simplify(), Node::Identifier { name: "T".into(), variant: mathra::tree::Variant::Italic });
    assert!(matches!(&r.tree.children()[1], Node::Scripts { base, sub: Some(_), sup: None } if **base == Node::row(vec![])));
    // indices on the same side stay together
    assert_eq!(en(r"\tensor{R}{^a^b_c}").text, "R^(ab)_c");
    assert_eq!(en(r"\tensor[^a_b]{T}{_c}").text, "^a_bT_c");
    assert_eq!(en(r"g\indices{_\mu_\nu}").text, "g_(μν)");
    assert_eq!(en(r"\prescript{14}{6}{\mathrm{C}}").text, "_6^14C");
    assert_eq!(en(r"\prescript{14}{6}{\mathrm{C}}").tree, en(r"{}^{14}_{6}{\mathrm{C}}").tree);
}
