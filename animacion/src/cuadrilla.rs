//! Los hilos que componen fotogramas. Una figura se compone en un solo hilo, pero fotogramas
//! distintos no comparten nada: el trabajo se reparte por fotogramas.
//!
//! Son fijos a propósito: `tikra` guarda por hilo su motor ya definido, y un hilo nuevo lo reconstruye.
//! Medir las 525 muestras de K1 en hilos fijos tarda 200 ms frente a 575, con 381 MiB frente a 525.

use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
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

/// `trabajo(0)`, `trabajo(1)`… en la cuadrilla y en quien llama, con los resultados en orden. Quien
/// llama trabaja también: aunque la cuadrilla esté ocupada o no tenga hilos, esto acaba.
pub(crate) fn en_paralelo<T, F>(cuantos: usize, trabajo: F) -> Vec<T>
where
    T: Send + 'static,
    F: Fn(usize) -> T + Send + Sync + 'static,
{
    let (entrega, llegadas) = channel();
    let siguiente = Arc::new(AtomicUsize::new(0));
    let trabajo = Arc::new(trabajo);
    let peon = move || {
        loop {
            let n = siguiente.fetch_add(1, Ordering::Relaxed);
            if n >= cuantos {
                return;
            }
            let hecho = catch_unwind(AssertUnwindSafe(|| trabajo(n)));
            if entrega.send((n, hecho)).is_err() {
                return;
            }
        }
    };
    for _ in 1..hilos_disponibles().min(cuantos) {
        Cuadrilla::global().encargar(Box::new(peon.clone()));
    }
    peon();
    let mut hechos: Vec<_> = llegadas.iter().take(cuantos).collect();
    hechos.sort_by_key(|(n, _)| *n);
    hechos
        .into_iter()
        .map(|(_, hecho)| hecho.unwrap_or_else(|panico| resume_unwind(panico)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_paralelo_devuelve_cada_resultado_en_su_sitio() {
        let cuadrados = en_paralelo(100, |n| n * n);
        assert_eq!(cuadrados, (0..100).map(|n| n * n).collect::<Vec<_>>());
    }

    /// RAYO: un hilo nuevo reconstruye el motor. Si esto falla, K1 vuelve a tardar el triple en medir
    /// sus animaciones y a gastar 150 MiB más.
    #[test]
    fn repartir_no_crea_hilos_nuevos() {
        let hilos = |_| {
            std::thread::sleep(std::time::Duration::from_millis(2));
            thread::current().id()
        };
        let mut vistos: Vec<_> = (0..4).flat_map(|_| en_paralelo(64, hilos)).collect();
        vistos.sort_by_key(|id| format!("{id:?}"));
        vistos.dedup();
        assert!(vistos.len() <= hilos_disponibles() + 1, "{}", vistos.len());
    }

    #[test]
    fn un_panico_llega_a_quien_llama_y_la_cuadrilla_sigue() {
        let hecho = catch_unwind(|| {
            en_paralelo(8, |n| {
                assert!(n != 5, "a propósito");
                n
            })
        });
        assert!(hecho.is_err());
        assert_eq!(en_paralelo(8, |n| n), (0..8).collect::<Vec<_>>());
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
