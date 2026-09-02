# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Host-injected property functions on the native query/update surface.

A property function is a RELATION invoked from predicate position: unlike an
extension function it is a row source, so one call may emit zero, one, or many
rows. The Rust tier has carried the seam for a while; these tests hold the Python
surface to what makes it usable and safe from a host that writes Python:

* **A relation is DATA, not a callable.** ``relations`` carries tuples,
  ``relations_from_graph`` carries the head of an ``rdf:List`` of ``rdf:List``s
  written in the store's own default graph, and ``path_relations`` carries a
  TRAVERSAL SPECIFICATION over the store's own edges. Nothing the engine invokes can
  re-enter the interpreter, which is why the whole evaluation still runs with the GIL
  released — the path spelling keeps that property by declaring which edges a hop may
  follow rather than by calling back per hop.
* **A path relation binds the DERIVATION, not the endpoints.** ``?start <iri> ( ?end
  ?pathId ?len ?step ?node ?edge )`` emits one row per hop, and ``?edge`` is the
  traversed statement as a first-class RDF 1.2 term that joins straight back into the
  dataset. ``GROUP BY ?pathId`` with ``ORDER BY ?step`` reassembles a whole walk.
  Every field of the envelope is mandatory: there is no default ``min_hops``, no
  default ``max_hops``, and no default guard, because a zero-hop path has no witness
  and an unbounded depth is a stack-overflow abort.
* **Registration is per call.** A relation named on one call is not in scope on the
  next; the default (no keyword) is exactly the pre-existing evaluation.
* **A registered IRI is reachable EXACTLY**, with no namespace declaration.
  Declaring ``property_fn_namespaces`` asks for the stricter reading instead, in
  which an unregistered IRI under the namespace is a hard error rather than a
  triple pattern that quietly matches nothing.
* **A misconfiguration raises.** A duplicate IRI, a ragged table, a torn list, and
  a head naming nothing are all refused where they are supplied, carrying the
  kernel's own diagnostic text.
"""

from __future__ import annotations

from typing import Any

import pytest

import purrdf

EX = "http://example.org/"
REL = "http://example.org/rel/"

MEMBER_OF = f"{REL}memberOf"
TEAM_SITE = f"{REL}teamSite"
SEEDS = f"{REL}seeds"
DISPLAY_NAME = f"{REL}displayName"

SELECT_MEMBERS = f"SELECT ?person ?team WHERE {{ ?person <{MEMBER_OF}> ?team }}"

# The `rdf:List` of `rdf:List`s encoding `relations_from_graph` reads: one inner
# list per row, holding that row's values in flattened order. The head is an IRI
# rather than a blank node so a caller has a stable name to pass — its cell is
# spelled out and only the tail uses the collection shorthand.
MEMBER_TABLE_TTL = f"""
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix ex: <{EX}> .

ex:memberTable
    rdf:first ( ex:ada ex:alpha ) ;
    rdf:rest  ( ( ex:brian ex:alpha ) ( ex:chen ex:beta ) ) .
"""

TORN_TABLE_TTL = f"""
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix ex: <{EX}> .

ex:tornTable
    rdf:first ( ex:ada ex:alpha ) ;
    rdf:rest  ex:notACons .
"""

WIDE_TABLE_TTL = f"""
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix ex: <{EX}> .

ex:wideTable
    rdf:first ( ex:ada ex:alpha ex:extra ) ;
    rdf:rest  rdf:nil .
