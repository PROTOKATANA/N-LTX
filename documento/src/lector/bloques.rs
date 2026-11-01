//! Lo que se apila: párrafos, títulos, listas, tablas y entornos.

use super::*;

impl<'a> Lector<'a> {
    pub(super) fn bloques(&mut self, parada: &Parada) -> Vec<Bloque> {
        if self.profundidad >= PROFUNDIDAD_MAXIMA {
            self.avisar("anidamiento demasiado profundo: el resto se ignora".into());
            self.i = self.t.len();
            return Vec::new();
        }
        self.profundidad += 1;
        let (mut salida, mut tramos) = (Vec::new(), Vec::new());
        while !self.fin() && !self.en_parada(parada) {
            let inicio = self.i;
            if self.linea_en_blanco() {
                cerrar_parrafo(&mut tramos, &mut salida);
            } else if let Some(bloques) = self.bloque_propio() {
                cerrar_parrafo(&mut tramos, &mut salida);
                if let (false, Some((a, b))) = (bloques.is_empty(), self.origen(inicio)) {
                    salida.push(Bloque::Origen(a, b));
                }
                salida.extend(bloques);
            } else {
                self.uno_anotado(Estilo::default(), &mut tramos);
                // fuera de un grupo no hay dónde acabaría: una figura no cierra el suyo aquí
                self.declarado = None;
            }
        }
        cerrar_parrafo(&mut tramos, &mut salida);
        self.profundidad -= 1;
        salida
    }

