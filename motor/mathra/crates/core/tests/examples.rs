//! The README gallery: famous equations and constructions that show what mathra draws.
//! Each `docs/examples/<name>.svg` must be exactly what the current code produces, on a white card so
//! it stays readable on dark pages. After a layout change:
//! `MATHRA_UPDATE_EXAMPLES=1 cargo test -p mathra --test examples`.

use mathra::{convert, Locale, Options};

pub struct Example {
    pub name: &'static str,
    pub locale: Locale,
    pub caption: &'static str,
    pub tex: &'static str,
}

pub const EXAMPLES: &[Example] = &[
    Example { name: "euler", locale: Locale::En, caption: "Euler's identity", tex: r"e^{i\pi} + 1 = 0" },
    Example { name: "gaussian", locale: Locale::En, caption: "Gaussian integral", tex: r"\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}" },
    Example { name: "basel", locale: Locale::En, caption: "Basel problem", tex: r"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}" },
    Example { name: "quadratic", locale: Locale::En, caption: "Quadratic formula", tex: r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}" },
    Example {
        name: "maxwell",
        locale: Locale::En,
        caption: "Maxwell's equations",
        tex: r"\begin{aligned} \nabla \cdot \vec{E} &= \frac{\rho}{\varepsilon_0} & \nabla \cdot \vec{B} &= 0 \\ \nabla \times \vec{E} &= -\frac{\partial \vec{B}}{\partial t} & \nabla \times \vec{B} &= \mu_0 \vec{J} + \mu_0 \varepsilon_0 \frac{\partial \vec{E}}{\partial t} \end{aligned}",
    },
    Example {
        name: "schrodinger",
        locale: Locale::En,
        caption: "Schrödinger equation",
        tex: r"i\hbar \frac{\partial}{\partial t} \Psi(\mathbf{r}, t) = \left[ -\frac{\hbar^2}{2m} \nabla^2 + V(\mathbf{r}, t) \right] \Psi(\mathbf{r}, t)",
    },
    Example {
        name: "einstein",
        locale: Locale::En,
        caption: "Einstein field equations",
        tex: r"R_{\mu\nu} - \frac{1}{2} R\, g_{\mu\nu} + \Lambda g_{\mu\nu} = \frac{8\pi G}{c^4} T_{\mu\nu}",
    },
    Example { name: "fourier", locale: Locale::En, caption: "Fourier transform", tex: r"\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)\, e^{-2\pi i x \xi}\, dx" },
    Example { name: "cauchy", locale: Locale::En, caption: "Cauchy's integral formula", tex: r"f(a) = \frac{1}{2\pi i} \oint_{\gamma} \frac{f(z)}{z - a}\, dz" },
    Example { name: "bayes", locale: Locale::En, caption: "Bayes' theorem", tex: r"P(A \mid B) = \frac{P(B \mid A)\, P(A)}{P(B)}" },
    Example {
        name: "navierstokes",
        locale: Locale::En,
        caption: "Navier–Stokes equations (incompressible)",
        tex: r"\rho \left( \frac{\partial \mathbf{u}}{\partial t} + (\mathbf{u} \cdot \nabla) \mathbf{u} \right) = -\nabla p + \mu \nabla^2 \mathbf{u} + \rho \mathbf{g}, \qquad \nabla \cdot \mathbf{u} = 0",
    },
    Example {
        name: "stokes",
        locale: Locale::En,
        caption: "Stokes' theorem",
        tex: r"\oint_{\partial \Sigma} \vec{F} \cdot d\vec{r} = \iint_{\Sigma} \left( \nabla \times \vec{F} \right) \cdot d\vec{S}",
    },
    Example {
        name: "eulerlagrange",
        locale: Locale::En,
        caption: "Euler–Lagrange equation",
        tex: r"\frac{d}{dt} \frac{\partial L}{\partial \dot{q}} - \frac{\partial L}{\partial q} = 0",
    },
    Example {
        name: "zeta",
        locale: Locale::En,
        caption: "Riemann zeta as an Euler product",
        tex: r"\zeta(s) = \sum_{n=1}^{\infty} \frac{1}{n^s} = \prod_{p \text{ prime}} \frac{1}{1 - p^{-s}}",
    },
    Example { name: "dirac", locale: Locale::En, caption: "Dirac equation", tex: r"\left( i\hbar \gamma^\mu \partial_\mu - mc \right) \psi = 0" },
    Example { name: "planck", locale: Locale::En, caption: "Planck's law", tex: r"B_\nu(T) = \frac{2 h \nu^3}{c^2} \, \frac{1}{e^{h\nu / k_B T} - 1}" },
    Example {
        name: "ramanujan",
        locale: Locale::En,
        caption: "Ramanujan's series for 1/π",
        tex: r"\frac{1}{\pi} = \frac{2\sqrt{2}}{9801} \sum_{k=0}^{\infty} \frac{(4k)!\,(1103 + 26390k)}{(k!)^4\, 396^{4k}}",
    },
    Example {
        name: "blackscholes",
        locale: Locale::En,
        caption: "Black–Scholes equation",
        tex: r"\frac{\partial V}{\partial t} + \frac{1}{2} \sigma^2 S^2 \frac{\partial^2 V}{\partial S^2} + r S \frac{\partial V}{\partial S} - r V = 0",
    },
    Example {
        name: "navierstokes3d",
        locale: Locale::En,
        caption: "Navier–Stokes in Cartesian components, with continuity",
        tex: r"\begin{aligned} \rho \left( \frac{\partial u}{\partial t} + u \frac{\partial u}{\partial x} + v \frac{\partial u}{\partial y} + w \frac{\partial u}{\partial z} \right) &= -\frac{\partial p}{\partial x} + \mu \left( \frac{\partial^2 u}{\partial x^2} + \frac{\partial^2 u}{\partial y^2} + \frac{\partial^2 u}{\partial z^2} \right) + \rho g_x \\ \rho \left( \frac{\partial v}{\partial t} + u \frac{\partial v}{\partial x} + v \frac{\partial v}{\partial y} + w \frac{\partial v}{\partial z} \right) &= -\frac{\partial p}{\partial y} + \mu \left( \frac{\partial^2 v}{\partial x^2} + \frac{\partial^2 v}{\partial y^2} + \frac{\partial^2 v}{\partial z^2} \right) + \rho g_y \\ \rho \left( \frac{\partial w}{\partial t} + u \frac{\partial w}{\partial x} + v \frac{\partial w}{\partial y} + w \frac{\partial w}{\partial z} \right) &= -\frac{\partial p}{\partial z} + \mu \left( \frac{\partial^2 w}{\partial x^2} + \frac{\partial^2 w}{\partial y^2} + \frac{\partial^2 w}{\partial z^2} \right) + \rho g_z \\ \frac{\partial u}{\partial x} + \frac{\partial v}{\partial y} + \frac{\partial w}{\partial z} &= 0 \end{aligned}",
    },
    Example {
        name: "compressible",
        locale: Locale::En,
        caption: "Compressible Navier–Stokes, with the viscous stress tensor",
        tex: r"\frac{\partial (\rho \mathbf{u})}{\partial t} + \nabla \cdot \left( \rho\, \mathbf{u} \otimes \mathbf{u} \right) = -\nabla p + \nabla \cdot \left[ \mu \left( \nabla \mathbf{u} + \left( \nabla \mathbf{u} \right)^{\mathsf{T}} \right) - \frac{2}{3} \mu \left( \nabla \cdot \mathbf{u} \right) \mathbf{I} \right] + \rho \mathbf{g}",
    },
    Example {
        name: "riemann",
        locale: Locale::En,
        caption: "Riemann curvature tensor and Christoffel symbols",
        tex: r"R^{\rho}{}_{\sigma\mu\nu} = \partial_\mu \Gamma^{\rho}_{\nu\sigma} - \partial_\nu \Gamma^{\rho}_{\mu\sigma} + \Gamma^{\rho}_{\mu\lambda} \Gamma^{\lambda}_{\nu\sigma} - \Gamma^{\rho}_{\nu\lambda} \Gamma^{\lambda}_{\mu\sigma}, \qquad \Gamma^{\lambda}_{\mu\nu} = \frac{1}{2} g^{\lambda\kappa} \left( \partial_\mu g_{\kappa\nu} + \partial_\nu g_{\kappa\mu} - \partial_\kappa g_{\mu\nu} \right)",
    },
    Example {
        name: "qed",
        locale: Locale::En,
        caption: "Lagrangian of quantum electrodynamics",
        tex: r"\mathcal{L} = -\frac{1}{4} F_{\mu\nu} F^{\mu\nu} + \bar{\psi} \left( i \gamma^\mu D_\mu - m \right) \psi, \qquad D_\mu = \partial_\mu + i e A_\mu, \qquad F_{\mu\nu} = \partial_\mu A_\nu - \partial_\nu A_\mu",
    },
    Example {
        name: "gaussproof",
        locale: Locale::En,
        caption: "The Gaussian integral, step by step in polar coordinates",
        tex: r"\begin{aligned} \left( \int_{-\infty}^{\infty} e^{-x^2} \, dx \right)^2 &= \int_{-\infty}^{\infty} \int_{-\infty}^{\infty} e^{-(x^2 + y^2)} \, dx \, dy \\ &= \int_{0}^{2\pi} \int_{0}^{\infty} e^{-r^2} \, r \, dr \, d\theta \\ &= 2\pi \left[ -\frac{1}{2} e^{-r^2} \right]_{0}^{\infty} = \pi \end{aligned}",
    },
    Example {
        name: "vandermonde",
        locale: Locale::En,
        caption: "Vandermonde determinant",
        tex: r"\det \begin{pmatrix} 1 & x_1 & x_1^2 & \cdots & x_1^{n-1} \\ 1 & x_2 & x_2^2 & \cdots & x_2^{n-1} \\ 1 & x_3 & x_3^2 & \cdots & x_3^{n-1} \\ \vdots & \vdots & \vdots & \ddots & \vdots \\ 1 & x_n & x_n^2 & \cdots & x_n^{n-1} \end{pmatrix} = \prod_{1 \le i < j \le n} \left( x_j - x_i \right)",
    },
    Example { name: "golden", locale: Locale::En, caption: "Golden ratio as a continued fraction", tex: r"\varphi = 1 + \cfrac{1}{1 + \cfrac{1}{1 + \cfrac{1}{1 + \cdots}}}" },
    Example {
        name: "matrix",
        locale: Locale::En,
        caption: "m × n matrix with dots",
        tex: r"A = \begin{pmatrix} a_{11} & a_{12} & \cdots & a_{1n} \\ a_{21} & a_{22} & \cdots & a_{2n} \\ \vdots & \vdots & \ddots & \vdots \\ a_{m1} & a_{m2} & \cdots & a_{mn} \end{pmatrix}",
    },
    Example { name: "piecewise", locale: Locale::En, caption: "Piecewise definition", tex: r"|x| = \begin{cases} x & \text{if } x \ge 0 \\ -x & \text{if } x < 0 \end{cases}" },
    Example { name: "binomial", locale: Locale::En, caption: "Binomial theorem", tex: r"(x + y)^n = \sum_{k=0}^{n} \binom{n}{k} x^{n-k} y^{k}" },
    Example { name: "braces", locale: Locale::En, caption: "Labeled brace", tex: r"\underbrace{1 + 2 + \cdots + n}_{n \text{ terms}} = \frac{n(n+1)}{2}" },
    Example { name: "chemistry", locale: Locale::En, caption: "Chemical equation (mhchem)", tex: r"\ce{CH4 + 2 O2 -> CO2 + 2 H2O}" },
    Example { name: "units", locale: Locale::En, caption: "Quantity with units (siunitx)", tex: r"g = \SI{9.81}{\meter\per\second\squared}" },
    Example { name: "diagram", locale: Locale::En, caption: "Commutative diagram (amscd)", tex: r"\begin{CD} A @>f>> B \\ @VgVV @VVhV \\ C @>k>> D \end{CD}" },
    Example {
        name: "tikzcd",
        locale: Locale::En,
        caption: "Pullback with bent arrows (tikz-cd)",
        tex: r#"\begin{tikzcd} T \arrow[drr, bend left, "x"] \arrow[ddr, bend right, "y"'] \arrow[dr, dashed, "\exists!" description] & & \\ & P \arrow[r, "q"] \arrow[d, "p"'] \arrow[dr, phantom, "\lrcorner" very near start] & X \arrow[d, "f"] \\ & Y \arrow[r, "g"'] & Z \end{tikzcd}"#,
    },
    Example {
        name: "xymatrix",
        locale: Locale::En,
        caption: "Short exact sequence (xymatrix)",
        tex: r"\xymatrix{ 0 \ar[r] & A \ar@{^{(}->}[r]^{\iota} \ar[d]_{\alpha} & B \ar@{->>}[r]^{\pi} \ar[d]^{\beta} & C \ar[r] \ar@{=}[d] & 0 \\ 0 \ar[r] & A' \ar[r] & B' \ar[r] & C \ar[r] & 0 }",
    },
    Example {
        name: "ptbr",
        locale: Locale::PtBr,
        caption: "pt-BR locale: sen, decimal comma, open interval",
        tex: r"\sen^2 \theta + \cos^2 \theta = 1,\quad x \in \left]0{,}5;\, 2{,}5\right[",
    },
    Example { name: "division", locale: Locale::PtBr, caption: "Long division, computed (pt-BR)", tex: r"\divchave{247}{5}" },
];

