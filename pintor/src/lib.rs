//! El lado Rust del pintor GPU. El pintor está en C++ sobre Diligent (`linux/`) y habla con Rust
//! por `cxx`.
//!
//! Cada pieza tiene una responsabilidad: `compilador` recorre la escena y deja el trabajo en
//! `teselado`, `glifos` y `tintas`, que escriben en `lista`; `lienzo` decide qué se compila en cada
//! fotograma, `taller` lo hace en su hilo y `almacen` guarda lo hecho hasta que C++ lo copia.

pub mod almacen;
pub mod atlas;
pub mod carencias;
pub mod compilador;
pub mod elementos;
pub mod geometria;
pub mod glifos;
pub mod lienzo;
pub mod lista;
pub mod medidas;
pub mod puente_lienzo;
pub mod puente_masiva;
pub mod rampas;
pub mod taller;
pub mod teselado;
pub mod textura;
pub mod tintas;

pub use almacen::almacen;
pub use carencias::carencias;
pub use lienzo::{Animador, MARGEN, ancho_de_texto};
pub use puente_lienzo::ffi::Vista;
pub use taller::Taller;
