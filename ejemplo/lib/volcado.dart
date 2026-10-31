import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import 'escena.dart';
import 'nativo.dart';

/// Escribe todas las escenas en un PNG y termina. Sirve para comprobar el pintor sin depender de la
/// pantalla: `NLTX_VOLCAR=hoja.png ./nltx_ejemplo`.
Future<void> volcar(String ruta, Nativo motor, List<Escena> escenas) async {
  final animaciones = [
    for (final e in escenas) ...e.zonas.map((z) => z.fuente),
  ];
  medir(ruta, motor, animaciones);
  // cuatro fotogramas de cada animación, para ver el movimiento
  escenas = [
    ...escenas.where((e) => e.zonas.isEmpty),
    for (final fuente in animaciones)
      for (final t in [0.0, 0.5, 1.0, 1.5]) motor.fotograma(fuente, t),
  ];
  const ancho = 1500.0, zoom = 1.6, margen = 26.0;
  final grabadora = ui.PictureRecorder();
  final lienzo = Canvas(grabadora);
  final (x0, y0) = (margen, margen);
  var (x, y, alto) = (x0, y0, 0.0);
  final colocadas = <(Escena, double, double)>[];
  for (final e in escenas.where((e) => e.dibujo != null)) {
    final (w, h) = (e.ancho * zoom, e.alto * zoom);
    if (x + w > ancho - margen && x > x0) {
      (x, y, alto) = (x0, y + alto + margen, 0.0);
    }
    colocadas.add((e, x, y));
    x += w + margen;
    if (h > alto) alto = h;
  }
  final total = y + alto + margen;
  lienzo.drawRect(
    const Rect.fromLTWH(0, 0, ancho, 100000),
    Paint()..color = Colors.white,
  );
  for (final (e, px, py) in colocadas) {
    lienzo
      ..save()
      ..translate(px, py)
      ..scale(zoom)
      ..translate(-e.x, -e.y)
      ..drawPicture(e.dibujo!)
      ..restore();
  }
  final imagen = await grabadora.endRecording().toImage(
    ancho.toInt(),
    total.ceil(),
  );
  final png = await imagen.toByteData(format: ui.ImageByteFormat.png);
  File(ruta).writeAsBytesSync(png!.buffer.asUint8List());
  exit(0);
}

/// Mide la cadena de un fotograma: componer, pasar a Dart y grabar la escena.
void medir(String ruta, Nativo motor, List<String> animaciones) {
  final informe = StringBuffer();
  for (final (n, fuente) in animaciones.indexed) {
    final tiempos = <double>[];
    var primitivas = 0;
    for (var i = 0; i < 240; i++) {
      final reloj = Stopwatch()..start();
      final e = motor.fotograma(fuente, i / 60);
      tiempos.add(reloj.elapsedMicroseconds / 1000);
      primitivas = e.primitivas;
      e.liberar();
    }
    final ordenados = tiempos.skip(1).toList()..sort();
    informe.writeln(
      'animación $n: $primitivas primitivas · '
      'mediana ${ordenados[ordenados.length ~/ 2].toStringAsFixed(2)} ms · '
      'p95 ${ordenados[(ordenados.length * 0.95).floor()].toStringAsFixed(2)} ms · '
      'máx ${ordenados.last.toStringAsFixed(2)} ms · '
      'primero ${tiempos.first.toStringAsFixed(1)} ms',
    );
  }
  File('$ruta.tiempos.txt').writeAsStringSync(informe.toString());
}
