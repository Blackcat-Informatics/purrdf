# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""SPARQL processor / result plugin classes for the purrdf compat registry.

RDFLib registers ``("sparql", Processor)``, ``("sparql", UpdateProcessor)`` and
``("sparql", Result)`` so ``Graph.query``/``Graph.update`` can resolve an engine
by name. The purrdf shim executes SPARQL natively inside :meth:`Graph.query`, so
these classes simply delegate back to the graph — they exist so the registry
lookups resolve to a working implementation.
"""

from __future__ import annotations

from typing import Any

from ..query import Processor, Result, UpdateProcessor


class SPARQLProcessor(Processor):
    """A SPARQL query processor that delegates to the native graph engine."""

    def query(  # noqa: N803 - RDFLib API names
        self,
        strOrQuery: object,
        initBindings: dict[str, Any] | None = None,
        initNs: dict[str, object] | None = None,
        base: str | None = None,
        **kwargs: Any,
    ) -> Result:
        """Run ``strOrQuery`` against the bound graph (native execution)."""
        result = self.graph.query(
            str(strOrQuery),
            initBindings=initBindings,
            initNs=initNs,
            base=base,
            **kwargs,
        )
        if not isinstance(result, Result):
            raise TypeError(
                f"Graph.query returned {type(result).__name__}, expected Result"
            )
        return result


class SPARQLUpdateProcessor(UpdateProcessor):
    """A SPARQL update processor that delegates to the native graph engine."""

    def update(  # noqa: N803 - RDFLib API names
        self,
        strOrQuery: object,
        initBindings: dict[str, Any] | None = None,
        initNs: dict[str, object] | None = None,
        **kwargs: Any,
    ) -> None:
        """Run ``strOrQuery`` as an update against the bound graph."""
        self.graph.update(
            str(strOrQuery), initBindings=initBindings, initNs=initNs, **kwargs
        )


class SPARQLResult(Result):
    """The SPARQL query-result *kind* (RDFLib registers ``("sparql", Result)``).

    :meth:`Graph.query` already returns a :class:`~..query.Result`; this subclass
    exists so the ``("sparql", Result)`` registry lookup resolves to a concrete
    class, mirroring RDFLib's ``SPARQLResult``.
    """
