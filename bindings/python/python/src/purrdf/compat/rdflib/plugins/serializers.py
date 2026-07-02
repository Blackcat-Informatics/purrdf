# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Built-in RDF serializers for the purrdf compat plugin registry (#7/#11).

Each class implements the RDFLib serializer interface (``__init__(graph)`` +
``serialize(stream, ...)``) and is the target the plugin registry resolves for a
serializer name. :meth:`Graph.serialize` looks a name up through the registry,
instantiates the resolved class with the graph, and drives it into a byte
stream — so the registry is the single source of truth for name → implementation.

Emitters are byte-deterministic: Turtle routes through the native
``canonicalize_turtle``; N-Triples/N-Quads/TriG dump via the deterministic
oxigraph writers; JSON-LD-star and RDF/XML route through the purrdf-gts codecs.
TriX and HexTuples are registered as *unsupported* (lookup resolves, use raises
``NotImplementedError``) — see the xfail ledger (#7/#11).
"""

from __future__ import annotations

from typing import IO, Any

import purrdf

from ..serializer import Serializer

_TURTLE = purrdf.RdfFormat.TURTLE
_NT = purrdf.RdfFormat.N_TRIPLES
_NQ = purrdf.RdfFormat.N_QUADS
_TRIG = purrdf.RdfFormat.TRIG


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
    """``longturtle`` alias — the shim emits canonical Turtle (caveat #11)."""


class N3Serializer(TurtleSerializer):
    """``n3``/``text/n3`` alias — the shim emits Turtle (an N3 subset; caveat #11)."""


class JsonLDSerializer(Serializer):
    """JSON-LD (with RDF-star support) via the purrdf-gts codec."""

    def serialize(
        self,
        stream: IO[bytes],
        base: str | None = None,
        encoding: str | None = None,
        **args: Any,
    ) -> None:
        """Emit JSON-LD (store → N-Quads → ``to_json_ld``)."""
        nquads = self.store._store.dump(format=_NQ)
        stream.write(purrdf.to_json_ld(nquads, format=_NQ).encode("utf-8"))


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


class _UnsupportedSerializer(Serializer):
    """A registered-but-unimplemented serializer (lookup resolves, use raises)."""

    format_label = "this format"

    def serialize(
        self,
        stream: IO[bytes],
        base: str | None = None,
        encoding: str | None = None,
        **args: Any,
    ) -> None:
        """Raise ``NotImplementedError`` — the format is ledgered as unsupported."""
        raise NotImplementedError(
            f"{self.format_label} serialization is not yet supported by the purrdf "
            "compat shim (#7/#11)"
        )


class TriXSerializer(_UnsupportedSerializer):
    """TriX serializer — registered as unsupported (no native TriX writer; #7/#11)."""

    format_label = "TriX"


class HextuplesSerializer(_UnsupportedSerializer):
    """HexTuples serializer — registered as unsupported (#7/#11)."""

    format_label = "HexTuples"
