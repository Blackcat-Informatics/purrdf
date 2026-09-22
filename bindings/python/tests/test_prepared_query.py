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

A fifth and sixth promise cover ``run``'s own statefulness:

* **A binding does not outlive the call that made it.** ``run`` reads as total: a
  keyword-argument call means "these are the bindings", not "these, plus whatever an
  earlier call left behind". A bare ``run()`` after a prior ``run(this=X)`` must be
  refused exactly as an initially-unbound handle is, not silently re-answer for
  ``X``.
* **``run`` reads the owning store fresh, every call.** What ``prepare`` admits is
  the PLAN, not the data: a mutation made after ``prepare`` — including one made
  between two ``run`` calls on the SAME handle — must be visible to the next run.
  This is the fixpoint / incremental-SHACL contract the surface exists for, and a
  fixpoint mutates its store every round by definition.

That last promise is the one a fixture of ordinary triple patterns cannot actually
test, because nothing in such a fixture could have been read once and kept. Two of
the eight configuration axes could: ``relations_from_graph`` reads a table out of the
store's own graph and ``path_relations`` traverses the store's own edges, so a handle
that built them at prepare time and kept them answers about the store as it WAS while
the rest of the query answers about the store as it IS — one answer assembled from two
points in time, with no error. Both are therefore driven here across a mutation made
between two runs on the SAME handle, with a control row that appears only after it.

Finally, **every one of the eight axes is driven by an executed treatment/control
pair.** An axis accepted at admission and then dropped is invisible — the call
succeeds, the query runs, and only the rows say the configuration never arrived — so
each pair is built so the control FAILS if the axis were dropped: a row that only the
honoured reading can produce, or a refusal only the honoured reading can raise. A
control that merely counts rows would not do, because the dropped reading is free to
answer the same number of them.
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


FN_NS = "http://example.org/fn/"
UNKNOWN_CALL = f"ASK {{ FILTER(<{FN_NS}nope>(1)) }}"
PARSE_CODE = "native-sparql-query-parse"


def test_prepare_honours_the_declared_extension_namespace_and_still_admits_a_query_without_it() -> (
    None
):
    """`extension_namespaces` is PARSE configuration, and `prepare` is a parse.

    The relation tests above cover the registry axis; this covers the other half of
    what decides what a query text MEANS. A declared extension namespace buys a
    stricter, earlier reading — an unknown local name in that namespace becomes a
    parse error instead of an unevaluable function — and that reading has to reach
    `prepare`, which is where the parse happens, or the keyword is accepted and
    dropped.

    The two diagnostic CODES are the whole oracle, and they are why this can fail:
    both readings raise `ValueError` carrying a message, so "it raised" is satisfied
    equally by a dropped keyword. Only the code says which reading ran. This mirrors
    `test_engine_configuration.py`'s oracle for the same axis on `Store.query`, so
    the claim is that the two doors read a query text the same way — not that this
    door raises something.
    """
    store = purrdf.Store()

    # Undeclared: the IRI is an ordinary custom function, so the text PARSES and
    # `prepare` admits it. (Evaluation is what would object; that is `query`'s
    # graded behaviour and is unchanged.)
    store.prepare(UNKNOWN_CALL)

    # Declared: the same text is refused at PARSE time, by name.
    with pytest.raises(ValueError) as refused:
        store.prepare(UNKNOWN_CALL, extension_namespaces=[FN_NS])
    message = str(refused.value)
    assert PARSE_CODE in message, (
        f"a declared namespace must move the refusal to parse time: {message}"
    )
    assert f"{FN_NS}nope" in message, "…and the refusal must name the function"

    # The neighbouring VALID case: declaring the namespace must not refuse a query
    # that never mentions it — and the handle must still run and answer.
    prepared = store.prepare("ASK { ?s ?p ?o }", extension_namespaces=[FN_NS])
    assert bool(prepared.run()) is False


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


def test_a_prior_binding_does_not_survive_into_a_later_unbound_call() -> None:
    """`run` reads as total: it must not silently re-answer a bare `run()` with a
    value an EARLIER call bound.

    `prepared.run(this=X)` binds the engine's `this` slot to `X`. Because
    `PreparedExecution` has no public way to unbind that slot, a naive `run` that
    only applies the keywords it is GIVEN would leave the slot at `X` forever, and a
    later bare `run()` would silently re-answer for `X` instead of being refused —
    exactly the same silently-wider answer the initially-unbound case is refused
    for. `o0` and `o1` differ, so a leaked `X` binding is distinguishable from a
    correct refusal.
    """
    store = _store()
    prepared = store.prepare(QUERY, parameters=["this"])

    assert _first(prepared.run(this=purrdf.NamedNode(f"{EX}s0"))) == purrdf.NamedNode(
        f"{EX}o0"
    )

    with pytest.raises(ValueError, match="unbound"):
        prepared.run()

    # The neighbour: a second bound run for a DIFFERENT value answers for THAT
    # value, not the one the first call bound.
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


