# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: asserts the str, int, None and decimal.Decimal values the installed CPython extension and its rdflib-compat Literal conversion hand Python callers
"""Exact XSD integers and decimals through the Python surface.

The value space is Rust-owned (``purrdf_xsd``): integers past ``i128`` and decimals
past the bounded decimal are exact there. These tests pin what a Python caller sees:
``xsd_canonical_lexical`` and ``xsd_value_compare`` keep every digit, a SPARQL sum
past ``i128`` comes back with its exact lexical form, and the rdflib-compat
``Literal.toPython()`` turns those forms into an exact ``int`` / ``decimal.Decimal``.
Each refusal sits beside the valid neighbour it must keep.
"""

from __future__ import annotations

from decimal import Decimal
from types import ModuleType

import purrdf

XSD = "http://www.w3.org/2001/XMLSchema#"
INTEGER = XSD + "integer"
DECIMAL = XSD + "decimal"
DOUBLE = XSD + "double"
STRING = XSD + "string"

I128_MAX = "170141183460469231731687303715884105727"
PAST_I128 = "170141183460469231731687303715884105728"
SIXTY = "123456789012345678901234567890123456789012345678901234567890"
SIXTY_NEXT = "123456789012345678901234567890123456789012345678901234567891"
FORTY_FRACTION_DIGITS = "0.1000000000000000000000000000000000000001"


def test_integers_past_i128_have_their_exact_canonical_form() -> None:
    assert purrdf.xsd_canonical_lexical(PAST_I128, INTEGER) == PAST_I128
    assert int(PAST_I128) == 2**127
    assert purrdf.xsd_canonical_lexical(SIXTY, INTEGER) == SIXTY
    assert purrdf.xsd_canonical_lexical("+000" + SIXTY, INTEGER) == SIXTY
    assert purrdf.xsd_canonical_lexical("-" + PAST_I128, INTEGER) == "-" + PAST_I128


def test_long_decimals_have_their_exact_canonical_form() -> None:
    assert (
        purrdf.xsd_canonical_lexical(FORTY_FRACTION_DIGITS, DECIMAL)
        == FORTY_FRACTION_DIGITS
    )
    assert purrdf.xsd_canonical_lexical("1.50", DECIMAL) == "1.5"


def test_a_malformed_lexical_has_no_canonical_form_beside_its_valid_neighbour() -> None:
    assert purrdf.xsd_canonical_lexical("12x", INTEGER) is None
    assert purrdf.xsd_canonical_lexical("12", INTEGER) == "12"


def test_long_integers_compare_exactly() -> None:
    assert purrdf.xsd_value_compare(SIXTY, INTEGER, SIXTY_NEXT, INTEGER) == -1
    assert purrdf.xsd_value_compare(SIXTY_NEXT, INTEGER, SIXTY, INTEGER) == 1
    assert purrdf.xsd_value_compare(SIXTY, INTEGER, "0" + SIXTY, INTEGER) == 0
    # 10^42 + 1 and 10^42 + 2 are the same binary64 value.
    plus_one, plus_two = str(10**42 + 1), str(10**42 + 2)
    assert float(plus_one) == float(plus_two)
    assert purrdf.xsd_value_compare(plus_one, INTEGER, plus_two, INTEGER) == -1
    assert purrdf.xsd_value_compare(plus_one, INTEGER, plus_one + ".0", DECIMAL) == 0


def test_malformed_and_incomparable_values_have_no_order_beside_valid_neighbours() -> None:
    assert purrdf.xsd_value_compare("12x", INTEGER, "12", INTEGER) is None
    assert purrdf.xsd_value_compare("12", INTEGER, "12", INTEGER) == 0
    assert purrdf.xsd_value_compare("NaN", DOUBLE, "1", DOUBLE) is None
    assert purrdf.xsd_value_compare("1", INTEGER, "1", STRING) is None
    assert purrdf.xsd_value_compare("1", INTEGER, "1", DOUBLE) == 0


def test_a_native_sparql_sum_past_i128_is_exact() -> None:
    (row,) = list(purrdf.Store().query(f"SELECT ?n WHERE {{ BIND({I128_MAX} + 1 AS ?n) }}"))
    literal = row["n"]
    assert isinstance(literal, purrdf.Literal)
    assert literal.value == PAST_I128
    assert literal.datatype.value == INTEGER


def test_a_compat_sparql_sum_past_i128_is_an_exact_int(compat: ModuleType) -> None:
    graph = compat.Graph()
    (row,) = list(graph.query(f"SELECT ?n WHERE {{ BIND({I128_MAX} + 1 AS ?n) }}"))
    literal = row[0]
    assert str(literal) == PAST_I128
    assert str(literal.datatype) == INTEGER
    value = literal.toPython()
    assert type(value) is int
    assert value == 2**127


def test_a_compat_long_integer_and_decimal_convert_exactly(compat: ModuleType) -> None:
    integer = compat.Literal(SIXTY, datatype=compat.URIRef(INTEGER)).toPython()
    assert type(integer) is int
    assert integer == int(SIXTY)
    assert integer + 1 == int(SIXTY_NEXT)
    decimal = compat.Literal(FORTY_FRACTION_DIGITS, datatype=compat.URIRef(DECIMAL)).toPython()
    assert type(decimal) is Decimal
    assert decimal == Decimal(FORTY_FRACTION_DIGITS)
    assert str(decimal) == FORTY_FRACTION_DIGITS
    # The neighbour that differs only in its last digit is a different value.
    assert decimal != Decimal("0.1000000000000000000000000000000000000002")


def test_a_compat_sparql_decimal_sum_is_an_exact_decimal(compat: ModuleType) -> None:
    graph = compat.Graph()
    (row,) = list(graph.query(f"SELECT ?d WHERE {{ BIND({FORTY_FRACTION_DIGITS} + 1 AS ?d) }}"))
    literal = row[0]
    assert str(literal.datatype) == DECIMAL
    value = literal.toPython()
    assert type(value) is Decimal
    assert value == Decimal("1.1000000000000000000000000000000000000001")
