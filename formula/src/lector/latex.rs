//! Lector de LaTeX matemático (el subconjunto de KaTeX/MathJax que usan
//! Wikipedia y arXiv) → árbol de nodos.

use super::Lector;
use crate::catalogo::Catalogo;
use crate::modelo::{Alfabeto, Alin, Clase, Estilo, Nodo};
use crate::quimica::Quimica;

/// Lector de LaTeX con el catálogo de símbolos y la química inyectados.
pub struct LectorLatex<C, Q> {
    catalogo: C,
    quimica: Q,
}

impl<C: Catalogo, Q: Quimica> LectorLatex<C, Q> {
    pub fn new(catalogo: C, quimica: Q) -> Self {
        Self { catalogo, quimica }
    }
}

impl<C: Catalogo, Q: Quimica> Lector for LectorLatex<C, Q> {
    fn leer(&self, fuente: &str) -> Result<Vec<Nodo>, String> {
        parsear(fuente, &self.catalogo, &self.quimica)
    }
}

enum Fin {
    Eof,
    Llave,
    Corchete,
    Amp,
    NuevaFila,
    End(String),
    Right(Option<char>),
}

fn parsear<C: Catalogo, Q: Quimica>(fuente: &str, catalogo: &C, quimica: &Q) -> Result<Vec<Nodo>, String> {
    let mut p = Parser {
        s: fuente.chars().collect(),
        i: 0,
        alfabeto: Alfabeto::Cursiva,
        corchetes: 0,
        cat: catalogo,
        quim: quimica,
    };
    let (nodos, fin) = p.lista()?;
    match fin {
        Fin::Eof => Ok(nodos),
        Fin::Llave => Err("llave «}» sin abrir".into()),
        Fin::Corchete => Err("«]» inesperado".into()),
        Fin::Amp | Fin::NuevaFila => Ok(nodos), // fuera de un entorno: se ignora
        Fin::End(e) => Err(format!("\\end{{{e}}} sin \\begin")),
        Fin::Right(_) => Err("\\right sin \\left".into()),
    }
}

struct Parser<'a, C, Q> {
    cat: &'a C,
    quim: &'a Q,
    s: Vec<char>,
    i: usize,
    alfabeto: Alfabeto,
    /// Profundidad de argumentos opcionales: «]» solo cierra dentro de uno.
    corchetes: u32,
}

