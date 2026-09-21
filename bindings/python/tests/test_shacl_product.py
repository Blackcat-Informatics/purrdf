# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Proving the Python prepared-shapes-product surface, not just exercising it.

``purrdf.shapes`` gained ``PreparedShapes``, ``ShapesProduct`` and ``pack_product``
alongside the ``ShapesProductError`` raised at the admission boundary, but no test
ever called any of them through Python. The claim under test is that this binding
does not merely forward calls into the Rust engine — it preserves what makes the
prepared-product feature worth having: a restored preparation answers exactly what
a fresh parse answers, the one-call and three-step packing routes agree byte for
byte, and a refusal's structured ``.dimension`` survives the PyO3 boundary rather
than collapsing into an opaque string a caller has to substring-match.

Every refusal here is executed beside a NEIGHBOURING valid case: a suite of
refusals alone is satisfied by a binding that refuses everything.
"""

from __future__ import annotations

import pytest

import purrdf

# A shape requiring ex:age to be an xsd:integer.
_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .
"""

# Two people: one violates the shape (non-integer age), one satisfies it. Both
# present so the report compared below is not a single-triple vacuous case.
_DATA = (
    "<http://example.org/alice> "
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/Person> .\n"
    '<http://example.org/alice> <http://example.org/age> "not-an-int" .\n'
    "<http://example.org/bob> "
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/Person> .\n"
    '<http://example.org/bob> <http://example.org/age> "42"^^<http://www.w3.org/2001/XMLSchema#integer> .\n'
)

# Turtle that is not turtle at all: the shapes DOCUMENT never parses, so no
# product is ever written and no admission dimension can name the failure.
_NOT_TURTLE = "@@@ this is not turtle @@@"


def _pack() -> bytes:
    """Pack ``_SHAPES`` the three-step way: parse, prepare, encode."""
    return purrdf.shapes.Shapes(_SHAPES).prepare().to_product()


# ── 1. Round-trip parity ────────────────────────────────────────────────────────


def test_restored_product_matches_direct_validation() -> None:
    """A product opened, admitted and validated answers what direct validation does.

    Equality is asserted on the report's own content (canonical N-Triples and
    ``conforms``), never on "admitting and validating didn't raise" — a binding
    that silently dropped every result would also pass a check that lenient.
    """
    direct = purrdf.shapes.Shapes(_SHAPES).validate_nt(_DATA)
    assert direct.conforms is False, "the fixture must find its violation"

    restored = purrdf.shapes.ShapesProduct.open(_pack()).admit().validate_nt(_DATA)
    assert restored.conforms == direct.conforms
    assert restored.to_ntriples() == direct.to_ntriples()


def test_rebuild_also_matches_direct_validation() -> None:
    """``rebuild()`` re-derives the preparation from the carried dataset and must
    answer identically — it is reachable from no other test on any surface."""
    direct = purrdf.shapes.Shapes(_SHAPES).validate_nt(_DATA)

    rebuilt = purrdf.shapes.ShapesProduct.open(_pack()).rebuild().validate_nt(_DATA)
    assert rebuilt.conforms == direct.conforms
    assert rebuilt.to_ntriples() == direct.to_ntriples()


def test_certify_corroborates_a_genuine_product() -> None:
    """``certify()`` independently recomputes the shapes dataset's canonical
    identity and agrees with the binding the product carries; it raises nothing
    for a product that was never tampered with."""
    purrdf.shapes.ShapesProduct.open(_pack()).certify()


# ── 2. One-call parity ───────────────────────────────────────────────────────────


def test_pack_product_matches_the_three_step_route_byte_for_byte() -> None:
    """The one-call ``pack_product`` and ``Shapes(...).prepare().to_product()`` are
    the same composition, so they must produce identical bytes, not merely
    equivalent ones."""
    one_call = purrdf.shapes.pack_product(_SHAPES)
    three_step = _pack()
    assert one_call == three_step


# ── 3 & 6. A refusal carries its dimension label across the boundary ────────────


def _corrupt(data: bytes) -> bytes:
    """Flip one byte inside a section payload, well clear of the header,
    directory and 64-byte trailer, so the corruption is content corrupted in
    place rather than framing the envelope itself would already refuse."""
    offset = len(data) - 96
    assert offset > 64, "the fixture product must be large enough to carry a safe offset"
    mutable = bytearray(data)
    mutable[offset] ^= 0xFF
    return bytes(mutable)


def test_a_corrupted_product_is_refused_with_its_structured_dimension() -> None:
    """The raised exception's ``.dimension`` attribute — not ``str(exc)`` — names
    the failing dimension. Asserting on the message alone would not prove the
    structured label survived the language boundary; the whole point of
    ``ShapesProductError`` is that a caller branches on the attribute."""
    corrupted = _corrupt(_pack())

    with pytest.raises(purrdf.shapes.ShapesProductError) as refused:
        purrdf.shapes.ShapesProduct.open(corrupted)
    assert refused.value.dimension == "section-digest"

    # The neighbouring VALID case: the same bytes, unflipped, still opens and
    # admits. Without this, a binding that refused every buffer unconditionally
    # would also pass the assertion above.
    purrdf.shapes.ShapesProduct.open(_pack()).admit()


# ── 4. The paired case: a document that never parsed names NO dimension ────────


def test_a_shapes_document_that_does_not_parse_names_no_dimension() -> None:
    """A shapes graph that fails to parse never reaches the admission boundary, so
    no product ever existed for a dimension to describe. This is the case that
    proves ``.dimension`` is not just always populated: it is genuinely absent
    exactly when nothing was admitted."""
    with pytest.raises(purrdf.shapes.ShapesProductError) as refused:
        purrdf.shapes.pack_product(_NOT_TURTLE)
    assert refused.value.dimension is None

    # The neighbouring VALID case: a shapes document that DOES parse still packs.
    purrdf.shapes.pack_product(_SHAPES)


# ── 5. A restored preparation names the artifact it came from ──────────────────


def test_a_restored_preparation_names_the_product_it_came_from() -> None:
    """``provenance()`` answers the half admission does not.

    ``admit()`` establishes that this build MAY execute a product; nothing in the
    report it produces says WHICH product produced it, so a caller looking at a
    verdict afterwards cannot attribute it to an artifact. The digest asserted here
    is read off ``identity_digest()`` rather than restated, because the value is
    only useful if it is the one spelling ``admit_expecting()`` accepts back.
    """
    product = _pack()
    view = purrdf.shapes.ShapesProduct.open(product)
    digest = view.identity_digest()

    admitted = purrdf.shapes.ShapesProduct.open(product).admit()
    assert admitted.provenance() == f"restored-admitted {digest}"

    rebuilt = purrdf.shapes.ShapesProduct.open(product).rebuild()
    assert rebuilt.provenance() == f"restored-rebuilt {digest}"

    # The two restore seams stay distinguishable: the digest means "checked against
    # this process" on one and "recorded from the artifact, deliberately not
    # checked" on the other, and one token for both would report the stronger claim.
    assert admitted.provenance() != rebuilt.provenance()

    # The digest is exactly the selector the bound restore takes, which is what
    # makes reading it off a log worth anything.
    purrdf.shapes.ShapesProduct.open(product).admit_expecting(
        admitted.provenance().split(" ")[1]
    )


def test_a_parsed_preparation_names_no_artifact() -> None:
    """The paired case, and the one that proves the accessor is TOTAL rather than
    always-restored: a preparation built by parsing answers ``parsed``. It names no
    artifact because there is none, and PurRDF invents no fact it was not given."""
    assert purrdf.shapes.Shapes(_SHAPES).prepare().provenance() == "parsed"
