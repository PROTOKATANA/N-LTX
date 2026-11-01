# Motor

Nace de un fork de tres proyectos de Matheus Mota, integrados aquí con su historia (`git subtree`). N-LTX
diverge de ellos a propósito: su objetivo es SVG y el nuestro, la GPU y las animaciones.

| Carpeta | Proyecto original | Punto de partida | Qué aporta |
|---|---|---|---|
| `mathra/` | [mathra](https://github.com/matheusmota-dev-br/mathra) | `v1.9.2` | Fórmulas, química y física |
| `tikra/` | [tikra](https://github.com/matheusmota-dev-br/tikra) | `v1.8.1` | TikZ y pgfplots |
| `voltik/` | [voltik](https://github.com/matheusmota-dev-br/voltik) | `v0.16.0` | CircuiTikZ, como biblioteca de `tikra` |

Cada carpeta conserva sus licencias: MIT o Apache-2.0 para el código, GUST Font License para Latin
Modern Math y TeX Gyre Termes, y OFL 1.1 para STIX Two Math.

## Cómo se trabaja

- Este código es nuestro: se edita como el resto del repositorio. Ninguno de los crates se publica.
- Es un proyecto aparte: no se siguen los cambios de los originales ni se busca poder fusionarlos.
- Se conserva lo que las pruebas necesitan de cada proyecto: `corpus`, `docs` y `spec`.
- Se quitó lo que N-LTX no usa: las CLI, los enlaces a Python y WebAssembly, npm, las webs y el CI.
- Cada proyecto mantiene su `rustfmt.toml`, para no reformatear código que no se ha cambiado.

## Las piezas de `tikra`

Cada pieza se puede cambiar o sustituir por separado, y Cargo impide que las dependencias vayan al
revés: el lenguaje no conoce el dibujo, y el motor no conoce sus bibliotecas.

| Crate | Qué es | Depende de |
|---|---|---|
| `tikra_tex` | El lenguaje: TeX, pgfmath, pgfkeys, fechas y archivos | Nada |
| `tikra_canvas` | Lo dibujado: colores, escena y lienzo | `mathra` |
| `tikra_label` | El texto de los nodos | `tikra_canvas`, `tikra_tex` |
| `tikra_arrows`, `tikra_shapes` | Puntas de flecha y formas | `tikra_canvas`, `tikra_tex` |
| `tikra_engine` | El intérprete de TikZ, que se monta con piezas | Los anteriores |
| `tikra_calendar`, `tikra_graphs`, `tikra_datavis`, `tikra_pgfplots`, `tikra_decorations`, `tikra_libraries` | Las bibliotecas, como piezas | `tikra_engine` |
| `tikra` | La fachada, que compone las piezas | Todos |

Una biblioteca nueva es un `Piece` (`engine/piece.rs`) añadido a `PIECES` en `crates/core/src/lib.rs`:
declara sus órdenes y sus definiciones, y no hay que tocar el motor. Lo que guarda durante una figura
va en `Engine::piece_state`.

Se quedan en el motor `trees`, `chains`, `shadings`, `patterns`, `fadings`, `lsystem` y `nonlinear`:
el dibujo normal las usa, o sacarlas exigiría un gancho solo para ellas.

Para comprobar que un cambio no altera la salida:
`cargo test --release -p visor --test huella -- --ignored --nocapture`. Resume K1, sus animaciones y las
figuras de `tikra` y `voltik`. `adaptador/tests/presupuesto.rs` falla si vuelven las copias por elemento.

## Pendiente en el motor

- `tikra`: emitir `primitivas` directamente, sin el adaptador ni el SVG.
- `tikra`: `MAX_STEPS` limita una figura a 300.000 instrucciones.
- `tikra`: al importar se quedó sin resolver `repair_markdown`, que lee `\\coordinates` como
  `\coordinate`. N-LTX no pasa por ella, pero `tikra::convert` sí.
- `mathra`: sus fuentes son subconjuntos y faltan glifos, que dibuja como un cuadro (`E-REC-8`).
- `mathra`: `\ifmmode` no se soporta, y `\ ` falla dentro de la etiqueta de una flecha de `\ce`.
- `mathra`: el lector de AsciiMath entra en pánico con algunas entradas (`input/asciimath.rs`).
- Sin soporte en ninguno de los tres: `tikz-feynman`.
