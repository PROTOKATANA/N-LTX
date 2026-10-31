//! TikZ: `tikra` calcula la geometría y aquí se vuelve primitivas.
//!
//! Su driver de SVG (`tikra::driver`) es la referencia de lo que significa cada elemento de
//! `Canvas`; las costuras que añade entre rellenos contiguos (`seams`) no se traducen, porque son
//! un remedio del rasterizador y le tocan al pintor.
//!
//! La reparación de entrada de `tikra::convert` (`repair_markdown`) tampoco se aplica: es para TikZ
//! escapado por un generador de Markdown, y en LaTeX real confunde un salto de línea seguido de
//! una palabra (`\\\\coordinates`) con una orden escapada.

use std::collections::HashMap;

use mathra::layout::kerning::TextKerning;
use primitivas::{
    Caja, Capa, Color, Composicion, Compositor, Degradado, Discontinuo, ErrorComposicion, Escena, Matriz, Parada,
    Pintura, Primitiva, Punto, Relleno, Remate, Segmento, Trazo, Union,
};
use tikra::canvas::{Canvas, Cap, Fill, Item, Join, Paint, Shading, Stroke};
use tikra::extension::Library;
use tikra::scene::{Label, Point, Segment, Transform};

use crate::caja::{Colocacion, Destino};
use crate::contorno;

/// Bp por centímetro.
const BP: f64 = 72.0 / 2.54;

/// Compone una figura TikZ. `bibliotecas` añade paquetes; `voltik` solo se carga si la figura usa
/// `circuitikz`: sus definiciones gastan parte del límite de instrucciones y cambian figuras que no
/// tienen circuitos (`penrose-tiling` deja de caber y `fault-tree` se aleja de su PDF).
#[derive(Default)]
pub struct FiguraTikra {
    pub bibliotecas: &'static [&'static Library],
}

impl FiguraTikra {
    fn bibliotecas_para(&self, fuente: &str) -> Vec<&'static Library> {
        let mut bibliotecas = self.bibliotecas.to_vec();
        if fuente.contains("circuitikz") {
            bibliotecas.push(&voltik::LIBRARY);
        }
        bibliotecas
    }
}

impl Compositor for FiguraTikra {
    fn componer(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
        // `circuitikz` como el `tikzpicture` que es, y `siunitx` en las etiquetas
        let entrada = preparar(&voltik::environments(fuente))?;
        // `tikra::convert` lo hace antes de maquetar cualquier etiqueta
        tikra::label::set_kerning(TextKerning::Ot1);
        let (lienzo, avisos, hechos) = tikra::engine::run_document_with(&entrada, &self.bibliotecas_para(fuente))
            .map_err(|e| ErrorComposicion(e.to_string()))?;

        let mut destino = Destino {
            avisos,
            ..Destino::default()
        };
        let esquina = esquina_inferior(&lienzo);
        traducir(&lienzo.items, esquina, &mut destino);
        if !destino.pendientes.is_empty() {
            let palabras = tikra::describe::describe(&hechos, tikra::Locale::default());
            contornos_termes(&tikra::driver::svg(&lienzo, fuente, &palabras), &mut destino);
        }

        if destino.primitivas.is_empty() {
            destino.avisos.push("la figura no dibuja nada".into());
        }

        let (x0, y0, x1, y1) = limites(&lienzo);
        Ok(Composicion {
            escena: Escena {
                caja: Caja {
                    x: x0 * BP,
                    y: -y1 * BP,
                    ancho: ((x1 - x0) * BP).max(0.0),
                    alto: ((y1 - y0) * BP).max(0.0),
                },
                fondo: lienzo.page_color.as_ref().map(color),
                primitivas: destino.primitivas,
                contornos: destino.contornos,
            },
            avisos: destino.avisos,
        })
    }
}

