# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Namespaces, the standard vocabularies, and the ``NamespaceManager`` (RDFLib parity).

``Namespace`` is a ``str`` subclass whose attribute / item access mints a
:class:`~purrdf.compat.rdflib.term.URIRef` — ``RDF.type`` →
``URIRef("…#type")`` — exactly like RDFLib. The standard vocabularies are
provided as ``Namespace`` instances seeded with their canonical base IRIs.

``NamespaceManager`` mirrors RDFLib's public surface (``compute_qname``,
``qname``, ``expand_curie``, ``curie``, ``normalizeUri``, ``absolutize``,
``bind`` with its auto-suffix collision semantics). It keeps its own in-memory
prefix store (rdflib delegates to ``graph.store``); the trie/qname helpers are
ported verbatim from rdflib so ``compute_qname`` matches byte-for-byte.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any, Protocol
from unicodedata import category
from urllib.parse import urldefrag, urljoin

from .term import URIRef, Variable


class Namespace(str):
    """A namespace IRI prefix whose attribute/item access yields a ``URIRef``."""

    __slots__ = ()

    def __new__(cls, value: str) -> Namespace:
        """Construct from the base IRI string."""
        return str.__new__(cls, value)

    def term(self, name: str) -> URIRef:
        """Return ``URIRef(base + name)``."""
        return URIRef(str(self) + (name if isinstance(name, str) else ""))

    # `title` is a real ``str`` method, so normal attribute lookup would shadow the
    # vocabulary term (e.g. ``DCTERMS.title``). Override it to a URIRef, exactly as
    # RDFLib's Namespace does, so the one common collision resolves correctly.
    @property
    def title(self) -> URIRef:  # type: ignore[override]
        """Return ``URIRef(base + "title")`` (RDFLib parity for ``DCTERMS.title``)."""
        return URIRef(str(self) + "title")

    def __getitem__(self, key: str) -> URIRef:  # type: ignore[override]
        """Return ``URIRef(base + key)`` — supports names that are not identifiers."""
        return self.term(key)

    def __getattr__(self, name: str) -> URIRef:
        """Return ``URIRef(base + name)`` for a vocabulary member access."""
        if name.startswith("__"):
            raise AttributeError(name)
        return self.term(name)

    def __repr__(self) -> str:
        """Return ``Namespace('…')`` (RDFLib parity)."""
        return f"Namespace({str.__repr__(self)})"

    def __contains__(self, ref: object) -> bool:  # type: ignore[override]
        """Return whether ``ref`` (a URI) starts with this namespace's base IRI."""
        return str(ref).startswith(str(self))


class URIPattern(str):
    """A URI template filled via ``%`` or ``.format`` (RDFLib parity)."""

    __slots__ = ()

    def __new__(cls, value: str) -> URIPattern:
        """Construct from the template string."""
        return str.__new__(cls, value)

    def __mod__(self, *args: object, **kwargs: object) -> URIRef:  # type: ignore[override]
        """Return ``URIRef`` of the ``%``-formatted template."""
        return URIRef(str.__mod__(self, *args, **kwargs))

    def format(self, *args: object, **kwargs: object) -> URIRef:  # type: ignore[override]
        """Return ``URIRef`` of the ``str.format``-filled template."""
        return URIRef(str.format(self, *args, **kwargs))

    def __repr__(self) -> str:
        """Return ``URIPattern('…')`` (RDFLib parity)."""
        return f"URIPattern({str.__repr__(self)})"


# ── DefinedNamespace / ClosedNamespace (metaclass-driven closed vocabularies) ────

# Reserved metaclass attributes that must always raise, never mint a term.
_DFNS_RESERVED_ATTRS: frozenset[str] = frozenset(
    {"__slots__", "_NS", "_warn", "_fail", "_extras", "_underscore_num"}
)
# Names probed by third-party libraries that must never mint a term.
_IGNORED_ATTR_LOOKUP: frozenset[str] = frozenset(
    {"_pytestfixturefunction", "_partialmethod"}
)


