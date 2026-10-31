//! Animaciones por parámetro: la misma figura, recompuesta con otro valor en cada fotograma.
//!
//! Es lo que hace la macro `\engine{ángulo}` de las animaciones de tikz.net, en tiempo real:
//!
//! ```latex
//! \begin{animacion}[t=0:360, duracion=4, bucle]
//!   \begin{tikzpicture}
//!     \draw (0,0) -- (\t:1);
//!   \end{tikzpicture}
//! \end{animacion}
//! ```
//!
//! Recomponer cuesta unos 10 µs por elemento: sirve hasta el millar de elementos. Lo masivo no
//! puede pasar por aquí.

use primitivas::{Caja, Capa, Composicion, Compositor, ErrorComposicion, Matriz, Primitiva, Punto};

/// Valores del recorrido con los que se calcula la caja fija.
const MUESTRAS: usize = 24;

#[derive(Debug, Clone, PartialEq)]
pub struct Animacion {
    /// La figura, con el parámetro como macro (`\t`).
    pub fuente: String,
    pub parametro: String,
    pub desde: f64,
    pub hasta: f64,
    /// Segundos que dura un recorrido.
    pub duracion: f64,
    pub bucle: bool,
}

impl Animacion {
    /// Lee un entorno `animacion`, con lo que lleve delante (el preámbulo de la figura).
    pub fn leer(texto: &str) -> Result<Animacion, ErrorComposicion> {
        const INICIO: &str = "\\begin{animacion}";
        const FIN: &str = "\\end{animacion}";
        let inicio = texto
            .find(INICIO)
            .ok_or_else(|| ErrorComposicion("no hay ningún entorno animacion".into()))?;
        let tras = &texto[inicio + INICIO.len()..];
        let fin = tras
            .find(FIN)
            .ok_or_else(|| ErrorComposicion("animación sin cerrar".into()))?;
        let cuerpo = tras[..fin].trim_start();
        let (opciones, figura) = match cuerpo.strip_prefix('[').and_then(|r| r.split_once(']')) {
            Some((opciones, figura)) => (opciones, figura),
            None => ("", cuerpo),
        };
        let mut a = Animacion {
            fuente: format!("{}{figura}", &texto[..inicio]),
            parametro: "t".into(),
            desde: 0.0,
            hasta: 1.0,
            duracion: 4.0,
            bucle: true,
        };
        for opcion in opciones.split(',').map(str::trim).filter(|o| !o.is_empty()) {
            a.opcion(opcion)?;
        }
        Ok(a)
    }

    fn opcion(&mut self, opcion: &str) -> Result<(), ErrorComposicion> {
        let invalida = || ErrorComposicion(format!("opción de animación inválida: {opcion}"));
        let (clave, valor) = opcion
            .split_once('=')
            .map_or((opcion, ""), |(c, v)| (c.trim(), v.trim()));
        match clave {
            "duracion" => self.duracion = valor.parse().map_err(|_| invalida())?,
            "bucle" => self.bucle = true,
            "unavez" => self.bucle = false,
            parametro => {
                let (desde, hasta) = valor.split_once(':').ok_or_else(invalida)?;
                self.parametro = parametro.to_string();
                self.desde = desde.trim().parse().map_err(|_| invalida())?;
                self.hasta = hasta.trim().parse().map_err(|_| invalida())?;
            }
        }
        if self.duracion > 0.0 { Ok(()) } else { Err(invalida()) }
    }

    /// El valor del parámetro a los `segundos` del inicio.
    pub fn valor(&self, segundos: f64) -> f64 {
        let fase = segundos / self.duracion;
        let fase = if self.bucle {
            fase.rem_euclid(1.0)
        } else {
            fase.clamp(0.0, 1.0)
        };
        self.desde + (self.hasta - self.desde) * fase
    }

    fn con_valor(&self, valor: f64) -> String {
        // cuatro decimales, sin ceros de relleno: el valor puede verse en una etiqueta
        let valor = (valor * 1e4).round() / 1e4;
        format!("\\def\\{}{{{valor}}}\n{}", self.parametro, self.fuente)
    }
}

/// Reproduce una animación con cualquier compositor, en una caja que no cambia.
pub struct Reproductor<C> {
    animacion: Animacion,
    compositor: C,
    caja: Caja,
}

impl<C: Compositor> Reproductor<C> {
    pub fn nuevo(fuente: &str, compositor: C) -> Result<Self, ErrorComposicion> {
        let animacion = Animacion::leer(fuente)?;
        let caja = caja_del_recorrido(&animacion, &compositor)?;
        Ok(Reproductor {
            animacion,
            compositor,
            caja,
        })
    }

    pub fn caja(&self) -> Caja {
        self.caja
    }

    pub fn animacion(&self) -> &Animacion {
        &self.animacion
    }

    /// El fotograma de los `segundos`, colocado en la caja fija.
    pub fn fotograma(&self, segundos: f64) -> Result<Composicion, ErrorComposicion> {
        let valor = self.animacion.valor(segundos);
        let mut c = self.compositor.componer(&self.animacion.con_valor(valor))?;
        let origen = c.escena.origen;
        let primitivas = std::mem::take(&mut c.escena.primitivas);
        let mut envuelta = Vec::with_capacity(primitivas.len() + 2);
        envuelta.push(Primitiva::Abrir(Capa::Transformacion(traslacion(origen))));
        envuelta.extend(primitivas);
        envuelta.push(Primitiva::Cerrar);
        c.escena.primitivas = envuelta;
        c.escena.caja = self.caja;
        c.escena.origen = Punto::default();
        Ok(c)
    }
}