"""


def _node(local: str) -> purrdf.NamedNode:
    """An `example.org` IRI term."""
    return purrdf.NamedNode(f"{EX}{local}")


def _member_rows() -> list[list[purrdf.NamedNode]]:
    """The `memberOf` table: two members of one team, one of another."""
    return [
        [_node("ada"), _node("alpha")],
        [_node("brian"), _node("alpha")],
        [_node("chen"), _node("beta")],
    ]


def _member_relations() -> dict[str, Any]:
    """`memberOf` declared as a 1-subject / 1-object tuple relation."""
    return {MEMBER_OF: (1, 1, _member_rows())}


def _pairs(solutions: Any) -> list[tuple[str, str]]:
    """A two-column SELECT result as `(local, local)` pairs, in result order."""
    return [
        (str(row[0].value).removeprefix(EX), str(row[1].value).removeprefix(EX))
        for row in solutions
    ]


def _column(solutions: Any) -> list[str]:
    """A one-column SELECT result as local names, in result order."""
    return [str(row[0].value).removeprefix(EX) for row in solutions]


def _store_with(turtle: str) -> purrdf.Store:
    """A store holding `turtle` in its default graph."""
    store = purrdf.Store()
    store.load(turtle, purrdf.RdfFormat.TURTLE)
    return store


# ── tuple relations ───────────────────────────────────────────────────────────────


def test_a_tuple_relation_answers_a_query() -> None:
    """Rows obtainable ONLY from the relation: the store holds no such triple."""
    store = purrdf.Store()

    solutions = store.query(SELECT_MEMBERS, relations=_member_relations())

    assert _pairs(solutions) == [
        ("ada", "alpha"),
        ("brian", "alpha"),
        ("chen", "beta"),
    ]


def test_the_same_query_without_the_relation_answers_nothing() -> None:
    """Unregistered and undeclared, the call IRI is an ordinary triple pattern."""
    assert len(purrdf.Store().query(SELECT_MEMBERS)) == 0  # type: ignore[arg-type]


def test_a_relation_is_registered_for_one_call_only() -> None:
    """Registration is per call: the next query sees no relation at all."""
    store = purrdf.Store()

    assert len(store.query(SELECT_MEMBERS, relations=_member_relations())) == 3  # type: ignore[arg-type]
    assert len(store.query(SELECT_MEMBERS)) == 0  # type: ignore[arg-type]


def test_a_bound_subject_looks_the_object_side_up() -> None:
    """A `bf` call filters the table on the bound position."""
    store = purrdf.Store()

    solutions = store.query(
        f"SELECT ?team WHERE {{ <{EX}chen> <{MEMBER_OF}> ?team }}",
        relations=_member_relations(),
    )

    assert _column(solutions) == ["beta"]


def test_a_call_joins_against_the_stored_data() -> None:
    """The relation is a row source in a graph pattern, so it joins like any other."""
    store = _store_with(f"<{EX}ada> <{EX}status> <{EX}active> .")

    solutions = store.query(
        f"SELECT ?person ?team WHERE {{ ?person <{EX}status> <{EX}active> . "
        f"?person <{MEMBER_OF}> ?team }}",
        relations=_member_relations(),
    )

    assert _pairs(solutions) == [("ada", "alpha")]


def test_a_relation_may_have_several_object_positions() -> None:
    """A 1/2 relation binds two OUTPUT positions no triple pattern can bind at once."""
    store = purrdf.Store()

    solutions = store.query(
        f"SELECT ?city ?headcount WHERE {{ <{EX}alpha> <{TEAM_SITE}> ( ?city ?headcount ) }}",
        relations={
            TEAM_SITE: (
                1,
                2,
                [
                    [_node("alpha"), purrdf.Literal("Zurich"), purrdf.Literal("2")],
                    [_node("beta"), purrdf.Literal("Osaka"), purrdf.Literal("1")],
                ],
            )
        },
    )

    rows = [(str(row[0].value), str(row[1].value)) for row in solutions]  # type: ignore[union-attr]
    assert rows == [("Zurich", "2")]


def test_a_relation_may_have_an_empty_subject_side() -> None:
    """A 0/1 generator takes no input and enumerates its whole table."""
    store = purrdf.Store()

    solutions = store.query(
        f"SELECT ?team WHERE {{ () <{SEEDS}> ?team }}",
        relations={SEEDS: (0, 1, [[_node("alpha")], [_node("beta")]])},
    )

    assert _column(solutions) == ["alpha", "beta"]


def test_a_relation_cell_converts_exactly_as_a_substitution_value_does() -> None:
    """Literals reach the engine through the one Python→term converter."""
    store = purrdf.Store()

    solutions = store.query(
        f'SELECT ?name WHERE {{ <{EX}ada> <{DISPLAY_NAME}> ?name . FILTER(?name = "Ada") }}',
        relations={
            DISPLAY_NAME: (
                1,
                1,
                [
                    [_node("ada"), purrdf.Literal("Ada")],
                    [_node("brian"), purrdf.Literal("Brian", language="en")],
                ],
            )
        },
    )

    rows = [row[0] for row in solutions]  # type: ignore[union-attr]
    assert [term.value for term in rows] == ["Ada"]
    assert rows[0].datatype.value == "http://www.w3.org/2001/XMLSchema#string"


# ── relations read out of the store's own dataset ────────────────────────────────


def test_a_from_graph_relation_answers_a_query() -> None:
    """The table is data in the store's default graph, read through `from_graph`."""
    store = _store_with(MEMBER_TABLE_TTL)

    solutions = store.query(
        SELECT_MEMBERS,
        relations_from_graph={MEMBER_OF: (_node("memberTable"), 1, 1)},
    )

    assert _pairs(solutions) == [
        ("ada", "alpha"),
        ("brian", "alpha"),
        ("chen", "beta"),
    ]


def test_a_from_graph_relation_agrees_with_the_tuple_spelling() -> None:
    """Two spellings of one table, one answer."""
    from_graph = _store_with(MEMBER_TABLE_TTL).query(
        SELECT_MEMBERS,
        relations_from_graph={MEMBER_OF: (_node("memberTable"), 1, 1)},
    )
    tuples = purrdf.Store().query(SELECT_MEMBERS, relations=_member_relations())

    assert _pairs(from_graph) == _pairs(tuples)


