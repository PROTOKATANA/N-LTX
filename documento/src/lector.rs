//! Lee LaTeX en flujo continuo: títulos, párrafos, listas, tablas, fórmulas y figuras.
//!
//! No es un TeX: entiende el subconjunto con el que escribe el modelo, sin páginas ni flotantes.
//! Tolera la fuente a medio escribir: lo que queda abierto al terminar se cierra solo. Lo que no
//! entiende nunca se pierde en silencio: se conserva su texto y se avisa.

/// Hasta aquí se anidan grupos, listas y entornos: una fuente patológica no debe tumbar la pila.
const PROFUNDIDAD_MAXIMA: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Estilo {
    pub negrita: bool,
    pub cursiva: bool,
    pub mono: bool,
    pub subrayado: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tramo {
    Texto(String, Estilo),
    /// Fórmula en línea, con la fuente que entiende el compositor de fórmulas.
    Formula(String),
    Salto,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Bloque {
    Titulo { nivel: u8, tramos: Vec<Tramo> },
    Parrafo(Vec<Tramo>),
    Leyenda(Vec<Tramo>),
    Lista { ordenada: bool, items: Vec<Vec<Bloque>> },
    Cita(Vec<Bloque>),
    Codigo(String),
    Formula(String),
    Figura(String),
    Tabla(Vec<Vec<Vec<Tramo>>>),
    Regla,
}

/// El resultado de leer una fuente.
#[derive(Debug, Default, PartialEq)]
pub struct Lectura {
    pub bloques: Vec<Bloque>,
    /// `\usetikzlibrary`, `\definecolor`, `\tikzset`…: lo que las figuras necesitan delante.
    pub preambulo: String,
    /// `\newcommand` y `\def`: lo que las fórmulas necesitan delante.
    pub macros: String,
    pub avisos: Vec<String>,
}

const ENTORNOS_MATEMATICOS: [&str; 12] = [
    "equation",
    "equation*",
    "align",
    "align*",
    "gather",
    "gather*",
    "multline",
    "multline*",
    "flalign",
    "flalign*",
    "displaymath",
    "eqnarray",
];
const ENTORNOS_FIGURA: [&str; 3] = ["tikzpicture", "circuitikz", "pgfpicture"];
const ENTORNOS_CODIGO: [&str; 3] = ["verbatim", "lstlisting", "minted"];
/// Se entra en ellos como si no estuvieran.
const ENTORNOS_TRANSPARENTES: [&str; 9] = [
    "document",
    "figure",
    "figure*",
    "table",
    "table*",
    "center",
    "abstract",
    "flushleft",
    "flushright",
];
/// Órdenes que se quedan en el preámbulo, con sus argumentos.
const ORDENES_PREAMBULO: [&str; 6] = [
    "usepackage",
    "usetikzlibrary",
    "definecolor",
    "colorlet",
    "tikzset",
    "pgfplotsset",
];
const ORDENES_MACRO: [&str; 3] = ["newcommand", "renewcommand", "providecommand"];
/// Órdenes que no dejan rastro en flujo continuo. El número es cuántos argumentos llevan.
const ORDENES_IGNORADAS: [(&str, usize); 21] = [
    ("documentclass", 1),
    ("title", 1),
    ("author", 1),
    ("date", 1),
    ("maketitle", 0),
    ("tableofcontents", 0),
    ("label", 1),
    ("centering", 0),
    ("noindent", 0),
    ("indent", 0),
    ("clearpage", 0),
    ("newpage", 0),
    ("pagestyle", 1),
    ("bigskip", 0),
    ("medskip", 0),
    ("smallskip", 0),
    ("vspace", 1),
    ("hspace", 1),
    ("vfill", 0),
    ("hfill", 0),
    ("protect", 0),
];
/// Química, unidades y demás: `mathra` las entiende como fórmulas aunque vayan en el texto.
const ORDENES_FORMULA: [(&str, usize); 9] = [
    ("ce", 1),
    ("pu", 1),
    ("chemfig", 1),
    ("SI", 2),
    ("qty", 2),
    ("si", 1),
    ("unit", 1),
    ("num", 1),
    ("ang", 1),
];

#[derive(Clone, PartialEq)]
enum Parada {
    Final,
    Entorno(String),
    Item(String),
}

pub fn leer(fuente: &str) -> Lectura {
    let (preambulo, cuerpo) = separar(fuente);
    let mut l = Lector::nuevo(cuerpo);
    l.salida.preambulo.push_str(preambulo);
    let mut pre = Lector::nuevo(preambulo);
    pre.definiciones();
    l.salida.macros = pre.salida.macros;
    let bloques = l.bloques(&Parada::Final);
    let mut salida = l.salida;
    salida.bloques = bloques;
    if !l.notas.is_empty() {
        salida.bloques.push(Bloque::Regla);
        for (n, nota) in l.notas.into_iter().enumerate() {
            let mut tramos = vec![Tramo::Texto(format!("{}. ", n + 1), Estilo::default())];
            tramos.extend(nota);
            salida.bloques.push(Bloque::Leyenda(tramos));
        }
    }
    salida
}

/// Un documento completo se parte en su preámbulo y su cuerpo.
fn separar(fuente: &str) -> (&str, &str) {
    const INICIO: &str = "\\begin{document}";
    let Some(i) = fuente.find(INICIO) else {
        return ("", fuente);
    };
    let cuerpo = &fuente[i + INICIO.len()..];
    let fin = cuerpo.find("\\end{document}").unwrap_or(cuerpo.len());
    (&fuente[..i], &cuerpo[..fin])
}

struct Lector<'a> {
    t: &'a str,
    i: usize,
    profundidad: usize,
    notas: Vec<Vec<Tramo>>,
    salida: Lectura,
}

impl<'a> Lector<'a> {
    fn nuevo(t: &'a str) -> Self {
        Lector {
            t,
            i: 0,
            profundidad: 0,
            notas: Vec::new(),
            salida: Lectura::default(),
        }
    }

    fn resto(&self) -> &'a str {
        &self.t[self.i..]
    }

    fn fin(&self) -> bool {
        self.i >= self.t.len()
    }

    fn mira(&self) -> Option<char> {
        self.resto().chars().next()
    }

    fn avanza(&mut self, c: char) {
        self.i += c.len_utf8();
    }

    fn salta(&mut self, n: usize) {
        self.i = (self.i + n).min(self.t.len());
    }

    fn avisar(&mut self, aviso: String) {
        if !self.salida.avisos.contains(&aviso) {
            self.salida.avisos.push(aviso);
        }
    }

    /// Solo recoge lo que definen las órdenes del preámbulo.
    fn definiciones(&mut self) {
        while !self.fin() {
            if self.resto().starts_with('\\') {
                let nombre = self.nombre_en(self.i + 1);
                if ORDENES_MACRO.contains(&nombre) || nombre == "def" {
                    self.salta(1 + nombre.len());
                    self.macro_propia(nombre);
                    continue;
                }
            }
            match self.mira() {
                Some(c) => self.avanza(c),
                None => break,
            }
        }
    }

    /// El nombre de una orden que empieza en `i`.
    fn nombre_en(&self, i: usize) -> &'a str {
        let t = &self.t[i.min(self.t.len())..];
        let letras = t.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(t.len());
        let n = if t[letras..].starts_with('*') {
            letras + 1
        } else {
            letras
        };
        &t[..n]
    }

    fn espacios(&mut self) {
        let t = self.resto();
        self.salta(t.len() - t.trim_start().len());
    }

    /// Un argumento entre llaves sin interpretar, o una sola orden o carácter.
    fn argumento(&mut self) -> &'a str {
        self.espacios();
        match self.mira() {
            Some('{') => {
                let t = self.resto();
                let fin = cierre(t, '{', '}').unwrap_or(t.len());
                self.salta(fin + 1);
                &t[1.min(t.len())..fin.max(1).min(t.len())]
            }
            Some('\\') => {
                let inicio = self.i;
                let nombre = self.nombre_en(self.i + 1);
                self.salta(1 + nombre.len().max(1));
                &self.t[inicio..self.i]
            }
            Some(c) => {
                let inicio = self.i;
                self.avanza(c);
                &self.t[inicio..self.i]
            }
            None => "",
        }
    }

    fn opcional(&mut self) -> Option<&'a str> {
        let t = self.resto();
        if !t.trim_start().starts_with('[') {
            return None;
        }
        self.espacios();
        let t = self.resto();
        let fin = cierre(t, '[', ']').unwrap_or(t.len());
        self.salta(fin + 1);
        Some(&t[1.min(t.len())..fin.max(1).min(t.len())])
    }

    /// Avanza hasta `\end{entorno}` y devuelve lo de dentro, o `None` si no se cierra aún.
    fn hasta_fin_de(&mut self, entorno: &str) -> Option<&'a str> {
        let fin = format!("\\end{{{entorno}}}");
        let t = self.resto();
        match t.find(&fin) {
            Some(n) => {
                self.salta(n + fin.len());
                Some(&t[..n])
            }
            None => {
                self.i = self.t.len();
                None
            }
        }
    }

    fn macro_propia(&mut self, nombre: &str) {
        let inicio = self.i;
        if nombre == "def" {
            // el nombre, sus parámetros (#1#2) y el cuerpo
            self.argumento();
            let t = self.resto();
            self.salta(t.find('{').unwrap_or(t.len()));
            self.argumento();
        } else {
            self.argumento();
            self.opcional();
            self.opcional();
            self.argumento();
        }
        let texto = format!("\\{nombre}{}\n", &self.t[inicio..self.i]);
        self.salida.macros.push_str(&texto);
        self.salida.preambulo.push_str(&texto);
    }

    fn en_parada(&self, parada: &Parada) -> bool {
        match parada {
            Parada::Final => false,
            Parada::Entorno(e) => self.resto().starts_with(&format!("\\end{{{e}}}")),
            Parada::Item(e) => {
                self.resto().starts_with(&format!("\\end{{{e}}}")) || (self.nombre_si_orden() == Some("item"))
            }
        }
    }

    fn nombre_si_orden(&self) -> Option<&'a str> {
        self.resto().starts_with('\\').then(|| self.nombre_en(self.i + 1))
    }

    fn linea_en_blanco(&mut self) -> bool {
        let t = self.resto();
        let Some(tras) = t.strip_prefix('\n') else {
            return false;
        };
        let hueco = tras.len() - tras.trim_start_matches([' ', '\t', '\r']).len();
        if tras[hueco..].starts_with('\n') {
            self.salta(1 + hueco);
            return true;
        }
        false
    }

    fn bloques(&mut self, parada: &Parada) -> Vec<Bloque> {
        if self.profundidad >= PROFUNDIDAD_MAXIMA {
            self.avisar("anidamiento demasiado profundo: el resto se ignora".into());
            self.i = self.t.len();
            return Vec::new();
        }
        self.profundidad += 1;
        let (mut salida, mut tramos) = (Vec::new(), Vec::new());
        while !self.fin() && !self.en_parada(parada) {
            if self.linea_en_blanco() {
                cerrar_parrafo(&mut tramos, &mut salida);
            } else if let Some(bloques) = self.bloque_propio() {
                cerrar_parrafo(&mut tramos, &mut salida);
                salida.extend(bloques);
            } else {
                self.uno(Estilo::default(), &mut tramos);
            }
        }
        cerrar_parrafo(&mut tramos, &mut salida);
        self.profundidad -= 1;
        salida
    }

    /// Lo que no es texto de un párrafo. `None` si lo que sigue es texto.
    fn bloque_propio(&mut self) -> Option<Vec<Bloque>> {
        let t = self.resto();
        if t.starts_with("$$") {
            self.salta(2);
            let cuerpo = self.resto();
            let fin = cuerpo.find("$$").unwrap_or(cuerpo.len());
            self.salta(fin + 2);
            return Some(vec![Bloque::Formula(cuerpo[..fin].trim().to_string())]);
        }
        if t.starts_with("\\[") {
            self.salta(2);
            let cuerpo = self.resto();
            let fin = cuerpo.find("\\]").unwrap_or(cuerpo.len());
            self.salta(fin + 2);
            return Some(vec![Bloque::Formula(cuerpo[..fin].trim().to_string())]);
        }
        let nombre = self.nombre_si_orden()?;
        match nombre {
            "begin" => self.entorno(),
            "section" | "section*" | "chapter" | "chapter*" => Some(vec![self.titulo(1, nombre)]),
            "subsection" | "subsection*" => Some(vec![self.titulo(2, nombre)]),
            "subsubsection" | "subsubsection*" | "paragraph" | "paragraph*" => Some(vec![self.titulo(3, nombre)]),
            "par" => {
                self.salta(4);
                Some(Vec::new())
            }
            "caption" => {
                self.salta(1 + nombre.len());
                let texto = self.argumento();
                Some(vec![Bloque::Leyenda(self.tramos_de(texto))])
            }
            "hrule" | "rule" => {
                self.salta(1 + nombre.len());
                Some(vec![Bloque::Regla])
            }
            "tikz" => {
                // `\tikz …;` llega hasta su punto y coma, fuera de llaves
                let t = self.resto();
                let fin = fin_de_instruccion(t);
                self.salta(fin);
                Some(vec![Bloque::Figura(t[..fin].to_string())])
            }
            _ => None,
        }
    }

    fn titulo(&mut self, nivel: u8, nombre: &str) -> Bloque {
        self.salta(1 + nombre.len());
        self.opcional();
        let texto = self.argumento();
        Bloque::Titulo {
            nivel,
            tramos: self.tramos_de(texto),
        }
    }

    /// Interpreta un argumento como texto en línea.
    fn tramos_de(&mut self, texto: &'a str) -> Vec<Tramo> {
        self.tramos_con(texto, Estilo::default())
    }

    fn tramos_con(&mut self, texto: &'a str, estilo: Estilo) -> Vec<Tramo> {
        let mut tramos = Vec::new();
        if self.profundidad >= PROFUNDIDAD_MAXIMA {
            self.avisar("anidamiento demasiado profundo: el resto se ignora".into());
            return tramos;
        }
        let (t, i) = (self.t, self.i);
        self.t = texto;
        self.i = 0;
        self.profundidad += 1;
        while !self.fin() {
            self.uno(estilo, &mut tramos);
        }
        self.profundidad -= 1;
        self.t = t;
        self.i = i;
        tramos
    }

    fn entorno(&mut self) -> Option<Vec<Bloque>> {
        self.salta("\\begin".len());
        let nombre = self.argumento();
        if ENTORNOS_MATEMATICOS.contains(&nombre) {
            return Some(match self.hasta_fin_de(nombre) {
                Some(cuerpo) if nombre == "equation" || nombre == "equation*" || nombre == "displaymath" => {
                    vec![Bloque::Formula(cuerpo.trim().to_string())]
                }
                Some(cuerpo) => vec![Bloque::Formula(format!("\\begin{{{nombre}}}{cuerpo}\\end{{{nombre}}}"))],
                None => {
                    self.avisar(format!("entorno sin cerrar: {nombre}"));
                    Vec::new()
                }
            });
        }
        if ENTORNOS_FIGURA.contains(&nombre) {
            return Some(match self.hasta_fin_de(nombre) {
                Some(cuerpo) => vec![Bloque::Figura(format!("\\begin{{{nombre}}}{cuerpo}\\end{{{nombre}}}"))],
                None => {
                    self.avisar(format!("figura sin cerrar: {nombre}"));
                    Vec::new()
                }
            });
        }
        if ENTORNOS_CODIGO.contains(&nombre) {
            self.opcional();
            return Some(match self.hasta_fin_de(nombre) {
                Some(cuerpo) => vec![Bloque::Codigo(cuerpo.trim_matches('\n').to_string())],
                None => Vec::new(),
            });
        }
        if ENTORNOS_TRANSPARENTES.contains(&nombre) {
            self.opcional();
            return Some(Vec::new());
        }
        match nombre {
            "itemize" | "enumerate" | "description" => Some(vec![self.lista(nombre)]),
            "quote" | "quotation" => {
                let dentro = self.bloques(&Parada::Entorno(nombre.to_string()));
                self.hasta_fin_de(nombre);
                Some(vec![Bloque::Cita(dentro)])
            }
            "tabular" | "tabular*" | "tabularx" => Some(vec![self.tabla(nombre)]),
            otro => {
                self.avisar(format!("entorno desconocido: {otro}"));
                Some(Vec::new())
            }
        }
    }

    fn lista(&mut self, nombre: &str) -> Bloque {
        self.opcional();
        let mut items = Vec::new();
        let parada = Parada::Item(nombre.to_string());
        // lo que hubiera antes del primer `\item` no es parte de la lista
        self.bloques(&parada);
        while self.nombre_si_orden() == Some("item") {
            self.salta("\\item".len());
            self.opcional();
            items.push(self.bloques(&parada));
        }
        self.hasta_fin_de(nombre);
        Bloque::Lista {
            ordenada: nombre == "enumerate",
            items,
        }
    }

    fn tabla(&mut self, nombre: &str) -> Bloque {
        self.opcional();
        if nombre != "tabular" {
            self.argumento();
        }
        self.argumento();
        let cuerpo = self.hasta_fin_de(nombre).unwrap_or("");
        let mut filas = Vec::new();
        for fila in partir(cuerpo, "\\\\") {
            let limpia = sin_reglas(fila);
            if limpia.trim().is_empty() {
                continue;
            }
            let celdas: Vec<String> = partir(&limpia, "&").into_iter().map(str::to_string).collect();
            filas.push(celdas);
        }
        let filas = filas
            .into_iter()
            .map(|celdas| {
                celdas
                    .into_iter()
                    .map(|c| {
                        let texto = c.trim().to_string();
                        self.tramos_propios(&texto)
                    })
                    .collect()
            })
            .collect();
        Bloque::Tabla(filas)
    }

    /// `tramos_de` para un texto que no vive tanto como la fuente.
    fn tramos_propios(&mut self, texto: &str) -> Vec<Tramo> {
        let mut otro = Lector::nuevo(texto);
        otro.profundidad = self.profundidad;
        let tramos = otro.tramos_de(texto);
        for aviso in otro.salida.avisos {
            self.avisar(aviso);
        }
        self.notas.extend(otro.notas);
        tramos
    }

    /// Consume un elemento de texto.
    fn uno(&mut self, estilo: Estilo, tramos: &mut Vec<Tramo>) {
        let Some(c) = self.mira() else { return };
        match c {
            '\\' => self.orden(estilo, tramos),
            '$' => self.formula_en_linea(tramos),
            '%' => {
                let t = self.resto();
                self.salta(t.find('\n').map_or(t.len(), |n| n + 1));
            }
            '{' => {
                let t = self.resto();
                let fin = cierre(t, '{', '}').unwrap_or(t.len());
                self.salta(fin + 1);
                let dentro = &t[1.min(t.len())..fin.max(1).min(t.len())];
                let (dentro, estilo) = estilo_de_grupo(dentro, estilo);
                fundir(tramos, self.tramos_con(dentro, estilo));
            }
            '}' => self.avanza(c),
            '~' => {
                self.avanza(c);
                empujar(tramos, "\u{a0}", estilo);
            }
            '\n' | '\r' | '\t' => {
                self.avanza(c);
                empujar(tramos, " ", estilo);
            }
            '-' if self.resto().starts_with("---") => {
                self.salta(3);
                empujar(tramos, "—", estilo);
            }
            '-' if self.resto().starts_with("--") => {
                self.salta(2);
                empujar(tramos, "–", estilo);
            }
            '`' if self.resto().starts_with("``") => {
                self.salta(2);
                empujar(tramos, "“", estilo);
            }
            '\'' if self.resto().starts_with("''") => {
                self.salta(2);
                empujar(tramos, "”", estilo);
            }
            _ => {
                self.avanza(c);
                let mut b = [0u8; 4];
                empujar(tramos, c.encode_utf8(&mut b), estilo);
            }
        }
    }

    fn formula_en_linea(&mut self, tramos: &mut Vec<Tramo>) {
        self.salta(1);
        let t = self.resto();
        let fin = dolar_de_cierre(t).unwrap_or(t.len());
        self.salta(fin + 1);
        tramos.push(Tramo::Formula(t[..fin].to_string()));
    }

    fn orden(&mut self, estilo: Estilo, tramos: &mut Vec<Tramo>) {
        let nombre = self.nombre_en(self.i + 1);
        if nombre.is_empty() {
            self.simbolo(estilo, tramos);
            return;
        }
        if let Some((_, args)) = ORDENES_FORMULA.iter().find(|(n, _)| *n == nombre) {
            let inicio = self.i;
            self.salta(1 + nombre.len());
            for _ in 0..*args {
                self.opcional();
                self.argumento();
            }
            tramos.push(Tramo::Formula(self.t[inicio..self.i].to_string()));
            return;
        }
        if nombre == "schemestart" {
            let t = self.resto();
            let fin = t.find("\\schemestop").map_or(t.len(), |n| n + "\\schemestop".len());
            self.salta(fin);
            tramos.push(Tramo::Formula(t[..fin].to_string()));
            return;
        }
        self.salta(1 + nombre.len());
        if ORDENES_PREAMBULO.contains(&nombre) {
            let inicio = self.i;
            self.opcional();
            self.argumento();
            if nombre == "definecolor" {
                self.argumento();
                self.argumento();
            }
            if nombre == "colorlet" {
                self.argumento();
            }
            let texto = format!("\\{nombre}{}\n", &self.t[inicio..self.i]);
            self.salida.preambulo.push_str(&texto);
            return;
        }
        if ORDENES_MACRO.contains(&nombre) || nombre == "def" {
            self.macro_propia(nombre);
            return;
        }
        if let Some((_, args)) = ORDENES_IGNORADAS.iter().find(|(n, _)| *n == nombre) {
            if nombre == "documentclass" {
                self.opcional();
            }
            for _ in 0..*args {
                self.argumento();
            }
            return;
        }
        self.orden_de_texto(nombre, estilo, tramos);
    }

    fn orden_de_texto(&mut self, nombre: &str, estilo: Estilo, tramos: &mut Vec<Tramo>) {
        let con = |cambio: fn(&mut Estilo)| {
            let mut e = estilo;
            cambio(&mut e);
            e
        };
        let envuelto = match nombre {
            "textbf" | "bfseries" => Some(con(|e| e.negrita = true)),
            "textit" | "emph" | "textsl" | "itshape" => Some(con(|e| e.cursiva = true)),
            "texttt" => Some(con(|e| e.mono = true)),
            "underline" => Some(con(|e| e.subrayado = true)),
            "textrm" | "textsc" | "textnormal" | "textsf" | "mbox" | "text" => Some(estilo),
            _ => None,
        };
        if let Some(e) = envuelto {
            let texto = self.argumento();
            fundir(tramos, self.tramos_con(texto, e));
            return;
        }
        match nombre {
            "end" => {
                self.argumento();
            }
            "item" => {}
            "textcolor" => {
                self.argumento();
                let texto = self.argumento();
                fundir(tramos, self.tramos_con(texto, estilo));
            }
            "href" => {
                self.argumento();
                let texto = self.argumento();
                fundir(tramos, self.tramos_con(texto, con(|e| e.subrayado = true)));
            }
            "url" => {
                let texto = self.argumento();
                fundir(tramos, self.tramos_con(texto, con(|e| e.mono = true)));
            }
            "verb" => self.verbatim_en_linea(con(|e| e.mono = true), tramos),
            "footnote" => {
                let texto = self.argumento();
                let nota = self.tramos_de(texto);
                self.notas.push(nota);
                empujar(tramos, &format!("[{}]", self.notas.len()), estilo);
            }
            "cite" => {
                self.opcional();
                let clave = self.argumento();
                empujar(tramos, &format!("[{clave}]"), estilo);
            }
            "ref" | "eqref" | "pageref" => {
                let clave = self.argumento();
                empujar(tramos, clave, estilo);
            }
            "newline" | "linebreak" => tramos.push(Tramo::Salto),
            "LaTeX" => empujar(tramos, "LaTeX", estilo),
            "TeX" => empujar(tramos, "TeX", estilo),
            "ldots" | "dots" => empujar(tramos, "…", estilo),
            "textbackslash" => empujar(tramos, "\\", estilo),
            "S" => empujar(tramos, "§", estilo),
            "quad" => empujar(tramos, "\u{2003}", estilo),
            "qquad" => empujar(tramos, "\u{2003}\u{2003}", estilo),
            "(" => {}
            otra => {
                self.avisar(format!("orden desconocida: \\{otra}"));
                // su argumento se ve: es mejor que perder el texto
                if self.mira() == Some('{') {
                    let texto = self.argumento();
                    fundir(tramos, self.tramos_con(texto, estilo));
                }
            }
        }
    }

    fn verbatim_en_linea(&mut self, estilo: Estilo, tramos: &mut Vec<Tramo>) {
        let Some(delimitador) = self.mira() else { return };
        self.avanza(delimitador);
        let t = self.resto();
        let fin = t.find(delimitador).unwrap_or(t.len());
        self.salta(fin + delimitador.len_utf8());
        empujar(tramos, &t[..fin], estilo);
    }

    /// Órdenes de un solo carácter: `\%`, `\\`, `\(`, los acentos…
    fn simbolo(&mut self, estilo: Estilo, tramos: &mut Vec<Tramo>) {
        self.salta(1);
        let Some(c) = self.mira() else { return };
        self.avanza(c);
        match c {
            '\\' => {
                self.espacios_de_linea();
                tramos.push(Tramo::Salto);
            }
            '(' => {
                let t = self.resto();
                let fin = t.find("\\)").unwrap_or(t.len());
                self.salta(fin + 2);
                tramos.push(Tramo::Formula(t[..fin].to_string()));
            }
            ' ' | ',' | ';' | ':' | '!' => empujar(tramos, " ", estilo),
            '\'' | '`' | '^' | '~' | '"' => {
                let base = self.argumento();
                let texto = acentuar(c, base).unwrap_or_else(|| base.to_string());
                empujar(tramos, &texto, estilo);
            }
            otro => {
                let mut b = [0u8; 4];
                empujar(tramos, otro.encode_utf8(&mut b), estilo);
            }
        }
    }

    fn espacios_de_linea(&mut self) {
        if self.resto().starts_with('*') {
            self.salta(1);
        }
        self.opcional();
    }
}

