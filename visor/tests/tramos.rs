//! El documento llega por tramos: lo de arriba enseguida y el resto detrás, sin cambiar el resultado.
//! Cada prueba lleva su tinta: lo compuesto se recuerda por tinta, y así todas empiezan en frío.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use adaptador::piezas_con_imagenes;
use documento::{Documento, Formato};
use pintor::almacen::Almacen;
use pintor::{Animador, Taller, ancho_de_texto};
use primitivas::{Color, Escena};
use visor::{Avisos, Compositor, Destino, Encargo, Muestras, Resultado, Seleccion};

const K1: &str = include_str!("../../ejemplo/assets/k1.tex");
const RECURSOS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ejemplo/assets");
const ANCHO: f64 = 800.0;
const TOPE: Duration = Duration::from_secs(120);

fn carpetas() -> Vec<PathBuf> {
    vec![PathBuf::from(RECURSOS)]
}

fn tinta(n: u32) -> Color {
    Color {
        r: f64::from(n) * 1e-3,
        g: 0.0,
        b: 0.0,
    }
}

fn encargo(fuente: &str, tinta: Color) -> Encargo {
    Encargo {
        fuente: fuente.into(),
        ancho: ANCHO,
        con_muestras: false,
        tinta,
    }
}

struct Contar(Sender<(Resultado, Instant)>);

impl Avisos for Contar {
    fn publicado(&self, resultado: Resultado) {
        let _ = self.0.send((resultado, Instant::now()));
    }
}

#[derive(Default)]
struct Escenas(Mutex<Vec<Escena>>);

impl Destino for Escenas {
    fn publicar(&self, escena: Escena, _animador: Arc<dyn Animador>, _fuentes: Vec<String>) {
        self.0.lock().unwrap().push(escena);
    }
}

fn abrir<D: Destino>(destino: Arc<D>) -> (Compositor, Receiver<(Resultado, Instant)>) {
    let (tx, rx) = channel();
    let c = Compositor::abrir(destino, Contar(tx), Muestras::default(), carpetas());
    (c, rx)
}

/// Lo que se cuenta hasta el resultado final, incluido.
fn hasta_el_final(rx: &Receiver<(Resultado, Instant)>) -> Vec<(Resultado, Instant)> {
    let mut vistos = Vec::new();
    loop {
        let visto = rx.recv_timeout(TOPE).expect("no ha acabado");
        let completo = visto.0.completo;
        vistos.push(visto);
        if completo {
            return vistos;
        }
    }
}

fn entero(tinta: Color) -> Escena {
    let formato = Formato {
        tam: 14.0,
        color: tinta,
        ..Formato::default()
    };
    let d = Documento::nuevo(formato, piezas_con_imagenes(tinta, carpetas()));
    let hilo = std::thread::Builder::new().stack_size(64 << 20);
    let hecho = hilo.spawn(move || d.componer_a(K1, ancho_de_texto(ANCHO)).unwrap());
    hecho.unwrap().join().unwrap().escena
}

fn es_principio<T: PartialEq>(corto: &[T], largo: &[T]) -> bool {
    corto.len() <= largo.len() && corto == &largo[..corto.len()]
}

/// RAYO: si falla, K1 vuelve a tardar en enseñar nada lo que tarda en componerse entero (0,8 s en
/// una máquina de 32 hilos, medidas de 25 muestras de 21 animaciones incluidas).
#[test]
fn k1_enseña_lo_de_arriba_mucho_antes_de_acabar() {
    let color = tinta(1);
    let escenas = Arc::new(Escenas::default());
    let (c, rx) = abrir(Arc::clone(&escenas));
    let inicio = Instant::now();
    c.encargar(encargo(K1, color));
    let vistos = hasta_el_final(&rx);
    let primero = vistos[0].1 - inicio;
    let total = vistos[vistos.len() - 1].1 - inicio;
    println!("{} tramos, primero {primero:.1?}, entero {total:.1?}", vistos.len());
    assert!(vistos.len() >= 3, "{}", vistos.len());
    assert!(primero * 3 < total, "primero {primero:?}, entero {total:?}");
    let hecho: Vec<bool> = vistos.iter().map(|v| v.0.completo).collect();
    assert_eq!(hecho.iter().filter(|c| **c).count(), 1);
    assert!(vistos.windows(2).all(|p| p[0].0.alto < p[1].0.alto), "el alto crece");
}

/// Un tramo no se pinta distinto de lo que acabará siendo: nada se mueve al llegar el resto.
#[test]
fn cada_tramo_es_un_principio_exacto_de_lo_que_sale_sin_tramos() {
    let color = tinta(2);
    let esperado = entero(color);
    let escenas = Arc::new(Escenas::default());
    let (c, rx) = abrir(Arc::clone(&escenas));
    c.encargar(encargo(K1, color));
    hasta_el_final(&rx);
    drop(c);
    let escenas = escenas.0.lock().unwrap();
    let (fin, tramos) = escenas.split_last().unwrap();
    assert!(fin == &esperado, "el final es lo mismo que sin tramos");
    assert!(tramos.len() >= 2);
    for tramo in tramos {
        assert!(es_principio(&tramo.primitivas, &esperado.primitivas));
        assert!(es_principio(&tramo.anclas, &esperado.anclas));
        assert!(es_principio(&tramo.zonas, &esperado.zonas));
        let glifos = tramo.contornos.iter();
        assert!(glifos.into_iter().all(|(g, c)| esperado.contornos.get(g) == Some(c)));
    }
}

