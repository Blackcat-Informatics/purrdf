# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Built-in RDF serializers for the purrdf compat plugin registry.

Each class implements the RDFLib serializer interface (``__init__(graph)`` +
``serialize(stream, ...)``) and is the target the plugin registry resolves for a
serializer name. :meth:`Graph.serialize` looks a name up through the registry,
instantiates the resolved class with the graph, and drives it into a byte
stream — so the registry is the single source of truth for name → implementation.

Emitters are byte-deterministic: Turtle routes through the native
``canonicalize_turtle``; N-Triples/N-Quads/TriG/TriX/HexTuples dump via the
deterministic native codec writers; JSON-LD-star and RDF/XML route through the
purrdf-gts codecs. TriX and HexTuples are native codecs (``purrdf-rdf``) that
emit deterministic quad documents.
"""

from __future__ import annotations

import json
from typing import IO, Any

import purrdf

from ...serializer import Serializer

__all__ = [
    "HextuplesSerializer",
    "JsonLDSerializer",
    "LongTurtleSerializer",
    "N3Serializer",
    "NQuadsSerializer",
    "NT11Serializer",
    "NTSerializer",
    "PrettyXMLSerializer",
    "TriGSerializer",
    "TriXSerializer",
    "TurtleSerializer",
    "XMLSerializer",
]

_TURTLE = purrdf.RdfFormat.TURTLE
_NT = purrdf.RdfFormat.N_TRIPLES
_NQ = purrdf.RdfFormat.N_QUADS
_TRIG = purrdf.RdfFormat.TRIG
_TRIX = purrdf.RdfFormat.TRIX
_HEXT = purrdf.RdfFormat.HEXTUPLES


class _NativeStoreSerializer(Serializer):
    """Serialize the whole store via a native oxigraph writer (deterministic)."""

    rdf_format: purrdf.RdfFormat = _NT

    def serialize(
        self,
        stream: IO[bytes],
        base: str | None = None,
        encoding: str | None = None,
        **args: Any,
    ) -> None:
        """Dump the store in :attr:`rdf_format` to ``stream``."""
        stream.write(self.store._store.dump(format=self.rdf_format))


class NTSerializer(_NativeStoreSerializer):
    """N-Triples serializer (``nt``/``ntriples``/``application/n-triples``)."""

    rdf_format = _NT


class NT11Serializer(NTSerializer):
    """N-Triples 1.1 serializer alias (``nt11``)."""


class NQuadsSerializer(_NativeStoreSerializer):
    """N-Quads serializer (``nquads``/``nq``/``application/n-quads``)."""

    rdf_format = _NQ


class TriGSerializer(_NativeStoreSerializer):
    """TriG serializer (``trig``/``application/trig``)."""

    rdf_format = _TRIG


class TurtleSerializer(Serializer):
    """Turtle serializer via the native canonicalizer (deterministic, dogfooded)."""

    def serialize(
        self,
        stream: IO[bytes],
        base: str | None = None,
        encoding: str | None = None,
        **args: Any,
    ) -> None:
        """Emit canonical Turtle using the graph's bound prefixes."""
        nt = self.store._store.dump(format=_NT)
        prefixes = [(prefix, str(ns)) for prefix, ns in self.store._nsm.namespaces()]
        stream.write(purrdf.canonicalize_turtle(nt, prefixes))


class LongTurtleSerializer(TurtleSerializer):
    """``longturtle`` alias — the shim emits canonical Turtle."""


class N3Serializer(TurtleSerializer):
    """``n3``/``text/n3`` alias — the shim emits Turtle (an N3 subset)."""


class JsonLDSerializer(Serializer):
    """JSON-LD (with RDF-star support) via the purrdf-gts codec."""

    def serialize(
        self,
        stream: IO[bytes],
        base: str | None = None,
        encoding: str | None = None,
        **args: Any,
    ) -> None:
        """Emit JSON-LD through the shared configured context engine."""
        nquads = self.store._store.dump(format=_NQ)
        options = args.get("jsonld_options")
        context = args.get("jsonld_context")
        if options is not None and not isinstance(options, str):
            options = json.dumps(options, sort_keys=True, separators=(",", ":"))
        if options is None and context is None:
            prefixes = self.store._nsm.jsonld_prefixes()
            if prefixes:
                options = json.dumps(
                    {"version": 1, "mode": "context", "prefixes": prefixes},
                    sort_keys=True,
                    separators=(",", ":"),
                )
        if options is None and context is None:
            rendered = purrdf.to_json_ld(nquads, format=_NQ)
        else:
            rendered = purrdf.serialize_jsonld(
                nquads,
                format=_NQ,
                output_format="jsonld",
                options_json=options,
                context=context,
            )
        stream.write(rendered.encode("utf-8"))


class XMLSerializer(Serializer):
    """RDF/XML serializer via the purrdf-gts codec (``xml``/``application/rdf+xml``)."""

    def serialize(
        self,
        stream: IO[bytes],
        base: str | None = None,
        encoding: str | None = None,
        **args: Any,
    ) -> None:
        """Emit RDF/XML (store → N-Quads → ``to_rdf_xml``)."""
        nquads = self.store._store.dump(format=_NQ)
        stream.write(purrdf.to_rdf_xml(nquads, format=_NQ).encode("utf-8"))


class PrettyXMLSerializer(XMLSerializer):
    """``pretty-xml`` alias — routes through the same RDF/XML codec."""


class TriXSerializer(_NativeStoreSerializer):
    """TriX serializer (``trix``/``application/trix``) via the native codec."""

    rdf_format = _TRIX


class HextuplesSerializer(_NativeStoreSerializer):
    """HexTuples serializer (``hext``) via the native codec."""

    rdf_format = _HEXT
