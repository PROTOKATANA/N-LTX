import 'package:flutter/material.dart';
import 'package:rinf/rinf.dart';

import 'muestras.dart';
import 'repositorio.dart';
import 'src/bindings/bindings.dart';
import 'vista_documento.dart';
import 'vista_masiva.dart';

Future<void> main() async {
  await initializeRust(assignRustSignal);
  runApp(Ejemplo(motor: RepositorioRinf()));
}

class Ejemplo extends StatelessWidget {
  const Ejemplo({super.key, required this.motor});

  final RepositorioMotor motor;

  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'N-LTX',
    debugShowCheckedModeBanner: false,
    theme: ThemeData(
      colorSchemeSeed: Colors.indigo,
      scaffoldBackgroundColor: const Color(0xFFECEEF3),
    ),
    home: Galeria(motor: motor),
  );
}

class Galeria extends StatefulWidget {
  const Galeria({super.key, required this.motor});

  final RepositorioMotor motor;

  @override
  State<Galeria> createState() => _GaleriaState();
}

class _GaleriaState extends State<Galeria> {
  final _editor = TextEditingController(
    text: r'Prueba: la ley $F=ma$ y el agua \ce{H2O}.',
  );
  double _zoom = 1.6;
  bool _masiva = false;

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(
      title: const Text('N-LTX · Vulkan'),
      backgroundColor: Colors.white,
      actions: [
        SegmentedButton<bool>(
          showSelectedIcon: false,
          segments: const [
            ButtonSegment(value: false, label: Text('Documento')),
            ButtonSegment(value: true, label: Text('Prueba masiva')),
          ],
          selected: {_masiva},
          onSelectionChanged: (s) => setState(() => _masiva = s.first),
        ),
        const SizedBox(width: 20),
        const Icon(Icons.zoom_in, size: 18),
        SizedBox(
          width: 170,
          child: Slider(
            value: _zoom,
            min: 0.8,
            max: 4,
            onChanged: (v) => setState(() => _zoom = v),
          ),
        ),
        const SizedBox(width: 8),
      ],
    ),
    body: _masiva
        ? Center(child: VistaMasiva(motor: widget.motor))
        : Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              SizedBox(
                width: 340,
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: TextField(
                    controller: _editor,
                    expands: true,
                    maxLines: null,
                    textAlignVertical: TextAlignVertical.top,
                    onChanged: (_) => setState(() {}),
                    style: const TextStyle(
                      fontFamily: 'monospace',
                      fontSize: 13,
                    ),
                    decoration: const InputDecoration(
                      labelText: 'Escribe LaTeX',
                      alignLabelWithHint: true,
                      border: OutlineInputBorder(),
                    ),
                  ),
                ),
              ),
              Expanded(
                child: VistaDocumento(
                  motor: widget.motor,
                  fuente: '${_editor.text}\n\n$galeria',
                  zoom: _zoom,
                ),
              ),
            ],
          ),
  );
}
