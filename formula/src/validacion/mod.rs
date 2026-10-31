//! Pieza «validador»: ¿es esto una fórmula bien formada?

mod arbol;

pub use arbol::ValidadorPorArbol;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Veredicto {
    /// Se lee entera y todos sus comandos son conocidos.
    Valida,
    /// Se lee, pero usa comandos que el catálogo no conoce.
    Desconocidos(Vec<String>),
    /// No se puede leer (llaves sin cerrar, \left sin \right…).
    Malformada(String),
    /// Tiene pinta de prosa entre dos `$` (importes, código), no de fórmula.
    Prosa,
}

pub trait Validador {
    fn validar(&self, latex: &str) -> Veredicto;
}
