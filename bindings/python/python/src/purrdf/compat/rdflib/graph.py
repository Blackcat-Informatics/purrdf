# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""The mutable ``Graph`` facade for the purrdf rdflib compat shim.

Backed by a native :class:`purrdf.MutableDataset` COW dataset. Presents the RDFLib
``Graph`` surface the internal toolchain uses — ``parse``/``serialize``, ``add``/
``remove``, wildcard ``triples``/``value`` + the accessor family, ``query``,
``bind``/``namespace_manager``, and set algebra. ``serialize(format="turtle")``
routes through the native ``canonicalize_turtle`` (deterministic, dogfooded).

``Dataset``/``ConjunctiveGraph`` subclass ``Graph`` (mirroring RDFLib's
``Dataset is-a Graph``) so both ``isinstance(x, Dataset)`` and
``isinstance(x, Graph)`` dispatch correctly; they default to the N-Quads format.
"""

from __future__ import annotations

import builtins
import io
import random
import re
from collections.abc import Iterable, Iterator
from pathlib import Path
from typing import IO, TYPE_CHECKING, Any, overload
from urllib.parse import urljoin, urlparse

import purrdf

from .namespace import RDF, NamespaceManager
from .query import Result, ResultRow
from .term import (
    BNode,
    Identifier,
    Literal,
    URIRef,
    from_native,
    to_native,
)

if TYPE_CHECKING:
    from .collection import Collection
    from .paths import Path as PropertyPath
    from .resource import Resource

_NT = purrdf.RdfFormat.N_TRIPLES
_NQ = purrdf.RdfFormat.N_QUADS

_XSD_STRING = "http://www.w3.org/2001/XMLSchema#string"

#: RDFLib's default skolemization authority + genid base paths (rdf11 §skolemization).
_SKOLEM_AUTHORITY = "https://rdflib.github.io"
_SKOLEM_GENID = "/.well-known/genid/"
_RDFLIB_SKOLEM_GENID = "/.well-known/genid/rdflib/"

#: RDFLib's ``Dataset`` default-graph identifier (its default context's name).
DATASET_DEFAULT_GRAPH_ID = URIRef("urn:x-rdflib:default")

#: External-skolem → blank-node memo (RDFLib de-skolemizes non-rdflib genids to
#: fresh, but stable-per-URI, blank nodes; we mirror that with a process-wide map).
_EXTERNAL_SKOLEMS: dict[str, BNode] = {}


def _skolemize_bnode(
    bnode: BNode, authority: str | None = None, basepath: str | None = None
) -> URIRef:
    """Return the skolem IRI for ``bnode`` (RDFLib ``BNode.skolemize`` semantics)."""
    authority = authority or _SKOLEM_AUTHORITY
    basepath = basepath or _RDFLIB_SKOLEM_GENID
    return URIRef(urljoin(authority, basepath + str(bnode)))


def _is_rdflib_skolem(uri: str) -> bool:
    """Return whether ``uri`` is an rdflib-minted skolem IRI (round-trippable)."""
    parsed = urlparse(str(uri))
    if parsed.params or parsed.query or parsed.fragment:
        return False
    return parsed.path.rfind(_RDFLIB_SKOLEM_GENID) == 0


def _is_external_skolem(uri: str) -> bool:
    """Return whether ``uri`` is a (non-rdflib) well-known genid skolem IRI."""
    return urlparse(str(uri)).path.rfind(_SKOLEM_GENID) == 0


def _de_skolemize_uri(uri: URIRef) -> BNode:
    """Convert a skolem IRI back to its blank node (RDFLib ``de_skolemize``)."""
    if _is_rdflib_skolem(uri):
        return BNode(value=urlparse(str(uri)).path[len(_RDFLIB_SKOLEM_GENID) :])
    key = str(uri)
    bnode = _EXTERNAL_SKOLEMS.get(key)
    if bnode is None:
        bnode = BNode()
        _EXTERNAL_SKOLEMS[key] = bnode
    return bnode

#: A Turtle/TriG/N3/SPARQL prefix declaration: ``@prefix foo: <iri>`` / ``PREFIX foo: <iri>``.
_PREFIX_DECL_RE = re.compile(
    r"@?prefix\s+([^\s:]*)\s*:\s*<([^>\s]*)>", re.IGNORECASE
)


def _scan_prefixes(text: str) -> list[tuple[str, str]]:
    """Extract ``(prefix, iri)`` declarations from Turtle/TriG/N3/SPARQL source text.

    A lightweight lexical scan (no full parse): rdflib records document prefixes on
    the graph's ``NamespaceManager`` during parsing; the native parser does not yet
    surface them, so we recover them from the source. Non-textual/binary sources and
    JSON-LD/RDF/XML documents are handled by the caller (see the strict-xfail
    ledger for the residual prefix-wiring gap).
    """
    return [(m.group(1), m.group(2)) for m in _PREFIX_DECL_RE.finditer(text)]

#: A graph triple of compat terms.
_Triple = tuple[Identifier, Identifier, Identifier]
#: A wildcard triple pattern (``None`` = any).
_Pattern = tuple[Identifier | None, Identifier | None, Identifier | None]
_QuadPattern = tuple[
    Identifier | None, Identifier | None, Identifier | None, Identifier | None
]
_GraphName = Identifier | None
_LiteralBucket = tuple[str, str | None, str | None]
_LiteralQuadKey = tuple[Identifier, Identifier, _LiteralBucket, _GraphName]


def _native_subject(term: Identifier) -> purrdf.NamedNode | purrdf.BlankNode:
    """Convert a term to a native subject (IRI or blank node)."""
    native = to_native(term)
    if isinstance(native, purrdf.Literal):
        raise TypeError(f"a literal cannot be a subject: {term!r}")
    return native


def _native_predicate(term: Identifier) -> purrdf.NamedNode:
    """Convert a term to a native predicate (must be an IRI)."""
    native = to_native(term)
    if not isinstance(native, purrdf.NamedNode):
        raise TypeError(f"a predicate must be an IRI: {term!r}")
    return native


def _require(value: object) -> Identifier:
    """Assert a converted term is bound (non-``None``) and return it."""
    assert isinstance(value, Identifier)
    return value


def _graph_name_from_native(
    graph_name: purrdf.NamedNode | purrdf.BlankNode | purrdf.DefaultGraph,
) -> _GraphName:
    """Convert a native graph name to the compat quad slot value."""
    if isinstance(graph_name, purrdf.DefaultGraph):
        return None
    converted = from_native(graph_name)
    assert converted is None or isinstance(converted, Identifier)
    return converted


def _context_graph_name(context: object) -> _GraphName:
    """Return the graph-name slot for a quad context (a ``Graph`` or identifier).

    RDFLib's default-graph identifier (``urn:x-rdflib:default``) maps to the
    unnamed default graph (``None``), matching the native ``DefaultGraph`` slot.
    """
    if context is None:
        return None
    if isinstance(context, Graph):
        return context._graph_name
    if context == DATASET_DEFAULT_GRAPH_ID:
        return None
    if isinstance(context, Identifier):
        return context
    return URIRef(str(context))


def _literal_bucket(literal: Literal) -> _LiteralBucket:
    """Return the native-equivalence bucket for shim-side literal provenance."""
    datatype = None if literal.datatype is None else str(literal.datatype)
    language = None if literal.language is None else literal.language.lower()
    if language is None and datatype in (None, _XSD_STRING):
        datatype = _XSD_STRING
    return (str(literal), datatype, language)


def _literal_quad_key(
    subject: Identifier,
    predicate: Identifier,
    literal: Literal,
    graph_name: _GraphName,
) -> _LiteralQuadKey:
    """Return the provenance key for a literal quad collapsed by the native IR."""
    return (subject, predicate, _literal_bucket(literal), graph_name)


def _literal_terms_snapshot(
    literal_terms: dict[_LiteralQuadKey, set[Literal]],
) -> list[tuple[Identifier, Identifier, Literal, _GraphName]]:
    """Flatten literal provenance for pickle state."""
    return [
        (subject, predicate, literal, graph_name)
        for (subject, predicate, _bucket, graph_name), literals in literal_terms.items()
        for literal in literals
    ]


def _rebuild_graph(
    cls: type[Graph],
    nquads: bytes,
    prefixes: list[tuple[str, str]],
    literal_terms: list[tuple[Identifier, Identifier, Literal, _GraphName]]
    | None = None,
) -> Graph:
    """Reconstruct a graph from its N-Quads content (the pickle restore hook)."""
    graph = cls()
    if nquads.strip():
        graph._store.load(nquads, format=_NQ)
    for subject, predicate, literal, graph_name in literal_terms or []:
        key = _literal_quad_key(subject, predicate, literal, graph_name)
        graph._literal_terms.setdefault(key, set()).add(literal)
    for prefix, namespace in prefixes:
        graph.bind(prefix, namespace)
    return graph


#: The four SPARQL query-form keywords (the first one in a query fixes its form).
_QUERY_FORM_RE = re.compile(r"\b(SELECT|ASK|CONSTRUCT|DESCRIBE)\b", re.IGNORECASE)


def _query_form(query_text: str) -> str:
    """Return the SPARQL query form (``SELECT``/``ASK``/``CONSTRUCT``/``DESCRIBE``).

    A lexical scan: the first form keyword after the (PREFIX/BASE) prologue fixes
    the form. Used to distinguish a CONSTRUCT from a DESCRIBE, since both
    materialize to the same native triple result.
    """
    match = _QUERY_FORM_RE.search(query_text)
    return match.group(1).upper() if match else "SELECT"


def _native_substitutions(
    bindings: dict[str, Identifier],
) -> dict[
    purrdf.Variable,
    purrdf.NamedNode | purrdf.BlankNode | purrdf.Literal | purrdf.Triple,
]:
    """Convert RDFLib ``initBindings`` to the native ``substitutions`` kwarg.

    Each key becomes a native :class:`purrdf.Variable` (sigil-stripped) and each
    value a native term (a bare Python value is coerced to a typed ``Literal``,
    matching RDFLib's pre-binding of non-term values).
    """
    substitutions: dict[
        purrdf.Variable,
        purrdf.NamedNode | purrdf.BlankNode | purrdf.Literal | purrdf.Triple,
    ] = {}
    for name, term in bindings.items():
        variable = purrdf.Variable(str(name).lstrip("?$"))
        if not isinstance(term, Identifier):
            term = Literal(term)
        substitutions[variable] = to_native(term)
    return substitutions


#: A ``?var``/``$var`` token, greedily matching the full variable name so
#: ``?s2`` is never mistaken for a substitution of ``?s``.
_VAR_TOKEN_RE = re.compile(r"[?$](\w+)")


def _inline_bound_variables(text: str, bindings: dict[str, Identifier]) -> str:
    """Textually substitute ``?var``/``$var`` tokens with each bound term's N3 form.

    ``Store.update`` (unlike ``Store.query``) has no native ``substitutions``
    kwarg, so an UPDATE body's ``initBindings`` cannot be pre-bound by the
    engine. UPDATE has no outer projection whose variable names must survive
    the substitution (unlike a SELECT's result columns), so literal textual
    substitution is a safe stand-in: it reproduces the same effect as pre-
    binding for the WHERE-clause matching and template instantiation UPDATE
    relies on.
    """
    literal_by_name = {
        str(name).lstrip("?$"): term.n3() for name, term in bindings.items()
    }
    return _VAR_TOKEN_RE.sub(
        lambda m: literal_by_name.get(m.group(1), m.group(0)), text
    )


def _literal_matches(
    candidate: Identifier,
    pattern: Literal,
    *,
    exact_string_provenance: bool,
) -> bool:
    """Return whether a stored literal satisfies an object literal pattern."""
    if not isinstance(candidate, Literal):
        return False
    if (
        exact_string_provenance
        and _literal_bucket(candidate) == (str(candidate), _XSD_STRING, None)
        and candidate.datatype != pattern.datatype
    ):
        return bool(candidate == pattern)
    return bool(candidate == pattern or candidate.eq(pattern))


class Graph:
    """An RDFLib-shaped mutable RDF graph over a native COW dataset."""

    def __init__(
        self,
        store: purrdf.Store | purrdf.MutableDataset | None = None,
        identifier: object | None = None,
        *,
        namespace_manager: NamespaceManager | None = None,
        base: str | None = None,
        bind_namespaces: str | None = None,
    ) -> None:
        """Create an empty graph (or import an existing native store/dataset).

        ``bind_namespaces`` is accepted for RDFLib 7.x API parity but is currently a
        no-op; namespace prefixes are managed through the graph's namespace manager.
        """
        _ = bind_namespaces
        if isinstance(store, purrdf.MutableDataset):
            self._store = store
        else:
            self._store = purrdf.MutableDataset()
            if isinstance(store, purrdf.Store):
                nquads = store.dump(format=_NQ)
                if nquads.strip():
                    self._store.load(nquads, format=_NQ)
        self._nsm = (
            namespace_manager if namespace_manager is not None else (NamespaceManager())
        )
        # RDFLib assigns a fresh BNode name when no identifier is given, and wraps
        # a bare string as a URIRef — its __hash__/__eq__ contract keys on this.
        if identifier is None:
            self.identifier: Identifier = BNode()
        elif isinstance(identifier, Identifier):
            self.identifier = identifier
        else:
            self.identifier = URIRef(str(identifier))
        self.base = base
        self._literal_terms: dict[_LiteralQuadKey, set[Literal]] = {}
        #: The named-graph slot this facade writes/reads (``None`` = default graph).
        self._graph_name: _GraphName = None
        #: Whether pattern access spans every graph (``Dataset.default_union``).
        self._any_graph = False

    def __reduce__(
        self,
    ) -> tuple[
        object,
        tuple[
            type[Graph],
            bytes,
            list[tuple[str, str]],
            list[tuple[Identifier, Identifier, Literal, _GraphName]],
        ],
    ]:
        """Pickle by content (N-Quads) — the native ``Store`` is not picklable.

        RDFLib's ``Graph`` is picklable, and the parallel generator/test runners
        ship graphs across process boundaries, so the compat ``Graph`` must be too.
        """
        return (
            _rebuild_graph,
            (
                type(self),
                self._store.dump(format=_NQ),
                [(prefix, str(ns)) for prefix, ns in self._nsm.namespaces()],
                _literal_terms_snapshot(self._literal_terms),
            ),
        )

    # ── prefixes ────────────────────────────────────────────────────────────────

    @property
    def namespace_manager(self) -> NamespaceManager:
        """The prefix registry feeding Turtle serialization."""
        return self._nsm

    def bind(
        self,
        prefix: str | None,
        namespace: object,
        *,
        override: bool = True,
        replace: bool = False,
    ) -> None:
        """Bind ``prefix`` → ``namespace`` for serialization."""
        self._nsm.bind(prefix, namespace, override=override, replace=replace)

    def namespaces(self) -> Iterator[tuple[str, URIRef]]:
        """Yield bound ``(prefix, namespace_iri)`` pairs."""
        for prefix, ns in self._nsm.namespaces():
            yield (prefix, URIRef(ns))

    # ── mutation ────────────────────────────────────────────────────────────────

    def add(self, triple: tuple[Identifier, Identifier, Identifier]) -> None:
        """Add a ``(subject, predicate, object)`` triple."""
        s, p, o = triple
        self._add_quad(s, p, o, self._graph_name)

    def addN(  # noqa: N802 - RDFLib API name
        self, quads: Iterable[tuple[Identifier, Identifier, Identifier, object]]
    ) -> None:
        """Add a sequence of ``(s, p, o, context)`` quads (RDFLib ``addN``).

        The context is the graph the triple belongs to — either a ``Graph``
        facade (its graph slot is used) or a graph-name identifier.
        """
        for s, p, o, context in quads:
            self._add_quad(s, p, o, _context_graph_name(context))

    def remove(self, triple: _Pattern) -> None:
        """Remove every triple matching the (possibly wildcard) pattern."""
        s, p, o = triple
        # Snapshot matches first — deleting while iterating the store is unsafe.
        matched = list(self.triples((s, p, o)))
        for ms, mp, mo in matched:
            self._remove_quad(ms, mp, mo, self._graph_name)

    def set(self, triple: tuple[Identifier, Identifier, Identifier]) -> None:
        """Replace all ``(s, p, *)`` objects with this single triple's object."""
        s, p, o = triple
        self.remove((s, p, None))
        self.add((s, p, o))

    # ── pattern access ────────────────────────────────────────────────────────────

    def triples(self, pattern: _Pattern) -> Iterator[_Triple]:
        """Yield triples matching the wildcard pattern (``None`` = any).

        Scoped to this facade's graph slot (``self._graph_name``); a
        ``default_union`` dataset (``self._any_graph``) spans every graph and
        de-duplicates triples that appear in more than one graph.

        A :class:`~.paths.Path` in the predicate slot is evaluated as a SPARQL
        property path (RDFLib parity: the yielded triples carry the path object
        as their predicate).
        """
        from .paths import Path as PropertyPath

        s, p, o = pattern
        if isinstance(p, PropertyPath):
            yield from self._triples_path(s, p, o)
            return
        pattern_literal = o if isinstance(o, Literal) else None
        quads = self._store.quads_for_pattern(
            None if s is None else _native_subject(s),
            None if p is None else _native_predicate(p),
            None
            if pattern_literal is not None
            else (None if o is None else to_native(o)),
            None if self._graph_name is None else _native_subject(self._graph_name),
            any_graph=self._any_graph,
        )
        seen: builtins.set[_Triple] | None = set() if self._any_graph else None
        for quad in quads:
            rs = s if s is not None else _require(from_native(quad.subject))
            rp = p if p is not None else _require(from_native(quad.predicate))
            candidate = _require(from_native(quad.object))
            qgraph = _graph_name_from_native(quad.graph_name)
            if isinstance(candidate, Literal):
                variants, exact = self._literal_variants(rs, rp, candidate, qgraph)
                for variant in variants:
                    if pattern_literal is not None and not _literal_matches(
                        variant,
                        pattern_literal,
                        exact_string_provenance=exact,
                    ):
                        continue
                    if seen is not None:
                        if (rs, rp, variant) in seen:
                            continue
                        seen.add((rs, rp, variant))
                    yield (rs, rp, variant)
            elif pattern_literal is None:
                if seen is not None:
                    if (rs, rp, candidate) in seen:
                        continue
                    seen.add((rs, rp, candidate))
                yield (rs, rp, candidate)

    def _triples_path(
        self,
        subject: Identifier | None,
        path: PropertyPath,
        object: Identifier | None,
    ) -> Iterator[tuple[Identifier, PropertyPath, Identifier]]:
        """Evaluate a SPARQL property path, yielding ``(s, path, o)`` triples.

        Translates the path to SPARQL property-path syntax and runs it as an
        internal query, binding the given endpoints and projecting the free ones
        (RDFLib's ``evalPath`` equivalent).
        """
        s_bound = isinstance(subject, Identifier)
        o_bound = isinstance(object, Identifier)
        s_term = subject.n3() if isinstance(subject, Identifier) else "?s"
        o_term = object.n3() if isinstance(object, Identifier) else "?o"
        path_n3 = path.n3()
        if isinstance(subject, Identifier) and isinstance(object, Identifier):
            if bool(self.query(f"ASK {{ {s_term} {path_n3} {o_term} }}")):
                yield (subject, path, object)
            return
        proj = " ".join(
            var for var, bound in (("?s", s_bound), ("?o", o_bound)) if not bound
        )
        query = f"SELECT {proj} WHERE {{ {s_term} {path_n3} {o_term} }}"
        for row in self.query(query):
            rs = subject if isinstance(subject, Identifier) else row["s"]
            ro = object if isinstance(object, Identifier) else row["o"]
            assert isinstance(rs, Identifier) and isinstance(ro, Identifier)
            yield (rs, path, ro)

    def __iter__(self) -> Iterator[_Triple]:
        """Iterate every triple as ``(subject, predicate, object)``."""
        yield from self.triples((None, None, None))

    def __len__(self) -> int:
        """Return the triple count."""
        return len(self._store) + sum(
            max(0, len(variants) - 1) for variants in self._literal_terms.values()
        )

    def __contains__(self, triple: _Pattern) -> bool:
        """Return whether any triple matches the pattern."""
        for _ in self.triples(triple):
            return True
        return False

    def __getitem__(self, item: Any) -> Any:
        """Slice a graph as a shortcut for :meth:`triples` (RDFLib ``__getitem__``).

        ``g[s:p:o]`` maps the slice's ``start``/``stop``/``step`` to the
        subject/predicate/object pattern; supplying only some parts returns the
        matching accessor generator (e.g. ``g[s]`` → ``predicate_objects(s)``,
        ``g[:p]`` → ``subject_objects(p)``, ``g[::o]`` → ``subject_predicates(o)``).
        """
        if isinstance(item, slice):
            s, p, o = item.start, item.stop, item.step
            if s is None and p is None and o is None:
                return self.triples((s, p, o))
            if s is None and p is None:
                return self.subject_predicates(o)
            if s is None and o is None:
                return self.subject_objects(p)
            if p is None and o is None:
                return self.predicate_objects(s)
            if s is None:
                return self.subjects(p, o)
            if p is None:
                return self.predicates(s, o)
            if o is None:
                return self.objects(s, p)
            return (s, p, o) in self
        if isinstance(item, Identifier):
            return self.predicate_objects(item)
        raise TypeError(
            "You can only index a graph by a single rdflib term or a slice of "
            "rdflib terms."
        )

    def __hash__(self) -> int:
        """Hash over the graph's identifier — mirrors RDFLib's contract."""
        return hash(self.identifier)

    def __eq__(self, other: object) -> bool:
        """Equal to another graph with the same identifier (RDFLib parity)."""
        return isinstance(other, Graph) and self.identifier == other.identifier

    def __ne__(self, other: object) -> bool:
        """Negate :meth:`__eq__`."""
        return not self == other

    def __lt__(self, other: object) -> bool:
        """Order graphs by identifier (RDFLib parity)."""
        return (other is None) or (
            isinstance(other, Graph) and self.identifier < other.identifier
        )

    def __le__(self, other: object) -> bool:
        """Less-than-or-equal by identifier."""
        return self < other or self == other

    def __gt__(self, other: object) -> bool:
        """Strictly greater by identifier."""
        return (isinstance(other, Graph) and self.identifier > other.identifier) or (
            other is not None and not isinstance(other, Graph)
        )

    def __ge__(self, other: object) -> bool:
        """Greater-than-or-equal by identifier."""
        return self > other or self == other

    def value(
        self,
        subject: Identifier | None = None,
        predicate: Identifier | None = None,
        object: Identifier | None = None,
        default: Identifier | None = None,
        any: bool = True,
    ) -> Identifier | None:
        """Return the single unspecified term of the first matching triple."""
        for s, p, o in self.triples((subject, predicate, object)):
            if object is None:
                return o
            if subject is None:
                return s
            return p
        return default

    def subjects(
        self, predicate: Identifier | None = None, object: Identifier | None = None
    ) -> Iterator[Identifier]:
        """Yield subjects of triples matching ``(*, predicate, object)``."""
        for s, _p, _o in self.triples((None, predicate, object)):
            yield s

    def predicates(
        self, subject: Identifier | None = None, object: Identifier | None = None
    ) -> Iterator[Identifier]:
        """Yield predicates of triples matching ``(subject, *, object)``."""
        for _s, p, _o in self.triples((subject, None, object)):
            yield p

    def objects(
        self, subject: Identifier | None = None, predicate: Identifier | None = None
    ) -> Iterator[Identifier]:
        """Yield objects of triples matching ``(subject, predicate, *)``."""
        for _s, _p, o in self.triples((subject, predicate, None)):
            yield o

    def subject_objects(
        self, predicate: Identifier | None = None
    ) -> Iterator[tuple[Identifier, Identifier]]:
        """Yield ``(subject, object)`` pairs for ``(*, predicate, *)``."""
        for s, _p, o in self.triples((None, predicate, None)):
            yield (s, o)

    def subject_predicates(
        self, object: Identifier | None = None
    ) -> Iterator[tuple[Identifier, Identifier]]:
        """Yield ``(subject, predicate)`` pairs for ``(*, *, object)``."""
        for s, p, _o in self.triples((None, None, object)):
            yield (s, p)

    def predicate_objects(
        self, subject: Identifier | None = None
    ) -> Iterator[tuple[Identifier, Identifier]]:
        """Yield ``(predicate, object)`` pairs for ``(subject, *, *)``."""
        for _s, p, o in self.triples((subject, None, None)):
            yield (p, o)

    def items(self, list_node: Identifier) -> Iterator[Identifier]:
        """Yield the members of the ``rdf:List`` anchored at ``list_node``."""
        node: Identifier | None = list_node
        while node is not None and node != RDF.nil:
            first = self.value(node, RDF.first)
            if first is not None:
                yield first
            node = self.value(node, RDF.rest)

    def transitive_objects(
        self,
        subject: Identifier,
        predicate: Identifier,
        remember: builtins.set[Identifier] | None = None,
    ) -> Iterator[Identifier]:
        """Yield ``subject`` and every object reachable via ``predicate`` (RDFLib)."""
        if remember is None:
            remember = set[Identifier]()
        if subject in remember:
            return
        remember.add(subject)
        yield subject
        for obj in self.objects(subject, predicate):
            yield from self.transitive_objects(obj, predicate, remember)

    def transitive_subjects(
        self,
        predicate: Identifier,
        object: Identifier,
        remember: builtins.set[Identifier] | None = None,
    ) -> Iterator[Identifier]:
        """Yield ``object`` and every subject reaching it via ``predicate`` (RDFLib)."""
        if remember is None:
            remember = set[Identifier]()
        if object in remember:
            return
        remember.add(object)
        yield object
        for subj in self.subjects(predicate, object):
            yield from self.transitive_subjects(predicate, subj, remember)

    def isomorphic(self, other: Graph) -> bool:
        """Return whether this graph is isomorphic to ``other`` (RDFC-1.0)."""
        from .compare import isomorphic

        return isomorphic(self, other)

    # ── graph topology / views ─────────────────────────────────────────────────────

    def all_nodes(self) -> builtins.set[Identifier]:
        """Return every node appearing as a subject or object (RDFLib parity)."""
        nodes: builtins.set[Identifier] = set(self.objects())
        nodes.update(self.subjects())
        return nodes

    def connected(self) -> bool:
        """Return whether the graph is connected (treated as undirected)."""
        all_nodes = list(self.all_nodes())
        if not all_nodes:
            return False
        discovered: list[Identifier] = []
        visiting = [all_nodes[random.randrange(len(all_nodes))]]
        while visiting:
            x = visiting.pop()
            if x not in discovered:
                discovered.append(x)
            for new_x in self.objects(subject=x):
                if new_x not in discovered and new_x not in visiting:
                    visiting.append(new_x)
            for new_x in self.subjects(object=x):
                if new_x not in discovered and new_x not in visiting:
                    visiting.append(new_x)
        return len(all_nodes) == len(discovered)

    def collection(self, identifier: Identifier) -> Collection:
        """Return a :class:`~.collection.Collection` over the list at ``identifier``."""
        from .collection import Collection

        return Collection(self, identifier)

    def resource(self, identifier: Identifier | str) -> Resource:
        """Return a :class:`~.resource.Resource` bound to ``identifier``."""
        from .resource import Resource

        if not isinstance(identifier, Identifier):
            identifier = URIRef(str(identifier))
        return Resource(self, identifier)

    # ── skolemization ───────────────────────────────────────────────────────────────

    def _process_skolem_tuples(
        self, target: Graph, func: Any
    ) -> None:
        """Copy every triple through ``func`` into ``target`` (RDFLib helper)."""
        for t in self.triples((None, None, None)):
            target.add(func(t))

    def skolemize(
        self,
        new_graph: Graph | None = None,
        bnode: BNode | None = None,
        authority: str | None = None,
        basepath: str | None = None,
    ) -> Graph:
        """Replace blank nodes with skolem IRIs (RDFLib ``skolemize``).

        With ``bnode`` given, only that blank node is skolemized; otherwise every
        blank node is. The result is written to ``new_graph`` (or a fresh graph).
        """

        def one(t: _Triple) -> _Triple:
            s, p, o = t
            if s == bnode and isinstance(s, BNode):
                s = _skolemize_bnode(s, authority, basepath)
            if o == bnode and isinstance(o, BNode):
                o = _skolemize_bnode(o, authority, basepath)
            return (s, p, o)

        def each(t: _Triple) -> _Triple:
            s, p, o = t
            if isinstance(s, BNode):
                s = _skolemize_bnode(s, authority, basepath)
            if isinstance(o, BNode):
                o = _skolemize_bnode(o, authority, basepath)
            return (s, p, o)

        retval = Graph() if new_graph is None else new_graph
        if bnode is None:
            self._process_skolem_tuples(retval, each)
        elif isinstance(bnode, BNode):
            self._process_skolem_tuples(retval, one)
        return retval

    def de_skolemize(
        self, new_graph: Graph | None = None, uriref: URIRef | None = None
    ) -> Graph:
        """Replace skolem IRIs with blank nodes (RDFLib ``de_skolemize``).

        With ``uriref`` given, only that skolem IRI is reverted; otherwise every
        rdflib/well-known genid skolem IRI is. Writes to ``new_graph`` or a fresh
        graph.
        """

        def one(t: _Triple) -> _Triple:
            s, p, o = t
            if s == uriref and isinstance(s, URIRef):
                s = _de_skolemize_uri(s)
            if o == uriref and isinstance(o, URIRef):
                o = _de_skolemize_uri(o)
            return (s, p, o)

        def each(t: _Triple) -> _Triple:
            s, p, o = t
            if isinstance(s, URIRef) and (
                _is_rdflib_skolem(s) or _is_external_skolem(s)
            ):
                s = _de_skolemize_uri(s)
            if isinstance(o, URIRef) and (
                _is_rdflib_skolem(o) or _is_external_skolem(o)
            ):
                o = _de_skolemize_uri(o)
            return (s, p, o)

        retval = Graph() if new_graph is None else new_graph
        if uriref is None:
            self._process_skolem_tuples(retval, each)
        elif isinstance(uriref, URIRef):
            self._process_skolem_tuples(retval, one)
        return retval

    def cbd(
        self,
        resource: Identifier,
        *,
        target_graph: Graph | None = None,
        include_reifications: bool = True,
    ) -> Graph:
        """Return the Concise Bounded Description of ``resource`` (RDFLib ``cbd``)."""
        subgraph = Graph() if target_graph is None else target_graph

        def add_to_cbd(uri: Identifier) -> None:
            reif_index: dict[_Triple, builtins.set[Identifier]] = {}
            if include_reifications:
                for stmt in self.subjects(RDF.subject, uri):
                    p = self.value(stmt, RDF.predicate)
                    o = self.value(stmt, RDF.object)
                    if p is not None and o is not None:
                        reif_index.setdefault((uri, p, o), set()).add(stmt)
            for s, p, o in self.triples((uri, None, None)):
                subgraph.add((s, p, o))
                if type(o) is BNode and (o, None, None) not in subgraph:
                    add_to_cbd(o)
                if include_reifications:
                    for stmt in reif_index.get((s, p, o), set()):
                        if (stmt, None, None) not in subgraph:
                            add_to_cbd(stmt)

        add_to_cbd(resource)
        return subgraph

    # ── name resolution / persistence stubs ─────────────────────────────────────────

    def qname(self, uri: str) -> str:
        """Return the ``prefix:local`` form of ``uri`` (delegates to the nsm)."""
        return self._nsm.qname(uri)

    def compute_qname(
        self, uri: str, generate: bool = True
    ) -> tuple[str, URIRef, str]:
        """Return the ``(prefix, namespace, local)`` split of ``uri``."""
        return self._nsm.compute_qname(uri, generate)

    def absolutize(self, uri: str, defrag: int = 1) -> URIRef:
        """Return ``uri`` as an absolute IRI (delegates to the nsm)."""
        return self._nsm.absolutize(uri, defrag)

    def open(
        self, configuration: str | tuple[str, str], create: bool = False
    ) -> int | None:
        """Open the backing store (no-op: the native COW store is always open)."""
        return None

    def close(self, commit_pending_transaction: bool = False) -> None:
        """Close the backing store (no-op for the in-memory native store)."""
        return None

    def commit(self) -> Graph:
        """Commit pending writes (no-op: native writes are immediate)."""
        return self

    def rollback(self) -> Graph:
        """Roll back pending writes (no-op: native writes are immediate)."""
        return self

    def destroy(self, configuration: str) -> Graph:
        """Destroy the store (no-op stub for RDFLib persistence parity)."""
        return self

    # ── parse / serialize ─────────────────────────────────────────────────────────

    def _bind_source_prefixes(self, payload: bytes) -> None:
        """Bind any ``@prefix``/``PREFIX`` declarations found in ``payload`` text."""
        try:
            text = payload.decode("utf-8")
        except UnicodeDecodeError:
            return
        for prefix, iri in _scan_prefixes(text):
            self._nsm.bind(prefix, iri)

    def parse(
        self,
        source: object | None = None,
        publicID: str | None = None,  # noqa: N803 - RDFLib API name
        format: str | None = None,
        location: str | None = None,
        file: IO[bytes] | None = None,
        data: str | bytes | None = None,
        **kwargs: object,
    ) -> Graph:
        """Parse RDF from a path/file/``data`` into this graph (any compat format).

        The format name is resolved to a parser class through the plugin registry
        (:mod:`purrdf.compat.rdflib.plugin`) — the single source of truth for
        name → implementation. Native formats read from a filesystem path keep the
        direct-load fast-path (via the parser's ``rdf_format`` marker); every other
        source is read to a ``bytes`` payload and handed to the resolved parser.
        """
        from . import plugin
        from .parser import Parser

        f = (format or "turtle").lower()
        try:
            parser_cls = plugin.get(f, Parser)
        except plugin.PluginException as exc:
            raise ValueError(f"unsupported RDF format: {format!r}") from exc
        native_format = getattr(parser_cls, "rdf_format", None)
        prefix_bearing = getattr(parser_cls, "prefix_bearing", False)
        payload: bytes
        if data is not None:
            payload = data.encode("utf-8") if isinstance(data, str) else data
        else:
            src: object | None = source if source is not None else location
            if src is None and file is not None:
                src = file
            if src is None:
                raise ValueError("parse requires one of: source, data, location, file")
            reader = getattr(src, "read", None)
            if callable(reader):
                raw = reader()
                payload = raw.encode("utf-8") if isinstance(raw, str) else raw
            elif native_format is not None:
                # A path source for an oxigraph-native format loads directly. Recover
                # document prefixes (turtle/trig/n3) from the file text so serialize/
                # qname see them, since the native loader does not surface them.
                if prefix_bearing:
                    self._bind_source_prefixes(Path(str(src)).read_bytes())
                self._store.load(path=str(src), format=native_format)
                return self
            else:
                payload = Path(str(src)).read_bytes()
        parser_cls().parse(payload, self)
        return self

    def _dump_bytes(self, fmt: str | None) -> bytes:
        """Serialize the store to bytes in the requested format.

        The format name is resolved to a serializer class through the plugin
        registry (:mod:`purrdf.compat.rdflib.plugin`) — the single source of truth
        for name → implementation. Turtle routes through the native
        ``canonicalize_turtle``; JSON-LD-star and RDF/XML route through the
        purrdf-gts codecs; the rest dump directly via oxigraph. Every emitter is
        byte-deterministic.
        """
        from . import plugin
        from .serializer import Serializer

        f = (fmt or "turtle").lower()
        try:
            serializer_cls = plugin.get(f, Serializer)
        except plugin.PluginException as exc:
            raise ValueError(f"unsupported RDF format: {fmt!r}") from exc
        buffer = io.BytesIO()
        serializer_cls(self).serialize(buffer)
        return buffer.getvalue()

    @overload
    def serialize(
        self,
        destination: None = ...,
        *,
        format: str = ...,
        encoding: None = ...,
        **kwargs: object,
    ) -> str: ...

    @overload
    def serialize(
        self,
        destination: None = ...,
        *,
        format: str = ...,
        encoding: str,
        **kwargs: object,
    ) -> bytes: ...

    @overload
    def serialize(
        self,
        destination: str | Path | IO[bytes],
        *,
        format: str = ...,
        encoding: str | None = ...,
        **kwargs: object,
    ) -> None: ...

    def serialize(
        self,
        destination: str | Path | IO[bytes] | None = None,
        *,
        format: str = "turtle",
        encoding: str | None = None,
        **kwargs: object,
    ) -> str | bytes | None:
        """Serialize the graph; return ``str``/``bytes`` or write to ``destination``."""
        out = self._dump_bytes(format)
        if destination is None:
            return out if encoding is not None else out.decode("utf-8")
        writer = getattr(destination, "write", None)
        if callable(writer):
            writer(out)
        elif isinstance(destination, str | Path):
            Path(destination).write_bytes(out)
        else:
            raise TypeError(f"unsupported serialize destination: {destination!r}")
        return None

    # ── query ─────────────────────────────────────────────────────────────────────

    def query(
        self,
        query_object: str,
        *,
        initBindings: dict[str, Identifier] | None = None,  # noqa: N803 - RDFLib API
        initNs: dict[str, object] | None = None,  # noqa: N803 - RDFLib API
        base: str | None = None,
        extension_namespaces: list[str] | None = None,
        standpoint_predicates: tuple[str, str] | None = None,
        **kwargs: object,
    ) -> Result:
        """Run a SPARQL query; return a :class:`~.query.Result`.

        ``initBindings`` are applied through the native ``substitutions`` kwarg —
        the engine pre-binds each variable (keeping it projectable and propagating
        into ``OPTIONAL``/``MINUS``/``EXISTS``/sub-queries), matching RDFLib's
        ``initBindings`` semantics without an injected ``VALUES`` row.

        ``base`` is spliced in as a leading ``BASE`` prologue declaration so
        relative IRIs in the query text resolve against it, matching RDFLib's
        ``base`` semantics. ``extension_namespaces``/``standpoint_predicates``
        are the native engine configuration knobs (see ``Store.query``) and are
        forwarded verbatim. Any other keyword is a param the native surface
        genuinely cannot honor, so it raises rather than being swallowed.
        """
        if kwargs:
            raise TypeError(
                f"Graph.query() got unsupported keyword argument(s): {sorted(kwargs)}"
            )
        if base:
            query_object = f"BASE <{base}>\n" + query_object
        if initNs:
            prefixes = "".join(
                f"PREFIX {prefix}: <{namespace}>\n"
                for prefix, namespace in initNs.items()
            )
            query_object = prefixes + query_object
        substitutions = _native_substitutions(initBindings) if initBindings else None
        res = self._store.query(
            query_object,
            substitutions=substitutions,
            extension_namespaces=extension_namespaces,
            standpoint_predicates=standpoint_predicates,
        )
        if isinstance(res, purrdf.QueryBoolean):
            return Result("ASK", ask=bool(res))
        if isinstance(res, purrdf.QueryTriples):
            constructed = Graph()
            nt = res.serialize(_NT)
            if nt:
                constructed._store.load(nt, format=_NT)
            form = "DESCRIBE" if _query_form(query_object) == "DESCRIBE" else "CONSTRUCT"
            return Result(form, graph=constructed)
        variables = list(res.variables)
        var_names = tuple(v.value for v in variables)
        rows = [
            ResultRow(tuple(from_native(sol[v]) for v in variables), var_names)
            for sol in res
        ]
        return Result("SELECT", rows=rows, variables=var_names)

    def update(
        self,
        update_object: str,
        *,
        initBindings: dict[str, Identifier] | None = None,  # noqa: N803 - RDFLib API
        initNs: dict[str, object] | None = None,  # noqa: N803 - RDFLib API
        extension_namespaces: list[str] | None = None,
        standpoint_predicates: tuple[str, str] | None = None,
        **kwargs: object,
    ) -> None:
        """Run a SPARQL UPDATE against this graph.

        ``Store.update`` has no native ``substitutions`` kwarg (unlike
        ``Store.query``), so ``initBindings`` is honored by textually inlining
        each bound term's N3 form in place of its ``?var``/``$var`` token
        (see :func:`_inline_bound_variables`) before the update reaches the
        native engine. ``extension_namespaces``/``standpoint_predicates`` are the
        native engine configuration knobs (see ``Store.update``) and are
        forwarded verbatim. Any other keyword is a param the native surface
        genuinely cannot honor, so it raises rather than being swallowed.
        """
        if kwargs:
            raise TypeError(
                f"Graph.update() got unsupported keyword argument(s): {sorted(kwargs)}"
            )
        if initNs:
            prefixes = "".join(
                f"PREFIX {prefix}: <{namespace}>\n"
                for prefix, namespace in initNs.items()
            )
            update_object = prefixes + update_object
        if initBindings:
            update_object = _inline_bound_variables(update_object, initBindings)
        self._store.update(
            update_object,
            extension_namespaces=extension_namespaces,
            standpoint_predicates=standpoint_predicates,
        )
        self._reconcile_literal_terms()

    def _reconcile_literal_terms(self) -> None:
        """Prune literal provenance whose backing quad no longer exists.

        SPARQL ``UPDATE`` rewrites the native store out from under the shim's
        literal-variant map. Rather than discard the map wholesale (which loses
        the plain-vs-``xsd:string`` provenance for quads the update left intact),
        we keep every entry still backed by a native quad and drop the rest — so
        provenance survives an unrelated update and stays consistent with
        ``triples()``/``quads()``. See the strict-xfail ledger for the residual
        gap (variants an update *introduces* cannot be recovered post-collapse).
        """
        for key in list(self._literal_terms):
            subject, predicate, (lexical, datatype, language), graph_name = key
            if language is not None:
                probe: Literal = Literal(lexical, lang=language)
            elif datatype is not None:
                probe = Literal(lexical, datatype=URIRef(datatype))
            else:
                probe = Literal(lexical)
            native_graph = (
                purrdf.DefaultGraph()
                if graph_name is None
                else _native_subject(graph_name)
            )
            quad = purrdf.Quad(
                _native_subject(subject),
                _native_predicate(predicate),
                to_native(probe),
                native_graph,
            )
            if not self._store.contains(quad):
                del self._literal_terms[key]

    # ── set algebra ───────────────────────────────────────────────────────────────

    def __iadd__(self, other: Iterable[_Triple]) -> Graph:
        """Add every triple from ``other`` (the ``+=`` operator)."""
        for triple in other:
            self.add(triple)
        return self

    def __isub__(self, other: Iterable[_Pattern]) -> Graph:
        """Remove every triple in ``other`` (the ``-=`` operator)."""
        for triple in other:
            self.remove(triple)
        return self

    def __add__(self, other: Iterable[_Triple]) -> Graph:
        """Return a new graph = the union of this graph and ``other``."""
        result = Graph()
        for triple in self:
            result.add(triple)
        for triple in other:
            result.add(triple)
        return result

    def __sub__(self, other: Iterable[_Triple]) -> Graph:
        """Return a new graph = this graph minus the triples in ``other``."""
        result = Graph()
        removed = set(other)
        for triple in self:
            if triple not in removed:
                result.add(triple)
        return result

    def __mul__(self, other: Iterable[_Triple]) -> Graph:
        """Return a new graph = intersection of this graph and ``other``."""
        result = Graph()
        other_set = set(other)
        for triple in self:
            if triple in other_set:
                result.add(triple)
        return result

    def __xor__(self, other: Iterable[_Triple]) -> Graph:
        """Return a new graph = symmetric difference of this graph and ``other``."""
        result = Graph()
        self_set = set(self)
        other_set = set(other)
        for triple in self_set ^ other_set:
            result.add(triple)
        return result

    def _add_quad(
        self,
        subject: Identifier,
        predicate: Identifier,
        object: Identifier,
        graph_name: _GraphName,
    ) -> None:
        """Add a quad and remember exact literal provenance at the RDFLib boundary."""
        native_graph = (
            purrdf.DefaultGraph()
            if graph_name is None
            else _native_subject(graph_name)
        )
        self._store.add(
            purrdf.Quad(
                _native_subject(subject),
                _native_predicate(predicate),
                to_native(object),
                native_graph,
            )
        )
        if isinstance(object, Literal):
            key = _literal_quad_key(subject, predicate, object, graph_name)
            self._literal_terms.setdefault(key, set()).add(object)

    def _remove_quad(
        self,
        subject: Identifier,
        predicate: Identifier,
        object: Identifier,
        graph_name: _GraphName,
    ) -> None:
        """Remove one exact quad variant, preserving other literal variants."""
        should_remove_native = True
        if isinstance(object, Literal):
            key = _literal_quad_key(subject, predicate, object, graph_name)
            variants = self._literal_terms.get(key)
            if variants is not None:
                variants.discard(object)
                if variants:
                    should_remove_native = False
                else:
                    del self._literal_terms[key]
        if should_remove_native:
            native_graph = (
                purrdf.DefaultGraph()
                if graph_name is None
                else _native_subject(graph_name)
            )
            self._store.remove(
                purrdf.Quad(
                    _native_subject(subject),
                    _native_predicate(predicate),
                    to_native(object),
                    native_graph,
                )
            )

    def _literal_variants(
        self,
        subject: Identifier,
        predicate: Identifier,
        candidate: Literal,
        graph_name: _GraphName,
    ) -> tuple[tuple[Literal, ...], bool]:
        """Return literal variants and whether they came from exact provenance."""
        key = _literal_quad_key(subject, predicate, candidate, graph_name)
        variants = self._literal_terms.get(key)
        if variants is None:
            return ((candidate,), False)
        return (tuple(variants), True)


class _DatasetGraph(Graph):
    """A readable/writable :class:`Graph` view over one graph slot of a Dataset.

    Shares the parent dataset's native store and literal-provenance map, so its
    ``triples``/``add``/``remove``/``__iter__``/``__len__`` operate on exactly one
    named-graph slot (or the default graph). RDFLib's ``Dataset.graph`` /
    ``get_context`` return an object of this shape.
    """

    def __init__(self, dataset: Graph, identifier: object | None) -> None:
        """Bind to ``dataset``'s store, scoped to the graph named ``identifier``."""
        self._store = dataset._store
        self._nsm = dataset._nsm
        self._literal_terms = dataset._literal_terms
        self._dataset = dataset
        self.base = None
        self._any_graph = False
        if identifier is None or identifier == DATASET_DEFAULT_GRAPH_ID:
            self.identifier = DATASET_DEFAULT_GRAPH_ID
            self._graph_name = None
        elif isinstance(identifier, Identifier):
            self.identifier = identifier
            self._graph_name = identifier
        else:
            wrapped = URIRef(str(identifier))
            self.identifier = wrapped
            self._graph_name = wrapped

    def __len__(self) -> int:
        """Return the triple count within this graph slot alone."""
        return sum(1 for _ in self.triples((None, None, None)))


class Dataset(Graph):
    """A quad-capable graph facade (RDFLib ``Dataset``); defaults to N-Quads.

    A dataset holds one unnamed default graph plus zero or more named graphs.
    Simple triples (and ``add``) target the default graph; :meth:`graph` /
    :meth:`get_context` return :class:`_DatasetGraph` views scoped to a named
    graph. With ``default_union`` set, whole-dataset pattern access
    (``triples``) spans every graph rather than the default graph alone.
    """

    def __init__(
        self,
        store: purrdf.Store | purrdf.MutableDataset | None = None,
        default_union: bool = False,
        **kwargs: object,
    ) -> None:
        """Create an empty dataset."""
        super().__init__(store)
        self.default_union = default_union
        self._any_graph = default_union
        #: Named graphs explicitly registered (so empty ones still enumerate).
        self._graphs: builtins.set[Identifier] = set()

    @property
    def default_graph(self) -> _DatasetGraph:
        """Return a :class:`Graph`-like view of the (unnamed) default graph."""
        return _DatasetGraph(self, DATASET_DEFAULT_GRAPH_ID)

    def graph(
        self, identifier: object | None = None, base: str | None = None
    ) -> _DatasetGraph:
        """Return (and register) a :class:`Graph` view for a named graph.

        ``identifier=None`` mints a fresh skolemized blank-node graph name (RDFLib
        semantics). Passing a plain ``Graph`` copies its triples into the slot.
        """
        copy_from: Graph | None = None
        if identifier is None:
            identifier = _skolemize_bnode(BNode())
        elif isinstance(identifier, Graph) and not isinstance(identifier, Dataset):
            copy_from = identifier
            identifier = identifier.identifier
        view = _DatasetGraph(self, identifier)
        view.base = base
        if view._graph_name is not None:
            self._graphs.add(view._graph_name)
        if copy_from is not None:
            for triple in copy_from:
                view.add(triple)
        return view

    def get_context(
        self,
        identifier: object | None,
        quoted: bool = False,
        base: str | None = None,
    ) -> _DatasetGraph:
        """Return a :class:`Graph` view for ``identifier`` (without registering it)."""
        view = _DatasetGraph(self, identifier)
        view.base = base
        return view

    def add_graph(self, g: object | None) -> _DatasetGraph:
        """Register a named graph — an alias of :meth:`graph` (RDFLib parity)."""
        return self.graph(g)

    def remove_graph(self, g: object | None) -> Dataset:
        """Remove a named graph's triples and unregister it (the default persists)."""
        view = g if isinstance(g, _DatasetGraph) else self.get_context(g)
        for s, p, o in list(view.triples((None, None, None))):
            view._remove_quad(s, p, o, view._graph_name)
        if view._graph_name is not None:
            self._graphs.discard(view._graph_name)
        return self

    def _context_names(self) -> builtins.set[Identifier]:
        """Return every named-graph identifier present or explicitly registered."""
        names: builtins.set[Identifier] = set(self._graphs)
        for quad in self._store.quads_for_pattern(
            None, None, None, None, any_graph=True
        ):
            name = _graph_name_from_native(quad.graph_name)
            if name is not None:
                names.add(name)
        return names

    def contexts(
        self, triple: _Pattern | None = None
    ) -> Iterator[_DatasetGraph]:
        """Yield each graph (default + named) as a context :class:`Graph`.

        With ``triple`` given, only graphs containing a matching triple are
        yielded. The default graph is always considered (RDFLib semantics).
        """
        candidates: list[_DatasetGraph] = [self.default_graph]
        candidates.extend(
            _DatasetGraph(self, name) for name in sorted(self._context_names())
        )
        for context in candidates:
            if triple is None or triple in context:
                yield context

    def graphs(self, triple: _Pattern | None = None) -> Iterator[_DatasetGraph]:
        """Yield each graph (default + named) — RDFLib's spelling of :meth:`contexts`."""
        yield from self.contexts(triple)

    def quads(
        self, pattern: _QuadPattern | None = None
    ) -> Iterator[tuple[Identifier, Identifier, Identifier, _GraphName]]:
        """Yield ``(s, p, o, graph_name)`` quads matching ``pattern``.

        Each ``pattern`` slot is a wildcard when ``None`` (RDFLib quads() semantics).
        A ``None`` graph slot spans every graph; ``urn:x-rdflib:default`` (or a
        default-graph view) restricts to the default graph. Matching real RDFLib
        7.x, default-graph quads carry the graph name ``urn:x-rdflib:default``
        (its ``Dataset.quads`` never actually collapses that to ``None``).
        """
        ps, pp, po, pg = pattern if pattern is not None else (None, None, None, None)
        union = pg is None
        scope = None if union else _context_graph_name(pg)
        native_graph_name = None if scope is None else _native_subject(scope)
        pattern_literal = po if isinstance(po, Literal) else None
        for quad in self._store.quads_for_pattern(
            None if ps is None else _native_subject(ps),
            None if pp is None else _native_predicate(pp),
            None
            if pattern_literal is not None
            else (None if po is None else to_native(po)),
            native_graph_name,
            any_graph=union,
        ):
            gname = _graph_name_from_native(quad.graph_name)
            out_gname: _GraphName = DATASET_DEFAULT_GRAPH_ID if gname is None else gname
            s = _require(from_native(quad.subject))
            p = _require(from_native(quad.predicate))
            o = _require(from_native(quad.object))
            if ps is not None:
                s = ps
            if pp is not None:
                p = pp
            if isinstance(o, Literal):
                variants, exact = self._literal_variants(s, p, o, gname)
                for variant in variants:
                    if pattern_literal is not None and not _literal_matches(
                        variant,
                        pattern_literal,
                        exact_string_provenance=exact,
                    ):
                        continue
                    yield (s, p, variant, out_gname)
            elif pattern_literal is None:
                yield (s, p, o, out_gname)

    def __iter__(self) -> Iterator[Any]:  # type: ignore[override]
        """Iterate every quad in the dataset (RDFLib ``Dataset.__iter__``)."""
        yield from self.quads((None, None, None, None))

    def parse(
        self,
        source: object | None = None,
        publicID: str | None = None,  # noqa: N803 - RDFLib API name
        format: str | None = None,
        location: str | None = None,
        file: IO[bytes] | None = None,
        data: str | bytes | None = None,
        **kwargs: object,
    ) -> Dataset:
        """Parse RDF (default N-Quads) into the dataset."""
        super().parse(
            source,
            publicID,
            format if format is not None else "nquads",
            location,
            file,
            data,
            **kwargs,
        )
        return self

    def serialize(  # type: ignore[override]
        self,
        destination: str | Path | IO[bytes] | None = None,
        *,
        format: str = "nquads",
        encoding: str | None = None,
        **kwargs: object,
    ) -> str | bytes | None:
        """Serialize the dataset (default N-Quads)."""
        return super().serialize(
            destination, format=format, encoding=encoding, **kwargs
        )


class ConjunctiveGraph(Dataset):
    """RDFLib ``ConjunctiveGraph`` alias over the dataset facade."""


class Seq:
    """A read view over an ``rdf:Seq`` resource, ordered by ``rdf:_1``/``_2``/… .

    Mirrors RDFLib's ``Seq``: it reads the container member predicates
    (``rdf:_N``) off ``subject`` and orders them by their integer index.
    """

    def __init__(self, graph: Graph, subject: Identifier) -> None:
        """Collect ``subject``'s ``rdf:_N`` members from ``graph``, ordered by ``N``."""
        li_index = str(RDF) + "_"
        items: list[tuple[int, Identifier]] = []
        for p, o in graph.predicate_objects(subject):
            if p.startswith(li_index):
                items.append((int(p.replace(li_index, "")), o))
        items.sort()
        self._list = items

    def toPython(self) -> Seq:  # noqa: N802 - RDFLib API name
        """Return self (RDFLib parity)."""
        return self

    def __iter__(self) -> Iterator[Identifier]:
        """Iterate the members in ``rdf:_N`` order."""
        for _index, item in self._list:
            yield item

    def __len__(self) -> int:
        """Return the number of members."""
        return len(self._list)

    def __getitem__(self, index: int) -> Identifier:
        """Return the member at ``index`` (position, not ``rdf:_N`` value)."""
        _index, item = self._list[index]
        return item


class BatchAddGraph:
    """Batch ``add`` calls on a wrapped graph into fewer ``addN`` flushes.

    Mirrors RDFLib's ``BatchAddGraph`` context manager: buffered triples are
    flushed when the buffer fills and again on context exit (unless an exception
    propagates).
    """

    def __init__(
        self, graph: Graph, batch_size: int = 1000, batch_addn: bool = False
    ) -> None:
        """Wrap ``graph``, flushing every ``batch_size`` buffered triples."""
        if not batch_size or batch_size < 2:
            raise ValueError("batch_size must be a positive number")
        self.graph = graph
        self.__graph_tuple = (graph,)
        self.__batch_size = batch_size
        self.__batch_addn = batch_addn
        self.reset()

    def reset(self) -> BatchAddGraph:
        """Clear the buffer and reset the count."""
        self.batch: list[tuple[Identifier, Identifier, Identifier, object]] = []
        self.count = 0
        return self

    def add(
        self,
        triple_or_quad: tuple[Any, ...],
    ) -> BatchAddGraph:
        """Buffer a triple/quad, flushing via ``addN`` when the batch is full."""
        if len(self.batch) >= self.__batch_size:
            self.graph.addN(self.batch)
            self.batch = []
        self.count += 1
        if len(triple_or_quad) == 3:
            self.batch.append(triple_or_quad + self.__graph_tuple)  # type: ignore[arg-type]
        else:
            self.batch.append(triple_or_quad)  # type: ignore[arg-type]
        return self

    def addN(  # noqa: N802 - RDFLib API name
        self, quads: Iterable[tuple[Identifier, Identifier, Identifier, object]]
    ) -> BatchAddGraph:
        """Buffer (or pass through) a sequence of quads."""
        if self.__batch_addn:
            for q in quads:
                self.add(q)
        else:
            self.graph.addN(quads)
        return self

    def __enter__(self) -> BatchAddGraph:
        """Enter the batch context (resets the buffer)."""
        self.reset()
        return self

    def __exit__(self, *exc: object) -> None:
        """Flush the buffer on a clean exit (a propagating exception drops it)."""
        if exc[0] is None:
            self.graph.addN(self.batch)
