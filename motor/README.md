# Motor

Fork propio de tres proyectos de Matheus Mota, integrados aquí con su historia (`git subtree`). N-LTX
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
- Se conserva la estructura de cada proyecto (`crates/core`, `corpus`, `docs`, `spec`), porque sus
  pruebas dependen de ella y facilita traer cambios del original:
  `git fetch <repo> <etiqueta> && git cherry-pick -Xsubtree=motor/tikra <commit>`.
- Se quitó lo que N-LTX no usa: las CLI, los enlaces a Python y WebAssembly, npm, las webs y el CI.
- Cada proyecto mantiene su `rustfmt.toml`, para no reformatear código que no se ha cambiado.

## Pendiente en el motor

- `tikra`: emitir `primitivas` directamente, sin el adaptador ni el SVG.
- `tikra`: `MAX_STEPS` limita una figura a 100.000 instrucciones (`engine/mod.rs:404`).
- `tikra`: al importar se quedó sin resolver `repair_markdown`, que lee `\\coordinates` como
  `\coordinate`. N-LTX no pasa por ella, pero `tikra::convert` sí.
- `mathra`: sus fuentes son subconjuntos y faltan glifos, que dibuja como un cuadro (`E-REC-8`).
- `mathra`: `\ifmmode` no se soporta, y `\ ` falla dentro de la etiqueta de una flecha de `\ce`.
- Sin soporte en ninguno de los tres: `tikz-feynman`.
