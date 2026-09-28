# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The ``shapes_graph`` keyword on the rules entry points, ``apply_rules`` and ``entail``.

A SHACL-AF SPARQL rule runs in the shapes-graph context: named, its ``$shapesGraph`` is
pre-bound to the IRI and ``GRAPH $shapesGraph`` reads the shapes graph; omitted,
``$shapesGraph`` is an ordinary, unbound variable. The rule infers
``COALESCE($shapesGraph, ex:none)``, so both answers are observed. Beside ``srl``, which
has no shapes graph, the keyword raises ``ValueError``, and the rule set alone runs.
"""

from __future__ import annotations

import pytest

import purrdf

_SHAPES = '''@prefix ex: <http://example.org/ns#> .
@prefix sh: <http://www.w3.org/ns/shacl#> .
ex:S a sh:NodeShape ; sh:targetClass ex:Person ; ex:marker ex:secret ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """
    CONSTRUCT { $this <http://example.org/ns#shapesGraph> ?g }
    WHERE { BIND (COALESCE($shapesGraph, <http://example.org/ns#none>) AS ?g) }""" ] ;
  sh:rule [ a sh:SPARQLRule ; sh:construct """
    CONSTRUCT { $this <http://example.org/ns#marked> ?m }
    WHERE { GRAPH $shapesGraph { $currentShape <http://example.org/ns#marker> ?m } }""" ] .
'''
_DATA = (
    "<http://example.org/ns#alice> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/ns#Person> .\n"
)
_GRAPH = "http://example.org/shapes-graph"
_NAMED = "<http://example.org/ns#alice> <http://example.org/ns#shapesGraph> <http://example.org/shapes-graph> ."
_UNNAMED = "<http://example.org/ns#alice> <http://example.org/ns#shapesGraph> <http://example.org/ns#none> ."
_MARKED = "<http://example.org/ns#alice> <http://example.org/ns#marked> <http://example.org/ns#secret> ."
_SRL = "PREFIX ex: <http://example.org/ns#>\nRULE { ?x ex:tagged true } WHERE { ?x a ex:Person }\n"


def test_py_apply_rules_sees_the_shapes_graph_iri() -> None:
    named = purrdf.shapes.apply_rules(_DATA, _SHAPES, shapes_graph=_GRAPH)["inferred"]
    assert _NAMED in named
    assert _MARKED in named
    unnamed = purrdf.shapes.apply_rules(_DATA, _SHAPES)["inferred"]
    assert _UNNAMED in unnamed
    assert _MARKED not in unnamed
    relative = purrdf.shapes.apply_rules(
        _DATA, _SHAPES, shapes_base="http://example.org/doc", shapes_graph="shapes-graph"
    )["inferred"]
    assert _NAMED in relative


def test_py_entail_sees_the_shapes_graph_iri() -> None:
    named = purrdf.shapes.entail(_SHAPES, _DATA, shapes_graph=_GRAPH)["ntriples"]
    assert _NAMED in named
    assert _MARKED in named
    unnamed = purrdf.shapes.entail(_SHAPES, _DATA)["ntriples"]
    assert _UNNAMED in unnamed
    assert _MARKED not in unnamed


def test_py_apply_rules_refuses_a_shapes_graph_beside_srl() -> None:
    with pytest.raises(ValueError, match="has no shapes graph"):
        purrdf.shapes.apply_rules(_DATA, srl=_SRL, shapes_graph=_GRAPH)
    assert "<http://example.org/ns#tagged>" in purrdf.shapes.apply_rules(_DATA, srl=_SRL)["inferred"]
