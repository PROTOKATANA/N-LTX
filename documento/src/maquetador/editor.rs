//! Un bloque de código como un editor: un panel con pestaña, números de línea, guías de sangría y
//! los colores de su tema. El panel sale del propio tema: cambiarlo cambia el editor entero.

use cosmic_text::Align;
use primitivas::{Caja, Color, Discontinuo, Pintura, Primitiva, Punto, Relleno, Segmento, Trazo};

use super::texto::{Hueco, Letra};
use super::{Columna, Maquetador, SEPARACION};
use crate::ColoresEditor;
use crate::lector::{Estilo, Tramo};
use crate::tema::{EstiloCodigo, Opciones};

/// El cuerpo del código frente al del texto.
const TAM_CODIGO: f64 = 0.84;
/// Alto de cada línea, en em: un editor deja aire entre ellas.
const ALTO_DE_LINEA: f64 = 1.6;
/// Radio de las esquinas del panel, en bp.
const RADIO: f64 = 3.0;
/// Lo que se separa de cada borde, en em.
const MARGEN: f64 = 1.0;
/// Alto de la barra de la pestaña, en em.
const ALTO_BARRA: f64 = 2.3;
/// El color de la barra frente al del panel, en un tema oscuro y en uno claro: el mismo factor
/// apenas se nota en el primero y ensucia el segundo.
const SOMBRA_BARRA: (f64, f64) = (0.82, 0.94);
/// Una guía de sangría: su grosor y sus trazos, en bp.
const GROSOR_GUIA: f64 = 0.6;
const TRAZOS_GUIA: [f64; 2] = [2.0, 2.0];
/// Cuánto se acerca el color de una guía al de los números, desde el del panel.
const PESO_GUIA: f64 = 0.8;
/// Los espacios de un tabulador.
const TABULADOR: &str = "    ";
/// Dónde está la curva que aproxima un cuarto de círculo.
const KAPPA: f64 = 0.5523;

/// El contorno de `c` con cada esquina redondeada a su radio: arriba a la izquierda, arriba a la
/// derecha, abajo a la derecha y abajo a la izquierda.
fn redondeado(c: Caja, radios: [f64; 4], color: Color) -> Primitiva {
    let p = |x, y| Punto { x, y };
    let (x0, y0, x1, y1) = (c.x, c.y, c.x + c.ancho, c.y + c.alto);
    let [a, b, d, e] = radios;
    let k = |r: f64| r * (1.0 - KAPPA);
    Primitiva::Trazado {
        segmentos: vec![
            Segmento::Mover(p(x0 + a, y0)),
            Segmento::Linea(p(x1 - b, y0)),
            Segmento::Curva(p(x1 - k(b), y0), p(x1, y0 + k(b)), p(x1, y0 + b)),
            Segmento::Linea(p(x1, y1 - d)),
            Segmento::Curva(p(x1, y1 - k(d)), p(x1 - k(d), y1), p(x1 - d, y1)),
            Segmento::Linea(p(x0 + e, y1)),
            Segmento::Curva(p(x0 + k(e), y1), p(x0, y1 - k(e)), p(x0, y1 - e)),
            Segmento::Linea(p(x0, y0 + a)),
            Segmento::Curva(p(x0, y0 + k(a)), p(x0 + k(a), y0), p(x0 + a, y0)),
            Segmento::Cerrar,
        ],
        relleno: Some(Relleno {
            pintura: Pintura::Solida(color),
            opacidad: 1.0,
            par_impar: false,
        }),
        trazo: None,
    }
}

fn escurecido(c: Color, factor: f64) -> Color {
    Color {
        r: c.r * factor,
        g: c.g * factor,
        b: c.b * factor,
    }
}

fn mezcla(a: Color, b: Color, peso: f64) -> Color {
    let entre = |x: f64, y: f64| x * peso + y * (1.0 - peso);
    Color {
        r: entre(a.r, b.r),
        g: entre(a.g, b.g),
        b: entre(a.b, b.b),
    }
}

fn color_de_barra(fondo: Color) -> Color {
    let claro = (fondo.r + fondo.g + fondo.b) / 3.0 > 0.5;
    escurecido(fondo, if claro { SOMBRA_BARRA.1 } else { SOMBRA_BARRA.0 })
}

