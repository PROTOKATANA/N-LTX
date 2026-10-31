//! Validador que lee la fórmula con un `Lector` inyectado y examina el árbol.

use super::{Validador, Veredicto};
use crate::lector::Lector;
use crate::modelo::{Nodo, recorrer};

pub struct ValidadorPorArbol<L> {
    lector: L,
}

impl<L: Lector> ValidadorPorArbol<L> {
    pub fn new(lector: L) -> Self {
        Self { lector }
    }
}

impl<L: Lector> Validador for ValidadorPorArbol<L> {
    fn validar(&self, latex: &str) -> Veredicto {
        if parece_prosa(latex) {
            return Veredicto::Prosa;
        }
        match self.lector.leer(latex) {
            Err(e) => Veredicto::Malformada(e),
            Ok(nodos) => {
                let mut desconocidos = Vec::new();
                recorrer(&nodos, &mut |n| {
                    if let Nodo::Desconocido(nombre) = n {
                        desconocidos.push(nombre.clone());
                    }
                });
                if desconocidos.is_empty() {
                    Veredicto::Valida
                } else {
                    Veredicto::Desconocidos(desconocidos)
                }
            }
        }
    }
}

/// Dos palabras seguidas de tres o más letras, separadas solo por blancos y
/// fuera de comandos y de sus argumentos de texto, son prosa: en una fórmula
/// las palabras van en `\text{…}` o son comandos (`\sin`, `\max`).
fn parece_prosa(latex: &str) -> bool {
    let c: Vec<char> = latex.chars().collect();
    let mut i = 0;
    let mut previa_larga = false; // la última palabra tenía ≥3 letras…
    let mut solo_blancos = false; // …y desde entonces solo ha habido blancos
    while i < c.len() {
        if c[i] == '\\' {
            let ini = i + 1;
            let mut j = ini;
            while j < c.len() && c[j].is_ascii_alphabetic() {
                j += 1;
            }
            let nombre: String = c[ini..j].iter().collect();
            i = j.max(ini + 1);
            if matches!(
                nombre.as_str(),
                "text" | "textrm" | "textit" | "textbf" | "mbox" | "operatorname" | "ce" | "pu"
            ) {
                while i < c.len() && c[i].is_whitespace() {
                    i += 1;
                }
                if c.get(i) == Some(&'{') {
                    let mut nivel = 0;
                    while i < c.len() {
                        match c[i] {
                            '\\' => i += 1,
                            '{' => nivel += 1,
                            '}' => nivel -= 1,
                            _ => {}
                        }
                        i += 1;
                        if nivel == 0 {
                            break;
                        }
                    }
                }
            }
            previa_larga = false;
            continue;
        }
        if c[i].is_ascii_alphabetic() {
            let ini = i;
            while i < c.len() && c[i].is_ascii_alphabetic() {
                i += 1;
            }
            let larga = i - ini >= 3;
            if larga && previa_larga && solo_blancos {
                return true;
            }
            previa_larga = larga;
            solo_blancos = false;
            continue;
        }
        if c[i].is_whitespace() {
            solo_blancos = previa_larga;
        } else {
            previa_larga = false;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Lector falso: devuelve lo que se le diga.
    struct LectorFijo(Result<Vec<Nodo>, String>);
    impl Lector for LectorFijo {
        fn leer(&self, _: &str) -> Result<Vec<Nodo>, String> {
            self.0.clone()
        }
    }

    #[test]
    fn veredictos_segun_el_lector() {
        let ok = ValidadorPorArbol::new(LectorFijo(Ok(vec![Nodo::Espacio(1.0)])));
        assert_eq!(ok.validar("x"), Veredicto::Valida);
        let mal = ValidadorPorArbol::new(LectorFijo(Err("llave".into())));
        assert_eq!(mal.validar("x"), Veredicto::Malformada("llave".into()));
        let raro = ValidadorPorArbol::new(LectorFijo(Ok(vec![Nodo::Grupo(vec![Nodo::Desconocido("foo".into())])])));
        assert_eq!(raro.validar("x"), Veredicto::Desconocidos(vec!["foo".into()]));
    }

    #[test]
    fn la_prosa_no_es_formula() {
        let v = ValidadorPorArbol::new(LectorFijo(Ok(Vec::new())));
        assert_eq!(
            v.validar("100 billion. According to Forbes, his net worth"),
            Veredicto::Prosa
        );
        assert_eq!(v.validar("5,000 for the"), Veredicto::Prosa);
        for formula in [
            "\\sin x + \\max_{i} a_i",
            "x{\\text{ for all real numbers }}y",
            "a b c d",
            "\\operatorname{arg max} f",
            "{\\ce {2H2 + O2 -> 2H2O}}",
            "\\mathrm{Re}\\, z",
        ] {
            assert_eq!(v.validar(formula), Veredicto::Valida, "{formula}");
        }
    }
}
