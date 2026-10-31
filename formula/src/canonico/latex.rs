//! Canonizador de LaTeX: tokeniza, aplica las reglas inyectadas en orden y
//! vuelve a escribir.

use super::lexico::{escribir, tokenizar};
use super::{Canonizador, Regla};

pub struct CanonizadorLatex {
    reglas: Vec<Box<dyn Regla + Send + Sync>>,
}

impl CanonizadorLatex {
    pub fn new(reglas: Vec<Box<dyn Regla + Send + Sync>>) -> Self {
        Self { reglas }
    }
}

impl Canonizador for CanonizadorLatex {
    fn canonizar(&self, latex: &str) -> String {
        // Un «%» sin escapar abre un comentario hasta fin de línea: quitar
        // saltos de línea cambiaría qué queda comentado. Se deja tal cual.
        if comentario(latex) {
            return latex.to_string();
        }
        let tokens = self.reglas.iter().fold(tokenizar(latex), |t, regla| regla.aplicar(t));
        escribir(&tokens)
    }
}

fn comentario(latex: &str) -> bool {
    let b = latex.as_bytes();
    latex
        .match_indices('%')
        .any(|(p, _)| b[..p].iter().rev().take_while(|&&c| c == b'\\').count() % 2 == 0)
}

#[cfg(test)]
mod tests {
    use super::super::Token;
    use super::*;

    /// Regla falsa que deja constancia de que se le llamó y en qué orden.
    struct Marca(char);
    impl Regla for Marca {
        fn aplicar(&self, mut tokens: Vec<Token>) -> Vec<Token> {
            tokens.push(Token::Caracter(self.0));
            tokens
        }
    }

    #[test]
    fn aplica_las_reglas_inyectadas_en_orden() {
        let c = CanonizadorLatex::new(vec![Box::new(Marca('1')), Box::new(Marca('2'))]);
        assert_eq!(c.canonizar("x"), "x12");
    }

    #[test]
    fn sin_reglas_no_cambia_nada() {
        let c = CanonizadorLatex::new(Vec::new());
        assert_eq!(c.canonizar("\\frac {1}{2}"), "\\frac {1}{2}");
    }

    #[test]
    fn respeta_los_comentarios() {
        let c = CanonizadorLatex::new(vec![Box::new(Marca('!'))]);
        assert_eq!(c.canonizar("a % nota\nb"), "a % nota\nb");
        assert_eq!(c.canonizar("50\\%"), "50\\%!");
    }
}