/// Structural chemistry (L-CHE-2), with the `chem` feature.
#[cfg(feature = "chem")]
pub const CHEMISTRY: &[Example] = &[
    Example { name: "caffeine", locale: Locale::En, caption: "Caffeine: fused rings (chemfig)", tex: r"\chemfig{*6((=O)-N(-CH_3)-*5(-N=-N(-CH_3)-=)--(=O)-N(-H_3C)-)}" },
    Example {
        name: "nitration",
        locale: Locale::En,
        caption: "Nitration of benzene: structures, mhchem and conditions on the arrow",
        tex: r"\schemestart \chemfig{**6(------)} \+ \ce{HNO3} \arrow{->[\ce{H2SO4}][$\Delta$]} \chemfig{**6(---(-NO_2)---)} \+ \ce{H2O} \schemestop",
    },
    Example {
        name: "sn2",
        locale: Locale::En,
        caption: "SN2 mechanism: lone pairs and curved arrows",
        tex: r"\schemestart \chemfig{@{nu}\lewis{2:4:6:,HO}^{-}} \+ \chemfig{H_3C-@{c}C(-[2]H)(-[6]H)-@{cl}Cl} \arrow{->[][S$_\text{N}$2]} \chemfig{HO-CH_3} \+ \chemfig{\lewis{0:2:4:6:,Cl}^{-}} \schemestop \chemmove{\draw[->](nu)..controls +(60:1.2cm) and +(120:1.2cm)..(c); \draw[->](c) to[bend left] (cl);}",
    },
    Example {
        name: "esterification",
        locale: Locale::PtBr,
        caption: "Esterification and reduction: equilibrium, a reagent drawn over a vertical arrow (pt-BR)",
        tex: r"\schemestart \chemfig{R-C(=[1]O)-[7]OH} \+ \chemfig{HO-R'} \arrow{<=>[H$^+$][$-$H$_2$O]} \chemfig{R-C(=[1]O)-[7]O-R'} \arrow{->[\chemfig{LiAlH_4}][depois H$_2$O]}[-90] \chemfig{R-CH_2-OH} \schemestop",
    },
];

