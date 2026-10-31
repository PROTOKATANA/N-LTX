// ignore_for_file: type=lint, type=warning
part of 'signals.dart';

/// Dart → Rust: la muestra que hay que leer.
@immutable
class PedirMuestra {
  const PedirMuestra({required this.indice});

  static PedirMuestra deserialize(BinaryDeserializer deserializer) {
    deserializer.increaseContainerDepth();
    final instance = PedirMuestra(indice: deserializer.deserializeUint32());
    deserializer.decreaseContainerDepth();
    return instance;
  }

  static PedirMuestra bincodeDeserialize(Uint8List input) {
    final deserializer = BincodeDeserializer(input);
    final value = PedirMuestra.deserialize(deserializer);
    if (deserializer.offset < input.length) {
      throw Exception('Some input bytes were not read');
    }
    return value;
  }

  final int indice;

  PedirMuestra copyWith({int? indice}) {
    return PedirMuestra(indice: indice ?? this.indice);
  }

  void serialize(BinarySerializer serializer) {
    serializer.increaseContainerDepth();
    serializer.serializeUint32(indice);
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

    return other is PedirMuestra && indice == other.indice;
  }

  @override
  int get hashCode => indice.hashCode;

  @override
  String toString() {
    String? fullString;

    assert(() {
      fullString =
          '$runtimeType('
          'indice: $indice'
          ')';
      return true;
    }());

    return fullString ?? 'PedirMuestra';
  }
}

extension PedirMuestraDartSignalExt on PedirMuestra {
  /// Sends the signal to Rust.
  /// Passing data from Rust to Dart involves a memory copy
  /// because Rust cannot own data managed by Dart's garbage collector.
  void sendSignalToRust() {
    final messageBytes = bincodeSerialize();
    final binary = Uint8List(0);
    sendDartSignal('rinf_send_dart_signal_pedir_muestra', messageBytes, binary);
  }
}
