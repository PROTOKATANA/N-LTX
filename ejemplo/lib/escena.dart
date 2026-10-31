import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';

/// Pintor de referencia de la lista neutra, con el Canvas de Flutter.
///
/// La escena se graba en un `Picture`: sigue siendo vectorial, así que se ve nítida a cualquier
/// zoom y grabarla solo se hace una vez.
class Escena {
  Escena._({
    this.error,
    this.dibujo,
    this.x = 0,
    this.y = 0,
    this.ancho = 0,
    this.alto = 0,
    this.avisos = const [],
    this.primitivas = 0,
  });

  factory Escena.desdeJson(Map<String, dynamic> j) {
    if (j['error'] != null) return Escena._(error: j['error'] as String);
    final caja = _doubles(j['caja']);
    final lista = j['primitivas'] as List<dynamic>;
    final grabadora = ui.PictureRecorder();
    final lienzo = Canvas(grabadora);
    final fondo = j['fondo'];
    if (fondo != null) {
      lienzo.drawRect(
        Rect.fromLTWH(caja[0], caja[1], caja[2], caja[3]),
        Paint()..color = _color(_doubles(fondo)),
      );
    }
    _Pintor(lienzo, j['contornos'] as Map<String, dynamic>).pintar(lista);
    return Escena._(
      dibujo: grabadora.endRecording(),
      x: caja[0],
      y: caja[1],
      ancho: caja[2],
      alto: caja[3],
      avisos: (j['avisos'] as List<dynamic>).cast<String>(),
      primitivas: lista.length,
    );
  }

  final String? error;
  final ui.Picture? dibujo;
  final double x, y, ancho, alto;
  final List<String> avisos;
  final int primitivas;
}

List<double> _doubles(dynamic v) => [
  for (final n in v as List<dynamic>) (n as num).toDouble(),
];

Color _color(List<double> c, [double opacidad = 1]) =>
    Color.from(alpha: opacidad, red: c[0], green: c[1], blue: c[2]);

Float64List _matriz(dynamic v) {
  final m = _doubles(v);
  return Float64List.fromList([
    m[0], m[1], 0, 0, //
    m[2], m[3], 0, 0, //
    0, 0, 1, 0, //
    m[4], m[5], 0, 1,
  ]);
}

Path _ruta(dynamic plano, {bool parImpar = false}) {
  final v = _doubles(plano);
  final p = Path()
    ..fillType = parImpar ? PathFillType.evenOdd : PathFillType.nonZero;
  var i = 0;
  while (i < v.length) {
    switch (v[i].toInt()) {
      case 0:
        p.moveTo(v[i + 1], v[i + 2]);
        i += 3;
      case 1:
        p.lineTo(v[i + 1], v[i + 2]);
        i += 3;
      case 2:
        p.cubicTo(v[i + 1], v[i + 2], v[i + 3], v[i + 4], v[i + 5], v[i + 6]);
        i += 7;
      default:
        p.close();
        i += 1;
    }
  }
  return p;
}

/// Flutter no sabe de trazos discontinuos: se recortan a mano.
Path _discontinua(Path ruta, List<double> tramos, double fase) {
  final ciclo = tramos.length.isOdd ? [...tramos, ...tramos] : tramos;
  final total = ciclo.fold<double>(0, (a, b) => a + b);
  final salida = Path();
  if (total <= 0) return ruta;
  for (final medida in ruta.computeMetrics()) {
    var d = -(fase % total);
    var i = 0;
    while (d < medida.length) {
      final fin = d + ciclo[i];
      if (i.isEven) {
        final desde = d < 0 ? 0.0 : d;
        // un tramo de largo cero con remate redondo es un punto
        final hasta = fin <= desde ? desde + 0.01 : fin;
        salida.addPath(medida.extractPath(desde, hasta), Offset.zero);
      }
      d = fin;
      i = (i + 1) % ciclo.length;
    }
  }
  return salida;
}

const _mezclas = {
  'multiply': BlendMode.multiply,
  'screen': BlendMode.screen,
  'overlay': BlendMode.overlay,
  'darken': BlendMode.darken,
  'lighten': BlendMode.lighten,
  'color-dodge': BlendMode.colorDodge,
  'color-burn': BlendMode.colorBurn,
  'hard-light': BlendMode.hardLight,
  'soft-light': BlendMode.softLight,
  'difference': BlendMode.difference,
  'exclusion': BlendMode.exclusion,
  'hue': BlendMode.hue,
  'saturation': BlendMode.saturation,
  'color': BlendMode.color,
  'luminosity': BlendMode.luminosity,
};

class _Pintor {
  _Pintor(this._lienzo, this._contornos);

  final Canvas _lienzo;
  final Map<String, dynamic> _contornos;
  final _glifos = <String, Path>{};

  void pintar(List<dynamic> primitivas) {
    for (final p in primitivas) {
      final m = p as Map<String, dynamic>;
      switch (m['t']) {
        case 'trazado':
          _trazado(m);
        case 'glifo':
          _glifo(m);
        case 'abrir':
          _abrir(m);
        default:
          _lienzo.restore();
      }
    }
  }

  void _glifo(Map<String, dynamic> m) {
    final clave = m['g'] as String;
    final ruta = _glifos[clave] ??= _ruta(_contornos[clave]);
    _lienzo
      ..save()
      ..transform(_matriz(m['m']))
      ..drawPath(ruta, Paint()..color = _color(_doubles(m['v'])))
      ..restore();
  }

