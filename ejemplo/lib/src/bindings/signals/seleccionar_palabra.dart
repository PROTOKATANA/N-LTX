// ignore_for_file: type=lint, type=warning
part of 'signals.dart';

/// Dart → Rust: doble clic en (`x`, `y`).
@immutable
class SeleccionarPalabra {
  const SeleccionarPalabra({
    required this.x,
    required this.y,
  });

  static SeleccionarPalabra deserialize(BinaryDeserializer deserializer) {
    deserializer.increaseContainerDepth();
    final instance = SeleccionarPalabra(
      x: deserializer.deserializeFloat64(),
      y: deserializer.deserializeFloat64(),
    );
    deserializer.decreaseContainerDepth();
    return instance;
  }

  static SeleccionarPalabra bincodeDeserialize(Uint8List input) {
    final deserializer = BincodeDeserializer(input);
    final value = SeleccionarPalabra.deserialize(deserializer);
    if (deserializer.offset < input.length) {
      throw Exception('Some input bytes were not read');
    }
    return value;
  }

  final double x;
  final double y;

  SeleccionarPalabra copyWith({
    double? x,
    double? y,
  }) {
    return SeleccionarPalabra(
      x: x ?? this.x,
      y: y ?? this.y,
    );
  }

  void serialize(BinarySerializer serializer) {
    serializer.increaseContainerDepth();
    serializer.serializeFloat64(x);
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

    return other is SeleccionarPalabra
      && x == other.x
      && y == other.y;
  }

  @override
  int get hashCode => Object.hash(
        x,
        y,
      );

  @override
  String toString() {
    String? fullString;

    assert(() {
      fullString = '$runtimeType('
        'x: $x, '
        'y: $y'
        ')';
      return true;
    }());

    return fullString ?? 'SeleccionarPalabra';
  }
}

extension SeleccionarPalabraDartSignalExt on SeleccionarPalabra {
  /// Sends the signal to Rust.
  /// Passing data from Rust to Dart involves a memory copy
  /// because Rust cannot own data managed by Dart's garbage collector.
  void sendSignalToRust() {
    final messageBytes = bincodeSerialize();
    final binary = Uint8List(0);
    sendDartSignal(
      'rinf_send_dart_signal_seleccionar_palabra',
      messageBytes,
      binary,
    );
  }
}
