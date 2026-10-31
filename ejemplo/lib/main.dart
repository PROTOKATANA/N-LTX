import 'dart:async';

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

enum Pantalla { documento, masiva, muestras }

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
  final _editorMuestra = TextEditingController();
  late final StreamSubscription<ArchivoMuestra> _muestras;
  ArchivoMuestra? _muestra;
  double _zoom = 1.6;
  Pantalla _pantalla = Pantalla.documento;

  @override
  void initState() {
    super.initState();
    _muestras = widget.motor.muestras.listen((m) {
      setState(() {
        _muestra = m;
        _editorMuestra.text = m.fuente;
      });
    });
  }

  @override
  void dispose() {
    _muestras.cancel();
    _editor.dispose();
    _editorMuestra.dispose();
    super.dispose();
  }

  void _elegir(Pantalla p) {
    setState(() => _pantalla = p);
    if (p == Pantalla.muestras && _muestra == null) {
      widget.motor.pedirMuestra(0);
    }
  }

  /// Da la vuelta en los dos extremos.
  void _mover(int paso) {
    final m = _muestra;
    if (m == null || m.total == 0) return;
    widget.motor.pedirMuestra((m.indice + paso + m.total) % m.total);
  }

  Widget _panel({required TextEditingController editor, Widget? cabecera}) =>
      Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          SizedBox(
            width: 340,
            child: Padding(
              padding: const EdgeInsets.all(12),
              child: Column(
                children: [
                  ?cabecera,
                  Expanded(
                    child: TextField(
                      controller: editor,
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
                ],
              ),
            ),
          ),
          Expanded(
            child: VistaDocumento(
              motor: widget.motor,
              fuente: editor == _editorMuestra
                  ? editor.text
                  : '${editor.text}\n\n$galeria',
              zoom: _zoom,
            ),
          ),
        ],
      );

  Widget _barraMuestras() {
    final m = _muestra;
    return Row(
      children: [
        IconButton(
          tooltip: 'Anterior',
          onPressed: () => _mover(-1),
          icon: const Icon(Icons.chevron_left),
        ),
        Expanded(
          child: Text(
            m == null || m.total == 0
                ? 'Sin muestras: arranca con NLTX_MUESTRAS=<carpeta>'
                : '${m.indice + 1} de ${m.total} · ${m.nombre}',
            textAlign: TextAlign.center,
            overflow: TextOverflow.ellipsis,
            style: Theme.of(context).textTheme.bodySmall,
          ),
        ),
        IconButton(
          tooltip: 'Siguiente',
          onPressed: () => _mover(1),
          icon: const Icon(Icons.chevron_right),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) => Scaffold(
    appBar: AppBar(
      title: const Text('N-LTX · Vulkan'),
      backgroundColor: Colors.white,
      actions: [
        SegmentedButton<Pantalla>(
          showSelectedIcon: false,
          segments: const [
            ButtonSegment(value: Pantalla.documento, label: Text('Documento')),
            ButtonSegment(value: Pantalla.muestras, label: Text('Muestras')),
            ButtonSegment(value: Pantalla.masiva, label: Text('Prueba masiva')),
          ],
          selected: {_pantalla},
          onSelectionChanged: (s) => _elegir(s.first),
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
    body: switch (_pantalla) {
      Pantalla.masiva => Center(child: VistaMasiva(motor: widget.motor)),
      Pantalla.documento => _panel(editor: _editor),
      Pantalla.muestras => _panel(
        editor: _editorMuestra,
        cabecera: _barraMuestras(),
      ),
    },
  );
}
