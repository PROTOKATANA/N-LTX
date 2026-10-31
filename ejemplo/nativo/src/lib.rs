//! Puerta C del ejemplo: compone LaTeX y entrega la escena en JSON al pintor de Dart.
//!
//! El JSON es cosa de este ejemplo, no del motor: un pintor de verdad recibiría las primitivas por
//! un formato binario. Los trazados van planos: 0 = mover (x, y), 1 = línea (x, y), 2 = curva
//! (x1, y1, x2, y2, x, y), 3 = cerrar.

use std::panic::catch_unwind;
use std::{ptr, slice};

use adaptador::{FiguraTikra, FormulaMathra};
use primitivas::{
    Capa, Color, Composicion, Compositor, Degradado, Escena, GlifoId, Matriz, Parada, Pintura, Primitiva, Punto,
    Relleno, Remate, Segmento, Trazo, Union,
};
use serde_json::{Value, json};

/// Bp por em de las fórmulas.
const TAM_FORMULA: f64 = 14.0;

const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

/// Reserva `largo` bytes para que Dart escriba en ellos.
#[unsafe(no_mangle)]
pub extern "C" fn nltx_reservar(largo: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; largo].into_boxed_slice()).cast()
}

/// Libera lo que devolvieron `nltx_reservar` o `nltx_componer`.
///
/// # Safety
/// `bufer` y `largo` deben ser exactamente los que entregó una de esas funciones, una sola vez.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nltx_liberar(bufer: *mut u8, largo: usize) {
    // SAFETY: el contrato de arriba: es el `Box<[u8]>` que se creó con este largo.
    drop(unsafe { Box::from_raw(ptr::slice_from_raw_parts_mut(bufer, largo)) });
}

/// Compone `fuente` y devuelve un JSON UTF-8 de `*largo_salida` bytes, a liberar con `nltx_liberar`.
///
/// # Safety
/// `fuente` debe apuntar a `largo` bytes legibles y `largo_salida` a un `usize` escribible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nltx_componer(
    fuente: *const u8,
    largo: usize,
    figura: i32,
    largo_salida: *mut usize,
) -> *mut u8 {
    // SAFETY: el contrato de arriba: Dart mantiene vivo el búfer durante la llamada.
    let texto = String::from_utf8_lossy(unsafe { slice::from_raw_parts(fuente, largo) });
    // un pánico no puede cruzar la frontera C, y aquí se escribe a mano
    let valor = catch_unwind(|| componer(&texto, figura != 0))
        .unwrap_or_else(|_| json!({ "error": "el compositor falló con esta entrada" }));
    let salida = valor.to_string().into_bytes().into_boxed_slice();
    // SAFETY: el contrato de arriba.
    unsafe { *largo_salida = salida.len() };
    Box::into_raw(salida).cast()
}

