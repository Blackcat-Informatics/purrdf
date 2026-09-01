# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""The SEP-0008 SHA-3 built-ins, reached the way a Python caller reaches them.

The Rust evaluator's unit tests already pin these digests. They cannot pin what a
Python caller receives: between the query string and the `Literal` handed back
there is a lexer that must keep `SHA3-256` as ONE word, a dispatch table that must
send each size to its own digest arm, and a PyO3 egress that must carry the result
term across intact. This module drives that whole path through `Store.query`.

Vector provenance
-----------------
NIST FIPS 202 publishes `"abc"` as a worked example for all four SHA-3 sizes. Each
expected digest below was taken from that published table and independently
cross-checked against two implementations that are NOT the code under test:

* OpenSSL — ``printf 'abc' | openssl dgst -sha3-256``
* CPython's ``hashlib`` — ``hashlib.new("sha3_256", b"abc").hexdigest()``

The test itself re-derives them from ``hashlib`` at run time as well, so a vector
transcribed wrongly into this file fails before it can be blamed on the engine.
"""

from __future__ import annotations

import hashlib

import purrdf
import pytest

EX = "https://example.org/"

# The NIST FIPS 202 example message, as one N-Triples statement.
_DATA = f'<{EX}s> <{EX}message> "abc" .\n'.encode()

# (function name, SELECT alias, published FIPS 202 digest of "abc").
_VECTORS = [
    ("SHA3-224", "h224", "e642824c3f8cf24ad09234ee7d3c766fc9a3a5168d0c94ad73b46fdf"),
    (
        "SHA3-256",
        "h256",
        "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532",
    ),
    (
        "SHA3-384",
        "h384",
        "ec01498288516fc926459f58e2c6ad8df9b473cb0fc08c2596da7cf0e49be4b298d88cea927ac7f5"
        "39f1edf228376d25",
    ),
    (
        "SHA3-512",
        "h512",
        "b751850b1a57168a5693cd924b6b096e08f621827444f70d884f5d0240d2712e10e116e9192af3c9"
        "1a7ec57647e3934057340b4cf408d5a56592f8274eec53f0",
    ),
]


def _store() -> purrdf.Store:
    """A store holding the single `"abc"` message statement."""
    store = purrdf.Store()
    store.load(_DATA, format=purrdf.RdfFormat.N_TRIPLES)
    return store


def _select(spelling: str = "-") -> str:
    """A SELECT projecting all four digests of ?m, hyphenated or underscored."""
    projections = " ".join(
        f"({name.replace('-', spelling)}(?m) AS ?{alias})" for name, alias, _ in _VECTORS
    )
    return f"PREFIX ex: <{EX}> SELECT {projections} WHERE {{ ?s ex:message ?m }}"


def _row(query: str) -> purrdf.QuerySolution:
    """The single solution row of `query`."""
    solutions = _store().query(query)
    assert isinstance(solutions, purrdf.QuerySolutions)
    rows = list(solutions)
    assert len(rows) == 1, "the fixture binds exactly one row"
    return rows[0]


def test_the_expected_vectors_are_the_published_ones() -> None:
    """The table above agrees with an implementation that is not purrdf.

    `hashlib` is CPython's own SHA-3 (a distinct implementation from the Rust
    `sha3` crate under test), so this catches a mistranscribed vector here rather
    than letting it be reported as an engine defect below.
    """
    for name, _alias, want in _VECTORS:
        size = name.removeprefix("SHA3-")
        assert hashlib.new(f"sha3_{size}", b"abc").hexdigest() == want


def test_sha3_builtins_reach_their_published_digests_through_store_query() -> None:
    """Each SEP-0008 name reaches ITS OWN digest, out through the Python surface."""
    row = _row(_select())
    for name, alias, want in _VECTORS:
        term = row[alias]
        assert isinstance(term, purrdf.Literal), f"{name} must bind a Literal"
        assert term.value == want, f"{name} does not match its published FIPS 202 vector"

    # The four sizes are four different functions: 224/256/384/512 bits are
    # 56/64/96/128 hex characters, and no two digests collide.
    values = [row[alias].value for _, alias, _ in _VECTORS]
    assert [len(v) for v in values] == [56, 64, 96, 128]
    assert len(set(values)) == 4


def test_sha3_underscored_sep_spelling_reaches_the_same_digests() -> None:
    """SEP-0008 spells its functions with an underscore; that spelling parses too."""
    row = _row(_select(spelling="_"))
    for name, alias, want in _VECTORS:
        assert row[alias].value == want, f"the underscored spelling of {name} must agree"


def test_a_spaced_sha3_hyphen_is_refused_rather_than_answered() -> None:
    """`SHA3 - 256` is not the built-in: `SHA3` alone is no function or keyword."""
    with pytest.raises(ValueError):
        _store().query(f"PREFIX ex: <{EX}> SELECT (SHA3 - 256 AS ?h) WHERE {{ ?s ex:message ?m }}")


def test_a_subtraction_beside_a_sha3_call_is_still_a_subtraction() -> None:
    """`STRLEN(SHA3-256(?m)) - 4` is arithmetic: 64 hex characters less four."""
    row = _row(
        f"PREFIX ex: <{EX}> "
        "SELECT (STRLEN(SHA3-256(?m)) - 4 AS ?n) WHERE { ?s ex:message ?m }"
    )
    assert row["n"].value == "60"


def test_a_language_tagged_argument_hashes_its_lexical_form() -> None:
    """A tagged literal hashes the TEXT, not the tag — the same digest as plain `"abc"`.

    The hash built-ins read their argument by reference rather than copying it, and
    this pins that the borrow accepts exactly the literal shapes the copy did: a
    language-tagged literal is hashed on its lexical form, and a non-string literal
    is an expression error (an unbound projection), not a digest of its text.
    """
    store = purrdf.Store()
    store.load(
        (
            f'<{EX}s> <{EX}message> "abc"@en .\n'
            f'<{EX}s> <{EX}count> "7"^^<http://www.w3.org/2001/XMLSchema#integer> .\n'
        ).encode(),
        format=purrdf.RdfFormat.N_TRIPLES,
    )
    rows = list(
        store.query(
            f"PREFIX ex: <{EX}> SELECT (SHA3-256(?m) AS ?h) (SHA3-256(?c) AS ?bad) "
            "WHERE { ?s ex:message ?m . ?s ex:count ?c }"
        )
    )
    assert len(rows) == 1
    assert rows[0]["h"].value == dict((n, d) for n, _, d in _VECTORS)["SHA3-256"]
    assert rows[0]["bad"] is None, "a non-string argument is an error, not a digest"
