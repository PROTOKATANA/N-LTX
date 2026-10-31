//! Fórmulas matemáticas como piezas intercambiables.
//!
//! - `modelo`: el árbol de una fórmula, contrato común de todas las piezas.
//! - `catalogo` (trait `Catalogo`): significado de comandos y caracteres.
//! - `quimica` (trait `Quimica`): notación química → LaTeX.
//! - `lector` (trait `Lector`): notación → árbol.
//! - `canonico` (traits `Canonizador` y `Regla`): LaTeX → LaTeX canónico.
//! - `validacion` (trait `Validador`): ¿fórmula bien formada?
//! - `segmentos` (trait `Segmentador`): texto → prosa y fórmulas.
//!
//! Cada pieza recibe sus dependencias inyectadas; las implementaciones
//! concretas solo se componen en quien usa el crate.

pub mod canonico;
pub mod catalogo;
pub mod lector;
pub mod modelo;
pub mod quimica;
pub mod segmentos;
pub mod validacion;