class DefinedNamespaceMeta(type):
    """Metaclass minting ``URIRef`` terms for a :class:`DefinedNamespace`."""

    _NS: Namespace
    _warn: bool = True
    _fail: bool = False  # True mimics ClosedNamespace (raise on unknown terms)
    _extras: list[str] = []  # non-identifier members
    _underscore_num: bool = False  # allow ``_n`` members

    def __getitem__(cls, name: str, default: object = None) -> URIRef:
        """Return ``URIRef(base + name)``, honoring ``_fail``/``_warn``."""
        name = str(name)
        if name in _DFNS_RESERVED_ATTRS:
            raise KeyError(
                f"DefinedNamespace like object has no access item named {name!r}"
            )
        if name in _IGNORED_ATTR_LOOKUP:
            raise KeyError(name)
        if cls._fail and name not in cls:
            raise AttributeError(f"term '{name}' not in namespace '{cls._NS}'")
        return cls._NS[name]

    def __getattr__(cls, name: str) -> URIRef:
        """Return the term for ``name`` (attribute access → item access)."""
        if name in _IGNORED_ATTR_LOOKUP:
            raise AttributeError(name)
        if name in _DFNS_RESERVED_ATTRS:
            raise AttributeError(
                f"DefinedNamespace like object has no attribute {name!r}"
            )
        if name.startswith("__"):
            return super().__getattribute__(name)  # type: ignore[no-any-return]
        return cls.__getitem__(name)

    def __repr__(cls) -> str:
        """Return ``Namespace('…')`` (RDFLib parity)."""
        try:
            return f"Namespace({cls._NS!r})"
        except AttributeError:
            return "Namespace(<DefinedNamespace>)"

    def __str__(cls) -> str:
        """Return the base IRI string."""
        try:
            return str(cls._NS)
        except AttributeError:
            return "<DefinedNamespace>"

    def __add__(cls, other: str) -> URIRef:
        """Return ``URIRef(base + other)``."""
        return cls.__getitem__(other)

    def __contains__(cls, item: object) -> bool:
        """Return whether ``item`` (a term or full IRI) belongs to this namespace."""
        try:
            this_ns = cls._NS
        except AttributeError:
            return False
        item_str = str(item)
        if item_str.startswith(str(this_ns)):
            item_str = item_str[len(str(this_ns)) :]
        return any(
            item_str in getattr(c, "__annotations__", {})
            or item_str in c._extras
            or (cls._underscore_num and item_str[:1] == "_" and item_str[1:].isdigit())
            for c in cls.mro()
            if isinstance(c, DefinedNamespaceMeta)
        )

    def __dir__(cls) -> list[str]:  # type: ignore[override]
        """Return the full member URIRefs (RDFLib parity for ``dir(ns)``)."""
        try:
            this_ns = cls._NS
        except AttributeError:
            return []
        members: set[str] = set()
        for c in cls.mro():
            if not isinstance(c, DefinedNamespaceMeta):
                continue
            members.update(getattr(c, "__annotations__", {}))
            members.update(c._extras)
        if cls._underscore_num:
            members = {name for name in members if not (name[:1] == "_" and name[1:].isdigit())}
        members -= _DFNS_RESERVED_ATTRS
        return sorted(str(this_ns[name]) for name in members)


class DefinedNamespace(metaclass=DefinedNamespaceMeta):
    """A namespace with an enumerated list of members (RDFLib parity)."""

    __slots__: tuple[str, ...] = ()

    def __init__(self) -> None:
        """Refuse instantiation — a DefinedNamespace is used via the class."""
        raise TypeError("namespace may not be instantiated")


