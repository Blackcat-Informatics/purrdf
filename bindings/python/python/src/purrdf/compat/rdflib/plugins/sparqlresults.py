# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""SPARQL-results serializer / parser plugins for the compat registry.

The SPARQL-results codecs now land behind the registry slots reserved earlier.
The serializers (JSON / XML / CSV / TSV) route through the native
``purrdf-sparql-results`` crate (byte-deterministic); the parsers (JSON / XML)
route through the native readers. Each resolves via
``plugin.get(name, ResultSerializer)`` / ``plugin.get(name, ResultParser)``.

CSV/TSV *parsing* has no native reader, so it is implemented here with the
Python standard library. The plaintext ``txt`` table serializer is likewise
implemented locally (it is not a W3C SPARQL-results format).

``GraphResultParser`` mirrors RDFLib: a *graph* parser name (turtle, xml, …) is
also a ``ResultParser`` so a CONSTRUCT/DESCRIBE result document can be parsed
back into a graph; that path delegates to :meth:`Graph.parse` in the compat
facade.
"""

from __future__ import annotations

import csv
import io
from typing import IO, Any

from ..query import (
    Result,
    ResultException,
    ResultParser,
    ResultRow,
    ResultSerializer,
    Variable,
    _read_result_source,
)
from ..term import BNode, Literal, URIRef, from_native


class _NativeResultSerializer(ResultSerializer):
    """A SPARQL-results serializer backed by the native codec (JSON/XML/CSV/TSV)."""

    format_id = "?"

    def serialize(
        self, stream: IO[Any], encoding: str | None = None, **kwargs: Any
    ) -> None:
        """Write the bound result's serialized document to ``stream`` (bytes)."""
        from ..query import _serialize_result_bytes

        stream.write(_serialize_result_bytes(self.result, self.format_id))


class _NativeResultParser(ResultParser):
    """A SPARQL-results parser backed by the native reader (JSON/XML)."""

    format_id = "?"

    def parse(self, source: Any, **kwargs: Any) -> Result:
        """Parse ``source`` into a :class:`Result` via the native reader."""
        return Result.parse(source, format=self.format_id)


class JSONResultSerializer(_NativeResultSerializer):
    """SPARQL Results JSON serializer (native)."""

    format_id = "json"


class XMLResultSerializer(_NativeResultSerializer):
    """SPARQL Results XML serializer (native)."""

    format_id = "xml"


class CSVResultSerializer(_NativeResultSerializer):
    """SPARQL Results CSV serializer (native)."""

    format_id = "csv"


class TSVResultSerializer(_NativeResultSerializer):
    """SPARQL Results TSV serializer (native)."""

    format_id = "tsv"


class JSONResultParser(_NativeResultParser):
    """SPARQL Results JSON parser (native)."""

    format_id = "json"


class XMLResultParser(_NativeResultParser):
    """SPARQL Results XML parser (native)."""

    format_id = "xml"


def _read_source_text(source: Any) -> str:
    """Read a SPARQL-results document source into a decoded string."""
    raw = _read_result_source(source)
    return raw.decode("utf-8-sig")


def _parse_csv_term(cell: str) -> Identifier | None:
    """Turn one CSV result cell into an RDF term (or ``None`` if unbound)."""
    if cell == "":
        return None
    if cell.startswith("_:"):
        return BNode(cell[2:])
    if cell.startswith(("http://", "https://")):
        return URIRef(cell)
    return Literal(cell)


class CSVResultParser(ResultParser):
    """SPARQL Results CSV parser (Python stdlib)."""

    def parse(
        self, source: Any, content_type: str | None = None, **kwargs: Any
    ) -> Result:
        """Parse a SPARQL Results CSV document into a :class:`Result`."""
        text = _read_source_text(source)
        reader = csv.reader(io.StringIO(text))
        try:
            header = next(reader)
        except StopIteration:
            return Result("SELECT", rows=[], variables=())

        variables = tuple(Variable(name) for name in header)
        rows: list[ResultRow] = []
        var_names = tuple(str(v) for v in variables)
        for cells in reader:
            values = [_parse_csv_term(cell) for cell in cells]
            if len(values) < len(variables):
                values.extend([None] * (len(variables) - len(values)))
            rows.append(ResultRow(tuple(values[: len(variables)]), var_names))

        return Result("SELECT", rows=rows, variables=var_names)