/// Los trozos de código, repartidos por sus líneas.
fn en_lineas(tramos: Vec<Tramo>) -> Vec<Vec<Tramo>> {
    let mut lineas = vec![Vec::new()];
    for tramo in tramos {
        let Tramo::Texto(texto, estilo) = tramo else { continue };
        for (n, trozo) in texto.split('\n').enumerate() {
            if n > 0 {
                lineas.push(Vec::new());
            }
            if let (false, Some(linea)) = (trozo.is_empty(), lineas.last_mut()) {
                linea.push(Tramo::Texto(trozo.into(), estilo));
            }
        }
    }
    lineas
}

/// Una línea de código ya colocada, para saber por dónde pasan las guías.
/// Cómo se ve un panel de código.
#[derive(Clone, Copy)]
struct Aspecto<'a> {
    estilo: &'a EstiloCodigo,
    colores: ColoresEditor,
}

/// Las letras y las columnas de las líneas de un panel.
#[derive(Clone, Copy)]
struct Disposicion {
    tam: f64,
    mono: Estilo,
    tenue: Estilo,
    /// Lo que ocupan los números de línea: nada si no se muestran.
    numeros: f64,
    x_numeros: f64,
    codigo: Columna,
}

#[derive(Debug, Clone, Copy)]
struct Renglon {
    arriba: f64,
    abajo: f64,
    /// Hasta dónde llega su primera línea visible.
    hasta: f64,
    /// Si se ha partido: lo que sigue empieza en la columna 0 y una guía lo pisaría.
    partida: bool,
}

/// Un tramo de guía: su nivel de sangría y lo que ocupa en vertical.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Guia {
    nivel: usize,
    desde: f64,
    hasta: f64,
}

/// Los espacios con que empieza una línea. `None` si no tiene nada escrito.
fn sangria(linea: &[Tramo]) -> Option<usize> {
    let texto: String = linea
        .iter()
        .filter_map(|t| match t {
            Tramo::Texto(x, _) => Some(&**x),
            _ => None,
        })
        .collect();
    let sin_espacios = texto.trim_start_matches(' ');
    (!sin_espacios.is_empty()).then_some(texto.len() - sin_espacios.len())
}

/// La sangría de cada línea. Una vacía toma la menor de las que la rodean: así las guías de un
/// bloque siguen por sus saltos de línea y acaban con él.
fn sangrias(lineas: &[Vec<Tramo>]) -> Vec<usize> {
    let propias: Vec<Option<usize>> = lineas.iter().map(|l| sangria(l)).collect();
    let vecinas = |i: usize| {
        let antes = propias[..i].iter().rev().flatten().next();
        let despues = propias[i + 1..].iter().flatten().next();
        antes.zip(despues).map_or(0, |(a, d)| *a.min(d))
    };
    propias
        .iter()
        .enumerate()
        .map(|(i, p)| p.unwrap_or_else(|| vecinas(i)))
        .collect()
}

/// Los tramos de guía de un código: uno por nivel y por bloque seguido de líneas tan sangradas.
fn guias(sangrias: &[usize], renglones: &[Renglon]) -> (usize, Vec<Guia>) {
    let Some(unidad) = sangrias.iter().copied().filter(|s| *s > 0).min() else {
        return (0, Vec::new());
    };
    let niveles = sangrias.iter().max().map_or(0, |m| m / unidad);
    let mut tramos = Vec::new();
    for nivel in 0..niveles {
        let mut abierta = None;
        for (sangria, r) in sangrias.iter().zip(renglones) {
            if *sangria < (nivel + 1) * unidad {
                if let Some(desde) = abierta.take() {
                    tramos.push(Guia {
                        nivel,
                        desde,
                        hasta: r.arriba,
                    });
                }
                continue;
            }
            let desde = *abierta.get_or_insert(r.arriba);
            if r.partida {
                tramos.push(Guia {
                    nivel,
                    desde,
                    hasta: r.hasta,
                });
                abierta = None;
            }
        }
        if let (Some(desde), Some(ultimo)) = (abierta, renglones.last()) {
            tramos.push(Guia {
                nivel,
                desde,
                hasta: ultimo.abajo,
            });
        }
    }
    (unidad, tramos)
}

fn trazo_de_guia(x: f64, g: Guia, color: Color) -> Primitiva {
    Primitiva::Trazado {
        segmentos: vec![
            Segmento::Mover(Punto { x, y: g.desde }),
            Segmento::Linea(Punto { x, y: g.hasta }),
        ],
        relleno: None,
        trazo: Some(Trazo {
            discontinuo: Some(Discontinuo {
                tramos: TRAZOS_GUIA.to_vec(),
                fase: 0.0,
            }),
            ..Trazo::simple(color, GROSOR_GUIA)
        }),
    }
}

