//! Lo que el modo texto lleva dentro: fórmulas y figuras compuestas por el motor.

use std::panic::{AssertUnwindSafe, catch_unwind};

use animacion::Reproductor;
use documento::Incrustados;
use primitivas::{Caja, Color, Composicion, Compositor, ErrorComposicion};

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
        protegido(|| FiguraTikra::default().componer(fuente))
    }

    fn caja_de_animacion(&self, fuente: &str) -> Result<Caja, ErrorComposicion> {
        protegido(|| Ok(Reproductor::nuevo(fuente, FiguraTikra::default())?.caja()))
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
