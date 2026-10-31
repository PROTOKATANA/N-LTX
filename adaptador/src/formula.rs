//! Fórmulas, química y física: `mathra` las maqueta y aquí se vuelven primitivas.

use mathra::font::MathFont;
use mathra::layout::kerning::TextKerning;
use mathra::{Applied, Options, typeset_in_font};
use primitivas::{Caja, Color, Composicion, Compositor, ErrorComposicion, Escena, Punto};

use crate::caja::{Colocacion, Destino};

/// La misma fuente que las etiquetas de las figuras, para que todo se vea igual.
const FUENTE: MathFont = MathFont::LatinModern;

/// Compone una fórmula con su línea base en y = 0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormulaMathra {
    /// En bloque (display); si no, en línea.
    pub bloque: bool,
    /// Bp por em.
    pub tam: f64,
    /// Rechaza lo que no entiende en lugar de recuperarse con un aviso.
    pub estricto: bool,
    pub color: Color,
}

impl Compositor for FormulaMathra {
    fn componer(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
        let opciones = Options {
            display: self.bloque,
            strict: self.estricto,
            text_kerning: TextKerning::Ot1,
            ..Options::default()
        };
        let (caja, avisos) = typeset_in_font(fuente, &opciones, FUENTE).map_err(|e| ErrorComposicion(e.to_string()))?;

        let escala = self.tam / f64::from(FUENTE.get().units_per_em());
        let mut destino = Destino {
            avisos: avisos.iter().map(texto_aviso).collect(),
            ..Destino::default()
        };
        destino.caja(
            &caja.items,
            &Colocacion {
                origen: Punto::default(),
                escala,
                ancho_glifo: 1.0,
                fuente: FUENTE,
                color: self.color,
            },
        );
        let alto = f64::from(caja.height) * escala;
        Ok(Composicion {
            escena: Escena {
                caja: Caja {
                    x: 0.0,
                    y: -alto,
                    ancho: f64::from(caja.width) * escala,
                    alto: alto + f64::from(caja.depth) * escala,
                },
                fondo: None,
                primitivas: destino.primitivas,
                contornos: destino.contornos,
            },
            avisos: destino.avisos,
        })
    }
}

fn texto_aviso(aviso: &Applied) -> String {
    format!("{}: {}", aviso.rule, aviso.description)
}
