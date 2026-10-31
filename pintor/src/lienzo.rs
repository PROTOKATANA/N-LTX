//! El lienzo: la escena publicada desde el lado de la aplicación y lo que le pide el pintor.
//!
//! La escena base se compila una sola vez por escala; lo único que cuesta en cada fotograma son
//! las animaciones que se ven.

use std::sync::Arc;

use primitivas::{Escena, Punto};

use crate::atlas::Atlas;
use crate::compilador::{Destino, Encuadre, compilar};
use crate::geometria::Afin;
use crate::lista::Lista;
use crate::puente_lienzo::ffi::{Comando, Tinta, Vertice, Vista};
use crate::rampas::Rampas;
use crate::textura::Sucio;

/// Margen alrededor de la escena, en bp.
pub const MARGEN: f64 = 16.0;

/// Por muy estrecha que sea la vista, el texto no baja de aquí, en bp.
const MENOR_TEXTO: f64 = 40.0;

/// El ancho que le queda al texto en una vista de `vista` bp.
pub fn ancho_de_texto(vista: f64) -> f64 {
    (vista - 2.0 * MARGEN).max(MENOR_TEXTO)
}

/// Sabe recomponer las animaciones de una escena. Quien lo implementa no debe entrar en pánico:
/// corre en el hilo de render.
pub trait Animador: Send + Sync {
    /// El fotograma de `fuente` a los `segundos`, o `None` si esa fuente no se puede componer.
    fn fotograma(&self, fuente: &str, segundos: f64) -> Option<Escena>;
}

/// La lista base y para qué escena y escala se compiló.
struct Hecha {
    version: u64,
    escala: f64,
    lista: Arc<Lista>,
    filas: u32,
    generacion: u64,
}

#[derive(Default)]
pub(crate) struct Lienzo {
    escena: Escena,
    animador: Option<Arc<dyn Animador>>,
    version: u64,
    hecha: Option<Hecha>,
    generaciones: u64,
    atlas: Atlas,
    rampas: Rampas,
}

fn raiz(escena: &Escena, escala: f64) -> Afin {
    let c = escena.caja;
    Afin::escala(escala).por(Afin::traslacion(MARGEN - c.x, MARGEN - c.y))
}

impl Lienzo {
    pub(crate) fn publicar(&mut self, escena: Escena, animador: Option<Arc<dyn Animador>>) {
        self.escena = escena;
        self.animador = animador;
        self.version += 1;
    }

    fn compilar_base(&mut self, escala: f64) {
        // otra escala son otras máscaras; con la misma, escribir o reflujar reaprovecha las que hay
        if self.hecha.as_ref().is_none_or(|h| h.escala != escala) {
            self.atlas.limpiar();
        }
        self.rampas.limpiar();
        let mut lista = Lista::base();
        let encuadre = Encuadre {
            raiz: raiz(&self.escena, escala),
            recortada: false,
        };
        let destino = Destino {
            lista: &mut lista,
            atlas: &mut self.atlas,
            rampas: &mut self.rampas,
        };
        compilar(&self.escena, encuadre, destino);
        self.generaciones += 1;
        self.hecha = Some(Hecha {
            version: self.version,
            escala,
            lista: Arc::new(lista),
            filas: self.rampas.filas(),
            generacion: self.generaciones,
        });
    }

    pub(crate) fn preparar(&mut self, vista: Vista, nuevos: bool) -> Marco {
        let escala = vista.escala.max(1e-3);
        let vista = Vista { escala, ..vista };
        let al_dia = |h: &Hecha| h.version == self.version && h.escala == escala;
        if !self.hecha.as_ref().is_some_and(al_dia) {
            self.compilar_base(escala);
        }
        if nuevos {
            self.atlas.marcar_todo();
            self.rampas.marcar_todo();
        }
        let (base, filas, generacion) = match &self.hecha {
            Some(h) => (Arc::clone(&h.lista), h.filas, h.generacion),
            None => (Arc::new(Lista::base()), 0, 0),
        };
        self.rampas.truncar(filas);
        let mut vivos = Lista::tras(&base);
        let animada = self.animar(vista, &mut vivos);
        let mut tintas = base.tintas.clone();
        tintas.extend_from_slice(&vivos.tintas);
        Marco {
            generacion,
            base,
            vivos,
            tintas,
            atlas: self.atlas.tomar_sucio(),
            rampas: self.rampas.tomar_sucio(),
            animada,
        }
    }