def _rows(solutions: object) -> set[tuple[str, ...]]:
    """Every row of `solutions`, as a tuple of term values — `_pairs` generalized to
    an arbitrary column count.

    `QuerySolution` has no `__iter__`/`__len__` of its own (only `__getitem__` by
    name, `Variable`, or position), so the column count comes from `solutions`'
    own `variables` rather than iterating a row directly.
    """
    width = len(solutions.variables)  # type: ignore[attr-defined]
    return {
        tuple(str(row[i].value) for i in range(width))  # type: ignore[union-attr]
        for row in solutions  # type: ignore[attr-defined]
    }


YES = f"{EX}Yes"


def test_prepare_parameter_reaches_inside_optional() -> None:
    """(g) `?this` reaches an `OPTIONAL`, used ONLY inside its right arm.

    Unlike `EXISTS`, `OPTIONAL`'s right arm keeps its own columns in the
    `LeftJoin`'s output schema, so a parameter used only there still surfaces as an
    ordinary column the top-level binding can join against — this is the "ordinary
    correlation" the module doc promises, not a special case for `OPTIONAL`.

    Fixture: one anchor row (independent of the parameter) and two DIFFERENT
    `bonus` facts, one per candidate parameter value. A binding that reached inside
    the `OPTIONAL` answers with exactly the bound person's bonus; a binding that
    was silently dropped (leaving `?this` free inside the `OPTIONAL`) would let the
    right arm match BOTH bonus facts freely, producing the OTHER person's bonus
    too, regardless of which value was bound — the control row a dropped binding
    cannot avoid answering.
    """
    store = purrdf.Store()
    store.load(
        f"<{EX}anchor> <{EX}isAnchor> <{YES}> .\n"
        f"<{EX}alice> <{EX}bonus> <{EX}bonusA> .\n"
        f"<{EX}bob> <{EX}bonus> <{EX}bonusB> .\n",
        purrdf.RdfFormat.N_TRIPLES,
    )
    query = f"""
    SELECT ?anchor ?bonus WHERE {{
      ?anchor <{EX}isAnchor> <{YES}> .
      OPTIONAL {{ ?this <{EX}bonus> ?bonus }}
    }}
    """
    prepared = store.prepare(query, parameters=["this"])

    rows_alice = _rows(prepared.run(this=purrdf.NamedNode(f"{EX}alice")))
    assert rows_alice == {(f"{EX}anchor", f"{EX}bonusA")}
    assert (f"{EX}anchor", f"{EX}bonusB") not in rows_alice

    rows_bob = _rows(prepared.run(this=purrdf.NamedNode(f"{EX}bob")))
    assert rows_bob == {(f"{EX}anchor", f"{EX}bonusB")}
    assert (f"{EX}anchor", f"{EX}bonusA") not in rows_bob

    # Parity: `Store.query` given `?this` as an ordinary substitution must agree.
    direct_alice = _rows(
        store.query(query, substitutions={purrdf.Variable("this"): purrdf.NamedNode(f"{EX}alice")})
    )
    assert direct_alice == rows_alice


