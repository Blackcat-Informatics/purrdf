# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: asserts the exception attributes the installed CPython extension raises to Python callers
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


def _assert_no_presentation(error: BaseException) -> None:
    assert isinstance(error, ValueError)
    assert error.message_id is None
    assert error.presentation is None


def _short_row_relation() -> dict:
    """A relation declared 1 x 1 whose one row carries a single term."""
    return {f"{EX}r": (1, 1, [(purrdf.NamedNode(f"{EX}a"),)])}


def _full_row_relation() -> dict:
    return {f"{EX}r": (1, 1, [(purrdf.NamedNode(f"{EX}a"), purrdf.NamedNode(f"{EX}b"))])}


# Every argument refusal of the six SPARQL methods, each beside its parse-failure
# neighbour on the same method, which carries a presentation.
ARGUMENT_REFUSALS = [
    (
        "unknown entailment regime",
        lambda s: s.query_entailment_governed("ASK {}", "bogus"),
        lambda s: s.query_entailment_governed("ASK {", "rdfs"),
        "unknown entailment regime",
    ),
    (
        "a rule document on a regime that takes none",
        lambda s: s.query_entailment_governed("ASK {}", "rdfs", program="garbage((("),
        lambda s: s.query_entailment_governed("ASK {", "rdfs"),
        "takes no rule document",
    ),
    (
        "query: a relation row of the wrong arity",
        lambda s: s.query("ASK {}", relations=_short_row_relation()),
        lambda s: s.query("ASK {", relations=_full_row_relation()),
        r"row 0 has 1 value\(s\)",
    ),
    (
        "query_governed: a ceiling named beside no ceiling",
        lambda s: s.query_governed("ASK {}", no_ceiling=True, fuel=5),
        lambda s: s.query_governed("ASK {", fuel=5),
        "no ceiling at all",
    ),
    (
        "prepare: a parameter declared twice",
        lambda s: s.prepare("SELECT ?x WHERE { ?x ?p ?o }", parameters=["x", "x"]),
        lambda s: s.prepare("ASK {", parameters=["x"]),
        "declared more than once",
    ),
    (
        "update: a relation row of the wrong arity",
        lambda s: s.update("INSERT DATA {}", relations=_short_row_relation()),
        lambda s: s.update("DELETE WHERE {", relations=_full_row_relation()),
        r"row 0 has 1 value\(s\)",
    ),
    (
        "update_governed: a ceiling named beside no ceiling",
        lambda s: s.update_governed("INSERT DATA {}", no_ceiling=True, fuel=5),
        lambda s: s.update_governed("DELETE WHERE {", fuel=5),
        "no ceiling at all",
    ),
]


@pytest.mark.parametrize(
    ("refused", "parse_failure", "words"),
    [case[1:] for case in ARGUMENT_REFUSALS],
    ids=[case[0] for case in ARGUMENT_REFUSALS],
)
def test_every_value_error_of_the_sparql_methods_carries_both_attributes(
    refused, parse_failure, words: str
) -> None:
    store = _store()
    with pytest.raises(ValueError, match=words) as raised:
        refused(store)
    _assert_no_presentation(raised.value)
    with pytest.raises(ValueError) as neighbour:
        parse_failure(store)
    assert neighbour.value.message_id.startswith("sparql-parse-")
    assert neighbour.value.presentation["message_id"] == neighbour.value.message_id


def test_the_valid_neighbours_of_the_argument_refusals_run() -> None:
    store = _store()
    assert store.query_entailment_governed("ASK {}", "rdfs").is_complete
    assert store.query("ASK {}", relations=_full_row_relation())
    assert store.query_governed("ASK {}", fuel=1_000_000).is_complete
    store.prepare("SELECT ?x WHERE { ?x ?p ?o }", parameters=["x"])
    store.update("INSERT DATA {}", relations=_full_row_relation())
    store.update_governed("INSERT DATA {}", fuel=1_000_000)
    assert len(store) == 1


def test_a_type_error_is_left_as_it_was() -> None:
    """Only `ValueError` gains the attributes; a wrong-typed argument stays a plain `TypeError`."""
    with pytest.raises(TypeError) as raised:
        _store().query("ASK {}", relations={f"{EX}r": []})
    assert not hasattr(raised.value, "message_id")
    assert not hasattr(raised.value, "presentation")
