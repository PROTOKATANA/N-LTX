//! El texto corrido: cosmic-text lo parte en líneas y cada glifo se vuelve una primitiva.

use cosmic_text::{Align, Attrs, Buffer, Color as ColorTexto, Family, Metrics, Shaping, Style, Weight, Wrap};
use primitivas::{Caja, Color, GlifoId, Matriz, Primitiva, Punto};

use super::{Colocable, Columna, Maquetador, rectangulo};
use crate::contornos;
use crate::lector::{Estilo, Tramo};

const ROMANA: &str = "Latin Modern Roman";
const MONO: &str = "JetBrains Mono";
/// Las fuentes propias del texto se numeran desde aquí: las de las fórmulas usan las primeras.
const PRIMERA_FUENTE: u8 = 16;
/// Lo que ocupa el lugar de una fórmula, con su ancho exacto. Es una letra y no un espacio porque
/// cosmic-text descarta los espacios al final de una línea, y con ellos se perdería la fórmula. No
/// llega a pintarse: en su sitio va la fórmula.
const RESERVA: &str = "x";
/// `metadata` de un glifo: desde aquí, una fórmula; 1, subrayado.
const BASE_FORMULA: usize = 1 << 40;
const SUBRAYADO: usize = 1;
/// `metadata` de lo que no lleva tramo propio. cosmic-text da por buena una palabra ASCII si sus
/// tramos con atributos propios son compatibles, sin mirar el resto: en `\textbf{a}:`, los dos puntos
/// salían en negrita. Así ningún texto lleva los atributos por defecto.
const SIN_TRAMO: usize = 2;
const INTERLINEA: f64 = 1.3;

/// Con qué se escribe un texto: su cuerpo y lo que se suma al estilo de cada tramo.
#[derive(Debug, Clone, Copy)]
pub(super) struct Letra {
    pub(super) tam: f64,
    pub(super) base: Estilo,
}

/// Un trozo de texto con sus atributos, como lo quiere `cosmic-text`.
type Parte = (String, Attrs<'static>);

/// El estilo propio de un tramo sobre el de lo que lo rodea.
fn sobre(propio: &Estilo, base: Estilo) -> Estilo {
    Estilo {
        negrita: propio.negrita || base.negrita,
        cursiva: propio.cursiva || base.cursiva,
        mono: propio.mono || base.mono,
        subrayado: propio.subrayado || base.subrayado,
        color: propio.color.or(base.color),
        escala: propio.escala.or(base.escala),
    }
}

/// Un trozo de línea subrayado.
#[derive(Clone, Copy)]
struct Subrayado {
    desde: f64,
    hasta: f64,
    tam: f32,
}

/// El sitio de un texto: su ancho, si tiene límite, y cómo se alinea dentro.
#[derive(Debug, Clone, Copy)]
pub(super) struct Hueco {
    pub(super) ancho: Option<f64>,
    pub(super) alineado: Option<Align>,
    /// Si una palabra que no quepa puede partirse. Sin partir, el ancho de lo escrito dice lo mínimo
    /// que necesita: la palabra o la fórmula mayor.
    pub(super) partir_palabras: bool,
}

impl Hueco {
    /// Sin límite de ancho: el texto toma el que necesita.
    pub(super) fn libre() -> Hueco {
        Hueco {
            ancho: None,
            alineado: None,
            partir_palabras: true,
        }
    }

    pub(super) fn de(ancho: f64, alineado: Option<Align>) -> Hueco {
        Hueco {
            ancho: Some(ancho),
            alineado,
            partir_palabras: true,
        }
    }

    /// El más estrecho posible, para saber qué no se puede partir.
    pub(super) fn minimo() -> Hueco {
        Hueco {
            ancho: Some(1.0),
            alineado: None,
            partir_palabras: false,
        }
    }
}

/// Un texto ya partido en líneas, por colocar.
pub(super) struct Escrito {
    /// Los glifos de cada línea y la altura de su base.
    lineas: Vec<(Vec<cosmic_text::LayoutGlyph>, f32)>,
    formulas: Vec<Option<Colocable>>,
    pub(super) ancho: f64,
    pub(super) alto: f64,
}

impl Escrito {
    /// Hasta dónde llega su primera línea: en un texto partido, la que lleva el número.
    pub(super) fn alto_primera_linea(&self) -> f64 {
        self.alto / self.lineas.len().max(1) as f64
    }

    /// Si no ha cabido en una línea.
    pub(super) fn lineas_partidas(&self) -> bool {
        self.lineas.len() > 1
    }
}

impl Letra {
    pub(super) fn normal(tam: f64) -> Letra {
        Letra {
            tam,
            base: Estilo::default(),
        }
    }
}

fn atributos(estilo: Estilo, tam: f32) -> Attrs<'static> {
    let familia = if estilo.mono { MONO } else { ROMANA };
    let tam = estilo.escala.map_or(tam, |e| tam * f32::from(e) / 100.0);
    let mut a = Attrs::new()
        .family(Family::Name(familia))
        .metrics(Metrics::new(tam, tam * INTERLINEA as f32));
    if estilo.negrita {
        a = a.weight(Weight::BOLD);
    }
    if estilo.cursiva {
        a = a.style(Style::Italic);
    }
    if estilo.subrayado {
        a = a.metadata(SUBRAYADO);
    }
    a
}

