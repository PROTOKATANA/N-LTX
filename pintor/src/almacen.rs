//! Donde Rust deja lo hecho y C++ lo recoge cuando va a dibujar.
//!
//! El candado solo cubre cambiar o copiar valores ya hechos, nunca cálculos: los cálculos son del
//! taller, en su hilo. Por eso C++ nunca espera a que se compile nada.

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Duration;

use primitivas::MapaDeBits;

use crate::lienzo::{Entrega, Hecho};
use crate::lista::Lista;
use crate::puente_lienzo::ffi::{Comando, Tinta, Vertice, Vista};
use crate::taller::Orden;
use crate::textura::Sucio;

/// Parches por recoger, como mucho. Con más, sale a cuenta volver a mandar las texturas enteras.
const MAX_PARCHES: usize = 64;

#[derive(Default)]
struct Hoja {
    hecho: Option<Arc<Hecho>>,
    atlas: Vec<Sucio>,
    rampas: Vec<Sucio>,
    entregas: u64,
    taller: Option<Sender<Orden>>,
}

impl Hoja {
    fn pedir_todo(&mut self) {
        self.atlas.clear();
        self.rampas.clear();
        if let Some(t) = &self.taller {
            // si el taller ya cerró, no hay nada que volver a mandar
            let _ = t.send(Orden::Reenviar);
        }
    }
}

/// Un almacén por aplicación, con una hoja por cada superficie.
#[derive(Default)]
pub struct Almacen {
    hojas: Mutex<HashMap<u32, Hoja>>,
    cambio: Condvar,
}

impl Almacen {
    fn hojas(&self) -> MutexGuard<'_, HashMap<u32, Hoja>> {
        self.hojas.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn conectar(&self, id: u32, taller: Sender<Orden>) {
        self.hojas().entry(id).or_default().taller = Some(taller);
    }

    /// Lo hecho sustituye a lo anterior, y lo que C++ aún no recogió de las texturas se acumula.
    pub fn dejar(&self, id: u32, entrega: Entrega) {
        let Entrega { hecho, atlas, rampas } = entrega;
        let mut hojas = self.hojas();
        let hoja = hojas.entry(id).or_default();
        hoja.hecho = Some(hecho);
        hoja.entregas += 1;
        if !atlas.bytes.is_empty() {
            hoja.atlas.push(atlas);
        }
        if !rampas.bytes.is_empty() {
            hoja.rampas.push(rampas);
        }
        if hoja.atlas.len() > MAX_PARCHES || hoja.rampas.len() > MAX_PARCHES {
            hoja.pedir_todo();
        }
        drop(hojas);
        self.cambio.notify_all();
    }

    /// Lo último hecho, con los parches pendientes. `nuevos`: la GPU aún no tiene nada subido.
    pub fn tomar(&self, id: u32, nuevos: bool) -> Marco {
        let mut hojas = self.hojas();
        let hoja = hojas.entry(id).or_default();
        if nuevos {
            hoja.pedir_todo();
        }
        Marco {
            hecho: hoja.hecho.clone(),
            atlas: std::mem::take(&mut hoja.atlas),
            rampas: std::mem::take(&mut hoja.rampas),
        }
    }

    /// Espera a que haya más entregas que `vistas`, como mucho `tope`. Devuelve las que hay.
    pub fn esperar(&self, id: u32, vistas: u64, tope: Duration) -> u64 {
        let entregas = |h: &HashMap<u32, Hoja>| h.get(&id).map_or(0, |h| h.entregas);
        let hojas = self.hojas();
        if entregas(&hojas) != vistas {
            return entregas(&hojas);
        }
        let (hojas, _) = self
            .cambio
            .wait_timeout(hojas, tope)
            .unwrap_or_else(PoisonError::into_inner);
        entregas(&hojas)
    }

    /// Hace volver a quien espera, sin esperar al tope.
    pub fn despertar(&self) {
        self.cambio.notify_all();
    }
}

