//! LaTeX ⇄ unidades léxicas. `escribir(tokenizar(x)) == x` siempre.

use super::Token;

/// Comandos cuyos argumentos son texto (o notación propia) y no matemáticas:
/// cuántos argumentos `{…}` se toman literales.
fn argumentos_literales(nombre: &str) -> usize {
    match nombre {
        "text" | "textrm" | "textit" | "textbf" | "textsf" | "texttt" | "textnormal" | "textup" | "mbox" | "hbox"
        | "operatorname" | "ce" | "pu" | "label" | "tag" | "color" | "textcolor" | "colorbox" | "hspace" | "vspace"
        | "mspace" | "begin" | "end" | "url" | "href" => 1,
        "definecolor" => 3,
        _ => 0,
    }
}

pub fn tokenizar(latex: &str) -> Vec<Token> {
    let c: Vec<char> = latex.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch == '\\' {
            let ini = i + 1;
            let mut j = ini;
            while j < c.len() && c[j].is_ascii_alphabetic() {
                j += 1;
            }
            if j > ini {
                let nombre: String = c[ini..j].iter().collect();
                let literales = argumentos_literales(&nombre);
                out.push(Token::Palabra(nombre));
                i = j;
                for _ in 0..literales {
                    // blancos y un «*» opcional (\operatorname*) antes del argumento
                    let mut k = i;
                    while k < c.len() && c[k].is_whitespace() {
                        k += 1;
                    }
                    if k < c.len() && c[k] == '*' {
                        k += 1;
                        while k < c.len() && c[k].is_whitespace() {
                            k += 1;
                        }
                    }
                    let Some(fin) = grupo(&c, k) else { break };
                    if k > i {
                        empujar_texto(&mut out, &c[i..k]);
                    }
                    out.push(Token::Literal(c[k..fin].iter().collect()));
                    i = fin;
                }
            } else if ini < c.len() {
                out.push(Token::Simbolo(c[ini]));
                i = ini + 1;
            } else {
                out.push(Token::Caracter('\\'));
                i = ini;
            }
            continue;
        }
        let ini = i;
        if ch.is_whitespace() {
            while i < c.len() && c[i].is_whitespace() {
                i += 1;
            }
            out.push(Token::Blanco(c[ini..i].iter().collect()));
        } else {
            out.push(Token::Caracter(ch));
            i += 1;
        }
    }
    out
}

/// Blancos y caracteres sueltos (entre un comando literal y su argumento).
fn empujar_texto(out: &mut Vec<Token>, cs: &[char]) {
    let mut i = 0;
    while i < cs.len() {
        if cs[i].is_whitespace() {
            let ini = i;
            while i < cs.len() && cs[i].is_whitespace() {
                i += 1;
            }
            out.push(Token::Blanco(cs[ini..i].iter().collect()));
        } else {
            out.push(Token::Caracter(cs[i]));
            i += 1;
        }
    }
}

/// Fin (exclusivo) del grupo `{…}` que empieza en `ini`; las llaves escapadas
/// (`\{`, `\}`) no cuentan.
fn grupo(c: &[char], ini: usize) -> Option<usize> {
    if c.get(ini) != Some(&'{') {
        return None;
    }
    let mut nivel = 0;
    let mut i = ini;
    while i < c.len() {
        match c[i] {
            '\\' => i += 1,
            '{' => nivel += 1,
            '}' => {
                nivel -= 1;
                if nivel == 0 {
                    return Some(i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

pub fn escribir(tokens: &[Token]) -> String {
    let mut s = String::new();
    for t in tokens {
        match t {
            Token::Palabra(n) => {
                s.push('\\');
                s.push_str(n);
            }
            Token::Simbolo(ch) => {
                s.push('\\');
                s.push(*ch);
            }
            Token::Caracter(ch) => s.push(*ch),
            Token::Blanco(b) | Token::Literal(b) => s.push_str(b),
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ida_y_vuelta_exacta() {
        for f in [
            "\\frac {1}{2}",
            "\\alpha _{1}+\\text{ for all } x",
            "{\\ce {2H2 + O2 -> 2H2O}}",
            "\\operatorname* {arg\\,max} _x",
            "a\\\\ [2pt] b\\,c\\{d\\}",
            "\\",
            "\\text{sin cerrar",
        ] {
            assert_eq!(escribir(&tokenizar(f)), f);
        }
    }

    #[test]
    fn argumentos_de_texto_son_literales() {
        let t = tokenizar("\\text {a  b}x");
        assert_eq!(
            t,
            vec![
                Token::Palabra("text".into()),
                Token::Blanco(" ".into()),
                Token::Literal("{a  b}".into()),
                Token::Caracter('x'),
            ]
        );
    }
}
