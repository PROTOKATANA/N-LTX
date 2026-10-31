//! Las señales entre Dart y Rust. Tras cambiarlas se regeneran los bindings con `rinf gen`.

use rinf::{DartSignal, RustSignal};
use serde::{Deserialize, Serialize};

/// Dart → Rust: el documento que hay que mostrar en una vista de `ancho` bp.
#[derive(Clone, Deserialize, DartSignal)]
pub struct PublicarDocumento {
    pub fuente: String,
    pub ancho: f64,
}

/// Dart → Rust: qué parte del documento se ve, en píxeles físicos.
#[derive(Clone, Copy, Deserialize, DartSignal)]
pub struct VerDocumento {
    pub ancho: u32,
    pub alto: u32,
    pub escala: f64,
    pub y: f64,
}

/// Rust → Dart: lo que hace falta saber para colocar los widgets. `error` vacío si no hubo.
#[derive(Clone, Serialize, RustSignal)]
pub struct DocumentoPublicado {
    pub alto: f64,
    pub avisos: Vec<String>,
    pub error: String,
}

/// Rust → Dart, una vez por segundo.
#[derive(Clone, Copy, Serialize, RustSignal)]
pub struct MedidasPintor {
    pub vertices: u64,
    pub ms_taller: f64,
    pub ms_carril: f64,
    pub elementos: u64,
    pub fps_masiva: f64,
    pub ms_masiva: f64,
}

/// Dart → Rust: la muestra que hay que leer.
#[derive(Clone, Copy, Deserialize, DartSignal)]
pub struct PedirMuestra {
    pub indice: u32,
}

/// Rust → Dart: una muestra de la carpeta, o vacía si no hay ninguna.
#[derive(Clone, Serialize, RustSignal)]
pub struct MuestraLeida {
    pub indice: u32,
    pub total: u32,
    pub nombre: String,
    pub fuente: String,
}
