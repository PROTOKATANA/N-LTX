//! Avanzar por la fuente: caracteres, argumentos, entornos y macros del propio documento.

use super::*;

impl<'a> Lector<'a> {
    pub(super) fn nuevo(t: &'a str) -> Self {
        Lector {
            fuente: "",
            t,
            i: 0,
            profundidad: 0,
            notas: Vec::new(),
            calculos: String::new(),
            declarado: None,
            salida: Lectura::default(),
        }
    }

    /// Los bytes de la fuente que van de la posición `inicio` a la actual. `None` si lo que se lee
    /// no está en ella: una extensión, por ejemplo, lee lo que ella misma ha generado.
    pub(super) fn origen(&self, inicio: usize) -> Option<(u32, u32)> {
        let base = self.t.as_ptr() as usize;
        let cero = self.fuente.as_ptr() as usize;
        let a = (base + inicio).checked_sub(cero)?;
        let b = (base + self.i).checked_sub(cero)?;
        (a <= b && b <= self.fuente.len()).then_some((a as u32, b as u32))
    }

    pub(super) fn resto(&self) -> &'a str {
        &self.t[self.i..]
    }

    pub(super) fn fin(&self) -> bool {
        self.i >= self.t.len()
    }

    pub(super) fn mira(&self) -> Option<char> {
        self.resto().chars().next()
    }

    pub(super) fn avanza(&mut self, c: char) {
        self.i += c.len_utf8();
    }

    pub(super) fn salta(&mut self, n: usize) {
        self.i = (self.i + n).min(self.t.len());
    }

    pub(super) fn avisar(&mut self, aviso: String) {
        if !self.salida.avisos.contains(&aviso) {
            self.salida.avisos.push(aviso);
        }
    }

    /// Solo recoge lo que definen las órdenes del preámbulo.
    pub(super) fn definiciones(&mut self) {
        while !self.fin() {
            if self.resto().starts_with('\\') {
                let nombre = self.nombre_en(self.i + 1);
                if ORDENES_MACRO.contains(&nombre) || nombre == "def" {
                    self.salta(1 + nombre.len());
                    self.macro_propia(nombre);
                    continue;
                }
                if nombre == "estilotabla" {
                    self.salta(1 + nombre.len());
                    self.definir_estilo_de_tabla();
                    continue;
                }
                if nombre == "estilocodigo" {
                    self.salta(1 + nombre.len());
                    self.definir_estilo_de_codigo();
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
    pub(super) fn nombre_en(&self, i: usize) -> &'a str {
        let t = &self.t[i.min(self.t.len())..];
        let letras = t.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(t.len());
        let n = if t[letras..].starts_with('*') {
            letras + 1
        } else {
            letras
        };
        &t[..n]
    }

    pub(super) fn espacios(&mut self) {
        let t = self.resto();
        self.salta(t.len() - t.trim_start().len());
    }

    /// Un argumento entre llaves sin interpretar, o una sola orden o carácter.
    pub(super) fn argumento(&mut self) -> &'a str {
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

    pub(super) fn opcional(&mut self) -> Option<&'a str> {
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
    pub(super) fn hasta_fin_de(&mut self, entorno: &str) -> Option<&'a str> {
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

    pub(super) fn macro_propia(&mut self, nombre: &str) {
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

    pub(super) fn en_parada(&self, parada: &Parada) -> bool {
        match parada {
            Parada::Final => false,
            Parada::Entorno(e) => self.resto().starts_with(&format!("\\end{{{e}}}")),
            Parada::Item(e) => {
                self.resto().starts_with(&format!("\\end{{{e}}}")) || (self.nombre_si_orden() == Some("item"))
            }
        }
    }

    pub(super) fn nombre_si_orden(&self) -> Option<&'a str> {
        self.resto().starts_with('\\').then(|| self.nombre_en(self.i + 1))
    }

    pub(super) fn linea_en_blanco(&mut self) -> bool {
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
}
