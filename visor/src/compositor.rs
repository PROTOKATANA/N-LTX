//! Compone documentos en su propio hilo: la primera vez puede tardar, y quien encarga no debe
//! bloquearse nunca. Lo de arriba se enseña en cuanto está y el resto sigue detrás, por tramos. Si
//! llegan varios encargos seguidos, solo se hace el último.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;

use adaptador::{filmoteca, piezas_con_imagenes};
use documento::{Apartado, Documento, Formato, Seguimiento};
use pintor::{Animador, ancho_de_texto, carencias, hilos};
use primitivas::{Color, Composicion, ErrorComposicion, Escena};

use crate::muestras::Muestras;
use crate::tramos::{Mostrado, Tramos, alto_con_margen};

/// Bp por em del texto y de las fórmulas.
const TAM_FORMULA: f64 = 14.0;

/// Lo que se cuenta a la interfaz al publicar. `error` vacío si no hubo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Resultado {
    pub alto: f64,
    pub avisos: Vec<String>,
    pub error: String,
    /// Cuántas muestras lleva el documento.
    pub muestras: u32,
    /// `false` en un tramo: el documento sigue componiéndose, y `alto` y `avisos` aún crecerán.
    pub completo: bool,
}

/// A quién se le cuenta lo publicado: una vez por tramo y otra al acabar. Lo que se abandona porque
/// llega otro encargo no acaba.
pub trait Avisos: Send + 'static {
    fn publicado(&self, resultado: Resultado);
}

/// A quién se le entrega la escena compuesta y de qué documentos sale. Un documento puede llegar
/// varias veces: cada tramo es un principio más largo del mismo resultado.
pub trait Destino: Send + Sync + 'static {
    fn publicar(&self, escena: Escena, animador: Arc<dyn Animador>, fuentes: Vec<String>);
}

/// Un documento a un ancho de vista, en bp, seguido o no de todas las muestras.
#[derive(Debug, Clone, PartialEq)]
pub struct Encargo {
    pub fuente: String,
    pub ancho: f64,
    pub con_muestras: bool,
    /// El color de lo que el documento no colorea. Cambiarla recompone; lo que pide un color, aunque
    /// sea `black`, lo conserva. El fondo no se compone: lo lleva `Taller::fondo`.
    pub tinta: Color,
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

impl Trabajo<'_> {
    /// Da lo compuesto a quien lo pinta. Devuelve lo que hay que contar.
    fn entregar(&self, encargo: &Encargo, c: Composicion, completo: bool) -> Resultado {
        let Composicion { escena, mut avisos } = c;
        let alto = alto_con_margen(escena.caja.alto);
        avisos.extend(carencias(&escena));
        let animador = Arc::new(AnimadorTikra { tinta: encargo.tinta });
        self.destino
            .publicar(escena, animador, fuentes_de(encargo, self.muestras));
        Resultado {
            alto,
            avisos,
            error: String::new(),
            muestras: if encargo.con_muestras {
                self.muestras.cuantas() as u32
            } else {
                0
            },
            completo,
        }
    }
}

fn trabajar(encargos: &Receiver<Encargo>, t: &Trabajo<'_>) {
    let mut mostrado = Mostrado::default();
    let mut pendiente = None;
    while let Some(mut encargo) = pendiente.take().or_else(|| encargos.recv().ok()) {
        while let Ok(siguiente) = encargos.try_recv() {
            encargo = siguiente;
        }
        let mostrar = |parcial| {
            let resultado = t.entregar(&encargo, parcial, false);
            let alto = resultado.alto;
            t.avisos.publicado(resultado);
            alto
        };
        let mut tramos = Tramos::nuevos(&encargo, encargos, &mostrado, &mostrar);
        let resultado = componer(t, &encargo, &mut tramos);
        let (alcanzado, siguiente) = tramos.acabar();
        pendiente = siguiente;
        let alto = resultado.as_ref().filter(|r| r.error.is_empty()).map(|r| r.alto);
        if let Some(alto) = alto.or(alcanzado) {
            mostrado = Mostrado {
                fuente: encargo.fuente.clone(),
                alto,
            };
        }
        if let Some(resultado) = resultado {
            t.avisos.publicado(resultado);
        }
    }
}

