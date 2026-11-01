//! Coloca una tabla: reparte las columnas, mide las filas y pinta cada celda. No sabe de dónde
//! vienen sus colores: se los da el estilo, ya resuelto.

use std::collections::HashSet;

use cosmic_text::Align;
use primitivas::{Caja, Color, Punto};

use super::texto::{Escrito, Hueco, Letra};
use super::{Columna, Maquetador, SEPARACION, rectangulo};
use crate::lector::{Alineado, Celda, Estilo, Tabla, Tramo};
use crate::tema::{Ajuste, EstiloTabla, Reparto};

const BLANCO: Color = Color::BLANCO;
/// En una cabecera, el blanco vale mientras contraste al menos esto: un título se lee por su peso.
const CONTRASTE_CABECERA: f64 = 2.0;
/// El margen vertical de una celda frente al horizontal.
const RELLENO_VERTICAL: f64 = 0.55;
/// Lo que se da de más al ancho de una celda, en bp: lo que cabe justo no debe partirse por un redondeo.
const HOLGURA: f64 = 0.5;
/// Hasta dónde se siguen los estilos que heredan unos de otros.
const MAX_HERENCIA: usize = 8;

/// Una celda en su sitio. Lo que tapa un `\multirow` no llega aquí.
struct Sitio<'t> {
    celda: &'t Celda,
    fila: usize,
    columna: usize,
}

struct Rejilla<'t> {
    tabla: &'t Tabla,
    sitios: Vec<Sitio<'t>>,
    filas: usize,
    columnas: usize,
}

impl<'t> Rejilla<'t> {
    fn de(tabla: &'t Tabla) -> Rejilla<'t> {
        let mut sitios = Vec::new();
        let mut tapadas = HashSet::new();
        let mut columnas = tabla.columnas.len();
        for (fila, f) in tabla.filas.iter().enumerate() {
            let mut columna = 0;
            for celda in &f.celdas {
                if !tapadas.contains(&(fila, columna)) {
                    for abajo in 1..celda.alto {
                        tapadas.extend((0..celda.ancho).map(|c| (fila + abajo, columna + c)));
                    }
                    sitios.push(Sitio { celda, fila, columna });
                }
                columna += celda.ancho;
            }
            columnas = columnas.max(columna);
        }
        Rejilla {
            tabla,
            sitios,
            filas: tabla.filas.len(),
            columnas,
        }
    }
}

/// Los colores que usa una tabla, ya resueltos.
struct Paleta {
    cabecera: Vec<Color>,
    texto_cabecera: Option<Color>,
    filas: Vec<Color>,
    texto: Color,
    /// El texto de lo que no lleva fondo.
    sobre_pagina: Color,
    reglas: Option<Color>,
}

/// Cómo se ve una tabla.
#[derive(Clone, Copy)]
struct Aspecto<'a> {
    estilo: &'a EstiloTabla,
    paleta: &'a Paleta,
}

/// Lo que hace falta para escribir las celdas: su aspecto y a qué medidas.
struct Trazado<'a> {
    aspecto: Aspecto<'a>,
    anchos: &'a [f64],
    tam: f64,
    hueco: f64,
}

/// El reparto de una tabla: dónde empieza cada columna y fila y cuánto mide.
struct Medidas {
    anchos: Vec<f64>,
    altos: Vec<f64>,
    hueco: f64,
    origen: Punto,
}

impl Medidas {
    fn caja(&self, s: &Sitio<'_>) -> Caja {
        let tramo = |medidas: &[f64], desde: usize, cuantas: usize| {
            let hasta = (desde + cuantas).min(medidas.len());
            let dentro = medidas[desde.min(hasta)..hasta].iter().sum::<f64>();
            dentro + self.hueco * cuantas.saturating_sub(1) as f64
        };
        let antes =
            |medidas: &[f64], n: usize| medidas[..n.min(medidas.len())].iter().sum::<f64>() + self.hueco * n as f64;
        Caja {
            x: self.origen.x + antes(&self.anchos, s.columna),
            y: self.origen.y + antes(&self.altos, s.fila),
            ancho: tramo(&self.anchos, s.columna, s.celda.ancho),
            alto: tramo(&self.altos, s.fila, s.celda.alto),
        }
    }

    fn alto_total(&self) -> f64 {
        self.altos.iter().sum::<f64>() + self.hueco * self.altos.len().saturating_sub(1) as f64
    }

    fn ancho_total(&self) -> f64 {
        self.anchos.iter().sum::<f64>() + self.hueco * self.anchos.len().saturating_sub(1) as f64
    }
}