impl Maquetador<'_> {
    fn con_color(&self, a: Attrs<'static>, estilo: Estilo) -> Attrs<'static> {
        match estilo.color.and_then(|i| self.colores.get(i).copied().flatten()) {
            Some(c) => a.color(color_de_texto(c)),
            None => a,
        }
    }

    /// Ancho de un espacio de reserva y medidas de su línea.
    fn sonda(&mut self, tam: f32) -> (f32, f32, f32) {
        let a = atributos(Estilo::default(), tam);
        let mut b = Buffer::new(self.sistema, Metrics::new(tam, tam * INTERLINEA as f32));
        b.set_rich_text([(RESERVA, a.clone())], &a, Shaping::Advanced, None);
        b.shape_until_scroll(self.sistema, false);
        let medidas = b.layout_runs().next().map(|run| {
            let avance = run.glyphs.first().map_or(tam * 0.3, |g| g.w);
            (avance, run.line_y - run.line_top, run.line_height)
        });
        medidas.unwrap_or((tam * 0.3, tam, tam * INTERLINEA as f32))
    }

    pub(super) fn texto(&mut self, tramos: &[Tramo], c: Columna, letra: Letra) {
        let escrito = self.escribir(tramos, Hueco::de(c.ancho, None), letra);
        let alto = escrito.alto;
        self.poner(escrito, Punto { x: c.x, y: self.y });
        self.y += alto;
    }

    /// Lo que se ve en color lo lleva también una fórmula: en una cabecera oscura no se leería.
    fn formula_en_color(&self, latex: &str, estilo: Estilo) -> String {
        match estilo.color.and_then(|i| self.colores.get(i).copied().flatten()) {
            Some(c) => {
                let ColorHtml(html) = ColorHtml::de(c);
                format!("\\textcolor[HTML]{{{html}}}{{{latex}}}")
            }
            None => latex.to_string(),
        }
    }

    /// Parte `tramos` en líneas sin ponerlos en ninguna parte: así se puede medir.
    pub(super) fn escribir(&mut self, tramos: &[Tramo], hueco: Hueco, letra: Letra) -> Escrito {
        let (partes, formulas) = self.partes_de(tramos, letra);
        let texto = self.en_lineas(&partes, hueco, letra);
        let (mut ancho, mut alto) = (0.0f64, 0.0f64);
        let mut lineas = Vec::new();
        for run in texto.layout_runs() {
            ancho = ancho.max(f64::from(run.line_w));
            alto = alto.max(f64::from(run.line_top + run.line_height));
            lineas.push((run.glyphs.to_vec(), run.line_y));
        }
        Escrito {
            lineas,
            formulas: formulas.into_iter().map(Some).collect(),
            ancho,
            alto,
        }
    }

    /// Cada tramo como texto con sus atributos; una fórmula, como un hueco de su tamaño.
    fn partes_de(&mut self, tramos: &[Tramo], letra: Letra) -> (Vec<Parte>, Vec<Colocable>) {
        let tam = letra.tam as f32;
        let mut partes = Vec::new();
        let mut formulas = Vec::new();
        for tramo in tramos {
            match tramo {
                Tramo::Texto(t, e) => {
                    let estilo = sobre(e, letra.base);
                    partes.push((t.clone(), self.con_color(atributos(estilo, tam), estilo)));
                }
                Tramo::Salto => partes.push(("\n".into(), atributos(letra.base, tam))),
                Tramo::Formula(latex) => partes.push(self.parte_de_formula(latex, letra, &mut formulas)),
            }
        }
        (partes, formulas)
    }

    /// El hueco de una fórmula, que deja en `formulas`; si no se compone, su fuente en un aviso.
    fn parte_de_formula(&mut self, latex: &str, letra: Letra, formulas: &mut Vec<Colocable>) -> Parte {
        let Letra { tam, base } = letra;
        let tam32 = tam as f32;
        let fuente = self.con_macros(&self.formula_en_color(latex, base));
        let compuesta = match self.piezas.incrustados.formula(&fuente, false, tam) {
            Ok(c) => c,
            Err(e) => {
                self.avisos.push(e.to_string());
                let mono = Estilo {
                    mono: true,
                    ..Estilo::default()
                };
                return (format!("${latex}$"), atributos(mono, tam32));
            }
        };
        let (avance, ascenso, _) = self.sonda(tam32);
        let colocable = self.absorber(compuesta);
        let caja = colocable.composicion.escena.caja;
        let (alto, prof) = (-caja.y as f32, (caja.alto + caja.y) as f32);
        // el interlineado crece lo justo para que quepa la fórmula
        let exceso = (alto - ascenso).max(prof - (tam32 * 0.3)).max(0.0);
        let linea = tam32 * INTERLINEA as f32 + 2.0 * exceso;
        let atributos = atributos(Estilo::default(), tam32)
            .metrics(Metrics::new(tam32, linea))
            .letter_spacing((caja.ancho as f32 - avance) / tam32)
            .metadata(BASE_FORMULA + formulas.len());
        formulas.push(colocable);
        (RESERVA.into(), atributos)
    }

    /// `cosmic-text` reparte las partes en líneas.
    fn en_lineas(&mut self, partes: &[Parte], hueco: Hueco, letra: Letra) -> Buffer {
        let tam = letra.tam as f32;
        let defecto = atributos(letra.base, tam).metadata(SIN_TRAMO);
        let mut b = Buffer::new(self.sistema, Metrics::new(tam, tam * INTERLINEA as f32));
        b.set_size(hueco.ancho.map(|a| a as f32), None);
        b.set_wrap(if hueco.partir_palabras {
            Wrap::WordOrGlyph
        } else {
            Wrap::Word
        });
        b.set_rich_text(
            partes.iter().map(|(t, a)| (t.as_str(), a.clone())),
            &defecto,
            Shaping::Advanced,
            hueco.alineado,
        );
        b.shape_until_scroll(self.sistema, false);
        b
    }

    /// Pone un texto con su esquina superior izquierda en `origen`.
    pub(super) fn poner(&mut self, escrito: Escrito, origen: Punto) {
        let Escrito {
            lineas, mut formulas, ..
        } = escrito;
        for (glifos, base_linea) in lineas {
            let y = origen.y + f64::from(base_linea);
            self.linea(&glifos, Punto { x: origen.x, y }, &mut formulas);
        }
    }

    fn linea(&mut self, glifos: &[cosmic_text::LayoutGlyph], base: Punto, formulas: &mut [Option<Colocable>]) {
        let Punto { x, y } = base;
        let mut subrayado: Option<Subrayado> = None;
        for g in glifos {
            let gx = x + f64::from(g.x + g.x_offset * g.font_size);
            if g.metadata >= BASE_FORMULA {
                if let Some(c) = formulas.get_mut(g.metadata - BASE_FORMULA).and_then(Option::take) {
                    self.colocar(c, Punto { x: gx, y }, 1.0);
                }
                continue;
            }
            self.glifo(g, gx, y - f64::from(g.y_offset * g.font_size) + f64::from(g.y));
            if g.metadata == SUBRAYADO {
                subrayado = Some(Subrayado {
                    desde: subrayado.map_or(gx, |s| s.desde),
                    hasta: gx + f64::from(g.w),
                    tam: g.font_size,
                });
            } else if let Some(tramo) = subrayado.take() {
                self.subrayar(tramo, y);
            }
        }
        if let Some(tramo) = subrayado {
            self.subrayar(tramo, y);
        }
    }

    fn subrayar(&mut self, Subrayado { desde, hasta, tam }: Subrayado, y: f64) {
        let grosor = f64::from(tam) * 0.05;
        let color = self.formato.color;
        let raya = Caja {
            x: desde,
            y: y + f64::from(tam) * 0.12,
            ancho: hasta - desde,
            alto: grosor,
        };
        self.primitivas.push(rectangulo(raya, color));
    }

    fn glifo(&mut self, g: &cosmic_text::LayoutGlyph, x: f64, y: f64) {
        let siguiente = PRIMERA_FUENTE.saturating_add(self.fuentes.len().min(200) as u8);
        let fuente = *self.fuentes.entry(g.font_id).or_insert(siguiente);
        let glifo = GlifoId { fuente, id: g.glyph_id };
        let cara = self
            .sistema
            .db()
            .with_face_data(g.font_id, |datos, indice| contornos::de_cara(datos, indice, g.glyph_id))
            .flatten();
        let Some((contorno, unidades)) = cara else {
            return;
        };
        self.contornos.entry(glifo).or_insert(contorno);
        let k = f64::from(g.font_size) / unidades;
        let color = g.color_opt.map_or(self.formato.color, color_texto);
        self.primitivas.push(Primitiva::Glifo {
            glifo,
            matriz: Matriz {
                a: k,
                b: 0.0,
                c: 0.0,
                d: -k,
                e: x,
                f: y,
            },
            color,
        });
    }
}

/// Un color como lo escribe `\\textcolor[HTML]`.
struct ColorHtml(String);

impl ColorHtml {
    fn de(c: Color) -> ColorHtml {
        let t = color_de_texto(c);
        ColorHtml(format!("{:02X}{:02X}{:02X}", t.r(), t.g(), t.b()))
    }
}

fn color_de_texto(c: Color) -> ColorTexto {
    let canal = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    ColorTexto::rgb(canal(c.r), canal(c.g), canal(c.b))
}

fn color_texto(c: ColorTexto) -> Color {
    Color {
        r: f64::from(c.r()) / 255.0,
        g: f64::from(c.g()) / 255.0,
        b: f64::from(c.b()) / 255.0,
    }
}