def test_prepare_parameter_reaches_inside_minus_and_respects_domain_disjointness() -> (
    None
):
    """(h) `?this` reaches a `MINUS`'s right arm — and only when the fixture keeps
    `MINUS`'s own domain-disjointness rule satisfied.

    `MINUS`'s right arm shares no columns with its output (unlike `OPTIONAL`'s):
    `Minus{Omega1, Omega2}` keeps only `Omega1`'s schema, and the SPARQL spec says a
    row of `Omega1` survives whenever it shares NO variable with `Omega2` at all — a
    `MINUS` whose right arm is disjoint from the left removes nothing, no matter
    what it says. So a parameter can only ever restrict a genuine `MINUS` by being
    the SAME variable on both arms; that is not a workaround, it is what "the
    parameter reaches inside `MINUS`" has to mean here.

    Fixture: `c1`/`c2` are owned by `alice`, `c3` by `bob`; `alice` alone is
    blocked. A genuine `MINUS` (right arm shares `?this` with left) removes every
    candidate owned by the bound value when, and only when, that value is blocked —
    `this=alice` empties the answer, `this=bob` keeps `c3`, two DIFFERENT specific
    answers a dropped binding could not produce (it would answer the same set
    either way). The SAME parameter value run through a VACUOUS `MINUS` (right arm
    uses `?owner`, not `?this` — no shared variable) removes nothing regardless,
    which is the control that shows the genuine case is doing real work and not
    just returning fewer rows by coincidence.
    """
    store = purrdf.Store()
    store.load(
        f"<{EX}c1> <{EX}ownedBy> <{EX}alice> .\n"
        f"<{EX}c2> <{EX}ownedBy> <{EX}alice> .\n"
        f"<{EX}c3> <{EX}ownedBy> <{EX}bob> .\n"
        f"<{EX}alice> <{EX}blocked> <{YES}> .\n",
        purrdf.RdfFormat.N_TRIPLES,
    )
    genuine = f"""
    SELECT ?candidate WHERE {{
      ?candidate <{EX}ownedBy> ?this .
      MINUS {{ ?this <{EX}blocked> <{YES}> }}
    }}
    """
    vacuous = f"""
    SELECT ?candidate WHERE {{
      ?candidate <{EX}ownedBy> ?owner .
      MINUS {{ ?this <{EX}blocked> <{YES}> }}
    }}
    """
    prepared_genuine = store.prepare(genuine, parameters=["this"])
    prepared_vacuous = store.prepare(vacuous, parameters=["this"])

    # Genuine MINUS: owner alice is blocked, so every candidate alice owns is
    # removed — the answer is EMPTY, not merely smaller.
    assert _rows(prepared_genuine.run(this=purrdf.NamedNode(f"{EX}alice"))) == set()
    # The neighbour: owner bob is not blocked, so bob's candidate survives.
    assert _rows(prepared_genuine.run(this=purrdf.NamedNode(f"{EX}bob"))) == {
        (f"{EX}c3",)
    }

    # Vacuous MINUS: `?this` shares no variable with the left arm, so the SAME
    # blocked value removes NOTHING — all three candidates answer, including the
    # ones a genuine `MINUS` over the same value emptied out above.
    assert _rows(prepared_vacuous.run(this=purrdf.NamedNode(f"{EX}alice"))) == {
        (f"{EX}c1",),
        (f"{EX}c2",),
        (f"{EX}c3",),
    }

    # Parity: `Store.query` with the same value as an ordinary substitution agrees
    # on the genuine case.
    direct = _rows(
        store.query(genuine, substitutions={purrdf.Variable("this"): purrdf.NamedNode(f"{EX}bob")})
    )
    assert direct == {(f"{EX}c3",)}


def test_prepare_parameter_reaches_inside_exists() -> None:
    """(i) `?this` reaches a `FILTER EXISTS`.

    `EXISTS` is boolean-valued: none of its own pattern's variables ever become a
    column of the surrounding row, so — unlike `OPTIONAL` — a parameter used ONLY
    inside an `EXISTS` body has no row to correlate through at all; that is
    ordinary SPARQL scoping, not something a pre-binding mechanism could fix. The
    fixture below instead exercises exactly what "reaches inside EXISTS by ordinary
    correlation" can mean: `?this` is ALSO bound by the surrounding pattern (as the
    correlation key `?candidate` joins through), and the question is whether the
    row's `?this`-value — the one the parameter supplied — is what `EXISTS` sees
    when it runs, not some other, unconstrained occurrence.

    Fixture: `alice` reports to `carol`, who IS verified; `bob` reports to `dave`,
    who is NOT verified, but `carol`'s verification fact is still present in the
    graph as a decoy. A binding that reached inside `EXISTS` answers `alice` for
    `this=carol` and EMPTY for `this=dave` — bob is excluded precisely because his
    manager fails the `EXISTS` check. A binding whose `EXISTS` reference to `?this`
    were left free (silently dropped) would see `carol`'s verified fact regardless
    of which manager the row actually named, and would wrongly answer `bob` too —
    the decoy this fixture exists to catch.
    """
    store = purrdf.Store()
    store.load(
        f"<{EX}alice> <{EX}reportsTo> <{EX}carol> .\n"
        f"<{EX}bob> <{EX}reportsTo> <{EX}dave> .\n"
        f"<{EX}carol> <{EX}verified> <{YES}> .\n",
        purrdf.RdfFormat.N_TRIPLES,
    )
    query = f"""
    SELECT ?candidate WHERE {{
      ?candidate <{EX}reportsTo> ?this .
      FILTER EXISTS {{ ?this <{EX}verified> <{YES}> }}
    }}
    """
    prepared = store.prepare(query, parameters=["this"])

    assert _rows(prepared.run(this=purrdf.NamedNode(f"{EX}carol"))) == {(f"{EX}alice",)}
    # The neighbour and the oracle in one: dave is not verified, so bob — whose
    # OWN `reportsTo` fact is perfectly genuine — must still be excluded. A leaked,
    # unconstrained `EXISTS` would answer `bob` here because `carol` (someone
    # else's manager) is verified.
    assert _rows(prepared.run(this=purrdf.NamedNode(f"{EX}dave"))) == set()

    # Parity: `Store.query` given `?this` as an ordinary substitution agrees.
    direct = _rows(
        store.query(query, substitutions={purrdf.Variable("this"): purrdf.NamedNode(f"{EX}carol")})
    )
    assert direct == {(f"{EX}alice",)}


