//! Compone documentos en su propio hilo: la primera vez puede tardar, y quien encarga no debe
//! bloquearse nunca. Si llegan varios encargos seguidos, solo se hace el último.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;

use adaptador::{filmoteca, piezas_con_imagenes};
use documento::{Apartado, Documento, Formato};
use pintor::{Animador, ancho_de_texto, carencias, hilos};
use primitivas::{Color, Composicion, ErrorComposicion, Escena};

use crate::muestras::Muestras;

/// Bp por em del texto y de las fórmulas.
const TAM_FORMULA: f64 = 14.0;

const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

/// Lo que se cuenta a la interfaz tras componer. `error` vacío si no hubo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resultado {
    pub alto: f64,
    pub avisos: Vec<String>,
    pub error: String,
    /// Cuántas muestras lleva el documento.
    pub muestras: u32,
}

/// A quién se le cuenta el resultado.
pub trait Avisos: Send + 'static {
    fn publicado(&self, resultado: Resultado);
}

/// A quién se le entrega la escena compuesta y de qué documentos sale.
pub trait Destino: Send + Sync + 'static {
    fn publicar(&self, escena: Escena, animador: Arc<dyn Animador>, fuentes: Vec<String>);
}

/// Un documento a un ancho de vista, en bp, seguido o no de todas las muestras.
#[derive(Debug, Clone, PartialEq)]
pub struct Encargo {
    pub fuente: String,
    pub ancho: f64,
    pub con_muestras: bool,
}

pub struct Compositor {
    encargos: Option<Sender<Encargo>>,
    hilo: Option<JoinHandle<()>>,
}

