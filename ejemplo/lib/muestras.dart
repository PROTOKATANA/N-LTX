/// Los ejemplos de la galería.
library;

enum Modo { formula, figura, documento }

class Muestra {
  const Muestra(this.fuente, {this.modo = Modo.formula});

  final String fuente;
  final Modo modo;
}

const _secciones = <String, List<Muestra>>{
  'Documento': [
    Muestra(modo: Modo.documento, r'''
\section{Movimiento armónico}
Un resorte de constante $k$ cumple la ley de Hooke, $F=-kx$, mientras trabaja en \textbf{régimen elástico}. Si la masa vale $m$, la ecuación del movimiento es
\[ \dv[2]{x}{t} + \omega^2 x = 0, \qquad \omega=\sqrt{\frac{k}{m}} \]
El agua es \ce{H2O} y la gravedad vale \SI{9.81}{m/s^2}. El resultado \emph{no depende} de la amplitud:
\begin{itemize}
\item la frecuencia solo depende de $k$ y de $m$;
\item la energía se conserva: $E=\tfrac{1}{2}kA^2$.
\end{itemize}
\usetikzlibrary{decorations.pathmorphing}
\begin{tikzpicture}
  \fill[gray!40] (0,0) rectangle (0.2,1.2);
  \draw[decorate,decoration={coil,aspect=0.6,segment length=3mm,amplitude=2mm}] (0.2,0.6) -- (3,0.6);
  \fill[blue!30,draw=black] (3,0.2) rectangle (3.8,1);
  \node at (3.4,0.6) {$m$};
\end{tikzpicture}
\begin{tabular}{ccc}
Magnitud & Símbolo & Unidad \\
Fuerza & $F$ & \si{N} \\
Energía & $E$ & \si{J} \\
\end{tabular}
\begin{quote}La física es una forma de mirar.\end{quote}
'''),
  ],
  'Animaciones': [
    Muestra(modo: Modo.documento, r'''
\begin{animacion}[t=0:360, duracion=3]
\begin{tikzpicture}
  \pgfmathsetmacro{\px}{cos(\t)}
  \pgfmathsetmacro{\py}{sin(\t)}
  \pgfmathsetmacro{\lx}{\px+sqrt(9-\py*\py)}
  \draw[thick] (0,0) circle (1);
  \draw (1.6,-0.7) -- (5.2,-0.7) (1.6,0.7) -- (5.2,0.7);
  \fill[gray!30,draw=black] (\lx-0.4,-0.6) rectangle (\lx+0.8,0.6);
  \draw[very thick,blue] (0,0) -- (\px,\py) -- (\lx,0);
  \fill (0,0) circle (0.06) (\px,\py) circle (0.06) (\lx,0) circle (0.06);
  \node at (0,-1.4) {$\theta=\t^\circ$};
\end{tikzpicture}
\end{animacion}
'''),
    Muestra(modo: Modo.documento, r'''
\begin{animacion}[t=0:360, duracion=2]
\begin{tikzpicture}
  \draw[->] (-0.2,0) -- (6.6,0) node[right] {$x$};
  \draw[->] (0,-1.2) -- (0,1.4) node[above] {$y$};
  \foreach \i in {0,...,60} {
    \pgfmathsetmacro{\y}{sin(\i*12-\t)}
    \fill[blue!60] (\i*0.1,\y) circle (0.05);
  }
\end{tikzpicture}
\end{animacion}
'''),
    Muestra(modo: Modo.documento, r'''
\begin{animacion}[t=0:360, duracion=4]
\begin{tikzpicture}
  \begin{axis}[domain=0:360,samples=80,width=9cm,height=5cm,xlabel=$x$,ylim={-1.2:1.2}]
    \addplot[blue,thick] {sin(x+\t)};
    \addplot[red,thick] {cos(2*x-\t)};
  \end{axis}
\end{tikzpicture}
\end{animacion}
'''),
  ],
  'Fórmulas': [
    Muestra(
      r'\overbrace{1+2+\cdots+n}^{n\ \text{términos}} = \frac{n(n+1)}{2}',
    ),
    Muestra(r'\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}'),
    Muestra(
      r'\begin{pmatrix} a & b \\ c & d \end{pmatrix}^{-1} = \frac{1}{ad-bc}\begin{pmatrix} d & -b \\ -c & a \end{pmatrix}',
    ),
    Muestra(
      r'\cfrac{1}{\sqrt{2}+\cfrac{1}{\sqrt{2}+\cfrac{1}{\sqrt{2}+\cdots}}}',
    ),
    Muestra(
      r'f(x)=\begin{cases} x^2 & \text{si } x\ge 0 \\ -x & \text{si } x<0 \end{cases}',
    ),
    Muestra(
      r'\textcolor{red}{\boldsymbol{\alpha}} + \textcolor{blue}{\boldsymbol{\beta}}',
    ),
  ],
  'Química': [
    Muestra(r'\ce{CO2 + H2O <=> H2CO3}'),
    Muestra(r'\ce{Fe^3+ + 3OH- -> Fe(OH)3 v}'),
    Muestra(r'\chemfig{*6(=-=-=-)}'),
    Muestra(r'\chemfig{H_3C-CH_2-OH}'),
    Muestra(r'\schemestart A \arrow B \schemestop'),
  ],
  'Física': [
    Muestra(r'i\hbar\pdv{\psi}{t} = -\frac{\hbar^2}{2m}\laplacian\psi + V\psi'),
    Muestra(r'\curl\vb{B} = \mu_0\qty(\vb{J} + \varepsilon_0\pdv{\vb{E}}{t})'),
    Muestra(r'\expval{A}{\psi} \qquad \braket{\psi}{\phi} \qquad \comm{A}{B}'),
    Muestra(r'g = \SI{9.81}{m/s^2} \qquad v = \qty{120}{km/h}'),
  ],
  'TikZ': [
    Muestra(modo: Modo.figura, r'''
\begin{tikzpicture}[scale=2]
  \draw[->] (-1.3,0) -- (1.3,0) node[right] {$x$};
  \draw[->] (0,-1.3) -- (0,1.3) node[above] {$y$};
  \draw (0,0) circle (1);
  \draw[thick,red] (0,0) -- (30:1) node[midway,above] {$r$};
  \draw[blue] (0.3,0) arc (0:30:0.3) node[midway,right] {$\theta$};
  \draw[dashed] (30:1) -- ({cos(30)},0);
\end{tikzpicture}'''),
    Muestra(modo: Modo.figura, r'''
\begin{tikzpicture}
  \foreach \a in {0,30,...,330} {
    \fill[red!70,opacity=0.45,rotate=\a] (0.65,0) ellipse (0.65 and 0.2);
  }
  \shade[ball color=blue!60] (2.7,0) circle (0.9);
  \shade[left color=yellow,right color=red] (4,-0.9) rectangle (6,0.9);
\end{tikzpicture}'''),
    Muestra(modo: Modo.figura, r'''
\usetikzlibrary{positioning}
\begin{tikzpicture}[caja/.style={draw,rounded corners,fill=blue!10,minimum width=2.2cm,minimum height=0.9cm}]
  \node[caja] (a) {LaTeX};
  \node[caja,right=1.1cm of a] (b) {\texttt{adaptador}};
  \node[caja,right=1.1cm of b] (c) {primitivas};
  \node[caja,below=0.8cm of c,fill=green!15] (d) {pintor};
  \draw[->,thick] (a) -- (b);
  \draw[->,thick] (b) -- (c);
  \draw[->,thick] (c) -- (d);
\end{tikzpicture}'''),
    Muestra(modo: Modo.figura, r'''
\usetikzlibrary{decorations.pathmorphing}
\begin{tikzpicture}
  \fill[gray!40] (0,0) rectangle (0.2,1.2);
  \draw[thick] (0.2,-0.1) -- (0.2,1.3);
  \draw[decorate,decoration={coil,aspect=0.6,segment length=3mm,amplitude=2mm}] (0.2,0.6) -- (3,0.6);
  \fill[blue!30,draw=black] (3,0.2) rectangle (3.8,1);
  \node at (3.4,0.6) {$m$};
  \draw[->] (3.8,0.6) -- (4.8,0.6) node[right] {$F=-kx$};
\end{tikzpicture}'''),
    Muestra(modo: Modo.figura, r'''
\begin{tikzpicture}
  \begin{axis}[xlabel=$x$,ylabel=$\sin x$,domain=-6:6,samples=120,width=9cm,height=5cm,grid=major]
    \addplot[blue,thick] {sin(deg(x))};
    \addplot[red,dashed] {0.5*x};
  \end{axis}
\end{tikzpicture}'''),
  ],
};

/// Toda la galería en un solo documento: una vista, una superficie.
final String galeria = () {
  final b = StringBuffer();
  for (final s in _secciones.entries) {
    if (s.key != 'Documento') b.writeln('\\section{${s.key}}');
    for (final m in s.value) {
      b
        ..writeln(m.modo == Modo.formula ? '\\[ ${m.fuente} \\]' : m.fuente)
        ..writeln();
    }
  }
  return '$b';
}();
