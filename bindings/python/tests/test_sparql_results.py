# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""SPARQL Results serialization / parsing for the compat shim.

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


# ── registry wiring (the reserved slots now serve real codecs) ────────────────────


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


# ── registered-but-deferred plugin implementations ────────────────────────────────


def test_csv_result_parser_variables_and_bindings(compat: ModuleType) -> None:
    """CSV parsing reconstructs variable names, IRIs, literals, and unbound slots."""
    from purrdf.compat.rdflib import plugin
    from purrdf.compat.rdflib.query import ResultParser

    data = (
        "s,o\n"
        "http://example.org/s1,alpha\n"
        "http://example.org/s2,42\n"
        ",unbound\n"
    )
    cls = plugin.get("csv", ResultParser)
    result = cls().parse(io.BytesIO(data.encode("utf-8")))
    assert result.type == "SELECT"
    assert [str(v) for v in result.vars] == ["s", "o"]
    rows = list(result)
    assert len(rows) == 3
    assert rows[0][0] == compat.URIRef("http://example.org/s1")
    assert rows[0][1] == compat.Literal("alpha")
    assert rows[1][0] == compat.URIRef("http://example.org/s2")
    assert rows[1][1] == compat.Literal("42")
    assert rows[1][1].datatype is None
    assert rows[2][0] is None
    assert rows[2][1] == compat.Literal("unbound")


def test_tsv_result_parser_preserves_terms(compat: ModuleType) -> None:
    """TSV parsing preserves IRIs, typed literals, language tags, blank nodes, and unbound."""
    from purrdf.compat.rdflib import plugin
    from purrdf.compat.rdflib.query import ResultParser

    data = (
        "?s\t?o\n"
        "<http://example.org/s1>\t\"alpha\"\n"
        "<http://example.org/s2>\t\"42\"^^<http://www.w3.org/2001/XMLSchema#integer>\n"
        "<http://example.org/s3>\t\"bonjour\"@fr\n"
        "_:b0\t\n"
    )
    cls = plugin.get("tsv", ResultParser)
    result = cls().parse(io.BytesIO(data.encode("utf-8")))
    assert result.type == "SELECT"
    assert [str(v) for v in result.vars] == ["s", "o"]
    rows = {str(row[0]): row[1] for row in result}
    assert rows["http://example.org/s1"] == compat.Literal("alpha")
    assert rows["http://example.org/s2"].datatype == compat.URIRef(
        "http://www.w3.org/2001/XMLSchema#integer"
    )
    assert rows["http://example.org/s3"].language == "fr"
    # The blank-node row has an unbound object.
    blank_row = [row for row in result if isinstance(row[0], compat.BNode)][0]
    assert blank_row[1] is None


def test_txt_result_serializer_emits_table(compat: ModuleType) -> None:
    """The txt serializer emits a non-empty text table with headers and rows."""
    from purrdf.compat.rdflib import plugin
    from purrdf.compat.rdflib.query import ResultSerializer

    result = _select(compat)
    cls = plugin.get("txt", ResultSerializer)
    buf = io.BytesIO()
    cls(result).serialize(buf)
    text = buf.getvalue().decode("utf-8")
    assert "?s" in text
    assert "?o" in text
    assert "alpha" in text
    assert "42" in text
    assert "\n" in text


def test_graph_result_parser_parses_turtle(compat: ModuleType) -> None:
    """A graph parser registered as a ResultParser loads a CONSTRUCT graph."""
    from purrdf.compat.rdflib import plugin
    from purrdf.compat.rdflib.query import ResultParser

    turtle = (
        "@prefix ex: <http://example.org/> .\n"
        "ex:s ex:p ex:o .\n"
    )
    cls = plugin.get("turtle", ResultParser)
    result = cls().parse(
        io.BytesIO(turtle.encode("utf-8")), content_type="turtle"
    )
    assert result.type == "CONSTRUCT"
    assert (
        compat.URIRef("http://example.org/s"),
        compat.URIRef("http://example.org/p"),
        compat.URIRef("http://example.org/o"),
    ) in result


def test_graph_result_parser_accepts_positional_content_type(
    compat: ModuleType,
) -> None:
    """``GraphResultParser.parse`` accepts ``content_type`` positionally (rdflib parity)."""
    from purrdf.compat.rdflib import plugin
    from purrdf.compat.rdflib.query import ResultParser

    nt = "<http://example.org/s> <http://example.org/p> <http://example.org/o> .\n"
    cls = plugin.get("nt", ResultParser)
    result = cls().parse(io.BytesIO(nt.encode("utf-8")), "nt")
    assert result.type == "CONSTRUCT"
    assert len(result) == 1


# ── differential interop for the newly implemented codecs ─────────────────────────


def test_oracle_parses_purrdf_csv_output(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """The real rdflib parses a purrdf-emitted SPARQL Results CSV document."""
    data = _select(compat).serialize(format="csv", encoding="utf-8")
    oracle_result = oracle.query.Result.parse(io.BytesIO(data), format="csv")
    rows = sorted((str(row[0]), str(row[1])) for row in oracle_result)
    assert rows == [
        (f"{EX}s1", "alpha"),
        (f"{EX}s2", "42"),
        (f"{EX}s3", "bonjour"),
    ]


def test_purrdf_parses_oracle_csv_output(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """purrdf's CSV plugin parses a real-rdflib-emitted CSV result document."""
    from purrdf.compat.rdflib import plugin
    from purrdf.compat.rdflib.query import ResultParser

    oracle_result = _select(oracle)
    data = oracle_result.serialize(format="csv", encoding="utf-8")
    cls = plugin.get("csv", ResultParser)
    parsed = cls().parse(io.BytesIO(data))
    rows = sorted((str(row[0]), str(row[1])) for row in parsed)
    assert rows == [
        (f"{EX}s1", "alpha"),
        (f"{EX}s2", "42"),
        (f"{EX}s3", "bonjour"),
    ]


def test_oracle_parses_purrdf_tsv_output(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """The real rdflib parses a purrdf-emitted SPARQL Results TSV document."""
    data = _select(compat).serialize(format="tsv", encoding="utf-8")
    oracle_result = oracle.query.Result.parse(io.BytesIO(data), format="tsv")
    rows = sorted((str(row[0]), str(row[1])) for row in oracle_result)
    assert rows == [
        (f"{EX}s1", "alpha"),
        (f"{EX}s2", "42"),
        (f"{EX}s3", "bonjour"),
    ]
