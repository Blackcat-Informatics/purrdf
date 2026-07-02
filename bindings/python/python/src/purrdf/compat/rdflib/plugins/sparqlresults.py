# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""SPARQL-results serializer / parser plugin slots for the compat registry (#7/#11).

The SPARQL Results formats (JSON / XML / CSV / TSV) are the responsibility of
Task 6; this module provides the *registry slots* now so that
``plugin.get(name, ResultSerializer)`` / ``plugin.get(name, ResultParser)``
resolve for every result format a drop-in caller (SPARQLWrapper et al.) expects.

The implementations raise ``NotImplementedError`` until Task 6 lands the actual
codecs — each is covered by a strict-xfail ledger entry (#6/#7/#11), so the
lookup resolving is a hard guarantee while the serialization is explicitly
deferred rather than silently missing.

``GraphResultParser`` mirrors RDFLib: any *graph* parser name (turtle, xml, …) is
also registered as a ``ResultParser`` so a CONSTRUCT/DESCRIBE result document can
be parsed back into a graph. It is likewise deferred to Task 6.
"""

from __future__ import annotations

from typing import IO, Any

from ..query import Result, ResultParser, ResultSerializer

_DEFERRED = (
    "SPARQL Results {name} {kind} is deferred to purrdf compat Task 6 "
    "(#6/#7/#11); the plugin registry slot resolves but the codec is not "
    "yet implemented"
)


class _DeferredResultSerializer(ResultSerializer):
    """A registered-but-deferred SPARQL-results serializer (Task 6)."""

    format_name = "?"

    def serialize(
        self, stream: IO[Any], encoding: str | None = None, **kwargs: Any
    ) -> None:
        """Raise ``NotImplementedError`` — deferred to Task 6."""
        raise NotImplementedError(
            _DEFERRED.format(name=self.format_name, kind="serialization")
        )


class _DeferredResultParser(ResultParser):
    """A registered-but-deferred SPARQL-results parser (Task 6)."""

    format_name = "?"

    def parse(self, source: Any, **kwargs: Any) -> Result:
        """Raise ``NotImplementedError`` — deferred to Task 6."""
        raise NotImplementedError(
            _DEFERRED.format(name=self.format_name, kind="parsing")
        )


class JSONResultSerializer(_DeferredResultSerializer):
    """SPARQL Results JSON serializer slot (deferred, Task 6)."""

    format_name = "JSON"


class XMLResultSerializer(_DeferredResultSerializer):
    """SPARQL Results XML serializer slot (deferred, Task 6)."""

    format_name = "XML"


class CSVResultSerializer(_DeferredResultSerializer):
    """SPARQL Results CSV serializer slot (deferred, Task 6)."""

    format_name = "CSV"


class TXTResultSerializer(_DeferredResultSerializer):
    """SPARQL Results plaintext table serializer slot (deferred, Task 6)."""

    format_name = "TXT"


class JSONResultParser(_DeferredResultParser):
    """SPARQL Results JSON parser slot (deferred, Task 6)."""

    format_name = "JSON"


class XMLResultParser(_DeferredResultParser):
    """SPARQL Results XML parser slot (deferred, Task 6)."""

    format_name = "XML"


class CSVResultParser(_DeferredResultParser):
    """SPARQL Results CSV parser slot (deferred, Task 6)."""

    format_name = "CSV"


class TSVResultParser(_DeferredResultParser):
    """SPARQL Results TSV parser slot (deferred, Task 6)."""

    format_name = "TSV"


class GraphResultParser(_DeferredResultParser):
    """Parse a CONSTRUCT/DESCRIBE result document into a graph (deferred, Task 6)."""

    format_name = "graph"
