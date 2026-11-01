//! Lo que se cuenta a Dart, con señales de rinf.

use pintor::medidas::Medidas;
use rinf::RustSignal;
use visor::{Avisos, Resultado};

use crate::signals;

pub struct AvisosRinf;

impl Avisos for AvisosRinf {
    fn publicado(&self, r: Resultado) {
        signals::DocumentoPublicado {
            alto: r.alto,
            avisos: r.avisos,
            error: r.error,
            muestras: r.muestras,
        }
        .send_signal_to_dart();
    }
}

pub fn enviar_copiado(texto: String) {
    signals::TextoCopiado { texto }.send_signal_to_dart();
}

pub fn enviar_medidas(m: Medidas) {
    signals::MedidasPintor {
        vertices: m.vertices,
        ms_taller: m.ms_taller,
        ms_carril: m.ms_carril,
    }
    .send_signal_to_dart();
}