impl<C: Catalogo, Q: Quimica> Parser<'_, C, Q> {
    fn peek(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn saltar_blancos(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.i += 1;
        }
    }

    /// Nombre de comando tras «\»: letras, o un único carácter no letra.
    fn nombre_comando(&mut self) -> String {
        let ini = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            self.i += 1;
        }
        if self.i == ini
            && let Some(c) = self.peek()
        {
            self.i += 1;
            return c.to_string();
        }
        self.s[ini..self.i].iter().collect()
    }

    fn atomo(&self, c: char) -> Nodo {
        let c = match c {
            '-' => '−',
            '*' => '∗',
            _ => c,
        };
        let clase = self.cat.clase_caracter(c);
        let c = if c.is_ascii_alphanumeric() || ('α'..='ω').contains(&c) {
            self.cat.estilizar(c, self.alfabeto)
        } else {
            c
        };
        Nodo::Atomo { c, clase }
    }

    fn lista(&mut self) -> Result<(Vec<Nodo>, Fin), String> {
        let mut nodos: Vec<Nodo> = Vec::new();
        loop {
            self.saltar_blancos();
            let Some(c) = self.peek() else {
                return Ok((nodos, Fin::Eof));
            };
            match c {
                '}' => {
                    self.i += 1;
                    return Ok((nodos, Fin::Llave));
                }
                ']' if self.corchetes > 0 => {
                    self.i += 1;
                    return Ok((nodos, Fin::Corchete));
                }
                '&' => {
                    self.i += 1;
                    return Ok((nodos, Fin::Amp));
                }
                '^' | '_' => {
                    self.i += 1;
                    let arg = self.argumento()?;
                    adjuntar(&mut nodos, arg, c == '^');
                }
                '\'' => {
                    let mut n = 0;
                    while self.peek() == Some('\'') {
                        self.i += 1;
                        n += 1;
                    }
                    let prima = match n {
                        1 => '′',
                        2 => '″',
                        _ => '‴',
                    };
                    adjuntar(
                        &mut nodos,
                        Nodo::Atomo {
                            c: prima,
                            clase: Clase::Ord,
                        },
                        true,
                    );
                }
                '{' => {
                    self.i += 1;
                    let (dentro, fin) = self.lista()?;
                    if !matches!(fin, Fin::Llave) {
                        return Err("llave «{» sin cerrar".into());
                    }
                    nodos.push(Nodo::Grupo(dentro));
                }
                '~' => {
                    self.i += 1;
                    nodos.push(Nodo::Espacio(0.33));
                }
                '\\' => {
                    self.i += 1;
                    let nombre = self.nombre_comando();
                    match nombre.as_str() {
                        "\\" | "cr" | "newline" => {
                            // «\\[2pt]»: el espaciado extra se ignora
                            self.saltar_blancos();
                            if self.peek() == Some('[') {
                                while let Some(c) = self.peek() {
                                    self.i += 1;
                                    if c == ']' {
                                        break;
                                    }
                                }
                            }
                            return Ok((nodos, Fin::NuevaFila));
                        }
                        "end" => {
                            let entorno = self.argumento_crudo()?;
                            return Ok((nodos, Fin::End(entorno)));
                        }
                        "right" => {
                            let d = self.delimitador()?;
                            return Ok((nodos, Fin::Right(d)));
                        }
                        "over" | "atop" | "choose" => {
                            let num = std::mem::take(&mut nodos);
                            let (den, fin) = self.lista()?;
                            nodos.push(Nodo::Fraccion {
                                num: Box::new(Nodo::Grupo(num)),
                                den: Box::new(Nodo::Grupo(den)),
                                estilo: None,
                                regla: nombre != "atop" && nombre != "choose",
                                delims: (nombre == "choose").then_some(('(', ')')),
                            });
                            return Ok((nodos, fin));
                        }
                        "limits" | "nolimits" | "displaylimits" => {
                            let poner = nombre != "nolimits";
                            match nodos.last_mut() {
                                Some(Nodo::GranOp { limites, .. }) => *limites = poner,
                                Some(Nodo::Funcion { limites, .. }) => *limites = poner,
                                _ => {}
                            }
                        }
                        _ => {
                            if let Some(n) = self.comando(&nombre)? {
                                nodos.push(n);
                            }
                        }
                    }
                }
                _ => {
                    self.i += 1;
                    nodos.push(self.atomo(c));
                }
            }
        }
    }

    /// Un argumento: «{…}», un comando o un carácter suelto.
    fn argumento(&mut self) -> Result<Nodo, String> {
        self.saltar_blancos();
        match self.peek() {
            None => Err("falta un argumento".into()),
            Some('{') => {
                self.i += 1;
                let (dentro, fin) = self.lista()?;
                if !matches!(fin, Fin::Llave) {
                    return Err("llave «{» sin cerrar".into());
                }
                Ok(Nodo::Grupo(dentro))
            }
            Some('\\') => {
                self.i += 1;
                let nombre = self.nombre_comando();
                Ok(self.comando(&nombre)?.unwrap_or(Nodo::Grupo(Vec::new())))
            }
            Some(c) => {
                self.i += 1;
                Ok(self.atomo(c))
            }
        }
    }

    /// Argumento «{…}» como texto sin interpretar (para \text, entornos…).
    fn argumento_crudo(&mut self) -> Result<String, String> {
        self.saltar_blancos();
        if self.peek() != Some('{') {
            // un solo carácter sin llaves
            return match self.peek() {
                Some(c) => {
                    self.i += 1;
                    Ok(c.to_string())
                }
                None => Err("falta un argumento".into()),
            };
        }
        self.i += 1;
        let mut nivel = 1;
        let mut out = String::new();
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                '\\' => {
                    // «\{», «\}», «\ »…: el carácter escapado, literal
                    if let Some(n) = self.peek() {
                        self.i += 1;
                        if n.is_ascii_alphabetic() {
                            out.push('\\');
                        }
                        out.push(n);
                    }
                }
                '{' => {
                    nivel += 1;
                    out.push(c);
                }
                '}' => {
                    nivel -= 1;
                    if nivel == 0 {
                        return Ok(out);
                    }
                    out.push(c);
                }
                _ => out.push(c),
            }
        }
        Err("llave «{» sin cerrar".into())
    }

    /// Argumento «{…}» tal cual, con sus barras (para \ce, que lo traduce).
    fn argumento_literal(&mut self) -> Result<String, String> {
        self.saltar_blancos();
        if self.peek() != Some('{') {
            return self.argumento_crudo();
        }
        self.i += 1;
        let ini = self.i;
        let mut nivel = 1;
        while let Some(c) = self.peek() {
            self.i += 1;
            match c {
                '\\' => self.i += 1,
                '{' => nivel += 1,
                '}' => {
                    nivel -= 1;
                    if nivel == 0 {
                        return Ok(self.s[ini..self.i - 1].iter().collect());
                    }
                }
                _ => {}
            }
        }
        Err("llave «{» sin cerrar".into())
    }

    fn opcional(&mut self) -> Result<Option<Vec<Nodo>>, String> {
        self.saltar_blancos();
        if self.peek() != Some('[') {
            return Ok(None);
        }
        self.i += 1;
        self.corchetes += 1;
        let (dentro, fin) = self.lista()?;
        self.corchetes -= 1;
        if !matches!(fin, Fin::Corchete) {
            return Err("«[» sin cerrar".into());
        }
        Ok(Some(dentro))
    }

    fn delimitador(&mut self) -> Result<Option<char>, String> {
        self.saltar_blancos();
        let token = match self.peek() {
            Some('\\') => {
                self.i += 1;
                format!("\\{}", self.nombre_comando())
            }
            Some(c) => {
                self.i += 1;
                c.to_string()
            }
            None => return Err("falta el delimitador".into()),
        };
        self.cat
            .delimitador(&token)
            .ok_or_else(|| format!("delimitador desconocido «{token}»"))
    }

    fn con_alfabeto(&mut self, alfabeto: Alfabeto) -> Result<Nodo, String> {
        let antes = self.alfabeto;
        self.alfabeto = alfabeto;
        let r = self.argumento();
        self.alfabeto = antes;
        r
    }

    fn comando(&mut self, nombre: &str) -> Result<Option<Nodo>, String> {
        let espacio = |em: f32| Ok(Some(Nodo::Espacio(em)));
        match nombre {
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let num = self.argumento()?;
                let den = self.argumento()?;
                let estilo = match nombre {
                    "dfrac" | "cfrac" => Some(Estilo::Display),
                    "tfrac" => Some(Estilo::Texto),
                    _ => None,
                };
                Ok(Some(Nodo::Fraccion {
                    num: Box::new(num),
                    den: Box::new(den),
                    estilo,
                    regla: true,
                    delims: None,
                }))
            }
            "binom" | "dbinom" | "tbinom" => {
                let num = self.argumento()?;
                let den = self.argumento()?;
                let estilo = match nombre {
                    "dbinom" => Some(Estilo::Display),
                    "tbinom" => Some(Estilo::Texto),
                    _ => None,
                };
                Ok(Some(Nodo::Fraccion {
                    num: Box::new(num),
                    den: Box::new(den),
                    estilo,
                    regla: false,
                    delims: Some(('(', ')')),
                }))
            }
            "sqrt" => {
                let indice = self.opcional()?;
                let radicando = self.argumento()?;
                Ok(Some(Nodo::Raiz {
                    indice: indice.map(|v| Box::new(Nodo::Grupo(v))),
                    radicando: Box::new(radicando),
                }))
            }
            "text" | "textrm" | "textit" | "textbf" | "textsf" | "texttt" | "textnormal" | "textup" | "mbox"
            | "hbox" => Ok(Some(Nodo::Texto(self.argumento_crudo()?))),
            "operatorname" => {
                let limites = self.peek() == Some('*');
                if limites {
                    self.i += 1;
                }
                let nombre = self.argumento_crudo()?;
                Ok(Some(Nodo::Funcion { nombre, limites }))
            }
            "mathrm" | "rm" | "mathup" => self.con_alfabeto(Alfabeto::Recta).map(Some),
            "mathbf" | "bf" | "mathbfup" => self.con_alfabeto(Alfabeto::Negrita).map(Some),
            "boldsymbol" | "bm" | "mathbfit" => self.con_alfabeto(Alfabeto::NegritaCursiva).map(Some),
            "mathit" | "it" => self.con_alfabeto(Alfabeto::Cursiva).map(Some),
            "mathbb" | "Bbb" => self.con_alfabeto(Alfabeto::Pizarra).map(Some),
            "mathcal" | "mathscr" | "cal" => self.con_alfabeto(Alfabeto::Caligrafica).map(Some),
            "mathfrak" | "frak" => self.con_alfabeto(Alfabeto::Fraktur).map(Some),
            "mathsf" | "sf" => self.con_alfabeto(Alfabeto::SinSerifa).map(Some),
            "mathtt" | "tt" => self.con_alfabeto(Alfabeto::Mono).map(Some),
            "left" => {
                let izq = self.delimitador()?;
                let (cuerpo, fin) = self.lista()?;
                let Fin::Right(der) = fin else {
                    return Err("\\left sin \\right".into());
                };
                Ok(Some(Nodo::Delimitado { izq, der, cuerpo }))
            }
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "Bigl" | "biggl" | "Biggl" | "bigr" | "Bigr" | "biggr"
            | "Biggr" | "bigm" | "Bigm" | "biggm" | "Biggm" => {
                let nivel = match nombre.trim_end_matches(['l', 'r', 'm']) {
                    "big" => 1,
                    "Big" => 2,
                    "bigg" => 3,
                    _ => 4,
                };
                let clase = match nombre.chars().last() {
                    Some('l') => Clase::Open,
                    Some('r') => Clase::Close,
                    Some('m') => Clase::Rel,
                    _ => Clase::Ord,
                };
                Ok(self.delimitador()?.map(|c| Nodo::Tamano { c, clase, nivel }))
            }
            "overline" | "underline" => {
                let base = self.argumento()?;
                Ok(Some(Nodo::Raya {
                    arriba: nombre == "overline",
                    base: Box::new(base),
                }))
            }
            "overset" | "stackrel" | "underset" => {
                let marca = self.argumento()?;
                let base = self.argumento()?;
                let (sup, sub) = if nombre == "underset" {
                    (None, Some(Box::new(marca)))
                } else {
                    (Some(Box::new(marca)), None)
                };
                Ok(Some(Nodo::Limites {
                    base: Box::new(base),
                    sup,
                    sub,
                }))
            }
            "overbrace" | "underbrace" | "boxed" | "mathop" | "mathord" | "mathbin" | "mathrel" | "mathopen"
            | "mathclose" | "mathpunct" | "mathinner" | "displaylines" => self.argumento().map(Some),
            "textcolor" | "colorbox" => {
                self.argumento_crudo()?;
                self.argumento().map(Some)
            }
            "color" | "label" | "tag" | "hspace" | "vspace" | "mspace" => {
                let arg = self.argumento_crudo()?;
                if nombre == "hspace" || nombre == "mspace" {
                    // solo se respetan anchos en em; el resto, medio em
                    let em = arg.trim().strip_suffix("em").and_then(|n| n.trim().parse().ok());
                    return espacio(em.unwrap_or(0.5));
                }
                Ok(None)
            }
            "phantom" | "hphantom" | "vphantom" | "smash" => {
                let base = self.argumento()?;
                Ok((nombre == "smash").then_some(base))
            }
            "displaystyle" => Ok(Some(Nodo::Estilo(Estilo::Display))),
            "textstyle" => Ok(Some(Nodo::Estilo(Estilo::Texto))),
            "scriptstyle" => Ok(Some(Nodo::Estilo(Estilo::Indice))),
            "scriptscriptstyle" => Ok(Some(Nodo::Estilo(Estilo::SubIndice))),
            "," | "thinspace" => espacio(3.0 / 18.0),
            ":" | ">" | "medspace" => espacio(4.0 / 18.0),
            ";" | "thickspace" => espacio(5.0 / 18.0),
            "!" | "negthinspace" => espacio(-3.0 / 18.0),
            "negmedspace" => espacio(-4.0 / 18.0),
            "negthickspace" => espacio(-5.0 / 18.0),
            " " => espacio(0.33),
            "enspace" => espacio(0.5),
            "quad" => espacio(1.0),
            "qquad" => espacio(2.0),
            "mathstrut" | "strut" | "nonumber" | "notag" | "limits" | "nolimits" | "relax" => Ok(None),
            "not" => {
                // \not seguido de una relación: las combinaciones con forma
                // negada propia; si no, la barra sobre lo que siga
                self.saltar_blancos();
                let inicio = self.i;
                let siguiente = match self.peek() {
                    Some('\\') => {
                        self.i += 1;
                        self.nombre_comando()
                    }
                    Some(c) => {
                        self.i += 1;
                        c.to_string()
                    }
                    None => String::new(),
                };
                let c = match siguiente.as_str() {
                    "=" => '≠',
                    "in" => '∉',
                    "subset" => '⊄',
                    "supset" => '⊅',
                    "equiv" => '≢',
                    "sim" => '≁',
                    "approx" => '≉',
                    "leq" | "le" => '≰',
                    "geq" | "ge" => '≱',
                    "<" => '≮',
                    ">" => '≯',
                    "mid" => '∤',
                    "exists" => '∄',
                    "ni" | "owns" => '∌',
                    "subseteq" => '⊈',
                    "supseteq" => '⊉',
                    "vdash" => '⊬',
                    "models" | "vDash" => '⊭',
                    "Vdash" => '⊮',
                    "parallel" => '∦',
                    "prec" => '⊀',
                    "succ" => '⊁',
                    "cong" => '≇',
                    "simeq" => '≄',
                    "sqsubseteq" => '⋢',
                    "sqsupseteq" => '⋣',
                    "preceq" => '⋠',
                    "succeq" => '⋡',
                    "triangleleft" => '⋪',
                    "triangleright" => '⋫',
                    "leftarrow" => '↚',
                    "rightarrow" | "to" => '↛',
                    "Leftarrow" => '⇍',
                    "Rightarrow" => '⇏',
                    "leftrightarrow" => '↮',
                    "Leftrightarrow" => '⇎',
                    "asymp" => '≭',
                    "lesssim" => '≴',
                    "gtrsim" => '≵',
                    "propto" => '∝',
                    _ => {
                        self.i = inicio;
                        if matches!(self.peek(), None | Some('}' | '&')) {
                            // «{\not }D» (notación de Feynman): barra sin ancho
                            // que pisa lo que venga después
                            return Ok(Some(Nodo::Tachado(Box::new(Nodo::Grupo(Vec::new())))));
                        }
                        let base = self.argumento()?;
                        return Ok(Some(Nodo::Tachado(Box::new(base))));
                    }
                };
                Ok(Some(Nodo::Atomo {
                    c,
                    clase: self.cat.clase_caracter(c).max_rel(),
                }))
            }
            "begin" => self.entorno().map(Some),
            // envoltorios de LaTeX de editorial (metadatos de APS, IEEE…)
            "ensuremath" => self.argumento().map(Some),
            "ifmmode" | "fi" => Ok(None),
            "else" => {
                // `\ifmmode A\else B\fi`: en una fórmula vale A; B se salta
                while self.i < self.s.len() {
                    if self.s[self.i] == '\\' {
                        self.i += 1;
                        let nombre = self.nombre_comando();
                        if nombre == "fi" {
                            break;
                        }
                    } else {
                        self.i += 1;
                    }
                }
                Ok(None)
            }
            "substack" => {
                // filas apiladas y centradas bajo un operador
                self.saltar_blancos();
                if self.peek() != Some('{') {
                    return Err("\\substack sin argumento".into());
                }
                self.i += 1;
                let mut filas = Vec::new();
                loop {
                    let (fila, fin) = self.lista()?;
                    filas.push(vec![Nodo::Grupo(fila)]);
                    match fin {
                        Fin::NuevaFila => {}
                        Fin::Llave => break,
                        _ => return Err("llave «{» sin cerrar".into()),
                    }
                }
                Ok(Some(Nodo::Tabla {
                    filas,
                    columnas: vec![Alin::Centro],
                    separacion: 0.0,
                    delims: (None, None),
                    estilo: Estilo::Indice,
                }))
            }
            "pmod" => {
                let arg = self.argumento()?;
                Ok(Some(Nodo::Grupo(vec![
                    Nodo::Espacio(0.6),
                    Nodo::Atomo {
                        c: '(',
                        clase: Clase::Open,
                    },
                    Nodo::Funcion {
                        nombre: "mod".into(),
                        limites: false,
                    },
                    Nodo::Espacio(0.33),
                    arg,
                    Nodo::Atomo {
                        c: ')',
                        clase: Clase::Close,
                    },
                ])))
            }
            "xrightarrow" | "xleftarrow" | "xleftrightarrow" | "xRightarrow" | "xLeftarrow" | "xLeftrightarrow"
            | "xmapsto" | "xrightleftharpoons" | "xlongequal" => {
                let sub = self.opcional()?.map(|v| Box::new(Nodo::Grupo(v)));
                let sup = self.argumento()?;
                let sup = (!matches!(&sup, Nodo::Grupo(v) if v.is_empty())).then(|| Box::new(sup));
                let c = match nombre {
                    "xleftarrow" => '←',
                    "xleftrightarrow" => '↔',
                    "xRightarrow" => '⇒',
                    "xLeftarrow" => '⇐',
                    "xLeftrightarrow" => '⇔',
                    "xmapsto" => '↦',
                    "xrightleftharpoons" => '⇌',
                    "xlongequal" => '=',
                    _ => '→',
                };
                Ok(Some(Nodo::Flecha { c, sup, sub }))
            }
            "cancel" | "bcancel" | "xcancel" => {
                let base = self.argumento()?;
                Ok(Some(Nodo::Tachado(Box::new(base))))
            }
            "sout" => self.argumento().map(Some),
            "sideset" => {
                // \sideset{_a^b}{_c^d}\sum: índices a ambos lados del operador
                let izq = self.argumento()?;
                let der = self.argumento()?;
                let op = self.argumento()?;
                let indices = |n: Nodo| match n {
                    Nodo::Grupo(mut v) if v.len() == 1 => match v.pop() {
                        Some(Nodo::Guiones { sup, sub, .. }) => (sup, sub),
                        _ => (None, None),
                    },
                    _ => (None, None),
                };
                let (sup, sub) = indices(der);
                Ok(Some(Nodo::Grupo(vec![
                    izq,
                    Nodo::Guiones {
                        base: Box::new(op),
                        sup,
                        sub,
                    },
                ])))
            }
            "definecolor" => {
                for _ in 0..3 {
                    self.argumento_crudo()?;
                }
                Ok(None)
            }
            "vline" => Ok(None),
            "varinjlim" | "varprojlim" | "injlim" | "projlim" => Ok(Some(Nodo::Funcion {
                nombre: "lim".into(),
                limites: true,
            })),
            "varlimsup" | "varliminf" => Ok(Some(Nodo::Funcion {
                nombre: nombre.trim_start_matches("var").into(),
                limites: true,
            })),
            "cancelto" => {
                self.argumento()?;
                let base = self.argumento()?;
                Ok(Some(Nodo::Tachado(Box::new(base))))
            }
            "hline" | "hdashline" => Ok(None),
            "cline" => {
                self.argumento_crudo()?;
                Ok(None)
            }
            "ce" | "pu" => {
                // química (mhchem): se traduce a LaTeX matemático y se parsea
                let crudo = self.argumento_literal()?;
                let latex = self.quim.a_latex(&crudo);
                let mut p = Parser {
                    s: latex.chars().collect(),
                    i: 0,
                    alfabeto: Alfabeto::Recta,
                    corchetes: 0,
                    cat: self.cat,
                    quim: self.quim,
                };
                let (nodos, _) = p.lista()?;
                Ok(Some(Nodo::Grupo(nodos)))
            }
            _ => {
                if let Some((c, clase)) = self.cat.simbolo(nombre) {
                    let c = if clase == Clase::Ord {
                        self.cat.estilizar(c, self.alfabeto)
                    } else {
                        c
                    };
                    return Ok(Some(Nodo::Atomo { c, clase }));
                }
                if let Some((c, limites)) = self.cat.gran_operador(nombre) {
                    return Ok(Some(Nodo::GranOp { c, limites }));
                }
                if let Some(limites) = self.cat.funcion(nombre) {
                    return Ok(Some(Nodo::Funcion {
                        nombre: nombre.to_string(),
                        limites,
                    }));
                }
                if let Some(c) = self.cat.acento(nombre) {
                    let base = self.argumento()?;
                    return Ok(Some(Nodo::Acento {
                        c,
                        base: Box::new(base),
                    }));
                }
                Ok(Some(Nodo::Desconocido(nombre.to_string())))
            }
        }
    }

    fn entorno(&mut self) -> Result<Nodo, String> {
        let nombre = self.argumento_crudo()?;
        let base = nombre.trim_end_matches('*');
        // patrón de columnas
        let mut columnas: Vec<Alin> = match base {
            "aligned" | "align" | "alignat" | "alignedat" | "split" | "eqnarray" => {
                vec![Alin::Der, Alin::Izq]
            }
            "cases" | "dcases" | "rcases" => vec![Alin::Izq, Alin::Izq],
            "gathered" | "gather" | "multline" => vec![Alin::Centro],
            _ => vec![Alin::Centro],
        };
        if matches!(base, "array" | "darray" | "subarray") {
            let spec = self.argumento_crudo()?;
            let cols: Vec<Alin> = spec
                .chars()
                .filter_map(|c| match c {
                    'l' => Some(Alin::Izq),
                    'c' => Some(Alin::Centro),
                    'r' => Some(Alin::Der),
                    _ => None,
                })
                .collect();
            if !cols.is_empty() {
                columnas = cols;
            }
        }
        if matches!(base, "alignat" | "alignedat") {
            self.argumento_crudo()?; // número de columnas
        }
        let delims = match base {
            "pmatrix" => (Some('('), Some(')')),
            "bmatrix" => (Some('['), Some(']')),
            "Bmatrix" => (Some('{'), Some('}')),
            "vmatrix" => (Some('|'), Some('|')),
            "Vmatrix" => (Some('‖'), Some('‖')),
            "cases" | "dcases" => (Some('{'), None),
            "rcases" => (None, Some('}')),
            _ => (None, None),
        };
        let (separacion, estilo) = match base {
            "aligned" | "align" | "alignat" | "alignedat" | "split" | "eqnarray" | "gathered" | "gather"
            | "multline" | "dcases" | "darray" => (0.0, Estilo::Display),
            "cases" | "rcases" => (1.0, Estilo::Texto),
            "smallmatrix" | "subarray" => (0.3, Estilo::Indice),
            _ => (1.0, Estilo::Texto),
        };
        let mut filas: Vec<Vec<Nodo>> = vec![Vec::new()];
        loop {
            let (celda, fin) = self.lista()?;
            filas.last_mut().expect("siempre hay fila").push(Nodo::Grupo(celda));
            match fin {
                Fin::Amp => {}
                Fin::NuevaFila => filas.push(Vec::new()),
                Fin::End(e) if e.trim_end_matches('*') == base => break,
                Fin::End(e) => return Err(format!("\\begin{{{nombre}}} cerrado con \\end{{{e}}}")),
                _ => return Err(format!("\\begin{{{nombre}}} sin \\end")),
            }
        }
        // una «\\» final deja una fila vacía: fuera
        if filas.len() > 1
            && filas
                .last()
                .is_some_and(|f| f.len() == 1 && matches!(&f[0], Nodo::Grupo(v) if v.is_empty()))
        {
            filas.pop();
        }
        Ok(Nodo::Tabla {
            filas,
            columnas,
            separacion,
            delims,
            estilo,
        })
    }
}

