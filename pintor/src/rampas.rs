//! Los colores de los degradados: una fila de una textura por cada degradado distinto.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use primitivas::{Color, Parada};

use crate::textura::{Filas, Sucio};

pub const ANCHO_RAMPA: u32 = 256;
pub const FILAS_RAMPAS: u32 = 4096;
/// Las últimas filas son de lo que se compone en cada fotograma: un documento que gaste todas las
/// demás no debe dejar sin color a sus animaciones.
const FILAS_VIVAS: u32 = 256;

/// Una fila de color por cada degradado distinto.
pub struct Rampas {
    pixeles: Vec<u8>,
    filas: u32,
    /// Hasta dónde se puede pintar ahora.
    tope: u32,
    conocidas: HashMap<u64, u32>,
    sucio: Filas,
}

impl Default for Rampas {
    fn default() -> Self {
        Rampas {
            pixeles: vec![0; (ANCHO_RAMPA * 4 * FILAS_RAMPAS) as usize],
            filas: 0,
            tope: FILAS_RAMPAS - FILAS_VIVAS,
            conocidas: HashMap::new(),
            sucio: Filas::default(),
        }
    }
}

impl Rampas {
    pub fn limpiar(&mut self) {
        self.olvidar(0);
        self.tope = FILAS_RAMPAS - FILAS_VIVAS;
    }

    pub fn marcar_todo(&mut self) {
        self.sucio.tocar(0, FILAS_RAMPAS);
    }

    pub fn filas(&self) -> u32 {
        self.filas
    }

    /// Olvida lo añadido tras `filas`: lo de cada fotograma no debe acumularse.
    pub fn truncar(&mut self, filas: u32) {
        self.olvidar(filas);
        self.tope = FILAS_RAMPAS;
    }

    fn olvidar(&mut self, filas: u32) {
        self.filas = filas;
        self.conocidas.retain(|_, f| *f < filas);
    }

    /// Dónde está la rampa de `paradas`, entre 0 y 1. Si no quedan filas, la más parecida.
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
                let f = self.nueva_o_parecida(paradas);
                self.conocidas.insert(clave, f);
                f
            }
        };
        (fila as f32 + 0.5) / FILAS_RAMPAS as f32
    }

    fn nueva_o_parecida(&mut self, paradas: &[Parada]) -> u32 {
        if self.filas >= self.tope {
            return self.parecida(paradas);
        }
        let fila = self.filas;
        self.pintar(fila, paradas);
        self.filas += 1;
        fila
    }

    /// La fila ya pintada que más se parece a `paradas`: sin sitio, mejor un color cercano que uno
    /// ajeno, y sin pisar el de nadie.
    fn parecida(&self, paradas: &[Parada]) -> u32 {
        let largo = (ANCHO_RAMPA * 4) as usize;
        let muestras: Vec<(usize, [u8; 3])> = (0..ANCHO_RAMPA as usize)
            .step_by(36)
            .map(|i| {
                let c = color_en(paradas, i as f64 / (ANCHO_RAMPA - 1) as f64);
                (i * 4, [canal(c.r), canal(c.g), canal(c.b)])
            })
            .collect();
        let distancia = |f: &u32| -> u32 {
            let fila = &self.pixeles[*f as usize * largo..][..largo];
            let de_muestra =
                |(i, c): &(usize, [u8; 3])| -> u32 { (0..3).map(|k| u32::from(fila[i + k].abs_diff(c[k]))).sum() };
            muestras.iter().map(de_muestra).sum()
        };
        (0..self.filas).min_by_key(distancia).unwrap_or(0)
    }

    fn pintar(&mut self, fila: u32, paradas: &[Parada]) {
        let largo = (ANCHO_RAMPA * 4) as usize;
        let inicio = fila as usize * largo;
        let destino = &mut self.pixeles[inicio..inicio + largo];
        for (i, px) in destino.chunks_exact_mut(4).enumerate() {
            let c = color_en(paradas, i as f64 / (ANCHO_RAMPA - 1) as f64);
            px.copy_from_slice(&[canal(c.r), canal(c.g), canal(c.b), 255]);
        }
        self.sucio.tocar(fila, fila + 1);
    }

    pub fn tomar_sucio(&mut self) -> Sucio {
        self.sucio.tomar(&self.pixeles, (ANCHO_RAMPA * 4) as usize)
    }
}

fn canal(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
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

    /// Una base que gasta todas sus filas, con rampas que van de `i/tope` a negro.
    fn base_llena() -> Rampas {
        let mut r = Rampas::default();
        let tope = FILAS_RAMPAS - FILAS_VIVAS;
        for i in 0..tope {
            r.fila(&[parada(0.0, f64::from(i) / f64::from(tope)), parada(1.0, 0.0)]);
        }
        assert_eq!(r.filas(), tope);
        r
    }

    #[test]
    fn sin_sitio_usa_la_fila_mas_parecida_y_no_pisa_ninguna() {
        let mut r = base_llena();
        let v = r.fila(&[parada(0.0, 0.5003), parada(1.0, 0.0)]);
        let fila = (v * FILAS_RAMPAS as f32).floor();
        let mitad = (FILAS_RAMPAS - FILAS_VIVAS) as f32 / 2.0;
        assert!((fila - mitad).abs() < 12.0, "{fila} no está cerca de {mitad}");
        assert_eq!(r.filas(), FILAS_RAMPAS - FILAS_VIVAS, "no se ha pintado ninguna fila");
    }

    #[test]
    fn un_fotograma_tiene_filas_aunque_la_base_las_gaste_todas() {
        let mut r = base_llena();
        let filas = r.filas();
        r.truncar(filas);
        r.fila(&[parada(0.0, 0.25), parada(0.5, 0.75), parada(1.0, 0.1)]);
        assert_eq!(r.filas(), filas + 1);
    }
}
