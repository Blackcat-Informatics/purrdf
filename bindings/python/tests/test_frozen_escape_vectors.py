# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Frozen UCHAR/ECHAR answers through the exported SPARQL parser.

The raw decoder returns a scalar and consumed prefix length. A successful vector
is replayed using exactly that frozen prefix as a string body, so surrounding
query grammar adds no restrictions on its trailing bytes. Refused vectors retain
their complete body and must fail on the invalid initial escape.
"""

from __future__ import annotations

import hashlib

import purrdf
import pytest

from test_frozen_lex_vectors import BATCH, _records


@pytest.mark.parametrize("body,uchar,string", _records("escape_vectors.txt"))
def test_frozen_string_escape_prefix(body: str, uchar: str, string: str) -> None:
    store = purrdf.Store()
    if string == "-":
        with pytest.raises(ValueError):
            store.query('SELECT ("' + body + '" AS ?value) WHERE {}')
    else:
        scalar, consumed = string.split("/")
        prefix = body[:int(consumed)]
        rows = list(store.query('SELECT ("' + prefix + '" AS ?value) WHERE {}'))
        assert rows[0][0].value == chr(int(scalar, 16))


def _escaped_values(store, bodies: list[str]) -> list[str]:
    bindings = " ".join(f'({i} "{body}")' for i, body in enumerate(bodies))
    rows = store.query("SELECT ?value WHERE { VALUES (?i ?value) { " + bindings + " } } ORDER BY ?i")
    return [row[0].value for row in rows]


@pytest.mark.parametrize("form,case,first,last,expected", _records("uchar_scalar_vectors.txt"))
def test_every_frozen_uchar_scalar_digest(form, case, first, last, expected) -> None:
    store = purrdf.Store()
    digest = hashlib.sha256()
    digits = ("04" if form == "u" else "08") + ("X" if case == "upper" else "x")
    consumed = 6 if form == "u" else 10
    start, stop = int(first, 16), int(last, 16) + 1
    for offset in range(start, stop, BATCH):
        points = list(range(offset, min(offset + BATCH, stop)))
        bodies = ["\\" + form + format(cp, digits) for cp in points]
        if offset >= 0x110000 or 0xD800 <= offset < 0xE000:
            for body in bodies:
                with pytest.raises(ValueError):
                    _escaped_values(store, [body])
                digest.update(b"-\n")
        else:
            values = _escaped_values(store, bodies)
            assert len(values) == len(points)
            for cp, value in zip(points, values, strict=True):
                assert value == chr(cp)
                digest.update(f"{ord(value):04X}/{consumed}\n".encode())
    assert digest.hexdigest() == expected
