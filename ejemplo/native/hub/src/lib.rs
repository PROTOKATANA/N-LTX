//! El hub de rinf del ejemplo: cablea las piezas de N-LTX, que no saben nada de rinf.
//! - `senales`: las señales de Dart, traducidas a llamadas.
//! - `compositor`: compone los documentos en su propio hilo.
//! - `muestras`: una carpeta de `.tex` para ver seguidos, si `NLTX_MUESTRAS` dice cuál.
//! - `seleccion`: lo seleccionado en el documento publicado.
//! - `avisos`: lo que se cuenta a Dart.
//!
//! El pintor en C++ lee del almacén de `pintor` por `cxx`: esta biblioteca exporta esos símbolos y
//! el ejecutable la enlaza.

/// El intérprete de TikZ pide y suelta memoria por cada elemento. Con `banco_animacion` (5.000
/// elementos), frente al asignador del sistema: entre un 5 % y un 13 % menos por fotograma, a cambio
/// de un 14 % más de memoria pico (621 frente a 543 MiB, en una ejecución).
#[global_allocator]
static ASIGNADOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod avisos;
mod compositor;
mod muestras;
mod seleccion;
mod senales;
mod signals;

use std::path::Path;
use std::sync::Arc;

use pintor::{Taller, almacen};
use rinf::{dart_shutdown, write_interface};

use avisos::AvisosRinf;
use compositor::Compositor;
use muestras::Muestras;
use seleccion::Seleccion;

/// La superficie del lienzo: el ejemplo tiene una sola vista.
const SUPERFICIE: u32 = 0;

write_interface!();

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let taller = Arc::new(Taller::abrir(SUPERFICIE, almacen()));
    let carpeta = std::env::var_os("NLTX_MUESTRAS");
    let muestras = carpeta.map_or_else(Muestras::default, |c| Muestras::de_carpeta(Path::new(&c)));
    let seleccion = Arc::new(Seleccion::nueva(Arc::clone(&taller)));
    let compositor = Compositor::abrir(Arc::clone(&seleccion), AvisosRinf, muestras);
    tokio::spawn(senales::escuchar(compositor, taller, seleccion));
    dart_shutdown().await;
}
