# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Byte-compatibility baseline for the Python JSON-LD codec surface."""

from __future__ import annotations

from types import ModuleType

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
