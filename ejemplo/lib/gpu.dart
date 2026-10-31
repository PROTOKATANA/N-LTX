import 'dart:async';
import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';

import 'nativo.dart';

/// El motor de la aplicación. Lo abre `main`.
late Nativo motor;

/// Margen de la escena: `pintor::MARGEN`.
const _margen = 16.0;

/// Un documento en la superficie Vulkan: Flutter solo muestra la textura y lleva el desplazamiento.
class VistaDocumento extends StatefulWidget {
  const VistaDocumento({super.key, required this.fuente, required this.zoom});

  final String fuente;
  final double zoom;

  @override
  State<VistaDocumento> createState() => _VistaDocumentoState();
}

class _VistaDocumentoState extends State<VistaDocumento> {
  static const _canal = MethodChannel('ltx/lienzo');

  final _desplazamiento = ScrollController();
  int? _textura;
  Size _tam = Size.zero;
  double _pixeles = 1;
  Publicado _publicado = const Publicado(alto: 0, avisos: []);
  String _medidas = '';
  Timer? _reloj;
  bool _hayQuePublicar = true;

  @override
  void initState() {
    super.initState();
    _desplazamiento.addListener(_verVista);
    _abrir();
  }

  Future<void> _abrir() async {
    final id = await _canal.invokeMethod<int>('create', 0);
    if (!mounted) return;
    setState(() => _textura = id);
    _publicar();
    _reloj = Timer.periodic(const Duration(seconds: 1), (_) => _medir());
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
    final ancho = max(_tam.width / widget.zoom - 2 * _margen, 40.0);
    final publicado = motor.publicar(widget.fuente, ancho: ancho);
    setState(() => _publicado = publicado);
    _verVista();
  }

  /// En píxeles físicos, y el desplazamiento entero para que el texto no se emborrone.
  void _verVista() {
    final y = _desplazamiento.hasClients ? _desplazamiento.offset : 0.0;
    _canal.invokeMethod<void>('vista', [
      (_tam.width * _pixeles).round(),
      (_tam.height * _pixeles).round(),
      widget.zoom * _pixeles,
      (y * _pixeles).roundToDouble(),
    ]);
  }

  Future<void> _medir() async {
    final m = await _canal.invokeListMethod<Object?>('medidas');
    if (!mounted || m == null) return;
    final (vertices, ms) = (m[0]! as int, m[1]! as double);
    setState(() {
      _medidas = '$vertices vértices · ${ms.toStringAsFixed(2)} ms de CPU';
    });
  }

  @override
  void dispose() {
    _reloj?.cancel();
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
            if (textura == null)
              const ColoredBox(color: Colors.white)
            else
              Texture(textureId: textura),
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

/// La prueba masiva: geometría subida una vez que el vertex shader mueve en cada fotograma.
class VistaMasiva extends StatefulWidget {
  const VistaMasiva({super.key});

  @override
  State<VistaMasiva> createState() => _VistaMasivaState();
}

class _VistaMasivaState extends State<VistaMasiva> {
  static const _canal = MethodChannel('ltx/masiva');

  int? _textura;
  String _medidas = 'arrancando Vulkan…';
  Timer? _reloj;

  @override
  void initState() {
    super.initState();
    _abrir();
  }

  Future<void> _abrir() async {
    final id = await _canal.invokeMethod<int>('create', 0);
    if (!mounted) return;
    setState(() => _textura = id);
    await _canal.invokeMethod<void>('mark', 0);
    _reloj = Timer.periodic(const Duration(seconds: 1), (_) => _medir());
  }

  Future<void> _medir() async {
    final m = await _canal.invokeListMethod<Object?>('medidas', 0);
    if (!mounted || m == null) return;
    final (elementos, fps, ms) = (
      m[0]! as int,
      m[1]! as double,
      m[2]! as double,
    );
    setState(() {
      _medidas =
          '$elementos elementos · ${fps.toStringAsFixed(0)} fps · '
          '${ms.toStringAsFixed(2)} ms de CPU por fotograma';
    });
  }

  @override
  void dispose() {
    _reloj?.cancel();
    _canal.invokeMethod<void>('dispose', 0);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final textura = _textura;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(_medidas, style: Theme.of(context).textTheme.bodySmall),
        const SizedBox(height: 6),
        SizedBox(
          width: 960,
          height: 540,
          child: textura == null
              ? const ColoredBox(color: Colors.black)
              : Texture(textureId: textura),
        ),
      ],
    );
  }
}