/// Una figura se deja tal cual y unas órdenes sueltas se envuelven en un `tikzpicture`, como hace
/// `tikra::convert`.
fn preparar(fuente: &str) -> Result<String, ErrorComposicion> {
    let texto = sin_comentarios(fuente);
    let es_figura = ["\\begin{tikzpicture}", "\\begin{pgfpicture}", "\\pgfuseshading"]
        .iter()
        .any(|m| texto.contains(m));
    if es_figura || tiene_orden_tikz(&texto) {
        return Ok(fuente.to_string());
    }
    let sueltas = [
        "\\begin{axis}",
        "\\draw",
        "\\fill",
        "\\node",
        "\\path",
        "\\filldraw",
        "\\coordinate",
    ];
    if sueltas.iter().any(|m| texto.contains(m)) {
        return Ok(format!("\\begin{{tikzpicture}}\\relax\n{fuente}\n\\end{{tikzpicture}}"));
    }
    Err(ErrorComposicion("no hay ninguna figura TikZ en la fuente".into()))
}

/// `\tikz` como orden propia, no como prefijo de `\tikzset` y parecidas.
fn tiene_orden_tikz(texto: &str) -> bool {
    texto
        .match_indices("\\tikz")
        .any(|(i, orden)| !texto[i + orden.len()..].starts_with(|c: char| c.is_ascii_alphabetic()))
}

fn sin_comentarios(fuente: &str) -> String {
    fuente.lines().map(linea_sin_comentario).collect::<Vec<_>>().join("\n")
}

fn linea_sin_comentario(linea: &str) -> &str {
    let mut escapado = false;
    for (i, c) in linea.char_indices() {
        match c {
            '%' if !escapado => return &linea[..i],
            '\\' => escapado = !escapado,
            _ => escapado = false,
        }
    }
    linea
}

/// Los límites de la figura con su margen, en cm y con la y hacia arriba.
fn limites(lienzo: &Canvas) -> (f64, f64, f64, f64) {
    let m = lienzo.margin;
    let (min, max) = (lienzo.min.unwrap_or_default(), lienzo.max.unwrap_or_default());
    (min.x - m, min.y - m, max.x + m, max.y + m)
}

/// La esquina inferior izquierda de la página: desde ahí se repiten patrones y baldosas, como en PDF.
fn esquina_inferior(lienzo: &Canvas) -> Punto {
    let (x0, y0, _, _) = limites(lienzo);
    Punto {
        x: x0 * BP,
        y: -y0 * BP,
    }
}

fn traducir(items: &[Item], esquina: Punto, d: &mut Destino) {
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
            Item::Text { label, place, opacity } => etiqueta(label, place, *opacity, d),
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

fn etiqueta(l: &Label, lugar: &Transform, opacidad: f64, d: &mut Destino) {
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
                ancho_glifo: l.stretch,
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
fn contornos_termes(svg: &str, d: &mut Destino) {
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

fn punto(p: Point) -> Punto {
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

/// Un nombre de color según xcolor, que es lo que significa en LaTeX: `green` es #00ff00, no el
/// #008000 de CSS, y `OliveGreen` existe.
pub(crate) fn color_xcolor(nombre: &str) -> Option<Color> {
    tikra::color::parse(nombre, &HashMap::new()).map(|c| color(&c))
}

fn color(c: &tikra::color::Color) -> Color {
    Color {
        r: c.r.clamp(0.0, 1.0),
        g: c.g.clamp(0.0, 1.0),
        b: c.b.clamp(0.0, 1.0),
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
        limite_inglete: s.miter_limit.max(1.0),
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
    fn un_porcentaje_escapado_no_es_comentario() {
        assert_eq!(linea_sin_comentario(r"50\% de \draw % resto"), r"50\% de \draw ");
        assert_eq!(linea_sin_comentario(r"\\% comentario"), r"\\");
    }

    #[test]
    fn solo_se_acepta_lo_que_es_una_figura() {
        assert!(preparar("% \\draw comentado\nnada").is_err());
        assert!(preparar(r"\tikzset{a/.style={red}}").is_err());
        assert_eq!(preparar(r"\tikz \draw (0,0);").unwrap(), r"\tikz \draw (0,0);");
        assert!(preparar(r"\draw (0,0);").unwrap().starts_with(r"\begin{tikzpicture}"));
    }

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
