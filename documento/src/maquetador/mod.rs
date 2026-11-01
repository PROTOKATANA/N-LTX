//! Coloca lo leído: los bloques se apilan y cada texto lo parte cosmic-text.

use std::collections::{BTreeMap, HashMap};

use cosmic_text::{FontSystem, fontdb};
use primitivas::{
    Ancla, Caja, Capa, Color, Composicion, ErrorComposicion, Escena, GlifoId, Matriz, Pintura, Primitiva, Punto,
    Relleno, Segmento, Zona,
};

use crate::lector::{Bloque, Estilo, Lectura, Tramo};
use crate::tema::Opciones;
use crate::{ColorDefinido, Formato, Piezas};

mod editor;
mod imagen;
mod tablas;
mod texto;

use editor::BloqueDeCodigo;
use texto::Letra;

/// Separación entre bloques y sangría de las listas, en cuerpos de letra.
const SEPARACION: f64 = 0.7;
const SANGRIA: f64 = 1.8;
/// Ancho de las reglas, en bp.
const REGLA: f64 = 0.4;

/// Un documento dentro de una serie, ya leído.
pub(crate) struct Entrada<'t> {
    pub(crate) titulo: Option<&'t str>,
    pub(crate) lectura: Lectura,
}

pub(crate) fn maquetar(lectura: Lectura, formato: Formato, piezas: &Piezas, sistema: &mut FontSystem) -> Composicion {
    let mut m = Maquetador::nuevo(formato, piezas, sistema);
    let bloques = m.adoptar(lectura);
    m.bloques(&bloques, m.columna());
    m.cerrar()
}

/// Varios documentos seguidos en una misma escena. Cada uno se lee aparte y no ve el preámbulo de
/// los demás. Sus avisos se escriben debajo de él: juntos al final no se sabría de cuál son.
pub(crate) fn maquetar_serie(
    entradas: Vec<Entrada<'_>>,
    formato: Formato,
    piezas: &Piezas,
    sistema: &mut FontSystem,
) -> Composicion {
    let mut m = Maquetador::nuevo(formato, piezas, sistema);
    for (n, entrada) in entradas.into_iter().enumerate() {
        m.entrada(entrada, n == 0);
    }
    m.cerrar()
}

struct Maquetador<'a> {
    formato: Formato,
    piezas: &'a Piezas,
    sistema: &'a mut FontSystem,
    preambulo: String,
    macros: String,
    /// Los colores de `\textcolor`, en el orden de [`Lectura::colores`].
    colores: Vec<Option<Color>>,
    definidos: Vec<ColorDefinido>,
    estilos: Vec<(String, Opciones)>,
    codigo_del_documento: Opciones,
    avisos: Vec<String>,
    primitivas: Vec<Primitiva>,
    contornos: BTreeMap<GlifoId, Vec<Segmento>>,
    fuentes: HashMap<fontdb::ID, u8>,
    zonas: Vec<Zona>,
    anclas: Vec<Ancla>,
    /// Qué documento de la serie se está colocando.
    documento: u32,
    /// Dónde va el siguiente bloque.
    y: f64,
}

/// El espacio horizontal de un bloque.
#[derive(Debug, Clone, Copy)]
struct Columna {
    x: f64,
    ancho: f64,
}

impl Columna {
    /// Lo que queda a la derecha de `cuanto`.
    fn sangrada(self, cuanto: f64) -> Columna {
        Columna {
            x: self.x + cuanto,
            ancho: self.ancho - cuanto,
        }
    }

    /// Lo que queda con `margen` a cada lado.
    fn con_margen(self, margen: f64) -> Columna {
        Columna {
            x: self.x + margen,
            ancho: self.ancho - 2.0 * margen,
        }
    }

    /// Una caja de `alto` que ocupa toda la columna.
    fn franja(self, y: f64, alto: f64) -> Caja {
        Caja {
            x: self.x,
            y,
            ancho: self.ancho,
            alto,
        }
    }
}

