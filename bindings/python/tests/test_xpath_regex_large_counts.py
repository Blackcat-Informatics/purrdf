# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: exercises the installed CPython extension's real query and validator keyword dispatch and returned Python objects.

"""Large finite counts through the installed dated XPath binding surfaces."""

from __future__ import annotations

import json

import pytest
import purrdf

from test_xpath_regex_profiles import (
    EX, LAWS, P, counted_values, lexical, shacl_validate, shex_conformant,
)


@pytest.fixture(scope="module")
def large_count_values(counted_values: dict[str, str]) -> dict[str, str]:
    prose = counted_values["prose"][: 4 << 20]
    ab = ("abbaab" * ((4 << 20) // 6 + 1))[: 4 << 20]
    assert len(prose) == len(ab) == 4 << 20
    return {
        "prose": prose,
        "prose_lt": prose + "<",
        "prose_bang": prose + "!",
        "ab": ab,
        "ab_longer": ab + "a",
        "ab_broken": ab[:3_999_999] + "c" + ab[:194_305],
    }


LARGE_COUNTS = (
    (r"^[\s\S]{0,10000000}$", "prose", True),
    (r"^[^\n]{0,10000000}$", "prose", True),
    (r"^[^<>]{1,5000000}$", "prose", True),
    (r"^[^<>]{1,5000000}$", "prose_lt", False),
    (r"^[\p{L} ]{1,5000000}$", "prose", True),
    (r"^(\w+\s?){1,5000000}$", "prose", True),
    (r"^(\w+\s?){1,5000000}$", "prose_bang", False),
    ("^(a|b){4194304}$", "ab", True),
    ("^(a|b){4194304}$", "ab_longer", False),
    ("(a|b){4000000}", "ab", True),
    ("(a|b){4000000}", "ab_broken", False),
)


@pytest.mark.parametrize("law", LAWS)
def test_large_counted_queries(large_count_values: dict[str, str], law: str) -> None:
    store = purrdf.Store()
    store.load(
        "".join(
            f"<{EX}{name}> {P} {json.dumps(value)} .\n"
            for name, value in large_count_values.items()
        ).encode(),
        purrdf.RdfFormat.N_TRIPLES,
    )
    rows = " ".join(
        f"({index} <{EX}{subject}> {json.dumps(pattern)})"
        for index, (pattern, subject, _) in enumerate(LARGE_COUNTS)
    )
    query = (
        f"SELECT ?r WHERE {{ VALUES (?i ?s ?p) {{ {rows} }} "
        f"?s {P} ?v BIND(REGEX(?v, ?p) AS ?r) }} ORDER BY ?i"
    )
    result = store.query(query, xpath_regex=law)
    assert isinstance(result, purrdf.QuerySolutions)
    assert [lexical(row["r"]) for row in result] == [
        str(matched).lower() for _, _, matched in LARGE_COUNTS
    ]


@pytest.mark.parametrize("law", LAWS)
def test_large_counted_replacement(large_count_values: dict[str, str], law: str) -> None:
    half = large_count_values["prose"][: 2 << 20]
    store = purrdf.Store()
    store.load(f"<{EX}s> {P} {json.dumps(half)} .\n".encode(), purrdf.RdfFormat.N_TRIPLES)
    pattern = json.dumps(r"^(\w+\s?){1,5000000}$")
    query = f'SELECT ?r WHERE {{ <{EX}s> {P} ?v BIND(REPLACE(?v, {pattern}, "[$1]") AS ?r) }}'
    result = store.query(query, xpath_regex=law)
    assert isinstance(result, purrdf.QuerySolutions)
    assert lexical(next(iter(result))["r"]) == f"[{half[half.rstrip(' ').rfind(' ') + 1 :]}]"


@pytest.mark.parametrize("law", LAWS)
@pytest.mark.parametrize(("pattern", "subject", "expected"), LARGE_COUNTS)
def test_large_counted_validator_facets(
    large_count_values: dict[str, str], law: str, pattern: str, subject: str, expected: bool
) -> None:
    value = large_count_values[subject]
    assert shacl_validate(pattern, value, law)[0] is expected
    assert shex_conformant(pattern, value, law) is expected