/// Engancha un superíndice o subíndice al último nodo de la lista.
fn adjuntar(nodos: &mut Vec<Nodo>, arg: Nodo, arriba: bool) {
    let base = nodos.pop().unwrap_or(Nodo::Grupo(Vec::new()));
    let nuevo = match base {
        Nodo::Guiones { base, sup, sub } => {
            let ocupado = if arriba { sup.is_some() } else { sub.is_some() };
            if ocupado {
                // «x^a^b»: el segundo índice va sobre un grupo vacío
                let previo = Nodo::Guiones { base, sup, sub };
                nodos.push(previo);
                guiones(Nodo::Grupo(Vec::new()), arg, arriba)
            } else if arriba {
                Nodo::Guiones {
                    base,
                    sup: Some(Box::new(arg)),
                    sub,
                }
            } else {
                Nodo::Guiones {
                    base,
                    sup,
                    sub: Some(Box::new(arg)),
                }
            }
        }
        otro => guiones(otro, arg, arriba),
    };
    nodos.push(nuevo);
}

fn guiones(base: Nodo, arg: Nodo, arriba: bool) -> Nodo {
    let (sup, sub) = if arriba {
        (Some(Box::new(arg)), None)
    } else {
        (None, Some(Box::new(arg)))
    };
    Nodo::Guiones {
        base: Box::new(base),
        sup,
        sub,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::catalogo::CatalogoLatex;
    use crate::quimica::Mhchem;

    fn leer(s: &str) -> Result<Vec<Nodo>, String> {
        LectorLatex::new(CatalogoLatex, Mhchem).leer(s)
    }

    fn ok(s: &str) -> Vec<Nodo> {
        leer(s).unwrap_or_else(|e| panic!("«{s}»: {e}"))
    }

    #[test]
    fn el_ejemplo_del_usuario_se_parsea() {
        for f in [
            "x^{0}=1",
            "x^{-1}={\\frac {1}{x}}",
            "x^{-n}={\\frac {1}{x^{n}}}{\\text{ etc.}}",
            "x^{1/2}={\\sqrt {x}}",
            "x^{2/3}={\\sqrt[{3}]{x^{2}}}{\\text{ etc.}}",
            "x^{1/n}={\\sqrt[{n}]{x}}",
            "x^{p/q}={\\sqrt[{q}]{x^{p}}}",
        ] {
            ok(f);
        }
    }

    #[test]
    fn construcciones_habituales_de_wikipedia() {
        for f in [
            "{{\\ln k} \\over {k^{2}}}",
            "\\sum _{k=2}^{\\infty }{{\\ln k} \\over {k^{2}}}",
            "\\left[12\\ln A-\\gamma -\\ln(2\\pi )\\right]",
            "\\begin{aligned}a&=b\\\\c&=d\\end{aligned}",
            "\\begin{pmatrix}1&0\\\\0&1\\end{pmatrix}",
            "f(x)={\\begin{cases}1&x>0\\\\0&{\\text{otherwise}}\\end{cases}}",
            "\\mathbb {R} ^{n}",
            "\\hat {H}\\psi =E\\psi",
            "\\int _{a}^{b}f(x)\\,dx",
            "\\lim _{n\\to \\infty }\\left(1+{\\tfrac {1}{n}}\\right)^{n}",
            "{\\tbinom {n}{k}}",
            "f'(x)",
            "\\{{|00\\rangle }",
            "\\not \\in",
            "{\\ce {^{226}_{88}Ra + ^{1}_{0}n -> ^{227}_{88}Ra ->[\\beta^-][42.2 \\ {\\ce {min}}] ^{227}_{89}Ac}}",
            "{\\ce {Am^3+ + 3F^- -> AmF3(v)}}",
            "a\\equiv b{\\pmod {n}}",
            "{\\ce {Zn(s) + 2HCl(aq) -> ZnCl2(aq) + H2 ^}}",
            "{\\ce {Ba^2+ + SO4^2- -> BaSO4 v}}",
            "RCA_{0}\\not \\vdash RT",
            "\\displaystyle {\\not }D",
            "1\\!\\!\\!\\not 9",
            "\\sideset{_a^b}{_c^d}\\sum",
            "{\\ce {{Ar'ZnCl}+ArX->[{\\begin{matrix}{}\\\\{\\ce {Ni(PPh3)4}}\\end{matrix}}] ArAr}}",
            "\\sum_{\\substack{v_0\\in T\\\\v_0=o}} A",
            "\\ensuremath{\\alpha}{\\ifmmode\\times\\else\\texttimes\\fi}2",
            "A\\xrightarrow {f} B",
        ] {
            ok(f);
        }
    }

    /// Catálogo falso: solo conoce `\\estrella`; los caracteres van tal cual.
    struct CatalogoFalso;
    impl Catalogo for CatalogoFalso {
        fn simbolo(&self, nombre: &str) -> Option<(char, Clase)> {
            (nombre == "estrella").then_some(('★', Clase::Ord))
        }
        fn gran_operador(&self, _: &str) -> Option<(char, bool)> {
            None
        }
        fn funcion(&self, _: &str) -> Option<bool> {
            None
        }
        fn acento(&self, _: &str) -> Option<char> {
            None
        }
        fn delimitador(&self, _: &str) -> Option<Option<char>> {
            None
        }
        fn clase_caracter(&self, _: char) -> Clase {
            Clase::Ord
        }
        fn estilizar(&self, c: char, _: Alfabeto) -> char {
            c
        }
    }

    /// Química falsa: cualquier `\\ce{…}` es una estrella.
    struct QuimicaFalsa;
    impl Quimica for QuimicaFalsa {
        fn a_latex(&self, _: &str) -> String {
            "\\estrella".into()
        }
    }

    #[test]
    fn usa_las_piezas_inyectadas() {
        let l = LectorLatex::new(CatalogoFalso, QuimicaFalsa);
        let estrella = Nodo::Atomo {
            c: '★',
            clase: Clase::Ord,
        };
        assert_eq!(l.leer("\\estrella"), Ok(vec![estrella.clone()]));
        assert_eq!(l.leer("\\ce{H2O}"), Ok(vec![Nodo::Grupo(vec![estrella])]));
        assert_eq!(l.leer("\\alpha"), Ok(vec![Nodo::Desconocido("alpha".into())]));
        assert_eq!(
            l.leer("x"),
            Ok(vec![Nodo::Atomo {
                c: 'x',
                clase: Clase::Ord
            }])
        );
    }

    #[test]
    fn ifmmode_toma_la_rama_matematica() {
        assert_eq!(ok("{\\ifmmode\\times\\else\\texttimes\\fi}"), ok("{\\times}"));
        assert_eq!(ok("\\ensuremath{\\alpha}"), ok("{\\alpha}"));
    }

    #[test]
    fn errores_se_detectan() {
        assert!(leer("{x").is_err());
        assert!(leer("x}").is_err());
        assert!(leer("\\left( x").is_err());
    }

    #[test]
    fn letras_en_cursiva_y_digitos_rectos() {
        let n = ok("x2");
        assert!(matches!(n[0], Nodo::Atomo { c: '𝑥', .. }));
        assert!(matches!(n[1], Nodo::Atomo { c: '2', .. }));
        let rm = ok("\\mathrm{d}");
        assert!(matches!(&rm[0], Nodo::Grupo(v) if matches!(v[0], Nodo::Atomo { c: 'd', .. })));
    }
}
