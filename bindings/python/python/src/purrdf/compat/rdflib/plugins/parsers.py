# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Built-in RDF parsers for the purrdf compat plugin registry.

Each class implements the RDFLib parser interface (``parse(source, sink)``) and
is the target the plugin registry resolves for a parser name. :meth:`Graph.parse`
resolves a name through the registry and dispatches to the resolved class; for a
native format read from a filesystem path it keeps the direct-load fast-path,
using the parser class's :attr:`rdf_format` marker rather than a hardcoded map.

The native parsers carry an :attr:`rdf_format` marker (Turtle/N-Triples/N-Quads/
TriG/TriX/HexTuples/RDF/XML) and load straight into the store through the
``purrdf-rdf`` codecs, which also report the document's prefixes; the JSON-LD-star
parser routes its bytes through the dedicated JSON-LD converter.
"""

from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any

import purrdf

from ..parser import Parser
from ..term import URIRef

if TYPE_CHECKING:
    from ..graph import Graph

_TURTLE = purrdf.RdfFormat.TURTLE
_NT = purrdf.RdfFormat.N_TRIPLES
_NQ = purrdf.RdfFormat.N_QUADS
_TRIG = purrdf.RdfFormat.TRIG
_TRIX = purrdf.RdfFormat.TRIX
_HEXT = purrdf.RdfFormat.HEXTUPLES
_RDFXML = purrdf.RdfFormat.RDF_XML


def _as_bytes(source: Any) -> bytes:
    """Coerce a parse source (``bytes``/``str``) to UTF-8 bytes."""
    if isinstance(source, bytes):
        return source
    if isinstance(source, str):
        return source.encode("utf-8")
    raise TypeError(f"unsupported parse source: {source!r}")


def _as_text(source: Any) -> str:
    """Coerce a parse source (``bytes``/``str``) to text."""
    if isinstance(source, str):
        return source
    if isinstance(source, bytes):
        return source.decode("utf-8")
    raise TypeError(f"unsupported parse source: {source!r}")


class _NativeParser(Parser):
    """Load a native-format payload into the sink store (deterministic)."""

    rdf_format: purrdf.RdfFormat = _TURTLE
    #: Whether the format declares prefixes (``@prefix``/``PREFIX``, ``xmlns``) to bind.
    prefix_bearing: bool = False

    def parse(self, source: Any, sink: Graph, **kwargs: Any) -> None:
        """Load ``source`` bytes into ``sink``'s store, binding the parsed prefixes."""
        payload = _as_bytes(source)
        prefixes = sink._store.load(payload, format=self.rdf_format, base=kwargs.get("base"))
        if self.prefix_bearing:
            sink._bind_document_prefixes(prefixes)


class TurtleParser(_NativeParser):
    """Turtle parser (``turtle``/``ttl``/``n3``/``longturtle``)."""

    rdf_format = _TURTLE
    prefix_bearing = True


class NTParser(_NativeParser):
    """N-Triples parser (``nt``/``ntriples``/``nt11``/``application/n-triples``)."""

    rdf_format = _NT


class NQuadsParser(_NativeParser):
    """N-Quads parser (``nquads``/``nq``/``application/n-quads``)."""

    rdf_format = _NQ


class TriGParser(_NativeParser):
    """TriG parser (``trig``/``application/trig``)."""

    rdf_format = _TRIG
    prefix_bearing = True


class JsonLDParser(Parser):
    """JSON-LD (with RDF-star support) parser via the purrdf-gts codec."""

    #: Reserved JSON-LD keywords that are never prefix mappings.
    _RESERVED_CONTEXT_KEYS: frozenset[str] = frozenset(
        (
            "@vocab",
            "@language",
            "@base",
            "@version",
            "@propagate",
            "@protected",
            "@import",
            "@scope",
        )
    )

    def parse(self, source: Any, sink: Graph, **kwargs: Any) -> None:
        """Load JSON-LD text (``from_json_ld`` → N-Quads → store)."""
        text = _as_text(source)
        sink._store.load(purrdf.from_json_ld(text), format=_NQ)
        self._bind_jsonld_context_prefixes(text, sink)

    def _bind_jsonld_context_prefixes(self, text: str, sink: Graph) -> None:
        """Extract prefix → namespace mappings from a JSON-LD ``@context``."""
        try:
            doc = json.loads(text)
        except json.JSONDecodeError:
            return
        if not isinstance(doc, dict):
            return
        context = doc.get("@context")
        if context is None:
            return
        if isinstance(context, str):
            # Remote context URL; cannot recover prefixes from this document.
            return
        if isinstance(context, dict):
            contexts = [context]
        elif isinstance(context, list):
            contexts = context
        else:
            return
        for ctx in contexts:
            if not isinstance(ctx, dict):
                continue
            for prefix, value in ctx.items():
                if prefix in self._RESERVED_CONTEXT_KEYS:
                    continue
                namespace: str | None = None
                if isinstance(value, str):
                    namespace = value
                elif (
                    isinstance(value, dict)
                    and value.get("@prefix") is True
                    and isinstance(value.get("@id"), str)
                ):
                    namespace = value["@id"]
                if namespace is not None and namespace.endswith(("/", "#", ":")):
                    sink.bind(prefix, URIRef(namespace))


class RDFXMLParser(_NativeParser):
    """RDF/XML parser (``xml``/``application/rdf+xml``) via the native codec.

    Its prefixes are the document's ``xmlns`` declarations as the XML parser scoped
    them, returned by the same ``load`` — never ``xmlns:`` text inside element
    content or inside an ``rdf:parseType="Literal"`` value.
    """

    rdf_format = _RDFXML
    prefix_bearing = True


class TriXParser(_NativeParser):
    """TriX parser (``trix``/``application/trix``) via the native codec."""

    rdf_format = _TRIX


class HextuplesParser(_NativeParser):
    """HexTuples parser (``hext``) via the native codec."""

    rdf_format = _HEXT
