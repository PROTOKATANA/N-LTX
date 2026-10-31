//! Fórmulas entre dólares, con estas reglas:
//! - `\$` es un dólar literal;
//! - `$$…$$` en una misma línea es una fórmula en bloque;
//! - `$…$` en una misma línea, sin blanco tras la apertura ni antes del
//!   cierre; si el cierre va seguido de una cifra, solo vale si dentro hay
//!   matemáticas (`\`, `^`, `_`, llaves, `=`): así «US$100 … US$106» no es
//!   una fórmula pero «$\sim$10 K» sí. Si no, el `$` es texto.

/// Las fórmulas de `texto`, sin sus delimitadores.
pub fn formulas(texto: &str) -> Vec<&str> {
    let b = texto.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while let Some(p) = dolar(texto, i) {
        let fin_linea = texto[p..].find('\n').map_or(texto.len(), |n| p + n);
        let hallada = if b.get(p + 1) == Some(&b'$') {
            cierre_doble(texto, p + 2, fin_linea)
                .filter(|&c| !texto[p + 2..c].trim().is_empty())
                .map(|c| (p + 2, c, c + 2))
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
                .map(|c| (p + 1, c, c + 1))
        };
        match hallada {
            Some((ini, fin, siguiente)) => {
                out.push(&texto[ini..fin]);
                i = siguiente;
            }
            None => i = p + 1,
        }
    }
    out
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
