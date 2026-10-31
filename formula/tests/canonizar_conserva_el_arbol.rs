//! Garantía de la canonización: la fórmula canonizada se lee igual que la
//! original y canonizar dos veces no cambia nada.

use formula::canonico::{Canonizador, CanonizadorLatex, Espacios, Sinonimos};
use formula::catalogo::CatalogoLatex;
use formula::lector::{Lector, LectorLatex};
use formula::quimica::Mhchem;

const FORMULAS: &[&str] = &[
    "x^{-1}={\\frac {1}{x}}",
    "x^{2/3}={\\sqrt[{3}]{x^{2}}}{\\text{ etc.}}",
    "\\sum _{k=2}^{\\infty }{{\\ln k} \\over {k^{2}}}",
    "\\left[12\\ln A-\\gamma -\\ln(2\\pi )\\right]",
    "\\begin{aligned}a&=b\\\\ [2pt] c&=d\\end{aligned}",
    "f(x)={\\begin{cases}1&x>0\\\\0&{\\text{otherwise}}\\end{cases}}",
    "\\mathbb {R} ^{n}\\alpha  x\\,\\mathrm {d} x",
    "\\lim _{n\\to \\infty }\\left(1+{\\tfrac {1}{n}}\\right)^{n}",
    "{\\ce {^{226}_{88}Ra + ^{1}_{0}n -> ^{227}_{88}Ra ->[\\beta^-][42.2 \\ {\\ce {min}}] ^{227}_{89}Ac}}",
    "\\operatorname* {arg\\,max} _{x}f(x)\\le \\epsilon \\ne 0",
    "a\\land b\\lor \\lnot c\\gets d",
    "\\frac 12+\\frac a b",
];

fn lector() -> impl Lector {
    LectorLatex::new(CatalogoLatex, Mhchem)
}

#[test]
fn espacios_conserva_el_arbol_y_es_idempotente() {
    let c = CanonizadorLatex::new(vec![Box::new(Espacios)]);
    let l = lector();
    for f in FORMULAS {
        let canon = c.canonizar(f);
        assert_eq!(l.leer(&canon), l.leer(f), "«{f}» → «{canon}»");
        assert_eq!(c.canonizar(&canon), canon, "no idempotente: «{f}»");
    }
}

#[test]
fn sinonimos_conserva_el_arbol_y_es_idempotente() {
    let c = CanonizadorLatex::new(vec![Box::new(Espacios), Box::new(Sinonimos)]);
    let l = lector();
    for f in FORMULAS {
        let canon = c.canonizar(f);
        assert_eq!(l.leer(&canon), l.leer(f), "«{f}» → «{canon}»");
        assert_eq!(c.canonizar(&canon), canon, "no idempotente: «{f}»");
    }
}
