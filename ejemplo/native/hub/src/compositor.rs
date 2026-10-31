//! Compone documentos en su propio hilo: la primera vez puede tardar, y el bucle de señales no debe
//! bloquearse nunca. Si llegan varios encargos seguidos, solo se hace el último.

use std::collections::HashMap;
use std::panic::catch_unwind;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::thread::{self, JoinHandle};

use adaptador::{FiguraTikra, IncrustadosMotor};
use animacion::Reproductor;
use documento::{Documento, Formato, Memoria};
use pintor::{Animador, Taller, ancho_de_texto, carencias};
use primitivas::{Color, Composicion, Escena};

/// Bp por em del texto y de las fórmulas.
const TAM_FORMULA: f64 = 14.0;

const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

/// Lo que se cuenta a la interfaz tras componer. `error` vacío si no hubo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resultado {
    pub alto: f64,
    pub avisos: Vec<String>,
    pub error: String,
}

/// A quién se le cuenta el resultado.
pub trait Avisos: Send + 'static {
    fn publicado(&self, resultado: Resultado);
}

/// A quién se le entrega la escena compuesta.
pub trait Destino: Send + Sync + 'static {
    fn publicar(&self, escena: Escena, animador: Arc<dyn Animador>);
}

impl Destino for Taller {
    fn publicar(&self, escena: Escena, animador: Arc<dyn Animador>) {
        Taller::publicar(self, escena, Some(animador));
    }
}

/// Un documento a un ancho de vista, en bp.
#[derive(Debug, Clone, PartialEq)]
pub struct Encargo {
    pub fuente: String,
    pub ancho: f64,
}

pub struct Compositor {
    encargos: Option<Sender<Encargo>>,
    hilo: Option<JoinHandle<()>>,
}

impl Compositor {
    pub fn abrir<D: Destino, A: Avisos>(destino: Arc<D>, avisos: A) -> Compositor {
        let (tx, rx) = channel();
        let hilo = thread::spawn(move || trabajar(&rx, destino.as_ref(), &avisos));
        Compositor {
            encargos: Some(tx),
            hilo: Some(hilo),
        }
    }

    pub fn encargar(&self, encargo: Encargo) {
        if let Some(tx) = &self.encargos {
            // solo falla si el hilo ya terminó, y entonces nadie mira
            let _ = tx.send(encargo);
        }
    }
}

impl Drop for Compositor {
    fn drop(&mut self) {
        self.encargos = None;
        if let Some(h) = self.hilo.take() {
            // un pánico de lo compuesto ya se contuvo: aquí no hay nada que hacer con él
            let _ = h.join();
        }
    }
}

fn trabajar(encargos: &Receiver<Encargo>, destino: &dyn Destino, avisos: &dyn Avisos) {
    while let Ok(mut encargo) = encargos.recv() {
        while let Ok(siguiente) = encargos.try_recv() {
            encargo = siguiente;
        }
        avisos.publicado(componer(&encargo, destino));
    }
}

/// Cargar las fuentes y componer lo incrustado es lo caro: se hace una vez para toda la aplicación.
fn documento() -> &'static Documento {
    static DOCUMENTO: OnceLock<Documento> = OnceLock::new();
    DOCUMENTO.get_or_init(|| {
        let formato = Formato {
            tam: TAM_FORMULA,
            color: NEGRO,
            ..Formato::default()
        };
        Documento::nuevo(formato, Box::new(Memoria::nueva(IncrustadosMotor { color: NEGRO })))
    })
}

fn componer(encargo: &Encargo, destino: &dyn Destino) -> Resultado {
    let ancho = ancho_de_texto(encargo.ancho);
    let compuesto = catch_unwind(|| documento().componer_a(&encargo.fuente, ancho));
    match compuesto {
        Ok(Ok(Composicion { escena, mut avisos })) => {
            let alto = escena.caja.alto + 2.0 * pintor::MARGEN;
            avisos.extend(carencias(&escena));
            destino.publicar(escena, Arc::new(AnimadorTikra));
            Resultado {
                alto,
                avisos,
                error: String::new(),
            }
        }
        Ok(Err(e)) => Resultado {
            error: e.to_string(),
            ..Resultado::default()
        },
        Err(_) => Resultado {
            error: "el compositor falló con esta entrada".into(),
            ..Resultado::default()
        },
    }
}

