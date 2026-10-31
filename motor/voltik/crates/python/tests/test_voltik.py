"""The Python package, used the way someone who installed it would."""

import pytest

import voltik

FIGURE = r"\begin{circuitikz}\draw (0,0) to[R=$R$] (2,0);\end{circuitikz}"


def test_svg():
    svg = voltik.circuitikz_to_svg(FIGURE)
    assert svg.startswith("<svg")


def test_every_input_form():
    for code in [
        r"\documentclass{standalone}\usepackage{circuitikz}\begin{document}\begin{circuitikz}\draw (0,0) to[R] (2,0);\end{circuitikz}\end{document}",
        r"\begin{circuitikz}\draw (0,0) to[C] (2,0);\end{circuitikz}",
        r"\begin{tikzpicture}\draw (0,0) to[L] (2,0);\end{tikzpicture}",
        r"\draw (0,0) to[R, o-*] (2,0);",
    ]:
        assert voltik.render(code)["svg"].startswith("<svg")


def test_render_reports_warnings():
    r = voltik.render(FIGURE)
    assert r["warnings"] == []
    r = voltik.render(r"\begin{circuitikz}\draw (0,0) to[R, no such option] (2,0);\end{circuitikz}")
    assert r["svg"].startswith("<svg")
    assert r["warnings"]


def test_render_describes_the_figure():
    r = voltik.render(FIGURE)
    assert r["alt"] == "Drawing of 2 shapes with the text “R”"
    assert r["description"] == "Text in the figure: R."
    assert "<desc" in r["svg"]
    assert voltik.render(FIGURE, locale="pt-BR")["description"] == "Texto na figura: R."
    assert "Texte de la figure" in voltik.circuitikz_to_svg(FIGURE, locale="fr")
    with pytest.raises(ValueError, match="unknown locale"):
        voltik.render(FIGURE, locale="xx")


def test_no_figure_is_value_error():
    with pytest.raises(ValueError, match="no tikzpicture"):
        voltik.circuitikz_to_svg("just text")
    with pytest.raises(ValueError):
        voltik.render("just text")


def test_png():
    svg = voltik.circuitikz_to_svg(FIGURE)
    opaque = voltik.png(svg)
    assert isinstance(opaque, bytes)
    assert opaque.startswith(b"\x89PNG")
    assert voltik.png(svg, background=False) != opaque
    assert len(voltik.png(svg, dpi=72)) < len(opaque)


def test_png_errors():
    with pytest.raises(ValueError, match="invalid dpi"):
        voltik.png(voltik.circuitikz_to_svg(FIGURE), dpi=0)
    with pytest.raises(ValueError):
        voltik.png("not svg")


def test_versions():
    assert voltik.__version__.count(".") == 2
    assert voltik.core_version.count(".") == 2