/// Las órdenes de tamaño y forma que afectan a todo su grupo: `{\bf texto}`, `{\small texto}`.
fn estilo_de_grupo(dentro: &str, mut estilo: Estilo) -> (&str, Estilo) {
    let t = dentro.trim_start();
    for (orden, cambio) in [
        ("\\bfseries", 0),
        ("\\bf", 0),
        ("\\itshape", 1),
        ("\\it", 1),
        ("\\em", 1),
        ("\\ttfamily", 2),
        ("\\tt", 2),
    ] {
        if let Some(resto) = t.strip_prefix(orden)
            && !resto.starts_with(|c: char| c.is_ascii_alphabetic())
        {
            match cambio {
                0 => estilo.negrita = true,
                1 => estilo.cursiva = true,
                _ => estilo.mono = true,
            }
            return (resto, estilo);
        }
    }
    (dentro, estilo)
}

fn cerrar_parrafo(tramos: &mut Vec<Tramo>, salida: &mut Vec<Bloque>) {
    let mut parrafo = std::mem::take(tramos);
    limpiar_bordes(&mut parrafo);
    if !parrafo.is_empty() {
        salida.push(Bloque::Parrafo(parrafo));
    }
}

/// Quita el blanco del principio y del final de un párrafo.
fn limpiar_bordes(tramos: &mut Vec<Tramo>) {
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
fn empujar(tramos: &mut Vec<Tramo>, texto: &str, estilo: Estilo) {
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
fn fundir(tramos: &mut Vec<Tramo>, nuevos: Vec<Tramo>) {
    for tramo in nuevos {
        match tramo {
            Tramo::Texto(texto, estilo) => empujar(tramos, &texto, estilo),
            otro => tramos.push(otro),
        }
    }
}

/// Posición del cierre que empareja la apertura con la que empieza `t`.
fn cierre(t: &str, abre: char, cierra: char) -> Option<usize> {
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
fn dolar_de_cierre(t: &str) -> Option<usize> {
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
fn fin_de_instruccion(t: &str) -> usize {
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
fn partir<'t>(t: &'t str, separador: &str) -> Vec<&'t str> {
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

fn sin_reglas(fila: &str) -> String {
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

fn acentuar(acento: char, base: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn texto(t: &str) -> Tramo {
        Tramo::Texto(t.into(), Estilo::default())
    }

    fn solo_bloques(fuente: &str) -> Vec<Bloque> {
        leer(fuente).bloques
    }

    #[test]
    fn dos_parrafos_separados_por_una_linea_en_blanco() {
        assert_eq!(
            solo_bloques("Hola\nmundo.\n\nOtro."),
            vec![
                Bloque::Parrafo(vec![texto("Hola mundo.")]),
                Bloque::Parrafo(vec![texto("Otro.")]),
            ]
        );
    }

    #[test]
    fn titulos_por_nivel() {
        let b = solo_bloques(r"\section{Uno}\subsection*{Dos}");
        assert_eq!(
            b,
            vec![
                Bloque::Titulo {
                    nivel: 1,
                    tramos: vec![texto("Uno")]
                },
                Bloque::Titulo {
                    nivel: 2,
                    tramos: vec![texto("Dos")]
                },
            ]
        );
    }

    #[test]
    fn estilos_en_linea() {
        let negrita = Estilo {
            negrita: true,
            ..Estilo::default()
        };
        assert_eq!(
            solo_bloques(r"a \textbf{b} c"),
            vec![Bloque::Parrafo(vec![
                texto("a "),
                Tramo::Texto("b".into(), negrita),
                texto(" c"),
            ])]
        );
    }

    #[test]
    fn formulas_en_linea_y_en_bloque() {
        assert_eq!(
            solo_bloques(r"Sea $x^2$ y \(y\). $$z$$ \[w\]"),
            vec![
                Bloque::Parrafo(vec![
                    texto("Sea "),
                    Tramo::Formula("x^2".into()),
                    texto(" y "),
                    Tramo::Formula("y".into()),
                    texto("."),
                ]),
                Bloque::Formula("z".into()),
                Bloque::Formula("w".into()),
            ]
        );
    }

    #[test]
    fn la_quimica_y_las_unidades_son_formulas_aunque_vayan_en_el_texto() {
        let b = solo_bloques(r"Agua: \ce{H2O} a \SI{20}{\celsius}.");
        assert_eq!(
            b,
            vec![Bloque::Parrafo(vec![
                texto("Agua: "),
                Tramo::Formula(r"\ce{H2O}".into()),
                texto(" a "),
                Tramo::Formula(r"\SI{20}{\celsius}".into()),
                texto("."),
            ])]
        );
    }

    #[test]
    fn listas_anidadas() {
        let b = solo_bloques(
            r"\begin{itemize}\item a
            \item b \begin{enumerate}\item c\end{enumerate}
            \end{itemize}",
        );
        let [Bloque::Lista { ordenada, items }] = &b[..] else {
            panic!("{b:?}")
        };
        assert!(!ordenada);
        assert_eq!(items.len(), 2);
        assert!(matches!(items[1][1], Bloque::Lista { ordenada: true, .. }));
    }

    #[test]
    fn tablas_sin_sus_reglas() {
        let b = solo_bloques(r"\begin{tabular}{c|c}\hline a & $b$ \\ \hline c & d \\ \end{tabular}");
        let [Bloque::Tabla(filas)] = &b[..] else {
            panic!("{b:?}")
        };
        assert_eq!(filas.len(), 2);
        assert_eq!(filas[0][1], vec![Tramo::Formula("b".into())]);
    }

    #[test]
    fn un_documento_completo_separa_su_preambulo() {
        let l = leer(
            r"\documentclass{article}\usetikzlibrary{calc}\newcommand{\R}{\mathbb{R}}
            \begin{document}Hola $\R$\end{document}",
        );
        assert!(l.preambulo.contains("usetikzlibrary"));
        assert!(l.macros.contains(r"\newcommand{\R}"));
        assert_eq!(l.bloques.len(), 1);
    }

    #[test]
    fn las_figuras_llegan_enteras() {
        let b = solo_bloques("Antes\n\n\\begin{tikzpicture}\\draw (0,0);\\end{tikzpicture}\n\nDespués");
        assert_eq!(b.len(), 3);
        assert_eq!(
            b[1],
            Bloque::Figura(r"\begin{tikzpicture}\draw (0,0);\end{tikzpicture}".into())
        );
    }

    #[test]
    fn un_entorno_matematico_conserva_su_alineacion() {
        let b = solo_bloques(r"\begin{align} a &= b \\ c &= d \end{align}");
        assert!(matches!(&b[..], [Bloque::Formula(f)] if f.starts_with(r"\begin{align}")));
    }

    #[test]
    fn caracteres_especiales() {
        assert_eq!(
            solo_bloques(r"50\% de \$3 --- ``ca\~non'' \'a"),
            vec![Bloque::Parrafo(vec![texto("50% de $3 — “cañon” á")])]
        );
    }

    #[test]
    fn un_comentario_no_es_texto() {
        assert_eq!(
            solo_bloques("hola % esto no\nmundo"),
            vec![Bloque::Parrafo(vec![texto("hola mundo")])]
        );
    }

    #[test]
    fn lo_desconocido_se_avisa_y_su_texto_se_ve() {
        let l = leer(r"a \ordenrara{b} c");
        assert_eq!(l.avisos, ["orden desconocida: \\ordenrara"]);
        assert_eq!(l.bloques, vec![Bloque::Parrafo(vec![texto("a b c")])]);
    }

    #[test]
    fn las_notas_al_pie_van_al_final() {
        let b = solo_bloques(r"Texto\footnote{nota}.");
        assert_eq!(b.len(), 3);
        assert_eq!(b[0], Bloque::Parrafo(vec![texto("Texto[1].")]));
        assert_eq!(b[1], Bloque::Regla);
    }

    /// Lo que recibe el motor mientras el modelo escribe.
    #[test]
    fn la_fuente_a_medio_escribir_no_rompe_nada() {
        let fuente = "\\section{Tí}\n\\begin{itemize}\\item $x^{2$ y \\textbf{ne\n\\begin{tikzpicture}\\draw (0,0)\n\\begin{tabular}{c}a & \\ce{H2";
        for (i, _) in fuente.char_indices() {
            leer(&fuente[..i]);
        }
    }

    #[test]
    fn el_anidamiento_extremo_no_tumba_la_pila() {
        let lista = "\\begin{itemize}\\item ".repeat(5000);
        assert!(!leer(&lista).avisos.is_empty());
        let grupos = format!("{}x{}", "{".repeat(50000), "}".repeat(50000));
        assert!(!leer(&grupos).avisos.is_empty());
    }
}