/// El color a la altura `t` de una lista repartida entre 0 y 1.
fn interpolar(colores: &[Color], t: f64) -> Option<Color> {
    let ultimo = colores.len().checked_sub(1)?;
    let posicion = t.clamp(0.0, 1.0) * ultimo as f64;
    let i = (posicion.floor() as usize).min(ultimo);
    let (a, b) = (colores[i], colores[(i + 1).min(ultimo)]);
    let u = posicion - i as f64;
    let entre = |x: f64, y: f64| x + (y - x) * u;
    Some(Color {
        r: entre(a.r, b.r),
        g: entre(a.g, b.g),
        b: entre(a.b, b.b),
    })
}

fn vacia(celda: &Celda) -> bool {
    celda
        .tramos
        .iter()
        .all(|t| matches!(t, Tramo::Texto(texto, _) if texto.trim().is_empty()))
}

fn alineacion(a: Alineado) -> Align {
    match a {
        Alineado::Izquierda => Align::Left,
        Alineado::Centro => Align::Center,
        Alineado::Derecha => Align::Right,
    }
}

/// Lo que pide cada columna.
struct Anchos {
    /// Sin partir nada.
    naturales: Vec<f64>,
    /// Lo que no se puede partir: una fórmula, una palabra. Nada se sale de su celda.
    minimos: Vec<f64>,
}

/// Recorta los anchos que pasan de lo disponible, empezando por los mayores y sin bajar de su
/// mínimo: una columna estrecha de números no debe partirse porque otra de texto sea larga.
fn encajar(anchos: &mut [f64], minimos: &[f64], disponible: f64) {
    let con_tope = |tope: f64, a: f64, m: f64| a.min(tope).max(m);
    let total = |tope: f64| {
        anchos
            .iter()
            .zip(minimos)
            .map(|(a, m)| con_tope(tope, *a, *m))
            .sum::<f64>()
    };
    let (mut bajo, mut alto) = (0.0, anchos.iter().copied().fold(0.0, f64::max));
    if total(alto) <= disponible {
        return;
    }
    for _ in 0..48 {
        let medio = (bajo + alto) / 2.0;
        if total(medio) > disponible {
            alto = medio;
        } else {
            bajo = medio;
        }
    }
    for (a, m) in anchos.iter_mut().zip(minimos) {
        *a = con_tope(bajo, *a, *m);
    }
}