/// El almacén de la aplicación: la frontera con C++ son funciones libres.
pub fn almacen() -> Arc<Almacen> {
    static ALMACEN: OnceLock<Arc<Almacen>> = OnceLock::new();
    Arc::clone(ALMACEN.get_or_init(Default::default))
}

/// Lo que C++ se lleva para dibujar. Es suyo: lo lee sin bloquear a nadie.
pub struct Marco {
    hecho: Option<Arc<Hecho>>,
    atlas: Vec<Sucio>,
    rampas: Vec<Sucio>,
}

fn lista_vacia() -> &'static Lista {
    static VACIA: OnceLock<Lista> = OnceLock::new();
    VACIA.get_or_init(Lista::default)
}

impl Marco {
    fn base(&self) -> &Lista {
        self.hecho.as_ref().map_or(lista_vacia(), |h| &h.base)
    }

    fn vivos(&self) -> &Lista {
        self.hecho.as_ref().map_or(lista_vacia(), |h| &h.vivos)
    }

    /// Sin nada hecho, una vista de tamaño cero.
    pub fn vista(&self) -> Vista {
        self.hecho.as_ref().map_or(
            Vista {
                ancho: 0,
                alto: 0,
                escala: 1.0,
                y: 0.0,
            },
            |h| h.vista,
        )
    }

    pub fn generacion_base(&self) -> u64 {
        self.hecho.as_ref().map_or(0, |h| h.generacion_base)
    }

    pub fn generacion_vivos(&self) -> u64 {
        self.hecho.as_ref().map_or(0, |h| h.generacion_vivos)
    }

    pub fn vertices_base(&self) -> &[Vertice] {
        &self.base().vertices
    }

    pub fn indices_base(&self) -> &[u32] {
        &self.base().indices
    }

    pub fn comandos_base(&self) -> &[Comando] {
        &self.base().comandos
    }

    pub fn vertices_vivos(&self) -> &[Vertice] {
        &self.vivos().vertices
    }

    pub fn indices_vivos(&self) -> &[u32] {
        &self.vivos().indices
    }

    pub fn comandos_vivos(&self) -> &[Comando] {
        &self.vivos().comandos
    }

    pub fn tintas(&self) -> &[Tinta] {
        self.hecho.as_ref().map_or(&[], |h| &h.tintas)
    }

    pub fn parches_atlas(&self) -> usize {
        self.atlas.len()
    }

    pub fn parche_atlas(&self, i: usize) -> &[u8] {
        self.atlas.get(i).map_or(&[], |s| &s.bytes)
    }

    pub fn fila_atlas(&self, i: usize) -> u32 {
        self.atlas.get(i).map_or(0, |s| s.y)
    }

    pub fn parches_rampas(&self) -> usize {
        self.rampas.len()
    }

    pub fn parche_rampas(&self, i: usize) -> &[u8] {
        self.rampas.get(i).map_or(&[], |s| &s.bytes)
    }

    pub fn fila_rampas(&self, i: usize) -> u32 {
        self.rampas.get(i).map_or(0, |s| s.y)
    }

    fn imagen(&self, i: usize) -> Option<&MapaDeBits> {
        let (base, vivos) = (self.base(), self.vivos());
        let n = base.imagenes.len();
        let mapa = if i < n {
            base.imagenes.get(i)
        } else {
            vivos.imagenes.get(i - n)
        };
        mapa.map(AsRef::as_ref)
    }

    pub fn imagenes(&self) -> usize {
        self.base().imagenes.len() + self.vivos().imagenes.len()
    }

    pub fn id_imagen(&self, i: usize) -> u64 {
        self.imagen(i).map_or(0, |m| m.id)
    }

    pub fn ancho_imagen(&self, i: usize) -> u32 {
        self.imagen(i).map_or(0, |m| m.ancho)
    }

    pub fn alto_imagen(&self, i: usize) -> u32 {
        self.imagen(i).map_or(0, |m| m.alto)
    }