def test_prepare_parameter_reaches_inside_a_sub_select() -> None:
    """(j) `?this` reaches a sub-`SELECT`, used ONLY inside it — because the
    sub-query PROJECTS it back out.

    A sub-`SELECT` is a separate scope: a variable it does not project is a
    DIFFERENT variable of the same name, invisible outside it (this is ordinary
    SPARQL scoping, not a `purrdf`-specific rule). `?this` is projected here, so it
    becomes a real column of the sub-query's own output, and the SAME top-level
    join that ordinarily correlates a bound variable does the rest — no special
    case is needed for a sub-`SELECT` any more than for `OPTIONAL`.

    Fixture: `alice` leads `teamA`, `bob` leads `teamB`; `carol` is `teamA`'s
    member, `dave` is `teamB`'s. The sub-`SELECT` resolves `?this`'s team; the
    outer pattern joins that team to its member. `this=alice` must answer `carol`
    only, `this=bob` must answer `dave` only — two DIFFERENT, specific answers. A
    binding that failed to reach the sub-`SELECT` would leave `?this` free inside
    it, so the sub-query would resolve BOTH teams regardless of the bound value,
    and BOTH `carol` and `dave` would leak into every answer — the control this
    fixture is built to catch.
    """
    store = purrdf.Store()
    store.load(
        f"<{EX}alice> <{EX}leads> <{EX}teamA> .\n"
        f"<{EX}bob> <{EX}leads> <{EX}teamB> .\n"
        f"<{EX}carol> <{EX}memberOfTeam> <{EX}teamA> .\n"
        f"<{EX}dave> <{EX}memberOfTeam> <{EX}teamB> .\n",
        purrdf.RdfFormat.N_TRIPLES,
    )
    query = f"""
    SELECT ?member WHERE {{
      ?member <{EX}memberOfTeam> ?team .
      {{ SELECT ?this ?team WHERE {{ ?this <{EX}leads> ?team }} }}
    }}
    """
    prepared = store.prepare(query, parameters=["this"])

    rows_alice = _rows(prepared.run(this=purrdf.NamedNode(f"{EX}alice")))
    assert rows_alice == {(f"{EX}carol",)}
    assert (f"{EX}dave",) not in rows_alice

    rows_bob = _rows(prepared.run(this=purrdf.NamedNode(f"{EX}bob")))
    assert rows_bob == {(f"{EX}dave",)}
    assert (f"{EX}carol",) not in rows_bob

    # Parity: `Store.query` given `?this` as an ordinary substitution agrees.
    direct = _rows(
        store.query(query, substitutions={purrdf.Variable("this"): purrdf.NamedNode(f"{EX}alice")})
    )
    assert direct == rows_alice


def test_a_prepared_query_reads_the_owning_store_fresh_on_every_run() -> None:
    """What `prepare` admits is the PLAN, not the data: a mutation made after
    `prepare` — even between two `run` calls on the SAME handle — must be visible to
    the next `run`.

    This is the fixpoint / incremental-SHACL contract `Store.prepare`'s module doc
    names as the motivating use case, and a fixpoint mutates its store every round
    by definition. The control below (`s0`, asserted BEFORE the mutation) is what
    keeps this test from passing vacuously: a handle that answered nothing, or that
    answered everything regardless of the mutation, would fail it too.
    """
    store = _store(subjects=2)
    prepared = store.prepare(QUERY, parameters=["this"])

    # Control: the pre-mutation answer, over what the store already held.
    assert len(prepared.run(this=purrdf.NamedNode(f"{EX}s0"))) == 1

    # A statement added after preparing — and after this SAME handle already ran —
    # must be visible to the next run on it.
    store.load(f"<{EX}s9> <{EX}p> <{EX}o9> .", purrdf.RdfFormat.N_TRIPLES)
    assert _first(prepared.run(this=purrdf.NamedNode(f"{EX}s9"))) == purrdf.NamedNode(
        f"{EX}o9"
    )


