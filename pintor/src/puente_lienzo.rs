//! La frontera entre Rust y el pintor del lienzo, en C++. Lo que comparten ambos lados se define
//! solo aquí: ni los límites ni los tipos de tinta se copian a mano.

use std::time::Duration;

use crate::almacen::{Marco, almacen};
use crate::atlas::LADO_ATLAS;
use crate::medidas::anotar_lienzo;
use crate::rampas::{ANCHO_RAMPA, FILAS_RAMPAS};
use crate::tintas::MAX_TINTAS;

#[cxx::bridge(namespace = "ltx")]
pub mod ffi {
    /// Un vértice de una lista, en píxeles físicos del documento.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Vertice {
        pub x: f32,
        pub y: f32,
        /// Dónde cae en el atlas, si es un glifo.
        pub u: f32,
        pub v: f32,
        /// 0xAABBGGRR sin premultiplicar.
        pub rgba: u32,
        /// Índice en las tintas del fotograma.
        pub tinta: u32,
    }

    #[derive(Debug)]
    #[repr(u32)]
    pub enum TipoTinta {
        Solido,
        Glifo,
        Lineal,
        Radial,
        Trazo,
    }

    /// Cómo se colorea lo que no es de un solo color. La lee el pixel shader.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Tinta {
        pub tipo: TipoTinta,
        /// Dónde cae su rampa de color, de 0 a 1.
        pub fila: f32,
        pub radio: f32,
        pub reservado: f32,
        /// De píxeles del documento al espacio del degradado.
        pub a: f32,
        pub b: f32,
        pub c: f32,
        pub d: f32,
        pub e: f32,
        pub f: f32,
        /// Lineal: el inicio y el eje entre su largo al cuadrado. Radial: el foco.
        pub p: f32,
        pub q: f32,
        pub r: f32,
        pub s: f32,
    }

    #[derive(Debug)]
    #[repr(u32)]
    pub enum Orden {
        /// Pinta los índices `inicio..inicio + cuenta`.
        Dibujar,
        /// Pinta y marca lo pintado en el stencil: cada muestra se pinta una sola vez.
        Unica,
        /// Sube el stencil sin pintar: abre un recorte.
        Recortar,
        /// Baja el stencil sin pintar: cierra un recorte o borra las marcas de `Unica`.
        Soltar,
        /// Empieza a pintar en una capa nueva, transparente.
        AbrirCapa,
        /// Termina una capa que sirve de máscara para la que la contiene.
        CerrarMascara,
        /// Termina una capa y la compone sobre la anterior.
        CerrarCapa,
    }

    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Comando {
        pub orden: Orden,
        pub inicio: u32,
        pub cuenta: u32,
        /// Lo que debe valer el stencil para que el comando pinte.
        pub nivel: u32,
        /// Al cerrar una capa: su opacidad y su modo de mezcla (0 es normal).
        pub opacidad: f32,
        pub mezcla: u32,
        pub con_mascara: bool,
        /// Al abrir una capa: su `CerrarCapa` y el alto que ocupa, para saltarla si no se ve.
        pub salto: u32,
        pub y0: f32,
        pub y1: f32,
    }

    /// Qué se ve del documento, en píxeles físicos.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Vista {
        pub ancho: u32,
        pub alto: u32,
        /// Píxeles físicos por punto.
        pub escala: f64,
        /// Desplazamiento vertical.
        pub y: f64,
    }

    /// Lo que mide lo que viaja a la GPU.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Limites {
        pub lado_atlas: u32,
        pub ancho_rampa: u32,
        pub filas_rampas: u32,
        pub max_tintas: u32,
    }

    extern "Rust" {
        type Marco;

        fn limites() -> Limites;

        /// Hilo del carril del pintor. Lo último hecho, que no se calcula aquí: solo se copia. Con
        /// `nuevos`, la GPU aún no tiene nada subido.
        fn tomar(id: u32, nuevos: bool) -> Box<Marco>;
        /// Hilo vigía del pintor. Espera a que haya más entregas que `vistas`, como mucho `ms`.
        fn esperar_entrega(id: u32, vistas: u64, ms: u32) -> u64;
        /// Hace volver a `esperar_entrega` para poder cerrar sin esperar al tope.
        fn despertar();
        /// Lo que gastó el carril en un fotograma.
        fn anotar_lienzo(ms: f64);

        fn vista(self: &Marco) -> Vista;
        /// Mientras no cambien, lo que ya se copió sigue valiendo.
        fn generacion_base(self: &Marco) -> u64;
        fn generacion_vivos(self: &Marco) -> u64;
        fn vertices_base(self: &Marco) -> &[Vertice];
        fn indices_base(self: &Marco) -> &[u32];
        fn comandos_base(self: &Marco) -> &[Comando];
        /// Lo que se recompone en cada fotograma: las animaciones.
        fn vertices_vivos(self: &Marco) -> &[Vertice];
        fn indices_vivos(self: &Marco) -> &[u32];
        fn comandos_vivos(self: &Marco) -> &[Comando];
        fn tintas(self: &Marco) -> &[Tinta];
        /// Las filas de textura que cambiaron: cada parche empieza en su `fila_*`.
        fn parches_atlas(self: &Marco) -> usize;
        fn parche_atlas(self: &Marco, i: usize) -> &[u8];
        fn fila_atlas(self: &Marco, i: usize) -> u32;
        fn parches_rampas(self: &Marco) -> usize;
        fn parche_rampas(self: &Marco, i: usize) -> &[u8];
        fn fila_rampas(self: &Marco, i: usize) -> u32;
    }
}

fn limites() -> ffi::Limites {
    ffi::Limites {
        lado_atlas: LADO_ATLAS,
        ancho_rampa: ANCHO_RAMPA,
        filas_rampas: FILAS_RAMPAS,
        max_tintas: MAX_TINTAS as u32,
    }
}

fn tomar(id: u32, nuevos: bool) -> Box<Marco> {
    Box::new(almacen().tomar(id, nuevos))
}

fn esperar_entrega(id: u32, vistas: u64, ms: u32) -> u64 {
    almacen().esperar(id, vistas, Duration::from_millis(u64::from(ms)))
}

fn despertar() {
    almacen().despertar();
}
