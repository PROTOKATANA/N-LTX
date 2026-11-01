//! Cuándo enseñar un documento que aún se está componiendo y cuándo dejarlo: lo que sigue una
//! composición sin saber nada de quién la pinta.

use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use documento::Seguimiento;
use primitivas::Composicion;

use crate::compositor::Encargo;

/// Lo que se espera a enseñar el primer tramo: K1 llena una pantalla en 5 ms, en frío. Lo que se
/// compone antes sale entero, sin tramos.
const PRIMER_TRAMO: Duration = Duration::from_millis(4);
/// Hasta dónde crece la espera entre tramos: basta para ver crecer un documento largo.
const MAYOR_ESPERA: Duration = Duration::from_millis(250);

/// El alto de un documento en la vista.
pub(crate) fn alto_con_margen(alto: f64) -> f64 {
    alto + 2.0 * pintor::MARGEN
}

/// Lo último que se ha dejado a la vista.
#[derive(Debug, Default)]
pub(crate) struct Mostrado {
    pub(crate) fuente: String,
    pub(crate) alto: f64,
}

/// Si `nuevo` deja sin sentido lo que se está componiendo. Más texto detrás o otro ancho, no: acabar
/// enseña algo mejor que lo que hay, y un modelo que escribe más deprisa de lo que se compone, o una
/// ventana que se arrastra, no llegarían a verse nunca.
fn sustituye(actual: &Encargo, nuevo: &Encargo) -> bool {
    nuevo.tinta != actual.tinta
        || nuevo.con_muestras != actual.con_muestras
        || !nuevo.fuente.starts_with(&actual.fuente)
}

/// Cuándo enseñar lo colocado. Cada tramo copia la escena y la recompila entera (K1: 1,1 ms y de 6 a
/// 17 ms), así que se espacian: la espera se dobla y K1 se queda en menos de diez, en vez de uno por bloque.
struct Ritmo {
    ultimo: Instant,
    espera: Duration,
    /// Hasta dónde se ve lo mismo o algo mejor: un tramo más corto se comería lo que ya está a la vista.
    visto: f64,
    alguno: bool,
}

impl Ritmo {
    /// Otra composición del mismo documento, o el mismo con más texto, deja lo que hay hasta que lo
    /// supere: cambiar de tinta o de ancho no debe dejar la vista en blanco, ni un modelo que escribe
    /// hacer parpadear lo ya escrito.
    fn nuevo(encargo: &Encargo, mostrado: &Mostrado) -> Ritmo {
        let sigue = encargo.fuente.starts_with(&mostrado.fuente);
        Ritmo {
            ultimo: Instant::now(),
            espera: PRIMER_TRAMO,
            visto: if sigue { mostrado.alto } else { 0.0 },
            alguno: false,
        }
    }

    /// `alto`, con el margen. Lo que puede tardar no espera: lo de arriba no debe quedarse sin enseñar
    /// por una animación que venga detrás.
    fn toca(&self, alto: f64, lento: bool, pasado: Duration) -> bool {
        alto > self.visto && (pasado >= self.espera || (lento && !self.alguno))
    }

    fn apuntar(&mut self, alto: f64) {
        self.ultimo = Instant::now();
        self.espera = (self.espera * 2).min(MAYOR_ESPERA);
        self.visto = alto;
        self.alguno = true;
    }

    /// Hasta dónde se ha enseñado algo de este encargo.
    fn alcanzado(&self) -> Option<f64> {
        self.alguno.then_some(self.visto)
    }
}

/// Sigue una composición: enseña lo que va saliendo y la deja si llega algo que la hace inútil.
pub(crate) struct Tramos<'a> {
    encargo: &'a Encargo,
    encargos: &'a Receiver<Encargo>,
    ritmo: Ritmo,
    /// El último encargo que ha llegado entretanto.
    siguiente: Option<Encargo>,
    /// Enseña lo colocado y dice hasta dónde llega, con el margen.
    mostrar: &'a dyn Fn(Composicion) -> f64,
}

impl<'a> Tramos<'a> {
    pub(crate) fn nuevos(
        encargo: &'a Encargo,
        encargos: &'a Receiver<Encargo>,
        mostrado: &Mostrado,
        mostrar: &'a dyn Fn(Composicion) -> f64,
    ) -> Self {
        Tramos {
            encargo,
            encargos,
            ritmo: Ritmo::nuevo(encargo, mostrado),
            siguiente: None,
            mostrar,
        }
    }

    /// Hasta dónde se ha enseñado algo de este encargo y lo que llegó entretanto.
    pub(crate) fn acabar(self) -> (Option<f64>, Option<Encargo>) {
        (self.ritmo.alcanzado(), self.siguiente)
    }
}

