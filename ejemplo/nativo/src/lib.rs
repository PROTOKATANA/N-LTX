//! Puerta C del ejemplo: Dart entrega LaTeX y el pintor GPU recoge la escena directamente.
//!
//! Lo que vuelve a Dart, el alto y los avisos, va en JSON. Eso es cosa de este ejemplo: en una
//! aplicación esta puerta sería `rinf`.

use std::collections::HashMap;
use std::panic::catch_unwind;
use std::sync::{Arc, Mutex, OnceLock};
use std::{ptr, slice};

use adaptador::{FiguraTikra, IncrustadosMotor};
use animacion::Reproductor;
use documento::{Documento, Formato, Memoria};
use pintor::{Animador, ancho_de_texto};
use primitivas::{Color, Composicion, Escena};
use serde_json::{Value, json};

/// Bp por em del texto y de las fórmulas.
const TAM_FORMULA: f64 = 14.0;

const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

/// Reserva `largo` bytes para que Dart escriba en ellos.
#[unsafe(no_mangle)]
pub extern "C" fn nltx_reservar(largo: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; largo].into_boxed_slice()).cast()
}

/// Libera lo que devolvieron `nltx_reservar` o `nltx_publicar`.
///
/// # Safety
/// `bufer` y `largo` deben ser exactamente los que entregó una de esas funciones, una sola vez.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nltx_liberar(bufer: *mut u8, largo: usize) {
    // SAFETY: el contrato de arriba: es el `Box<[u8]>` que se creó con este largo.
    drop(unsafe { Box::from_raw(ptr::slice_from_raw_parts_mut(bufer, largo)) });
}

type Reproductores = HashMap<String, Result<Arc<Reproductor<FiguraTikra>>, String>>;

/// Prepara una animación una sola vez: su caja exige componer el recorrido.
fn reproductor(fuente: &str) -> Result<Arc<Reproductor<FiguraTikra>>, String> {
    static LISTOS: OnceLock<Mutex<Reproductores>> = OnceLock::new();
    let mut listos = LISTOS.get_or_init(Default::default).lock().map_err(|e| e.to_string())?;
    listos
        .entry(fuente.to_string())
        .or_insert_with(|| {
            Reproductor::nuevo(fuente, FiguraTikra::default())
                .map(Arc::new)
                .map_err(|e| e.to_string())
        })
        .clone()
}

/// Recompone las animaciones en el hilo de render del pintor.
struct AnimadorTikra;

impl Animador for AnimadorTikra {
    fn fotograma(&self, fuente: &str, segundos: f64) -> Option<Escena> {
        let composicion =
            catch_unwind(|| reproductor(fuente).and_then(|r| r.fotograma(segundos).map_err(|e| e.to_string())));
        composicion.ok()?.ok().map(|c| c.escena)
    }
}

/// Cargar las fuentes y componer lo incrustado es lo caro: se hace una vez para toda la aplicación.
fn documento() -> &'static Documento {
    static DOCUMENTO: OnceLock<Documento> = OnceLock::new();
    DOCUMENTO.get_or_init(|| {
        let formato = Formato {
            tam: TAM_FORMULA,
            color: NEGRO,
            ..Formato::default()
        };
        Documento::nuevo(formato, Box::new(Memoria::nueva(IncrustadosMotor { color: NEGRO })))
    })
}

/// Compone `fuente` para una vista de `ancho` bp y se la entrega al pintor. Devuelve el alto en bp y
/// los avisos.
pub fn publicar(id: u32, fuente: &str, ancho: f64) -> Value {
    match documento().componer_a(fuente, ancho_de_texto(ancho)) {
        Ok(Composicion { escena, mut avisos }) => {
            let alto = escena.caja.alto + 2.0 * pintor::MARGEN;
            avisos.extend(pintor::publicar(id, escena, Some(Arc::new(AnimadorTikra))));
            json!({ "alto": alto, "avisos": avisos })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

/// Compone `fuente` para la superficie `id` y devuelve un JSON UTF-8 de `*largo_salida` bytes, a
/// liberar con `nltx_liberar`.
///
/// # Safety
/// `fuente` debe apuntar a `largo` bytes legibles y `largo_salida` a un `usize` escribible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nltx_publicar(
    id: u32,
    fuente: *const u8,
    largo: usize,
    ancho: f64,
    largo_salida: *mut usize,
) -> *mut u8 {
    // SAFETY: el contrato de arriba: Dart mantiene vivo el búfer durante la llamada.
    let texto = String::from_utf8_lossy(unsafe { slice::from_raw_parts(fuente, largo) });
    let valor = catch_unwind(|| publicar(id, &texto, ancho))
        .unwrap_or_else(|_| json!({ "error": "el compositor falló con esta entrada" }));
    let salida = valor.to_string().into_bytes().into_boxed_slice();
    // SAFETY: el contrato de arriba.
    unsafe { *largo_salida = salida.len() };
    Box::into_raw(salida).cast()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publicar_da_el_alto_con_su_margen_y_pasa_los_avisos() {
        let v = publicar(0xf0, "Hola", 400.0);
        assert!(v["alto"].as_f64().unwrap() > 2.0 * pintor::MARGEN, "{v}");
        let v = publicar(
            0xf0,
            r"\usetikzlibrary{patterns}\begin{tikzpicture}\fill[pattern=dots] (0,0) rectangle (1,1);\end{tikzpicture}",
            400.0,
        );
        assert!(v["avisos"].to_string().contains("patrones"), "{v}");
    }

    #[test]
    fn un_ancho_menor_alarga_el_documento() {
        let fuente = "Un párrafo con bastantes palabras para que se parta en varias líneas al estrecharlo.";
        let alto = |ancho| publicar(0xf1, fuente, ancho)["alto"].as_f64().unwrap();
        assert!(alto(120.0) > alto(800.0));
    }

    #[test]
    fn un_documento_mezcla_texto_y_formulas() {
        let v = publicar(0xf2, r"Sea $x^2$ y \ce{H2O}.", 400.0);
        assert_eq!(v["avisos"].as_array().unwrap().len(), 0, "{v}");
    }

    #[test]
    fn el_animador_no_deja_pasar_un_error() {
        let a = AnimadorTikra;
        assert!(a.fotograma("nada", 0.0).is_none());
        let fuente =
            r"\begin{animacion}[t=0:90]\begin{tikzpicture}\draw (0,0) -- (\t:1);\end{tikzpicture}\end{animacion}";
        assert!(a.fotograma(fuente, 1.0).is_some());
    }

    #[test]
    fn el_contrato_c_funciona_de_ida_y_vuelta() {
        let fuente = b"Hola";
        let mut largo = 0usize;
        // SAFETY: búfer válido y largo escribible; se libera con el largo que devolvió.
        let texto = unsafe {
            let salida = nltx_publicar(0xf3, fuente.as_ptr(), fuente.len(), 400.0, &mut largo);
            let texto = String::from_utf8(slice::from_raw_parts(salida, largo).to_vec()).unwrap();
            nltx_liberar(salida, largo);
            texto
        };
        assert!(texto.contains("alto"));
    }
}
