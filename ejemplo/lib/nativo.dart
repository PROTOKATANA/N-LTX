import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

typedef _ReservarC = Pointer<Uint8> Function(IntPtr);
typedef _Reservar = Pointer<Uint8> Function(int);
typedef _LiberarC = Void Function(Pointer<Uint8>, IntPtr);
typedef _Liberar = void Function(Pointer<Uint8>, int);
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

/// La biblioteca Rust del ejemplo, en `lib/` junto al ejecutable.
class Nativo {
  Nativo._(this._reservar, this._liberar, this._publicar);

  factory Nativo.abrir() {
    final carpeta = File(Platform.resolvedExecutable).parent.path;
    final lib = DynamicLibrary.open('$carpeta/lib/libejemplo_nativo.so');
    return Nativo._(
      lib.lookupFunction<_ReservarC, _Reservar>('nltx_reservar'),
      lib.lookupFunction<_LiberarC, _Liberar>('nltx_liberar'),
      lib.lookupFunction<_PublicarC, _Publicar>('nltx_publicar'),
    );
  }

  final _Reservar _reservar;
  final _Liberar _liberar;
  final _Publicar _publicar;

  /// Compone `fuente` a `ancho` bp y se la entrega al pintor de la superficie `id`.
  Publicado publicar(String fuente, {required double ancho, int id = 0}) {
    final bytes = utf8.encode(fuente);
    final largo = bytes.isEmpty ? 1 : bytes.length;
    final entrada = _reservar(largo);
    entrada.asTypedList(largo).setAll(0, bytes);
    final celda = _reservar(sizeOf<IntPtr>());
    final salida = _publicar(
      id,
      entrada,
      bytes.length,
      ancho,
      celda.cast<IntPtr>(),
    );
    final largoSalida = celda.cast<IntPtr>().value;
    final texto = utf8.decode(salida.asTypedList(largoSalida));
    _liberar(salida, largoSalida);
    _liberar(celda, sizeOf<IntPtr>());
    _liberar(entrada, largo);
    final j = jsonDecode(texto) as Map<String, dynamic>;
    return Publicado(
      alto: (j['alto'] as num?)?.toDouble() ?? 0,
      avisos: (j['avisos'] as List<dynamic>? ?? const []).cast<String>(),
      error: j['error'] as String?,
    );
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
