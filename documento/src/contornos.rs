//! Contornos de los glifos de cosmic-text, para que la escena se baste a sí misma.

use primitivas::{Punto, Segmento};
use ttf_parser::{Face, GlyphId, OutlineBuilder};

#[derive(Default)]
struct Contorno {
    segmentos: Vec<Segmento>,
    actual: Punto,
}

fn punto(x: f32, y: f32) -> Punto {
    Punto {
        x: f64::from(x),
        y: f64::from(y),
    }
}

impl OutlineBuilder for Contorno {
    fn move_to(&mut self, x: f32, y: f32) {
        self.actual = punto(x, y);
        self.segmentos.push(Segmento::Mover(self.actual));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.actual = punto(x, y);
        self.segmentos.push(Segmento::Linea(self.actual));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (control, fin) = (punto(x1, y1), punto(x, y));
        let hacia = |p: Punto| Punto {
            x: p.x + (control.x - p.x) * 2.0 / 3.0,
            y: p.y + (control.y - p.y) * 2.0 / 3.0,
        };
        self.segmentos
            .push(Segmento::Curva(hacia(self.actual), hacia(fin), fin));
        self.actual = fin;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.actual = punto(x, y);
        self.segmentos
            .push(Segmento::Curva(punto(x1, y1), punto(x2, y2), self.actual));
    }

    fn close(&mut self) {
        self.segmentos.push(Segmento::Cerrar);
    }
}

/// El contorno de un glifo, en unidades de su fuente y con la y hacia arriba, y esas unidades.
pub(crate) fn de_cara(datos: &[u8], indice: u32, glifo: u16) -> Option<(Vec<Segmento>, f64)> {
    let cara = Face::parse(datos, indice).ok()?;
    let mut contorno = Contorno::default();
    cara.outline_glyph(GlyphId(glifo), &mut contorno);
    Some((contorno.segmentos, f64::from(cara.units_per_em())))
}