def test_the_two_relation_keywords_compose() -> None:
    """One call may take some relations as data and some out of the dataset."""
    store = _store_with(MEMBER_TABLE_TTL)

    solutions = store.query(
        f"SELECT ?person ?team WHERE {{ ?person <{MEMBER_OF}> ?team . "
        f"() <{SEEDS}> ?team }}",
        relations={SEEDS: (0, 1, [[_node("beta")]])},
        relations_from_graph={MEMBER_OF: (_node("memberTable"), 1, 1)},
    )

    assert _pairs(solutions) == [("chen", "beta")]


# ── namespace declaration and the unregistered call ──────────────────────────────


def test_an_unregistered_iri_under_a_declared_namespace_is_a_hard_error() -> None:
    """The stricter reading: a call that resolves to nothing is refused, not empty."""
    store = purrdf.Store()

    with pytest.raises(ValueError, match="no property function is registered"):
        store.query(
            f"SELECT ?team WHERE {{ <{EX}ada> <{REL}notRegistered> ?team }}",
            property_fn_namespaces=[REL],
            relations=_member_relations(),
        )


def test_an_unregistered_iri_without_a_declared_namespace_is_a_triple_pattern() -> None:
    """Without the declaration only REGISTERED IRIs are calls, so nothing is hijacked."""
    store = _store_with(f"<{EX}ada> <{REL}notRegistered> <{EX}alpha> .")

    solutions = store.query(
        f"SELECT ?team WHERE {{ <{EX}ada> <{REL}notRegistered> ?team }}",
        relations=_member_relations(),
    )

    assert _column(solutions) == ["alpha"]


def test_a_registered_iri_needs_no_namespace_declaration() -> None:
    """Exact-IRI recognition is derived from the registry itself."""
    solutions = purrdf.Store().query(SELECT_MEMBERS, relations=_member_relations())

    assert len(solutions) == 3  # type: ignore[arg-type]


# ── the governed surface ─────────────────────────────────────────────────────────


def test_a_governed_query_over_a_relation_returns_rows_and_a_receipt() -> None:
    """A relation's rows are charged like every other row source."""
    outcome = purrdf.Store().query_governed(
        SELECT_MEMBERS, relations=_member_relations()
    )

    assert outcome.is_complete
    assert outcome.tripped is None
    assert _pairs(outcome.result) == _pairs(
        purrdf.Store().query(SELECT_MEMBERS, relations=_member_relations())
    )
    assert outcome.evidence.consumed["answer-rows"] == 3
    assert outcome.evidence.consumed["fuel"] > 0


def test_a_governed_query_over_a_relation_can_be_stopped_by_a_ceiling() -> None:
    """An answer cap one below the relation's row count trips, and is not raised."""
    outcome = purrdf.Store().query_governed(
        SELECT_MEMBERS, relations=_member_relations(), max_answers=2
    )

    assert not outcome.is_complete
    assert outcome.tripped is not None


def test_a_governed_query_over_a_from_graph_relation_returns_rows() -> None:
    """The dataset-read spelling reaches the governed entry too."""
    outcome = _store_with(MEMBER_TABLE_TTL).query_governed(
        SELECT_MEMBERS,
        relations_from_graph={MEMBER_OF: (_node("memberTable"), 1, 1)},
    )

    assert outcome.is_complete
    assert len(outcome.result) == 3  # type: ignore[arg-type]


# ── UPDATE ───────────────────────────────────────────────────────────────────────


def test_an_update_where_over_a_relation_inserts_its_rows() -> None:
    """An UPDATE's WHERE is a triple-pattern context exactly as a query's is."""
    store = purrdf.Store()

    store.update(
        f"INSERT {{ ?person <{EX}team> ?team }} WHERE {{ ?person <{MEMBER_OF}> ?team }}",
        relations=_member_relations(),
    )

    solutions = store.query(
        f"SELECT ?person ?team WHERE {{ ?person <{EX}team> ?team }} ORDER BY ?person"
    )
    assert _pairs(solutions) == [
        ("ada", "alpha"),
        ("brian", "alpha"),
        ("chen", "beta"),
    ]


def test_a_governed_update_over_a_relation_applies_and_reports() -> None:
    """The governed UPDATE entry carries the registry and reports it applied."""
    store = purrdf.Store()

    outcome = store.update_governed(
        f"INSERT {{ ?person <{EX}team> ?team }} WHERE {{ ?person <{MEMBER_OF}> ?team }}",
        relations=_member_relations(),
    )

    assert outcome.is_applied
    assert len(store.query(f"SELECT ?p WHERE {{ ?p <{EX}team> ?t }}")) == 3  # type: ignore[arg-type]


def test_an_update_reads_its_from_graph_table_from_the_pre_update_state() -> None:
    """The table and the WHERE clause see one and the same snapshot."""
    store = _store_with(MEMBER_TABLE_TTL)

    store.update(
        f"INSERT {{ ?person <{EX}team> ?team }} WHERE {{ ?person <{MEMBER_OF}> ?team }}",
        relations_from_graph={MEMBER_OF: (_node("memberTable"), 1, 1)},
    )

    assert len(store.query(f"SELECT ?p WHERE {{ ?p <{EX}team> ?t }}")) == 3  # type: ignore[arg-type]


