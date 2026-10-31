//! El árbol de una fórmula: el contrato que comparten todas las piezas (los
//! lectores lo producen, los escritores y el maquetador lo consumen).

/// Clase TeX de un átomo: decide el espaciado entre átomos vecinos.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Clase {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
}

/// Alfabeto con el que se escriben las letras (y dígitos) de una fórmula.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alfabeto {
    /// Por defecto en fórmulas: letras en cursiva matemática.
    Cursiva,
    Recta,
    Negrita,
    NegritaCursiva,
    Pizarra,
    Caligrafica,
    Fraktur,
    SinSerifa,
    Mono,
}

/// Estilos de TeX: display, texto, índice y subíndice de índice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estilo {
    Display,
    Texto,
    Indice,
    SubIndice,
}

/// Alineación horizontal de una columna de tabla.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alin {
    Izq,
    Centro,
    Der,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Nodo {
    Atomo {
        c: char,
        clase: Clase,
    },
    Grupo(Vec<Nodo>),
    Fraccion {
        num: Box<Nodo>,
        den: Box<Nodo>,
        estilo: Option<Estilo>,
        regla: bool,
        delims: Option<(char, char)>,
    },
    Raiz {
        indice: Option<Box<Nodo>>,
        radicando: Box<Nodo>,
    },
    Guiones {
        base: Box<Nodo>,
        sup: Option<Box<Nodo>>,
        sub: Option<Box<Nodo>>,
    },
    /// Base con algo encima y/o debajo (\overset, \underset, \stackrel).
    Limites {
        base: Box<Nodo>,
        sup: Option<Box<Nodo>>,
        sub: Option<Box<Nodo>>,
    },
    Texto(String),
    Funcion {
        nombre: String,
        limites: bool,
    },
    GranOp {
        c: char,
        limites: bool,
    },
    /// Espacio en em (puede ser negativo).
    Espacio(f32),
    Delimitado {
        izq: Option<char>,
        der: Option<char>,
        cuerpo: Vec<Nodo>,
    },
    /// Delimitador de tamaño fijo (\big, \Big, \bigg, \Bigg).
    Tamano {
        c: char,
        clase: Clase,
        nivel: u8,
    },
    Acento {
        c: char,
        base: Box<Nodo>,
    },
    /// \overline (arriba) o \underline.
    Raya {
        arriba: bool,
        base: Box<Nodo>,
    },
    /// Cambio de estilo para lo que sigue en el grupo (\displaystyle…).
    Estilo(Estilo),
    Tabla {
        filas: Vec<Vec<Nodo>>,
        /// Patrón de alineación de columnas (se repite si hay más).
        columnas: Vec<Alin>,
        /// Separación entre columnas, en em.
        separacion: f32,
        delims: (Option<char>, Option<char>),
        estilo: Estilo,
    },
    /// Algo tachado con una barra (\not ante algo sin forma negada propia,
    /// \cancel).
    Tachado(Box<Nodo>),
    /// Flecha que se estira bajo/sobre sus etiquetas (\xrightarrow).
    Flecha {
        c: char,
        sup: Option<Box<Nodo>>,
        sub: Option<Box<Nodo>>,
    },
    /// Comando que no se sabe maquetar: se muestra su nombre.
    Desconocido(String),
}

impl Clase {
    /// Tras \not el resultado sigue siendo una relación aunque el carácter
    /// combinado no figure en la tabla.
    pub fn max_rel(self) -> Clase {
        if self == Clase::Ord { Clase::Rel } else { self }
    }
}

/// Recorre en preorden todos los nodos (los de dentro de grupos, fracciones,
/// índices, tablas…).
pub fn recorrer(nodos: &[Nodo], f: &mut impl FnMut(&Nodo)) {
    for n in nodos {
        f(n);
        match n {
            Nodo::Grupo(v) | Nodo::Delimitado { cuerpo: v, .. } => recorrer(v, f),
            Nodo::Fraccion { num, den, .. } => {
                recorrer(std::slice::from_ref(num), f);
                recorrer(std::slice::from_ref(den), f);
            }
            Nodo::Raiz { indice, radicando } => {
                if let Some(i) = indice {
                    recorrer(std::slice::from_ref(i), f);
                }
                recorrer(std::slice::from_ref(radicando), f);
            }
            Nodo::Guiones { base, sup, sub } | Nodo::Limites { base, sup, sub } => {
                recorrer(std::slice::from_ref(base), f);
                for x in [sup, sub].into_iter().flatten() {
                    recorrer(std::slice::from_ref(x), f);
                }
            }
            Nodo::Flecha { sup, sub, .. } => {
                for x in [sup, sub].into_iter().flatten() {
                    recorrer(std::slice::from_ref(x), f);
                }
            }
            Nodo::Acento { base, .. } | Nodo::Raya { base, .. } | Nodo::Tachado(base) => {
                recorrer(std::slice::from_ref(base), f)
            }
            Nodo::Tabla { filas, .. } => {
                for fila in filas {
                    recorrer(fila, f);
                }
            }
            Nodo::Atomo { .. }
            | Nodo::Texto(_)
            | Nodo::Funcion { .. }
            | Nodo::GranOp { .. }
            | Nodo::Espacio(_)
            | Nodo::Tamano { .. }
            | Nodo::Estilo(_)
            | Nodo::Desconocido(_) => {}
        }
    }
}
