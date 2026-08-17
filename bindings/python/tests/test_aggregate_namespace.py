# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""`aggregate_namespace`: the Python surface for purrdf's first-party statistical
aggregate set (`MEDIAN`, `PERCENTILE`, `STDDEV`, `STDDEV_POP`, `VARIANCE`,
`VAR_POP`, `MODE`, `FIRST`, `LAST`, `TOPK` —
`purrdf_sparql_eval::agg_fn::AggregateRegistry::register_statistical_aggregates`).

`register_statistical_aggregates` takes only an IRI namespace string — no host
Rust closure to marshal — so it crosses the Python boundary exactly the way
`property_fn_namespaces` does: a keyword on `query` / `query_governed` / `update`
/ `update_governed`, on both `Store` and `MutableDataset`.

These tests drive a REAL query/update and assert the COMPUTED value, not merely
that the keyword is accepted.
"""

from __future__ import annotations

import pytest

import purrdf

EX = "http://example.org/"
NS = "https://example.org/agg#"

NUMBERS_TTL = f"""
<{EX}s1> <{EX}value> 1 .
<{EX}s2> <{EX}value> 2 .
<{EX}s3> <{EX}value> 3 .
"""

MEDIAN_QUERY = f"SELECT (AGG(<{NS}MEDIAN>, ?v) AS ?m) WHERE {{ ?s <{EX}value> ?v }}"


def _numbers_store() -> purrdf.Store:
    store = purrdf.Store()
    store.load(NUMBERS_TTL, purrdf.RdfFormat.TURTLE)
    return store


def _numbers_mutable() -> purrdf.MutableDataset:
    dataset = purrdf.MutableDataset()
    dataset.load(NUMBERS_TTL, purrdf.RdfFormat.TURTLE)
    return dataset


def test_query_computes_median_through_aggregate_namespace() -> None:
    store = _numbers_store()

    solutions = store.query(MEDIAN_QUERY, aggregate_namespace=NS)

    assert len(solutions) == 1
    assert str(next(iter(solutions))[0].value) == "2"


def test_query_governed_computes_median_through_aggregate_namespace() -> None:
    store = _numbers_store()

    outcome = store.query_governed(MEDIAN_QUERY, aggregate_namespace=NS)

    assert outcome.is_complete
    assert len(outcome.result) == 1
    assert str(next(iter(outcome.result))[0].value) == "2"


def test_mutable_dataset_query_computes_median_through_aggregate_namespace() -> None:
    dataset = _numbers_mutable()

    solutions = dataset.query(MEDIAN_QUERY, aggregate_namespace=NS)

    assert len(solutions) == 1
    assert str(next(iter(solutions))[0].value) == "2"


def test_omitting_aggregate_namespace_leaves_the_statistical_set_unregistered() -> None:
    """No fabricated default: the namespace stays caller-supplied, and the
    existing typed error surfaces unchanged."""
    store = _numbers_store()

    with pytest.raises(ValueError, match="query evaluation error"):
        store.query(MEDIAN_QUERY)


def test_update_reaches_median_from_a_nested_select_group_by() -> None:
    """SPARQL UPDATE's grammar admits an aggregate only inside a nested
    `SELECT … GROUP BY` in the WHERE clause — the only shape a `DELETE`/
    `INSERT … WHERE` can host one through."""
    store = _numbers_store()
    update = (
        f"INSERT {{ <{EX}summary> <{EX}median> ?m }} "
        f"WHERE {{ SELECT (AGG(<{NS}MEDIAN>, ?v) AS ?m) "
        f"WHERE {{ ?s <{EX}value> ?v }} }}"
    )

    store.update(update, aggregate_namespace=NS)

    solutions = store.query(
        f"SELECT ?m WHERE {{ <{EX}summary> <{EX}median> ?m }}"
    )
    assert len(solutions) == 1
    assert str(next(iter(solutions))[0].value) == "2"


def test_update_governed_reaches_median_from_a_nested_select_group_by() -> None:
    store = _numbers_store()
    update = (
        f"INSERT {{ <{EX}summary> <{EX}median> ?m }} "
        f"WHERE {{ SELECT (AGG(<{NS}MEDIAN>, ?v) AS ?m) "
        f"WHERE {{ ?s <{EX}value> ?v }} }}"
    )

    outcome = store.update_governed(update, aggregate_namespace=NS)
    assert outcome.is_applied

    solutions = store.query(
        f"SELECT ?m WHERE {{ <{EX}summary> <{EX}median> ?m }}"
    )
    assert len(solutions) == 1
    assert str(next(iter(solutions))[0].value) == "2"


def test_mutable_dataset_update_governed_reaches_median() -> None:
    dataset = _numbers_mutable()
    update = (
        f"INSERT {{ <{EX}summary> <{EX}median> ?m }} "
        f"WHERE {{ SELECT (AGG(<{NS}MEDIAN>, ?v) AS ?m) "
        f"WHERE {{ ?s <{EX}value> ?v }} }}"
    )

    outcome = dataset.update_governed(update, aggregate_namespace=NS)
    assert outcome.is_applied

    solutions = dataset.query(
        f"SELECT ?m WHERE {{ <{EX}summary> <{EX}median> ?m }}"
    )
    assert len(solutions) == 1
    assert str(next(iter(solutions))[0].value) == "2"


def test_aggregate_namespace_registration_does_not_leak_across_calls() -> None:
    """Registration is per call, exactly like `property_fn_namespaces` /
    `relations`: a namespace named on one call is not in scope on the next."""
    store = _numbers_store()

    assert len(store.query(MEDIAN_QUERY, aggregate_namespace=NS)) == 1
    with pytest.raises(ValueError):
        store.query(MEDIAN_QUERY)


# ── the entailment-governed lane (it took no `QueryOptions` at all) ───────────────

ANIMALS_TTL = f"""
<{EX}Cat> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <{EX}Animal> .
<{EX}tom> a <{EX}Cat> ; <{EX}weight> 1 .
<{EX}felix> a <{EX}Cat> ; <{EX}weight> 2 .
<{EX}garfield> a <{EX}Cat> ; <{EX}weight> 3 .
"""

ENTAILED_MEDIAN_QUERY = (
    f"SELECT (AGG(<{NS}MEDIAN>, ?w) AS ?m) WHERE {{ "
    f"?s a <{EX}Animal> . ?s <{EX}weight> ?w }}"
)


def _animals_store() -> purrdf.Store:
    store = purrdf.Store()
    store.load(ANIMALS_TTL, purrdf.RdfFormat.TURTLE)
    return store


def test_query_entailment_governed_computes_median_over_an_entailed_closure() -> None:
    """`aggregate_namespace` reaches `query_entailment_governed` exactly as it
    reaches `query_governed`: `MEDIAN` folds over the `?s a Animal` binding the
    RDFS closure itself produced, not a binding present in the raw data."""
    store = _animals_store()

    outcome = store.query_entailment_governed(
        ENTAILED_MEDIAN_QUERY, "rdfs", aggregate_namespace=NS
    )

    assert outcome.phase == "answered"
    assert outcome.is_complete
    assert outcome.outcome is not None and outcome.outcome.is_complete
    rows = list(outcome.outcome.result)
    assert len(rows) == 1
    assert str(rows[0][0].value) == "2"


def test_mutable_dataset_query_entailment_governed_computes_median() -> None:
    dataset = purrdf.MutableDataset()
    dataset.load(ANIMALS_TTL, purrdf.RdfFormat.TURTLE)

    outcome = dataset.query_entailment_governed(
        ENTAILED_MEDIAN_QUERY, "rdfs", aggregate_namespace=NS
    )

    assert outcome.is_complete
    assert outcome.outcome is not None
    rows = list(outcome.outcome.result)
    assert len(rows) == 1
    assert str(rows[0][0].value) == "2"


def test_omitting_aggregate_namespace_leaves_entailment_lane_unregistered() -> None:
    store = _animals_store()

    with pytest.raises(ValueError, match="entailment query failed"):
        store.query_entailment_governed(ENTAILED_MEDIAN_QUERY, "rdfs")
