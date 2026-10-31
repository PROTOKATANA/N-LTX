//! Reproduce una animación con los fotogramas preparados por adelantado.
//!
//! Un fotograma solo depende de su instante: se puede componer antes de que haga falta, en otro
//! hilo, y guardarlo. Quien pinta nunca espera a nadie: toma el más reciente que haya, los siguientes
//! se encargan a la cuadrilla y, cuando la vuelta entera está hecha, reproducir no compone nada.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Instant;

use primitivas::{Caja, Compositor, ErrorComposicion, Escena, Primitiva, Segmento};

use crate::cuadrilla::{Cuadrilla, hilos_disponibles};
use crate::{Animacion, Reproductor};

/// Fotogramas por segundo de animación.
const POR_SEGUNDO: f64 = 60.0;
/// Una animación pesada baja hasta uno de cada tantos fotogramas: más allá ya no se ve fluida.
const MAYOR_SALTO: u32 = 6;
/// Hasta dónde se prepara por delante de lo que se ve.
const ADELANTO: u32 = 30;
/// Lo que puede ocupar lo guardado de una animación. Si no cabe la vuelta entera, se olvida lo que
/// quedó más atrás y se recompone al volver.
pub(crate) const PRESUPUESTO: usize = 256 << 20;

/// Cada cuántos fotogramas se compone uno, si cada uno cuesta `milisegundos`: una animación no debe
/// llevarse más de la mitad de los `hilos`, y es mejor bajar de 60 fps que ahogar la máquina.
fn salto(milisegundos: f64, hilos: usize) -> u32 {
    let nucleos_a_60 = milisegundos / 1000.0 * POR_SEGUNDO;
    let nucleos_propios = (hilos as f64 / 2.0).max(1.0);
    ((nucleos_a_60 / nucleos_propios).ceil() as u32).clamp(1, MAYOR_SALTO)
}

/// Lo que ocupa una escena en memoria, a grandes rasgos: decide qué se olvida, no hace falta más.
fn peso(e: &Escena) -> usize {
    let de_trazados: usize = e
        .primitivas
        .iter()
        .map(|p| match p {
            Primitiva::Trazado { segmentos, .. } => segmentos.len() * size_of::<Segmento>(),
            _ => 0,
        })
        .sum();
    let de_contornos: usize = e.contornos.values().map(|c| c.len() * size_of::<Segmento>()).sum();
    size_of::<Escena>() + e.primitivas.len() * size_of::<Primitiva>() + de_trazados + de_contornos
}

#[derive(Default)]
struct Estado {
    hechos: BTreeMap<u32, Arc<Escena>>,
    pendientes: BTreeSet<u32>,
    /// El fotograma que se está viendo.
    cabeza: u32,
}

pub struct Proyector<C> {
    reproductor: Reproductor<C>,
    /// Fotogramas de una vuelta.
    total: u32,
    estado: Mutex<Estado>,
    peso: AtomicUsize,
    /// Lo que puede ocupar lo guardado.
    limite: AtomicUsize,
    /// Lo que ocupa el último fotograma hecho: dice cuántos caben.
    peso_de_uno: AtomicUsize,
    /// Lo que costó componerlo, en microsegundos.
    coste_de_uno: AtomicU64,
    /// Cuándo se vio por última vez, en milisegundos desde que arrancó el proceso.
    ultimo_uso: AtomicU64,
}

/// Los milisegundos desde que arrancó el proceso.
pub(crate) fn reloj() -> u64 {
    static INICIO: OnceLock<Instant> = OnceLock::new();
    INICIO.get_or_init(Instant::now).elapsed().as_millis() as u64
}

fn fotogramas_de(a: &Animacion) -> u32 {
    (a.duracion * POR_SEGUNDO).ceil().clamp(1.0, f64::from(u32::MAX >> 8)) as u32
}

