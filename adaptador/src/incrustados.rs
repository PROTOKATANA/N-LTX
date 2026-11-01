//! Lo que el modo texto lleva dentro: fórmulas y figuras compuestas por el motor.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::OnceLock;

use animacion::Filmoteca;
use documento::{Incrustados, Memoria, Piezas};
use primitivas::{Caja, Color, Composicion, Compositor, ErrorComposicion};

use crate::imagenes::ImagenesEnCarpetas;
use crate::resaltado::ResaltadorSyntect;
use crate::traduccion::ColoresXcolor;
use crate::{FiguraTikra, FormulaMathra};

/// Un pánico del motor en una figura no debe llevarse el documento entero: pasa a ser un error, y
/// esa figura se ve como fuente.
fn protegido<T>(componer: impl FnOnce() -> Result<T, ErrorComposicion>) -> Result<T, ErrorComposicion> {
    catch_unwind(AssertUnwindSafe(componer))
        .unwrap_or_else(|_| Err(ErrorComposicion("el compositor falló con esta entrada".into())))
}

/// Compone con `mathra` y `tikra`, recuperándose de lo que no entienda con un aviso.
pub struct IncrustadosMotor {
    pub color: Color,
}

/// Las animaciones de toda la aplicación: el documento pide su caja y el pintor sus fotogramas, y
/// prepararlas cuesta demasiado para hacerlo dos veces.
pub fn filmoteca() -> &'static Filmoteca<FiguraTikra> {
    static FILMOTECA: OnceLock<Filmoteca<FiguraTikra>> = OnceLock::new();
    FILMOTECA.get_or_init(|| {
        Filmoteca::nueva(|tinta| FiguraTikra {
            tinta,
            ..FiguraTikra::default()
        })
    })
}

impl Incrustados for IncrustadosMotor {
    fn formula(&self, latex: &str, bloque: bool, tam: f64) -> Result<Composicion, ErrorComposicion> {
        let formula = FormulaMathra {
            bloque,
            tam,
            estricto: false,
            color: self.color,
        };
        protegido(|| formula.componer(latex))
    }

    fn figura(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
        let figura = FiguraTikra {
            tinta: self.color,
            ..FiguraTikra::default()
        };
        protegido(|| figura.componer(fuente))
    }

    fn caja_de_animacion(&self, fuente: &str) -> Result<Caja, ErrorComposicion> {
        protegido(|| filmoteca().caja(fuente, self.color).map_err(ErrorComposicion))
    }

    fn anticipar(&self, animaciones: &[String]) {
        filmoteca().anticipar(animaciones, self.color);
    }
}

/// Las piezas que componen de verdad, sin repetir trabajo: `mathra` y `tikra`, `xcolor` y `syntect`.
pub fn piezas_del_motor(color: Color) -> Piezas {
    piezas_con_imagenes(color, Vec::new())
}

/// Las mismas piezas, con las imágenes de `carpetas`.
pub fn piezas_con_imagenes(color: Color, carpetas: Vec<PathBuf>) -> Piezas {
    Piezas {
        incrustados: Box::new(Memoria::nueva(IncrustadosMotor { color })),
        colores: Box::new(ColoresXcolor),
        resaltador: Box::new(Memoria::nueva(ResaltadorSyntect)),
        imagenes: Box::new(ImagenesEnCarpetas::nuevas(carpetas)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_panico_se_vuelve_un_error() {
        let r: Result<(), _> = protegido(|| panic!("a propósito"));
        assert!(r.is_err_and(|e| e.0.contains("falló")));
    }

    #[test]
    fn lo_demas_pasa_tal_cual() {
        assert_eq!(protegido(|| Ok(1)), Ok(1));
    }
}
