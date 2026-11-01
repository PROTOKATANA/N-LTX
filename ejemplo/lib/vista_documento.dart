import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';

import 'repositorio.dart';

/// Un documento en la superficie Vulkan: Flutter muestra la textura y lleva el desplazamiento.
class VistaDocumento extends StatefulWidget {
  const VistaDocumento({
    super.key,
    required this.motor,
    required this.pagina,
    required this.fuente,
    required this.zoom,
    this.fondo = Colors.white,
    this.tinta = Colors.black,
    this.conMuestras = false,
  });

  final RepositorioMotor motor;

  /// Qué se está viendo: al cambiar de página, se vuelve arriba.
  final Object pagina;
  final String fuente;
  final double zoom;

  /// Sobre qué color se pinta: cambiarlo no recompone.
  final Color fondo;

  /// El color de lo que el documento no colorea: cambiarlo recompone.
  final Color tinta;

  /// Tras `fuente` van todas las muestras: las tiene Rust.
  final bool conMuestras;

  @override
  State<VistaDocumento> createState() => _VistaDocumentoState();
}

/// Un arrastre empieza cuando el ratón se ha movido esto, en píxeles lógicos.
const _umbralArrastre = 3.0;

/// La franja de la derecha donde el ratón es de la barra de desplazamiento.
const _anchoBarra = 16.0;

/// Cerca de un borde, arrastrar desplaza el documento: la franja y lo que se mueve en cada paso.
const _bordeArrastre = 24.0;
const _pasoArrastre = 14.0;
const _ritmoArrastre = Duration(milliseconds: 16);

class _VistaDocumentoState extends State<VistaDocumento> {
  /// Solo para lo que exige el motor de Flutter: la textura y su tamaño.
  static const _canal = MethodChannel('ltx/lienzo');

  final _desplazamiento = ScrollController();
  final _foco = FocusNode(debugLabel: 'documento');
  int? _textura;

  /// El tamaño de la textura, en píxeles físicos. No cambia con la ventana: solo se pinta lo que se
  /// ve, y por eso redimensionar no la recrea.
  Size _lienzo = Size.zero;
  Size _tam = Size.zero;
  double _pixeles = 1;
  Publicado _publicado = const Publicado(alto: 0, avisos: []);
  String _medidas = '';
  bool _hayQuePublicar = true;

  /// Dónde empezó el arrastre, en bp del documento; nulo si no hay ninguno.
  Offset? _inicio;
  Offset _raton = Offset.zero;
  bool _arrastrando = false;
  Duration? _ultimoClic;
  Offset _lugarDelClic = Offset.zero;
  Timer? _bordes;

  /// Lo de arriba llega enseguida y el resto detrás: que se note que aún falta.
  bool _componiendo = false;
  final _suscripciones = <StreamSubscription<Object?>>[];

  @override
  void initState() {
    super.initState();
    _desplazamiento.addListener(_verVista);
    _suscripciones
      ..add(
        widget.motor.publicados.listen((p) {
          setState(() {
            _publicado = p;
            _componiendo = !p.completo;
          });
        }),
      )
      ..add(
        widget.motor.copiados.listen((texto) {
          if (texto.isNotEmpty) Clipboard.setData(ClipboardData(text: texto));
        }),
      )
      ..add(
        widget.motor.medidas.listen((m) {
          setState(() {
            _medidas =
                '${m.vertices} vértices · '
                '${m.msTaller.toStringAsFixed(2)} ms de taller · '
                '${m.msCarril.toStringAsFixed(2)} ms de carril';
          });
        }),
      );
    _abrir();
  }

  Future<void> _abrir() async {
    widget.motor.fondo(widget.fondo);
    final id = await _canal.invokeMethod<int>('create', 0);
    final lienzo = await _canal.invokeListMethod<int>('lienzo', 0);
    if (!mounted) return;
    setState(() {
      _textura = id;
      _lienzo = lienzo == null
          ? Size.zero
          : Size(lienzo[0].toDouble(), lienzo[1].toDouble());
    });
    _publicar();
  }