/// Every example the gallery holds with the features this build has.
fn all() -> Vec<&'static Example> {
    #[cfg(feature = "chem")]
    let chemistry = CHEMISTRY;
    #[cfg(not(feature = "chem"))]
    let chemistry: &[Example] = &[];
    EXAMPLES.iter().chain(chemistry).collect()
}

/// Padding around the equation, in font units (1000 per em).
const PADDING: f32 = 320.0;
/// Pixels per em in the README.
const PX_PER_EM: f32 = 30.0;

fn number(v: f32) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    if r.fract() == 0.0 {
        format!("{}", r as i64)
    } else {
        format!("{r}")
    }
}

/// The equation's SVG with padding, a white card behind it and a size in px.
fn on_card(svg: &str) -> String {
    let attr = |name: &str| svg.split(&format!(" {name}=\"")).nth(1).unwrap().split('"').next().unwrap().to_string();
    let v: Vec<f32> = attr("viewBox").split(' ').map(|n| n.parse().unwrap()).collect();
    let (x, y, w, h) = (v[0] - PADDING, v[1] - PADDING, v[2] + 2.0 * PADDING, v[3] + 2.0 * PADDING);
    let px = |units: f32| number(units / 1000.0 * PX_PER_EM);
    let svg = svg
        .replacen(&format!(" viewBox=\"{}\"", attr("viewBox")), &format!(" viewBox=\"{} {} {} {}\"", number(x), number(y), number(w), number(h)), 1)
        .replacen(&format!(" width=\"{}\"", attr("width")), &format!(" width=\"{}\"", px(w)), 1)
        .replacen(&format!(" height=\"{}\"", attr("height")), &format!(" height=\"{}\"", px(h)), 1);
    let open_end = svg.find('>').unwrap() + 1;
    let card = format!(r##"<rect x="{}" y="{}" width="{}" height="{}" rx="{}" fill="#ffffff"/>"##, number(x), number(y), number(w), number(h), number(PADDING / 2.0));
    format!("{}{card}{}", &svg[..open_end], &svg[open_end..])
}

#[test]
fn committed_examples_match_the_code() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/examples");
    let update = std::env::var_os("MATHRA_UPDATE_EXAMPLES").is_some();
    for e in all() {
        // the gallery shows valid LaTeX: strict mode must accept every example
        let r = convert(e.tex, &Options { locale: e.locale, strict: true, ..Options::default() }).unwrap_or_else(|err| panic!("{}: {err}", e.name));
        let svg = on_card(&r.svg);
        let path = format!("{dir}/{}.svg", e.name);
        if update {
            std::fs::write(&path, &svg).unwrap();
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_default();
        assert!(committed == svg, "docs/examples/{}.svg is stale: run MATHRA_UPDATE_EXAMPLES=1 cargo test -p mathra --test examples", e.name);
    }
}

#[test]
fn every_committed_example_is_in_the_list() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/examples");
    for f in std::fs::read_dir(dir).unwrap() {
        let name = f.unwrap().file_name().into_string().unwrap();
        let name = name.trim_end_matches(".svg");
        // without `chem`, the chemistry examples are not drawn but still belong to the gallery
        #[cfg(not(feature = "chem"))]
        if ["caffeine", "nitration", "sn2", "esterification"].contains(&name) {
            continue;
        }
        assert!(all().iter().any(|e| e.name == name), "docs/examples/{name}.svg is not generated by this test");
    }
}

