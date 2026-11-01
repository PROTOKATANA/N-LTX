// ignore_for_file: type=lint, type=warning
part of 'signals.dart';

/// Rust → Dart: el LaTeX en bruto de lo seleccionado, para el portapapeles.
@immutable
class TextoCopiado {
  /// An async broadcast stream that listens for signals from Rust.
  /// It supports multiple subscriptions.
  /// Make sure to cancel the subscription when it's no longer needed,
  /// such as when a widget is disposed.
  static final rustSignalStream =
      _textoCopiadoStreamController.stream.asBroadcastStream();
        
  /// The latest signal value received from Rust.
  /// This is updated every time a new signal is received.
  /// It can be null if no signals have been received yet.
  static RustSignalPack<TextoCopiado>? latestRustSignal = null;

  const TextoCopiado({
    required this.texto,
  });

  static TextoCopiado deserialize(BinaryDeserializer deserializer) {
    deserializer.increaseContainerDepth();
    final instance = TextoCopiado(
      texto: deserializer.deserializeString(),
    );
    deserializer.decreaseContainerDepth();
    return instance;
  }

  static TextoCopiado bincodeDeserialize(Uint8List input) {
    final deserializer = BincodeDeserializer(input);
    final value = TextoCopiado.deserialize(deserializer);
    if (deserializer.offset < input.length) {
      throw Exception('Some input bytes were not read');
    }
    return value;
  }

  final String texto;

  TextoCopiado copyWith({
    String? texto,
  }) {
    return TextoCopiado(
      texto: texto ?? this.texto,
    );
  }

  void serialize(BinarySerializer serializer) {
    serializer.increaseContainerDepth();
    serializer.serializeString(texto);
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

    return other is TextoCopiado
      && texto == other.texto;
  }

  @override
  int get hashCode => texto.hashCode;

  @override
  String toString() {
    String? fullString;

    assert(() {
      fullString = '$runtimeType('
        'texto: $texto'
        ')';
      return true;
    }());

    return fullString ?? 'TextoCopiado';
  }
}

final _textoCopiadoStreamController =
    StreamController<RustSignalPack<TextoCopiado>>();
