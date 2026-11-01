//! Lo que el pintor aún no sabe hacer: se dice al publicar, no se descarta en silencio.

use primitivas::{Capa, Escena, Pintura, Primitiva};

/// Lo que este pintor no sabe hacer y por eso no se ve igual.
pub fn carencias(escena: &Escena) -> Vec<String> {
    let mut avisos = Vec::new();
    revisar(&escena.primitivas, &mut avisos);
    avisos
}

fn revisar(primitivas: &[Primitiva], avisos: &mut Vec<String>) {
    for p in primitivas {
        match p {
            Primitiva::Trazado { relleno: Some(r), .. } => match r.pintura {
                Pintura::Patron { .. } => avisar(avisos, "este pintor aproxima los patrones con su color"),
                Pintura::Mosaico { .. } => avisar(avisos, "este pintor no dibuja mosaicos"),
                _ => {}
            },
            Primitiva::Abrir(Capa::MascaraEscena(dentro)) => revisar(dentro, avisos),
            _ => {}
        }
    }
}

/// Cada carencia se dice una vez por escena.
fn avisar(avisos: &mut Vec<String>, aviso: &str) {
    if !avisos.iter().any(|a| a == aviso) {
        avisos.push(aviso.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitivas::{Color, Matriz, Punto, Relleno};

    const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

    #[test]
    fn lo_que_falta_se_avisa_una_vez_y_tambien_dentro_de_una_mascara() {
        let patron = Primitiva::Trazado {
            segmentos: Vec::new(),
            relleno: Some(Relleno {
                pintura: Pintura::Patron {
                    nombre: "dots".into(),
                    color: NEGRO,
                    esquina: Punto::default(),
                },
                opacidad: 1.0,
                par_impar: false,
            }),
            trazo: None,
        };
        let mosaico = Primitiva::Trazado {
            segmentos: Vec::new(),
            relleno: Some(Relleno {
                pintura: Pintura::Mosaico {
                    primitivas: Vec::new(),
                    origen: Punto::default(),
                    ancho: 1.0,
                    alto: 1.0,
                    matriz: Matriz {
                        a: 1.0,
                        b: 0.0,
                        c: 0.0,
                        d: 1.0,
                        e: 0.0,
                        f: 0.0,
                    },
                },
                opacidad: 1.0,
                par_impar: false,
            }),
            trazo: None,
        };
        let e = Escena {
            primitivas: vec![
                patron.clone(),
                patron,
                Primitiva::Abrir(Capa::MascaraEscena(vec![mosaico])),
                Primitiva::Cerrar,
            ],
            ..Escena::default()
        };
        assert_eq!(carencias(&e).len(), 2, "{:?}", carencias(&e));
    }
}
