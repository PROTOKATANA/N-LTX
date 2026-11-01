//! De `Escena` a `Lista`: recorre las primitivas y deja cada cosa en manos de su pieza.
//!
//! Todo queda en píxeles físicos del documento: un cambio de escala obliga a compilar de nuevo.
//! Los rellenos son triángulos que suaviza el MSAA. Los trazos llevan una franja de un píxel a cada
//! lado y el shader calcula su cobertura exacta con la distancia al centro: con unas pocas muestras
//! por píxel, una línea fina saldría borrosa o a dientes. Los glifos son máscaras del atlas, de área
//! exacta. Un recorte es geometría en el stencil y solo las capas de verdad (grupos, mezclas y
//! máscaras) cuestan una textura aparte.

use std::sync::Arc;

use primitivas::{
    Caja, Capa, Color, Degradado, Escena, GlifoId, MapaDeBits, Matriz, Pintura, Primitiva, Punto, Relleno, Remate,
    Segmento, Trazo, Union,
};

use crate::atlas::Atlas;
use crate::geometria::{Afin, razon_de_inglete, ruta};
use crate::glifos::{Colocado, Glifos};
use crate::lista::{Acabado, FRANJA, Lista, Rango, comando_vacio};
use crate::puente_lienzo::ffi::{Comando, Orden};
use crate::rampas::Rampas;
use crate::teselado::{Pincel, Teselador};
use crate::tintas;

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

/// Dónde se compila: la lista y las texturas que comparte con las demás listas del lienzo.
pub struct Destino<'d> {
    pub lista: &'d mut Lista,
    pub atlas: &'d mut Atlas,
    pub rampas: &'d mut Rampas,
}

/// Dónde cae la escena.
#[derive(Debug, Clone, Copy)]
pub struct Encuadre {
    /// Lleva la escena a píxeles del documento.
    pub raiz: Afin,
    /// Nada se sale de la caja de la escena.
    pub recortada: bool,
}