impl Maquetador<'_> {
    pub(super) fn tabla(&mut self, tabla: &Tabla, c: Columna) {
        let estilo = self.estilo_de(tabla);
        let rejilla = Rejilla::de(tabla);
        if rejilla.columnas == 0 {
            return;
        }
        let paleta = self.paleta_de(&estilo);
        let (anchos, escala) = self.repartir(&rejilla, &estilo, c.ancho);
        let tam = self.formato.tam * escala;
        let hueco = estilo.separacion * escala;
        let aspecto = Aspecto {
            estilo: &estilo,
            paleta: &paleta,
        };
        let trazado = Trazado {
            aspecto,
            anchos: &anchos,
            tam,
            hueco,
        };
        let escritos = self.escribir_celdas(&rejilla, &trazado);
        let altos = altos_de_filas(&rejilla, &escritos, &trazado);
        let mut medidas = Medidas {
            anchos,
            altos,
            hueco,
            origen: Punto { x: c.x, y: self.y },
        };
        medidas.origen.x += ((c.ancho - medidas.ancho_total()) / 2.0).max(0.0);
        self.pintar_fondos(&rejilla, aspecto, &medidas);
        for (sitio, escrito) in rejilla.sitios.iter().zip(escritos) {
            let caja = medidas.caja(sitio);
            let margen = estilo.relleno * tam;
            let y = caja.y + (caja.alto - escrito.alto) / 2.0;
            self.poner(escrito, Punto { x: caja.x + margen, y });
        }
        self.y += medidas.alto_total();
        self.separar(SEPARACION);
    }

    /// Tema, estilos con nombre y opciones de la tabla, en ese orden.
    fn estilo_de(&mut self, tabla: &Tabla) -> EstiloTabla {
        let mut estilo = self.formato.tema.tabla.clone();
        self.aplicar_opciones(&mut estilo, &tabla.opciones, 0);
        estilo
    }

    fn aplicar_opciones(&mut self, estilo: &mut EstiloTabla, opciones: &[(String, String)], vueltas: usize) {
        for (clave, valor) in opciones {
            if clave == "estilo" {
                let base = self
                    .estilos
                    .iter()
                    .rev()
                    .find(|(n, _)| n == valor)
                    .map(|(_, o)| o.clone());
                match base {
                    Some(base) if vueltas < MAX_HERENCIA => self.aplicar_opciones(estilo, &base, vueltas + 1),
                    Some(_) => self
                        .avisos
                        .push(format!("tabla: el estilo {valor} se hereda a sí mismo")),
                    None => self.avisos.push(format!("tabla: estilo desconocido: {valor}")),
                }
            } else if !(valor.is_empty() && clave.len() == 1) {
                // una sola letra es la posición de LaTeX: `[t]`
                if let Err(aviso) = estilo.aplicar(clave, valor) {
                    self.avisos.push(aviso);
                }
            }
        }
    }

    fn color_de(&mut self, nombre: &str) -> Option<Color> {
        let color = self.piezas.colores.color(nombre, &self.definidos);
        if color.is_none() {
            self.avisos.push(format!("tabla: color desconocido: {nombre}"));
        }
        color
    }

    fn paleta_de(&mut self, e: &EstiloTabla) -> Paleta {
        let lista = |m: &mut Self, nombres: &[String]| nombres.iter().filter_map(|n| m.color_de(n)).collect();
        let tinta = self.formato.color;
        let texto = self.color_de(&e.texto).unwrap_or(tinta);
        // el gris del tema solo se lee sobre una página clara; un color pedido se respeta
        let del_tema = e.texto == self.formato.tema.tabla.texto;
        let clara = tinta.contraste(Color::BLANCO) < tinta.contraste(Color::NEGRO);
        Paleta {
            cabecera: lista(self, &e.cabecera),
            texto_cabecera: e.texto_cabecera.as_deref().and_then(|n| self.color_de(n)),
            filas: lista(self, &e.filas),
            texto,
            sobre_pagina: if del_tema && clara { tinta } else { texto },
            reglas: e.reglas.as_deref().and_then(|n| self.color_de(n)),
        }
    }

    /// Lo que pide cada columna. Una celda de varias columnas reparte lo que le falte entre ellas.
    fn anchos_de_contenido(&mut self, r: &Rejilla<'_>, e: &EstiloTabla) -> Anchos {
        let tam = self.formato.tam;
        let margen = 2.0 * e.relleno * tam;
        let mut naturales = vec![margen; r.columnas];
        let mut minimos = vec![margen; r.columnas];
        let mut por_cubrir = Vec::new();
        for s in &r.sitios {
            let base = Estilo {
                negrita: s.fila == 0 && !e.cabecera.is_empty(),
                ..Estilo::default()
            };
            let letra = Letra { tam, base };
            let minimo = self.escribir(&s.celda.tramos, Hueco::minimo(), letra).ancho + margen;
            if s.celda.ancho > 1 {
                por_cubrir.push((s.columna, s.celda.ancho, minimo));
                continue;
            }
            let natural = self.escribir(&s.celda.tramos, Hueco::libre(), letra).ancho + margen;
            naturales[s.columna] = naturales[s.columna].max(natural);
            minimos[s.columna] = minimos[s.columna].max(minimo);
        }
        for (columna, cuantas, minimo) in por_cubrir {
            let hasta = (columna + cuantas).min(r.columnas);
            let tienen = minimos[columna..hasta].iter().sum::<f64>() + e.separacion * (hasta - columna - 1) as f64;
            let falta = ((minimo - tienen) / (hasta - columna) as f64).max(0.0);
            minimos[columna..hasta].iter_mut().for_each(|m| *m += falta);
        }
        for (n, m) in naturales.iter_mut().zip(&minimos) {
            *n = n.max(*m);
        }
        Anchos { naturales, minimos }
    }

    /// El ancho de cada columna y lo que se reduce la tabla entera, si se reduce.
    fn repartir(&mut self, r: &Rejilla<'_>, e: &EstiloTabla, disponible: f64) -> (Vec<f64>, f64) {
        let Anchos { mut naturales, minimos } = self.anchos_de_contenido(r, e);
        if e.reparto == Reparto::Iguales {
            let mayor = naturales.iter().copied().fold(0.0, f64::max);
            naturales.fill(mayor);
        }
        let huecos = e.separacion * (r.columnas - 1) as f64;
        let sitio = disponible - huecos;
        // si ni siquiera cabe lo que no se puede partir, solo queda reducir la tabla
        let irreducibles = if e.ajuste == Ajuste::Escalar {
            &naturales
        } else {
            &minimos
        };
        let necesario = irreducibles.iter().sum::<f64>();
        if necesario > sitio {
            let escala = (disponible / (necesario + huecos)).max(0.1);
            return (irreducibles.iter().map(|a| a * escala).collect(), escala);
        }
        encajar(&mut naturales, &minimos, sitio);
        if e.completa {
            let actual = naturales.iter().sum::<f64>();
            if actual > 0.0 && actual < sitio {
                naturales.iter_mut().for_each(|a| *a *= sitio / actual);
            }
        }
        (naturales, 1.0)
    }

    fn fondo_de(&self, r: &Rejilla<'_>, s: &Sitio<'_>, a: Aspecto<'_>) -> Option<Color> {
        let (t, e, p) = (r.tabla, a.estilo, a.paleta);
        let propio = s.celda.fondo.as_deref().or(t.filas[s.fila].fondo.as_deref());
        if let Some(c) = propio.and_then(|n| self.piezas.colores.color(n, &self.definidos)) {
            return Some(c);
        }
        if vacia(s.celda) && !e.vacias {
            return None;
        }
        if s.fila == 0 && !p.cabecera.is_empty() {
            let columnas = t
                .filas
                .iter()
                .map(|f| f.celdas.iter().map(|c| c.ancho).sum::<usize>())
                .max();
            let ultima = columnas.unwrap_or(1).saturating_sub(1).max(1);
            return interpolar(&p.cabecera, s.columna as f64 / ultima as f64);
        }
        let cuerpo = s.fila - usize::from(!p.cabecera.is_empty());
        (!p.filas.is_empty()).then(|| p.filas[cuerpo % p.filas.len()])
    }

    fn pintar_fondos(&mut self, r: &Rejilla<'_>, a: Aspecto<'_>, m: &Medidas) {
        for s in &r.sitios {
            if let Some(color) = self.fondo_de(r, s, a) {
                self.primitivas.push(rectangulo(m.caja(s), color));
            }
        }
        let Some(color) = a.paleta.reglas else { return };
        let mut y = m.origen.y;
        for alto in &m.altos {
            y += alto + m.hueco;
            let regla = Caja {
                x: m.origen.x,
                y: y - m.hueco,
                ancho: m.ancho_total(),
                alto: 0.5,
            };
            self.primitivas.push(rectangulo(regla, color));
        }
    }

    /// Cada celda, ya partida en líneas a su ancho.
    fn escribir_celdas(&mut self, r: &Rejilla<'_>, trazado: &Trazado<'_>) -> Vec<Escrito> {
        let Trazado {
            aspecto,
            anchos,
            tam,
            hueco,
        } = *trazado;
        let p = aspecto.paleta;
        let margen = 2.0 * aspecto.estilo.relleno * tam;
        let mut escritos = Vec::with_capacity(r.sitios.len());
        for s in &r.sitios {
            let hasta = (s.columna + s.celda.ancho).min(anchos.len());
            let columnas = anchos[s.columna..hasta].iter().sum::<f64>();
            let ancho = columnas + hueco * s.celda.ancho.saturating_sub(1) as f64;
            let es_cabecera = s.fila == 0 && !p.cabecera.is_empty();
            let fondo = self.fondo_de(r, s, aspecto);
            let color = color_sobre(fondo, es_cabecera, p);
            let base = Estilo {
                negrita: es_cabecera,
                color: Some(self.indice_de_color(color)),
                ..Estilo::default()
            };
            let alineado = s.celda.alineado.or_else(|| r.tabla.columnas.get(s.columna).copied());
            let hueco_celda = Hueco::de((ancho - margen + HOLGURA).max(1.0), alineado.map(alineacion));
            escritos.push(self.escribir(&s.celda.tramos, hueco_celda, Letra { tam, base }));
        }
        escritos
    }
}