# ── misconfiguration is refused where it is supplied ─────────────────────────────


def test_a_ragged_tuple_table_is_refused() -> None:
    """A row narrower than the declared arity is a host configuration error."""
    with pytest.raises(ValueError, match="requires 2"):
        purrdf.Store().query(
            SELECT_MEMBERS,
            relations={MEMBER_OF: (1, 1, [[_node("ada")]])},
        )


def test_a_torn_list_is_refused() -> None:
    """An `rdf:rest` pointing at a non-cell is not a table, and says so."""
    store = _store_with(TORN_TABLE_TTL)

    with pytest.raises(ValueError, match="not an rdf:List"):
        store.query(
            SELECT_MEMBERS,
            relations_from_graph={MEMBER_OF: (_node("tornTable"), 1, 1)},
        )


def test_a_from_graph_row_of_the_wrong_width_is_refused() -> None:
    """The arity check applies to the dataset-read spelling identically."""
    store = _store_with(WIDE_TABLE_TTL)

    with pytest.raises(ValueError, match="requires 2"):
        store.query(
            SELECT_MEMBERS,
            relations_from_graph={MEMBER_OF: (_node("wideTable"), 1, 1)},
        )


def test_a_head_that_names_nothing_is_refused() -> None:
    """A head absent from the dataset is configuration pointing at nothing."""
    with pytest.raises(ValueError, match="not present in the dataset"):
        purrdf.Store().query(
            SELECT_MEMBERS,
            relations_from_graph={MEMBER_OF: (_node("noSuchTable"), 1, 1)},
        )


def test_one_iri_declared_twice_is_refused() -> None:
    """A shadowed relation is a wrong-answer channel no query text can reveal."""
    store = _store_with(MEMBER_TABLE_TTL)

    with pytest.raises(ValueError, match="declared twice"):
        store.query(
            SELECT_MEMBERS,
            relations=_member_relations(),
            relations_from_graph={MEMBER_OF: (_node("memberTable"), 1, 1)},
        )


@pytest.mark.parametrize(
    "declaration",
    [
        (1, 1),
        (1, 1, [[_node("ada"), _node("alpha")]], "extra"),
        "not-a-tuple",
    ],
)
def test_a_malformed_declaration_is_a_type_error(declaration: object) -> None:
    """The expected shape is named in the error, not left to an anonymous failure."""
    with pytest.raises(TypeError, match="must be declared as"):
        purrdf.Store().query(SELECT_MEMBERS, relations={MEMBER_OF: declaration})


def test_a_non_term_cell_is_a_type_error() -> None:
    """A row cell goes through the same extractor a substitution value does."""
    with pytest.raises(TypeError, match="expected an RDF term"):
        purrdf.Store().query(
            SELECT_MEMBERS,
            relations={MEMBER_OF: (1, 1, [["ada", "alpha"]])},
        )


def test_a_negative_arity_is_a_type_error() -> None:
    """An arity is a count, so it is read as a non-negative integer."""
    with pytest.raises(TypeError, match="non-negative integer"):
        purrdf.Store().query(
            SELECT_MEMBERS,
            relations={MEMBER_OF: (-1, 1, [])},
        )


def test_a_non_string_relation_key_is_a_type_error() -> None:
    """A relation is keyed by the IRI a query spells in predicate position."""
    with pytest.raises(TypeError, match="must be IRI strings"):
        purrdf.Store().query(
            SELECT_MEMBERS,
            relations={_node("memberOf"): (1, 1, [])},
        )


def test_the_relation_keywords_are_keyword_only() -> None:
    """Every engine-configuration argument is named at the call site."""
    with pytest.raises(TypeError):
        purrdf.Store().query(SELECT_MEMBERS, _member_relations())  # type: ignore[misc]


# ── the same surface on MutableDataset ───────────────────────────────────────────


def test_mutable_dataset_carries_the_same_relation_surface() -> None:
    """`MutableDataset` is the compat shim's store; it must not be the weaker one."""
    dataset = purrdf.MutableDataset()

    solutions = dataset.query(SELECT_MEMBERS, relations=_member_relations())

    assert _pairs(solutions) == _pairs(
        purrdf.Store().query(SELECT_MEMBERS, relations=_member_relations())
    )


def test_mutable_dataset_update_over_a_relation_inserts_its_rows() -> None:
    """The UPDATE lane matches too, including the governed sibling's registry."""
    dataset = purrdf.MutableDataset()

    dataset.update(
        f"INSERT {{ ?person <{EX}team> ?team }} WHERE {{ ?person <{MEMBER_OF}> ?team }}",
        relations=_member_relations(),
    )

    assert len(dataset.query(f"SELECT ?p WHERE {{ ?p <{EX}team> ?t }}")) == 3  # type: ignore[arg-type]


