#![allow(clippy::unwrap_used, clippy::expect_used)]

//! La tinta es el color de lo que una figura no colorea. Sobre un fondo oscuro, todo lo que estaba
//! en negro por defecto debe seguir leyéndose, y lo que el documento pide se respeta.

use adaptador::FiguraTikra;
use primitivas::{Color, Composicion, Compositor};

const BLANCA: Color = Color::BLANCO;

fn con_tinta(fuente: &str, tinta: Color) -> Composicion {
    let figura = FiguraTikra {
        tinta,
        ..FiguraTikra::default()
    };
    figura.componer(fuente).unwrap()
}

fn tikz(cuerpo: &str) -> String {
    format!(r"\begin{{tikzpicture}}{cuerpo}\end{{tikzpicture}}")
}

/// Cuántas veces aparece un color en lo pintado.
fn veces(c: &Composicion, color: Color) -> usize {
    let buscado = format!("{color:?}");
    format!("{:?}", c.escena.primitivas).matches(&buscado).count()
}

#[test]
fn lo_que_no_se_colorea_lleva_la_tinta() {
    for cuerpo in [
        r"\draw (0,0) -- (1,1);",
        r"\fill (0,0) circle (1);",
        r"\node at (0,0) {hola};",
        r"\draw[->] (0,0) -- (1,0) node[right] {$x$};",
    ] {
        let c = con_tinta(&tikz(cuerpo), BLANCA);
        assert_eq!(veces(&c, Color::NEGRO), 0, "{cuerpo}");
        assert!(veces(&c, BLANCA) > 0, "{cuerpo}");
    }
}

#[test]
fn los_circuitos_y_las_graficas_tambien_la_llevan() {
    let circuito = r"\begin{circuitikz}\draw (0,0) to[R] (2,0);\end{circuitikz}";
    assert_eq!(veces(&con_tinta(circuito, BLANCA), Color::NEGRO), 0);
    let grafica = tikz(r"\begin{axis}\addplot coordinates {(0,0) (1,1)};\end{axis}");
    assert_eq!(veces(&con_tinta(&grafica, BLANCA), Color::NEGRO), 0);
}

#[test]
fn un_color_pedido_se_respeta_aunque_sea_negro() {
    let rojo = Color { r: 1.0, g: 0.0, b: 0.0 };
    let c = con_tinta(&tikz(r"\draw[red] (0,0) -- (1,1);"), BLANCA);
    assert!(veces(&c, rojo) > 0);
    let c = con_tinta(&tikz(r"\draw[black] (0,0) -- (1,1);"), BLANCA);
    assert!(veces(&c, Color::NEGRO) > 0);
    assert_eq!(veces(&c, BLANCA), 0);
}

/// `fill=white` sin color de texto: el autor contaba con leerlo, y blanco sobre blanco no se ve.
#[test]
fn el_texto_sobre_un_relleno_claro_se_sigue_leyendo() {
    for cuerpo in [
        r"\node[fill=white] {a};",
        r"\node[circle,draw,fill=blue!10] {a};",
        r"\matrix[fill=white] {\node {a}; \\};",
    ] {
        let c = con_tinta(&tikz(cuerpo), BLANCA);
        assert!(
            veces(&c, Color::NEGRO) > 0,
            "{cuerpo}: el texto no puede ir en la tinta"
        );
    }
}

#[test]
fn sobre_un_relleno_oscuro_se_queda_la_tinta() {
    let c = con_tinta(&tikz(r"\node[fill=black!80] {a};"), BLANCA);
    assert_eq!(veces(&c, Color::NEGRO), 0);
}

#[test]
fn un_color_de_texto_pedido_gana_al_contraste() {
    let c = con_tinta(&tikz(r"\node[fill=white,text=white] {a};"), BLANCA);
    assert_eq!(veces(&c, Color::NEGRO), 0);
}

/// Un relleno casi transparente deja ver lo de detrás: no se sabe sobre qué va el texto.
#[test]
fn un_relleno_que_apenas_cubre_no_cuenta() {
    let c = con_tinta(&tikz(r"\node[fill=white,fill opacity=0.2,text opacity=1] {a};"), BLANCA);
    assert_eq!(veces(&c, Color::NEGRO), 0);
}

/// Sin marco: el marco es negro, y el negro que quede solo puede ser el texto.
#[test]
fn la_leyenda_de_una_grafica_se_lee_en_su_caja_blanca() {
    let fuente =
        tikz(r"\begin{axis}[legend style={draw=none}]\addplot coordinates {(0,0) (1,1)};\legend{uno}\end{axis}");
    let c = con_tinta(&fuente, BLANCA);
    assert!(
        veces(&c, Color::NEGRO) > 0,
        "el texto de la leyenda no puede ir en la tinta"
    );
}

/// El blanco de una puerta hace de papel: con otra tinta, el papel no es blanco.
#[test]
fn las_puertas_logicas_no_se_rellenan_de_blanco() {
    let fuente = r"\begin{circuitikz}\node[or port] {};\node[xor port] at (0,2) {};\end{circuitikz}";
    let pintado = format!("{:?}", con_tinta(fuente, BLANCA).escena.primitivas);
    let relleno = format!("Relleno {{ pintura: Solida({BLANCA:?})");
    assert!(!pintado.contains(&relleno), "{pintado}");
    let negra = format!("{:?}", con_tinta(fuente, Color::NEGRO).escena.primitivas);
    assert!(negra.contains(&relleno), "sobre papel blanco, como siempre");
}

#[test]
fn con_tinta_negra_no_cambia_nada() {
    let fuente = tikz(r"\node[fill=black,text=white] {a}; \node[fill=white] at (1,0) {b}; \draw (0,0) -- (1,1);");
    let negra = format!("{:?}", con_tinta(&fuente, Color::NEGRO).escena.primitivas);
    let por_defecto = format!(
        "{:?}",
        FiguraTikra::default().componer(&fuente).unwrap().escena.primitivas
    );
    assert_eq!(negra, por_defecto);
}
