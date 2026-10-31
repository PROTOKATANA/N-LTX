//! El lienzo: la escena publicada y lo que se pinta de ella en cada momento.
//!
//! La escena base se compila una sola vez por escala; lo único que cuesta en cada fotograma son
//! las animaciones que se ven. Lo que sale de aquí son valores ya hechos, que no cambian.

use std::sync::Arc;

use primitivas::{Escena, Punto};

use crate::atlas::Atlas;
use crate::compilador::{Destino, Encuadre, compilar};
use crate::geometria::Afin;
use crate::lista::Lista;
use crate::puente_lienzo::ffi::{Tinta, Vista};
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
/// corre en el hilo del taller.
pub trait Animador: Send + Sync {
    /// El fotograma de `fuente` a los `segundos`, o `None` si esa fuente no se puede componer.
    fn fotograma(&self, fuente: &str, segundos: f64) -> Option<Escena>;
}

/// La lista base y para qué escena y escala se compiló.
struct Compilada {
    version: u64,
    escala: f64,
    lista: Arc<Lista>,
    filas: u32,
    generacion: u64,
}

/// Un fotograma hecho. Cada parte lleva su generación: mientras no cambie, no hay que copiarla.
#[derive(Debug, Clone, PartialEq)]
pub struct Hecho {
    pub vista: Vista,
    pub generacion_base: u64,
    pub base: Arc<Lista>,
    pub generacion_vivos: u64,
    pub vivos: Lista,
    pub tintas: Vec<Tinta>,
    /// Hay algo moviéndose a la vista: hará falta otro fotograma.
    pub animada: bool,
}

/// Un fotograma y las filas de textura que cambiaron para pintarlo.
#[derive(Debug, Clone, PartialEq)]
pub struct Entrega {
    pub hecho: Arc<Hecho>,
    pub atlas: Sucio,
    pub rampas: Sucio,
}

#[derive(Default)]
pub struct Lienzo {
    escena: Escena,
    animador: Option<Arc<dyn Animador>>,
    version: u64,
    compilada: Option<Compilada>,
    generaciones_base: u64,
    generaciones_vivos: u64,
    habia_vivos: bool,
    atlas: Atlas,
    rampas: Rampas,
}

fn raiz(escena: &Escena, escala: f64) -> Afin {
    let c = escena.caja;
    Afin::escala(escala).por(Afin::traslacion(MARGEN - c.x, MARGEN - c.y))
}

impl Lienzo {
    pub fn publicar(&mut self, escena: Escena, animador: Option<Arc<dyn Animador>>) {
        self.escena = escena;
        self.animador = animador;
        self.version += 1;
    }

    /// La GPU no tiene nada: el próximo fotograma llevará las texturas enteras.
    pub fn reenviar(&mut self) {
        self.atlas.marcar_todo();
        self.rampas.marcar_todo();
    }

