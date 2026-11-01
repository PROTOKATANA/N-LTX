//! Mirar un documento mientras se compone: el contrato con quien enseña lo que va saliendo.

use primitivas::Composicion;

/// Quien va mirando un documento mientras se compone. Se le pregunta entre dos bloques del documento:
/// ahí no queda nada abierto y lo colocado es un principio exacto del resultado final.
pub trait Seguimiento {
    /// Si hay que dejarlo: lo que está saliendo ya no hace falta.
    fn parar(&mut self) -> bool;
    /// Si hay que enseñar ya lo colocado, que llega hasta `alto` bp. `lento`: lo que viene puede
    /// tardar. Copiarlo cuesta, así que no se copia sin que lo pida.
    fn quiere_tramo(&mut self, alto: f64, lento: bool) -> bool;
    /// Lo colocado hasta aquí.
    fn tramo(&mut self, parcial: Composicion);
}

/// Sin nadie mirando: el documento sale entero de una vez.
pub struct SinSeguimiento;

impl Seguimiento for SinSeguimiento {
    fn parar(&mut self) -> bool {
        false
    }

    fn quiere_tramo(&mut self, _alto: f64, _lento: bool) -> bool {
        false
    }

    fn tramo(&mut self, _parcial: Composicion) {}
}
