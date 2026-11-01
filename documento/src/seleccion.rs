//! Seleccionar y copiar. Lo que se copia es la fuente en bruto: un texto y una animación dan el
//! LaTeX de los dos. Estas funciones son puras: el estado lo lleva quien las usa.

use primitivas::{Ancla, Caja, Punto};

/// Entre dos cajas seguidas de una línea cabe esto, en altos de línea, y se ven como una sola.
const HUECO_DE_UNA_LINEA: f64 = 0.5;

/// Lo que cuesta alejarse de `p` hasta `c`: primero en vertical, para no saltar de línea.
fn distancia(c: &Caja, p: Punto) -> (f64, f64) {
    let fuera = |v: f64, a: f64, b: f64| (a - v).max(v - b).max(0.0);
    (fuera(p.y, c.y, c.y + c.alto), fuera(p.x, c.x, c.x + c.ancho))
}

/// El hueco entre anclas más cercano a `p`: `n` es justo antes de la ancla `n`.
pub fn cursor_en(anclas: &[Ancla], p: Punto) -> usize {
    let mas_cerca = anclas.iter().enumerate().min_by(|(_, a), (_, b)| {
        let (a, b) = (distancia(&a.caja, p), distancia(&b.caja, p));
        a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
    });
    let (Some((n, a)), Some(primera), Some(ultima)) = (mas_cerca, anclas.first(), anclas.last()) else {
        return 0;
    };
    if p.y < primera.caja.y {
        0
    } else if p.y > ultima.caja.y + ultima.caja.alto {
        anclas.len()
    } else if p.x >= a.caja.x + a.caja.ancho / 2.0 {
        n + 1
    } else {
        n
    }
}

/// La palabra de la ancla `n`, o esa sola ancla si no está en una: un rango de anclas. Las palabras
/// acaban en un blanco de la fuente, y lo que no se puede partir es una palabra por sí mismo.
pub fn palabra_en(anclas: &[Ancla], fuentes: &[&str], n: usize) -> (usize, usize) {
    let de_palabra = |a: &Ancla| {
        let fuente = fuentes.get(a.documento as usize).copied().unwrap_or("");
        let trozo = fuente.get(a.desde as usize..a.hasta as usize).unwrap_or("");
        trozo.chars().count() == 1 && !trozo.chars().all(|c| c.is_whitespace() || c.is_ascii_punctuation())
    };
    let Some(ancla) = anclas.get(n) else {
        return (n, n);
    };
    if !de_palabra(ancla) {
        return (n, n + 1);
    }
    let inicio = anclas[..n].iter().rposition(|a| !de_palabra(a)).map_or(0, |i| i + 1);
    let fin = anclas[n..]
        .iter()
        .position(|a| !de_palabra(a))
        .map_or(anclas.len(), |i| n + i);
    (inicio, fin)
}

/// Lo que hay que marcar en pantalla para las anclas `desde..hasta`: una caja por línea.
pub fn cajas(anclas: &[Ancla], desde: usize, hasta: usize) -> Vec<Caja> {
    let mut salida: Vec<Caja> = Vec::new();
    for a in anclas.get(desde..hasta).unwrap_or(&[]) {
        let c = a.caja;
        let seguida = salida.last_mut().filter(|u| {
            let fin = u.x + u.ancho;
            u.y == c.y && u.alto == c.alto && c.x <= fin + c.alto * HUECO_DE_UNA_LINEA && c.x >= u.x
        });
        match seguida {
            Some(u) => u.ancho = (c.x + c.ancho - u.x).max(u.ancho),
            None => salida.push(c),
        }
    }
    salida
}