class ClosedNamespace(Namespace):
    """A namespace whose members are a closed list; unknown terms raise."""

    __slots__ = ()
    # Keyed off the base-IRI string because ``str.__slots__`` forbids instance dicts.
    _registry: dict[str, dict[str, URIRef]] = {}

    def __new__(cls, uri: str, terms: list[str]) -> ClosedNamespace:
        """Construct with a fixed set of member ``terms``."""
        rt: ClosedNamespace = str.__new__(cls, uri)
        ClosedNamespace._registry[str(rt)] = {t: URIRef(str(rt) + t) for t in terms}
        return rt

    @property
    def uri(self) -> str:
        """Return the base IRI string (RDFLib back-compat)."""
        return str(self)

    def term(self, name: str) -> URIRef:
        """Return the member ``name`` or raise ``KeyError`` if not a member."""
        uri = ClosedNamespace._registry.get(str(self), {}).get(name)
        if uri is None:
            raise KeyError(f"term '{name}' not in namespace '{self}'")
        return uri

    def __getitem__(self, key: str) -> URIRef:  # type: ignore[override]
        """Return the member ``key`` (item access)."""
        return self.term(key)

    def __getattr__(self, name: str) -> URIRef:
        """Return the member ``name`` (attribute access)."""
        if name.startswith("__"):
            raise AttributeError(name)
        try:
            return self.term(name)
        except KeyError as exc:
            raise AttributeError(str(exc)) from exc

    def __repr__(self) -> str:
        """Return ``ClosedNamespace('…')``."""
        return f"{type(self).__name__}({str(self)!r})"

    def __dir__(self) -> list[str]:
        """Return the member names."""
        return list(ClosedNamespace._registry.get(str(self), {}))

    def __contains__(self, ref: object) -> bool:  # type: ignore[override]
        """Return whether ``ref`` is one of the closed member IRIs."""
        return ref in ClosedNamespace._registry.get(str(self), {}).values()


# ── qname / trie helpers (ported verbatim from rdflib for byte-exact parity) ─────

NAME_START_CATEGORIES = ["Ll", "Lu", "Lo", "Lt", "Nl"]
SPLIT_START_CATEGORIES = [*NAME_START_CATEGORIES, "Nd"]
NAME_CATEGORIES = [*NAME_START_CATEGORIES, "Mc", "Me", "Mn", "Lm", "Nd"]
ALLOWED_NAME_CHARS = ["·", "·", "-", ".", "_", "%", "(", ")"]

_INVALID_URI_CHARS = '<>" {}|\\^`'


def _is_valid_uri(uri: str) -> bool:
    """Return whether ``uri`` contains no characters illegal in an IRI."""
    return not any(c in uri for c in _INVALID_URI_CHARS)


def is_ncname(name: str) -> int:
    """Return 1 if ``name`` is a valid XML NCName, else 0."""
    if name:
        first = name[0]
        if first == "_" or category(first) in NAME_START_CATEGORIES:
            for i in range(1, len(name)):
                c = name[i]
                if category(c) not in NAME_CATEGORIES:
                    if c in ALLOWED_NAME_CHARS:
                        continue
                    return 0
            return 1
    return 0


def split_uri(uri: str, split_start: list[str] = SPLIT_START_CATEGORIES) -> tuple[str, str]:
    """Split ``uri`` into ``(namespace, local_name)`` (RDFLib algorithm)."""
    if uri.startswith(XMLNS):
        return (XMLNS, uri.split(XMLNS)[1])
    length = len(uri)
    for i in range(length):
        c = uri[-i - 1]
        if category(c) not in NAME_CATEGORIES:
            if c in ALLOWED_NAME_CHARS:
                continue
            for j in range(-1 - i, length):
                if category(uri[j]) in split_start or uri[j] == "_":
                    ns = uri[:j]
                    if not ns:
                        break
                    ln = uri[j:]
                    return (ns, ln)
            break
    raise ValueError(f"Can't split '{uri}'")


def insert_trie(trie: dict[str, Any], value: str) -> dict[str, Any]:
    """Insert ``value`` into ``trie``, returning its subtree."""
    if value in trie:
        return trie[value]  # type: ignore[no-any-return]
    multi_check = False
    for key in tuple(trie.keys()):
        if len(value) > len(key) and value.startswith(key):
            return insert_trie(trie[key], value)
        if key.startswith(value):
            if not multi_check:
                trie[value] = {}
                multi_check = True
            dict_ = trie.pop(key)
            trie[value][key] = dict_
    if value not in trie:
        trie[value] = {}
    return trie[value]  # type: ignore[no-any-return]


