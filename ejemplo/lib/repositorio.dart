import 'package:rinf/rinf.dart';

import 'src/bindings/bindings.dart';

/// Lo que devuelve el motor al recibir un documento.
class Publicado {
  const Publicado({required this.alto, required this.avisos, this.error});

  /// En bp, con el margen.
  final double alto;
  final List<String> avisos;
  final String? error;
}

/// Lo que se está midiendo en el pintor.
class Medidas {
  const Medidas({
    required this.vertices,
    required this.msTaller,
    required this.msCarril,
    required this.elementos,
    required this.fpsMasiva,
    required this.msMasiva,
  });

  final int vertices;
  final double msTaller;
  final double msCarril;
  final int elementos;
  final double fpsMasiva;
  final double msMasiva;
}

/// Qué parte del documento se ve, en píxeles físicos.
class Encuadre {
  const Encuadre({
    required this.ancho,
    required this.alto,
    required this.escala,
    required this.y,
  });

  final int ancho;
  final int alto;
  final double escala;
  final double y;
}

/// Cómo habla la interfaz con el motor: los widgets no conocen las señales.
abstract interface class RepositorioMotor {
  /// Compone `fuente` para una vista de `ancho` bp.
  void publicar(String fuente, {required double ancho});

  void ver(Encuadre encuadre);

  Stream<Publicado> get publicados;

  Stream<Medidas> get medidas;
}

/// El repositorio sobre las señales de rinf.
class RepositorioRinf implements RepositorioMotor {
  @override
  void publicar(String fuente, {required double ancho}) =>
      PublicarDocumento(fuente: fuente, ancho: ancho).sendSignalToRust();

  @override
  void ver(Encuadre e) => VerDocumento(
    ancho: e.ancho,
    alto: e.alto,
    escala: e.escala,
    y: e.y,
  ).sendSignalToRust();

  @override
  Stream<Publicado> get publicados => DocumentoPublicado.rustSignalStream.map(
    (RustSignalPack<DocumentoPublicado> s) => Publicado(
      alto: s.message.alto,
      avisos: s.message.avisos,
      error: s.message.error.isEmpty ? null : s.message.error,
    ),
  );

  @override
  Stream<Medidas> get medidas => MedidasPintor.rustSignalStream.map(
    (RustSignalPack<MedidasPintor> s) => Medidas(
      vertices: s.message.vertices.toInt(),
      msTaller: s.message.msTaller,
      msCarril: s.message.msCarril,
      elementos: s.message.elementos.toInt(),
      fpsMasiva: s.message.fpsMasiva,
      msMasiva: s.message.msMasiva,
    ),
  );
}
