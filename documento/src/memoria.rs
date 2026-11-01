//! Recordar lo que ya se compuso.
//!
//! Lo que el texto lleva dentro no depende del ancho ni de lo que lo rodea, y componerlo es lo más
//! caro de un documento: reflujar una ventana o añadir un párrafo no debe repetirlo.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Mutex, PoisonError};

use primitivas::{Caja, Composicion, ErrorComposicion};

use crate::{ColoresEditor, Fragmento, Incrustados, Resaltador};

/// Entradas de cada tipo, como mucho. Un modelo que escribe en directo deja muchas a medias.
const MAX_RECUERDOS: usize = 1024;

struct Recuerdos<K, V>(Mutex<HashMap<K, Result<V, ErrorComposicion>>>);

impl<K: Eq + Hash, V: Clone> Recuerdos<K, V> {
    fn nuevos() -> Self {
        Recuerdos(Mutex::new(HashMap::new()))
    }

    /// Sin bloquear mientras se compone: componer es lo lento, y un pánico no debe envenenarlo.
    fn dar(&self, clave: K, componer: impl FnOnce() -> Result<V, ErrorComposicion>) -> Result<V, ErrorComposicion> {
        if let Some(r) = self.bloquear().get(&clave) {
            return r.clone();
        }
        let r = componer();
        let mut recuerdos = self.bloquear();
        if recuerdos.len() >= MAX_RECUERDOS {
            recuerdos.clear();
        }
        recuerdos.insert(clave, r.clone());
        r
    }

    fn bloquear(&self) -> std::sync::MutexGuard<'_, HashMap<K, Result<V, ErrorComposicion>>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Otros `Incrustados` o un `Resaltador` que no repiten trabajo. Los errores también se recuerdan: una fórmula rota
/// no se vuelve a intentar en cada pulsación.
pub struct Memoria<I> {
    dentro: I,
    formulas: Recuerdos<(String, bool, u64), Composicion>,
    figuras: Recuerdos<String, Composicion>,
    cajas: Recuerdos<String, Caja>,
    resaltados: Recuerdos<(String, String, String), Vec<Fragmento>>,
}

impl<I> Memoria<I> {
    pub fn nueva(dentro: I) -> Self {
        Memoria {
            dentro,
            formulas: Recuerdos::nuevos(),
            figuras: Recuerdos::nuevos(),
            cajas: Recuerdos::nuevos(),
            resaltados: Recuerdos::nuevos(),
        }
    }
}

impl<I: Incrustados> Incrustados for Memoria<I> {
    fn formula(&self, latex: &str, bloque: bool, tam: f64) -> Result<Composicion, ErrorComposicion> {
        self.formulas.dar((latex.to_string(), bloque, tam.to_bits()), || {
            self.dentro.formula(latex, bloque, tam)
        })
    }

    fn figura(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
        self.figuras.dar(fuente.to_string(), || self.dentro.figura(fuente))
    }

    fn caja_de_animacion(&self, fuente: &str) -> Result<Caja, ErrorComposicion> {
        self.cajas
            .dar(fuente.to_string(), || self.dentro.caja_de_animacion(fuente))
    }

    fn recuerda(&self, fuente: &str) -> bool {
        self.figuras.bloquear().contains_key(fuente) || self.cajas.bloquear().contains_key(fuente)
    }

    fn anticipar(&self, animaciones: &[String]) {
        let cajas = self.cajas.bloquear();
        let faltan: Vec<String> = animaciones
            .iter()
            .filter(|a| !cajas.contains_key(a.as_str()))
            .cloned()
            .collect();
        drop(cajas);
        if !faltan.is_empty() {
            self.dentro.anticipar(&faltan);
        }
    }
}

impl<I: Resaltador> Resaltador for Memoria<I> {
    fn resaltar(&self, codigo: &str, lenguaje: &str, tema: &str) -> Vec<Fragmento> {
        let clave = (codigo.to_string(), lenguaje.to_string(), tema.to_string());
        self.resaltados
            .dar(clave, || Ok(self.dentro.resaltar(codigo, lenguaje, tema)))
            .unwrap_or_default()
    }

    fn editor(&self, tema: &str) -> Option<ColoresEditor> {
        self.dentro.editor(tema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Falla con lo que contiene «rota» y cuenta cuántas veces se le llama.
    #[derive(Default)]
    struct Contador(AtomicU32, Mutex<Vec<String>>);

    impl Contador {
        fn sumar(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
            self.0.fetch_add(1, Ordering::Relaxed);
            if fuente.contains("rota") {
                return Err(ErrorComposicion("rota".into()));
            }
            Ok(Composicion::default())
        }
    }

    impl Incrustados for Contador {
        fn formula(&self, latex: &str, _bloque: bool, _tam: f64) -> Result<Composicion, ErrorComposicion> {
            self.sumar(latex)
        }

        fn figura(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
            self.sumar(fuente)
        }

        fn caja_de_animacion(&self, fuente: &str) -> Result<Caja, ErrorComposicion> {
            self.sumar(fuente).map(|c| c.escena.caja)
        }

        fn anticipar(&self, animaciones: &[String]) {
            self.1.lock().unwrap().extend_from_slice(animaciones);
        }
    }

    fn llamadas(m: &Memoria<Contador>) -> u32 {
        m.dentro.0.load(Ordering::Relaxed)
    }

    #[test]
    fn lo_compuesto_no_se_repite() {
        let m = Memoria::nueva(Contador::default());
        m.figura("a").unwrap();
        m.figura("a").unwrap();
        m.caja_de_animacion("a").unwrap();
        m.caja_de_animacion("a").unwrap();
        m.formula("a", true, 10.0).unwrap();
        m.formula("a", true, 10.0).unwrap();
        assert_eq!(llamadas(&m), 3);
    }

    #[test]
    fn la_misma_formula_en_otro_modo_o_tamano_es_otra() {
        let m = Memoria::nueva(Contador::default());
        m.formula("a", true, 10.0).unwrap();
        m.formula("a", false, 10.0).unwrap();
        m.formula("a", true, 12.0).unwrap();
        assert_eq!(llamadas(&m), 3);
    }

    #[test]
    fn solo_anticipa_lo_que_no_tiene() {
        let m = Memoria::nueva(Contador::default());
        m.caja_de_animacion("a").unwrap();
        m.anticipar(&["a".to_string(), "b".to_string()]);
        assert_eq!(*m.dentro.1.lock().unwrap(), ["b"]);
    }

    #[test]
    fn sabe_lo_que_ya_tiene_hecho() {
        let m = Memoria::nueva(Contador::default());
        assert!(!m.recuerda("a") && !m.recuerda("b"));
        m.figura("a").unwrap();
        m.caja_de_animacion("b").unwrap();
        assert!(m.recuerda("a") && m.recuerda("b"));
        assert_eq!(llamadas(&m), 2, "preguntar no compone");
    }

    #[test]
    fn un_error_tambien_se_recuerda() {
        let m = Memoria::nueva(Contador::default());
        assert!(m.figura("rota").is_err());
        assert!(m.figura("rota").is_err());
        assert_eq!(llamadas(&m), 1);
    }

    #[test]
    fn llena_olvida_y_sigue_funcionando() {
        let m = Memoria::nueva(Contador::default());
        for i in 0..MAX_RECUERDOS + 1 {
            m.figura(&format!("{i}")).unwrap();
        }
        assert_eq!(m.figuras.bloquear().len(), 1);
        m.figura(&format!("{MAX_RECUERDOS}")).unwrap();
        assert_eq!(llamadas(&m), MAX_RECUERDOS as u32 + 1, "lo último sigue recordado");
    }
}
