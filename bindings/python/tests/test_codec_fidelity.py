# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""What the ingress and egress codecs must NOT change about a document.

``purrdf.parse`` is the one Python entry point that hands back terms without
interning them into a store's term table, so it is the only place a caller can
see what the parse itself did. These tests hold it to the two promises a codec
owes a document:

* **A language tag crosses verbatim, in every syntax.** This project's own
  private-use tags (``@x-purrdf-*``) carry subtags past BCP-47's eight-character
  limit, so a codec running in strict mode rejects them. All three codecs run
  lenient, deliberately and identically, and production code depends on it: the
  compat ``Literal`` constructor recovers from its own strict refusal by
  round-tripping through this parse path precisely so those tags survive.
* **A lexical form crosses verbatim.** The parse is not a normalizer. RDF term
  identity is over the lexical form, so a codec that canonicalized
  ``+00:00`` to ``Z`` on the way in would silently return a different term than
  the document wrote — and the canonicalization that IS available
  (``purrdf.xsd_canonical_lexical``) is what proves the preserved form was not
  already canonical.

On egress, two cross-checks that no single-format test can make: every one of
the eight ``RdfFormat`` constants routes to its own codec — asserted by syntax
only that codec emits, which is what catches a constant wired to the wrong
format — and a base-free triple serialization is byte-identical to the
default-graph selection of a dataset dump, in all eight.
"""

from __future__ import annotations

import pytest

import purrdf
from purrdf import RdfFormat

EX = "https://example.org/"
XSD = "http://www.w3.org/2001/XMLSchema#"
RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"

#: A private-use language tag whose subtag is nine characters — one past what
#: strict BCP-47 admits, and exactly the shape this project uses.
PRIVATE_TAG = "x-purrdf-afrikaans"

#: The three line/graph syntaxes `parse_quads` routes, and a document in each
#: carrying the private-use tag. N-Quads gets a graph name so the three
#: documents are not one document read three ways.
PRIVATE_TAG_DOCUMENTS = (
    (RdfFormat.N_TRIPLES, f'<{EX}s> <{EX}p> "hallo"@{PRIVATE_TAG} .\n'),
    (RdfFormat.TURTLE, f'<{EX}s> <{EX}p> "hallo"@{PRIVATE_TAG} .\n'),
    (RdfFormat.N_QUADS, f'<{EX}s> <{EX}p> "hallo"@{PRIVATE_TAG} <{EX}g> .\n'),
)

#: A `dateTime` whose lexical form is well-formed but NOT canonical: the
#: canonical spelling of a zero UTC offset is `Z`.
NON_CANONICAL_DATETIME = "2026-06-19T00:00:00+00:00"

#: Each of the eight `RdfFormat` constants, with syntax the codec it names DOES
#: emit and syntax it does NOT, over the two-graph document below. The negative
#: half is what makes the set a partition rather than eight coincidences: Turtle
#: and TriG share `@prefix`, JSON-LD and YAML-LD share an `@context` member, and
#: N-Triples and N-Quads share every line of the default graph, so a positive
#: marker alone would leave three pairs interchangeable. A serialize/reparse
#: round trip under ONE constant cannot catch a constant wired to the wrong
#: native format at all — that round trip is self-consistent by construction.
#: (`RdfFormat` values are not hashable, so this is a sequence, not a mapping.)
FORMAT_MARKERS = (
    # Turtle carries prefixes and no graph syntax, so the named graph is dropped.
    (RdfFormat.TURTLE, "@prefix", (f"<{EX}g>", "{")),
    # N-Triples carries no prefixes and no graph name.
    (RdfFormat.N_TRIPLES, f'<{EX}s> <{EX}p> "hi" .', ("@prefix", f"<{EX}g>")),
    # N-Quads names the graph in a fourth column, still with no prefixes.
    (RdfFormat.N_QUADS, f'"hi2" <{EX}g> .', ("@prefix", "{")),
    # TriG names the graph and opens a block for it.
    (RdfFormat.TRIG, f"<{EX}g> {{", ()),
    (RdfFormat.TRIX, "<TriX", ()),
    (RdfFormat.HEXTUPLES, f'["{EX}s2","{EX}p","hi2"', ()),
    # JSON-LD quotes its keys; YAML-LD does not.
    (RdfFormat.JSON_LD, '"@context": {}', ("# yaml-language-server:",)),
    (RdfFormat.YAML_LD, "# yaml-language-server:", ('"@context"',)),
)

#: All eight formats the `RdfFormat` constants name.
ALL_FORMATS = tuple(data_format for data_format, _, _ in FORMAT_MARKERS)


def _literal_store() -> purrdf.Store:
    """One default-graph triple whose object is a plain literal."""
    store = purrdf.Store()
    store.load(f'<{EX}s> <{EX}p> "hi" .\n', RdfFormat.N_TRIPLES)
    return store


def _two_graph_store() -> purrdf.Store:
    """One default-graph triple and one named-graph quad.

    The named graph is what makes the eight formats tell each other apart: a
    document with no graph name renders identically in Turtle and TriG, and in
    N-Triples and N-Quads, so a single-graph fixture cannot distinguish either
    pair.
    """
    store = purrdf.Store()
    store.load(f'<{EX}s> <{EX}p> "hi" .\n', RdfFormat.N_TRIPLES)
    store.load(f'<{EX}s2> <{EX}p> "hi2" <{EX}g> .\n', RdfFormat.N_QUADS)
    return store


def _constructed(store: purrdf.Store) -> purrdf.QueryTriples:
    """Every triple in *store*, as a CONSTRUCT result — the `serialize_triples` lane."""
    return store.query("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }")


# ── ingress: what the parse must not change ─────────────────────────────────────


@pytest.mark.parametrize(("data_format", "document"), PRIVATE_TAG_DOCUMENTS)
def test_a_private_use_language_tag_survives_the_parse(
    data_format: RdfFormat, document: str
) -> None:
    """The nine-character private-use subtag crosses the codec verbatim.

    Strict BCP-47 caps a subtag at eight characters, so a strict codec refuses
    ``@x-purrdf-afrikaans`` outright. All three codecs run lenient, and they must
    run lenient identically: the compat ``Literal`` constructor recovers from its
    own strict refusal by re-parsing through here, so a format that tightened
    would take that recovery with it.
    """
    quads = purrdf.parse(document, data_format)
    assert len(quads) == 1
    literal = quads[0].object
    assert isinstance(literal, purrdf.Literal)
    assert literal.value == "hallo"
    assert literal.language == PRIVATE_TAG, (
        "the tag crosses verbatim, subtag length and all"
    )


def test_an_ordinary_language_tag_parses_in_every_format_too() -> None:
    """The neighbouring in-spec tag is not swept up by the lenient reading.

    Leniency is about admitting more, never about reading tags differently: an
    ordinary ``@en`` must still arrive as ``"en"``, so the case above is about the
    subtag length and nothing else.
    """
    for data_format, document in PRIVATE_TAG_DOCUMENTS:
        quads = purrdf.parse(document.replace(PRIVATE_TAG, "en"), data_format)
        assert quads[0].object.language == "en", data_format


def test_the_parse_preserves_a_non_canonical_lexical_form() -> None:
    """A parse is not a normalizer: the lexical form is the document's.

    RDF term identity is over the lexical form, so normalizing on ingress returns
    a term the document did not contain — and every downstream comparison,
    digest and canonical serialization inherits the substitution. The contrast is
    made with the canonicalizer this surface DOES expose, which is what proves the
    preserved form was not canonical to begin with.
    """
    document = (
        f'<{EX}s> <{EX}p> "{NON_CANONICAL_DATETIME}"^^<{XSD}dateTime> .\n'
    )
    canonical = purrdf.xsd_canonical_lexical(NON_CANONICAL_DATETIME, XSD + "dateTime")
    assert canonical != NON_CANONICAL_DATETIME, (
        "this fixture is only a witness while the source form is non-canonical"
    )

    for data_format in (RdfFormat.TURTLE, RdfFormat.N_TRIPLES):
        quads = purrdf.parse(document, data_format)
        assert quads[0].object.value == NON_CANONICAL_DATETIME, (
            f"{data_format}: the source lexical form survives the parse"
        )
        assert quads[0].object.datatype.value == XSD + "dateTime"


def test_the_turtle_codec_reads_a_quoted_triple_into_a_triple_term() -> None:
    """RDF 1.2 ``<<( s p o )>>`` in object position parses as a triple term.

    "It parsed" is not the claim — a codec that read the reifier row and dropped
    the quoted triple to a blank node would parse too, and every text-level
    assertion about the document would still pass. The claim is that the object
    IS a triple term, and that its three positions came back intact.
    """
    document = (
        f"<{EX}r> <{RDF_REIFIES}> "
        f"<<( <{EX}s> <{EX}p> <{EX}o> )>> .\n"
    )
    for data_format in (RdfFormat.TURTLE, RdfFormat.N_TRIPLES):
        quads = purrdf.parse(document, data_format)
        assert len(quads) == 1, data_format
        inner = quads[0].object
        assert isinstance(inner, purrdf.Triple), f"{data_format}: {inner!r}"
        assert inner == purrdf.Triple(
            purrdf.NamedNode(EX + "s"),
            purrdf.NamedNode(EX + "p"),
            purrdf.NamedNode(EX + "o"),
        )
        assert isinstance(quads[0].subject, purrdf.NamedNode)


# ── egress: the format registry and the default-graph selection ─────────────────


def test_a_constructed_literal_is_emitted_quoted() -> None:
    """The triple serializer writes a literal object as a literal.

    The CONSTRUCT-result lane is its own serializer entry point, and every other
    test of it serializes all-IRI triples — where a literal that came out as an
    IRI, or not at all, would be invisible.
    """
    payload = _constructed(_literal_store()).serialize(RdfFormat.N_TRIPLES)
    assert b'"hi"' in payload, payload
    assert payload.decode().strip() == f'<{EX}s> <{EX}p> "hi" .'


@pytest.mark.parametrize(("data_format", "present", "absent"), FORMAT_MARKERS)
def test_each_format_constant_emits_its_own_syntax(
    data_format: RdfFormat, present: str, absent: tuple[str, ...]
) -> None:
    """Eight constants, eight codecs, each pinned to the one it names.

    A serialize-then-reparse round trip under ONE constant is self-consistent
    whatever the constant maps to, so it cannot catch a constant wired to the
    wrong native format. Syntax the intended codec writes, paired with syntax it
    does not, can — and the pairing is what separates the three format pairs that
    share their positive marker.
    """
    payload = _two_graph_store().dump(format=data_format).decode()
    assert present in payload, (
        f"{data_format} did not emit its own syntax: {payload!r}"
    )
    for forbidden in absent:
        assert forbidden not in payload, (
            f"{data_format} emitted {forbidden!r}, which belongs to another "
            f"codec: {payload!r}"
        )

    # …and the bytes still reparse, so the markers are not the only agreement.
    assert purrdf.parse(payload, data_format)


@pytest.mark.parametrize("data_format", ALL_FORMATS)
def test_a_base_free_triple_serialization_is_the_default_graph_selection(
    data_format: RdfFormat,
) -> None:
    """The two egress paths agree byte for byte where they must.

    The triple serializer moved from a hard-wired default-graph selection to the
    dataset selection a dump applies, so that it could carry a document base at
    all. On this input the two cannot legitimately differ — every triple is a
    default-graph quad and the RDF 1.2 statement layer is empty — so any
    divergence is the move having changed something it was not supposed to.
    """
    store = _literal_store()
    triples = _constructed(store).serialize(data_format)
    dataset = store.dump(format=data_format, from_graph=purrdf.DefaultGraph())
    assert triples == dataset, f"{data_format}: the two selections disagree"
