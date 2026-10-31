//! Plantillas de tablas. Una extensión no pinta nada: convierte sus opciones en una tabla del
//! dialecto, de modo que todo lo que hace se podría escribir a mano y el motor de tablas es uno solo.

mod elementos;
mod periodica;

use crate::tema::Opciones;

pub(crate) trait Extension: Sync {
    /// La orden que la llama, sin la barra.
    fn orden(&self) -> &'static str;
    /// La fuente LaTeX que la sustituye.
    fn expandir(&self, opciones: &Opciones) -> String;
}

const EXTENSIONES: [&dyn Extension; 1] = [&periodica::Periodica];

pub(crate) fn buscar(orden: &str) -> Option<&'static dyn Extension> {
    EXTENSIONES.iter().copied().find(|e| e.orden() == orden)
}
