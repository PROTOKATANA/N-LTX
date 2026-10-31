//! Las dos texturas del pintor: las máscaras de los glifos y las rampas de los degradados.
//!
//! Cada una guarda aquí su copia completa y apunta qué filas cambiaron, para subir solo esas.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

use primitivas::{Color, Parada};

/// Lado del atlas de glifos, en píxeles: 4 MiB, de sobra para decenas de miles de glifos.
pub const LADO_ATLAS: u32 = 2048;

/// Por encima de este tamaño en píxeles un glifo se pinta como trazado: una máscara ya gastaría
/// demasiado atlas, y lo que se ve grande perdona menos los dientes que el texto.
pub const MAYOR_GLIFO: f64 = 256.0;

/// Posiciones por píxel en cada eje: colocar los glifos al píxel entero hace bailar las letras y
/// cambia la forma de los trazos finos, también en los grandes, como una raíz alta.
pub const SUBPIXELES: f64 = 4.0;

/// Separación entre máscaras, para que el filtrado no toque a la vecina.
const HUECO: u32 = 1;

pub const ANCHO_RAMPA: u32 = 256;
pub const FILAS_RAMPAS: u32 = 512;

/// Las filas que hay que subir.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sucio {
    pub y: u32,
    pub bytes: Vec<u8>,
}

/// El rango de filas que cambió desde la última subida.
#[derive(Debug, Clone, Copy, Default)]
struct Filas(Option<(u32, u32)>);

impl Filas {
    fn tocar(&mut self, desde: u32, hasta: u32) {
        self.0 = Some(self.0.map_or((desde, hasta), |(a, b)| (a.min(desde), b.max(hasta))));
    }

    fn tomar(&mut self, pixeles: &[u8], largo_fila: usize) -> Sucio {
        match self.0.take() {
            Some((a, b)) => Sucio {
                y: a,
                bytes: pixeles[a as usize * largo_fila..b as usize * largo_fila].to_vec(),
            },
            None => Sucio::default(),
        }
    }
}

/// Un glifo a un tamaño y en una posición subpíxel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Clave {
    pub contorno: u64,
    pub matriz: [i32; 4],
    pub sx: u8,
    pub sy: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entrada {
    pub x: u32,
    pub y: u32,
    pub ancho: u32,
    pub alto: u32,
    /// Desde el origen del glifo hasta la esquina de la máscara.
    pub izquierda: i32,
    pub arriba: i32,
}

/// Empaquetado por filas. No expulsa: lo que no cabe se pinta como trazado.
pub struct Atlas {
    pixeles: Vec<u8>,
    x: u32,
    y: u32,
    alto_fila: u32,
    entradas: HashMap<Clave, Entrada>,
    sucio: Filas,
}

impl Default for Atlas {
    fn default() -> Self {
        Atlas {
            pixeles: vec![0; (LADO_ATLAS * LADO_ATLAS) as usize],
            x: 0,
            y: 0,
            alto_fila: 0,
            entradas: HashMap::new(),
            sucio: Filas::default(),
        }
    }
}

impl Atlas {
    /// Lo que queda en la GPU no estorba: solo se lee lo que se vuelva a insertar.
    pub fn limpiar(&mut self) {
        self.entradas.clear();
        (self.x, self.y, self.alto_fila) = (0, 0, 0);
    }

    pub fn marcar_todo(&mut self) {
        self.sucio.tocar(0, LADO_ATLAS);
    }

    pub fn buscar(&self, clave: &Clave) -> Option<Entrada> {
        self.entradas.get(clave).copied()
    }

    /// `None` si ya no cabe.
    pub fn insertar(
        &mut self,
        clave: Clave,
        mascara: &[u8],
        ancho: u32,
        alto: u32,
        izquierda: i32,
        arriba: i32,
    ) -> Option<Entrada> {
        if self.x + ancho > LADO_ATLAS {
            self.y += self.alto_fila + HUECO;
            (self.x, self.alto_fila) = (0, 0);
        }
        if ancho > LADO_ATLAS || self.y + alto > LADO_ATLAS {
            return None;
        }
        let entrada = Entrada {
            x: self.x,
            y: self.y,
            ancho,
            alto,
            izquierda,
            arriba,
        };
        for (fila, origen) in mascara.chunks_exact(ancho.max(1) as usize).enumerate() {
            let inicio = (self.y as usize + fila) * LADO_ATLAS as usize + self.x as usize;
            self.pixeles[inicio..inicio + ancho as usize].copy_from_slice(origen);
        }
        self.sucio.tocar(self.y, self.y + alto);
        self.x += ancho + HUECO;
        self.alto_fila = self.alto_fila.max(alto);
        self.entradas.insert(clave, entrada);
        Some(entrada)
    }

    pub fn tomar_sucio(&mut self) -> Sucio {
        self.sucio.tomar(&self.pixeles, LADO_ATLAS as usize)
    }
}

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

    fn clave(n: u64) -> Clave {
        Clave {
            contorno: n,
            matriz: [0; 4],
            sx: 0,
            sy: 0,
        }
    }

    fn parada(posicion: f64, r: f64) -> Parada {
        Parada {
            posicion,
            color: Color { r, g: 0.0, b: 0.0 },
        }
    }

    #[test]
    fn una_mascara_cae_donde_dice_su_entrada() {
        let mut a = Atlas::default();
        a.insertar(clave(1), &[1, 2, 3, 4], 2, 2, 0, 0).unwrap();
        let e = a.insertar(clave(2), &[9, 8, 7, 6], 2, 2, -1, -3).unwrap();
        assert_eq!((e.x, e.y), (3, 0), "con su hueco");
        assert_eq!((e.izquierda, e.arriba), (-1, -3));
        let s = a.tomar_sucio();
        assert_eq!((s.y, s.bytes.len()), (0, 2 * LADO_ATLAS as usize));
        assert_eq!(s.bytes[3], 9);
        assert_eq!(s.bytes[LADO_ATLAS as usize + 3], 7);
        assert_eq!(a.tomar_sucio(), Sucio::default(), "ya se subió");
    }

    #[test]
    fn lo_que_no_cabe_en_la_fila_baja_a_la_siguiente() {
        let mut a = Atlas::default();
        let lado = 1200usize;
        let m = vec![1; lado * 10];
        a.insertar(clave(1), &m, lado as u32, 10, 0, 0).unwrap();
        let e = a.insertar(clave(2), &m, lado as u32, 10, 0, 0).unwrap();
        assert_eq!((e.x, e.y), (0, 11));
    }

    #[test]
    fn un_atlas_lleno_dice_que_no() {
        let mut a = Atlas::default();
        let lado = LADO_ATLAS as usize;
        let m = vec![1; lado * lado];
        assert!(a.insertar(clave(1), &m, LADO_ATLAS, LADO_ATLAS, 0, 0).is_some());
        assert!(a.insertar(clave(2), &[1], 1, 1, 0, 0).is_none());
        a.limpiar();
        assert_eq!(a.buscar(&clave(1)), None);
        assert!(a.insertar(clave(2), &[1], 1, 1, 0, 0).is_some());
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
