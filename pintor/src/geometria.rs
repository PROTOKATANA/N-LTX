//! De `Segmento` a los trazados de lyon, con las transformaciones y los discontinuos.

use lyon::math::{Point, Vector, point};
use lyon::path::iterator::PathIterator;
use lyon::path::path::Builder;
use lyon::path::{Path, PathEvent};
use primitivas::{Discontinuo, Matriz, Punto, Segmento};

/// Transformación afín: un punto (x, y) va a (a·x + c·y + e, b·x + d·y + f).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Afin {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Afin {
    pub const IDENTIDAD: Afin = Afin {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub fn escala(s: f64) -> Afin {
        Afin {
            a: s,
            d: s,
            ..Afin::IDENTIDAD
        }
    }

    pub fn traslacion(x: f64, y: f64) -> Afin {
        Afin {
            e: x,
            f: y,
            ..Afin::IDENTIDAD
        }
    }

    /// Primero `o` y después `self`.
    pub fn por(self, o: Afin) -> Afin {
        Afin {
            a: self.a * o.a + self.c * o.b,
            b: self.b * o.a + self.d * o.b,
            c: self.a * o.c + self.c * o.d,
            d: self.b * o.c + self.d * o.d,
            e: self.a * o.e + self.c * o.f + self.e,
            f: self.b * o.e + self.d * o.f + self.f,
        }
    }

    pub fn aplicar(self, p: Punto) -> Punto {
        Punto {
            x: self.a * p.x + self.c * p.y + self.e,
            y: self.b * p.x + self.d * p.y + self.f,
        }
    }

    /// Cuánto agranda de media: la raíz del área de un cuadrado unidad.
    pub fn factor(self) -> f64 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }

    pub fn inversa(self) -> Option<Afin> {
        let det = self.a * self.d - self.b * self.c;
        if det == 0.0 || !det.is_finite() {
            return None;
        }
        Some(Afin {
            a: self.d / det,
            b: -self.b / det,
            c: -self.c / det,
            d: self.a / det,
            e: (self.c * self.f - self.d * self.e) / det,
            f: (self.b * self.e - self.a * self.f) / det,
        })
    }
}

impl From<Matriz> for Afin {
    fn from(m: Matriz) -> Afin {
        Afin {
            a: m.a,
            b: m.b,
            c: m.c,
            d: m.d,
            e: m.e,
            f: m.f,
        }
    }
}

/// `None` si tiene coordenadas que no son números: lyon no las admite.
pub fn ruta(segmentos: &[Segmento], m: Afin) -> Option<Path> {
    let mut b = Path::builder();
    let mut abierta = false;
    let mut inicio = point(0.0, 0.0);
    let p = |q: Punto| {
        let r = m.aplicar(q);
        (r.x.is_finite() && r.y.is_finite()).then_some(point(r.x as f32, r.y as f32))
    };
    for s in segmentos {
        match *s {
            Segmento::Mover(q) => {
                if abierta {
                    b.end(false);
                }
                inicio = p(q)?;
                b.begin(inicio);
                abierta = true;
            }
            Segmento::Linea(q) => {
                let q = p(q)?;
                seguir(&mut b, &mut abierta, inicio);
                b.line_to(q);
            }
            Segmento::Curva(x, y, z) => {
                let (x, y, z) = (p(x)?, p(y)?, p(z)?);
                seguir(&mut b, &mut abierta, inicio);
                b.cubic_bezier_to(x, y, z);
            }
            Segmento::Cerrar => {
                if abierta {
                    b.end(true);
                    abierta = false;
                }
            }
        }
    }
    if abierta {
        b.end(false);
    }
    Some(b.build())
}

/// Tras cerrar, el siguiente tramo sale del inicio del subtrazado.
fn seguir(b: &mut Builder, abierta: &mut bool, inicio: Point) {
    if !*abierta {
        b.begin(inicio);
        *abierta = true;
    }
}

/// La razón de inglete del codo más agudo: 1/sen(ángulo interior / 2), como en PDF y SVG. Un trazado
/// sin codos da 1 y uno que da media vuelta, infinito.
pub fn razon_de_inglete(ruta: &Path, tolerancia: f32) -> f32 {
    let mut mayor = 1.0f32;
    let mut primera = None;
    let mut anterior: Option<Vector> = None;
    let mut codo = |d: Vector, a: Option<Vector>| {
        if let Some(a) = a {
            let coseno_medio = ((1.0 + a.dot(d)) / 2.0).max(0.0).sqrt();
            mayor = mayor.max(if coseno_medio > 0.0 {
                1.0 / coseno_medio
            } else {
                f32::INFINITY
            });
        }
    };
    for e in ruta.iter().flattened(tolerancia) {
        match e {
            PathEvent::Begin { .. } => (primera, anterior) = (None, None),
            PathEvent::Line { from, to } => {
                if let Some(d) = direccion(from, to) {
                    codo(d, anterior);
                    primera = primera.or(Some(d));
                    anterior = Some(d);
                }
            }
            PathEvent::End {
                last,
                first,
                close: true,
            } => {
                if let Some(d) = direccion(last, first) {
                    codo(d, anterior);
                    anterior = Some(d);
                }
                if let Some(p) = primera {
                    codo(p, anterior);
                }
            }
            _ => {}
        }
    }
    mayor
}

