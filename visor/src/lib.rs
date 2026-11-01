//! Lo que hace falta para ver un documento, sin interfaz. Quien integra N-LTX traduce sus señales a
//! estas llamadas: aquí no se sabe nada de Flutter ni de rinf.
//! - `compositor`: compone los documentos en su propio hilo.
//! - `seleccion`: lo seleccionado en el documento publicado.
//! - `muestras`: una carpeta de `.tex` para ver seguidos.

mod compositor;
mod muestras;
mod seleccion;

pub use primitivas::Color;

pub use compositor::{Avisos, Compositor, Destino, Encargo, Resultado};
pub use muestras::Muestras;
pub use seleccion::Seleccion;
