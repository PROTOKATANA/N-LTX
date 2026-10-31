//! El lado Rust del pintor GPU. El pintor está en C++ sobre Diligent (`linux/`) y habla con Rust
//! por `cxx`, en los dos sentidos si hace falta.

pub mod elementos;
pub mod puente;
