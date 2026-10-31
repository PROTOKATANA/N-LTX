import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'repositorio.dart';

/// La prueba masiva: geometría subida una vez que el vertex shader mueve en cada fotograma.
class VistaMasiva extends StatefulWidget {
  const VistaMasiva({super.key, required this.motor});

  final RepositorioMotor motor;

  @override
  State<VistaMasiva> createState() => _VistaMasivaState();
}

class _VistaMasivaState extends State<VistaMasiva> {
  static const _canal = MethodChannel('ltx/masiva');

  int? _textura;
  String _medidas = 'arrancando Vulkan…';
  StreamSubscription<Medidas>? _suscripcion;

  @override
  void initState() {
    super.initState();
    _suscripcion = widget.motor.medidas.listen((m) {
      if (m.elementos == 0) return;
      setState(() {
        _medidas =
            '${m.elementos} elementos · ${m.fpsMasiva.toStringAsFixed(0)} fps · '
            '${m.msMasiva.toStringAsFixed(2)} ms de CPU por fotograma';
      });
    });
    _abrir();
  }

  Future<void> _abrir() async {
    final id = await _canal.invokeMethod<int>('create', 0);
    if (!mounted) return;
    setState(() => _textura = id);
    await _canal.invokeMethod<void>('mark', 0);
  }

  @override
  void dispose() {
    _suscripcion?.cancel();
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
