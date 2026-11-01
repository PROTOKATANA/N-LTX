//! Traduce `mathra` y `tikra` a la lista neutra de `primitivas`.
//!
//! Es el único crate que conoce sus tipos. El motor (`motor/`) es un fork propio, así que esta
//! traducción desaparecerá cuando emita `primitivas` directamente.
//!
//! ```
//! use adaptador::{FiguraTikra, FormulaMathra};
//! use primitivas::{Color, Compositor};
//!
//! let negro = Color { r: 0.0, g: 0.0, b: 0.0 };
//! let formula = FormulaMathra { bloque: true, tam: 20.0, estricto: false, color: negro };
//! let c = formula.componer(r"\dv{f}{x} = \ce{H2O}").unwrap();
//! assert!(c.avisos.is_empty());
//!
//! let figura = FiguraTikra::default();
//! let c = figura.componer(r"\begin{tikzpicture}\draw (0,0) circle (1);\end{tikzpicture}").unwrap();
//! assert!(!c.escena.primitivas.is_empty());
//! ```

mod caja;
mod contorno;
mod figura;
mod formula;
mod imagenes;
mod incrustados;
mod resaltado;
mod traduccion;

pub use figura::FiguraTikra;
pub use formula::FormulaMathra;
pub use imagenes::{ImagenesEnCarpetas, decodificar};
pub use incrustados::{IncrustadosMotor, filmoteca, piezas_con_imagenes, piezas_del_motor};
