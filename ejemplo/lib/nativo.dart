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

/// La biblioteca Rust del ejemplo, en `lib/` junto al ejecutable.
class Nativo {
  Nativo._(this._reservar, this._liberar, this._componer);

  factory Nativo.abrir() {
    final carpeta = File(Platform.resolvedExecutable).parent.path;
    final lib = DynamicLibrary.open('$carpeta/lib/libejemplo_nativo.so');
    return Nativo._(
      lib.lookupFunction<_ReservarC, _Reservar>('nltx_reservar'),
      lib.lookupFunction<_LiberarC, _Liberar>('nltx_liberar'),
      lib.lookupFunction<_ComponerC, _Componer>('nltx_componer'),
    );
  }

  final _Reservar _reservar;
  final _Liberar _liberar;
  final _Componer _componer;

  Escena componer(String fuente, {required bool figura}) {
    final bytes = utf8.encode(fuente);
    final largo = bytes.isEmpty ? 1 : bytes.length;
    final entrada = _reservar(largo);
    entrada.asTypedList(largo).setAll(0, bytes);
    final celda = _reservar(sizeOf<IntPtr>());
    final salida = _componer(
      entrada,
      bytes.length,
      figura ? 1 : 0,
      celda.cast<IntPtr>(),
    );
    final largoSalida = celda.cast<IntPtr>().value;
    final texto = utf8.decode(salida.asTypedList(largoSalida));
    _liberar(salida, largoSalida);
    _liberar(celda, sizeOf<IntPtr>());
    _liberar(entrada, largo);
    return Escena.desdeJson(jsonDecode(texto) as Map<String, dynamic>);
  }
}