impl<C: Compositor + Send + Sync + 'static> Proyector<C> {
    /// Prepara la caja y el primer fotograma: a partir de ahí, siempre hay algo que enseñar.
    pub fn nuevo(fuente: &str, compositor: C) -> Result<Arc<Self>, ErrorComposicion> {
        Self::con_limite(fuente, compositor, PRESUPUESTO)
    }

    fn con_limite(fuente: &str, compositor: C, limite: usize) -> Result<Arc<Self>, ErrorComposicion> {
        let reproductor = Reproductor::nuevo(fuente, compositor)?;
        let total = fotogramas_de(reproductor.animacion());
        let proyector = Proyector {
            reproductor,
            total,
            estado: Mutex::default(),
            peso: AtomicUsize::new(0),
            limite: AtomicUsize::new(limite),
            peso_de_uno: AtomicUsize::new(0),
            coste_de_uno: AtomicU64::new(0),
            ultimo_uso: AtomicU64::new(0),
        };
        let primero = proyector.componer(0)?;
        proyector.guardar(0, primero);
        Ok(Arc::new(proyector))
    }

    pub fn caja(&self) -> Caja {
        self.reproductor.caja()
    }

    pub fn animacion(&self) -> &Animacion {
        self.reproductor.animacion()
    }

    /// El fotograma de los `segundos`, o el más cercano que ya esté hecho. Nunca espera.
    pub fn ver(self: &Arc<Self>, segundos: f64) -> Option<Arc<Escena>> {
        let cabeza = self.indice(segundos);
        self.ultimo_uso.store(reloj(), Ordering::Relaxed);
        let mut estado = self.bloquear();
        estado.cabeza = cabeza;
        let visto = self.mas_reciente(&estado, cabeza);
        let encargos = self.encargos(&mut estado, cabeza);
        drop(estado);
        for fotograma in encargos {
            let proyector = Arc::clone(self);
            Cuadrilla::global().encargar(Box::new(move || proyector.hacer(fotograma)));
        }
        visto
    }

    fn indice(&self, segundos: f64) -> u32 {
        let a = self.reproductor.animacion();
        let fase = segundos / a.duracion;
        if a.bucle {
            ((fase.rem_euclid(1.0) * f64::from(self.total)) as u32).min(self.total - 1)
        } else {
            (fase.clamp(0.0, 1.0) * f64::from(self.total - 1)).round() as u32
        }
    }

    /// El valor del parámetro en un fotograma: el mismo en cada vuelta, para poder guardarlo.
    fn valor(&self, fotograma: u32) -> f64 {
        let a = self.reproductor.animacion();
        let tramos = if a.bucle { self.total } else { (self.total - 1).max(1) };
        a.desde + (a.hasta - a.desde) * f64::from(fotograma) / f64::from(tramos)
    }

    fn componer(&self, fotograma: u32) -> Result<Escena, ErrorComposicion> {
        let reloj = Instant::now();
        let c = self.reproductor.fotograma_de(self.valor(fotograma))?;
        self.coste_de_uno
            .store(reloj.elapsed().as_micros() as u64, Ordering::Relaxed);
        Ok(c.escena)
    }

    fn bloquear(&self) -> MutexGuard<'_, Estado> {
        self.estado.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Cuántos fotogramas hay de `desde` a `hasta`, avanzando.
    fn distancia(&self, desde: u32, hasta: u32) -> u32 {
        if hasta >= desde {
            hasta - desde
        } else if self.reproductor.animacion().bucle {
            hasta + self.total - desde
        } else {
            u32::MAX
        }
    }

    /// El último fotograma hecho que no pase de `cabeza`, dando la vuelta si hace falta.
    fn mas_reciente(&self, estado: &Estado, cabeza: u32) -> Option<Arc<Escena>> {
        let atras = estado.hechos.range(..=cabeza).next_back();
        atras
            .or_else(|| estado.hechos.iter().next_back())
            .map(|(_, e)| Arc::clone(e))
    }

    /// Los fotogramas que hacen falta y nadie está haciendo, los más cercanos primero.
    fn encargos(&self, estado: &mut Estado, cabeza: u32) -> Vec<u32> {
        let tope = hilos_disponibles() * 2;
        let bucle = self.reproductor.animacion().bucle;
        let milisegundos = self.coste_de_uno.load(Ordering::Relaxed) as f64 / 1000.0;
        let salto = salto(milisegundos, hilos_disponibles());
        // siempre los mismos: si no, cada vuelta compondría fotogramas distintos
        let primero = cabeza - cabeza % salto;
        let mut encargos = Vec::new();
        for paso in 0..self.adelanto().min(self.total) {
            if estado.pendientes.len() >= tope {
                break;
            }
            let fotograma = primero + paso * salto;
            if !bucle && fotograma >= self.total {
                break;
            }
            let fotograma = fotograma % self.total;
            if !estado.hechos.contains_key(&fotograma) && estado.pendientes.insert(fotograma) {
                encargos.push(fotograma);
            }
        }
        encargos
    }

    /// Hasta dónde se prepara: lo que no cupiera se olvidaría antes de verse, y sería trabajo tirado.
    fn adelanto(&self) -> u32 {
        let cabrian = self.limite.load(Ordering::Relaxed) / self.peso_de_uno.load(Ordering::Relaxed).max(1);
        cabrian.clamp(1, ADELANTO as usize) as u32
    }

    fn hacer(&self, fotograma: u32) {
        let hecho = {
            // si la reproducción ya lo ha pasado, solo estorbaría a los que sí hacen falta
            let cabeza = self.bloquear().cabeza;
            (self.distancia(cabeza, fotograma) < self.adelanto())
                .then(|| self.componer(fotograma).ok())
                .flatten()
        };
        self.bloquear().pendientes.remove(&fotograma);
        if let Some(escena) = hecho {
            self.guardar(fotograma, escena);
        }
    }

    fn guardar(&self, fotograma: u32, escena: Escena) {
        let nuevo = peso(&escena);
        self.peso_de_uno.store(nuevo, Ordering::Relaxed);
        let mut estado = self.bloquear();
        estado.hechos.insert(fotograma, Arc::new(escena));
        self.peso.fetch_add(nuevo, Ordering::Relaxed);
        self.recortar(&mut estado);
    }

    /// Olvida lo que más tardará en volver a verse, que es lo que acaba de pasar, hasta caber.
    fn recortar(&self, estado: &mut Estado) {
        let limite = self.limite.load(Ordering::Relaxed);
        let cabeza = estado.cabeza;
        while self.peso.load(Ordering::Relaxed) > limite && estado.hechos.len() > 1 {
            let lejano = estado.hechos.keys().copied().max_by_key(|f| self.distancia(cabeza, *f));
            let Some(escena) = lejano.and_then(|f| estado.hechos.remove(&f)) else {
                break;
            };
            self.peso.fetch_sub(peso(&escena), Ordering::Relaxed);
        }
    }

    /// Cambia lo que puede ocupar lo guardado.
    pub(crate) fn limitar(&self, limite: usize) {
        self.limite.store(limite, Ordering::Relaxed);
        let mut estado = self.bloquear();
        self.recortar(&mut estado);
    }

    /// Lo que ocupa lo guardado.
    pub(crate) fn peso(&self) -> usize {
        self.peso.load(Ordering::Relaxed)
    }

    pub(crate) fn ultimo_uso(&self) -> u64 {
        self.ultimo_uso.load(Ordering::Relaxed)
    }

    /// Olvida lo guardado, menos un fotograma para seguir teniendo qué enseñar. Devuelve lo liberado.
    pub(crate) fn soltar(&self) -> usize {
        let mut estado = self.bloquear();
        let cabeza = estado.cabeza;
        let queda = self.mas_reciente(&estado, cabeza);
        estado.hechos.clear();
        if let Some(escena) = queda {
            let ocupa = peso(&escena);
            estado.hechos.insert(cabeza, escena);
            return self.peso.swap(ocupa, Ordering::Relaxed).saturating_sub(ocupa);
        }
        self.peso.swap(0, Ordering::Relaxed)
    }

    /// Cuántos fotogramas hay guardados.
    pub fn guardados(&self) -> usize {
        self.bloquear().hechos.len()
    }

    /// Los fotogramas de una vuelta.
    pub fn total(&self) -> u32 {
        self.total
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    use primitivas::Composicion;

    /// Un fotograma con tantas primitivas como su valor, para reconocerlo. Cuenta lo que compone.
    #[derive(Clone, Default)]
    pub(crate) struct Contador {
        pub(crate) compuestos: Arc<AtomicUsize>,
        pub(crate) espera: Duration,
    }

    impl Compositor for Contador {
        fn componer(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
            self.compuestos.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(self.espera);
            let valor: f64 = fuente
                .split(['{', '}'])
                .nth(1)
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| ErrorComposicion(format!("sin valor: {fuente}")))?;
            Ok(Composicion {
                escena: Escena {
                    primitivas: vec![Primitiva::Cerrar; valor.round() as usize],
                    ..Escena::default()
                },
                avisos: Vec::new(),
            })
        }
    }

    /// Una vuelta de 60 fotogramas con el parámetro igual a su número.
    pub(crate) const FUENTE: &str = r"\begin{animacion}[t=0:60, duracion=1]\draw;\end{animacion}";

    /// El valor de un fotograma: sin las dos primitivas que lo colocan en la caja.
    fn valor_de(e: &Escena) -> usize {
        e.primitivas.len() - 2
    }

    fn esperar(mut hecho: impl FnMut() -> bool) {
        let inicio = Instant::now();
        while !hecho() {
            assert!(inicio.elapsed() < Duration::from_secs(20), "no ha llegado");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Da vueltas hasta que estén todos.
    fn llenar(p: &Arc<Proyector<Contador>>) {
        let mut segundos = 0.0;
        esperar(|| {
            segundos += 1.0 / 60.0;
            p.ver(segundos);
            p.guardados() == p.total() as usize
        });
    }

    #[test]
    fn una_animacion_pesada_baja_de_fotogramas_en_vez_de_ahogar_la_maquina() {
        assert_eq!(salto(2.0, 8), 1, "ligera: todos");
        assert_eq!(salto(100.0, 16), 1, "seis núcleos a 60 fps: con ocho para ella, llegan");
        assert_eq!(salto(100.0, 8), 2, "con cuatro, uno de cada dos");
        assert_eq!(salto(60.0, 1), 4, "con uno, uno de cada cuatro");
        assert_eq!(salto(5000.0, 8), MAYOR_SALTO, "tiene un tope");
    }

    #[test]
    fn una_vuelta_se_parte_en_fotogramas_exactos() {
        let p = Proyector::nuevo(FUENTE, Contador::default()).unwrap();
        assert_eq!(p.total(), 60);
        llenar(&p);
        for n in 0..60 {
            let e = p.ver((f64::from(n) + 0.5) / 60.0).unwrap();
            assert_eq!(valor_de(&e), n as usize);
        }
    }

    #[test]
    fn con_la_vuelta_hecha_reproducir_no_compone_nada() {
        let contador = Contador::default();
        let p = Proyector::nuevo(FUENTE, contador.clone()).unwrap();
        llenar(&p);
        // los encargos en marcha acaban solos
        std::thread::sleep(Duration::from_millis(50));
        let antes = contador.compuestos.load(Ordering::SeqCst);
        for n in 0..600 {
            p.ver(f64::from(n) / 60.0);
        }
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(contador.compuestos.load(Ordering::SeqCst), antes);
    }

    #[test]
    fn ver_nunca_espera_a_que_se_componga() {
        let lento = Contador {
            espera: Duration::from_millis(200),
            ..Contador::default()
        };
        let p = Proyector::nuevo(FUENTE, lento).unwrap();
        let reloj = Instant::now();
        let e = p.ver(0.5).unwrap();
        assert!(reloj.elapsed() < Duration::from_millis(100), "{:?}", reloj.elapsed());
        assert_eq!(valor_de(&e), 0, "lo que hay hecho, aunque aún no sea el suyo");
    }

    #[test]
    fn una_animacion_de_una_vez_se_queda_en_el_ultimo_fotograma() {
        let fuente = r"\begin{animacion}[t=0:59, duracion=1, unavez]\draw;\end{animacion}";
        let p = Proyector::nuevo(fuente, Contador::default()).unwrap();
        esperar(|| valor_de(&p.ver(5.0).unwrap()) == 59);
    }

    #[test]
    fn lo_guardado_no_pasa_de_su_limite_y_se_olvida_lo_mas_lejano() {
        let limite = 12 * (size_of::<Escena>() + 62 * size_of::<Primitiva>());
        let p = Proyector::con_limite(FUENTE, Contador::default(), limite).unwrap();
        let mut segundos = 0.0;
        for _ in 0..3000 {
            segundos += 1.0 / 600.0;
            p.ver(segundos);
            std::thread::sleep(Duration::from_micros(200));
        }
        assert!(p.peso() <= limite, "{} > {limite}", p.peso());
        assert!(p.guardados() < 60);
    }

    #[test]
    fn sin_sitio_para_todo_se_conserva_lo_que_esta_por_venir() {
        let limite = 12 * (size_of::<Escena>() + 62 * size_of::<Primitiva>());
        let p = Proyector::con_limite(FUENTE, Contador::default(), limite).unwrap();
        esperar(|| {
            p.ver(0.5);
            p.guardados() >= 8
        });
        // quieto en la mitad de la vuelta, lo guardado debe ser lo que viene detrás de él
        std::thread::sleep(Duration::from_millis(30));
        let estado = p.bloquear();
        let por_venir = estado.hechos.keys().filter(|f| p.distancia(30, **f) < ADELANTO).count();
        assert!(
            por_venir >= estado.hechos.len() - 1,
            "{:?}",
            estado.hechos.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn no_se_prepara_mas_de_lo_que_cabe() {
        let cuatro = 4 * (size_of::<Escena>() + 62 * size_of::<Primitiva>());
        let contador = Contador::default();
        let p = Proyector::con_limite(FUENTE, contador.clone(), cuatro).unwrap();
        for _ in 0..3 {
            p.ver(0.5);
            std::thread::sleep(Duration::from_millis(30));
        }
        let hechos = contador.compuestos.load(Ordering::SeqCst);
        // el recorrido de la caja, el primer fotograma y lo que cabe por delante
        assert!(hechos <= crate::MUESTRAS + 2 + 8, "{hechos}");
    }

    #[test]
    fn limitar_recorta_lo_guardado_al_momento() {
        let p = Proyector::nuevo(FUENTE, Contador::default()).unwrap();
        llenar(&p);
        p.limitar(p.peso() / 2);
        assert!(p.guardados() < 45, "{}", p.guardados());
        assert!(p.guardados() > 1);
    }

    #[test]
    fn soltar_deja_un_fotograma_para_seguir_enseñando_algo() {
        let p = Proyector::nuevo(FUENTE, Contador::default()).unwrap();
        llenar(&p);
        assert!(p.soltar() > 0);
        assert_eq!(p.guardados(), 1);
        assert!(p.ver(0.5).is_some());
    }
}
