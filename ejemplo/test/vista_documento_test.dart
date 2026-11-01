import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:nltx_ejemplo/repositorio.dart';
import 'package:nltx_ejemplo/vista_documento.dart';

/// El motor de mentira: apunta lo que le piden.
class MotorFalso implements RepositorioMotor {
  final arrastres = <(Offset, Offset)>[];
  final palabras = <Offset>[];
  var todos = 0;
  var borrados = 0;
  var copias = 0;
  final _publicados = StreamController<Publicado>.broadcast();
  final _copiados = StreamController<String>.broadcast();

  void publicado(double alto) =>
      _publicados.add(Publicado(alto: alto, avisos: const []));

  void dar(String texto) => _copiados.add(texto);

  @override
  void publicar(
    String fuente, {
    required double ancho,
    bool conMuestras = false,
  }) {}

  @override
  void ver(Encuadre encuadre) {}

  @override
  void arrastrar(Offset desde, Offset hasta) => arrastres.add((desde, hasta));

  @override
  void seleccionarPalabra(Offset punto) => palabras.add(punto);

  @override
  void seleccionarTodo() => todos++;

  @override
  void borrarSeleccion() => borrados++;

  @override
  void copiar() => copias++;

  @override
  Stream<String> get copiados => _copiados.stream;

  @override
  Stream<Publicado> get publicados => _publicados.stream;

  @override
  Stream<Medidas> get medidas => const Stream.empty();
}

const _zoom = 2.0;

Future<MotorFalso> montar(WidgetTester t) async {
  final mensajero = t.binding.defaultBinaryMessenger;
  mensajero.setMockMethodCallHandler(const MethodChannel('ltx/lienzo'), (
    llamada,
  ) async {
    return switch (llamada.method) {
      'create' => 1,
      'lienzo' => [800, 600],
      _ => null,
    };
  });
  addTearDown(
    () => mensajero.setMockMethodCallHandler(
      const MethodChannel('ltx/lienzo'),
      null,
    ),
  );
  await t.binding.setSurfaceSize(const Size(800, 600));
  addTearDown(() => t.binding.setSurfaceSize(null));
  final motor = MotorFalso();
  await t.pumpWidget(
    MaterialApp(
      home: VistaDocumento(
        motor: motor,
        pagina: 0,
        fuente: 'hola',
        zoom: _zoom,
      ),
    ),
  );
  await t.pumpAndSettle();
  motor.publicado(5000);
  await t.pump();
  // el aviso llega tras el primer fotograma, y su `setState` pide otro
  await t.pump();
  return motor;
}

Future<TestGesture> raton(WidgetTester t, Offset donde) async {
  final g = await t.startGesture(
    donde,
    kind: PointerDeviceKind.mouse,
    buttons: kPrimaryMouseButton,
  );
  addTearDown(g.removePointer);
  return g;
}

void main() {
  testWidgets('arrastrar selecciona, en bp del documento', (t) async {
    final motor = await montar(t);
    final g = await raton(t, const Offset(100, 100));
    await g.moveTo(const Offset(300, 180));
    await g.up();
    await t.pump();
    expect(motor.arrastres.last, (const Offset(50, 50), const Offset(150, 90)));
    expect(motor.borrados, 0);
  });

  testWidgets('un clic sin mover quita la selección', (t) async {
    final motor = await montar(t);
    final g = await raton(t, const Offset(100, 100));
    await g.up();
    await t.pump();
    expect(motor.arrastres, isEmpty);
    expect(motor.borrados, 1);
  });

  testWidgets('un temblor de la mano no es un arrastre', (t) async {
    final motor = await montar(t);
    final g = await raton(t, const Offset(100, 100));
    await g.moveTo(const Offset(101, 101));
    await g.up();
    await t.pump();
    expect(motor.arrastres, isEmpty);
  });

  testWidgets('el doble clic selecciona la palabra', (t) async {
    final motor = await montar(t);
    final primero = await raton(t, const Offset(100, 100));
    await primero.up();
    await t.pump(const Duration(milliseconds: 50));
    final segundo = await raton(t, const Offset(100, 100));
    await segundo.up();
    await t.pump();
    expect(motor.palabras, [const Offset(50, 50)]);
  });

  testWidgets('la barra de desplazamiento no selecciona', (t) async {
    final motor = await montar(t);
    final g = await raton(t, const Offset(795, 100));
    await g.moveTo(const Offset(795, 300));
    await g.up();
    await t.pump();
    expect(motor.arrastres, isEmpty);
    expect(motor.borrados, 0);
  });

  testWidgets('con el dedo se desplaza: no selecciona', (t) async {
    final motor = await montar(t);
    final g = await t.startGesture(const Offset(100, 300));
    addTearDown(g.removePointer);
    await g.moveTo(const Offset(100, 100));
    await g.up();
    await t.pump();
    expect(motor.arrastres, isEmpty);
  });

  testWidgets('tras desplazar, el punto cae más abajo en el documento', (
    t,
  ) async {
    final motor = await montar(t);
    final barra = t.state<ScrollableState>(find.byType(Scrollable));
    barra.position.jumpTo(1000);
    await t.pump();
    final g = await raton(t, const Offset(100, 100));
    await g.moveTo(const Offset(100, 200));
    await g.up();
    await t.pump();
    expect(motor.arrastres.last, (
      const Offset(50, 550),
      const Offset(50, 600),
    ));
  });

  testWidgets('arrastrar contra un borde sigue bajando sin mover el ratón', (
    t,
  ) async {
    final motor = await montar(t);
    final g = await raton(t, const Offset(100, 100));
    await g.moveTo(const Offset(100, 595));
    final cuantos = motor.arrastres.length;
    await t.pump(const Duration(milliseconds: 200));
    final barra = t.state<ScrollableState>(find.byType(Scrollable));
    expect(barra.position.pixels, greaterThan(0));
    expect(motor.arrastres.length, greaterThan(cuantos));
    final (_, hasta) = motor.arrastres.last;
    expect(hasta.dy, (595 + barra.position.pixels) / _zoom);
    await g.up();
    await t.pump();
    final parado = barra.position.pixels;
    await t.pump(const Duration(milliseconds: 200));
    expect(barra.position.pixels, parado, reason: 'al soltar se para');
  });

  testWidgets('Ctrl+C copia y Ctrl+A selecciona todo', (t) async {
    final motor = await montar(t);
    await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await t.sendKeyEvent(LogicalKeyboardKey.keyC);
    await t.sendKeyEvent(LogicalKeyboardKey.keyA);
    await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    expect(motor.copias, 1);
    expect(motor.todos, 1);
  });

  testWidgets('lo copiado llega al portapapeles; lo vacío no lo pisa', (
    t,
  ) async {
    final motor = await montar(t);
    final guardado = <String>[];
    final mensajero = t.binding.defaultBinaryMessenger;
    mensajero.setMockMethodCallHandler(SystemChannels.platform, (
      llamada,
    ) async {
      if (llamada.method == 'Clipboard.setData') {
        final datos = llamada.arguments as Map<Object?, Object?>;
        guardado.add(datos['text']! as String);
      }
      return null;
    });
    addTearDown(
      () => mensajero.setMockMethodCallHandler(SystemChannels.platform, null),
    );
    motor.dar('');
    motor.dar(r'\textbf{hola} $x$');
    await t.pump();
    expect(guardado, [r'\textbf{hola} $x$']);
  });
}