def test_mutable_dataset_refuses_a_declared_unregistered_call() -> None:
    """The namespace declaration reaches the same parser options on both surfaces."""
    with pytest.raises(ValueError, match="no property function is registered"):
        purrdf.MutableDataset().query(
            f"SELECT ?team WHERE {{ <{EX}ada> <{REL}notRegistered> ?team }}",
            property_fn_namespaces=[REL],
        )


# ── path-witness relations ───────────────────────────────────────────────────────
#
# The third relation kind, and the one that is not a table: it declares a TRAVERSAL and
# the relation binds the walks it finds — every hop, in order, with the traversed
# statement as a first-class RDF 1.2 term.

WALK = f"{REL}walk"

#: `ex:a -> ex:b -> ex:c -> ex:d`, one predicate.
CHAIN_TTL = f"""
@prefix ex: <{EX}> .

ex:a ex:p ex:b .
ex:b ex:p ex:c .
ex:c ex:p ex:d .
"""

#: `ex:a -> {{ex:b, ex:c}} -> ex:d`: two distinct two-hop derivations reach `ex:d`,
#: which is exactly where "walk" and "shortest" must disagree.
DIAMOND_TTL = f"""
@prefix ex: <{EX}> .

ex:a ex:p ex:b .
ex:a ex:p ex:c .
ex:b ex:p ex:d .
ex:c ex:p ex:d .
"""

#: A call seeded at `ex:a`, projecting the four columns whose values are exactly
#: pinnable (`?pathId` is a content digest, asserted structurally instead).
WALK_QUERY = (
    f"SELECT ?end ?len ?step ?node WHERE {{ <{EX}a> <{WALK}> "
    "( ?end ?pathId ?len ?step ?node ?edge ) } ORDER BY ?len ?step"
)


def _walk_relation(mode: str = "walk") -> dict[str, Any]:
    """One `ex:p`-forward step, with every envelope field stated explicitly."""
    return {WALK: ([(_node("p"), "forward")], 1, 4, 1024, 100_000, mode)}


def _walk_rows(solutions: Any) -> list[tuple[str, int, int, str]]:
    """`(end, len, step, node)` per row, with the two counts as Python ints."""
    return [
        (
            str(row[0].value).removeprefix(EX),
            int(row[1].value),
            int(row[2].value),
            str(row[3].value).removeprefix(EX),
        )
        for row in solutions
    ]


def test_a_path_relation_binds_every_hop_of_a_multi_hop_chain() -> None:
    """One row per hop of every simple-prefix walk out of `ex:a`, in (len, step) order.

    Three walks leave `ex:a` (`->b`, `->b->c`, `->b->c->d`), so `1 + 2 + 3 = 6` rows.
    `?step` and `?len` are `xsd:integer` literals — read back as Python ints here —
    precisely so `ORDER BY ?step` orders numerically rather than by codepoint.
    """
    store = _store_with(CHAIN_TTL)

    solutions = store.query(WALK_QUERY, path_relations=_walk_relation())

    assert _walk_rows(solutions) == [
        ("b", 1, 1, "b"),
        ("c", 2, 1, "b"),
        ("c", 2, 2, "c"),
        ("d", 3, 1, "b"),
        ("d", 3, 2, "c"),
        ("d", 3, 3, "d"),
    ]


def test_an_inverse_step_traverses_the_same_statements_backwards() -> None:
    """`"inverse"` is the other half of the accepted direction set, executed.

    `test_a_malformed_path_relation_is_refused_by_name` proves `"sideways"` is refused,
    and every other test here spells `"forward"` — so until this test existed, writing
    ``"inverse" => PathDirection::Forward`` in the binding's match arm passed the entire
    Python suite. A refusal is only evidence about the set it excludes if the set it
    ADMITS is executed too, and half of that set was not.

    The same chain, read the other way: seeded at `ex:d`, an inverse `ex:p` step walks
    `d -> c -> b -> a`. The rows are the mirror of the forward test's, which is what
    makes this a direction assertion rather than merely a non-empty one — a step that
    quietly ran forwards from `ex:d` would answer nothing at all.
    """
    store = _store_with(CHAIN_TTL)

    solutions = store.query(
        f"SELECT ?end ?len ?step ?node WHERE {{ <{EX}d> <{WALK}> "
        "( ?end ?pathId ?len ?step ?node ?edge ) } ORDER BY ?len ?step",
        path_relations={WALK: ([(_node("p"), "inverse")], 1, 4, 1024, 100_000, "walk")},
    )

    assert _walk_rows(solutions) == [
        ("c", 1, 1, "c"),
        ("b", 2, 1, "c"),
        ("b", 2, 2, "b"),
        ("a", 3, 1, "c"),
        ("a", 3, 2, "b"),
        ("a", 3, 3, "a"),
    ]


