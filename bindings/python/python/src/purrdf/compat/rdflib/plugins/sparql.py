# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""RDFLib ``rdflib.plugins.sparql`` compat shim (purrdf P0).

``prepareQuery`` in RDFLib pre-parses a query for reuse. The native engine parses
on each call, so here it folds any ``initNs`` prefix bindings into the query text
and returns the (string) query — which :meth:`Graph.query` accepts directly.

``register_custom_function`` / ``CUSTOM_EVALS`` mirror RDFLib's extension-function
registry surface (``rdflib.plugins.sparql.operators`` / ``rdflib.plugins.sparql``).
RDFLib evaluates SPARQL in Python, so a registered Python callable is invoked
during evaluation; the purrdf engine evaluates natively in Rust and cannot call
back into an arbitrary Python function, so the registry is recorded for API
compatibility but a registered function is **not** invoked at query time. That
runtime gap is ledgered as a strict xfail (#10). The native engine's own closed
extension-function set is enabled through the ``extension_namespaces`` engine
config kwarg instead.
"""

from __future__ import annotations

from typing import Any, Callable

__all__ = [
    "prepareQuery",
    "register_custom_function",
    "unregister_custom_function",
    "CUSTOM_EVALS",
]

#: RDFLib's whole-algebra custom-eval hook table (``rdflib.plugins.sparql.CUSTOM_EVALS``).
#: Recorded for API parity; the native engine does not consult it (see module docstring).
CUSTOM_EVALS: dict[str, Any] = {}

#: ``{ uri: (function, raise_not_bound) }`` — the registered custom SPARQL functions.
_CUSTOM_FUNCTIONS: dict[str, tuple[Callable[..., Any], bool]] = {}


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


def register_custom_function(
    uri: object,
    func: Callable[..., Any],
    override: bool = False,
    raise_not_bound: bool = True,
) -> Callable[..., Any]:
    """Register a custom SPARQL function for ``uri`` (RDFLib API surface).

    Records the callable so the registry round-trips like RDFLib's; note the
    native engine does not invoke it at query time (see the module docstring and
    the #10 ledger note). Returns ``func`` so it can be used as a decorator.
    """
    key = str(uri)
    if not override and key in _CUSTOM_FUNCTIONS:
        raise ValueError(f"A function is already registered for {key}")
    _CUSTOM_FUNCTIONS[key] = (func, raise_not_bound)
    return func


def unregister_custom_function(
    uri: object, func: Callable[..., Any] | None = None
) -> None:
    """Remove a previously registered custom function (RDFLib API surface)."""
    key = str(uri)
    if key in _CUSTOM_FUNCTIONS and (func is None or _CUSTOM_FUNCTIONS[key][0] is func):
        del _CUSTOM_FUNCTIONS[key]