    /// Compone las animaciones que se ven. Las que no, no gastan nada.
    fn animar(&mut self, vista: Vista, vivos: &mut Lista) -> bool {
        let Some(animador) = &self.animador else {
            return false;
        };
        let escala = vista.escala;
        let m = raiz(&self.escena, escala);
        let mut animada = false;
        for zona in &self.escena.zonas {
            let esquina = m.aplicar(Punto {
                x: zona.caja.x,
                y: zona.caja.y,
            });
            if esquina.y + zona.caja.alto * escala < vista.y || esquina.y > vista.y + vista.alto {
                continue;
            }
            animada = true;
            let Some(f) = animador.fotograma(&zona.fuente, vista.segundos) else {
                continue;
            };
            let dentro = Afin::traslacion(zona.caja.x - f.caja.x, zona.caja.y - f.caja.y);
            let encuadre = Encuadre {
                raiz: m.por(dentro),
                recortada: true,
            };
            let destino = Destino {
                lista: &mut *vivos,
                atlas: &mut self.atlas,
                rampas: &mut self.rampas,
            };
            compilar(&f, encuadre, destino);
        }
        animada
    }
}

/// Todo lo que el pintor necesita para un fotograma. Es suyo: lo lee sin bloquear a nadie.
pub struct Marco {
    generacion: u64,
    base: Arc<Lista>,
    vivos: Lista,
    tintas: Vec<Tinta>,
    atlas: Sucio,
    rampas: Sucio,
    animada: bool,
}

impl Marco {
    pub(crate) fn vacio() -> Marco {
        Marco {
            generacion: 0,
            base: Arc::new(Lista::base()),
            vivos: Lista::default(),
            tintas: Lista::base().tintas,
            atlas: Sucio::default(),
            rampas: Sucio::default(),
            animada: false,
        }
    }

    pub fn generacion(&self) -> u64 {
        self.generacion
    }

    pub fn vertices_base(&self) -> &[Vertice] {
        &self.base.vertices
    }

    pub fn indices_base(&self) -> &[u32] {
        &self.base.indices
    }

    pub fn comandos_base(&self) -> &[Comando] {
        &self.base.comandos
    }

    pub fn vertices_vivos(&self) -> &[Vertice] {
        &self.vivos.vertices
    }

    pub fn indices_vivos(&self) -> &[u32] {
        &self.vivos.indices
    }

    pub fn comandos_vivos(&self) -> &[Comando] {
        &self.vivos.comandos
    }

    pub fn tintas(&self) -> &[Tinta] {
        &self.tintas
    }

    pub fn atlas(&self) -> &[u8] {
        &self.atlas.bytes
    }

    pub fn atlas_y(&self) -> u32 {
        self.atlas.y
    }

    pub fn rampas(&self) -> &[u8] {
        &self.rampas.bytes
    }

    pub fn rampas_y(&self) -> u32 {
        self.rampas.y
    }

    pub fn animada(&self) -> bool {
        self.animada
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::LADO_ATLAS;
    use crate::puente_lienzo::ffi::Orden;
    use primitivas::{Caja, Color, GlifoId, Matriz, Pintura, Primitiva, Relleno, Segmento, Zona};
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicU32, Ordering};

    fn cuadro() -> Primitiva {
        let p = |x, y| Punto { x, y };
        Primitiva::Trazado {
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
        }
    }

    fn vista(y: f64) -> Vista {
        Vista {
            alto: 100.0,
            escala: 1.0,
            y,
            segundos: 0.0,
        }
    }

    fn escena(zonas: Vec<Zona>) -> Escena {
        Escena {
            caja: Caja {
                x: 0.0,
                y: 0.0,
                ancho: 100.0,
                alto: 1000.0,
            },
            primitivas: vec![cuadro()],
            zonas,
            ..Escena::default()
        }
    }

    #[derive(Default)]
    struct Cuenta(AtomicU32);

