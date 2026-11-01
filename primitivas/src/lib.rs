//! Lista neutra de primitivas: lo que un compositor entrega y un pintor consume.
//!
//! Este crate es la interfaz entre piezas y no depende de nada: ni de quien calcula la geometría
//! ni de quien la pinta. Una escena se basta a sí misma: trae los contornos de sus glifos, así que
//! el pintor no necesita acceso a las fuentes.
//!
//! Unidades: puntos PostScript (bp, 1/72 de pulgada), con la y hacia abajo. Salvo en los contornos
//! de `Escena::contornos`, que están en unidades de su fuente y con la y hacia arriba: la
//! `matriz` de cada `Primitiva::Glifo` los lleva a la escena.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Punto {
    pub x: f64,
    pub y: f64,
}

/// Transformación afín: un punto (x, y) va a (a·x + c·y + e, b·x + d·y + f).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matriz {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

/// Color sRGB, con cada canal de 0 a 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

impl Color {
    pub const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };
    pub const BLANCO: Color = Color { r: 1.0, g: 1.0, b: 1.0 };

    /// De `0xRRGGBB`, como se escribe en CSS.
    pub fn de_rgb(rgb: u32) -> Color {
        let canal = |desplazado: u32| f64::from((rgb >> desplazado) & 0xff) / 255.0;
        Color {
            r: canal(16),
            g: canal(8),
            b: canal(0),
        }
    }

    /// La luminancia relativa de WCAG: de 0 a 1.
    pub fn luminancia(self) -> f64 {
        let lineal = |v: f64| {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * lineal(self.r) + 0.7152 * lineal(self.g) + 0.0722 * lineal(self.b)
    }

    /// El contraste de WCAG entre dos colores: de 1 a 21.
    pub fn contraste(self, otro: Color) -> f64 {
        let (a, b) = (self.luminancia(), otro.luminancia());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segmento {
    Mover(Punto),
    Linea(Punto),
    /// Bézier cúbica: dos puntos de control y el punto final.
    Curva(Punto, Punto, Punto),
    Cerrar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Remate {
    Plano,
    Redondo,
    Cuadrado,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Union {
    Inglete,
    Redonda,
    Biselada,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Discontinuo {
    pub tramos: Vec<f64>,
    pub fase: f64,
}

/// Hasta dónde llega la punta de un codo desde su vértice, en medios anchos de línea: más allá, el
/// codo se bisela. Así un trazo no sobresale de su vértice más que un ancho, sea cual sea el ángulo,
/// y aún deja en punta un triángulo equilátero (razón 2). El 10 de PDF permite puntas de cinco anchos,
/// que en un codo muy cerrado se ven como una línea que se pasa del punto donde debía acabar.
pub const LIMITE_INGLETE: f64 = 2.1;

#[derive(Debug, Clone, PartialEq)]
pub struct Trazo {
    pub color: Color,
    pub ancho: f64,
    pub discontinuo: Option<Discontinuo>,
    pub remate: Remate,
    pub union: Union,
    pub limite_inglete: f64,
    pub opacidad: f64,
}

impl Trazo {
    /// Línea continua y opaca, con remate plano.
    pub fn simple(color: Color, ancho: f64) -> Trazo {
        Trazo {
            color,
            ancho,
            discontinuo: None,
            remate: Remate::Plano,
            union: Union::Inglete,
            limite_inglete: LIMITE_INGLETE,
            opacidad: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parada {
    pub posicion: f64,
    pub color: Color,
}

/// Degradado definido en su propio espacio; `matriz` lo lleva a la escena.
#[derive(Debug, Clone, PartialEq)]
pub enum Degradado {
    Lineal {
        desde: Punto,
        hasta: Punto,
        matriz: Matriz,
        paradas: Vec<Parada>,
    },
    /// Centrado en el origen de su espacio.
    Radial {
        radio: f64,
        foco: Punto,
        matriz: Matriz,
        paradas: Vec<Parada>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pintura {
    Solida(Color),
    Degradado(Degradado),
    /// Patrón con nombre de la biblioteca `patterns` de TikZ, repetido desde `esquina`.
    Patron {
        nombre: String,
        color: Color,
        esquina: Punto,
    },
    /// Baldosa de `ancho`×`alto` con esquina en `origen`, repetida; `matriz` la lleva a la escena.
    Mosaico {
        primitivas: Vec<Primitiva>,
        origen: Punto,
        ancho: f64,
        alto: f64,
        matriz: Matriz,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Relleno {
    pub pintura: Pintura,
    pub opacidad: f64,
    pub par_impar: bool,
}

/// Un glifo de una fuente. `fuente` solo distingue fuentes dentro de una misma `Escena`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlifoId {
    pub fuente: u8,
    pub id: u16,
}

/// Lo que abre `Primitiva::Abrir`: afecta a todo hasta su `Primitiva::Cerrar`.
#[derive(Debug, Clone, PartialEq)]
pub enum Capa {
    Recorte {
        segmentos: Vec<Segmento>,
        par_impar: bool,
    },
    /// Grupo aislado: se compone aparte y entra en la escena con esta opacidad.
    Grupo {
        opacidad: f64,
    },
    Transformacion(Matriz),
    /// Modo de mezcla con su nombre CSS (`multiply`, `screen`…).
    Mezcla(String),
    /// La luminancia del degradado da la transparencia.
    MascaraDegradado(Degradado),
    /// La luminancia de estas primitivas da la transparencia.
    MascaraEscena(Vec<Primitiva>),
    /// Bordes sin suavizar.
    BordesNitidos,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Primitiva {
    /// Relleno y trazo de una misma geometría se pintan como una sola cosa, igual que en PDF. Con
    /// la misma opacidad son un grupo: el trazo no oscurece el relleno. Con un trazo translúcido y
    /// otra opacidad en el relleno, el relleno no se ve debajo del trazo.
    Trazado {
        segmentos: Vec<Segmento>,
        relleno: Option<Relleno>,
        trazo: Option<Trazo>,
    },
    Glifo {
        glifo: GlifoId,
        matriz: Matriz,
        color: Color,
    },
    /// Una imagen en el cuadrado unidad, con su primera fila en y = 0; `matriz` la lleva a la escena.
    Imagen {
        mapa: Arc<MapaDeBits>,
        matriz: Matriz,
    },
    Abrir(Capa),
    Cerrar,
}

/// Una imagen decodificada: RGBA de 8 bits por canal, con el alfa ya multiplicado. Quien la pinta
/// la reconoce por `id`, que depende solo de sus píxeles: republicar no la vuelve a subir.
#[derive(Clone)]
pub struct MapaDeBits {
    pub id: u64,
    pub ancho: u32,
    pub alto: u32,
    pub rgba: Vec<u8>,
}

impl PartialEq for MapaDeBits {
    fn eq(&self, otro: &MapaDeBits) -> bool {
        self.id == otro.id
    }
}

impl fmt::Debug for MapaDeBits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MapaDeBits({:016x}, {}×{})", self.id, self.ancho, self.alto)
    }
}

/// Rectángulo de la escena.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Caja {
    pub x: f64,
    pub y: f64,
    pub ancho: f64,
    pub alto: f64,
}

/// Un trozo de la escena que se puede seleccionar y los bytes de la fuente de los que sale.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Ancla {
    pub caja: Caja,
    /// Qué documento de una serie.
    pub documento: u32,
    pub desde: u32,
    pub hasta: u32,
}

/// Una animación dentro de una escena: el pintor reproduce `fuente` dentro de `caja`.
#[derive(Debug, Clone, PartialEq)]
pub struct Zona {
    pub caja: Caja,
    pub fuente: String,
}

/// Lo que hay que pintar. En una fórmula, la línea base es y = 0.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Escena {
    pub caja: Caja,
    pub fondo: Option<Color>,
    pub primitivas: Vec<Primitiva>,
    /// El contorno de cada glifo usado, en unidades de su fuente y con la y hacia arriba.
    pub contornos: BTreeMap<GlifoId, Vec<Segmento>>,
    /// Dónde cae el origen del dibujo. Una figura se desplaza para empezar en su esquina, así que
    /// dos fotogramas solo coinciden si se restan esto.
    pub origen: Punto,
    pub zonas: Vec<Zona>,
    /// Lo seleccionable, en orden de lectura.
    pub anclas: Vec<Ancla>,
}

/// Una escena y lo que el compositor tuvo que ignorar o aproximar para obtenerla. Sin avisos, la
/// fuente se entendió entera: quien muestre el resultado no debe descartarlos en silencio.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Composicion {
    pub escena: Escena,
    pub avisos: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorComposicion(pub String);

impl fmt::Display for ErrorComposicion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ErrorComposicion {}

/// Convierte una fuente LaTeX en una escena.
pub trait Compositor {
    fn componer(&self, fuente: &str) -> Result<Composicion, ErrorComposicion>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_color_se_escribe_como_en_css() {
        let c = Color::de_rgb(0x575965);
        assert_eq!(c.r, 87.0 / 255.0);
        assert_eq!(c.g, 89.0 / 255.0);
        assert_eq!(c.b, 101.0 / 255.0);
        assert_eq!(Color::de_rgb(0xffffff), Color::BLANCO);
        assert_eq!(Color::de_rgb(0), Color::NEGRO);
    }

    #[test]
    fn el_contraste_va_de_1_a_21() {
        assert!((Color::BLANCO.contraste(Color::NEGRO) - 21.0).abs() < 1e-9);
        assert!((Color::NEGRO.contraste(Color::NEGRO) - 1.0).abs() < 1e-9);
    }
}