  @override
  void didUpdateWidget(VistaDocumento antes) {
    super.didUpdateWidget(antes);
    if (antes.pagina != widget.pagina) {
      if (_desplazamiento.hasClients) _desplazamiento.jumpTo(0);
      setState(() => _componiendo = true);
    }
    if (antes.fuente != widget.fuente ||
        antes.zoom != widget.zoom ||
        antes.tinta != widget.tinta ||
        antes.conMuestras != widget.conMuestras) {
      _pedirPublicacion();
    }
    if (antes.fondo != widget.fondo) widget.motor.fondo(widget.fondo);
  }

  void _pedirPublicacion() {
    if (_hayQuePublicar) return;
    _hayQuePublicar = true;
    SchedulerBinding.instance.addPostFrameCallback((_) => _publicar());
  }

  void _publicar() {
    if (!mounted || _textura == null || _tam.isEmpty) return;
    _hayQuePublicar = false;
    widget.motor.publicar(
      widget.fuente,
      ancho: _tam.width / widget.zoom,
      tinta: widget.tinta,
      conMuestras: widget.conMuestras,
    );
    _verVista();
  }

  /// El desplazamiento va entero para que el texto no se emborrone.
  void _verVista() {
    final y = _desplazamiento.hasClients ? _desplazamiento.offset : 0.0;
    widget.motor.ver(
      Encuadre(
        ancho: (_tam.width * _pixeles).round(),
        alto: (_tam.height * _pixeles).round(),
        escala: widget.zoom * _pixeles,
        y: (y * _pixeles).roundToDouble(),
      ),
    );
  }

  Offset _enDocumento(Offset local) {
    final y = _desplazamiento.hasClients ? _desplazamiento.offset : 0.0;
    return Offset(local.dx, local.dy + y) / widget.zoom;
  }

  void _alBajar(PointerDownEvent e) {
    final conRaton = e.kind == PointerDeviceKind.mouse;
    if (!conRaton || e.buttons != kPrimaryMouseButton) return;
    if (e.localPosition.dx > _tam.width - _anchoBarra) return;
    _foco.requestFocus();
    final punto = _enDocumento(e.localPosition);
    final ultimo = _ultimoClic;
    final doble =
        ultimo != null &&
        e.timeStamp - ultimo < kDoubleTapTimeout &&
        (e.localPosition - _lugarDelClic).distance < kDoubleTapSlop;
    _ultimoClic = doble ? null : e.timeStamp;
    _lugarDelClic = e.localPosition;
    if (doble) {
      widget.motor.seleccionarPalabra(punto);
      return;
    }
    _inicio = punto;
    _raton = e.localPosition;
    _arrastrando = false;
  }

  void _alMover(PointerMoveEvent e) {
    final inicio = _inicio;
    if (inicio == null) return;
    _raton = e.localPosition;
    if (!_arrastrando) {
      final movido = (e.localPosition - _lugarDelClic).distance;
      if (movido < _umbralArrastre) return;
      _arrastrando = true;
      _bordes = Timer.periodic(_ritmoArrastre, (_) => _desplazarEnBorde());
    }
    widget.motor.arrastrar(inicio, _enDocumento(_raton));
  }

  /// Fuera de la vista hay más documento: se sigue seleccionando sin mover el ratón.
  void _desplazarEnBorde() {
    final inicio = _inicio;
    if (inicio == null || !_desplazamiento.hasClients) return;
    final arriba = _raton.dy < _bordeArrastre;
    final abajo = _raton.dy > _tam.height - _bordeArrastre;
    if (!arriba && !abajo) return;
    final p = _desplazamiento.position;
    final destino = p.pixels + (abajo ? _pasoArrastre : -_pasoArrastre);
    _desplazamiento.jumpTo(
      destino.clamp(p.minScrollExtent, p.maxScrollExtent).toDouble(),
    );
    widget.motor.arrastrar(inicio, _enDocumento(_raton));
  }

