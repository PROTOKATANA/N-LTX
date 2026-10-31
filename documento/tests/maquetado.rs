#![allow(clippy::unwrap_used, clippy::expect_used)]

use documento::{
    Apartado, ColorDefinido, Colores, ColoresEditor, Documento, EstiloTabla, Formato, Fragmento, Incrustados, Piezas,
    Resaltador, Tema,
};
use primitivas::{
    Caja, Capa, Color, Composicion, Compositor, ErrorComposicion, Escena, Pintura, Primitiva, Relleno, Segmento,
};

/// Compositor de mentira: una fórmula es una caja del ancho de su fuente.
struct Falsos;

impl Colores for Falsos {
    fn color(&self, nombre: &str, _definidos: &[ColorDefinido]) -> Option<Color> {
        match nombre {
            "rojo" | "#FF0000" => Some(ROJO),
            "azul" => Some(AZUL),
            _ if nombre.starts_with('#') => Some(Color { r: 0.5, g: 0.5, b: 0.5 }),
            _ => None,
        }
    }
}

impl Resaltador for Falsos {
    fn editor(&self, tema: &str) -> Option<ColoresEditor> {
        (tema != "inventado").then_some(COLORES_EDITOR)
    }

    fn resaltar(&self, codigo: &str, lenguaje: &str, _tema: &str) -> Vec<Fragmento> {
        if lenguaje != "falso" {
            return Vec::new();
        }
        let partir = codigo.len() / 2;
        let trozo = |texto: &str, color| Fragmento {
            texto: texto.to_string(),
            color,
            negrita: false,
            cursiva: false,
        };
        vec![trozo(&codigo[..partir], ROJO), trozo(&codigo[partir..], AZUL)]
    }
}

impl Incrustados for Falsos {
    fn formula(&self, latex: &str, _bloque: bool, tam: f64) -> Result<Composicion, ErrorComposicion> {
        if latex.contains("rota") {
            return Err(ErrorComposicion("fórmula rota".into()));
        }
        Ok(Composicion {
            escena: Escena {
                caja: Caja {
                    x: 0.0,
                    y: -tam,
                    ancho: latex.len() as f64 * tam,
                    alto: tam * 1.2,
                },
                primitivas: vec![Primitiva::Abrir(Capa::BordesNitidos), Primitiva::Cerrar],
                ..Escena::default()
            },
            avisos: Vec::new(),
        })
    }

    fn caja_de_animacion(&self, _fuente: &str) -> Result<Caja, ErrorComposicion> {
        Ok(Caja {
            x: 0.0,
            y: 0.0,
            ancho: 100.0,
            alto: 50.0,
        })
    }

    fn figura(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
        if fuente.contains("rota") {
            return Err(ErrorComposicion("figura rota".into()));
        }
        Ok(Composicion {
            escena: Escena {
                caja: Caja {
                    x: 0.0,
                    y: 0.0,
                    ancho: 1000.0,
                    alto: 200.0,
                },
                ..Escena::default()
            },
            avisos: vec!["aviso de la figura".into()],
        })
    }
}

fn falsos() -> Piezas {
    Piezas {
        incrustados: Box::new(Falsos),
        colores: Box::new(Falsos),
        resaltador: Box::new(Falsos),
    }
}

fn documento(ancho: f64) -> Documento {
    Documento::nuevo(
        Formato {
            ancho,
            ..Formato::default()
        },
        falsos(),
    )
}

fn glifos(e: &Escena) -> Vec<(f64, f64)> {
    e.primitivas
        .iter()
        .filter_map(|p| match p {
            Primitiva::Glifo { matriz, .. } => Some((matriz.e, matriz.f)),
            _ => None,
        })
        .collect()
}

fn lineas(e: &Escena) -> usize {
    let mut ys: Vec<i64> = glifos(e).iter().map(|(_, y)| y.round() as i64).collect();
    ys.dedup();
    ys.len()
}

const ROJO: Color = Color { r: 1.0, g: 0.0, b: 0.0 };
const AZUL: Color = Color { r: 0.0, g: 0.0, b: 1.0 };
const VERDE: Color = Color { r: 0.0, g: 1.0, b: 0.0 };
const COLORES_EDITOR: ColoresEditor = ColoresEditor {
    fondo: Color { r: 0.1, g: 0.1, b: 0.1 },
    texto: Color { r: 0.8, g: 0.8, b: 0.8 },
    numeros: VERDE,
};

