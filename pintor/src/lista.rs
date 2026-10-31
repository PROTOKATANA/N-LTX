//! De `Escena` a lo que pinta la GPU: vértices, índices, tintas y una lista de comandos.
//!
//! Todo queda en píxeles físicos del documento: un cambio de escala obliga a compilar de nuevo.
//! Los rellenos y los trazos son triángulos que suaviza el MSAA; los glifos pequeños, máscaras del
//! atlas, porque con el MSAA el texto pequeño pierde demasiada calidad. Un recorte es geometría en
//! el stencil y solo las capas de verdad (grupos, mezclas y máscaras) cuestan una textura aparte.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hasher;

use lyon::math::Point;
use lyon::path::{FillRule, LineCap, LineJoin};
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillTessellator, FillVertex, StrokeOptions, StrokeTessellator, StrokeVertex,
    VertexBuffers,
};
use primitivas::{
    Caja, Capa, Color, Degradado, Escena, GlifoId, Matriz, Pintura, Primitiva, Punto, Relleno, Remate, Segmento, Trazo,
    Union,
};
use zeno::{Command, Format, Mask, Placement, Vector};

use crate::atlas::{Atlas, Clave, Entrada, LADO_ATLAS, MAYOR_GLIFO, Rampas, SUBPIXELES};
use crate::geometria::{Afin, discontinua, ruta};
use crate::puente::ffi::{Comando, Orden, Tinta, Vertice};

/// Desviación máxima al aproximar curvas con segmentos, en píxeles.
const TOLERANCIA: f64 = 0.1;

/// Dibujos discontinuos más finos que esto no se distinguen de una línea continua, en píxeles.
const MENOR_CICLO: f64 = 0.25;

/// Las tintas que caben en la GPU. Las dos primeras son fijas.
pub const MAX_TINTAS: usize = 4096;

const TINTA_SOLIDA: u32 = 0;
const TINTA_GLIFO: u32 = 1;
const TIPO_LINEAL: u32 = 2;
const TIPO_RADIAL: u32 = 3;

const BLANCO: Color = Color { r: 1.0, g: 1.0, b: 1.0 };

/// Los modos de mezcla: su posición más uno es el número que entiende el shader.
const MEZCLAS: [&str; 15] = [
    "multiply",
    "screen",
    "overlay",
    "darken",
    "lighten",
    "color-dodge",
    "color-burn",
    "hard-light",
    "soft-light",
    "difference",
    "exclusion",
    "hue",
    "saturation",
    "color",
    "luminosity",
];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Lista {
    pub vertices: Vec<Vertice>,
    pub indices: Vec<u32>,
    pub tintas: Vec<Tinta>,
    pub comandos: Vec<Comando>,
}

impl Lista {
    /// La lista base de un fotograma: la que aporta las tintas fijas.
    pub fn base() -> Lista {
        let tinta = |tipo| Tinta {
            tipo,
            fila: 0.0,
            radio: 0.0,
            reservado: 0.0,
            a: 0.0,
            b: 0.0,
            c: 0.0,
            d: 0.0,
            e: 0.0,
            f: 0.0,
            p: 0.0,
            q: 0.0,
            r: 0.0,
            s: 0.0,
        };
        Lista {
            tintas: vec![tinta(TINTA_SOLIDA), tinta(TINTA_GLIFO)],
            ..Lista::default()
        }
    }
}

/// Dónde se compila: la lista y las texturas que comparte con las demás listas del lienzo.
pub struct Destino<'a> {
    pub lista: &'a mut Lista,
    pub atlas: &'a mut Atlas,
    pub rampas: &'a mut Rampas,
    /// Cuántas tintas hay ya por delante de las de esta lista.
    pub primera_tinta: u32,
}

/// Compila `escena`, con `raiz` llevándola a píxeles del documento. Con `recortada`, nada se sale
/// de su caja.
pub fn compilar(escena: &Escena, raiz: Afin, recortada: bool, destino: Destino<'_>) {
    let mut c = Compilador {
        d: destino,
        contornos: &escena.contornos,
        preparados: HashMap::new(),
        caja: escena.caja,
        raiz,
        ctm: raiz,
        nivel: 0,
        relleno: FillTessellator::new(),
        trazo: StrokeTessellator::new(),
        malla: VertexBuffers::new(),
    };
    let marco = recortada.then(|| c.abrir_recorte(&rectangulo(escena.caja), false, raiz));
    if let Some(fondo) = escena.fondo {
        c.pintar_fondo(fondo);
    }
    c.primitivas(&escena.primitivas);
    if let Some(Abierta::Recorte(r)) = marco {
        c.nivel -= 1;
        c.comando(Orden::Soltar, r, c.nivel + 1);
    }
}

/// Lo que este pintor no sabe hacer y por eso no se ve igual.
pub fn carencias(escena: &Escena) -> Vec<String> {
    let mut avisos = Vec::new();
    revisar(&escena.primitivas, &mut avisos);
    avisos
}

fn revisar(primitivas: &[Primitiva], avisos: &mut Vec<String>) {
    for p in primitivas {
        match p {
            Primitiva::Imagen { .. } => avisar(avisos, "este pintor no dibuja imágenes"),
            Primitiva::Trazado { relleno: Some(r), .. } => match r.pintura {
                Pintura::Patron { .. } => avisar(avisos, "este pintor aproxima los patrones con su color"),
                Pintura::Mosaico { .. } => avisar(avisos, "este pintor no dibuja mosaicos"),
                _ => {}
            },
            Primitiva::Abrir(Capa::MascaraEscena(dentro)) => revisar(dentro, avisos),
            _ => {}
        }
    }
}

/// Cada carencia se dice una vez por escena.
fn avisar(avisos: &mut Vec<String>, aviso: &str) {
    if !avisos.iter().any(|a| a == aviso) {
        avisos.push(aviso.to_string());
    }
}

fn rectangulo(c: Caja) -> [Segmento; 5] {
    let p = |x, y| Punto { x, y };
    let (x1, y1) = (c.x + c.ancho, c.y + c.alto);
    [
        Segmento::Mover(p(c.x, c.y)),
        Segmento::Linea(p(x1, c.y)),
        Segmento::Linea(p(x1, y1)),
        Segmento::Linea(p(c.x, y1)),
        Segmento::Cerrar,
    ]
}

fn rgba(c: Color, opacidad: f64) -> u32 {
    let canal = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
    canal(c.r) | canal(c.g) << 8 | canal(c.b) << 16 | canal(opacidad) << 24
}

/// Un tramo de `Lista::indices`.
#[derive(Debug, Clone, Copy)]
struct Rango {
    inicio: u32,
    cuenta: u32,
}

