mod comun;

use adaptador::FormulaMathra;
use primitivas::{Color, Composicion, Compositor, ErrorComposicion, Primitiva};

const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

fn componer(latex: &str, estricto: bool) -> Result<Composicion, ErrorComposicion> {
    FormulaMathra {
        bloque: true,
        tam: 20.0,
        estricto,
        color: NEGRO,
    }
    .componer(latex)
}

/// Sin error y sin avisos en modo estricto, con algo que pintar y sus contornos.
fn se_entiende_entera(latex: &str) {
    let c = componer(latex, true).unwrap_or_else(|e| panic!("{latex}: {e}"));
    assert!(c.avisos.is_empty(), "{latex}: {:?}", c.avisos);
    assert!(!c.escena.primitivas.is_empty(), "{latex}");
    assert!(comun::glifos_con_contorno(&c.escena), "{latex}");
    assert!(comun::capas_equilibradas(&c.escena.primitivas), "{latex}");
}

/// La batería que probaba el motor del visor de HILO.
#[test]
fn las_formulas_habituales_se_maquetan() {
    for latex in [
        r"x^{0}=1",
        r"x^{-1}={\frac {1}{x}}",
        r"x^{-n}={\frac {1}{x^{n}}}{\text{ etc.}}",
        r"x^{1/2}={\sqrt {x}}",
        r"x^{2/3}={\sqrt[{3}]{x^{2}}}{\text{ etc.}}",
        r"x^{1/n}={\sqrt[{n}]{x}}",
        r"x^{p/q}={\sqrt[{q}]{x^{p}}}",
        r"\int_{0}^{\infty} e^{-x^{2}}\,dx = \frac{\sqrt{\pi}}{2}",
        r"\left[\begin{matrix}a&b\\c&d\end{matrix}\right]\qquad f(x)=\begin{cases}1&x>0\\0&{\text{si no}}\end{cases}",
        r"{\ce {2H2 + O2 -> 2H2O}}\qquad {\ce {Fe^3+ + 3OH- <=> Fe(OH)3}}\qquad a\equiv b{\pmod {n}}",
        r"\left(\begin{matrix}a\\b\\c\\d\\e\\f\end{matrix}\right)\quad \left\{\begin{matrix}1\\2\\3\\4\\5\\6\end{matrix}\right.\quad \sqrt{\frac{\frac{\frac{a}{b}}{\frac{c}{d}}}{\frac{\frac{e}{f}}{\frac{g}{h}}}}\quad \left|\begin{matrix}x\\y\\z\\w\end{matrix}\right|",
        r"A\xrightarrow{\text{una etiqueta bastante larga}}B\qquad \overrightarrow{ABCDEFGH}\qquad X\xrightleftharpoons[\text{abajo largo}]{\text{arriba}}Y",
        r"\pi\approx 3.14",
        r"\sum_{k=1}^{n} k = \frac{n(n+1)}{2}",
        r"e^{i\pi}+1=0",
    ] {
        se_entiende_entera(latex);
    }
}

/// Límite de `mathra`: no entiende `\ ` dentro de la etiqueta de una flecha de `\ce`. Candidato a
/// pull request; cuando se arregle, esta prueba fallará y la fórmula volverá a la batería.
#[test]
fn un_espacio_de_control_en_una_flecha_de_ce_aun_no_se_entiende() {
    let latex = r"{\ce {^{226}_{88}Ra + ^{1}_{0}n -> ^{227}_{88}Ra ->[\beta^-][42.2 \ {\ce {min}}] ^{227}_{89}Ac}}";
    assert!(componer(latex, true).is_err());
    assert!(!componer(latex, false).expect("tolerante").avisos.is_empty());
}

/// Lo que el motor anterior pintaba mal sin avisar.
#[test]
fn lo_que_antes_fallaba_en_silencio() {
    for latex in [
        r"\overbrace{1+2}^{n}",
        r"\underbrace{a+b}_{k}",
        r"\begin{array}{c|c}\hline a&b\\\hline\end{array}",
        r"\boldsymbol{\alpha}",
        r"\operatorname*{arg\,max}_x f",
        r"\textcolor{red}{x+y}",
        r"\newcommand{\R}{\mathbb{R}} \R^n",
        r"\begin{tikzcd} A \arrow[r] & B \end{tikzcd}",
        r"\xymatrix{A \ar[r] & B}",
    ] {
        se_entiende_entera(latex);
    }
}

