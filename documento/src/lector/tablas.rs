//! Lee una tabla: `tabular` y lo estándar de LaTeX (`\multicolumn`, `\multirow`, `\rowcolor`,
//! `\cellcolor`), con lo que LaTeX no tiene en sus opciones: `\begin{tabular}[estilo=…]{lcr}`.
//! Aquí solo se lee la rejilla; cómo se ve lo decide el estilo.

use super::*;
use crate::tema::Opciones;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alineado {
    Izquierda,
    Centro,
    Derecha,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Celda {
    pub tramos: Vec<Tramo>,
    /// Columnas y filas que ocupa.
    pub ancho: usize,
    pub alto: usize,
    pub alineado: Option<Alineado>,
    pub fondo: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fila {
    pub celdas: Vec<Celda>,
    pub fondo: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tabla {
    pub columnas: Vec<Alineado>,
    pub filas: Vec<Fila>,
    pub opciones: Opciones,
}

/// `clave=valor, clave={a,b}`, sin las llaves de los valores.
pub(super) fn leer_opciones(texto: &str) -> Opciones {
    let sin_llaves = |v: &str| {
        let v = v.trim();
        v.strip_prefix('{')
            .and_then(|r| r.strip_suffix('}'))
            .unwrap_or(v)
            .to_string()
    };
    partir(texto, ",")
        .into_iter()
        .map(str::trim)
        .filter(|o| !o.is_empty())
        .map(|o| {
            let (clave, valor) = o.split_once('=').unwrap_or((o, ""));
            (clave.trim().to_string(), sin_llaves(valor))
        })
        .collect()
}

/// El primer `{…}` de `t` y lo que le sigue.
fn argumento_de(t: &str) -> Option<(&str, &str)> {
    let t = t.trim_start();
    if !t.starts_with('{') {
        return None;
    }
    let fin = cierre(t, '{', '}')?;
    Some((&t[1..fin], &t[fin + 1..]))
}

/// `t` sin un `[…]` delante, si lo lleva, y lo que había dentro.
fn sin_opcional(t: &str) -> (Option<&str>, &str) {
    let recortado = t.trim_start();
    if !recortado.starts_with('[') {
        return (None, t);
    }
    match cierre(recortado, '[', ']') {
        Some(fin) => (Some(&recortado[1..fin]), &recortado[fin + 1..]),
        None => (None, t),
    }
}

/// Saca `orden[modelo]{color}` de `t`: lo que queda y el color, por su nombre. Un color por modelo
/// se escribe `modelo:valor`, y en HTML `#valor`.
fn sacar_color(t: &str, orden: &str) -> (String, Option<String>) {
    let Some(inicio) = t.find(orden) else {
        return (t.to_string(), None);
    };
    let tras = &t[inicio + orden.len()..];
    let (modelo, tras) = sin_opcional(tras);
    let Some((valor, resto)) = argumento_de(tras) else {
        return (t.to_string(), None);
    };
    let color = match modelo {
        Some("HTML") => format!("#{}", valor.trim()),
        Some(m) => format!("{m}:{}", valor.trim()),
        None => valor.trim().to_string(),
    };
    (format!("{}{resto}", &t[..inicio]), Some(color))
}

fn alineado_de(letra: char) -> Option<Alineado> {
    match letra {
        'l' | 'p' | 'm' | 'b' | 'X' => Some(Alineado::Izquierda),
        'c' => Some(Alineado::Centro),
        'r' => Some(Alineado::Derecha),
        _ => None,
    }
}

/// Las columnas de `{l|c|r}`, con `*{n}{…}` y sin lo que no decide la alineación (`@{}`, `p{3cm}`).
pub(super) fn leer_columnas(t: &str) -> Vec<Alineado> {
    let mut columnas = Vec::new();
    let mut resto = t;
    while let Some(c) = resto.chars().next() {
        resto = &resto[c.len_utf8()..];
        if c == '*' {
            let Some((veces, tras)) = argumento_de(resto) else {
                break;
            };
            let Some((dentro, tras)) = argumento_de(tras) else {
                break;
            };
            let repetida = leer_columnas(dentro);
            for _ in 0..veces.trim().parse::<usize>().unwrap_or(1).min(256) {
                columnas.extend_from_slice(&repetida);
            }
            resto = tras;
            continue;
        }
        if let Some(a) = alineado_de(c) {
            columnas.push(a);
        }
        // lo que va entre llaves es de esa columna: un ancho, un `@{}`
        if let Some((_, tras)) = argumento_de(resto) {
            resto = tras;
        }
    }
    columnas
}

impl<'a> Lector<'a> {
    pub(super) fn tabla(&mut self, nombre: &str) -> Bloque {
        let opciones = self.opcional().map(leer_opciones).unwrap_or_default();
        if nombre != "tabular" {
            self.argumento();
        }
        let columnas = leer_columnas(self.argumento());
        let cuerpo = self.hasta_fin_de(nombre).unwrap_or("");
        let mut filas = Vec::new();
        for fila in partir(cuerpo, "\\\\") {
            let limpia = sin_reglas(fila);
            if !limpia.trim().is_empty() {
                filas.push(self.fila(&limpia));
            }
        }
        Bloque::Tabla(Tabla {
            columnas,
            filas,
            opciones,
        })
    }

    fn fila(&mut self, cruda: &str) -> Fila {
        let (cuerpo, fondo) = sacar_color(cruda, "\\rowcolor");
        let (_, cuerpo_) = sin_opcional(&cuerpo);
        let celdas = partir(cuerpo_, "&").into_iter().map(|c| self.celda(c)).collect();
        Fila { celdas, fondo }
    }

    fn celda(&mut self, cruda: &str) -> Celda {
        let (cuerpo, fondo) = sacar_color(cruda, "\\cellcolor");
        let mut celda = Celda {
            tramos: Vec::new(),
            ancho: 1,
            alto: 1,
            alineado: None,
            fondo,
        };
        let dentro = abrir_envolturas(cuerpo.trim(), &mut celda);
        celda.tramos = self.tramos_propios(dentro.trim());
        celda
    }

    /// `\estilocodigo{opciones}`, ya pasada la orden.
    pub(super) fn definir_estilo_de_codigo(&mut self) {
        let opciones = leer_opciones(self.argumento());
        self.salida.codigo.extend(opciones);
    }

    /// `\estilotabla{nombre}{opciones}`, ya pasada la orden.
    pub(super) fn definir_estilo_de_tabla(&mut self) {
        let nombre = self.argumento().trim().to_string();
        let opciones = leer_opciones(self.argumento());
        self.salida.estilos.push((nombre, opciones));
    }
}

/// `\multicolumn` y `\multirow`, que pueden ir uno dentro de otro: lo de dentro y lo que ocupa.
fn abrir_envolturas<'t>(t: &'t str, celda: &mut Celda) -> &'t str {
    let numero = |n: &str| n.trim().parse::<usize>().unwrap_or(1).clamp(1, 256);
    let mut t = t;
    loop {
        if let Some(r) = t.strip_prefix("\\multicolumn")
            && let Some((n, r)) = argumento_de(r)
            && let Some((columna, r)) = argumento_de(r)
            && let Some((dentro, _)) = argumento_de(r)
        {
            celda.ancho = numero(n);
            celda.alineado = leer_columnas(columna).first().copied();
            t = dentro.trim();
        } else if let Some(r) = t.strip_prefix("\\multirow")
            && let (_, r) = sin_opcional(r)
            && let Some((n, r)) = argumento_de(r)
            && let (_, r) = sin_opcional(r)
            && let Some((_, r)) = argumento_de(r)
            && let (_, r) = sin_opcional(r)
            && let Some((dentro, _)) = argumento_de(r)
        {
            celda.alto = numero(n);
            t = dentro.trim();
        } else {
            return t;
        }
    }
}