/// Lo que hay que deshacer al llegar a su `Primitiva::Cerrar`.
enum Abierta<'p> {
    Nada,
    Transformacion(Afin),
    Recorte(Rango),
    Capa {
        abrir: usize,
        primer_vertice: usize,
        nivel: u32,
        opacidad: f32,
        mezcla: u32,
        mascara: Option<Mascara<'p>>,
    },
}

enum Mascara<'p> {
    Escena(&'p [Primitiva]),
    Degradado(&'p Degradado),
}

/// Lo que hace falta de un contorno para pintarlo.
#[derive(Clone, Copy)]
struct Preparado {
    huella: u64,
    /// Sus límites, en unidades de su fuente.
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}

struct Compilador<'a> {
    d: Destino<'a>,
    contornos: &'a BTreeMap<GlifoId, Vec<Segmento>>,
    preparados: HashMap<GlifoId, Option<Preparado>>,
    caja: Caja,
    raiz: Afin,
    ctm: Afin,
    /// El valor del stencil dentro de los recortes abiertos en esta capa.
    nivel: u32,
    relleno: FillTessellator,
    trazo: StrokeTessellator,
    malla: VertexBuffers<Point, u32>,
}

impl<'a> Compilador<'a> {
    fn primitivas<'p>(&mut self, primitivas: &'p [Primitiva]) {
        let mut pila: Vec<Abierta<'p>> = Vec::new();
        for p in primitivas {
            match p {
                Primitiva::Trazado {
                    segmentos,
                    relleno,
                    trazo,
                } => self.trazado(segmentos, relleno.as_ref(), trazo.as_ref()),
                Primitiva::Glifo { glifo, matriz, color } => self.glifo(*glifo, *matriz, *color),
                // ya avisado por `carencias`
                Primitiva::Imagen { .. } => {}
                Primitiva::Abrir(capa) => {
                    let abierta = self.abrir(capa);
                    pila.push(abierta);
                }
                Primitiva::Cerrar => {
                    if let Some(a) = pila.pop() {
                        self.cerrar(a);
                    }
                }
            }
        }
        while let Some(a) = pila.pop() {
            self.cerrar(a);
        }
    }

    fn abrir<'p>(&mut self, capa: &'p Capa) -> Abierta<'p> {
        match capa {
            Capa::Transformacion(m) => {
                let antes = self.ctm;
                self.ctm = antes.por((*m).into());
                Abierta::Transformacion(antes)
            }
            Capa::Recorte { segmentos, par_impar } => self.abrir_recorte(segmentos, *par_impar, self.ctm),
            // un grupo opaco y normal se ve igual que su contenido
            Capa::Grupo { opacidad } if *opacidad >= 1.0 => Abierta::Nada,
            Capa::Grupo { opacidad } => self.abrir_capa(*opacidad as f32, 0, None),
            Capa::Mezcla(modo) => {
                let mezcla = MEZCLAS.iter().position(|m| m == modo).map_or(0, |i| i as u32 + 1);
                self.abrir_capa(1.0, mezcla, None)
            }
            Capa::MascaraEscena(p) => self.abrir_capa(1.0, 0, Some(Mascara::Escena(p))),
            Capa::MascaraDegradado(d) => self.abrir_capa(1.0, 0, Some(Mascara::Degradado(d))),
            // el MSAA suaviza por igual: solo se pierde la intención de esos bordes
            Capa::BordesNitidos => Abierta::Nada,
        }
    }

    fn cerrar(&mut self, abierta: Abierta<'_>) {
        match abierta {
            Abierta::Nada => {}
            Abierta::Transformacion(antes) => self.ctm = antes,
            Abierta::Recorte(r) => {
                self.nivel -= 1;
                self.comando(Orden::Soltar, r, self.nivel + 1);
            }
            Abierta::Capa {
                abrir,
                primer_vertice,
                nivel,
                opacidad,
                mezcla,
                mascara,
            } => {
                let con_mascara = mascara.is_some();
                if let Some(m) = mascara {
                    self.pintar_mascara(m);
                }
                let salto = self.d.lista.comandos.len() as u32;
                let cierre = Comando {
                    orden: Orden::CerrarCapa,
                    nivel,
                    opacidad,
                    mezcla,
                    con_mascara,
                    ..comando_vacio()
                };
                self.d.lista.comandos.push(cierre);
                let (y0, y1) = alturas(&self.d.lista.vertices[primer_vertice..]);
                let abierto = &mut self.d.lista.comandos[abrir];
                (abierto.salto, abierto.y0, abierto.y1) = (salto, y0, y1);
                self.nivel = nivel;
            }
        }
    }

    fn abrir_recorte<'p>(&mut self, segmentos: &[Segmento], par_impar: bool, m: Afin) -> Abierta<'p> {
        if !self.teselar_relleno(segmentos, par_impar, m) {
            return Abierta::Nada;
        }
        let r = self.volcar(0, TINTA_SOLIDA, None);
        self.comando(Orden::Recortar, r, self.nivel);
        self.nivel += 1;
        Abierta::Recorte(r)
    }

    fn abrir_capa<'p>(&mut self, opacidad: f32, mezcla: u32, mascara: Option<Mascara<'p>>) -> Abierta<'p> {
        let abrir = self.d.lista.comandos.len();
        self.d.lista.comandos.push(Comando {
            orden: Orden::AbrirCapa,
            ..comando_vacio()
        });
        let nivel = self.nivel;
        self.nivel = 0;
        Abierta::Capa {
            abrir,
            primer_vertice: self.d.lista.vertices.len(),
            nivel,
            opacidad,
            mezcla,
            mascara,
        }
    }

    /// Una capa más cuya luminancia dejará pasar a la que se está cerrando.
    fn pintar_mascara(&mut self, mascara: Mascara<'_>) {
        // sin alturas: una máscara nunca se salta sola, la capa que tapa ya se saltó entera
        self.d.lista.comandos.push(Comando {
            orden: Orden::AbrirCapa,
            y0: f32::MIN,
            y1: f32::MAX,
            ..comando_vacio()
        });
        self.nivel = 0;
        match mascara {
            Mascara::Escena(p) => self.primitivas(p),
            Mascara::Degradado(d) => {
                if let Some(tinta) = self.tinta_degradado(d) {
                    let caja = rectangulo(self.caja);
                    if self.teselar_relleno(&caja, false, self.raiz) {
                        let r = self.volcar(rgba(BLANCO, 1.0), tinta, None);
                        self.comando(Orden::Dibujar, r, 0);
                    }
                }
            }
        }
        self.d.lista.comandos.push(Comando {
            orden: Orden::CerrarMascara,
            ..comando_vacio()
        });
    }

    fn comando(&mut self, orden: Orden, r: Rango, nivel: u32) {
        let comandos = &mut self.d.lista.comandos;
        // dibujar seguido es un solo draw call
        if orden == Orden::Dibujar
            && let Some(u) = comandos.last_mut()
            && u.orden == orden
            && u.nivel == nivel
            && u.inicio + u.cuenta == r.inicio
        {
            u.cuenta += r.cuenta;
            return;
        }
        comandos.push(Comando {
            orden,
            inicio: r.inicio,
            cuenta: r.cuenta,
            nivel,
            ..comando_vacio()
        });
    }

    fn pintar_fondo(&mut self, fondo: Color) {
        let caja = rectangulo(self.caja);
        if self.teselar_relleno(&caja, false, self.raiz) {
            let r = self.volcar(rgba(fondo, 1.0), TINTA_SOLIDA, None);
            self.comando(Orden::Dibujar, r, self.nivel);
        }
    }

    fn trazado(&mut self, segmentos: &[Segmento], relleno: Option<&Relleno>, trazo: Option<&Trazo>) {
        // relleno y trazo son una sola cosa: con transparencia, ninguno se ve a través del otro
        let translucido = |o: f64| o < 1.0;
        let cada_uno_una_vez = match (relleno, trazo) {
            (Some(r), Some(t)) => translucido(r.opacidad) || translucido(t.opacidad),
            _ => false,
        };
        if cada_uno_una_vez {
            let mut marcas = Vec::with_capacity(2);
            if let Some(t) = trazo
                && let Some(r) = self.pintar_trazo(segmentos, t)
            {
                self.comando(Orden::Unica, r, self.nivel);
                marcas.push(r);
            }
            if let Some(r) = relleno.and_then(|r| self.pintar_relleno(segmentos, r)) {
                self.comando(Orden::Unica, r, self.nivel);
                marcas.push(r);
            }
            for r in marcas {
                self.comando(Orden::Soltar, r, self.nivel + 1);
            }
            return;
        }
        if let Some(r) = relleno.and_then(|r| self.pintar_relleno(segmentos, r)) {
            self.comando(Orden::Dibujar, r, self.nivel);
        }
        if let Some(t) = trazo
            && let Some(r) = self.pintar_trazo(segmentos, t)
        {
            // la malla de un trazo se solapa en las uniones: con transparencia se notaría
            if t.opacidad * self.alfa_fino(t) < 1.0 {
                self.comando(Orden::Unica, r, self.nivel);
                self.comando(Orden::Soltar, r, self.nivel + 1);
            } else {
                self.comando(Orden::Dibujar, r, self.nivel);
            }
        }
    }

    fn pintar_relleno(&mut self, segmentos: &[Segmento], r: &Relleno) -> Option<Rango> {
        let (color, tinta) = self.tinta(r)?;
        self.teselar_relleno(segmentos, r.par_impar, self.ctm)
            .then(|| self.volcar(color, tinta, None))
    }

    fn pintar_trazo(&mut self, segmentos: &[Segmento], t: &Trazo) -> Option<Rango> {
        let alfa = self.alfa_fino(t);
        self.teselar_trazo(segmentos, t)
            .then(|| self.volcar(rgba(t.color, t.opacidad * alfa), TINTA_SOLIDA, Some(self.ctm)))
    }

    /// Una línea de menos de un píxel se pinta de un píxel y más tenue: con solo unas muestras por
    /// píxel, tal cual saldría a trozos. Ancho cero es la línea más fina posible, igual que en PDF.
    fn alfa_fino(&self, t: &Trazo) -> f64 {
        let px = t.ancho * self.ctm.factor();
        if t.ancho <= 0.0 { 1.0 } else { px.min(1.0) }
    }

    fn tinta(&mut self, r: &Relleno) -> Option<(u32, u32)> {
        match &r.pintura {
            Pintura::Solida(c) => Some((rgba(*c, r.opacidad), TINTA_SOLIDA)),
            Pintura::Patron { color, .. } => Some((rgba(*color, r.opacidad), TINTA_SOLIDA)),
            Pintura::Degradado(d) => Some((rgba(BLANCO, r.opacidad), self.tinta_degradado(d)?)),
            Pintura::Mosaico { .. } => None,
        }
    }

    fn tinta_degradado(&mut self, d: &Degradado) -> Option<u32> {
        let (matriz, paradas) = match d {
            Degradado::Lineal { matriz, paradas, .. } | Degradado::Radial { matriz, paradas, .. } => (matriz, paradas),
        };
        let g = self.ctm.por((*matriz).into()).inversa()?;
        let fila = self.d.rampas.fila(paradas);
        let (tipo, radio, p, q, r, s) = match d {
            Degradado::Lineal { desde, hasta, .. } => {
                let (ex, ey) = (hasta.x - desde.x, hasta.y - desde.y);
                let largo = (ex * ex + ey * ey).max(1e-12);
                (TIPO_LINEAL, 0.0, desde.x, desde.y, ex / largo, ey / largo)
            }
            Degradado::Radial { radio, foco, .. } => (TIPO_RADIAL, *radio, foco.x, foco.y, 0.0, 0.0),
        };
        let tinta = Tinta {
            tipo,
            fila,
            radio: radio as f32,
            reservado: 0.0,
            a: g.a as f32,
            b: g.b as f32,
            c: g.c as f32,
            d: g.d as f32,
            e: g.e as f32,
            f: g.f as f32,
            p: p as f32,
            q: q as f32,
            r: r as f32,
            s: s as f32,
        };
        let tintas = &mut self.d.lista.tintas;
        let primera = self.d.primera_tinta as usize;
        if tintas.last() != Some(&tinta) {
            if primera + tintas.len() >= MAX_TINTAS {
                return None;
            }
            tintas.push(tinta);
        }
        Some((primera + tintas.len() - 1) as u32)
    }

    /// Deja el relleno en `malla`, en píxeles del documento.
    fn teselar_relleno(&mut self, segmentos: &[Segmento], par_impar: bool, m: Afin) -> bool {
        self.malla.clear();
        let Some(r) = ruta(segmentos, m) else {
            return false;
        };
        let regla = if par_impar {
            FillRule::EvenOdd
        } else {
            FillRule::NonZero
        };
        let opciones = FillOptions::tolerance(TOLERANCIA as f32).with_fill_rule(regla);
        let mut salida = BuffersBuilder::new(&mut self.malla, |v: FillVertex| v.position());
        self.relleno.tessellate_path(&r, &opciones, &mut salida).is_ok()
    }

    /// Deja el trazo en `malla`, en el espacio del trazado: la transformación deforma también su
    /// ancho, y por eso se aplica después.
    fn teselar_trazo(&mut self, segmentos: &[Segmento], t: &Trazo) -> bool {
        self.malla.clear();
        let factor = self.ctm.factor();
        if factor <= 0.0 || !factor.is_finite() {
            return false;
        }
        let Some(mut r) = ruta(segmentos, Afin::IDENTIDAD) else {
            return false;
        };
        let tolerancia = (TOLERANCIA / factor).max(1e-6) as f32;
        if let Some(d) = &t.discontinuo
            && d.tramos.iter().sum::<f64>() * factor >= MENOR_CICLO
        {
            r = discontinua(&r, d, tolerancia);
        }
        let ancho = t.ancho.max(1.0 / factor) as f32;
        let remate = match t.remate {
            Remate::Plano => LineCap::Butt,
            Remate::Redondo => LineCap::Round,
            Remate::Cuadrado => LineCap::Square,
        };
        let union = match t.union {
            Union::Inglete => LineJoin::Miter,
            Union::Redonda => LineJoin::Round,
            Union::Biselada => LineJoin::Bevel,
        };
        let opciones = StrokeOptions::tolerance(tolerancia)
            .with_line_width(ancho)
            .with_line_cap(remate)
            .with_line_join(union)
            .with_miter_limit(t.limite_inglete.max(1.0) as f32);
        let mut salida = BuffersBuilder::new(&mut self.malla, |v: StrokeVertex| v.position());
        self.trazo.tessellate_path(&r, &opciones, &mut salida).is_ok()
    }

    /// Pasa `malla` a la lista.
    fn volcar(&mut self, color: u32, tinta: u32, m: Option<Afin>) -> Rango {
        let lista = &mut *self.d.lista;
        let primero = lista.vertices.len() as u32;
        let inicio = lista.indices.len() as u32;
        lista.vertices.extend(self.malla.vertices.iter().map(|p| {
            let (x, y) = match m {
                Some(m) => {
                    let q = m.aplicar(Punto {
                        x: p.x as f64,
                        y: p.y as f64,
                    });
                    (q.x as f32, q.y as f32)
                }
                None => (p.x, p.y),
            };
            Vertice {
                x,
                y,
                u: 0.0,
                v: 0.0,
                rgba: color,
                tinta,
            }
        }));
        lista.indices.extend(self.malla.indices.iter().map(|i| i + primero));
        Rango {
            inicio,
            cuenta: self.malla.indices.len() as u32,
        }
    }

    fn glifo(&mut self, glifo: GlifoId, m: Matriz, color: Color) {
        let Some(contorno) = self.contornos.get(&glifo) else {
            return;
        };
        let Some(p) = *self.preparados.entry(glifo).or_insert_with(|| preparar(contorno)) else {
            return;
        };
        let d = self.ctm.por(m.into());
        let color = rgba(color, 1.0);
        match self.mascara(contorno, p, d) {
            Some((e, x, y)) => self.cuadrado(e, x, y, color),
            None => {
                if self.teselar_relleno(contorno, false, d) {
                    let r = self.volcar(color, TINTA_SOLIDA, None);
                    self.comando(Orden::Dibujar, r, self.nivel);
                }
            }
        }
    }

    /// La máscara del glifo y dónde va su origen, si es lo bastante pequeño y cabe en el atlas.
    fn mascara(&mut self, contorno: &[Segmento], p: Preparado, d: Afin) -> Option<(Entrada, f64, f64)> {
        let esquinas = [(p.x0, p.y0), (p.x1, p.y0), (p.x1, p.y1), (p.x0, p.y1)];
        let cruz = esquinas.map(|(x, y)| (d.a * x + d.c * y, d.b * x + d.d * y));
        let (mut ancho, mut alto) = ((f64::MAX, f64::MIN), (f64::MAX, f64::MIN));
        for (x, y) in cruz {
            ancho = (ancho.0.min(x), ancho.1.max(x));
            alto = (alto.0.min(y), alto.1.max(y));
        }
        if ancho.1 - ancho.0 > MAYOR_GLIFO || alto.1 - alto.0 > MAYOR_GLIFO {
            return None;
        }
        // al más cercano: redondear hacia abajo subiría todo el texto un octavo de píxel de media
        let subpixel = |v: f64| {
            let cuantizado = (v * SUBPIXELES).round() / SUBPIXELES;
            let entero = cuantizado.floor();
            (entero, (cuantizado - entero) * SUBPIXELES)
        };
        let ((x, sx), (y, sy)) = (subpixel(d.e), subpixel(d.f));
        let cuantizar = |v: f64| (v * 1048576.0).round() as i32;
        let clave = Clave {
            contorno: p.huella,
            matriz: [cuantizar(d.a), cuantizar(d.b), cuantizar(d.c), cuantizar(d.d)],
            sx: sx as u8,
            sy: sy as u8,
        };
        if let Some(e) = self.d.atlas.buscar(&clave) {
            return Some((e, x, y));
        }
        let (mascara, sitio) = rasterizar(contorno, d, sx / SUBPIXELES, sy / SUBPIXELES);
        let e = self
            .d
            .atlas
            .insertar(clave, &mascara, sitio.width, sitio.height, sitio.left, sitio.top)?;
        Some((e, x, y))
    }

    fn cuadrado(&mut self, e: Entrada, x: f64, y: f64, color: u32) {
        if e.ancho == 0 || e.alto == 0 {
            return;
        }
        let lista = &mut *self.d.lista;
        let primero = lista.vertices.len() as u32;
        let inicio = lista.indices.len() as u32;
        let (x0, y0) = (x as f32 + e.izquierda as f32, y as f32 + e.arriba as f32);
        let (x1, y1) = (x0 + e.ancho as f32, y0 + e.alto as f32);
        let lado = LADO_ATLAS as f32;
        let (u0, v0) = (e.x as f32 / lado, e.y as f32 / lado);
        let (u1, v1) = ((e.x + e.ancho) as f32 / lado, (e.y + e.alto) as f32 / lado);
        for (x, y, u, v) in [(x0, y0, u0, v0), (x1, y0, u1, v0), (x1, y1, u1, v1), (x0, y1, u0, v1)] {
            lista.vertices.push(Vertice {
                x,
                y,
                u,
                v,
                rgba: color,
                tinta: TINTA_GLIFO,
            });
        }
        lista.indices.extend([0, 1, 2, 0, 2, 3].map(|i: u32| i + primero));
        self.comando(Orden::Dibujar, Rango { inicio, cuenta: 6 }, self.nivel);
    }
}

