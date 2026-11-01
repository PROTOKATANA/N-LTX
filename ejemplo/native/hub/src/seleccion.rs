//! La selección del documento publicado. Guarda las anclas y las fuentes, responde a las señales de
//! Dart y le dice al taller qué marcar. Lo que se copia sale de `documento::seleccion`.

use std::sync::{Arc, Mutex, PoisonError};

use documento::seleccion::{cajas, cursor_en, fuente_de, palabra_en};
use pintor::{Animador, MARGEN, Taller};
use primitivas::{Ancla, Caja, Escena, Punto};

use crate::compositor::Destino;

#[derive(Debug, Default)]
struct Estado {
    anclas: Vec<Ancla>,
    fuentes: Vec<String>,
    /// Las anclas seleccionadas.
    desde: usize,
    hasta: usize,
}

impl Estado {
    /// Las anclas de otra composición de lo mismo, por ejemplo al cambiar el ancho, conservan lo
    /// seleccionado: los bytes de la fuente no se mueven.
    fn adoptar(&mut self, anclas: Vec<Ancla>, fuentes: Vec<String>) {
        let misma = fuentes == self.fuentes;
        let elegidas = self.anclas.get(self.desde..self.hasta).unwrap_or(&[]);
        let (primera, ultima) = (elegidas.first().copied(), elegidas.last().copied());
        (self.desde, self.hasta) = match (misma, primera, ultima) {
            (true, Some(a), Some(b)) => {
                let igual = |x: &Ancla, y: &Ancla| (x.documento, x.desde, x.hasta) == (y.documento, y.desde, y.hasta);
                let desde = anclas.iter().position(|x| igual(x, &a));
                let hasta = anclas.iter().rposition(|x| igual(x, &b));
                desde.zip(hasta).map_or((0, 0), |(d, h)| (d, h + 1))
            }
            _ => (0, 0),
        };
        self.anclas = anclas;
        self.fuentes = fuentes;
    }

    fn poner(&mut self, a: usize, b: usize) -> Vec<Caja> {
        (self.desde, self.hasta) = (a.min(b), a.max(b));
        cajas(&self.anclas, self.desde, self.hasta)
    }

    fn fuentes(&self) -> Vec<&str> {
        self.fuentes.iter().map(String::as_str).collect()
    }
}

/// De los bp de la vista a los del documento: se pinta con un margen alrededor.
fn en_documento(x: f64, y: f64) -> Punto {
    Punto {
        x: x - MARGEN,
        y: y - MARGEN,
    }
}

pub struct Seleccion {
    taller: Arc<Taller>,
    estado: Mutex<Estado>,
}

impl Seleccion {
    pub fn nueva(taller: Arc<Taller>) -> Seleccion {
        Seleccion {
            taller,
            estado: Mutex::default(),
        }
    }

    fn cambiar(&self, cambio: impl FnOnce(&mut Estado) -> Vec<Caja>) {
        let cajas = cambio(&mut self.estado.lock().unwrap_or_else(PoisonError::into_inner));
        self.taller.seleccionar(cajas);
    }

    pub fn arrastrar(&self, a: (f64, f64), b: (f64, f64)) {
        let (a, b) = (en_documento(a.0, a.1), en_documento(b.0, b.1));
        self.cambiar(|e| {
            let (a, b) = (cursor_en(&e.anclas, a), cursor_en(&e.anclas, b));
            e.poner(a, b)
        });
    }

    pub fn palabra(&self, x: f64, y: f64) {
        let p = en_documento(x, y);
        self.cambiar(|e| {
            // el cursor cae entre dos anclas: la de la palabra es la que tiene debajo el clic
            let n = cursor_en(&e.anclas, p).min(e.anclas.len().saturating_sub(1));
            let caja = |n: usize| e.anclas.get(n).map(|a| a.caja);
            let dentro = |c: Caja| p.x >= c.x && p.x <= c.x + c.ancho;
            let n = if caja(n).is_some_and(dentro) {
                n
            } else {
                n.saturating_sub(1)
            };
            let (a, b) = palabra_en(&e.anclas, &e.fuentes(), n);
            e.poner(a, b)
        });
    }

    pub fn todo(&self) {
        self.cambiar(|e| {
            let n = e.anclas.len();
            e.poner(0, n)
        });
    }

    pub fn borrar(&self) {
        self.cambiar(|e| e.poner(0, 0));
    }

    /// El LaTeX en bruto de lo seleccionado.
    pub fn copiar(&self) -> String {
        let e = self.estado.lock().unwrap_or_else(PoisonError::into_inner);
        fuente_de(&e.fuentes(), &e.anclas, e.desde, e.hasta)
    }
}

impl Destino for Seleccion {
    fn publicar(&self, mut escena: Escena, animador: Arc<dyn Animador>, fuentes: Vec<String>) {
        let anclas = std::mem::take(&mut escena.anclas);
        self.taller.publicar(escena, Some(animador));
        self.cambiar(|e| {
            e.adoptar(anclas, fuentes);
            cajas(&e.anclas, e.desde, e.hasta)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ancla(n: u32) -> Ancla {
        Ancla {
            desde: n,
            hasta: n + 1,
            ..Ancla::default()
        }
    }

    #[test]
    fn otra_composicion_de_lo_mismo_conserva_lo_seleccionado() {
        let mut e = Estado::default();
        let fuentes = vec!["abcd".to_string()];
        e.adoptar((0..4).map(ancla).collect(), fuentes.clone());
        e.poner(1, 3);
        let reflujo = vec![ancla(0), ancla(1), ancla(2), ancla(3)];
        e.adoptar(reflujo, fuentes);
        assert_eq!((e.desde, e.hasta), (1, 3));
    }

    #[test]
    fn otro_documento_la_borra() {
        let mut e = Estado::default();
        e.adoptar((0..4).map(ancla).collect(), vec!["abcd".to_string()]);
        e.poner(1, 3);
        e.adoptar((0..4).map(ancla).collect(), vec!["otro".to_string()]);
        assert_eq!((e.desde, e.hasta), (0, 0));
    }

    #[test]
    fn un_arrastre_hacia_atras_selecciona_lo_mismo() {
        let mut e = Estado::default();
        e.adoptar((0..4).map(ancla).collect(), vec!["abcd".to_string()]);
        e.poner(3, 1);
        assert_eq!((e.desde, e.hasta), (1, 3));
    }
}