# ── the two axes whose ROWS come out of the store's own graph ────────────────────
#
# The test above drives ordinary triple patterns only, so it is green over a fixture
# that cannot tell "fresh" from "frozen": there is nothing in it that could have been
# read once and kept. `relations_from_graph` and `path_relations` are exactly that
# something — a table read out of an `rdf:List` of `rdf:List`s and a traversal over
# the store's own edges — and a handle that built them at prepare time and kept them
# answers about the store as it WAS while the rest of the query answers about the
# store as it IS. One answer, two points in time, no error.
#
# Both fixtures below therefore mutate BETWEEN runs on the SAME handle, and both
# assert the ROWS: the control row appears only after the mutation, so a frozen table
# cannot pass by answering the right count.

RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"

MEMBER_TABLE = f"{EX}memberTable"

#: The `rdf:List` of `rdf:List`s `relations_from_graph` reads — two rows, with the
#: tail cell NAMED rather than a blank node so a mutation can append a third row to
#: it. Written in the store's default graph, which is where the relation reads it.
_TABLE_TWO_ROWS_TTL = f"""
@prefix rdf: <{RDF}> .
@prefix ex: <{EX}> .

ex:memberTable rdf:first ( ex:ada ex:alpha ) ; rdf:rest ex:memberTableTail .
ex:memberTableTail rdf:first ( ex:brian ex:alpha ) ; rdf:rest rdf:nil .
"""

#: The third row, appended by re-pointing the named tail cell's `rdf:rest`.
_TABLE_THIRD_ROW_TTL = f"""
@prefix rdf: <{RDF}> .
@prefix ex: <{EX}> .

ex:memberTableTail rdf:rest ex:memberTableThird .
ex:memberTableThird rdf:first ( ex:chen ex:beta ) ; rdf:rest rdf:nil .
"""

_TABLE_FIRST_TWO_ROWS = {
    (f"{EX}ada", f"{EX}alpha"),
    (f"{EX}brian", f"{EX}alpha"),
}


def _from_graph_relations() -> dict[str, object]:
    """`memberOf` read out of the store's own graph, 1 subject / 1 object."""
    return {MEMBER_OF: (purrdf.NamedNode(MEMBER_TABLE), 1, 1)}


def _store_with_table_and_control_triple() -> purrdf.Store:
    """The two-row table, plus the same `dede`/`gamma` control triple the inline
    `relations` fixture uses — so a dropped registry is distinguishable by CONTENT
    here too, not merely by row count."""
    store = _store_with_control_triple()
    store.load(_TABLE_TWO_ROWS_TTL, purrdf.RdfFormat.TURTLE)
    return store


def _append_third_table_row(store: purrdf.Store) -> None:
    """Append `chen`/`beta` to the table already in `store`: un-terminate the named
    tail cell, then write the new cell."""
    store.remove(
        purrdf.Quad(
            purrdf.NamedNode(f"{EX}memberTableTail"),
            purrdf.NamedNode(f"{RDF}rest"),
            purrdf.NamedNode(f"{RDF}nil"),
        )
    )
    store.load(_TABLE_THIRD_ROW_TTL, purrdf.RdfFormat.TURTLE)


WALK = f"{REL}walk"

#: One row per hop of every simple-prefix walk out of `ex:a`, ordered by walk length
#: then hop. The object is a collection, which is also a well-formed `rdf:List`
#: pattern — that coincidence is what gives the control arm below a row to answer.
WALK_QUERY = (
    f"SELECT ?end ?len ?step ?node WHERE {{ <{EX}a> <{WALK}> "
    "( ?end ?pathId ?len ?step ?node ?edge ) } ORDER BY ?len ?step"
)

#: `ex:a -> ex:b -> ex:c`. Two hops, so three rows: `->b`, `->b->c`.
_TWO_HOP_CHAIN_TTL = f"""
@prefix ex: <{EX}> .

ex:a ex:p ex:b .
ex:b ex:p ex:c .
"""

#: The three-hop chain, plus an `rdf:List` written under the walk predicate itself
#: whose members no traversal of `ex:p` could ever produce.
_THREE_HOP_CHAIN_WITH_DECOY_TTL = f"""
@prefix ex: <{EX}> .
@prefix rel: <{REL}> .

ex:a ex:p ex:b .
ex:b ex:p ex:c .
ex:c ex:p ex:d .
ex:a rel:walk ( ex:decoyEnd ex:decoyPath ex:decoyLen ex:decoyStep ex:decoyNode ex:decoyEdge ) .
"""


def _walk_relations() -> dict[str, object]:
    """One `ex:p`-forward step, with every envelope field stated explicitly: there is
    no default `min_hops`, no default `max_hops` and no default guard."""
    return {WALK: ([(purrdf.NamedNode(f"{EX}p"), "forward")], 1, 4, 1024, 100_000, "walk")}


