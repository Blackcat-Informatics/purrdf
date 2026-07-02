# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""SPARQL property-path algebra for the purrdf rdflib compat shim (``rdflib.paths``).

Mirrors RDFLib's ``rdflib.paths``: the operator overloads on :class:`URIRef`
(``p1 / p2``, ``p * OneOrMore``, ``p1 | p2``, ``~p``, ``-p``) build a small path
algebra whose :meth:`Path.n3` renders SPARQL 1.1 property-path syntax. A path used
in a triple pattern's predicate slot (``graph.triples((s, path, o))`` and the
accessor family) is evaluated by translating it to an internal SPARQL query — the
compat equivalent of RDFLib's ``evalPath`` — so the same call sites work unchanged.

The ``{n,m}`` fixed-cardinality operator RDFLib does not model (SPARQL 1.1 dropped
it) is out of scope; the modelled operators are ``/ * + ? | ~ !`` (see the ``#10``
ledger note for the ``{n,m}`` gap).
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Iterator, Union

from .term import Identifier, URIRef

if TYPE_CHECKING:
    from .graph import Graph
    from .namespace import NamespaceManager

__all__ = [
    "Path",
    "SequencePath",
    "AlternativePath",
    "InvPath",
    "NegatedPath",
    "MulPath",
    "ZeroOrMore",
    "OneOrMore",
    "ZeroOrOne",
    "mul_path",
    "inv_path",
    "neg_path",
    "evalPath",
]

#: The three SPARQL cardinality modifiers (RDFLib parity — bare strings).
ZeroOrMore = "*"
OneOrMore = "+"
ZeroOrOne = "?"

_PathElement = Union["Path", URIRef]


def _n3(arg: _PathElement, namespace_manager: NamespaceManager | None = None) -> str:
    """Render a path element, parenthesizing a multi-arg sequence/alternative.

    Matches RDFLib's ``rdflib.paths._n3`` precedence handling exactly.
    """
    if isinstance(arg, (SequencePath, AlternativePath)) and len(arg.args) > 1:
        return f"({arg.n3(namespace_manager)})"
    return arg.n3(namespace_manager)


class Path:
    """Base class for a SPARQL property path (RDFLib ``rdflib.paths.Path``)."""

    def n3(self, namespace_manager: NamespaceManager | None = None) -> str:
        """Return the SPARQL property-path syntax (overridden by subclasses)."""
        raise NotImplementedError

    def eval(
        self,
        graph: Graph,
        subj: Identifier | None = None,
        obj: Identifier | None = None,
    ) -> Iterator[tuple[Identifier, Identifier]]:
        """Yield ``(subject, object)`` endpoint pairs satisfying the path in ``graph``."""
        for s, _p, o in graph._triples_path(subj, self, obj):
            yield (s, o)

    def __truediv__(self, other: _PathElement) -> SequencePath:
        """``self / other`` — a sequence path."""
        return SequencePath(self, other)

    def __or__(self, other: _PathElement) -> AlternativePath:
        """``self | other`` — an alternative path."""
        return AlternativePath(self, other)

    def __mul__(self, mod: str) -> MulPath:
        """``self * mod`` — a cardinality path (``*``/``+``/``?``)."""
        return MulPath(self, mod)

    def __invert__(self) -> InvPath:
        """``~self`` — an inverse path."""
        return InvPath(self)

    def __neg__(self) -> NegatedPath:
        """``-self`` — a negated property set."""
        return NegatedPath(self)

    def __hash__(self) -> int:
        """Hash over the rendered path syntax."""
        return hash(self.n3())

    def __eq__(self, other: object) -> bool:
        """Two paths are equal iff they render to the same syntax."""
        return isinstance(other, Path) and self.n3() == other.n3()

    def __ne__(self, other: object) -> bool:
        """Negate :meth:`__eq__`."""
        return not self.__eq__(other)