pub fn compilar(escena: &Escena, encuadre: Encuadre, destino: Destino<'_>) {
    let Encuadre { raiz, recortada } = encuadre;
    let mut c = Compilador {
        d: destino,
        glifos: Glifos::nuevos(&escena.contornos),
        caja: escena.caja,
        raiz,
        ctm: raiz,
        nivel: 0,
        teselador: Teselador::default(),
        lineas_enteras: !recortada,
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

pub(crate) fn rectangulo(c: Caja) -> [Segmento; 5] {
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

/// Basta con no confundir una curva con un codo: no depende de la escala.
const TOLERANCIA_CODOS: f32 = 0.01;

/// Lo agudo que puede ser un codo con bisel o unión redonda sin perder tinta, como razón de inglete.
const CODO_SEGURO: f32 = 3.0;

/// La franja calcula la cobertura interpolando la distancia al centro entre los dos lados del trazo.
/// No sirve con un remate redondo o cuadrado, que crecería con ella. Tampoco en un codo agudo con
/// bisel o unión redonda: en su extremo solo hay vértices del lado de fuera y el trazo no llegaría a
/// su final. Con inglete el centro queda a medio camino de la punta, y sirve mientras no se bisele.
fn con_franja(segmentos: &[Segmento], t: &Trazo) -> bool {
    if t.remate != Remate::Plano {
        return false;
    }
    let Some(r) = ruta(segmentos, Afin::IDENTIDAD) else {
        return false;
    };
    let razon = razon_de_inglete(&r, TOLERANCIA_CODOS);
    razon <= CODO_SEGURO || (t.union == Union::Inglete && f64::from(razon) <= t.limite_inglete)
}

/// La línea base en un píxel entero, si no hay giro: es lo que hace Skia (`SkFont::setBaselineSnap`).
/// Con la y en cuartos de píxel, a escala 1,25 cada línea cae en una fracción distinta y sus trazos
/// horizontales se reparten entre dos filas: en K1 el salto de tinta en la base iba de 10 a 83.
fn en_linea_entera(d: Afin) -> Afin {
    if d.b == 0.0 && d.c == 0.0 {
        Afin { f: d.f.round(), ..d }
    } else {
        d
    }
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
        mascara: Option<Luminancia<'p>>,
    },
}

/// Lo que deja pasar a una capa: su luminancia.
enum Luminancia<'p> {
    Escena(&'p [Primitiva]),
    Degradado(&'p Degradado),
}

struct Compilador<'d, 'e> {
    d: Destino<'d>,
    glifos: Glifos<'e>,
    caja: Caja,
    raiz: Afin,
    ctm: Afin,
    /// El valor del stencil dentro de los recortes abiertos en esta capa.
    nivel: u32,
    teselador: Teselador,
    /// El texto de un documento va a líneas enteras. Lo de una animación no: una etiqueta en
    /// movimiento iría a saltos.
    lineas_enteras: bool,
}

impl Compilador<'_, '_> {
    fn primitivas<'p>(&mut self, primitivas: &'p [Primitiva]) {
        let mut pila: Vec<Abierta<'p>> = Vec::new();
        for p in primitivas {
            match p {
                Primitiva::Trazado {
                    segmentos,
                    relleno,
                    trazo,
                } => self.trazado(segmentos, relleno.as_ref(), trazo.as_ref()),
                Primitiva::Glifo { glifo, matriz, color } => {
                    let d = self.ctm.por((*matriz).into());
                    let suelto = self.lineas_enteras && pila.is_empty();
                    self.glifo(*glifo, if suelto { en_linea_entera(d) } else { d }, *color);
                }
                Primitiva::Imagen { mapa, matriz } => self.imagen(mapa, *matriz),
                Primitiva::Abrir(capa) => {
                    let suelto = self.lineas_enteras && pila.is_empty();
                    let abierta = self.abrir(capa);
                    // una fórmula en línea sigue a su texto; dentro, sus índices y raíces no se tocan
                    if suelto && matches!(capa, Capa::Transformacion(_)) {
                        self.ctm = en_linea_entera(self.ctm);
                    }
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

    fn imagen(&mut self, mapa: &Arc<MapaDeBits>, matriz: Matriz) {
        let m = self.ctm.por(matriz.into());
        let esquina = |x, y| m.aplicar(Punto { x, y });
        let esquinas = [
            esquina(0.0, 0.0),
            esquina(1.0, 0.0),
            esquina(1.0, 1.0),
            esquina(0.0, 1.0),
        ];
        let r = self.d.lista.anadir_imagen(esquinas, mapa);
        let nivel = self.nivel;
        self.d.lista.comandos.push(Comando {
            orden: Orden::Imagen,
            inicio: r.inicio,
            cuenta: r.cuenta,
            nivel,
            imagen: mapa.id,
            ..comando_vacio()
        });
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
            Capa::MascaraEscena(p) => self.abrir_capa(1.0, 0, Some(Luminancia::Escena(p))),
            Capa::MascaraDegradado(d) => self.abrir_capa(1.0, 0, Some(Luminancia::Degradado(d))),
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
                let (y0, y1) = self.d.lista.alturas_desde(primer_vertice);
                let abierto = &mut self.d.lista.comandos[abrir];
                (abierto.salto, abierto.y0, abierto.y1) = (salto, y0, y1);
                self.nivel = nivel;
            }
        }
    }

    fn abrir_recorte<'p>(&mut self, segmentos: &[Segmento], par_impar: bool, m: Afin) -> Abierta<'p> {
        if !self.teselador.rellenar(segmentos, par_impar, m) {
            return Abierta::Nada;
        }
        let r = self.volcar(0, tintas::SOLIDA);
        self.comando(Orden::Recortar, r, self.nivel);
        self.nivel += 1;
        Abierta::Recorte(r)
    }

    fn abrir_capa<'p>(&mut self, opacidad: f32, mezcla: u32, mascara: Option<Luminancia<'p>>) -> Abierta<'p> {
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
    fn pintar_mascara(&mut self, mascara: Luminancia<'_>) {
        // sin alturas: una máscara nunca se salta sola, la capa que tapa ya se saltó entera
        self.d.lista.comandos.push(Comando {
            orden: Orden::AbrirCapa,
            y0: f32::MIN,
            y1: f32::MAX,
            ..comando_vacio()
        });
        self.nivel = 0;
        match mascara {
            Luminancia::Escena(p) => {
                let lineas_enteras = std::mem::replace(&mut self.lineas_enteras, false);
                self.primitivas(p);
                self.lineas_enteras = lineas_enteras;
            }
            Luminancia::Degradado(d) => {
                if let Some(tinta) = self.tinta_degradado(d) {
                    let caja = rectangulo(self.caja);
                    if self.teselador.rellenar(&caja, false, self.raiz) {
                        let r = self.volcar(rgba(BLANCO, 1.0), tinta);
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
        self.d.lista.comando(orden, r, nivel);
    }

    fn pintar_fondo(&mut self, fondo: Color) {
        let caja = rectangulo(self.caja);
        if self.teselador.rellenar(&caja, false, self.raiz) {
            let r = self.volcar(rgba(fondo, 1.0), tintas::SOLIDA);
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
            // sin franja: marcaría en el stencil un píxel de más y el relleno no llegaría al trazo
            if let Some(t) = trazo
                && let Some(r) = self.pintar_trazo(segmentos, t, false)
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
            && let franja = con_franja(segmentos, t)
            && let Some(r) = self.pintar_trazo(segmentos, t, franja)
        {
            // la malla de un trazo se solapa en las uniones: con transparencia se notaría
            let tenue = !franja && self.alfa_fino(t) < 1.0;
            if t.opacidad < 1.0 || tenue {
                self.comando(Orden::Unica, r, self.nivel);
                self.comando(Orden::Soltar, r, self.nivel + 1);
            } else {
                self.comando(Orden::Dibujar, r, self.nivel);
            }
        }
    }

    /// Pasa la malla recién teselada a la lista, sin más transformación.
    fn volcar(&mut self, color: u32, tinta: u32) -> Rango {
        let acabado = Acabado {
            color,
            tinta,
            transformacion: None,
            medio: 0.0,
        };
        self.d.lista.anadir_malla(self.teselador.malla(), acabado)
    }

    fn pintar_relleno(&mut self, segmentos: &[Segmento], r: &Relleno) -> Option<Rango> {
        let (color, tinta) = self.tinta(r)?;
        self.teselador
            .rellenar(segmentos, r.par_impar, self.ctm)
            .then(|| self.volcar(color, tinta))
    }

    fn pintar_trazo(&mut self, segmentos: &[Segmento], t: &Trazo, franja: bool) -> Option<Rango> {
        let factor = self.ctm.factor();
        let px = self.ancho_px(t);
        let (ancho, acabado) = if franja {
            let acabado = Acabado {
                color: rgba(t.color, t.opacidad),
                tinta: tintas::TRAZO,
                transformacion: Some(self.ctm),
                medio: px as f32 / 2.0,
            };
            ((px + 2.0 * FRANJA as f64) / factor, acabado)
        } else {
            let acabado = Acabado {
                color: rgba(t.color, t.opacidad * self.alfa_fino(t)),
                tinta: tintas::SOLIDA,
                transformacion: Some(self.ctm),
                medio: 0.0,
            };
            (t.ancho.max(1.0 / factor), acabado)
        };
        let pincel = Pincel {
            trazo: t,
            ancho,
            factor,
        };
        self.teselador
            .trazar(segmentos, pincel)
            .then(|| self.d.lista.anadir_malla(self.teselador.malla(), acabado))
    }

    /// Ancho cero es la línea más fina posible, igual que en PDF.
    fn ancho_px(&self, t: &Trazo) -> f64 {
        if t.ancho <= 0.0 {
            1.0
        } else {
            t.ancho * self.ctm.factor()
        }
    }

    /// Sin franja, una línea de menos de un píxel se pinta de un píxel y más tenue: con solo unas
    /// muestras por píxel, tal cual saldría a trozos.
    fn alfa_fino(&self, t: &Trazo) -> f64 {
        self.ancho_px(t).min(1.0)
    }

    fn tinta(&mut self, r: &Relleno) -> Option<(u32, u32)> {
        match &r.pintura {
            Pintura::Solida(c) => Some((rgba(*c, r.opacidad), tintas::SOLIDA)),
            Pintura::Patron { color, .. } => Some((rgba(*color, r.opacidad), tintas::SOLIDA)),
            Pintura::Degradado(d) => Some((rgba(BLANCO, r.opacidad), self.tinta_degradado(d)?)),
            Pintura::Mosaico { .. } => None,
        }
    }

    fn tinta_degradado(&mut self, d: &Degradado) -> Option<u32> {
        let tinta = tintas::de_degradado(d, self.ctm, self.d.rampas)?;
        self.d.lista.anadir_tinta(tinta)
    }

    /// `d` lleva el contorno a píxeles del documento.
    fn glifo(&mut self, glifo: GlifoId, d: Afin, color: Color) {
        let color = rgba(color, 1.0);
        match self.glifos.colocar(glifo, d, self.d.atlas) {
            Colocado::Nada => {}
            Colocado::EnAtlas { entrada, x, y } => {
                if let Some(r) = self.d.lista.anadir_cuadrado(entrada, (x, y), color) {
                    self.comando(Orden::Dibujar, r, self.nivel);
                }
            }
            Colocado::Contorno(contorno) => {
                if self.teselador.rellenar(contorno, false, d) {
                    let r = self.volcar(color, tintas::SOLIDA);
                    self.comando(Orden::Dibujar, r, self.nivel);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::LADO_ATLAS;
    use crate::puente_lienzo::ffi::TipoTinta;
    use primitivas::{Discontinuo, Parada};
    use std::collections::BTreeMap;

    const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

    fn p(x: f64, y: f64) -> Punto {
        Punto { x, y }
    }

    fn mapa(id: u64) -> Arc<MapaDeBits> {
        Arc::new(MapaDeBits {
            id,
            ancho: 1,
            alto: 1,
            rgba: vec![255; 4],
        })
    }

    #[test]
    fn una_imagen_es_un_cuadrado_con_su_textura_y_no_se_repite() {
        let matriz = Matriz {
            a: 10.0,
            b: 0.0,
            c: 0.0,
            d: 20.0,
            e: 3.0,
            f: 4.0,
        };
        let imagen = Primitiva::Imagen { mapa: mapa(9), matriz };
        let escena = Escena {
            primitivas: vec![imagen.clone(), imagen],
            ..Escena::default()
        };
        let (mut lista, mut atlas, mut rampas) = (Lista::base(), Atlas::default(), Rampas::default());
        let destino = Destino {
            lista: &mut lista,
            atlas: &mut atlas,
            rampas: &mut rampas,
        };
        let encuadre = Encuadre {
            raiz: Afin::escala(2.0),
            recortada: false,
        };
        compilar(&escena, encuadre, destino);
        assert_eq!(lista.imagenes.len(), 1);
        assert_eq!(lista.comandos.len(), 2);
        assert!(lista.comandos.iter().all(|c| c.orden == Orden::Imagen && c.imagen == 9));
        let v = &lista.vertices[..4];
        assert_eq!((v[0].x, v[0].y, v[0].u, v[0].v), (6.0, 8.0, 0.0, 0.0));
        assert_eq!((v[2].x, v[2].y, v[2].u, v[2].v), (26.0, 48.0, 1.0, 1.0));
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

    /// Arriba y alto de cada máscara dibujada. Un cuadrado de 5 px en una fila entera ocupa 5 filas;
    /// con la y en fracciones, 6: es el borrón que se ve en sus lados.
    fn cuadrados(c: &Compilado) -> Vec<(f32, f32)> {
        let q = c.lista.vertices.chunks(4);
        q.map(|q| (q[0].y, q[2].y - q[0].y)).collect()
    }

    fn compilado(e: &Escena, raiz: Afin, recortada: bool) -> Compilado {
        let (mut lista, mut atlas, mut rampas) = (Lista::base(), Atlas::default(), Rampas::default());
        let destino = Destino {
            lista: &mut lista,
            atlas: &mut atlas,
            rampas: &mut rampas,
        };
        compilar(e, Encuadre { raiz, recortada }, destino);
        Compilado { lista, atlas }
    }

    fn escena_de_glifos(primitivas: Vec<Primitiva>) -> Escena {
        Escena {
            caja: Caja {
                x: 0.0,
                y: 0.0,
                ancho: 100.0,
                alto: 100.0,
            },
            primitivas,
            contornos: BTreeMap::from([(GlifoId { fuente: 0, id: 1 }, cuadrado_de(4.0))]),
            ..Escena::default()
        }
    }

    fn cuadrado_de(lado: f64) -> Vec<Segmento> {
        let p = |x, y| Punto { x, y };
        vec![
            Segmento::Mover(p(0.0, 0.0)),
            Segmento::Linea(p(lado, 0.0)),
            Segmento::Linea(p(lado, lado)),
            Segmento::Linea(p(0.0, lado)),
            Segmento::Cerrar,
        ]
    }

    fn llevado(x: f64, y: f64) -> Matriz {
        Matriz {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            e: x,
            f: y,
        }
    }

    /// RAYO: si falla, el texto vuelve a verse borroso en unas líneas sí y en otras no.
    #[test]
    fn a_una_escala_cualquiera_cada_linea_cae_en_filas_enteras() {
        let e = escena_de_glifos(
            (0..8)
                .map(|n| glifo(1, llevado(0.0, 10.0 + f64::from(n) * 0.2)))
                .collect(),
        );
        let c = compilado(&e, Afin::escala(1.25), false);
        assert_eq!(c.lista.vertices.len(), 8 * 4);
        assert!(cuadrados(&c).iter().all(|q| q.1 == 5.0), "{:?}", cuadrados(&c));
    }

    #[test]
    fn la_formula_en_linea_cae_en_la_fila_de_su_texto_y_dentro_no_se_toca() {
        let y = 10.2;
        let formula = vec![
            Primitiva::Abrir(Capa::Transformacion(llevado(0.0, y))),
            glifo(1, llevado(0.0, 0.0)),
            glifo(1, llevado(0.0, 0.2)),
            Primitiva::Cerrar,
        ];
        let e = escena_de_glifos([vec![glifo(1, llevado(0.0, y))], formula].concat());
        let hechos = cuadrados(&compilado(&e, Afin::escala(1.25), false));
        assert_eq!(hechos[0], hechos[1], "misma línea");
        assert_eq!(hechos[2].1, 6.0, "el índice conserva su fracción");
    }

    #[test]
    fn lo_animado_y_lo_girado_conservan_su_sitio_exacto() {
        let e = escena_de_glifos(vec![glifo(1, llevado(0.0, 10.2))]);
        let hechos = cuadrados(&compilado(&e, Afin::escala(1.25), true));
        assert_eq!(hechos.last().map(|q| q.1), Some(6.0), "los anteriores son su marco");
        let giro = Afin {
            b: 0.3,
            c: -0.3,
            f: 1.3,
            ..Afin::escala(1.25)
        };
        assert_eq!(en_linea_entera(giro).f, 1.3);
    }

    struct Compilado {
        lista: Lista,
        atlas: Atlas,
    }

    fn compilada_con(e: &Escena, raiz: Afin) -> Compilado {
        let mut lista = Lista::base();
        let mut atlas = Atlas::default();
        let mut rampas = Rampas::default();
        let encuadre = Encuadre { raiz, recortada: false };
        let destino = Destino {
            lista: &mut lista,
            atlas: &mut atlas,
            rampas: &mut rampas,
        };
        compilar(e, encuadre, destino);
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
        // 10 por 3 por 2 de largo y 2 por 3 por 2 de ancho, más la franja de cada lado
        assert!(
            (area(&l, &l.comandos[0]) - 60.0 * 14.0).abs() < 1e-2,
            "{}",
            area(&l, &l.comandos[0])
        );
        assert_eq!(l.vertices[0].v, 6.0, "medio ancho en píxeles");
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
        // 7 de largo por 1 de ancho y la franja de cada lado
        assert!((area(&l, &l.comandos[0]) - 21.0).abs() < 1e-3);
    }

    fn linea(trazo: Trazo) -> Lista {
        compilada(vec![Primitiva::Trazado {
            segmentos: vec![Segmento::Mover(p(0.0, 0.0)), Segmento::Linea(p(10.0, 0.0))],
            relleno: None,
            trazo: Some(trazo),
        }])
    }

    #[test]
    fn un_trazo_lleva_su_distancia_al_centro_y_su_medio_ancho() {
        let l = linea(Trazo::simple(NEGRO, 3.0));
        assert_eq!(ordenes(&l), [Orden::Dibujar]);
        for v in &l.vertices {
            assert_eq!(v.tinta, tintas::TRAZO);
            assert_eq!(v.u.abs(), 1.5 + FRANJA, "{v:?}");
            assert_eq!(v.v, 1.5);
            // la malla llega justo hasta donde dice la distancia
            assert_eq!(v.y.abs(), v.u.abs());
        }
    }

    #[test]
    fn una_linea_fina_no_pierde_tinta_ni_se_pinta_de_un_pixel() {
        let l = linea(Trazo::simple(NEGRO, 0.25));
        assert_eq!(ordenes(&l), [Orden::Dibujar], "opaca: no necesita marcar nada");
        assert_eq!(
            l.vertices[0].rgba >> 24,
            255,
            "la cobertura sale de la distancia, no del color"
        );
        assert_eq!(l.vertices[0].v, 0.125);
    }

    #[test]
    fn el_ancho_cero_es_la_linea_mas_fina() {
        let l = linea(Trazo::simple(NEGRO, 0.0));
        assert_eq!(l.vertices[0].v, 0.5, "un píxel de ancho");
        assert_eq!(l.vertices[0].rgba >> 24, 255);
    }

    #[test]
    fn un_trazo_translucido_se_pinta_una_vez_por_muestra() {
        let l = linea(Trazo {
            opacidad: 0.5,
            ..Trazo::simple(NEGRO, 3.0)
        });
        assert_eq!(ordenes(&l), [Orden::Unica, Orden::Soltar]);
        assert_eq!(l.vertices[0].tinta, tintas::TRAZO);
    }

    #[test]
    fn un_remate_que_no_es_plano_no_lleva_franja() {
        for remate in [Remate::Redondo, Remate::Cuadrado] {
            let l = linea(Trazo {
                remate,
                ..Trazo::simple(NEGRO, 0.25)
            });
            assert_eq!(l.vertices[0].tinta, tintas::SOLIDA, "{remate:?}");
            assert_eq!(l.vertices[0].rgba >> 24, 64, "un píxel de ancho y más tenue");
            assert_eq!(ordenes(&l), [Orden::Unica, Orden::Soltar]);
        }
    }

    /// Un codo de `grados` de ángulo interior.
    fn codo(grados: f64, union: Union) -> Lista {
        let a = grados.to_radians();
        compilada(vec![Primitiva::Trazado {
            segmentos: vec![
                Segmento::Mover(p(100.0, 0.0)),
                Segmento::Linea(p(0.0, 0.0)),
                Segmento::Linea(p(100.0 * a.cos(), 100.0 * a.sin())),
            ],
            relleno: None,
            trazo: Some(Trazo {
                union,
                limite_inglete: 10.0,
                ..Trazo::simple(NEGRO, 3.0)
            }),
        }])
    }

    #[test]
    fn un_codo_abierto_lleva_franja_con_cualquier_union() {
        for union in [Union::Inglete, Union::Redonda, Union::Biselada] {
            assert_eq!(codo(90.0, union).vertices[0].tinta, tintas::TRAZO, "{union:?}");
        }
    }

    #[test]
    fn un_codo_agudo_con_bisel_o_redondo_no_lleva_franja_o_se_quedaria_corto() {
        for union in [Union::Redonda, Union::Biselada] {
            assert_eq!(codo(15.0, union).vertices[0].tinta, tintas::SOLIDA, "{union:?}");
        }
    }

    #[test]
    fn un_inglete_agudo_lleva_franja_mientras_no_pase_a_bisel() {
        assert_eq!(codo(15.0, Union::Inglete).vertices[0].tinta, tintas::TRAZO, "razón 7,7");
        assert_eq!(
            codo(8.5, Union::Inglete).vertices[0].tinta,
            tintas::SOLIDA,
            "razón 13,5"
        );
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
        assert_eq!(l.vertices[l.indices[dibujo.inicio as usize] as usize].tinta, 3);
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
        assert_eq!((v.tinta, v.rgba >> 24), (3, 128));
        let t = l.tintas[3];
        assert_eq!(t.tipo, TipoTinta::Lineal);
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
        assert_eq!(l.tintas.len(), 4);
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
        assert_eq!(v.tinta, tintas::GLIFO);
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
        // dentro de un grupo, como el índice de una fórmula: el texto suelto va a filas enteras.
        // 0,24 está más cerca de 0,25 que de 0: redondear hacia abajo lo subiría casi un cuarto
        let grupo = Primitiva::Abrir(Capa::Transformacion(llevado(0.0, 0.0)));
        let e = escena_de_glifo(8.0, vec![grupo, glifo(1, en(10.0, 20.24)), Primitiva::Cerrar]);
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
    fn un_glifo_alto_sigue_en_el_atlas_y_en_su_cuarto_de_pixel() {
        // como una raíz alta: al píxel entero su trazo fino cambiaría de forma
        let e = escena_de_glifo(100.0, vec![glifo(1, en(10.25, 150.0)), glifo(1, en(200.0, 150.0))]);
        let l = compilada_con(&e, Afin::IDENTIDAD).lista;
        assert_eq!(l.vertices[0].tinta, tintas::GLIFO);
        let ancho = |i: usize| l.vertices[i * 4 + 1].x - l.vertices[i * 4].x;
        assert_eq!(
            (ancho(0), ancho(1)),
            (101.0, 100.0),
            "el desplazado ocupa una columna más"
        );
    }

    #[test]
    fn un_glifo_grande_es_un_trazado() {
        let e = escena_de_glifo(500.0, vec![glifo(1, en(0.0, 500.0))]);
        let l = compilada_con(&e, Afin::IDENTIDAD).lista;
        assert_eq!(l.vertices[0].tinta, tintas::SOLIDA);
        assert!((area(&l, &l.comandos[0]) - 250000.0).abs() < 1.0);
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
        let encuadre = Encuadre {
            raiz: Afin::IDENTIDAD,
            recortada: true,
        };
        let destino = Destino {
            lista: &mut lista,
            atlas: &mut Atlas::default(),
            rampas: &mut Rampas::default(),
        };
        compilar(&e, encuadre, destino);
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
}
