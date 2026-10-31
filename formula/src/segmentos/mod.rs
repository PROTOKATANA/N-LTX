//! Pieza «segmentador»: separa en un texto la prosa de las fórmulas.

mod dolares;

pub use dolares::SegmentadorDolares;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segmento<'a> {
    Prosa(&'a str),
    /// Fórmula sin sus delimitadores; `bloque` si iba entre `$$`.
    Formula {
        latex: &'a str,
        bloque: bool,
    },
}

pub trait Segmentador {
    /// Trozos en orden; unidos con `unir` devuelven el texto exacto.
    fn segmentar<'a>(&self, texto: &'a str) -> Vec<Segmento<'a>>;
}

/// Vuelve a escribir los trozos, con `formula` aplicada al LaTeX de cada
/// fórmula (la identidad devuelve el texto original).
pub fn unir(segmentos: &[Segmento], mut formula: impl FnMut(&str) -> String) -> String {
    let mut s = String::new();
    for seg in segmentos {
        match seg {
            Segmento::Prosa(p) => s.push_str(p),
            Segmento::Formula { latex, bloque } => {
                let d = if *bloque { "$$" } else { "$" };
                s.push_str(d);
                s.push_str(&formula(latex));
                s.push_str(d);
            }
        }
    }
    s
}
