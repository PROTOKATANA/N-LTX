#![allow(clippy::unwrap_used, clippy::expect_used)]

mod comun;

use adaptador::FiguraTikra;
use primitivas::{Capa, Compositor, LIMITE_INGLETE, Pintura, Primitiva, Segmento};
use tikra::extension::{Context, Library, Tokens};

/// Bp por centímetro.
const BP: f64 = 72.0 / 2.54;

fn componer(fuente: &str) -> primitivas::Composicion {
    FiguraTikra::default().componer(fuente).expect("figura")
}

fn trazados(primitivas: &[Primitiva]) -> Vec<&Primitiva> {
    primitivas
        .iter()
        .filter(|p| matches!(p, Primitiva::Trazado { .. }))
        .collect()
}

fn sin_avisos(fuente: &str) -> primitivas::Composicion {
    let c = componer(fuente);
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    assert!(!c.escena.primitivas.is_empty());
    c
}

#[test]
fn las_opciones_de_una_celda_admiten_corchetes_entre_llaves() {
    sin_avisos(
        r"\usetikzlibrary{matrix}\begin{tikzpicture}
        \matrix [matrix of nodes] { |[label={[draw]:a}]|b & c \\ };
        \end{tikzpicture}",
    );
}

#[test]
fn smash_en_el_texto_de_un_nodo_deja_ver_lo_que_lleva() {
    sin_avisos(r"\begin{tikzpicture}\node {\smash{a} \smash[b]{b}};\end{tikzpicture}");
}

#[test]
fn pdftooltip_se_queda_en_lo_que_se_ve() {
    sin_avisos(r"\begin{tikzpicture}\node {\pdftooltip{a}{una nota}};\end{tikzpicture}");
}

#[test]
fn tikzmath_antes_de_la_figura_le_llega_a_la_figura() {
    sin_avisos(
        r"\usetikzlibrary{math}\tikzmath{ \R = 2; }\begin{tikzpicture}\draw (0,0) circle (\R);\end{tikzpicture}",
    );
}

#[test]
fn tikzcdset_no_se_pinta_como_texto() {
    let figura = r"\begin{tikzpicture}\node {x};\end{tikzpicture}";
    let con = sin_avisos(&format!(r"\tikzcdset{{cells={{nodes={{draw}}}}}}{figura}"));
    assert_eq!(format!("{:?}", con.escena), format!("{:?}", componer(figura).escena));
}

fn limites_de_inglete(fuente: &str) -> Vec<f64> {
    componer(fuente)
        .escena
        .primitivas
        .iter()
        .filter_map(|p| match p {
            Primitiva::Trazado { trazo: Some(t), .. } => Some(t.limite_inglete),
            _ => None,
        })
        .collect()
}

#[test]
fn un_trazo_sin_limite_pedido_usa_el_del_motor_y_uno_pedido_se_respeta() {
    let codo = r"(0,0) -- (1,0) -- (0,0.2);\end{tikzpicture}";
    assert_eq!(
        limites_de_inglete(&format!(r"\begin{{tikzpicture}}\draw {codo}")),
        [LIMITE_INGLETE]
    );
    let pedido = format!(r"\begin{{tikzpicture}}\draw[miter limit=20] {codo}");
    assert_eq!(limites_de_inglete(&pedido), [20.0]);
}

/// El ancho de cada glifo frente a su alto: 1 si la letra no está deformada.
fn proporciones(fuente: &str) -> Vec<f64> {
    componer(fuente)
        .escena
        .primitivas
        .iter()
        .filter_map(|p| match p {
            Primitiva::Glifo { matriz, .. } => Some((matriz.a / matriz.d).abs()),
            _ => None,
        })
        .collect()
}

#[test]
fn una_linea_grande_bajo_una_pequena_no_ensancha_sus_letras() {
    let sola = proporciones(r"\begin{tikzpicture}\node {Na};\end{tikzpicture}");
    let bajo = proporciones(r"\begin{tikzpicture}\node[align=center] {\tiny 11\\Na};\end{tikzpicture}");
    assert_eq!(bajo.len(), sola.len() + 2);
    let dos_ultimas = &bajo[bajo.len() - 2..];
    assert!((dos_ultimas[0] - sola[0]).abs() < 1e-6, "{bajo:?} frente a {sola:?}");
    assert!((dos_ultimas[1] - sola[1]).abs() < 1e-6, "{bajo:?} frente a {sola:?}");
}

#[test]
fn subrayar_en_el_texto_de_un_nodo() {
    sin_avisos(r"\begin{tikzpicture}\node {\underline{movie\_id}};\end{tikzpicture}");
}

#[test]
fn un_diagrama_conmutativo_dentro_de_un_nodo() {
    sin_avisos(r"\begin{tikzpicture}\node {\begin{tikzcd} a \ar[r] & b \end{tikzcd}};\end{tikzpicture}");
}

