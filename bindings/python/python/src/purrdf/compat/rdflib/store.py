# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Store plugin *kind* base class (RDFLib ``rdflib.store``).

The purrdf compat shim keeps triples in a native COW dataset rather than an
RDFLib ``Store`` implementation, so this module exists to provide the *kind*
identity ``plugin.get(name, Store)`` resolves against (and to let
``from rdflib.store import Store`` succeed). It is intentionally thin: the shim
does not expose pluggable stores.
"""

from __future__ import annotations

__all__ = ["Store", "NO_STORE", "VALID_STORE", "CORRUPTED_STORE", "UNKNOWN"]

#: RDFLib ``Store.open`` status codes (kept for import-compatibility).
VALID_STORE = 1
CORRUPTED_STORE = 0
NO_STORE = -1
UNKNOWN: None = None


class Store:
    """Base class for an RDF store *kind* (the ``plugin.get`` key type)."""

    context_aware: bool = False
    formula_aware: bool = False
    transaction_aware: bool = False
    graph_aware: bool = False

    def __init__(
        self,
        configuration: str | None = None,
        identifier: object | None = None,
    ) -> None:
        """Record the (unused) configuration/identifier (RDFLib signature parity)."""
        self.identifier = identifier
