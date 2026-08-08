# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Host-injected property functions on the native query/update surface.

A property function is a RELATION invoked from predicate position: unlike an
extension function it is a row source, so one call may emit zero, one, or many
rows. The Rust tier has carried the seam for a while; these tests hold the Python
surface to what makes it usable and safe from a host that writes Python:

* **A relation is DATA, not a callable.** ``relations`` carries tuples and
  ``relations_from_graph`` carries the head of an ``rdf:List`` of ``rdf:List``s
  written in the store's own default graph. Nothing the engine invokes can re-enter
  the interpreter, which is why the whole evaluation still runs with the GIL
  released.
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