type Reproductores = HashMap<String, Result<Arc<Reproductor<FiguraTikra>>, String>>;

/// Prepara una animación una sola vez: su caja exige componer el recorrido.
fn reproductor(fuente: &str) -> Result<Arc<Reproductor<FiguraTikra>>, String> {
    static LISTOS: OnceLock<Mutex<Reproductores>> = OnceLock::new();
    let mut listos = LISTOS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    listos
        .entry(fuente.to_string())
        .or_insert_with(|| {
            Reproductor::nuevo(fuente, FiguraTikra::default())
                .map(Arc::new)
                .map_err(|e| e.to_string())
        })
        .clone()
}

/// Recompone las animaciones en el hilo del taller.
struct AnimadorTikra;

impl Animador for AnimadorTikra {
    fn fotograma(&self, fuente: &str, segundos: f64) -> Option<Escena> {
        let composicion =
            catch_unwind(|| reproductor(fuente).and_then(|r| r.fotograma(segundos).map_err(|e| e.to_string())));
        composicion.ok()?.ok().map(|c| c.escena)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Falso(Mutex<Vec<f64>>);

    impl Destino for Falso {
        fn publicar(&self, escena: Escena, _animador: Arc<dyn Animador>) {
            self.0.lock().unwrap().push(escena.caja.ancho);
        }
    }

    fn encargo(fuente: &str, ancho: f64) -> Encargo {
        Encargo {
            fuente: fuente.into(),
            ancho,
        }
    }

    #[test]
    fn publicar_da_el_alto_con_su_margen_y_pasa_los_avisos() {
        let falso = Falso::default();
        let r = componer(&encargo("Hola", 400.0), &falso);
        assert!(r.alto > 2.0 * pintor::MARGEN, "{r:?}");
        assert_eq!(falso.0.lock().unwrap()[0], ancho_de_texto(400.0));
        let fuente =
            r"\usetikzlibrary{patterns}\begin{tikzpicture}\fill[pattern=dots] (0,0) rectangle (1,1);\end{tikzpicture}";
        let r = componer(&encargo(fuente, 400.0), &falso);
        assert!(r.avisos.iter().any(|a| a.contains("patrones")), "{r:?}");
    }

    #[test]
    fn un_ancho_menor_alarga_el_documento() {
        let fuente = "Un párrafo con bastantes palabras para que se parta en varias líneas al estrecharlo.";
        let alto = |ancho| componer(&encargo(fuente, ancho), &Falso::default()).alto;
        assert!(alto(120.0) > alto(800.0));
    }

    #[test]
    fn un_documento_mezcla_texto_y_formulas() {
        let r = componer(&encargo(r"Sea $x^2$ y \ce{H2O}.", 400.0), &Falso::default());
        assert!(r.avisos.is_empty() && r.error.is_empty(), "{r:?}");
    }

    #[test]
    fn el_animador_no_deja_pasar_un_error() {
        let a = AnimadorTikra;
        assert!(a.fotograma("nada", 0.0).is_none());
        let fuente =
            r"\begin{animacion}[t=0:90]\begin{tikzpicture}\draw (0,0) -- (\t:1);\end{tikzpicture}\end{animacion}";
        assert!(a.fotograma(fuente, 1.0).is_some());
    }

    #[derive(Default)]
    struct Avisados(Arc<Mutex<Vec<Resultado>>>);

    impl Avisos for Avisados {
        fn publicado(&self, resultado: Resultado) {
            self.0.lock().unwrap().push(resultado);
        }
    }

    #[test]
    fn el_hilo_avisa_y_al_soltarlo_se_ha_hecho_lo_ultimo() {
        let avisados = Avisados::default();
        let vistos = Arc::clone(&avisados.0);
        let c = Compositor::abrir(Arc::new(Falso::default()), avisados);
        c.encargar(encargo("uno", 400.0));
        c.encargar(encargo("dos", 400.0));
        drop(c);
        let vistos = vistos.lock().unwrap();
        assert!(!vistos.is_empty() && vistos.len() <= 2, "{vistos:?}");
    }
}
