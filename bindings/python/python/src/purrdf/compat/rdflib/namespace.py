# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Namespaces + the closed RDF/RDFS/OWL/SKOS/XSD/… vocabularies (RDFLib parity).

``Namespace`` is a ``str`` subclass whose attribute / item access mints a
:class:`~purrdf.compat.rdflib.term.URIRef` — ``RDF.type`` →
``URIRef("…#type")`` — exactly like RDFLib. The standard vocabularies are
provided as ``Namespace`` instances seeded with their canonical base IRIs.
"""

from __future__ import annotations

from .term import URIRef


class Namespace(str):
    """A namespace IRI prefix whose attribute/item access yields a ``URIRef``."""

    __slots__ = ()

    def __new__(cls, value: str) -> Namespace:
        """Construct from the base IRI string."""
        return str.__new__(cls, value)

    def term(self, name: str) -> URIRef:
        """Return ``URIRef(base + name)``."""
        return URIRef(str(self) + name)

    # `title` is a real ``str`` method, so normal attribute lookup would shadow the
    # vocabulary term (e.g. ``DCTERMS.title``). Override it to a URIRef, exactly as
    # RDFLib's Namespace does, so the one common collision resolves correctly.
    @property
    def title(self) -> URIRef:  # type: ignore[override]
        """Return ``URIRef(base + "title")`` (RDFLib parity for ``DCTERMS.title``)."""
        return URIRef(str(self) + "title")

    def __getitem__(self, key: str) -> URIRef:  # type: ignore[override]
        """Return ``URIRef(base + key)`` — supports names that are not identifiers."""
        return URIRef(str(self) + str(key))

    def __getattr__(self, name: str) -> URIRef:
        """Return ``URIRef(base + name)`` for a vocabulary member access."""
        if name.startswith("__"):
            raise AttributeError(name)
        return URIRef(str(self) + name)


class NamespaceManager:
    """A minimal prefix registry feeding the Turtle serializer's prefix header."""

    def __init__(self) -> None:
        """Start with an empty prefix map."""
        self._prefixes: dict[str, str] = {}

    def bind(
        self,
        prefix: str | None,
        namespace: object,
        *,
        override: bool = True,
        replace: bool = False,
    ) -> None:
        """Bind ``prefix`` → ``namespace`` (last write wins)."""
        self._prefixes[prefix or ""] = str(namespace)

    def namespaces(self) -> list[tuple[str, str]]:
        """Return the bound ``(prefix, namespace_iri)`` pairs."""
        return list(self._prefixes.items())

    def normalizeUri(self, uri: str) -> str:  # noqa: N802 - RDFLib API name
        """Return ``prefix:local`` for ``uri`` if a prefix matches, else ``<uri>``."""
        text = str(uri)
        best_prefix = ""
        best_ns = ""
        for prefix, namespace in self._prefixes.items():
            if text.startswith(namespace) and len(namespace) > len(best_ns):
                best_prefix, best_ns = prefix, namespace
        if best_ns:
            return f"{best_prefix}:{text[len(best_ns) :]}"
        return f"<{text}>"


# ── The closed standard vocabularies (RDFLib-equivalent attribute access) ────────

RDF = Namespace("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
RDFS = Namespace("http://www.w3.org/2000/01/rdf-schema#")
OWL = Namespace("http://www.w3.org/2002/07/owl#")
SKOS = Namespace("http://www.w3.org/2004/02/skos/core#")
XSD = Namespace("http://www.w3.org/2001/XMLSchema#")
DC = Namespace("http://purl.org/dc/elements/1.1/")
DCTERMS = Namespace("http://purl.org/dc/terms/")
FOAF = Namespace("http://xmlns.com/foaf/0.1/")
DCAT = Namespace("http://www.w3.org/ns/dcat#")
VOID = Namespace("http://rdfs.org/ns/void#")
SH = Namespace("http://www.w3.org/ns/shacl#")
SDO = Namespace("https://schema.org/")
PROV = Namespace("http://www.w3.org/ns/prov#")
