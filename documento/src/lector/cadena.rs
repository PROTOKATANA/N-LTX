//! Un texto ya interpretado que recuerda de qué trozo de la fuente sale: seleccionar en pantalla
//! copia la fuente, no lo que se ve.

use std::ops::Deref;

/// Un trozo del texto y la parte de la fuente que lo produce. Si miden lo mismo se corresponden
/// byte a byte; si no (`\'a` da `á`, `\textbf{hola}` da `hola`), es indivisible.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Pieza {
    inicio: u32,
    fin: u32,
    desde: u32,
    hasta: u32,
}

impl Pieza {
    fn literal(&self) -> bool {
        self.fin - self.inicio == self.hasta - self.desde
    }
}

/// Lo que no sale de la fuente, como la numeración de una lista, no tiene piezas. Dos cadenas son
/// iguales si dicen lo mismo, salgan de donde salgan.
#[derive(Debug, Clone, Default)]
pub struct Cadena {
    texto: String,
    piezas: Vec<Pieza>,
}

impl Cadena {
    pub(super) fn push_str(&mut self, texto: &str) {
        self.texto.push_str(texto);
    }

    /// Lo escrito desde el byte `desde` sale de `rango`, diga lo que dijera antes.
    pub(super) fn anotar(&mut self, desde: usize, (a, b): (u32, u32)) {
        let (inicio, fin) = (desde as u32, self.texto.len() as u32);
        if fin <= inicio {
            return;
        }
        while self.piezas.last().is_some_and(|p| p.inicio >= inicio) {
            self.piezas.pop();
        }
        let nueva = Pieza {
            inicio,
            fin,
            desde: a,
            hasta: b,
        };
        match self.piezas.last_mut() {
            // texto corrido: una pieza por párrafo y no una por letra
            Some(p) if p.literal() && nueva.literal() && p.fin == inicio && p.hasta == a => {
                (p.fin, p.hasta) = (fin, b);
            }
            _ => self.piezas.push(nueva),
        }
    }

    /// Todo sale de `rango`, sin partes: una orden se copia entera aunque se toque un trozo.
    pub(super) fn abarcar(&mut self, rango: (u32, u32)) {
        self.piezas.clear();
        self.anotar(0, rango);
    }

    /// De qué parte de la fuente salen los bytes `a..b`.
    pub(crate) fn fuente_de(&self, a: usize, b: usize) -> Option<(u32, u32)> {
        let (a, b) = (a as u32, b as u32);
        let primera = self.piezas.iter().find(|p| p.fin > a)?;
        let ultima = self.piezas.iter().rev().find(|p| p.inicio < b)?;
        let desde = if primera.literal() {
            primera.desde + a.saturating_sub(primera.inicio)
        } else {
            primera.desde
        };
        let hasta = if ultima.literal() {
            ultima.hasta - ultima.fin.saturating_sub(b)
        } else {
            ultima.hasta
        };
        (desde < hasta).then_some((desde, hasta))
    }

    /// De qué parte de la fuente sale toda ella.
    pub(crate) fn fuente(&self) -> Option<(u32, u32)> {
        Some((self.piezas.first()?.desde, self.piezas.last()?.hasta))
    }

    /// Quita el blanco del principio.
    pub(super) fn sin_blanco_delante(&mut self) {
        let n = (self.texto.len() - self.texto.trim_start().len()) as u32;
        self.texto.drain(..n as usize);
        self.piezas.retain_mut(|p| {
            if p.fin <= n {
                return false;
            }
            if p.inicio < n && p.literal() {
                p.desde += n - p.inicio;
            }
            p.inicio = p.inicio.saturating_sub(n);
            p.fin -= n;
            true
        });
    }

    /// Quita el blanco del final.
    pub(super) fn sin_blanco_detras(&mut self) {
        let n = self.texto.trim_end().len() as u32;
        self.texto.truncate(n as usize);
        self.piezas.retain(|p| p.inicio < n);
        if let Some(p) = self.piezas.last_mut().filter(|p| p.fin > n) {
            if p.literal() {
                p.hasta -= p.fin - n;
            }
            p.fin = n;
        }
    }
}

impl Deref for Cadena {
    type Target = str;

    fn deref(&self) -> &str {
        &self.texto
    }
}

impl From<String> for Cadena {
    fn from(texto: String) -> Cadena {
        Cadena {
            texto,
            piezas: Vec::new(),
        }
    }
}

impl From<&str> for Cadena {
    fn from(texto: &str) -> Cadena {
        texto.to_string().into()
    }
}

impl PartialEq for Cadena {
    fn eq(&self, otra: &Cadena) -> bool {
        self.texto == otra.texto
    }
}

impl PartialEq<str> for Cadena {
    fn eq(&self, otra: &str) -> bool {
        self.texto == otra
    }
}

impl PartialEq<&str> for Cadena {
    fn eq(&self, otra: &&str) -> bool {
        self.texto == *otra
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cadena(texto: &str) -> Cadena {
        texto.into()
    }

    #[test]
    fn el_texto_literal_se_corresponde_byte_a_byte_y_cabe_en_una_pieza() {
        let mut c = Cadena::default();
        for (i, letra) in ["h", "o", "l", "a"].into_iter().enumerate() {
            let antes = c.len();
            c.push_str(letra);
            c.anotar(antes, (10 + i as u32, 11 + i as u32));
        }
        assert_eq!(c.piezas.len(), 1);
        assert_eq!(c.fuente_de(1, 3), Some((11, 13)));
        assert_eq!(c.fuente(), Some((10, 14)));
    }

    #[test]
    fn lo_que_no_mide_lo_mismo_es_indivisible() {
        let mut c = cadena("a");
        c.anotar(0, (0, 1));
        c.push_str("hola");
        c.anotar(1, (1, 14)); // \textbf{hola}
        assert_eq!(c.fuente_de(2, 3), Some((1, 14)));
        assert_eq!(c.fuente_de(0, 3), Some((0, 14)));
    }

    #[test]
    fn lo_de_fuera_gana_a_lo_de_dentro() {
        let mut c = cadena("hola");
        c.anotar(0, (8, 12));
        c.anotar(0, (0, 13));
        assert_eq!(c.fuente(), Some((0, 13)));
        assert_eq!(c.piezas.len(), 1);
    }

    #[test]
    fn quitar_los_blancos_mueve_lo_que_les_sigue() {
        let mut c = cadena("  ab  ");
        c.anotar(0, (100, 106));
        c.sin_blanco_delante();
        c.sin_blanco_detras();
        assert_eq!(&*c, "ab");
        assert_eq!(c.fuente(), Some((102, 104)));
    }

    #[test]
    fn un_texto_que_no_sale_de_la_fuente_no_tiene_origen() {
        assert_eq!(cadena("1.").fuente_de(0, 2), None);
    }

    #[test]
    fn dos_cadenas_son_iguales_si_dicen_lo_mismo() {
        let mut a = cadena("x");
        a.anotar(0, (5, 6));
        assert_eq!(a, cadena("x"));
    }
}