impl<'a> Maquetador<'a> {
    fn nuevo(formato: Formato, piezas: &'a Piezas, sistema: &'a mut FontSystem) -> Self {
        Maquetador {
            formato,
            piezas,
            sistema,
            preambulo: String::new(),
            macros: String::new(),
            colores: Vec::new(),
            definidos: Vec::new(),
            estilos: Vec::new(),
            codigo_del_documento: Vec::new(),
            avisos: Vec::new(),
            primitivas: Vec::new(),
            contornos: BTreeMap::new(),
            fuentes: HashMap::new(),
            zonas: Vec::new(),
            anclas: Vec::new(),
            documento: 0,
            y: 0.0,
        }
    }

    fn columna(&self) -> Columna {
        Columna {
            x: 0.0,
            ancho: self.formato.ancho,
        }
    }

    /// Lo que afecta a todo un documento. Devuelve lo que hay que colocar.
    fn adoptar(&mut self, lectura: Lectura) -> Vec<Bloque> {
        let Lectura {
            bloques,
            preambulo,
            macros,
            colores,
            definidos,
            estilos,
            codigo,
            avisos,
        } = lectura;
        self.preambulo = preambulo;
        self.macros = macros;
        self.colores = colores
            .iter()
            .map(|n| self.piezas.colores.color(n, &definidos))
            .collect();
        self.definidos = definidos;
        self.estilos = estilos;
        self.codigo_del_documento = codigo;
        self.avisos.extend(avisos);
        bloques
    }

    fn entrada(&mut self, entrada: Entrada<'_>, primera: bool) {
        let columna = self.columna();
        if !primera {
            self.bloque(&Bloque::Regla, columna);
        }
        let bloques = self.adoptar(entrada.lectura);
        if let Some(titulo) = entrada.titulo {
            let tramos = vec![Tramo::Texto(titulo.into(), Estilo::default())];
            self.bloque(&Bloque::Titulo { nivel: 3, tramos }, columna);
        }
        self.bloques(&bloques, columna);
        for aviso in std::mem::take(&mut self.avisos) {
            let tramos = vec![Tramo::Texto(format!("⚠ {aviso}").into(), Estilo::default())];
            self.bloque(&Bloque::Leyenda(tramos), columna);
        }
        self.documento += 1;
    }

    fn cerrar(self) -> Composicion {
        let Maquetador {
            formato,
            primitivas,
            contornos,
            avisos,
            zonas,
            anclas,
            y,
            ..
        } = self;
        Composicion {
            escena: Escena {
                caja: Caja {
                    x: 0.0,
                    y: 0.0,
                    ancho: formato.ancho,
                    alto: y,
                },
                primitivas,
                contornos,
                zonas,
                anclas,
                ..Escena::default()
            },
            avisos,
        }
    }
}

fn rectangulo(c: Caja, color: Color) -> Primitiva {
    let p = |x, y| Punto { x, y };
    let (x1, y1) = (c.x + c.ancho, c.y + c.alto);
    Primitiva::Trazado {
        segmentos: vec![
            Segmento::Mover(p(c.x, c.y)),
            Segmento::Linea(p(x1, c.y)),
            Segmento::Linea(p(x1, y1)),
            Segmento::Linea(p(c.x, y1)),
            Segmento::Cerrar,
        ],
        relleno: Some(Relleno {
            pintura: Pintura::Solida(color),
            opacidad: 1.0,
            par_impar: false,
        }),
        trazo: None,
    }
}

/// Lo que se selecciona entero: su fuente no se puede partir.
fn indivisible(bloque: &Bloque) -> bool {
    !matches!(
        bloque,
        Bloque::Titulo { .. } | Bloque::Parrafo(_) | Bloque::Leyenda(_) | Bloque::Lista { .. } | Bloque::Cita(_)
    )
}

/// Una fórmula o figura ya compuesta, a la espera de su sitio.
struct Colocable {
    composicion: Composicion,
}

