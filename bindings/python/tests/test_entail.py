# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""SHACL-AF rule-entailment surface tests for the Python binding.

Exercises ``purrdf.shapes.entail(shapes_ttl, data_nt)`` — the entailment twin of
``purrdf.shapes.validate`` — proving a ``sh:TripleRule`` materializes its inferred
head triple into the returned N-Triples dataset, deterministically.
"""

from __future__ import annotations

import pytest

import purrdf

# A shape whose sh:TripleRule types every ex:Person as ex:adult ex:yes.
_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .

ex:PersonRule a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:rule [ a sh:TripleRule ;
        sh:subject sh:this ; sh:predicate ex:adult ; sh:object ex:yes ] .
"""

_DATA = (
    "<http://example.org/alice> "
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/Person> .\n"
)

_INFERRED = (
    "<http://example.org/alice> <http://example.org/adult> <http://example.org/yes> ."
)
_BASE = (
    "<http://example.org/alice> "
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> ."
)


def test_entail_is_callable() -> None:
    assert callable(purrdf.shapes.entail)


def test_entail_materializes_inferred_triple() -> None:
    out = purrdf.shapes.entail(_SHAPES, _DATA)
    assert _INFERRED in out, f"inferred triple missing from:\n{out}"
    # The base fact survives into the materialized dataset.
    assert _BASE in out, f"base triple missing from:\n{out}"


def test_entail_is_deterministic() -> None:
    assert purrdf.shapes.entail(_SHAPES, _DATA) == purrdf.shapes.entail(_SHAPES, _DATA)


def test_entail_rejects_malformed_shapes() -> None:
    with pytest.raises(ValueError):
        purrdf.shapes.entail("@@@ not turtle", _DATA)
