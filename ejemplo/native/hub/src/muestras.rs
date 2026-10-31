//! Una carpeta de archivos `.tex` para recorrer en la aplicación: sirve para ver cómo se comporta el
//! motor con LaTeX de fuera. Las muestras son datos del usuario y no se versionan.

use std::fs;
use std::path::Path;

/// Una muestra, con su posición.
#[derive(Debug, Clone, PartialEq)]
pub struct Muestra {
    pub indice: u32,
    pub total: u32,
    pub nombre: String,
    pub fuente: String,
}

/// Se leen al arrancar: el bucle de señales no debe tocar el disco.
#[derive(Debug, Default)]
pub struct Muestras(Vec<(String, String)>);

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
        Muestras(leidas.collect())
    }

    /// La muestra `indice`, dando la vuelta al llegar al final. Sin muestras, una vacía.
    pub fn leer(&self, indice: u32) -> Muestra {
        let total = self.0.len() as u32;
        let indice = indice.checked_rem(total).unwrap_or(0);
        let (nombre, fuente) = self.0.get(indice as usize).cloned().unwrap_or_default();
        Muestra {
            indice,
            total,
            nombre,
            fuente,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Una carpeta propia de cada prueba, dentro de la de compilación.
    fn carpeta(nombre: &str) -> std::path::PathBuf {
        let c = std::env::temp_dir().join(format!("nltx-muestras-{}-{nombre}", std::process::id()));
        let _ = fs::remove_dir_all(&c);
        fs::create_dir_all(&c).unwrap();
        c
    }

    #[test]
    fn van_por_orden_de_nombre_y_solo_los_tex() {
        let c = carpeta("orden");
        fs::write(c.join("b.tex"), "dos").unwrap();
        fs::write(c.join("a.tex"), "uno").unwrap();
        fs::write(c.join("nota.txt"), "nada").unwrap();
        let m = Muestras::de_carpeta(&c);
        assert_eq!(m.leer(0).nombre, "a");
        assert_eq!(m.leer(1).fuente, "dos");
        assert_eq!(m.leer(1).total, 2);
    }

    #[test]
    fn al_pasar_del_final_da_la_vuelta() {
        let c = carpeta("vuelta");
        fs::write(c.join("a.tex"), "uno").unwrap();
        fs::write(c.join("b.tex"), "dos").unwrap();
        assert_eq!(Muestras::de_carpeta(&c).leer(3).indice, 1);
    }

    #[test]
    fn sin_carpeta_no_hay_muestras_y_no_rompe() {
        let m = Muestras::de_carpeta(Path::new("/no/existe"));
        assert_eq!(
            m.leer(7),
            Muestra {
                indice: 0,
                total: 0,
                nombre: String::new(),
                fuente: String::new(),
            }
        );
    }
}
