# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Acceptance driver: SPARQLWrapper against the purrdf ``rdflib`` shadow.

SPARQLWrapper's core job is to POST a query to a SPARQL endpoint and parse the
response. The *transport* half needs a live endpoint (exercised by no offline
row — see the ledgered network test); the *result-parsing* half is where its
rdflib coupling lives and is fully exercisable offline by feeding a canned
response document into :class:`SPARQLWrapper.Wrapper.QueryResult`.

Two paths are driven, both on ``example.org`` fixtures:

1. A ``SELECT`` result in SPARQL-Results JSON — SPARQLWrapper's own parser (no
   rdflib), confirming the result surface round-trips.
2. A ``CONSTRUCT`` result in RDF/XML — the rdflib-backed path: ``_convertRDF``
   does ``from rdflib import ConjunctiveGraph`` (→ shadow → purrdf) and
   ``graph.parse(..., format="xml")``, so the parse dispatches through the shim's
   parser-plugin lookup and yields a purrdf-backed graph.
"""

from __future__ import annotations

import json
from typing import Any

import _harness

_PACKAGE = "SPARQLWrapper"

_harness.require_installed(_PACKAGE)
_harness.require_shadow(_PACKAGE)


class _Response:
    """The minimal ``http.client.HTTPResponse``-shaped object QueryResult needs."""

    def __init__(self, data: bytes, content_type: str) -> None:
        self._data = data
        self._content_type = content_type

    def read(self) -> bytes:
        return self._data

    def info(self) -> dict[str, str]:
        return {"content-type": self._content_type}


def _main() -> None:
    import SPARQLWrapper
    from SPARQLWrapper import JSON, RDFXML
    from SPARQLWrapper.Wrapper import QueryResult

    # 1. SELECT results in SPARQL-Results JSON (SPARQLWrapper's own parser).
    select_doc = {
        "head": {"vars": ["s"]},
        "results": {
            "bindings": [
                {"s": {"type": "uri", "value": "http://example.org/alice"}}
            ]
        },
    }
    json_result = QueryResult(
        (
            _Response(
                json.dumps(select_doc).encode("utf-8"),
                "application/sparql-results+json",
            ),
            JSON,
        )  # type: ignore[arg-type]  # duck-typed response stands in for HTTPResponse
    )
    converted: Any = json_result.convert()
    binding = converted["results"]["bindings"][0]["s"]["value"]
    assert binding == "http://example.org/alice", binding

    # 2. CONSTRUCT results in RDF/XML — the rdflib-backed conversion path.
    rdf_xml = (
        '<?xml version="1.0"?>\n'
        '<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"'
        ' xmlns:ex="http://example.org/">\n'
        '  <rdf:Description rdf:about="http://example.org/alice">'
        "<ex:name>Alice</ex:name></rdf:Description>\n"
        "</rdf:RDF>\n"
    )
    rdf_result = QueryResult(
        (_Response(rdf_xml.encode("utf-8"), "application/rdf+xml"), RDFXML)  # type: ignore[arg-type]
    )
    graph: Any = rdf_result.convert()
    graph_type = f"{type(graph).__module__}.{type(graph).__name__}"
    assert type(graph).__module__.startswith("purrdf"), graph_type
    assert len(graph) == 1, len(graph)

    _harness.passed(
        _PACKAGE,
        version=SPARQLWrapper.__version__,
        graph_type=graph_type,
        detail=(
            "SPARQL-Results JSON SELECT parsed; RDF/XML CONSTRUCT parsed into a "
            "purrdf-backed ConjunctiveGraph via the shim parser-plugin lookup"
        ),
    )


try:
    _main()
except SystemExit:
    raise
except BaseException as exc:  # noqa: BLE001 - report any failure as a ledgered row
    _harness.failed(_PACKAGE, "result-conversion", exc)