impl Seguimiento for Tramos<'_> {
    fn parar(&mut self) -> bool {
        while let Ok(encargo) = self.encargos.try_recv() {
            self.siguiente = Some(encargo);
        }
        self.siguiente.as_ref().is_some_and(|s| sustituye(self.encargo, s))
    }

    fn quiere_tramo(&mut self, alto: f64, lento: bool) -> bool {
        // sin nada colocado, el margen solo parecería un documento en blanco
        alto > 0.0
            && self
                .ritmo
                .toca(alto_con_margen(alto), lento, self.ritmo.ultimo.elapsed())
    }

    fn tramo(&mut self, parcial: Composicion) {
        let alto = (self.mostrar)(parcial);
        self.ritmo.apuntar(alto);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::mpsc::channel;

    use primitivas::Color;

    fn encargo(fuente: &str, ancho: f64) -> Encargo {
        Encargo {
            fuente: fuente.into(),
            ancho,
            con_muestras: false,
            tinta: Color::NEGRO,
        }
    }

    fn ritmo(visto: f64) -> Ritmo {
        Ritmo::nuevo(
            &encargo("", 400.0),
            &Mostrado {
                fuente: String::new(),
                alto: visto,
            },
        )
    }

    #[test]
    fn lo_que_se_compone_deprisa_no_se_enseña_a_trozos() {
        assert!(!ritmo(0.0).toca(100.0, false, PRIMER_TRAMO / 2));
        assert!(ritmo(0.0).toca(100.0, false, PRIMER_TRAMO));
    }

    /// RAYO: sin esto, un párrafo seguido de una animación tarda lo que tarde en medirse (K1: hasta
    /// 208 ms una sola) en verse.
    #[test]
    fn lo_lento_no_retiene_lo_que_ya_esta_colocado() {
        let mut r = ritmo(0.0);
        assert!(r.toca(100.0, true, Duration::ZERO));
        r.apuntar(100.0);
        assert!(!r.toca(200.0, true, Duration::ZERO), "solo el primero se adelanta");
    }

    #[test]
    fn sin_nada_nuevo_no_hay_tramo() {
        assert!(!ritmo(0.0).toca(0.0, true, MAYOR_ESPERA));
        assert!(!ritmo(300.0).toca(300.0, true, MAYOR_ESPERA));
    }

    /// RAYO: cada tramo recompila la escena entera. Si dejara de crecer, un documento largo la
    /// recompilaría cien veces en vez de una decena.
    #[test]
    fn los_tramos_se_espacian_hasta_un_tope() {
        let mut r = ritmo(0.0);
        let mut esperas = Vec::new();
        for n in 1..=8 {
            esperas.push(r.espera);
            r.apuntar(f64::from(n));
        }
        assert_eq!(esperas[1], PRIMER_TRAMO * 2);
        assert_eq!(r.espera, MAYOR_ESPERA);
        assert!(esperas.windows(2).all(|p| p[0] <= p[1]));
    }

    #[test]
    fn lo_mismo_o_con_mas_texto_no_se_tapa_con_un_tramo_mas_corto() {
        let mostrado = Mostrado {
            fuente: "Hola".into(),
            alto: 500.0,
        };
        let mismo = Ritmo::nuevo(&encargo("Hola", 200.0), &mostrado);
        let mas = Ritmo::nuevo(&encargo("Hola, mundo", 400.0), &mostrado);
        let otro = Ritmo::nuevo(&encargo("Adiós", 400.0), &mostrado);
        assert!(!mismo.toca(400.0, true, MAYOR_ESPERA));
        assert!(!mas.toca(400.0, true, MAYOR_ESPERA));
        assert!(mas.toca(600.0, true, MAYOR_ESPERA));
        assert!(otro.toca(400.0, true, MAYOR_ESPERA));
    }

    #[test]
    fn solo_lo_que_cambia_de_documento_o_de_tinta_sustituye() {
        let actual = encargo("Hola", 400.0);
        assert!(!sustituye(&actual, &encargo("Hola", 300.0)), "otro ancho");
        assert!(!sustituye(&actual, &encargo("Hola, mundo", 400.0)), "más texto");
        assert!(sustituye(&actual, &encargo("Adiós", 400.0)));
        let blanca = Encargo {
            tinta: Color::BLANCO,
            ..actual.clone()
        };
        assert!(sustituye(&actual, &blanca));
    }

    #[test]
    fn se_deja_lo_que_otro_encargo_sustituye_y_se_acaba_lo_que_no() {
        let (tx, rx) = channel();
        let actual = encargo("Hola", 400.0);
        let mostrar = |_: Composicion| 0.0;
        let mut t = Tramos::nuevos(&actual, &rx, &Mostrado::default(), &mostrar);
        assert!(!t.parar());
        tx.send(encargo("Hola, mundo", 400.0)).unwrap();
        assert!(!t.parar(), "más texto detrás no la deja");
        tx.send(encargo("Adiós", 400.0)).unwrap();
        assert!(t.parar());
        let (alcanzado, siguiente) = t.acabar();
        assert_eq!(alcanzado, None);
        assert_eq!(siguiente.unwrap().fuente, "Adiós", "lo que llegó no se pierde");
    }

    #[test]
    fn sin_nada_colocado_no_se_enseña_nada_aunque_toque() {
        let (_tx, rx) = channel();
        let actual = encargo("Hola", 400.0);
        let mostrar = |_: Composicion| 0.0;
        let mut t = Tramos::nuevos(&actual, &rx, &Mostrado::default(), &mostrar);
        assert!(!t.quiere_tramo(0.0, true), "el margen solo no es un documento");
        assert!(t.quiere_tramo(10.0, true));
    }

    #[test]
    fn lo_enseñado_se_recuerda() {
        let (_tx, rx) = channel();
        let actual = encargo("Hola", 400.0);
        let veces = Cell::new(0);
        let mostrar = |_: Composicion| {
            veces.set(veces.get() + 1);
            300.0
        };
        let mut t = Tramos::nuevos(&actual, &rx, &Mostrado::default(), &mostrar);
        t.tramo(Composicion::default());
        assert_eq!(veces.get(), 1);
        assert_eq!(t.acabar().0, Some(300.0));
    }
}