pub fn componer(fuente: &str, figura: bool) -> Value {
    let resultado = if figura {
        FiguraTikra::default().componer(fuente)
    } else {
        FormulaMathra {
            bloque: true,
            tam: TAM_FORMULA,
            estricto: false,
            color: NEGRO,
        }
        .componer(fuente)
    };
    match resultado {
        Ok(composicion) => escena(composicion),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn escena(c: Composicion) -> Value {
    let Composicion { escena, mut avisos } = c;
    let Escena {
        caja,
        fondo,
        primitivas,
        contornos,
    } = escena;
    let lista = lista(&primitivas, &mut avisos);
    let contornos: serde_json::Map<String, Value> = contornos
        .iter()
        .map(|(glifo, contorno)| (clave(*glifo), json!(plano(contorno))))
        .collect();
    json!({
        "caja": [caja.x, caja.y, caja.ancho, caja.alto],
        "fondo": fondo.map(rgb),
        "primitivas": lista,
        "contornos": contornos,
        "avisos": avisos,
    })
}

fn clave(g: GlifoId) -> String {
    format!("{}:{}", g.fuente, g.id)
}

fn rgb(c: Color) -> [f64; 3] {
    [c.r, c.g, c.b]
}

fn seis(m: &Matriz) -> [f64; 6] {
    [m.a, m.b, m.c, m.d, m.e, m.f]
}

fn dos(p: Punto) -> [f64; 2] {
    [p.x, p.y]
}

fn plano(segmentos: &[Segmento]) -> Vec<f64> {
    let mut v = Vec::with_capacity(segmentos.len() * 3);
    for s in segmentos {
        match s {
            Segmento::Mover(p) => v.extend([0.0, p.x, p.y]),
            Segmento::Linea(p) => v.extend([1.0, p.x, p.y]),
            Segmento::Curva(a, b, p) => v.extend([2.0, a.x, a.y, b.x, b.y, p.x, p.y]),
            Segmento::Cerrar => v.push(3.0),
        }
    }
    v
}

fn lista(primitivas: &[Primitiva], avisos: &mut Vec<String>) -> Vec<Value> {
    primitivas.iter().filter_map(|p| primitiva(p, avisos)).collect()
}

fn primitiva(p: &Primitiva, avisos: &mut Vec<String>) -> Option<Value> {
    Some(match p {
        Primitiva::Trazado {
            segmentos,
            relleno: r,
            trazo: t,
        } => json!({
            "t": "trazado",
            "s": plano(segmentos),
            "r": r.as_ref().and_then(|r| relleno(r, avisos)),
            "k": t.as_ref().map(trazo),
        }),
        Primitiva::Glifo { glifo, matriz, color } => json!({
            "t": "glifo",
            "g": clave(*glifo),
            "m": seis(matriz),
            "v": rgb(*color),
        }),
        Primitiva::Imagen { .. } => {
            avisar(avisos, "este pintor no dibuja imágenes");
            return None;
        }
        Primitiva::Abrir(c) => capa(c, avisos),
        Primitiva::Cerrar => json!({ "t": "cerrar" }),
    })
}

/// Lo que este pintor no sabe hacer se dice, una vez por escena.
fn avisar(avisos: &mut Vec<String>, aviso: &str) {
    if !avisos.iter().any(|a| a == aviso) {
        avisos.push(aviso.to_string());
    }
}

fn capa(c: &Capa, avisos: &mut Vec<String>) -> Value {
    match c {
        Capa::Recorte { segmentos, par_impar } => {
            json!({ "t": "abrir", "c": "recorte", "s": plano(segmentos), "e": par_impar })
        }
        Capa::Grupo { opacidad } => json!({ "t": "abrir", "c": "grupo", "o": opacidad }),
        Capa::Transformacion(m) => json!({ "t": "abrir", "c": "matriz", "m": seis(m) }),
        Capa::Mezcla(modo) => json!({ "t": "abrir", "c": "mezcla", "n": modo }),
        Capa::MascaraDegradado(_) | Capa::MascaraEscena(_) => {
            avisar(avisos, "este pintor ignora las máscaras");
            json!({ "t": "abrir", "c": "nada" })
        }
        Capa::BordesNitidos => json!({ "t": "abrir", "c": "nada" }),
    }
}

fn relleno(r: &Relleno, avisos: &mut Vec<String>) -> Option<Value> {
    let pintura = match &r.pintura {
        Pintura::Solida(c) => json!({ "c": "solida", "v": rgb(*c) }),
        Pintura::Degradado(d) => degradado(d),
        Pintura::Patron { color, .. } => {
            avisar(avisos, "este pintor aproxima los patrones con su color");
            json!({ "c": "solida", "v": rgb(*color) })
        }
        Pintura::Mosaico { .. } => {
            avisar(avisos, "este pintor no dibuja mosaicos");
            return None;
        }
    };
    Some(json!({ "p": pintura, "o": r.opacidad, "e": r.par_impar }))
}

fn degradado(d: &Degradado) -> Value {
    let paradas = |p: &[Parada]| -> Vec<[f64; 4]> {
        p.iter()
            .map(|p| [p.posicion, p.color.r, p.color.g, p.color.b])
            .collect()
    };
    match d {
        Degradado::Lineal {
            desde,
            hasta,
            matriz,
            paradas: p,
        } => json!({ "c": "lineal", "a": dos(*desde), "b": dos(*hasta), "m": seis(matriz), "p": paradas(p) }),
        Degradado::Radial {
            radio,
            foco,
            matriz,
            paradas: p,
        } => json!({ "c": "radial", "r": radio, "f": dos(*foco), "m": seis(matriz), "p": paradas(p) }),
    }
}

fn trazo(t: &Trazo) -> Value {
    json!({
        "v": rgb(t.color),
        "w": t.ancho,
        "d": t.discontinuo.as_ref().map(|d| d.tramos.clone()),
        "f": t.discontinuo.as_ref().map_or(0.0, |d| d.fase),
        "c": match t.remate {
            Remate::Plano => 0,
            Remate::Redondo => 1,
            Remate::Cuadrado => 2,
        },
        "j": match t.union {
            Union::Inglete => 0,
            Union::Redonda => 1,
            Union::Biselada => 2,
        },
        "l": t.limite_inglete,
        "o": t.opacidad,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_formula_trae_primitivas_y_contornos() {
        let v = componer(r"\frac{a}{b}", false);
        assert!(v["error"].is_null(), "{v}");
        assert!(!v["primitivas"].as_array().unwrap().is_empty());
        assert!(!v["contornos"].as_object().unwrap().is_empty());
    }

    #[test]
    fn un_error_llega_como_tal() {
        assert!(componer("solo texto", true)["error"].is_string());
    }

    #[test]
    fn lo_que_no_se_pinta_se_avisa() {
        let v = componer(
            r"\usetikzlibrary{patterns}\begin{tikzpicture}\fill[pattern=dots] (0,0) rectangle (1,1);\end{tikzpicture}",
            true,
        );
        let avisos: Vec<&str> = v["avisos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a.as_str().unwrap())
            .collect();
        assert!(avisos.iter().any(|a| a.contains("patrones")), "{avisos:?}");
    }

    #[test]
    fn el_contrato_c_funciona_de_ida_y_vuelta() {
        let fuente = b"x^2";
        let mut largo = 0usize;
        // SAFETY: búfer válido y largo escribible; se libera con el largo que devolvió.
        let texto = unsafe {
            let salida = nltx_componer(fuente.as_ptr(), fuente.len(), 0, &mut largo);
            let texto = String::from_utf8(slice::from_raw_parts(salida, largo).to_vec()).unwrap();
            nltx_liberar(salida, largo);
            texto
        };
        assert!(texto.contains("primitivas"));
    }
}
