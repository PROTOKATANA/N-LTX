//! Lo que pinta la GPU: vértices, índices, tintas y una lista de comandos.

use std::sync::Arc;

use primitivas::{MapaDeBits, Punto};

use crate::atlas::{Entrada, LADO_ATLAS};
use crate::geometria::Afin;
use crate::puente_lienzo::ffi::{Comando, Orden, Tinta, Vertice};
use crate::teselado::Bruto;
use crate::tintas::{self, MAX_TINTAS};

/// Lo que sobresale la malla de un trazo por cada lado, en píxeles: el shader calcula la cobertura.
pub const FRANJA: f32 = 1.0;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lista {
    pub vertices: Vec<Vertice>,
    pub indices: Vec<u32>,
    pub tintas: Vec<Tinta>,
    pub comandos: Vec<Comando>,
    /// Cuántas tintas hay por delante de las de esta lista cuando se pinta tras otra.
    pub primera_tinta: u32,
    /// Las imágenes que usan sus comandos, sin repetir.
    pub imagenes: Vec<Arc<MapaDeBits>>,
}

/// Un tramo de `Lista::indices`.
#[derive(Debug, Clone, Copy)]
pub struct Rango {
    pub inicio: u32,
    pub cuenta: u32,
}

/// Cómo se pinta una malla.
#[derive(Debug, Clone, Copy)]
pub struct Acabado {
    pub color: u32,
    pub tinta: u32,
    /// Lo que falta para llevarla a píxeles del documento.
    pub transformacion: Option<Afin>,
    /// Medio ancho en píxeles de un trazo con franja: con él y la distancia al centro, que se
    /// interpola, el shader sabe cuánto cubre cada píxel.
    pub medio: f32,
}

impl Lista {
    /// La lista que va primero en un fotograma: la que aporta las tintas fijas.
    pub fn base() -> Lista {
        Lista {
            tintas: tintas::fijas(),
            ..Lista::default()
        }
    }

    /// Una lista que se pinta después de `base`.
    pub fn tras(base: &Lista) -> Lista {
        Lista {
            primera_tinta: base.tintas.len() as u32,
            ..Lista::default()
        }
    }

    pub fn anadir_malla(&mut self, (vertices, indices): (&[Bruto], &[u32]), acabado: Acabado) -> Rango {
        let Acabado {
            color,
            tinta,
            transformacion,
            medio,
        } = acabado;
        let primero = self.vertices.len() as u32;
        let inicio = self.indices.len() as u32;
        self.vertices.extend(vertices.iter().map(|b| {
            let (x, y) = match transformacion {
                Some(m) => {
                    let q = m.aplicar(Punto {
                        x: b.x as f64,
                        y: b.y as f64,
                    });
                    (q.x as f32, q.y as f32)
                }
                None => (b.x, b.y),
            };
            Vertice {
                x,
                y,
                u: b.lado * (medio + FRANJA),
                v: medio,
                rgba: color,
                tinta,
            }
        }));
        self.indices.extend(indices.iter().map(|i| i + primero));
        Rango {
            inicio,
            cuenta: indices.len() as u32,
        }
    }

    /// El cuadrado de una máscara del atlas con el origen de su glifo en (`x`, `y`). `None` si la
    /// máscara está vacía.
    pub fn anadir_cuadrado(&mut self, e: Entrada, (x, y): (f64, f64), color: u32) -> Option<Rango> {
        if e.ancho == 0 || e.alto == 0 {
            return None;
        }
        let primero = self.vertices.len() as u32;
        let inicio = self.indices.len() as u32;
        let (x0, y0) = (x as f32 + e.izquierda as f32, y as f32 + e.arriba as f32);
        let (x1, y1) = (x0 + e.ancho as f32, y0 + e.alto as f32);
        let lado = LADO_ATLAS as f32;
        let (u0, v0) = (e.x as f32 / lado, e.y as f32 / lado);
        let (u1, v1) = ((e.x + e.ancho) as f32 / lado, (e.y + e.alto) as f32 / lado);
        for (x, y, u, v) in [(x0, y0, u0, v0), (x1, y0, u1, v0), (x1, y1, u1, v1), (x0, y1, u0, v1)] {
            self.vertices.push(Vertice {
                x,
                y,
                u,
                v,
                rgba: color,
                tinta: tintas::GLIFO,
            });
        }
        self.indices.extend([0, 1, 2, 0, 2, 3].map(|i: u32| i + primero));
        Some(Rango { inicio, cuenta: 6 })
    }