/// La fuente de cada glifo, por orden.
fn fuentes(fuente: &str) -> Vec<u8> {
    let c = Documento::nuevo(Formato::default(), falsos())
        .componer_a(fuente, 400.0)
        .unwrap();
    c.escena
        .primitivas
        .iter()
        .filter_map(|p| match p {
            Primitiva::Glifo { glifo, .. } => Some(glifo.fuente),
            _ => None,
        })
        .collect()
}

/// Los rellenos sólidos de una composición: su caja y su color.
fn rellenos(c: &Composicion) -> Vec<(f64, f64, Color)> {
    c.escena
        .primitivas
        .iter()
        .filter_map(|p| match p {
            Primitiva::Trazado {
                segmentos,
                relleno:
                    Some(Relleno {
                        pintura: Pintura::Solida(color),
                        ..
                    }),
                ..
            } => match segmentos.first() {
                Some(Segmento::Mover(q)) => Some((q.x, q.y, *color)),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn componer_tabla(fuente: &str) -> Composicion {
    let formato = Formato {
        tema: Tema {
            tabla: EstiloTabla {
                cabecera: vec!["rojo".into(), "azul".into()],
                filas: vec!["azul".into()],
                texto: "rojo".into(),
                ..EstiloTabla::default()
            },
            ..Tema::default()
        },
        ..Formato::default()
    };
    Documento::nuevo(formato, falsos()).componer_a(fuente, 400.0).unwrap()
}

#[test]
fn el_tema_colorea_la_cabecera_por_columnas_y_las_filas() {
    let c = componer_tabla(r"\begin{tabular}{ccc}a & b & c \\ d & e & f \\ \end{tabular}");
    let colores: Vec<Color> = rellenos(&c).iter().map(|r| r.2).collect();
    assert_eq!(colores.len(), 6, "{colores:?}");
    assert_eq!((colores[0], colores[2]), (ROJO, AZUL), "los extremos de la paleta");
    assert!(
        colores[1].r > 0.4 && colores[1].b > 0.4,
        "en medio, la mezcla: {:?}",
        colores[1]
    );
    assert!(colores[3..].iter().all(|c| *c == AZUL));
}

#[test]
fn el_documento_manda_sobre_el_tema_y_la_celda_sobre_la_fila() {
    let c = componer_tabla(
        r"\estilotabla{mio}{cabecera=no,filas=rojo}
        \begin{tabular}[estilo=mio]{cc}a & \cellcolor{azul}b \\ \end{tabular}",
    );
    let colores: Vec<Color> = rellenos(&c).iter().map(|r| r.2).collect();
    assert_eq!(colores, [ROJO, AZUL]);
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
}

#[test]
fn un_estilo_hereda_de_otro_y_una_opcion_lo_corrige() {
    let c = componer_tabla(
        r"\estilotabla{base}{cabecera=no,filas=rojo}
        \estilotabla{hija}{estilo=base}
        \begin{tabular}[estilo=hija,filas=azul]{c}a \\ \end{tabular}",
    );
    assert_eq!(rellenos(&c).iter().map(|r| r.2).collect::<Vec<_>>(), [AZUL]);
}

#[test]
fn con_vacias_no_un_hueco_no_se_pinta_y_la_rejilla_toma_cualquier_forma() {
    let c = componer_tabla(r"\begin{tabular}[cabecera=no,vacias=no]{ccc}a & & b \\ \end{tabular}");
    assert_eq!(rellenos(&c).len(), 2);
    let c = componer_tabla(r"\begin{tabular}[cabecera=no]{ccc}a & & b \\ \end{tabular}");
    assert_eq!(rellenos(&c).len(), 3);
}

#[test]
fn una_celda_alta_tapa_lo_que_ocupa_y_una_ancha_empuja_lo_que_sigue() {
    let c = componer_tabla(
        r"\begin{tabular}[cabecera=no,filas=azul]{ccc}
        \multirow{2}{*}{a} & \multicolumn{2}{c}{b} \\
        & c & d \\
        \end{tabular}",
    );
    assert_eq!(rellenos(&c).len(), 4, "a, b, c y d");
}

#[test]
fn lo_que_no_se_entiende_de_una_tabla_se_avisa() {
    let c = componer_tabla(r"\begin{tabular}[colorines=1,estilo=nada,filas=violeta]{c}a\\ \end{tabular}");
    let avisos = c.avisos.join(" | ");
    for esperado in ["colorines", "nada", "violeta"] {
        assert!(avisos.contains(esperado), "{avisos}");
    }
}

#[test]
fn la_posicion_de_latex_no_es_una_clave_desconocida() {
    let c = componer_tabla(r"\begin{tabular}[t]{c}a\\ \end{tabular}");
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
}

#[test]
fn una_tabla_que_no_cabe_se_reduce_si_se_le_pide() {
    let alto_con = |ajuste: &str| {
        let celda = "uno dos tres cuatro cinco seis siete";
        let fila = [celda; 6].join(" & ");
        let c = componer_tabla(&format!(
            r"\begin{{tabular}}[cabecera=no,columnas=iguales,ajuste={ajuste}]{{*{{6}}{{c}}}}
            {fila} \\ \end{{tabular}}"
        ));
        c.escena.caja.alto
    };
    assert!(alto_con("escalar") < alto_con("partir"));
}

#[test]
fn una_formula_no_se_sale_de_su_columna_aunque_sean_iguales() {
    let formula = "abcdefghijklmnopqrstuvwxyz";
    let ancho_de_la_formula = formula.len() as f64 * Formato::default().tam;
    let c = componer_tabla(&format!(
        r"\begin{{tabular}}[cabecera=no,columnas=iguales]{{ccc}}${formula}$ & a & b \\ \end{{tabular}}"
    ));
    let x: Vec<f64> = rellenos(&c).iter().map(|r| r.0).collect();
    assert!(
        x[1] - x[0] >= ancho_de_la_formula,
        "{x:?}: la primera columna mide menos que su fórmula"
    );
    assert!(x[2] + 1.0 < 400.0, "{x:?}: la tabla sigue dentro de la página");
}

#[test]
fn si_ni_lo_que_no_se_parte_cabe_la_tabla_se_reduce() {
    let formula = "abcdefghijklmnopqrstuvwxyz";
    let c = componer_tabla(&format!(
        r"\begin{{tabular}}[cabecera=no]{{ccc}}${formula}$ & ${formula}$ & ${formula}$ \\ \end{{tabular}}"
    ));
    let x: Vec<f64> = rellenos(&c).iter().map(|r| r.0).collect();
    assert!(x[2] < 400.0, "{x:?}");
    assert!(x[2] - x[1] < 400.0 / 2.0, "{x:?}: las tres columnas caben");
}

#[test]
fn lo_que_cabe_justo_no_se_parte_a_ningun_ancho() {
    let doble = |ancho: f64| {
        let fuente = format!(
            r"\begin{{tabular}}[cabecera=no,columnas=iguales,ajuste=escalar,relleno=0.05]{{*{{18}}{{c}}}}
            {} \\ \end{{tabular}}",
            ["\\textbf{Am}"; 18].join(" & ")
        );
        let c = Documento::nuevo(Formato::default(), falsos())
            .componer_a(&fuente, ancho)
            .unwrap();
        let alturas: std::collections::BTreeSet<i64> = c
            .escena
            .primitivas
            .iter()
            .filter_map(|p| match p {
                Primitiva::Glifo { matriz, .. } => Some((matriz.f * 100.0) as i64),
                _ => None,
            })
            .collect();
        alturas.len()
    };
    for ancho in (120..900).step_by(3) {
        assert_eq!(doble(f64::from(ancho)), 1, "a {ancho} bp hay más de una línea");
    }
}

fn colores_de_glifos(c: &Composicion) -> Vec<Color> {
    c.escena
        .primitivas
        .iter()
        .filter_map(|p| match p {
            Primitiva::Glifo { color, .. } => Some(*color),
            _ => None,
        })
        .collect()
}

fn componer_codigo(fuente: &str) -> Composicion {
    Documento::nuevo(Formato::default(), falsos())
        .componer_a(fuente, 400.0)
        .unwrap()
}

/// Cuántos glifos llevan el color de los números.
fn numeros(fuente: &str) -> usize {
    colores_de_glifos(&componer_codigo(fuente))
        .iter()
        .filter(|c| **c == VERDE)
        .count()
}

/// Cuántos trazos discontinuos lleva: en un bloque de código, las guías de sangría.
fn guias(fuente: &str) -> usize {
    let c = componer_codigo(fuente);
    c.escena
        .primitivas
        .iter()
        .filter(|p| matches!(p, Primitiva::Trazado { trazo: Some(t), .. } if t.discontinuo.is_some()))
        .count()
}

#[test]
fn la_sangria_lleva_su_guia_y_se_puede_quitar() {
    let codigo = "a {\n    b {\n        c\n\n        d\n    }\n}";
    assert_eq!(
        guias(&format!("\\begin{{lstlisting}}\n{codigo}\n\\end{{lstlisting}}")),
        2,
        "dos niveles"
    );
    assert_eq!(
        guias(&format!(
            "\\begin{{lstlisting}}[guias=no]\n{codigo}\n\\end{{lstlisting}}"
        )),
        0
    );
    assert_eq!(
        guias("\\begin{lstlisting}\na\nb\n\\end{lstlisting}"),
        0,
        "sin sangría no hay guías"
    );
    assert_eq!(
        guias("\\begin{lstlisting}\na\n\tb\n\\end{lstlisting}"),
        1,
        "un tabulador también sangra"
    );
}

#[test]
fn cada_linea_lleva_su_numero() {
    assert_eq!(numeros("\\begin{verbatim}\na\nb\nc\n\\end{verbatim}"), 3);
    assert_eq!(
        numeros("\\begin{lstlisting}\na\n\nc\n\\end{lstlisting}"),
        3,
        "también las vacías"
    );
}

#[test]
fn los_numeros_empiezan_donde_se_pida_y_se_pueden_quitar() {
    assert_eq!(
        numeros("\\begin{lstlisting}[firstnumber=99]\na\nb\n\\end{lstlisting}"),
        2 + 3,
        "99 y 100"
    );
    assert_eq!(numeros("\\begin{lstlisting}[numbers=none]\na\nb\n\\end{lstlisting}"), 0);
    assert_eq!(
        numeros("\\estilocodigo{numbers=none}\n\\begin{verbatim}\na\n\\end{verbatim}"),
        0
    );
}

#[test]
fn una_linea_larga_que_se_parte_lleva_un_solo_numero() {
    let larga = "palabra ".repeat(60);
    let c = componer_codigo(&format!("\\begin{{lstlisting}}\n{larga}\n\\end{{lstlisting}}"));
    assert_eq!(colores_de_glifos(&c).iter().filter(|c| **c == VERDE).count(), 1);
    let doble = componer_codigo("\\begin{lstlisting}\npalabra\n\\end{lstlisting}");
    assert!(c.escena.caja.alto > 2.0 * doble.escena.caja.alto, "se ha partido");
}

#[test]
fn la_pestana_dice_el_titulo_y_si_no_hay_el_lenguaje() {
    let sin = componer_codigo("\\begin{verbatim}\na\n\\end{verbatim}");
    let con = componer_codigo("\\begin{lstlisting}[language=Rust]\na\n\\end{lstlisting}");
    let titulo = componer_codigo("\\begin{lstlisting}[language=Rust,title=programa.rs]\na\n\\end{lstlisting}");
    let cuantos = |c: &Composicion| colores_de_glifos(c).len();
    assert_eq!(cuantos(&sin), 2, "el número y la a");
    assert_eq!(cuantos(&con), 2 + "Rust".len());
    assert_eq!(
        cuantos(&titulo),
        2 + "programa.rs".len(),
        "con título, el lenguaje no se repite"
    );
    assert!(con.escena.caja.alto > sin.escena.caja.alto, "la barra ocupa su sitio");
}

#[test]
fn el_codigo_se_colorea_por_trozos_y_sin_lenguaje_no() {
    let con = colores_de_glifos(&componer_codigo(
        r"\begin{lstlisting}[language=falso,numbers=none]abcd\end{lstlisting}",
    ));
    assert_eq!(con.iter().filter(|c| **c == ROJO).count(), 2);
    assert_eq!(con.iter().filter(|c| **c == AZUL).count(), 2);
    let sin = colores_de_glifos(&componer_codigo(r"\begin{verbatim}abcd\end{verbatim}"));
    assert!(!sin.contains(&ROJO) && sin.contains(&COLORES_EDITOR.texto));
}

#[test]
fn un_lenguaje_que_no_se_sabe_resaltar_se_avisa_y_el_codigo_se_ve() {
    let c = componer_codigo(r"\begin{lstlisting}[language=klingon]abcd\end{lstlisting}");
    assert!(c.avisos.iter().any(|a| a.contains("klingon")), "{:?}", c.avisos);
    assert!(c.escena.primitivas.iter().any(|p| matches!(p, Primitiva::Glifo { .. })));
}

#[test]
fn un_tema_desconocido_se_avisa_y_el_codigo_se_ve() {
    let c = componer_codigo(r"\begin{lstlisting}[tema=inventado]abcd\end{lstlisting}");
    assert!(c.avisos.iter().any(|a| a.contains("inventado")), "{:?}", c.avisos);
    assert_eq!(colores_de_glifos(&c).len(), 5);
}

#[test]
fn la_puntuacion_tras_la_negrita_no_es_negrita() {
    for (fuente, negritas) in [
        (r"\textbf{Texto}: a", 5),
        (r"\textbf{negrita}, a", 7),
        (r"\textbf{Temático}: a", 8),
    ] {
        let f = fuentes(fuente);
        assert_eq!(f[..negritas], vec![f[0]; negritas], "{fuente}");
        assert_ne!(f[negritas], f[0], "{fuente}");
    }
}

#[test]
fn textcolor_llega_a_los_glifos_y_un_color_desconocido_no_estorba() {
    let colores = |fuente: &str| -> Vec<Color> {
        let c = Documento::nuevo(Formato::default(), falsos())
            .componer_a(fuente, 400.0)
            .unwrap();
        c.escena
            .primitivas
            .iter()
            .filter_map(|p| match p {
                Primitiva::Glifo { color, .. } => Some(*color),
                _ => None,
            })
            .collect()
    };
    assert!(colores(r"\textcolor{rojo}{a}").iter().all(|c| *c == ROJO));
    assert!(colores(r"a\textcolor{rojo}{b}").contains(&Formato::default().color));
    assert!(!colores(r"\textcolor{nada}{a}").is_empty());
    assert!(!colores(r"\textcolor{nada}{a}").contains(&ROJO));
}

#[test]
fn el_texto_trae_sus_contornos() {
    let c = documento(400.0).componer("Hola mundo").unwrap();
    let g: Vec<_> = c
        .escena
        .primitivas
        .iter()
        .filter(|p| matches!(p, Primitiva::Glifo { .. }))
        .collect();
    assert_eq!(g.len(), 10, "el espacio también es un glifo");
    assert!(c.escena.contornos.values().any(|s| !s.is_empty()));
    assert!(c.escena.caja.alto > 0.0);
}

#[test]
fn un_ancho_menor_parte_el_parrafo_en_mas_lineas() {
    let fuente = "Un párrafo con bastantes palabras para que se parta en varias líneas al estrecharlo.";
    let anchas = lineas(&documento(800.0).componer(fuente).unwrap().escena);
    let estrechas = lineas(&documento(120.0).componer(fuente).unwrap().escena);
    assert_eq!(anchas, 1);
    assert!(estrechas > 3, "{estrechas}");
}

#[test]
fn el_mismo_documento_reflujo_a_otro_ancho() {
    let fuente = "Un párrafo con bastantes palabras para que se parta en varias líneas al estrecharlo.";
    let d = documento(800.0);
    let estrecho = d.componer_a(fuente, 120.0).unwrap().escena;
    assert!(lineas(&estrecho) > 3);
    assert_eq!(estrecho.caja.ancho, 120.0);
    assert_eq!(
        lineas(&d.componer(fuente).unwrap().escena),
        1,
        "el ancho propio no cambia"
    );
}

#[test]
fn una_formula_en_linea_empuja_el_texto_que_sigue() {
    let tam = Formato::default().tam;
    let x_tras = |fuente: &str| {
        let e = documento(800.0).componer(fuente).unwrap().escena;
        glifos(&e).last().map(|p| p.0).unwrap()
    };
    let sin = x_tras("a b");
    let con = x_tras("a $xxxx$ b");
    // la fórmula de mentira mide 4 cuerpos
    assert!(((con - sin) - 4.0 * tam).abs() < tam, "{sin} {con}");
}

#[test]
fn una_formula_en_linea_no_se_pierde_a_ningun_ancho() {
    let formulas = |e: &Escena| {
        e.primitivas
            .iter()
            .filter(|p| matches!(p, Primitiva::Abrir(Capa::Transformacion(_))))
            .count()
    };
    for ancho in 30..400 {
        let e = documento(ancho as f64)
            .componer("La ley $x$ cambia en vivo.")
            .unwrap()
            .escena;
        assert_eq!(formulas(&e), 1, "a {ancho} bp");
    }
}

#[test]
fn una_formula_en_linea_se_coloca_en_su_linea_base() {
    let e = documento(800.0).componer("a $x$ b").unwrap().escena;
    let base = glifos(&e)[0].1;
    let y = e.primitivas.iter().find_map(|p| match p {
        Primitiva::Abrir(Capa::Transformacion(m)) => Some(m.f),
        _ => None,
    });
    assert!((y.unwrap() - base).abs() < 1e-3);
}

#[test]
fn las_capas_quedan_equilibradas_y_los_bloques_apilados() {
    let c = documento(400.0)
        .componer("\\section{T}\nUno $x$.\n\n\\begin{itemize}\\item a\\item $$y$$\\end{itemize}\n\n\\begin{tikzpicture}\\end{tikzpicture}\n\nFin")
        .unwrap();
    let (abre, cierra) = c.escena.primitivas.iter().fold((0, 0), |(a, b), p| match p {
        Primitiva::Abrir(_) => (a + 1, b),
        Primitiva::Cerrar => (a, b + 1),
        _ => (a, b),
    });
    assert_eq!(abre, cierra);
    assert!(c.avisos.iter().any(|a| a == "aviso de la figura"));
}

#[test]
fn una_figura_ancha_se_reduce_para_caber() {
    let e = documento(400.0)
        .componer("\\begin{tikzpicture}\\end{tikzpicture}")
        .unwrap()
        .escena;
    let m = e.primitivas.iter().find_map(|p| match p {
        Primitiva::Abrir(Capa::Transformacion(m)) => Some(*m),
        _ => None,
    });
    assert!((m.unwrap().a - 0.4).abs() < 1e-9);
    assert!(e.caja.alto < 120.0, "la altura también se reduce: {}", e.caja.alto);
}

#[test]
fn una_formula_en_bloque_ancha_se_reduce_para_caber() {
    let escala = |ancho: f64, fuente: &str| {
        let e = documento(ancho).componer(fuente).unwrap().escena;
        e.primitivas.iter().find_map(|p| match p {
            Primitiva::Abrir(Capa::Transformacion(m)) => Some(m.a),
            _ => None,
        })
    };
    let ancha = r"\[ xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx \]";
    assert!(escala(100.0, ancha).unwrap() < 0.5);
    assert_eq!(escala(4000.0, ancha), Some(1.0), "la que cabe no cambia");
}

fn apartado<'a>(titulo: &'a str, fuente: &'a str) -> Apartado<'a> {
    Apartado {
        titulo: Some(titulo),
        fuente,
    }
}

#[test]
fn una_serie_apila_sus_documentos_con_su_titulo() {
    let solo = documento(400.0).componer("Hola").unwrap().escena;
    let serie = documento(400.0)
        .componer_serie(&[apartado("uno", "Hola"), apartado("dos", "Hola")], 400.0)
        .unwrap()
        .escena;
    assert!(glifos(&serie).len() >= 2 * glifos(&solo).len() + 6, "con los títulos");
    assert!(serie.caja.alto > 2.0 * solo.caja.alto);
}

#[test]
fn en_una_serie_cada_documento_tiene_su_preambulo() {
    let con = "\\documentclass{article}\\usepackage{a}\\begin{document}\\[ x \\]\\end{document}";
    let sin = "\\[ y \\]";
    let e = documento(400.0)
        .componer_serie(
            &[
                Apartado {
                    titulo: None,
                    fuente: con,
                },
                Apartado {
                    titulo: None,
                    fuente: sin,
                },
            ],
            400.0,
        )
        .unwrap()
        .escena;
    let formulas = e
        .primitivas
        .iter()
        .filter(|p| matches!(p, Primitiva::Abrir(Capa::Transformacion(_))))
        .count();
    assert_eq!(formulas, 2, "el segundo \\begin{{document}} no se pierde");
}

#[test]
fn en_una_serie_las_macros_de_uno_no_llegan_al_siguiente() {
    use std::sync::{Arc, Mutex};

    /// Apunta lo que le llega a componer.
    struct Apuntador(Arc<Mutex<Vec<String>>>);

    impl Incrustados for Apuntador {
        fn formula(&self, latex: &str, b: bool, t: f64) -> Result<Composicion, ErrorComposicion> {
            self.0.lock().unwrap().push(latex.to_string());
            Falsos.formula(latex, b, t)
        }

        fn figura(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
            Falsos.figura(fuente)
        }

        fn caja_de_animacion(&self, fuente: &str) -> Result<Caja, ErrorComposicion> {
            Falsos.caja_de_animacion(fuente)
        }
    }

    let vistas = Arc::new(Mutex::new(Vec::new()));
    let d = Documento::nuevo(Formato::default(), Piezas::de(Apuntador(vistas.clone())));
    let primero = "\\newcommand{\\a}{b}\n\\[ \\a \\]";
    let segundo = "\\[ \\a \\]";
    d.componer_serie(
        &[
            Apartado {
                titulo: None,
                fuente: primero,
            },
            Apartado {
                titulo: None,
                fuente: segundo,
            },
        ],
        400.0,
    )
    .unwrap();
    let vistas = vistas.lock().unwrap();
    assert!(vistas[0].contains("newcommand"), "{vistas:?}");
    assert!(!vistas[1].contains("newcommand"), "{vistas:?}");
}

#[test]
fn los_avisos_de_una_serie_se_escriben_bajo_su_documento() {
    let figura_rota = r"\begin{tikzpicture}rota\end{tikzpicture}";
    let c = documento(400.0)
        .componer_serie(&[apartado("a", figura_rota), apartado("b", "Hola")], 400.0)
        .unwrap();
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    let solo_el_titulo = documento(400.0)
        .componer_serie(&[apartado("a", ""), apartado("b", "Hola")], 400.0)
        .unwrap();
    assert!(
        glifos(&c.escena).len() > glifos(&solo_el_titulo.escena).len() + 10,
        "se leen"
    );
}

#[test]
fn lo_que_no_se_puede_componer_se_ve_como_fuente() {
    let c = documento(400.0).componer("a $rota$ b").unwrap();
    assert!(c.avisos.iter().any(|a| a.contains("fórmula rota")));
    assert!(glifos(&c.escena).len() > 5, "la fuente de la fórmula se ve");
    let c = documento(400.0)
        .componer(r"\begin{tikzpicture}rota\end{tikzpicture}")
        .unwrap();
    assert!(c.avisos.iter().any(|a| a.contains("figura rota")));
}

#[test]
fn listas_tablas_codigo_y_citas() {
    let c = documento(400.0)
        .componer(
            r"\begin{enumerate}\item uno\item dos\end{enumerate}
\begin{tabular}{cc}a & b \\ c & d\end{tabular}
\begin{verbatim}
let x = 1;
\end{verbatim}
\begin{quote}cita\end{quote}",
        )
        .unwrap();
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    assert!(glifos(&c.escena).len() > 15);
    assert!(c.escena.caja.alto > 40.0);
}

#[test]
fn los_acentos_y_las_tildes_tienen_glifo() {
    let c = documento(400.0).componer("El niño comió piñón, ¿verdad? ¡Sí!").unwrap();
    let (mut vacios, mut con) = (0, 0);
    for contorno in c.escena.contornos.values() {
        if contorno.is_empty() { vacios += 1 } else { con += 1 }
    }
    // solo el espacio no tiene contorno
    assert!(vacios <= 1, "{vacios} vacíos, {con} con contorno");
}

#[test]
fn una_animacion_reserva_su_sitio_en_el_flujo() {
    let e = documento(400.0)
        .componer("Antes\n\n\\begin{animacion}\\draw (0,0);\\end{animacion}\n\nDespués")
        .unwrap()
        .escena;
    let [zona] = &e.zonas[..] else { panic!("{:?}", e.zonas) };
    assert_eq!((zona.caja.ancho, zona.caja.alto), (100.0, 50.0));
    assert!((zona.caja.x - 150.0).abs() < 1e-9, "centrada: {}", zona.caja.x);
    let despues = glifos(&e).last().unwrap().1;
    assert!(despues > zona.caja.y + zona.caja.alto, "el texto sigue debajo");
}

#[test]
fn una_escena_de_texto_cruza_hilos() {
    fn es_send<T: Send + Sync>() {}
    es_send::<Documento>();
}
