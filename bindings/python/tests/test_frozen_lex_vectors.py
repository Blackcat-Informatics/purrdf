# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check escaped results and URI strings at the Python binding boundary."""

import purrdf
import pytest


def _parse_json_result(value: bytes):
    document = (
        b'{"head":{"vars":["value"]},"results":{"bindings":'
        b'[{"value":{"type":"literal","value":"' + value + b'"}}]}}'
    )
    return purrdf.parse_sparql_results("json", document)


def test_escaped_json_result_yields_a_python_literal() -> None:
    kind, variables, rows = _parse_json_result(rb"\u00e9\ud83d\ude00\u0000\n")
    assert (kind, variables) == ("SELECT", ["value"])
    [[literal]] = rows
    assert isinstance(literal, purrdf.Literal)
    assert literal.value == "é😀\0\n"

    with pytest.raises(ValueError):
        _parse_json_result(rb"\ud800")


def test_store_query_returns_a_utf8_percent_encoded_literal() -> None:
    rows = list(purrdf.Store().query(
        'SELECT (ENCODE_FOR_URI("~é 😀") AS ?value) WHERE {}'
    ))
    [literal] = [row[0] for row in rows]
    assert isinstance(literal, purrdf.Literal)
    assert literal.value == "~%C3%A9%20%F0%9F%98%80"
