//! De trazados a triángulos. Es la única pieza que conoce lyon.

use lyon::path::{FillRule, LineCap, LineJoin, Side};
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator, StrokeVertex,
    VertexBuffers,
};
use primitivas::{Remate, Segmento, Trazo, Union};

use crate::geometria::{Afin, discontinua, ruta};

/// Desviación máxima al aproximar curvas con segmentos, en píxeles.
const TOLERANCIA: f64 = 0.1;

/// Dibujos discontinuos más finos que esto no se distinguen de una línea continua, en píxeles.
const MENOR_CICLO: f64 = 0.25;

/// Un vértice recién teselado. `lado` solo vale en un trazo: a qué lado del centro cae.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bruto {
    pub x: f32,
    pub y: f32,
    pub lado: f32,
}

/// Cómo trazar: el ancho en el espacio del trazado, que puede no ser el de `trazo`, y cuántos píxeles
/// mide ahí una unidad.
#[derive(Debug, Clone, Copy)]
pub struct Pincel<'t> {
    pub trazo: &'t Trazo,
    pub ancho: f64,
    pub factor: f64,
}

/// Teseladores y memoria de trabajo: se reutilizan entre trazados.
pub struct Teselador {
    relleno: FillTessellator,
    trazo: StrokeTessellator,
    malla: VertexBuffers<Bruto, u32>,
}

impl Default for Teselador {
    fn default() -> Self {
        Teselador {
            relleno: FillTessellator::new(),
            trazo: StrokeTessellator::new(),
            malla: VertexBuffers::new(),
        }
    }
}

impl Teselador {
    /// Lo que dejó el último `rellenar` o `trazar` que devolvió `true`.
    pub fn malla(&self) -> (&[Bruto], &[u32]) {
        (&self.malla.vertices, &self.malla.indices)
    }

    /// Teselado en píxeles del documento: `m` lleva ahí los segmentos.
    pub fn rellenar(&mut self, segmentos: &[Segmento], par_impar: bool, m: Afin) -> bool {
        self.malla.clear();
        let Some(r) = ruta(segmentos, m) else {
            return false;
        };
        let regla = if par_impar {
            FillRule::EvenOdd
        } else {
            FillRule::NonZero
        };
        let opciones = FillOptions::tolerance(TOLERANCIA as f32).with_fill_rule(regla);
        let mut salida = BuffersBuilder::new(&mut self.malla, |v: FillVertex| Bruto {
            x: v.position().x,
            y: v.position().y,
            lado: 0.0,
        });
        self.relleno.tessellate_path(&r, &opciones, &mut salida).is_ok()
    }

