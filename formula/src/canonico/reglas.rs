//! Reglas de normalización. Cada una es una pieza independiente.

use super::{Regla, Token};

/// Quita los blancos, que en modo matemático no cuentan. Solo deja uno tras
/// una palabra de control seguida de una letra (`\alpha x`: sin él serían un
/// comando `\alphax`). Los argumentos de texto (`Token::Literal`) no se tocan.
#[derive(Clone, Copy, Debug, Default)]
pub struct Espacios;

impl Regla for Espacios {
    fn aplicar(&self, tokens: Vec<Token>) -> Vec<Token> {
        let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
        for (i, t) in tokens.iter().enumerate() {
            if let Token::Blanco(_) = t {
                let tras_palabra = matches!(out.last(), Some(Token::Palabra(_)));
                let antes_de_letra = matches!(
                    tokens[i + 1..].iter().find(|t| !matches!(t, Token::Blanco(_))),
                    Some(Token::Caracter(c)) if c.is_ascii_alphabetic()
                );
                if tras_palabra && antes_de_letra {
                    out.push(Token::Blanco(" ".into()));
                }
                continue;
            }
            out.push(t.clone());
        }
        out
    }
}

/// Sinónimos exactos de LaTeX (mismo símbolo, misma clase, mismo espaciado)
/// → una sola forma.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sinonimos;

impl Sinonimos {
    pub const TABLA: [(&'static str, &'static str); 9] = [
        ("le", "leq"),
        ("ge", "geq"),
        ("ne", "neq"),
        ("to", "rightarrow"),
        ("gets", "leftarrow"),
        ("land", "wedge"),
        ("lor", "vee"),
        ("lnot", "neg"),
        ("owns", "ni"),
    ];

    fn canonico(nombre: &str) -> Option<&'static str> {
        Self::TABLA.iter().find(|(de, _)| *de == nombre).map(|(_, a)| *a)
    }
}

impl Regla for Sinonimos {
    fn aplicar(&self, tokens: Vec<Token>) -> Vec<Token> {
        tokens
            .into_iter()
            .map(|t| match t {
                Token::Palabra(n) => Token::Palabra(Self::canonico(&n).map_or(n, str::to_string)),
                otro => otro,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::super::lexico::{escribir, tokenizar};
    use super::*;

    fn con(regla: &impl Regla, f: &str) -> String {
        escribir(&regla.aplicar(tokenizar(f)))
    }

    #[test]
    fn espacios_fuera_salvo_entre_comando_y_letra() {
        assert_eq!(con(&Espacios, "\\frac {1}{2}"), "\\frac{1}{2}");
        assert_eq!(con(&Espacios, "\\alpha _{1} x"), "\\alpha_{1}x");
        assert_eq!(con(&Espacios, "\\alpha  x"), "\\alpha x");
        assert_eq!(con(&Espacios, "\\mathbb {R} ^{n}"), "\\mathbb{R}^{n}");
        assert_eq!(con(&Espacios, "a\\ b"), "a\\ b");
        assert_eq!(con(&Espacios, "x{\\text{ for all }}y"), "x{\\text{ for all }}y");
        assert_eq!(con(&Espacios, "\\ce {2H2 + O2}"), "\\ce{2H2 + O2}");
    }

    #[test]
    fn sinonimos_a_una_forma() {
        assert_eq!(con(&Sinonimos, "a\\le b\\to c"), "a\\leq b\\rightarrow c");
        assert_eq!(con(&Sinonimos, "\\left\\lbrace"), "\\left\\lbrace");
    }
}