class SequencePath(Path):
    """A sequence path ``p1 / p2 / …`` (RDFLib parity, flattening nested sequences)."""

    def __init__(self, *args: _PathElement) -> None:
        """Flatten any nested sequence paths into a single argument list."""
        self.args: list[_PathElement] = []
        for arg in args:
            if isinstance(arg, SequencePath):
                self.args += arg.args
            else:
                self.args.append(arg)

    def n3(self, namespace_manager: NamespaceManager | None = None) -> str:
        """Return ``a/b/c``."""
        return "/".join(_n3(a, namespace_manager) for a in self.args)


class AlternativePath(Path):
    """An alternative path ``p1 | p2 | …`` (flattening nested alternatives)."""

    def __init__(self, *args: _PathElement) -> None:
        """Flatten any nested alternative paths into a single argument list."""
        self.args: list[_PathElement] = []
        for arg in args:
            if isinstance(arg, AlternativePath):
                self.args += arg.args
            else:
                self.args.append(arg)

    def n3(self, namespace_manager: NamespaceManager | None = None) -> str:
        """Return ``a|b|c``."""
        return "|".join(_n3(a, namespace_manager) for a in self.args)


class InvPath(Path):
    """An inverse path ``~p`` → ``^p`` (RDFLib parity)."""

    def __init__(self, arg: _PathElement) -> None:
        """Wrap the inverted element."""
        self.arg = arg

    def n3(self, namespace_manager: NamespaceManager | None = None) -> str:
        """Return ``^p``."""
        return f"^{_n3(self.arg, namespace_manager)}"


class MulPath(Path):
    """A cardinality path ``p*`` / ``p+`` / ``p?`` (RDFLib parity)."""

    def __init__(self, path: _PathElement, mod: str) -> None:
        """Bind the modified path and its modifier (``*``/``+``/``?``)."""
        self.path = path
        self.mod = mod
        if mod == ZeroOrOne:
            self.zero, self.more = True, False
        elif mod == ZeroOrMore:
            self.zero, self.more = True, True
        elif mod == OneOrMore:
            self.zero, self.more = False, True
        else:
            raise ValueError(f"Unknown path modifier {mod!r}")

    def n3(self, namespace_manager: NamespaceManager | None = None) -> str:
        """Return ``p*`` / ``p+`` / ``p?``."""
        return f"{_n3(self.path, namespace_manager)}{self.mod}"


class NegatedPath(Path):
    """A negated property set ``-p`` → ``!(…)`` (RDFLib parity)."""

    def __init__(self, arg: _PathElement) -> None:
        """Collect the negated URIRef(s)/inverse(s).

        Only a ``URIRef``, an ``InvPath``, or an ``AlternativePath`` of those is a
        legal negated property set; any other path raises (RDFLib parity).
        """
        self.args: list[_PathElement]
        if isinstance(arg, (URIRef, InvPath)):
            self.args = [arg]
        elif isinstance(arg, AlternativePath):
            self.args = arg.args
        else:
            raise TypeError(
                "Can only negate URIRefs, InvPaths or AlternativePaths, "
                f"not: {arg!r}"
            )

    def n3(self, namespace_manager: NamespaceManager | None = None) -> str:
        """Return ``!(a|b|…)``."""
        return "!(%s)" % "|".join(_n3(a, namespace_manager) for a in self.args)


def mul_path(p: _PathElement, mul: str) -> MulPath:
    """Return a cardinality path (RDFLib ``rdflib.paths.mul_path``)."""
    return MulPath(p, mul)


def inv_path(p: _PathElement) -> InvPath:
    """Return an inverse path (RDFLib ``rdflib.paths.inv_path``)."""
    return InvPath(p)


def neg_path(p: URIRef | AlternativePath | InvPath) -> NegatedPath:
    """Return a negated property set (RDFLib ``rdflib.paths.neg_path``)."""
    return NegatedPath(p)


def evalPath(  # noqa: N802 - RDFLib API name
    graph: Graph,
    t: tuple[Identifier | None, Path, Identifier | None],
) -> Iterator[tuple[Identifier, Identifier]]:
    """Evaluate ``(subject, path, object)`` against ``graph`` (RDFLib ``evalPath``)."""
    subj, path, obj = t
    return path.eval(graph, subj, obj)
