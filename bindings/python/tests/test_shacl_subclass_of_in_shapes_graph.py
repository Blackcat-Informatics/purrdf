# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""The ``subclass_of_in_shapes_graph=`` keyword: SHACL 1.2 Core section 6.3.

"SHACL processors SHOULD offer a parameter subClassOfInShapesGraph that, if set
to true, should alter the definition of SHACL Type so that the rdfs:subClassOf
triples are queried from the shapes graph in addition to the data graph." The
class target below is reached only through the shapes graph's
``rdfs:subClassOf``; the control row, a direct instance of the target class,
fires both ways, so a keyword that was silently ignored would fail.
"""

from __future__ import annotations

import purrdf

_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix ex: <http://example.org/> .

ex:Student rdfs:subClassOf ex:Person .
ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:name ; sh:minCount 1 ] .
"""

_TYPE = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type>"
_DATA = (
    f"<http://example.org/alice> {_TYPE} <http://example.org/Student> .\n"
    f"<http://example.org/bob> {_TYPE} <http://example.org/Person> .\n"
)


def _focus(report: object) -> set[str]:
    results = report["results"] if isinstance(report, dict) else report.results
    return {result["focus"] for result in results}


def test_py_validate_subclass_of_in_shapes_graph() -> None:
    off = purrdf.shapes.validate(_SHAPES, _DATA)
    assert _focus(off) == {"<http://example.org/bob>"}
    on = purrdf.shapes.validate(_SHAPES, _DATA, subclass_of_in_shapes_graph=True)
    assert _focus(on) == {"<http://example.org/alice>", "<http://example.org/bob>"}


def test_py_shapes_carries_subclass_of_in_shapes_graph() -> None:
    off = purrdf.shapes.Shapes(_SHAPES)
    assert _focus(off.validate_nt(_DATA)) == {"<http://example.org/bob>"}
    on = purrdf.shapes.Shapes(_SHAPES, subclass_of_in_shapes_graph=True)
    assert _focus(on.validate_nt(_DATA)) == {
        "<http://example.org/alice>",
        "<http://example.org/bob>",
    }