/// Qué lleva escrito un editor.
pub(super) struct BloqueDeCodigo<'t> {
    pub(super) texto: &'t str,
    pub(super) lenguaje: Option<&'t str>,
    pub(super) opciones: &'t Opciones,
}

impl Maquetador<'_> {
    pub(super) fn editor(&mut self, bloque: BloqueDeCodigo<'_>, c: Columna) {
        let estilo = self.estilo_de_codigo(bloque.opciones);
        let colores = self.colores_de_editor(&estilo.tema);
        let tam = self.formato.tam * TAM_CODIGO;
        let inicio = self.primitivas.len();
        let arriba = self.y;
        let aspecto = Aspecto {
            estilo: &estilo,
            colores,
        };
        let barra = self.barra(&bloque, aspecto, c);
        self.y += tam * MARGEN * 0.7;
        let guias = self.lineas(&bloque, aspecto, c);
        self.y += tam * MARGEN * 0.7;
        let panel = Caja {
            x: c.x,
            y: arriba,
            ancho: c.ancho,
            alto: self.y - arriba,
        };
        let mut fondos = vec![redondeado(panel, [RADIO; 4], colores.fondo)];
        if let Some((alto, pestana)) = barra {
            let barra = Caja { alto, ..panel };
            let color = color_de_barra(colores.fondo);
            fondos.push(redondeado(barra, [RADIO, RADIO, 0.0, 0.0], color));
            fondos.push(pestana);
        }
        fondos.extend(guias);
        self.primitivas.splice(inicio..inicio, fondos);
        self.separar(SEPARACION);
    }

    /// El tema, lo que diga el documento y lo que diga el bloque, por ese orden.
    fn estilo_de_codigo(&self, opciones: &Opciones) -> EstiloCodigo {
        let mut estilo = self.formato.tema.codigo.clone();
        for (clave, valor) in self.codigo_del_documento.iter().chain(opciones) {
            estilo.aplicar(clave, valor);
        }
        estilo
    }

    fn colores_de_editor(&mut self, tema: &str) -> ColoresEditor {
        self.piezas.resaltador.editor(tema).unwrap_or_else(|| {
            self.avisos.push(format!("tema de código desconocido: {tema}"));
            ColoresEditor {
                fondo: self.formato.tema.fondo_codigo,
                texto: self.formato.color,
                numeros: escurecido(self.formato.tema.fondo_codigo, 0.55),
            }
        })
    }

    /// Escribe la barra de arriba, con el título en su pestaña o, si no hay, el lenguaje. Devuelve
    /// su alto y el fondo de la pestaña. Sin nada que decir no hay barra.
    fn barra(
        &mut self,
        b: &BloqueDeCodigo<'_>,
        Aspecto { estilo: e, colores }: Aspecto<'_>,
        c: Columna,
    ) -> Option<(f64, Primitiva)> {
        let nombre = e.titulo.as_deref().or(b.lenguaje)?;
        let tam = self.formato.tam * TAM_CODIGO;
        let alto = tam * ALTO_BARRA;
        let mono = Estilo {
            mono: true,
            color: Some(self.indice_de_color(colores.texto)),
            ..Estilo::default()
        };
        let tramos = [Tramo::Texto(nombre.into(), mono)];
        let escrito = self.escribir(&tramos, Hueco::libre(), Letra { tam, base: mono });
        let margen = tam * MARGEN;
        let pestana = Caja {
            x: c.x,
            y: self.y,
            ancho: escrito.ancho + 2.0 * margen,
            alto,
        };
        let pestana = redondeado(pestana, [RADIO, 0.0, 0.0, 0.0], colores.fondo);
        let y = self.y + (alto - escrito.alto) / 2.0;
        self.poner(escrito, Punto { x: c.x + margen, y });
        self.y += alto;
        Some((alto, pestana))
    }

    /// El código en trozos de un color, o entero si no se sabe resaltar su lenguaje.
    fn tramos_coloreados(&mut self, b: &BloqueDeCodigo<'_>, tema: &str, base: Estilo) -> Vec<Tramo> {
        let texto = b.texto.replace('\t', TABULADOR);
        let fragmentos = b
            .lenguaje
            .map_or_else(Vec::new, |l| self.piezas.resaltador.resaltar(&texto, l, tema));
        if fragmentos.is_empty() {
            if let Some(l) = b.lenguaje {
                self.avisos.push(format!("no sé resaltar el lenguaje {l}"));
            }
            return vec![Tramo::Texto(texto.into(), base)];
        }
        fragmentos
            .into_iter()
            .map(|f| {
                let estilo = Estilo {
                    negrita: f.negrita,
                    cursiva: f.cursiva,
                    color: Some(self.indice_de_color(f.color)),
                    ..base
                };
                Tramo::Texto(f.texto.into(), estilo)
            })
            .collect()
    }

    /// Lo que ocupan `cuantos` caracteres de la letra de código.
    fn ancho_de_caracteres(&mut self, cuantos: usize, letra: Letra) -> f64 {
        let texto = [Tramo::Texto("0".repeat(cuantos).into(), letra.base)];
        self.escribir(&texto, Hueco::libre(), letra).ancho
    }

    /// Escribe las líneas y devuelve sus guías, para que queden debajo del texto.
    fn lineas(
        &mut self,
        b: &BloqueDeCodigo<'_>,
        Aspecto { estilo: e, colores }: Aspecto<'_>,
        c: Columna,
    ) -> Vec<Primitiva> {
        let tam = self.formato.tam * TAM_CODIGO;
        let mono = Estilo {
            mono: true,
            color: Some(self.indice_de_color(colores.texto)),
            ..Estilo::default()
        };
        let tenue = Estilo {
            color: Some(self.indice_de_color(colores.numeros)),
            ..mono
        };
        let lineas = en_lineas(self.tramos_coloreados(b, &e.tema, mono));
        let ultimo = e.primera as usize + lineas.len().saturating_sub(1);
        let margen = tam * MARGEN;
        let numeros = if e.numeros {
            self.ancho_de_caracteres(ultimo.to_string().len(), Letra { tam, base: tenue })
        } else {
            0.0
        };
        let codigo = c.sangrada(margen + numeros + if e.numeros { margen } else { 0.0 });
        let disposicion = Disposicion {
            tam,
            mono,
            tenue,
            numeros,
            x_numeros: c.x + margen,
            codigo: Columna {
                ancho: (codigo.ancho - margen).max(tam),
                ..codigo
            },
        };
        let mut renglones = Vec::with_capacity(lineas.len());
        for (n, linea) in lineas.iter().enumerate() {
            let numero = e.numeros.then_some(e.primera as usize + n);
            renglones.push(self.renglon(linea, numero, &disposicion));
        }
        if !e.guias {
            return Vec::new();
        }
        let (unidad, tramos) = guias(&sangrias(&lineas), &renglones);
        let columna = self.ancho_de_caracteres(unidad.max(1), Letra { tam, base: mono });
        let color = mezcla(colores.numeros, colores.fondo, PESO_GUIA);
        tramos
            .into_iter()
            .map(|g| trazo_de_guia(disposicion.codigo.x + g.nivel as f64 * columna, g, color))
            .collect()
    }

    /// Escribe una línea, con su número si lo lleva, y baja lo que ocupa.
    fn renglon(&mut self, linea: &[Tramo], numero: Option<usize>, d: &Disposicion) -> Renglon {
        let Disposicion { tam, mono, tenue, .. } = *d;
        let hueco = Hueco::de(d.codigo.ancho, None);
        let escrito = self.escribir(linea, hueco, Letra { tam, base: mono });
        let alto_minimo = tam * ALTO_DE_LINEA;
        let alto_primera = escrito.alto_primera_linea();
        let aire = (alto_minimo - alto_primera).max(0.0) / 2.0;
        let alto = (escrito.alto + 2.0 * aire).max(alto_minimo);
        let renglon = Renglon {
            arriba: self.y,
            abajo: self.y + alto,
            hasta: self.y + alto_primera + 2.0 * aire,
            partida: escrito.lineas_partidas(),
        };
        let y = self.y + aire;
        if let Some(n) = numero {
            let numero = [Tramo::Texto(n.to_string().into(), tenue)];
            let hueco = Hueco::de(d.numeros + 0.5, Some(Align::Right));
            let escrito = self.escribir(&numero, hueco, Letra { tam, base: tenue });
            self.poner(escrito, Punto { x: d.x_numeros, y });
        }
        self.poner(escrito, Punto { x: d.codigo.x, y });
        self.y += alto;
        renglon
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn textos(lineas: &[Vec<Tramo>]) -> Vec<String> {
        lineas
            .iter()
            .map(|l| {
                l.iter()
                    .map(|t| match t {
                        Tramo::Texto(x, _) => &**x,
                        _ => "",
                    })
                    .collect()
            })
            .collect()
    }

    fn lineas_de(codigo: &str) -> Vec<Vec<Tramo>> {
        en_lineas(vec![Tramo::Texto(codigo.into(), Estilo::default())])
    }

    /// Un renglón de alto 1 por línea, ninguno partido.
    fn renglones(cuantos: usize) -> Vec<Renglon> {
        (0..cuantos)
            .map(|n| Renglon {
                arriba: n as f64,
                abajo: n as f64 + 1.0,
                hasta: n as f64 + 1.0,
                partida: false,
            })
            .collect()
    }

    #[test]
    fn una_linea_vacia_toma_la_menor_sangria_que_la_rodea() {
        assert_eq!(sangrias(&lineas_de("a\n    b\n\n    c\n\nd")), [0, 4, 4, 4, 0, 0]);
    }

    #[test]
    fn la_guia_sigue_por_los_saltos_de_linea_y_acaba_con_el_bloque() {
        let s = sangrias(&lineas_de("a\n    b\n\n    c\nd"));
        let (unidad, g) = guias(&s, &renglones(5));
        assert_eq!(unidad, 4);
        assert_eq!(
            g,
            [Guia {
                nivel: 0,
                desde: 1.0,
                hasta: 4.0
            }]
        );
    }

    #[test]
    fn cada_nivel_de_sangria_lleva_su_guia() {
        let s = sangrias(&lineas_de("a\n  b\n    c\n  d\ne"));
        let (unidad, g) = guias(&s, &renglones(5));
        assert_eq!(unidad, 2);
        let esperado = [
            Guia {
                nivel: 0,
                desde: 1.0,
                hasta: 4.0,
            },
            Guia {
                nivel: 1,
                desde: 2.0,
                hasta: 3.0,
            },
        ];
        assert_eq!(g, esperado);
    }

    #[test]
    fn dos_bloques_separados_son_dos_guias() {
        let s = sangrias(&lineas_de("a\n  b\nc\n  d\ne"));
        assert_eq!(guias(&s, &renglones(5)).1.len(), 2);
    }

    #[test]
    fn sin_sangria_no_hay_guias() {
        assert_eq!(guias(&sangrias(&lineas_de("a\nb")), &renglones(2)).1, []);
    }

    #[test]
    fn una_linea_partida_corta_la_guia_para_no_pisar_lo_que_sigue() {
        let s = sangrias(&lineas_de("a\n  b\n  c\nd"));
        let mut r = renglones(4);
        r[1] = Renglon {
            hasta: 1.5,
            partida: true,
            ..r[1]
        };
        let (_, g) = guias(&s, &r);
        let esperado = [
            Guia {
                nivel: 0,
                desde: 1.0,
                hasta: 1.5,
            },
            Guia {
                nivel: 0,
                desde: 2.0,
                hasta: 3.0,
            },
        ];
        assert_eq!(g, esperado);
    }

    #[test]
    fn la_barra_se_distingue_del_panel_sin_ensuciar_un_tema_claro() {
        let (claro, oscuro) = (
            Color {
                r: 0.98,
                g: 0.98,
                b: 0.98,
            },
            Color {
                r: 0.16,
                g: 0.17,
                b: 0.2,
            },
        );
        assert!(color_de_barra(claro).r > 0.9 && color_de_barra(claro).r < claro.r);
        assert!(color_de_barra(oscuro).r < oscuro.r);
    }

    #[test]
    fn los_trozos_se_reparten_por_sus_lineas() {
        let e = Estilo::default();
        let t = |x: &str| Tramo::Texto(x.into(), e);
        let lineas = en_lineas(vec![t("fn a"), t("() {\n"), t("\n"), t("}")]);
        assert_eq!(textos(&lineas), ["fn a() {", "", "}"]);
    }

    #[test]
    fn un_panel_redondeado_es_un_contorno_cerrado_de_ocho_tramos() {
        let c = Caja {
            x: 0.0,
            y: 0.0,
            ancho: 100.0,
            alto: 50.0,
        };
        let Primitiva::Trazado { segmentos, .. } = redondeado(c, [RADIO; 4], Color { r: 0.0, g: 0.0, b: 0.0 }) else {
            panic!();
        };
        assert_eq!(segmentos.len(), 10);
        assert!(matches!(segmentos.last(), Some(Segmento::Cerrar)));
    }
}