  void _alSubir(PointerEvent e) {
    if (_inicio != null && !_arrastrando) widget.motor.borrarSeleccion();
    _inicio = null;
    _bordes?.cancel();
    _bordes = null;
  }

  /// Cada píxel de la textura, en un píxel de la pantalla: reescalarla emborronaría el texto.
  Widget _textura1a1(int textura, double pixeles) {
    final imagen = Texture(
      textureId: textura,
      filterQuality: FilterQuality.none,
    );
    if (_lienzo.isEmpty) return imagen;
    final tam = _lienzo / pixeles;
    return ClipRect(
      child: OverflowBox(
        alignment: Alignment.topLeft,
        minWidth: tam.width,
        maxWidth: tam.width,
        minHeight: tam.height,
        maxHeight: tam.height,
        child: imagen,
      ),
    );
  }

  @override
  void dispose() {
    for (final s in _suscripciones) {
      s.cancel();
    }
    _bordes?.cancel();
    _foco.dispose();
    _desplazamiento.dispose();
    _canal.invokeMethod<void>('dispose', 0);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final pixeles = MediaQuery.devicePixelRatioOf(context);
    final textura = _textura;
    return LayoutBuilder(
      builder: (context, limites) {
        if (limites.biggest != _tam || pixeles != _pixeles) {
          _tam = limites.biggest;
          _pixeles = pixeles;
          _hayQuePublicar = false;
          _pedirPublicacion();
        }
        return CallbackShortcuts(
          bindings: {
            const SingleActivator(LogicalKeyboardKey.keyC, control: true):
                widget.motor.copiar,
            const SingleActivator(LogicalKeyboardKey.keyA, control: true):
                widget.motor.seleccionarTodo,
          },
          child: Focus(
            focusNode: _foco,
            autofocus: true,
            child: Stack(
              fit: StackFit.expand,
              children: [
                ColoredBox(color: widget.fondo),
                if (textura != null) _textura1a1(textura, pixeles),
                Listener(
                  behavior: HitTestBehavior.translucent,
                  onPointerDown: _alBajar,
                  onPointerMove: _alMover,
                  onPointerUp: _alSubir,
                  onPointerCancel: _alSubir,
                  child: MouseRegion(
                    cursor: SystemMouseCursors.text,
                    child: Scrollbar(
                      controller: _desplazamiento,
                      thumbVisibility: true,
                      child: SingleChildScrollView(
                        controller: _desplazamiento,
                        child: SizedBox(
                          width: double.infinity,
                          height: _publicado.alto * widget.zoom,
                        ),
                      ),
                    ),
                  ),
                ),
                Positioned(
                  left: 12,
                  right: 12,
                  bottom: 8,
                  child: IgnorePointer(
                    child: _Pie(
                      _publicado,
                      _componiendo ? 'Componiendo…' : _medidas,
                      conMuestras: widget.conMuestras,
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _Pie extends StatelessWidget {
  const _Pie(this.publicado, this.medidas, {required this.conMuestras});

  final Publicado publicado;
  final String medidas;
  final bool conMuestras;

  @override
  Widget build(BuildContext context) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    mainAxisSize: MainAxisSize.min,
    children: [
      if (publicado.error case final error?)
        Text(error, style: const TextStyle(color: Colors.red)),
      for (final aviso in publicado.avisos)
        Text(
          '⚠ $aviso',
          style: const TextStyle(fontSize: 11, color: Color(0xFFB06000)),
        ),
      Text(
        conMuestras ? '${publicado.muestras} muestras · $medidas' : medidas,
        style: Theme.of(context).textTheme.bodySmall,
      ),
    ],
  );
}
