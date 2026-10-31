//! Pieza «lector»: texto de una notación matemática → árbol (`modelo::Nodo`).
//! Hoy, LaTeX; un lector de MathML o de Typst encaja implementando el mismo
//! trait sin tocar a quien lo use.

mod latex;

pub use latex::LectorLatex;

use crate::modelo::Nodo;

pub trait Lector {
    /// Lee una fórmula (sin delimitadores `$`). Error si está mal formada.
    fn leer(&self, fuente: &str) -> Result<Vec<Nodo>, String>;
}
