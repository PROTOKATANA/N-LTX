# Ejemplo de N-LTX en Flutter

Documento completo y galería de fórmulas, química, física y TikZ, con un editor para escribir LaTeX y
ver el resultado al momento. Linux.

```bash
cd ejemplo
flutter run -d linux --release
```

La primera vez, CMake descarga DiligentCore v2.5.6 en `deps/`, fijado a un commit y sin versionar, para
sobrevivir a `flutter clean`. `DILIGENT_CORE_SOURCE_DIR` usa otra copia.

## El pegamento

N-LTX no sabe nada de Flutter ni de `rinf`: todo lo que los une está aquí, y es lo que un proyecto copia y
adapta para integrarlo (ver el README de la raíz).

```
native/hub/    las señales de rinf, traducidas a `visor` y `pintor`
lib/           el widget del documento y el repositorio que lo separa de las señales
linux/gpu/     la base: el dispositivo Vulkan, el hilo de render y la imagen compartida con Flutter
linux/lienzo/  une `ltx::Pintor` con esa base y registra la textura
```

`flutter` compila el hub de Rust con `cargokit`, y el ejecutable lo enlaza. Tras cambiar una señal de
`native/hub/src/signals` se regeneran los bindings de Dart con `rinf gen`.

Flutter habla con Rust por `rinf`, a través de un repositorio (`lib/repositorio.dart`), y el canal de método
solo se usa para registrar la textura. Todo se pinta con Vulkan en una sola superficie: ver
`pintor/README.md`. Lo que el pintor no sabe hacer se avisa en pantalla: patrones (los aproxima con su
color) y mosaicos.

El botón de la luna alterna fondo #575965 y tinta blanca en ejecución: `lib/main.dart` solo cambia dos
propiedades de `VistaDocumento`.

Se puede medir con `NLTX_BANCO_GPU=1` y volcar cada fotograma a un PPM con `NLTX_VOLCAR_LIENZO=<archivo>`.
`linux/gpu/pruebas/probar.sh` prueba las piezas puras de la base.

## Lo que enseña

`\includegraphics` lee PNG, JPEG y SVG de `assets/` y, si la hay, de la carpeta de `NLTX_MUESTRAS`. Una
fuente puede venir de un modelo de lenguaje: una ruta absoluta, con `..` o que salga de esas carpetas por un
enlace no se lee. Admite `width`, `height`, `scale` y `keepaspectratio`; lo demás se avisa. Un PDF o EPS se
ve como fuente, con su aviso, y el texto de un SVG no se pinta: `resvg` va sin su módulo de texto.

Con el ratón se selecciona arrastrando, doble clic marca una palabra, Ctrl+A todo y Ctrl+C copia. Lo copiado
es la fuente en bruto: un texto y una animación dan el LaTeX de los dos. Fórmulas, figuras, tablas,
animaciones, imágenes, títulos y órdenes como `\textbf{…}` se copian enteros aunque se toque un trozo, y un
entorno que quede a medias se completa, para que lo copiado compile.

Con `NLTX_MUESTRAS=<carpeta>`, la pestaña Muestras pone todos los `.tex` de esa carpeta en una sola lista,
cada uno con su título y sus avisos debajo: sirve para ver cómo se comporta el motor con LaTeX de fuera, por ejemplo con [DaTikZ](https://huggingface.co/datasets/nllg/datikz).
Las muestras no se versionan: ese conjunto no declara licencia y mezcla orígenes distintos.

La pestaña K1 es `assets/k1.tex`: un solo documento con todo lo que el motor sabe pintar (texto, matemáticas,
física, química, diagramas, electrónica y animaciones). Una prueba de `visor` exige que se componga sin avisos,
también en cada fotograma de sus animaciones. Sin avisos no significa bien: hay que mirarlo pintado.

Una tabla es una rejilla más un estilo: `\begin{tabular}[estilo=…]{lcr}` admite lo estándar de LaTeX
(`\multicolumn`, `\multirow`, `\rowcolor`, `\cellcolor`, `\makecell`) y las claves de
`documento/src/tema.rs`; `\estilotabla{nombre}{opciones}` les pone nombre y pueden heredar con `estilo=`.
Lo que no se dice viene del tema de `Formato`. `\tablaperiodica` es una extensión: escribe una tabla más.

Un bloque de código se ve como un editor: `\begin{lstlisting}[language=Rust,title=a.rs,firstnumber=10,tema=Nord]`.
`numbers=none` quita los números, `guias=no` las guías de sangría y `\estilocodigo{tema=…}` cambia todos los del documento. El tema es
cualquiera de los de `two-face`, y da a la vez los colores de la sintaxis y los del panel.

Las animaciones se escriben con `\begin{animacion}[t=0:360, duracion=4]` y un `tikzpicture` que use `\t`.
Cada fotograma solo depende de su instante: `animacion::Proyector` los compone por adelantado en varios
hilos y los guarda, así que una vuelta que cabe en memoria (256 MiB entre todas) deja de gastar CPU. Solo
trabajan las que están en pantalla, y ninguna se lleva más de la mitad de los hilos: si no le alcanza, baja
de 60 fps. El coste se mide con `cargo test --release -p visor --test banco_animacion -- --ignored --nocapture`.

El runner desactiva Impeller: pixela los recortes y los trazados.
