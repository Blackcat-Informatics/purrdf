# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""RDF term model for the purrdf rdflib compat shim (``purrdf.compat.rdflib``).

The terms are ``str`` subclasses — exactly as in RDFLib 7.6 — so existing call
sites that do ``str(uri)``, slicing, set/dict membership, and lexical comparison
keep working unchanged. Each term also owns the *value* needed to build its
native :mod:`purrdf` counterpart on demand (:func:`to_native`), and the inverse
(:func:`from_native`) reconstitutes a compat term from a native query/store term.

This is the **P0 subset** of the eventual P9 public shim: it presents the
constructor / namespace / accessor surface the internal toolchain uses. The full
RDFLib equality/ordering hardening (value-space ``eq``, ``xsd:string`` provenance)
is a P9 concern — here term equality follows RDFLib's *term* equality over
``(lexical, datatype, language)`` so differential tests against real RDFLib match.
"""

from __future__ import annotations

from decimal import Decimal
from functools import total_ordering
from typing import TYPE_CHECKING, Any
from uuid import uuid4

import purrdf

if TYPE_CHECKING:
    from purrdf import BlankNode, NamedNode
    from purrdf import Literal as _NativeLiteral

# XSD datatype IRIs used by the value-space coercion. Kept as bare strings here so
# this module has no import cycle with :mod:`.namespace` (which imports ``URIRef``).
_XSD = "http://www.w3.org/2001/XMLSchema#"
_XSD_STRING = _XSD + "string"
_XSD_BOOLEAN = _XSD + "boolean"
_XSD_DECIMAL = _XSD + "decimal"
_XSD_DOUBLE = _XSD + "double"
_XSD_FLOAT = _XSD + "float"
_XSD_INTEGERS = frozenset(
    _XSD + name
    for name in (
        "integer",
        "int",
        "long",
        "short",
        "byte",
        "nonNegativeInteger",
        "nonPositiveInteger",
        "negativeInteger",
        "positiveInteger",
        "unsignedInt",
        "unsignedLong",
        "unsignedShort",
        "unsignedByte",
    )
)


class Identifier(str):
    """A ``str``-subclass RDF term (mirrors ``rdflib.term.Identifier``).

    The compat surface collapses RDFLib's abstract ``Node`` base into this class
    (``Node`` below is an alias): every concrete term is a ``str`` subclass, so a
    separate non-``str`` base buys nothing and only introduces type-boundary noise.
    """

    __slots__ = ()

    def __new__(cls, value: str) -> Identifier:
        """Construct the term as its lexical/IRI string."""
        return str.__new__(cls, value)

    def n3(self, namespace_manager: object | None = None) -> str:
        """Return the N3/Turtle form (subclasses override; default = IRI form)."""
        return f"<{self}>"


#: RDFLib's abstract ``Node`` base — collapsed to :class:`Identifier` here.
Node = Identifier


class URIRef(Identifier):
    """An IRI term — RDFLib-shaped, backed by :class:`purrdf.NamedNode`."""

    __slots__ = ()

    def __new__(cls, value: str) -> URIRef:
        """Construct from the IRI string (no angle brackets)."""
        return str.__new__(cls, value)

    def n3(self, namespace_manager: object | None = None) -> str:
        """Return the N3/Turtle form ``<iri>``."""
        return f"<{self}>"

    def toPython(self) -> str:  # noqa: N802 - RDFLib API name
        """Return the IRI as a plain ``str`` (RDFLib parity)."""
        return str(self)

    def to_native(self) -> NamedNode:
        """Return the native :class:`purrdf.NamedNode` counterpart."""
        return purrdf.NamedNode(str(self))


class BNode(Identifier):
    """A blank-node term — RDFLib-shaped, backed by :class:`purrdf.BlankNode`."""

    __slots__ = ()

    def __new__(cls, value: str | None = None) -> BNode:
        """Construct from a label, or mint a fresh unique label when ``None``."""
        if value is None:
            value = f"N{uuid4().hex}"
        return str.__new__(cls, value)

    def n3(self, namespace_manager: object | None = None) -> str:
        """Return the N3/Turtle form ``_:label``."""
        return f"_:{self}"

    def toPython(self) -> str:  # noqa: N802 - RDFLib API name
        """Return the blank-node label as a plain ``str``."""
        return str(self)

    def to_native(self) -> BlankNode:
        """Return the native :class:`purrdf.BlankNode` counterpart."""
        return purrdf.BlankNode(str(self))


def _coerce_value(lexical: str, datatype: URIRef | None, language: str | None) -> Any:
    """Map a (lexical, datatype) pair to a Python value (RDFLib ``toPython``)."""
    if language is not None or datatype is None:
        return lexical
    dt = str(datatype)
    if dt == _XSD_STRING:
        return lexical
    if dt == _XSD_BOOLEAN:
        return lexical.strip() in ("true", "1")
    if dt in _XSD_INTEGERS:
        try:
            return int(lexical)
        except ValueError:
            return lexical
    if dt == _XSD_DECIMAL:
        try:
            return Decimal(lexical)
        except (ValueError, ArithmeticError):
            return lexical
    if dt in (_XSD_DOUBLE, _XSD_FLOAT):
        try:
            return float(lexical)
        except ValueError:
            return lexical
    return lexical


def _infer_typed(value: object) -> tuple[str, URIRef | None]:
    """Infer (lexical, datatype) for a non-``str`` Python value (RDFLib parity)."""
    if isinstance(value, bool):
        return ("true" if value else "false", URIRef(_XSD_BOOLEAN))
    if isinstance(value, int):
        return (str(value), URIRef(_XSD + "integer"))
    if isinstance(value, Decimal):
        return (str(value), URIRef(_XSD_DECIMAL))
    if isinstance(value, float):
        return (repr(value), URIRef(_XSD_DOUBLE))
    return (str(value), None)


def _literal_value_datatype(literal: Literal) -> str | None:
    """Return the datatype IRI used for XSD value comparison, if any."""
    if literal.language is not None:
        return None
    if literal.datatype is None:
        return _XSD_STRING
    return str(literal.datatype)


def _xsd_value_compare(left: Literal, right: Literal) -> int | None:
    """Native XSD value comparison, or ``None`` when not comparable."""
    left_dt = _literal_value_datatype(left)
    right_dt = _literal_value_datatype(right)
    if left_dt is None or right_dt is None:
        return None
    compared = purrdf.xsd_value_compare(str(left), left_dt, str(right), right_dt)
    return compared if isinstance(compared, int) else None


def _literal_fallback_key(literal: Literal) -> tuple[str, str, str]:
    """Ordering fallback after value comparison ties or is unavailable."""
    return (
        "" if literal.datatype is None else str(literal.datatype),
        "" if literal.language is None else literal.language,
        str(literal),
    )


def _language_eq(left: str | None, right: str | None) -> bool:
    """RDFLib language-tag equality: preserve spelling, compare case-insensitively."""
    if left is None or right is None:
        return left is right
    return left.lower() == right.lower()


def _language_hash_key(language: str | None) -> str | None:
    """Hash key matching RDFLib's case-insensitive language-tag equality."""
    return None if language is None else language.lower()


