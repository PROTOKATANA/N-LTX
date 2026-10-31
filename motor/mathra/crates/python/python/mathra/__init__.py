"""Math LaTeX to accessible SVG, without a TeX install.

>>> import mathra
>>> mathra.render(r"\\frac{3}{4}", locale="fr")["speech"]
'trois quarts'
"""

from __future__ import annotations

import json
from typing import Any, List, Optional, TypedDict

from . import _mathra

__version__: str = _mathra.__version__
#: The version of mathra's core inside this package.
core_version: str = _mathra.core_version


class Rule(TypedDict):
    """A numbered rule from spec/rules.md or spec/tolerance.md, and what it did to this equation."""

    rule: str
    description: str


class Result(TypedDict):
    svg: str
    """Standalone SVG with the speech in <title> and the source and tree in <metadata>."""
    speech: str
    """The phrase a screen reader says, in the locale's language."""
    text: str
    """Linear text, close to UnicodeMath."""
    braille: str
    """The equation in braille, as Unicode braille cells."""
    braille_code: str
    """The braille code used: "cmu" or "nemeth"."""
    tree: Any
    """The tree of meaning."""
    applied: List[Rule]
    """Rules that changed something in this equation, in the order they acted."""
    warnings: List[Rule]
    """Input problems the tolerant mode worked around (E-REC-*); empty in strict mode."""


class Language(TypedDict):
    code: str
    name: str


def tex_to_svg(tex: str, locale: str = "en", *, display: bool = True, strict: bool = False, input: str = "latex", braille: Optional[str] = None, animate: bool = False, steps: bool = False) -> str:
    """Converts an equation to SVG.

    ``locale`` is a language code such as "en", "pt-BR", "fr" or "ja" (see :func:`locales`); ``display``
    False gives inline style; ``strict`` refuses invalid LaTeX instead of recovering with warnings;
    ``input`` is "latex" (default), "mathml" or "asciimath"; ``braille`` is "cmu" or "nemeth" (default:
    the locale's, CMU in pt-BR and es, Nemeth elsewhere). ``animate=True`` makes the SVG write itself in
    the browser, in the order the layout set it (CSS in the SVG); ``steps=True`` numbers each element by
    that order instead (``data-step``, and ``data-steps`` on the ``<svg>``), with no CSS. Raises
    ValueError on input it cannot read.
    """
    return _mathra.tex_to_svg(tex, locale, display=display, strict=strict, input=input, braille=braille, animate=animate, steps=steps)


def tex_to_tree(tex: str, locale: str = "en", *, display: bool = True, strict: bool = False, input: str = "latex", braille: Optional[str] = None, animate: bool = False, steps: bool = False) -> str:
    """The tree of meaning, as a JSON string."""
    return _mathra.tex_to_tree(tex, locale, display=display, strict=strict, input=input, braille=braille, animate=animate, steps=steps)


def render(tex: str, locale: str = "en", *, display: bool = True, strict: bool = False, input: str = "latex", braille: Optional[str] = None, animate: bool = False, steps: bool = False) -> Result:
    """Every output at once: svg, speech, text, braille, braille_code, tree, applied and warnings."""
    return json.loads(_mathra.render(tex, locale, display=display, strict=strict, input=input, braille=braille, animate=animate, steps=steps))


def locales() -> List[Language]:
    """The supported languages, English first."""
    return [{"code": code, "name": name} for code, name in _mathra.locales()]


__all__ = ["Language", "Result", "Rule", "__version__", "core_version", "locales", "render", "tex_to_svg", "tex_to_tree"]
