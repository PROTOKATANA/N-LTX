//! Las señales entre Dart y Rust. Tras cambiarlas se regeneran los bindings con `rinf gen`.

use rinf::{DartSignal, RustSignal};
use serde::{Deserialize, Serialize};

/// Dart → Rust: el documento que hay que mostrar en una vista de `ancho` bp, seguido o no de todas
/// las muestras. Las muestras las tiene Rust: no cruzan Dart.
#[derive(Clone, Deserialize, DartSignal)]
pub struct PublicarDocumento {
    pub fuente: String,
    pub ancho: f64,
    pub con_muestras: bool,
    /// El color de lo que el documento no colorea, `0xRRGGBB`.
    pub tinta: u32,
}

/// Dart → Rust: sobre qué color se pinta, `0xRRGGBB`. No recompone.
#[derive(Clone, Copy, Deserialize, DartSignal)]
pub struct PonerFondo {
    pub color: u32,
}

/// Dart → Rust: qué parte del documento se ve, en píxeles físicos.
#[derive(Clone, Copy, Deserialize, DartSignal)]
pub struct VerDocumento {
    pub ancho: u32,
    pub alto: u32,
    pub escala: f64,
    pub y: f64,
}

/// Dart → Rust: se ha arrastrado de (`ax`, `ay`) a (`bx`, `by`), en bp desde la esquina del documento.
#[derive(Clone, Copy, Deserialize, DartSignal)]
pub struct SeleccionarTexto {
    pub ax: f64,
    pub ay: f64,
    pub bx: f64,
    pub by: f64,
}

/// Dart → Rust: doble clic en (`x`, `y`).
#[derive(Clone, Copy, Deserialize, DartSignal)]
pub struct SeleccionarPalabra {
    pub x: f64,
    pub y: f64,
}

/// Dart → Rust: seleccionar todo el documento.
#[derive(Clone, Copy, Deserialize, DartSignal)]
pub struct SeleccionarTodo;

/// Dart → Rust: quitar la selección.
#[derive(Clone, Copy, Deserialize, DartSignal)]
pub struct BorrarSeleccion;

/// Dart → Rust: copiar lo seleccionado.
#[derive(Clone, Copy, Deserialize, DartSignal)]
pub struct CopiarSeleccion;

/// Rust → Dart: el LaTeX en bruto de lo seleccionado, para el portapapeles.
#[derive(Clone, Serialize, RustSignal)]
pub struct TextoCopiado {
    pub texto: String,
}

/// Rust → Dart: lo que hace falta saber para colocar los widgets. `error` vacío si no hubo.
#[derive(Clone, Serialize, RustSignal)]
pub struct DocumentoPublicado {
    pub alto: f64,
    pub avisos: Vec<String>,
    pub error: String,
    pub muestras: u32,
    /// `false` en un tramo: el documento sigue componiéndose y `alto` crecerá.
    pub completo: bool,
}

/// Rust → Dart, una vez por segundo.
#[derive(Clone, Copy, Serialize, RustSignal)]
pub struct MedidasPintor {
    pub vertices: u64,
    pub ms_taller: f64,
    pub ms_carril: f64,
}