def insert_strie(strie: dict[str, Any], trie: dict[str, Any], value: str) -> None:
    """Memoize ``value``'s subtree into ``strie`` via :func:`insert_trie`."""
    if value not in strie:
        strie[value] = insert_trie(trie, value)


def get_longest_namespace(trie: dict[str, Any], value: str) -> str | None:
    """Return the longest key in ``trie`` that is a prefix of ``value``."""
    for key in trie:
        if value.startswith(key):
            out = get_longest_namespace(trie[key], value)
            return key if out is None else out
    return None


# ── the in-memory prefix store (rdflib delegates this to the graph's store) ──────


def _coalesce(*args: str | None, default: str | None = None) -> str | None:
    """Return the first non-``None`` argument, or ``default``."""
    for arg in args:
        if arg is not None:
            return arg
    return default


class _PrefixStore:
    """A minimal prefix ↔ namespace registry mirroring rdflib's Memory store."""

    def __init__(self) -> None:
        """Start empty."""
        self.__namespace: dict[str, URIRef] = {}
        self.__prefix: dict[URIRef, str] = {}

    def bind(self, prefix: str, namespace: URIRef, override: bool = True) -> None:
        """Bind ``prefix`` ↔ ``namespace`` (identical to rdflib's Memory.bind)."""
        bound_namespace = self.__namespace.get(prefix)
        bound_prefix = _coalesce(
            self.__prefix.get(namespace),
            self.__prefix.get(bound_namespace) if bound_namespace is not None else None,
        )
        if override:
            if bound_prefix is not None:
                del self.__namespace[bound_prefix]
            if bound_namespace is not None:
                del self.__prefix[bound_namespace]
            self.__prefix[namespace] = prefix
            self.__namespace[prefix] = namespace
        else:
            key_ns = URIRef(_coalesce(bound_namespace, namespace) or "")
            self.__prefix[key_ns] = _coalesce(bound_prefix, default=prefix) or prefix
            key_pfx = _coalesce(bound_prefix, prefix) or prefix
            self.__namespace[key_pfx] = URIRef(
                _coalesce(bound_namespace, default=namespace) or namespace
            )

    def namespace(self, prefix: str) -> URIRef | None:
        """Return the namespace bound to ``prefix`` (or ``None``)."""
        return self.__namespace.get(prefix)

    def prefix(self, namespace: URIRef) -> str | None:
        """Return the prefix bound to ``namespace`` (or ``None``)."""
        return self.__prefix.get(namespace)

    def namespaces(self) -> list[tuple[str, URIRef]]:
        """Yield the bound ``(prefix, namespace)`` pairs."""
        return list(self.__namespace.items())


class _SupportsStore(Protocol):
    """The store surface a :class:`NamespaceManager` needs from a graph."""

    def bind(self, prefix: str, namespace: URIRef, override: bool = ...) -> None: ...
    def namespace(self, prefix: str) -> URIRef | None: ...
    def prefix(self, namespace: URIRef) -> str | None: ...
    def namespaces(self) -> list[tuple[str, URIRef]]: ...


