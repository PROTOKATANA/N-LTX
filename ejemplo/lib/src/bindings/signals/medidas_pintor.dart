// ignore_for_file: type=lint, type=warning
part of 'signals.dart';

/// Rust → Dart, una vez por segundo.
@immutable
class MedidasPintor {
  /// An async broadcast stream that listens for signals from Rust.
  /// It supports multiple subscriptions.
  /// Make sure to cancel the subscription when it's no longer needed,
  /// such as when a widget is disposed.
  static final rustSignalStream =
      _medidasPintorStreamController.stream.asBroadcastStream();
        
  /// The latest signal value received from Rust.
  /// This is updated every time a new signal is received.
  /// It can be null if no signals have been received yet.
  static RustSignalPack<MedidasPintor>? latestRustSignal = null;

  const MedidasPintor({
    required this.vertices,
    required this.msTaller,
    required this.msCarril,
  });

  static MedidasPintor deserialize(BinaryDeserializer deserializer) {
    deserializer.increaseContainerDepth();
    final instance = MedidasPintor(
      vertices: deserializer.deserializeUint64(),
      msTaller: deserializer.deserializeFloat64(),
      msCarril: deserializer.deserializeFloat64(),
    );
    deserializer.decreaseContainerDepth();
    return instance;
  }

  static MedidasPintor bincodeDeserialize(Uint8List input) {
    final deserializer = BincodeDeserializer(input);
    final value = MedidasPintor.deserialize(deserializer);
    if (deserializer.offset < input.length) {
      throw Exception('Some input bytes were not read');
    }
    return value;
  }

  final Uint64 vertices;
  final double msTaller;
  final double msCarril;

  MedidasPintor copyWith({
    Uint64? vertices,
    double? msTaller,
    double? msCarril,
  }) {
    return MedidasPintor(
      vertices: vertices ?? this.vertices,
      msTaller: msTaller ?? this.msTaller,
      msCarril: msCarril ?? this.msCarril,
    );
  }

  void serialize(BinarySerializer serializer) {
    serializer.increaseContainerDepth();
    serializer.serializeUint64(vertices);
    serializer.serializeFloat64(msTaller);
    serializer.serializeFloat64(msCarril);
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

    return other is MedidasPintor
      && vertices == other.vertices
      && msTaller == other.msTaller
      && msCarril == other.msCarril;
  }

  @override
  int get hashCode => Object.hash(
        vertices,
        msTaller,
        msCarril,
      );

  @override
  String toString() {
    String? fullString;

    assert(() {
      fullString = '$runtimeType('
        'vertices: $vertices, '
        'msTaller: $msTaller, '
        'msCarril: $msCarril'
        ')';
      return true;
    }());

    return fullString ?? 'MedidasPintor';
  }
}

final _medidasPintorStreamController =
    StreamController<RustSignalPack<MedidasPintor>>();