def _parse_tsv_term(cell: str) -> Identifier | None:
    """Parse one TSV result cell using the native N-Triples term parser."""
    if cell == "":
        return None
    import purrdf

    nt = f"<urn:purrdf:tsv> <urn:purrdf:tsv> {cell} .\n".encode("utf-8")
    try:
        quads = purrdf.parse(nt, format=purrdf.RdfFormat.N_TRIPLES)
    except Exception as exc:
        raise ResultException(f"invalid TSV result term: {cell!r}") from exc
    if not quads:
        raise ResultException(f"invalid TSV result term: {cell!r}")
    term = from_native(quads[0].object)
    assert term is not None
    return term


class TSVResultParser(ResultParser):
    """SPARQL Results TSV parser (Python stdlib + native term parsing)."""

    def parse(
        self, source: Any, content_type: str | None = None, **kwargs: Any
    ) -> Result:
        """Parse a SPARQL Results TSV document into a :class:`Result`."""
        text = _read_source_text(source)
        lines = text.splitlines()
        if not lines:
            return Result("SELECT", rows=[], variables=())

        header = lines[0].split("\t")
        variables = tuple(Variable(name.lstrip("?")) for name in header)
        var_names = tuple(str(v) for v in variables)
        rows: list[ResultRow] = []
        for line in lines[1:]:
            if line == "":
                continue
            cells = line.split("\t")
            values = [_parse_tsv_term(cell) for cell in cells]
            if len(values) < len(variables):
                values.extend([None] * (len(variables) - len(values)))
            rows.append(ResultRow(tuple(values[: len(variables)]), var_names))

        return Result("SELECT", rows=rows, variables=var_names)


class TXTResultSerializer(ResultSerializer):
    """SPARQL Results plaintext table serializer (not a W3C format)."""

    def serialize(
        self, stream: IO[Any], encoding: str | None = None, **kwargs: Any
    ) -> None:
        """Write a simple aligned text table of the SELECT result to ``stream``."""
        if self.result.type != "SELECT":
            raise ResultException("TXT serializer only supports SELECT results")

        variables = [str(v) for v in self.result.vars]
        headers = [f"?{v}" for v in variables]
        rows = [
            ["" if cell is None else str(cell) for cell in row]
            for row in self.result._rows
        ]

        widths = [
            max(
                len(headers[idx]),
                max((len(row[idx]) for row in rows), default=0),
            )
            for idx in range(len(variables))
        ]

        separator = " | "

        def fmt(values: list[str]) -> str:
            return separator.join(
                value.ljust(widths[idx]) for idx, value in enumerate(values)
            )

        lines: list[str] = []
        if widths:
            lines.append(fmt(headers))
            lines.append(
                "-" * (sum(widths) + len(separator) * (len(widths) - 1))
            )
        lines.extend(fmt(row) for row in rows)
        text = "\n".join(lines) + "\n"

        try:
            stream.write(text.encode("utf-8"))
        except (TypeError, ValueError):
            stream.write(text)


class GraphResultParser(ResultParser):
    """Parse a CONSTRUCT/DESCRIBE result document into a graph."""

    def parse(
        self, source: Any, content_type: str | None = None, **kwargs: Any
    ) -> Result:
        """Parse ``source`` as an RDF graph and wrap it in a CONSTRUCT :class:`Result`."""
        from ..graph import Graph

        graph = kwargs.get("graph") or Graph()
        graph.parse(source, format=content_type)
        return Result("CONSTRUCT", graph=graph)
