//! Modo texto: LaTeX en flujo continuo, con fórmulas y figuras dentro.
//!
//! `lector` entiende la fuente y `maquetador` la coloca con cosmic-text, igual que el motor de
//! texto del visor de HILO: una fórmula en línea ocupa un carácter de reserva de su ancho exacto,
//! así que el texto la rodea y parte las líneas solo. El resultado es una `Escena` más.
//!
//! Este crate no sabe quién compone las fórmulas y las figuras: se le inyecta (`Piezas`).

mod contornos;
mod extensiones;
pub mod lector;
mod maquetador;
mod memoria;
pub mod seleccion;
mod tema;

use std::sync::{Arc, Mutex};

use cosmic_text::{FontSystem, fontdb};
use primitivas::{Caja, Color, Composicion, Compositor, ErrorComposicion, MapaDeBits};

pub use memoria::Memoria;
pub use tema::{Ajuste, EstiloCodigo, EstiloTabla, Opciones, Reparto, Tema};

const FUENTES: [&[u8]; 8] = [
    include_bytes!("../fuentes/lmroman10-regular.otf"),
    include_bytes!("../fuentes/lmroman10-bold.otf"),
    include_bytes!("../fuentes/lmroman10-italic.otf"),
    include_bytes!("../fuentes/lmroman10-bolditalic.otf"),
    include_bytes!("../fuentes/JetBrainsMono-Regular.ttf"),
    include_bytes!("../fuentes/JetBrainsMono-Bold.ttf"),
    include_bytes!("../fuentes/JetBrainsMono-Italic.ttf"),
    include_bytes!("../fuentes/JetBrainsMono-BoldItalic.ttf"),
];

/// Un color con nombre propio, de `\definecolor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorDefinido {
    pub nombre: String,
    pub modelo: String,
    pub valor: String,
}

/// Los colores del panel de un editor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColoresEditor {
    pub fondo: Color,
    pub texto: Color,
    pub numeros: Color,
}

/// Un trozo de código con el aspecto que le da la gramática de su lenguaje.
#[derive(Debug, Clone, PartialEq)]
pub struct Fragmento {
    pub texto: String,
    pub color: Color,
    pub negrita: bool,
    pub cursiva: bool,
}

/// Quien compone lo que el texto lleva dentro. Una fórmula deja su línea base en y = 0.
pub trait Incrustados: Send + Sync {
    fn formula(&self, latex: &str, bloque: bool, tam: f64) -> Result<Composicion, ErrorComposicion>;
    fn figura(&self, fuente: &str) -> Result<Composicion, ErrorComposicion>;
    /// El tamaño que necesita una animación durante todo su recorrido.
    fn caja_de_animacion(&self, fuente: &str) -> Result<Caja, ErrorComposicion>;
    /// Si esta figura o animación ya está hecha y pedirla será inmediato. Sin saberlo, no.
    fn recuerda(&self, _fuente: &str) -> bool {
        false
    }
    /// Las animaciones que se van a pedir, por orden: quien pueda, que las adelante en otros hilos.
    fn anticipar(&self, _animaciones: &[String]) {}
}

/// Quien va mirando un documento mientras se compone. Se le pregunta entre dos bloques del documento:
/// ahí no queda nada abierto y lo colocado es un principio exacto del resultado final.
pub trait Seguimiento {
    /// Si hay que dejarlo: lo que está saliendo ya no hace falta.
    fn parar(&mut self) -> bool;
    /// Si hay que enseñar ya lo colocado, que llega hasta `alto` bp. `lento`: lo que viene puede
    /// tardar. Copiarlo cuesta, así que no se copia sin que lo pida.
    fn quiere_tramo(&mut self, alto: f64, lento: bool) -> bool;
    /// Lo colocado hasta aquí.
    fn tramo(&mut self, parcial: Composicion);
}

/// Sin nadie mirando: el documento sale entero de una vez.
pub struct SinSeguimiento;

impl Seguimiento for SinSeguimiento {
    fn parar(&mut self) -> bool {
        false
    }

    fn quiere_tramo(&mut self, _alto: f64, _lento: bool) -> bool {
        false
    }

    fn tramo(&mut self, _parcial: Composicion) {}
}

/// Quien sabe qué color es un nombre.
pub trait Colores: Send + Sync {
    /// Qué color es `nombre`, una expresión como `blue!30`, que puede usar los `definidos`.
    fn color(&self, nombre: &str, definidos: &[ColorDefinido]) -> Option<Color>;
}

/// Quien colorea el código.
pub trait Resaltador: Send + Sync {
    /// El `codigo` de un `lenguaje` con los colores de `tema`, por trozos. Vacío si no se sabe.
    fn resaltar(&self, codigo: &str, lenguaje: &str, tema: &str) -> Vec<Fragmento>;
    /// Los colores del panel de un editor con `tema`. `None` si no se conoce.
    fn editor(&self, tema: &str) -> Option<ColoresEditor>;
}

/// Una imagen lista para colocar, con su tamaño natural en bp.
#[derive(Debug, Clone, PartialEq)]
pub struct Imagen {
    pub mapa: Arc<MapaDeBits>,
    pub ancho: f64,
    pub alto: f64,
}