#[test]
fn una_orden_desconocida_no_se_lleva_la_instruccion_siguiente() {
    let c = componer(r"\begin{tikzpicture}\desconocida[a=1]\otra \node {x};\end{tikzpicture}");
    assert!(c.avisos.iter().any(|a| a.contains("not supported")), "{:?}", c.avisos);
    assert!(!c.escena.primitivas.is_empty());
}

const TABLA: &str = "\\begin{filecontents*}{datos.dat}\nx y\n0 0\n1 2\n2 1\n\\end{filecontents*}\n";

#[test]
fn una_tabla_de_filecontents_se_lee_y_no_se_pinta_como_texto() {
    let c = sin_avisos(&format!(
        r"{TABLA}\begin{{tikzpicture}}\begin{{axis}}\addplot table {{datos.dat}};\end{{axis}}\end{{tikzpicture}}"
    ));
    let sin_tabla = componer(
        r"\begin{tikzpicture}\begin{axis}\addplot coordinates {(0,0) (1,2) (2,1)};\end{axis}\end{tikzpicture}",
    );
    let (a, b) = (
        format!("{:?}", trazados(&c.escena.primitivas)),
        format!("{:?}", trazados(&sin_tabla.escena.primitivas)),
    );
    assert_eq!(a, b);
}

#[test]
fn update_limits_false_pinta_sin_mover_los_limites() {
    let eje = |plots: &str| {
        componer(&format!(
            r"\begin{{tikzpicture}}\begin{{axis}}\addplot coordinates {{(0,0) (1,1)}};{plots}\end{{axis}}\end{{tikzpicture}}"
        ))
    };
    let (solo, con) = (
        eje(""),
        eje(r"\addplot [update limits=false] coordinates {(0,0) (9,9)};"),
    );
    assert!(con.avisos.is_empty(), "{:?}", con.avisos);
    let dibujado: Vec<String> = trazados(&con.escena.primitivas)
        .iter()
        .map(|t| format!("{t:?}"))
        .collect();
    for t in trazados(&solo.escena.primitivas) {
        assert!(dibujado.contains(&format!("{t:?}")), "ha cambiado: {t:?}");
    }
}

#[test]
fn las_coordenadas_pasan_a_bp_con_la_y_hacia_abajo() {
    let c = componer(r"\begin{tikzpicture}\draw (0,0) -- (1,1);\end{tikzpicture}");
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    let lista = trazados(&c.escena.primitivas);
    let Primitiva::Trazado { segmentos, trazo, .. } = lista[0] else {
        unreachable!()
    };
    // `tikra` desplaza el dibujo para que su caja empiece en la esquina: cuenta la distancia
    let [Segmento::Mover(a), Segmento::Linea(b)] = segmentos[..] else {
        panic!("{segmentos:?}")
    };
    assert!((b.x - a.x - BP).abs() < 1e-9, "{a:?} {b:?}");
    assert!(
        (b.y - a.y + BP).abs() < 1e-9,
        "subir en TikZ es bajar la y: {a:?} {b:?}"
    );
    assert!(trazo.is_some());
}

#[test]
fn una_etiqueta_trae_el_contorno_de_sus_glifos() {
    let c = componer(r"\begin{tikzpicture}\node {$E=mc^2$};\end{tikzpicture}");
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    assert!(c.escena.primitivas.iter().any(|p| matches!(p, Primitiva::Glifo { .. })));
    assert!(comun::glifos_con_contorno(&c.escena));
    assert!(comun::capas_equilibradas(&c.escena.primitivas));
}

#[test]
fn el_texto_en_times_tambien_trae_sus_contornos() {
    // sus contornos solo existen en el SVG de `tikra`
    let c = componer(r"\begin{tikzpicture}\node[font=\fontfamily{ptm}\selectfont] {Texto};\end{tikzpicture}");
    assert!(comun::glifos_con_contorno(&c.escena), "{:?}", c.avisos);
    assert!(c.escena.contornos.values().all(|contorno| !contorno.is_empty()));
}

#[test]
fn se_conserva_lo_que_el_pintor_necesita() {
    let c = componer(
        r"\begin{tikzpicture}
          \clip (0,0) rectangle (4,4);
          \draw[dashed, red] (0,0) -- (2,2);
          \fill[opacity=0.5, blue] (0,0) circle (1);
          \shade[left color=red, right color=blue] (1,1) rectangle (2,2);
        \end{tikzpicture}",
    );
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    let e = &c.escena.primitivas;
    assert!(e.iter().any(|p| matches!(p, Primitiva::Abrir(Capa::Recorte { .. }))));
    assert!(comun::capas_equilibradas(e));
    assert!(e.iter().any(|p| matches!(
        p,
        Primitiva::Trazado { trazo: Some(t), .. } if t.discontinuo.is_some()
    )));
    assert!(e.iter().any(|p| matches!(
        p,
        Primitiva::Trazado { relleno: Some(r), .. } if r.opacidad == 0.5
    )));
    assert!(e.iter().any(|p| matches!(
        p,
        Primitiva::Trazado { relleno: Some(r), .. } if matches!(r.pintura, Pintura::Degradado(_))
    )));
}

