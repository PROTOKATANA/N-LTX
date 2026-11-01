//! Lo que hace el maquetador por quien lo sigue: entre dos bloques del documento, enseñar lo colocado
//! o dejarlo, y adelantar lo que más va a tardar.

use primitivas::{Composicion, Escena};

use crate::lector::Bloque;

use super::Maquetador;

impl Maquetador<'_> {
    pub(super) fn entre_bloques(&mut self, siguiente: &Bloque) {
        self.parado = self.seguimiento.parar();
        if self.parado {
            return;
        }
        let lento = self.puede_tardar(siguiente);
        if self.seguimiento.quiere_tramo(self.y, lento) {
            let parcial = self.lo_colocado();
            self.seguimiento.tramo(parcial);
        }
    }

    /// Lo que compone otro y aún no está hecho: en K1, en frío, 630 de los 790 ms que tarda.
    fn puede_tardar(&self, bloque: &Bloque) -> bool {
        let (Bloque::Figura(fuente) | Bloque::Animacion(fuente)) = bloque else {
            return false;
        };
        !self.piezas.incrustados.recuerda(&format!("{}{fuente}", self.preambulo))
    }

    /// Medir las animaciones es lo que más tarda: mientras se coloca lo demás, se van adelantando.
    pub(super) fn anticipar(&self, bloques: &[Bloque]) {
        let animaciones: Vec<String> = bloques
            .iter()
            .filter_map(|b| match b {
                Bloque::Animacion(fuente) => Some(format!("{}{fuente}", self.preambulo)),
                _ => None,
            })
            .collect();
        if !animaciones.is_empty() {
            self.piezas.incrustados.anticipar(&animaciones);
        }
    }

    /// Una copia de lo colocado: `cerrar` da lo mismo cuando ya no hay más.
    fn lo_colocado(&self) -> Composicion {
        Composicion {
            escena: Escena {
                caja: self.caja(),
                primitivas: self.primitivas.clone(),
                contornos: self.contornos.clone(),
                zonas: self.zonas.clone(),
                anclas: self.anclas.clone(),
                ..Escena::default()
            },
            avisos: self.avisos.clone(),
        }
    }
}