    pub fn rgba_imagen(&self, i: usize) -> &[u8] {
        self.imagen(i).map_or(&[], |m| &m.rgba)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc::channel;
    use std::thread;

    fn hecho(y: f64) -> Arc<Hecho> {
        Arc::new(Hecho {
            vista: Vista {
                ancho: 10,
                alto: 10,
                escala: 1.0,
                y,
            },
            generacion_base: 1,
            base: Arc::new(Lista::base()),
            generacion_vivos: 0,
            vivos: Lista::default(),
            tintas: Vec::new(),
            animada: false,
        })
    }

    fn entrega(y: f64, parche: &[u8]) -> Entrega {
        let sucio = Sucio {
            y: 7,
            bytes: parche.to_vec(),
        };
        Entrega {
            hecho: hecho(y),
            atlas: sucio.clone(),
            rampas: sucio,
        }
    }

    #[test]
    fn sin_nada_hecho_no_hay_nada_que_pintar_y_no_rompe() {
        let m = Almacen::default().tomar(1, false);
        assert_eq!(m.vista().ancho, 0);
        assert!(m.vertices_base().is_empty() && m.tintas().is_empty());
        assert!(m.parche_atlas(3).is_empty());
    }

    #[test]
    fn lo_ultimo_gana_pero_los_parches_se_acumulan_hasta_que_se_recogen() {
        let a = Almacen::default();
        a.dejar(1, entrega(1.0, &[1]));
        a.dejar(1, entrega(2.0, &[2]));
        let m = a.tomar(1, false);
        assert_eq!(m.vista().y, 2.0);
        assert_eq!((m.parches_atlas(), m.parches_rampas()), (2, 2));
        assert_eq!((m.parche_atlas(0), m.fila_atlas(1)), (&[1u8][..], 7));
        let otra_vez = a.tomar(1, false);
        assert_eq!(otra_vez.vista().y, 2.0, "lo hecho sigue ahí");
        assert_eq!(otra_vez.parches_atlas(), 0, "los parches solo se suben una vez");
    }

    #[test]
    fn las_superficies_no_se_mezclan() {
        let a = Almacen::default();
        a.dejar(1, entrega(1.0, &[]));
        assert_eq!(a.tomar(2, false).vista().ancho, 0);
    }

    #[test]
    fn una_gpu_nueva_pide_todo_al_taller() {
        let a = Almacen::default();
        let (tx, rx) = channel();
        a.conectar(1, tx);
        a.dejar(1, entrega(1.0, &[1]));
        let m = a.tomar(1, true);
        assert_eq!(m.parches_atlas(), 0, "lo viejo ya no sirve");
        assert!(matches!(rx.try_recv(), Ok(Orden::Reenviar)));
    }

    #[test]
    fn demasiados_parches_sin_recoger_se_cambian_por_las_texturas_enteras() {
        let a = Almacen::default();
        let (tx, rx) = channel();
        a.conectar(1, tx);
        for _ in 0..=MAX_PARCHES {
            a.dejar(1, entrega(1.0, &[1]));
        }
        assert_eq!(a.tomar(1, false).parches_atlas(), 0);
        assert!(matches!(rx.try_recv(), Ok(Orden::Reenviar)));
    }

    #[test]
    fn esperar_vuelve_si_ya_hay_algo_nuevo_y_al_llegar_algo() {
        let a = Arc::new(Almacen::default());
        a.dejar(1, entrega(1.0, &[]));
        assert_eq!(a.esperar(1, 0, Duration::from_secs(5)), 1, "ya había");
        let otro = Arc::clone(&a);
        let hilo = thread::spawn(move || otro.esperar(1, 1, Duration::from_secs(5)));
        // hasta que el hilo esté esperando, una entrega también vale: `esperar` la ve al entrar
        a.dejar(1, entrega(2.0, &[]));
        assert_eq!(hilo.join().ok(), Some(2));
    }

    #[test]
    fn esperar_no_pasa_del_tope() {
        let a = Almacen::default();
        assert_eq!(a.esperar(1, 0, Duration::from_millis(1)), 0);
    }
}
