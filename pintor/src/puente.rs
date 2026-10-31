//! La frontera entre Rust y el pintor en C++.

use crate::elementos;
use crate::lienzo::{Marco, preparar};

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

    /// Cómo se colorea lo que no es de un solo color. La lee el pixel shader.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Tinta {
        /// 0 sólido, 1 glifo, 2 degradado lineal, 3 degradado radial.
        pub tipo: u32,
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

    extern "Rust" {
        type Marco;

        /// Cuántos elementos pinta la prueba masiva.
        fn elementos_pedidos() -> u32;
        /// Escribe los elementos directamente en la memoria que da C++, sin copias.
        fn llenar_elementos(destino: &mut [Elemento]);

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

fn elementos_pedidos() -> u32 {
    elementos::pedidos()
}

fn llenar_elementos(destino: &mut [ffi::Elemento]) {
    elementos::galaxia(destino);
}
