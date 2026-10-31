//! El hub de rinf del ejemplo: cablea las piezas de N-LTX, que no saben nada de rinf.
//! - `senales`: las señales de Dart, traducidas a llamadas.
//! - `compositor`: compone los documentos en su propio hilo.
//! - `avisos`: lo que se cuenta a Dart.
//!
//! El pintor en C++ lee del almacén de `pintor` por `cxx`: esta biblioteca exporta esos símbolos y
//! el ejecutable la enlaza.

mod avisos;
mod compositor;
mod senales;
mod signals;

use std::sync::Arc;

use pintor::{Taller, almacen};
use rinf::{dart_shutdown, write_interface};

use avisos::AvisosRinf;
use compositor::Compositor;

/// La superficie del lienzo: el ejemplo tiene una sola vista.
const SUPERFICIE: u32 = 0;

write_interface!();

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let taller = Arc::new(Taller::abrir(SUPERFICIE, almacen()));
    let compositor = Compositor::abrir(Arc::clone(&taller), AvisosRinf);
    tokio::spawn(senales::escuchar(compositor, taller));
    dart_shutdown().await;
}
