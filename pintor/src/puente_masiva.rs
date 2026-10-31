//! La frontera entre Rust y la prueba masiva, en C++.

use crate::elementos;
use crate::medidas::anotar_masiva;

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

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct MedidasMasiva {
        pub elementos: u64,
        pub fotogramas_por_segundo: f64,
        /// Lo que gasta el carril en un fotograma, sin esperar a la GPU.
        pub ms_carril: f64,
    }

    extern "Rust" {
        /// Cuántos elementos pinta la prueba masiva.
        fn elementos_pedidos() -> u32;
        /// Escribe los elementos directamente en la memoria que da C++, sin copias.
        fn llenar_elementos(destino: &mut [Elemento]);
        /// Hilo del carril de la prueba, una vez por segundo.
        fn anotar_masiva(medidas: MedidasMasiva);
    }
}

fn elementos_pedidos() -> u32 {
    elementos::pedidos()
}

fn llenar_elementos(destino: &mut [ffi::Elemento]) {
    elementos::galaxia(destino);
}
