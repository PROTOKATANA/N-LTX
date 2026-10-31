//! Utilidades sobre el texto de la fuente: no saben nada del lector.

use super::*;

/// El tamaño de una orden como `\\tiny`, en tanto por ciento del normal: los de LaTeX a 10 pt.
fn escala_de(orden: &str) -> Option<u16> {
    let escala = match orden {
        "tiny" => 50,
        "scriptsize" => 70,
        "footnotesize" => 80,
        "small" => 90,
        "normalsize" => 100,
        "large" => 120,
        "Large" => 144,
        "LARGE" => 173,
        "huge" => 207,
        "Huge" => 249,
        _ => return None,
    };
    Some(escala)
}

/// El código entre el texto: un poco menor, porque la letra de código es más ancha que la romana.
pub(super) fn a_mono(estilo: &mut Estilo) {
    estilo.mono = true;
    estilo.escala = estilo.escala.or(Some(90));
}

/// Lo que hace una orden que cambia el estilo de lo que le sigue, hasta el fin de su grupo.
pub(super) fn declaracion(orden: &str, mut estilo: Estilo) -> Option<Estilo> {
    match orden {
        "bfseries" | "bf" => estilo.negrita = true,
        "itshape" | "it" | "em" | "slshape" | "sl" => estilo.cursiva = true,
        "ttfamily" | "tt" => a_mono(&mut estilo),
        "mdseries" => estilo.negrita = false,
        "upshape" => estilo.cursiva = false,
        _ => estilo.escala = Some(escala_de(orden)?),
    }
    Some(estilo)
}

/// Los `\\definecolor{nombre}{modelo}{valor}` de un preámbulo.
pub(super) fn colores_definidos(preambulo: &str) -> Vec<ColorDefinido> {
    const ORDEN: &str = "\\definecolor";
    let argumento = |t: &'_ str| -> Option<(String, usize)> {
        let abre = t.find('{')?;
        let fin = cierre(&t[abre..], '{', '}')?;
        Some((t[abre + 1..abre + fin].trim().to_string(), abre + fin + 1))
    };
    preambulo
        .match_indices(ORDEN)
        .filter_map(|(i, _)| {
            let t = &preambulo[i + ORDEN.len()..];
            let (nombre, a) = argumento(t)?;
            let (modelo, b) = argumento(&t[a..])?;
            let (valor, _) = argumento(&t[a + b..])?;
            Some(ColorDefinido { nombre, modelo, valor })
        })
        .collect()
}

pub(super) fn cerrar_parrafo(tramos: &mut Vec<Tramo>, salida: &mut Vec<Bloque>) {
    let mut parrafo = std::mem::take(tramos);
    limpiar_bordes(&mut parrafo);
    if !parrafo.is_empty() {
        salida.push(Bloque::Parrafo(parrafo));
    }
}

/// Quita el blanco del principio y del final de un párrafo.
pub(super) fn limpiar_bordes(tramos: &mut Vec<Tramo>) {
    if let Some(Tramo::Texto(t, _)) = tramos.first_mut() {
        *t = t.trim_start().to_string();
        if t.is_empty() {
            tramos.remove(0);
        }
    }
    if let Some(Tramo::Texto(t, _)) = tramos.last_mut() {
        *t = t.trim_end().to_string();
        if t.is_empty() {
            tramos.pop();
        }
    }
}

/// Añade texto al último tramo si tiene el mismo estilo, sin repetir blancos.
pub(super) fn empujar(tramos: &mut Vec<Tramo>, texto: &str, estilo: Estilo) {
    if let Some(Tramo::Texto(t, e)) = tramos.last_mut()
        && *e == estilo
    {
        if !(texto == " " && t.ends_with(' ')) {
            t.push_str(texto);
        }
        return;
    }
    tramos.push(Tramo::Texto(texto.to_string(), estilo));
}

/// Añade tramos ya leídos, sin partir el texto contiguo del mismo estilo.
pub(super) fn fundir(tramos: &mut Vec<Tramo>, nuevos: Vec<Tramo>) {
    for tramo in nuevos {
        match tramo {
            Tramo::Texto(texto, estilo) => empujar(tramos, &texto, estilo),
            otro => tramos.push(otro),
        }
    }
}

