import 'dart:io';

import 'package:flutter/material.dart';

import 'escena.dart';
import 'nativo.dart';
import 'volcado.dart';

void main() => runApp(Ejemplo(Nativo.abrir()));

class Ejemplo extends StatelessWidget {
  const Ejemplo(this.nativo, {super.key});

  final Nativo nativo;

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'N-LTX',
    debugShowCheckedModeBanner: false,
    theme: ThemeData(
      colorSchemeSeed: Colors.indigo,
      scaffoldBackgroundColor: const Color(0xFFECEEF3),
    ),
    home: Galeria(nativo),
  );
}

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

class Galeria extends StatefulWidget {
  const Galeria(this.nativo, {super.key});

  final Nativo nativo;

  @override
  State<Galeria> createState() => _GaleriaState();
}

class _GaleriaState extends State<Galeria> {
  final _editor = TextEditingController(
    text: r'Prueba: la ley $F=ma$ y el agua \ce{H2O}.',
  );
  late final Map<String, List<(Muestra, Escena)>> _escenas;
  late Escena _propia;
  Modo _modo = Modo.documento;
  double _zoom = 1.6;
  int _microsegundos = 0;

  @override
  void initState() {
    super.initState();
    final reloj = Stopwatch()..start();
    _escenas = {
      for (final s in _secciones.entries)
        s.key: [
          for (final m in s.value)
            (m, widget.nativo.componer(m.fuente, modo: m.modo)),
        ],
    };
    _microsegundos = reloj.elapsedMicroseconds;
    _componerPropia();
    final volcado = Platform.environment['NLTX_VOLCAR'];
    if (volcado != null) {
      volcar(volcado, [
        for (final s in _escenas.values)
          for (final (_, e) in s) e,
      ]);
    }
  }

  void _componerPropia() {
    _propia = widget.nativo.componer(_editor.text, modo: _modo);
  }

  @override
  Widget build(BuildContext context) {
    final total = _escenas.values.fold<int>(0, (a, b) => a + b.length);
    return Scaffold(
      appBar: AppBar(
        title: const Text('N-LTX · mathra y tikra en Flutter'),
        backgroundColor: Colors.white,
        actions: [
          Text(
            '$total ejemplos en ${(_microsegundos / 1000).toStringAsFixed(0)} ms · '
            'pintor Canvas, sin Vulkan',
            style: Theme.of(context).textTheme.bodySmall,
          ),
          const SizedBox(width: 20),
          const Icon(Icons.zoom_in, size: 18),
          SizedBox(
            width: 170,
            child: Slider(
              value: _zoom,
              min: 0.8,
              max: 6,
              onChanged: (v) => setState(() => _zoom = v),
            ),
          ),
          const SizedBox(width: 8),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          _Editor(
            controlador: _editor,
            modo: _modo,
            escena: _propia,
            zoom: _zoom,
            alCambiarTexto: () => setState(_componerPropia),
            alCambiarModo: (v) => setState(() {
              _modo = v;
              _componerPropia();
            }),
          ),
          for (final s in _escenas.entries) ...[
            Padding(
              padding: const EdgeInsets.fromLTRB(4, 22, 0, 8),
              child: Text(
                s.key,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
            Wrap(
              spacing: 12,
              runSpacing: 12,
              children: [
                for (final (m, e) in s.value)
                  _Tarjeta(muestra: m, escena: e, zoom: _zoom),
              ],
            ),
          ],
        ],
      ),
    );
  }
}

class _Lamina extends StatelessWidget {
  const _Lamina({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => Container(
    padding: const EdgeInsets.all(16),
    decoration: BoxDecoration(
      color: Colors.white,
      borderRadius: BorderRadius.circular(8),
      border: Border.all(color: const Color(0xFFD9DCE3)),
    ),
    child: child,
  );
}

class _Resultado extends StatelessWidget {
  const _Resultado({required this.escena, required this.zoom});

  final Escena escena;
  final double zoom;

  @override
  Widget build(BuildContext context) {
    final error = escena.error;
    if (error != null) {
      return Text(error, style: const TextStyle(color: Colors.red));
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          child: VistaEscena(escena: escena, zoom: zoom),
        ),
        for (final aviso in escena.avisos)
          Padding(
            padding: const EdgeInsets.only(top: 6),
            child: Text(
              '⚠ $aviso',
              style: const TextStyle(fontSize: 11, color: Color(0xFFB06000)),
            ),
          ),
      ],
    );
  }
}

class _Tarjeta extends StatelessWidget {
  const _Tarjeta({
    required this.muestra,
    required this.escena,
    required this.zoom,
  });

  final Muestra muestra;
  final Escena escena;
  final double zoom;

  @override
  Widget build(BuildContext context) => ConstrainedBox(
    constraints: const BoxConstraints(maxWidth: 900),
    child: _Lamina(
      child: _Resultado(escena: escena, zoom: zoom),
    ),
  );
}

class _Editor extends StatelessWidget {
  const _Editor({
    required this.controlador,
    required this.modo,
    required this.escena,
    required this.zoom,
    required this.alCambiarTexto,
    required this.alCambiarModo,
  });

  final TextEditingController controlador;
  final Modo modo;
  final Escena escena;
  final double zoom;
  final VoidCallback alCambiarTexto;
  final ValueChanged<Modo> alCambiarModo;

  @override
  Widget build(BuildContext context) => _Lamina(
    child: Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Text(
              'Escribe LaTeX',
              style: Theme.of(context).textTheme.titleMedium,
            ),
            const Spacer(),
            SegmentedButton<Modo>(
              showSelectedIcon: false,
              segments: const [
                ButtonSegment(value: Modo.documento, label: Text('Documento')),
                ButtonSegment(value: Modo.formula, label: Text('Fórmula')),
                ButtonSegment(value: Modo.figura, label: Text('TikZ')),
              ],
              selected: {modo},
              onSelectionChanged: (s) => alCambiarModo(s.first),
            ),
          ],
        ),
        const SizedBox(height: 10),
        TextField(
          controller: controlador,
          minLines: 2,
          maxLines: 8,
          onChanged: (_) => alCambiarTexto(),
          style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
          decoration: const InputDecoration(border: OutlineInputBorder()),
        ),
        const SizedBox(height: 14),
        _Resultado(escena: escena, zoom: zoom),
      ],
    ),
  );
}
