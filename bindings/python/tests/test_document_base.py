# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""The Python surface's DOCUMENT BASE contract.

The Rust, WebAssembly, and C surfaces all carry a document base on both their parse
and their serialize entry points. These tests pin that the Python one does too, and
that it behaves identically: the base resolves relative references on ingress, is
written and relativized against on egress by the syntaxes that can express one, and
is never fabricated when absent.
"""

from __future__ import annotations

import pytest

import purrdf
from purrdf import RdfFormat

#: A Turtle document whose subject is a relative reference and which declares no
#: `@base` of its own, so the caller's base is the only one that can be in scope.
RELATIVE_TURTLE = "<rel> <https://example.org/p> <https://example.org/o> .\n"

BASE = "https://example.org/base/"

#: The shared `purrdf-iri` diagnostic code for "a relative reference with no base in
#: scope". One identity for every surface — never a per-binding respelling.
NO_BASE_CODE = "iri-relative-no-base"


# ── parse: the ingress base ─────────────────────────────────────────────────────


def test_parse_resolves_a_relative_iri_against_the_supplied_base() -> None:
    quads = purrdf.parse(RELATIVE_TURTLE, RdfFormat.TURTLE, base=BASE)
    assert len(quads) == 1
    assert str(quads[0].subject) == f"<{BASE}rel>"


def test_parse_without_a_base_hard_fails_with_the_shared_code() -> None:
    with pytest.raises(ValueError) as excinfo:
        purrdf.parse(RELATIVE_TURTLE, RdfFormat.TURTLE)
    # No base is invented from a retrieval IRI or the filesystem; the relative
    # reference is refused, and the message carries the shared diagnostic code.
    assert NO_BASE_CODE in str(excinfo.value)


def test_parse_rejects_a_base_that_is_not_absolute() -> None:
    with pytest.raises(ValueError):
        purrdf.parse(RELATIVE_TURTLE, RdfFormat.TURTLE, base="not-absolute/")


def test_an_in_document_base_wins_over_the_supplied_one() -> None:
    doc = (
        "@base <https://example.org/inner/> .\n"
        "<rel> <https://example.org/p> <https://example.org/o> .\n"
    )
    quads = purrdf.parse(doc, RdfFormat.TURTLE, base=BASE)
    assert str(quads[0].subject) == "<https://example.org/inner/rel>"


def test_parse_without_a_base_still_accepts_absolute_documents() -> None:
    # The base is optional, not required: a document with only absolute IRIs needs
    # none and must keep parsing exactly as before.
    doc = "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n"
    assert len(purrdf.parse(doc, RdfFormat.N_TRIPLES)) == 1


# ── serialize: the egress base ──────────────────────────────────────────────────


def _constructed_triples(base: str | None = None) -> purrdf.QueryTriples:
    """A `QueryTriples` over one triple whose IRIs sit under `BASE`."""
    store = purrdf.Store()
    store.load(
        f"<{BASE}s> <{BASE}p> <{BASE}o> .\n",
        RdfFormat.N_TRIPLES,
        base=base,
    )
    return store.query("CONSTRUCT { ?s ?p ?o } WHERE { ?s ?p ?o }")


def test_serialize_emits_the_base_declaration_for_a_base_capable_format() -> None:
    text = purrdf.serialize(
        _constructed_triples(), format=RdfFormat.TURTLE, base=BASE
    ).decode()
    assert f"@base <{BASE}> ." in text


def test_serialize_relativizes_against_the_base() -> None:
    text = purrdf.serialize(
        _constructed_triples(), format=RdfFormat.TURTLE, base=BASE
    ).decode()
    # The subject is spelled relative to the declared base, not absolutely.
    assert f"<{BASE}s>" not in text
    assert "<s>" in text


def test_serialize_to_a_base_incapable_format_stays_absolute_without_erroring() -> None:
    # N-Triples cannot express a base. The parameter is still read and validated;
    # the answer is the only spelling that grammar admits.
    text = purrdf.serialize(
        _constructed_triples(), format=RdfFormat.N_TRIPLES, base=BASE
    ).decode()
    assert f"<{BASE}s>" in text
    assert "@base" not in text


def test_serialize_rejects_a_base_that_is_not_absolute() -> None:
    with pytest.raises(ValueError):
        purrdf.serialize(
            _constructed_triples(), format=RdfFormat.TURTLE, base="not-absolute/"
        )


def test_serialize_without_a_base_is_unchanged() -> None:
    text = purrdf.serialize(_constructed_triples(), format=RdfFormat.TURTLE).decode()
    assert "@base" not in text
    assert f"<{BASE}s>" in text


def test_query_triples_serialize_carries_the_same_base() -> None:
    # The method on `QueryTriples` and the module-level function share one core, so
    # they must agree byte-for-byte under the same base.
    triples = _constructed_triples()
    direct = triples.serialize(RdfFormat.TURTLE, base=BASE)
    assert direct == purrdf.serialize(
        _constructed_triples(), format=RdfFormat.TURTLE, base=BASE
    )
    assert f"@base <{BASE}> .".encode() in direct


# ── the base reaches the other Python parse surfaces ────────────────────────────


def test_rdf_dataset_constructor_takes_the_base() -> None:
    dataset = purrdf.RdfDataset(RELATIVE_TURTLE, RdfFormat.TURTLE, base=BASE)
    assert dataset.quad_count() == 1
    assert f"<{BASE}rel>" in dataset.to_nquads()

    with pytest.raises(ValueError) as excinfo:
        purrdf.RdfDataset(RELATIVE_TURTLE, RdfFormat.TURTLE)
    assert NO_BASE_CODE in str(excinfo.value)


def test_store_load_and_module_parse_agree_on_the_base() -> None:
    store = purrdf.Store()
    store.load(RELATIVE_TURTLE, RdfFormat.TURTLE, base=BASE)
    loaded = {str(q.subject) for q in store}
    assert loaded == {f"<{BASE}rel>"}


def test_to_json_ld_carries_the_base_on_both_legs() -> None:
    # Ingress: the relative subject resolves. Egress: JSON-LD can express a base, so
    # it reaches the emitted document.
    text = purrdf.to_json_ld(
        RELATIVE_TURTLE.encode(), format=RdfFormat.TURTLE, base=BASE
    )
    assert BASE in text


def test_from_json_ld_takes_a_base() -> None:
    # N-Quads is the output syntax and admits no base, so the emitted IRIs are
    # absolute — which is exactly why the ingress base matters here.
    jsonld = (
        '{"@id": "rel", "https://example.org/p": [{"@id": "https://example.org/o"}]}'
    )
    nquads = purrdf.from_json_ld(jsonld, base=BASE).decode()
    assert f"<{BASE}rel>" in nquads


def test_from_rdf_xml_takes_a_base() -> None:
    xml = (
        '<?xml version="1.0"?>'
        '<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"'
        ' xmlns:ex="https://example.org/">'
        '<rdf:Description rdf:about="rel">'
        '<ex:p rdf:resource="https://example.org/o"/>'
        "</rdf:Description></rdf:RDF>"
    )
    nquads = purrdf.from_rdf_xml(xml, base=BASE).decode()
    assert f"<{BASE}rel>" in nquads


def test_gts_producer_entry_points_take_a_base() -> None:
    # A relative-IRI source has no other way to say what it is relative to, so the
    # producer surface must accept one; without it the parse is refused.
    data = RELATIVE_TURTLE.encode()
    assert purrdf.gts_from_quads(data, format=RdfFormat.TURTLE, base=BASE)
    assert purrdf.snapshot_content_id_native(
        data, format=RdfFormat.TURTLE, base=BASE
    ).startswith("blake3:")

    with pytest.raises(ValueError) as excinfo:
        purrdf.gts_from_quads(data, format=RdfFormat.TURTLE)
    assert NO_BASE_CODE in str(excinfo.value)


def test_the_base_is_the_same_parameter_across_parse_surfaces() -> None:
    # One base, one meaning: the module parse, the frozen dataset, and the store all
    # resolve the same document to the same subject IRI.
    expected = f"<{BASE}rel>"
    store = purrdf.Store()
    store.load(RELATIVE_TURTLE, RdfFormat.TURTLE, base=BASE)
    assert str(purrdf.parse(RELATIVE_TURTLE, RdfFormat.TURTLE, base=BASE)[0].subject) == (
        expected
    )
    assert expected in purrdf.RdfDataset(
        RELATIVE_TURTLE, RdfFormat.TURTLE, base=BASE
    ).to_nquads()
    assert {str(q.subject) for q in store} == {expected}
