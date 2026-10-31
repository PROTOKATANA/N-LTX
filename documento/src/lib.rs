//! Modo texto: LaTeX en flujo continuo, con fórmulas y figuras dentro.
//!
//! `lector` entiende la fuente y `maquetador` la coloca con cosmic-text, igual que el motor de
//! texto del visor de HILO: una fórmula en línea ocupa un carácter de reserva de su ancho exacto,
//! así que el texto la rodea y parte las líneas solo. El resultado es una `Escena` más.
//!
//! Este crate no sabe quién compone las fórmulas y las figuras: se le inyecta (`Incrustados`).

mod contornos;
pub mod lector;
mod maquetador;
mod memoria;

use std::sync::Mutex;

use cosmic_text::{FontSystem, fontdb};
use primitivas::{Caja, Color, Composicion, Compositor, ErrorComposicion};

pub use memoria::Memoria;

const LATIN_MODERN: [&[u8]; 5] = [
    include_bytes!("../fuentes/lmroman10-regular.otf"),
    include_bytes!("../fuentes/lmroman10-bold.otf"),
    include_bytes!("../fuentes/lmroman10-italic.otf"),
    include_bytes!("../fuentes/lmroman10-bolditalic.otf"),
    include_bytes!("../fuentes/lmmono10-regular.otf"),
];

/// Quien compone lo que el texto lleva dentro. Una fórmula deja su línea base en y = 0.
pub trait Incrustados: Send + Sync {
    fn formula(&self, latex: &str, bloque: bool, tam: f64) -> Result<Composicion, ErrorComposicion>;
    fn figura(&self, fuente: &str) -> Result<Composicion, ErrorComposicion>;
    /// El tamaño que necesita una animación durante todo su recorrido.
    fn caja_de_animacion(&self, fuente: &str) -> Result<Caja, ErrorComposicion>;
}

/// Medidas de un documento, en bp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Formato {
    pub ancho: f64,
    pub tam: f64,
    pub color: Color,
}

impl Default for Formato {
    fn default() -> Self {
        Formato {
            ancho: 480.0,
            tam: 11.0,
            color: Color { r: 0.0, g: 0.0, b: 0.0 },
        }
    }
}

pub struct Documento {
    formato: Formato,
    incrustados: Box<dyn Incrustados>,
    fuentes: Mutex<FontSystem>,
}

impl Documento {
    /// Solo con las fuentes empaquetadas: el resultado no depende de la máquina.
    pub fn nuevo(formato: Formato, incrustados: Box<dyn Incrustados>) -> Self {
        Self::con_base(formato, incrustados, base_empaquetada())
    }

    /// Además, las fuentes del sistema: dan cobertura a escrituras que Latin Modern no tiene.
    pub fn con_fuentes_del_sistema(formato: Formato, incrustados: Box<dyn Incrustados>) -> Self {
        let mut base = base_empaquetada();
        base.load_system_fonts();
        Self::con_base(formato, incrustados, base)
    }

    fn con_base(formato: Formato, incrustados: Box<dyn Incrustados>, base: fontdb::Database) -> Self {
        Documento {
            formato,
            incrustados,
            fuentes: Mutex::new(FontSystem::new_with_locale_and_db("es".into(), base)),
        }
    }
}

fn base_empaquetada() -> fontdb::Database {
    let mut base = fontdb::Database::new();
    for fuente in LATIN_MODERN {
        base.load_font_data(fuente.to_vec());
    }
    base
}

/// Un documento dentro de una serie. `titulo`, si lo hay, se escribe encima de él.
#[derive(Debug, Clone, Copy)]
pub struct Apartado<'a> {
    pub titulo: Option<&'a str>,
    pub fuente: &'a str,
}

impl Documento {
    /// Varios documentos completos, uno detrás de otro y separados por una regla. Ninguno ve el
    /// preámbulo ni las macros de los demás, y los avisos de cada uno se escriben debajo de él.
    pub fn componer_serie(&self, apartados: &[Apartado<'_>], ancho: f64) -> Result<Composicion, ErrorComposicion> {
        let entradas = apartados
            .iter()
            .map(|a| maquetador::Entrada {
                titulo: a.titulo,
                lectura: lector::leer(a.fuente),
            })
            .collect();
        let mut sistema = self
            .fuentes
            .lock()
            .map_err(|_| ErrorComposicion("el sistema de fuentes quedó envenenado".into()))?;
        let formato = Formato { ancho, ..self.formato };
        Ok(maquetador::maquetar_serie(
            entradas,
            formato,
            &*self.incrustados,
            &mut sistema,
        ))
    }

    /// Compone a `ancho` bp. Reflujar no exige otro documento: cargar las fuentes es caro.
    pub fn componer_a(&self, fuente: &str, ancho: f64) -> Result<Composicion, ErrorComposicion> {
        let lectura = lector::leer(fuente);
        let mut sistema = self
            .fuentes
            .lock()
            .map_err(|_| ErrorComposicion("el sistema de fuentes quedó envenenado".into()))?;
        let formato = Formato { ancho, ..self.formato };
        Ok(maquetador::maquetar(lectura, formato, &*self.incrustados, &mut sistema))
    }
}

impl Compositor for Documento {
    fn componer(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
        self.componer_a(fuente, self.formato.ancho)
    }
}