/// Un trozo que deja un entorno a medias se amplía hasta cerrarlo: lo copiado debe compilar.
fn equilibrar(fuente: &str, mut a: usize, mut b: usize) -> (usize, usize) {
    const BEGIN: &str = "\\begin{";
    const END: &str = "\\end{";
    let trozo = &fuente[a..b];
    let (mut abiertos, mut huerfanos): (Vec<&str>, Vec<&str>) = (Vec::new(), Vec::new());
    let mut resto = trozo;
    while let Some(i) = resto.find(['\\']) {
        resto = &resto[i..];
        let (es_begin, largo) = if resto.starts_with(BEGIN) {
            (true, BEGIN.len())
        } else if resto.starts_with(END) {
            (false, END.len())
        } else {
            resto = &resto[1..];
            resto = resto
                .get(resto.chars().next().map_or(0, char::len_utf8)..)
                .unwrap_or("");
            continue;
        };
        let cola = &resto[largo..];
        let Some(fin) = cola.find('}') else { break };
        let nombre = &cola[..fin];
        if es_begin {
            abiertos.push(nombre);
        } else if abiertos.last() == Some(&nombre) {
            abiertos.pop();
        } else {
            huerfanos.push(nombre);
        }
        resto = &cola[fin + 1..];
    }
    for nombre in huerfanos {
        if let Some(i) = fuente[..a].rfind(&format!("{BEGIN}{nombre}}}")) {
            a = i;
        }
    }
    for nombre in abiertos.into_iter().rev() {
        let fin = format!("{END}{nombre}}}");
        if let Some(i) = fuente[b..].find(&fin) {
            b += i + fin.len();
        }
    }
    (a, b)
}

