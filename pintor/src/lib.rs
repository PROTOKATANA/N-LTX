//! El lado Rust del pintor GPU. El pintor está en C++ sobre Diligent (`linux/`) y habla con Rust
//! por `cxx`, en los dos sentidos si hace falta.
//!
//! Cada pieza tiene una responsabilidad: `compilador` recorre la escena y deja el trabajo en
//! `teselado`, `glifos` y `tintas`, que escriben en `lista`; `lienzo` decide qué se compila en cada
//! fotograma y `pizarra` conecta a quien publica con quien pinta.

pub mod atlas;
pub mod carencias;
pub mod compilador;
pub mod elementos;
pub mod geometria;
pub mod glifos;
pub mod lienzo;
pub mod lista;
pub mod pizarra;
pub mod puente_lienzo;
pub mod puente_masiva;
pub mod rampas;
pub mod teselado;
pub mod textura;
pub mod tintas;

pub use lienzo::{Animador, MARGEN, ancho_de_texto};
pub use pizarra::publicar;
