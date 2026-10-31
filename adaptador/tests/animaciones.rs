#![allow(clippy::unwrap_used, clippy::expect_used)]

mod comun;

use adaptador::{FiguraTikra, piezas_del_motor};
use animacion::Reproductor;
use documento::{Documento, Formato};
use primitivas::{Capa, Color, Compositor, Matriz, Primitiva, Segmento};

const PENDULO: &str = r"\begin{animacion}[t=0:360, duracion=2]
\begin{tikzpicture}
  \draw (0,0) -- (\t:1.5);
  \fill[red] (\t:1.5) circle (0.1);
\end{tikzpicture}
\end{animacion}";

/// El extremo de la varilla en la escena, deshaciendo la transformación del fotograma.
fn extremo(primitivas: &[Primitiva]) -> (f64, f64) {
    let m: Matriz = primitivas
        .iter()
        .find_map(|p| match p {
            Primitiva::Abrir(Capa::Transformacion(m)) => Some(*m),
            _ => None,
        })
        .expect("transformación");
    let punto = primitivas
        .iter()
        .find_map(|p| match p {
            Primitiva::Trazado { segmentos, .. } => segmentos.iter().find_map(|s| match s {
                Segmento::Linea(q) => Some(*q),
                _ => None,
            }),
            _ => None,
        })
        .expect("varilla");
    (punto.x + m.e, punto.y + m.f)
}

#[test]
fn la_figura_se_mueve_sin_temblar() {
    const BP: f64 = 72.0 / 2.54;
    let r = Reproductor::nuevo(PENDULO, FiguraTikra::default()).unwrap();
    // a 0°, 90° y 180° el extremo está a 1,5 cm del centro, que no debe moverse
    let (a, b, c) = (
        extremo(&r.fotograma(0.0).unwrap().escena.primitivas),
        extremo(&r.fotograma(0.5).unwrap().escena.primitivas),
        extremo(&r.fotograma(1.0).unwrap().escena.primitivas),
    );
    let radio = 1.5 * BP;
    assert!((a.0 - radio).abs() < 1e-3 && a.1.abs() < 1e-3, "{a:?}");
    assert!(b.0.abs() < 1e-3 && (b.1 + radio).abs() < 1e-3, "{b:?}");
    assert!((c.0 + radio).abs() < 1e-3 && c.1.abs() < 1e-3, "{c:?}");
}

#[test]
fn todos_los_fotogramas_comparten_caja() {
    let r = Reproductor::nuevo(PENDULO, FiguraTikra::default()).unwrap();
    let caja = r.caja();
    for n in 0..12 {
        let f = r.fotograma(f64::from(n) / 6.0).unwrap();
        assert!(f.avisos.is_empty(), "{:?}", f.avisos);
        assert_eq!(f.escena.caja, caja);
        assert!(comun::capas_equilibradas(&f.escena.primitivas));
    }
}

#[test]
fn un_documento_reserva_la_zona_de_su_animacion() {
    let negro = Color { r: 0.0, g: 0.0, b: 0.0 };
    let doc = Documento::nuevo(Formato::default(), piezas_del_motor(negro));
    let c = doc.componer(&format!("Un péndulo:\n\n{PENDULO}\n\nFin.")).unwrap();
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    let [zona] = &c.escena.zonas[..] else {
        panic!("{:?}", c.escena.zonas)
    };
    assert!(zona.caja.alto > 80.0);
    let r = Reproductor::nuevo(&zona.fuente, FiguraTikra::default()).unwrap();
    assert!(!r.fotograma(0.3).unwrap().escena.primitivas.is_empty());
}
