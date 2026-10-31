// ignore_for_file: type=lint, type=warning
part of 'signals.dart';

/// Dart → Rust: qué parte del documento se ve, en píxeles físicos.
@immutable
class VerDocumento {
  const VerDocumento({
    required this.ancho,
    required this.alto,
    required this.escala,
    required this.y,
  });

  static VerDocumento deserialize(BinaryDeserializer deserializer) {
    deserializer.increaseContainerDepth();
    final instance = VerDocumento(
      ancho: deserializer.deserializeUint32(),
      alto: deserializer.deserializeUint32(),
      escala: deserializer.deserializeFloat64(),
      y: deserializer.deserializeFloat64(),
    );
    deserializer.decreaseContainerDepth();
    return instance;
  }

  static VerDocumento bincodeDeserialize(Uint8List input) {
    final deserializer = BincodeDeserializer(input);
    final value = VerDocumento.deserialize(deserializer);
    if (deserializer.offset < input.length) {
      throw Exception('Some input bytes were not read');
    }
    return value;
  }

  final int ancho;
  final int alto;
  final double escala;
  final double y;

  VerDocumento copyWith({
    int? ancho,
    int? alto,
    double? escala,
    double? y,
  }) {
    return VerDocumento(
      ancho: ancho ?? this.ancho,
      alto: alto ?? this.alto,
      escala: escala ?? this.escala,
      y: y ?? this.y,
    );
  }

  void serialize(BinarySerializer serializer) {
    serializer.increaseContainerDepth();
    serializer.serializeUint32(ancho);
    serializer.serializeUint32(alto);
    serializer.serializeFloat64(escala);
    serializer.serializeFloat64(y);
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

    return other is VerDocumento
      && ancho == other.ancho
      && alto == other.alto
      && escala == other.escala
      && y == other.y;
  }

  @override
  int get hashCode => Object.hash(
        ancho,
        alto,
        escala,
        y,
      );

  @override
  String toString() {
    String? fullString;

    assert(() {
      fullString = '$runtimeType('
        'ancho: $ancho, '
        'alto: $alto, '
        'escala: $escala, '
        'y: $y'
        ')';
      return true;
    }());

    return fullString ?? 'VerDocumento';
  }
}

extension VerDocumentoDartSignalExt on VerDocumento {
  /// Sends the signal to Rust.
  /// Passing data from Rust to Dart involves a memory copy
  /// because Rust cannot own data managed by Dart's garbage collector.
  void sendSignalToRust() {
    final messageBytes = bincodeSerialize();
    final binary = Uint8List(0);
    sendDartSignal(
      'rinf_send_dart_signal_ver_documento',
      messageBytes,
      binary,
    );
  }
}