@total_ordering
class Literal(Identifier):
    """An RDF literal — ``str``-subclass over the lexical form (RDFLib-shaped).

    Carries ``.datatype`` / ``.language`` and the value-space ``.value`` /
    ``.toPython()``. Equality and hashing follow RDFLib *term* equality over
    ``(lexical, datatype, language)``.
    """

    __slots__ = ("_datatype", "_language", "_value")

    _datatype: URIRef | None
    _language: str | None
    _value: Any

    def __new__(
        cls,
        lexical_or_value: object,
        lang: str | None = None,
        datatype: URIRef | str | None = None,
        *,
        normalize: bool | None = None,
    ) -> Literal:
        """Construct from a lexical string (with optional lang/datatype) or value."""
        dt: URIRef | None
        if isinstance(datatype, str) and not isinstance(datatype, URIRef):
            dt = URIRef(datatype)
        else:
            dt = datatype
        if isinstance(lexical_or_value, Literal):
            # Re-wrapping an existing literal: inherit its lang/datatype unless the
            # caller explicitly overrode them. Literal subclasses str, so without
            # this branch the bare-str case below would silently drop them
            # (RDFLib preserves them — parity).
            lexical = str(lexical_or_value)
            if lang is None and dt is None:
                lang = lexical_or_value._language
                dt = lexical_or_value._datatype
        elif isinstance(lexical_or_value, str):
            lexical = str(lexical_or_value)
        else:
            inferred_lexical, inferred_dt = _infer_typed(lexical_or_value)
            lexical = inferred_lexical
            if dt is None and lang is None:
                dt = inferred_dt
        self = str.__new__(cls, lexical)
        self._language = lang
        self._datatype = dt
        self._value = _coerce_value(lexical, dt, lang)
        return self

    @property
    def language(self) -> str | None:
        """The language tag, or ``None``."""
        return self._language

    @property
    def datatype(self) -> URIRef | None:
        """The datatype IRI, or ``None`` for a plain literal (RDFLib parity)."""
        return self._datatype

    @property
    def value(self) -> Any:
        """The Python value-space form (``int``/``bool``/``Decimal``/… or ``str``)."""
        return self._value

    def toPython(self) -> Any:  # noqa: N802 - RDFLib API name
        """Return the Python value-space form (RDFLib ``toPython``)."""
        return self._value

    def n3(self, namespace_manager: object | None = None) -> str:
        """Return the N3/Turtle form (quoted lexical + lang or ``^^datatype``)."""
        escaped = (
            str(self)
            .replace("\\", "\\\\")
            .replace('"', '\\"')
            .replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t")
        )
        body = f'"{escaped}"'
        if self._language is not None:
            return f"{body}@{self._language}"
        if self._datatype is not None:
            return f"{body}^^<{self._datatype}>"
        return body

    def to_native(self) -> _NativeLiteral:
        """Return the native :class:`purrdf.Literal` counterpart."""
        if self._language is not None:
            try:
                return purrdf.Literal(str(self), language=self._language)
            except ValueError:
                # The native Literal *constructor* validates language tags
                # strictly (RFC 5646: private-use subtags ≤ 8 chars), but the
                # lenient parser preserves the project's longer ``@x-purrdf-*`` tags
                # (e.g. ``x-purrdf-traditional``). Round-trip through N-Triples so
                # those tags survive construction, matching the parse path.
                nt = f"<urn:x> <urn:x> {self.n3()} .".encode()
                quad = purrdf.parse(nt, format=purrdf.RdfFormat.N_TRIPLES)[0]
                obj = quad.object
                assert isinstance(obj, purrdf.Literal)
                return obj
        if self._datatype is not None:
            return purrdf.Literal(
                str(self), datatype=purrdf.NamedNode(str(self._datatype))
            )
        return purrdf.Literal(str(self))

    def __eq__(self, other: object) -> bool:
        """RDFLib term equality over ``(lexical, datatype, language)``."""
        if self is other:
            return True
        if not isinstance(other, Literal):
            return NotImplemented
        return (
            str.__eq__(self, other) is True
            and self._datatype == other._datatype
            and _language_eq(self._language, other._language)
        )

    def __ne__(self, other: object) -> bool:
        """Negate :meth:`__eq__` (``str`` provides its own ``__ne__`` otherwise)."""
        result = self.__eq__(other)
        if result is NotImplemented:
            return NotImplemented
        return not result

    def __hash__(self) -> int:
        """Hash over ``(lexical, datatype, language)`` — follows ``__eq__``."""
        return hash((str(self), self._datatype, _language_hash_key(self._language)))

    def eq(self, other: object) -> bool:
        """Return RDFLib value-space equality, distinct from term equality."""
        if not isinstance(other, Literal):
            return False
        compared = _xsd_value_compare(self, other)
        if compared is not None:
            return compared == 0
        return bool(self == other)

    def __lt__(self, other: object) -> bool:
        """Return deterministic literal ordering for RDFLib sorting paths."""
        if not isinstance(other, Literal):
            return NotImplemented
        compared = _xsd_value_compare(self, other)
        if compared is not None and compared != 0:
            return compared < 0
        return _literal_fallback_key(self) < _literal_fallback_key(other)


