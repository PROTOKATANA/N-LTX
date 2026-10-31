//! Lee LaTeX en flujo continuo: títulos, párrafos, listas, tablas, fórmulas y figuras.
//!
//! No es un TeX: entiende el subconjunto con el que escribe el modelo, sin páginas ni flotantes.
//! Tolera la fuente a medio escribir: lo que queda abierto al terminar se cierra solo. Lo que no
//! entiende nunca se pierde en silencio: se conserva su texto y se avisa.

mod bloques;
mod cursor;
mod texto;
mod tramos;

use texto::*;

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
    Titulo {
        nivel: u8,
        tramos: Vec<Tramo>,
    },
    Parrafo(Vec<Tramo>),
    Leyenda(Vec<Tramo>),
    Lista {
        ordenada: bool,
        items: Vec<Vec<Bloque>>,
    },
    Cita(Vec<Bloque>),
    Codigo(String),
    Formula(String),
    Figura(String),
    /// `\\begin{animacion}…`, entero: lo interpreta quien lo reproduce.
    Animacion(String),
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
