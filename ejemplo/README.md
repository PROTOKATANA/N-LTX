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

Con `NLTX_MUESTRAS=<carpeta>`, la pestaña Muestras pone todos los `.tex` de esa carpeta en una sola lista,
cada uno con su título y sus avisos debajo: sirve para ver cómo se comporta el motor con LaTeX de fuera, por ejemplo con [DaTikZ](https://huggingface.co/datasets/nllg/datikz).
Las muestras no se versionan: ese conjunto no declara licencia y mezcla orígenes distintos.

La pestaña K1 es `assets/k1.tex`: un solo documento con todo lo que el motor sabe pintar (texto, matemáticas,
física, química, diagramas, electrónica y animaciones). Una prueba del hub exige que se componga sin avisos,
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
de 60 fps. El coste se mide con `cargo test --release -p hub --test banco_animacion -- --ignored --nocapture`.

El runner desactiva Impeller, como en CORTEX: en esta GPU pixela los recortes y los trazados.
