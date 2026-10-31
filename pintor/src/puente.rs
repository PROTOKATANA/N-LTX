//! La frontera entre Rust y el pintor en C++.

use crate::elementos;

#[cxx::bridge(namespace = "ltx")]
pub mod ffi {
    /// Un elemento animado. El mismo diseño en memoria para Rust, C++ y el vertex shader.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Elemento {
        /// Posición en reposo, de -1 a 1.
        pub x: f32,
        pub y: f32,
        /// Desfase de su movimiento, en radianes.
        pub fase: f32,
        /// En píxeles.
        pub radio: f32,
        /// 0xAABBGGRR, el orden de los bytes en memoria.
        pub rgba: u32,
    }

    extern "Rust" {
        /// Cuántos elementos pinta la prueba masiva.
        fn elementos_pedidos() -> u32;
        /// Escribe los elementos directamente en la memoria que da C++, sin copias.
        fn llenar_elementos(destino: &mut [Elemento]);
    }
}

fn elementos_pedidos() -> u32 {
    elementos::pedidos()
}

fn llenar_elementos(destino: &mut [ffi::Elemento]) {
    elementos::galaxia(destino);
}
