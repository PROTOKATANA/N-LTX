mod comun;

use adaptador::IncrustadosMotor;
use documento::{Documento, Formato};
use primitivas::{Color, Composicion, Compositor, Primitiva};

const NEGRO: Color = Color { r: 0.0, g: 0.0, b: 0.0 };

fn componer(ancho: f64, fuente: &str) -> Composicion {
    Documento::nuevo(
        Formato {
            ancho,
            ..Formato::default()
        },
        Box::new(IncrustadosMotor { color: NEGRO }),
    )
    .componer(fuente)
    .expect("documento")
}

const COMPLETO: &str = r"\documentclass{article}
\usepackage{tikz}
\usetikzlibrary{positioning}
\newcommand{\R}{\mathbb{R}}
\begin{document}
\section{Movimiento armónico}
Un resorte de constante $k$ cumple la ley de Hooke, $F=-kx$, en \textbf{régimen elástico}. El agua
es \ce{H2O} y la gravedad vale \SI{9.81}{m/s^2}.
\[ \dv[2]{x}{t} + \omega^2 x = 0 \qquad x \in \R \]
\begin{itemize}
\item La frecuencia es $\omega=\sqrt{k/m}$.
\item El periodo no depende de la amplitud.
\end{itemize}
\begin{tikzpicture}
  \node[draw] (a) {masa};
  \node[draw,right=1cm of a] (b) {resorte};
  \draw[->] (a) -- (b);
\end{tikzpicture}
\end{document}";

#[test]
fn un_documento_mezcla_texto_formulas_y_figuras() {
    let c = componer(420.0, COMPLETO);
    assert!(c.avisos.is_empty(), "{:?}", c.avisos);
    let e = &c.escena;
    assert!(comun::capas_equilibradas(&e.primitivas));
    assert!(comun::glifos_con_contorno(e));
    assert!(e.primitivas.iter().any(|p| matches!(p, Primitiva::Trazado { .. })));
    // fuentes del texto y de las fórmulas, cada una con sus contornos
    let fuentes: std::collections::BTreeSet<u8> = e.contornos.keys().map(|g| g.fuente).collect();
    assert!(fuentes.len() >= 3, "{fuentes:?}");
    assert!(e.caja.alto > 150.0, "{}", e.caja.alto);
}

#[test]
fn un_documento_se_puede_pintar() {
    let e = componer(420.0, COMPLETO).escena;
    let svg = comun::svg(&e).expect("sin patrones");
    assert!(comun::rasterizar(&svg, 96.0).is_some());
}

#[test]
fn estrechar_el_documento_lo_alarga() {
    assert!(componer(240.0, COMPLETO).escena.caja.alto > componer(600.0, COMPLETO).escena.caja.alto);
}

/// Lo que recibe el motor mientras el modelo escribe.
#[test]
fn un_documento_a_medio_escribir_se_compone_en_cada_paso() {
    let doc = Documento::nuevo(Formato::default(), Box::new(IncrustadosMotor { color: NEGRO }));
    for (i, _) in COMPLETO.char_indices().step_by(7) {
        let c = doc.componer(&COMPLETO[..i]).expect("prefijo");
        assert!(comun::capas_equilibradas(&c.escena.primitivas), "prefijo {i}");
    }
}
