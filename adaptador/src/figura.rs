//! TikZ: `tikra` calcula la geometría y aquí se vuelve primitivas.
//!
//! Su driver de SVG (`tikra::driver`) es la referencia de lo que significa cada elemento de
//! `Canvas`; las costuras que añade entre rellenos contiguos (`seams`) no se traducen, porque son
//! un remedio del rasterizador y le tocan al pintor.
//!
//! La reparación de entrada de `tikra::convert` (`repair_markdown`) tampoco se aplica: es para TikZ
//! escapado por un generador de Markdown, y en LaTeX real confunde un salto de línea seguido de
//! una palabra (`\\\\coordinates`) con una orden escapada.

use mathra::layout::kerning::TextKerning;
use primitivas::{Caja, Composicion, Compositor, ErrorComposicion, Escena, Punto};
use tikra::canvas::Canvas;
use tikra::extension::Library;

use crate::caja::Destino;
use crate::traduccion::{color, contornos_termes, punto, traducir};

/// Bp por centímetro.
pub(crate) const BP: f64 = 72.0 / 2.54;

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
                origen: lienzo.origin.map_or(Punto::default(), punto),
                zonas: Vec::new(),
                anclas: Vec::new(),
            },
            avisos: destino.avisos,
        })
    }
}

/// Una figura se deja tal cual y unas órdenes sueltas se envuelven en un `tikzpicture`, como hace
/// `tikra::convert`.
/// Órdenes de paquetes que solo añaden algo que aquí no existe: se reducen a lo que se ve.
const ORDENES_DE_CORTESIA: [(&str, &str); 2] = [
    ("\\pdftooltip", "\\newcommand{\\pdftooltip}[2]{#1}\n"),
    ("\\tikzcdset", "\\newcommand{\\tikzcdset}[1]{}\n"),
];

fn preparar(fuente: &str) -> Result<String, ErrorComposicion> {
    let cortesia: String = ORDENES_DE_CORTESIA
        .iter()
        .filter(|(orden, _)| fuente.contains(orden))
        .map(|(_, definicion)| *definicion)
        .collect();
    envolver(fuente).map(|f| format!("{cortesia}{f}"))
}

fn envolver(fuente: &str) -> Result<String, ErrorComposicion> {
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
}
