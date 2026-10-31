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
            limite_inglete: 4.0,
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
    /// PNG en el cuadrado unidad; `matriz` lo lleva a la escena.
    Imagen {
        png: Vec<u8>,
        matriz: Matriz,
    },
    Abrir(Capa),
    Cerrar,
}

/// Rectángulo de la escena.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Caja {
    pub x: f64,
    pub y: f64,
    pub ancho: f64,
    pub alto: f64,
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
