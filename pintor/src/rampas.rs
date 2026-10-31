//! Los colores de los degradados: una fila de una textura por cada degradado distinto.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use primitivas::{Color, Parada};

use crate::textura::{Filas, Sucio};

pub const ANCHO_RAMPA: u32 = 256;
pub const FILAS_RAMPAS: u32 = 512;

/// Una fila de color por cada degradado distinto.
pub struct Rampas {
    pixeles: Vec<u8>,
    filas: u32,
    conocidas: HashMap<u64, u32>,
    sucio: Filas,
}

impl Default for Rampas {
    fn default() -> Self {
        Rampas {
            pixeles: vec![0; (ANCHO_RAMPA * 4 * FILAS_RAMPAS) as usize],
            filas: 0,
            conocidas: HashMap::new(),
            sucio: Filas::default(),
        }
    }
}

impl Rampas {
    pub fn limpiar(&mut self) {
        self.truncar(0);
    }

    pub fn marcar_todo(&mut self) {
        self.sucio.tocar(0, FILAS_RAMPAS);
    }

    pub fn filas(&self) -> u32 {
        self.filas
    }

    /// Olvida lo añadido tras `filas`: lo de cada fotograma no debe acumularse.
    pub fn truncar(&mut self, filas: u32) {
        self.filas = filas;
        self.conocidas.retain(|_, f| *f < filas);
    }

    /// Dónde está la rampa de `paradas`, entre 0 y 1. Si no quedan filas, se reutiliza la última.
    pub fn fila(&mut self, paradas: &[Parada]) -> f32 {
        let mut huella = DefaultHasher::new();
        for p in paradas {
            for v in [p.posicion, p.color.r, p.color.g, p.color.b] {
                huella.write_u64(v.to_bits());
            }
        }
        let clave = huella.finish();
        let fila = match self.conocidas.get(&clave) {
            Some(f) => *f,
            None => {
                let f = self.filas.min(FILAS_RAMPAS - 1);
                self.pintar(f, paradas);
                self.filas = (f + 1).min(FILAS_RAMPAS);
                self.conocidas.insert(clave, f);
                f
            }
        };
        (fila as f32 + 0.5) / FILAS_RAMPAS as f32
    }

    fn pintar(&mut self, fila: u32, paradas: &[Parada]) {
        let largo = (ANCHO_RAMPA * 4) as usize;
        let inicio = fila as usize * largo;
        let destino = &mut self.pixeles[inicio..inicio + largo];
        for (i, px) in destino.chunks_exact_mut(4).enumerate() {
            let c = color_en(paradas, i as f64 / (ANCHO_RAMPA - 1) as f64);
            let canal = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            px.copy_from_slice(&[canal(c.r), canal(c.g), canal(c.b), 255]);
        }
        self.sucio.tocar(fila, fila + 1);
    }

    pub fn tomar_sucio(&mut self) -> Sucio {
        self.sucio.tomar(&self.pixeles, (ANCHO_RAMPA * 4) as usize)
    }
}

/// Antes de la primera parada y después de la última, el color se queda fijo.
pub(crate) fn color_en(paradas: &[Parada], t: f64) -> Color {
    let negro = Color { r: 0.0, g: 0.0, b: 0.0 };
    let Some(primera) = paradas.first() else {
        return negro;
    };
    let mut anterior = primera;
    for p in paradas {
        if t < p.posicion {
            let tramo = p.posicion - anterior.posicion;
            if tramo <= 0.0 {
                return p.color;
            }
            let u = (t - anterior.posicion) / tramo;
            let entre = |a: f64, b: f64| a + (b - a) * u;
            return Color {
                r: entre(anterior.color.r, p.color.r),
                g: entre(anterior.color.g, p.color.g),
                b: entre(anterior.color.b, p.color.b),
            };
        }
        anterior = p;
    }
    anterior.color
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parada(posicion: f64, r: f64) -> Parada {
        Parada {
            posicion,
            color: Color { r, g: 0.0, b: 0.0 },
        }
    }

    #[test]
    fn la_rampa_va_de_parada_en_parada() {
        let p = [parada(0.25, 0.0), parada(0.75, 1.0)];
        assert_eq!(color_en(&p, 0.0).r, 0.0, "antes de la primera");
        assert_eq!(color_en(&p, 1.0).r, 1.0, "después de la última");
        assert!((color_en(&p, 0.5).r - 0.5).abs() < 1e-9);
    }

    #[test]
    fn dos_paradas_iguales_son_un_salto() {
        let p = [parada(0.0, 0.0), parada(0.5, 0.0), parada(0.5, 1.0), parada(1.0, 1.0)];
        assert_eq!(color_en(&p, 0.49).r, 0.0);
        assert_eq!(color_en(&p, 0.51).r, 1.0);
    }

    #[test]
    fn un_degradado_repetido_comparte_fila_y_truncar_las_olvida() {
        let mut r = Rampas::default();
        let (a, b) = (
            [parada(0.0, 0.0), parada(1.0, 1.0)],
            [parada(0.0, 1.0), parada(1.0, 0.0)],
        );
        let primera = r.fila(&a);
        assert_eq!(r.fila(&a), primera);
        assert_ne!(r.fila(&b), primera);
        assert_eq!(r.filas(), 2);
        r.truncar(1);
        assert_eq!(r.fila(&a), primera, "la que queda sigue siendo válida");
        assert_eq!(r.filas(), 1);
        r.fila(&b);
        assert_eq!(r.filas(), 2);
    }

    #[test]
    fn la_ultima_fila_aguanta_lo_que_no_cabe() {
        let mut r = Rampas::default();
        for i in 0..FILAS_RAMPAS + 10 {
            r.fila(&[parada(0.0, i as f64 / 1000.0), parada(1.0, 1.0)]);
        }
        assert_eq!(r.filas(), FILAS_RAMPAS);
    }
}