class NamespaceManager:
    """A prefix registry mirroring RDFLib's ``NamespaceManager`` public surface."""

    def __init__(
        self, graph: object | None = None, bind_namespaces: str = "rdflib"
    ) -> None:
        """Create a manager, binding the requested default namespace set.

        ``bind_namespaces`` selects ``"none"``, ``"core"`` (owl/rdf/rdfs/xsd/xml)
        or ``"rdflib"`` (all shipped vocabularies — the default, matching rdflib).
        """
        self.graph = graph
        self._prefix_store = _PrefixStore()
        self.__cache: dict[str, tuple[str, URIRef, str]] = {}
        self.__cache_strict: dict[str, tuple[str, URIRef, str]] = {}
        self.__strie: dict[str, Any] = {}
        self.__trie: dict[str, Any] = {}
        if bind_namespaces == "none":
            pass
        elif bind_namespaces == "core":
            for prefix, ns in _NAMESPACE_PREFIXES_CORE.items():
                self.bind(prefix, ns)
        elif bind_namespaces == "rdflib":
            for prefix, ns in _NAMESPACE_PREFIXES_RDFLIB.items():
                self.bind(prefix, ns)
            for prefix, ns in _NAMESPACE_PREFIXES_CORE.items():
                self.bind(prefix, ns)
        else:
            raise ValueError(f"unsupported namespace set {bind_namespaces}")

    @property
    def store(self) -> _SupportsStore:
        """The backing prefix store (rdflib compat shim)."""
        return self._prefix_store

    def __contains__(self, ref: object) -> bool:
        """Return whether ``ref`` starts with any bound namespace."""
        return any(str(ref).startswith(ns) for _prefix, ns in self.namespaces())

    def reset(self) -> None:
        """Clear the qname caches and rebuild the namespace trie."""
        self.__cache = {}
        self.__strie = {}
        self.__trie = {}
        for _p, n in self.namespaces():
            insert_trie(self.__trie, str(n))

    def qname(self, uri: str) -> str:
        """Return ``prefix:local`` (or bare ``local`` for the empty prefix)."""
        prefix, _namespace, name = self.compute_qname(uri)
        return name if prefix == "" else ":".join((prefix, name))

    def qname_strict(self, uri: str) -> str:  # noqa: N802 - RDFLib API name
        """Like :meth:`qname` but forcing a strict (NCName) local part."""
        prefix, _namespace, name = self.compute_qname_strict(uri)
        return name if prefix == "" else ":".join((prefix, name))

    def curie(self, uri: str, generate: bool = True) -> str:
        """Return a CURIE for ``uri`` (always includes the colon)."""
        prefix, _namespace, name = self.compute_qname(uri, generate=generate)
        return ":".join((prefix, name))

    def normalizeUri(self, rdfTerm: str) -> str:  # noqa: N802, N803 - RDFLib API name
        """Return ``prefix:local`` for ``rdfTerm``, else its N3 ``<uri>`` form."""
        try:
            namespace, _name = split_uri(str(rdfTerm))
            if namespace not in self.__strie:
                insert_strie(self.__strie, self.__trie, str(namespace))
            namespace = URIRef(str(namespace))
        except Exception:  # noqa: BLE001 - RDFLib swallows every split failure here
            if isinstance(rdfTerm, Variable):
                return f"?{rdfTerm}"
            return f"<{rdfTerm}>"
        prefix = self.store.prefix(namespace)
        if prefix is None and isinstance(rdfTerm, Variable):
            return f"?{rdfTerm}"
        if prefix is None:
            return f"<{rdfTerm}>"
        parts = self.compute_qname(str(rdfTerm))
        return ":".join([parts[0], parts[-1]])

    def compute_qname(self, uri: str, generate: bool = True) -> tuple[str, URIRef, str]:
        """Return ``(prefix, namespace, local)`` for ``uri`` (rdflib algorithm)."""
        prefix: str | None
        if uri not in self.__cache:
            if not _is_valid_uri(uri):
                raise ValueError(
                    f'"{uri}" does not look like a valid URI, cannot serialize this. '
                    "Did you want to urlencode it?"
                )
            try:
                namespace, name = split_uri(uri)
            except ValueError as exc:
                namespace = URIRef(uri)
                prefix = self.store.prefix(URIRef(namespace))
                name = ""
                if not prefix:
                    raise exc
            if namespace not in self.__strie:
                insert_strie(self.__strie, self.__trie, namespace)
            if self.__strie[namespace]:
                pl_namespace = get_longest_namespace(self.__strie[namespace], uri)
                if pl_namespace is not None:
                    namespace = pl_namespace
                    name = uri[len(namespace) :]
            namespace = URIRef(namespace)
            prefix = self.store.prefix(namespace)
            if prefix is None:
                if not generate:
                    raise KeyError(
                        f"No known prefix for {namespace} and generate=False"
                    )
                num = 1
                while True:
                    prefix = f"ns{num}"
                    if not self.store.namespace(prefix):
                        break
                    num += 1
                self.bind(prefix, namespace)
            self.__cache[uri] = (prefix, URIRef(namespace), name)
        return self.__cache[uri]

    def compute_qname_strict(
        self, uri: str, generate: bool = True
    ) -> tuple[str, str, str]:
        """Like :meth:`compute_qname` but the local part must be a strict NCName."""
        namespace: str
        prefix: str | None
        prefix, namespace, name = self.compute_qname(uri, generate)
        if is_ncname(str(name)):
            return prefix, str(namespace), name
        if uri not in self.__cache_strict:
            try:
                namespace, name = split_uri(uri, NAME_START_CATEGORIES)
            except ValueError as exc:
                raise ValueError(
                    "This graph cannot be serialized to a strict format "
                    f"because there is no valid way to shorten {uri}"
                ) from exc
            if namespace not in self.__strie:
                insert_strie(self.__strie, self.__trie, namespace)
            namespace = URIRef(namespace)
            prefix = self.store.prefix(namespace)
            if prefix is None:
                if not generate:
                    raise KeyError(
                        f"No known prefix for {namespace} and generate=False"
                    )
                num = 1
                while True:
                    prefix = f"ns{num}"
                    if not self.store.namespace(prefix):
                        break
                    num += 1
                self.bind(prefix, namespace)
            self.__cache_strict[uri] = (prefix, URIRef(namespace), name)
        cached = self.__cache_strict[uri]
        return cached[0], str(cached[1]), cached[2]

    def expand_curie(self, curie: str) -> URIRef:
        """Expand ``prefix:local`` into a full ``URIRef`` (raises if unbound)."""
        if type(curie) is not str:
            raise TypeError(
                f"Argument must be a string, not {type(curie).__name__}."
            )
        parts = curie.split(":", 1)
        if len(parts) != 2:
            raise ValueError(
                "Malformed curie argument, format should be e.g. “foaf:name”."
            )
        ns = self.store.namespace(parts[0])
        if ns is not None:
            return URIRef(f"{ns}{parts[1]}")
        raise ValueError(
            f"Prefix \"{curie.split(':')[0]}\" not bound to any namespace."
        )

    def bind(
        self,
        prefix: str | None,
        namespace: object,
        override: bool = True,
        replace: bool = False,
    ) -> None:
        """Bind ``prefix`` → ``namespace`` with rdflib's collision semantics."""
        namespace = URIRef(str(namespace))
        if prefix is None:
            prefix = ""
        elif " " in prefix:
            raise KeyError("Prefixes may not contain spaces.")

        bound_namespace = self.store.namespace(prefix)
        if bound_namespace:
            bound_namespace = URIRef(bound_namespace)
        if bound_namespace and bound_namespace != namespace:
            if replace:
                self.store.bind(prefix, namespace, override=override)
                insert_trie(self.__trie, str(namespace))
                return
            if not prefix:
                prefix = "default"
            num = 1
            while True:
                new_prefix = f"{prefix}{num}"
                tnamespace = self.store.namespace(new_prefix)
                if tnamespace and namespace == URIRef(tnamespace):
                    return  # already bound to the right namespace
                if not self.store.namespace(new_prefix):
                    break
                num += 1
            self.store.bind(new_prefix, namespace, override=override)
        else:
            bound_prefix = self.store.prefix(namespace)
            if bound_prefix is None:
                self.store.bind(prefix, namespace, override=override)
            elif bound_prefix == prefix:
                pass  # already bound
            elif override or bound_prefix.startswith("_"):
                self.store.bind(prefix, namespace, override=override)
        insert_trie(self.__trie, str(namespace))

    def namespaces(self) -> list[tuple[str, URIRef]]:
        """Return the bound ``(prefix, namespace)`` pairs."""
        return [(prefix, URIRef(ns)) for prefix, ns in self.store.namespaces()]

    def absolutize(self, uri: str, defrag: int = 1) -> URIRef:
        """Resolve ``uri`` against the current working directory URI."""
        base = Path.cwd().as_uri()
        result = urljoin(f"{base}/", uri, allow_fragments=not defrag)
        if defrag:
            result = urldefrag(result)[0]
        else:
            if uri and uri[-1] == "#" and result[-1] != "#":
                result = f"{result}#"
        return URIRef(result)


