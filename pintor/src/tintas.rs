//! Las tintas: lo que le dice al pixel shader cómo colorear.

use primitivas::Degradado;

use crate::geometria::Afin;
use crate::puente_lienzo::ffi::{Tinta, TipoTinta};
use crate::rampas::Rampas;

/// Las tintas que caben en la GPU.
pub const MAX_TINTAS: usize = 4096;

/// Los índices de las tintas fijas, que abren siempre la lista base.
pub const SOLIDA: u32 = 0;
pub const GLIFO: u32 = 1;
pub const TRAZO: u32 = 2;

fn sin_parametros(tipo: TipoTinta) -> Tinta {
    Tinta {
        tipo,
        fila: 0.0,
        radio: 0.0,
        reservado: 0.0,
        a: 0.0,
        b: 0.0,
        c: 0.0,
        d: 0.0,
        e: 0.0,
        f: 0.0,
        p: 0.0,
        q: 0.0,
        r: 0.0,
        s: 0.0,
    }
}

pub fn fijas() -> Vec<Tinta> {
    [TipoTinta::Solido, TipoTinta::Glifo, TipoTinta::Trazo]
        .map(sin_parametros)
        .to_vec()
}

/// `None` si `ctm` aplasta el degradado y no se puede invertir.
pub fn de_degradado(d: &Degradado, ctm: Afin, rampas: &mut Rampas) -> Option<Tinta> {
    let (matriz, paradas) = match d {
        Degradado::Lineal { matriz, paradas, .. } | Degradado::Radial { matriz, paradas, .. } => (matriz, paradas),
    };
    let g = ctm.por((*matriz).into()).inversa()?;
    let fila = rampas.fila(paradas);
    let (tipo, radio, p, q, r, s) = match d {
        Degradado::Lineal { desde, hasta, .. } => {
            let (ex, ey) = (hasta.x - desde.x, hasta.y - desde.y);
            let largo = (ex * ex + ey * ey).max(1e-12);
            (TipoTinta::Lineal, 0.0, desde.x, desde.y, ex / largo, ey / largo)
        }
        Degradado::Radial { radio, foco, .. } => (TipoTinta::Radial, *radio, foco.x, foco.y, 0.0, 0.0),
    };
    Some(Tinta {
        fila,
        radio: radio as f32,
        a: g.a as f32,
        b: g.b as f32,
        c: g.c as f32,
        d: g.d as f32,
        e: g.e as f32,
        f: g.f as f32,
        p: p as f32,
        q: q as f32,
        r: r as f32,
        s: s as f32,
        ..sin_parametros(tipo)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitivas::{Color, Matriz, Parada, Punto};

    fn degradado(a: f64) -> Degradado {
        let parada = |posicion, r| Parada {
            posicion,
            color: Color { r, g: 0.0, b: 0.0 },
        };
        Degradado::Lineal {
            desde: Punto { x: 10.0, y: 0.0 },
            hasta: Punto { x: 14.0, y: 0.0 },
            matriz: Matriz {
                a,
                b: 0.0,
                c: 0.0,
                d: a,
                e: 0.0,
                f: 0.0,
            },
            paradas: vec![parada(0.0, 0.0), parada(1.0, 1.0)],
        }
    }

    #[test]
    fn las_tintas_fijas_estan_donde_dicen_sus_indices() {
        let f = fijas();
        assert_eq!(f[SOLIDA as usize].tipo, TipoTinta::Solido);
        assert_eq!(f[GLIFO as usize].tipo, TipoTinta::Glifo);
        assert_eq!(f[TRAZO as usize].tipo, TipoTinta::Trazo);
    }

    #[test]
    fn un_degradado_lleva_de_pixeles_a_su_espacio() {
        let t = de_degradado(&degradado(1.0), Afin::escala(2.0), &mut Rampas::default()).unwrap();
        assert_eq!(t.tipo, TipoTinta::Lineal);
        // el píxel (24, 0) es el punto (12, 0): la mitad del degradado
        assert_eq!((t.a * 24.0 + t.e - t.p) * t.r, 0.5);
    }

    #[test]
    fn un_degradado_aplastado_no_se_puede_pintar() {
        assert!(de_degradado(&degradado(0.0), Afin::IDENTIDAD, &mut Rampas::default()).is_none());
    }
}