/// The site's gallery page (`corpus/gallery.json`, by group): a row that names a README example is
/// that same equation, every README example is on the page, and every row converts in strict mode
/// with no warning.
#[test]
fn the_site_gallery_matches_the_readme_gallery() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus/gallery.json");
    let data: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    let mut named = Vec::new();
    for group in data.as_array().unwrap() {
        let id = group["id"].as_str().unwrap();
        if id == "chemistry" && !cfg!(feature = "chem") {
            continue;
        }
        for row in group["rows"].as_array().unwrap() {
            let (tex, locale, caption) = (row["tex"].as_str().unwrap(), row["locale"].as_str().unwrap(), row["caption"].as_str().unwrap());
            if let Some(name) = row["name"].as_str() {
                let e = all().into_iter().find(|e| e.name == name).unwrap_or_else(|| panic!("{id}: {name} is not a README example"));
                assert_eq!((tex, locale, caption), (e.tex, e.locale.to_string().as_str(), e.caption), "{id}: {name}");
                named.push(name);
            }
            let input = row["input"].as_str().unwrap_or("latex").parse().unwrap();
            let options = Options { locale: locale.parse().unwrap(), input, strict: true, ..Options::default() };
            let r = convert(tex, &options).unwrap_or_else(|err| panic!("{id}: {caption}: {err}"));
            assert!(r.warnings.is_empty(), "{id}: {caption}: {:?}", r.warnings);
        }
    }
    for e in all() {
        assert!(named.contains(&e.name), "{} is in the README gallery but not on the site's gallery page", e.name);
    }
}

