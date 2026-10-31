//! De los elementos de `Canvas` a primitivas: la única pieza que conoce sus tipos.

use std::collections::HashMap;

use documento::{ColorDefinido, Colores};
use primitivas::{
    Capa, Color, Degradado, Discontinuo, LIMITE_INGLETE, Matriz, Parada, Pintura, Primitiva, Punto, Relleno, Remate,
    Segmento, Trazo, Union,
};
use tikra::canvas::{Cap, Fill, Item, Join, Paint, Shading, Stroke};
use tikra::scene::{Label, Point, Segment, Transform};

use crate::caja::{Colocacion, Destino};
use crate::contorno;
use crate::figura::BP;

pub(crate) fn traducir(items: &[Item], esquina: Punto, d: &mut Destino) {
    for item in items {
        match item {
            Item::Path { segments, fill, stroke } => {
                if segments.is_empty() || (fill.is_none() && stroke.is_none()) {
                    continue;
                }
                let relleno = fill.as_ref().map(|f| relleno(f, esquina, d));
                d.primitivas.push(Primitiva::Trazado {
                    segmentos: segmentos(segments),
                    relleno,
                    trazo: stroke.as_ref().map(trazo),
                });
            }
            Item::Text { label, place, opacity } => {
                let puesto = Puesto {
                    lugar: place,
                    opacidad: *opacity,
                };
                etiqueta(label, puesto, d);
            }
            Item::Clip { segments, even_odd } => d.primitivas.push(Primitiva::Abrir(Capa::Recorte {
                segmentos: segmentos(segments),
                par_impar: *even_odd,
            })),
            Item::Group { opacity } => d.primitivas.push(Primitiva::Abrir(Capa::Grupo {
                opacidad: opacity.clamp(0.0, 1.0),
            })),
            Item::Crisp => d.primitivas.push(Primitiva::Abrir(Capa::BordesNitidos)),
            Item::Blend { mode } => d.primitivas.push(Primitiva::Abrir(Capa::Mezcla(mode.clone()))),
            Item::Transform { transform } => {
                d.primitivas
                    .push(Primitiva::Abrir(Capa::Transformacion(matriz(transform))));
            }
            Item::Fade { mask } => d
                .primitivas
                .push(Primitiva::Abrir(Capa::MascaraDegradado(degradado(mask)))),
            Item::FadePicture { items } => {
                let mascara = aparte(items, esquina, d);
                d.primitivas.push(Primitiva::Abrir(Capa::MascaraEscena(mascara)));
            }
            Item::Image { png, place: t } => d.primitivas.push(Primitiva::Imagen {
                png: png.as_ref().clone(),
                // el cuadrado unidad con la y hacia arriba, en la escena
                matriz: Matriz {
                    a: t.a * BP,
                    b: -t.b * BP,
                    c: -t.c * BP,
                    d: t.d * BP,
                    e: (t.c + t.e) * BP,
                    f: -(t.d + t.f) * BP,
                },
            }),
            Item::End => d.primitivas.push(Primitiva::Cerrar),
        }
    }
}

/// Traduce `items` a su propia lista; sus contornos y avisos van a la escena.
fn aparte(items: &[Item], esquina: Punto, d: &mut Destino) -> Vec<Primitiva> {
    let principal = std::mem::take(&mut d.primitivas);
    traducir(items, esquina, d);
    std::mem::replace(&mut d.primitivas, principal)
}

/// Dónde va un texto y con qué opacidad.
struct Puesto<'a> {
    lugar: &'a Transform,
    opacidad: f64,
}