/// El texto de una celda: el del estilo, salvo que el blanco se lea mejor sobre su fondo.
fn color_sobre(fondo: Option<Color>, es_cabecera: bool, p: &Paleta) -> Color {
    if let (true, Some(c)) = (es_cabecera, p.texto_cabecera) {
        return c;
    }
    let Some(fondo) = fondo else { return p.sobre_pagina };
    let blanco = BLANCO.contraste(fondo);
    if (es_cabecera && blanco >= CONTRASTE_CABECERA) || blanco > p.texto.contraste(fondo) {
        BLANCO
    } else {
        p.texto
    }
}

/// Lo que mide cada fila para que quepa lo que lleva.
fn altos_de_filas(r: &Rejilla<'_>, escritos: &[Escrito], t: &Trazado<'_>) -> Vec<f64> {
    let (e, anchos, tam) = (t.aspecto.estilo, t.anchos, t.tam);
    let columna = anchos.iter().copied().fold(0.0, f64::max);
    let minimo = (e.alto * tam).max(e.proporcion.map_or(0.0, |p| p * columna));
    let margen = 2.0 * e.relleno * tam * RELLENO_VERTICAL;
    let mut altos = vec![minimo; r.filas];
    for (s, escrito) in r.sitios.iter().zip(escritos).filter(|(s, _)| s.celda.alto == 1) {
        altos[s.fila] = altos[s.fila].max(escrito.alto + margen);
    }
    // lo que no cabe en las filas que ocupa una celda alta, a la última de ellas
    for (s, escrito) in r.sitios.iter().zip(escritos).filter(|(s, _)| s.celda.alto > 1) {
        let hasta = (s.fila + s.celda.alto).min(r.filas);
        let tienen = altos[s.fila..hasta].iter().sum::<f64>() + e.separacion * (hasta - s.fila - 1) as f64;
        let falta = escrito.alto + margen - tienen;
        if falta > 0.0 {
            altos[hasta - 1] += falta;
        }
    }
    altos
}

