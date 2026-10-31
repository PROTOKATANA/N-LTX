//! El texto corrido: cosmic-text lo parte en líneas y cada glifo se vuelve una primitiva.

use cosmic_text::{Attrs, Buffer, Color as ColorTexto, Family, Metrics, Shaping, Style, Weight};
use primitivas::{Caja, Color, GlifoId, Matriz, Primitiva, Punto};

use super::{Colocable, Columna, Maquetador, rectangulo};
use crate::contornos;
use crate::lector::{Estilo, Tramo};

const ROMANA: &str = "Latin Modern Roman";
const MONO: &str = "Latin Modern Mono";
/// Las fuentes propias del texto se numeran desde aquí: las de las fórmulas usan las primeras.
const PRIMERA_FUENTE: u8 = 16;
/// Lo que ocupa el lugar de una fórmula, con su ancho exacto. Es una letra y no un espacio porque
/// cosmic-text descarta los espacios al final de una línea, y con ellos se perdería la fórmula. No
/// llega a pintarse: en su sitio va la fórmula.
const RESERVA: &str = "x";
/// `metadata` de un glifo: desde aquí, una fórmula; 1, subrayado.
const BASE_FORMULA: usize = 1 << 40;
const SUBRAYADO: usize = 1;
const INTERLINEA: f64 = 1.3;

/// Con qué se escribe un texto: su cuerpo y lo que se suma al estilo de cada tramo.
#[derive(Debug, Clone, Copy)]
pub(super) struct Letra {
    pub(super) tam: f64,
    pub(super) base: Estilo,
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
        let Letra { tam, base } = letra;
        let Columna { x, ancho } = c;
        let tam32 = tam as f32;
        let (avance, ascenso, _) = self.sonda(tam32);
        let mut partes: Vec<(String, Attrs<'static>)> = Vec::new();
        let mut formulas: Vec<Colocable> = Vec::new();
        for tramo in tramos {
            match tramo {
                Tramo::Texto(t, e) => {
                    let estilo = Estilo {
                        negrita: e.negrita || base.negrita,
                        cursiva: e.cursiva || base.cursiva,
                        mono: e.mono || base.mono,
                        subrayado: e.subrayado || base.subrayado,
                    };
                    partes.push((t.clone(), atributos(estilo, tam32)));
                }
                Tramo::Salto => partes.push(("\n".into(), atributos(base, tam32))),
                Tramo::Formula(latex) => {
                    let fuente = self.con_macros(latex);
                    match self.incrustados.formula(&fuente, false, tam) {
                        Ok(c) => {
                            let colocable = self.absorber(c);
                            let caja = colocable.composicion.escena.caja;
                            let (alto, prof) = (-caja.y as f32, (caja.alto + caja.y) as f32);
                            // el interlineado crece lo justo para que quepa la fórmula
                            let exceso = (alto - ascenso).max(prof - (tam32 * 0.3)).max(0.0);
                            let linea = tam32 * INTERLINEA as f32 + 2.0 * exceso;
                            let a = atributos(Estilo::default(), tam32)
                                .metrics(Metrics::new(tam32, linea))
                                .letter_spacing((caja.ancho as f32 - avance) / tam32)
                                .metadata(BASE_FORMULA + formulas.len());
                            partes.push((RESERVA.into(), a));
                            formulas.push(colocable);
                        }
                        Err(e) => {
                            self.avisos.push(e.to_string());
                            let mono = Estilo {
                                mono: true,
                                ..Estilo::default()
                            };
                            partes.push((format!("${latex}$"), atributos(mono, tam32)));
                        }
                    }
                }
            }
        }
        let defecto = atributos(base, tam32);
        let mut b = Buffer::new(self.sistema, Metrics::new(tam32, tam32 * INTERLINEA as f32));
        b.set_size(Some(ancho as f32), None);
        b.set_rich_text(
            partes.iter().map(|(t, a)| (t.as_str(), a.clone())),
            &defecto,
            Shaping::Advanced,
            None,
        );
        b.shape_until_scroll(self.sistema, false);
        let mut alto = 0.0f64;
        let mut formulas: Vec<Option<Colocable>> = formulas.into_iter().map(Some).collect();
        let lineas: Vec<(Vec<cosmic_text::LayoutGlyph>, f32, f32)> = b
            .layout_runs()
            .map(|run| (run.glyphs.to_vec(), run.line_y, run.line_top + run.line_height))
            .collect();
        for (glifos, base_linea, fondo) in lineas {
            let y = self.y + f64::from(base_linea);
            self.linea(&glifos, Punto { x, y }, &mut formulas);
            alto = alto.max(f64::from(fondo));
        }
        self.y += alto;
    }

    fn linea(&mut self, glifos: &[cosmic_text::LayoutGlyph], base: Punto, formulas: &mut [Option<Colocable>]) {
        let Punto { x, y } = base;
        let mut subrayado: Option<(f64, f64, f32)> = None;
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
                let (inicio, _, _) = subrayado.unwrap_or((gx, gx, g.font_size));
                subrayado = Some((inicio, gx + f64::from(g.w), g.font_size));
            } else if let Some(tramo) = subrayado.take() {
                self.subrayar(tramo, y);
            }
        }
        if let Some(tramo) = subrayado {
            self.subrayar(tramo, y);
        }
    }

    fn subrayar(&mut self, (desde, hasta, tam): (f64, f64, f32), y: f64) {
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

fn color_texto(c: ColorTexto) -> Color {
    Color {
        r: f64::from(c.r()) / 255.0,
        g: f64::from(c.g()) / 255.0,
        b: f64::from(c.b()) / 255.0,
    }
}
