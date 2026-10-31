//! `\tablaperiodica[opciones]`: una tabla de 18 columnas con huecos y un color por familia.
//!
//! - `periodos=3`: solo los primeros.
//! - `nombres=si`, `masas=si`: más líneas en cada celda.
//! - `leyenda=no`: sin los colores explicados debajo.
//! - `nobles=cyan!30` y demás familias (ver [`FAMILIAS`]): su color.
//! - Cualquier otra opción es de la tabla: `separacion=3pt`, `proporcion=1`…

use std::fmt::Write;

use super::Extension;
use super::elementos::ELEMENTOS;
use crate::tema::Opciones;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Familia {
    Alcalinos,
    Alcalinoterreos,
    Transicion,
    Postransicion,
    Metaloides,
    NoMetales,
    Halogenos,
    Nobles,
    Lantanidos,
    Actinidos,
}

pub(super) struct Elemento {
    pub(super) z: u8,
    pub(super) simbolo: &'static str,
    pub(super) nombre: &'static str,
    pub(super) masa: &'static str,
    pub(super) familia: Familia,
    pub(super) periodo: u8,
    /// 0: el bloque f, en sus dos filas aparte.
    pub(super) grupo: u8,
}

/// Cada familia: su clave en las opciones, su nombre en la leyenda y su color por defecto.
const FAMILIAS: [(Familia, &str, &str, &str); 10] = [
    (Familia::Alcalinos, "alcalinos", "Alcalinos", "#F7C6C7"),
    (
        Familia::Alcalinoterreos,
        "alcalinoterreos",
        "Alcalinotérreos",
        "#FAD7B0",
    ),
    (Familia::Transicion, "transicion", "Transición", "#FBEBB5"),
    (Familia::Postransicion, "postransicion", "Postransición", "#CFE8C6"),
    (Familia::Metaloides, "metaloides", "Metaloides", "#B9E4D4"),
    (Familia::NoMetales, "nometales", "No metales", "#BFE3F0"),
    (Familia::Halogenos, "halogenos", "Halógenos", "#C4D3F2"),
    (Familia::Nobles, "nobles", "Gases nobles", "#D5C9F0"),
    (Familia::Lantanidos, "lantanidos", "Lantánidos", "#F2C9E3"),
    (Familia::Actinidos, "actinidos", "Actínidos", "#EBC1CF"),
];

const COLUMNAS: usize = 18;
const AJUSTES: &str = "cabecera=no,filas=no,vacias=no,columnas=iguales,ajuste=escalar,proporcion=1.15,separacion=1.5,relleno=0.05,texto=#202428";

pub(super) struct Periodica;

/// El color de cada familia, con lo que digan las opciones.
fn colores(opciones: &Opciones) -> Vec<(Familia, String)> {
    let propio = |clave: &str| opciones.iter().find(|(c, _)| c == clave).map(|(_, v)| v.clone());
    FAMILIAS
        .iter()
        .map(|(f, clave, _, color)| (*f, propio(clave).unwrap_or_else(|| (*color).to_string())))
        .collect()
}

fn si(opciones: &Opciones, clave: &str, por_defecto: bool) -> bool {
    opciones
        .iter()
        .find(|(c, _)| c == clave)
        .map_or(por_defecto, |(_, v)| v == "si")
}

/// Lo que no es de la tabla periódica es de la tabla.
fn opciones_de_tabla(opciones: &Opciones) -> String {
    let propias = |clave: &str| {
        ["periodos", "nombres", "masas", "leyenda"].contains(&clave) || FAMILIAS.iter().any(|f| f.1 == clave)
    };
    let mut texto = AJUSTES.to_string();
    for (clave, valor) in opciones.iter().filter(|(c, _)| !propias(c)) {
        let _ = write!(texto, ",{clave}={{{valor}}}");
    }
    texto
}

struct Plantilla {
    colores: Vec<(Familia, String)>,
    nombres: bool,
    masas: bool,
}

