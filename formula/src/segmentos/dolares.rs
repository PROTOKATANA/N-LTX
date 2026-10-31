//! Fórmulas entre dólares, con estas reglas:
//! - `\$` es un dólar literal;
//! - `$$…$$` en una misma línea es una fórmula en bloque;
//! - `$…$` en una misma línea, sin blanco tras la apertura ni antes del
//!   cierre; si el cierre va seguido de una cifra, solo vale si dentro hay
//!   matemáticas (`\`, `^`, `_`, llaves, `=`): así «US$100 … US$106» no es
//!   una fórmula pero «$\sim$10 K» sí. Si no, el `$` es texto.

use super::{Segmentador, Segmento};

#[derive(Clone, Copy, Debug, Default)]
pub struct SegmentadorDolares;

impl Segmentador for SegmentadorDolares {
    fn segmentar<'a>(&self, texto: &'a str) -> Vec<Segmento<'a>> {
        let b = texto.as_bytes();
        let mut out = Vec::new();
        let mut prosa_desde = 0;
        let mut i = 0;
        while let Some(p) = dolar(texto, i) {
            let fin_linea = texto[p..].find('\n').map_or(texto.len(), |n| p + n);
            let hallada = if b.get(p + 1) == Some(&b'$') {
                cierre_doble(texto, p + 2, fin_linea)
                    .filter(|&c| !texto[p + 2..c].trim().is_empty())
                    .map(|c| (p + 2, c, c + 2, true))
            } else {
                dolar(texto, p + 1)
                    .filter(|&c| c < fin_linea)
                    .filter(|&c| {
                        let dentro = &texto[p + 1..c];
                        !dentro.is_empty()
                            && !dentro.starts_with(char::is_whitespace)
                            && !dentro.ends_with(char::is_whitespace)
                            && (!b.get(c + 1).is_some_and(u8::is_ascii_digit)
                                || dentro.contains(['\\', '^', '_', '{', '}', '=']))
                    })
                    .map(|c| (p + 1, c, c + 1, false))
            };
            match hallada {
                Some((ini, fin, siguiente, bloque)) => {
                    if p > prosa_desde {
                        out.push(Segmento::Prosa(&texto[prosa_desde..p]));
                    }
                    out.push(Segmento::Formula {
                        latex: &texto[ini..fin],
                        bloque,
                    });
                    prosa_desde = siguiente;
                    i = siguiente;
                }
                None => i = p + 1,
            }
        }
        if prosa_desde < texto.len() {
            out.push(Segmento::Prosa(&texto[prosa_desde..]));
        }
        out
    }
}

/// Primer `$` sin escapar desde `desde`.
fn dolar(t: &str, desde: usize) -> Option<usize> {
    let b = t.as_bytes();
    let mut i = desde;
    while let Some(r) = t.get(i..)?.find('$') {
        let p = i + r;
        if b[..p].iter().rev().take_while(|&&c| c == b'\\').count() % 2 == 0 {
            return Some(p);
        }
        i = p + 1;
    }
    None
}

/// Primer `$$` sin escapar entre `desde` y `hasta`.
fn cierre_doble(t: &str, desde: usize, hasta: usize) -> Option<usize> {
    let mut i = desde;
    while let Some(p) = dolar(t, i).filter(|&p| p + 1 < hasta) {
        if t.as_bytes()[p + 1] == b'$' {
            return Some(p);
        }
        i = p + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::super::unir;
    use super::*;

    fn seg(t: &str) -> Vec<Segmento<'_>> {
        SegmentadorDolares.segmentar(t)
    }

    #[test]
    fn formulas_en_linea_y_en_bloque() {
        assert_eq!(
            seg("sea $x$ y\n$$y=2$$\nfin"),
            vec![
                Segmento::Prosa("sea "),
                Segmento::Formula {
                    latex: "x",
                    bloque: false
                },
                Segmento::Prosa(" y\n"),
                Segmento::Formula {
                    latex: "y=2",
                    bloque: true
                },
                Segmento::Prosa("\nfin"),
            ]
        );
    }

    #[test]
    fn dolares_que_no_son_formulas() {
        for t in [
            "costs $5 and $6 each",
            "surpassed US$100 billion. As of July, US$106.5 billion",
            "un \\$ suelto y $ x$ con blanco",
            "abre $ en una línea\ny cierra $ en otra",
        ] {
            assert!(seg(t).iter().all(|s| matches!(s, Segmento::Prosa(_))), "{t}");
        }
    }

    #[test]
    fn cierre_seguido_de_cifra() {
        // estilo de arXiv: la fórmula sí lo es aunque le siga una cifra
        assert_eq!(
            seg("at $\\sim$10 K and $\\sim$100 K"),
            vec![
                Segmento::Prosa("at "),
                Segmento::Formula {
                    latex: "\\sim",
                    bloque: false
                },
                Segmento::Prosa("10 K and "),
                Segmento::Formula {
                    latex: "\\sim",
                    bloque: false
                },
                Segmento::Prosa("100 K"),
            ]
        );
    }

    #[test]
    fn dolar_escapado_dentro_de_formula() {
        assert_eq!(
            seg("a $\\$1+x$ b"),
            vec![
                Segmento::Prosa("a "),
                Segmento::Formula {
                    latex: "\\$1+x",
                    bloque: false
                },
                Segmento::Prosa(" b"),
            ]
        );
    }

    #[test]
    fn unir_reconstruye_el_texto_exacto() {
        for t in [
            "sea $x$ y\n$$y=2$$\nfin",
            "costs $5 and $6",
            "$a$$b$ y $$c$$$d$",
            "$$",
            "$",
            "",
            "café $\\alpha$ ñ",
        ] {
            assert_eq!(unir(&seg(t), str::to_string), t);
        }
    }
}
