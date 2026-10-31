//! Coloca lo leído: los bloques se apilan y cada texto lo parte cosmic-text.

use std::collections::{BTreeMap, HashMap};

use cosmic_text::{Attrs, Buffer, Color as ColorTexto, Family, FontSystem, Metrics, Shaping, Style, Weight, fontdb};
use primitivas::{
    Caja, Capa, Color, Composicion, ErrorComposicion, Escena, GlifoId, Matriz, Pintura, Primitiva, Punto, Relleno,
    Segmento, Zona,
};

use crate::contornos;
use crate::lector::{Bloque, Estilo, Lectura, Tramo};
use crate::{Formato, Incrustados};

const ROMANA: &str = "Latin Modern Roman";
const MONO: &str = "Latin Modern Mono";
/// Las fuentes propias del texto se numeran desde aquí: las de las fórmulas usan las primeras.
const PRIMERA_FUENTE: u8 = 16;
/// Espacio sin corte, del ancho exacto de la fórmula que ocupa su lugar.
const RESERVA: &str = "\u{a0}";
/// `metadata` de un glifo: desde aquí, una fórmula; 1, subrayado.
const BASE_FORMULA: usize = 1 << 40;
const SUBRAYADO: usize = 1;
const INTERLINEA: f64 = 1.3;
/// Separación entre bloques y sangría de las listas, en cuerpos de letra.
const SEPARACION: f64 = 0.7;
const SANGRIA: f64 = 1.8;
/// Ancho de las reglas, en bp.
const REGLA: f64 = 0.4;
const FONDO_CODIGO: Color = Color {
    r: 0.95,
    g: 0.95,
    b: 0.96,
};

pub(crate) fn maquetar(
    lectura: Lectura,
    formato: Formato,
    incrustados: &dyn Incrustados,
    sistema: &mut FontSystem,
) -> Composicion {
    let Lectura {
        bloques,
        preambulo,
        macros,
        avisos,
    } = lectura;
    let mut m = Maquetador {
        formato,
        incrustados,
        sistema,
        preambulo,
        macros,
        avisos,
        primitivas: Vec::new(),
        contornos: BTreeMap::new(),
        fuentes: HashMap::new(),
        zonas: Vec::new(),
        y: 0.0,
    };
    m.bloques(&bloques, 0.0, formato.ancho);
    let Maquetador {
        primitivas,
        contornos,
        avisos,
        zonas,
        y,
        ..
    } = m;
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
            ..Escena::default()
        },
        avisos,
    }
}

struct Maquetador<'a> {
    formato: Formato,
    incrustados: &'a dyn Incrustados,
    sistema: &'a mut FontSystem,
    preambulo: String,
    macros: String,
    avisos: Vec<String>,
    primitivas: Vec<Primitiva>,
    contornos: BTreeMap<GlifoId, Vec<Segmento>>,
    fuentes: HashMap<fontdb::ID, u8>,
    zonas: Vec<Zona>,
    /// Dónde va el siguiente bloque.
    y: f64,
}

