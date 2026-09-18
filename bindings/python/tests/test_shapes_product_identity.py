# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Binding a prepared shapes product to the product the caller MEANT.

Every other check ``ShapesProduct.admit()`` runs asks about this process: is it the
build that wrote the memo, are these the registries the product was prepared against,
does this build's class walk re-derive the pinned analysis. None of them asks whether
the bytes in hand are the ones the caller wanted, because nothing in a product states
which product was meant — so admitting a cache entry, a downloaded artifact, or a path
built from a configuration string returns a well-formed report about a shapes graph
nobody asked about. ``admit_expecting()`` is the caller's half of that statement.

Each refusal here is executed beside a NEIGHBOURING valid case, because an expectation
nobody can satisfy is worse than no expectation at all: it sends every caller straight
back to the unbound call it exists to replace.
"""

from __future__ import annotations

import pytest

import purrdf

# A shape requiring ex:age to be an xsd:integer; the data below violates it.
_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [ sh:path ex:age ; sh:datatype xsd:integer ] .
"""

# A second shapes graph over different classes, so the two products genuinely carry two
# input bindings rather than two copies of one.
_OTHER_SHAPES = """@prefix sh: <http://www.w3.org/ns/shacl#> .
@prefix ex: <http://example.org/> .

ex:WidgetShape a sh:NodeShape ;
    sh:targetClass ex:Widget ;
    sh:property [ sh:path ex:maker ; sh:minCount 1 ] .
"""

_DATA = (
    "<http://example.org/alice> "
    "<http://www.w3.org/1999/02/22-rdf-syntax-ns#type> "
    "<http://example.org/Person> .\n"
    '<http://example.org/alice> <http://example.org/age> "not-an-int" .\n'
)


def _product(shapes_ttl: str) -> purrdf.shapes.ShapesProduct:
    return purrdf.shapes.ShapesProduct.open(purrdf.shapes.pack_product(shapes_ttl))


def test_a_product_that_is_not_the_expected_one_is_refused() -> None:
    """The wrong product is refused on a named dimension, before anything is decoded."""
    held = _product(_SHAPES)
    wanted = _product(_OTHER_SHAPES).identity_digest()
    assert wanted != held.identity_digest()

    with pytest.raises(purrdf.shapes.ShapesProductError) as refused:
        held.admit_expecting(wanted)
    assert refused.value.dimension == "shapes-graph"

    # The gap this closes, stated as a passing assertion: the unbound path admits the
    # very same bytes, because nothing in them says which product was meant.
    held.admit()


def test_a_product_required_to_be_itself_admits_and_answers_identically() -> None:
    """The paired neighbour: a satisfied expectation changes the door, not the answer."""
    product = _product(_SHAPES)

    bound = product.admit_expecting(product.identity_digest()).validate_nt(_DATA)
    unbound = product.admit().validate_nt(_DATA)
    assert bound.conforms is False, "the fixture must find its violation"
    assert bound.conforms == unbound.conforms
    assert bound.to_ntriples() == unbound.to_ntriples()


def test_the_rendered_identity_is_the_accepted_identity() -> None:
    """The selector is a ROUND TRIP: what a product reports is what it accepts back."""
    product = _product(_SHAPES)
    reported = product.identity_digest()
    assert len(reported) == 64

    product.admit_expecting(reported)

    # Case is not significant on the way in. The reporting side emits lowercase, but a
    # selector that travelled through a shell, a manifest or a CI variable may not have
    # stayed that way, and refusing it for a shape the mechanism does not care about
    # would be refusing input that is actually valid.
    product.admit_expecting(reported.upper())


@pytest.mark.parametrize("bad", ["", "not-a-digest", "abc", "f" * 63, "f" * 65])
def test_a_selector_that_is_not_a_digest_names_no_dimension(bad: str) -> None:
    """A mis-typed selector is the caller's argument, not the artifact's fault.

    It raises the plain ``ValueError`` rather than ``ShapesProductError``: no product
    was opened, so there is nothing for an admission dimension to name, and blaming the
    product would send the caller to inspect an artifact that is not at fault.
    """
    product = _product(_SHAPES)
    with pytest.raises(ValueError) as refused:
        product.admit_expecting(bad)
    assert not isinstance(refused.value, purrdf.shapes.ShapesProductError)
    assert "64 hexadecimal digits" in str(refused.value)

    # The neighbouring valid spelling still admits.
    product.admit_expecting(product.identity_digest())


def test_a_rebuilt_product_that_is_not_the_expected_one_is_refused() -> None:
    """``rebuild_expecting()`` answers the same "is this the product I asked for?"
    question ``admit_expecting()`` does: a product whose binding is not the one
    required is refused on ``shapes-graph`` even though its stage id is one this
    build knows and the unbound ``rebuild()`` would otherwise happily re-derive it.
    """
    held = _product(_SHAPES)
    wanted = _product(_OTHER_SHAPES).identity_digest()
    assert wanted != held.identity_digest()

    with pytest.raises(purrdf.shapes.ShapesProductError) as refused:
        held.rebuild_expecting(wanted)
    assert refused.value.dimension == "shapes-graph"

    # The gap this closes: the unbound rebuild restores the very same bytes,
    # because nothing in them states which product was meant.
    held.rebuild()


def test_a_rebuilt_product_required_to_be_itself_answers_identically() -> None:
    """The paired neighbour: a satisfied expectation changes the door, not the
    answer — rebuilding rather than admitting must not change it either."""
    product = _product(_SHAPES)

    bound_rebuild = product.rebuild_expecting(product.identity_digest()).validate_nt(_DATA)
    unbound_rebuild = product.rebuild().validate_nt(_DATA)
    assert bound_rebuild.conforms is False, "the fixture must find its violation"
    assert bound_rebuild.conforms == unbound_rebuild.conforms
    assert bound_rebuild.to_ntriples() == unbound_rebuild.to_ntriples()

    bound_admit = product.admit_expecting(product.identity_digest()).validate_nt(_DATA)
    assert bound_rebuild.to_ntriples() == bound_admit.to_ntriples()
