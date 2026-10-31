//! Dónde se encuentran los dos hilos: Flutter publica escenas y el carril del pintor pide fotogramas.
//!
//! La frontera con C++ son funciones libres, y quien publica y quien pinta no se conocen: esto es
//! lo único global del pintor, y por eso no contiene lógica.

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use primitivas::Escena;

use crate::carencias::carencias;
use crate::lienzo::{Animador, Lienzo, Marco};
use crate::puente_lienzo::ffi::Vista;

type Lienzos = Mutex<HashMap<u32, Lienzo>>;

fn lienzos() -> &'static Lienzos {
    static LIENZOS: OnceLock<Lienzos> = OnceLock::new();
    LIENZOS.get_or_init(Default::default)
}

/// Lo que verá la superficie `id` a partir de su próximo fotograma. Devuelve lo que no se podrá
/// ver como dice la escena: no hay que descartarlo en silencio.
pub fn publicar(id: u32, escena: Escena, animador: Option<Arc<dyn Animador>>) -> Vec<String> {
    let avisos = carencias(&escena);
    let mut lienzos = lienzos().lock().unwrap_or_else(PoisonError::into_inner);
    lienzos.entry(id).or_default().publicar(escena, animador);
    avisos
}

/// Cruza a C++: un pánico aquí tumbaría la aplicación, y un fotograma vacío no.
pub fn preparar(id: u32, vista: Vista, nuevos: bool) -> Box<Marco> {
    let marco = catch_unwind(AssertUnwindSafe(|| {
        let mut lienzos = lienzos().lock().unwrap_or_else(PoisonError::into_inner);
        lienzos.entry(id).or_default().preparar(vista, nuevos)
    }));
    Box::new(marco.unwrap_or_else(|_| Marco::vacio()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitivas::{Caja, Color, Pintura, Primitiva, Punto, Relleno, Segmento, Zona};

    fn vista() -> Vista {
        Vista {
            alto: 100.0,
            escala: 1.0,
            y: 0.0,
            segundos: 0.0,
        }
    }

    fn escena(zonas: Vec<Zona>) -> Escena {
        let p = |x, y| Punto { x, y };
        let cuadro = Primitiva::Trazado {
            segmentos: vec![
                Segmento::Mover(p(0.0, 0.0)),
                Segmento::Linea(p(10.0, 0.0)),
                Segmento::Linea(p(10.0, 10.0)),
                Segmento::Cerrar,
            ],
            relleno: Some(Relleno {
                pintura: Pintura::Solida(Color { r: 1.0, g: 0.0, b: 0.0 }),
                opacidad: 1.0,
                par_impar: false,
            }),
            trazo: None,
        };
        Escena {
            primitivas: vec![cuadro],
            zonas,
            ..Escena::default()
        }
    }

    #[test]
    fn un_pintor_que_llega_antes_que_la_escena_no_rompe_y_las_superficies_no_se_mezclan() {
        let (a, b) = (0xfff0, 0xfff1);
        publicar(a, escena(Vec::new()), None);
        assert!(!preparar(a, vista(), false).vertices_base().is_empty());
        assert!(preparar(b, vista(), false).vertices_base().is_empty());
    }

    struct Rompe;

    impl Animador for Rompe {
        fn fotograma(&self, _fuente: &str, _segundos: f64) -> Option<Escena> {
            panic!("a propósito");
        }
    }

    #[test]
    fn un_panico_da_un_fotograma_vacio_y_el_lienzo_sigue_vivo() {
        let id = 0xfff2;
        let zona = Zona {
            caja: Caja {
                x: 0.0,
                y: 0.0,
                ancho: 10.0,
                alto: 10.0,
            },
            fuente: "a".into(),
        };
        publicar(id, escena(vec![zona]), Some(Arc::new(Rompe)));
        assert!(preparar(id, vista(), false).comandos_base().is_empty());
        publicar(id, escena(Vec::new()), None);
        assert!(!preparar(id, vista(), false).comandos_base().is_empty());
    }
}