/// Quien lee las imágenes de un documento.
pub trait Imagenes: Send + Sync {
    /// La imagen de `ruta`, tal como se escribe en `\\includegraphics`, o por qué no se puede.
    fn imagen(&self, ruta: &str) -> Result<Imagen, String>;
}

/// Lo que el documento no sabe hacer solo.
pub struct Piezas {
    pub incrustados: Box<dyn Incrustados>,
    pub colores: Box<dyn Colores>,
    pub resaltador: Box<dyn Resaltador>,
    pub imagenes: Box<dyn Imagenes>,
}

impl Piezas {
    /// Solo quien compone: sin más colores que los básicos y con el código en un solo color.
    pub fn de(incrustados: impl Incrustados + 'static) -> Piezas {
        Piezas {
            incrustados: Box::new(incrustados),
            colores: Box::new(SinColores),
            resaltador: Box::new(SinResaltado),
            imagenes: Box::new(SinImagenes),
        }
    }
}

struct SinImagenes;

impl Imagenes for SinImagenes {
    fn imagen(&self, ruta: &str) -> Result<Imagen, String> {
        Err(format!("no se encuentra la imagen {ruta}"))
    }
}

struct SinColores;

impl Colores for SinColores {
    fn color(&self, _nombre: &str, _definidos: &[ColorDefinido]) -> Option<Color> {
        None
    }
}

struct SinResaltado;

impl Resaltador for SinResaltado {
    fn resaltar(&self, _codigo: &str, _lenguaje: &str, _tema: &str) -> Vec<Fragmento> {
        Vec::new()
    }

    fn editor(&self, _tema: &str) -> Option<ColoresEditor> {
        None
    }
}

/// Medidas de un documento, en bp.
#[derive(Debug, Clone, PartialEq)]
pub struct Formato {
    pub ancho: f64,
    pub tam: f64,
    pub color: Color,
    pub tema: Tema,
}

impl Default for Formato {
    fn default() -> Self {
        Formato {
            ancho: 480.0,
            tam: 11.0,
            color: Color { r: 0.0, g: 0.0, b: 0.0 },
            tema: Tema::default(),
        }
    }
}

pub struct Documento {
    formato: Formato,
    piezas: Piezas,
    fuentes: Mutex<FontSystem>,
}

impl Documento {
    /// Solo con las fuentes empaquetadas: el resultado no depende de la máquina.
    pub fn nuevo(formato: Formato, piezas: Piezas) -> Self {
        Self::con_base(formato, piezas, base_empaquetada())
    }

    /// Además, las fuentes del sistema: dan cobertura a escrituras que Latin Modern no tiene.
    pub fn con_fuentes_del_sistema(formato: Formato, piezas: Piezas) -> Self {
        let mut base = base_empaquetada();
        base.load_system_fonts();
        Self::con_base(formato, piezas, base)
    }

    fn con_base(formato: Formato, piezas: Piezas, base: fontdb::Database) -> Self {
        Documento {
            formato,
            piezas,
            fuentes: Mutex::new(FontSystem::new_with_locale_and_db("es".into(), base)),
        }
    }
}

fn base_empaquetada() -> fontdb::Database {
    let mut base = fontdb::Database::new();
    for fuente in FUENTES {
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
        self.seguir_serie(apartados, ancho, &mut SinSeguimiento)
    }

    /// Como [`Documento::componer_serie`], enseñando lo que va saliendo. Si `seguimiento` lo para,
    /// devuelve lo colocado hasta ahí.
    pub fn seguir_serie(
        &self,
        apartados: &[Apartado<'_>],
        ancho: f64,
        seguimiento: &mut dyn Seguimiento,
    ) -> Result<Composicion, ErrorComposicion> {
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
        let formato = Formato {
            ancho,
            ..self.formato.clone()
        };
        Ok(maquetador::maquetar_serie(
            entradas,
            formato,
            &self.piezas,
            &mut sistema,
            seguimiento,
        ))
    }

    /// Compone a `ancho` bp. Reflujar no exige otro documento: cargar las fuentes es caro.
    pub fn componer_a(&self, fuente: &str, ancho: f64) -> Result<Composicion, ErrorComposicion> {
        self.seguir_a(fuente, ancho, &mut SinSeguimiento)
    }

    /// Como [`Documento::componer_a`], enseñando lo que va saliendo. Si `seguimiento` lo para,
    /// devuelve lo colocado hasta ahí.
    pub fn seguir_a(
        &self,
        fuente: &str,
        ancho: f64,
        seguimiento: &mut dyn Seguimiento,
    ) -> Result<Composicion, ErrorComposicion> {
        let lectura = lector::leer(fuente);
        let mut sistema = self
            .fuentes
            .lock()
            .map_err(|_| ErrorComposicion("el sistema de fuentes quedó envenenado".into()))?;
        let formato = Formato {
            ancho,
            ..self.formato.clone()
        };
        Ok(maquetador::maquetar(
            lectura,
            formato,
            &self.piezas,
            &mut sistema,
            seguimiento,
        ))
    }
}

impl Compositor for Documento {
    fn componer(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
        self.componer_a(fuente, self.formato.ancho)
    }
}
