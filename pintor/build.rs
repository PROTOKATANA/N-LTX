//! Genera la frontera con C++ y deja a mano lo que necesita CMake para compilarla en el runner.
//!
//! `bridge` sin `compile`: el C++ se compila una sola vez, en el runner.

use std::path::{Path, PathBuf};
use std::{env, fs};

/// Una frontera por cada renderizador: ninguno ve lo que no usa.
const PUENTES: [&str; 2] = ["puente_lienzo", "puente_masiva"];

fn main() {
    let _ = cxx_build::bridges(PUENTES.map(|p| format!("src/{p}.rs")));
    for puente in PUENTES {
        println!("cargo:rerun-if-changed=src/{puente}.rs");
    }
    exportar_a_cmake();
}

/// `target/<perfil>/cxx-pintor`: las cabeceras y fuentes generadas, y `cxx.cc`, en un sitio fijo.
fn exportar_a_cmake() {
    let salida = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let perfil = salida.ancestors().nth(3).expect("carpeta del perfil");
    let destino = perfil.join("cxx-pintor");
    let generado = salida.join("cxxbridge");
    for puente in PUENTES {
        let cabecera = format!("pintor/src/{puente}.rs.h");
        copiar(&generado.join("include").join(&cabecera), &destino.join(&cabecera));
        let fuente = format!("{puente}.rs.cc");
        copiar(
            &generado.join("sources/pintor/src").join(&fuente),
            &destino.join(&fuente),
        );
    }
    copiar(&generado.join("include/rust/cxx.h"), &destino.join("rust/cxx.h"));
    let cabecera = PathBuf::from(env::var("DEP_CXXBRIDGE1_HEADER").expect("cabecera de cxx"));
    let raiz = cabecera.ancestors().nth(2).expect("raíz del crate cxx");
    // `cxx.cc` incluye `../include/cxx.h`: se conserva esa estructura
    copiar(&raiz.join("src/cxx.cc"), &destino.join("src/cxx.cc"));
    copiar(&cabecera, &destino.join("include/cxx.h"));
}

/// Solo reescribe lo que cambió: si no, CMake lo recompilaría todo cada vez.
fn copiar(origen: &Path, destino: &Path) {
    fs::create_dir_all(destino.parent().expect("carpeta")).expect("carpeta de cxx-pintor");
    // los archivos generados pueden ser enlaces simbólicos
    let contenido = fs::read(origen).unwrap_or_else(|e| panic!("{}: {e}", origen.display()));
    if fs::read(destino).is_ok_and(|actual| actual == contenido) {
        return;
    }
    fs::write(destino, contenido).expect("copia de cxx");
}
