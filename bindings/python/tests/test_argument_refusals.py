# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: asserts the ValueError type, message and attributes the installed CPython extension raises at its keyword-argument surface
"""Argument refusals of the Python surface, each beside the valid neighbour it must keep.

The logic is Rust-owned: a premise IRI is judged by ``purrdf_validate``'s premise-IRI
check over the workspace IRI parser, and a prepared parameter by
``PreparedExecution::check_parameters_mentioned``. These tests pin what a Python caller
sees: a ``ValueError`` naming the refused argument, with ``message_id`` and
``presentation`` carrying the typed condition, and the valid neighbour still running.
"""

from __future__ import annotations

import pytest

import purrdf
from purrdf import entail

EX = "http://example.org/"
ONTOLOGY = f"{EX}onto"
DOC = f"<{EX}s> <{EX}p> <{EX}o> .\n"
PATTERN = f"?s <{EX}p> ?o .\n"


def _store() -> purrdf.Store:
    store = purrdf.Store()
    store.load(DOC, purrdf.RdfFormat.N_TRIPLES)
    return store


# Every Python entry point that takes `premise_iris`, as a call over one premise IRI list.
PREMISE_CALLS = [
    (
        "materialize",
        lambda iris: entail.materialize(
            purrdf.RdfDataset(DOC, purrdf.RdfFormat.N_QUADS), entail.Regime.RDFS, "", [], iris
        ),
    ),
    (
        "materialize_nt",
        lambda iris: entail.materialize_nt(DOC, entail.Regime.RDFS, "", [], iris),
    ),
    ("consistency", lambda iris: entail.consistency(DOC, [], iris)),
    (
        "certain_answers",
        lambda iris: entail.certain_answers(entail.Regime.RDFS, DOC, PATTERN, [], iris),
    ),
    (
        "graph_entails",
        lambda iris: entail.graph_entails(entail.Regime.RDFS, DOC, DOC, [], iris),
    ),
    (
        "verify_entailment",
        lambda iris: entail.verify_entailment(entail.Regime.RDFS, DOC, DOC, [], iris),
    ),
    (
        "Store.query_entailment_governed",
        lambda iris: _store().query_entailment_governed(
            "ASK { ?s ?p ?o }", "rdfs", imports=[], premise_iris=iris
        ),
    ),
]


@pytest.mark.parametrize(
    "call", [case[1] for case in PREMISE_CALLS], ids=[case[0] for case in PREMISE_CALLS]
)
def test_a_premise_iri_that_is_not_an_iri_is_refused_with_its_typed_presentation(
    call,
) -> None:
    with pytest.raises(ValueError, match="premise IRI") as raised:
        call(["::bad"])
    error = raised.value
    assert '"::bad"' in str(error)
    assert error.message_id == "premise-iri-not-absolute"
    presentation = error.presentation
    assert presentation["message_id"] == error.message_id
    assert presentation["parameters"]["iri"] == {"kind": "text", "value": "::bad"}
    # The workspace IRI parser's own condition, nested exactly as a SPARQL parse
    # failure nests its `iri-*` cause.
    assert presentation["detail"]["message_id"].startswith("iri-")


@pytest.mark.parametrize(
    "call", [case[1] for case in PREMISE_CALLS], ids=[case[0] for case in PREMISE_CALLS]
)
def test_a_relative_premise_iri_is_refused_as_not_absolute(call) -> None:
    with pytest.raises(ValueError, match="premise IRI") as raised:
        call(["lib"])
    detail = raised.value.presentation["detail"]
    assert detail["message_id"] == "iri-not-absolute-by-grammar.absent"
    assert detail["parameters"]["reference"] == {"kind": "text", "value": "lib"}


@pytest.mark.parametrize(
    "call", [case[1] for case in PREMISE_CALLS], ids=[case[0] for case in PREMISE_CALLS]
)
def test_an_absolute_premise_iri_is_still_accepted(call) -> None:
    # The neighbour: the same call with a valid premise IRI answers.
    call([ONTOLOGY])
    call([])


def test_an_undeclared_prepared_parameter_is_refused_naming_it() -> None:
    store = _store()
    query = f"SELECT ?o WHERE {{ ?this <{EX}p> ?o }}"
    with pytest.raises(ValueError, match='"zz"') as raised:
        store.prepare(query, parameters=["zz"])
    assert "never mentions" in str(raised.value)
    assert raised.value.message_id == "sparql-prepared-parameter-unmentioned"
    parameters = raised.value.presentation["parameters"]
    assert parameters["parameters"] == {"kind": "text", "value": '"zz"'}
    # A used parameter beside an unused one names only the unused one.
    with pytest.raises(ValueError, match='"zz"') as raised:
        store.prepare(query, parameters=["this", "zz"])
    assert '"this"' not in str(raised.value)

    # The neighbour: a declared, used parameter prepares and runs.
    prepared = store.prepare(query, parameters=["this"])
    rows = list(prepared.run(this=purrdf.NamedNode(f"{EX}s")))
    assert [row[0] for row in rows] == [purrdf.NamedNode(f"{EX}o")]


@pytest.mark.parametrize(
    "query",
    [
        f"SELECT ?o WHERE {{ ?o <{EX}p> ?x FILTER(?x != ?this) }}",
        f"SELECT ?o WHERE {{ ?s <{EX}p> ?o FILTER EXISTS {{ ?this <{EX}p> ?o }} }}",
        f"CONSTRUCT {{ ?this <{EX}q> ?o }} WHERE {{ ?s <{EX}p> ?o }}",
        "DESCRIBE ?this",
        f"SELECT ?o WHERE {{ $this <{EX}p> ?o }}",
    ],
    ids=["filter", "exists", "construct-template", "describe-target", "dollar-sigil"],
)
def test_a_parameter_mentioned_anywhere_in_the_query_prepares(query: str) -> None:
    prepared = _store().prepare(query, parameters=["this"])
    assert prepared.parameters == ["this"]
    prepared.run(this=purrdf.NamedNode(f"{EX}s"))
