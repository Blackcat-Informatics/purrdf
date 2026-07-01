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
from typing import TYPE_CHECKING, Any

from .term import Identifier

if TYPE_CHECKING:
    from .graph import Graph


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
