//! `\includegraphics`: cuánto mide una imagen y dónde cae.

use primitivas::{ErrorComposicion, Matriz, Primitiva};

use super::{Columna, Maquetador, SEPARACION};
use crate::Imagen;
use crate::tema::Opciones;

const BP_POR_PULGADA: f64 = 72.0;

/// Una longitud de LaTeX en bp: `5cm`, `0.8\textwidth`, `\linewidth`… `None` si no se entiende.
fn longitud(texto: &str, columna: f64, tam: f64) -> Option<f64> {
    let texto = texto.trim();
    let corte = texto
        .find(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '+' | '-')))
        .unwrap_or(texto.len());
    let (cifra, unidad) = texto.split_at(corte);
    let cifra = if cifra.is_empty() {
        1.0
    } else {
        cifra.parse::<f64>().ok()?
    };
    let base = match unidad.trim() {
        "\\textwidth" | "\\linewidth" | "\\columnwidth" | "\\hsize" => columna,
        "cm" => BP_POR_PULGADA / 2.54,
        "mm" => BP_POR_PULGADA / 25.4,
        "in" => BP_POR_PULGADA,
        "pt" => BP_POR_PULGADA / 72.27,
        "bp" => 1.0,
        "em" => tam,
        "ex" => tam * 0.45,
        _ => return None,
    };
    Some(cifra * base)
}

/// Lo que piden las opciones de una imagen.
#[derive(Debug, Default, PartialEq)]
struct Pedido {
    ancho: Option<f64>,
    alto: Option<f64>,
    escala: Option<f64>,
    proporcion: bool,
    /// Las opciones que no se han podido atender.
    ignoradas: Vec<String>,
}

fn pedido(opciones: &Opciones, columna: f64, tam: f64) -> Pedido {
    let mut p = Pedido::default();
    for (clave, valor) in opciones {
        let hecho = match clave.as_str() {
            "width" => {
                p.ancho = longitud(valor, columna, tam);
                p.ancho.is_some()
            }
            "height" | "totalheight" => {
                p.alto = longitud(valor, columna, tam);
                p.alto.is_some()
            }
            "scale" => {
                p.escala = valor.trim().parse().ok();
                p.escala.is_some()
            }
            "keepaspectratio" => {
                p.proporcion = valor != "false";
                true
            }
            _ => false,
        };
        if !hecho {
            p.ignoradas.push(clave.clone());
        }
    }
    p
}

/// Lo que mide en la página. Como en `graphicx`: con ancho y alto se deforma, salvo con
/// `keepaspectratio`, y con uno solo conserva la proporción. Lo que no cabe se reduce: un documento
/// no se desplaza de lado.
fn medidas(natural: (f64, f64), p: &Pedido, columna: f64) -> (f64, f64) {
    let (ancho, alto) = natural;
    let razon = ancho / alto;
    let (w, h) = match (p.ancho, p.alto) {
        (Some(w), Some(h)) if p.proporcion => {
            let k = (w / ancho).min(h / alto);
            (ancho * k, alto * k)
        }
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, w / razon),
        (None, Some(h)) => (h * razon, h),
        (None, None) => {
            let k = p.escala.unwrap_or(1.0);
            (ancho * k, alto * k)
        }
    };
    if w > columna {
        (columna, h * columna / w)
    } else {
        (w, h)
    }
}

impl Maquetador<'_> {
    pub(super) fn imagen(&mut self, ruta: &str, opciones: &Opciones, c: Columna) {
        let imagen = self.piezas.imagenes.imagen(ruta);
        match imagen {
            Ok(i) => self.colocar_imagen(i, ruta, opciones, c),
            Err(e) => self.fuente_visible(&format!("\\includegraphics{{{ruta}}}"), &ErrorComposicion(e), c),
        }
    }

    fn colocar_imagen(&mut self, i: Imagen, ruta: &str, opciones: &Opciones, c: Columna) {
        let p = pedido(opciones, c.ancho, self.formato.tam);
        if !p.ignoradas.is_empty() {
            let ignoradas = p.ignoradas.join(", ");
            self.avisos
                .push(format!("{ruta}: opciones que no se atienden: {ignoradas}"));
        }
        let (ancho, alto) = medidas((i.ancho, i.alto), &p, c.ancho);
        self.separar(0.3);
        self.primitivas.push(Primitiva::Imagen {
            mapa: i.mapa,
            matriz: Matriz {
                a: ancho,
                b: 0.0,
                c: 0.0,
                d: alto,
                e: c.x + ((c.ancho - ancho) / 2.0).max(0.0),
                f: self.y,
            },
        });
        self.y += alto;
        self.separar(SEPARACION);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NATURAL: (f64, f64) = (200.0, 100.0);

    fn con(opciones: &[(&str, &str)]) -> Pedido {
        let o: Opciones = opciones.iter().map(|(c, v)| (c.to_string(), v.to_string())).collect();
        pedido(&o, 400.0, 10.0)
    }

    #[test]
    fn las_longitudes_de_latex() {
        let l = |t| longitud(t, 400.0, 10.0);
        assert_eq!(l("0.5\\textwidth"), Some(200.0));
        assert_eq!(l("\\linewidth"), Some(400.0));
        assert_eq!(l(".5\\columnwidth"), Some(200.0));
        assert_eq!(l("2in"), Some(144.0));
        assert_eq!(l("2em"), Some(20.0));
        assert!((l("2.54cm").unwrap() - 72.0).abs() < 1e-9);
        assert_eq!(l("72bp"), Some(72.0));
        assert_eq!(l("mucho"), None);
    }

    #[test]
    fn sin_opciones_mide_lo_natural() {
        assert_eq!(medidas(NATURAL, &con(&[]), 400.0), (200.0, 100.0));
    }

    #[test]
    fn con_un_solo_lado_conserva_la_proporcion() {
        assert_eq!(medidas(NATURAL, &con(&[("width", "100bp")]), 400.0), (100.0, 50.0));
        assert_eq!(medidas(NATURAL, &con(&[("height", "100bp")]), 400.0), (200.0, 100.0));
    }

    #[test]
    fn con_los_dos_se_deforma_salvo_que_se_pida_lo_contrario() {
        let ambos = [("width", "100bp"), ("height", "100bp")];
        assert_eq!(medidas(NATURAL, &con(&ambos), 400.0), (100.0, 100.0));
        let fija = [("width", "100bp"), ("height", "100bp"), ("keepaspectratio", "")];
        assert_eq!(medidas(NATURAL, &con(&fija), 400.0), (100.0, 50.0));
    }

    #[test]
    fn la_escala_parte_de_lo_natural() {
        assert_eq!(medidas(NATURAL, &con(&[("scale", "0.5")]), 400.0), (100.0, 50.0));
    }

    #[test]
    fn lo_que_no_cabe_se_reduce_a_la_columna() {
        assert_eq!(medidas((800.0, 400.0), &con(&[]), 400.0), (400.0, 200.0));
    }

    #[test]
    fn lo_que_no_se_atiende_se_dice() {
        let p = con(&[("trim", "1 2 3 4"), ("width", "ancho")]);
        assert_eq!(p.ignoradas, ["trim", "width"]);
    }
}