    /// Los cuatro vértices de una imagen, en el orden de su cuadrado unidad.
    pub fn anadir_imagen(&mut self, esquinas: [Punto; 4], mapa: &Arc<MapaDeBits>) -> Rango {
        let primero = self.vertices.len() as u32;
        let inicio = self.indices.len() as u32;
        for (p, (u, v)) in esquinas
            .into_iter()
            .zip([(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)])
        {
            self.vertices.push(Vertice {
                x: p.x as f32,
                y: p.y as f32,
                u,
                v,
                rgba: u32::MAX,
                tinta: tintas::SOLIDA,
            });
        }
        self.indices.extend([0, 1, 2, 0, 2, 3].map(|i: u32| i + primero));
        if !self.imagenes.iter().any(|m| m.id == mapa.id) {
            self.imagenes.push(Arc::clone(mapa));
        }
        Rango { inicio, cuenta: 6 }
    }

    /// El índice de `tinta`, que no se repite si es igual a la anterior. `None` si ya no caben.
    pub fn anadir_tinta(&mut self, tinta: Tinta) -> Option<u32> {
        let primera = self.primera_tinta as usize;
        if self.tintas.last() != Some(&tinta) {
            if primera + self.tintas.len() >= MAX_TINTAS {
                return None;
            }
            self.tintas.push(tinta);
        }
        Some((primera + self.tintas.len() - 1) as u32)
    }

    pub fn comando(&mut self, orden: Orden, r: Rango, nivel: u32) {
        // dibujar seguido es un solo draw call
        if orden == Orden::Dibujar
            && let Some(u) = self.comandos.last_mut()
            && u.orden == orden
            && u.nivel == nivel
            && u.inicio + u.cuenta == r.inicio
        {
            u.cuenta += r.cuenta;
            return;
        }
        self.comandos.push(Comando {
            orden,
            inicio: r.inicio,
            cuenta: r.cuenta,
            nivel,
            ..comando_vacio()
        });
    }

    /// Lo que ocupa en vertical lo añadido desde el vértice `primero`.
    pub fn alturas_desde(&self, primero: usize) -> (f32, f32) {
        self.vertices[primero..]
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v.y), b.max(v.y)))
    }
}

pub fn comando_vacio() -> Comando {
    Comando {
        orden: Orden::Dibujar,
        inicio: 0,
        cuenta: 0,
        nivel: 0,
        opacidad: 1.0,
        mezcla: 0,
        con_mascara: false,
        salto: 0,
        y0: 0.0,
        y1: 0.0,
        imagen: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rango(inicio: u32) -> Rango {
        Rango { inicio, cuenta: 3 }
    }

    #[test]
    fn solo_se_funden_los_dibujos_seguidos_y_del_mismo_nivel() {
        let mut l = Lista::default();
        l.comando(Orden::Dibujar, rango(0), 0);
        l.comando(Orden::Dibujar, rango(3), 0);
        assert_eq!(l.comandos.len(), 1);
        l.comando(Orden::Dibujar, rango(6), 1);
        l.comando(Orden::Dibujar, rango(100), 1);
        l.comando(Orden::Soltar, rango(103), 1);
        l.comando(Orden::Soltar, rango(106), 1);
        assert_eq!(l.comandos.len(), 5);
    }

    #[test]
    fn una_lista_tras_otra_numera_sus_tintas_a_continuacion() {
        let base = Lista::base();
        let mut viva = Lista::tras(&base);
        let t = tintas::fijas()[0];
        assert_eq!(viva.anadir_tinta(t), Some(base.tintas.len() as u32));
        assert_eq!(viva.anadir_tinta(t), Some(base.tintas.len() as u32), "no se repite");
    }

    #[test]
    fn las_tintas_tienen_tope() {
        let mut l = Lista {
            primera_tinta: MAX_TINTAS as u32,
            ..Lista::default()
        };
        assert_eq!(l.anadir_tinta(tintas::fijas()[0]), None);
    }

    #[test]
    fn una_mascara_vacia_no_pinta_nada() {
        let e = Entrada {
            x: 0,
            y: 0,
            ancho: 0,
            alto: 0,
            izquierda: 0,
            arriba: 0,
        };
        assert!(Lista::default().anadir_cuadrado(e, (0.0, 0.0), 0).is_none());
    }
}
