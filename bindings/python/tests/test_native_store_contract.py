# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""The native ``Store``'s own contract: blank-node scoping and the snapshot seam.

Two invariants live here, neither of them visible from the compat shim and
neither of them a property of any single call:

**Blank-node scoping.** A blank node label is document-local. Two separate
``load`` calls that both write ``_:b0`` are talking about two different nodes, and
merging them would fabricate a join no document asked for. Within one ``load``
the opposite is true: the same label twice IS one node, because that is what
makes an intra-document join work. The store therefore qualifies every surfaced
label with the scope that produced it, and the scoping has to reach every blank
position — including the ones nested inside an RDF 1.2 quoted triple, which is
the place a recursive walk is most likely to stop one level early.

**The snapshot seam.** ``Shapes.validate_store`` borrows a frozen snapshot of a
quad container rather than serialising it to N-Triples, and it reaches it through
an internal capsule whose value is the address of a heap-boxed ``Arc``. Four
things about that must hold from Python, and nothing else can check them: a
report already produced is unaffected by a later mutation of the container it
came from (it holds a snapshot, not a live view); the dataset stays alive exactly
as long as it is reachable — the capsule's destructor owns the one box, so a
report outliving its producer is fine and neither end frees twice; BOTH
containers on this surface answer the protocol, and either one validated gives
the one answer the triples deserve; and a value that answers no such protocol is
refused by name, because the seam is private and an ``AttributeError`` about it
diagnoses nothing for the caller who never wrote that name.
"""

from __future__ import annotations

import gc

import pytest

import purrdf
from purrdf import RdfFormat

EX = "https://example.org/"
XSD = "http://www.w3.org/2001/XMLSchema#"
RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"

#: One row whose subject is a document-local blank label.
BLANK_ROW = f"_:b0 <{EX}p> <{EX}o> .\n"

#: Two rows sharing that label — an intra-document join, which is the case the
#: scoping must NOT break.
JOINED_ROWS = f"_:b0 <{EX}p> <{EX}o> .\n_:b0 <{EX}q> <{EX}o2> .\n"

#: A reifier binding whose subject AND whose quoted triple's subject and object
#: are all blank. Every one of the three has to carry the load's scope.
NESTED_BLANKS = f"_:r <{RDF_REIFIES}> <<( _:is <{EX}q> _:io )>> .\n"

_SHAPES = f"""@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <{EX}> .
@prefix xsd: <{XSD}> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .
"""


def _violating_person(name: str) -> str:
    """One ``ex:Person`` whose age is not an ``xsd:integer`` — one violation."""
    return (
        f"<{EX}{name}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
        f"<{EX}Person> .\n"
        f'<{EX}{name}> <{EX}age> "not-an-integer" .\n'
    )


# ── blank-node scoping ──────────────────────────────────────────────────────────


def test_one_blank_label_loaded_twice_is_two_nodes_and_loaded_once_is_one() -> None:
    """The document-local scope of a blank label, from both sides.

    Two ``load`` calls are two documents. RDF says their ``_:b0`` labels are
    unrelated, so the store must surface two distinct nodes; collapsing them
    would silently merge two descriptions into one subject — a wrong answer that
    looks like a deduplication. And the mirror failure is just as bad: if the
    scope were per-row rather than per-load, the same label twice inside ONE
    document would stop joining, and every Turtle document with a shared blank
    would start returning fewer answers than it should.
    """
    twice = purrdf.Store()
    twice.load(BLANK_ROW, RdfFormat.N_TRIPLES)
    twice.load(BLANK_ROW, RdfFormat.N_TRIPLES)
    quads = list(twice)
    assert len(quads) == 2, "two documents, two rows — neither absorbed the other"
    assert len({quad.subject.value for quad in quads}) == 2, (
        "the same document-local label from two loads is two distinct nodes"
    )

    once = purrdf.Store()
    once.load(JOINED_ROWS, RdfFormat.N_TRIPLES)
    joined = list(once)
    assert len(joined) == 2
    assert len({quad.subject.value for quad in joined}) == 1, (
        "the same label within one document is one node, which is what makes an "
        "intra-document join answer"
    )

    # …and that join really is answerable through the query engine, not only
    # through the surfaced labels.
    solutions = once.query(
        f"SELECT ?b WHERE {{ ?b <{EX}p> <{EX}o> . ?b <{EX}q> <{EX}o2> }}"
    )
    assert len(solutions) == 1, "one node satisfies both patterns"


def test_blank_node_scoping_recurses_into_a_quoted_triples_own_terms() -> None:
    """A quoted triple's blanks are in the same document, so they carry its scope.

    A recursive walk that surfaced the reifier's subject but left the inner
    triple's terms unqualified would leak raw document labels into the term
    table, where a second load of the same document would collide with them —
    exactly the merge the scope exists to prevent, arriving one level down where
    the shallower test cannot see it.
    """
    store = purrdf.Store()
    store.load(NESTED_BLANKS, RdfFormat.N_TRIPLES)
    (quad,) = list(store)

    assert isinstance(quad.object, purrdf.Triple), "the object stays a quoted triple"
    surfaced = (
        quad.subject.value,
        quad.object.subject.value,
        quad.object.object.value,
    )
    assert len(set(surfaced)) == 3, f"three distinct blank nodes: {surfaced}"
    for label in surfaced:
        assert label not in {"r", "is", "io"}, (
            f"{label!r} is the raw document label, unqualified by the load's scope"
        )

    # The load's scope reaches all three, so a SECOND load of the same document
    # collides with none of them.
    store.load(NESTED_BLANKS, RdfFormat.N_TRIPLES)
    both = list(store)
    assert len(both) == 2
    nested = {
        (q.subject.value, q.object.subject.value, q.object.object.value) for q in both
    }
    assert len(nested) == 2, f"two loads, two disjoint reifiers: {nested}"
    flat = [label for triple in nested for label in triple]
    assert len(set(flat)) == 6, (
        "every blank position — the reifier and both nested terms — is scoped, so "
        f"no label is shared across the two loads: {flat}"
    )


def test_an_explicit_xsd_string_literal_comes_back_without_a_synthetic_datatype() -> None:
    """A stored ``xsd:string`` is the plain literal it is, on the way out too.

    RDF 1.1 made ``xsd:string`` the datatype of a plain literal, so the canonical
    N-Triples for one is ``"hi"`` and never ``"hi"^^<…#string>``. The store keeps
    a datatype on every literal internally, so the collapse on egress is what
    stands between that representation choice and a document no other RDF tool
    writes.
    """
    store = purrdf.Store()
    subject = purrdf.NamedNode(EX + "s")
    predicate = purrdf.NamedNode(EX + "p")
    store.add(
        purrdf.Quad(
            subject,
            predicate,
            purrdf.Literal("hi", datatype=purrdf.NamedNode(XSD + "string")),
        )
    )
    dumped = store.dump(format=RdfFormat.N_TRIPLES).decode()
    assert '"hi"' in dumped
    assert XSD + "string" not in dumped, (
        f"the plain literal's datatype is implied, never written: {dumped!r}"
    )

    # A literal that entered plain leaves the same bytes, which is the point: the
    # two spellings are one term and one document.
    plain = purrdf.Store()
    plain.add(purrdf.Quad(subject, predicate, purrdf.Literal("hi")))
    assert plain.dump(format=RdfFormat.N_TRIPLES) == store.dump(
        format=RdfFormat.N_TRIPLES
    )

    # The neighbouring literal that DOES carry a datatype still writes it, so the
    # collapse above is about `xsd:string` and not about datatypes in general.
    typed = purrdf.Store()
    typed.add(
        purrdf.Quad(
            subject,
            predicate,
            purrdf.Literal("1", datatype=purrdf.NamedNode(XSD + "integer")),
        )
    )
    typed_dump = typed.dump(format=RdfFormat.N_TRIPLES).decode()
    assert f'"1"^^<{XSD}integer>' in typed_dump, typed_dump


# ── the validation snapshot seam ────────────────────────────────────────────────


def test_a_validation_report_holds_a_snapshot_and_not_a_live_store() -> None:
    """A report already produced does not move when its store does.

    ``validate_store`` freezes the store and validates the frozen dataset, so the
    report is a statement about the store as it was. If it borrowed a live view
    instead, a later ``add`` would retroactively change what an earlier report
    said — and a caller comparing two reports to see what a mutation did would be
    comparing one answer against itself.
    """
    shapes = purrdf.shapes.Shapes(_SHAPES)
    store = purrdf.Store()
    store.load(_violating_person("alice"), RdfFormat.N_TRIPLES)

    first = shapes.validate_store(store)
    assert first.conforms is False
    assert len(first.results) == 1, "one person, one violation"
    before = first.to_ntriples()

    store.load(_violating_person("bob"), RdfFormat.N_TRIPLES)
    second = shapes.validate_store(store)

    assert len(second.results) == 2, "a fresh snapshot sees the added violation"
    assert len(first.results) == 1, "the earlier report is unchanged by the mutation"
    assert first.to_ntriples() == before
    assert second.to_ntriples() != before


def test_a_validation_report_outlives_the_store_it_was_taken_from() -> None:
    """The snapshot is owned for exactly as long as it is reachable.

    The capsule that carries the snapshot across the boundary hands out the
    address of a heap-boxed ``Arc`` and gives its destructor the only copy of
    that box, so the dataset is freed once and only once. Repeating the borrow
    and then dropping the producing store while holding every report is the
    Python-visible shape of that: a use-after-free or a double free here is a
    crash, not a failed assertion, so the test is the run completing with the
    reports still readable.
    """
    shapes = purrdf.shapes.Shapes(_SHAPES)
    store = purrdf.Store()
    store.load(_violating_person("alice"), RdfFormat.N_TRIPLES)

    # Repeated borrows of the same store: each takes its own snapshot and each
    # destructor frees its own box.
    reports = [shapes.validate_store(store) for _ in range(8)]
    rendered = [report.to_ntriples() for report in reports]
    assert len(set(rendered)) == 1, "the same store validated eight times, one answer"

    del store
    gc.collect()

    # Every report is still readable after its producer is gone.
    for report, text in zip(reports, rendered, strict=True):
        assert report.conforms is False
        assert len(report.results) == 1
        assert report.to_ntriples() == text


def test_validate_store_reads_a_mutable_dataset_through_the_same_seam() -> None:
    """``MutableDataset`` is a data graph validation can see, not one it must copy.

    Both quad containers on this surface hold a frozen native dataset behind a
    copy-on-write overlay, so both can answer the snapshot protocol, and the
    protocol is the only thing ``validate_store`` asks of its argument. A
    ``MutableDataset`` that could not answer it would have to be serialised to
    N-Triples and parsed back to be validated at all — a full copy, and a round
    trip through a syntax, to reach a dataset that was already sitting there.

    The two halves held here are that the report is real (not an accepted
    argument producing an empty answer) and that it is the SAME report the
    equivalent ``Store`` produces: the container a caller happened to load into
    is not part of what SHACL says about the data.
    """
    shapes = purrdf.shapes.Shapes(_SHAPES)
    document = _violating_person("alice")

    dataset = purrdf.MutableDataset()
    dataset.load(document, RdfFormat.N_TRIPLES)
    from_dataset = shapes.validate_store(dataset)

    assert from_dataset.conforms is False
    assert len(from_dataset.results) == 1, "one person, one violation"
    assert from_dataset.results[0]["value"] == '"not-an-integer"'

    store = purrdf.Store()
    store.load(document, RdfFormat.N_TRIPLES)
    from_store = shapes.validate_store(store)
    assert from_dataset.to_ntriples() == from_store.to_ntriples(), (
        "the same triples validated through either container, one answer"
    )

    # And the snapshot rule holds on this side too: a report already produced is
    # a statement about the dataset as it was, so mutating it afterwards moves
    # the next report and not the one already in hand.
    before = from_dataset.to_ntriples()
    dataset.load(_violating_person("bob"), RdfFormat.N_TRIPLES)
    assert len(shapes.validate_store(dataset).results) == 2
    assert len(from_dataset.results) == 1
    assert from_dataset.to_ntriples() == before


def test_validate_store_refuses_a_value_that_is_no_data_graph_by_name() -> None:
    """A value that cannot hand over a dataset is refused by name, not by traceback.

    ``validate_store`` reaches its argument through a private protocol method, so
    an argument that does not implement it used to surface as a bare
    ``AttributeError`` about ``_store_capsule`` — a name the caller never wrote,
    from a call whose real problem is that it was handed no data graph. The
    refusal names the type that arrived and the three things that are accepted,
    including the one a caller holding TEXT wants (``validate_nt``), which is the
    most likely way to arrive here by mistake.

    The neighbours that must keep working are the two containers themselves, and
    they are held by the tests above and by this one's second half.
    """
    shapes = purrdf.shapes.Shapes(_SHAPES)

    text = _violating_person("alice")
    with pytest.raises(TypeError) as refused:
        shapes.validate_store(text)
    message = str(refused.value)
    assert f"a {type(text).__name__} exposes no" in message, (
        "the refusal names the type that arrived, read off the value"
    )
    # And read off THE value: a second refusal over a different type names that
    # type instead. A message with the type name spelled into it would satisfy
    # the assertion above and tell the next caller about the wrong argument.
    number = 42
    with pytest.raises(TypeError) as refused_number:
        shapes.validate_store(number)
    number_message = str(refused_number.value)
    assert f"a {type(number).__name__} exposes no" in number_message, (
        f"the arriving type is the value's, not a constant: {number_message}"
    )
    assert type(text).__name__ not in number_message, (
        "and nothing of the first argument survives into the second refusal"
    )
    assert "purrdf.Store" in message and "purrdf.MutableDataset" in message
    assert "validate_nt" in message, "and the exit for a caller holding text"

    # An object that answers the protocol with something that is not a capsule is
    # refused the same way, rather than reading an address out of whatever it is.
    class _Pretender:
        def _store_capsule(self) -> str:
            return "not a capsule"

    with pytest.raises(TypeError, match="not honoured"):
        shapes.validate_store(_Pretender())

    # The neighbour: both real containers still validate.
    store = purrdf.Store()
    store.load(_violating_person("carol"), RdfFormat.N_TRIPLES)
    assert shapes.validate_store(store).conforms is False
    dataset = purrdf.MutableDataset()
    dataset.load(_violating_person("carol"), RdfFormat.N_TRIPLES)
    assert shapes.validate_store(dataset).conforms is False
