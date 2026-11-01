# N-LTX

Motor LaTeX sobre la GPU, sin depender de una instalación de TeX. Entra LaTeX y sale una lista neutra de
primitivas (glifos, trazados, recortes, grupos) que un pintor convierte en píxeles.

**Estado:** el motor produce la lista para texto, fórmulas, química, física y TikZ, y `ejemplo/` la
pinta en Flutter sobre Vulkan (`pintor/`), con animaciones por parámetro.

N-LTX es una biblioteca: no sabe nada de Flutter, de Dart ni de `rinf`. Cada proyecto pone el pegamento
que lo une a su interfaz (ver [Integrar N-LTX](#integrar-n-ltx)), y `ejemplo/` es el de referencia. No
depende de otros repositorios: `pintor/nucleo` lleva su propio código de pintado sobre DiligentCore.

```
primitivas/   el contrato: la lista neutra y el trait `Compositor`. Sin dependencias.
documento/    modo texto: LaTeX en flujo continuo, con fórmulas y figuras inyectadas.
animacion/    animaciones por parámetro: la misma figura, recompuesta en cada fotograma.
pintor/       pintor GPU: Rust compila la escena y C++ la pinta sobre Diligent/Vulkan, unidos por `cxx`.
adaptador/    `Compositor` sobre el motor. Desaparecerá cuando el motor emita `primitivas`.
visor/        ver un documento, sin interfaz: compone en su propio hilo y lleva la selección.
motor/        `mathra` (fórmulas, química, física), `tikra` (TikZ) y `voltik` (CircuiTikZ): nacieron
              de un fork y hoy son código propio, en piezas (ver motor/README.md).
ejemplo/      app Flutter y pegamento de referencia: `rinf`, el widget y la base Vulkan (ver su README).
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

## Integrar N-LTX

Lo que pone N-LTX son los crates de Rust y el pintor en C++ (`pintor/nucleo`). Lo que pone cada proyecto es
el pegamento, y `ejemplo/` lo tiene entero para copiarlo:

- **Rust:** depende de `visor` y `pintor`, y traduce sus señales a `Compositor`, `Seleccion` y `Taller`
  (`ejemplo/native/hub`). `Compositor::abrir` recibe de qué carpetas se pueden leer las imágenes.
- **Colores:** se cambian en ejecución. `Taller::fondo` fija el fondo: no recompone ni recompila, va con el
  fotograma. `Encargo::tinta` fija el color de lo que el documento no colorea, TikZ incluido: recompone. Lo
  que el documento pide se respeta, aunque sea `black`, y el texto de un nodo con relleno propio solo deja
  la tinta si no se leería sobre ese relleno. Un texto sobre un relleno que no es de su nodo no se puede
  saber: `\fill[green!20]` y un `\node` encima se quedan con la tinta.
- **Rendimiento:** el perfil y el asignador son de quien compila. Sin `lto = "fat"` y `codegen-units = 1`,
  un fotograma de animación cuesta un 11 % más, y sin `mimalloc` en el hub, entre un 5 % y un 13 % más.
- **C++:** añade DiligentCore, dice en `NLTX_CXX` dónde deja `cxx` lo que genera y hace
  `add_subdirectory` de `pintor/nucleo`. `ltx::Pintor` recibe su dispositivo y su contexto y devuelve una
  textura (`pintor_lienzo.h`). N-LTX no crea el dispositivo: una app que ya use Diligent le da el suyo.
- **Interfaz:** muestra la textura y le dice al `Taller` qué parte se ve (`ejemplo/lib`, `ejemplo/linux/lienzo`,
  y `ejemplo/linux/gpu` si el proyecto no tiene su propia base Vulkan).

Por ahora, el único pegamento probado es el de `ejemplo/`.

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
