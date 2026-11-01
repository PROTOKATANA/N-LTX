#!/bin/bash
# Compila y corre las pruebas de las piezas puras de la base GPU (sin Flutter
# ni Diligent), con los mismos avisos que la biblioteca : todo aviso es un
# error.
set -euo pipefail
aqui="$(cd "$(dirname "$0")" && pwd)"
linux="$aqui/.."
salida="$(mktemp -d)"
trap 'rm -rf "$salida"' EXIT
"${CXX:-g++}" -std=c++17 -O2 -Wall -Wextra -Wpedantic -Werror -pthread \
  -I"$linux" \
  "$aqui/pruebas.cc" "$linux/relevo.cc" \
  -o "$salida/pruebas"
"$salida/pruebas"
