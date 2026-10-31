//! Lo que va dentro de un párrafo: texto con su estilo, fórmulas en línea y órdenes de texto.

use super::*;

impl<'a> Lector<'a> {
    /// `tramos_de` para un texto que no vive tanto como la fuente.
    pub(super) fn tramos_propios(&mut self, texto: &str) -> Vec<Tramo> {
        let mut otro = Lector::nuevo(texto);
        otro.profundidad = self.profundidad;
        otro.salida.colores = std::mem::take(&mut self.salida.colores);
        let tramos = otro.tramos_de(texto);
        self.salida.colores = std::mem::take(&mut otro.salida.colores);
        for aviso in otro.salida.avisos {
            self.avisar(aviso);
        }
        self.notas.extend(otro.notas);
        tramos
    }

    /// Consume un elemento de texto.
    pub(super) fn uno(&mut self, estilo: Estilo, tramos: &mut Vec<Tramo>) {
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

    pub(super) fn formula_en_linea(&mut self, tramos: &mut Vec<Tramo>) {
        self.salta(1);
        let t = self.resto();
        let fin = dolar_de_cierre(t).unwrap_or(t.len());
        self.salta(fin + 1);
        tramos.push(Tramo::Formula(t[..fin].to_string()));
    }

    pub(super) fn orden(&mut self, estilo: Estilo, tramos: &mut Vec<Tramo>) {
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
        if let Some(declarado) = declaracion(nombre, estilo) {
            self.declarado = Some(declarado);
            self.saltar_espacios();
            return;
        }
        if nombre == "color" {
            let nombre = self.argumento();
            let color = Some(self.indice_de_color(nombre));
            self.declarado = Some(Estilo { color, ..estilo });
            return;
        }
        if nombre == "estilotabla" {
            self.definir_estilo_de_tabla();
            return;
        }
        if nombre == "estilocodigo" {
            self.definir_estilo_de_codigo();
            return;
        }
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
        if nombre == "tikzmath" {
            let inicio = self.i;
            self.argumento();
            let texto = format!("\\tikzmath{}\n", &self.t[inicio..self.i]);
            self.calculos.push_str(&texto);
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

    pub(super) fn orden_de_texto(&mut self, nombre: &str, estilo: Estilo, tramos: &mut Vec<Tramo>) {
        let con = |cambio: fn(&mut Estilo)| {
            let mut e = estilo;
            cambio(&mut e);
            e
        };
        let envuelto = match nombre {
            "textbf" => Some(con(|e| e.negrita = true)),
            "textit" | "emph" | "textsl" => Some(con(|e| e.cursiva = true)),
            "texttt" => Some(con(a_mono)),
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
            "makecell" => {
                self.opcional();
                let lineas = self.argumento();
                for (n, linea) in partir(lineas, "\\\\").into_iter().enumerate() {
                    if n > 0 {
                        tramos.push(Tramo::Salto);
                    }
                    fundir(tramos, self.tramos_con(linea.trim(), estilo));
                }
            }
            "textcolor" => {
                let nombre = self.argumento();
                let color = Some(self.indice_de_color(nombre));
                let texto = self.argumento();
                fundir(tramos, self.tramos_con(texto, Estilo { color, ..estilo }));
            }
            "href" => {
                self.argumento();
                let texto = self.argumento();
                fundir(tramos, self.tramos_con(texto, con(|e| e.subrayado = true)));
            }
            "url" => {
                let texto = self.argumento();
                fundir(tramos, self.tramos_con(texto, con(a_mono)));
            }
            "verb" => self.verbatim_en_linea(con(a_mono), tramos),
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

    fn saltar_espacios(&mut self) {
        while let Some(c @ (' ' | '\n' | '\t')) = self.mira() {
            self.avanza(c);
        }
    }

    fn indice_de_color(&mut self, nombre: &str) -> usize {
        let colores = &mut self.salida.colores;
        colores.iter().position(|c| c == nombre).unwrap_or_else(|| {
            colores.push(nombre.to_string());
            colores.len() - 1
        })
    }

    pub(super) fn verbatim_en_linea(&mut self, estilo: Estilo, tramos: &mut Vec<Tramo>) {
        let Some(delimitador) = self.mira() else { return };
        self.avanza(delimitador);
        let t = self.resto();
        let fin = t.find(delimitador).unwrap_or(t.len());
        self.salta(fin + delimitador.len_utf8());
        empujar(tramos, &t[..fin], estilo);
    }

    /// Órdenes de un solo carácter: `\%`, `\\`, `\(`, los acentos…
    pub(super) fn simbolo(&mut self, estilo: Estilo, tramos: &mut Vec<Tramo>) {
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

    pub(super) fn espacios_de_linea(&mut self) {
        if self.resto().starts_with('*') {
            self.salta(1);
        }
        self.opcional();
    }
}
