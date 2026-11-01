#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Cuánto cuesta un fotograma de animación, con figuras de muchos elementos. A mano:
//! `cargo test --release -p visor --test banco_animacion -- --ignored --nocapture`

use std::time::{Duration, Instant};

use adaptador::FiguraTikra;
use animacion::{Proyector, Reproductor};
use pintor::atlas::Atlas;
use pintor::compilador::{Destino, Encuadre, compilar};
use pintor::geometria::Afin;
use pintor::lista::Lista;
use pintor::rampas::Rampas;

/// El mismo que la app: lo declara el `hub` de cada proyecto, y esta prueba no enlaza ninguno.
#[global_allocator]
static ASIGNADOR: mimalloc::MiMalloc = mimalloc::MiMalloc;

const FOTOGRAMAS: u32 = 12;

/// Los tipos de figura que más se repiten en una animación, con `n` elementos.
fn casos(n: usize) -> Vec<(&'static str, String)> {
    let envolver = |cuerpo: String| {
        format!(
            "\\begin{{animacion}}[t=0:360,duracion=2]\n\\begin{{tikzpicture}}\n{cuerpo}\n\\end{{tikzpicture}}\n\\end{{animacion}}"
        )
    };
    vec![
        (
            "puntos con macros",
            envolver(format!(
                r"\foreach \i in {{1,...,{n}}} {{
              \pgfmathsetmacro{{\x}}{{4*sin(\i*7+\t)}}
              \pgfmathsetmacro{{\y}}{{3*cos(\i*11+2*\t)}}
              \fill[blue!60] (\x,\y) circle (0.04);
            }}"
            )),
        ),
        (
            "puntos en línea",
            envolver(format!(
                r"\foreach \i in {{1,...,{n}}} {{ \fill[red] ({{4*sin(\i*7+\t)}},{{3*cos(\i*11+\t)}}) circle (0.04); }}"
            )),
        ),
        (
            "flechas",
            envolver(format!(
                r"\foreach \i in {{1,...,{n}}} {{ \draw[->] ({{4*sin(\i*7)}},{{3*cos(\i*11)}}) -- ++(\i+\t:0.3); }}"
            )),
        ),
        (
            "curva",
            envolver(format!(
                r"\draw[thick,blue,domain=0:720,samples={n}] plot ({{\x/90}},{{sin(\x+\t)+0.3*sin(5*\x-\t)}});"
            )),
        ),
        (
            "nodos con texto",
            envolver(format!(
                r"\foreach \i in {{1,...,{n}}} {{ \node at ({{4*sin(\i*7+\t)}},{{3*cos(\i*11)}}) {{\i}}; }}"
            )),
        ),
    ]
}

fn medir(nombre: &str, n: usize, fuente: &str) {
    let r = Reproductor::nuevo(fuente, FiguraTikra::default()).unwrap();
    let (mut atlas, mut rampas) = (Atlas::default(), Rampas::default());
    let (mut componer, mut pintar, mut vertices) = (0.0, 0.0, 0);
    for f in 0..FOTOGRAMAS {
        let reloj = Instant::now();
        let c = r.fotograma(f64::from(f) * 0.31).unwrap();
        componer += reloj.elapsed().as_secs_f64();
        let reloj = Instant::now();
        let mut lista = Lista::base();
        let destino = Destino {
            lista: &mut lista,
            atlas: &mut atlas,
            rampas: &mut rampas,
        };
        let encuadre = Encuadre {
            raiz: Afin::escala(2.0),
            recortada: true,
        };
        compilar(&c.escena, encuadre, destino);
        pintar += reloj.elapsed().as_secs_f64();
        vertices = lista.vertices.len();
    }
    let ms = |s: f64| s * 1e3 / f64::from(FOTOGRAMAS);
    let total = ms(componer + pintar);
    println!(
        "BANCO {nombre:<18} n={n:<6} componer {:8.2} ms  pintar {:7.2} ms  total {total:8.2} ms  {:6.1} µs/elem  {vertices} vértices",
        ms(componer),
        ms(pintar),
        total * 1e3 / n as f64,
    );
}

