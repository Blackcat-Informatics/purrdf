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

A fourth promise is added here: **``Store.prepare`` carries the SAME engine
configuration ``Store.query`` accepts** — ``relations`` / ``relations_from_graph`` /
``path_relations`` and the rest — and ``PreparedQuery.run`` evaluates under the SAME
registries the plan was admitted under. Before this was true, ``Store.prepare``
accepted none of those keywords at all: a relation's predicate was lowered to an
ordinary triple pattern at prepare time, with no registry ever in scope to reach it,
and a run answered the empty bag with no error and no warning. The fixture below is
built so that outcome and the honoured one are DIFFERENT IN CONTENT, not merely
different in row count, per the repository's rule that a valid-neighbour assertion
over a fixture that cannot distinguish "honoured" from "silently dropped" is
laundering.
"""

from __future__ import annotations

import pytest

import purrdf

EX = "http://example.org/"
QUERY = f"SELECT ?o WHERE {{ ?this <{EX}p> ?o }}"

REL = "http://example.org/rel/"
MEMBER_OF = f"{REL}memberOf"
SELECT_MEMBERS = f"SELECT ?person ?team WHERE {{ ?person <{MEMBER_OF}> ?team }}"


def _member(local: str) -> purrdf.NamedNode:
    return purrdf.NamedNode(f"{EX}{local}")


def _member_rows() -> list[list[purrdf.NamedNode]]:
    """The registered `memberOf` table: two members of one team, one of another."""
    return [
        [_member("ada"), _member("alpha")],
        [_member("brian"), _member("alpha")],
        [_member("chen"), _member("beta")],
    ]


def _member_relations() -> dict[str, object]:
    """`memberOf` declared as a 1-subject / 1-object tuple relation."""
    return {MEMBER_OF: (1, 1, _member_rows())}


def _store_with_control_triple() -> purrdf.Store:
    """A store holding one ORDINARY `memberOf` triple that names a pair absent from
    every row of `_member_rows()`: `dede`/`gamma`.

    This is the oracle a silently-dropped registry cannot pass. A relation that is
    honoured answers exactly `_member_rows()`'s three pairs, none of which is
    `dede`/`gamma` — those rows come from the relation's own table, never from the
    graph. A registry that was silently dropped instead lowers `<MEMBER_OF>` to an
    ordinary triple pattern, and the ONLY `memberOf` triple this graph holds is
    `dede`/`gamma` — so a dropped registry answers exactly that one pair, and an
    honoured one never does. The two outcomes cannot be confused for one another,
    unlike a fixture that only differs by row COUNT.
    """
    store = purrdf.Store()
    store.load(f"<{EX}dede> <{MEMBER_OF}> <{EX}gamma> .", purrdf.RdfFormat.N_TRIPLES)
    return store


def _pairs(solutions: object) -> set[tuple[str, str]]:
    """A two-column SELECT result as a set of `(iri, iri)` pairs."""
    return {(str(row[0].value), str(row[1].value)) for row in solutions}  # type: ignore[union-attr]


_HONOURED_ROWS = {
    (f"{EX}ada", f"{EX}alpha"),
    (f"{EX}brian", f"{EX}alpha"),
    (f"{EX}chen", f"{EX}beta"),
}
_DROPPED_ROW = (f"{EX}dede", f"{EX}gamma")


def test_prepare_honours_relations_against_a_control_row_that_would_differ_if_dropped() -> (
    None
):
    """(a) The honoured case, with an oracle that can fail.

    `Store.prepare(..., relations=...)` must admit the plan under the SAME registry
    `Store.query` would, and `PreparedQuery.run` must evaluate under it too — so the
    rows answered are the relation's own table, and never the control triple a
    dropped registry would have answered instead.
    """
    store = _store_with_control_triple()
    prepared = store.prepare(SELECT_MEMBERS, relations=_member_relations())

    rows = _pairs(prepared.run())

    assert rows == _HONOURED_ROWS
    assert _DROPPED_ROW not in rows


def test_prepare_and_query_agree_under_the_same_relations() -> None:
    """(b) The parity case — the regression oracle for the exact gap this closes.

    `Store.query` and `Store.prepare` over identical query text and identical
    `relations` configuration must answer the SAME rows. This is the exact
    demonstration that opened the gap: `Store.query` answered the relation's rows,
    `Store.prepare` silently answered the empty bag, with no error and no warning.
    """
    store = _store_with_control_triple()

    direct = _pairs(store.query(SELECT_MEMBERS, relations=_member_relations()))
    prepared = store.prepare(SELECT_MEMBERS, relations=_member_relations())
    via_prepare = _pairs(prepared.run())

    assert direct == via_prepare == _HONOURED_ROWS


def test_a_relation_iri_with_no_relation_configured_answers_the_graphs_own_triples() -> (
    None
):
    """(c) The refusal case — observed, not assumed.

    An unregistered relation IRI is not a hard error on this surface, on `query` or
    on `prepare`: with no `property_fn_namespaces` declared for it, it is an
    ORDINARY predicate IRI, and the query answers whatever the graph actually holds
    under that predicate — here, exactly the control triple. This is the SAME
    behaviour `Store.query` has always had for an unregistered relation IRI; the
    control triple below is only reachable this way, which is what makes this the
    neighbour of the two tests above rather than an independent claim.
    """
    store = _store_with_control_triple()

    prepared = store.prepare(SELECT_MEMBERS)

    assert _pairs(prepared.run()) == {_DROPPED_ROW}
    # The neighbour: `Store.query` (no `relations`) answers identically, so this is
    # `prepare`'s existing "no registry configured" behaviour, not a new one.
    assert _pairs(store.query(SELECT_MEMBERS)) == {_DROPPED_ROW}


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
