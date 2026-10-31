//! Lo que un documento no dice de su aspecto. La app elige un tema, cada documento o tabla puede
//! cambiarlo por encima, y los colores van por nombre: los resuelve quien sabe de colores.

use primitivas::Color;

/// Las claves de una tabla, tal como se escriben: `clave=valor`.
pub type Opciones = Vec<(String, String)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reparto {
    /// Cada columna, según lo que lleva.
    Contenido,
    Iguales,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ajuste {
    /// Si no cabe, el texto se parte en líneas.
    Partir,
    /// Si no cabe, se reduce toda la tabla.
    Escalar,
}

/// Cómo se ve una tabla. La rejilla no sabe nada de esto.
#[derive(Debug, Clone, PartialEq)]
pub struct EstiloTabla {
    /// Colores de la cabecera, repartidos por las columnas. Vacío: la primera fila es una más.
    pub cabecera: Vec<String>,
    /// El texto de la cabecera. `None`: el que mejor contraste con su fondo.
    pub texto_cabecera: Option<String>,
    /// Fondos de las filas, uno detrás de otro. Vacío: sin fondo.
    pub filas: Vec<String>,
    pub texto: String,
    /// Hueco entre celdas, en bp.
    pub separacion: f64,
    /// Margen de la celda, en em.
    pub relleno: f64,
    /// Alto mínimo de una fila, en em.
    pub alto: f64,
    /// Alto de una fila frente al ancho de sus columnas, si debe guardar la forma.
    pub proporcion: Option<f64>,
    /// Color de una regla fina bajo cada fila.
    pub reglas: Option<String>,
    /// Si ocupa toda la columna del documento o solo lo que necesita.
    pub completa: bool,
    pub reparto: Reparto,
    pub ajuste: Ajuste,
    /// Si una celda vacía lleva fondo. Sin él, la rejilla puede tener la forma que haga falta.
    pub vacias: bool,
}

impl Default for EstiloTabla {
    fn default() -> Self {
        EstiloTabla {
            cabecera: ["#22435C", "#3F6796", "#5B93BF", "#00B3B0", "#00A67E"]
                .map(String::from)
                .to_vec(),
            texto_cabecera: None,
            filas: vec!["#F1F2F4".to_string()],
            texto: "#3C4043".to_string(),
            separacion: 1.5,
            relleno: 0.5,
            alto: 0.0,
            proporcion: None,
            reglas: None,
            completa: true,
            reparto: Reparto::Contenido,
            ajuste: Ajuste::Partir,
            vacias: true,
        }
    }
}

fn lista(valor: &str) -> Vec<String> {
    if valor == "no" {
        return Vec::new();
    }
    valor
        .split(',')
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect()
}

fn opcional(valor: &str) -> Option<String> {
    (valor != "no").then(|| valor.to_string())
}

fn numero(clave: &str, valor: &str) -> Result<f64, String> {
    let cifras = valor.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    cifras
        .trim()
        .parse()
        .map_err(|_| format!("tabla: {clave} no es un número: {valor}"))
}

fn eleccion<T>(clave: &str, valor: &str, opciones: [(&str, T); 2]) -> Result<T, String> {
    let posibles = opciones.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(" o ");
    opciones
        .into_iter()
        .find_map(|(n, v)| (n == valor).then_some(v))
        .ok_or_else(|| format!("tabla: {clave} debe ser {posibles}, no {valor}"))
}

impl EstiloTabla {
    /// Cambia lo que diga una clave. El error explica qué no se ha entendido.
    pub fn aplicar(&mut self, clave: &str, valor: &str) -> Result<(), String> {
        match clave {
            "cabecera" => self.cabecera = lista(valor),
            "texto cabecera" => self.texto_cabecera = opcional(valor),
            "filas" => self.filas = lista(valor),
            "texto" => self.texto = valor.to_string(),
            "separacion" => self.separacion = numero(clave, valor)?,
            "relleno" => self.relleno = numero(clave, valor)?,
            "alto" => self.alto = numero(clave, valor)?,
            "proporcion" => self.proporcion = Some(numero(clave, valor)?),
            "reglas" => self.reglas = opcional(valor),
            "ancho" => self.completa = eleccion(clave, valor, [("completo", true), ("natural", false)])?,
            "columnas" => {
                let opciones = [("contenido", Reparto::Contenido), ("iguales", Reparto::Iguales)];
                self.reparto = eleccion(clave, valor, opciones)?;
            }
            "ajuste" => {
                let opciones = [("partir", Ajuste::Partir), ("escalar", Ajuste::Escalar)];
                self.ajuste = eleccion(clave, valor, opciones)?;
            }
            "vacias" => self.vacias = eleccion(clave, valor, [("si", true), ("no", false)])?,
            otra => return Err(format!("tabla: clave desconocida: {otra}")),
        }
        Ok(())
    }
}

/// Cómo se ve un bloque de código.
#[derive(Debug, Clone, PartialEq)]
pub struct EstiloCodigo {
    /// El tema del editor, por su nombre: da los colores de la sintaxis y los del panel.
    pub tema: String,
    pub numeros: bool,
    /// El número de la primera línea.
    pub primera: u32,
    /// Lo que dice la pestaña. `None`: el lenguaje.
    pub titulo: Option<String>,
    /// Las líneas verticales que marcan cada nivel de sangría.
    pub guias: bool,
}

impl Default for EstiloCodigo {
    fn default() -> Self {
        EstiloCodigo {
            tema: "OneHalfDark".to_string(),
            numeros: true,
            primera: 1,
            titulo: None,
            guias: true,
        }
    }
}

impl EstiloCodigo {
    /// Cambia lo que diga una clave. Los nombres son los de `listings`, que tiene muchas más: las
    /// que no deciden nada aquí, como `frame=single`, no se avisan.
    pub fn aplicar(&mut self, clave: &str, valor: &str) {
        match clave {
            "tema" => self.tema = valor.to_string(),
            "numbers" => self.numeros = valor != "none",
            "firstnumber" => self.primera = valor.parse().unwrap_or(self.primera),
            "title" | "caption" => self.titulo = Some(valor.to_string()),
            "guias" => self.guias = valor != "no",
            _ => {}
        }
    }
}

/// El aspecto por defecto de todo lo que un documento no diga.
#[derive(Debug, Clone, PartialEq)]
pub struct Tema {
    pub tabla: EstiloTabla,
    pub codigo: EstiloCodigo,
    /// Lo que no se pudo componer se enseña como fuente, en un bloque sencillo.
    pub fondo_codigo: Color,
}

impl Default for Tema {
    fn default() -> Self {
        Tema {
            tabla: EstiloTabla::default(),
            codigo: EstiloCodigo::default(),
            fondo_codigo: Color {
                r: 0.965,
                g: 0.97,
                b: 0.98,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_codigo_entiende_las_claves_de_listings_y_no_se_queja_del_resto() {
        let mut e = EstiloCodigo::default();
        assert!(e.numeros);
        for (clave, valor) in [
            ("numbers", "none"),
            ("firstnumber", "10"),
            ("title", "a.rs"),
            ("frame", "single"),
        ] {
            e.aplicar(clave, valor);
        }
        assert_eq!((e.numeros, e.primera, e.titulo.as_deref()), (false, 10, Some("a.rs")));
    }

    #[test]
    fn cada_clave_cambia_lo_suyo() {
        let mut e = EstiloTabla::default();
        e.aplicar("cabecera", "red, blue").unwrap();
        e.aplicar("separacion", "2pt").unwrap();
        e.aplicar("columnas", "iguales").unwrap();
        assert_eq!(e.cabecera, ["red", "blue"]);
        assert_eq!(e.separacion, 2.0);
        assert_eq!(e.reparto, Reparto::Iguales);
    }

    #[test]
    fn no_apaga_lo_que_se_puede_apagar() {
        let mut e = EstiloTabla::default();
        e.aplicar("cabecera", "no").unwrap();
        e.aplicar("filas", "no").unwrap();
        assert!(e.cabecera.is_empty() && e.filas.is_empty());
    }

    #[test]
    fn lo_que_no_se_entiende_se_explica() {
        let mut e = EstiloTabla::default();
        assert!(e.aplicar("colorines", "1").unwrap_err().contains("colorines"));
        assert!(e.aplicar("ancho", "enorme").unwrap_err().contains("completo o natural"));
        assert!(e.aplicar("alto", "mucho").unwrap_err().contains("mucho"));
    }
}
