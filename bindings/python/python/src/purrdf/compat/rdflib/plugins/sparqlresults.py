# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""SPARQL-results serializer / parser plugins for the compat registry.

The SPARQL-results codecs now land behind the registry slots reserved earlier.
The serializers (JSON / XML / CSV / TSV) route through the native
``purrdf-sparql-results`` crate (byte-deterministic); the parsers (JSON / XML)
route through the native readers. Each resolves via
``plugin.get(name, ResultSerializer)`` / ``plugin.get(name, ResultParser)``.

CSV/TSV *parsing* has no native reader and stays deferred (the native crate is a
value-only writer for those formats); the plaintext ``txt`` table serializer is
likewise deferred (not a W3C SPARQL-results format).

``GraphResultParser`` mirrors RDFLib: a *graph* parser name (turtle, xml, …) is
also a ``ResultParser`` so a CONSTRUCT/DESCRIBE result document can be parsed
back into a graph; that path is served by :meth:`Graph.parse` in the compat
facade and remains a deferred stub here.
"""

from __future__ import annotations

from typing import IO, Any

from ..query import Result, ResultParser, ResultSerializer

_DEFERRED = (
    "SPARQL Results {name} {kind} is deferred in the purrdf compat shim: "
    "the plugin registry slot resolves but this codec is not "
    "implemented"
)


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


class _DeferredResultSerializer(ResultSerializer):
    """A registered-but-deferred SPARQL-results serializer."""

    format_name = "?"

    def serialize(
        self, stream: IO[Any], encoding: str | None = None, **kwargs: Any
    ) -> None:
        """Raise ``NotImplementedError`` — this codec is deferred."""
        raise NotImplementedError(
            _DEFERRED.format(name=self.format_name, kind="serialization")
        )


class _DeferredResultParser(ResultParser):
    """A registered-but-deferred SPARQL-results parser."""

    format_name = "?"

    def parse(self, source: Any, **kwargs: Any) -> Result:
        """Raise ``NotImplementedError`` — this codec is deferred."""
        raise NotImplementedError(
            _DEFERRED.format(name=self.format_name, kind="parsing")
        )


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


class TXTResultSerializer(_DeferredResultSerializer):
    """SPARQL Results plaintext table serializer (deferred — not a W3C format)."""

    format_name = "TXT"


class CSVResultParser(_DeferredResultParser):
    """SPARQL Results CSV parser (deferred — no native CSV reader)."""

    format_name = "CSV"


class TSVResultParser(_DeferredResultParser):
    """SPARQL Results TSV parser (deferred — no native TSV reader)."""

    format_name = "TSV"


class GraphResultParser(_DeferredResultParser):
    """Parse a CONSTRUCT/DESCRIBE result document into a graph (deferred)."""

    format_name = "graph"
