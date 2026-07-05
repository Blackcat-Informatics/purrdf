# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Plugin registry for the purrdf compat shim (RDFLib ``rdflib.plugin``).

Mirrors RDFLib's plugin registry so a drop-in caller can resolve an
implementation by ``(name, kind)``:

    from rdflib.plugin import get
    from rdflib.serializer import Serializer
    get("turtle", Serializer)  # -> a serializer class

Two registration paths are honoured, exactly as in RDFLib:

* **Built-ins** — :func:`register` records a ``(name, kind) -> (module_path,
  class_name)`` entry; the class is imported lazily on first :func:`get`.
* **Entry points** — third-party packages that publish plugins against RDFLib's
  ``rdf.plugins.*`` entry-point groups are discovered at import time through
  :func:`importlib.metadata.entry_points`, so a plugin registered for the real
  ``rdflib`` is equally discoverable through this shim.

The *kind* classes are the shim's own ``Parser``/``Serializer``/``Store`` and the
``rdflib.query`` hierarchy, so a caller that imports them from the shim
(``from rdflib.serializer import Serializer``) gets the same *kind* identity used
as the registry key here.
"""

from __future__ import annotations

from importlib.metadata import EntryPoint, entry_points
from typing import Generic, Iterator, Optional, TypeVar

from .parser import Parser
from .query import (
    Processor,
    Result,
    ResultParser,
    ResultSerializer,
    UpdateProcessor,
)
from .serializer import Serializer
from .store import Store

__all__ = [
    "register",
    "get",
    "plugins",
    "PluginException",
    "Plugin",
    "PKGPlugin",
    "PluginT",
]

PluginT = TypeVar("PluginT")


class PluginException(Exception):  # noqa: N818 - RDFLib API name
    """Raised when no plugin is registered for a requested ``(name, kind)``."""


#: RDFLib's entry-point group → plugin *kind* map (verified against rdflib 7.6).
rdflib_entry_points: dict[str, type] = {
    "rdf.plugins.store": Store,
    "rdf.plugins.serializer": Serializer,
    "rdf.plugins.parser": Parser,
    "rdf.plugins.resultparser": ResultParser,
    "rdf.plugins.resultserializer": ResultSerializer,
    "rdf.plugins.queryprocessor": Processor,
    "rdf.plugins.queryresult": Result,
    "rdf.plugins.updateprocessor": UpdateProcessor,
}

_plugins: dict[tuple[str, type], Plugin] = {}


class Plugin(Generic[PluginT]):
    """A lazily-imported built-in plugin (``module_path``:``class_name``)."""

    def __init__(
        self, name: str, kind: type, module_path: str, class_name: str
    ) -> None:
        """Record the plugin's name, kind, and dotted import location."""
        self.name = name
        self.kind = kind
        self.module_path = module_path
        self.class_name = class_name
        self._class: Optional[type] = None

    def getClass(self) -> type:  # noqa: N802 - RDFLib API name
        """Import and return the plugin class (memoized)."""
        if self._class is None:
            module = __import__(self.module_path, globals(), locals(), [""])
            self._class = getattr(module, self.class_name)
        return self._class


class PKGPlugin(Plugin[PluginT]):
    """A plugin discovered through an ``importlib.metadata`` entry point."""

    def __init__(self, name: str, kind: type, ep: EntryPoint) -> None:
        """Record the entry point backing this plugin."""
        self.name = name
        self.kind = kind
        self.ep = ep
        self._class = None

    def getClass(self) -> type:  # noqa: N802 - RDFLib API name
        """Load and return the entry-point target (memoized)."""
        if self._class is None:
            self._class = self.ep.load()
        return self._class


def register(name: str, kind: type, module_path: str, class_name: str) -> None:
    """Register the built-in plugin class ``module_path:class_name`` for ``(name, kind)``."""
    _plugins[(name, kind)] = Plugin(name, kind, module_path, class_name)


def get(name: str, kind: type) -> type:
    """Return the class registered for ``(name, kind)`` (raises :class:`PluginException`)."""
    try:
        plugin = _plugins[(name, kind)]
    except KeyError:
        raise PluginException(
            f"No plugin registered for ({name}, {kind})"
        ) from None
    return plugin.getClass()


def plugins(
    name: Optional[str] = None, kind: Optional[type] = None
) -> Iterator[Plugin]:
    """Yield registered plugins, optionally filtered by ``name`` and/or ``kind``."""
    for plugin in _plugins.values():
        if (name is None or name == plugin.name) and (
            kind is None or kind == plugin.kind
        ):
            yield plugin


def _discover_entry_points() -> None:
    """Register third-party plugins published against RDFLib's entry-point groups."""
    all_entry_points = entry_points()
    for group, kind in rdflib_entry_points.items():
        for ep in all_entry_points.select(group=group):
            _plugins[(ep.name, kind)] = PKGPlugin(ep.name, kind, ep)


_discover_entry_points()


