//! Pieza «química»: traduce una notación química (el `\\ce{…}` de mhchem) a
//! LaTeX matemático que el lector sabe leer. Inyectada en el lector.

mod mhchem;

pub use mhchem::Mhchem;

pub trait Quimica {
    /// Argumento de `\\ce{…}`/`\\pu{…}`, tal cual → LaTeX matemático.
    fn a_latex(&self, notacion: &str) -> String;
}