def _walk_rows(solutions: object) -> list[tuple[str, str, str, str]]:
    """`(end, len, step, node)` per row, local names only, values as written.

    Read as strings rather than ints because the control arm's cells are IRIs, not
    `xsd:integer` literals — which is precisely the difference being observed.
    """
    return [
        tuple(str(row[index].value).removeprefix(EX) for index in range(4))  # type: ignore[misc]
        for row in solutions  # type: ignore[attr-defined]
    ]


#: Three walks leave `ex:a` on the three-hop chain (`->b`, `->b->c`, `->b->c->d`), so
#: `1 + 2 + 3 = 6` rows.
_SIX_WALK_ROWS = [
    ("b", "1", "1", "b"),
    ("c", "2", "1", "b"),
    ("c", "2", "2", "c"),
    ("d", "3", "1", "b"),
    ("d", "3", "2", "c"),
    ("d", "3", "3", "d"),
]

#: What the ordinary-predicate reading answers: the decoy list, one row, no hops.
_DECOY_WALK_ROW = ("decoyEnd", "decoyLen", "decoyStep", "decoyNode")


def test_a_from_graph_relation_is_rebuilt_from_the_store_on_every_run() -> None:
    """`relations_from_graph` reads the STORE, so a mutation to the table between two
    runs on the SAME handle must reach the next run.

    The control is `chen`/`beta`: it is in no run's answer before the mutation and in
    every run's answer after it, so a handle that had frozen the table at prepare time
    fails here — it would keep answering the first two rows, which is exactly what it
    did before this was fixed. `dede`/`gamma` is asserted absent throughout, which is
    what keeps a silently-dropped registry (which answers that pair and nothing else)
    from passing either.
    """
    store = _store_with_table_and_control_triple()
    prepared = store.prepare(SELECT_MEMBERS, relations_from_graph=_from_graph_relations())

    before = _pairs(prepared.run())
    assert before == _TABLE_FIRST_TWO_ROWS
    assert _DROPPED_ROW not in before

    _append_third_table_row(store)

    after = _pairs(prepared.run())
    assert after == _HONOURED_ROWS
    assert (f"{EX}chen", f"{EX}beta") in after
    assert _DROPPED_ROW not in after

    # Parity: a handle prepared AFTER the mutation, and `Store.query` itself, answer
    # the same rows — so "after" is the store's real content and not a third reading.
    assert (
        _pairs(store.query(SELECT_MEMBERS, relations_from_graph=_from_graph_relations()))
        == after
    )


def test_a_path_relation_traverses_the_stores_edges_as_they_are_on_every_run() -> None:
    """`path_relations` traverses the STORE's edges, so an edge added between two runs
    on the SAME handle must be traversable by the next run.

    The control is the `ex:d` endpoint and the three-hop walk that reaches it: absent
    from every row before the mutation, present after. A handle that had frozen the
    traversal graph at prepare time keeps answering the three rows of the two-hop
    chain, which is what it did before this was fixed.
    """
    store = purrdf.Store()
    store.load(_TWO_HOP_CHAIN_TTL, purrdf.RdfFormat.TURTLE)
    prepared = store.prepare(WALK_QUERY, path_relations=_walk_relations())

    before = _walk_rows(prepared.run())
    assert before == [("b", "1", "1", "b"), ("c", "2", "1", "b"), ("c", "2", "2", "c")]
    assert not [row for row in before if row[0] == "d"]

    # One more edge on the end of the chain, added after this handle already ran.
    store.load(f"@prefix ex: <{EX}> .\nex:c ex:p ex:d .\n", purrdf.RdfFormat.TURTLE)

    after = _walk_rows(prepared.run())
    assert after == _SIX_WALK_ROWS
    assert [row for row in after if row[0] == "d"] == [
        ("d", "3", "1", "b"),
        ("d", "3", "2", "c"),
        ("d", "3", "3", "d"),
    ]

    # Parity with the door that never held a snapshot at all.
    assert _walk_rows(store.query(WALK_QUERY, path_relations=_walk_relations())) == after


SELECT_TEAM_OF = f"SELECT ?team WHERE {{ ?this <{MEMBER_OF}> ?team }}"