# ── The standard vocabularies (RDFLib-equivalent attribute access) ───────────────
#
# These are the community/W3C vocabularies rdflib ships, presented in the compat
# layer with rdflib's exact base IRIs. This is drop-in parity, NOT purrdf minting
# its own ontology — every IRI below is a well-known external vocabulary.

class _RDF(DefinedNamespace):
    """The RDF 1.1 / 1.2 vocabulary."""

    _NS = Namespace("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
    _fail = True
    _underscore_num = True

    nil: URIRef
    direction: URIRef
    first: URIRef
    language: URIRef
    object: URIRef
    predicate: URIRef
    rest: URIRef
    subject: URIRef
    type: URIRef
    value: URIRef
    Alt: URIRef
    Bag: URIRef
    CompoundLiteral: URIRef
    List: URIRef
    Property: URIRef
    Seq: URIRef
    Statement: URIRef
    HTML: URIRef
    JSON: URIRef
    PlainLiteral: URIRef
    XMLLiteral: URIRef
    langString: URIRef


RDF = _RDF
RDFS = Namespace("http://www.w3.org/2000/01/rdf-schema#")
OWL = Namespace("http://www.w3.org/2002/07/owl#")
XSD = Namespace("http://www.w3.org/2001/XMLSchema#")
XMLNS = Namespace("http://www.w3.org/XML/1998/namespace")
SKOS = Namespace("http://www.w3.org/2004/02/skos/core#")
DC = Namespace("http://purl.org/dc/elements/1.1/")
DCTERMS = Namespace("http://purl.org/dc/terms/")
DCAM = Namespace("http://purl.org/dc/dcam/")
DCMITYPE = Namespace("http://purl.org/dc/dcmitype/")
FOAF = Namespace("http://xmlns.com/foaf/0.1/")
DCAT = Namespace("http://www.w3.org/ns/dcat#")
VOID = Namespace("http://rdfs.org/ns/void#")
SH = Namespace("http://www.w3.org/ns/shacl#")
SDO = Namespace("https://schema.org/")
PROV = Namespace("http://www.w3.org/ns/prov#")
GEO = Namespace("http://www.opengis.net/ont/geosparql#")
TIME = Namespace("http://www.w3.org/2006/time#")
ORG = Namespace("http://www.w3.org/ns/org#")
QB = Namespace("http://purl.org/linked-data/cube#")
CSVW = Namespace("http://www.w3.org/ns/csvw#")
ODRL2 = Namespace("http://www.w3.org/ns/odrl/2/")
PROF = Namespace("http://www.w3.org/ns/dx/prof/")
VANN = Namespace("http://purl.org/vocab/vann/")
WGS = Namespace("https://www.w3.org/2003/01/geo/wgs84_pos#")
BRICK = Namespace("https://brickschema.org/schema/Brick#")
DOAP = Namespace("http://usefulinc.com/ns/doap#")
SOSA = Namespace("http://www.w3.org/ns/sosa/")
SSN = Namespace("http://www.w3.org/ns/ssn/")


# prefixes for the core Namespaces shipped with rdflib
_NAMESPACE_PREFIXES_CORE: dict[str, Namespace] = {
    "owl": OWL,
    "rdf": RDF,
    "rdfs": RDFS,
    "xsd": XSD,
    "xml": XMLNS,
}

# prefixes for all non-core Namespaces shipped with rdflib
_NAMESPACE_PREFIXES_RDFLIB: dict[str, Namespace] = {
    "brick": BRICK,
    "csvw": CSVW,
    "dc": DC,
    "dcat": DCAT,
    "dcmitype": DCMITYPE,
    "dcterms": DCTERMS,
    "dcam": DCAM,
    "doap": DOAP,
    "foaf": FOAF,
    "geo": GEO,
    "odrl": ODRL2,
    "org": ORG,
    "prof": PROF,
    "prov": PROV,
    "qb": QB,
    "schema": SDO,
    "sh": SH,
    "skos": SKOS,
    "sosa": SOSA,
    "ssn": SSN,
    "time": TIME,
    "vann": VANN,
    "void": VOID,
    "wgs": WGS,
}
