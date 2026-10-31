//! Lo que se mide mientras se pinta: cada parte anota lo suyo y quien quiera mostrarlo lo lee.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::puente_masiva::ffi::MedidasMasiva;

static VERTICES: AtomicU64 = AtomicU64::new(0);
static MS_TALLER: AtomicU64 = AtomicU64::new(0);
static MS_CARRIL: AtomicU64 = AtomicU64::new(0);
static ELEMENTOS: AtomicU64 = AtomicU64::new(0);
static FPS_MASIVA: AtomicU64 = AtomicU64::new(0);
static MS_MASIVA: AtomicU64 = AtomicU64::new(0);

fn guardar(celda: &AtomicU64, v: f64) {
    celda.store(v.to_bits(), Ordering::Relaxed);
}

fn leer_f64(celda: &AtomicU64) -> f64 {
    f64::from_bits(celda.load(Ordering::Relaxed))
}

/// Una foto de las medidas, en milisegundos y fotogramas por segundo.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Medidas {
    pub vertices: u64,
    pub ms_taller: f64,
    pub ms_carril: f64,
    pub elementos: u64,
    pub fps_masiva: f64,
    pub ms_masiva: f64,
}

pub fn anotar_taller(vertices: usize, ms: f64) {
    VERTICES.store(vertices as u64, Ordering::Relaxed);
    guardar(&MS_TALLER, ms);
}

pub fn anotar_lienzo(ms: f64) {
    guardar(&MS_CARRIL, ms);
}

pub fn anotar_masiva(m: MedidasMasiva) {
    ELEMENTOS.store(m.elementos, Ordering::Relaxed);
    guardar(&FPS_MASIVA, m.fotogramas_por_segundo);
    guardar(&MS_MASIVA, m.ms_carril);
}

pub fn leer() -> Medidas {
    Medidas {
        vertices: VERTICES.load(Ordering::Relaxed),
        ms_taller: leer_f64(&MS_TALLER),
        ms_carril: leer_f64(&MS_CARRIL),
        elementos: ELEMENTOS.load(Ordering::Relaxed),
        fps_masiva: leer_f64(&FPS_MASIVA),
        ms_masiva: leer_f64(&MS_MASIVA),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lo_anotado_se_lee() {
        anotar_masiva(MedidasMasiva {
            elementos: 7,
            fotogramas_por_segundo: 60.0,
            ms_carril: 0.5,
        });
        let m = leer();
        assert_eq!((m.elementos, m.fps_masiva, m.ms_masiva), (7, 60.0, 0.5));
    }
}