fn comando_vacio() -> Comando {
    Comando {
        orden: Orden::Dibujar,
        inicio: 0,
        cuenta: 0,
        nivel: 0,
        opacidad: 1.0,
        mezcla: 0,
        con_mascara: false,
        salto: 0,
        y0: 0.0,
        y1: 0.0,
    }
}

fn alturas(vertices: &[Vertice]) -> (f32, f32) {
    vertices
        .iter()
        .fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v.y), b.max(v.y)))
}

/// `None` si el glifo no tiene nada que pintar, como el espacio.
fn preparar(contorno: &[Segmento]) -> Option<Preparado> {
    // las coordenadas enteras solo varían en sus bits altos: hace falta un hash que los mezcle bien
    let mut huella = DefaultHasher::new();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    let mut punto = |p: Punto| {
        huella.write_u64(p.x.to_bits());
        huella.write_u64(p.y.to_bits());
        (x0, y0) = (x0.min(p.x), y0.min(p.y));
        (x1, y1) = (x1.max(p.x), y1.max(p.y));
    };
    for s in contorno {
        match *s {
            Segmento::Mover(p) | Segmento::Linea(p) => punto(p),
            Segmento::Curva(a, b, c) => {
                punto(a);
                punto(b);
                punto(c);
            }
            Segmento::Cerrar => {}
        }
    }
    (x1 >= x0).then_some(Preparado {
        huella: huella.finish(),
        x0,
        y0,
        x1,
        y1,
    })
}

