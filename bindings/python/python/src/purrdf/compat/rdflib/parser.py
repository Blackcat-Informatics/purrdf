# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Parser plugin *kind* base class (RDFLib ``rdflib.parser``).

This mirrors RDFLib's ``rdflib.parser`` module far enough for
``plugin.get(name, Parser)`` to resolve against the same *kind* identity RDFLib
uses. The concrete built-in parsers live in
:mod:`purrdf.compat.rdflib.plugins.parsers`; they are the classes the plugin
registry points at and that :meth:`Graph.parse` dispatches to.

The ``InputSource`` family is present for import-compatibility (``from
rdflib.parser import InputSource``) — the shim's :meth:`Graph.parse` extracts a
``bytes`` payload itself and hands that to the resolved parser, so these are
lightweight carriers rather than the full SAX ``xml.sax.xmlreader.InputSource``
surface.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from .graph import Graph

__all__ = [
    "Parser",
    "InputSource",
    "StringInputSource",
    "URLInputSource",
    "FileInputSource",
    "PythonInputSource",
]


class Parser:
    """Base class for an RDF parser *kind* (the ``plugin.get`` key type)."""

    __slots__ = ()

    def __init__(self) -> None:
        """Construct the parser (RDFLib parsers take no constructor arguments)."""

    def parse(self, source: Any, sink: Graph, **kwargs: Any) -> None:
        """Parse ``source`` into ``sink`` (overridden by concrete parsers)."""
        raise NotImplementedError


class InputSource:
    """A minimal parse-source carrier (RDFLib ``rdflib.parser.InputSource``)."""

    def __init__(self, system_id: str | None = None) -> None:
        """Record the optional system id (public/base IRI)."""
        self.system_id = system_id
        self.public_id: str | None = None

    def setPublicId(self, public_id: str) -> None:  # noqa: N802 - RDFLib API name
        """Set the public id (base IRI)."""
        self.public_id = public_id

    def getPublicId(self) -> str | None:  # noqa: N802 - RDFLib API name
        """Return the public id."""
        return self.public_id

    def setSystemId(self, system_id: str) -> None:  # noqa: N802 - RDFLib API name
        """Set the system id."""
        self.system_id = system_id

    def getSystemId(self) -> str | None:  # noqa: N802 - RDFLib API name
        """Return the system id."""
        return self.system_id


class StringInputSource(InputSource):
    """An in-memory ``str``/``bytes`` parse source (RDFLib parity)."""

    def __init__(self, value: str | bytes, system_id: str | None = None) -> None:
        """Wrap ``value`` as the parse payload."""
        super().__init__(system_id)
        self.value = value


class URLInputSource(InputSource):
    """A URL-backed parse source (RDFLib parity, carrier only)."""

    def __init__(self, system_id: str | None = None, format: str | None = None) -> None:
        """Record the source URL and its declared format."""
        super().__init__(system_id)
        self.url = system_id
        self.content_type = format


class FileInputSource(InputSource):
    """A file-object parse source (RDFLib parity, carrier only)."""

    def __init__(self, file: Any) -> None:
        """Wrap an open file object as the parse source."""
        super().__init__(getattr(file, "name", None))
        self.file = file


class PythonInputSource(InputSource):
    """An in-memory Python-object parse source (RDFLib parity, carrier only)."""

    def __init__(self, data: Any, system_id: str | None = None) -> None:
        """Wrap a native Python structure (e.g. parsed JSON-LD) as the source."""
        super().__init__(system_id)
        self.data = data