class Variable(Identifier):
    """A SPARQL variable term (mirrors ``rdflib.term.Variable``)."""

    __slots__ = ()

    def n3(self, namespace_manager: object | None = None) -> str:
        """Return the SPARQL form ``?name``."""
        return f"?{self}"

    def to_native(self) -> purrdf.Variable:
        """Return the native :class:`purrdf.Variable` counterpart."""
        return purrdf.Variable(str(self))


def to_native(
    term: Identifier,
) -> purrdf.NamedNode | purrdf.BlankNode | purrdf.Literal:
    """Convert a compat term to its native :mod:`purrdf` counterpart."""
    if isinstance(term, URIRef):
        return term.to_native()
    if isinstance(term, BNode):
        return term.to_native()
    if isinstance(term, Literal):
        return term.to_native()
    # A bare ``Identifier`` (or an unknown subclass): treat its string as an IRI,
    # matching how RDFLib widens to URIRef for raw identifiers in term position.
    return purrdf.NamedNode(str(term))


def from_native(
    value: purrdf.NamedNode
    | purrdf.BlankNode
    | purrdf.Literal
    | purrdf.Triple
    | None,
) -> URIRef | BNode | Literal | None:
    """Convert a native :mod:`purrdf` term back to a compat term.

    Returns ``None`` for an unbound value. RDF 1.2 quoted-triple terms have no
    RDFLib counterpart and are surfaced explicitly rather than mishandled.
    """
    if value is None:
        return None
    if isinstance(value, purrdf.NamedNode):
        return URIRef(value.value)
    if isinstance(value, purrdf.BlankNode):
        return BNode(value.value)
    if isinstance(value, purrdf.Literal):
        if value.language is not None:
            return Literal(value.value, lang=value.language)
        datatype = value.datatype.value
        # The native IR expands a plain literal to ``xsd:string``; RDFLib keeps a
        # plain literal datatype-less. Drop ``xsd:string`` on the way back so the
        # compat term matches a plain RDFLib literal (the documented asymmetry).
        if datatype == _XSD_STRING:
            return Literal(value.value)
        return Literal(value.value, datatype=URIRef(datatype))
    raise NotImplementedError(
        "RDF 1.2 quoted-triple term has no rdflib counterpart and is not "
        f"representable through the compat Graph facade: {value!r}"
    )