fn direccion(desde: Point, hasta: Point) -> Option<Vector> {
    let eje = hasta - desde;
    (eje.square_length() > 0.0).then(|| eje.normalize())
}

/// Corta `ruta` en tramos. Cada subtrazado empieza el dibujo desde la fase, igual que en PDF.
pub fn discontinua(ruta: &Path, d: &Discontinuo, tolerancia: f32) -> Path {
    let tramos: Vec<f32> = d.tramos.iter().map(|t| t.max(0.0) as f32).collect();
    let ciclo = if !tramos.len().is_multiple_of(2) {
        [&tramos[..], &tramos[..]].concat()
    } else {
        tramos
    };
    if ciclo.iter().sum::<f32>() <= 0.0 {
        return ruta.clone();
    }
    let mut pluma = Pluma {
        b: Path::builder(),
        abierta: false,
        ciclo: &ciclo,
        fase: d.fase as f32,
        i: 0,
        resto: 0.0,
    };
    for e in ruta.iter().flattened(tolerancia) {
        match e {
            PathEvent::Begin { .. } => pluma.empezar(),
            PathEvent::Line { from, to } => pluma.avanzar(from, to),
            PathEvent::End { last, first, close } => {
                if close {
                    pluma.avanzar(last, first);
                }
                pluma.levantar();
            }
            _ => {}
        }
    }
    pluma.b.build()
}

struct Pluma<'a> {
    b: Builder,
    abierta: bool,
    ciclo: &'a [f32],
    fase: f32,
    i: usize,
    resto: f32,
}

