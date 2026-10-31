# Pintor GPU

C++ sobre DiligentCore y Vulkan, unido a Rust por `cxx`. Es una familia más de texturas sobre la base
`N-CNX/gpu` de CORTEX, que ya resuelve el dispositivo, el hilo de render y compartir la imagen con
Flutter.

```
src/puente.rs      la frontera cxx: tipos compartidos y funciones de Rust que llama C++
src/elementos.rs   la prueba masiva
linux/             el renderizador (gpu_ltx) y su familia de texturas (familia_ltx)
```

**Estado:** solo pinta la prueba masiva: una galaxia cuyos elementos mueve el vertex shader, con la
geometría subida una vez. Todavía no pinta `primitivas`: faltan los glifos, los trazados, los recortes
y los degradados.

## Cómo se compila

La base está en CORTEX: por defecto en `../CORTEX/N-CNX`, u otra ruta con `NLTX_CNX`. Sacarla a su
propio repositorio está pendiente de decidir.

`cargo build --release -p ejemplo_nativo` deja lo que genera `cxx` en `target/release/cxx-pintor`, y
`flutter build linux --release` lo compila en el runner. El ejecutable enlaza con la biblioteca Rust:
C++ llama a Rust directamente, sin `dlopen`.

Se puede medir con `NODOS_BANCO_GPU=1`, cambiar la carga con `NLTX_ELEMENTOS=<n>` y volcar la textura
con `NLTX_VOLCAR_GPU=<archivo.ppm>`.