    impl Animador for Cuenta {
        fn fotograma(&self, _fuente: &str, _segundos: f64) -> Option<Escena> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Some(Escena {
                caja: Caja {
                    x: -5.0,
                    y: -5.0,
                    ancho: 10.0,
                    alto: 10.0,
                },
                primitivas: vec![cuadro()],
                ..Escena::default()
            })
        }
    }

    fn zona(y: f64) -> Zona {
        Zona {
            caja: Caja {
                x: 20.0,
                y,
                ancho: 10.0,
                alto: 10.0,
            },
            fuente: "a".into(),
        }
    }

    #[test]
    fn la_base_se_compila_una_vez_por_escala_y_por_escena() {
        let mut l = Lienzo::default();
        l.publicar(escena(Vec::new()), None);
        let a = l.preparar(vista(0.0), false).generacion;
        assert_eq!(l.preparar(vista(50.0), false).generacion, a, "desplazar no recompila");
        let b = l
            .preparar(
                Vista {
                    escala: 2.0,
                    ..vista(0.0)
                },
                false,
            )
            .generacion;
        assert_ne!(a, b);
        l.publicar(escena(Vec::new()), None);
        assert_ne!(
            l.preparar(
                Vista {
                    escala: 2.0,
                    ..vista(0.0)
                },
                false
            )
            .generacion,
            b
        );
    }

    #[test]
    fn la_escena_cae_dentro_del_margen() {
        let mut l = Lienzo::default();
        l.publicar(escena(Vec::new()), None);
        let m = l.preparar(
            Vista {
                escala: 2.0,
                ..vista(0.0)
            },
            false,
        );
        assert_eq!(m.vertices_base()[0].x, (MARGEN * 2.0) as f32);
    }

    #[test]
    fn una_animacion_que_no_se_ve_no_se_compone() {
        let cuenta = Arc::new(Cuenta::default());
        let mut l = Lienzo::default();
        l.publicar(escena(vec![zona(500.0)]), Some(cuenta.clone()));
        let fuera = l.preparar(vista(0.0), false);
        assert!(!fuera.animada());
        assert_eq!(cuenta.0.load(Ordering::Relaxed), 0);
        let dentro = l.preparar(vista(480.0), false);
        assert!(dentro.animada());
        assert_eq!(cuenta.0.load(Ordering::Relaxed), 1);
        assert_eq!(
            dentro.comandos_vivos().iter().map(|c| c.orden).collect::<Vec<_>>(),
            [Orden::Recortar, Orden::Dibujar, Orden::Soltar]
        );
    }

    #[test]
    fn el_fotograma_cae_en_su_zona() {
        let mut l = Lienzo::default();
        l.publicar(escena(vec![zona(50.0)]), Some(Arc::new(Cuenta::default())));
        let m = l.preparar(vista(0.0), false);
        // el rectángulo del recorte: la caja del fotograma, sobre la zona y con el margen
        let v = m.vertices_vivos();
        let x = v.iter().map(|v| v.x).fold(f32::MAX, f32::min);
        let y = v.iter().map(|v| v.y).fold(f32::MAX, f32::min);
        assert_eq!((x, y), ((20.0 + MARGEN) as f32, (50.0 + MARGEN) as f32));
    }

    #[test]
    fn una_gpu_nueva_recibe_el_atlas_entero() {
        let mut l = Lienzo::default();
        l.publicar(escena(Vec::new()), None);
        l.preparar(vista(0.0), false);
        assert!(l.preparar(vista(0.0), false).atlas().is_empty(), "ya se subió");
        let nuevo = l.preparar(vista(0.0), true);
        assert_eq!(nuevo.atlas().len(), (LADO_ATLAS * LADO_ATLAS) as usize);
    }

    fn con_glifo() -> Escena {
        let g = GlifoId { fuente: 0, id: 1 };
        let p = |x, y| Punto { x, y };
        Escena {
            primitivas: vec![Primitiva::Glifo {
                glifo: g,
                matriz: Matriz {
                    a: 1.0,
                    b: 0.0,
                    c: 0.0,
                    d: -1.0,
                    e: 5.0,
                    f: 20.0,
                },
                color: Color { r: 0.0, g: 0.0, b: 0.0 },
            }],
            contornos: BTreeMap::from([(
                g,
                vec![
                    Segmento::Mover(p(0.0, 0.0)),
                    Segmento::Linea(p(8.0, 0.0)),
                    Segmento::Linea(p(8.0, 8.0)),
                    Segmento::Cerrar,
                ],
            )]),
            ..Escena::default()
        }
    }

    #[test]
    fn republicar_a_la_misma_escala_no_vuelve_a_subir_el_atlas() {
        let mut l = Lienzo::default();
        l.publicar(con_glifo(), None);
        assert!(!l.preparar(vista(0.0), false).atlas().is_empty());
        l.publicar(con_glifo(), None);
        assert!(l.preparar(vista(0.0), false).atlas().is_empty(), "la máscara ya estaba");
        assert!(
            !l.preparar(
                Vista {
                    escala: 2.0,
                    ..vista(0.0)
                },
                false
            )
            .atlas()
            .is_empty()
        );
    }

    #[test]
    fn el_texto_descuenta_los_margenes_y_tiene_un_minimo() {
        assert_eq!(ancho_de_texto(552.0), 520.0);
        assert_eq!(ancho_de_texto(10.0), MENOR_TEXTO);
    }
}
