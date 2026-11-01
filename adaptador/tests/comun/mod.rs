//! Lo que comparten las pruebas: escribir una escena como SVG y rasterizarla. Es un pintor de
//! referencia que prueba que la lista neutra no pierde nada frente a lo que dibuja `tikra`.

#![allow(dead_code)]

pub mod dolares;

use std::fmt::Write;

use primitivas::{
    Capa, Color, Degradado, Escena, GlifoId, Matriz, Parada, Pintura, Primitiva, Relleno, Remate, Segmento, Trazo,
    Union,
};
use resvg::{tiny_skia, usvg};

struct Salida {
    defs: String,
    cuenta: usize,
    hay_patron: bool,
}

impl Salida {
    fn id(&mut self, prefijo: &str) -> String {
        self.cuenta += 1;
        format!("{prefijo}{}", self.cuenta)
    }
}

/// `None` si la escena usa un patrón con nombre: su dibujo vive en un driver privado de `tikra`.
pub fn svg(e: &Escena) -> Option<String> {
    let mut salida = Salida {
        defs: String::new(),
        cuenta: 0,
        hay_patron: false,
    };
    let mut cuerpo = String::new();
    if let Some(c) = e.fondo {
        let caja = e.caja;
        let _ = write!(
            cuerpo,
            r#"<rect x="{}" y="{}" width="{}" height="{}" fill="{}"/>"#,
            n(caja.x),
            n(caja.y),
            n(caja.ancho),
            n(caja.alto),
            hex(c)
        );
    }
    escribir(&e.primitivas, &mut cuerpo, &mut salida);
    for (glifo, contorno) in &e.contornos {
        let _ = write!(
            salida.defs,
            r#"<path id="{}" d="{}"/>"#,
            id_glifo(*glifo),
            ruta(contorno)
        );
    }
    if salida.hay_patron {
        return None;
    }
    let c = e.caja;
    Some(format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{} {} {} {}" width="{}pt" height="{}pt"><defs>{}</defs>{cuerpo}</svg>"#,
        n(c.x),
        n(c.y),
        n(c.ancho),
        n(c.alto),
        n(c.ancho),
        n(c.alto),
        salida.defs
    ))
}

/// Redondeo a `lugares` decimales, como escribe sus números el driver de `tikra`: al comparar, el
/// suavizado no debe cambiar por una milésima de punto.
fn r(v: f64, lugares: i32) -> f64 {
    let k = 10f64.powi(lugares);
    (v * k).round() / k
}

fn n(v: f64) -> f64 {
    r(v, 3)
}

fn id_glifo(g: GlifoId) -> String {
    format!("g{}-{}", g.fuente, g.id)
}