/// El tiempo de CPU que lleva gastado el proceso, en segundos.
fn cpu() -> f64 {
    let stat = std::fs::read_to_string("/proc/self/stat").unwrap();
    let campos: Vec<&str> = stat.rsplit(')').next().unwrap().split_whitespace().collect();
    let ticks: f64 = campos[11].parse::<f64>().unwrap() + campos[12].parse::<f64>().unwrap();
    ticks / 100.0
}

/// Lo que se ve de verdad: dos vueltas a 60 Hz, como las pide el pintor, sin esperar a nadie.
fn reproducir(nombre: &str, n: usize, fuente: &str) {
    let reloj = Instant::now();
    let p = match Proyector::nuevo(fuente, FiguraTikra::default()) {
        Ok(p) => p,
        Err(e) => {
            println!("VIVO  {nombre:<18} n={n:<6} ERROR {e}");
            return;
        }
    };
    let preparar = reloj.elapsed().as_secs_f64() * 1e3;
    let duracion = p.animacion().duracion;
    let vuelta = |desde: f64| {
        let (inicio, gasto) = (Instant::now(), cpu());
        let (mut distintos, mut anterior) = (0u32, None);
        while inicio.elapsed().as_secs_f64() < duracion {
            let reloj = Instant::now();
            let visto = p.ver(desde + inicio.elapsed().as_secs_f64());
            let huella = visto.as_ref().map(|e| std::sync::Arc::as_ptr(e) as usize);
            if huella != anterior {
                distintos += 1;
                anterior = huella;
            }
            std::thread::sleep(Duration::from_secs_f64(1.0 / 60.0).saturating_sub(reloj.elapsed()));
        }
        (f64::from(distintos) / duracion, (cpu() - gasto) / duracion * 100.0)
    };
    let (fps1, cpu1) = vuelta(0.0);
    let (fps2, cpu2) = vuelta(0.0);
    let (fps3, cpu3) = vuelta(0.0);
    println!(
        "VIVO  {nombre:<18} n={n:<6} preparar {preparar:7.0} ms  vuelta 1: {fps1:5.1} fps {cpu1:6.0} %cpu  vuelta 2: {fps2:5.1} fps {cpu2:6.0} %cpu  vuelta 3: {fps3:5.1} fps {cpu3:6.0} %cpu  guardados {}/{}",
        p.guardados(),
        p.total(),
    );
}

#[test]
#[ignore]
fn reproduccion_en_vivo() {
    let n: Vec<usize> = std::env::var("BANCO_N").map_or_else(
        |_| vec![1000, 5000, 20000],
        |v| v.split(',').map(|x| x.parse().unwrap()).collect(),
    );
    let solo = std::env::var("BANCO_SOLO").unwrap_or_default();
    for n in n {
        for (nombre, fuente) in casos(n) {
            if nombre.contains(&solo) {
                reproducir(nombre, n, &fuente);
            }
        }
    }
}

const K1: &str = include_str!("../../ejemplo/assets/k1.tex");

/// La carga real: las animaciones de K1, con su preámbulo.
fn animaciones_de_k1() -> Vec<String> {
    let preambulo = &K1[..K1.find("\\section").unwrap()];
    let trozos = K1.split("\\begin{animacion}").skip(1);
    trozos
        .map(|t| {
            let cuerpo = &t[..t.find("\\end{animacion}").unwrap()];
            format!("{preambulo}\\begin{{animacion}}{cuerpo}\\end{{animacion}}")
        })
        .collect()
}

#[test]
#[ignore]
fn coste_en_k1() {
    let hilo = std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        for (i, fuente) in animaciones_de_k1().iter().enumerate() {
            medir(&format!("k1 {i:02}"), 1, fuente);
        }
    });
    hilo.unwrap().join().unwrap();
}

#[test]
#[ignore]
fn coste_por_fotograma() {
    let n: Vec<usize> = std::env::var("BANCO_N").map_or_else(
        |_| vec![1000, 5000],
        |v| v.split(',').map(|x| x.parse().unwrap()).collect(),
    );
    let hilo = std::thread::Builder::new().stack_size(256 << 20).spawn(move || {
        for n in n {
            let solo = std::env::var("BANCO_SOLO").unwrap_or_default();
            for (nombre, fuente) in casos(n) {
                if nombre.contains(&solo) {
                    medir(nombre, n, &fuente);
                }
            }
        }
    });
    hilo.unwrap().join().unwrap();
}
