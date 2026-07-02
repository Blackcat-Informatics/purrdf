# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""SPARQL result wrappers for the purrdf rdflib compat shim.

``ResultRow`` is a tuple-like SELECT row that also supports ``row["var"]`` /
``row.var`` access (RDFLib parity); ``Result`` is the iterable a
:meth:`Graph.query` call returns (rows for SELECT, triples for CONSTRUCT, a bool
for ASK).
"""

from __future__ import annotations

from collections.abc import Iterator
from pathlib import Path
from typing import IO, TYPE_CHECKING, Any

import purrdf

from .term import Identifier, from_native, to_native

if TYPE_CHECKING:
    from .graph import Graph

__all__ = [
    "Result",
    "ResultRow",
    "ResultException",
    "Processor",
    "UpdateProcessor",
    "ResultParser",
    "ResultSerializer",
]

#: RDFLib SPARQL-result format names / media types → the native short format id.
_RESULT_FORMAT_IDS: dict[str, str] = {
    "json": "json",
    "srj": "json",
    "application/sparql-results+json": "json",
    "xml": "xml",
    "srx": "xml",
    "application/sparql-results+xml": "xml",
    "csv": "csv",
    "text/csv": "csv",
    "tsv": "tsv",
    "text/tab-separated-values": "tsv",
}


def _result_format_id(fmt: str | None) -> str:
    """Resolve an RDFLib result format name / media type to a native id."""
    key = (fmt or "xml").lower()
    try:
        return _RESULT_FORMAT_IDS[key]
    except KeyError:
        raise ResultException(f"unsupported SPARQL result format: {fmt!r}") from None


def _serialize_result_bytes(result: Result, fmt_id: str) -> bytes:
    """Serialize a SELECT/ASK :class:`Result` to SPARQL-Results bytes via the native codec."""
    if result.type == "ASK":
        out = purrdf.serialize_sparql_boolean(fmt_id, bool(result.askAnswer))
        assert isinstance(out, bytes)
        return out
    if result.type == "SELECT":
        rows: list[list[Any]] = [
            [None if cell is None else to_native(cell) for cell in row]
            for row in result._rows
        ]
        out = purrdf.serialize_sparql_solutions(fmt_id, list(result.vars), rows)
        assert isinstance(out, bytes)
        return out
    raise ResultException(
        f"cannot serialize a {result.type} result as SPARQL results "
        "(CONSTRUCT/DESCRIBE serialize as an RDF graph)"
    )


def _read_result_source(source: Any) -> bytes:
    """Read a SPARQL-results document source (file-like / bytes / str / path) to bytes."""
    if source is None:
        raise ResultException("Result.parse requires a source")
    reader = getattr(source, "read", None)
    if callable(reader):
        raw = reader()
        return raw.encode("utf-8") if isinstance(raw, str) else raw
    if isinstance(source, bytes):
        return source
    if isinstance(source, str):
        return source.encode("utf-8")
    if isinstance(source, Path):
        return source.read_bytes()
    raise ResultException(f"unsupported Result.parse source: {source!r}")


class ResultException(Exception):  # noqa: N818 - RDFLib API name
    """Raised for malformed / unsupported SPARQL results (RDFLib parity)."""


class ResultRow(tuple[Identifier | None, ...]):
    """A SELECT solution row: positional ``row[0]`` and named ``row["var"]``."""

    _vars: tuple[str, ...]

    def __new__(
        cls, values: tuple[Identifier | None, ...], variables: tuple[str, ...]
    ) -> ResultRow:
        """Construct from the projected values and their variable names."""
        self = super().__new__(cls, values)
        self._vars = variables
        return self

    @property
    def labels(self) -> dict[str, int]:
        """Map each variable name to its positional index (RDFLib parity)."""
        return {name: idx for idx, name in enumerate(self._vars)}

    def __getitem__(self, key: int | str | slice) -> Any:  # type: ignore[override]
        """Index by position (``int``/``slice``) or by variable name (``str``)."""
        if isinstance(key, str):
            try:
                idx = self._vars.index(key)
            except ValueError as exc:
                raise KeyError(key) from exc
            return tuple.__getitem__(self, idx)
        return tuple.__getitem__(self, key)

    def __getattr__(self, name: str) -> Identifier | None:
        """Return the binding for variable ``name`` (RDFLib ``row.var`` access)."""
        if name.startswith("__"):
            raise AttributeError(name)
        try:
            idx = self._vars.index(name)
        except ValueError as exc:
            raise AttributeError(name) from exc
        return tuple.__getitem__(self, idx)

    def get(self, name: str, default: Identifier | None = None) -> Identifier | None:
        """Return the binding for ``name`` or ``default`` if absent/unbound."""
        try:
            idx = self._vars.index(name)
        except ValueError:
            return default
        value = tuple.__getitem__(self, idx)
        return value if value is not None else default


class Result:
    """The iterable returned by :meth:`Graph.query` for any SPARQL form."""

    def __init__(
        self,
        type_: str,
        *,
        rows: list[ResultRow] | None = None,
        variables: tuple[str, ...] | None = None,
        graph: Graph | None = None,
        ask: bool | None = None,
    ) -> None:
        """Build a SELECT (``rows``), CONSTRUCT/DESCRIBE (``graph``), or ASK result."""
        self.type = type_
        self._rows = rows or []
        self.vars = list(variables) if variables is not None else []
        self.graph = graph
        self.askAnswer = ask

    def __iter__(self) -> Iterator[Any]:
        """Iterate SELECT rows, CONSTRUCT triples, or yield the ASK boolean once.

        Yields ``Any`` — matching RDFLib's duck-typed query results — so callers
        can use ``row["var"]`` / ``row.var`` / triple-unpacking without casts.
        """
        if self.type == "ASK":
            yield bool(self.askAnswer)
        elif self.type == "CONSTRUCT" or self.type == "DESCRIBE":
            assert self.graph is not None
            yield from self.graph
        else:
            yield from self._rows

    def __len__(self) -> int:
        """Return the SELECT row count (or constructed-triple count)."""
        if self.type in ("CONSTRUCT", "DESCRIBE"):
            assert self.graph is not None
            return len(self.graph)
        if self.type == "ASK":
            return 1
        return len(self._rows)

    def __bool__(self) -> bool:
        """Return the ASK boolean, or whether the result set is non-empty."""
        if self.type == "ASK":
            return bool(self.askAnswer)
        return len(self) > 0

    def serialize(
        self,
        destination: str | Path | IO[bytes] | None = None,
        encoding: str | None = None,
        format: str | None = None,
        **args: Any,
    ) -> bytes | str | None:
        """Serialize the result (RDFLib ``Result.serialize`` parity).

        SELECT/ASK results emit a W3C SPARQL Results document (JSON/XML/CSV/TSV)
        via the native codec; CONSTRUCT/DESCRIBE results serialize as an RDF graph
        (delegating to :meth:`Graph.serialize`). With no ``destination`` the bytes
        are returned (a ``str`` when ``encoding`` is ``None``); otherwise they are
        written to the file-like / path destination.
        """
        if self.type in ("CONSTRUCT", "DESCRIBE"):
            assert self.graph is not None
            return self.graph.serialize(
                destination, format=format or "xml", encoding=encoding, **args
            )
        data = _serialize_result_bytes(self, _result_format_id(format))
        if destination is None:
            return data if encoding is not None else data.decode("utf-8")
        writer = getattr(destination, "write", None)
        if callable(writer):
            writer(data)
        elif isinstance(destination, str | Path):
            Path(destination).write_bytes(data)
        else:
            raise TypeError(f"unsupported serialize destination: {destination!r}")
        return None

    @staticmethod
    def parse(
        source: Any = None,
        format: str | None = None,
        content_type: str | None = None,
        **kwargs: Any,
    ) -> Result:
        """Parse a SPARQL Results document into a :class:`Result` (JSON/XML).

        Mirrors RDFLib's ``Result.parse`` static method: reads a SELECT or ASK
        result document (from a file-like, ``bytes``, ``str``, or path) in the
        given ``format`` (or ``content_type`` media type) and returns the
        reconstructed :class:`Result`.
        """
        fmt_id = _result_format_id(format or content_type)
        data = _read_result_source(source)
        parsed = purrdf.parse_sparql_results(fmt_id, data)
        kind = parsed[0]
        if kind == "ASK":
            return Result("ASK", ask=bool(parsed[1]))
        variables = tuple(parsed[1])
        rows = [
            ResultRow(tuple(from_native(cell) for cell in row), variables)
            for row in parsed[2]
        ]
        return Result("SELECT", rows=rows, variables=variables)


# ── plugin *kind* base classes (RDFLib ``rdflib.query`` hierarchy) ───────────────
#
# These give ``plugin.get(name, kind)`` the same *kind* identities RDFLib exposes
# from ``rdflib.query`` (``Processor``/``UpdateProcessor``/``ResultParser``/
# ``ResultSerializer``), plus ``Result`` above as the query-result kind. Concrete
# implementations live under ``purrdf.compat.rdflib.plugins``.


class Processor:
    """Base class for a SPARQL query *processor* kind (RDFLib parity)."""

    def __init__(self, graph: Graph) -> None:
        """Bind the processor to the graph it queries."""
        self.graph = graph

    def query(self, strOrQuery: object, **kwargs: Any) -> Any:  # noqa: N803
        """Execute a query (overridden by concrete processors)."""
        raise NotImplementedError


class UpdateProcessor:
    """Base class for a SPARQL *update* processor kind (RDFLib parity)."""

    def __init__(self, graph: Graph) -> None:
        """Bind the processor to the graph it updates."""
        self.graph = graph

    def update(self, strOrQuery: object, **kwargs: Any) -> None:  # noqa: N803
        """Execute an update (overridden by concrete processors)."""
        raise NotImplementedError


class ResultParser:
    """Base class for a SPARQL-results *parser* kind (RDFLib parity)."""

    def parse(self, source: Any, **kwargs: Any) -> Result:
        """Parse a SPARQL result document (overridden by concrete parsers)."""
        raise NotImplementedError


class ResultSerializer:
    """Base class for a SPARQL-results *serializer* kind (RDFLib parity)."""

    def __init__(self, result: Result) -> None:
        """Bind the serializer to the result it emits."""
        self.result = result

    def serialize(self, stream: IO[Any], encoding: str | None = None, **kwargs: Any) -> None:
        """Serialize a SPARQL result (overridden by concrete serializers)."""
        raise NotImplementedError
