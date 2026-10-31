//! Los hilos que componen fotogramas. Una figura se compone en un solo hilo, pero fotogramas
//! distintos no comparten nada: el trabajo se reparte por fotogramas.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::thread;

/// Una figura puede anidar mucho: los hilos llevan la misma pila que el resto del motor.
const PILA: usize = 64 << 20;
/// Más hilos apenas ayudan a una animación y dejan sin aire al resto de la máquina.
const MAX_HILOS: usize = 16;

type Encargo = Box<dyn FnOnce() + Send>;

/// Cuántos hilos conviene usar: todos menos el de la interfaz.
pub(crate) fn hilos_disponibles() -> usize {
    let nucleos = thread::available_parallelism().map_or(1, usize::from);
    nucleos.saturating_sub(1).clamp(1, MAX_HILOS)
}

/// Unos hilos que esperan encargos y no gastan nada mientras no los hay.
pub(crate) struct Cuadrilla {
    cola: Sender<Encargo>,
}

fn atender(cola: &Mutex<Receiver<Encargo>>) {
    loop {
        let encargo = cola.lock().unwrap_or_else(PoisonError::into_inner).recv();
        let Ok(encargo) = encargo else { return };
        // un fotograma que falla no se lleva el hilo
        let _ = catch_unwind(AssertUnwindSafe(encargo));
    }
}

impl Cuadrilla {
    /// La de todo el proceso: las animaciones se turnan, y cada una con sus hilos serían demasiados.
    pub(crate) fn global() -> &'static Cuadrilla {
        static CUADRILLA: OnceLock<Cuadrilla> = OnceLock::new();
        CUADRILLA.get_or_init(|| {
            let (cola, entrada) = channel();
            let entrada = Arc::new(Mutex::new(entrada));
            for n in 0..hilos_disponibles() {
                let entrada = Arc::clone(&entrada);
                let hilo = thread::Builder::new()
                    .name(format!("nltx-fotogramas-{n}"))
                    .stack_size(PILA)
                    .spawn(move || atender(&entrada));
                // si el sistema no da más hilos, trabajan los que haya
                drop(hilo);
            }
            Cuadrilla { cola }
        })
    }

    pub(crate) fn encargar(&self, encargo: Encargo) {
        // solo falla si no queda ningún hilo: entonces el fotograma se queda sin hacer
        let _ = self.cola.send(encargo);
    }
}

/// `trabajo(0)`, `trabajo(1)`… en varios hilos, con los resultados en orden.
pub(crate) fn en_paralelo<T: Send>(cuantos: usize, trabajo: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let siguiente = AtomicUsize::new(0);
    let hechos = Mutex::new(Vec::with_capacity(cuantos));
    let peon = || {
        loop {
            let n = siguiente.fetch_add(1, Ordering::Relaxed);
            if n >= cuantos {
                return;
            }
            let hecho = trabajo(n);
            hechos.lock().unwrap_or_else(PoisonError::into_inner).push((n, hecho));
        }
    };
    thread::scope(|ambito| {
        for _ in 1..hilos_disponibles().min(cuantos) {
            // si no se puede lanzar, el resto lo hace quien llama
            let _ = thread::Builder::new().stack_size(PILA).spawn_scoped(ambito, peon);
        }
        peon();
    });
    let mut hechos = hechos.into_inner().unwrap_or_else(PoisonError::into_inner);
    hechos.sort_by_key(|(n, _)| *n);
    hechos.into_iter().map(|(_, hecho)| hecho).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_paralelo_devuelve_cada_resultado_en_su_sitio() {
        let cuadrados = en_paralelo(100, |n| n * n);
        assert_eq!(cuadrados, (0..100).map(|n| n * n).collect::<Vec<_>>());
    }

    #[test]
    fn un_encargo_que_falla_no_se_lleva_el_hilo() {
        let (avisar, aviso) = channel();
        let cuadrilla = Cuadrilla::global();
        for _ in 0..hilos_disponibles() * 2 {
            cuadrilla.encargar(Box::new(|| panic!("a propósito")));
        }
        cuadrilla.encargar(Box::new(move || {
            let _ = avisar.send(());
        }));
        assert!(aviso.recv_timeout(std::time::Duration::from_secs(10)).is_ok());
    }
}
