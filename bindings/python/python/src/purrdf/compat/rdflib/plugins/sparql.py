# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""RDFLib ``rdflib.plugins.sparql`` compat shim (purrdf P0).

``prepareQuery`` in RDFLib pre-parses a query for reuse. The native engine parses
on each call, so here it folds any ``initNs`` prefix bindings into the query text
and returns the (string) query — which :meth:`Graph.query` accepts directly.
"""

from __future__ import annotations


def prepareQuery(  # noqa: N802 - RDFLib API name
    query_string: str,
    initNs: dict[str, object] | None = None,  # noqa: N803 - RDFLib API name
    base: str | None = None,
) -> str:
    """Return a query string with ``initNs`` prefix declarations prepended."""
    if initNs:
        prefixes = "".join(f"PREFIX {p}: <{ns}>\n" for p, ns in initNs.items())
        return prefixes + query_string
    return query_string
