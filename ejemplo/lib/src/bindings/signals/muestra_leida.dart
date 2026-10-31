// ignore_for_file: type=lint, type=warning
part of 'signals.dart';

/// Rust → Dart: una muestra de la carpeta, o vacía si no hay ninguna.
@immutable
class MuestraLeida {
  /// An async broadcast stream that listens for signals from Rust.
  /// It supports multiple subscriptions.
  /// Make sure to cancel the subscription when it's no longer needed,
  /// such as when a widget is disposed.
  static final rustSignalStream = _muestraLeidaStreamController.stream
      .asBroadcastStream();

  /// The latest signal value received from Rust.
  /// This is updated every time a new signal is received.
  /// It can be null if no signals have been received yet.
  static RustSignalPack<MuestraLeida>? latestRustSignal = null;

  const MuestraLeida({
    required this.indice,
    required this.total,
    required this.nombre,
    required this.fuente,
  });

  static MuestraLeida deserialize(BinaryDeserializer deserializer) {
    deserializer.increaseContainerDepth();
    final instance = MuestraLeida(
      indice: deserializer.deserializeUint32(),
      total: deserializer.deserializeUint32(),
      nombre: deserializer.deserializeString(),
      fuente: deserializer.deserializeString(),
    );
    deserializer.decreaseContainerDepth();
    return instance;
  }

  static MuestraLeida bincodeDeserialize(Uint8List input) {
    final deserializer = BincodeDeserializer(input);
    final value = MuestraLeida.deserialize(deserializer);
    if (deserializer.offset < input.length) {
      throw Exception('Some input bytes were not read');
    }
    return value;
  }

  final int indice;
  final int total;
  final String nombre;
  final String fuente;

  MuestraLeida copyWith({
    int? indice,
    int? total,
    String? nombre,
    String? fuente,
  }) {
    return MuestraLeida(
      indice: indice ?? this.indice,
      total: total ?? this.total,
      nombre: nombre ?? this.nombre,
      fuente: fuente ?? this.fuente,
    );
  }

  void serialize(BinarySerializer serializer) {
    serializer.increaseContainerDepth();
    serializer.serializeUint32(indice);
    serializer.serializeUint32(total);
    serializer.serializeString(nombre);
    serializer.serializeString(fuente);
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

    return other is MuestraLeida &&
        indice == other.indice &&
        total == other.total &&
        nombre == other.nombre &&
        fuente == other.fuente;
  }

  @override
  int get hashCode => Object.hash(indice, total, nombre, fuente);

  @override
  String toString() {
    String? fullString;

    assert(() {
      fullString =
          '$runtimeType('
          'indice: $indice, '
          'total: $total, '
          'nombre: $nombre, '
          'fuente: $fuente'
          ')';
      return true;
    }());

    return fullString ?? 'MuestraLeida';
  }
}

final _muestraLeidaStreamController =
    StreamController<RustSignalPack<MuestraLeida>>();
