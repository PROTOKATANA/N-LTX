//! Lo que se cuenta a Dart, con señales de rinf.

use pintor::medidas::Medidas;
use rinf::RustSignal;

use crate::compositor::{Avisos, Resultado};
use crate::muestras::Muestra;
use crate::signals;

pub struct AvisosRinf;

impl Avisos for AvisosRinf {
    fn publicado(&self, r: Resultado) {
        signals::DocumentoPublicado {
            alto: r.alto,
            avisos: r.avisos,
            error: r.error,
        }
        .send_signal_to_dart();
    }
}

pub fn enviar_medidas(m: Medidas) {
    signals::MedidasPintor {
        vertices: m.vertices,
        ms_taller: m.ms_taller,
        ms_carril: m.ms_carril,
        elementos: m.elementos,
        fps_masiva: m.fps_masiva,
        ms_masiva: m.ms_masiva,
    }
    .send_signal_to_dart();
}

pub fn enviar_muestra(m: Muestra) {
    signals::MuestraLeida {
        indice: m.indice,
        total: m.total,
        nombre: m.nombre,
        fuente: m.fuente,
    }
    .send_signal_to_dart();
}
