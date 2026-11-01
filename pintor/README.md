# Pintor GPU

C++ sobre DiligentCore y Vulkan, unido a Rust por `cxx`. `linux/base` resuelve el dispositivo, el hilo de
render y compartir la imagen con Flutter, y `linux` pinta.

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
linux/                el renderizador en C++, sus recursos y su familia de texturas
linux/base/           la superficie compartida con Flutter, el hilo de render y DiligentCore
```

El límite de inglete significa lo mismo que en PDF y SVG, comparado con 1/sen(ángulo/2). lyon lo compara
con el doble de su propio límite: `teselado` le pasa la mitad.

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
  solo, las líneas finas salen a dientes. Van sin franja, con su malla exacta y MSAA, tres casos: un
  remate redondo o cuadrado, que crecería con ella; un codo agudo con bisel o unión redonda, que se
  quedaría corto; y el trazo translúcido con relleno, porque dejaría un hueco entre los dos.
- **Glifos:** máscaras de área exacta en un atlas, siempre con cuatro posiciones por píxel: colocarlos
  al píxel entero cambia la forma de los trazos finos. Los que pasan de 256 píxeles, o los que ya no
  caben en el atlas, se pintan como trazados, con lo que vuelven a depender del MSAA.
- **Imágenes:** un cuadrado con su textura y todos sus mipmaps, para que una foto reducida no salga a
  dientes. Llegan decodificadas y con el alfa multiplicado, y se reconocen por su `id`: republicar no las
  vuelve a subir, y al dejar de pintarse se sueltan.
- **Selección:** cajas translúcidas sobre todo lo demás. Van con las animaciones: cambiarla no recompila
  la escena, y lo que no se ve no se tesela.
- **Recortes:** geometría en el stencil, sin texturas aparte.
- **Relleno y trazo translúcidos:** cada muestra se pinta una sola vez, con el stencil.
- **Grupos, mezclas y máscaras:** una textura por capa, del tamaño de la vista. Son lo único caro, y las
  que no se ven no se pintan.
- **Animaciones:** se recomponen en cada fotograma solo las que se ven; la base no se vuelve a subir.

La geometría va en píxeles físicos: cambiar la escala recompila la escena y vacía el atlas.

**Redimensionar:** la superficie mide lo que el monitor más grande (`lienzo.h`) y no se recrea: solo
cambia la parte que se pinta. Flutter la muestra sin reescalar, recortada a la vista.

**Sin soporte todavía**, y por eso se avisa al publicar: los patrones con nombre (salen de su color, lisos)
y los mosaicos. `Capa::BordesNitidos` se ignora: esos bordes salen suavizados.

## Cómo se compila

DiligentCore v2.5.6 se descarga la primera vez en `pintor/deps`, fijado a un commit y sin versionar, para
sobrevivir a `flutter clean`. `DILIGENT_CORE_SOURCE_DIR` usa otra copia.

**Diligent solo admite un dispositivo por proceso:** usa punteros globales a Vulkan y rechaza el segundo.
No importa mientras N-LTX sea la única aplicación. Una aplicación que ya use Diligent y quiera llevar N-LTX
dentro tendrá que aislarlo en su propia biblioteca, con sus símbolos ocultos, o cambiarlo por Vulkan directo.

El `build.rs` deja lo que genera `cxx` en `pintor/cxx`, que se ignora a sí misma (u otra carpeta con
`NLTX_CXX`), y el runner lo compila después del hub. `cxx` y `cxx-build` van fijados a una versión
exacta. El ejecutable enlaza la biblioteca de Rust: C++ la llama directamente, sin `dlopen`.

Se puede medir con `NLTX_BANCO_GPU=1`, pedir otro MSAA con `NLTX_MSAA=2|4|8` y volcar cada fotograma a
un PPM con `NLTX_VOLCAR_LIENZO=<archivo>`. `linux/base/pruebas/probar.sh` prueba las piezas puras.