impl Pluma<'_> {
    fn empezar(&mut self) {
        let total: f32 = self.ciclo.iter().sum();
        let mut f = self.fase.rem_euclid(total);
        self.i = 0;
        while f > 0.0 && f >= self.ciclo[self.i] {
            f -= self.ciclo[self.i];
            self.i = (self.i + 1) % self.ciclo.len();
        }
        self.resto = self.ciclo[self.i] - f;
    }

    fn pinta(&self) -> bool {
        self.i.is_multiple_of(2)
    }

    fn levantar(&mut self) {
        if self.abierta {
            self.b.end(false);
            self.abierta = false;
        }
    }

    fn avanzar(&mut self, desde: Point, hasta: Point) {
        let eje = hasta - desde;
        let largo = eje.length();
        if largo <= 0.0 {
            return;
        }
        let direccion = eje / largo;
        let mut hecho = 0.0;
        while self.resto <= largo - hecho {
            let corte = desde + direccion * (hecho + self.resto);
            if self.pinta() {
                if !self.abierta {
                    self.b.begin(desde + direccion * hecho);
                }
                // un tramo de largo cero con remate redondo es un punto
                let fin = if self.resto > 0.0 {
                    corte
                } else {
                    corte + direccion * 0.01
                };
                self.b.line_to(fin);
                self.b.end(false);
                self.abierta = false;
            }
            hecho += self.resto;
            self.i = (self.i + 1) % self.ciclo.len();
            self.resto = self.ciclo[self.i];
        }
        if self.pinta() {
            if !self.abierta {
                self.b.begin(desde + direccion * hecho);
                self.abierta = true;
            }
            self.b.line_to(hasta);
        }
        self.resto -= largo - hecho;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lyon::path::iterator::PathIterator;

    const P: fn(f64, f64) -> Punto = |x, y| Punto { x, y };

    fn largo(r: &Path) -> f32 {
        r.iter()
            .flattened(0.01)
            .map(|e| match e {
                PathEvent::Line { from, to } => (to - from).length(),
                PathEvent::End {
                    last,
                    first,
                    close: true,
                } => (first - last).length(),
                _ => 0.0,
            })
            .sum()
    }

    fn subtrazados(r: &Path) -> usize {
        r.iter().filter(|e| matches!(e, PathEvent::Begin { .. })).count()
    }

    fn razon(puntos: &[(f64, f64)], cerrar: bool) -> f32 {
        let mut s: Vec<Segmento> = puntos.iter().map(|&(x, y)| Segmento::Linea(P(x, y))).collect();
        s[0] = Segmento::Mover(P(puntos[0].0, puntos[0].1));
        if cerrar {
            s.push(Segmento::Cerrar);
        }
        razon_de_inglete(&ruta(&s, Afin::IDENTIDAD).unwrap(), 0.01)
    }

    #[test]
    fn la_razon_de_inglete_sale_del_codo_mas_agudo() {
        assert_eq!(razon(&[(0.0, 0.0), (5.0, 0.0), (9.0, 0.0)], false), 1.0, "recto");
        assert!((razon(&[(0.0, 0.0), (5.0, 0.0), (5.0, 5.0)], false) - 2f32.sqrt()).abs() < 1e-4);
        assert_eq!(
            razon(&[(0.0, 0.0), (5.0, 0.0), (1.0, 0.0)], false),
            f32::INFINITY,
            "media vuelta"
        );
    }

    #[test]
    fn un_trazado_cerrado_cuenta_el_codo_del_cierre() {
        // el triángulo solo es agudo en el vértice donde empieza y acaba
        let t = [(0.0, 0.0), (10.0, 1.0), (10.0, -1.0)];
        assert!(razon(&t, false) < 2.0);
        assert!(razon(&t, true) > 8.0);
    }

    #[test]
    fn un_tramo_de_largo_cero_no_es_un_codo() {
        assert_eq!(razon(&[(0.0, 0.0), (5.0, 0.0), (5.0, 0.0), (9.0, 0.0)], false), 1.0);
    }

    #[test]
    fn por_aplica_primero_la_derecha() {
        let m = Afin::traslacion(10.0, 0.0).por(Afin::escala(2.0));
        assert_eq!(m.aplicar(P(1.0, 1.0)), P(12.0, 2.0));
    }

    #[test]
    fn la_inversa_deshace() {
        let m = Afin {
            a: 2.0,
            b: 1.0,
            c: -3.0,
            d: 4.0,
            e: 5.0,
            f: 6.0,
        };
        let q = m.inversa().unwrap().aplicar(m.aplicar(P(7.0, -2.0)));
        assert!((q.x - 7.0).abs() < 1e-9 && (q.y + 2.0).abs() < 1e-9, "{q:?}");
        assert_eq!(Afin::escala(0.0).inversa(), None);
    }

    #[test]
    fn tras_cerrar_se_sigue_desde_el_inicio() {
        let r = ruta(
            &[
                Segmento::Mover(P(1.0, 1.0)),
                Segmento::Linea(P(2.0, 1.0)),
                Segmento::Cerrar,
                Segmento::Linea(P(1.0, 5.0)),
            ],
            Afin::IDENTIDAD,
        )
        .unwrap();
        assert_eq!(subtrazados(&r), 2);
        assert!((largo(&r) - 6.0).abs() < 1e-3, "{}", largo(&r));
    }

    #[test]
    fn lo_que_no_es_un_numero_no_llega_a_lyon() {
        assert!(ruta(&[Segmento::Mover(P(f64::NAN, 0.0))], Afin::IDENTIDAD).is_none());
    }

    #[test]
    fn los_tramos_cubren_lo_que_dicen() {
        let r = ruta(
            &[Segmento::Mover(P(0.0, 0.0)), Segmento::Linea(P(10.0, 0.0))],
            Afin::IDENTIDAD,
        )
        .unwrap();
        let d = discontinua(
            &r,
            &Discontinuo {
                tramos: vec![2.0, 1.0],
                fase: 0.0,
            },
            0.01,
        );
        assert_eq!(subtrazados(&d), 4);
        assert!((largo(&d) - 7.0).abs() < 1e-3, "{}", largo(&d));
    }

    #[test]
    fn la_fase_mueve_el_dibujo() {
        let r = ruta(
            &[Segmento::Mover(P(0.0, 0.0)), Segmento::Linea(P(3.0, 0.0))],
            Afin::IDENTIDAD,
        )
        .unwrap();
        let d = discontinua(
            &r,
            &Discontinuo {
                tramos: vec![2.0, 1.0],
                fase: 2.0,
            },
            0.01,
        );
        // empieza en el hueco: 1 sin pintar y 2 pintados
        assert!((largo(&d) - 2.0).abs() < 1e-3, "{}", largo(&d));
    }

    #[test]
    fn un_tramo_impar_se_repite_alternando() {
        let r = ruta(
            &[Segmento::Mover(P(0.0, 0.0)), Segmento::Linea(P(4.0, 0.0))],
            Afin::IDENTIDAD,
        )
        .unwrap();
        let d = discontinua(
            &r,
            &Discontinuo {
                tramos: vec![1.0],
                fase: 0.0,
            },
            0.01,
        );
        assert!((largo(&d) - 2.0).abs() < 1e-3, "{}", largo(&d));
    }

    #[test]
    fn un_tramo_de_largo_cero_deja_un_punto() {
        let r = ruta(
            &[Segmento::Mover(P(0.0, 0.0)), Segmento::Linea(P(5.0, 0.0))],
            Afin::IDENTIDAD,
        )
        .unwrap();
        let d = discontinua(
            &r,
            &Discontinuo {
                tramos: vec![0.0, 2.0],
                fase: 0.0,
            },
            0.01,
        );
        assert_eq!(subtrazados(&d), 3);
    }

    #[test]
    fn un_dibujo_sin_largo_no_corta_nada() {
        let r = ruta(
            &[Segmento::Mover(P(0.0, 0.0)), Segmento::Linea(P(5.0, 0.0))],
            Afin::IDENTIDAD,
        )
        .unwrap();
        let d = discontinua(
            &r,
            &Discontinuo {
                tramos: vec![0.0, 0.0],
                fase: 0.0,
            },
            0.01,
        );
        assert_eq!(subtrazados(&d), 1);
    }
}
