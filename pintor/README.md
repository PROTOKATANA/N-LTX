# Pintor GPU

C++ sobre DiligentCore y Vulkan, unido a Rust por `cxx`. Es una familia más de texturas sobre la base
`N-CNX/gpu` de CORTEX, que ya resuelve el dispositivo, el hilo de render y compartir la imagen con
Flutter.

```
src/lista.rs       de `Escena` a una lista de dibujo: triángulos, tintas y comandos
src/atlas.rs       las máscaras de los glifos y las rampas de los degradados
src/lienzo.rs      la escena publicada y lo que se pinta en cada fotograma
src/puente.rs      la frontera cxx: tipos compartidos y funciones de Rust que llama C++
src/elementos.rs   la prueba masiva
linux/             los renderizadores (gpu_lienzo, gpu_masiva) y sus familias de texturas
```

## Cómo pinta

Cada vista tiene una superficie, con texto, fórmulas, figuras y animaciones juntos. Flutter no sabe
de su contenido: solo muestra la textura y dice qué parte se ve.

- **Rellenos y trazos:** triángulos de lyon, suavizados con MSAA 4×. Una línea de menos de un píxel se
  pinta de un píxel y más tenue: con solo cuatro muestras, tal cual saldría a trozos.
- **Glifos:** máscaras de área exacta en un atlas, con cuatro posiciones por píxel. Los que pasan de 96
  píxeles, o los que ya no caben en el atlas, se pintan como trazados.
- **Recortes:** geometría en el stencil, sin texturas aparte.
- **Relleno y trazo translúcidos:** cada muestra se pinta una sola vez, con el stencil.
- **Grupos, mezclas y máscaras:** una textura por capa, del tamaño de la vista. Son lo único caro, y las
  que no se ven no se pintan.
- **Animaciones:** se recomponen en cada fotograma solo las que se ven; la base no se vuelve a subir.

La geometría va en píxeles físicos: cambiar la escala recompila la escena y vacía el atlas.

**Sin soporte todavía**, y por eso se avisa al publicar: las imágenes, los patrones con nombre (salen de
su color, lisos) y los mosaicos. `Capa::BordesNitidos` se ignora: esos bordes salen suavizados.

**Prueba masiva:** una galaxia cuyos elementos mueve el vertex shader, con la geometría subida una vez.

## Cómo se compila

La base está en CORTEX: por defecto en `../CORTEX/N-CNX`, u otra ruta con `NLTX_CNX`. Sacarla a su
propio repositorio está pendiente de decidir.

`cargo build --release -p ejemplo_nativo` deja lo que genera `cxx` en `target/release/cxx-pintor`, y
`flutter build linux --release` lo compila en el runner. El ejecutable enlaza con la biblioteca Rust:
C++ llama a Rust directamente, sin `dlopen`.

Se puede medir con `NODOS_BANCO_GPU=1`, cambiar la carga de la prueba masiva con `NLTX_ELEMENTOS=<n>`,
volcar su textura con `NLTX_VOLCAR_GPU=<archivo.ppm>` y pedir otro MSAA con `NLTX_MSAA=2|4|8`. El canal
`ltx/lienzo` tiene el método `volcar`, que escribe el próximo fotograma del documento en un PPM.
