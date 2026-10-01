# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Byte-compatibility baseline for the Python JSON-LD codec surface."""

from __future__ import annotations

from types import ModuleType
from decimal import Decimal
import json

import purrdf


SOURCE = b'<https://example.org/alice> <https://schema.org/name> "Alice" .\n'
EXPANDED = """{
  "@context": {},
  "@graph": [
    {
      "@id": "https://example.org/alice",
      "https://schema.org/name": {
        "@value": "Alice"
      }
    }
  ]
}"""


def test_to_json_ld_expanded_bytes_are_frozen() -> None:
    """The no-options Python helper remains the exact expanded compatibility route."""
    assert purrdf.to_json_ld(SOURCE, format=purrdf.RdfFormat.N_QUADS) == EXPANDED


def test_rdflib_graph_json_ld_expanded_bytes_are_frozen(compat: ModuleType) -> None:
    """The RDFLib-compatible plugin reaches the same exact expanded Rust bytes."""
    graph = compat.Graph()
    graph.parse(data=SOURCE, format="nquads")
    assert graph.serialize(format="json-ld") == EXPANDED


def test_json_document_conversion_and_dict_numeric_contracts() -> None:
    document = '[{"@id":"https://example.org/s","https://example.org/p":[{"@value":{"integer":9007199254740993,"decimal":0.12345678901234567890123456789},"@type":"@json"}]}]'
    dataset = purrdf.from_json_ld(document)
    rendered = purrdf.to_json_ld(dataset, format=purrdf.RdfFormat.N_QUADS)
    assert isinstance(rendered, str)
    converted = json.loads(rendered, parse_int=int, parse_float=Decimal)
    lexical = converted["@graph"][0]["https://example.org/p"]["@value"]
    assert isinstance(lexical, str)
    value = json.loads(lexical, parse_int=int, parse_float=Decimal)
    assert type(value["integer"]) is int
    assert value["integer"] == 9007199254740993
    assert type(value["decimal"]) is Decimal
    assert value["decimal"] == Decimal("0.12345678901234568")
    provenance = purrdf.provenance_from_json(
        b'{"head":{"vars":[]},"results":{"bindings":[]}}',
        "ex", "https://example.org/provenance",
    )
    assert isinstance(provenance, dict)
    assert provenance == {"query_hash": None, "engine": None}
