//! Decide cómo se pinta un glifo y rasteriza su máscara. Es la única pieza que conoce zeno.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hasher;

use primitivas::{GlifoId, Punto, Segmento};
use zeno::{Command, Format, Mask, Placement, Vector};

use crate::atlas::{Atlas, Clave, Entrada, MAYOR_GLIFO, Mascara, SUBPIXELES};
use crate::geometria::Afin;

/// Lo que hace falta de un contorno para pintarlo.
#[derive(Clone, Copy)]
struct Preparado {
    huella: u64,
    /// Sus límites, en unidades de su fuente.
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

/// Cómo se pinta un glifo en un sitio concreto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Colocado<'e> {
    /// No tiene nada que pintar, como el espacio.
    Nada,
    /// Una máscara del atlas, con su origen en el píxel (`x`, `y`).
    EnAtlas { entrada: Entrada, x: f64, y: f64 },
    /// Demasiado grande, o ya no cabe: hay que rellenar su contorno.
    Contorno(&'e [Segmento]),
}

/// La línea base en un píxel entero, si no hay giro: es lo que hace Skia (`SkFont::setBaselineSnap`).
/// Con la y en cuartos de píxel, a escala 1,25 cada línea cae en una fracción distinta y sus trazos
/// horizontales se reparten entre dos filas: en K1 el salto de tinta en la base iba de 10 a 83.
pub fn en_linea_entera(d: Afin) -> Afin {
    if d.b == 0.0 && d.c == 0.0 {
        Afin { f: d.f.round(), ..d }
    } else {
        d
    }
}

/// Los glifos de una escena.
pub struct Glifos<'e> {
    contornos: &'e BTreeMap<GlifoId, Vec<Segmento>>,
    preparados: HashMap<GlifoId, Option<Preparado>>,
}

impl<'e> Glifos<'e> {
    pub fn nuevos(contornos: &'e BTreeMap<GlifoId, Vec<Segmento>>) -> Self {
        Glifos {
            contornos,
            preparados: HashMap::new(),
        }
    }

    /// `d` lleva el contorno a píxeles del documento.
    pub fn colocar(&mut self, glifo: GlifoId, d: Afin, atlas: &mut Atlas) -> Colocado<'e> {
        let Some(contorno) = self.contornos.get(&glifo) else {
            return Colocado::Nada;
        };
        let Some(p) = *self.preparados.entry(glifo).or_insert_with(|| preparar(contorno)) else {
            return Colocado::Nada;
        };
        let aparicion = Aparicion {
            contorno,
            preparado: p,
            d,
        };
        match en_atlas(aparicion, atlas) {
            Some((entrada, x, y)) => Colocado::EnAtlas { entrada, x, y },
            None => Colocado::Contorno(contorno),
        }
    }
}

/// Un glifo en un sitio: su contorno, lo que ya se sabe de él y adónde lo lleva `d`.
#[derive(Clone, Copy)]
struct Aparicion<'c> {
    contorno: &'c [Segmento],
    preparado: Preparado,
    d: Afin,
}

/// La máscara del glifo y dónde va su origen, si es lo bastante pequeño y cabe en el atlas.
fn en_atlas(aparicion: Aparicion<'_>, atlas: &mut Atlas) -> Option<(Entrada, f64, f64)> {
    let Aparicion {
        contorno,
        preparado: p,
        d,
    } = aparicion;
    if lado_en_pixeles(p, d) > MAYOR_GLIFO {
        return None;
    }
    // al más cercano: redondear hacia abajo subiría todo el texto un octavo de píxel de media
    let subpixel = |v: f64| {
        let cuantizado = (v * SUBPIXELES).round() / SUBPIXELES;
        let entero = cuantizado.floor();
        (entero, cuantizado - entero)
    };
    let ((x, sx), (y, sy)) = (subpixel(d.e), subpixel(d.f));
    let cuantizar = |v: f64| (v * 1048576.0).round() as i32;
    let clave = Clave {
        contorno: p.huella,
        matriz: [cuantizar(d.a), cuantizar(d.b), cuantizar(d.c), cuantizar(d.d)],
        sx: (sx * SUBPIXELES) as u8,
        sy: (sy * SUBPIXELES) as u8,
    };
    if let Some(e) = atlas.buscar(&clave) {
        return Some((e, x, y));
    }
    let (pixeles, sitio) = rasterizar(contorno, Afin { e: sx, f: sy, ..d });
    let mascara = Mascara {
        pixeles: &pixeles,
        ancho: sitio.width,
        alto: sitio.height,
        izquierda: sitio.left,
        arriba: sitio.top,
    };
    Some((atlas.insertar(clave, mascara)?, x, y))
}

