//! El adaptador de rinf: recibe las señales de Dart y las traduce a llamadas. No decide nada, y
//! nada de esto se bloquea: el trabajo está en los hilos del compositor y del taller.

use std::sync::Arc;
use std::time::Duration;

use pintor::{Taller, Vista, medidas};
use rinf::DartSignal;
use visor::{Color, Compositor, Encargo, Seleccion};

use crate::avisos;
use crate::signals;

const CADA_MEDIDA: Duration = Duration::from_secs(1);

pub async fn escuchar(compositor: Compositor, taller: Arc<Taller>, seleccion: Arc<Seleccion>) {
    let publicar = signals::PublicarDocumento::get_dart_signal_receiver();
    let fondo = signals::PonerFondo::get_dart_signal_receiver();
    let ver = signals::VerDocumento::get_dart_signal_receiver();
    let arrastrar = signals::SeleccionarTexto::get_dart_signal_receiver();
    let palabra = signals::SeleccionarPalabra::get_dart_signal_receiver();
    let todo = signals::SeleccionarTodo::get_dart_signal_receiver();
    let borrar = signals::BorrarSeleccion::get_dart_signal_receiver();
    let copiar = signals::CopiarSeleccion::get_dart_signal_receiver();
    let mut reloj = tokio::time::interval(CADA_MEDIDA);
    loop {
        tokio::select! {
            Some(s) = publicar.recv() => compositor.encargar(Encargo {
                fuente: s.message.fuente,
                ancho: s.message.ancho,
                con_muestras: s.message.con_muestras,
                tinta: Color::de_rgb(s.message.tinta),
            }),
            Some(s) = fondo.recv() => taller.fondo(Color::de_rgb(s.message.color)),
            Some(s) = ver.recv() => taller.ver(Vista {
                ancho: s.message.ancho,
                alto: s.message.alto,
                escala: s.message.escala,
                y: s.message.y,
            }),
            Some(s) = arrastrar.recv() => {
                let m = s.message;
                seleccion.arrastrar((m.ax, m.ay), (m.bx, m.by));
            }
            Some(s) = palabra.recv() => seleccion.palabra(s.message.x, s.message.y),
            Some(_) = todo.recv() => seleccion.todo(),
            Some(_) = borrar.recv() => seleccion.borrar(),
            Some(_) = copiar.recv() => avisos::enviar_copiado(seleccion.copiar()),
            _ = reloj.tick() => avisos::enviar_medidas(medidas::leer()),
            else => return,
        }
    }
}
