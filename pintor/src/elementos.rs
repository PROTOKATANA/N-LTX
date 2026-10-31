//! La prueba masiva: una galaxia de elementos que el vertex shader hace girar.
//!
//! La geometría se sube una vez y en cada fotograma solo viaja el tiempo: las posiciones las evalúa
//! la GPU. Por eso no le afecta el coste de recomponer, que limita las animaciones de TikZ.

use std::f32::consts::TAU;

use crate::puente_masiva::ffi::Elemento;

const POR_DEFECTO: u32 = 300_000;
const BRAZOS: usize = 4;

pub(crate) fn pedidos() -> u32 {
    std::env::var("NLTX_ELEMENTOS")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(POR_DEFECTO)
}

/// Generador congruencial: la prueba debe ser igual en cada ejecución.
struct Azar(u32);

impl Azar {
    fn siguiente(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
}

fn color(r: f32) -> u32 {
    // del centro, cálido, a los brazos, azules
    let t = r.clamp(0.0, 1.0);
    let canal = |dentro: f32, fuera: f32| ((dentro + (fuera - dentro) * t) * 255.0) as u32;
    (canal(1.0, 0.35)) | (canal(0.82, 0.55) << 8) | (canal(0.55, 1.0) << 16) | (200 << 24)
}

pub fn galaxia(destino: &mut [Elemento]) {
    let mut azar = Azar(0x4e4c_5458);
    for (i, e) in destino.iter_mut().enumerate() {
        let u = azar.siguiente();
        let r = u.sqrt() * 0.97 + 0.02;
        let brazo = (i % BRAZOS) as f32 * TAU / BRAZOS as f32;
        let dispersion = (azar.siguiente() + azar.siguiente() - 1.0) * (0.45 + 1.6 * (1.0 - r));
        let angulo = brazo + r * 4.5 + dispersion;
        *e = Elemento {
            x: r * angulo.cos(),
            y: r * angulo.sin(),
            fase: azar.siguiente() * TAU,
            radio: 0.4 + azar.siguiente() * 0.8,
            rgba: color(r),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vacios(n: usize) -> Vec<Elemento> {
        vec![
            Elemento {
                x: 0.0,
                y: 0.0,
                fase: 0.0,
                radio: 0.0,
                rgba: 0
            };
            n
        ]
    }

    #[test]
    fn el_diseno_en_memoria_es_el_del_shader() {
        assert_eq!(std::mem::size_of::<Elemento>(), 20);
    }

    #[test]
    fn dentro_del_lienzo_y_con_tamano() {
        let mut d = vacios(10_000);
        galaxia(&mut d);
        for e in &d {
            assert!(e.x.abs() <= 1.0 && e.y.abs() <= 1.0, "{e:?}");
            assert!(e.radio >= 0.4 && e.radio <= 1.2, "{e:?}");
        }
    }

    #[test]
    fn siempre_sale_lo_mismo() {
        let (mut a, mut b) = (vacios(100), vacios(100));
        galaxia(&mut a);
        galaxia(&mut b);
        assert_eq!(a, b);
    }

    #[test]
    fn el_color_va_del_centro_a_los_brazos() {
        let rojo = |c: u32| c & 0xff;
        let azul = |c: u32| (c >> 16) & 0xff;
        assert!(rojo(color(0.0)) > azul(color(0.0)));
        assert!(rojo(color(1.0)) < azul(color(1.0)));
    }
}