/// CHANGELOG.md (read by scripts/changelog.py for the GitHub release and the site's evolution page):
/// releases newest first, each with a date and a headline, and the equations each one shows are a
/// gallery name or LaTeX that converts in strict mode.
#[test]
fn the_changelog_shows_equations_that_convert() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
    let text = std::fs::read_to_string(format!("{root}/CHANGELOG.md")).unwrap();
    let gallery: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(format!("{root}/corpus/gallery.json")).unwrap()).unwrap();
    let names: Vec<&str> = gallery.as_array().unwrap().iter().flat_map(|g| g["rows"].as_array().unwrap()).filter_map(|r| r["name"].as_str()).collect();
    let mut versions = Vec::new();
    for section in text.split("\n## ").skip(1) {
        let (title, body) = section.split_once('\n').unwrap();
        if title != "Unreleased" {
            let (version, date) = title.split_once(" - ").unwrap_or_else(|| panic!("## {title}: `## <version> - <date>`"));
            assert_eq!(date.len(), 10, "{title}");
            versions.push(version.split('.').map(|n| n.parse::<u32>().unwrap()).collect::<Vec<_>>());
        }
        let headline = body.trim_start().lines().next().unwrap_or("");
        assert!(!headline.is_empty() && !headline.starts_with("- "), "{title}: no headline");
        let listed = body.lines().find_map(|l| l.strip_prefix("Equations:")).unwrap_or_else(|| panic!("{title}: no Equations: line"));
        for item in listed.split('`').skip(1).step_by(2) {
            if names.contains(&item) {
                continue;
            }
            // a gallery-like name that is not in the gallery is a typo, not LaTeX
            assert!(item.contains(['\\', '^', '_', '=', '{']), "{title}: `{item}` is neither a gallery name nor LaTeX");
            if cfg!(not(feature = "chem")) && item.contains("\\chemfig") {
                continue;
            }
            convert(item, &Options { strict: true, ..Options::default() }).unwrap_or_else(|e| panic!("{title}: {item}: {e}"));
        }
    }
    assert!(versions.len() >= 8 && versions.windows(2).all(|w| w[0] > w[1]), "newest first: {versions:?}");
}
