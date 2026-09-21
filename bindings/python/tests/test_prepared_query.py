# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Prepared, parameterized queries on the Python surface.

``Store.query`` parses and admits its text on every call, so a caller running one
query per row pays that cost per row to be handed back the same plan — and a caller
who instead splices the row's value into the text pays a real re-parse, because a
spliced query is a different query. ``Store.prepare`` is the alternative the engine
already pointed callers at.

These tests hold that surface to three promises:

* **Each binding answers for its own value.** A handle that reused the first
  binding, or that failed to narrow at all, is distinguishable here because every
  subject has a different object.
* **A parameter left unbound is refused, not defaulted.** Treating it as
  unrestricted would answer over every subject — a silently WIDER answer.
* **A name that was not declared is refused.** Dropping it would leave the
  parameter it was meant for at its previous value and answer a query nobody asked.

Each refusal is paired with the neighbouring case that must still succeed. A
refusal that can never be satisfied is indistinguishable from one that is never
right.
"""

from __future__ import annotations

import pytest

import purrdf

EX = "http://example.org/"
QUERY = f"SELECT ?o WHERE {{ ?this <{EX}p> ?o }}"


def _first(solutions) -> purrdf.NamedNode:
    """The first column of the single solution in `solutions`."""
    assert len(solutions) == 1, f"expected exactly one solution, got {len(solutions)}"
    return next(iter(solutions))[0]


def _store(subjects: int = 4) -> purrdf.Store:
    store = purrdf.Store()
    lines = [f"<{EX}s{i}> <{EX}p> <{EX}o{i}> ." for i in range(subjects)]
    store.load("\n".join(lines), purrdf.RdfFormat.N_TRIPLES)
    return store


def test_each_binding_answers_for_its_own_value() -> None:
    store = _store()
    prepared = store.prepare(QUERY, parameters=["this"])
    assert prepared.parameters == ["this"]

    for i in range(4):
        # o{i} is the control: every other subject's object differs, so this cannot
        # pass for a binding that was dropped or reused.
        assert _first(prepared.run(this=purrdf.NamedNode(f"{EX}s{i}"))) == (
            purrdf.NamedNode(f"{EX}o{i}")
        )


def test_one_prepared_query_serves_many_runs() -> None:
    store = _store()
    prepared = store.prepare(QUERY, parameters=["this"])
    seen = [_first(prepared.run(this=purrdf.NamedNode(f"{EX}s{i}"))) for i in range(4)]
    assert seen == [purrdf.NamedNode(f"{EX}o{i}") for i in range(4)]
    # And the same handle still answers after all of them, so nothing about running
    # consumed it.
    assert _first(prepared.run(this=purrdf.NamedNode(f"{EX}s0"))) == purrdf.NamedNode(
        f"{EX}o0"
    )


def test_an_unbound_parameter_is_refused_but_a_bound_one_runs() -> None:
    store = _store()
    prepared = store.prepare(QUERY, parameters=["this"])

    with pytest.raises(ValueError, match="unbound"):
        prepared.run()

    # The neighbour: the same handle, once bound, must answer.
    assert _first(prepared.run(this=purrdf.NamedNode(f"{EX}s1"))) == purrdf.NamedNode(
        f"{EX}o1"
    )


def test_an_undeclared_parameter_name_is_refused() -> None:
    store = _store()
    prepared = store.prepare(QUERY, parameters=["this"])

    with pytest.raises(ValueError, match="absent"):
        prepared.run(absent=purrdf.NamedNode(f"{EX}s0"))

    # The neighbour: the declared name still binds.
    assert _first(prepared.run(this=purrdf.NamedNode(f"{EX}s2"))) == purrdf.NamedNode(
        f"{EX}o2"
    )


def test_declaring_one_parameter_twice_is_refused() -> None:
    store = _store()
    with pytest.raises(ValueError, match="more than once"):
        store.prepare(QUERY, parameters=["this", "this"])

    # The neighbour: distinct parameters prepare, including one the query does not
    # mention, which is an unused binding rather than an error.
    prepared = store.prepare(QUERY, parameters=["this", "other"])
    assert prepared.parameters == ["this", "other"]


def test_a_prepared_query_answers_over_its_snapshot() -> None:
    store = _store(subjects=2)
    prepared = store.prepare(QUERY, parameters=["this"])
    assert len(prepared.run(this=purrdf.NamedNode(f"{EX}s0"))) == 1

    # A statement added after preparing is not visible to the prepared query — it
    # holds the snapshot it was built from, which `prepare` states.
    store.load(f"<{EX}s9> <{EX}p> <{EX}o9> .", purrdf.RdfFormat.N_TRIPLES)
    assert len(prepared.run(this=purrdf.NamedNode(f"{EX}s9"))) == 0
    # But a freshly prepared one sees it, so the snapshot is the reason rather than
    # a lost write.
    reprepared = store.prepare(QUERY, parameters=["this"])
    assert _first(reprepared.run(this=purrdf.NamedNode(f"{EX}s9"))) == purrdf.NamedNode(
        f"{EX}o9"
    )
