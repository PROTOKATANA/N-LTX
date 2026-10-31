import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import 'escena.dart';

/// Escribe todas las escenas en un PNG y termina. Sirve para comprobar el pintor sin depender de la
/// pantalla: `NLTX_VOLCAR=hoja.png ./nltx_ejemplo`.
Future<void> volcar(String ruta, List<Escena> escenas) async {
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