impl Plantilla {
    fn celda(&self, e: &Elemento) -> String {
        let color = self
            .colores
            .iter()
            .find(|(f, _)| *f == e.familia)
            .map_or("", |(_, c)| c.as_str());
        let mut lineas = format!("\\tiny {}\\\\\\textbf{{{}}}", e.z, e.simbolo);
        if self.nombres {
            let _ = write!(lineas, "\\\\\\tiny {}", e.nombre);
        }
        if self.masas {
            let _ = write!(lineas, "\\\\\\tiny {}", masa_visible(e.masa));
        }
        format!("\\cellcolor{{{color}}}\\makecell{{{lineas}}}")
    }

    /// Una fila de la rejilla: una celda por columna, vacía si no hay nada.
    fn fila(&self, celdas: &[(usize, String)]) -> String {
        let mut fila = vec![String::new(); COLUMNAS];
        for (columna, celda) in celdas {
            if let Some(hueco) = fila.get_mut(*columna) {
                hueco.clone_from(celda);
            }
        }
        format!("{} \\\\\n", fila.join(" & "))
    }

    /// Las filas de los periodos, con el sitio del bloque f señalado en el grupo 3.
    fn periodos(&self, ultimo: u8) -> String {
        let mut salida = String::new();
        for periodo in 1..=ultimo {
            let mut celdas: Vec<(usize, String)> = ELEMENTOS
                .iter()
                .filter(|e| e.periodo == periodo && e.grupo > 0)
                .map(|e| (usize::from(e.grupo) - 1, self.celda(e)))
                .collect();
            if let Some((familia, rango)) = bloque_f(periodo) {
                let color = self
                    .colores
                    .iter()
                    .find(|(f, _)| *f == familia)
                    .map_or("", |(_, c)| c.as_str());
                celdas.push((2, format!("\\cellcolor{{{color}}}\\makecell{{\\tiny {rango}}}")));
            }
            salida.push_str(&self.fila(&celdas));
        }
        salida
    }

    /// Lantánidos y actínidos, tras una fila vacía, desde el grupo 3.
    fn bloque_f(&self, ultimo: u8) -> String {
        let mut salida = String::new();
        for periodo in (6..=ultimo).take(2) {
            if periodo == 6 {
                salida.push_str(&self.fila(&[]));
            }
            let celdas: Vec<(usize, String)> = ELEMENTOS
                .iter()
                .filter(|e| e.periodo == periodo && e.grupo == 0)
                .enumerate()
                .map(|(n, e)| (n + 2, self.celda(e)))
                .collect();
            salida.push_str(&self.fila(&celdas));
        }
        salida
    }
}

/// Una masa para leerla en una celda: con tres decimales y coma.
fn masa_visible(masa: &str) -> String {
    masa.parse::<f64>()
        .map_or_else(|_| masa.to_string(), |m| format!("{m:.3}").replace('.', ","))
}

fn bloque_f(periodo: u8) -> Option<(Familia, &'static str)> {
    match periodo {
        6 => Some((Familia::Lantanidos, "57–71")),
        7 => Some((Familia::Actinidos, "89–103")),
        _ => None,
    }
}

/// Los colores explicados: otra tabla más, con su propio estilo.
fn leyenda(colores: &[(Familia, String)]) -> String {
    let celdas: Vec<String> = FAMILIAS
        .iter()
        .zip(colores)
        .map(|(f, (_, color))| format!("\\cellcolor{{{color}}}\\tiny {}", f.2))
        .collect();
    let opciones = "cabecera=no,filas=no,columnas=iguales,ajuste=escalar,separacion=1.5,relleno=0.2";
    let filas = celdas
        .chunks(5)
        .map(|f| f.join(" & "))
        .collect::<Vec<_>>()
        .join(" \\\\\n");
    format!("\\begin{{tabular}}[{opciones}]{{*{{5}}{{c}}}}\n{filas}\n\\end{{tabular}}\n")
}

