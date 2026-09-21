# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Proving the Python incremental-SHACL lane, not just exercising it.

``purrdf-shapes`` has carried an incremental validation surface — expand a graph
change into the focus nodes whose verdict it can move, then re-validate only those
— with no Python door onto it at all. ``PreparedShapes.validate_store_changes`` is
that door, and these tests pin what makes it worth having rather than that it
returns something:

* the incremental report is IDENTICAL, results and ordering alike, to what a full
  validation of the mutated graph reports — a cheaper answer that is a different
  answer is not the feature;
* ``Store.checkpoint()`` is load-bearing, not decoration: without it a store's
  whole contents are its pending change, and the test proves that rather than
  asserting it in a docstring;
* a shapes graph whose reads hide inside SPARQL query text falls back to a FULL
  validation and SAYS SO, because an under-approximated change set and a clean
  bill of health are the same report;
* a REMOVAL moves a verdict exactly as an addition does.

Fixtures use ``example.org``.
"""

from __future__ import annotations

import purrdf

# `ex:age` must be an integer and must be present. Core constraints only — which is
# what makes this shapes graph's change footprint boundable.
_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ; sh:minCount 1 ] .
"""

# The same obligation expressed through SHACL-SPARQL. The engine cannot bound a
# footprint it can only read as query text, so this is the fallback fixture.
_SPARQL_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:sparql [
        a sh:SPARQLConstraint ;
        sh:message "every person needs an integer age" ;
        sh:select \"\"\"SELECT $this WHERE {
            $this a <http://example.org/Person> .
            FILTER NOT EXISTS {
                $this <http://example.org/age> ?a .
                FILTER(datatype(?a) = <http://www.w3.org/2001/XMLSchema#integer>)
            }
        }\"\"\"
    ] .
