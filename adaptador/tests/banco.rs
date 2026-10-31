#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Comparaciones lentas contra datos reales que no viven en el repositorio.

mod comun;

use std::collections::BTreeMap;
use std::io::BufRead;

use adaptador::{FiguraTikra, FormulaMathra};
use primitivas::{Capa, Color, Compositor, Primitiva};

/// Los `fuente.tex` de un banco de figuras.
fn figuras(banco: &str) -> Vec<std::path::PathBuf> {
    let mut rutas = Vec::new();
    for grupo in ["casos", "casos_todos"] {
        for caso in std::fs::read_dir(format!("{banco}/{grupo}")).expect("banco") {
            let ruta = caso.expect("caso").path().join("fuente.tex");
            if ruta.exists() {
                rutas.push(ruta);
            }
        }
    }
    rutas.sort();
    rutas
}

/// ¿Pierde algo la lista neutra? Cada figura se dibuja dos veces: como la dibuja `tikra` y pasando
/// por `primitivas`, y se comparan los píxeles. A mano:
/// `BANCO=carpeta cargo test --release -p adaptador --test banco la_lista -- --ignored --nocapture`.
#[test]
#[ignore]
fn la_lista_neutra_dibuja_lo_mismo_que_tikra() {
    let banco = std::env::var("BANCO").expect("BANCO=carpeta");
    let (dpi, tolerancia) = (100.0, 24);
    let (mut iguales, mut casi, mut distintas, mut con_patron, mut sin_figura) = (0, 0, 0, 0, 0);
    let mut peores = Vec::new();
    let rutas = figuras(&banco);
    for ruta in &rutas {
        let fuente = String::from_utf8_lossy(&std::fs::read(ruta).expect("fuente")).into_owned();
        let referencia = if fuente.contains("circuitikz") {
            voltik::convert(&fuente)
        } else {
            tikra::convert(&fuente)
        };
        let propia = FiguraTikra::default().componer(&fuente);
        let (Ok(referencia), Ok(propia)) = (referencia, propia) else {
            sin_figura += 1;
            continue;
        };
        assert!(comun::capas_equilibradas(&propia.escena.primitivas), "{ruta:?}");
        assert!(comun::glifos_con_contorno(&propia.escena), "{ruta:?}");
        let Some(svg) = comun::svg(&propia.escena) else {
            con_patron += 1;
            continue;
        };
        let a = comun::rasterizar(&referencia.svg, dpi).expect("referencia");
        let b = comun::rasterizar(&svg, dpi).expect("propia");
        if let Ok(carpeta) = std::env::var("VOLCAR")
            && comun::diferencia(&a, &b, tolerancia).is_none_or(|d| d >= 0.001)
        {
            let caso = ruta
                .parent()
                .and_then(|p| p.file_name())
                .unwrap()
                .to_string_lossy()
                .into_owned();
            std::fs::write(format!("{carpeta}/{caso}-tikra.svg"), &referencia.svg).expect("volcado");
            std::fs::write(format!("{carpeta}/{caso}-lista.svg"), &svg).expect("volcado");
            a.save_png(format!("{carpeta}/{caso}-tikra.png")).expect("volcado");
            b.save_png(format!("{carpeta}/{caso}-lista.png")).expect("volcado");
        }
        let nombre = ruta
            .parent()
            .and_then(|p| p.file_name())
            .unwrap()
            .to_string_lossy()
            .into_owned();
        match comun::diferencia(&a, &b, tolerancia) {
            Some(0.0) => iguales += 1,
            Some(d) if d < 0.001 => casi += 1,
            Some(d) => {
                distintas += 1;
                peores.push((d, nombre));
            }
            None => {
                distintas += 1;
                let tamanos = format!("{}×{} y {}×{}", a.width(), a.height(), b.width(), b.height());
                peores.push((1.0, format!("{nombre} (otro tamaño: {tamanos})")));
            }
        }
    }
    peores.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!(
        "figuras {} · idénticas {iguales} · diferencia < 0,1 % {casi} · distintas {distintas} · con patrón {con_patron} · sin figura {sin_figura}",
        rutas.len()
    );
    for (d, nombre) in peores.iter().take(14) {
        println!("  {:6.2} %  {nombre}", d * 100.0);
    }
}

/// Todas las fórmulas de un corpus JSONL con un campo `texto`. A mano:
/// `CORPUS=archivo.jsonl cargo test --release -p adaptador --test banco el_corpus -- --ignored --nocapture`.
#[test]
#[ignore]
fn el_corpus_se_maqueta() {
    let ruta = std::env::var("CORPUS").expect("CORPUS=archivo.jsonl");
    let compositor = FormulaMathra {
        bloque: true,
        tam: 20.0,
        estricto: false,
        color: Color { r: 0.0, g: 0.0, b: 0.0 },
    };
    let (mut limpias, mut con_avisos, mut errores) = (0u64, 0u64, 0u64);
    let mut motivos: BTreeMap<String, u64> = BTreeMap::new();
    let mut codigos: BTreeMap<String, u64> = BTreeMap::new();
    let inicio = std::time::Instant::now();
    let archivo = std::io::BufReader::new(std::fs::File::open(ruta).expect("corpus"));
    for linea in archivo.lines() {
        let linea = linea.expect("línea");
        let Ok(valor) = serde_json::from_str::<serde_json::Value>(&linea) else {
            continue;
        };
        let Some(texto) = valor["texto"].as_str() else {
            continue;
        };
        for latex in comun::dolares::formulas(texto) {
            match compositor.componer(latex) {
                Ok(c) if c.avisos.is_empty() => limpias += 1,
                Ok(c) => {
                    con_avisos += 1;
                    for aviso in &c.avisos {
                        let motivo: String = aviso.chars().take(60).collect();
                        *motivos.entry(motivo).or_default() += 1;
                        let codigo = aviso.split(':').next().unwrap_or(aviso).to_string();
                        *codigos.entry(codigo).or_default() += 1;
                    }
                }
                Err(e) => {
                    errores += 1;
                    *motivos
                        .entry(format!("ERROR {}", e.0.chars().take(50).collect::<String>()))
                        .or_default() += 1;
                }
            }
        }
    }
    let total = limpias + con_avisos + errores;
    println!(
        "fórmulas {total} · limpias {limpias} ({:.3} %) · con avisos {con_avisos} · errores {errores} · {:?}",
        limpias as f64 * 100.0 / total as f64,
        inicio.elapsed()
    );
    println!("por código: {codigos:?}");
    let mut lista: Vec<_> = motivos.into_iter().collect();
    lista.sort_by(|a, b| b.1.cmp(&a.1));
    for (motivo, n) in lista.iter().take(16) {
        println!("  {n:6}  {motivo}");
    }
}

#[test]
fn una_capa_sin_cerrar_se_detecta() {
    let abierta = [Primitiva::Abrir(Capa::BordesNitidos)];
    assert!(!comun::capas_equilibradas(&abierta));
    assert!(!comun::capas_equilibradas(&[Primitiva::Cerrar]));
}
