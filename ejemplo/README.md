# Ejemplo de N-LTX en Flutter

Documento completo y galería de fórmulas, química, física y TikZ, con un editor para escribir LaTeX y
ver el resultado al momento. Linux.

```bash
cd ejemplo
flutter run -d linux --release
```

Necesita `N-CNX`, la base GPU, en `../CORTEX/N-CNX` o donde diga `NLTX_CNX`: ver `pintor/README.md`.

`flutter` compila el hub de Rust (`native/hub`) con `cargokit`, y el ejecutable lo enlaza. Tras cambiar
una señal de `native/hub/src/signals` se regeneran los bindings de Dart con `rinf gen`.

Las fronteras son las de `PUENTES.md`: Flutter habla con Rust por `rinf`, a través de un repositorio
(`lib/repositorio.dart`), y el canal de método solo se usa para registrar la textura. Todo se pinta con
Vulkan en una sola superficie: ver `pintor/README.md`. Lo que el pintor no sabe hacer
se avisa en pantalla: imágenes, patrones (los aproxima con su color) y mosaicos.

El runner desactiva Impeller, como en CORTEX: en esta GPU pixela los recortes y los trazados.
