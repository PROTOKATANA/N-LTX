//! Pieza «canonizador»: LaTeX → la misma fórmula escrita de una sola manera.
//!
//! Trabaja sobre unidades léxicas (`Token`) y aplica una lista de reglas
//! (`Regla`) inyectadas: cada normalización es una pieza aparte que se añade,
//! quita o sustituye sin tocar las demás.

mod latex;
mod lexico;
mod reglas;

pub use latex::CanonizadorLatex;
pub use reglas::{Espacios, Sinonimos};

pub trait Canonizador {
    fn canonizar(&self, latex: &str) -> String;
}

/// Unidad léxica de LaTeX.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Token {
    /// Palabra de control (`\frac` → «frac»).
    Palabra(String),
    /// Símbolo de control (`\,`, `\{`, `\\`, `\ ` → el carácter).
    Simbolo(char),
    /// Cualquier otro carácter.
    Caracter(char),
    /// Uno o más blancos seguidos, tal cual.
    Blanco(String),
    /// Argumento de un comando de texto (`\text`, `\ce`…) con sus llaves:
    /// sus espacios cuentan y ninguna regla lo toca.
    Literal(String),
}

/// Una normalización sobre la secuencia de unidades léxicas.
pub trait Regla {
    fn aplicar(&self, tokens: Vec<Token>) -> Vec<Token>;
}
