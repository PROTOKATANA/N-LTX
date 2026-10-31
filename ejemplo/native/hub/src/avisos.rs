//! Lo que se cuenta a Dart, con señales de rinf.

use pintor::medidas::Medidas;
use rinf::RustSignal;

use crate::compositor::{Avisos, Resultado};
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
