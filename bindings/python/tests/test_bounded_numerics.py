# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: asserts what the installed CPython extension's XSD helpers return to Python callers
"""Numeric literals past the bounded representation, from Python.

``xsd:integer`` computes in ``i128`` and ``xsd:decimal`` in an ``i128`` mantissa with at
most 18 fractional digits. A literal past those bounds is still a well-typed number:
the native helpers compare it exactly and give its canonical form, and a SPARQL query
orders and averages it.
"""

from __future__ import annotations

import purrdf

XSD = "http://www.w3.org/2001/XMLSchema#"
BIG = "100000000000000000000000000000000000000000"
MAX = "170141183460469231731687303715884105727"


def test_values_past_the_bounds_compare_exactly() -> None:
    integer = XSD + "integer"
    decimal = XSD + "decimal"
    assert purrdf.xsd_value_compare(BIG, integer, MAX, integer) == 1
    assert purrdf.xsd_value_compare(MAX, integer, BIG, integer) == -1
    assert purrdf.xsd_value_compare("+000" + BIG, integer, BIG + ".000", decimal) == 0
    assert (
        purrdf.xsd_value_compare("0.10000000000000000001", decimal, "0.1", decimal)
        == 1
    )
    # A malformed literal is still incomparable.
    assert purrdf.xsd_value_compare("1.5", integer, "1", integer) is None


def test_values_past_the_bounds_have_a_canonical_form() -> None:
    assert purrdf.xsd_canonical_lexical("+000" + BIG, XSD + "integer") == BIG
    assert (
        purrdf.xsd_canonical_lexical("-00.100000000000000000010", XSD + "decimal")
        == "-0.10000000000000000001"
    )
    # The bounded neighbour, and a malformed one.
    assert purrdf.xsd_canonical_lexical("+007", XSD + "integer") == "7"
    assert purrdf.xsd_canonical_lexical("1.5", XSD + "integer") is None


def test_the_rdflib_shim_reads_them_as_well_typed_numbers() -> None:
    from purrdf.compat.rdflib import Literal, URIRef

    big = Literal(BIG, datatype=URIRef(XSD + "integer"))
    assert big.ill_typed is False
    assert big.toPython() == int(BIG)
    same = Literal(BIG + ".0", datatype=URIRef(XSD + "decimal"))
    assert big.eq(same)
    smaller = Literal(MAX, datatype=URIRef(XSD + "integer"))
    assert smaller < big
    assert not big < smaller


def test_shex_numeric_facets_compare_against_the_bound_as_written() -> None:
    from purrdf import shex

    schema = (
        "PREFIX ex: <http://example.org/>\n"
        "ex:Big { ex:n MININCLUSIVE 100000000000000000001 }\n"
        "ex:Third { ex:n MAXEXCLUSIVE 0.30000000000000001 }\n"
    )
    data = (
        "@prefix ex: <http://example.org/> .\n"
        "ex:a ex:n 100000000000000000000 .\n"
        "ex:b ex:n 100000000000000000001 .\n"
        "ex:e ex:n 0.3 .\n"
        "ex:f ex:n 0.30000000000000001 .\n"
    )
    for node, shape, conformant in [
        ("a", "Big", False),
        ("b", "Big", True),
        ("e", "Third", True),
        ("f", "Third", False),
    ]:
        entries = shex.validate(
            schema,
            data,
            [(f"http://example.org/{node}", f"http://example.org/{shape}")],
        )
        assert entries[0]["conformant"] is conformant, (node, shape, entries)


def test_an_xsd_refusal_names_its_fo_code() -> None:
    from purrdf import shex

    schema = (
        "PREFIX ex: <http://example.org/>\n"
        "PREFIX xsd: <http://www.w3.org/2001/XMLSchema#>\n"
        "ex:S { ex:n xsd:integer }\n"
    )
    data = (
        "@prefix ex: <http://example.org/> .\n"
        "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n"
        'ex:bad ex:n "1.5"^^xsd:integer .\n'
        f'ex:big ex:n "{BIG}"^^xsd:integer .\n'
    )
    bad, big = shex.validate(
        schema,
        data,
        [
            ("http://example.org/bad", "http://example.org/S"),
            ("http://example.org/big", "http://example.org/S"),
        ],
    )
    assert bad["conformant"] is False
    assert "err:FORG0001" in bad["reason"], bad
    # Neighbour: a well-formed integer past i128 conforms.
    assert big["conformant"] is True, big
