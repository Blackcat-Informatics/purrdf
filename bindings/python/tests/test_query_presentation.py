# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Typed presentations on SPARQL parse failures.

A refused query or update raises the same ``ValueError``, with the same message and
``args``, that the surface has always raised. The exception also carries the
parser's typed condition: ``message_id`` (``"sparql-parse-syntax"``, …) and
``presentation``, a dict of typed parameters (byte offsets as exact ``int``) whose
``detail`` nests an IRI refusal's own ``iri-*`` condition. A failure with no typed
presentation carries ``None`` in both attributes.
"""

from __future__ import annotations

import pytest

import purrdf

EX = "http://example.org/"
CDT_MAP = "http://w3id.org/awslabs/neptune/SPARQL-CDTs/Map"


def _store() -> purrdf.Store:
    store = purrdf.Store()
    store.load(f"<{EX}s> <{EX}p> <{EX}o> .", purrdf.RdfFormat.N_TRIPLES)
    return store


# One case per parse-failure identity: the query, its identity, the typed
# parameters it must carry, and the exact message the exception has always read.
QUERY_CASES = [
    (
        'SELECT * WHERE { ?s ?p "unterminated }',
        "sparql-parse-lex",
        {"at": ("unsigned", 23), "reason": ("text", "unterminated string literal")},
        "SPARQL lex error at byte 23: unterminated string literal",
    ),
    (
        "ASK {",
        "sparql-parse-syntax",
        {"at": ("unsigned", 5)},
        "SPARQL syntax error at byte 5: expected an RDF term, found None",
    ),
    (
        "ASK {} ORDER BY ?x",
        "sparql-parse-unsupported",
        {"feature": ("text", "solution modifiers on ASK")},
        "unsupported SPARQL construct: solution modifiers on ASK is outside the "
        "SPARQL 1.2 query language this processor implements",
    ),
    (
        f"SELECT * WHERE {{ <{EX}%zz> ?p ?o }}",
        "sparql-parse-iri",
        {"lexical": ("text", f"{EX}%zz")},
        f'invalid IRI "{EX}%zz" in term position: iri-bad-percent-encoding: '
        "malformed percent-encoding at byte 19",
    ),
    (
        f'SELECT (<{CDT_MAP}>("key") AS ?m) WHERE {{}}',
        "sparql-parse-cdt-arity",
        {"at": ("unsigned", 57), "found": ("unsigned", 1)},
        f"SPARQL syntax error at byte 57: <{CDT_MAP}> takes an even number of "
        "arguments (key/value pairs), not 1",
    ),
]


def _assert_parameters(presentation: dict, expected: dict) -> None:
    for name, (kind, value) in expected.items():
        parameter = presentation["parameters"][name]
        assert parameter == {"kind": kind, "value": value}, (name, presentation)
        assert type(parameter["value"]) is type(value), name


@pytest.mark.parametrize(("query", "identity", "parameters", "english"), QUERY_CASES)
def test_query_parse_failure_carries_its_typed_presentation(
    query: str, identity: str, parameters: dict, english: str
) -> None:
    with pytest.raises(ValueError) as raised:
        _store().query(query)
    error = raised.value
    expected = f"query evaluation error: error native-sparql-query-parse: {english}"
    assert error.args == (expected,)
    assert str(error) == expected
    assert error.message_id == identity
    assert error.presentation["message_id"] == identity
    _assert_parameters(error.presentation, parameters)
    if identity == "sparql-parse-iri":
        detail = error.presentation["detail"]
        assert detail["message_id"] == "iri-bad-percent-encoding"
        assert detail["parameters"] == {"offset": {"kind": "unsigned", "value": 19}}
        assert detail["detail"] is None
    else:
        assert error.presentation["detail"] is None


def test_relative_iri_without_base_names_its_typed_cause() -> None:
    with pytest.raises(ValueError) as raised:
        _store().query("SELECT * WHERE { <relative> ?p ?o }")
    detail = raised.value.presentation["detail"]
    assert raised.value.message_id == "sparql-parse-iri"
    assert detail["message_id"] == "iri-relative-no-base"
    assert detail["parameters"] == {"reference": {"kind": "text", "value": "relative"}}


def test_every_query_surface_carries_the_presentation() -> None:
    store = _store()
    for call in (
        lambda: store.query_governed("ASK {"),
        lambda: store.prepare("ASK {"),
        lambda: store.query_entailment_governed("ASK {", "rdfs"),
    ):
        with pytest.raises(ValueError) as raised:
            call()
        assert raised.value.message_id == "sparql-parse-syntax"
        _assert_parameters(raised.value.presentation, {"at": ("unsigned", 5)})


@pytest.mark.parametrize("governed", [False, True])
def test_update_parse_failure_carries_its_typed_presentation(governed: bool) -> None:
    store = _store()
    update = store.update_governed if governed else store.update
    with pytest.raises(ValueError) as raised:
        update("DELETE WHERE {")
    assert str(raised.value) == (
        "update evaluation error: error native-sparql-update-parse: "
        "SPARQL syntax error at byte 14: expected an RDF term, found None"
    )
    assert raised.value.message_id == "sparql-parse-syntax"
    _assert_parameters(raised.value.presentation, {"at": ("unsigned", 14)})
    with pytest.raises(ValueError) as raised:
        update(f"INSERT DATA {{ <{EX}%zz> <{EX}p> 1 }}")
    assert raised.value.message_id == "sparql-parse-iri"
    assert raised.value.presentation["detail"]["parameters"] == {
        "offset": {"kind": "unsigned", "value": 19}
    }
    assert len(store) == 1


def test_a_failure_without_a_presentation_carries_none() -> None:
    """An engine refusal that is not a parse failure has no typed presentation."""
    with pytest.raises(ValueError) as raised:
        _store().query(f"SELECT (AGG(<{EX}unregistered>, ?o) AS ?x) WHERE {{ ?s ?p ?o }}")
    assert str(raised.value).startswith("query evaluation error: ")
    assert "parse" not in str(raised.value)
    assert raised.value.message_id is None
    assert raised.value.presentation is None


def test_a_valid_neighbour_still_runs() -> None:
    store = _store()
    assert store.query("ASK {}")
    assert store.query(f"SELECT * WHERE {{ <{EX}s> ?p ?o }}")
    store.update(f"INSERT DATA {{ <{EX}a> <{EX}p> 1 }}")
    assert len(store) == 2
