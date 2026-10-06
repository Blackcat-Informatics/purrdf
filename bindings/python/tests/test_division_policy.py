# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""`division=` and `GovernorEvidence.expression_errors` on the Python surface.

`division` is the precision of every `xsd:integer`/`xsd:decimal` quotient, in the
one text form every PurRDF surface reads (`exact`, `N`, `N:ROUNDING`). Under
`exact` a quotient with no finite decimal expansion is a SPARQL expression error:
unbound, like `1/0`, never a raised error. Each refusal and each unbound value here
sits beside a valid neighbour that must still answer.
"""

# Why not Rust: these tests exercise the PyO3 `division` keyword and the
# `expression_errors` dict, which exist only on the Python binding surface.

from __future__ import annotations

import pytest

import purrdf

EX = "http://example.org/"

ALL_CODES = [
    "err:FOAR0001",
    "err:FOAR0002",
    "err:FOCA0001",
    "err:FOCA0002",
    "err:FOCA0003",
    "err:FOCA0006",
    "err:FORG0001",
    "err:XPTY0004",
]


def _select(quotient: str) -> str:
    return f"SELECT (({quotient}) AS ?x) {{}}"


def _value(solutions: object) -> str:
    rows = list(solutions)  # type: ignore[call-overload]
    assert len(rows) == 1
    return str(rows[0][0].value)


def _unbound(solutions: object) -> bool:
    rows = list(solutions)  # type: ignore[call-overload]
    assert len(rows) == 1
    return rows[0][0] is None


def _store() -> purrdf.Store:
    store = purrdf.Store()
    store.load(f"<{EX}s> <{EX}p> <{EX}o> .\n", purrdf.RdfFormat.N_TRIPLES)
    return store


def test_exact_answers_a_terminating_quotient_and_leaves_a_non_terminating_one_unbound() -> None:
    store = _store()

    assert _value(store.query(_select("1/8"), division="exact")) == "0.125"
    assert _unbound(store.query(_select("1/3"), division="exact"))
    caught = store.query('SELECT (COALESCE(1/3, "caught") AS ?x) {}', division="exact")
    assert _value(caught) == "caught"


def test_a_scale_and_rounding_policy_rounds_the_quotient() -> None:
    store = _store()

    assert _value(store.query(_select("2/3"), division="5:half-even")) == "0.66667"
    # The bare digit count truncates toward zero.
    assert _value(store.query(_select("2/3"), division="5")) == "0.66666"


def test_the_default_is_eighteen_digits_truncated_toward_zero() -> None:
    store = _store()

    assert _value(store.query(_select("1/3"))) == "0.333333333333333333"
    assert _value(store.query(_select("1/3"), division=None)) == "0.333333333333333333"


@pytest.mark.parametrize("text", ["5:sideways", "", "exactly", "-1", "5:"])
def test_an_unreadable_policy_is_refused_and_a_valid_one_still_answers(text: str) -> None:
    store = _store()

    with pytest.raises(ValueError, match="division policy"):
        store.query(_select("1/8"), division=text)
    assert _value(store.query(_select("1/8"), division="exact")) == "0.125"


def test_an_unreadable_policy_is_refused_on_every_entry_point() -> None:
    store = _store()
    with pytest.raises(ValueError, match="division policy"):
        store.query_governed(_select("1/8"), division="5:sideways")
    with pytest.raises(ValueError, match="division policy"):
        store.query_entailment_governed(_select("1/8"), "rdfs", division="5:sideways")
    with pytest.raises(ValueError, match="division policy"):
        store.update(f"INSERT DATA {{ <{EX}a> <{EX}b> 1 }}", division="5:sideways")
    with pytest.raises(ValueError, match="division policy"):
        store.update_governed(f"INSERT DATA {{ <{EX}a> <{EX}b> 1 }}", division="5:sideways")
    with pytest.raises(ValueError, match="division policy"):
        store.prepare(_select("1/8"), division="5:sideways")
    # The refused update applied nothing; a valid neighbour applies.
    assert len(store) == 1
    store.update(f"INSERT DATA {{ <{EX}a> <{EX}b> 1 }}", division="exact")
    assert len(store) == 2


def test_division_reaches_query_governed() -> None:
    store = _store()

    outcome = store.query_governed(_select("1/8"), division="exact")
    assert outcome.is_complete
    assert _value(outcome.result) == "0.125"
    third = store.query_governed(_select("1/3"), division="exact")
    assert third.is_complete
    assert _unbound(third.result)
    assert third.evidence.expression_errors["err:FOAR0002"] == 1
    rounded = store.query_governed(_select("2/3"), division="5:half-even")
    assert _value(rounded.result) == "0.66667"


def test_division_reaches_query_entailment_governed() -> None:
    store = _store()

    outcome = store.query_entailment_governed(_select("1/8"), "rdfs", division="exact")
    assert outcome.outcome is not None and outcome.outcome.is_complete
    assert _value(outcome.outcome.result) == "0.125"
    third = store.query_entailment_governed(_select("1/3"), "rdfs", division="exact")
    assert third.outcome is not None and third.outcome.is_complete
    assert _unbound(third.outcome.result)


def _insert(quotient: str) -> str:
    return f"INSERT {{ <{EX}r> <{EX}v> ?x }} WHERE {{ BIND(({quotient}) AS ?x) }}"


def _stored(store: purrdf.Store) -> str:
    return _value(store.query(f"SELECT ?x WHERE {{ <{EX}r> <{EX}v> ?x }}"))


def test_division_reaches_update() -> None:
    unbound = _store()
    unbound.update(_insert("1/3"), division="exact")
    assert len(unbound) == 1, "an unbound ?x inserts nothing"

    applied = _store()
    applied.update(_insert("1/8"), division="exact")
    assert _stored(applied) == "0.125"

    rounded = _store()
    rounded.update(_insert("2/3"), division="5:half-even")
    assert _stored(rounded) == "0.66667"


def test_division_reaches_update_governed() -> None:
    unbound = _store()
    outcome = unbound.update_governed(_insert("1/3"), division="exact")
    assert outcome.is_applied
    assert outcome.evidence.expression_errors["err:FOAR0002"] == 1
    assert len(unbound) == 1, "an unbound ?x inserts nothing"

    applied = _store()
    outcome = applied.update_governed(_insert("1/8"), division="exact")
    assert outcome.is_applied
    assert _stored(applied) == "0.125"


def test_division_reaches_mutable_dataset() -> None:
    dataset = purrdf.MutableDataset()

    assert _value(dataset.query(_select("1/8"), division="exact")) == "0.125"
    assert _unbound(dataset.query(_select("1/3"), division="exact"))


def test_division_reaches_a_prepared_query_on_every_run() -> None:
    store = _store()

    exact = store.prepare(_select("1/8"), division="exact")
    assert _value(exact.run()) == "0.125"
    assert _value(exact.run()) == "0.125"
    unbound = store.prepare(_select("1/3"), division="exact")
    assert _unbound(unbound.run())
    assert _unbound(unbound.run())
    rounded = store.prepare(_select("2/3"), division="5:half-even")
    assert _value(rounded.run()) == "0.66667"
    default = store.prepare(_select("1/3"))
    assert _value(default.run()) == "0.333333333333333333"


def test_evidence_counts_an_absorbed_division_by_zero() -> None:
    store = _store()

    outcome = store.query_governed(_select("1/0"))
    assert outcome.is_complete
    rows = list(outcome.result)
    assert len(rows) == 1 and rows[0][0] is None
    errors = outcome.evidence.expression_errors
    assert list(errors) == ALL_CODES
    assert errors["err:FOAR0001"] == 1
    assert all(count == 0 for code, count in errors.items() if code != "err:FOAR0001")


def test_evidence_of_an_error_free_query_is_all_zero() -> None:
    store = _store()

    outcome = store.query_governed(_select("1/8"))
    errors = outcome.evidence.expression_errors
    assert list(errors) == ALL_CODES
    assert all(count == 0 for count in errors.values())


def test_update_evidence_counts_expression_errors() -> None:
    store = _store()

    outcome = store.update_governed(_insert("1/0"))
    assert outcome.is_applied
    assert outcome.evidence.expression_errors["err:FOAR0001"] == 1
    neighbour = _store().update_governed(_insert("1/8"))
    assert all(count == 0 for count in neighbour.evidence.expression_errors.values())