#[test]
fn las_animaciones_llegan_con_su_tramo() {
    let color = tinta(3);
    let escenas = Arc::new(Escenas::default());
    let (c, rx) = abrir(Arc::clone(&escenas));
    c.encargar(encargo(K1, color));
    hasta_el_final(&rx);
    drop(c);
    let escenas = escenas.0.lock().unwrap();
    let zonas: Vec<usize> = escenas.iter().map(|e| e.zonas.len()).collect();
    assert!(zonas.windows(2).all(|p| p[0] <= p[1]), "{zonas:?}");
    assert!(zonas[0] < zonas[zonas.len() - 1], "{zonas:?}");
}

fn con_seleccion() -> (Compositor, Receiver<(Resultado, Instant)>, Arc<Seleccion>) {
    let taller = Arc::new(Taller::abrir(1, Arc::new(Almacen::default())));
    let seleccion = Arc::new(Seleccion::nueva(taller));
    let (c, rx) = abrir(Arc::clone(&seleccion));
    (c, rx, seleccion)
}

#[test]
fn a_medias_se_puede_seleccionar_y_copiar_y_todo_sigue_siendo_todo() {
    let (c, rx, seleccion) = con_seleccion();
    c.encargar(encargo(K1, tinta(4)));
    let (primero, _) = rx.recv_timeout(TOPE).unwrap();
    assert!(!primero.completo);
    assert!(
        primero.alto > 2.0 * pintor::MARGEN,
        "un tramo no sale vacío: {primero:?}"
    );
    seleccion.todo();
    let a_medias = seleccion.copiar();
    assert!(!a_medias.is_empty() && K1.contains(&a_medias), "es trozo de la fuente");
    hasta_el_final(&rx);
    let al_final = seleccion.copiar();
    assert!(al_final.len() > a_medias.len(), "{} {}", al_final.len(), a_medias.len());
}

/// RAYO: sin esto, abrir otro documento espera a que acabe el que ya no se quiere.
#[test]
fn otro_documento_deja_a_medias_el_que_se_estaba_componiendo() {
    let color = tinta(5);
    let escenas = Arc::new(Escenas::default());
    let (c, rx) = abrir(Arc::clone(&escenas));
    c.encargar(encargo(K1, color));
    let (primero, _) = rx.recv_timeout(TOPE).unwrap();
    assert!(!primero.completo);
    c.encargar(encargo("Otro documento", color));
    let vistos = hasta_el_final(&rx);
    assert!(vistos.len() <= 4, "K1 se ha dejado enseguida: {}", vistos.len());
    let fin = &vistos[vistos.len() - 1].0;
    assert!(
        fin.alto < primero.alto,
        "lo que acaba es el otro: {} {}",
        fin.alto,
        primero.alto
    );
}

/// Cambiar de ancho deja lo que hay hasta tener algo mejor: la vista no se queda en blanco.
#[test]
fn reflujar_no_enseña_tramos_mas_cortos_que_lo_que_se_ve() {
    let color = tinta(6);
    let (c, rx) = abrir(Arc::new(Escenas::default()));
    c.encargar(encargo(K1, color));
    let antes = hasta_el_final(&rx).pop().unwrap().0.alto;
    let estrecho = Encargo {
        ancho: ANCHO + 100.0,
        ..encargo(K1, color)
    };
    c.encargar(estrecho);
    let vistos = hasta_el_final(&rx);
    assert!(
        vistos[..vistos.len() - 1].iter().all(|v| v.0.alto > antes),
        "{vistos:?}"
    );
}

/// Un modelo que escribe: la fuente crece y lo ya compuesto no se repite. Más deprisa de lo que se
/// compone, lo último acaba viéndose.
#[test]
fn una_fuente_que_crece_acaba_en_lo_ultimo_escrito() {
    let color = tinta(7);
    let escenas = Arc::new(Escenas::default());
    let (c, rx) = abrir(Arc::clone(&escenas));
    let parrafos: Vec<&str> = K1.split_inclusive("\n\n").collect();
    let mut escrito = String::new();
    for parrafo in &parrafos[..parrafos.len() / 3] {
        escrito.push_str(parrafo);
        c.encargar(encargo(&escrito, color));
    }
    drop(c);
    while rx.recv_timeout(TOPE).is_ok() {}
    let esperado = Documento::nuevo(
        Formato {
            tam: 14.0,
            color,
            ..Formato::default()
        },
        piezas_con_imagenes(color, carpetas()),
    );
    let hecho = esperado.componer_a(&escrito, ancho_de_texto(ANCHO)).unwrap().escena;
    assert!(escenas.0.lock().unwrap().last() == Some(&hecho));
}