/// Posición del cierre que empareja la apertura con la que empieza `t`.
pub(super) fn cierre(t: &str, abre: char, cierra: char) -> Option<usize> {
    let mut nivel = 0usize;
    let mut escapado = false;
    for (i, c) in t.char_indices() {
        if escapado {
            escapado = false;
        } else if c == '\\' {
            escapado = true;
        } else if c == abre {
            nivel += 1;
        } else if c == cierra {
            nivel = nivel.saturating_sub(1);
            if nivel == 0 {
                return Some(i);
            }
        }
    }
    None
}

/// El `$` que cierra una fórmula en línea: no cuenta el escapado.
pub(super) fn dolar_de_cierre(t: &str) -> Option<usize> {
    let mut escapado = false;
    for (i, c) in t.char_indices() {
        if escapado {
            escapado = false;
        } else if c == '\\' {
            escapado = true;
        } else if c == '$' {
            return Some(i);
        }
    }
    None
}

/// Hasta el `;` que termina una instrucción TikZ, incluido, fuera de llaves.
pub(super) fn fin_de_instruccion(t: &str) -> usize {
    let mut nivel = 0usize;
    for (i, c) in t.char_indices() {
        match c {
            '{' => nivel += 1,
            '}' => nivel = nivel.saturating_sub(1),
            ';' if nivel == 0 => return i + 1,
            _ => {}
        }
    }
    t.len()
}

/// Parte `t` por `separador`, fuera de llaves y sin contar los escapados.
pub(super) fn partir<'t>(t: &'t str, separador: &str) -> Vec<&'t str> {
    let (mut partes, mut inicio, mut nivel, mut i) = (Vec::new(), 0, 0usize, 0);
    let bytes = t.as_bytes();
    while i < t.len() {
        let resto = &t[i..];
        if nivel == 0 && resto.starts_with(separador) {
            partes.push(&t[inicio..i]);
            i += separador.len();
            inicio = i;
            continue;
        }
        match bytes[i] {
            b'\\' => {
                i += 1;
                while i < t.len() && !t.is_char_boundary(i + 1) {
                    i += 1;
                }
            }
            b'{' => nivel += 1,
            b'}' => nivel = nivel.saturating_sub(1),
            _ => {}
        }
        i += 1;
        while i < t.len() && !t.is_char_boundary(i) {
            i += 1;
        }
    }
    partes.push(&t[inicio.min(t.len())..]);
    partes
}

pub(super) fn sin_reglas(fila: &str) -> String {
    let mut limpia = fila.to_string();
    for regla in ["\\hline", "\\toprule", "\\midrule", "\\bottomrule"] {
        limpia = limpia.replace(regla, "");
    }
    while let Some(i) = limpia.find("\\cline{") {
        let fin = limpia[i..].find('}').map_or(limpia.len(), |n| i + n + 1);
        limpia.replace_range(i..fin, "");
    }
    limpia
}

pub(super) fn acentuar(acento: char, base: &str) -> Option<String> {
    let c = base.trim_start_matches('\\').chars().next()?;
    let tabla: &[(char, char)] = match acento {
        '\'' => &[
            ('a', 'á'),
            ('e', 'é'),
            ('i', 'í'),
            ('o', 'ó'),
            ('u', 'ú'),
            ('A', 'Á'),
            ('E', 'É'),
            ('I', 'Í'),
            ('O', 'Ó'),
            ('U', 'Ú'),
        ],
        '`' => &[('a', 'à'), ('e', 'è'), ('i', 'ì'), ('o', 'ò'), ('u', 'ù')],
        '^' => &[('a', 'â'), ('e', 'ê'), ('i', 'î'), ('o', 'ô'), ('u', 'û')],
        '~' => &[('n', 'ñ'), ('N', 'Ñ'), ('a', 'ã'), ('o', 'õ')],
        '"' => &[('a', 'ä'), ('e', 'ë'), ('i', 'ï'), ('o', 'ö'), ('u', 'ü'), ('U', 'Ü')],
        _ => return None,
    };
    tabla.iter().find(|(a, _)| *a == c).map(|(_, b)| b.to_string())
}