fn etiqueta(l: &Label, Puesto { lugar, opacidad }: Puesto<'_>, d: &mut Destino) {
    d.primitivas.push(Primitiva::Abrir(Capa::Transformacion(matriz(lugar))));
    if opacidad < 1.0 {
        d.primitivas.push(Primitiva::Abrir(Capa::Grupo { opacidad }));
    }
    let ancho = l.width();
    for (k, linea) in l.lines.iter().enumerate() {
        let w = f64::from(linea.width) * l.scale;
        let x = match l.align {
            'l' => 0.0,
            'r' => ancho - w,
            _ => (ancho - w) / 2.0,
        };
        d.caja(
            &linea.items,
            &Colocacion {
                origen: Punto {
                    x: x * BP,
                    y: -l.baseline(k) * BP,
                },
                escala: l.scale * BP,
                ancho_glifo: l.stretch_of(k),
                fuente: l.font,
                color: color(&l.color),
            },
        );
    }
    if opacidad < 1.0 {
        d.primitivas.push(Primitiva::Cerrar);
    }
    d.primitivas.push(Primitiva::Cerrar);
}

/// Los contornos de Termes solo salen en el SVG de `tikra`: `tikra::times` es privado.
pub(crate) fn contornos_termes(svg: &str, d: &mut Destino) {
    for glifo in std::mem::take(&mut d.pendientes) {
        let marca = format!(r#"<path id="tr-gt{}" d=""#, glifo.id);
        let contorno = svg.find(&marca).and_then(|i| {
            let resto = &svg[i + marca.len()..];
            resto.find('"').map(|fin| contorno::segmentos(&resto[..fin]))
        });
        match contorno {
            Some(contorno) => {
                d.contornos.insert(glifo, contorno);
            }
            None => d.avisos.push(format!("glifo sin contorno: {}", glifo.id)),
        }
    }
}

pub(crate) fn punto(p: Point) -> Punto {
    Punto {
        x: p.x * BP,
        y: -p.y * BP,
    }
}

fn segmentos(segmentos: &[Segment]) -> Vec<Segmento> {
    segmentos
        .iter()
        .map(|s| match s {
            Segment::MoveTo(p) => Segmento::Mover(punto(*p)),
            Segment::LineTo(p) => Segmento::Linea(punto(*p)),
            Segment::CurveTo(a, b, p) => Segmento::Curva(punto(*a), punto(*b), punto(*p)),
            Segment::Close => Segmento::Cerrar,
        })
        .collect()
}

/// Una transformación del lienzo (cm, y hacia arriba) en la escena (bp, y hacia abajo).
fn matriz(t: &Transform) -> Matriz {
    Matriz {
        a: t.a,
        b: -t.b,
        c: -t.c,
        d: t.d,
        e: t.e * BP,
        f: -t.f * BP,
    }
}

/// Los colores con el nombre que les da `xcolor`.
pub struct ColoresXcolor;

impl Colores for ColoresXcolor {
    fn color(&self, nombre: &str, definidos: &[ColorDefinido]) -> Option<Color> {
        color_xcolor(nombre, definidos)
    }
}

/// Un nombre de color según xcolor, que es lo que significa en LaTeX: `green` es #00ff00, no el
/// #008000 de CSS, y `OliveGreen` existe.
pub(crate) fn color_xcolor(nombre: &str, definidos: &[ColorDefinido]) -> Option<Color> {
    use tikra::color::{define, parse};
    if let Some(html) = nombre.strip_prefix('#') {
        return define("HTML", html).map(|c| color(&c));
    }
    if let Some((modelo, valor)) = nombre.split_once(':') {
        return define(modelo, valor).map(|c| color(&c));
    }
    let propios: HashMap<_, _> = definidos
        .iter()
        .filter_map(|d| Some((d.nombre.clone(), define(&d.modelo, &d.valor)?)))
        .collect();
    parse(nombre, &propios).map(|c| color(&c))
}

pub(crate) fn color(c: &tikra::color::Color) -> Color {
    Color {
        r: c.r.clamp(0.0, 1.0),
        g: c.g.clamp(0.0, 1.0),
        b: c.b.clamp(0.0, 1.0),
    }
}

/// Lo que TikZ hereda de PDF: si es este, la fuente no ha pedido ninguno y manda el del motor.
fn limite_de_inglete(pedido: f64) -> f64 {
    const POR_OMISION_EN_TIKZ: f64 = 10.0;
    if pedido == POR_OMISION_EN_TIKZ {
        LIMITE_INGLETE
    } else {
        pedido.max(1.0)
    }
}

fn trazo(s: &Stroke) -> Trazo {
    Trazo {
        color: color(&s.color),
        ancho: s.width * BP,
        discontinuo: s.dash.as_ref().and_then(discontinuo),
        remate: match s.cap {
            Cap::Butt => Remate::Plano,
            Cap::Round => Remate::Redondo,
            Cap::Square => Remate::Cuadrado,
        },
        union: match s.join {
            Join::Miter => Union::Inglete,
            Join::Round => Union::Redonda,
            Join::Bevel => Union::Biselada,
        },
        limite_inglete: limite_de_inglete(s.miter_limit),
        opacidad: s.opacity.clamp(0.0, 1.0),
    }
}

fn discontinuo((tramos, fase): &(Vec<f64>, f64)) -> Option<Discontinuo> {
    tramos.iter().any(|t| *t > 0.0).then(|| Discontinuo {
        tramos: tramos.iter().map(|t| t * BP).collect(),
        fase: fase * BP,
    })
}

fn relleno(f: &Fill, esquina: Punto, d: &mut Destino) -> Relleno {
    Relleno {
        pintura: pintura(&f.paint, esquina, d),
        opacidad: f.opacity.clamp(0.0, 1.0),
        par_impar: f.even_odd,
    }
}

fn pintura(p: &Paint, esquina: Punto, d: &mut Destino) -> Pintura {
    match p {
        Paint::Solid(c) => Pintura::Solida(color(c)),
        Paint::Pattern { name, color: c } => Pintura::Patron {
            nombre: name.clone(),
            color: color(c),
            esquina,
        },
        Paint::Shading(s) => Pintura::Degradado(degradado(s)),
        Paint::Tile {
            items,
            origin,
            size,
            transform: t,
        } => Pintura::Mosaico {
            primitivas: aparte(&items.0, esquina, d),
            origen: Punto {
                x: origin.x * BP,
                y: -(origin.y + size.y) * BP,
            },
            ancho: size.x * BP,
            alto: size.y * BP,
            matriz: Matriz {
                a: t.a,
                b: -t.b,
                c: -t.c,
                d: t.d,
                e: t.e * BP + esquina.x,
                f: -t.f * BP + esquina.y,
            },
        },
    }
}

fn degradado(s: &Shading) -> Degradado {
    // la caja de sombreado de pgf (de -50 a 50) llevada a la escena por su transformación
    let en_escena = |t: &Transform| Matriz {
        a: t.a * BP,
        b: -t.b * BP,
        c: t.c * BP,
        d: -t.d * BP,
        e: t.e * BP,
        f: -t.f * BP,
    };
    let paradas = |paradas: &[(f64, tikra::color::Color)]| {
        paradas
            .iter()
            .map(|(p, c)| Parada {
                posicion: p.clamp(0.0, 1.0),
                color: color(c),
            })
            .collect()
    };
    match s {
        Shading::Axial { transform, stops } => Degradado::Lineal {
            desde: Punto { x: 0.0, y: -50.0 },
            hasta: Punto { x: 0.0, y: 50.0 },
            matriz: en_escena(transform),
            paradas: paradas(stops),
        },
        Shading::Radial {
            transform,
            focus,
            stops,
        } => Degradado::Radial {
            radio: 50.0,
            foco: Punto { x: focus.x, y: focus.y },
            matriz: en_escena(transform),
            paradas: paradas(stops),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_trazo_sin_tramos_positivos_es_continuo() {
        assert_eq!(discontinuo(&(vec![0.0], 0.0)), None);
        assert_eq!(
            discontinuo(&(vec![1.0], 0.5)),
            Some(Discontinuo {
                tramos: vec![BP],
                fase: 0.5 * BP
            })
        );
    }
}
