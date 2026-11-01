//! Las máscaras de los glifos: una textura compartida, de solo cobertura.

use std::collections::HashMap;

use crate::textura::{Filas, Sucio};

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

/// Un glifo a un tamaño y en una posición subpíxel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Clave {
    pub contorno: u64,
    pub matriz: [i32; 4],
    pub sx: u8,
    pub sy: u8,
}

/// Una máscara rasterizada y dónde cae respecto al origen de su glifo.
#[derive(Debug, Clone, Copy)]
pub struct Mascara<'m> {
    pub pixeles: &'m [u8],
    pub ancho: u32,
    pub alto: u32,
    pub izquierda: i32,
    pub arriba: i32,
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

    /// Solo lo usado: el resto no se lee, y subir el lado entero son 4 MiB.
    pub fn marcar_todo(&mut self) {
        let usadas = self.y + self.alto_fila;
        if usadas > 0 {
            self.sucio.tocar(0, usadas);
        }
    }

    pub fn buscar(&self, clave: &Clave) -> Option<Entrada> {
        self.entradas.get(clave).copied()
    }

    /// `None` si ya no cabe.
    pub fn insertar(&mut self, clave: Clave, mascara: Mascara<'_>) -> Option<Entrada> {
        let Mascara {
            pixeles,
            ancho,
            alto,
            izquierda,
            arriba,
        } = mascara;
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
        for (fila, origen) in pixeles.chunks_exact(ancho.max(1) as usize).enumerate() {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// RAYO: reabrir un panel no debe subir el lado entero, 4 MiB, por unas pocas filas.
    #[test]
    fn marcar_todo_cubre_solo_lo_usado() {
        let mut a = Atlas::default();
        a.marcar_todo();
        assert_eq!(a.tomar_sucio(), Sucio::default(), "sin nada, nada que subir");
        let clave = Clave {
            contorno: 1,
            matriz: [0; 4],
            sx: 0,
            sy: 0,
        };
        a.insertar(clave, mascara(&[1; 6], 2, 3));
        a.tomar_sucio();
        a.marcar_todo();
        assert_eq!(a.tomar_sucio().bytes.len(), 3 * LADO_ATLAS as usize);
    }

    fn mascara(pixeles: &[u8], ancho: u32, alto: u32) -> Mascara<'_> {
        Mascara {
            pixeles,
            ancho,
            alto,
            izquierda: 0,
            arriba: 0,
        }
    }

    fn clave(n: u64) -> Clave {
        Clave {
            contorno: n,
            matriz: [0; 4],
            sx: 0,
            sy: 0,
        }
    }

    #[test]
    fn una_mascara_cae_donde_dice_su_entrada() {
        let mut a = Atlas::default();
        a.insertar(clave(1), mascara(&[1, 2, 3, 4], 2, 2)).unwrap();
        let m = Mascara {
            izquierda: -1,
            arriba: -3,
            ..mascara(&[9, 8, 7, 6], 2, 2)
        };
        let e = a.insertar(clave(2), m).unwrap();
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
        a.insertar(clave(1), mascara(&m, lado as u32, 10)).unwrap();
        let e = a.insertar(clave(2), mascara(&m, lado as u32, 10)).unwrap();
        assert_eq!((e.x, e.y), (0, 11));
    }

    #[test]
    fn un_atlas_lleno_dice_que_no() {
        let mut a = Atlas::default();
        let lado = LADO_ATLAS as usize;
        let m = vec![1; lado * lado];
        assert!(a.insertar(clave(1), mascara(&m, LADO_ATLAS, LADO_ATLAS)).is_some());
        assert!(a.insertar(clave(2), mascara(&[1], 1, 1)).is_none());
        a.limpiar();
        assert_eq!(a.buscar(&clave(1)), None);
        assert!(a.insertar(clave(2), mascara(&[1], 1, 1)).is_some());
    }
}