def test_a_parameter_binds_against_the_rebuilt_relation_not_the_one_prepare_built() -> (
    None
):
    """Rebuilding a graph-derived relation re-admits the plan, which is a NEW set of
    parameter slots — so the two have to meet in the right order, and the handle has
    to look unchanged from Python.

    `chen` is the control, and it is the tightest one available: before the mutation
    the table names no `chen` at all, so the query answers nothing for that binding;
    after it, `chen` is in the table and the answer is `beta`. A handle that bound
    into the execution the rebuild then discarded would refuse the run as unbound
    instead; one that skipped the rebuild would keep answering nothing.
    """
    store = _store_with_table_and_control_triple()
    prepared = store.prepare(
        SELECT_TEAM_OF, parameters=["this"], relations_from_graph=_from_graph_relations()
    )
    assert prepared.parameters == ["this"]

    ada = purrdf.NamedNode(f"{EX}ada")
    chen = purrdf.NamedNode(f"{EX}chen")
    assert [str(row[0].value) for row in prepared.run(this=ada)] == [f"{EX}alpha"]  # type: ignore[union-attr]
    assert len(prepared.run(this=chen)) == 0  # type: ignore[arg-type]

    _append_third_table_row(store)

    # The declared slots are the ones the caller named, across the re-admission.
    assert prepared.parameters == ["this"]
    assert [str(row[0].value) for row in prepared.run(this=chen)] == [f"{EX}beta"]  # type: ignore[union-attr]
    # …and `ada` still answers, so the rebuild replaced the table rather than the
    # relation's meaning.
    assert [str(row[0].value) for row in prepared.run(this=ada)] == [f"{EX}alpha"]  # type: ignore[union-attr]

    # `run`'s own promises survive the re-admission: a binding does not outlive the
    # call that made it, and an undeclared name is still refused.
    with pytest.raises(ValueError, match="still unbound"):
        prepared.run()
    with pytest.raises(ValueError, match="no parameter named"):
        prepared.run(nope=ada)


# ── the five configuration axes, each with the control that can fail ─────────────


def test_prepare_honours_relations_from_graph_against_a_control_row_that_would_differ_if_dropped() -> (
    None
):
    """Treatment: the table's three rows. Control: the one `memberOf` TRIPLE the graph
    holds, `dede`/`gamma`, which is in no row of the table.

    An axis that was accepted and dropped lowers `<MEMBER_OF>` to an ordinary triple
    pattern, and this graph holds exactly one — so a dropped axis answers exactly
    `dede`/`gamma` and an honoured one never does. The two outcomes differ in CONTENT,
    so neither can be mistaken for the other.
    """
    store = _store_with_table_and_control_triple()
    _append_third_table_row(store)

    honoured = _pairs(
        store.prepare(SELECT_MEMBERS, relations_from_graph=_from_graph_relations()).run()
    )
    assert honoured == _HONOURED_ROWS
    assert _DROPPED_ROW not in honoured

    # The control arm: the SAME store and the SAME query text, with the axis omitted.
    dropped = _pairs(store.prepare(SELECT_MEMBERS).run())
    assert dropped == {_DROPPED_ROW}


def test_prepare_honours_path_relations_against_a_control_row_that_would_differ_if_dropped() -> (
    None
):
    """Treatment: the six hop rows of every simple-prefix walk out of `ex:a`. Control:
    the decoy `rdf:List` written under the SAME predicate.

    `?start <iri> ( ?end ?pathId ?len ?step ?node ?edge )` is a collection in object
    position, which is an `rdf:List` pattern once `<iri>` is read as an ordinary
    predicate. This graph holds one such list, whose members are `decoy*` IRIs that no
    traversal can produce — so a dropped axis answers exactly that one row and an
    honoured one answers six rows naming nodes of the chain. Content, not count.
    """
    store = purrdf.Store()
    store.load(_THREE_HOP_CHAIN_WITH_DECOY_TTL, purrdf.RdfFormat.TURTLE)

    honoured = _walk_rows(store.prepare(WALK_QUERY, path_relations=_walk_relations()).run())
    assert honoured == _SIX_WALK_ROWS
    assert _DECOY_WALK_ROW not in honoured

    dropped = _walk_rows(store.prepare(WALK_QUERY).run())
    assert dropped == [_DECOY_WALK_ROW]


AGG_NS = "https://example.org/agg#"
MEDIAN_QUERY = f"SELECT (AGG(<{AGG_NS}MEDIAN>, ?v) AS ?m) WHERE {{ ?s <{EX}value> ?v }}"
AGGREGATE_CODE = "native-sparql-aggregate-function"

_NUMBERS_TTL = f"""
@prefix ex: <{EX}> .

ex:s1 ex:value 1 .
ex:s2 ex:value 2 .
ex:s3 ex:value 3 .
"""


