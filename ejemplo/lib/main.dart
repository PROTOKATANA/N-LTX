import 'package:flutter/material.dart';
import 'package:rinf/rinf.dart';

import 'k1.dart';
import 'muestras.dart';
import 'repositorio.dart';
import 'src/bindings/bindings.dart';
import 'vista_documento.dart';

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

enum Pantalla { documento, muestras, k1 }

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
  bool _oscuro = false;
  Pantalla _pantalla = Pantalla.documento;
  String _k1 = '';

  @override
  void initState() {
    super.initState();
    cargarK1().then((fuente) {
      if (mounted) setState(() => _k1 = fuente);
    });
  }

  @override
  void dispose() {
    _editor.dispose();
    super.dispose();
  }

  String get _fuente => switch (_pantalla) {
    Pantalla.k1 => _k1,
    Pantalla.muestras => _editor.text,
    Pantalla.documento => '${_editor.text}\n\n$galeria',
  };

  Widget _escritura() => SizedBox(
    width: 340,
    child: Padding(
      padding: const EdgeInsets.all(12),
      child: TextField(
        controller: _editor,
        expands: true,
        maxLines: null,
        textAlignVertical: TextAlignVertical.top,
        onChanged: (_) => setState(() {}),
        style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
        decoration: const InputDecoration(
          labelText: 'Escribe LaTeX',
          alignLabelWithHint: true,
          border: OutlineInputBorder(),
        ),
      ),
    ),
  );

  Widget _panel() => Row(
    crossAxisAlignment: CrossAxisAlignment.stretch,
    children: [
      if (_pantalla != Pantalla.k1) _escritura(),
      Expanded(
        child: VistaDocumento(
          motor: widget.motor,
          pagina: _pantalla,
          fuente: _fuente,
          zoom: _zoom,
          fondo: _oscuro ? const Color(0xFF575965) : Colors.white,
          tinta: _oscuro ? Colors.white : Colors.black,
          conMuestras: _pantalla == Pantalla.muestras,
        ),
      ),
    ],
  );

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
            ButtonSegment(value: Pantalla.k1, label: Text('K1')),
          ],
          selected: {_pantalla},
          onSelectionChanged: (s) => setState(() => _pantalla = s.first),
        ),
        const SizedBox(width: 12),
        IconButton(
          tooltip: 'Fondo oscuro',
          isSelected: _oscuro,
          icon: const Icon(Icons.dark_mode_outlined),
          selectedIcon: const Icon(Icons.dark_mode),
          onPressed: () => setState(() => _oscuro = !_oscuro),
        ),
        const SizedBox(width: 8),
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
    body: _panel(),
  );
}
