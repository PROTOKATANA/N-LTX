//! Genera la frontera con C++ y deja a mano lo que necesita CMake para compilarla en el runner.
//!
//! `bridge` sin `compile`: el C++ se compila una sola vez, en el runner.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::{env, fs};

/// Una frontera por cada renderizador: ninguno ve lo que no usa.
const PUENTES: [&str; 2] = ["puente_lienzo", "puente_masiva"];

fn main() -> Result<(), Box<dyn Error>> {
    let _ = cxx_build::bridges(PUENTES.map(|p| format!("src/{p}.rs")));
    for puente in PUENTES {
        println!("cargo:rerun-if-changed=src/{puente}.rs");
    }
    exportar_a_cmake()
}

/// `pintor/cxx`, o donde diga `NLTX_CXX`: las cabeceras y fuentes generadas y `cxx.cc`, en un sitio
/// fijo para CMake. La carpeta se ignora a sí misma.
fn exportar_a_cmake() -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed=NLTX_CXX");
    let destino = match env::var_os("NLTX_CXX") {
        Some(ruta) => PathBuf::from(ruta),
        None => PathBuf::from(env::var("CARGO_MANIFEST_DIR")?).join("cxx"),
    };
    let generado = PathBuf::from(env::var("OUT_DIR")?).join("cxxbridge");
    for puente in PUENTES {
        let cabecera = format!("pintor/src/{puente}.rs.h");
        copiar(&generado.join("include").join(&cabecera), &destino.join(&cabecera))?;
        let fuente = format!("{puente}.rs.cc");
        copiar(
            &generado.join("sources/pintor/src").join(&fuente),
            &destino.join(&fuente),
        )?;
    }
    copiar(&generado.join("include/rust/cxx.h"), &destino.join("rust/cxx.h"))?;
    let cabecera = PathBuf::from(env::var("DEP_CXXBRIDGE1_HEADER")?);
    let raiz = cabecera
        .ancestors()
        .nth(2)
        .ok_or("la cabecera de cxx no está en su crate")?;
    // `cxx.cc` incluye `../include/cxx.h`: se conserva esa estructura
    copiar(&raiz.join("src/cxx.cc"), &destino.join("src/cxx.cc"))?;
    copiar(&cabecera, &destino.join("include/cxx.h"))?;
    fs::write(destino.join(".gitignore"), "*\n")?;
    Ok(())
}

/// Solo reescribe lo que cambió: si no, CMake lo recompilaría todo cada vez.
fn copiar(origen: &Path, destino: &Path) -> Result<(), Box<dyn Error>> {
    let carpeta = destino.parent().ok_or("destino sin carpeta")?;
    fs::create_dir_all(carpeta)?;
    // los archivos generados pueden ser enlaces simbólicos
    let contenido = fs::read(origen).map_err(|e| format!("{}: {e}", origen.display()))?;
    if fs::read(destino).is_ok_and(|actual| actual == contenido) {
        return Ok(());
    }
    fs::write(destino, contenido)?;
    Ok(())
}
