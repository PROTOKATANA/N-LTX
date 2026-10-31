import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

const _canal = MethodChannel('ltx/textura');

/// La superficie Vulkan del pintor: Flutter solo muestra la textura.
class VistaGpu extends StatefulWidget {
  const VistaGpu({super.key});

  @override
  State<VistaGpu> createState() => _VistaGpuState();
}

class _VistaGpuState extends State<VistaGpu> {
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
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final textura = _textura;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          'Vulkan · $_medidas',
          style: Theme.of(context).textTheme.bodySmall,
        ),
        const SizedBox(height: 6),
        ClipRRect(
          borderRadius: BorderRadius.circular(8),
          child: SizedBox(
            width: 960,
            height: 540,
            child: textura == null
                ? const ColoredBox(color: Colors.black)
                : Texture(textureId: textura),
          ),
        ),
      ],
    );
  }
}
