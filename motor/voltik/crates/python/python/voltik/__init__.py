"""CircuiTikZ circuits to SVG and PNG, without a TeX install.

>>> import voltik
>>> voltik.circuitikz_to_svg(r"\\begin{circuitikz}\\draw (0,0) to[R] (2,0);\\end{circuitikz}")[:4]
'<svg'
"""

from __future__ import annotations

from typing import List, TypedDict

from . import _voltik

__version__: str = _voltik.__version__
#: The version of voltik's core inside this package.
core_version: str = _voltik.core_version


class Result(TypedDict):
    svg: str
    """Standalone SVG, with math labels typeset by mathra (through tikra)."""
    alt: str
    """The figure in a sentence, for readers who cannot see it: the SVG's ``<title>``, an image's alt text."""
    description: str
    """The figure in words (axes, data, nodes and their connections): the SVG's ``<desc>``."""
    warnings: List[str]
    """What was ignored or approximated: unsupported commands, labels with broken LaTeX."""


def circuitikz_to_svg(code: str, *, locale: str = "en", animate: bool = False, steps: bool = False) -> str:
    """Converts a CircuiTikZ circuit to SVG, with the figure in words (its ``<title>`` and ``<desc>``) in
    ``locale``: one of mathra's languages, ``en``, ``pt-BR``, ``es``, ``fr``, ``de``, ``it``, ``ru``,
    ``zh``, ``ja``, ``ko``, ``hi``, ``ar``, ``id``, ``tr``, ``nl``, ``pl``. The drawing is the same in all.

    ``code`` is a full ``standalone`` document, a ``circuitikz`` environment, a ``tikzpicture`` that
    uses CircuiTikZ components, or loose statements. What voltik does not understand is skipped (see
    :func:`render` for the warnings). Raises ValueError when there is no figure to draw or the
    locale is unknown.

    ``animate=True`` adds the construction animation: in a browser, the figure draws itself path by
    path, in the order TikZ drew it. ``steps=True`` numbers each element by its step instead
    (``data-step``, and ``data-steps`` on the ``<svg>``), with no CSS, for a page that walks through
    the figure itself.
    """
    return _voltik.circuitikz_to_svg(code, locale=locale, animate=animate, steps=steps)


def render(code: str, *, locale: str = "en", animate: bool = False, steps: bool = False) -> Result:
    """The SVG, the figure in words (in ``locale``) and the warnings at once."""
    svg, alt, description, warnings = _voltik.render(code, locale=locale, animate=animate, steps=steps)
    return {"svg": svg, "alt": alt, "description": description, "warnings": warnings}


def png(svg: str, *, dpi: float = 300, background: bool = True) -> bytes:
    """Rasterizes an SVG written by voltik to PNG bytes.

    ``dpi`` is the resolution (300 for print); ``background`` False keeps it transparent instead of
    white. Raises ValueError on an invalid SVG or dpi, or past 100 megapixels.
    """
    return bytes(_voltik.png(svg, dpi=dpi, background=background))


__all__ = ["Result", "__version__", "core_version", "png", "render", "circuitikz_to_svg"]
