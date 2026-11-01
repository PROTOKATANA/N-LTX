//! El hub de rinf del ejemplo: el pegamento entre Dart y N-LTX, que no sabe nada de rinf.
//! - `senales`: las señales de Dart, traducidas a llamadas.
//! - `avisos`: lo que se cuenta a Dart.
//!
//! `NLTX_MUESTRAS` dice qué carpeta de `.tex` se ve después del documento.
//!
//! El pintor en C++ lee del almacén de `pintor` por `cxx`: esta biblioteca exporta esos símbolos y
//! el ejecutable la enlaza.

/// El intérprete de TikZ pide y suelta memoria por cada elemento. Con `banco_animacion` (5.000
/// elementos), frente al asignador del sistema: entre un 5 % y un 13 % menos por fotograma, a cambio
/// de un 14 % más de memoria pico (621 frente a 543 MiB, en una ejecución). Con documentos la memoria
/// sube más: K1 ocupaba de 593 a 634 MiB, frente a 199-206 del sistema.
#[global_allocator]
static ASIGNADOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod avisos;
mod senales;
mod signals;

use std::path::PathBuf;
use std::sync::Arc;

use pintor::{Taller, almacen};
use rinf::{dart_shutdown, write_interface};
use visor::{Compositor, Muestras, Seleccion};

use avisos::AvisosRinf;

/// La superficie del lienzo: el ejemplo tiene una sola vista.
const SUPERFICIE: u32 = 0;

write_interface!();

/// Los recursos de la aplicación, junto al ejecutable: de ahí salen las imágenes de los documentos.
fn recursos() -> Option<PathBuf> {
    let ejecutable = std::env::current_exe().ok()?;
    Some(ejecutable.parent()?.join("data/flutter_assets/assets"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let taller = Arc::new(Taller::abrir(SUPERFICIE, almacen()));
    let carpeta = std::env::var_os("NLTX_MUESTRAS").map(PathBuf::from);
    let muestras = carpeta.as_deref().map_or_else(Muestras::default, Muestras::de_carpeta);
    let seleccion = Arc::new(Seleccion::nueva(Arc::clone(&taller)));
    let carpetas = recursos().into_iter().chain(carpeta).collect();
    let compositor = Compositor::abrir(Arc::clone(&seleccion), AvisosRinf, muestras, carpetas);
    tokio::spawn(senales::escuchar(compositor, taller, seleccion));
    dart_shutdown().await;
}
