//! Lee LaTeX en flujo continuo: títulos, párrafos, listas, tablas, fórmulas y figuras.
//!
//! No es un TeX: entiende el subconjunto con el que escribe el modelo, sin páginas ni flotantes.
//! Tolera la fuente a medio escribir: lo que queda abierto al terminar se cierra solo. Lo que no
//! entiende nunca se pierde en silencio: se conserva su texto y se avisa.

mod bloques;
mod cursor;
mod tablas;
mod texto;
mod tramos;

use crate::tema::Opciones;
use crate::{ColorDefinido, extensiones};
use tablas::leer_opciones;
pub use tablas::{Alineado, Celda, Fila, Tabla};
use texto::*;

/// Hasta aquí se anidan grupos, listas y entornos: una fuente patológica no debe tumbar la pila.
const PROFUNDIDAD_MAXIMA: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Estilo {
    pub negrita: bool,
    pub cursiva: bool,
    pub mono: bool,
    pub subrayado: bool,
    /// `\textcolor`: su sitio en [`Lectura::colores`].
    pub color: Option<usize>,
    /// `\tiny`, `\large`…: el tamaño, en tanto por ciento del normal.
    pub escala: Option<u16>,
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
    Codigo {
        texto: String,
        lenguaje: Option<String>,
        /// Lo que diga el bloque: `title=`, `numbers=`…
        opciones: Opciones,
    },
    Formula(String),
    Figura(String),
    /// `\\begin{animacion}…`, entero: lo interpreta quien lo reproduce.
    Animacion(String),
    Tabla(Tabla),
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
    /// Los nombres que usa `\textcolor`, sin repetir: los resuelve quien sabe de colores.
    pub colores: Vec<String>,
    /// Los `\definecolor` del documento.
    pub definidos: Vec<ColorDefinido>,
    /// Los `\estilotabla`: cada nombre con sus opciones.
    pub estilos: Vec<(String, Opciones)>,
    /// Lo que diga `\estilocodigo` para todos los bloques de código.
    pub codigo: Opciones,
    pub avisos: Vec<String>,
}