/// La máscara con el origen del glifo en (`sx`, `sy`) dentro de su píxel. zeno mide el área exacta
/// de cada píxel: es lo que hace que el texto pequeño se vea bien.
fn rasterizar(contorno: &[Segmento], d: Afin, sx: f64, sy: f64) -> (Vec<u8>, Placement) {
    let m = Afin { e: sx, f: sy, ..d };
    let v = |p: Punto| {
        let q = m.aplicar(p);
        Vector::new(q.x as f32, q.y as f32)
    };
    let comandos: Vec<Command> = contorno
        .iter()
        .map(|s| match *s {
            Segmento::Mover(p) => Command::MoveTo(v(p)),
            Segmento::Linea(p) => Command::LineTo(v(p)),
            Segmento::Curva(a, b, c) => Command::CurveTo(v(a), v(b), v(c)),
            Segmento::Cerrar => Command::Close,
        })
        .collect();
    Mask::new(&comandos).format(Format::Alpha).render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitivas::{Discontinuo, Parada};

    const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

    fn p(x: f64, y: f64) -> Punto {
        Punto { x, y }
    }

    fn caja(x: f64, y: f64, ancho: f64, alto: f64) -> Caja {
        Caja { x, y, ancho, alto }
    }

    fn cuadrado(lado: f64) -> Vec<Segmento> {
        rectangulo(caja(0.0, 0.0, lado, lado)).to_vec()
    }

    fn solido(opacidad: f64) -> Relleno {
        Relleno {
            pintura: Pintura::Solida(NEGRO),
            opacidad,
            par_impar: false,
        }
    }

    fn relleno(opacidad: f64) -> Primitiva {
        Primitiva::Trazado {
            segmentos: cuadrado(10.0),
            relleno: Some(solido(opacidad)),
            trazo: None,
        }
    }

    fn con_trazo(relleno_o: f64, trazo_o: f64) -> Primitiva {
        Primitiva::Trazado {
            segmentos: cuadrado(10.0),
            relleno: Some(solido(relleno_o)),
            trazo: Some(Trazo {
                opacidad: trazo_o,
                ..Trazo::simple(NEGRO, 2.0)
            }),
        }
    }

    fn glifo(id: u16, matriz: Matriz) -> Primitiva {
        Primitiva::Glifo {
            glifo: GlifoId { fuente: 0, id },
            matriz,
            color: NEGRO,
        }
    }

    struct Compilado {
        lista: Lista,
        atlas: Atlas,
    }

    fn compilada_con(e: &Escena, raiz: Afin) -> Compilado {
        let mut lista = Lista::base();
        let mut atlas = Atlas::default();
        let mut rampas = Rampas::default();
        compilar(
            e,
            raiz,
            false,
            Destino {
                lista: &mut lista,
                atlas: &mut atlas,
                rampas: &mut rampas,
                primera_tinta: 0,
            },
        );
        Compilado { lista, atlas }
    }

    fn compilada(primitivas: Vec<Primitiva>) -> Lista {
        let e = Escena {
            caja: caja(0.0, 0.0, 100.0, 100.0),
            primitivas,
            ..Escena::default()
        };
        compilada_con(&e, Afin::IDENTIDAD).lista
    }

    fn ordenes(l: &Lista) -> Vec<Orden> {
        l.comandos.iter().map(|c| c.orden).collect()
    }

    fn area(l: &Lista, c: &Comando) -> f64 {
        l.indices[c.inicio as usize..(c.inicio + c.cuenta) as usize]
            .chunks_exact(3)
            .map(|t| {
                let v = |i: usize| l.vertices[t[i] as usize];
                let (a, b, c) = (v(0), v(1), v(2));
                ((b.x - a.x) * (c.y - a.y) - (c.x - a.x) * (b.y - a.y)).abs() as f64 / 2.0
            })
            .sum()
    }

    #[test]
    fn un_relleno_cubre_su_area() {
        let l = compilada(vec![relleno(1.0)]);
        assert_eq!(ordenes(&l), [Orden::Dibujar]);
        assert!((area(&l, &l.comandos[0]) - 100.0).abs() < 1e-3);
    }

    #[test]
    fn la_regla_par_impar_vacia_el_centro() {
        let mut s = rectangulo(caja(0.0, 0.0, 10.0, 10.0)).to_vec();
        s.extend(rectangulo(caja(2.0, 2.0, 6.0, 6.0)));
        let con = |par_impar| {
            compilada(vec![Primitiva::Trazado {
                segmentos: s.clone(),
                relleno: Some(Relleno {
                    par_impar,
                    ..solido(1.0)
                }),
                trazo: None,
            }])
        };
        let (a, b) = (con(false), con(true));
        assert!((area(&a, &a.comandos[0]) - 100.0).abs() < 1e-3);
        assert!((area(&b, &b.comandos[0]) - 64.0).abs() < 1e-3);
    }

    #[test]
    fn dibujar_seguido_es_un_solo_comando() {
        let l = compilada(vec![relleno(1.0), relleno(1.0), relleno(1.0)]);
        assert_eq!(ordenes(&l), [Orden::Dibujar]);
        assert!((area(&l, &l.comandos[0]) - 300.0).abs() < 1e-3);
    }

    #[test]
    fn la_escala_llega_a_los_vertices_y_el_ancho_del_trazo_con_ella() {
        let e = Escena {
            primitivas: vec![
                Primitiva::Abrir(Capa::Transformacion(Matriz {
                    a: 3.0,
                    b: 0.0,
                    c: 0.0,
                    d: 3.0,
                    e: 0.0,
                    f: 0.0,
                })),
                Primitiva::Trazado {
                    segmentos: vec![Segmento::Mover(p(0.0, 0.0)), Segmento::Linea(p(10.0, 0.0))],
                    relleno: None,
                    trazo: Some(Trazo::simple(NEGRO, 2.0)),
                },
                Primitiva::Cerrar,
            ],
            ..Escena::default()
        };
        let l = compilada_con(&e, Afin::escala(2.0)).lista;
        // 10 por 3 por 2 de largo y 2 por 3 por 2 de ancho
        assert!((area(&l, &l.comandos[0]) - 60.0 * 12.0).abs() < 1e-2);
    }

    #[test]
    fn un_trazo_discontinuo_cubre_solo_sus_tramos() {
        let l = compilada(vec![Primitiva::Trazado {
            segmentos: vec![Segmento::Mover(p(0.0, 0.0)), Segmento::Linea(p(10.0, 0.0))],
            relleno: None,
            trazo: Some(Trazo {
                discontinuo: Some(Discontinuo {
                    tramos: vec![2.0, 1.0],
                    fase: 0.0,
                }),
                ..Trazo::simple(NEGRO, 1.0)
            }),
        }]);
        assert!((area(&l, &l.comandos[0]) - 7.0).abs() < 1e-3);
    }

    #[test]
    fn una_linea_fina_se_pinta_de_un_pixel_y_mas_tenue() {
        let l = compilada(vec![Primitiva::Trazado {
            segmentos: vec![Segmento::Mover(p(0.0, 0.0)), Segmento::Linea(p(10.0, 0.0))],
            relleno: None,
            trazo: Some(Trazo::simple(NEGRO, 0.25)),
        }]);
        assert_eq!(ordenes(&l), [Orden::Unica, Orden::Soltar]);
        assert!((area(&l, &l.comandos[0]) - 10.0).abs() < 1e-3);
        assert_eq!(l.vertices[0].rgba >> 24, 64);
    }

    #[test]
    fn el_ancho_cero_es_la_linea_mas_fina() {
        let l = compilada(vec![Primitiva::Trazado {
            segmentos: vec![Segmento::Mover(p(0.0, 0.0)), Segmento::Linea(p(10.0, 0.0))],
            relleno: None,
            trazo: Some(Trazo::simple(NEGRO, 0.0)),
        }]);
        assert_eq!(l.vertices[0].rgba >> 24, 255);
        assert!((area(&l, &l.comandos[0]) - 10.0).abs() < 1e-3);
    }

    #[test]
    fn un_inglete_imposible_no_rompe_lyon() {
        let l = compilada(vec![Primitiva::Trazado {
            segmentos: cuadrado(10.0),
            relleno: None,
            trazo: Some(Trazo {
                limite_inglete: 0.0,
                ..Trazo::simple(NEGRO, 1.0)
            }),
        }]);
        assert_eq!(ordenes(&l), [Orden::Dibujar]);
    }

    #[test]
    fn relleno_y_trazo_opacos_se_pintan_uno_sobre_otro() {
        let l = compilada(vec![con_trazo(1.0, 1.0)]);
        assert_eq!(ordenes(&l), [Orden::Dibujar]);
    }

    #[test]
    fn con_transparencia_cada_muestra_se_pinta_una_vez_y_primero_el_trazo() {
        let l = compilada(vec![con_trazo(0.5, 0.5)]);
        assert_eq!(ordenes(&l), [Orden::Unica, Orden::Unica, Orden::Soltar, Orden::Soltar]);
        // el trazo de ancho 2 alrededor de un cuadrado de lado 10: 12² - 8²
        assert!(
            (area(&l, &l.comandos[0]) - 80.0).abs() < 1e-2,
            "{}",
            area(&l, &l.comandos[0])
        );
        assert!((area(&l, &l.comandos[1]) - 100.0).abs() < 1e-3);
        assert_eq!(l.comandos[0].nivel, 0);
        assert_eq!(l.comandos[2].nivel, 1, "borra sus propias marcas");
        assert_eq!(l.comandos[2].cuenta, l.comandos[0].cuenta);
    }

    #[test]
    fn un_recorte_abre_y_cierra_con_la_misma_geometria() {
        let l = compilada(vec![
            Primitiva::Abrir(Capa::Recorte {
                segmentos: cuadrado(5.0),
                par_impar: false,
            }),
            relleno(1.0),
            Primitiva::Cerrar,
            relleno(1.0),
        ]);
        assert_eq!(
            ordenes(&l),
            [Orden::Recortar, Orden::Dibujar, Orden::Soltar, Orden::Dibujar]
        );
        let (r, d, s, f) = (l.comandos[0], l.comandos[1], l.comandos[2], l.comandos[3]);
        assert_eq!((r.nivel, d.nivel, s.nivel, f.nivel), (0, 1, 1, 0));
        assert_eq!((r.inicio, r.cuenta), (s.inicio, s.cuenta));
    }

    #[test]
    fn los_recortes_se_anidan() {
        let abrir = || {
            Primitiva::Abrir(Capa::Recorte {
                segmentos: cuadrado(5.0),
                par_impar: false,
            })
        };
        let l = compilada(vec![
            abrir(),
            abrir(),
            relleno(1.0),
            Primitiva::Cerrar,
            Primitiva::Cerrar,
        ]);
        let niveles: Vec<u32> = l.comandos.iter().map(|c| c.nivel).collect();
        assert_eq!(niveles, [0, 1, 2, 2, 1]);
    }

    #[test]
    fn un_cerrar_de_mas_no_rompe_y_lo_que_queda_abierto_se_cierra() {
        let l = compilada(vec![
            Primitiva::Cerrar,
            Primitiva::Abrir(Capa::Recorte {
                segmentos: cuadrado(5.0),
                par_impar: false,
            }),
        ]);
        assert_eq!(ordenes(&l), [Orden::Recortar, Orden::Soltar]);
    }

    #[test]
    fn un_grupo_opaco_no_gasta_una_capa() {
        let l = compilada(vec![
            Primitiva::Abrir(Capa::Grupo { opacidad: 1.0 }),
            relleno(1.0),
            Primitiva::Cerrar,
        ]);
        assert_eq!(ordenes(&l), [Orden::Dibujar]);
    }

    #[test]
    fn una_capa_empieza_su_stencil_y_vuelve_al_de_fuera() {
        let l = compilada(vec![
            Primitiva::Abrir(Capa::Recorte {
                segmentos: cuadrado(50.0),
                par_impar: false,
            }),
            Primitiva::Abrir(Capa::Grupo { opacidad: 0.5 }),
            relleno(1.0),
            Primitiva::Cerrar,
            relleno(1.0),
            Primitiva::Cerrar,
        ]);
        assert_eq!(
            ordenes(&l),
            [
                Orden::Recortar,
                Orden::AbrirCapa,
                Orden::Dibujar,
                Orden::CerrarCapa,
                Orden::Dibujar,
                Orden::Soltar
            ]
        );
        assert_eq!(l.comandos[2].nivel, 0, "dentro de la capa no hay recortes");
        assert_eq!(l.comandos[3].nivel, 1, "se compone dentro del recorte");
        assert_eq!(l.comandos[3].opacidad, 0.5);
        assert_eq!(l.comandos[4].nivel, 1);
    }

    #[test]
    fn una_capa_sabe_donde_acaba_y_que_alto_ocupa() {
        let l = compilada(vec![
            Primitiva::Abrir(Capa::Grupo { opacidad: 0.5 }),
            relleno(1.0),
            Primitiva::Cerrar,
        ]);
        let a = l.comandos[0];
        assert_eq!(l.comandos[a.salto as usize].orden, Orden::CerrarCapa);
        assert_eq!((a.y0, a.y1), (0.0, 10.0));
    }

    #[test]
    fn una_mezcla_lleva_su_numero() {
        let con = |modo: &str| {
            let l = compilada(vec![Primitiva::Abrir(Capa::Mezcla(modo.into())), Primitiva::Cerrar]);
            l.comandos[1].mezcla
        };
        assert_eq!(con("multiply"), 1);
        assert_eq!(con("luminosity"), 15);
        assert_eq!(con("inventado"), 0);
    }

    #[test]
    fn una_mascara_se_pinta_tras_su_contenido() {
        let l = compilada(vec![
            Primitiva::Abrir(Capa::MascaraEscena(vec![relleno(1.0)])),
            relleno(1.0),
            Primitiva::Cerrar,
        ]);
        assert_eq!(
            ordenes(&l),
            [
                Orden::AbrirCapa,
                Orden::Dibujar,
                Orden::AbrirCapa,
                Orden::Dibujar,
                Orden::CerrarMascara,
                Orden::CerrarCapa
            ]
        );
        assert!(l.comandos[5].con_mascara);
        assert_eq!(l.comandos[0].salto, 5, "saltar la capa salta también su máscara");
    }

    #[test]
    fn la_mascara_de_degradado_cubre_la_escena() {
        let d = Degradado::Lineal {
            desde: p(0.0, 0.0),
            hasta: p(1.0, 0.0),
            matriz: Matriz {
                a: 1.0,
                b: 0.0,
                c: 0.0,
                d: 1.0,
                e: 0.0,
                f: 0.0,
            },
            paradas: vec![
                Parada {
                    posicion: 0.0,
                    color: NEGRO,
                },
                Parada {
                    posicion: 1.0,
                    color: BLANCO,
                },
            ],
        };
        let l = compilada(vec![
            Primitiva::Abrir(Capa::MascaraDegradado(d)),
            relleno(1.0),
            Primitiva::Cerrar,
        ]);
        let dibujo = l.comandos[3];
        assert!((area(&l, &dibujo) - 10000.0).abs() < 1e-2);
        assert_eq!(l.vertices[l.indices[dibujo.inicio as usize] as usize].tinta, 2);
    }

    fn degradado() -> Relleno {
        Relleno {
            pintura: Pintura::Degradado(Degradado::Lineal {
                desde: p(10.0, 0.0),
                hasta: p(14.0, 0.0),
                matriz: Matriz {
                    a: 1.0,
                    b: 0.0,
                    c: 0.0,
                    d: 1.0,
                    e: 0.0,
                    f: 0.0,
                },
                paradas: vec![
                    Parada {
                        posicion: 0.0,
                        color: NEGRO,
                    },
                    Parada {
                        posicion: 1.0,
                        color: BLANCO,
                    },
                ],
            }),
            opacidad: 0.5,
            par_impar: false,
        }
    }

    #[test]
    fn un_degradado_lleva_de_pixeles_a_su_espacio() {
        let e = Escena {
            primitivas: vec![Primitiva::Trazado {
                segmentos: cuadrado(10.0),
                relleno: Some(degradado()),
                trazo: None,
            }],
            ..Escena::default()
        };
        let l = compilada_con(&e, Afin::escala(2.0)).lista;
        let v = l.vertices[0];
        assert_eq!((v.tinta, v.rgba >> 24), (2, 128));
        let t = l.tintas[2];
        assert_eq!(t.tipo, TIPO_LINEAL);
        // el píxel (24, 0) es el punto (12, 0): la mitad del degradado
        let x = t.a * 24.0 + t.e;
        assert_eq!(((x - t.p) * t.r), 0.5);
    }

    #[test]
    fn el_mismo_degradado_reutiliza_su_tinta() {
        let t = || Primitiva::Trazado {
            segmentos: cuadrado(10.0),
            relleno: Some(degradado()),
            trazo: None,
        };
        let l = compilada(vec![t(), t()]);
        assert_eq!(l.tintas.len(), 3);
    }

    #[test]
    fn lo_que_no_es_un_numero_no_se_pinta_ni_rompe() {
        let l = compilada(vec![Primitiva::Trazado {
            segmentos: vec![Segmento::Mover(p(f64::NAN, 0.0)), Segmento::Linea(p(1.0, 1.0))],
            relleno: Some(solido(1.0)),
            trazo: Some(Trazo::simple(NEGRO, 1.0)),
        }]);
        assert!(l.comandos.is_empty());
    }

    /// Un cuadrado de `lado` unidades.
    fn escena_de_glifo(lado: f64, primitivas: Vec<Primitiva>) -> Escena {
        Escena {
            primitivas,
            contornos: BTreeMap::from([
                (
                    GlifoId { fuente: 0, id: 1 },
                    rectangulo(caja(0.0, 0.0, lado, lado)).to_vec(),
                ),
                (GlifoId { fuente: 0, id: 2 }, Vec::new()),
            ]),
            ..Escena::default()
        }
    }

    fn en(x: f64, y: f64) -> Matriz {
        // la y de las fuentes va hacia arriba
        Matriz {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: -1.0,
            e: x,
            f: y,
        }
    }

    #[test]
    fn un_glifo_pequeno_es_un_cuadrado_del_atlas_que_mide_lo_que_el() {
        let e = escena_de_glifo(8.0, vec![glifo(1, en(10.0, 20.0))]);
        let l = compilada_con(&e, Afin::IDENTIDAD).lista;
        assert_eq!(l.vertices.len(), 4);
        let v = l.vertices[0];
        assert_eq!(v.tinta, TINTA_GLIFO);
        // el cuadrado va hacia arriba desde la línea base
        assert_eq!((v.x, v.y), (10.0, 12.0));
        assert_eq!((l.vertices[2].x, l.vertices[2].y), (18.0, 20.0));
    }

    #[test]
    fn la_mascara_del_atlas_es_del_glifo() {
        let e = escena_de_glifo(8.0, vec![glifo(1, en(10.0, 20.0))]);
        let mut c = compilada_con(&e, Afin::IDENTIDAD);
        let s = c.atlas.tomar_sucio();
        assert!(s.bytes.len() >= 8 * LADO_ATLAS as usize);
        let fila = |y: usize| &s.bytes[y * LADO_ATLAS as usize..y * LADO_ATLAS as usize + 8];
        assert!(fila(0).iter().all(|b| *b == 255), "{:?}", fila(0));
        assert!(fila(7).iter().all(|b| *b == 255));
    }

    #[test]
    fn dos_iguales_comparten_mascara_y_otra_posicion_subpixel_no() {
        let e = escena_de_glifo(
            8.0,
            vec![
                glifo(1, en(10.0, 20.0)),
                glifo(1, en(30.0, 20.0)),
                glifo(1, en(50.5, 20.0)),
            ],
        );
        let l = compilada_con(&e, Afin::IDENTIDAD).lista;
        let uv = |i: usize| (l.vertices[i * 4].u, l.vertices[i * 4].v);
        assert_eq!(uv(0), uv(1));
        assert_ne!(uv(0), uv(2));
        assert_eq!(ordenes(&l), [Orden::Dibujar], "en un solo draw call");
    }

    #[test]
    fn un_glifo_con_fraccion_de_pixel_se_rasteriza_desplazado() {
        let e = escena_de_glifo(8.0, vec![glifo(1, en(10.5, 20.0))]);
        let mut c = compilada_con(&e, Afin::IDENTIDAD);
        let s = c.atlas.tomar_sucio();
        let lado = LADO_ATLAS as usize;
        // medio píxel de ancho en cada borde: una columna más
        assert_eq!(c.lista.vertices[1].x - c.lista.vertices[0].x, 9.0);
        assert!((s.bytes[lado] as i32 - 128).abs() <= 1, "{}", s.bytes[lado]);
    }

    #[test]
    fn un_glifo_cae_en_el_cuarto_de_pixel_mas_cercano() {
        // 0,24 está más cerca de 0,25 que de 0: redondear hacia abajo lo subiría casi un cuarto
        let e = escena_de_glifo(8.0, vec![glifo(1, en(10.0, 20.24))]);
        let mut c = compilada_con(&e, Afin::IDENTIDAD);
        let s = c.atlas.tomar_sucio();
        // el borde inferior del cuadrado cae a 0,25 en su última fila, con una fila de más
        let lado = LADO_ATLAS as usize;
        let filas = (c.lista.vertices[2].y - c.lista.vertices[0].y) as usize;
        assert_eq!(filas, 9);
        assert!(
            (s.bytes[(filas - 1) * lado] as i32 - 64).abs() <= 1,
            "{}",
            s.bytes[(filas - 1) * lado]
        );
    }

    #[test]
    fn un_glifo_grande_es_un_trazado() {
        let e = escena_de_glifo(500.0, vec![glifo(1, en(0.0, 500.0))]);
        let l = compilada_con(&e, Afin::IDENTIDAD).lista;
        assert_eq!(l.vertices[0].tinta, TINTA_SOLIDA);
        assert!((area(&l, &l.comandos[0]) - 250000.0).abs() < 1.0);
    }

    #[test]
    fn dos_contornos_de_coordenadas_enteras_no_comparten_huella() {
        let mut vistas = std::collections::HashSet::new();
        for a in 0..300 {
            for b in 0..300 {
                let contorno = [
                    Segmento::Mover(p(a as f64, 0.0)),
                    Segmento::Linea(p(700.0, b as f64)),
                    Segmento::Linea(p(a as f64, 512.0)),
                    Segmento::Cerrar,
                ];
                assert!(vistas.insert(preparar(&contorno).unwrap().huella), "{a} {b}");
            }
        }
    }

    #[test]
    fn un_glifo_sin_contorno_no_pinta_nada() {
        let e = escena_de_glifo(8.0, vec![glifo(2, en(0.0, 0.0)), glifo(3, en(0.0, 0.0))]);
        assert!(compilada_con(&e, Afin::IDENTIDAD).lista.comandos.is_empty());
    }

    #[test]
    fn una_escena_recortada_no_se_sale_de_su_caja() {
        let e = Escena {
            caja: caja(0.0, 0.0, 5.0, 5.0),
            primitivas: vec![relleno(1.0)],
            ..Escena::default()
        };
        let mut lista = Lista::base();
        compilar(
            &e,
            Afin::IDENTIDAD,
            true,
            Destino {
                lista: &mut lista,
                atlas: &mut Atlas::default(),
                rampas: &mut Rampas::default(),
                primera_tinta: 0,
            },
        );
        assert_eq!(ordenes(&lista), [Orden::Recortar, Orden::Dibujar, Orden::Soltar]);
        assert_eq!(lista.comandos[1].nivel, 1);
    }

    #[test]
    fn el_fondo_va_debajo_de_todo() {
        let e = Escena {
            caja: caja(0.0, 0.0, 20.0, 20.0),
            fondo: Some(BLANCO),
            primitivas: vec![relleno(1.0)],
            ..Escena::default()
        };
        let l = compilada_con(&e, Afin::IDENTIDAD).lista;
        assert_eq!(l.vertices[0].rgba, 0xffff_ffff);
        assert!((area(&l, &l.comandos[0]) - 500.0).abs() < 1e-3);
    }

    #[test]
    fn lo_que_falta_se_avisa_una_vez_y_tambien_dentro_de_una_mascara() {
        let patron = Primitiva::Trazado {
            segmentos: cuadrado(1.0),
            relleno: Some(Relleno {
                pintura: Pintura::Patron {
                    nombre: "dots".into(),
                    color: NEGRO,
                    esquina: p(0.0, 0.0),
                },
                opacidad: 1.0,
                par_impar: false,
            }),
            trazo: None,
        };
        let imagen = Primitiva::Imagen {
            png: Vec::new(),
            matriz: en(0.0, 0.0),
        };
        let e = Escena {
            primitivas: vec![
                patron.clone(),
                patron,
                Primitiva::Abrir(Capa::MascaraEscena(vec![imagen])),
                Primitiva::Cerrar,
            ],
            ..Escena::default()
        };
        assert_eq!(carencias(&e).len(), 2, "{:?}", carencias(&e));
    }
}
