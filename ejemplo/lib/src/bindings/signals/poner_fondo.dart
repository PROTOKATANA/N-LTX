// ignore_for_file: type=lint, type=warning
part of 'signals.dart';

/// Dart → Rust: sobre qué color se pinta, `0xRRGGBB`. No recompone.
@immutable
class PonerFondo {
  const PonerFondo({
    required this.color,
  });

  static PonerFondo deserialize(BinaryDeserializer deserializer) {
    deserializer.increaseContainerDepth();
    final instance = PonerFondo(
      color: deserializer.deserializeUint32(),
    );
    deserializer.decreaseContainerDepth();
    return instance;
  }

  static PonerFondo bincodeDeserialize(Uint8List input) {
    final deserializer = BincodeDeserializer(input);
    final value = PonerFondo.deserialize(deserializer);
    if (deserializer.offset < input.length) {
      throw Exception('Some input bytes were not read');
    }
    return value;
  }

  final int color;

  PonerFondo copyWith({
    int? color,
  }) {
    return PonerFondo(
      color: color ?? this.color,
    );
  }

  void serialize(BinarySerializer serializer) {
    serializer.increaseContainerDepth();
    serializer.serializeUint32(color);
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

    return other is PonerFondo
      && color == other.color;
  }

  @override
  int get hashCode => color.hashCode;

  @override
  String toString() {
    String? fullString;

    assert(() {
      fullString = '$runtimeType('
        'color: $color'
        ')';
      return true;
    }());

    return fullString ?? 'PonerFondo';
  }
}

extension PonerFondoDartSignalExt on PonerFondo {
  /// Sends the signal to Rust.
  /// Passing data from Rust to Dart involves a memory copy
  /// because Rust cannot own data managed by Dart's garbage collector.
  void sendSignalToRust() {
    final messageBytes = bincodeSerialize();
    final binary = Uint8List(0);
    sendDartSignal(
      'rinf_send_dart_signal_poner_fondo',
      messageBytes,
      binary,
    );
  }
}