impl Extension for Periodica {
    fn orden(&self) -> &'static str {
        "tablaperiodica"
    }

    fn expandir(&self, opciones: &Opciones) -> String {
        let plantilla = Plantilla {
            colores: colores(opciones),
            nombres: si(opciones, "nombres", false),
            masas: si(opciones, "masas", false),
        };
        let pedido = opciones
            .iter()
            .find(|(c, _)| c == "periodos")
            .and_then(|(_, v)| v.parse().ok());
        let ultimo = pedido.unwrap_or(7u8).clamp(1, 7);
        let tabla = format!(
            "\\begin{{tabular}}[{}]{{*{{{COLUMNAS}}}{{c}}}}\n{}{}\\end{{tabular}}\n\n",
            opciones_de_tabla(opciones),
            plantilla.periodos(ultimo),
            plantilla.bloque_f(ultimo),
        );
        if si(opciones, "leyenda", true) {
            tabla + &leyenda(&plantilla.colores)
        } else {
            tabla
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hay_118_elementos_en_orden_y_cada_uno_en_una_casilla_propia() {
        assert_eq!(ELEMENTOS.len(), 118);
        let mut casillas = std::collections::HashSet::new();
        for (n, e) in ELEMENTOS.iter().enumerate() {
            assert_eq!(usize::from(e.z), n + 1, "{}", e.simbolo);
            assert!(e.grupo <= 18 && (1..=7).contains(&e.periodo), "{}", e.simbolo);
            if e.grupo > 0 {
                assert!(casillas.insert((e.periodo, e.grupo)), "{} repite casilla", e.simbolo);
            }
        }
    }

    #[test]
    fn los_elementos_conocidos_estan_donde_deben() {
        let en = |simbolo: &str| {
            let e = ELEMENTOS.iter().find(|e| e.simbolo == simbolo).unwrap();
            (e.z, e.periodo, e.grupo, e.familia)
        };
        assert_eq!(en("Na"), (11, 3, 1, Familia::Alcalinos));
        assert_eq!(en("Fe"), (26, 4, 8, Familia::Transicion));
        assert_eq!(en("Cl"), (17, 3, 17, Familia::Halogenos));
        assert_eq!(en("Au"), (79, 6, 11, Familia::Transicion));
        assert_eq!(en("U"), (92, 7, 0, Familia::Actinidos));
        assert_eq!(en("Og"), (118, 7, 18, Familia::Nobles));
    }

    #[test]
    fn las_masas_se_recortan_para_caber() {
        assert_eq!(masa_visible("22.9897693"), "22,990");
        assert_eq!(masa_visible("sin dato"), "sin dato");
    }

    #[test]
    fn el_bloque_f_tiene_quince_y_quince() {
        let de = |p| ELEMENTOS.iter().filter(|e| e.periodo == p && e.grupo == 0).count();
        assert_eq!((de(6), de(7)), (15, 15));
    }

    #[test]
    fn una_familia_cambia_de_color_y_el_resto_de_opciones_van_a_la_tabla() {
        let o = vec![
            ("nobles".to_string(), "red".to_string()),
            ("separacion".to_string(), "3".to_string()),
        ];
        let fuente = Periodica.expandir(&o);
        assert!(
            fuente.contains("\\cellcolor{red}\\makecell{\\tiny 2\\\\\\textbf{He}}"),
            "{fuente}"
        );
        assert!(fuente.contains("separacion={3}"));
        assert!(!fuente.contains("nobles="));
    }

    #[test]
    fn se_pueden_pedir_solo_los_primeros_periodos_y_las_lineas_de_mas() {
        let o = vec![
            ("periodos".to_string(), "2".to_string()),
            ("nombres".to_string(), "si".to_string()),
        ];
        let fuente = Periodica.expandir(&o);
        assert!(fuente.contains("Neón") && !fuente.contains("{Na}"));
        assert!(!fuente.contains("Lantano"));
    }
}