def test_a_forward_step_from_the_far_end_of_the_chain_answers_nothing() -> None:
    """The control for the test above: same seed, same data, direction flipped.

    `ex:d` has no outgoing `ex:p`, so the forward reading answers nothing. Without this
    row the inverse test could be satisfied by a step that ignored the direction and
    happened to enumerate the chain some other way; with it, the two spellings are shown
    to produce genuinely different answers from the same seed.
    """
    store = _store_with(CHAIN_TTL)

    solutions = store.query(
        f"SELECT ?end ?len ?step ?node WHERE {{ <{EX}d> <{WALK}> "
        "( ?end ?pathId ?len ?step ?node ?edge ) } ORDER BY ?len ?step",
        path_relations=_walk_relation(),
    )

    assert _walk_rows(solutions) == []


def test_the_step_and_len_columns_are_xsd_integer_literals() -> None:
    """Typed, never simple: a simple literal would sort `"10"` before `"2"`."""
    store = _store_with(CHAIN_TTL)

    solutions = store.query(WALK_QUERY, path_relations=_walk_relation())
    row = next(iter(solutions))  # type: ignore[call-overload]

    assert row[1].datatype.value == "http://www.w3.org/2001/XMLSchema#integer"
    assert row[2].datatype.value == "http://www.w3.org/2001/XMLSchema#integer"


def test_path_id_groups_the_hops_of_one_walk() -> None:
    """`?pathId` is constant across one walk's rows and distinct between walks.

    That is what makes `GROUP BY ?pathId` the reassembly operator: each group IS one
    walk, so concatenating `?node` in `?step` order recovers the whole route.
    """
    store = _store_with(CHAIN_TTL)

    solutions = store.query(
        f"SELECT ?len (GROUP_CONCAT(?node; separator=\"->\") AS ?route) WHERE {{ "
        f"<{EX}a> <{WALK}> ( ?end ?pathId ?len ?step ?node ?edge ) }} "
        "GROUP BY ?pathId ?len ORDER BY ?len",
        path_relations=_walk_relation(),
    )

    routes = [(int(row[0].value), str(row[1].value)) for row in solutions]  # type: ignore[union-attr]
    assert routes == [
        (1, f"{EX}b"),
        (2, f"{EX}b->{EX}c"),
        (3, f"{EX}b->{EX}c->{EX}d"),
    ]

    # And the identifiers themselves: three walks, three distinct grouping keys, each
    # shared by exactly that walk's hop rows.
    hops = store.query(
        f"SELECT ?pathId ?len WHERE {{ <{EX}a> <{WALK}> "
        "( ?end ?pathId ?len ?step ?node ?edge ) }",
        path_relations=_walk_relation(),
    )
    by_id: dict[str, set[int]] = {}
    for row in hops:  # type: ignore[union-attr]
        by_id.setdefault(str(row[0].value), set()).add(int(row[1].value))
    assert len(by_id) == 3, "one identifier per walk"
    assert all(len(lengths) == 1 for lengths in by_id.values()), (
        "each identifier's rows belong to a single walk length"
    )


def test_two_equal_length_walks_do_not_share_an_identifier() -> None:
    """Distinctness, on the fixture where length cannot stand in for identity.

    `test_path_id_groups_the_hops_of_one_walk` runs on the chain, which has exactly one
    walk per length — so "every identifier's rows agree on `?len`" is satisfied there by
    an implementation that keyed `?pathId` on the LENGTH alone, and the chain can never
    tell the two apart. That is a property of the fixture, not evidence about `?pathId`.

    The diamond is the discriminating case: `a->b->d` and `a->c->d` are two distinct
    walks of the same length to the same endpoint. Grouping must keep them apart, so
    there are four identifiers (`->b`, `->c`, `->b->d`, `->c->d`) and the two two-hop
    routes are both recovered. A length-keyed identifier would fuse the last two into one
    group and lose a derivation silently — which is the whole failure this column exists
    to prevent.
    """
    store = _store_with(DIAMOND_TTL)

    solutions = store.query(
        f"SELECT (GROUP_CONCAT(?node; separator=\"->\") AS ?route) WHERE {{ "
        f"<{EX}a> <{WALK}> ( ?end ?pathId ?len ?step ?node ?edge ) }} "
        "GROUP BY ?pathId ORDER BY ?route",
        path_relations=_walk_relation(),
    )

    routes = sorted(str(row[0].value).replace(EX, "") for row in solutions)  # type: ignore[union-attr]
    assert routes == ["b", "b->d", "c", "c->d"], (
        "four walks, four groups: the two equal-length routes to ex:d stay apart"
    )


