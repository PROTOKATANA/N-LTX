import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'escena.dart';

typedef _ReservarC = Pointer<Uint8> Function(IntPtr);
typedef _Reservar = Pointer<Uint8> Function(int);
typedef _LiberarC = Void Function(Pointer<Uint8>, IntPtr);
typedef _Liberar = void Function(Pointer<Uint8>, int);
typedef _ComponerC = Pointer<Uint8> Function(
  Pointer<Uint8>,
  IntPtr,
  Int32,
  Pointer<IntPtr>,
);
typedef _Componer = Pointer<Uint8> Function(
  Pointer<Uint8>,
  int,
  int,
  Pointer<IntPtr>,
);
typedef _FotogramaC = Pointer<Uint8> Function(
  Pointer<Uint8>,
  IntPtr,
  Double,
  Pointer<IntPtr>,
);
typedef _Fotograma = Pointer<Uint8> Function(
  Pointer<Uint8>,
  int,
  double,
  Pointer<IntPtr>,
);

typedef _PublicarC = Pointer<Uint8> Function(
  Uint32,
  Pointer<Uint8>,
  IntPtr,
  Double,
  Pointer<IntPtr>,
);
typedef _Publicar = Pointer<Uint8> Function(
  int,
  Pointer<Uint8>,
  int,
  double,
  Pointer<IntPtr>,
);

/// Lo mismo que los modos de `nativo/src/lib.rs`.
enum Modo { formula, figura, documento }

/// La biblioteca Rust del ejemplo, en `lib/` junto al ejecutable.
class Nativo {
  Nativo._(
    this._reservar,
    this._liberar,
    this._componer,
    this._fotograma,
    this._publicar,
  );

  factory Nativo.abrir() {
    final carpeta = File(Platform.resolvedExecutable).parent.path;
    final lib = DynamicLibrary.open('$carpeta/lib/libejemplo_nativo.so');
    return Nativo._(
      lib.lookupFunction<_ReservarC, _Reservar>('nltx_reservar'),
      lib.lookupFunction<_LiberarC, _Liberar>('nltx_liberar'),
      lib.lookupFunction<_ComponerC, _Componer>('nltx_componer'),
      lib.lookupFunction<_FotogramaC, _Fotograma>('nltx_fotograma'),
      lib.lookupFunction<_PublicarC, _Publicar>('nltx_publicar'),
    );
  }

  final _Reservar _reservar;
  final _Liberar _liberar;
  final _Componer _componer;
  final _Fotograma _fotograma;
  final _Publicar _publicar;

  /// Compone `fuente` a `ancho` bp y se la entrega al pintor de la superficie `id`.
  Publicado publicar(String fuente, {required double ancho, int id = 0}) {
    final j = _leer(
      fuente,
      (entrada, largo, celda) => _publicar(id, entrada, largo, ancho, celda),
    );
    return Publicado(
      alto: (j['alto'] as num?)?.toDouble() ?? 0,
      avisos: (j['avisos'] as List<dynamic>? ?? const []).cast<String>(),
      error: j['error'] as String?,
    );
  }

  Escena componer(String fuente, {required Modo modo}) => _llamar(
    fuente,
    (entrada, largo, celda) => _componer(entrada, largo, modo.index, celda),
  );

  /// El fotograma de una animación a los `segundos`.
  Escena fotograma(String fuente, double segundos) => _llamar(
    fuente,
    (entrada, largo, celda) => _fotograma(entrada, largo, segundos, celda),
  );

  Escena _llamar(
    String fuente,
    Pointer<Uint8> Function(Pointer<Uint8>, int, Pointer<IntPtr>) llamada,
  ) => Escena.desdeJson(_leer(fuente, llamada));

  Map<String, dynamic> _leer(
    String fuente,
    Pointer<Uint8> Function(Pointer<Uint8>, int, Pointer<IntPtr>) llamada,
  ) {
    final bytes = utf8.encode(fuente);
    final largo = bytes.isEmpty ? 1 : bytes.length;
    final entrada = _reservar(largo);
    entrada.asTypedList(largo).setAll(0, bytes);
    final celda = _reservar(sizeOf<IntPtr>());
    final salida = llamada(entrada, bytes.length, celda.cast<IntPtr>());
    final largoSalida = celda.cast<IntPtr>().value;
    final texto = utf8.decode(salida.asTypedList(largoSalida));
    _liberar(salida, largoSalida);
    _liberar(celda, sizeOf<IntPtr>());
    _liberar(entrada, largo);
    return jsonDecode(texto) as Map<String, dynamic>;
  }
}

/// Lo que devuelve el motor al recibir un documento.
class Publicado {
  const Publicado({required this.alto, required this.avisos, this.error});

  /// En bp, con el margen.
  final double alto;
  final List<String> avisos;
  final String? error;
}