#[test]
fn quimica() {
    for latex in [
        r"\ce{A ->[H2O] B}",
        r"\ce{^{14}_{6}C}",
        r"\ce{Na+ + Cl- -> NaCl v}",
        r"\ce{CH3-CH2-OH}",
        r"\ce{KCr(SO4)2*12H2O}",
        r"\pu{1.2e3 kJ/mol}",
        r"\chemfig{*6(=-=-=-)}",
        r"\schemestart A \arrow B \schemestop",
    ] {
        se_entiende_entera(latex);
    }
}

#[test]
fn fisica() {
    for latex in [
        r"\dv{f}{x}",
        r"\pdv{f}{x}{y}",
        r"\ket{\psi}",
        r"\braket{a}{b}",
        r"\expval{A}{\psi}",
        r"\comm{A}{B}",
        r"\norm{v}",
        r"\curl\vb{E}",
        r"\laplacian\psi",
        r"\dd{x}",
        r"\eval{f}_0^1",
        r"\SI{9.81}{m/s^2}",
        r"\si{\kilo\meter\per\hour}",
        r"\num{1.2e3}",
        r"\tensor{T}{^a_b}",
    ] {
        se_entiende_entera(latex);
    }
}

#[test]
fn el_color_de_una_formula_llega_a_sus_glifos() {
    let rojo = Color { r: 1.0, g: 0.0, b: 0.0 };
    let c = componer(r"\textcolor{red}{x}", true).expect("fórmula");
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    assert!(
        c.escena
            .primitivas
            .iter()
            .any(|p| matches!(p, Primitiva::Glifo { color, .. } if *color == rojo))
    );
}

/// En LaTeX, los colores son los de xcolor: `green` no es el verde oscuro de CSS.
#[test]
fn los_nombres_de_color_son_los_de_xcolor() {
    let color_de = |latex: &str| {
        let c = componer(latex, true).expect("fórmula");
        assert!(c.avisos.is_empty(), "{latex}: {:?}", c.avisos);
        c.escena.primitivas.iter().find_map(|p| match p {
            Primitiva::Glifo { color, .. } => Some(*color),
            _ => None,
        })
    };
    assert_eq!(
        color_de(r"\textcolor{green}{x}"),
        Some(Color { r: 0.0, g: 1.0, b: 0.0 })
    );
    assert_ne!(color_de(r"\textcolor{OliveGreen}{x}"), Some(NEGRO));
    assert_eq!(
        color_de(r"\textcolor[HTML]{FF0000}{x}"),
        Some(Color { r: 1.0, g: 0.0, b: 0.0 })
    );
}

#[test]
fn un_color_que_no_existe_se_avisa() {
    let c = componer(r"\textcolor{inventado}{x}", false).expect("fórmula");
    assert!(c.avisos.iter().any(|a| a.contains("inventado")), "{:?}", c.avisos);
}

#[test]
fn la_linea_base_queda_en_cero() {
    let e = componer(r"\frac{a}{b}", true).expect("fórmula").escena;
    assert!(e.caja.y < 0.0, "la fórmula sube sobre la línea base");
    assert!(e.caja.y + e.caja.alto > 0.0, "la fracción baja de la línea base");
    assert!(e.caja.ancho > 0.0);
}

#[test]
fn en_linea_es_mas_compacta_que_en_bloque() {
    let medir = |bloque| {
        FormulaMathra {
            bloque,
            tam: 20.0,
            estricto: true,
            color: NEGRO,
        }
        .componer(r"\sum_{i=1}^n i")
        .expect("fórmula")
        .escena
        .caja
        .alto
    };
    assert!(medir(false) < medir(true));
}

#[test]
fn lo_que_no_entiende_se_avisa_o_se_rechaza() {
    let orden = r"\ordeninventada{x}";
    assert!(!componer(orden, false).expect("tolerante").avisos.is_empty());
    assert!(componer(orden, true).is_err());
    assert!(componer(r"\frac{a}{", true).is_err());
}

/// Modo texto: `mathra` no lo cubre. El día que esta prueba falle, ya no hará falta el nuestro.
#[test]
fn el_modo_texto_no_es_de_mathra() {
    assert!(componer(r"\section{Intro}", true).is_err());
    assert!(componer(r"\begin{itemize}\item a\end{itemize}", true).is_err());
}