fn rectangulo(x: f64, y: f64, ancho: f64, alto: f64, color: Color) -> Primitiva {
    let p = |x, y| Punto { x, y };
    Primitiva::Trazado {
        segmentos: vec![
            Segmento::Mover(p(x, y)),
            Segmento::Linea(p(x + ancho, y)),
            Segmento::Linea(p(x + ancho, y + alto)),
            Segmento::Linea(p(x, y + alto)),
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

/// Una fórmula o figura ya compuesta, a la espera de su sitio.
struct Colocable {
    composicion: Composicion,
}

impl Maquetador<'_> {
    fn bloques(&mut self, bloques: &[Bloque], x: f64, ancho: f64) {
        for bloque in bloques {
            self.bloque(bloque, x, ancho);
        }
    }

    fn separar(&mut self, cuanto: f64) {
        self.y += cuanto * self.formato.tam;
    }

    fn bloque(&mut self, bloque: &Bloque, x: f64, ancho: f64) {
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
                self.texto(tramos, x, ancho, tam * escala, negrita);
                self.separar(0.3);
            }
            Bloque::Parrafo(tramos) => {
                self.texto(tramos, x, ancho, tam, Estilo::default());
                self.separar(SEPARACION);
            }
            Bloque::Leyenda(tramos) => {
                self.texto(tramos, x, ancho, tam * 0.9, Estilo::default());
                self.separar(SEPARACION);
            }
            Bloque::Lista { ordenada, items } => self.lista(*ordenada, items, x, ancho),
            Bloque::Cita(dentro) => {
                let (inicio, sangria) = (self.y, SANGRIA * tam);
                self.bloques(dentro, x + sangria, ancho - sangria);
                let gris = Color { r: 0.7, g: 0.7, b: 0.7 };
                self.primitivas.push(rectangulo(
                    x + tam * 0.4,
                    inicio,
                    1.2,
                    (self.y - inicio - SEPARACION * tam).max(0.0),
                    gris,
                ));
            }
            Bloque::Codigo(codigo) => self.codigo(codigo, x, ancho),
            Bloque::Formula(latex) => self.formula_en_bloque(latex, x, ancho),
            Bloque::Figura(fuente) => self.figura(fuente, x, ancho),
            Bloque::Animacion(fuente) => self.animacion(fuente, x, ancho),
            Bloque::Tabla(filas) => self.tabla(filas, x, ancho),
            Bloque::Regla => {
                self.separar(0.4);
                let color = self.formato.color;
                self.primitivas.push(rectangulo(x, self.y, ancho, REGLA, color));
                self.separar(0.6);
            }
        }
    }

    fn lista(&mut self, ordenada: bool, items: &[Vec<Bloque>], x: f64, ancho: f64) {
        let tam = self.formato.tam;
        let sangria = SANGRIA * tam;
        for (n, item) in items.iter().enumerate() {
            let marca = if ordenada { format!("{}.", n + 1) } else { "•".into() };
            let inicio = self.y;
            let tramos = [Tramo::Texto(marca, Estilo::default())];
            self.texto(&tramos, x + tam * 0.4, sangria, tam, Estilo::default());
            self.y = inicio;
            self.bloques(item, x + sangria, ancho - sangria);
        }
        self.separar(0.2);
    }

    fn codigo(&mut self, codigo: &str, x: f64, ancho: f64) {
        let tam = self.formato.tam * 0.9;
        let margen = tam * 0.5;
        let inicio = self.primitivas.len();
        let arriba = self.y;
        self.y += margen;
        let mono = Estilo {
            mono: true,
            ..Estilo::default()
        };
        let tramos = [Tramo::Texto(codigo.to_string(), mono)];
        self.texto(&tramos, x + margen, ancho - 2.0 * margen, tam, mono);
        self.y += margen;
        // el fondo va debajo de lo que ya se ha escrito
        let fondo = rectangulo(x, arriba, ancho, self.y - arriba, FONDO_CODIGO);
        self.primitivas.insert(inicio, fondo);
        self.separar(SEPARACION);
    }

    fn con_macros(&self, latex: &str) -> String {
        format!("{}{latex}", self.macros)
    }

    fn formula_en_bloque(&mut self, latex: &str, x: f64, ancho: f64) {
        let fuente = self.con_macros(latex);
        match self.incrustados.formula(&fuente, true, self.formato.tam) {
            Ok(c) => {
                self.separar(0.3);
                let colocable = self.absorber(c);
                self.colocar_centrado(colocable, x, ancho);
                self.separar(SEPARACION);
            }
            Err(e) => self.fuente_visible(latex, &e, x, ancho),
        }
    }

    /// Un bloque centrado. Lo que es más ancho que el texto se reduce para caber: un documento no
    /// se desplaza de lado.
    fn colocar_centrado(&mut self, colocable: Colocable, x: f64, ancho: f64) {
        let caja = colocable.composicion.escena.caja;
        let escala = if caja.ancho > ancho { ancho / caja.ancho } else { 1.0 };
        let px = x + ((ancho - caja.ancho * escala) / 2.0).max(0.0);
        self.colocar(colocable, px - caja.x * escala, self.y - caja.y * escala, escala);
        self.y += caja.alto * escala;
    }

    fn figura(&mut self, fuente: &str, x: f64, ancho: f64) {
        let completa = format!("{}{fuente}", self.preambulo);
        match self.incrustados.figura(&completa) {
            Ok(c) => {
                self.separar(0.3);
                let colocable = self.absorber(c);
                self.colocar_centrado(colocable, x, ancho);
                self.separar(SEPARACION);
            }
            Err(e) => self.fuente_visible(fuente, &e, x, ancho),
        }
    }

    /// Una animación reserva su sitio: el pintor la reproduce ahí.
    fn animacion(&mut self, fuente: &str, x: f64, ancho: f64) {
        let completa = format!("{}{fuente}", self.preambulo);
        match self.incrustados.caja_de_animacion(&completa) {
            Ok(caja) => {
                self.separar(0.3);
                let px = x + ((ancho - caja.ancho) / 2.0).max(0.0);
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
            Err(e) => self.fuente_visible(fuente, &e, x, ancho),
        }
    }

    /// Lo que no se pudo componer se ve como fuente: el lector sabe que algo falta.
    fn fuente_visible(&mut self, fuente: &str, error: &ErrorComposicion, x: f64, ancho: f64) {
        self.avisos.push(error.to_string());
        self.codigo(fuente, x, ancho);
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

    fn colocar(&mut self, colocable: Colocable, x: f64, y: f64, escala: f64) {
        self.primitivas.push(Primitiva::Abrir(Capa::Transformacion(Matriz {
            a: escala,
            b: 0.0,
            c: 0.0,
            d: escala,
            e: x,
            f: y,
        })));
        self.primitivas.extend(colocable.composicion.escena.primitivas);
        self.primitivas.push(Primitiva::Cerrar);
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

    fn texto(&mut self, tramos: &[Tramo], x: f64, ancho: f64, tam: f64, base: Estilo) {
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
            self.linea(&glifos, x, y, &mut formulas);
            alto = alto.max(f64::from(fondo));
        }
        self.y += alto;
    }

    fn linea(&mut self, glifos: &[cosmic_text::LayoutGlyph], x: f64, y: f64, formulas: &mut [Option<Colocable>]) {
        let mut subrayado: Option<(f64, f64, f32)> = None;
        for g in glifos {
            let gx = x + f64::from(g.x + g.x_offset * g.font_size);
            if g.metadata >= BASE_FORMULA {
                if let Some(c) = formulas.get_mut(g.metadata - BASE_FORMULA).and_then(Option::take) {
                    self.colocar(c, gx, y, 1.0);
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
        self.primitivas.push(rectangulo(
            desde,
            y + f64::from(tam) * 0.12,
            hasta - desde,
            grosor,
            color,
        ));
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

    fn tabla(&mut self, filas: &[Vec<Vec<Tramo>>], x: f64, ancho: f64) {
        let columnas = filas.iter().map(Vec::len).max().unwrap_or(0);
        if columnas == 0 {
            return;
        }
        let (tam, color) = (self.formato.tam, self.formato.color);
        let margen = tam * 0.5;
        let hueco = ancho / columnas as f64;
        self.primitivas.push(rectangulo(x, self.y, ancho, REGLA, color));
        for (n, fila) in filas.iter().enumerate() {
            let arriba = self.y + margen * 0.5;
            let mut abajo = arriba;
            for (c, celda) in fila.iter().enumerate() {
                self.y = arriba;
                let negrita = Estilo {
                    negrita: n == 0,
                    ..Estilo::default()
                };
                self.texto(celda, x + hueco * c as f64 + margen, hueco - 2.0 * margen, tam, negrita);
                abajo = abajo.max(self.y);
            }
            self.y = abajo + margen * 0.5;
            let grosor = if n == 0 || n + 1 == filas.len() {
                REGLA
            } else {
                REGLA / 2.0
            };
            self.primitivas.push(rectangulo(x, self.y, ancho, grosor, color));
        }
        self.separar(SEPARACION);
    }
}

fn color_texto(c: ColorTexto) -> Color {
    Color {
        r: f64::from(c.r()) / 255.0,
        g: f64::from(c.g()) / 255.0,
        b: f64::from(c.b()) / 255.0,
    }
}
