#![allow(clippy::unwrap_used, clippy::expect_used)]

//! La curva de la valoración de K1 no puede temblar: de pH 4 a pH 10 hay 0,1 mL, y si las muestras
//! se mueven con `\t`, el salto cambia de forma en cada fotograma. Con `domain=0.01:\t,samples=240`
//! se movía 0,6 pt entre fotogramas. La rejilla fija de 0,05 mL lo deja en 0 y da el ancho real del
//! salto (0,54 pt); la de 0,1 mL lo dobla. Cuesta 2,7 ms el último fotograma frente a 1,1 ms.
//! `cargo test -p hub --test temblor -- --nocapture` imprime lo que se desplaza.

use adaptador::FiguraTikra;
use animacion::Reproductor;
use primitivas::{Capa, Composicion, Primitiva, Punto, Segmento};

const K1: &str = include_str!("../../../assets/k1.tex");

/// Fotogramas por segundo de la app, y cuántos seguidos se miran.
const FPS: f64 = 60.0;
const FOTOGRAMAS: u32 = 24;

/// Tope de lo que un punto del salto puede moverse entre fotogramas, en puntos de la escena.
const TOPE: f64 = 0.01;

fn valoracion() -> String {
    let preambulo = &K1[..K1.find("\\section").unwrap()];
    let trozo = K1
        .split("\\begin{animacion}")
        .find(|t| t.contains("5e6"))
        .expect("la valoración sigue en K1");
    let cuerpo = &trozo[..trozo.find("\\end{animacion}").unwrap()];
    format!("{preambulo}\\begin{{animacion}}{cuerpo}\\end{{animacion}}")
}

/// La curva: el trazado con más tramos, ya con las transformaciones de sus capas aplicadas.
fn curva(c: &Composicion) -> Vec<Punto> {
    let mut pila = vec![(0.0, 0.0)];
    let mut mejor = Vec::new();
    for p in &c.escena.primitivas {
        match p {
            Primitiva::Abrir(Capa::Transformacion(m)) => {
                let (x, y) = *pila.last().unwrap();
                pila.push((x + m.e, y + m.f));
            }
            Primitiva::Abrir(_) => pila.push(*pila.last().unwrap()),
            Primitiva::Cerrar => {
                pila.pop();
            }
            Primitiva::Trazado { segmentos, .. } if segmentos.len() > mejor.len() => {
                let (dx, dy) = *pila.last().unwrap();
                mejor = segmentos
                    .iter()
                    .filter_map(|s| match s {
                        Segmento::Mover(p) | Segmento::Linea(p) => Some(Punto {
                            x: p.x + dx,
                            y: p.y + dy,
                        }),
                        _ => None,
                    })
                    .collect();
            }
            _ => {}
        }
    }
    mejor
}

/// Dónde cruza la curva la altura `y`, la primera vez.
fn cruce(curva: &[Punto], y: f64) -> Option<f64> {
    curva.windows(2).find_map(|t| {
        let (a, b) = (t[0], t[1]);
        let cruza = (a.y - y) * (b.y - y) <= 0.0 && a.y != b.y;
        cruza.then(|| a.x + (b.x - a.x) * (y - a.y) / (b.y - a.y))
    })
}

#[test]
fn el_salto_de_la_valoracion_no_tiembla() {
    let hilo = std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        let r = Reproductor::nuevo(&valoracion(), FiguraTikra::default()).unwrap();
        let curvas: Vec<Vec<Punto>> = (0..FOTOGRAMAS)
            .map(|f| {
                let segundos = 7.0 + f64::from(f) / FPS;
                curva(&r.fotograma(segundos).unwrap())
            })
            .collect();

        // las alturas salen de la propia curva: pH 1 al principio y pH 12 cerca del final
        let (y1, y12) = (curvas[0][0].y, curvas[0].last().unwrap().y);
        let altura = |ph: f64| y1 + (y12 - y1) * (ph - 1.0) / 11.0;
        let mut peor: f64 = 0.0;
        for ph in [3.5, 5.0, 7.0, 9.0, 10.5] {
            let xs: Vec<f64> = curvas.iter().map(|c| cruce(c, altura(ph)).unwrap()).collect();
            let salto = xs.windows(2).map(|p| (p[1] - p[0]).abs()).fold(0.0, f64::max);
            let (a, b) = xs.iter().fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(*x), b.max(*x)));
            println!("pH {ph:>4}: recorre {:.3} pt, entre fotogramas {salto:.3} pt", b - a);
            peor = peor.max(salto);
        }
        println!("{} puntos en la última curva", curvas.last().unwrap().len());
        assert!(peor < TOPE, "el salto se mueve {peor:.3} pt entre fotogramas");
    });
    hilo.unwrap().join().unwrap();
}
