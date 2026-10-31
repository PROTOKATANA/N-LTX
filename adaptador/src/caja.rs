//! Las cajas de `mathra` como primitivas: lo usan las fórmulas y las etiquetas de las figuras.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use mathra::font::{GlyphId, MathFont};
use mathra::layout::Item;
use primitivas::{Capa, Color, GlifoId, Matriz, Pintura, Primitiva, Punto, Relleno, Segmento, Trazo};

use crate::contorno;
use crate::traduccion::color_xcolor;

/// Los glifos desde este id son de TeX Gyre Termes: `tikra` los mete en el espacio de la fuente
/// Times y sus contornos no se piden a `mathra`.
pub(crate) const PRIMER_TERMES: u16 = 0xC000;

/// Reparto de la circunferencia en cuatro cúbicas: 4/3·(√2 − 1).
const KAPPA: f64 = 0.552_284_749_830_793_6;

/// Lo que va saliendo de traducir.
#[derive(Default)]
pub(crate) struct Destino {
    pub primitivas: Vec<Primitiva>,
    pub contornos: BTreeMap<GlifoId, Vec<Segmento>>,
    /// Glifos de Termes cuyo contorno debe poner quien conozca `tikra`.
    pub pendientes: BTreeSet<GlifoId>,
    pub avisos: Vec<String>,
}

/// Dónde y cómo va una caja.
pub(crate) struct Colocacion {
    /// La línea base, en bp y con la y hacia abajo.
    pub origen: Punto,
    /// Bp por unidad de fuente.
    pub escala: f64,
    /// Cuánto se ensanchan los glifos.
    pub ancho_glifo: f64,
    pub fuente: MathFont,
    /// El color de lo que no trae el suyo.
    pub color: Color,
}

impl Colocacion {
    fn punto(&self, x: f32, y: f32) -> Punto {
        Punto {
            x: self.origen.x + f64::from(x) * self.escala,
            y: self.origen.y - f64::from(y) * self.escala,
        }
    }
}

