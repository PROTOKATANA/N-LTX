//! El adaptador de rinf: recibe las señales de Dart y las traduce a llamadas. No decide nada, y
//! nada de esto se bloquea: el trabajo está en los hilos del compositor y del taller.

use std::sync::Arc;
use std::time::Duration;

use pintor::{Taller, Vista, medidas};
use rinf::DartSignal;

use crate::avisos;
use crate::compositor::{Compositor, Encargo};
use crate::muestras::Muestras;
use crate::signals;

const CADA_MEDIDA: Duration = Duration::from_secs(1);

pub async fn escuchar(compositor: Compositor, taller: Arc<Taller>, muestras: Muestras) {
    let publicar = signals::PublicarDocumento::get_dart_signal_receiver();
    let ver = signals::VerDocumento::get_dart_signal_receiver();
    let pedir = signals::PedirMuestra::get_dart_signal_receiver();
    let mut reloj = tokio::time::interval(CADA_MEDIDA);
    loop {
        tokio::select! {
            Some(s) = publicar.recv() => compositor.encargar(Encargo {
                fuente: s.message.fuente,
                ancho: s.message.ancho,
            }),
            Some(s) = ver.recv() => taller.ver(Vista {
                ancho: s.message.ancho,
                alto: s.message.alto,
                escala: s.message.escala,
                y: s.message.y,
            }),
            Some(s) = pedir.recv() => avisos::enviar_muestra(muestras.leer(s.message.indice)),
            _ = reloj.tick() => avisos::enviar_medidas(medidas::leer()),
            else => return,
        }
    }
}
