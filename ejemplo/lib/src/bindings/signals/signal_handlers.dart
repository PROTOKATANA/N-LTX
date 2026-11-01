part of 'signals.dart';

final assignRustSignal = <String, void Function(Uint8List, Uint8List)>{
  'DocumentoPublicado': (Uint8List messageBytes, Uint8List binary) {
    final message = DocumentoPublicado.bincodeDeserialize(messageBytes);
    final rustSignal = RustSignalPack(
      message,
      binary,
    );
    _documentoPublicadoStreamController.add(rustSignal);
    DocumentoPublicado.latestRustSignal = rustSignal;
  },
  'MedidasPintor': (Uint8List messageBytes, Uint8List binary) {
    final message = MedidasPintor.bincodeDeserialize(messageBytes);
    final rustSignal = RustSignalPack(
      message,
      binary,
    );
    _medidasPintorStreamController.add(rustSignal);
    MedidasPintor.latestRustSignal = rustSignal;
  },
  'TextoCopiado': (Uint8List messageBytes, Uint8List binary) {
    final message = TextoCopiado.bincodeDeserialize(messageBytes);
    final rustSignal = RustSignalPack(
      message,
      binary,
    );
    _textoCopiadoStreamController.add(rustSignal);
    TextoCopiado.latestRustSignal = rustSignal;
  },
};
