#![allow(clippy::unwrap_used, clippy::expect_used)]

mod comun;

use adaptador::FiguraTikra;
use primitivas::{Capa, Compositor, Pintura, Primitiva, Segmento};
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
