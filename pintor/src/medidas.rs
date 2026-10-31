//! Lo que se mide mientras se pinta: cada parte anota lo suyo y quien quiera mostrarlo lo lee.

use std::sync::atomic::{AtomicU64, Ordering};

static VERTICES: AtomicU64 = AtomicU64::new(0);
static MS_TALLER: AtomicU64 = AtomicU64::new(0);
static MS_CARRIL: AtomicU64 = AtomicU64::new(0);

fn guardar(celda: &AtomicU64, v: f64) {
    celda.store(v.to_bits(), Ordering::Relaxed);
}

fn leer_f64(celda: &AtomicU64) -> f64 {
    f64::from_bits(celda.load(Ordering::Relaxed))
}

/// Una foto de las medidas, en milisegundos.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Medidas {
    pub vertices: u64,
    pub ms_taller: f64,
    pub ms_carril: f64,
}

pub fn anotar_taller(vertices: usize, ms: f64) {
    VERTICES.store(vertices as u64, Ordering::Relaxed);
    guardar(&MS_TALLER, ms);
}

pub fn anotar_lienzo(ms: f64) {
    guardar(&MS_CARRIL, ms);
}

pub fn leer() -> Medidas {
    Medidas {
        vertices: VERTICES.load(Ordering::Relaxed),
        ms_taller: leer_f64(&MS_TALLER),
        ms_carril: leer_f64(&MS_CARRIL),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lo_anotado_se_lee() {
        anotar_taller(7, 2.5);
        anotar_lienzo(0.5);
        let m = leer();
        assert_eq!((m.vertices, m.ms_taller, m.ms_carril), (7, 2.5, 0.5));
    }
}
