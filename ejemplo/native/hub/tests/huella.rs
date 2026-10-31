#![allow(clippy::unwrap_used, clippy::expect_used)]

//! La salida exacta de la cadena, resumida en una huella: si un cambio que solo debía acelerar la
//! mueve, ha cambiado el resultado. A mano, con otra carpeta de muestras si se quiere:
//! `NLTX_HUELLA=carpeta cargo test --release -p hub --test huella -- --ignored --nocapture`

use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use adaptador::{FiguraTikra, piezas_del_motor};
use animacion::Reproductor;
use documento::{Documento, Formato};
use primitivas::Color;

const K1: &str = include_str!("../../../assets/k1.tex");
const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };
const PILA: usize = 256 << 20;

/// Los instantes de cada animación que entran en la huella.
const INSTANTES: u32 = 12;

fn sumar(huella: &mut DefaultHasher, valor: &impl Debug) {
    format!("{valor:?}").hash(huella);
}

fn documento() -> Documento {
    let formato = Formato {
        tam: 14.0,
        color: NEGRO,
        ..Formato::default()
    };
    Documento::nuevo(formato, piezas_del_motor(NEGRO))
}

fn composiciones(huella: &mut DefaultHasher) {
    let d = documento();
    for ancho in [800.0, 430.0] {
        let c = d.componer_a(K1, ancho).unwrap();
        sumar(huella, &(&c.escena, &c.avisos));
    }
    println!("HUELLA k1        {:016x}", huella.finish());
    let Some(carpeta) = std::env::var_os("NLTX_HUELLA") else {
        return;
    };
    let mut rutas: Vec<_> = std::fs::read_dir(carpeta).unwrap().map(|e| e.unwrap().path()).collect();
    rutas.sort();
    for ruta in &rutas {
        let c = d.componer_a(&std::fs::read_to_string(ruta).unwrap(), 552.0);
        sumar(huella, &c.map(|c| (c.escena, c.avisos)));
    }
    println!("HUELLA muestras  {:016x} ({} archivos)", huella.finish(), rutas.len());
}

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

fn fotogramas(huella: &mut DefaultHasher) {
    let fuentes = animaciones_de_k1();
    for fuente in &fuentes {
        let r = Reproductor::nuevo(fuente, FiguraTikra::default()).unwrap();
        let duracion = r.animacion().duracion;
        for n in 0..INSTANTES {
            let c = r.fotograma(duracion * f64::from(n) / f64::from(INSTANTES)).unwrap();
            sumar(huella, &(&c.escena, &c.avisos));
        }
    }
    println!(
        "HUELLA fotogramas {:016x} ({} animaciones)",
        huella.finish(),
        fuentes.len()
    );
}

#[test]
#[ignore]
fn huella_de_la_salida() {
    let hilo = std::thread::Builder::new().stack_size(PILA).spawn(|| {
        let mut huella = DefaultHasher::new();
        composiciones(&mut huella);
        fotogramas(&mut huella);
    });
    hilo.unwrap().join().unwrap();
}
