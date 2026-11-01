// ignore_for_file: type=lint, type=warning
part of 'signals.dart';

/// Dart → Rust: se ha arrastrado de (`ax`, `ay`) a (`bx`, `by`), en bp desde la esquina del documento.
@immutable
class SeleccionarTexto {
  const SeleccionarTexto({
    required this.ax,
    required this.ay,
    required this.bx,
    required this.by,
  });

  static SeleccionarTexto deserialize(BinaryDeserializer deserializer) {
    deserializer.increaseContainerDepth();
    final instance = SeleccionarTexto(
      ax: deserializer.deserializeFloat64(),
      ay: deserializer.deserializeFloat64(),
      bx: deserializer.deserializeFloat64(),
      by: deserializer.deserializeFloat64(),
    );
    deserializer.decreaseContainerDepth();
    return instance;
  }

  static SeleccionarTexto bincodeDeserialize(Uint8List input) {
    final deserializer = BincodeDeserializer(input);
    final value = SeleccionarTexto.deserialize(deserializer);
    if (deserializer.offset < input.length) {
      throw Exception('Some input bytes were not read');
    }
    return value;
  }

  final double ax;
  final double ay;
  final double bx;
  final double by;

  SeleccionarTexto copyWith({
    double? ax,
    double? ay,
    double? bx,
    double? by,
  }) {
    return SeleccionarTexto(
      ax: ax ?? this.ax,
      ay: ay ?? this.ay,
      bx: bx ?? this.bx,
      by: by ?? this.by,
    );
  }

  void serialize(BinarySerializer serializer) {
    serializer.increaseContainerDepth();
    serializer.serializeFloat64(ax);
    serializer.serializeFloat64(ay);
    serializer.serializeFloat64(bx);
    serializer.serializeFloat64(by);
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

    return other is SeleccionarTexto
      && ax == other.ax
      && ay == other.ay
      && bx == other.bx
      && by == other.by;
  }

  @override
  int get hashCode => Object.hash(
        ax,
        ay,
        bx,
        by,
      );

  @override
  String toString() {
    String? fullString;

    assert(() {
      fullString = '$runtimeType('
        'ax: $ax, '
        'ay: $ay, '
        'bx: $bx, '
        'by: $by'
        ')';
      return true;
    }());

    return fullString ?? 'SeleccionarTexto';
  }
}

extension SeleccionarTextoDartSignalExt on SeleccionarTexto {
  /// Sends the signal to Rust.
  /// Passing data from Rust to Dart involves a memory copy
  /// because Rust cannot own data managed by Dart's garbage collector.
  void sendSignalToRust() {
    final messageBytes = bincodeSerialize();
    final binary = Uint8List(0);
    sendDartSignal(
      'rinf_send_dart_signal_seleccionar_texto',
      messageBytes,
      binary,
    );
  }
}
