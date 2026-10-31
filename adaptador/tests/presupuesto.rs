#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Lo que cuesta un fotograma, contado en reservas de memoria: a diferencia del tiempo, es igual en
//! cada ejecución y en cada máquina, y por eso puede fallar una prueba. Si una de estas falla, se
//! ha perdido una ganancia medida (RAYO, paso 8): `ejemplo/native/hub/tests/banco_animacion.rs`
//! dice cuánto vale en milisegundos.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use adaptador::FiguraTikra;
use primitivas::Compositor;

struct Contador;

thread_local! {
    /// Por hilo: las pruebas corren a la vez. Con inicio constante, usarlo no reserva memoria.
    static RESERVAS: Cell<u64> = const { Cell::new(0) };
}

fn contar() {
    // al terminar un hilo ya no existe, y esas reservas no son de nadie que mire
    let _ = RESERVAS.try_with(|r| r.set(r.get() + 1));
}

// SAFETY: solo cuenta y delega en el asignador del sistema, con los mismos argumentos.
unsafe impl GlobalAlloc for Contador {
    unsafe fn alloc(&self, forma: Layout) -> *mut u8 {
        contar();
        // SAFETY: el contrato de `alloc` es el del que llama.
        unsafe { System.alloc(forma) }
    }

    unsafe fn dealloc(&self, puntero: *mut u8, forma: Layout) {
        // SAFETY: el contrato de `dealloc` es el del que llama.
        unsafe { System.dealloc(puntero, forma) }
    }

    unsafe fn realloc(&self, puntero: *mut u8, forma: Layout, nuevo: usize) -> *mut u8 {
        contar();
        // SAFETY: el contrato de `realloc` es el del que llama.
        unsafe { System.realloc(puntero, forma, nuevo) }
    }
}

#[global_allocator]
static ASIGNADOR: Contador = Contador;

const ELEMENTOS: u64 = 2000;

fn por_elemento(cuerpo: &str) -> u64 {
    let figura = FiguraTikra::default();
    let fuente =
        format!("\\begin{{tikzpicture}}\\foreach \\i in {{1,...,{ELEMENTOS}}} {{ {cuerpo} }}\\end{{tikzpicture}}");
    let hilo = std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        // la primera vez construye el motor de este hilo, que no es parte del fotograma
        figura.componer(&fuente).unwrap();
        let antes = RESERVAS.get();
        let c = figura.componer(&fuente).unwrap();
        let reservas = RESERVAS.get() - antes;
        assert_eq!(c.avisos, Vec::<String>::new());
        reservas / ELEMENTOS
    });
    hilo.unwrap().join().unwrap()
}

/// Con su holgura, que deja pasar cambios pequeños: lo que vigila son las copias que vuelven.
fn vigilar(nombre: &str, reservas: u64, tope: u64) {
    println!("PRESUPUESTO {nombre}: {reservas} reservas por elemento, tope {tope}");
    assert!(
        reservas <= tope,
        "{nombre}: {reservas} reservas por elemento, el tope era {tope}"
    );
}

#[test]
fn un_elemento_con_macros_no_vuelve_a_copiar() {
    let cuerpo = r"\pgfmathsetmacro{\x}{4*sin(\i*7)} \fill[blue!60] (\x,0) circle (0.04);";
    vigilar("macros", por_elemento(cuerpo), TOPE_MACROS);
}

#[test]
fn un_trazo_no_vuelve_a_copiar() {
    vigilar(
        "trazo",
        por_elemento(r"\fill[red] (\i*0.01,0) circle (0.04);"),
        TOPE_TRAZO,
    );
}

const TOPE_MACROS: u64 = 220;
const TOPE_TRAZO: u64 = 170;