impl Maquetador<'_> {
    fn bloques(&mut self, bloques: &[Bloque], columna: Columna) {
        let mut origen = None;
        for bloque in bloques {
            if let Bloque::Origen(a, b) = bloque {
                origen = Some((*a, *b));
                continue;
            }
            let (y, anclas) = (self.y, self.anclas.len());
            self.bloque(bloque, columna);
            match origen.take() {
                Some(rango) if indivisible(bloque) => {
                    // una tabla escribe texto, pero se selecciona entera
                    self.anclas.truncate(anclas);
                    self.anclar(columna.franja(y, self.y - y), rango);
                }
                _ => {}
            }
        }
    }

    fn anclar(&mut self, caja: Caja, (desde, hasta): (u32, u32)) {
        self.anclas.push(Ancla {
            caja,
            documento: self.documento,
            desde,
            hasta,
        });
    }

    fn separar(&mut self, cuanto: f64) {
        self.y += cuanto * self.formato.tam;
    }

    fn bloque(&mut self, bloque: &Bloque, c: Columna) {
        let tam = self.formato.tam;
        match bloque {
            Bloque::Titulo { nivel, tramos } => {
                let escala = match nivel {
                    1 => 1.5,
                    2 => 1.25,
                    _ => 1.1,
                };
                self.separar(0.6);
                let negrita = Estilo {
                    negrita: true,
                    ..Estilo::default()
                };
                let letra = Letra {
                    tam: tam * escala,
                    base: negrita,
                };
                self.texto(tramos, c, letra);
                self.separar(0.3);
            }
            Bloque::Parrafo(tramos) => {
                self.texto(tramos, c, Letra::normal(tam));
                self.separar(SEPARACION);
            }
            Bloque::Leyenda(tramos) => {
                self.texto(tramos, c, Letra::normal(tam * 0.9));
                self.separar(SEPARACION);
            }
            Bloque::Lista { ordenada, items } => self.lista(*ordenada, items, c),
            Bloque::Cita(dentro) => {
                let (inicio, sangria) = (self.y, SANGRIA * tam);
                self.bloques(dentro, c.sangrada(sangria));
                let gris = Color { r: 0.7, g: 0.7, b: 0.7 };
                let barra = Caja {
                    x: c.x + tam * 0.4,
                    y: inicio,
                    ancho: 1.2,
                    alto: (self.y - inicio - SEPARACION * tam).max(0.0),
                };
                self.primitivas.push(rectangulo(barra, gris));
            }
            Bloque::Codigo {
                texto,
                lenguaje,
                opciones,
            } => {
                let bloque = BloqueDeCodigo {
                    texto,
                    lenguaje: lenguaje.as_deref(),
                    opciones,
                };
                self.editor(bloque, c);
            }
            Bloque::Formula(latex) => self.formula_en_bloque(latex, c),
            Bloque::Figura(fuente) => self.figura(fuente, c),
            Bloque::Imagen { ruta, opciones } => self.imagen(ruta, opciones, c),
            Bloque::Animacion(fuente) => self.animacion(fuente, c),
            Bloque::Tabla(tabla) => self.tabla(tabla, c),
            Bloque::Regla => {
                self.separar(0.4);
                let color = self.formato.color;
                self.primitivas.push(rectangulo(c.franja(self.y, REGLA), color));
                self.separar(0.6);
            }
            Bloque::Origen(..) => {}
        }
    }

    fn lista(&mut self, ordenada: bool, items: &[Vec<Bloque>], c: Columna) {
        let tam = self.formato.tam;
        let sangria = SANGRIA * tam;
        for (n, item) in items.iter().enumerate() {
            let marca = if ordenada { format!("{}.", n + 1) } else { "•".into() };
            let inicio = self.y;
            let tramos = [Tramo::Texto(marca.into(), Estilo::default())];
            let hueco = Columna {
                x: c.x + tam * 0.4,
                ancho: sangria,
            };
            self.texto(&tramos, hueco, Letra::normal(tam));
            self.y = inicio;
            self.bloques(item, c.sangrada(sangria));
        }
        self.separar(0.2);
    }

    fn indice_de_color(&mut self, color: Color) -> usize {
        let buscado = Some(color);
        self.colores.iter().position(|c| *c == buscado).unwrap_or_else(|| {
            self.colores.push(buscado);
            self.colores.len() - 1
        })
    }

    /// La fuente de lo que no se pudo componer, en un bloque sencillo.
    fn fuente_cruda(&mut self, fuente: &str, c: Columna) {
        let tam = self.formato.tam * 0.84;
        let margen = tam * 0.75;
        let inicio = self.primitivas.len();
        let arriba = self.y;
        self.y += margen;
        let mono = Estilo {
            mono: true,
            ..Estilo::default()
        };
        let tramos = [Tramo::Texto(fuente.into(), mono)];
        self.texto(&tramos, c.con_margen(margen), Letra { tam, base: mono });
        self.y += margen;
        // el fondo va debajo de lo que ya se ha escrito
        let fondo = rectangulo(c.franja(arriba, self.y - arriba), self.formato.tema.fondo_codigo);
        self.primitivas.insert(inicio, fondo);
        self.separar(SEPARACION);
    }

    fn con_macros(&self, latex: &str) -> String {
        format!("{}{latex}", self.macros)
    }

    fn formula_en_bloque(&mut self, latex: &str, columna: Columna) {
        let fuente = self.con_macros(latex);
        match self.piezas.incrustados.formula(&fuente, true, self.formato.tam) {
            Ok(c) => {
                self.separar(0.3);
                let colocable = self.absorber(c);
                self.colocar_centrado(colocable, columna);
                self.separar(SEPARACION);
            }
            Err(e) => self.fuente_visible(latex, &e, columna),
        }
    }

    /// Un bloque centrado. Lo que es más ancho que el texto se reduce para caber: un documento no
    /// se desplaza de lado.
    fn colocar_centrado(&mut self, colocable: Colocable, c: Columna) {
        let caja = colocable.composicion.escena.caja;
        let escala = if caja.ancho > c.ancho {
            c.ancho / caja.ancho
        } else {
            1.0
        };
        let px = c.x + ((c.ancho - caja.ancho * escala) / 2.0).max(0.0);
        let esquina = Punto {
            x: px - caja.x * escala,
            y: self.y - caja.y * escala,
        };
        self.colocar(colocable, esquina, escala);
        self.y += caja.alto * escala;
    }

    fn figura(&mut self, fuente: &str, columna: Columna) {
        let completa = format!("{}{fuente}", self.preambulo);
        match self.piezas.incrustados.figura(&completa) {
            Ok(c) => {
                self.separar(0.3);
                let colocable = self.absorber(c);
                self.colocar_centrado(colocable, columna);
                self.separar(SEPARACION);
            }
            Err(e) => self.fuente_visible(fuente, &e, columna),
        }
    }

    /// Una animación reserva su sitio: el pintor la reproduce ahí.
    fn animacion(&mut self, fuente: &str, c: Columna) {
        let completa = format!("{}{fuente}", self.preambulo);
        match self.piezas.incrustados.caja_de_animacion(&completa) {
            Ok(caja) => {
                self.separar(0.3);
                let px = c.x + ((c.ancho - caja.ancho) / 2.0).max(0.0);
                self.zonas.push(Zona {
                    caja: Caja {
                        x: px,
                        y: self.y,
                        ancho: caja.ancho,
                        alto: caja.alto,
                    },
                    fuente: completa,
                });
                self.y += caja.alto;
                self.separar(SEPARACION);
            }
            Err(e) => self.fuente_visible(fuente, &e, c),
        }
    }

    /// Lo que no se pudo componer se ve como fuente: el lector sabe que algo falta.
    fn fuente_visible(&mut self, fuente: &str, error: &ErrorComposicion, c: Columna) {
        self.avisos.push(error.to_string());
        self.fuente_cruda(fuente, c);
    }

    /// Hace suyos los contornos y avisos de lo incrustado.
    fn absorber(&mut self, mut c: Composicion) -> Colocable {
        self.contornos.append(&mut c.escena.contornos);
        for aviso in std::mem::take(&mut c.avisos) {
            if !self.avisos.contains(&aviso) {
                self.avisos.push(aviso);
            }
        }
        Colocable { composicion: c }
    }

    fn colocar(&mut self, colocable: Colocable, esquina: Punto, escala: f64) {
        self.primitivas.push(Primitiva::Abrir(Capa::Transformacion(Matriz {
            a: escala,
            b: 0.0,
            c: 0.0,
            d: escala,
            e: esquina.x,
            f: esquina.y,
        })));
        self.primitivas.extend(colocable.composicion.escena.primitivas);
        self.primitivas.push(Primitiva::Cerrar);
    }
}
