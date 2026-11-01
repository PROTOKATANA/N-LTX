//! El hilo que trabaja: compila lo publicado, compone las animaciones y deja lo hecho en el
//! almacén. Es Rust quien lleva el ritmo: C++ se limita a copiar el último resultado.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use primitivas::{Caja, Color, Escena};

use crate::almacen::Almacen;
use crate::hilos;
use crate::lienzo::{Animador, Lienzo};
use crate::medidas;
use crate::puente_lienzo::ffi::Vista;

/// Cada cuánto se recomponen las animaciones a la vista.
const PASO: Duration = Duration::from_millis(16);

pub(crate) enum Orden {
    Publicar(Escena, Option<Arc<dyn Animador>>),
    Ver(Vista),
    DejarListo(Vista),
    Seleccionar(Vec<Caja>),
    Fondo(Color),
    /// La GPU perdió lo que tenía: hay que volver a mandar las texturas.
    Reenviar,
    Parar,
}

/// El taller de una superficie. Al soltarlo, su hilo termina.
pub struct Taller {
    ordenes: Option<Sender<Orden>>,
    hilo: Option<JoinHandle<()>>,
}

impl Taller {
    pub fn abrir(id: u32, almacen: Arc<Almacen>) -> Taller {
        let (tx, rx) = channel();
        almacen.conectar(id, tx.clone());
        let hilo = hilos::lanzar("nltx-taller", move || trabajar(id, &almacen, &rx));
        Taller {
            ordenes: Some(tx),
            hilo,
        }
    }

    fn pedir(&self, orden: Orden) {
        if let Some(tx) = &self.ordenes {
            // solo falla si el hilo ya terminó, y entonces nadie mira
            let _ = tx.send(orden);
        }
    }

    /// Lo que verá la superficie a partir de su próximo fotograma.
    pub fn publicar(&self, escena: Escena, animador: Option<Arc<dyn Animador>>) {
        self.pedir(Orden::Publicar(escena, animador));
    }

    /// Qué parte del documento se ve.
    pub fn ver(&self, vista: Vista) {
        self.pedir(Orden::Ver(vista));
    }

    /// Deja `vista` compilada, subida y pintada sin enseñarla, para que al verla no quede nada por
    /// hacer: sirve para cargar un documento antes de mostrarlo. Lo animado se pinta quieto y, sin
    /// nada nuevo, no se gasta CPU. La siguiente [`Taller::ver`] la reproduce.
    pub fn dejar_listo(&self, vista: Vista) {
        self.pedir(Orden::DejarListo(vista));
    }

    /// Sobre qué color se pinta. No recompone ni recompila: se ve en el próximo fotograma.
    pub fn fondo(&self, color: Color) {
        self.pedir(Orden::Fondo(color));
    }

    /// Qué partes del documento están seleccionadas, en bp.
    pub fn seleccionar(&self, cajas: Vec<Caja>) {
        self.pedir(Orden::Seleccionar(cajas));
    }
}

impl Drop for Taller {
    fn drop(&mut self) {
        // el almacén guarda otro extremo del canal, así que el hilo se para por esta bandera
        self.pedir(Orden::Parar);
        self.ordenes = None;
        if let Some(h) = self.hilo.take() {
            // un pánico del hilo ya se contuvo dentro: aquí no hay nada que hacer con él
            let _ = h.join();
        }
    }
}

/// El estado del bucle, para poder probar cada paso sin hilos.
struct Trabajo {
    lienzo: Lienzo,
    vista: Option<Vista>,
    animada: bool,
    inicio: Instant,
}

fn trabajar(id: u32, almacen: &Almacen, ordenes: &Receiver<Orden>) {
    let mut t = Trabajo {
        lienzo: Lienzo::default(),
        vista: None,
        animada: false,
        inicio: Instant::now(),
    };
    loop {
        match esperar_orden(ordenes, t.animada) {
            Espera::Orden(o) => {
                if !t.aplicar(o) {
                    return;
                }
            }
            Espera::Toca => {}
            Espera::Cerrado => return,
        }
        // de lo que se ha acumulado mientras se trabajaba, solo importa el resultado
        while let Ok(o) = ordenes.try_recv() {
            if !t.aplicar(o) {
                return;
            }
        }
        t.hacer(id, almacen);
    }
}

enum Espera {
    Orden(Orden),
    Toca,
    Cerrado,
}