fn hex(c: Color) -> String {
    let v = |x: f64| (x * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", v(c.r), v(c.g), v(c.b))
}

fn ruta(segmentos: &[Segmento]) -> String {
    let mut d = String::new();
    for s in segmentos {
        let _ = match s {
            Segmento::Mover(p) => write!(d, "M{} {} ", n(p.x), n(p.y)),
            Segmento::Linea(p) => write!(d, "L{} {} ", n(p.x), n(p.y)),
            Segmento::Curva(a, b, p) => {
                write!(d, "C{} {} {} {} {} {} ", n(a.x), n(a.y), n(b.x), n(b.y), n(p.x), n(p.y))
            }
            Segmento::Cerrar => write!(d, "Z "),
        };
    }
    d
}

fn matriz(m: &Matriz) -> String {
    format!(
        "matrix({} {} {} {} {} {})",
        r(m.a, 5),
        r(m.b, 5),
        r(m.c, 5),
        r(m.d, 5),
        n(m.e),
        n(m.f)
    )
}

fn atributos_trazo(t: &Trazo) -> String {
    let mut a = format!(r#" stroke="{}" stroke-width="{}""#, hex(t.color), n(t.ancho));
    if let Some(d) = &t.discontinuo {
        let tramos: Vec<String> = d.tramos.iter().map(|t| n(*t).to_string()).collect();
        let _ = write!(
            a,
            r#" stroke-dasharray="{}" stroke-dashoffset="{}""#,
            tramos.join(" "),
            n(d.fase)
        );
    }
    match t.remate {
        Remate::Plano => {}
        Remate::Redondo => a.push_str(r#" stroke-linecap="round""#),
        Remate::Cuadrado => a.push_str(r#" stroke-linecap="square""#),
    }
    match t.union {
        Union::Inglete => {
            let _ = write!(a, r#" stroke-miterlimit="{}""#, t.limite_inglete);
        }
        Union::Redonda => a.push_str(r#" stroke-linejoin="round""#),
        Union::Biselada => a.push_str(r#" stroke-linejoin="bevel""#),
    }
    if t.opacidad < 1.0 {
        let _ = write!(a, r#" stroke-opacity="{}""#, t.opacidad);
    }
    a
}

fn paradas(p: &[Parada]) -> String {
    p.iter()
        .map(|p| format!(r#"<stop offset="{}" stop-color="{}"/>"#, p.posicion, hex(p.color)))
        .collect()
}

fn degradado(d: &Degradado, s: &mut Salida) -> String {
    let id = s.id("d");
    let _ = match d {
        Degradado::Lineal {
            desde,
            hasta,
            matriz: m,
            paradas: p,
        } => write!(
            s.defs,
            r#"<linearGradient id="{id}" gradientUnits="userSpaceOnUse" gradientTransform="{}" x1="{}" y1="{}" x2="{}" y2="{}">{}</linearGradient>"#,
            matriz(m),
            desde.x,
            desde.y,
            hasta.x,
            hasta.y,
            paradas(p)
        ),
        Degradado::Radial {
            radio,
            foco,
            matriz: m,
            paradas: p,
        } => write!(
            s.defs,
            r#"<radialGradient id="{id}" gradientUnits="userSpaceOnUse" gradientTransform="{}" cx="0" cy="0" r="{radio}" fx="{}" fy="{}">{}</radialGradient>"#,
            matriz(m),
            foco.x,
            foco.y,
            paradas(p)
        ),
    };
    format!("url(#{id})")
}

fn valor_pintura(p: &Pintura, s: &mut Salida) -> String {
    match p {
        Pintura::Solida(c) => hex(*c),
        Pintura::Degradado(d) => degradado(d, s),
        Pintura::Patron { .. } => {
            s.hay_patron = true;
            "none".into()
        }
        Pintura::Mosaico {
            primitivas,
            origen,
            ancho,
            alto,
            matriz: m,
        } => {
            let mut dentro = String::new();
            escribir(primitivas, &mut dentro, s);
            let id = s.id("p");
            let _ = write!(
                s.defs,
                r#"<pattern id="{id}" patternUnits="userSpaceOnUse" x="{}" y="{}" width="{ancho}" height="{alto}" patternTransform="{}"><g transform="translate({} {})">{dentro}</g></pattern>"#,
                origen.x,
                origen.y,
                matriz(m),
                -origen.x,
                -origen.y
            );
            format!("url(#{id})")
        }
    }
}

fn atributos_relleno(r: &Relleno, s: &mut Salida) -> String {
    let mut a = format!(r#" fill="{}""#, valor_pintura(&r.pintura, s));
    if r.opacidad < 1.0 {
        let _ = write!(a, r#" fill-opacity="{}""#, r.opacidad);
    }
    if r.par_impar {
        a.push_str(r#" fill-rule="evenodd""#);
    }
    a
}

/// Máscara que cubre toda la escena.
fn mascara(contenido: &str, s: &mut Salida) -> String {
    let id = s.id("m");
    let _ = write!(
        s.defs,
        r#"<mask id="{id}" maskUnits="userSpaceOnUse" x="-100000" y="-100000" width="200000" height="200000">{contenido}</mask>"#
    );
    id
}

const LIENZO_MASCARA: &str = r#"<rect x="-100000" y="-100000" width="200000" height="200000" fill="#;

fn trazado(segmentos: &[Segmento], relleno: &Option<Relleno>, trazo: &Option<Trazo>, b: &mut String, s: &mut Salida) {
    let d = ruta(segmentos);
    let compartida = match (relleno, trazo) {
        (Some(r), Some(t)) if r.opacidad < 1.0 && (r.opacidad - t.opacidad).abs() < 1e-9 => Some(r.opacidad),
        _ => None,
    };
    if let (Some(r), Some(t), None) = (relleno, trazo, compartida)
        && t.opacidad < 1.0
    {
        let corte = atributos_trazo(&Trazo {
            color: Color { r: 0.0, g: 0.0, b: 0.0 },
            opacidad: 1.0,
            ..t.clone()
        });
        let id = mascara(
            &format!(r#"{LIENZO_MASCARA}"white"/><path d="{d}" fill="none"{corte}/>"#),
            s,
        );
        let fa = atributos_relleno(r, s);
        let _ = write!(
            b,
            r#"<g><path d="{d}"{fa} mask="url(#{id})"/><path d="{d}" fill="none"{}/></g>"#,
            atributos_trazo(t)
        );
        return;
    }
    let relleno_efectivo = relleno.clone().map(|r| match compartida {
        Some(_) => Relleno { opacidad: 1.0, ..r },
        None => r,
    });
    let trazo_efectivo = trazo.clone().map(|t| match compartida {
        Some(_) => Trazo { opacidad: 1.0, ..t },
        None => t,
    });
    let mut a = match &relleno_efectivo {
        Some(r) => atributos_relleno(r, s),
        None => r#" fill="none""#.to_string(),
    };
    if let Some(t) = &trazo_efectivo {
        a.push_str(&atributos_trazo(t));
    }
    if let Some(o) = compartida {
        let _ = write!(a, r#" opacity="{o}""#);
    }
    let _ = write!(b, r#"<path d="{d}"{a}/>"#);
}

fn escribir(primitivas: &[Primitiva], b: &mut String, s: &mut Salida) {
    for p in primitivas {
        match p {
            Primitiva::Trazado {
                segmentos,
                relleno,
                trazo,
            } => trazado(segmentos, relleno, trazo, b, s),
            Primitiva::Glifo {
                glifo,
                matriz: m,
                color,
            } => {
                let _ = write!(
                    b,
                    r##"<use href="#{}" transform="{}" fill="{}"/>"##,
                    id_glifo(*glifo),
                    matriz(m),
                    hex(*color)
                );
            }
            Primitiva::Imagen { mapa, matriz: m } => {
                let png = png_de(mapa);
                let _ = write!(
                    b,
                    r#"<image width="1" height="1" preserveAspectRatio="none" image-rendering="optimizeSpeed" transform="{}" href="data:image/png;base64,{}"/>"#,
                    matriz(m),
                    base64(&png)
                );
            }
            Primitiva::Abrir(capa) => abrir(capa, b, s),
            Primitiva::Cerrar => b.push_str("</g>"),
        }
    }
}

fn abrir(capa: &Capa, b: &mut String, s: &mut Salida) {
    let _ = match capa {
        Capa::Recorte { segmentos, par_impar } => {
            let id = s.id("c");
            let regla = if *par_impar { r#" clip-rule="evenodd""# } else { "" };
            let _ = write!(
                s.defs,
                r#"<clipPath id="{id}"><path d="{}"{regla}/></clipPath>"#,
                ruta(segmentos)
            );
            write!(b, r#"<g clip-path="url(#{id})">"#)
        }
        Capa::Grupo { opacidad } if *opacidad < 1.0 => write!(b, r#"<g opacity="{opacidad}">"#),
        Capa::Grupo { .. } => write!(b, r#"<g style="isolation:isolate">"#),
        Capa::Transformacion(m) => write!(b, r#"<g transform="{}">"#, matriz(m)),
        Capa::Mezcla(modo) => write!(b, r#"<g style="mix-blend-mode:{modo}">"#),
        Capa::MascaraDegradado(d) => {
            let pintura = degradado(d, s);
            let id = mascara(&format!(r#"{LIENZO_MASCARA}"{pintura}"/>"#), s);
            write!(b, r#"<g mask="url(#{id})">"#)
        }
        Capa::MascaraEscena(primitivas) => {
            let mut dentro = String::new();
            escribir(primitivas, &mut dentro, s);
            let id = mascara(&dentro, s);
            write!(b, r#"<g mask="url(#{id})">"#)
        }
        Capa::BordesNitidos => write!(b, r#"<g shape-rendering="crispEdges">"#),
    };
}

/// De vuelta a PNG: `resvg` solo acepta imágenes codificadas.
fn png_de(mapa: &primitivas::MapaDeBits) -> Vec<u8> {
    let mut salida = Vec::new();
    let mut e = png::Encoder::new(&mut salida, mapa.ancho, mapa.alto);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    let mut w = e.write_header().unwrap();
    w.write_image_data(&mapa.rgba).unwrap();
    w.finish().unwrap();
    salida
}

fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let v =
            (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        for k in 0..4 {
            s.push(if k <= c.len() {
                char::from(ABC[(v >> (18 - 6 * k)) as usize & 63])
            } else {
                '='
            });
        }
    }
    s
}

pub fn rasterizar(svg: &str, dpi: f32) -> Option<tiny_skia::Pixmap> {
    let arbol = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let escala = dpi / 96.0;
    let tam = arbol.size();
    let (ancho, alto) = (
        (tam.width() * escala).round() as u32,
        (tam.height() * escala).round() as u32,
    );
    let mut imagen = tiny_skia::Pixmap::new(ancho.max(1), alto.max(1))?;
    imagen.fill(tiny_skia::Color::WHITE);
    resvg::render(
        &arbol,
        tiny_skia::Transform::from_scale(escala, escala),
        &mut imagen.as_mut(),
    );
    Some(imagen)
}

/// Fracción de píxeles en los que algún canal difiere en más de `tolerancia`.
pub fn diferencia(a: &tiny_skia::Pixmap, b: &tiny_skia::Pixmap, tolerancia: u8) -> Option<f64> {
    if (a.width(), a.height()) != (b.width(), b.height()) {
        return None;
    }
    let distintos = a
        .data()
        .chunks_exact(4)
        .zip(b.data().chunks_exact(4))
        .filter(|(p, q)| p.iter().zip(q.iter()).any(|(x, y)| x.abs_diff(*y) > tolerancia))
        .count();
    Some(distintos as f64 / (f64::from(a.width()) * f64::from(a.height())))
}

/// Toda capa abierta se cierra y no se cierra nada de más.
pub fn capas_equilibradas(primitivas: &[Primitiva]) -> bool {
    let mut abiertas = 0i64;
    for p in primitivas {
        match p {
            Primitiva::Abrir(_) => abiertas += 1,
            Primitiva::Cerrar => abiertas -= 1,
            _ => {}
        }
        if abiertas < 0 {
            return false;
        }
    }
    abiertas == 0
}

/// Todos los glifos de la escena traen su contorno.
pub fn glifos_con_contorno(e: &Escena) -> bool {
    fn revisar(primitivas: &[Primitiva], e: &Escena) -> bool {
        primitivas.iter().all(|p| match p {
            Primitiva::Glifo { glifo, .. } => e.contornos.contains_key(glifo),
            Primitiva::Abrir(Capa::MascaraEscena(dentro)) => revisar(dentro, e),
            Primitiva::Trazado {
                relleno:
                    Some(Relleno {
                        pintura: Pintura::Mosaico { primitivas, .. },
                        ..
                    }),
                ..
            } => revisar(primitivas, e),
            _ => true,
        })
    }
    revisar(&e.primitivas, e)
}
