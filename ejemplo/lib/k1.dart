import 'package:flutter/services.dart';

/// K1: todo lo que el motor sabe pintar, en un solo documento. Una prueba de Rust lee el mismo
/// archivo y exige que se componga sin avisos.
Future<String> cargarK1() => rootBundle.loadString('assets/k1.tex');