impl Destino {
    pub(crate) fn caja(&mut self, items: &[Item], lugar: &Colocacion) {
        for item in items {
            match item {
                Item::Glyph {
                    id,
                    x,
                    y,
                    scale,
                    color,
                    flip,
                } => {
                    let lado = f64::from(*scale) * lugar.escala;
                    // un glifo reflejado (`\reflectbox`) se dibuja hacia la izquierda desde su x
                    let ancho = lado * lugar.ancho_glifo * if *flip { -1.0 } else { 1.0 };
                    let p = lugar.punto(*x, *y);
                    let color = self.color(color, lugar.color);
                    let glifo = self.registrar_glifo(lugar.fuente, *id);
                    self.primitivas.push(Primitiva::Glifo {
                        glifo,
                        matriz: Matriz {
                            a: ancho,
                            b: 0.0,
                            c: 0.0,
                            d: -lado,
                            e: p.x,
                            f: p.y,
                        },
                        color,
                    });
                }
                Item::Rule {
                    x,
                    y,
                    width,
                    height,
                    color,
                } => {
                    let color = self.color(color, lugar.color);
                    self.primitivas.push(Primitiva::Trazado {
                        segmentos: rectangulo(
                            lugar.punto(*x, *y + *height),
                            f64::from(*width) * lugar.escala,
                            f64::from(*height) * lugar.escala,
                        ),
                        relleno: Some(Relleno {
                            pintura: Pintura::Solida(color),
                            opacidad: 1.0,
                            par_impar: false,
                        }),
                        trazo: None,
                    });
                }
                Item::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    thickness,
                    color,
                } => {
                    let color = self.color(color, lugar.color);
                    self.primitivas.push(Primitiva::Trazado {
                        segmentos: vec![
                            Segmento::Mover(lugar.punto(*x1, *y1)),
                            Segmento::Linea(lugar.punto(*x2, *y2)),
                        ],
                        relleno: None,
                        trazo: Some(Trazo::simple(color, f64::from(*thickness) * lugar.escala)),
                    });
                }
                Item::Ellipse {
                    cx,
                    cy,
                    rx,
                    ry,
                    thickness,
                    color,
                } => {
                    let color = self.color(color, lugar.color);
                    self.primitivas.push(Primitiva::Trazado {
                        segmentos: elipse(
                            lugar.punto(*cx, *cy),
                            f64::from(*rx) * lugar.escala,
                            f64::from(*ry) * lugar.escala,
                        ),
                        relleno: None,
                        trazo: Some(Trazo::simple(color, f64::from(*thickness) * lugar.escala)),
                    });
                }
                Item::Transformed { matrix, items } => {
                    // lo de dentro se dibuja desde su origen y luego se lleva adonde dice la caja;
                    // (x, y) va a (a·x + c·y + e, b·x + d·y + f) con la y hacia arriba
                    let [a, b, c, d, e, f] = matrix.map(f64::from);
                    self.primitivas.push(Primitiva::Abrir(Capa::Transformacion(Matriz {
                        a,
                        b: -b,
                        c: -c,
                        d,
                        e: lugar.origen.x + e * lugar.escala,
                        f: lugar.origen.y - f * lugar.escala,
                    })));
                    self.caja(
                        items,
                        &Colocacion {
                            origen: Punto::default(),
                            ..*lugar
                        },
                    );
                    self.primitivas.push(Primitiva::Cerrar);
                }
            }
        }
    }

    fn registrar_glifo(&mut self, fuente: MathFont, id: GlyphId) -> GlifoId {
        let glifo = GlifoId {
            fuente: indice_fuente(fuente),
            id: id.0,
        };
        if id.0 >= PRIMER_TERMES {
            self.pendientes.insert(glifo);
        } else {
            self.contornos
                .entry(glifo)
                .or_insert_with(|| contorno::segmentos(&fuente.get().outline(id)));
        }
        glifo
    }

    /// El color propio de un elemento, o el heredado. Uno ilegible se avisa: nunca se pierde en
    /// silencio.
    fn color(&mut self, propio: &Option<String>, heredado: Color) -> Color {
        let Some(texto) = propio else { return heredado };
        let leido = color_xcolor(texto).or_else(|| {
            svgtypes::Color::from_str(texto).ok().map(|c| Color {
                r: f64::from(c.red) / 255.0,
                g: f64::from(c.green) / 255.0,
                b: f64::from(c.blue) / 255.0,
            })
        });
        leido.unwrap_or_else(|| {
            self.avisos.push(format!("color ilegible: {texto}"));
            heredado
        })
    }
}

pub(crate) fn indice_fuente(fuente: MathFont) -> u8 {
    match fuente {
        MathFont::LatinModern => 0,
        MathFont::StixTwo => 1,
    }
}

fn rectangulo(esquina: Punto, ancho: f64, alto: f64) -> Vec<Segmento> {
    let en = |x: f64, y: f64| Punto {
        x: esquina.x + x,
        y: esquina.y + y,
    };
    vec![
        Segmento::Mover(en(0.0, 0.0)),
        Segmento::Linea(en(ancho, 0.0)),
        Segmento::Linea(en(ancho, alto)),
        Segmento::Linea(en(0.0, alto)),
        Segmento::Cerrar,
    ]
}

fn elipse(centro: Punto, rx: f64, ry: f64) -> Vec<Segmento> {
    let en = |x: f64, y: f64| Punto {
        x: centro.x + x * rx,
        y: centro.y + y * ry,
    };
    let curva = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| Segmento::Curva(en(a.0, a.1), en(b.0, b.1), en(c.0, c.1));
    vec![
        Segmento::Mover(en(1.0, 0.0)),
        curva((1.0, KAPPA), (KAPPA, 1.0), (0.0, 1.0)),
        curva((-KAPPA, 1.0), (-1.0, KAPPA), (-1.0, 0.0)),
        curva((-1.0, -KAPPA), (-KAPPA, -1.0), (0.0, -1.0)),
        curva((KAPPA, -1.0), (1.0, -KAPPA), (1.0, 0.0)),
        Segmento::Cerrar,
    ]
}
