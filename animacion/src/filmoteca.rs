//! Todas las animaciones de la aplicación, cada una preparada una sola vez: medir su caja exige
//! componer su recorrido, y un documento la pide al maquetar y otra vez al reproducir.

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};

use primitivas::{Caja, Compositor, Escena};

use crate::proyector::{PRESUPUESTO, Proyector, reloj};

/// Un modelo que escribe en directo deja muchas animaciones a medias.
const MAX_PELICULAS: usize = 512;
/// Una animación pedida hace menos de esto, en milisegundos, está en pantalla.
const EN_PANTALLA: u64 = 1000;

type Preparada<C> = Result<Arc<Proyector<C>>, String>;
/// Se prepara fuera del candado: una animación pesada no debe parar la reproducción de las demás.
type Pelicula<C> = Arc<OnceLock<Preparada<C>>>;

pub struct Filmoteca<C> {
    fabrica: fn() -> C,
    peliculas: Mutex<HashMap<String, Pelicula<C>>>,
    presupuesto: usize,
}

impl<C: Compositor + Send + Sync + 'static> Filmoteca<C> {
    pub fn nueva(fabrica: fn() -> C) -> Self {
        Filmoteca {
            fabrica,
            peliculas: Mutex::default(),
            presupuesto: PRESUPUESTO,
        }
    }

    fn bloquear(&self) -> MutexGuard<'_, HashMap<String, Pelicula<C>>> {
        self.peliculas.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn preparar(&self, fuente: &str) -> Preparada<C> {
        let intento = catch_unwind(AssertUnwindSafe(|| Proyector::nuevo(fuente, (self.fabrica)())));
        match intento {
            Ok(Ok(proyector)) => Ok(proyector),
            Ok(Err(e)) => Err(e.to_string()),
            Err(_) => Err("la animación ha fallado al componerse".to_string()),
        }
    }

    pub fn proyector(&self, fuente: &str) -> Preparada<C> {
        let pelicula = {
            let mut peliculas = self.bloquear();
            if peliculas.len() >= MAX_PELICULAS && !peliculas.contains_key(fuente) {
                peliculas.clear();
            }
            Arc::clone(peliculas.entry(fuente.to_string()).or_default())
        };
        pelicula.get_or_init(|| self.preparar(fuente)).clone()
    }

    pub fn caja(&self, fuente: &str) -> Result<Caja, String> {
        self.proyector(fuente).map(|p| p.caja())
    }

    /// El fotograma de `fuente` a los `segundos`. Nunca espera a que se componga.
    pub fn fotograma(&self, fuente: &str, segundos: f64) -> Option<Arc<Escena>> {
        let proyector = self.proyector(fuente).ok()?;
        let escena = proyector.ver(segundos);
        self.cumplir_presupuesto();
        escena
    }

    /// Entre todas no pueden pasarse. Las que están en pantalla se reparten el presupuesto a partes
    /// iguales: quitarse fotogramas unas a otras las obligaría a recomponerlos sin parar. Lo que falte
    /// sale de las que hace más que no se ven.
    fn cumplir_presupuesto(&self) {
        let preparadas: Vec<Arc<Proyector<C>>> = self
            .bloquear()
            .values()
            .filter_map(|p| p.get().and_then(|r| r.as_ref().ok()).map(Arc::clone))
            .collect();
        let ahora = reloj();
        let en_pantalla = |p: &Arc<Proyector<C>>| ahora.saturating_sub(p.ultimo_uso()) < EN_PANTALLA;
        let (vistas, mut quietas): (Vec<_>, Vec<_>) = preparadas.into_iter().partition(en_pantalla);
        let cuota = self.presupuesto / vistas.len().max(1);
        vistas.iter().for_each(|p| p.limitar(cuota));
        let mut total: usize = vistas.iter().chain(&quietas).map(|p| p.peso()).sum();
        quietas.sort_by_key(|p| p.ultimo_uso());
        for p in &quietas {
            if total <= self.presupuesto {
                return;
            }
            total -= p.soltar();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use primitivas::{Composicion, ErrorComposicion};

    use super::*;
    use crate::proyector::tests::{Contador, FUENTE};

    fn cuantas(filmoteca: &Filmoteca<Contador>, fuente: &str) -> usize {
        filmoteca.proyector(fuente).map_or(0, |p| p.guardados())
    }

    #[test]
    fn la_caja_y_los_fotogramas_comparten_la_preparacion() {
        static COMPUESTOS: OnceLock<Contador> = OnceLock::new();
        let filmoteca = Filmoteca::nueva(|| COMPUESTOS.get_or_init(Contador::default).clone());
        filmoteca.caja(FUENTE).unwrap();
        let tras_la_caja = COMPUESTOS
            .get_or_init(Contador::default)
            .compuestos
            .load(Ordering::SeqCst);
        filmoteca.caja(FUENTE).unwrap();
        filmoteca.proyector(FUENTE).unwrap();
        let despues = COMPUESTOS
            .get_or_init(Contador::default)
            .compuestos
            .load(Ordering::SeqCst);
        assert_eq!(despues, tras_la_caja, "no se vuelve a medir el recorrido");
        assert!(cuantas(&filmoteca, FUENTE) >= 1);
    }

    #[test]
    fn un_error_se_recuerda_y_no_impide_las_demas() {
        let filmoteca = Filmoteca::nueva(Contador::default);
        assert!(filmoteca.caja("nada").is_err());
        assert!(filmoteca.fotograma("nada", 0.0).is_none());
        assert!(filmoteca.fotograma(FUENTE, 0.0).is_some());
    }

    #[test]
    fn las_que_se_ven_se_reparten_el_presupuesto_sin_quitarse_todo() {
        let cinco_fotogramas = 5 * (size_of::<Escena>() + 62 * size_of::<primitivas::Primitiva>());
        let filmoteca = Filmoteca {
            presupuesto: 2 * cinco_fotogramas,
            ..Filmoteca::nueva(Contador::default)
        };
        let otra = r"\begin{animacion}[t=0:60, duracion=1.1]\draw;\end{animacion}";
        let mut segundos = 0.0;
        for _ in 0..1500 {
            segundos += 1.0 / 600.0;
            filmoteca.fotograma(FUENTE, segundos);
            filmoteca.fotograma(otra, segundos);
            std::thread::sleep(std::time::Duration::from_micros(300));
        }
        assert!(cuantas(&filmoteca, FUENTE) > 1, "conserva sus fotogramas");
        assert!(cuantas(&filmoteca, otra) > 1, "y la otra los suyos");
    }

    #[test]
    fn lo_que_no_se_ve_cede_su_sitio() {
        let vuelta = 60 * (size_of::<Escena>() + 62 * size_of::<primitivas::Primitiva>());
        let filmoteca = Filmoteca {
            presupuesto: vuelta / 2,
            ..Filmoteca::nueva(Contador::default)
        };
        let otra = r"\begin{animacion}[t=0:60, duracion=1.1]\draw;\end{animacion}";
        filmoteca.fotograma(FUENTE, 0.0);
        std::thread::sleep(std::time::Duration::from_millis(50));
        filmoteca.fotograma(FUENTE, 0.0);
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let mut segundos = 0.0;
        for _ in 0..600 {
            segundos += 1.0 / 300.0;
            filmoteca.fotograma(otra, segundos);
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(cuantas(&filmoteca, FUENTE), 1, "la quieta se queda con uno");
        assert!(cuantas(&filmoteca, otra) > 1);
    }

    struct Roto;

    impl Compositor for Roto {
        fn componer(&self, _fuente: &str) -> Result<Composicion, ErrorComposicion> {
            panic!("a propósito")
        }
    }

    #[test]
    fn un_panico_al_preparar_se_vuelve_un_error() {
        let filmoteca = Filmoteca::nueva(|| Roto);
        assert!(
            filmoteca
                .caja(FUENTE)
                .is_err_and(|e| e.contains("falla") || e.contains("fall"))
        );
    }
}