    /// Lo que no es texto de un párrafo. `None` si lo que sigue es texto.
    pub(super) fn bloque_propio(&mut self) -> Option<Vec<Bloque>> {
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
                let inicio = self.i;
                self.salta(1 + nombre.len());
                let texto = self.argumento();
                let tramos = self.tramos_de(texto);
                Some(vec![Bloque::Leyenda(self.enteros(tramos, inicio))])
            }
            "hrule" | "rule" => {
                self.salta(1 + nombre.len());
                Some(vec![Bloque::Regla])
            }
            "includegraphics" | "includegraphics*" => {
                self.salta(1 + nombre.len());
                let opciones = self.opcional().map(leer_opciones).unwrap_or_default();
                let ruta = self.argumento().trim().to_string();
                Some(vec![Bloque::Imagen { ruta, opciones }])
            }
            n if ORDENES_ENVOLTORIO.iter().any(|(o, _)| *o == n) => Some(self.envoltorio(n)),
            n if extensiones::buscar(n).is_some() => Some(self.extension(n)),
            "tikz" => {
                // `\tikz …;` llega hasta su punto y coma, fuera de llaves
                let t = self.resto();
                let fin = fin_de_instruccion(t);
                self.salta(fin);
                Some(vec![Bloque::Figura(format!("{}{}", self.calculos, &t[..fin]))])
            }
            _ => None,
        }
    }

    /// Una extensión: se lee lo que escribe, como si estuviera en la fuente.
    fn extension(&mut self, orden: &str) -> Vec<Bloque> {
        self.salta(1 + orden.len());
        let opciones = self.opcional().map(leer_opciones).unwrap_or_default();
        let Some(extension) = extensiones::buscar(orden) else {
            return Vec::new();
        };
        let fuente = extension.expandir(&opciones);
        let mut otro = Lector::nuevo(&fuente);
        otro.profundidad = self.profundidad + 1;
        std::mem::swap(&mut otro.salida, &mut self.salida);
        let bloques = otro.bloques(&Parada::Final);
        std::mem::swap(&mut otro.salida, &mut self.salida);
        bloques
    }

    /// Lo de dentro de una caja, leído en este mismo lector: sus notas y macros son del documento.
    fn envoltorio(&mut self, nombre: &str) -> Vec<Bloque> {
        self.salta(1 + nombre.len());
        self.opcional();
        let previos = ORDENES_ENVOLTORIO
            .iter()
            .find_map(|(o, n)| (*o == nombre).then_some(*n));
        for _ in 0..previos.unwrap_or(0) {
            self.argumento();
        }
        let dentro = self.argumento();
        let (t, i) = (self.t, self.i);
        (self.t, self.i) = (dentro, 0);
        let bloques = self.bloques(&Parada::Final);
        (self.t, self.i) = (t, i);
        bloques
    }

    pub(super) fn titulo(&mut self, nivel: u8, nombre: &str) -> Bloque {
        let inicio = self.i;
        self.salta(1 + nombre.len());
        self.opcional();
        let texto = self.argumento();
        let tramos = self.tramos_de(texto);
        Bloque::Titulo {
            nivel,
            tramos: self.enteros(tramos, inicio),
        }
    }

    /// Lo que se escribe como argumento de una orden se copia con ella: `\\section{Uno}`, no `Uno`.
    fn enteros(&self, mut tramos: Vec<Tramo>, inicio: usize) -> Vec<Tramo> {
        if let Some(rango) = self.origen(inicio) {
            for cadena in tramos.iter_mut().filter_map(Tramo::cadena_mut) {
                cadena.abarcar(rango);
            }
        }
        tramos
    }

    /// Interpreta un argumento como texto en línea.
    pub(super) fn tramos_de(&mut self, texto: &'a str) -> Vec<Tramo> {
        self.tramos_con(texto, Estilo::default())
    }

    pub(super) fn tramos_con(&mut self, texto: &'a str, estilo: Estilo) -> Vec<Tramo> {
        let mut tramos = Vec::new();
        if self.profundidad >= PROFUNDIDAD_MAXIMA {
            self.avisar("anidamiento demasiado profundo: el resto se ignora".into());
            return tramos;
        }
        let (t, i) = (self.t, self.i);
        self.t = texto;
        self.i = 0;
        self.profundidad += 1;
        let mut estilo = estilo;
        while !self.fin() {
            self.uno_anotado(estilo, &mut tramos);
            estilo = self.declarado.take().unwrap_or(estilo);
        }
        self.profundidad -= 1;
        self.t = t;
        self.i = i;
        tramos
    }

    pub(super) fn entorno(&mut self) -> Option<Vec<Bloque>> {
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
                Some(cuerpo) => vec![Bloque::Figura(format!(
                    "{}\\begin{{{nombre}}}{cuerpo}\\end{{{nombre}}}",
                    self.calculos
                ))],
                None => {
                    self.avisar(format!("figura sin cerrar: {nombre}"));
                    Vec::new()
                }
            });
        }
        if nombre == "animacion" {
            return Some(match self.hasta_fin_de(nombre) {
                Some(cuerpo) => vec![Bloque::Animacion(format!(
                    "\\begin{{animacion}}{cuerpo}\\end{{animacion}}"
                ))],
                None => {
                    self.avisar("animación sin cerrar".into());
                    Vec::new()
                }
            });
        }
        if ENTORNOS_CODIGO.contains(&nombre) {
            let opciones = self.opcional().map(leer_opciones).unwrap_or_default();
            let propio = opciones.iter().find(|(c, _)| c == "language").map(|(_, v)| v.clone());
            let lenguaje = if nombre == "minted" {
                Some(self.argumento().trim().to_string())
            } else {
                propio
            };
            return Some(match self.hasta_fin_de(nombre) {
                Some(cuerpo) => vec![Bloque::Codigo {
                    texto: cuerpo.trim_matches('\n').to_string(),
                    lenguaje,
                    opciones,
                }],
                None => Vec::new(),
            });
        }
        if ENTORNOS_TRANSPARENTES.contains(&nombre) {
            self.opcional();
            if ENTORNOS_CON_ANCHO.contains(&nombre) {
                self.argumento();
            }
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

    /// `\item[etiqueta]`: va en negrita por delante de lo que diga el elemento.
    fn etiquetar(&mut self, item: &mut Vec<Bloque>, etiqueta: &'a str) {
        let negrita = Estilo {
            negrita: true,
            ..Estilo::default()
        };
        let mut tramos = self.tramos_con(etiqueta, negrita);
        empujar(&mut tramos, " ", Estilo::default());
        match item.first_mut() {
            Some(Bloque::Parrafo(resto)) => {
                tramos.append(resto);
                *resto = tramos;
            }
            _ => item.insert(0, Bloque::Parrafo(tramos)),
        }
    }

    pub(super) fn lista(&mut self, nombre: &str) -> Bloque {
        self.opcional();
        let mut items = Vec::new();
        let parada = Parada::Item(nombre.to_string());
        // lo que hubiera antes del primer `\item` no es parte de la lista
        self.bloques(&parada);
        while self.nombre_si_orden() == Some("item") {
            self.salta("\\item".len());
            let etiqueta = self.opcional();
            let mut item = self.bloques(&parada);
            if let Some(etiqueta) = etiqueta {
                self.etiquetar(&mut item, etiqueta);
            }
            items.push(item);
        }
        self.hasta_fin_de(nombre);
        Bloque::Lista {
            ordenada: nombre == "enumerate",
            items,
        }
    }
}
