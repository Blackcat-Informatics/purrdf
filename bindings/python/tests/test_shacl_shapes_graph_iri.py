# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The ``shapes_graph`` keyword -- the IRI SHACL-SPARQL sees the shapes graph under, as
``purrdf validate --shapes-graph`` names it -- on every Python shapes-graph entry point,
graded on the W3C SHACL 1.0 test ``sparql/pre-binding/shapesGraph-001``.

That test's constraint selects its focus node only when ``$shapesGraph`` is bound and
``GRAPH $shapesGraph { $currentShape ex:property 42 }`` reads the shapes graph. SHACL 1.0
pre-binds ``$shapesGraph`` and approves ONE result; SHACL 1.2 removed the pre-binding, so
with no IRI named ``$shapesGraph`` is an ordinary, unbound variable and the data graph
CONFORMS. Every entry point is asked both questions, so the keyword is observed rather
than assumed.
"""

from __future__ import annotations

import pathlib

import pytest

import purrdf

_ROOT = pathlib.Path(__file__).resolve().parents[3]
_TEST = _ROOT / "vectors/shacl/sparql/pre-binding/shapesGraph-001.ttl"
_FOCUS = "http://datashapes.org/sh/tests/sparql/pre-binding/shapesGraph-001.test#InvalidResource"


def _fixture() -> tuple[str, str, str]:
    """The test file's shapes graph (Turtle), its IRI, and its data graph (N-Triples)."""
    turtle = _TEST.read_text()
    base = _TEST.as_uri()
    store = purrdf.Store()
    store.load(turtle, purrdf.RdfFormat.TURTLE, base=base)
    data_nt = store.dump(format=purrdf.RdfFormat.N_TRIPLES).decode()
    return turtle, base, data_nt


def test_py_validate_shapes_graph_001() -> None:
    shapes_ttl, base, data_nt = _fixture()
    named = purrdf.shapes.validate(shapes_ttl, data_nt, shapes_base=base, shapes_graph=base)
    assert named["conforms"] is False
    assert len(named["results"]) == 1
    assert _FOCUS in named["results"][0]["focus"]
    # No IRI: SHACL 1.2's ordinary variable, unbound, so the data graph conforms.
    unnamed = purrdf.shapes.validate(shapes_ttl, data_nt, shapes_base=base)
    assert unnamed["conforms"] is True
    assert unnamed["results"] == []
    # A relative IRI resolves against the shapes document's base.
    relative = purrdf.shapes.validate(
        shapes_ttl, data_nt, shapes_base=base, shapes_graph="shapesGraph-001.ttl"
    )
    assert relative["conforms"] is False


def test_py_shapes_prepared_and_product_carry_the_shapes_graph() -> None:
    shapes_ttl, base, data_nt = _fixture()
    for graph, expected in ((base, 1), (None, 0)):
        shapes = purrdf.shapes.Shapes(shapes_ttl, base=base, shapes_graph=graph)
        assert len(shapes.validate_nt(data_nt).results) == expected
        prepared = shapes.prepare()
        assert len(prepared.validate_nt(data_nt).results) == expected
        restored = purrdf.shapes.ShapesProduct.open(prepared.to_product()).admit()
        assert len(restored.validate_nt(data_nt).results) == expected
        packed = purrdf.shapes.pack_product(shapes_ttl, shapes_base=base, shapes_graph=graph)
        admitted = purrdf.shapes.ShapesProduct.open(packed).admit()
        assert len(admitted.validate_nt(data_nt).results) == expected
        store = purrdf.Store()
        store.load(data_nt, purrdf.RdfFormat.N_TRIPLES)
        assert len(shapes.validate_store(store).results) == expected


def test_py_relative_shapes_graph_without_a_base_is_refused() -> None:
    shapes_ttl = (
        "@prefix sh: <http://www.w3.org/ns/shacl#> .\n"
        "@prefix ex: <http://example.org/> .\n"
        'ex:S a sh:NodeShape ; sh:targetNode ex:n ;\n'
        '  sh:sparql [ sh:select "SELECT $this WHERE { FILTER bound($shapesGraph) }" ] .\n'
    )
    for call in (
        lambda graph: purrdf.shapes.validate(shapes_ttl, "", shapes_graph=graph),
        lambda graph: purrdf.shapes.Shapes(shapes_ttl, shapes_graph=graph),
        lambda graph: purrdf.shapes.lint_shapes(shapes_ttl, shapes_graph=graph),
    ):
        with pytest.raises(ValueError, match="iri-relative-no-base"):
            call("shapes")
        # The absolute neighbour needs no base.
        call("http://example.org/shapes")
    with pytest.raises(purrdf.shapes.ShapesProductError, match="iri-relative-no-base"):
        purrdf.shapes.pack_product(shapes_ttl, shapes_graph="shapes")
    assert purrdf.shapes.pack_product(shapes_ttl, shapes_graph="http://example.org/shapes")
    report = purrdf.shapes.validate(shapes_ttl, "", shapes_graph="http://example.org/shapes")
    assert report["conforms"] is False