impl Compositor {
    /// `carpetas`: de dónde se pueden leer las imágenes de los documentos.
    pub fn abrir<D: Destino, A: Avisos>(
        destino: Arc<D>,
        avisos: A,
        muestras: Muestras,
        carpetas: Vec<PathBuf>,
    ) -> Compositor {
        let (tx, rx) = channel();
        let hilo = hilos::lanzar("nltx-compositor", move || {
            let imprenta = Imprenta::nueva(carpetas);
            let trabajo = Trabajo {
                imprenta: &imprenta,
                destino: destino.as_ref(),
                avisos: &avisos,
                muestras: &muestras,
            };
            trabajar(&rx, &trabajo);
        });
        Compositor {
            encargos: Some(tx),
            hilo,
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

/// A quién sirve el hilo del compositor.
struct Trabajo<'a> {
    imprenta: &'a Imprenta,
    destino: &'a dyn Destino,
    avisos: &'a dyn Avisos,
    muestras: &'a Muestras,
}

fn trabajar(encargos: &Receiver<Encargo>, t: &Trabajo<'_>) {
    while let Ok(mut encargo) = encargos.recv() {
        while let Ok(siguiente) = encargos.try_recv() {
            encargo = siguiente;
        }
        t.avisos
            .publicado(componer(t.imprenta, &encargo, t.muestras, t.destino));
    }
}

/// Cargar las fuentes y componer lo incrustado es lo caro: se hace una vez, con el primer encargo.
struct Imprenta {
    documento: OnceLock<Documento>,
    carpetas: Vec<PathBuf>,
}

impl Imprenta {
    fn nueva(carpetas: Vec<PathBuf>) -> Imprenta {
        Imprenta {
            documento: OnceLock::new(),
            carpetas,
        }
    }

    fn documento(&self) -> &Documento {
        self.documento.get_or_init(|| {
            let formato = Formato {
                tam: TAM_FORMULA,
                color: NEGRO,
                ..Formato::default()
            };
            Documento::nuevo(formato, piezas_con_imagenes(NEGRO, self.carpetas.clone()))
        })
    }
}

fn compuesto(imprenta: &Imprenta, encargo: &Encargo, muestras: &Muestras) -> Result<Composicion, ErrorComposicion> {
    let ancho = ancho_de_texto(encargo.ancho);
    if !encargo.con_muestras {
        return imprenta.documento().componer_a(&encargo.fuente, ancho);
    }
    let propio = Apartado {
        titulo: None,
        fuente: &encargo.fuente,
    };
    let apartados: Vec<Apartado<'_>> = std::iter::once(propio).chain(muestras.apartados()).collect();
    imprenta.documento().componer_serie(&apartados, ancho)
}

/// Los documentos de lo compuesto, en orden.
fn fuentes_de(encargo: &Encargo, muestras: &Muestras) -> Vec<String> {
    let propia = std::iter::once(encargo.fuente.clone());
    if !encargo.con_muestras {
        return propia.collect();
    }
    propia
        .chain(muestras.apartados().map(|a| a.fuente.to_string()))
        .collect()
}

fn componer(imprenta: &Imprenta, encargo: &Encargo, muestras: &Muestras, destino: &dyn Destino) -> Resultado {
    match catch_unwind(AssertUnwindSafe(|| compuesto(imprenta, encargo, muestras))) {
        Ok(Ok(Composicion { escena, mut avisos })) => {
            let alto = escena.caja.alto + 2.0 * pintor::MARGEN;
            avisos.extend(carencias(&escena));
            destino.publicar(escena, Arc::new(AnimadorTikra), fuentes_de(encargo, muestras));
            Resultado {
                alto,
                avisos,
                error: String::new(),
                muestras: if encargo.con_muestras {
                    muestras.cuantas() as u32
                } else {
                    0
                },
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

/// Da los fotogramas de las animaciones: los prepara la filmoteca, por delante de lo que se ve.
struct AnimadorTikra;

impl Animador for AnimadorTikra {
    fn fotograma(&self, fuente: &str, segundos: f64) -> Option<Arc<Escena>> {
        filmoteca().fotograma(fuente, segundos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Falso(Mutex<Vec<f64>>);

    impl Destino for Falso {
        fn publicar(&self, escena: Escena, _animador: Arc<dyn Animador>, _fuentes: Vec<String>) {
            self.0.lock().unwrap().push(escena.caja.ancho);
        }
    }

    fn encargo(fuente: &str, ancho: f64) -> Encargo {
        Encargo {
            fuente: fuente.into(),
            ancho,
            con_muestras: false,
        }
    }

    fn sin_muestras() -> Muestras {
        Muestras::default()
    }

    use std::sync::Mutex;

    use adaptador::FiguraTikra;
    use animacion::Reproductor;

    const K1: &str = include_str!("../../ejemplo/assets/k1.tex");

    /// Una para todas las pruebas: cargar las fuentes es lo caro.
    fn imprenta() -> &'static Imprenta {
        static IMPRENTA: OnceLock<Imprenta> = OnceLock::new();
        IMPRENTA.get_or_init(|| Imprenta::nueva(vec![PathBuf::from(RECURSOS)]))
    }

    const RECURSOS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../ejemplo/assets");

    /// Los hilos de prueba tienen poca pila, y K1 es largo.
    fn con_pila<T: Send + 'static>(trabajo: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(trabajo)
            .unwrap()
            .join()
            .unwrap()
    }

    #[test]
    fn k1_se_compone_sin_avisos() {
        let resultado = con_pila(|| componer(imprenta(), &encargo(K1, 800.0), &sin_muestras(), &Falso::default()));
        assert_eq!(resultado.error, "");
        assert!(resultado.avisos.is_empty(), "{:#?}", resultado.avisos);
    }

    /// Una animación cambia de valores en cada fotograma: uno solo puede romperse sin que se note.
    #[test]
    fn ningun_fotograma_de_k1_da_avisos() {
        let preambulo = &K1[..K1.find("\\section").unwrap()];
        let trozos = K1.split("\\begin{animacion}").skip(1);
        let fuentes: Vec<String> = trozos
            .map(|t| {
                let cuerpo = &t[..t.find("\\end{animacion}").unwrap()];
                format!("{preambulo}\\begin{{animacion}}{cuerpo}\\end{{animacion}}")
            })
            .collect();
        assert!(fuentes.len() >= 20);
        let malos = con_pila(move || {
            let mut malos = Vec::new();
            for fuente in &fuentes {
                let r = Reproductor::nuevo(fuente, FiguraTikra::default()).unwrap();
                let duracion = r.animacion().duracion;
                for n in 0..48 {
                    let c = r.fotograma(duracion * f64::from(n) / 48.0).unwrap();
                    malos.extend(c.avisos.into_iter().map(|a| format!("{n}: {a}")));
                }
            }
            malos
        });
        assert!(malos.is_empty(), "{malos:#?}");
    }

    #[test]
    fn publicar_da_el_alto_con_su_margen_y_pasa_los_avisos() {
        let falso = Falso::default();
        let r = componer(imprenta(), &encargo("Hola", 400.0), &sin_muestras(), &falso);
        assert!(r.alto > 2.0 * pintor::MARGEN, "{r:?}");
        assert_eq!(falso.0.lock().unwrap()[0], ancho_de_texto(400.0));
        let fuente =
            r"\usetikzlibrary{patterns}\begin{tikzpicture}\fill[pattern=dots] (0,0) rectangle (1,1);\end{tikzpicture}";
        let r = componer(imprenta(), &encargo(fuente, 400.0), &sin_muestras(), &falso);
        assert!(r.avisos.iter().any(|a| a.contains("patrones")), "{r:?}");
    }

    #[test]
    fn un_ancho_menor_alarga_el_documento() {
        let fuente = "Un párrafo con bastantes palabras para que se parta en varias líneas al estrecharlo.";
        let alto = |ancho| componer(imprenta(), &encargo(fuente, ancho), &sin_muestras(), &Falso::default()).alto;
        assert!(alto(120.0) > alto(800.0));
    }

    #[test]
    fn un_documento_mezcla_texto_y_formulas() {
        let r = componer(
            imprenta(),
            &encargo(r"Sea $x^2$ y \ce{H2O}.", 400.0),
            &sin_muestras(),
            &Falso::default(),
        );
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
        let c = Compositor::abrir(Arc::new(Falso::default()), avisados, sin_muestras(), Vec::new());
        c.encargar(encargo("uno", 400.0));
        c.encargar(encargo("dos", 400.0));
        drop(c);
        let vistos = vistos.lock().unwrap();
        assert!(!vistos.is_empty() && vistos.len() <= 2, "{vistos:?}");
    }
}
