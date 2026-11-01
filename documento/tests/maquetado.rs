#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;

use documento::{
    Apartado, ColorDefinido, Colores, ColoresEditor, Documento, EstiloTabla, Formato, Fragmento, Imagen, Imagenes,
    Incrustados, Memoria, Piezas, Resaltador, Seguimiento, Tema,
};
use primitivas::{
    Caja, Capa, Color, Composicion, Compositor, ErrorComposicion, Escena, MapaDeBits, Pintura, Primitiva, Relleno,
    Segmento,
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

impl Imagenes for Falsos {
    fn imagen(&self, ruta: &str) -> Result<Imagen, String> {
        let mapa = MapaDeBits {
            id: 7,
            ancho: 2,
            alto: 1,
            rgba: vec![255; 8],
        };
        (ruta == "foto.png")
            .then(|| Imagen {
                mapa: Arc::new(mapa),
                ancho: 100.0,
                alto: 50.0,
            })
            .ok_or_else(|| format!("no se encuentra la imagen {ruta}"))
    }
}

fn falsos() -> Piezas {
    Piezas {
        incrustados: Box::new(Falsos),
        colores: Box::new(Falsos),
        resaltador: Box::new(Falsos),
        imagenes: Box::new(Falsos),
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

fn imagenes_de(c: &Composicion) -> Vec<&primitivas::Matriz> {
    c.escena
        .primitivas
        .iter()
        .filter_map(|p| match p {
            Primitiva::Imagen { matriz, .. } => Some(matriz),
            _ => None,
        })
        .collect()
}

#[test]
fn una_imagen_se_centra_y_ocupa_su_sitio() {
    let c = documento(400.0).componer(r"\includegraphics{foto.png}").unwrap();
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    let [m] = imagenes_de(&c)[..] else {
        panic!("{:?}", c.escena.primitivas)
    };
    assert_eq!((m.a, m.d), (100.0, 50.0));
    assert_eq!(m.e, 150.0, "centrada");
    assert!(c.escena.caja.alto >= m.f + m.d, "cabe en la escena");
}

#[test]
fn una_figura_con_su_imagen_y_su_leyenda() {
    let fuente = r"\begin{figure}\centering\includegraphics[width=0.5\textwidth]{foto.png}\caption{Hola}\end{figure}";
    let c = documento(400.0).componer(fuente).unwrap();
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    let [m] = imagenes_de(&c)[..] else {
        panic!("{:?}", c.escena.primitivas)
    };
    assert_eq!((m.a, m.d), (200.0, 100.0));
}

#[test]
fn una_imagen_que_no_esta_se_ve_como_fuente_y_se_avisa() {
    let c = documento(400.0).componer(r"\includegraphics{no.png}").unwrap();
    assert!(imagenes_de(&c).is_empty());
    assert!(c.avisos.iter().any(|a| a.contains("no.png")), "{:?}", c.avisos);
    assert!(!c.escena.primitivas.is_empty(), "su fuente se ve");
}

/// Lo que se copia al seleccionar de la ancla `desde` a la `hasta`.
fn copiado(c: &Composicion, fuente: &str, desde: usize, hasta: usize) -> String {
    documento::seleccion::fuente_de(&[fuente], &c.escena.anclas, desde, hasta)
}

/// La ancla de la primera letra de `trozo`, que debe estar literal en la fuente.
fn ancla_de(c: &Composicion, fuente: &str, trozo: &str) -> usize {
    let byte = fuente.find(trozo).unwrap() as u32;
    c.escena
        .anclas
        .iter()
        .position(|a| a.desde <= byte && byte < a.hasta)
        .unwrap()
}

#[test]
fn seleccionar_palabras_de_un_parrafo_copia_esos_bytes() {
    let fuente = "Hola mundo\ncruel y feliz.";
    let c = documento(600.0).componer(fuente).unwrap();
    let (a, b) = (ancla_de(&c, fuente, "mundo"), ancla_de(&c, fuente, "y feliz"));
    assert_eq!(copiado(&c, fuente, a, b), "mundo\ncruel ");
}

#[test]
fn todo_lo_seleccionado_da_la_fuente_en_bruto_con_sus_ordenes() {
    let fuente = r"Sea \textbf{una fuerza} $F=ma$ y \'agua~sin \ce{H2O}. Fin.";
    let c = documento(600.0).componer(fuente).unwrap();
    assert_eq!(copiado(&c, fuente, 0, c.escena.anclas.len()), fuente);
}

#[test]
fn tocar_un_trozo_de_una_orden_copia_la_orden_entera() {
    let fuente = r"Sea \textbf{una fuerza} final.";
    let c = documento(600.0).componer(fuente).unwrap();
    let dentro = ancla_de(&c, fuente, "fuerza");
    assert_eq!(copiado(&c, fuente, dentro, dentro + 1), r"\textbf{una fuerza}");
}

#[test]
fn una_formula_en_linea_se_copia_entera_con_sus_dolares() {
    let fuente = r"Sea $F=ma$ ya.";
    let c = documento(600.0).componer(fuente).unwrap();
    let f = ancla_de(&c, fuente, "F=ma");
    assert_eq!(copiado(&c, fuente, f, f + 1), r"$F=ma$");
}

#[test]
fn un_titulo_se_copia_con_su_orden() {
    let fuente = r"\section{Uno dos}";
    let c = documento(600.0).componer(fuente).unwrap();
    assert_eq!(copiado(&c, fuente, 1, 2), fuente);
}

#[test]
fn un_texto_y_una_animacion_se_copian_juntos_en_bruto() {
    let animacion = "\\begin{animacion}[t=0:1]\n\\begin{tikzpicture}\\draw (0,0);\\end{tikzpicture}\n\\end{animacion}";
    let fuente = format!("Antes de ella.\n\n{animacion}\n\nDespués.");
    let c = documento(600.0).componer(&fuente).unwrap();
    let (a, b) = (ancla_de(&c, &fuente, "de ella"), ancla_de(&c, &fuente, "Después"));
    let esperado = format!("de ella.\n\n{animacion}\n\n");
    assert_eq!(copiado(&c, &fuente, a, b), esperado.trim_end());
}

#[test]
fn una_formula_en_bloque_y_una_figura_son_una_sola_ancla() {
    let fuente = "\\[ x^2 \\]\n\n\\begin{tikzpicture}\\draw (0,0);\\end{tikzpicture}";
    let c = documento(600.0).componer(fuente).unwrap();
    assert_eq!(c.escena.anclas.len(), 2, "{:?}", c.escena.anclas);
    assert_eq!(copiado(&c, fuente, 0, 1), "\\[ x^2 \\]");
    assert_eq!(
        copiado(&c, fuente, 1, 2),
        "\\begin{tikzpicture}\\draw (0,0);\\end{tikzpicture}"
    );
}

#[test]
fn una_tabla_y_una_imagen_se_seleccionan_enteras() {
    let tabla = r"\begin{tabular}{ll} a & b \\ c & d \end{tabular}";
    let fuente = format!("{tabla}\n\n\\includegraphics[width=50bp]{{foto.png}}");
    let c = documento(600.0).componer(&fuente).unwrap();
    assert_eq!(c.escena.anclas.len(), 2, "{:?}", c.escena.anclas);
    assert_eq!(copiado(&c, &fuente, 0, 1), tabla);
    assert_eq!(copiado(&c, &fuente, 1, 2), r"\includegraphics[width=50bp]{foto.png}");
}

#[test]
fn los_numeros_de_una_lista_no_se_seleccionan_y_sus_elementos_si() {
    let fuente = "\\begin{enumerate}\n\\item uno\n\\item dos\n\\end{enumerate}";
    let c = documento(600.0).componer(fuente).unwrap();
    assert_eq!(copiado(&c, fuente, 0, 3), "uno");
    let (a, b) = (ancla_de(&c, fuente, "uno"), c.escena.anclas.len());
    assert_eq!(copiado(&c, fuente, a, b), "uno\n\\item dos");
}

#[test]
fn con_preambulo_los_bytes_siguen_siendo_los_del_documento_entero() {
    let fuente = "\\usepackage{a}\n\\begin{document}\nHola mundo\n\\end{document}";
    let c = documento(600.0).componer(fuente).unwrap();
    let m = ancla_de(&c, fuente, "mundo");
    assert_eq!(copiado(&c, fuente, m, c.escena.anclas.len()), "mundo");
}

#[test]
fn en_una_serie_cada_ancla_sabe_de_que_documento_es() {
    let (uno, dos) = ("Uno", "Dos");
    let apartados = [
        Apartado {
            titulo: Some("primero"),
            fuente: uno,
        },
        Apartado {
            titulo: None,
            fuente: dos,
        },
    ];
    let c = documento(600.0).componer_serie(&apartados, 600.0).unwrap();
    let copiado = documento::seleccion::fuente_de(&[uno, dos], &c.escena.anclas, 0, c.escena.anclas.len());
    assert_eq!(copiado, "Uno\n\nDos");
}

#[test]
fn las_anclas_caen_donde_esta_el_texto() {
    let c = documento(600.0).componer("ab").unwrap();
    let [a, b] = c.escena.anclas[..] else {
        panic!("{:?}", c.escena.anclas)
    };
    assert!(b.caja.x > a.caja.x, "{a:?} {b:?}");
    assert!(a.caja.ancho > 0.0 && a.caja.alto > 0.0);
    assert_eq!(
        documento::seleccion::cursor_en(&c.escena.anclas, primitivas::Punto { x: 0.1, y: 5.0 }),
        0
    );
}

/// Pide todos los tramos y para tras `parar_tras` preguntas, si lo dice.
#[derive(Default)]
struct Siguiendo {
    tramos: Vec<Composicion>,
    lentos: Vec<bool>,
    preguntas: usize,
    parar_tras: Option<usize>,
}

impl Seguimiento for Siguiendo {
    fn parar(&mut self) -> bool {
        self.preguntas += 1;
        self.parar_tras.is_some_and(|n| self.preguntas > n)
    }

    fn quiere_tramo(&mut self, _alto: f64, lento: bool) -> bool {
        self.lentos.push(lento);
        true
    }

    fn tramo(&mut self, parcial: Composicion) {
        self.tramos.push(parcial);
    }
}

const POR_PARTES: &str = r"Un párrafo con $x$.

\begin{tikzpicture}\draw (0,0) -- (1,1);\end{tikzpicture}

Otro párrafo.

\begin{quote}Una cita con $y$.\end{quote}

\begin{itemize}\item Uno con $z$.\item Dos.\end{itemize}

Y el último.";

fn es_principio<T: PartialEq>(corto: &[T], largo: &[T]) -> bool {
    corto.len() <= largo.len() && corto == &largo[..corto.len()]
}

#[test]
fn seguir_da_lo_mismo_y_cada_tramo_es_un_principio_del_resultado() {
    let d = documento(400.0);
    let entero = d.componer_a(POR_PARTES, 400.0).unwrap();
    let mut s = Siguiendo::default();
    assert!(d.seguir_a(POR_PARTES, 400.0, &mut s).unwrap() == entero);
    assert!(s.tramos.len() >= 4, "{}", s.tramos.len());
    for tramo in &s.tramos {
        assert!(es_principio(&tramo.escena.primitivas, &entero.escena.primitivas));
        assert!(es_principio(&tramo.escena.anclas, &entero.escena.anclas));
        assert!(tramo.escena.caja.alto <= entero.escena.caja.alto);
    }
}

/// Un tramo se pinta tal cual: una capa abierta dejaría lo que venga después dentro de ella.
#[test]
fn un_tramo_no_deja_nada_abierto() {
    let mut s = Siguiendo::default();
    documento(400.0).seguir_a(POR_PARTES, 400.0, &mut s).unwrap();
    for tramo in &s.tramos {
        let abiertas = tramo.escena.primitivas.iter().fold(0_i32, |n, p| match p {
            Primitiva::Abrir(_) => n + 1,
            Primitiva::Cerrar => n - 1,
            _ => n,
        });
        assert_eq!(abiertas, 0);
    }
}

/// Lo que va dentro de una cita o de una lista se coloca junto: su barra se pinta al final.
#[test]
fn una_cita_o_una_lista_no_se_parten() {
    let mut s = Siguiendo::default();
    let fuente = r"\begin{quote}Uno.

Dos.

Tres.\end{quote}";
    documento(400.0).seguir_a(fuente, 400.0, &mut s).unwrap();
    let con_algo = s.tramos.iter().filter(|t| !t.escena.primitivas.is_empty());
    assert_eq!(con_algo.count(), 0, "solo se mira antes de la cita");
}

#[test]
fn parar_deja_de_componer_y_devuelve_lo_que_hay() {
    let d = documento(400.0);
    let entero = d.componer_a(POR_PARTES, 400.0).unwrap();
    let mut s = Siguiendo {
        parar_tras: Some(1),
        ..Siguiendo::default()
    };
    let hecho = d.seguir_a(POR_PARTES, 400.0, &mut s).unwrap();
    assert_eq!(s.preguntas, 2, "no sigue preguntando");
    assert!(hecho.escena.caja.alto < entero.escena.caja.alto);
    assert!(es_principio(&hecho.escena.primitivas, &entero.escena.primitivas));
}

#[test]
fn parar_en_una_serie_no_empieza_los_documentos_siguientes() {
    let d = documento(400.0);
    let mut s = Siguiendo {
        parar_tras: Some(0),
        ..Siguiendo::default()
    };
    let serie = [apartado("uno", "Hola"), apartado("dos", "Adiós")];
    d.seguir_serie(&serie, 400.0, &mut s).unwrap();
    assert_eq!(s.preguntas, 1);
}

/// RAYO: si lo ya hecho contara como lento, reflujar un documento lo enseñaría a trozos.
#[test]
fn solo_avisa_de_lo_que_aun_no_esta_hecho() {
    let piezas = Piezas {
        incrustados: Box::new(Memoria::nueva(Falsos)),
        colores: Box::new(Falsos),
        resaltador: Box::new(Falsos),
        imagenes: Box::new(Falsos),
    };
    let d = Documento::nuevo(Formato::default(), piezas);
    let fuente = r"Algo.

\begin{tikzpicture}\draw (0,0) -- (1,1);\end{tikzpicture}

\begin{animacion}[t=0:1]\begin{tikzpicture}\draw (0,0) -- (\t,1);\end{tikzpicture}\end{animacion}

Más.";
    let mut primera = Siguiendo::default();
    d.seguir_a(fuente, 400.0, &mut primera).unwrap();
    assert_eq!(primera.lentos, [false, true, true, false]);
    let mut segunda = Siguiendo::default();
    d.seguir_a(fuente, 400.0, &mut segunda).unwrap();
    assert_eq!(segunda.lentos, [false, false, false, false]);
}
