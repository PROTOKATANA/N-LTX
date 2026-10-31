import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';

import 'repositorio.dart';

/// Un documento en la superficie Vulkan: Flutter muestra la textura y lleva el desplazamiento.
class VistaDocumento extends StatefulWidget {
  const VistaDocumento({
    super.key,
    required this.motor,
    required this.fuente,
    required this.zoom,
  });

  final RepositorioMotor motor;
  final String fuente;
  final double zoom;

  @override
  State<VistaDocumento> createState() => _VistaDocumentoState();
}

class _VistaDocumentoState extends State<VistaDocumento> {
  /// Solo para lo que exige el motor de Flutter: la textura y su tamaño.
  static const _canal = MethodChannel('ltx/lienzo');

  final _desplazamiento = ScrollController();
  int? _textura;

  /// El tamaño de la textura, en píxeles físicos. No cambia con la ventana: solo se pinta lo que se
  /// ve, y por eso redimensionar no la recrea.
  Size _lienzo = Size.zero;
  Size _tam = Size.zero;
  double _pixeles = 1;
  Publicado _publicado = const Publicado(alto: 0, avisos: []);
  String _medidas = '';
  bool _hayQuePublicar = true;
  final _suscripciones = <StreamSubscription<Object?>>[];

  @override
  void initState() {
    super.initState();
    _desplazamiento.addListener(_verVista);
    _suscripciones
      ..add(
        widget.motor.publicados.listen((p) => setState(() => _publicado = p)),
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
    if (antes.fuente != widget.fuente || antes.zoom != widget.zoom) {
      _pedirPublicacion();
    }
  }

  void _pedirPublicacion() {
    if (_hayQuePublicar) return;
    _hayQuePublicar = true;
    SchedulerBinding.instance.addPostFrameCallback((_) => _publicar());
  }

  void _publicar() {
    if (!mounted || _textura == null || _tam.isEmpty) return;
    _hayQuePublicar = false;
    widget.motor.publicar(widget.fuente, ancho: _tam.width / widget.zoom);
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
        return Stack(
          fit: StackFit.expand,
          children: [
            const ColoredBox(color: Colors.white),
            if (textura != null) _textura1a1(textura, pixeles),
            Scrollbar(
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
            Positioned(
              left: 12,
              right: 12,
              bottom: 8,
              child: IgnorePointer(child: _Pie(_publicado, _medidas)),
            ),
          ],
        );
      },
    );
  }
}

class _Pie extends StatelessWidget {
  const _Pie(this.publicado, this.medidas);

  final Publicado publicado;
  final String medidas;

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
      Text(medidas, style: Theme.of(context).textTheme.bodySmall),
    ],
  );
}