def test_shortest_mode_yields_one_witness_per_reachable_pair_on_a_diamond() -> None:
    """`"shortest"` keeps one two-hop derivation of `ex:d`; `"walk"` keeps both.

    Two relation TYPES rather than one with a runtime flag, because the planner reads
    cardinality off the registration: "exponential" and "polynomial" cannot be a
    property of a value it cannot see.
    """
    store = _store_with(DIAMOND_TTL)
    query = (
        f"SELECT ?end ?len ?step ?node WHERE {{ <{EX}a> <{WALK}> "
        "( ?end ?pathId ?len ?step ?node ?edge ) } ORDER BY ?end ?len ?step"
    )

    shortest = store.query(query, path_relations=_walk_relation("shortest"))
    assert _walk_rows(shortest) == [
        ("b", 1, 1, "b"),
        ("c", 1, 1, "c"),
        ("d", 2, 1, "b"),
        ("d", 2, 2, "d"),
    ]

    walk = store.query(query, path_relations=_walk_relation("walk"))
    assert _walk_rows(walk) == [
        ("b", 1, 1, "b"),
        ("c", 1, 1, "c"),
        ("d", 2, 1, "b"),
        ("d", 2, 1, "c"),
        ("d", 2, 2, "d"),
        ("d", 2, 2, "d"),
    ]


def test_the_edge_column_is_an_rdf_12_statement_term_that_joins_back() -> None:
    """`?edge` is a triple TERM, and the dataset is where it came from.

    RDF 1.2 first-classness is a project invariant, so it is pinned from Python and not
    only from Rust: the binding is a `purrdf.Triple`, its parts are the ASSERTED
    subject/predicate/object, and re-binding those three in an ordinary basic graph
    pattern finds the very statement the hop traversed.
    """
    store = _store_with(CHAIN_TTL)

    solutions = store.query(
        f"SELECT ?edge WHERE {{ <{EX}a> <{WALK}> "
        "( ?end ?pathId ?len ?step ?node ?edge ) FILTER(?len = 1) }",
        path_relations=_walk_relation(),
    )
    edges = [row[0] for row in solutions]  # type: ignore[union-attr]

    assert len(edges) == 1
    edge = edges[0]
    assert isinstance(edge, purrdf.Triple), "a hop is a statement term, not a label"
    assert edge.subject == _node("a")
    assert edge.predicate == _node("p")
    assert edge.object == _node("b")

    # The join: the statement the relation handed back is a statement of the dataset.
    joined = store.query(
        f"SELECT ?o WHERE {{ <{EX}a> <{WALK}> ( ?end ?pathId ?len ?step ?node ?edge ) "
        "FILTER(?len = 1) . ?s ?p ?o . FILTER(?edge = TRIPLE(?s, ?p, ?o)) }",
        path_relations=_walk_relation(),
    )
    assert _column(joined) == ["b"], "the hop's statement joins back into the data"


def test_a_path_relation_is_registered_for_one_call_only() -> None:
    """Registration is per call, exactly as for the two table spellings."""
    store = _store_with(CHAIN_TTL)

    assert len(store.query(WALK_QUERY, path_relations=_walk_relation())) == 6  # type: ignore[arg-type]
    assert len(store.query(WALK_QUERY)) == 0  # type: ignore[arg-type]


def test_a_path_relation_reaches_the_governed_and_update_lanes() -> None:
    """Query and update are symmetric: the keyword exists on both, and works on both."""
    store = _store_with(CHAIN_TTL)

    outcome = store.query_governed(WALK_QUERY, path_relations=_walk_relation())
    assert outcome.is_complete
    assert outcome.evidence.consumed["answer-rows"] == 6

    store.update(
        f"INSERT {{ <{EX}a> <{EX}reaches> ?end }} WHERE {{ <{EX}a> <{WALK}> "
        "( ?end ?pathId ?len ?step ?node ?edge ) }",
        path_relations=_walk_relation("shortest"),
    )
    reached = store.query(
        f"SELECT ?end WHERE {{ <{EX}a> <{EX}reaches> ?end }} ORDER BY ?end"
    )
    assert _column(reached) == ["b", "c", "d"]


def test_mutable_dataset_carries_the_path_relation_surface_too() -> None:
    """The compat shim's store must not be the weaker one here either."""
    dataset = purrdf.MutableDataset()
    dataset.load(CHAIN_TTL, purrdf.RdfFormat.TURTLE)

    solutions = dataset.query(WALK_QUERY, path_relations=_walk_relation())

    assert _walk_rows(solutions) == _walk_rows(
        _store_with(CHAIN_TTL).query(WALK_QUERY, path_relations=_walk_relation())
    )


# ── path-relation misconfiguration is refused where it is supplied ───────────────


