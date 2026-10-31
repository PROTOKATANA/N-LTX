# Ejemplo de N-LTX en Flutter

Documento completo y galería de fórmulas, química, física y TikZ, con un editor para escribir LaTeX y
ver el resultado al momento. Linux.

```bash
cargo build --release -p ejemplo_nativo   # desde la raíz del repositorio
cd ejemplo
flutter run -d linux --release
```

`NLTX_NATIVO=/ruta/libejemplo_nativo.so` indica otra biblioteca si no se compiló en `target/release`.

Todo se pinta con Vulkan en una sola superficie: ver `pintor/README.md`. Lo que el pintor no sabe hacer
se avisa en pantalla: imágenes, patrones (los aproxima con su color) y mosaicos.

El runner desactiva Impeller, como en CORTEX: en esta GPU pixela los recortes y los trazados.