fn lado_en_pixeles(p: Preparado, d: Afin) -> f64 {
    let esquinas = [(p.x0, p.y0), (p.x1, p.y0), (p.x1, p.y1), (p.x0, p.y1)];
    let cruz = esquinas.map(|(x, y)| (d.a * x + d.c * y, d.b * x + d.d * y));
    let (mut ancho, mut alto) = ((f64::MAX, f64::MIN), (f64::MAX, f64::MIN));
    for (x, y) in cruz {
        ancho = (ancho.0.min(x), ancho.1.max(x));
        alto = (alto.0.min(y), alto.1.max(y));
    }
    (ancho.1 - ancho.0).max(alto.1 - alto.0)
}

/// `None` si el glifo no tiene nada que pintar, como el espacio.
fn preparar(contorno: &[Segmento]) -> Option<Preparado> {
    // las coordenadas enteras solo varían en sus bits altos: hace falta un hash que los mezcle bien
    let mut huella = DefaultHasher::new();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    let mut punto = |p: Punto| {
        huella.write_u64(p.x.to_bits());
        huella.write_u64(p.y.to_bits());
        (x0, y0) = (x0.min(p.x), y0.min(p.y));
        (x1, y1) = (x1.max(p.x), y1.max(p.y));
    };
    for s in contorno {
        match *s {
            Segmento::Mover(p) | Segmento::Linea(p) => punto(p),
            Segmento::Curva(a, b, c) => {
                punto(a);
                punto(b);
                punto(c);
            }
            Segmento::Cerrar => {}
        }
    }
    (x1 >= x0).then_some(Preparado {
        huella: huella.finish(),
        x0,
        y0,
        x1,
        y1,
    })
}

/// La máscara del contorno con su origen donde diga `m`. zeno mide el área exacta de cada píxel: es
/// lo que hace que el texto pequeño se vea bien.
fn rasterizar(contorno: &[Segmento], m: Afin) -> (Vec<u8>, Placement) {
    let v = |p: Punto| {
        let q = m.aplicar(p);
        Vector::new(q.x as f32, q.y as f32)
    };
    let comandos: Vec<Command> = contorno
        .iter()
        .map(|s| match *s {
            Segmento::Mover(p) => Command::MoveTo(v(p)),
            Segmento::Linea(p) => Command::LineTo(v(p)),
            Segmento::Curva(a, b, c) => Command::CurveTo(v(a), v(b), v(c)),
            Segmento::Cerrar => Command::Close,
        })
        .collect();
    Mask::new(&comandos).format(Format::Alpha).render()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Punto {
        Punto { x, y }
    }

    fn cuadrado(lado: f64) -> Vec<Segmento> {
        vec![
            Segmento::Mover(p(0.0, 0.0)),
            Segmento::Linea(p(lado, 0.0)),
            Segmento::Linea(p(lado, lado)),
            Segmento::Linea(p(0.0, lado)),
            Segmento::Cerrar,
        ]
    }

    #[test]
    fn dos_contornos_de_coordenadas_enteras_no_comparten_huella() {
        let mut vistas = std::collections::HashSet::new();
        for a in 0..300 {
            for b in 0..300 {
                let contorno = [
                    Segmento::Mover(p(a as f64, 0.0)),
                    Segmento::Linea(p(700.0, b as f64)),
                    Segmento::Linea(p(a as f64, 512.0)),
                    Segmento::Cerrar,
                ];
                assert!(vistas.insert(preparar(&contorno).unwrap().huella), "{a} {b}");
            }
        }
    }

    #[test]
    fn segun_su_tamano_va_al_atlas_o_se_rellena_y_sin_contorno_no_hay_nada() {
        let id = |id| GlifoId { fuente: 0, id };
        let contornos = BTreeMap::from([(id(1), cuadrado(8.0)), (id(2), cuadrado(500.0)), (id(3), Vec::new())]);
        let (mut g, mut atlas) = (Glifos::nuevos(&contornos), Atlas::default());
        let d = Afin::IDENTIDAD;
        assert!(matches!(g.colocar(id(1), d, &mut atlas), Colocado::EnAtlas { .. }));
        assert!(matches!(g.colocar(id(2), d, &mut atlas), Colocado::Contorno(_)));
        assert_eq!(g.colocar(id(3), d, &mut atlas), Colocado::Nada);
        assert_eq!(g.colocar(id(4), d, &mut atlas), Colocado::Nada);
    }
}