/// Lo compuesto lleva la tinta dentro: componer lo incrustado se hace una vez por tinta, con el primer
/// encargo, y al cambiarla se empieza de nuevo.
struct Imprenta {
    documento: Mutex<Option<(Color, Arc<Documento>)>>,
    carpetas: Vec<PathBuf>,
}

impl Imprenta {
    fn nueva(carpetas: Vec<PathBuf>) -> Imprenta {
        Imprenta {
            documento: Mutex::new(None),
            carpetas,
        }
    }

    fn documento(&self, tinta: Color) -> Arc<Documento> {
        let mut guardado = self.documento.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some((_, documento)) = guardado.as_ref().filter(|(t, _)| *t == tinta) {
            return Arc::clone(documento);
        }
        let formato = Formato {
            tam: TAM_FORMULA,
            color: tinta,
            ..Formato::default()
        };
        let piezas = piezas_con_imagenes(tinta, self.carpetas.clone());
        let documento = Arc::new(Documento::nuevo(formato, piezas));
        *guardado = Some((tinta, Arc::clone(&documento)));
        documento
    }
}

fn compuesto(
    imprenta: &Imprenta,
    encargo: &Encargo,
    muestras: &Muestras,
    seguimiento: &mut dyn Seguimiento,
) -> Result<Composicion, ErrorComposicion> {
    let ancho = ancho_de_texto(encargo.ancho);
    let documento = imprenta.documento(encargo.tinta);
    if !encargo.con_muestras {
        return documento.seguir_a(&encargo.fuente, ancho, seguimiento);
    }
    let propio = Apartado {
        titulo: None,
        fuente: &encargo.fuente,
    };
    let apartados: Vec<Apartado<'_>> = std::iter::once(propio).chain(muestras.apartados()).collect();
    documento.seguir_serie(&apartados, ancho, seguimiento)
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

/// Lo que hay que contar al acabar; `None` si se ha abandonado.
fn componer(t: &Trabajo<'_>, encargo: &Encargo, seguimiento: &mut dyn Seguimiento) -> Option<Resultado> {
    let hecho = catch_unwind(AssertUnwindSafe(|| {
        compuesto(t.imprenta, encargo, t.muestras, seguimiento)
    }));
    if seguimiento.parar() {
        return None;
    }
    let error = |error: String| Resultado {
        error,
        completo: true,
        ..Resultado::default()
    };
    Some(match hecho {
        Ok(Ok(composicion)) => t.entregar(encargo, composicion, true),
        Ok(Err(e)) => error(e.to_string()),
        Err(_) => error("el compositor falló con esta entrada".into()),
    })
}

/// Da los fotogramas de las animaciones: los prepara la filmoteca, por delante de lo que se ve.
struct AnimadorTikra {
    tinta: Color,
}

impl Animador for AnimadorTikra {
    fn fotograma(&self, fuente: &str, segundos: f64) -> Option<Arc<Escena>> {
        filmoteca().fotograma(fuente, self.tinta, segundos)
    }

    fn quieto(&self, fuente: &str) -> Option<Arc<Escena>> {
        filmoteca().quieto(fuente, self.tinta)
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
            tinta: Color::NEGRO,
        }
    }

    fn sin_muestras() -> Muestras {
        Muestras::default()
    }

    struct Callado;

    impl Avisos for Callado {
        fn publicado(&self, _resultado: Resultado) {}
    }

    /// Compone sin nadie mirando, de una vez.
    fn entero(imprenta: &Imprenta, encargo: &Encargo, muestras: &Muestras, destino: &dyn Destino) -> Resultado {
        let t = Trabajo {
            imprenta,
            destino,
            avisos: &Callado,
            muestras,
        };
        componer(&t, encargo, &mut documento::SinSeguimiento).unwrap()
    }

    use std::sync::OnceLock;

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
        let resultado = con_pila(|| entero(imprenta(), &encargo(K1, 800.0), &sin_muestras(), &Falso::default()));
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
        let r = entero(imprenta(), &encargo("Hola", 400.0), &sin_muestras(), &falso);
        assert!(r.alto > 2.0 * pintor::MARGEN, "{r:?}");
        assert_eq!(falso.0.lock().unwrap()[0], ancho_de_texto(400.0));
        let fuente =
            r"\usetikzlibrary{patterns}\begin{tikzpicture}\fill[pattern=dots] (0,0) rectangle (1,1);\end{tikzpicture}";
        let r = entero(imprenta(), &encargo(fuente, 400.0), &sin_muestras(), &falso);
        assert!(r.avisos.iter().any(|a| a.contains("patrones")), "{r:?}");
    }

    #[test]
    fn un_ancho_menor_alarga_el_documento() {
        let fuente = "Un párrafo con bastantes palabras para que se parta en varias líneas al estrecharlo.";
        let alto = |ancho| entero(imprenta(), &encargo(fuente, ancho), &sin_muestras(), &Falso::default()).alto;
        assert!(alto(120.0) > alto(800.0));
    }

    #[test]
    fn un_documento_mezcla_texto_y_formulas() {
        let r = entero(
            imprenta(),
            &encargo(r"Sea $x^2$ y \ce{H2O}.", 400.0),
            &sin_muestras(),
            &Falso::default(),
        );
        assert!(r.avisos.is_empty() && r.error.is_empty(), "{r:?}");
    }

    #[test]
    fn el_animador_no_deja_pasar_un_error() {
        let a = AnimadorTikra { tinta: Color::NEGRO };
        assert!(a.fotograma("nada", 0.0).is_none());
        let fuente =
            r"\begin{animacion}[t=0:90]\begin{tikzpicture}\draw (0,0) -- (\t:1);\end{tikzpicture}\end{animacion}";
        assert!(a.fotograma(fuente, 1.0).is_some());
    }

    #[derive(Default)]
    struct Pintado(Mutex<Vec<String>>);

    impl Destino for Pintado {
        fn publicar(&self, escena: Escena, _animador: Arc<dyn Animador>, _fuentes: Vec<String>) {
            self.0.lock().unwrap().push(format!("{:?}", escena.primitivas));
        }
    }

    /// Cada tinta compone lo suyo, con una imprenta propia: lo compuesto la lleva dentro.
    #[test]
    fn cambiar_de_tinta_recompone_con_la_nueva_sin_reiniciar() {
        let imprenta = Imprenta::nueva(Vec::new());
        let pintado = Pintado::default();
        let fuente = r"Sea $x$ \begin{tikzpicture}\draw (0,0) -- (1,1);\end{tikzpicture}";
        for tinta in [Color::NEGRO, Color::BLANCO, Color::NEGRO] {
            let mut e = encargo(fuente, 400.0);
            e.tinta = tinta;
            let r = entero(&imprenta, &e, &sin_muestras(), &pintado);
            assert_eq!(r.error, "");
        }
        let hechos = pintado.0.lock().unwrap();
        let [negro, blanco, otra_vez] = &hechos[..] else {
            panic!("tres composiciones");
        };
        assert_eq!(negro, otra_vez, "volver a la tinta deja lo mismo");
        let color = |c: Color| format!("{c:?}");
        assert!(negro.contains(&color(Color::NEGRO)) && !negro.contains(&color(Color::BLANCO)));
        assert!(blanco.contains(&color(Color::BLANCO)) && !blanco.contains(&color(Color::NEGRO)));
    }

    #[test]
    fn un_color_pedido_no_cambia_con_la_tinta() {
        let imprenta = Imprenta::nueva(Vec::new());
        let pintado = Pintado::default();
        let mut e = encargo(r"\textcolor{black}{negro}", 400.0);
        e.tinta = Color::BLANCO;
        entero(&imprenta, &e, &sin_muestras(), &pintado);
        assert!(pintado.0.lock().unwrap()[0].contains(&format!("{:?}", Color::NEGRO)));
    }

    #[test]
    fn las_animaciones_salen_con_la_tinta() {
        let fuente =
            r"\begin{animacion}[t=0:90]\begin{tikzpicture}\draw (0,0) -- (\t:1);\end{tikzpicture}\end{animacion}";
        let a = AnimadorTikra { tinta: Color::BLANCO };
        let escena = a.fotograma(fuente, 0.0).unwrap();
        let hecho = format!("{:?}", escena.primitivas);
        assert!(hecho.contains(&format!("{:?}", Color::BLANCO)), "{hecho}");
        assert!(!hecho.contains(&format!("{:?}", Color::NEGRO)), "{hecho}");
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
