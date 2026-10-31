# Ejemplo de N-LTX en Flutter

Galería de fórmulas, química, física y TikZ, con un editor para escribir LaTeX y ver el resultado al
momento. Linux.

```bash
cargo build --release -p ejemplo_nativo   # desde la raíz del repositorio
cd ejemplo
flutter run -d linux --release
```

`NLTX_NATIVO=/ruta/libejemplo_nativo.so` indica otra biblioteca si no se compiló en `target/release`.

**No usa Vulkan.** Es el pintor de referencia con el Canvas de Flutter (`lib/escena.dart`), que sirve
para ver la lista neutra y para comparar el pintor GPU cuando exista. Lo que este pintor no sabe
hacer se avisa en pantalla: imágenes, máscaras, patrones (los aproxima con su color) y mosaicos.

`NLTX_VOLCAR=hoja.png ./nltx_ejemplo` escribe todas las escenas en un PNG y termina: sirve para
comprobar el pintor sin depender de la pantalla.

El runner desactiva Impeller, como en CORTEX: en esta GPU pixela los recortes y los trazados.
