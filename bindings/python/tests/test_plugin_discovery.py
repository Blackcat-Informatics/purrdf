# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Plugin registry discovery + the drop-in acceptance matrix (issue #7, #11).

Acceptance matrix (derived FIRST, per #7's mandate)
===================================================

The purrdf compat shim must resolve, through ``rdflib.plugin.get(name, kind)``,
every ``(name, kind)`` lookup a real rdflib consumer performs. The three drop-in
targets called out by #7 are **pyshacl**, **SPARQLWrapper**, and **sssom**. Those
packages are not installed in this differential environment, so the required
lookups are derived from rdflib 7.6's own plugin surface (``rdflib/plugin.py``)
that each library drives, rather than by grepping their sources:

* **pyshacl** — loads/emits data + shapes graphs in Turtle, N-Triples, JSON-LD,
  and RDF/XML through ``Graph.parse``/``Graph.serialize`` (which resolve
  ``(fmt, Parser)`` / ``(fmt, Serializer)`` internally), guesses formats via
  ``rdflib.util.guess_format``, and runs SPARQL (``("sparql", Processor)`` /
  ``("sparql", Result)``). It never reaches for TriX/HexTuples.
* **SPARQLWrapper** — consumes SPARQL Results in JSON/XML/CSV/TSV and parses
  CONSTRUCT/DESCRIBE payloads back into graphs; a drop-in must expose the
  ``ResultSerializer`` / ``ResultParser`` slots for those media types plus the
  graph ``ResultParser`` fallback, and the ``("sparql", *)`` processors.
* **sssom** — round-trips mappings through Turtle / N-Triples / JSON-LD / RDF/XML
  ``Graph.serialize``/``parse`` and uses ``guess_format``; same triple/quad
  ``(fmt, Serializer/Parser)`` lookups as pyshacl.

The union of those lookups is pinned in the ``ACCEPTANCE_MATRIX`` tables below;
every entry must resolve (a concrete class), or — where the capability is
deferred (TriX/HexTuples emit; SPARQL Results codecs, which land in Task 6) — the
lookup must still resolve to a stub, with the *capability* ledgered as a strict
xfail (see ``xfail_ledger.toml``, #6/#7/#11). No silent skips.

Entry-point discovery
=====================
Third-party plugins registering against rdflib's ``rdf.plugins.*`` entry-point
groups must be discoverable through the shim; ``test_entry_point_group_discovery``
simulates a registered entry point and asserts it resolves through ``plugin.get``.
"""

from __future__ import annotations

import io
from importlib.metadata import EntryPoint
from types import ModuleType
from typing import Any

import pytest

from purrdf.compat.rdflib import plugin
from purrdf.compat.rdflib.parser import Parser
from purrdf.compat.rdflib.query import (
    Processor,
    Result,
    ResultParser,
    ResultSerializer,
    UpdateProcessor,
)
from purrdf.compat.rdflib.serializer import Serializer

EX = "http://example.org/"

# ── acceptance matrix: (name, kind) lookups that MUST resolve ─────────────────────

_SERIALIZER_NAMES = [
    "turtle",
    "ttl",
    "text/turtle",
    "longturtle",
    "n3",
    "text/n3",
    "nt",
    "ntriples",
    "nt11",
    "application/n-triples",
    "nquads",
    "nq",
    "application/n-quads",
    "trig",
    "application/trig",
    "json-ld",
    "jsonld",
    "application/ld+json",
    "xml",
    "application/rdf+xml",
    "pretty-xml",
    "trix",
    "application/trix",
    "hext",
]

_PARSER_NAMES = [
    "turtle",
    "ttl",
    "text/turtle",
    "n3",
    "text/n3",
    "longturtle",
    "nt",
    "ntriples",
    "nt11",
    "application/n-triples",
    "nquads",
    "nq",
    "application/n-quads",
    "trig",
    "application/trig",
    "json-ld",
    "jsonld",
    "application/ld+json",
    "xml",
    "application/rdf+xml",
    "pretty-xml",
    "trix",
    "application/trix",
    "hext",
]

_RESULT_SERIALIZER_NAMES = [
    "json",
    "application/sparql-results+json",
    "xml",
    "application/sparql-results+xml",
    "csv",
    "text/csv",
    "txt",
]

_RESULT_PARSER_NAMES = [
    "json",
    "application/sparql-results+json",
    "xml",
    "application/sparql-results+xml",
    "csv",
    "text/csv",
    "tsv",
    "text/tab-separated-values",
]

#: Supported triple/quad formats that must round-trip through the registry today.
_SUPPORTED_ROUNDTRIP = [
    "turtle",
    "nt",
    "nquads",
    "trig",
    "json-ld",
    "xml",
]


# ── the matrix resolves ──────────────────────────────────────────────────────────


@pytest.mark.parametrize("name", _SERIALIZER_NAMES)
def test_serializer_lookup_resolves(name: str) -> None:
    """Every built-in serializer name resolves to a Serializer subclass."""
    cls = plugin.get(name, Serializer)
    assert isinstance(cls, type) and issubclass(cls, Serializer)


@pytest.mark.parametrize("name", _PARSER_NAMES)
def test_parser_lookup_resolves(name: str) -> None:
    """Every built-in parser name resolves to a Parser subclass."""
    cls = plugin.get(name, Parser)
    assert isinstance(cls, type) and issubclass(cls, Parser)


@pytest.mark.parametrize("name", _RESULT_SERIALIZER_NAMES)
def test_result_serializer_lookup_resolves(name: str) -> None:
    """Every SPARQL-results serializer slot resolves (codec deferred to Task 6)."""
    cls = plugin.get(name, ResultSerializer)
    assert isinstance(cls, type) and issubclass(cls, ResultSerializer)


@pytest.mark.parametrize("name", _RESULT_PARSER_NAMES)
def test_result_parser_lookup_resolves(name: str) -> None:
    """Every SPARQL-results parser slot resolves (codec deferred to Task 6)."""
    cls = plugin.get(name, ResultParser)
    assert isinstance(cls, type) and issubclass(cls, ResultParser)


def test_graph_result_parser_fallback_resolves() -> None:
    """Graph parser names double as graph ResultParsers (rdflib parity)."""
    # e.g. a CONSTRUCT/DESCRIBE result document declared as ``turtle``.
    cls = plugin.get("turtle", ResultParser)
    assert issubclass(cls, ResultParser)


def test_sparql_processor_kinds_resolve() -> None:
    """The ``sparql`` processor / update / result kinds all resolve."""
    assert issubclass(plugin.get("sparql", Processor), Processor)
    assert issubclass(plugin.get("sparql", UpdateProcessor), UpdateProcessor)
    assert issubclass(plugin.get("sparql", Result), Result)


def test_unknown_plugin_raises_plugin_exception() -> None:
    """An unregistered ``(name, kind)`` raises PluginException (rdflib parity)."""
    with pytest.raises(plugin.PluginException):
        plugin.get("no-such-format", Serializer)


def test_plugins_iterator_filters_by_kind_and_name() -> None:
    """``plugins()`` filters by kind and by name (rdflib parity)."""
    serializers = list(plugin.plugins(kind=Serializer))
    assert serializers and all(p.kind is Serializer for p in serializers)
    named = list(plugin.plugins(name="turtle", kind=Serializer))
    assert len(named) == 1 and named[0].name == "turtle"


# ── the matrix dispatches (supported formats round-trip through the registry) ─────


@pytest.mark.parametrize("fmt", _SUPPORTED_ROUNDTRIP)
def test_supported_format_roundtrips_through_registry(
    compat: ModuleType, fmt: str
) -> None:
    """serialize/parse for a supported format dispatch through the registry."""
    g = compat.Graph()
    g.add((compat.URIRef(f"{EX}s"), compat.URIRef(f"{EX}p"), compat.Literal("v")))
    text = g.serialize(format=fmt)
    assert text
    g2 = compat.Graph()
    g2.parse(data=text, format=fmt)
    assert (
        compat.URIRef(f"{EX}s"),
        compat.URIRef(f"{EX}p"),
        compat.Literal("v"),
    ) in g2


# ── entry-point group discovery ──────────────────────────────────────────────────


class _FakeEntryPoints:
    """A stand-in for ``importlib.metadata.entry_points()`` results."""

    def __init__(self, eps: list[EntryPoint]) -> None:
        """Record the entry points to expose via ``select``."""
        self._eps = eps

    def select(self, group: str) -> list[EntryPoint]:
        """Return the entry points registered under ``group``."""
        return [ep for ep in self._eps if ep.group == group]


def test_entry_point_group_discovery(monkeypatch: pytest.MonkeyPatch) -> None:
    """A plugin registered against an ``rdf.plugins.*`` group is discoverable.

    Simulates a third-party package publishing a serializer entry point against
    rdflib's ``rdf.plugins.serializer`` group; after discovery the shim resolves
    it through ``plugin.get`` exactly as rdflib would.
    """
    ep = EntryPoint(
        name="x-fake-format",
        value="purrdf.compat.rdflib.plugins.serializers:NTSerializer",
        group="rdf.plugins.serializer",
    )
    monkeypatch.setattr(plugin, "entry_points", lambda: _FakeEntryPoints([ep]))
    try:
        plugin._discover_entry_points()
        resolved = plugin.get("x-fake-format", Serializer)
        from purrdf.compat.rdflib.plugins.serializers import NTSerializer

        assert resolved is NTSerializer
    finally:
        plugin._plugins.pop(("x-fake-format", Serializer), None)


def test_entry_point_groups_match_rdflib(oracle: ModuleType) -> None:
    """The shim honours exactly rdflib's ``rdf.plugins.*`` entry-point groups."""
    assert set(plugin.rdflib_entry_points) == set(oracle.plugin.rdflib_entry_points)


# ── deferred capabilities: lookups resolve, but the codec is ledgered (xfail) ─────
#
# These tests assert the *desired* capability (real emit/parse). They currently
# fail because the impl is a deferred stub, so they are strict-xfails in
# xfail_ledger.toml (#6/#7/#11). When the codec lands, the XPASS forces the ledger
# to shrink — the same discipline the Rust conformance harnesses use.


def _select_result(compat: ModuleType) -> Any:
    """Build a one-row SELECT result to feed a result serializer."""
    g = compat.Graph()
    g.add((compat.URIRef(f"{EX}s"), compat.URIRef(f"{EX}p"), compat.Literal("v")))
    return g.query(f"SELECT ?o WHERE {{ <{EX}s> <{EX}p> ?o }}")


def test_trix_serialization_supported(compat: ModuleType) -> None:
    """TriX serialization emits a TriX document (deferred; ledgered #7/#11)."""
    g = compat.Graph()
    g.add((compat.URIRef(f"{EX}s"), compat.URIRef(f"{EX}p"), compat.Literal("v")))
    out = g.serialize(format="trix")
    assert "TriX" in out


def test_hext_serialization_supported(compat: ModuleType) -> None:
    """HexTuples serialization emits a hextuples document (deferred; ledgered)."""
    g = compat.Graph()
    g.add((compat.URIRef(f"{EX}s"), compat.URIRef(f"{EX}p"), compat.Literal("v")))
    out = g.serialize(format="hext")
    assert out.strip().startswith("[")


def test_sparql_results_json_serialization_supported(compat: ModuleType) -> None:
    """SPARQL Results JSON serialization works (deferred to Task 6; ledgered)."""
    result = _select_result(compat)
    cls = plugin.get("json", ResultSerializer)
    buffer = io.BytesIO()
    cls(result).serialize(buffer)
    assert b'"bindings"' in buffer.getvalue()


def test_sparql_results_csv_serialization_supported(compat: ModuleType) -> None:
    """SPARQL Results CSV serialization works (deferred to Task 6; ledgered)."""
    result = _select_result(compat)
    cls = plugin.get("csv", ResultSerializer)
    buffer = io.BytesIO()
    cls(result).serialize(buffer)
    assert buffer.getvalue()


def test_sparql_results_xml_parsing_supported() -> None:
    """SPARQL Results XML parsing works (deferred to Task 6; ledgered)."""
    cls = plugin.get("xml", ResultParser)
    document = (
        b'<?xml version="1.0"?><sparql xmlns="http://www.w3.org/2005/sparql-results#">'
        b"<head><variable name='o'/></head><results></results></sparql>"
    )
    result = cls().parse(io.BytesIO(document))
    assert isinstance(result, Result)
