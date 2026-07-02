# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""SPARQL Results serialization / parsing for the compat shim (Task 6, #6/#11).

Covers the native SELECT/ASK result codecs behind ``Result.serialize`` /
``Result.parse`` (JSON / XML / CSV / TSV): byte-determinism against committed
goldens, round-trips through the native reader, and differential interop with
the *real* rdflib oracle (which can parse a purrdf-emitted SPARQL-JSON/XML
document, and whose emitted document purrdf can parse back).
"""

from __future__ import annotations

import io
from pathlib import Path
from types import ModuleType

import pytest

EX = "http://example.org/"
_GOLDENS = Path(__file__).parent / "goldens" / "sparql_results"


def _fixture_graph(mod: ModuleType) -> object:
    """Build the deterministic three-row graph the goldens were captured from."""
    g = mod.Graph()
    g.add((mod.URIRef(f"{EX}s1"), mod.URIRef(f"{EX}p"), mod.Literal("alpha")))
    g.add((mod.URIRef(f"{EX}s2"), mod.URIRef(f"{EX}p"), mod.Literal(42)))
    g.add((mod.URIRef(f"{EX}s3"), mod.URIRef(f"{EX}p"), mod.Literal("bonjour", lang="fr")))
    return g


def _select(mod: ModuleType) -> object:
    """Run the deterministic (ORDER BY) SELECT the goldens were captured from."""
    return _fixture_graph(mod).query(f"SELECT ?s ?o WHERE {{ ?s <{EX}p> ?o }} ORDER BY ?s")


def _ask(mod: ModuleType) -> object:
    """Run the ASK the goldens were captured from."""
    return _fixture_graph(mod).query(f"ASK {{ ?s <{EX}p> ?o }}")


# ── byte-determinism against committed goldens ────────────────────────────────────


@pytest.mark.parametrize(
    ("fmt", "golden"),
    [
        ("json", "select.srj"),
        ("xml", "select.srx"),
        ("csv", "select.csv"),
        ("tsv", "select.tsv"),
    ],
)
def test_select_serialization_matches_golden(
    compat: ModuleType, fmt: str, golden: str
) -> None:
    """A SELECT serializes to the committed golden bytes, deterministically."""
    result = _select(compat)
    first = result.serialize(format=fmt, encoding="utf-8")
    assert isinstance(first, bytes)
    assert first == (_GOLDENS / golden).read_bytes()
    # Determinism: a second (independent) serialization is byte-identical.
    assert _select(compat).serialize(format=fmt, encoding="utf-8") == first


@pytest.mark.parametrize(("fmt", "golden"), [("json", "ask.srj"), ("xml", "ask.srx")])
def test_ask_serialization_matches_golden(
    compat: ModuleType, fmt: str, golden: str
) -> None:
    """An ASK serializes to the committed golden bytes."""
    assert _ask(compat).serialize(format=fmt, encoding="utf-8") == (
        _GOLDENS / golden
    ).read_bytes()


def test_serialize_str_vs_bytes(compat: ModuleType) -> None:
    """No encoding → ``str``; an encoding → ``bytes`` (RDFLib parity)."""
    result = _select(compat)
    as_str = result.serialize(format="json")
    as_bytes = _select(compat).serialize(format="json", encoding="utf-8")
    assert isinstance(as_str, str)
    assert isinstance(as_bytes, bytes)
    assert as_str.encode("utf-8") == as_bytes


# ── native round-trip (serialize → parse) ─────────────────────────────────────────


@pytest.mark.parametrize("fmt", ["json", "xml"])
def test_select_native_roundtrip(compat: ModuleType, fmt: str) -> None:
    """serialize → parse reconstructs the SELECT variables and rows."""
    data = _select(compat).serialize(format=fmt, encoding="utf-8")
    parsed = compat.Result.parse(io.BytesIO(data), format=fmt)
    assert parsed.type == "SELECT"
    assert list(parsed.vars) == ["s", "o"]
    rows = sorted((str(row[0]), str(row[1])) for row in parsed)
    assert rows == [
        (f"{EX}s1", "alpha"),
        (f"{EX}s2", "42"),
        (f"{EX}s3", "bonjour"),
    ]


@pytest.mark.parametrize("fmt", ["json", "xml"])
def test_ask_native_roundtrip(compat: ModuleType, fmt: str) -> None:
    """serialize → parse reconstructs the ASK boolean."""
    data = _ask(compat).serialize(format=fmt, encoding="utf-8")
    parsed = compat.Result.parse(io.BytesIO(data), format=fmt)
    assert parsed.type == "ASK"
    assert bool(parsed) is True


def test_parse_preserves_datatype_and_language(compat: ModuleType) -> None:
    """A typed / language-tagged literal survives the JSON round-trip."""
    data = _select(compat).serialize(format="json", encoding="utf-8")
    parsed = compat.Result.parse(io.BytesIO(data), format="json")
    by_subject = {str(row[0]): row[1] for row in parsed}
    assert by_subject[f"{EX}s2"].datatype == compat.URIRef(
        "http://www.w3.org/2001/XMLSchema#integer"
    )
    assert by_subject[f"{EX}s3"].language == "fr"


# ── differential interop with the real rdflib oracle ──────────────────────────────


@pytest.mark.parametrize("fmt", ["json", "xml"])
def test_oracle_parses_purrdf_output(
    compat: ModuleType, oracle: ModuleType, fmt: str
) -> None:
    """The real rdflib parses a purrdf-emitted SPARQL Results document."""
    data = _select(compat).serialize(format=fmt, encoding="utf-8")
    oracle_result = oracle.query.Result.parse(io.BytesIO(data), format=fmt)
    rows = sorted((str(row[0]), str(row[1])) for row in oracle_result)
    assert rows == [
        (f"{EX}s1", "alpha"),
        (f"{EX}s2", "42"),
        (f"{EX}s3", "bonjour"),
    ]


@pytest.mark.parametrize("fmt", ["json", "xml"])
def test_purrdf_parses_oracle_output(
    compat: ModuleType, oracle: ModuleType, fmt: str
) -> None:
    """purrdf parses a real-rdflib-emitted SPARQL Results document."""
    oracle_result = _select(oracle)
    data = oracle_result.serialize(format=fmt, encoding="utf-8")
    parsed = compat.Result.parse(io.BytesIO(data), format=fmt)
    rows = sorted((str(row[0]), str(row[1])) for row in parsed)
    assert rows == [
        (f"{EX}s1", "alpha"),
        (f"{EX}s2", "42"),
        (f"{EX}s3", "bonjour"),
    ]


# ── registry wiring (the Task 5 slots now serve real codecs) ──────────────────────


def test_registry_serializer_slots_emit(compat: ModuleType) -> None:
    """``plugin.get(name, ResultSerializer)`` emits a real document (not a stub)."""
    from purrdf.compat.rdflib import plugin
    from purrdf.compat.rdflib.query import ResultParser, ResultSerializer

    result = _select(compat)
    for name, needle in (("json", b'"bindings"'), ("csv", b"s,o"), ("tsv", b"?s")):
        cls = plugin.get(name, ResultSerializer)
        buf = io.BytesIO()
        cls(result).serialize(buf)
        assert needle in buf.getvalue()
    # XML parser slot resolves and parses.
    xml_cls = plugin.get("xml", ResultParser)
    parsed = xml_cls().parse(
        io.BytesIO(_select(compat).serialize(format="xml", encoding="utf-8"))
    )
    assert parsed.type == "SELECT"