# ── built-in triple/quad serializers ─────────────────────────────────────────────
_SERIALIZERS = "purrdf.compat.rdflib.plugins.serializers"
for _name in ("text/turtle", "turtle", "ttl"):
    register(_name, Serializer, _SERIALIZERS, "TurtleSerializer")
for _name in ("longturtle",):
    register(_name, Serializer, _SERIALIZERS, "LongTurtleSerializer")
for _name in ("text/n3", "n3"):
    register(_name, Serializer, _SERIALIZERS, "N3Serializer")
for _name in ("application/n-triples", "ntriples", "nt", "nt11", "ntriples11"):
    register(_name, Serializer, _SERIALIZERS, "NTSerializer")
for _name in ("application/n-quads", "nquads", "nq"):
    register(_name, Serializer, _SERIALIZERS, "NQuadsSerializer")
for _name in ("application/trig", "trig"):
    register(_name, Serializer, _SERIALIZERS, "TriGSerializer")
for _name in ("json-ld", "jsonld", "application/ld+json"):
    register(_name, Serializer, _SERIALIZERS, "JsonLDSerializer")
for _name in ("xml", "application/rdf+xml"):
    register(_name, Serializer, _SERIALIZERS, "XMLSerializer")
for _name in ("pretty-xml",):
    register(_name, Serializer, _SERIALIZERS, "PrettyXMLSerializer")
for _name in ("application/trix", "trix"):
    register(_name, Serializer, _SERIALIZERS, "TriXSerializer")
for _name in ("hext",):
    register(_name, Serializer, _SERIALIZERS, "HextuplesSerializer")

# ── built-in triple/quad parsers ─────────────────────────────────────────────────
_PARSERS = "purrdf.compat.rdflib.plugins.parsers"
for _name in (
    "text/turtle",
    "turtle",
    "ttl",
    "text/n3",
    "n3",
    "longturtle",
):
    register(_name, Parser, _PARSERS, "TurtleParser")
for _name in ("application/n-triples", "ntriples", "nt", "nt11", "ntriples11"):
    register(_name, Parser, _PARSERS, "NTParser")
for _name in ("application/n-quads", "nquads", "nq"):
    register(_name, Parser, _PARSERS, "NQuadsParser")
for _name in ("application/trig", "trig"):
    register(_name, Parser, _PARSERS, "TriGParser")
for _name in ("json-ld", "jsonld", "application/ld+json"):
    register(_name, Parser, _PARSERS, "JsonLDParser")
for _name in ("xml", "application/rdf+xml", "pretty-xml"):
    register(_name, Parser, _PARSERS, "RDFXMLParser")
for _name in ("application/trix", "trix"):
    register(_name, Parser, _PARSERS, "TriXParser")
for _name in ("hext",):
    register(_name, Parser, _PARSERS, "HextuplesParser")

# ── SPARQL processors + query result ─────────────────────────────────────────────
_SPARQL = "purrdf.compat.rdflib.plugins.sparqlprocessor"
register("sparql", Result, _SPARQL, "SPARQLResult")
register("sparql", Processor, _SPARQL, "SPARQLProcessor")
register("sparql", UpdateProcessor, _SPARQL, "SPARQLUpdateProcessor")

# ── SPARQL result serializers / parsers (native codecs) ──────────────────────────
# JSON/XML/CSV/TSV serializers and JSON/XML parsers route through the native
# purrdf-sparql-results crate; CSV/TSV parsing and the txt table are implemented
# in purrdf.compat.rdflib.plugins.sparqlresults.
_RESULTS = "purrdf.compat.rdflib.plugins.sparqlresults"
for _name in ("json", "application/sparql-results+json"):
    register(_name, ResultSerializer, _RESULTS, "JSONResultSerializer")
    register(_name, ResultParser, _RESULTS, "JSONResultParser")
for _name in ("xml", "application/sparql-results+xml"):
    register(_name, ResultSerializer, _RESULTS, "XMLResultSerializer")
    register(_name, ResultParser, _RESULTS, "XMLResultParser")
for _name in ("csv", "text/csv"):
    register(_name, ResultSerializer, _RESULTS, "CSVResultSerializer")
    register(_name, ResultParser, _RESULTS, "CSVResultParser")
for _name in ("tsv", "text/tab-separated-values"):
    register(_name, ResultSerializer, _RESULTS, "TSVResultSerializer")
    register(_name, ResultParser, _RESULTS, "TSVResultParser")
for _name in ("txt",):
    register(_name, ResultSerializer, _RESULTS, "TXTResultSerializer")

# Mirror RDFLib: every graph-parser name is also a graph ResultParser, so a
# CONSTRUCT/DESCRIBE result document can be parsed back into a graph.
_graph_parsers = {plugin.name for plugin in plugins(kind=Parser)}
_result_parsers = {plugin.name for plugin in plugins(kind=ResultParser)}
for _name in _graph_parsers - _result_parsers:
    register(_name, ResultParser, _RESULTS, "GraphResultParser")

del _name