@pytest.mark.parametrize(
    ("declaration", "message"),
    [
        # An unknown direction string: never coerced to one of the two.
        (
            ([(_node("p"), "sideways")], 1, 4, 8, 64, "walk"),
            "unknown step direction",
        ),
        # An unknown mode string, likewise.
        (
            ([(_node("p"), "forward")], 1, 4, 8, 64, "cheapest"),
            "unknown mode",
        ),
        # An empty alternation: a step that traverses nothing defines no hop.
        (([], 1, 4, 8, 64, "walk"), "at least one"),
        # A repeated (predicate, direction) pair doubles every walk through the hop.
        (
            ([(_node("p"), "forward"), (_node("p"), "forward")], 1, 4, 8, 64, "walk"),
            "repeats",
        ),
        # A predicate must be an IRI; a literal names no statements at all.
        (
            ([(purrdf.Literal("p"), "forward")], 1, 4, 8, 64, "walk"),
            "must be an IRI",
        ),
        # A zero-hop path is the identity and has no witness.
        (([(_node("p"), "forward")], 0, 4, 8, 64, "walk"), "min_hops must be at least 1"),
        # An unbounded depth is a stack-overflow abort, so the cap is hard.
        (
            ([(_node("p"), "forward")], 1, 100_000, 8, 64, "walk"),
            "exceeds the hard cap",
        ),
        # An empty accepted-length interval is a caller who has not finished deciding.
        (([(_node("p"), "forward")], 3, 2, 8, 64, "walk"), "exceeds max_hops"),
        # A guard of zero can only ever produce an error.
        (
            ([(_node("p"), "forward")], 1, 4, 0, 64, "walk"),
            "max_paths_per_seed must be at least 1",
        ),
        (
            ([(_node("p"), "forward")], 1, 4, 8, 0, "walk"),
            "max_expansions_per_invocation must be at least 1",
        ),
    ],
)
def test_a_malformed_path_relation_is_refused_by_name(
    declaration: object, message: str
) -> None:
    """Each rule of the declaration is its own diagnostic, naming what is wrong."""
    store = _store_with(CHAIN_TTL)

    with pytest.raises(ValueError, match=message):
        store.query(WALK_QUERY, path_relations={WALK: declaration})


def test_a_step_the_data_has_no_edges_for_answers_nothing() -> None:
    """An edgeless alternative is valid configuration, not a misconfiguration.

    An alternation is an alternation: ``p|q`` does not fail when ``q`` matches nothing,
    and neither does a step. A host with a fixed step vocabulary querying many stores
    supplies valid configuration every time, and a store carrying none of those edges has
    a correct answer — the empty one.
    """
    store = _store_with(CHAIN_TTL)

    solutions = store.query(
        WALK_QUERY,
        path_relations={
            WALK: ([(_node("noSuchPredicate"), "forward")], 1, 4, 8, 64, "walk")
        },
    )

    assert len(solutions) == 0  # type: ignore[arg-type]


@pytest.mark.parametrize(
    "declaration",
    [
        ([(_node("p"), "forward")], 1, 4, 8, 64),
        ([(_node("p"), "forward")], 1, 4, 8, 64, "walk", "extra"),
        "not-a-tuple",
    ],
)
def test_a_malformed_path_declaration_is_a_type_error(declaration: object) -> None:
    """The expected six-position shape is named in the error."""
    with pytest.raises(TypeError, match="must be declared as"):
        _store_with(CHAIN_TTL).query(WALK_QUERY, path_relations={WALK: declaration})


def test_a_malformed_step_pair_is_a_type_error() -> None:
    """A step is a `(predicate, direction)` PAIR, and the error says so."""
    with pytest.raises(TypeError, match="forward"):
        _store_with(CHAIN_TTL).query(
            WALK_QUERY,
            path_relations={WALK: ([(_node("p"),)], 1, 4, 8, 64, "walk")},
        )


def test_a_non_integer_envelope_field_is_a_type_error() -> None:
    """An envelope field is a count, so it is read as a non-negative integer."""
    with pytest.raises(TypeError, match="`min_hops`"):
        _store_with(CHAIN_TTL).query(
            WALK_QUERY,
            path_relations={WALK: ([(_node("p"), "forward")], "one", 4, 8, 64, "walk")},
        )


def test_one_iri_declared_across_two_relation_kinds_is_refused() -> None:
    """A shadowed relation is refused across ALL THREE dicts, not only two.

    Both pairings that involve `path_relations` are executed, because "all three" is a
    completeness claim and one pairing is not evidence for it: the `relations` +
    `path_relations` collision and the `relations_from_graph` + `path_relations` one are
    separate insertions into the shared name table, and a guard that covered only the
    first would satisfy a test that only ran the first.
    """
    store = _store_with(CHAIN_TTL)

    with pytest.raises(ValueError, match="declared twice"):
        store.query(
            WALK_QUERY,
            relations={WALK: (1, 6, [])},
            path_relations=_walk_relation(),
        )

    with pytest.raises(ValueError, match="declared twice"):
        store.query(
            WALK_QUERY,
            relations_from_graph={WALK: (_node("memberTable"), 1, 1)},
            path_relations=_walk_relation(),
        )


def test_the_path_relation_keyword_is_keyword_only() -> None:
    """Every engine-configuration argument is named at the call site."""
    with pytest.raises(TypeError):
        _store_with(CHAIN_TTL).query(WALK_QUERY, _walk_relation())  # type: ignore[misc]
