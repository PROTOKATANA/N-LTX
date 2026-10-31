//! Lo que el modo texto lleva dentro: fórmulas y figuras compuestas por el motor.

use animacion::Reproductor;
use documento::Incrustados;
use primitivas::{Caja, Color, Composicion, Compositor, ErrorComposicion};

use crate::{FiguraTikra, FormulaMathra};

/// Compone con `mathra` y `tikra`, recuperándose de lo que no entienda con un aviso.
pub struct IncrustadosMotor {
    pub color: Color,
}

impl Incrustados for IncrustadosMotor {
    fn formula(&self, latex: &str, bloque: bool, tam: f64) -> Result<Composicion, ErrorComposicion> {
        FormulaMathra {
            bloque,
            tam,
            estricto: false,
            color: self.color,
        }
        .componer(latex)
    }

    fn figura(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
        FiguraTikra::default().componer(fuente)
    }

    fn caja_de_animacion(&self, fuente: &str) -> Result<Caja, ErrorComposicion> {
        Ok(Reproductor::nuevo(fuente, FiguraTikra::default())?.caja())
    }
}