"""

# A CONFORMING base. Anything the incremental lane reports below therefore came out
# of the change rather than out of the graph the change was applied to — which is
# what lets the incremental report be compared to a full one for equality.
_BASE_NT = (
    "<http://example.org/bob> "
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/Person> .\n"
    '<http://example.org/bob> <http://example.org/age> "42"'
    "^^<http://www.w3.org/2001/XMLSchema#integer> .\n"
    "<http://example.org/carol> "
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/Person> .\n"
    '<http://example.org/carol> <http://example.org/age> "31"'
    "^^<http://www.w3.org/2001/XMLSchema#integer> .\n"
)

_RDF_TYPE = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type"
_XSD_INTEGER = "http://www.w3.org/2001/XMLSchema#integer"


def _iri(value: str) -> purrdf.NamedNode:
    return purrdf.NamedNode(value)


def _typed(value: str, datatype: str) -> purrdf.Literal:
    return purrdf.Literal(value, datatype=_iri(datatype))


def _checkpointed_store(data_nt: str = _BASE_NT) -> purrdf.Store:
    """A store holding ``data_nt`` with an EMPTY pending change.

    The checkpoint is the whole point: a freshly constructed ``Store`` branches off
    an empty base, so without it everything ever loaded is part of the delta.
    """
    store = purrdf.Store()
    store.load(data_nt, purrdf.RdfFormat.N_TRIPLES)
    store.checkpoint()
    return store


def _mutated_nt(store: purrdf.Store) -> str:
    """The store's CURRENT contents as N-Triples — the oracle's input."""
    return store.dump(format=purrdf.RdfFormat.N_TRIPLES).decode()


def _bad_age_quad() -> purrdf.Quad:
    """An age that is a number and is not an ``xsd:integer``.

    Deliberately a TYPED literal rather than a plain one: these tests compare an
    incremental report (built over the store's IR) against a full one (built over a
    re-parse of the store's N-Triples dump) byte for byte, and a simple literal is
    the one term whose datatype an RDF 1.1 serializer may legitimately elide. Using
    ``xsd:decimal`` keeps the comparison about the validation lane instead of about
    the serializer.
    """
    return purrdf.Quad(
        _iri("http://example.org/alice"),
        _iri("http://example.org/age"),
        _typed("42.5", "http://www.w3.org/2001/XMLSchema#decimal"),
    )


def _person_quad(subject: str) -> purrdf.Quad:
    return purrdf.Quad(_iri(subject), _iri(_RDF_TYPE), _iri("http://example.org/Person"))


# ── 1. The claim the lane rests on ──────────────────────────────────────────────


def test_the_incremental_report_is_the_full_report_for_the_affected_nodes() -> None:
    """A cheaper answer that is a different answer is not the feature."""
    shapes = purrdf.shapes.Shapes(_SHAPES)
    store = _checkpointed_store()
    store.add(_person_quad("http://example.org/alice"))
    store.add(_bad_age_quad())

    outcome = shapes.prepare().validate_store_changes(store)
    full = shapes.validate_nt(_mutated_nt(store))

    assert outcome.bounded is True, outcome.reason
    assert outcome.reason is None
    assert outcome.focus_nodes == 1, "one new focus node moved"
    assert outcome.report.conforms is False
    # Byte-identical, not merely equivalent: a SHACL report is a deterministic
    # artifact, blank labels and result ORDER included.
    assert outcome.report.to_ntriples() == full.to_ntriples()


def test_a_conforming_change_reports_nothing_and_still_validates_something() -> None:
    """The neighbouring valid case.

    A lane that reported a violation for every change would pass the test above
    while being useless, so a change that breaks nothing must come back conforming
    — and must still have expanded to a non-empty focus set, or it conformed by
    never looking.
    """
    shapes = purrdf.shapes.Shapes(_SHAPES)
    store = _checkpointed_store()
    store.add(_person_quad("http://example.org/dave"))
    store.add(
        purrdf.Quad(
            _iri("http://example.org/dave"),
            _iri("http://example.org/age"),
            _typed("7", _XSD_INTEGER),
        )
    )

    outcome = shapes.prepare().validate_store_changes(store)
    assert outcome.bounded is True
    assert outcome.focus_nodes == 1, "the new person WAS examined"
    assert outcome.report.conforms is True
    assert outcome.report.results == []
    assert outcome.report.to_ntriples() == shapes.validate_nt(_mutated_nt(store)).to_ntriples()


# ── 2. The checkpoint is load-bearing ───────────────────────────────────────────


def test_checkpoint_is_what_makes_the_change_a_change() -> None:
    """Without a checkpoint the pending change is the whole store, and with one it
    is exactly the mutation. Both halves are executed, because the claim is a
    difference between them rather than a property of either."""
    uncheckpointed = purrdf.Store()
    uncheckpointed.load(_BASE_NT, purrdf.RdfFormat.N_TRIPLES)
    assert uncheckpointed.change_size() == (len(uncheckpointed), 0), (
        "a fresh Store branches off an EMPTY base, so every loaded row is in the delta"
    )

    uncheckpointed.checkpoint()
    assert uncheckpointed.change_size() == (0, 0), "the checkpoint folds the delta into the base"
    assert len(uncheckpointed) == 4, "and changes nothing about the store's contents"

    uncheckpointed.add(_person_quad("http://example.org/alice"))
    assert uncheckpointed.change_size() == (1, 0)


def test_change_size_counts_both_halves() -> None:
    store = _checkpointed_store()
    store.add(_person_quad("http://example.org/alice"))
    store.remove(
        purrdf.Quad(
            _iri("http://example.org/bob"),
            _iri("http://example.org/age"),
            _typed("42", _XSD_INTEGER),
        )
    )
    assert store.change_size() == (1, 1)


# ── 3. A removal moves a verdict too ────────────────────────────────────────────


def test_removing_a_row_moves_a_verdict_and_matches_a_full_validation() -> None:
    """`sh:minCount 1` turns a retraction into a violation, which is the direction a
    change set that only understood additions would miss entirely."""
    shapes = purrdf.shapes.Shapes(_SHAPES)
    store = _checkpointed_store()
    store.remove(
        purrdf.Quad(
            _iri("http://example.org/bob"),
            _iri("http://example.org/age"),
            _typed("42", _XSD_INTEGER),
        )
    )

    outcome = shapes.prepare().validate_store_changes(store)
    assert outcome.bounded is True
    assert outcome.report.conforms is False
    assert outcome.report.to_ntriples() == shapes.validate_nt(_mutated_nt(store)).to_ntriples()
    assert any(
        "http://example.org/bob" in result["focus"] for result in outcome.report.results
    ), outcome.report.results


# ── 4. The fallback is not optional ─────────────────────────────────────────────


def test_an_unbounded_footprint_falls_back_to_a_full_validation_and_names_why() -> None:
    """A shapes graph whose reads are query text has no bounded footprint. The lane
    validates the whole mutated graph instead of under-reporting, and says which
    construct cost it the bound."""
    shapes = purrdf.shapes.Shapes(_SPARQL_SHAPES)
    store = _checkpointed_store()
    store.add(_person_quad("http://example.org/alice"))

    outcome = shapes.prepare().validate_store_changes(store)
    assert outcome.bounded is False
    assert outcome.focus_nodes is None, (
        "`None` rather than a count: 'every focus node' is not a number"
    )
    assert isinstance(outcome.reason, str) and outcome.reason, outcome.reason
    assert outcome.report.conforms is False
    assert outcome.report.to_ntriples() == shapes.validate_nt(_mutated_nt(store)).to_ntriples()


def test_the_two_shapes_graphs_agree_so_the_fallback_is_not_a_different_answer() -> None:
    """The bounded and the unbounded routes are two ways of reaching one verdict.

    The two shapes graphs above express the same obligation, so over the same
    mutation they must find the same focus node violating it — otherwise the
    fallback would be a second opinion rather than a slower road to the first.
    """
    bounded_store = _checkpointed_store()
    bounded_store.add(_person_quad("http://example.org/alice"))
    sparql_store = _checkpointed_store()
    sparql_store.add(_person_quad("http://example.org/alice"))

    bounded = purrdf.shapes.Shapes(_SHAPES).prepare().validate_store_changes(bounded_store)
    unbounded = (
        purrdf.shapes.Shapes(_SPARQL_SHAPES).prepare().validate_store_changes(sparql_store)
    )

    assert bounded.bounded is True
    assert unbounded.bounded is False
    assert bounded.report.conforms is False
    assert unbounded.report.conforms is False
    assert [r["focus"] for r in bounded.report.results] == [
        r["focus"] for r in unbounded.report.results
    ]


# ── 5. The outcome object states its own scope ──────────────────────────────────


def test_repr_states_which_question_the_report_answered() -> None:
    """`ChangeValidation` exists because a report alone cannot say which question it
    answered, so the repr a caller reads in a debugger must say it."""
    store = _checkpointed_store()
    store.add(_person_quad("http://example.org/alice"))

    bounded = repr(purrdf.shapes.Shapes(_SHAPES).prepare().validate_store_changes(store))
    assert "bounded" in bounded and "focus_nodes=" in bounded, bounded

    sparql_store = _checkpointed_store()
    sparql_store.add(_person_quad("http://example.org/alice"))
    unbounded = repr(
        purrdf.shapes.Shapes(_SPARQL_SHAPES).prepare().validate_store_changes(sparql_store)
    )
    assert "everything" in unbounded and "reason=" in unbounded, unbounded


def test_an_empty_change_validates_nothing_and_conforms() -> None:
    """A checkpointed store with no pending mutation has an EMPTY expansion — not a
    full validation, and not a refusal. Executed beside the non-empty cases above
    because 'nothing changed' is the state a realtime caller is in most of the
    time, and a lane that quietly re-validated the graph there would cost exactly
    what it was built to save."""
    store = _checkpointed_store()
    outcome = purrdf.shapes.Shapes(_SHAPES).prepare().validate_store_changes(store)
    assert outcome.bounded is True
    assert outcome.focus_nodes == 0
    assert outcome.report.conforms is True


def test_a_restored_product_reaches_the_same_change_verdict_as_a_parse() -> None:
    """The incremental lane hangs off `PreparedShapes`, which is also what admitting
    a prepared PRODUCT hands back — so a product must reach the identical change
    verdict a fresh parse does, or the cache would silently change answers on the
    one route that was added after it."""
    store = _checkpointed_store()
    store.add(_person_quad("http://example.org/alice"))
    store.add(_bad_age_quad())

    parsed = purrdf.shapes.Shapes(_SHAPES).prepare().validate_store_changes(store)
    restored = (
        purrdf.shapes.ShapesProduct.open(purrdf.shapes.pack_product(_SHAPES))
        .admit()
        .validate_store_changes(store)
    )

    assert parsed.bounded == restored.bounded
    assert parsed.focus_nodes == restored.focus_nodes
    assert parsed.report.to_ntriples() == restored.report.to_ntriples()