#[cfg(test)]
mod tests {
    use super::*;

    const GRIS: Color = Color {
        r: 0.24,
        g: 0.25,
        b: 0.26,
    };

    fn paleta() -> Paleta {
        Paleta {
            cabecera: Vec::new(),
            texto_cabecera: None,
            filas: Vec::new(),
            texto: GRIS,
            sobre_pagina: GRIS,
            reglas: None,
        }
    }

    #[test]
    fn el_contraste_es_el_de_wcag() {
        let negro = Color { r: 0.0, g: 0.0, b: 0.0 };
        assert!((BLANCO.contraste(negro) - 21.0).abs() < 1e-9);
        assert!((negro.contraste(negro) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn una_cabecera_de_color_medio_lleva_texto_blanco() {
        let verde_azulado = Color {
            r: 0.0,
            g: 0.7,
            b: 0.69,
        };
        assert_eq!(color_sobre(Some(verde_azulado), true, &paleta()), BLANCO);
    }

    #[test]
    fn un_fondo_claro_conserva_el_texto_del_estilo() {
        let gris_claro = Color {
            r: 0.95,
            g: 0.95,
            b: 0.96,
        };
        assert_eq!(color_sobre(Some(gris_claro), false, &paleta()), GRIS);
        assert_eq!(color_sobre(Some(gris_claro), true, &paleta()), GRIS);
        assert_eq!(color_sobre(None, false, &paleta()), GRIS);
    }

    #[test]
    fn un_fondo_oscuro_en_una_celda_cualquiera_pasa_a_blanco() {
        let marino = Color { r: 0.1, g: 0.2, b: 0.4 };
        assert_eq!(color_sobre(Some(marino), false, &paleta()), BLANCO);
    }

    #[test]
    fn encajar_recorta_primero_lo_mas_ancho() {
        let mut anchos = [10.0, 100.0, 50.0];
        encajar(&mut anchos, &[0.0; 3], 100.0);
        assert_eq!(anchos[0], 10.0, "la estrecha no se toca");
        assert!((anchos.iter().sum::<f64>() - 100.0).abs() < 1e-6);
    }

    #[test]
    fn encajar_no_baja_de_lo_que_no_se_puede_partir() {
        let mut anchos = [100.0, 100.0, 100.0];
        encajar(&mut anchos, &[80.0, 10.0, 10.0], 150.0);
        assert!(anchos[0] >= 80.0, "{anchos:?}");
        assert!((anchos.iter().sum::<f64>() - 150.0).abs() < 1e-6, "{anchos:?}");
    }

    #[test]
    fn lo_que_ya_cabe_no_se_toca() {
        let mut anchos = [10.0, 20.0];
        encajar(&mut anchos, &[0.0; 2], 100.0);
        assert_eq!(anchos, [10.0, 20.0]);
    }

    #[test]
    fn la_paleta_se_reparte_entre_sus_extremos() {
        let a = Color { r: 0.0, g: 0.0, b: 0.0 };
        assert_eq!(interpolar(&[a, BLANCO], 0.0), Some(a));
        assert_eq!(interpolar(&[a, BLANCO], 1.0), Some(BLANCO));
        assert_eq!(interpolar(&[], 0.5), None);
    }
}
