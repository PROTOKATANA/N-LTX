//! Una carpeta de archivos `.tex` para ver cómo se comporta el motor con LaTeX de fuera. Las muestras
//! son datos del usuario y no se versionan.

use std::fs;
use std::path::Path;

use documento::Apartado;

#[derive(Debug, Clone, PartialEq)]
struct Muestra {
    titulo: String,
    fuente: String,
}

/// Se leen al arrancar: nadie más debe tocar el disco.
#[derive(Debug, Default)]
pub struct Muestras(Vec<Muestra>);

impl Muestras {
    /// Por orden de nombre. Lo que no se pueda leer se salta, y sin carpeta no hay muestras.
    pub fn de_carpeta(carpeta: &Path) -> Muestras {
        let Ok(entradas) = fs::read_dir(carpeta) else {
            return Muestras::default();
        };
        let mut rutas: Vec<_> = entradas.filter_map(|e| e.ok().map(|e| e.path())).collect();
        rutas.retain(|r| r.extension().is_some_and(|e| e == "tex"));
        rutas.sort();
        let leidas = rutas.iter().filter_map(|r| {
            let nombre = r.file_stem()?.to_string_lossy().into_owned();
            Some((nombre, fs::read_to_string(r).ok()?))
        });
        let muestras = leidas.enumerate().map(|(i, (nombre, fuente))| Muestra {
            titulo: format!("{} · {nombre}", i + 1),
            fuente,
        });
        Muestras(muestras.collect())
    }

    pub fn cuantas(&self) -> usize {
        self.0.len()
    }

    /// Cada muestra como un documento de una serie.
    pub fn apartados(&self) -> impl Iterator<Item = Apartado<'_>> {
        self.0.iter().map(|m| Apartado {
            titulo: Some(&m.titulo),
            fuente: &m.fuente,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una carpeta propia de cada prueba.
    fn carpeta(nombre: &str) -> std::path::PathBuf {
        let c = std::env::temp_dir().join(format!("nltx-muestras-{}-{nombre}", std::process::id()));
        let _ = fs::remove_dir_all(&c);
        fs::create_dir_all(&c).unwrap();
        c
    }

    #[test]
    fn van_por_orden_de_nombre_numeradas_y_solo_los_tex() {
        let c = carpeta("orden");
        fs::write(c.join("b.tex"), "dos").unwrap();
        fs::write(c.join("a.tex"), "uno").unwrap();
        fs::write(c.join("nota.txt"), "nada").unwrap();
        let m = Muestras::de_carpeta(&c);
        assert_eq!(m.cuantas(), 2);
        let a: Vec<_> = m.apartados().map(|a| (a.titulo, a.fuente)).collect();
        assert_eq!(a, [(Some("1 · a"), "uno"), (Some("2 · b"), "dos")]);
    }

    #[test]
    fn sin_carpeta_no_hay_muestras_y_no_rompe() {
        let m = Muestras::de_carpeta(Path::new("/no/existe"));
        assert_eq!(m.cuantas(), 0);
    }
}