/// La fuente en bruto de las anclas `desde..hasta`. `fuentes` son los documentos de la serie, y de
/// cada uno sale un trozo: del primer byte seleccionado al último.
pub fn fuente_de(fuentes: &[&str], anclas: &[Ancla], desde: usize, hasta: usize) -> String {
    let elegidas = anclas.get(desde..hasta).unwrap_or(&[]);
    let mut trozos: Vec<(u32, u32, u32)> = Vec::new();
    for a in elegidas {
        match trozos.iter_mut().find(|(d, ..)| *d == a.documento) {
            Some(t) => (t.1, t.2) = (t.1.min(a.desde), t.2.max(a.hasta)),
            None => trozos.push((a.documento, a.desde, a.hasta)),
        }
    }
    let copiados = trozos.into_iter().filter_map(|(d, a, b)| {
        let fuente = *fuentes.get(d as usize)?;
        let (a, b) = equilibrar(fuente, a as usize, b as usize);
        fuente.get(a..b)
    });
    copiados.collect::<Vec<_>>().join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ancla(x: f64, y: f64, desde: u32, hasta: u32) -> Ancla {
        Ancla {
            caja: Caja {
                x,
                y,
                ancho: 10.0,
                alto: 20.0,
            },
            documento: 0,
            desde,
            hasta,
        }
    }

    /// Dos líneas de tres letras.
    fn lineas() -> Vec<Ancla> {
        let mut a = Vec::new();
        for (n, x) in [0.0, 10.0, 20.0].into_iter().enumerate() {
            a.push(ancla(x, 0.0, n as u32, n as u32 + 1));
        }
        for (n, x) in [0.0, 10.0, 20.0].into_iter().enumerate() {
            a.push(ancla(x, 20.0, n as u32 + 3, n as u32 + 4));
        }
        a
    }

    fn p(x: f64, y: f64) -> Punto {
        Punto { x, y }
    }

    #[test]
    fn el_cursor_cae_antes_o_despues_de_la_letra_segun_el_lado() {
        let a = lineas();
        assert_eq!(cursor_en(&a, p(12.0, 5.0)), 1);
        assert_eq!(cursor_en(&a, p(18.0, 5.0)), 2);
        assert_eq!(cursor_en(&a, p(12.0, 25.0)), 4);
    }

    #[test]
    fn fuera_del_texto_vale_lo_mas_cercano() {
        let a = lineas();
        assert_eq!(cursor_en(&a, p(500.0, 5.0)), 3, "a la derecha, al final de la línea");
        assert_eq!(cursor_en(&a, p(-5.0, 25.0)), 3, "a la izquierda, al principio");
        assert_eq!(cursor_en(&a, p(5.0, 900.0)), 6, "por debajo, al final");
        assert_eq!(cursor_en(&a, p(-5.0, -900.0)), 0, "por encima, al principio");
        assert_eq!(cursor_en(&[], p(0.0, 0.0)), 0);
    }

    #[test]
    fn una_caja_por_linea() {
        let a = lineas();
        let c = cajas(&a, 1, 5);
        assert_eq!(c.len(), 2);
        assert_eq!((c[0].x, c[0].ancho), (10.0, 20.0));
        assert_eq!((c[1].x, c[1].ancho), (0.0, 20.0));
        assert!(cajas(&a, 2, 2).is_empty());
    }

    #[test]
    fn se_copia_la_fuente_tal_cual() {
        let fuente = r"Hola \textbf{mundo} $x$ fin";
        let anclas = vec![ancla(0.0, 0.0, 0, 5), ancla(10.0, 0.0, 5, 19), ancla(20.0, 0.0, 20, 23)];
        assert_eq!(fuente_de(&[fuente], &anclas, 1, 2), r"\textbf{mundo}");
        assert_eq!(fuente_de(&[fuente], &anclas, 0, 3), r"Hola \textbf{mundo} $x$");
    }

    #[test]
    fn un_entorno_a_medias_se_completa() {
        let fuente = r"a \begin{center}b\end{center} c";
        let despues = fuente.find(" c").unwrap() as u32;
        let dentro = fuente.find("b\\end").unwrap() as u32;
        let hasta_c = vec![
            ancla(0.0, 0.0, dentro, dentro + 1),
            ancla(10.0, 0.0, despues, despues + 2),
        ];
        assert_eq!(fuente_de(&[fuente], &hasta_c, 0, 2), r"\begin{center}b\end{center} c");
        let desde_a = vec![ancla(0.0, 0.0, 0, 1), ancla(10.0, 0.0, dentro, dentro + 1)];
        assert_eq!(fuente_de(&[fuente], &desde_a, 0, 2), r"a \begin{center}b\end{center}");
    }

    #[test]
    fn de_una_serie_sale_un_trozo_por_documento() {
        let mut anclas = vec![ancla(0.0, 0.0, 0, 3), ancla(0.0, 20.0, 0, 3)];
        anclas[1].documento = 1;
        assert_eq!(fuente_de(&["uno", "dos"], &anclas, 0, 2), "uno\n\ndos");
    }

    #[test]
    fn una_palabra_llega_hasta_los_blancos() {
        let fuente = "ab cd";
        let anclas: Vec<Ancla> = (0..5).map(|n| ancla(n as f64 * 10.0, 0.0, n, n + 1)).collect();
        assert_eq!(palabra_en(&anclas, &[fuente], 3), (3, 5));
        assert_eq!(palabra_en(&anclas, &[fuente], 0), (0, 2));
        assert_eq!(palabra_en(&anclas, &[fuente], 2), (2, 3), "un blanco es él solo");
    }

    #[test]
    fn lo_indivisible_es_una_palabra_por_si_mismo() {
        let fuente = r"a $x$ b";
        let anclas = vec![ancla(0.0, 0.0, 0, 1), ancla(10.0, 0.0, 2, 5), ancla(20.0, 0.0, 6, 7)];
        assert_eq!(palabra_en(&anclas, &[fuente], 1), (1, 2));
    }

    #[test]
    fn una_seleccion_fuera_de_rango_no_rompe_nada() {
        let a = lineas();
        assert_eq!(fuente_de(&["abcdef"], &a, 9, 20), "");
        assert!(cajas(&a, 9, 20).is_empty());
        assert_eq!(palabra_en(&a, &["abcdef"], 99), (99, 99));
    }
}