fn esperar_orden(ordenes: &Receiver<Orden>, animada: bool) -> Espera {
    if animada {
        return match ordenes.recv_timeout(PASO) {
            Ok(o) => Espera::Orden(o),
            Err(RecvTimeoutError::Timeout) => Espera::Toca,
            Err(RecvTimeoutError::Disconnected) => Espera::Cerrado,
        };
    }
    ordenes.recv().map_or(Espera::Cerrado, Espera::Orden)
}

impl Trabajo {
    /// `false` si hay que parar.
    fn aplicar(&mut self, orden: Orden) -> bool {
        match orden {
            Orden::Publicar(escena, animador) => self.lienzo.publicar(escena, animador),
            Orden::Ver(vista) => self.mirar(vista, false),
            Orden::DejarListo(vista) => self.mirar(vista, true),
            Orden::Seleccionar(cajas) => self.lienzo.seleccionar(cajas),
            Orden::Fondo(color) => self.lienzo.pintar_sobre(color),
            Orden::Reenviar => self.lienzo.reenviar(),
            Orden::Parar => return false,
        }
        true
    }

    fn mirar(&mut self, vista: Vista, oculta: bool) {
        self.vista = Some(vista);
        self.lienzo.ocultar(oculta);
    }

    /// Un pánico al compilar deja el último fotograma bueno en pantalla: tumbar la aplicación por
    /// una escena sería peor.
    fn hacer(&mut self, id: u32, almacen: &Almacen) {
        let Some(vista) = self.vista else {
            return;
        };
        // un fotograma vacío ocupa el relevo hasta que se muestra: el primero con algo esperaría
        // de 50 a 70 ms, y al abrir, `ver` suele llegar antes que lo compuesto
        if !self.lienzo.publicado() {
            return;
        }
        let segundos = self.inicio.elapsed().as_secs_f64();
        let reloj = Instant::now();
        let Ok(entrega) = catch_unwind(AssertUnwindSafe(|| self.lienzo.preparar(vista, segundos))) else {
            return;
        };
        self.animada = entrega.hecho.animada;
        let vertices = entrega.hecho.base.vertices.len() + entrega.hecho.vivos.vertices.len();
        medidas::anotar_taller(vertices, reloj.elapsed().as_secs_f64() * 1000.0);
        almacen.dejar(id, entrega);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitivas::{Caja, Color, Pintura, Primitiva, Punto, Relleno, Segmento, Zona};
    use std::sync::atomic::{AtomicU32, Ordering};

    const TOPE: Duration = Duration::from_secs(10);

    fn vista() -> Vista {
        Vista {
            ancho: 100,
            alto: 100,
            escala: 1.0,
            y: 0.0,
        }
    }

    fn escena(zonas: Vec<Zona>) -> Escena {
        let p = |x, y| Punto { x, y };
        let cuadro = Primitiva::Trazado {
            segmentos: vec![
                Segmento::Mover(p(0.0, 0.0)),
                Segmento::Linea(p(10.0, 0.0)),
                Segmento::Linea(p(10.0, 10.0)),
                Segmento::Cerrar,
            ],
            relleno: Some(Relleno {
                pintura: Pintura::Solida(Color { r: 1.0, g: 0.0, b: 0.0 }),
                opacidad: 1.0,
                par_impar: false,
            }),
            trazo: None,
        };
        Escena {
            primitivas: vec![cuadro],
            zonas,
            ..Escena::default()
        }
    }

    fn zona() -> Zona {
        Zona {
            caja: Caja {
                x: 0.0,
                y: 0.0,
                ancho: 10.0,
                alto: 10.0,
            },
            fuente: "a".into(),
        }
    }

    /// Cuenta fotogramas y entra en pánico en el segundo si se le pide.
    #[derive(Default)]
    struct Falso {
        fotogramas: AtomicU32,
        quietos: AtomicU32,
        romper: bool,
    }

    impl Animador for Falso {
        fn fotograma(&self, _fuente: &str, _segundos: f64) -> Option<Arc<Escena>> {
            let n = self.fotogramas.fetch_add(1, Ordering::Relaxed);
            assert!(!(self.romper && n == 1), "a propósito");
            Some(Arc::new(escena(Vec::new())))
        }

        fn quieto(&self, _fuente: &str) -> Option<Arc<Escena>> {
            self.quietos.fetch_add(1, Ordering::Relaxed);
            Some(Arc::new(escena(Vec::new())))
        }
    }

    /// RAYO: si falla, abrir un documento vuelve a empezar por un fotograma en blanco.
    #[test]
    fn sin_nada_publicado_no_se_entrega_ningun_fotograma() {
        let a = Arc::new(Almacen::default());
        let t = Taller::abrir(1, Arc::clone(&a));
        t.ver(vista());
        assert_eq!(a.esperar(1, 0, Duration::from_millis(30)), 0);
        t.publicar(escena(Vec::new()), None);
        let mut vistas = 0;
        while vistas == 0 {
            vistas = a.esperar(1, 0, TOPE);
        }
    }

    #[test]
    fn sin_vista_no_se_hace_nada_y_con_ella_llega_al_almacen() {
        let a = Arc::new(Almacen::default());
        let t = Taller::abrir(1, Arc::clone(&a));
        t.publicar(escena(Vec::new()), None);
        assert_eq!(a.esperar(1, 0, Duration::from_millis(30)), 0);
        t.ver(vista());
        let mut vistas = 0;
        while vistas == 0 {
            vistas = a.esperar(1, 0, TOPE);
        }
        assert!(!a.tomar(1, false).vertices_base().is_empty());
    }

    /// RAYO: si falla, un panel oculto vuelve a gastar CPU reproduciendo lo que nadie ve.
    #[test]
    fn oculto_se_pinta_una_vez_con_lo_animado_quieto_y_al_verlo_empieza() {
        let a = Arc::new(Almacen::default());
        let falso = Arc::new(Falso::default());
        let t = Taller::abrir(1, Arc::clone(&a));
        t.publicar(escena(vec![zona()]), Some(falso.clone()));
        t.dejar_listo(vista());
        let mut vistas = a.esperar(1, 0, TOPE);
        assert!(vistas > 0, "se entrega sin verse");
        assert!(!a.tomar(1, false).vertices_base().is_empty(), "ya está compilado");
        assert_eq!(a.esperar(1, vistas, Duration::from_millis(60)), vistas, "no sigue");
        assert_eq!(falso.fotogramas.load(Ordering::Relaxed), 0);
        assert!(falso.quietos.load(Ordering::Relaxed) >= 1);
        t.ver(vista());
        while vistas < 4 {
            let antes = vistas;
            vistas = a.esperar(1, vistas, TOPE);
            assert!(vistas > antes, "al verlo se reproduce");
        }
        assert!(falso.fotogramas.load(Ordering::Relaxed) >= 3);
    }

    #[test]
    fn volver_a_ocultar_para_lo_animado() {
        let a = Arc::new(Almacen::default());
        let falso = Arc::new(Falso::default());
        let t = Taller::abrir(1, Arc::clone(&a));
        t.publicar(escena(vec![zona()]), Some(falso.clone()));
        t.ver(vista());
        let mut vistas = 0;
        while vistas < 3 {
            let antes = vistas;
            vistas = a.esperar(1, vistas, TOPE);
            assert!(vistas > antes, "no se reproduce");
        }
        t.dejar_listo(vista());
        std::thread::sleep(Duration::from_millis(40));
        let parado = a.esperar(1, 0, Duration::ZERO);
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(a.esperar(1, 0, Duration::ZERO), parado);
    }

    #[test]
    fn lo_animado_sigue_haciendose_sin_ordenes() {
        let a = Arc::new(Almacen::default());
        let falso = Arc::new(Falso::default());
        let t = Taller::abrir(1, Arc::clone(&a));
        t.publicar(escena(vec![zona()]), Some(falso.clone()));
        t.ver(vista());
        let mut vistas = 0;
        while vistas < 3 {
            vistas = a.esperar(1, vistas, TOPE);
        }
        assert!(falso.fotogramas.load(Ordering::Relaxed) >= 3);
    }

    #[test]
    fn un_panico_deja_lo_ultimo_bueno_y_el_taller_sigue_vivo() {
        let a = Arc::new(Almacen::default());
        let falso = Arc::new(Falso {
            romper: true,
            ..Falso::default()
        });
        let t = Taller::abrir(1, Arc::clone(&a));
        t.publicar(escena(vec![zona()]), Some(falso.clone()));
        t.ver(vista());
        let mut vistas = 0;
        while vistas == 0 {
            vistas = a.esperar(1, 0, TOPE);
        }
        assert!(!a.tomar(1, false).vertices_base().is_empty());
        t.publicar(escena(Vec::new()), None);
        t.ver(Vista { y: 5.0, ..vista() });
        while a.tomar(1, false).vista().y != 5.0 {
            a.esperar(1, a.esperar(1, 0, Duration::ZERO), TOPE);
        }
    }

    #[test]
    fn soltar_el_taller_para_su_hilo() {
        drop(Taller::abrir(1, Arc::new(Almacen::default())));
    }
}
