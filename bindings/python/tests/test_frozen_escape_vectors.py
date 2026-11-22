# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Check the conversion of escaped SPARQL literals and failures to Python."""

import purrdf
import pytest


def test_store_query_returns_a_literal_with_the_decoded_escapes() -> None:
    store = purrdf.Store()
    rows = list(store.query(r'SELECT ("\t\u00e9\U0001F600" AS ?value) WHERE {}'))
    [literal] = [row[0] for row in rows]
    assert isinstance(literal, purrdf.Literal)
    assert literal.value == "\té😀"

    with pytest.raises(ValueError):
        store.query(r'SELECT ("\uD800" AS ?value) WHERE {}')
