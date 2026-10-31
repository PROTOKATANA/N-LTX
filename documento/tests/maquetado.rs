use documento::{Documento, Formato, Incrustados};
use primitivas::{Caja, Capa, Composicion, Compositor, ErrorComposicion, Escena, Primitiva};

/// Compositor de mentira: una fórmula es una caja del ancho de su fuente.
struct Falsos;

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

fn documento(ancho: f64) -> Documento {
    Documento::nuevo(
        Formato {
            ancho,
            ..Formato::default()
        },
        Box::new(Falsos),
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
fn una_escena_de_texto_cruza_hilos() {
    fn es_send<T: Send + Sync>() {}
    es_send::<Documento>();
}
