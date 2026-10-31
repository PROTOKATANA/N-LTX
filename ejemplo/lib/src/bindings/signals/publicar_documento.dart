// ignore_for_file: type=lint, type=warning
part of 'signals.dart';

/// Dart → Rust: el documento que hay que mostrar en una vista de `ancho` bp, seguido o no de todas
/// las muestras. Las muestras las tiene Rust: no cruzan Dart.
@immutable
class PublicarDocumento {
  const PublicarDocumento({
    required this.fuente,
    required this.ancho,
    required this.conMuestras,
  });

  static PublicarDocumento deserialize(BinaryDeserializer deserializer) {
    deserializer.increaseContainerDepth();
    final instance = PublicarDocumento(
      fuente: deserializer.deserializeString(),
      ancho: deserializer.deserializeFloat64(),
      conMuestras: deserializer.deserializeBool(),
    );
    deserializer.decreaseContainerDepth();
    return instance;
  }

  static PublicarDocumento bincodeDeserialize(Uint8List input) {
    final deserializer = BincodeDeserializer(input);
    final value = PublicarDocumento.deserialize(deserializer);
    if (deserializer.offset < input.length) {
      throw Exception('Some input bytes were not read');
    }
    return value;
  }

  final String fuente;
  final double ancho;
  final bool conMuestras;

  PublicarDocumento copyWith({
    String? fuente,
    double? ancho,
    bool? conMuestras,
  }) {
    return PublicarDocumento(
      fuente: fuente ?? this.fuente,
      ancho: ancho ?? this.ancho,
      conMuestras: conMuestras ?? this.conMuestras,
    );
  }

  void serialize(BinarySerializer serializer) {
    serializer.increaseContainerDepth();
    serializer.serializeString(fuente);
    serializer.serializeFloat64(ancho);
    serializer.serializeBool(conMuestras);
    serializer.decreaseContainerDepth();
  }

  Uint8List bincodeSerialize() {
    final serializer = BincodeSerializer();
    serialize(serializer);
    return serializer.bytes;
  }

  @override
  bool operator ==(Object other) {
    if (identical(this, other)) return true;
    if (other.runtimeType != runtimeType) return false;

    return other is PublicarDocumento &&
        fuente == other.fuente &&
        ancho == other.ancho &&
        conMuestras == other.conMuestras;
  }

  @override
  int get hashCode => Object.hash(fuente, ancho, conMuestras);

  @override
  String toString() {
    String? fullString;

    assert(() {
      fullString =
          '$runtimeType('
          'fuente: $fuente, '
          'ancho: $ancho, '
          'conMuestras: $conMuestras'
          ')';
      return true;
    }());

    return fullString ?? 'PublicarDocumento';
  }
}

extension PublicarDocumentoDartSignalExt on PublicarDocumento {
  /// Sends the signal to Rust.
  /// Passing data from Rust to Dart involves a memory copy
  /// because Rust cannot own data managed by Dart's garbage collector.
  void sendSignalToRust() {
    final messageBytes = bincodeSerialize();
    final binary = Uint8List(0);
    sendDartSignal(
      'rinf_send_dart_signal_publicar_documento',
      messageBytes,
      binary,
    );
  }
}