const ENTORNOS_MATEMATICOS: [&str; 14] = [
    "tikzcd",
    "CD",
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
const ENTORNOS_TRANSPARENTES: [&str; 17] = [
    "document",
    "figure",
    "figure*",
    "table",
    "table*",
    "center",
    "abstract",
    "flushleft",
    "flushright",
    "mdframed",
    "tcolorbox",
    "adjustbox",
    "samepage",
    "sloppypar",
    "landscape",
    "minipage",
    "varwidth",
];

/// Los que llevan un argumento obligatorio antes de su contenido: el ancho.
const ENTORNOS_CON_ANCHO: [&str; 2] = ["minipage", "varwidth"];

/// Cajas que solo cambian el tamaño o el giro de lo que llevan, con cuántos argumentos delante. Lo
/// de dentro se lee como siempre: en un documento en flujo una figura ya se ajusta sola a la columna.
const ORDENES_ENVOLTORIO: [(&str, usize); 6] = [
    ("scalebox", 1),
    ("resizebox", 2),
    ("resizebox*", 2),
    ("rotatebox", 1),
    ("adjustbox", 1),
    ("parbox", 1),
];
/// Órdenes que se quedan en el preámbulo, con sus argumentos.
const ORDENES_PREAMBULO: [&str; 8] = [
    "tikzcdset",
    "usepgfplotslibrary",
    "usepackage",
    "usetikzlibrary",
    "definecolor",
    "colorlet",
    "tikzset",
    "pgfplotsset",
];
const ORDENES_MACRO: [&str; 6] = [
    "newcommand",
    "renewcommand",
    "providecommand",
    "newcommand*",
    "renewcommand*",
    "providecommand*",
];
/// Órdenes que no dejan rastro en flujo continuo. El número es cuántos argumentos llevan.
const ORDENES_IGNORADAS: [(&str, usize); 27] = [
    ("thispagestyle", 1),
    ("raggedright", 0),
    ("raggedleft", 0),
    ("makeatletter", 0),
    ("makeatother", 0),
    ("setlength", 2),
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
    l.salida.estilos = pre.salida.estilos;
    l.salida.codigo = pre.salida.codigo;
    let bloques = l.bloques(&Parada::Final);
    let mut salida = l.salida;
    salida.bloques = bloques;
    salida.definidos = colores_definidos(&salida.preambulo);
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
    /// Los `\tikzmath` leídos hasta aquí: cada figura ve los de antes de ella, no los de después.
    calculos: String,
    /// Lo último que ha dicho una declaración como `\tiny`: vale hasta que acabe su grupo.
    declarado: Option<Estilo>,
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

    fn figuras(fuente: &str) -> Vec<String> {
        solo_bloques(fuente)
            .into_iter()
            .filter_map(|b| match b {
                Bloque::Figura(f) => Some(f),
                _ => None,
            })
            .collect()
    }

    const FIGURA: &str = r"\begin{tikzpicture}\draw (0,0);\end{tikzpicture}";

    #[test]
    fn las_cajas_dejan_pasar_la_figura_que_llevan() {
        for caja in [
            format!(r"\scalebox{{.5}}{{{FIGURA}}}"),
            format!(r"\resizebox{{\textwidth}}{{!}}{{{FIGURA}}}"),
            format!(r"\rotatebox{{90}}{{{FIGURA}}}"),
            format!(
                r"\adjustbox{{varwidth=\linewidth, scale=0.8}}{{%
                {FIGURA}}}"
            ),
            format!(r"\parbox[t]{{5cm}}{{{FIGURA}}}"),
        ] {
            assert_eq!(figuras(&caja), [FIGURA], "{caja}");
            assert!(leer(&caja).avisos.is_empty(), "{caja}: {:?}", leer(&caja).avisos);
        }
    }

    #[test]
    fn los_entornos_de_maquetacion_dejan_pasar_lo_que_llevan() {
        for entorno in [
            format!(r"\begin{{minipage}}[c]{{0.7\textwidth}}{FIGURA}\end{{minipage}}"),
            format!(r"\begin{{mdframed}}[backgroundcolor=gray!12]{FIGURA}\end{{mdframed}}"),
            format!(r"\begin{{tcolorbox}}{FIGURA}\end{{tcolorbox}}"),
        ] {
            assert_eq!(figuras(&entorno), [FIGURA], "{entorno}");
            assert!(
                leer(&entorno).avisos.is_empty(),
                "{entorno}: {:?}",
                leer(&entorno).avisos
            );
        }
    }

    #[test]
    fn unas_cajas_dentro_de_otras_y_el_texto_que_sigue() {
        let f = format!(
            r"\begin{{figure*}}\begin{{minipage}}{{5cm}}\scalebox{{.5}}{{{FIGURA}}}\end{{minipage}}\end{{figure*}}"
        );
        assert_eq!(figuras(&format!("{f}\n\nFin")), [FIGURA]);
        assert!(
            solo_bloques(&format!("{f}\n\nFin"))
                .iter()
                .any(|b| matches!(b, Bloque::Parrafo(_)))
        );
    }

    #[test]
    fn las_ordenes_de_tamano_y_de_pagina_no_avisan() {
        let l = leer(r"\thispagestyle{empty}\Large hola \small adios");
        assert!(l.avisos.is_empty(), "{:?}", l.avisos);
    }

    #[test]
    fn textcolor_guarda_cada_color_una_vez_y_lo_hereda_lo_de_dentro() {
        let l = leer(r"\textcolor{red}{a \textbf{b}} \textcolor{blue}{c} \textcolor{red}{d}");
        assert_eq!(l.colores, ["red", "blue"]);
        let Some(Bloque::Parrafo(tramos)) = l.bloques.first() else {
            panic!("{:?}", l.bloques);
        };
        let colores: Vec<_> = tramos
            .iter()
            .filter_map(|t| match t {
                Tramo::Texto(texto, e) if !texto.trim().is_empty() => Some((texto.trim(), e.color)),
                _ => None,
            })
            .collect();
        assert_eq!(
            colores,
            [("a", Some(0)), ("b", Some(0)), ("c", Some(1)), ("d", Some(0))]
        );
    }

    #[test]
    fn la_etiqueta_de_un_item_va_en_negrita_delante() {
        let [Bloque::Lista { items, .. }] =
            &solo_bloques(r"\begin{description}\item[Atlas] una textura\end{description}")[..]
        else {
            panic!();
        };
        let [Bloque::Parrafo(tramos)] = &items[0][..] else {
            panic!("{:?}", items[0]);
        };
        assert!(matches!(&tramos[0], Tramo::Texto(t, e) if t.starts_with("Atlas") && e.negrita));
        assert!(matches!(tramos.last(), Some(Tramo::Texto(t, e)) if t.ends_with("textura") && !e.negrita));
    }

    #[test]
    fn un_diagrama_conmutativo_es_una_formula() {
        let fuente = r"\begin{tikzcd} A \arrow[r] & B \end{tikzcd}";
        assert!(matches!(&solo_bloques(fuente)[..], [Bloque::Formula(f)] if f == fuente));
    }

    #[test]
    fn newcommand_con_estrella_es_una_macro() {
        let l = leer(r"\newcommand*{\a}{b} hola");
        assert!(l.avisos.is_empty(), "{:?}", l.avisos);
        assert!(l.macros.contains(r"\newcommand"), "{}", l.macros);
    }

    #[test]
    fn cada_figura_ve_los_calculos_de_antes_y_no_los_de_despues() {
        let f = figuras(&format!(
            r"\tikzmath{{ \a = 1; }}{FIGURA}\tikzmath{{ \a = 2; }}{FIGURA}"
        ));
        assert!(f[0].contains(r"\a = 1") && !f[0].contains(r"\a = 2"), "{}", f[0]);
        assert!(f[1].contains(r"\a = 2"), "{}", f[1]);
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
        let [Bloque::Tabla(t)] = &b[..] else { panic!("{b:?}") };
        assert_eq!(t.filas.len(), 2);
        assert_eq!(t.filas[0].celdas[1].tramos, vec![Tramo::Formula("b".into())]);
        assert_eq!(t.columnas, [Alineado::Centro, Alineado::Centro]);
    }

    fn tabla(fuente: &str) -> Tabla {
        match leer(fuente).bloques.into_iter().next() {
            Some(Bloque::Tabla(t)) => t,
            otro => panic!("{otro:?}"),
        }
    }

    #[test]
    fn las_columnas_entienden_lo_habitual_de_latex() {
        let t = tabla(r"\begin{tabular}{|l|c@{ }r|p{3cm}*{2}{c}|}a\end{tabular}");
        use Alineado::*;
        assert_eq!(t.columnas, [Izquierda, Centro, Derecha, Izquierda, Centro, Centro]);
    }

    #[test]
    fn las_celdas_pueden_ocupar_columnas_y_filas() {
        let t = tabla(
            r"\begin{tabular}{ccc}
            \multicolumn{2}{r}{a} & b \\
            \multirow{2}{*}{c} & d & e \\
            & f & g \\
            \end{tabular}",
        );
        let a = &t.filas[0].celdas[0];
        assert_eq!((a.ancho, a.alto, a.alineado), (2, 1, Some(Alineado::Derecha)));
        assert_eq!(t.filas[1].celdas[0].alto, 2);
        assert_eq!(
            t.filas[1].celdas[0].tramos,
            vec![Tramo::Texto("c".into(), Estilo::default())]
        );
    }

    #[test]
    fn filas_y_celdas_llevan_su_color() {
        let t = tabla(
            r"\begin{tabular}{cc}\rowcolor{red!20} a & \cellcolor[HTML]{00FF00}b \\ \cellcolor{blue}c & d \\ \end{tabular}",
        );
        assert_eq!(t.filas[0].fondo.as_deref(), Some("red!20"));
        assert_eq!(t.filas[0].celdas[1].fondo.as_deref(), Some("#00FF00"));
        assert_eq!(t.filas[1].celdas[0].fondo.as_deref(), Some("blue"));
        assert_eq!(
            t.filas[1].celdas[0].tramos,
            vec![Tramo::Texto("c".into(), Estilo::default())]
        );
    }

    #[test]
    fn las_opciones_de_la_tabla_se_leen_con_sus_listas() {
        let t = tabla(r"\begin{tabular}[cabecera={red,blue},filas=no,texto cabecera=white]{c}a\end{tabular}");
        let o: Vec<(&str, &str)> = t.opciones.iter().map(|(c, v)| (c.as_str(), v.as_str())).collect();
        assert_eq!(
            o,
            [("cabecera", "red,blue"), ("filas", "no"), ("texto cabecera", "white")]
        );
    }

    #[test]
    fn los_estilos_con_nombre_valen_en_el_preambulo_y_en_el_cuerpo() {
        let l = leer(
            r"\documentclass{article}\estilotabla{a}{filas=no}\begin{document}\estilotabla{b}{base=a}\end{document}",
        );
        let nombres: Vec<&str> = l.estilos.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(nombres, ["a", "b"]);
    }

    #[test]
    fn una_declaracion_vale_hasta_que_acaba_su_grupo() {
        let t = tabla(r"\begin{tabular}{c}{\tiny a}b \\\end{tabular}");
        let [Tramo::Texto(a, e), Tramo::Texto(b, f)] = &t.filas[0].celdas[0].tramos[..] else {
            panic!("{:?}", t.filas[0].celdas[0].tramos);
        };
        assert_eq!((a.as_str(), e.escala, b.as_str(), f.escala), ("a", Some(50), "b", None));
    }

    #[test]
    fn makecell_parte_la_celda_y_cada_linea_tiene_su_tamano() {
        let t = tabla(r"\begin{tabular}{c}\makecell{\tiny 11\\\textbf{Na}} \\\end{tabular}");
        let tramos = &t.filas[0].celdas[0].tramos;
        assert!(
            matches!(&tramos[0], Tramo::Texto(x, e) if x == "11" && e.escala == Some(50)),
            "{tramos:?}"
        );
        assert_eq!(tramos[1], Tramo::Salto);
        assert!(
            matches!(&tramos[2], Tramo::Texto(x, e) if x == "Na" && e.negrita && e.escala.is_none()),
            "{tramos:?}"
        );
    }

    #[test]
    fn el_color_de_una_celda_se_guarda_en_el_documento() {
        let l = leer(r"\begin{tabular}{c}\textcolor{red}{a}\\\end{tabular}");
        assert_eq!(l.colores, ["red"]);
    }

    #[test]
    fn un_definecolor_se_guarda_venga_de_donde_venga() {
        let l = leer(r"\definecolor{marino}{HTML}{22435C}Hola");
        let d = &l.definidos[0];
        assert_eq!(
            (d.nombre.as_str(), d.modelo.as_str(), d.valor.as_str()),
            ("marino", "HTML", "22435C")
        );
    }

    #[test]
    fn un_bloque_de_codigo_guarda_su_lenguaje() {
        for (fuente, lenguaje) in [
            (r"\begin{lstlisting}[language=Rust]fn a()\end{lstlisting}", Some("Rust")),
            (
                r"\begin{lstlisting}[frame=single,language={C++}]fn a()\end{lstlisting}",
                Some("C++"),
            ),
            (r"\begin{minted}{python}fn a()\end{minted}", Some("python")),
            (r"\begin{verbatim}fn a()\end{verbatim}", None),
        ] {
            let [Bloque::Codigo { texto, lenguaje: l, .. }] = &solo_bloques(fuente)[..] else {
                panic!("{fuente}");
            };
            assert_eq!((texto.as_str(), l.as_deref()), ("fn a()", lenguaje), "{fuente}");
        }
    }

    #[test]
    fn un_bloque_guarda_sus_opciones_y_el_documento_las_de_todos() {
        let l = leer(r"\estilocodigo{tema=Nord}\begin{lstlisting}[title=a.rs,numbers=none]x\end{lstlisting}");
        assert_eq!(l.codigo, [("tema".to_string(), "Nord".to_string())]);
        let Some(Bloque::Codigo { opciones, .. }) = l.bloques.first() else {
            panic!("{:?}", l.bloques);
        };
        assert_eq!(opciones.len(), 2);
    }

    #[test]
    fn la_tabla_periodica_es_una_tabla_mas() {
        let t = tabla(r"\tablaperiodica[periodos=3]");
        assert_eq!(t.columnas.len(), 18);
        assert_eq!(t.filas.len(), 3);
        assert!(t.filas[0].celdas[1].tramos.is_empty(), "entre H y He hay huecos");
        assert!(t.filas[2].celdas[0].fondo.is_some());
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
