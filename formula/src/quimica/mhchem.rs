//! mhchem (`\\ce{…}`) → LaTeX matemático.

use super::Quimica;

#[derive(Clone, Copy, Debug, Default)]
pub struct Mhchem;

impl Quimica for Mhchem {
    fn a_latex(&self, notacion: &str) -> String {
        mhchem(notacion)
    }
}

/// Traduce la notación química de mhchem (`\ce{…}`) a LaTeX matemático:
/// símbolos de elementos rectos, cifras tras un elemento como subíndice,
/// cargas («Fe3+», «Cl-») como superíndice, «->»/«<=>»… como flechas (con
/// etiquetas «->[encima][debajo]») y « + » como operador.
fn mhchem(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    // ¿lo anterior es parte de una especie (letra, cifra, paréntesis…)?
    let mut tras_especie = false;
    // longitud de `out` justo tras cerrar un índice «^{…}»/«_{…}»
    let mut fin_indice = usize::MAX;
    let corchete = |i: &mut usize| -> Option<String> {
        if c.get(*i) != Some(&'[') {
            return None;
        }
        let mut nivel = 0;
        let ini = *i + 1;
        while *i < c.len() {
            match c[*i] {
                '[' => nivel += 1,
                ']' => {
                    nivel -= 1;
                    if nivel == 0 {
                        let dentro: String = c[ini..*i].iter().collect();
                        *i += 1;
                        return Some(dentro);
                    }
                }
                _ => {}
            }
            *i += 1;
        }
        None
    };
    while i < c.len() {
        let resto: String = c[i..c.len().min(i + 4)].iter().collect();
        let flecha = [
            ("<=>", "\\rightleftharpoons"),
            ("<->", "\\leftrightarrow"),
            ("->", "\\rightarrow"),
            ("<-", "\\leftarrow"),
        ]
        .into_iter()
        .find(|(p, _)| resto.starts_with(p));
        if let Some((patron, cmd)) = flecha {
            i += patron.len();
            let encima = corchete(&mut i);
            let debajo = corchete(&mut i);
            if encima.is_some() || debajo.is_some() {
                let x = match cmd {
                    "\\leftarrow" => "\\xleftarrow",
                    "\\leftrightarrow" => "\\xleftrightarrow",
                    "\\rightleftharpoons" => "\\xrightleftharpoons",
                    _ => "\\xrightarrow",
                };
                out.push_str(&format!(
                    " {x}[{}]{{{}}} ",
                    mhchem(&debajo.unwrap_or_default()),
                    mhchem(&encima.unwrap_or_default())
                ));
            } else {
                out.push_str(&format!(" {cmd} "));
            }
            tras_especie = false;
            continue;
        }
        let ch = c[i];
        match ch {
            '\\' => {
                // comando: se copia tal cual
                out.push(ch);
                i += 1;
                let ini = i;
                while i < c.len() && c[i].is_ascii_alphabetic() {
                    out.push(c[i]);
                    i += 1;
                }
                if i == ini {
                    // «\ », «\,»…: un carácter
                    if i < c.len() {
                        out.push(c[i]);
                        i += 1;
                    }
                } else {
                    // el argumento de \ce (anidado), \text… se copia sin
                    // traducir: ya lo interpreta su propio comando
                    let nombre: String = c[ini..i].iter().collect();
                    if matches!(
                        nombre.as_str(),
                        "ce" | "pu" | "text" | "mathrm" | "mbox" | "textrm" | "begin" | "end"
                    ) {
                        while i < c.len() && c[i].is_whitespace() {
                            i += 1;
                        }
                        if c.get(i) == Some(&'{') {
                            let mut nivel = 0;
                            while i < c.len() {
                                out.push(c[i]);
                                match c[i] {
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
                    } else {
                        out.push(' ');
                    }
                }
                tras_especie = false;
                continue;
            }
            '^' if (i == 0 || c[i - 1].is_whitespace())
                && c.get(i + 1).is_none_or(|x| x.is_whitespace() || *x == '}') =>
            {
                // « ^ » suelto: desprende gas
                out.push_str(" \\uparrow ");
                i += 1;
                tras_especie = false;
                continue;
            }
            '^' | '_' => {
                // al inicio de un término son prefijos de isótopo (²²⁶₈₈Ra):
                // van sobre un grupo vacío, no sobre lo anterior (+, →…); el
                // segundo prefijo comparte el grupo del primero
                if !tras_especie && out.len() != fin_indice {
                    out.push_str("{}");
                }
                out.push(ch);
                i += 1;
                if c.get(i) == Some(&'{') {
                    let mut nivel = 0;
                    while i < c.len() {
                        out.push(c[i]);
                        match c[i] {
                            '{' => nivel += 1,
                            '}' => nivel -= 1,
                            _ => {}
                        }
                        i += 1;
                        if nivel == 0 {
                            break;
                        }
                    }
                    fin_indice = out.len();
                    continue;
                }
                // «^3+», «^-»: la carga entera como índice
                if i < c.len() && c[i] != '{' {
                    let ini = i;
                    while i < c.len() && (c[i].is_ascii_digit() || c[i] == '+' || c[i] == '-') {
                        i += 1;
                    }
                    if i > ini {
                        let carga: String = c[ini..i].iter().map(|&x| if x == '-' { '−' } else { x }).collect();
                        out.push_str(&format!("{{{carga}}}"));
                    }
                }
                tras_especie = false;
                continue;
            }
            _ if ch.is_ascii_alphabetic() => {
                let ini = i;
                while i < c.len() && c[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let nombre: String = c[ini..i].iter().collect();
                let suelta =
                    (ini == 0 || c[ini - 1].is_whitespace()) && c.get(i).is_none_or(|x| x.is_whitespace() || *x == '}');
                if nombre == "v" && suelta {
                    // « v » suelta: precipita
                    out.push_str(" \\downarrow ");
                    tras_especie = false;
                } else {
                    out.push_str(&format!("\\mathrm{{{nombre}}}"));
                    tras_especie = true;
                }
                continue;
            }
            _ if ch.is_ascii_digit() && tras_especie => {
                let ini = i;
                while i < c.len() && c[i].is_ascii_digit() {
                    i += 1;
                }
                let cifras: String = c[ini..i].iter().collect();
                // cifras seguidas de signo al final de la especie: carga
                if i < c.len()
                    && (c[i] == '+' || c[i] == '-')
                    && c.get(i + 1).is_none_or(|x| x.is_whitespace() || *x == '}')
                {
                    let signo = if c[i] == '-' { '−' } else { '+' };
                    out.push_str(&format!("^{{{cifras}{signo}}}"));
                    i += 1;
                } else {
                    out.push_str(&format!("_{{{cifras}}}"));
                }
                continue;
            }
            '+' | '-' if tras_especie && c.get(i + 1).is_none_or(|x| x.is_whitespace() || *x == '}') => {
                // carga sin cifras: «Na+», «Cl-»
                out.push_str(if ch == '-' { "^{−}" } else { "^{+}" });
                i += 1;
                continue;
            }
            '.' | '*' if tras_especie => {
                out.push_str("\\cdot ");
                tras_especie = false;
            }
            ')' | ']' => {
                out.push(ch);
                tras_especie = true;
            }
            _ if ch.is_whitespace() => {
                // un espacio entre palabras («42.2 min») se respeta; alrededor
                // de operadores y flechas ya hay espaciado matemático
                let siguiente = c[i..].iter().find(|x| !x.is_whitespace()).copied();
                let previo = out.trim_end().chars().last();
                let junto_a_operador = matches!(siguiente, None | Some('+' | '=' | '<' | '-' | '}'))
                    || matches!(previo, None | Some('+' | '=' | '{'))
                    || out.trim_end().ends_with("arrow")
                    || out.trim_end().ends_with("harpoons")
                    || out.trim_end().ends_with('}') && out.trim_end().contains("\\x");
                out.push_str(if junto_a_operador { " " } else { "\\ " });
                tras_especie = false;
            }
            _ => {
                out.push(ch);
                tras_especie = false;
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mhchem_basico() {
        assert_eq!(mhchem("H2O"), "\\mathrm{H}_{2}\\mathrm{O}");
        assert_eq!(mhchem("Fe3+"), "\\mathrm{Fe}^{3+}");
        assert_eq!(mhchem("Cl-"), "\\mathrm{Cl}^{−}");
        assert!(mhchem("2H2 + O2 -> 2H2O").contains("\\rightarrow"));
    }
}
