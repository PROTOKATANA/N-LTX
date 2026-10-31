//! Lo común a las texturas que viven aquí y se copian a la GPU: subir solo las filas que cambian.

/// Las filas que hay que subir.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sucio {
    pub y: u32,
    pub bytes: Vec<u8>,
}

/// El rango de filas que cambió desde la última subida.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Filas(Option<(u32, u32)>);

impl Filas {
    pub(crate) fn tocar(&mut self, desde: u32, hasta: u32) {
        self.0 = Some(self.0.map_or((desde, hasta), |(a, b)| (a.min(desde), b.max(hasta))));
    }

    pub(crate) fn tomar(&mut self, pixeles: &[u8], largo_fila: usize) -> Sucio {
        match self.0.take() {
            Some((a, b)) => Sucio {
                y: a,
                bytes: pixeles[a as usize * largo_fila..b as usize * largo_fila].to_vec(),
            },
            None => Sucio::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_sube_el_rango_que_cubre_lo_tocado_y_una_sola_vez() {
        let pixeles: Vec<u8> = (0..12).collect();
        let mut f = Filas::default();
        f.tocar(2, 3);
        f.tocar(0, 1);
        let s = f.tomar(&pixeles, 4);
        assert_eq!((s.y, s.bytes.len()), (0, 12));
        assert_eq!(f.tomar(&pixeles, 4), Sucio::default());
    }
}
