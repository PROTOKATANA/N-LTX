//! Los hilos de trabajo: componer y compilar figuras muy anidadas recurre mucho.

use std::thread::{self, JoinHandle};

/// El hilo principal tiene 8 MiB y uno nuevo solo 2. Es memoria virtual: lo que no se usa no se gasta.
const PILA: usize = 64 * 1024 * 1024;

/// `None` si el sistema no puede crear el hilo.
pub fn lanzar(nombre: &str, trabajo: impl FnOnce() + Send + 'static) -> Option<JoinHandle<()>> {
    thread::Builder::new()
        .name(nombre.to_string())
        .stack_size(PILA)
        .spawn(trabajo)
        .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Gasta en la pila más o menos un kibibyte por nivel.
    fn hundirse(niveles: usize) -> usize {
        std::hint::black_box([0u8; 1024]);
        if niveles == 0 {
            return 0;
        }
        hundirse(niveles - 1) + 1
    }

    #[test]
    fn hay_pila_para_lo_que_no_cabria_en_un_hilo_corriente() {
        // 16 MiB: el doble del hilo principal y ocho veces uno recién creado
        let hilo = lanzar("prueba", || assert_eq!(hundirse(16 * 1024), 16 * 1024)).unwrap();
        assert!(hilo.join().is_ok());
    }
}
