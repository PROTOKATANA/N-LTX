# N-LTX

Motor LaTeX para Flutter sobre la GPU, sin depender de una instalación de TeX. Entra LaTeX y sale una
lista neutra de primitivas (glifos, trazados, recortes, grupos) que un pintor convierte en píxeles.

**Estado:** el motor produce la lista para texto, fórmulas, química, física y TikZ, y `ejemplo/` la
pinta en Flutter sobre Vulkan (`pintor/`), con animaciones por parámetro.

La app no depende de otros repositorios: `pintor/linux` lleva su propio código Vulkan sobre DiligentCore,
que CMake descarga una vez en `pintor/deps`, fijado a un commit.

```
primitivas/   el contrato: la lista neutra y el trait `Compositor`. Sin dependencias.
documento/    modo texto: LaTeX en flujo continuo, con fórmulas y figuras inyectadas.
animacion/    animaciones por parámetro: la misma figura, recompuesta en cada fotograma.
pintor/       pintor GPU: Rust compila la escena y C++ la pinta sobre Diligent/Vulkan, unidos por `cxx`.
adaptador/    `Compositor` sobre el motor. Desaparecerá cuando el motor emita `primitivas`.
motor/        `mathra` (fórmulas, química, física), `tikra` (TikZ) y `voltik` (CircuiTikZ): nacieron
              de un fork y hoy son código propio, en piezas (ver motor/README.md).
ejemplo/      app Flutter: galería y editor de LaTeX, unida a Rust por `rinf` (ver su README).
```

## Uso

```rust
use adaptador::{FiguraTikra, FormulaMathra};
use primitivas::{Color, Compositor};

let negro = Color { r: 0.0, g: 0.0, b: 0.0 };
let formula = FormulaMathra { bloque: true, tam: 20.0, estricto: false, color: negro };
let c = formula.componer(r"\dv{f}{x} = \ce{H2O}")?;

let figura = FiguraTikra::default().componer(r"\begin{tikzpicture}\draw (0,0) circle (1);\end{tikzpicture}")?;

for aviso in &c.avisos {
    eprintln!("{aviso}");
}
// c.escena.primitivas, en bp y con la y hacia abajo; c.escena.contornos, los glifos usados
```

Lo que no se entiende nunca se pierde en silencio: o se rechaza (`estricto`), o llega como aviso en
`Composicion::avisos`. Una escena se basta a sí misma: trae los contornos de sus glifos.

## Para un pintor

Una escena es una lista en orden de pintado, con capas que se abren y se cierran (`Primitiva::Abrir`
y `Primitiva::Cerrar`). Dos cosas no están en la lista y le tocan al pintor:

- **Costuras:** dos rellenos contiguos del mismo color dejan ver una línea fina entre ellos si se
  suavizan por separado. El driver de SVG de `tikra` lo tapa con trazos que no son parte de la figura.
- **Patrones con nombre:** `Pintura::Patron` da el nombre de `pgf`; su dibujo vive en un driver
  privado de `tikra`.

## Comprobaciones

```bash
pwsh scripts/check.ps1     # puerta de calidad completa, antes de cada commit

# La lista neutra dibuja lo mismo que tikra, píxel a píxel, sobre un banco de figuras:
BANCO=carpeta cargo test --release -p adaptador --test banco la_lista -- --ignored --nocapture

# Todas las fórmulas de un corpus JSONL con un campo `texto`:
CORPUS=archivo.jsonl cargo test --release -p adaptador --test banco el_corpus -- --ignored --nocapture
```

## Licencias

El código propio es MIT (`LICENSE`). Lo de `motor/` conserva las suyas.