    fn compilar_base(&mut self, escala: f64) {
        // otra escala son otras máscaras; con la misma, escribir o reflujar reaprovecha las que hay
        if self.compilada.as_ref().is_none_or(|c| c.escala != escala) {
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
        self.generaciones_base += 1;
        self.compilada = Some(Compilada {
            version: self.version,
            escala,
            lista: Arc::new(lista),
            filas: self.rampas.filas(),
            generacion: self.generaciones_base,
        });
    }

    pub fn preparar(&mut self, vista: Vista, segundos: f64) -> Entrega {
        let escala = vista.escala.max(1e-3);
        let vista = Vista { escala, ..vista };
        let al_dia = |c: &Compilada| c.version == self.version && c.escala == escala;
        if !self.compilada.as_ref().is_some_and(al_dia) {
            self.compilar_base(escala);
        }
        let (base, filas, generacion_base) = match &self.compilada {
            Some(c) => (Arc::clone(&c.lista), c.filas, c.generacion),
            None => (Arc::new(Lista::base()), 0, 0),
        };
        self.rampas.truncar(filas);
        let mut vivos = Lista::tras(&base);
        let animada = self.animar(vista, segundos, &mut vivos);
        let hay_vivos = !vivos.comandos.is_empty();
        if hay_vivos || self.habia_vivos {
            self.generaciones_vivos += 1;
        }
        self.habia_vivos = hay_vivos;
        let mut tintas = base.tintas.clone();
        tintas.extend_from_slice(&vivos.tintas);
        let hecho = Hecho {
            vista,
            generacion_base,
            base,
            generacion_vivos: self.generaciones_vivos,
            vivos,
            tintas,
            animada,
        };
        Entrega {
            hecho: Arc::new(hecho),
            atlas: self.atlas.tomar_sucio(),
            rampas: self.rampas.tomar_sucio(),
        }
    }

    /// Compone las animaciones que se ven. Las que no, no gastan nada.
    fn animar(&mut self, vista: Vista, segundos: f64, vivos: &mut Lista) -> bool {
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
            if esquina.y + zona.caja.alto * escala < vista.y || esquina.y > vista.y + f64::from(vista.alto) {
                continue;
            }
            animada = true;
            let Some(f) = animador.fotograma(&zona.fuente, segundos) else {
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
            ancho: 100,
            alto: 100,
            escala: 1.0,
            y,
        }
    }

    fn doble(y: f64) -> Vista {
        Vista {
            escala: 2.0,
            ..vista(y)
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
    fn el_texto_descuenta_los_margenes_y_tiene_un_minimo() {
        assert_eq!(ancho_de_texto(552.0), 520.0);
        assert_eq!(ancho_de_texto(10.0), MENOR_TEXTO);
    }

    #[test]
    fn la_base_se_compila_una_vez_por_escala_y_por_escena() {
        let mut l = Lienzo::default();
        l.publicar(escena(Vec::new()), None);
        let a = l.preparar(vista(0.0), 0.0).hecho.generacion_base;
        let desplazada = l.preparar(vista(50.0), 0.0);
        assert_eq!(desplazada.hecho.generacion_base, a, "desplazar no recompila");
        assert_eq!(desplazada.hecho.vista.y, 50.0, "pero lleva la vista nueva");
        let b = l.preparar(doble(0.0), 0.0).hecho.generacion_base;
        assert_ne!(a, b);
        l.publicar(escena(Vec::new()), None);
        assert_ne!(l.preparar(doble(0.0), 0.0).hecho.generacion_base, b);
    }

    #[test]
    fn lo_vivo_solo_cambia_de_generacion_si_hay_o_habia_algo() {
        let mut l = Lienzo::default();
        l.publicar(escena(vec![zona(50.0)]), Some(Arc::new(Cuenta::default())));
        let a = l.preparar(vista(0.0), 0.0).hecho.generacion_vivos;
        let b = l.preparar(vista(0.0), 0.0).hecho.generacion_vivos;
        assert_ne!(a, b, "lo animado cambia en cada fotograma");
        let fuera = l.preparar(vista(500.0), 0.0).hecho.generacion_vivos;
        assert_ne!(fuera, b, "al salir de la vista hay que borrarlo una vez");
        assert_eq!(l.preparar(vista(500.0), 0.0).hecho.generacion_vivos, fuera);
    }

    #[test]
    fn la_escena_cae_dentro_del_margen() {
        let mut l = Lienzo::default();
        l.publicar(escena(Vec::new()), None);
        let e = l.preparar(doble(0.0), 0.0);
        assert_eq!(e.hecho.base.vertices[0].x, (MARGEN * 2.0) as f32);
    }

    #[test]
    fn una_animacion_que_no_se_ve_no_se_compone() {
        let cuenta = Arc::new(Cuenta::default());
        let mut l = Lienzo::default();
        l.publicar(escena(vec![zona(500.0)]), Some(cuenta.clone()));
        assert!(!l.preparar(vista(0.0), 0.0).hecho.animada);
        assert_eq!(cuenta.0.load(Ordering::Relaxed), 0);
        let dentro = l.preparar(vista(480.0), 0.0);
        assert!(dentro.hecho.animada);
        assert_eq!(cuenta.0.load(Ordering::Relaxed), 1);
        let ordenes: Vec<Orden> = dentro.hecho.vivos.comandos.iter().map(|c| c.orden).collect();
        assert_eq!(ordenes, [Orden::Recortar, Orden::Dibujar, Orden::Soltar]);
    }

    #[test]
    fn el_fotograma_cae_en_su_zona() {
        let mut l = Lienzo::default();
        l.publicar(escena(vec![zona(50.0)]), Some(Arc::new(Cuenta::default())));
        let e = l.preparar(vista(0.0), 0.0);
        // el rectángulo del recorte: la caja del fotograma, sobre la zona y con el margen
        let v = &e.hecho.vivos.vertices;
        let x = v.iter().map(|v| v.x).fold(f32::MAX, f32::min);
        let y = v.iter().map(|v| v.y).fold(f32::MAX, f32::min);
        assert_eq!((x, y), ((20.0 + MARGEN) as f32, (50.0 + MARGEN) as f32));
    }

    #[test]
    fn una_gpu_nueva_recibe_el_atlas_entero() {
        let mut l = Lienzo::default();
        l.publicar(escena(Vec::new()), None);
        l.preparar(vista(0.0), 0.0);
        assert!(l.preparar(vista(0.0), 0.0).atlas.bytes.is_empty(), "ya se subió");
        l.reenviar();
        let nueva = l.preparar(vista(0.0), 0.0);
        assert_eq!(nueva.atlas.bytes.len(), (LADO_ATLAS * LADO_ATLAS) as usize);
    }

    #[test]
    fn republicar_a_la_misma_escala_no_vuelve_a_subir_el_atlas() {
        let mut l = Lienzo::default();
        l.publicar(con_glifo(), None);
        assert!(!l.preparar(vista(0.0), 0.0).atlas.bytes.is_empty());
        l.publicar(con_glifo(), None);
        assert!(
            l.preparar(vista(0.0), 0.0).atlas.bytes.is_empty(),
            "la máscara ya estaba"
        );
        assert!(!l.preparar(doble(0.0), 0.0).atlas.bytes.is_empty());
    }
}
