"""TikZ figures to SVG and PNG, without a TeX install.

>>> import tikra
>>> tikra.tikz_to_svg(r"\\tikz \\draw (0,0) circle (1);")[:4]
'<svg'
"""

from __future__ import annotations

from typing import List, TypedDict

from . import _tikra

__version__: str = _tikra.__version__
#: The version of tikra's core inside this package.
core_version: str = _tikra.core_version


class Result(TypedDict):
    svg: str
    """Standalone SVG, with math labels typeset by mathra."""
    alt: str
    """The figure in a sentence, for readers who cannot see it: the SVG's ``<title>``, an image's alt text."""
    description: str
    """The figure in words (axes, data, nodes and their connections): the SVG's ``<desc>``."""
    warnings: List[str]
    """What was ignored or approximated: unsupported commands, labels with broken LaTeX."""


def tikz_to_svg(code: str, *, locale: str = "en", animate: bool = False, steps: bool = False) -> str:
    """Converts a TikZ figure to SVG, with the figure in words (its ``<title>`` and ``<desc>``) in
    ``locale``: one of mathra's languages, ``en``, ``pt-BR``, ``es``, ``fr``, ``de``, ``it``, ``ru``,
    ``zh``, ``ja``, ``ko``, ``hi``, ``ar``, ``id``, ``tr``, ``nl``, ``pl``. The drawing is the same in all.

    ``code`` is a full ``standalone`` document, a bare ``tikzpicture``, an inline ``\\tikz``, an
    ``axis`` environment or loose TikZ statements. What tikra does not understand is skipped (see
    :func:`render` for the warnings). Raises ValueError when there is no figure to draw or the
    locale is unknown.

    ``animate=True`` adds the construction animation: in a browser, the figure draws itself path by
    path, in the order TikZ drew it. ``steps=True`` numbers each element by its step instead
    (``data-step``, and ``data-steps`` on the ``<svg>``), with no CSS, for a page that walks through
    the figure itself.
    """
    return _tikra.tikz_to_svg(code, locale=locale, animate=animate, steps=steps)


def render(code: str, *, locale: str = "en", animate: bool = False, steps: bool = False) -> Result:
    """The SVG, the figure in words (in ``locale``) and the warnings at once."""
    svg, alt, description, warnings = _tikra.render(code, locale=locale, animate=animate, steps=steps)
    return {"svg": svg, "alt": alt, "description": description, "warnings": warnings}


def png(svg: str, *, dpi: float = 300, background: bool = True) -> bytes:
    """Rasterizes an SVG written by tikra to PNG bytes.

    ``dpi`` is the resolution (300 for print); ``background`` False keeps it transparent instead of
    white. Raises ValueError on an invalid SVG or dpi, or past 100 megapixels.
    """
    return bytes(_tikra.png(svg, dpi=dpi, background=background))


__all__ = ["Result", "__version__", "core_version", "png", "render", "tikz_to_svg"]