#[test]
fn el_motor_soporta_macros_y_bucles() {
    let c = componer(
        r"\newcommand{\lado}{2}
        \begin{tikzpicture}\foreach \x in {0,...,3} { \draw (\x,0) rectangle (\x+\lado,1); }\end{tikzpicture}",
    );
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    assert_eq!(trazados(&c.escena.primitivas).len(), 4);
}

#[test]
fn lo_desconocido_se_avisa() {
    // `tikra` se salta la instrucción desconocida hasta su `;`, y lo dice
    let c = componer(r"\begin{tikzpicture}\ordeninventada{1}; \draw (0,0) -- (1,0);\end{tikzpicture}");
    assert!(!c.avisos.is_empty());
    assert_eq!(trazados(&c.escena.primitivas).len(), 1);
}

/// `tikra::convert` lee `\\\\coordinates` como `\\coordinate` escapado y se inventa la figura.
#[test]
fn un_salto_de_linea_antes_de_una_palabra_no_es_una_orden() {
    let c = componer(r"\begin{tikzpicture}\node[align=left] {en\\coordinates};\end{tikzpicture}");
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
}

#[test]
fn una_figura_que_no_dibuja_nada_se_avisa() {
    let c = componer(r"\begin{tikzpicture}\end{tikzpicture}");
    assert!(c.escena.primitivas.is_empty());
    assert_eq!(c.avisos, ["la figura no dibuja nada"]);
}

#[test]
fn sin_figura_es_un_error_y_no_una_escena_vacia() {
    assert!(FiguraTikra::default().componer("solo texto").is_err());
}

#[test]
fn una_biblioteca_externa_se_inyecta_sin_tocar_tikra() {
    fn doble(cx: &mut Context, valor: Option<Tokens>) -> Result<(), tikra::Error> {
        let color = cx.expand(valor.unwrap_or_default())?;
        cx.keys(Tokens::text(&format!("double={color}, double distance=1pt")))
    }
    fn definiciones() -> String {
        r"\tikzset{carretera/.style={double line=gray, line width=2pt}}".into()
    }
    static CARRETERAS: Library = Library {
        name: "carreteras",
        definitions: definiciones,
        keys: &[("/tikz/double line", doble)],
    };

    static BIBLIOTECAS: [&Library; 1] = [&CARRETERAS];

    let c = FiguraTikra {
        bibliotecas: &BIBLIOTECAS,
        ..FiguraTikra::default()
    }
    .componer(r"\begin{tikzpicture}\draw[carretera] (0,0) -- (2,0);\end{tikzpicture}")
    .expect("figura");
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    // la doble línea son dos trazos
    assert_eq!(trazados(&c.escena.primitivas).len(), 2);
}

#[test]
fn los_circuitos_se_dibujan() {
    let c = componer(
        r"\begin{circuitikz}
          \draw (0,0) to[battery1=$V$] (0,2) to[R=$R_1$] (3,2) to[C=$C$] (3,0) -- (0,0);
        \end{circuitikz}",
    );
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    assert!(trazados(&c.escena.primitivas).len() > 3);
    assert!(comun::glifos_con_contorno(&c.escena));
}

/// `voltik` gasta parte del límite de instrucciones: una figura sin circuitos no debe pagarlo.
#[test]
fn voltik_solo_se_carga_con_circuitos() {
    let figura = r"\begin{tikzpicture}\draw (0,0) -- (1,0);\end{tikzpicture}";
    let sin = FiguraTikra::default().componer(figura).expect("figura");
    assert!(sin.avisos.is_empty(), "{:?}", sin.avisos);
    let con = componer(r"\begin{circuitikz}\draw (0,0) to[R] (2,0);\end{circuitikz}");
    assert!(con.avisos.is_empty(), "{:?}", con.avisos);
    assert!(trazados(&con.escena.primitivas).len() > trazados(&sin.escena.primitivas).len());
}

#[test]
fn una_macro_propia_con_nombre_de_siunitx_se_respeta() {
    let c = componer(r"\def\unit{0.7}\begin{tikzpicture}\draw (0,0) circle (\unit);\end{tikzpicture}");
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    assert_eq!(trazados(&c.escena.primitivas).len(), 1);
}

#[test]
fn siunitx_funciona_en_una_etiqueta() {
    let c = componer(r"\begin{tikzpicture}\node {\SI{12}{\volt}};\end{tikzpicture}");
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    assert!(comun::glifos_con_contorno(&c.escena));
}

#[test]
fn una_escena_cruza_hilos() {
    fn es_send<T: Send + Sync>() {}
    es_send::<primitivas::Escena>();
}
