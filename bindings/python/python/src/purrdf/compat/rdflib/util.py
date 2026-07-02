# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Utility helpers for the purrdf compat shim (RDFLib ``rdflib.util``)."""

from __future__ import annotations

import re
from pathlib import PurePath
from typing import TYPE_CHECKING

import purrdf

from .term import BNode, Literal, URIRef, from_native

if TYPE_CHECKING:
    from .namespace import NamespaceManager
    from .term import Identifier

_XSD = "http://www.w3.org/2001/XMLSchema#"

# Turtle numeric literal shorthands (RDFLib ``from_n3`` parity): a bare token maps to
# an ``xsd:integer`` / ``xsd:decimal`` / ``xsd:double`` typed literal.
_INTEGER_RE = re.compile(r"^[+-]?[0-9]+$")
_DECIMAL_RE = re.compile(r"^[+-]?[0-9]*\.[0-9]+$")
_DOUBLE_RE = re.compile(r"^[+-]?(?:[0-9]+\.[0-9]*|\.[0-9]+|[0-9]+)[eE][+-]?[0-9]+$")
# A ``^^prefix:local`` datatype suffix (a CURIE, not a ``^^<iri>`` full IRI).
_CURIE_DATATYPE_RE = re.compile(r"\^\^(?!<)([^\s<>\"]+)$")

#: File-suffix → RDFLib format name (the subset the native surface serves, plus
#: the suffixes the toolchain detects before dispatching).
_SUFFIX_FORMATS: dict[str, str] = {
    ".ttl": "turtle",
    ".turtle": "turtle",
    ".n3": "n3",
    ".nt": "nt",
    ".ntriples": "nt",
    ".nq": "nquads",
    ".nquads": "nquads",
    ".trig": "trig",
    ".jsonld": "json-ld",
    ".json": "json-ld",
    ".rdf": "xml",
    ".xml": "xml",
    ".owl": "xml",
}


def guess_format(
    path: str | PurePath, fmap: dict[str, str] | None = None
) -> str | None:
    """Guess an RDFLib format name from a path's suffix (RDFLib parity)."""
    suffix = PurePath(path).suffix.lower()
    table = fmap if fmap is not None else _SUFFIX_FORMATS
    return table.get(suffix)


def _prefix_map(nsm: NamespaceManager | dict[str, str] | None) -> dict[str, str]:
    """Return a ``{prefix: namespace_iri}`` map from a nsm, dict, or ``None``."""
    if nsm is None:
        return {}
    if isinstance(nsm, dict):
        return {str(k): str(v) for k, v in nsm.items()}
    # A compat ``NamespaceManager`` exposes ``namespaces()`` → ``(prefix, iri)`` pairs.
    namespaces = getattr(nsm, "namespaces", None)
    if callable(namespaces):
        return {str(prefix): str(iri) for prefix, iri in namespaces()}
    return {}


def _expand_curie(
    curie: str, nsm: NamespaceManager | dict[str, str] | None
) -> str | None:
    """Expand a ``prefix:local`` CURIE to a full IRI, or ``None`` if unresolvable."""
    if ":" not in curie:
        return None
    prefix, local = curie.split(":", 1)
    namespace = _prefix_map(nsm).get(prefix)
    if namespace is None:
        return None
    return namespace + local


def _nt_object_term(
    term_n3: str,
) -> purrdf.NamedNode | purrdf.BlankNode | purrdf.Literal | purrdf.Triple:
    """Parse a single N-Triples object term by round-tripping the native parser."""
    line = f"<urn:x> <urn:x> {term_n3} .".encode()
    quad = purrdf.parse(line, format=purrdf.RdfFormat.N_TRIPLES)[0]
    return quad.object


def _literal_from_n3(s: str, nsm: NamespaceManager | dict[str, str] | None) -> Literal:
    """Parse a quoted N3/Turtle literal (``"lit"``/``@lang``/``^^dt``) via the engine.

    A ``^^prefix:local`` datatype CURIE is expanded first, then the whole literal is
    round-tripped through the native N-Triples parser so escapes/lang/datatype are
    decoded by the engine (RDFLib parity).
    """
    body = s
    match = _CURIE_DATATYPE_RE.search(s)
    if match is not None:
        iri = _expand_curie(match.group(1), nsm)
        if iri is None:
            msg = f"cannot expand datatype CURIE in {s!r}: unknown prefix"
            raise ValueError(msg)
        body = f"{s[: match.start()]}^^<{iri}>"
    obj = _nt_object_term(body)
    result = from_native(obj)
    if not isinstance(result, Literal):
        msg = f"expected a literal from {s!r}"
        raise ValueError(msg)
    return result


def from_n3(
    s: str,
    default: Identifier | None = None,
    backend: object | None = None,
    nsm: NamespaceManager | dict[str, str] | None = None,
) -> Identifier | None:
    """Parse a single N3/Turtle *term* string into a compat term (RDFLib parity).

    Handles ``<iri>``, ``_:blank``, quoted literals (``"lit"``, ``"lit"@lang``,
    ``"lit"^^<dt>``, ``"lit"^^prefix:local``), the Turtle numeric/boolean shorthands
    (``42``, ``1.5``, ``1e3``, ``true``/``false``), and ``prefix:local`` CURIEs when a
    ``nsm`` (a :class:`NamespaceManager` or a ``{prefix: iri}`` dict) is supplied.
    Returns ``default`` for an empty string. ``backend`` is accepted for signature
    parity and unused.
    """
    _ = backend
    text = s.strip()
    if not text:
        return default
    if text.startswith("<") and text.endswith(">"):
        return URIRef(text[1:-1])
    if text.startswith("_:"):
        return BNode(text[2:])
    if text[0] in ('"', "'"):
        return _literal_from_n3(text, nsm)
    if text in ("true", "false"):
        return Literal(text, datatype=URIRef(_XSD + "boolean"))
    if _DOUBLE_RE.match(text):
        return Literal(text, datatype=URIRef(_XSD + "double"))
    if _DECIMAL_RE.match(text):
        return Literal(text, datatype=URIRef(_XSD + "decimal"))
    if _INTEGER_RE.match(text):
        return Literal(text, datatype=URIRef(_XSD + "integer"))
    expanded = _expand_curie(text, nsm)
    if expanded is not None:
        return URIRef(expanded)
    if default is not None:
        return default
    msg = f"cannot parse N3 term {s!r}"
    raise ValueError(msg)
