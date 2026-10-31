//! Contornos de glifo: `mathra` y `tikra` los dan como el `d` de un `<path>` de SVG.

use primitivas::{Punto, Segmento};

/// Interpreta las órdenes `M`, `L`, `Q`, `C` y `Z`, separadas como las escribe `mathra`.
pub(crate) fn segmentos(d: &str) -> Vec<Segmento> {
    let mut lector = Lector { resto: d };
    let mut salida = Vec::new();
    let mut actual = Punto::default();
    while let Some(orden) = lector.orden() {
        match orden {
            'M' => {
                actual = lector.punto();
                salida.push(Segmento::Mover(actual));
            }
            'L' => {
                actual = lector.punto();
                salida.push(Segmento::Linea(actual));
            }
            'Q' => {
                let (control, fin) = (lector.punto(), lector.punto());
                salida.push(cubica(actual, control, fin));
                actual = fin;
            }
            'C' => {
                let (a, b, fin) = (lector.punto(), lector.punto(), lector.punto());
                salida.push(Segmento::Curva(a, b, fin));
                actual = fin;
            }
            'Z' => salida.push(Segmento::Cerrar),
            otra => unreachable!("orden de contorno desconocida: {otra}"),
        }
    }
    salida
}

/// La cúbica idéntica a una Bézier cuadrática.
fn cubica(inicio: Punto, control: Punto, fin: Punto) -> Segmento {
    let hacia = |p: Punto| Punto {
        x: p.x + (control.x - p.x) * 2.0 / 3.0,
        y: p.y + (control.y - p.y) * 2.0 / 3.0,
    };
    Segmento::Curva(hacia(inicio), hacia(fin), fin)
}

struct Lector<'a> {
    resto: &'a str,
}

impl Lector<'_> {
    fn orden(&mut self) -> Option<char> {
        self.resto = self.resto.trim_start();
        let c = self.resto.chars().next()?;
        self.resto = &self.resto[c.len_utf8()..];
        Some(c)
    }

    fn numero(&mut self) -> f64 {
        self.resto = self.resto.trim_start();
        let fin = self
            .resto
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .unwrap_or(self.resto.len());
        let (numero, resto) = self.resto.split_at(fin);
        self.resto = resto;
        // lo escribe el driver de tikra: un número mal formado no llega, y si llegara no debe tumbar nada
        numero.parse().unwrap_or_default()
    }

    fn punto(&mut self) -> Punto {
        Punto {
            x: self.numero(),
            y: self.numero(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> Punto {
        Punto { x, y }
    }

    #[test]
    fn lee_cada_orden() {
        assert_eq!(
            segmentos("M1 2 L3.5 -4 C1 1 2 2 3 3 Z "),
            vec![
                Segmento::Mover(p(1.0, 2.0)),
                Segmento::Linea(p(3.5, -4.0)),
                Segmento::Curva(p(1.0, 1.0), p(2.0, 2.0), p(3.0, 3.0)),
                Segmento::Cerrar,
            ]
        );
    }

    #[test]
    fn la_cuadratica_se_vuelve_cubica_equivalente() {
        // control a 2/3 de cada extremo: es la elevación de grado exacta
        assert_eq!(
            segmentos("M0 0 Q3 3 6 0"),
            vec![
                Segmento::Mover(p(0.0, 0.0)),
                Segmento::Curva(p(2.0, 2.0), p(4.0, 2.0), p(6.0, 0.0)),
            ]
        );
    }

    #[test]
    fn un_glifo_sin_contorno_no_da_segmentos() {
        assert!(segmentos("").is_empty());
    }
}
