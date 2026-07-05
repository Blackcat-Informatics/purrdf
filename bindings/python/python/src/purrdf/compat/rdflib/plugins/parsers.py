# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Built-in RDF parsers for the purrdf compat plugin registry.

Each class implements the RDFLib parser interface (``parse(source, sink)``) and
is the target the plugin registry resolves for a parser name. :meth:`Graph.parse`
resolves a name through the registry and dispatches to the resolved class; for a
native format read from a filesystem path it keeps the direct-load fast-path,
using the parser class's :attr:`rdf_format` marker rather than a hardcoded map.

The native parsers carry an :attr:`rdf_format` marker (Turtle/N-Triples/N-Quads/
TriG/TriX/HexTuples); the codec parsers (JSON-LD-star, RDF/XML) route bytes
through the purrdf-gts codecs. TriX and HexTuples are native codecs
(``purrdf-rdf``): they load straight into the store like the other native
quad formats.
"""

from __future__ import annotations

import json
import re
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


#: An XML namespace declaration: ``xmlns:prefix="uri"`` (default ``xmlns`` is
#: intentionally excluded; the reserved ``xml`` prefix is skipped by the caller).
_XMLNS_PREFIX_RE = re.compile(
    r"""\sxmlns:([A-Za-z_][\w.\-]*)\s*=\s*(["'])(.*?)\2""",
    re.DOTALL,
)


def _scan_xml_prefixes(text: str) -> list[tuple[str, str]]:
    """Extract ``(prefix, iri)`` declarations from XML ``xmlns:`` attributes.

    A lightweight lexical scan (no full parse): RDF/XML records document prefixes
    on the graph's ``NamespaceManager`` during parsing, but the native codec does
    not surface them, so we recover them from the source text.
    """
    return [(match.group(1), match.group(3)) for match in _XMLNS_PREFIX_RE.finditer(text)]


class _NativeParser(Parser):
    """Load a native-format payload into the sink store (deterministic)."""

    rdf_format: purrdf.RdfFormat = _TURTLE
    #: Whether the format carries ``@prefix``/``PREFIX`` declarations to recover.
    prefix_bearing: bool = False

    def parse(self, source: Any, sink: Graph, **kwargs: Any) -> None:
        """Load ``source`` bytes into ``sink``'s store, recovering prefixes."""
        payload = _as_bytes(source)
        if self.prefix_bearing:
            sink._bind_source_prefixes(payload)
        sink._store.load(payload, format=self.rdf_format, base=kwargs.get("base"))


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
                if not isinstance(value, str):
                    continue
                if value.endswith(("/", "#", ":")):
                    sink.bind(prefix, URIRef(value))


class RDFXMLParser(Parser):
    """RDF/XML parser via the purrdf-gts codec (``xml``/``application/rdf+xml``)."""

    def parse(self, source: Any, sink: Graph, **kwargs: Any) -> None:
        """Load RDF/XML text (``from_rdf_xml`` → N-Quads → store)."""
        text = _as_text(source)
        sink._store.load(purrdf.from_rdf_xml(text), format=_NQ)
        for prefix, namespace in _scan_xml_prefixes(text):
            if prefix == "xml":
                continue
            sink.bind(prefix, URIRef(namespace))


class TriXParser(_NativeParser):
    """TriX parser (``trix``/``application/trix``) via the native codec."""

    rdf_format = _TRIX


class HextuplesParser(_NativeParser):
    """HexTuples parser (``hext``) via the native codec."""

    rdf_format = _HEXT
