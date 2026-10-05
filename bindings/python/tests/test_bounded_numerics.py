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