def test_prepare_honours_aggregate_namespace_and_refuses_the_call_without_it() -> None:
    """Treatment: `MEDIAN` over `1, 2, 3` is `2` — a COMPUTED value, which only a
    registered aggregate can produce. Control: with the namespace omitted, the same
    text names an unregistered custom aggregate and `prepare` refuses it.

    The control is what makes the treatment mean something: purrdf mints no aggregate
    vocabulary, so `<AGG_NS>MEDIAN` is the caller's own IRI and is registered only
    because `aggregate_namespace` said so. An axis accepted and dropped would leave
    the refusal in place, so the treatment could not answer `2` at all.
    """
    store = purrdf.Store()
    store.load(_NUMBERS_TTL, purrdf.RdfFormat.TURTLE)

    solutions = store.prepare(MEDIAN_QUERY, aggregate_namespace=AGG_NS).run()
    assert [str(row[0].value) for row in solutions] == ["2"]  # type: ignore[union-attr]

    with pytest.raises(ValueError) as refused:
        store.prepare(MEDIAN_QUERY)
    message = str(refused.value)
    assert AGGREGATE_CODE in message, (
        f"an unregistered custom aggregate is refused by name: {message}"
    )
    assert f"{AGG_NS}MEDIAN" in message, "…and the refusal names the aggregate"


UNREGISTERED_REL = f"{REL}unregistered"
UNREGISTERED_QUERY = f"SELECT ?o WHERE {{ <{EX}ada> <{UNREGISTERED_REL}> ?o }}"
PROPERTY_FN_CODE = "native-sparql-property-function"


def test_prepare_honours_property_fn_namespaces_and_reads_an_undeclared_iri_as_data() -> (
    None
):
    """Treatment: declaring the namespace makes an UNREGISTERED IRI under it a hard
    error at prepare time. Control: with nothing declared, the same IRI is an ordinary
    predicate and the same text answers the triple the graph actually holds.

    That control row is the whole oracle. Declaring a property-function namespace is a
    request for the STRICTER reading — "an IRI under here is a call, and a call to
    something unregistered is a mistake" — and the only way to see that the request
    arrived is that the looser reading, which answers a row, is what happens without
    it. An axis accepted and dropped would answer that row in both arms.
    """
    store = purrdf.Store()
    store.load(
        f"<{EX}ada> <{UNREGISTERED_REL}> <{EX}alpha> .", purrdf.RdfFormat.N_TRIPLES
    )

    with pytest.raises(ValueError) as refused:
        store.prepare(UNREGISTERED_QUERY, property_fn_namespaces=[REL])
    message = str(refused.value)
    assert PROPERTY_FN_CODE in message, (
        f"a declared namespace makes an unregistered IRI under it a call: {message}"
    )
    assert UNREGISTERED_REL in message, "…and the refusal names the IRI"

    # The neighbouring valid case: undeclared, the IRI is data and the query answers.
    undeclared = store.prepare(UNREGISTERED_QUERY).run()
    assert [str(row[0].value) for row in undeclared] == [f"{EX}alpha"]  # type: ignore[union-attr]


STANDPOINT_EX = "https://example.org/"
EXT_NS = f"{STANDPOINT_EX}ext/"
HELD_IN = f"ASK {{ FILTER(<{EXT_NS}heldIn>(<{STANDPOINT_EX}r>, <{STANDPOINT_EX}s>)) }}"
STANDPOINT = (f"{STANDPOINT_EX}accordingTo", f"{STANDPOINT_EX}sharpens")
HELD_IN_CODE = "native-sparql-heldin-unconfigured"


def test_prepare_honours_standpoint_predicates_with_extension_namespaces_held_fixed() -> (
    None
):
    """Treatment: `extension_namespaces` AND `standpoint_predicates` — the query
    ANSWERS `False`. Control: `extension_namespaces` alone — the run is refused,
    naming the configuration it has no default for.

    `extension_namespaces` is held fixed across both arms deliberately. Varying it
    would test that axis instead: without it the refusal is a different one entirely
    (an unevaluable call-position IRI), and a test whose two arms differ in two axes
    cannot say which one it observed. With it held fixed, the only difference is the
    predicate pair, and the two outcomes are an ANSWER and a refusal — an axis
    accepted and dropped leaves the refusal, so `False` is unreachable.
    """
    store = purrdf.Store()

    answered = store.prepare(
        HELD_IN, extension_namespaces=[EXT_NS], standpoint_predicates=STANDPOINT
    ).run()
    assert isinstance(answered, purrdf.QueryBoolean)
    assert bool(answered) is False, (
        "an empty store holds nothing in any standpoint, and saying so is an answer "
        "rather than a refusal"
    )

    # The control arm: the same namespace declaration, the same text, no pair.
    with pytest.raises(ValueError) as refused:
        store.prepare(HELD_IN, extension_namespaces=[EXT_NS]).run()
    message = str(refused.value)
    assert HELD_IN_CODE in message, (
        f"`heldIn` with no pair configured is refused by name: {message}"
    )
    assert "no built-in default" in message, (
        f"…and the refusal says there is nothing to fall back on: {message}"
    )