    /// Teselado en el espacio del trazado: la transformación deforma también su ancho, y por eso se
    /// aplica después.
    pub fn trazar(&mut self, segmentos: &[Segmento], pincel: Pincel<'_>) -> bool {
        self.malla.clear();
        let Pincel {
            trazo: t,
            ancho,
            factor,
        } = pincel;
        if factor <= 0.0 || !factor.is_finite() {
            return false;
        }
        let Some(mut r) = ruta(segmentos, Afin::IDENTIDAD) else {
            return false;
        };
        let tolerancia = (TOLERANCIA / factor).max(1e-6) as f32;
        if let Some(d) = &t.discontinuo
            && d.tramos.iter().sum::<f64>() * factor >= MENOR_CICLO
        {
            r = discontinua(&r, d, tolerancia);
        }
        let remate = match t.remate {
            Remate::Plano => LineCap::Butt,
            Remate::Redondo => LineCap::Round,
            Remate::Cuadrado => LineCap::Square,
        };
        let union = match t.union {
            Union::Inglete => LineJoin::Miter,
            Union::Redonda => LineJoin::Round,
            Union::Biselada => LineJoin::Bevel,
        };
        let mut opciones = StrokeOptions::tolerance(tolerancia)
            .with_line_width(ancho as f32)
            .with_line_cap(remate)
            .with_line_join(union);
        // lyon compara la razón del inglete con el doble de su límite: el de PDF y SVG es la mitad.
        // Se escribe el campo porque `with_miter_limit` no admite menos de 1
        opciones.miter_limit = (t.limite_inglete.max(1.0) / 2.0) as f32;
        let mut salida = BuffersBuilder::new(&mut self.malla, |v: StrokeVertex| Bruto {
            x: v.position().x,
            y: v.position().y,
            lado: if v.side() == Side::Positive { 1.0 } else { -1.0 },
        });
        self.trazo.tessellate_path(&r, &opciones, &mut salida).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitivas::LIMITE_INGLETE;
    use primitivas::{Color, Punto};

    const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

    fn p(x: f64, y: f64) -> Punto {
        Punto { x, y }
    }

    fn area((v, i): (&[Bruto], &[u32])) -> f32 {
        i.chunks_exact(3)
            .map(|t| {
                let (a, b, c) = (v[t[0] as usize], v[t[1] as usize], v[t[2] as usize]);
                ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() / 2.0
            })
            .sum()
    }

    #[test]
    fn un_relleno_queda_en_pixeles_y_sin_lado() {
        let mut t = Teselador::default();
        let s = [
            Segmento::Mover(p(0.0, 0.0)),
            Segmento::Linea(p(2.0, 0.0)),
            Segmento::Linea(p(2.0, 2.0)),
            Segmento::Linea(p(0.0, 2.0)),
            Segmento::Cerrar,
        ];
        assert!(t.rellenar(&s, false, Afin::escala(3.0)));
        assert!((area(t.malla()) - 36.0).abs() < 1e-3);
        assert!(t.malla().0.iter().all(|v| v.lado == 0.0));
    }

    #[test]
    fn un_trazo_marca_cada_lado_y_no_se_transforma() {
        let mut t = Teselador::default();
        let trazo = Trazo::simple(NEGRO, 2.0);
        let pincel = Pincel {
            trazo: &trazo,
            ancho: 2.0,
            factor: 5.0,
        };
        let s = [Segmento::Mover(p(0.0, 0.0)), Segmento::Linea(p(10.0, 0.0))];
        assert!(t.trazar(&s, pincel));
        assert!((area(t.malla()) - 20.0).abs() < 1e-3);
        assert!(t.malla().0.iter().all(|v| v.lado == 1.0 || v.lado == -1.0));
        assert!(t.malla().0.iter().any(|v| v.lado == 1.0) && t.malla().0.iter().any(|v| v.lado == -1.0));
    }

    /// Hasta dónde llega la malla desde el vértice de un codo cuyo ángulo interior mide `grados`.
    fn alcance_del_codo(grados: f64, limite: f64) -> f32 {
        let mut t = Teselador::default();
        let trazo = Trazo {
            limite_inglete: limite,
            ..Trazo::simple(NEGRO, 1.0)
        };
        let pincel = Pincel {
            trazo: &trazo,
            ancho: 1.0,
            factor: 1.0,
        };
        let a = grados.to_radians();
        let s = [
            Segmento::Mover(p(100.0, 0.0)),
            Segmento::Linea(p(0.0, 0.0)),
            Segmento::Linea(p(100.0 * a.cos(), 100.0 * a.sin())),
        ];
        assert!(t.trazar(&s, pincel));
        // lo que queda detrás del codo, en el sentido contrario a los dos tramos
        t.malla().0.iter().map(|v| -v.x).fold(f32::MIN, f32::max)
    }

    fn distancia_al_tramo(v: &Bruto, a: (f64, f64), b: (f64, f64)) -> f64 {
        let (x, y) = (f64::from(v.x), f64::from(v.y));
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let t = (((x - a.0) * dx + (y - a.1) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        (x - a.0 - t * dx).hypot(y - a.1 - t * dy)
    }

    /// La regla, para cualquier ángulo: la malla no se aleja de la línea más que `LIMITE_INGLETE`
    /// medios anchos, y un trazo no sobresale de sus vértices.
    #[test]
    fn ningun_codo_sobresale_mas_de_un_ancho_sea_cual_sea_su_angulo() {
        let trazo = Trazo::simple(NEGRO, 1.0);
        let mut t = Teselador::default();
        for decimas in 2..1800 {
            let a = (f64::from(decimas) / 10.0).to_radians();
            let (vertice, punta) = ((0.0, 0.0), (100.0 * a.cos(), 100.0 * a.sin()));
            let s = [
                Segmento::Mover(p(100.0, 0.0)),
                Segmento::Linea(p(vertice.0, vertice.1)),
                Segmento::Linea(p(punta.0, punta.1)),
            ];
            let pincel = Pincel {
                trazo: &trazo,
                ancho: 1.0,
                factor: 1.0,
            };
            assert!(t.trazar(&s, pincel));
            for v in t.malla().0 {
                let d = distancia_al_tramo(v, (100.0, 0.0), vertice).min(distancia_al_tramo(v, vertice, punta));
                assert!(
                    d <= LIMITE_INGLETE / 2.0 + 1e-3,
                    "{} grados: {d}",
                    f64::from(decimas) / 10.0
                );
            }
        }
    }

    #[test]
    fn el_limite_de_inglete_significa_lo_mismo_que_en_pdf_y_svg() {
        // la razón del inglete es 1/sen(ángulo/2): a 8,5° vale 13,5 y a 15°, 7,7
        assert!(alcance_del_codo(8.5, 10.0) < 1.0, "supera el límite de 10: se bisela");
        let punta = alcance_del_codo(15.0, 10.0);
        assert!((punta - 3.8).abs() < 0.1, "no lo supera: punta entera, {punta}");
        assert!(alcance_del_codo(15.0, 7.0) < 1.0, "con otro límite ya se bisela");
    }

    #[test]
    fn un_limite_de_uno_bisela_siempre() {
        // a 60° la razón vale 2: la punta, en diagonal, llega a 0,87 y el bisel, a 0,43
        assert!(alcance_del_codo(60.0, 1.0) < 0.6);
        assert!(alcance_del_codo(60.0, 2.5) > 0.6);
    }

    #[test]
    fn sin_escala_no_hay_trazo_y_lo_que_no_es_un_numero_tampoco() {
        let mut t = Teselador::default();
        let trazo = Trazo::simple(NEGRO, 1.0);
        let pincel = Pincel {
            trazo: &trazo,
            ancho: 1.0,
            factor: 0.0,
        };
        let s = [Segmento::Mover(p(0.0, 0.0)), Segmento::Linea(p(1.0, 0.0))];
        assert!(!t.trazar(&s, pincel));
        assert!(!t.rellenar(&[Segmento::Mover(p(f64::NAN, 0.0))], false, Afin::IDENTIDAD));
    }
}
