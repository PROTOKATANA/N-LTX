// ignore_for_file: type=lint, type=warning
part of 'signals.dart';

/// Rust → Dart: lo que hace falta saber para colocar los widgets. `error` vacío si no hubo.
@immutable
class DocumentoPublicado {
  /// An async broadcast stream that listens for signals from Rust.
  /// It supports multiple subscriptions.
  /// Make sure to cancel the subscription when it's no longer needed,
  /// such as when a widget is disposed.
  static final rustSignalStream = _documentoPublicadoStreamController.stream
      .asBroadcastStream();

  /// The latest signal value received from Rust.
  /// This is updated every time a new signal is received.
  /// It can be null if no signals have been received yet.
  static RustSignalPack<DocumentoPublicado>? latestRustSignal = null;

  const DocumentoPublicado({
    required this.alto,
    required this.avisos,
    required this.error,
  });

  static DocumentoPublicado deserialize(BinaryDeserializer deserializer) {
    deserializer.increaseContainerDepth();
    final instance = DocumentoPublicado(
      alto: deserializer.deserializeFloat64(),
      avisos: TraitHelpers.deserializeVectorStr(deserializer),
      error: deserializer.deserializeString(),
    );
    deserializer.decreaseContainerDepth();
    return instance;
  }

  static DocumentoPublicado bincodeDeserialize(Uint8List input) {
    final deserializer = BincodeDeserializer(input);
    final value = DocumentoPublicado.deserialize(deserializer);
    if (deserializer.offset < input.length) {
      throw Exception('Some input bytes were not read');
    }
    return value;
  }

  final double alto;
  final List<String> avisos;
  final String error;

  DocumentoPublicado copyWith({
    double? alto,
    List<String>? avisos,
    String? error,
  }) {
    return DocumentoPublicado(
      alto: alto ?? this.alto,
      avisos: avisos ?? this.avisos,
      error: error ?? this.error,
    );
  }

  void serialize(BinarySerializer serializer) {
    serializer.increaseContainerDepth();
    serializer.serializeFloat64(alto);
    TraitHelpers.serializeVectorStr(avisos, serializer);
    serializer.serializeString(error);
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

    return other is DocumentoPublicado &&
        alto == other.alto &&
        listEquals(avisos, other.avisos) &&
        error == other.error;
  }

  @override
  int get hashCode => Object.hash(alto, avisos, error);

  @override
  String toString() {
    String? fullString;

    assert(() {
      fullString =
          '$runtimeType('
          'alto: $alto, '
          'avisos: $avisos, '
          'error: $error'
          ')';
      return true;
    }());

    return fullString ?? 'DocumentoPublicado';
  }
}

final _documentoPublicadoStreamController =
    StreamController<RustSignalPack<DocumentoPublicado>>();
