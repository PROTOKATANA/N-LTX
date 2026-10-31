//! El color del código: las gramáticas de `syntect`, con el conjunto de `two-face`, que trae los
//! lenguajes que el de `syntect` no tiene. Cargarlas cuesta, y solo se hace si hay código.

use std::sync::OnceLock;

use documento::{ColoresEditor, Fragmento, Resaltador};
use primitivas::Color;
use syntect::easy::HighlightLines;
use syntect::highlighting::{FontStyle, Style, Theme};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;
use two_face::theme::{EmbeddedLazyThemeSet, extra};

/// El color del código, con las gramáticas de `syntect`.
pub struct ResaltadorSyntect;

impl Resaltador for ResaltadorSyntect {
    fn resaltar(&self, codigo: &str, lenguaje: &str, tema: &str) -> Vec<Fragmento> {
        resaltar(codigo, lenguaje, tema)
    }

    fn editor(&self, tema: &str) -> Option<ColoresEditor> {
        colores_de_editor(tema)
    }
}

fn gramaticas() -> &'static SyntaxSet {
    static GRAMATICAS: OnceLock<SyntaxSet> = OnceLock::new();
    GRAMATICAS.get_or_init(two_face::syntax::extra_newlines)
}

/// Un nombre sin lo que no lo distingue: `OneHalfDark`, `one-half-dark` y `one half dark` son uno.
fn normalizado(nombre: &str) -> String {
    nombre
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn tema(nombre: &str) -> Option<&'static Theme> {
    static TEMAS: OnceLock<EmbeddedLazyThemeSet> = OnceLock::new();
    let buscado = normalizado(nombre);
    let propio = EmbeddedLazyThemeSet::theme_names()
        .iter()
        .find(|t| normalizado(t.as_name()) == buscado)?;
    Some(TEMAS.get_or_init(extra).get(*propio))
}

/// Un color de un tema sobre `fondo`: los números suelen llevar transparencia.
fn sobre(c: syntect::highlighting::Color, fondo: Color) -> Color {
    let a = f64::from(c.a) / 255.0;
    let mezcla = |v: u8, f: f64| f64::from(v) / 255.0 * a + f * (1.0 - a);
    Color {
        r: mezcla(c.r, fondo.r),
        g: mezcla(c.g, fondo.g),
        b: mezcla(c.b, fondo.b),
    }
}

/// Si un color se lee sobre otro. Algunos temas apagan tanto los números que casi no se ven.
fn se_distingue(a: Color, b: Color) -> bool {
    const DIFERENCIA_MINIMA: f64 = 0.25;
    ((a.r - b.r).abs() + (a.g - b.g).abs() + (a.b - b.b).abs()) / 3.0 >= DIFERENCIA_MINIMA
}

/// Los colores del panel, que da el propio tema.
pub(crate) fn colores_de_editor(nombre: &str) -> Option<ColoresEditor> {
    let ajustes = &tema(nombre)?.settings;
    let negro = Color { r: 0.0, g: 0.0, b: 0.0 };
    let fondo = sobre(ajustes.background?, negro);
    let texto = sobre(ajustes.foreground?, fondo);
    // de lo que da el tema a lo que seguro que se lee: su texto
    let entre = |a: f64, b: f64| a * 0.7 + b * 0.3;
    let apagado = Color {
        r: entre(texto.r, fondo.r),
        g: entre(texto.g, fondo.g),
        b: entre(texto.b, fondo.b),
    };
    let numeros = ajustes
        .gutter_foreground
        .map(|c| sobre(c, fondo))
        .into_iter()
        .chain([apagado])
        .find(|c| se_distingue(*c, fondo))
        .unwrap_or(texto);
    Some(ColoresEditor { fondo, texto, numeros })
}

fn fragmento(estilo: Style, texto: &str) -> Fragmento {
    let canal = |v: u8| f64::from(v) / 255.0;
    let c = estilo.foreground;
    Fragmento {
        texto: texto.to_string(),
        color: Color {
            r: canal(c.r),
            g: canal(c.g),
            b: canal(c.b),
        },
        negrita: estilo.font_style.contains(FontStyle::BOLD),
        cursiva: estilo.font_style.contains(FontStyle::ITALIC),
    }
}

/// `codigo` por trozos de un color. Vacío si no se conoce `lenguaje` o `nombre_de_tema`.
pub(crate) fn resaltar(codigo: &str, lenguaje: &str, nombre_de_tema: &str) -> Vec<Fragmento> {
    let gramaticas = gramaticas();
    let (Some(gramatica), Some(tema)) = (gramaticas.find_syntax_by_token(lenguaje), tema(nombre_de_tema)) else {
        return Vec::new();
    };
    let mut coloreador = HighlightLines::new(gramatica, tema);
    let mut fragmentos = Vec::new();
    for linea in LinesWithEndings::from(codigo) {
        let Ok(trozos) = coloreador.highlight_line(linea, gramaticas) else {
            return Vec::new();
        };
        fragmentos.extend(trozos.into_iter().map(|(estilo, texto)| fragmento(estilo, texto)));
    }
    fragmentos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_lenguaje_conocido_da_varios_colores_y_el_mismo_texto() {
        let codigo = "fn main() { // saludo\n    let x = \"hola\";\n}\n";
        let f = resaltar(codigo, "rust", "OneHalfDark");
        assert_eq!(f.iter().map(|t| t.texto.as_str()).collect::<String>(), codigo);
        let mut colores: Vec<String> = f.iter().map(|t| format!("{:?}", t.color)).collect();
        colores.sort();
        colores.dedup();
        assert!(colores.len() >= 3, "{colores:?}");
    }

    #[test]
    fn entiende_los_lenguajes_que_mas_usara_el_documento() {
        for lenguaje in [
            "rust",
            "python",
            "cpp",
            "c",
            "dart",
            "bash",
            "json",
            "toml",
            "typescript",
            "latex",
            "sql",
            "yaml",
        ] {
            assert!(!resaltar("a = 1\n", lenguaje, "OneHalfDark").is_empty(), "{lenguaje}");
        }
    }

    #[test]
    fn un_lenguaje_desconocido_no_inventa_nada() {
        assert!(resaltar("a", "klingon", "OneHalfDark").is_empty());
        assert!(resaltar("a", "rust", "tema inventado").is_empty());
    }

    #[test]
    fn el_tema_da_los_colores_del_panel_y_se_nombra_sin_cuidado() {
        let oscuro = colores_de_editor("OneHalfDark").unwrap();
        assert!(oscuro.fondo.r < 0.3 && oscuro.texto.r > 0.5, "{oscuro:?}");
        let claro = colores_de_editor("one half light").unwrap();
        assert!(claro.fondo.r > 0.9 && claro.texto.r < 0.5, "{claro:?}");
        assert!(colores_de_editor("nord").is_some());
        assert!(colores_de_editor("dracula").is_some());
        assert!(colores_de_editor("tema inventado").is_none());
    }

    #[test]
    fn los_numeros_quedan_entre_el_texto_y_el_fondo_y_se_leen_en_cualquier_tema() {
        let c = colores_de_editor("OneHalfDark").unwrap();
        assert!(c.numeros.r > c.fondo.r && c.numeros.r < c.texto.r, "{c:?}");
        for tema in EmbeddedLazyThemeSet::theme_names() {
            if let Some(c) = colores_de_editor(tema.as_name()) {
                assert!(
                    se_distingue(c.numeros, c.fondo) || !se_distingue(c.texto, c.fondo),
                    "{tema:?}: {c:?}"
                );
            }
        }
    }
}
