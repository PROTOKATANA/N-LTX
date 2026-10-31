# Pintor GPU

C++ sobre DiligentCore y Vulkan, unido a Rust por `cxx`. Es una familia más de texturas sobre la base
`N-CNX/gpu` de CORTEX, que ya resuelve el dispositivo, el hilo de render y compartir la imagen con
Flutter.

```
src/compilador.rs     de `Escena` a una lista de dibujo: recorre y reparte
src/teselado.rs       triángulos: lo único que conoce lyon
src/glifos.rs         máscaras de los glifos: lo único que conoce zeno
src/tintas.rs         lo que le dice al pixel shader cómo colorear
src/lista.rs          vértices, índices, tintas y comandos
src/atlas.rs          las máscaras de los glifos
src/rampas.rs         los colores de los degradados
src/lienzo.rs         qué se compila en cada fotograma
src/taller.rs         el hilo que lo hace y lleva el ritmo
src/almacen.rs        lo hecho, hasta que C++ lo copia
src/puente_lienzo.rs  la frontera cxx del lienzo
src/puente_masiva.rs  la frontera cxx de la prueba masiva
linux/                los renderizadores en C++, sus recursos y sus familias de texturas
```

## Quién hace qué

Rust lleva el ritmo. El **taller**, en su propio hilo, compila lo publicado y compone las animaciones a
la vista; lo que produce son valores ya hechos, que deja en el **almacén**. El candado del almacén solo
cubre copiar esos valores, nunca un cálculo: C++ jamás espera a que se compile nada.

C++ solo copia lo que cambió, por generaciones: desplazar el documento copia la vista y nada más. Un
hilo vigía espera en Rust a que haya algo nuevo y pide el fotograma. Lo que se publica entra por `rinf`
en la aplicación; este crate no depende de `rinf`, y `Taller` y `Animador` son su interfaz.

## Cómo pinta

Cada vista tiene una superficie, con texto, fórmulas, figuras y animaciones juntos. Flutter no sabe
de su contenido: solo muestra la textura y dice qué parte se ve.

- **Rellenos:** triángulos de lyon, suavizados con MSAA 8×, o lo que dé la GPU.
- **Trazos:** la malla sobresale un píxel por cada lado y el shader calcula cuánto cubre el trazo cada
  píxel, con la distancia al centro. Es exacto a cualquier ancho y no depende de las muestras: con MSAA
  solo, las líneas finas salen a dientes. Un remate redondo o cuadrado no admite la franja, y va sin
  ella; lo mismo el trazo translúcido con relleno, porque la franja dejaría un hueco entre los dos.
- **Glifos:** máscaras de área exacta en un atlas, siempre con cuatro posiciones por píxel: colocarlos
  al píxel entero cambia la forma de los trazos finos. Los que pasan de 256 píxeles, o los que ya no
  caben en el atlas, se pintan como trazados, con lo que vuelven a depender del MSAA.
- **Recortes:** geometría en el stencil, sin texturas aparte.
- **Relleno y trazo translúcidos:** cada muestra se pinta una sola vez, con el stencil.
- **Grupos, mezclas y máscaras:** una textura por capa, del tamaño de la vista. Son lo único caro, y las
  que no se ven no se pintan.
- **Animaciones:** se recomponen en cada fotograma solo las que se ven; la base no se vuelve a subir.

La geometría va en píxeles físicos: cambiar la escala recompila la escena y vacía el atlas.

**Redimensionar:** la superficie mide lo que el monitor más grande (`lienzo.h`) y no se recrea: solo
cambia la parte que se pinta. Flutter la muestra sin reescalar, recortada a la vista.

**Sin soporte todavía**, y por eso se avisa al publicar: las imágenes, los patrones con nombre (salen de
su color, lisos) y los mosaicos. `Capa::BordesNitidos` se ignora: esos bordes salen suavizados.

**Prueba masiva:** una galaxia cuyos elementos mueve el vertex shader, con la geometría subida una vez.

## Cómo se compila

La base está en CORTEX: por defecto en `../CORTEX/N-CNX`, u otra ruta con `NLTX_CNX`. Sacarla a su
propio repositorio está pendiente de decidir.

El `build.rs` deja lo que genera `cxx` en `pintor/cxx`, que se ignora a sí misma (u otra carpeta con
`NLTX_CXX`), y el runner lo compila después del hub. `cxx` y `cxx-build` van fijados a una versión
exacta. El ejecutable enlaza la biblioteca de Rust: C++ la llama directamente, sin `dlopen`.

Se puede medir con `NODOS_BANCO_GPU=1`, cambiar la carga de la prueba masiva con `NLTX_ELEMENTOS=<n>`,
pedir otro MSAA con `NLTX_MSAA=2|4|8` y volcar cada fotograma a un PPM con `NLTX_VOLCAR_LIENZO=<archivo>`
o, en la prueba masiva, `NLTX_VOLCAR_GPU=<archivo>`.
