//! La frontera entre Rust y el pintor del lienzo, en C++. Lo que comparten ambos lados se define
//! solo aquí: ni los límites ni los tipos de tinta se copian a mano.

use crate::atlas::LADO_ATLAS;
use crate::lienzo::Marco;
use crate::pizarra::preparar;
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

    /// Qué se ve del documento.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Vista {
        /// Alto de la superficie en píxeles físicos.
        pub alto: f64,
        /// Píxeles físicos por punto.
        pub escala: f64,
        /// Desplazamiento en píxeles físicos.
        pub y: f64,
        /// Reloj de las animaciones.
        pub segundos: f64,
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

        /// Lo que hay que pintar en este fotograma. `nuevos`: la GPU aún no tiene nada subido.
        fn preparar(id: u32, vista: Vista, nuevos: bool) -> Box<Marco>;

        /// Cambia cuando cambia la lista base: mientras no cambie, no hay que volver a subirla.
        fn generacion(self: &Marco) -> u64;
        fn vertices_base(self: &Marco) -> &[Vertice];
        fn indices_base(self: &Marco) -> &[u32];
        fn comandos_base(self: &Marco) -> &[Comando];
        /// Lo que se recompone en cada fotograma: las animaciones.
        fn vertices_vivos(self: &Marco) -> &[Vertice];
        fn indices_vivos(self: &Marco) -> &[u32];
        fn comandos_vivos(self: &Marco) -> &[Comando];
        fn tintas(self: &Marco) -> &[Tinta];
        /// Las filas del atlas que cambiaron, desde la fila `atlas_y`.
        fn atlas(self: &Marco) -> &[u8];
        fn atlas_y(self: &Marco) -> u32;
        fn rampas(self: &Marco) -> &[u8];
        fn rampas_y(self: &Marco) -> u32;
        /// Hay algo moviéndose a la vista: hace falta otro fotograma.
        fn animada(self: &Marco) -> bool;
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