  void _abrir(Map<String, dynamic> m) {
    switch (m['c']) {
      case 'recorte':
        _lienzo
          ..save()
          ..clipPath(_ruta(m['s'], parImpar: m['e'] as bool));
      case 'grupo':
        _lienzo.saveLayer(
          null,
          Paint()
            ..color = Color.from(
              alpha: (m['o'] as num).toDouble(),
              red: 0,
              green: 0,
              blue: 0,
            ),
        );
      case 'matriz':
        _lienzo
          ..save()
          ..transform(_matriz(m['m']));
      case 'mezcla':
        _lienzo.saveLayer(
          null,
          Paint()..blendMode = _mezclas[m['n']] ?? BlendMode.srcOver,
        );
      default:
        _lienzo.save();
    }
  }

  void _trazado(Map<String, dynamic> m) {
    final relleno = m['r'] as Map<String, dynamic>?;
    final trazo = m['k'] as Map<String, dynamic>?;
    // relleno y trazo con la misma opacidad son un grupo: el trazo no oscurece el relleno
    final compartida =
        relleno != null &&
            trazo != null &&
            relleno['o'] == trazo['o'] &&
            (relleno['o'] as num) < 1
        ? (relleno['o'] as num).toDouble()
        : null;
    if (compartida != null) {
      _lienzo.saveLayer(
        null,
        Paint()
          ..color = Color.from(alpha: compartida, red: 0, green: 0, blue: 0),
      );
    }
    final opacidad = compartida == null ? null : 1.0;
    if (relleno != null) {
      final ruta = _ruta(m['s'], parImpar: relleno['e'] as bool);
      _lienzo.drawPath(ruta, _pinturaRelleno(relleno, opacidad));
    }
    if (trazo != null) {
      final ruta = _ruta(m['s']);
      final tramos = trazo['d'];
      final linea = tramos == null
          ? ruta
          : _discontinua(
              ruta,
              _doubles(tramos),
              (trazo['f'] as num).toDouble(),
            );
      _lienzo.drawPath(linea, _pinturaTrazo(trazo, opacidad));
    }
    if (compartida != null) _lienzo.restore();
  }

  Paint _pinturaRelleno(Map<String, dynamic> r, double? forzada) {
    final opacidad = forzada ?? (r['o'] as num).toDouble();
    final pintura = r['p'] as Map<String, dynamic>;
    final paint = Paint()..style = PaintingStyle.fill;
    if (pintura['c'] == 'solida') {
      return paint..color = _color(_doubles(pintura['v']), opacidad);
    }
    final paradas = [
      for (final p in pintura['p'] as List<dynamic>) _doubles(p),
    ];
    final colores = [
      for (final p in paradas) _color([p[1], p[2], p[3]]),
    ];
    final posiciones = [for (final p in paradas) p[0]];
    paint.color = Color.from(alpha: opacidad, red: 0, green: 0, blue: 0);
    if (pintura['c'] == 'lineal') {
      final a = _doubles(pintura['a']);
      final b = _doubles(pintura['b']);
      return paint
        ..shader = ui.Gradient.linear(
          Offset(a[0], a[1]),
          Offset(b[0], b[1]),
          colores,
          posiciones,
          TileMode.clamp,
          _matriz(pintura['m']),
        );
    }
    final f = _doubles(pintura['f']);
    return paint
      ..shader = ui.Gradient.radial(
        Offset.zero,
        (pintura['r'] as num).toDouble(),
        colores,
        posiciones,
        TileMode.clamp,
        _matriz(pintura['m']),
        Offset(f[0], f[1]),
        0,
      );
  }

  Paint _pinturaTrazo(Map<String, dynamic> t, double? forzada) {
    final opacidad = forzada ?? (t['o'] as num).toDouble();
    return Paint()
      ..style = PaintingStyle.stroke
      ..color = _color(_doubles(t['v']), opacidad)
      ..strokeWidth = (t['w'] as num).toDouble()
      ..strokeCap = const [
        StrokeCap.butt,
        StrokeCap.round,
        StrokeCap.square,
      ][t['c'] as int]
      ..strokeJoin = const [
        StrokeJoin.miter,
        StrokeJoin.round,
        StrokeJoin.bevel,
      ][t['j'] as int]
      ..strokeMiterLimit = (t['l'] as num).toDouble();
  }
}

class _PintorWidget extends CustomPainter {
  _PintorWidget(this.escena, this.zoom);

  final Escena escena;
  final double zoom;

  @override
  void paint(Canvas canvas, Size size) {
    canvas
      ..scale(zoom)
      ..translate(-escena.x, -escena.y)
      ..drawPicture(escena.dibujo!);
  }

  @override
  bool shouldRepaint(_PintorWidget old) =>
      old.escena != escena || old.zoom != zoom;
}

/// Una escena a escala `zoom`.
class VistaEscena extends StatelessWidget {
  const VistaEscena({super.key, required this.escena, required this.zoom});

  final Escena escena;
  final double zoom;

  @override
  Widget build(BuildContext context) => CustomPaint(
    size: Size(escena.ancho * zoom, escena.alto * zoom),
    painter: _PintorWidget(escena, zoom),
  );
}