/// Lleva el origen del dibujo a (0, 0): en esas coordenadas, los fotogramas coinciden.
fn traslacion(origen: Punto) -> Matriz {
    Matriz {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: -origen.x,
        f: -origen.y,
    }
}

/// La caja que contiene el recorrido entero, medida desde el origen del dibujo.
fn caja_del_recorrido(animacion: &Animacion, compositor: &dyn Compositor) -> Result<Caja, ErrorComposicion> {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for n in 0..=MUESTRAS {
        let valor = animacion.desde + (animacion.hasta - animacion.desde) * n as f64 / MUESTRAS as f64;
        let e = compositor.componer(&animacion.con_valor(valor))?.escena;
        let (x, y) = (e.caja.x - e.origen.x, e.caja.y - e.origen.y);
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x + e.caja.ancho);
        y1 = y1.max(y + e.caja.alto);
    }
    Ok(Caja {
        x: x0,
        y: y0,
        ancho: x1 - x0,
        alto: y1 - y0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use primitivas::Escena;

    /// Dibuja un punto en x = t: como `tikra`, deja el dibujo en su esquina y anota el origen.
    struct Falso;

    impl Compositor for Falso {
        fn componer(&self, fuente: &str) -> Result<Composicion, ErrorComposicion> {
            let t: f64 = fuente
                .split(['{', '}'])
                .nth(1)
                .and_then(|v| v.parse().ok())
                .ok_or_else(|| ErrorComposicion(format!("sin valor: {fuente}")))?;
            Ok(Composicion {
                escena: Escena {
                    caja: Caja {
                        x: 0.0,
                        y: 0.0,
                        ancho: 10.0,
                        alto: 10.0,
                    },
                    origen: Punto { x: -t, y: 0.0 },
                    ..Escena::default()
                },
                avisos: Vec::new(),
            })
        }
    }

    const FUENTE: &str = r"\begin{animacion}[t=0:100, duracion=2]\draw;\end{animacion}";

    #[test]
    fn lee_sus_opciones() {
        let a =
            Animacion::leer(r"\usepackage{x}\begin{animacion}[a=-1:2.5, duracion=8, unavez]F\end{animacion}").unwrap();
        assert_eq!(a.parametro, "a");
        assert_eq!((a.desde, a.hasta, a.duracion), (-1.0, 2.5, 8.0));
        assert!(!a.bucle);
        assert_eq!(a.fuente, r"\usepackage{x}F");
    }

    #[test]
    fn sin_opciones_usa_las_de_siempre() {
        let a = Animacion::leer(r"\begin{animacion}F\end{animacion}").unwrap();
        assert_eq!((a.parametro.as_str(), a.desde, a.hasta), ("t", 0.0, 1.0));
        assert!(a.bucle);
    }

    #[test]
    fn rechaza_lo_que_no_se_puede_reproducir() {
        assert!(Animacion::leer("nada").is_err());
        assert!(Animacion::leer(r"\begin{animacion}F").is_err());
        assert!(Animacion::leer(r"\begin{animacion}[duracion=0]F\end{animacion}").is_err());
        assert!(Animacion::leer(r"\begin{animacion}[t=a:b]F\end{animacion}").is_err());
    }

    #[test]
    fn el_bucle_vuelve_a_empezar_y_unavez_se_queda_al_final() {
        let bucle = Animacion::leer(FUENTE).unwrap();
        assert_eq!(bucle.valor(0.5), 25.0);
        assert_eq!(bucle.valor(2.5), 25.0);
        let unavez = Animacion::leer(r"\begin{animacion}[t=0:100, duracion=2, unavez]F\end{animacion}").unwrap();
        assert_eq!(unavez.valor(9.0), 100.0);
    }

    #[test]
    fn el_valor_se_escribe_sin_ceros_de_relleno() {
        let a = Animacion::leer(FUENTE).unwrap();
        assert!(a.con_valor(60.0).starts_with(r"\def\t{60}"), "{}", a.con_valor(60.0));
        assert!(a.con_valor(0.12345).starts_with(r"\def\t{0.1235}"));
    }

    #[test]
    fn la_caja_cubre_todo_el_recorrido() {
        let r = Reproductor::nuevo(FUENTE, Falso).unwrap();
        // el punto va de 0 a 100, y la caja de cada fotograma mide 10
        assert!((r.caja().ancho - 110.0).abs() < 1e-9, "{:?}", r.caja());
        assert_eq!(r.caja().alto, 10.0);
    }

    /// Sin restar el origen, la figura temblaría al cambiar su caja de un fotograma a otro.
    #[test]
    fn cada_fotograma_se_coloca_desde_el_origen_del_dibujo() {
        let r = Reproductor::nuevo(FUENTE, Falso).unwrap();
        let f = r.fotograma(1.0).unwrap().escena;
        assert_eq!(f.caja, r.caja());
        let Primitiva::Abrir(Capa::Transformacion(m)) = f.primitivas[0] else {
            panic!("{:?}", f.primitivas)
        };
        assert_eq!(m.e, 50.0);
        assert!(matches!(f.primitivas.last(), Some(Primitiva::Cerrar)));
    }
}
