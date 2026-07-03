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

import datetime
import re
import warnings
from decimal import Decimal
from functools import total_ordering
from typing import TYPE_CHECKING, Any, Callable, Protocol
from uuid import uuid4

import purrdf

if TYPE_CHECKING:
    from purrdf import BlankNode, NamedNode
    from purrdf import Literal as _NativeLiteral


__all__ = [
    "BNode",
    "Identifier",
    "IdentifiedNode",
    "Literal",
    "Node",
    "URIRef",
    "Variable",
    "from_native",
    "to_native",
]


class _SupportsNormalizeUri(Protocol):
    """Structural type for a namespace manager that can abbreviate an IRI.

    Both the compat :class:`~purrdf.compat.rdflib.namespace.NamespaceManager` and
    RDFLib's own satisfy it, so ``n3`` accepts either without an import cycle.
    """

    def normalizeUri(self, uri: str) -> str:  # noqa: N802 - RDFLib API name
        """Return ``prefix:local`` for ``uri`` if a prefix matches, else ``<uri>``."""
        ...


# XSD datatype IRIs used by the value-space coercion. Kept as bare strings here so
# this module has no import cycle with :mod:`.namespace` (which imports ``URIRef``).
_XSD = "http://www.w3.org/2001/XMLSchema#"
_XSD_STRING = _XSD + "string"
_XSD_BOOLEAN = _XSD + "boolean"
_XSD_DECIMAL = _XSD + "decimal"
_XSD_DOUBLE = _XSD + "double"
_XSD_FLOAT = _XSD + "float"
_XSD_DATE = _XSD + "date"
_XSD_TIME = _XSD + "time"
_XSD_DATETIME = _XSD + "dateTime"
_XSD_DURATION = _XSD + "duration"
_XSD_DAYTIMEDURATION = _XSD + "dayTimeDuration"
_XSD_YEARMONTHDURATION = _XSD + "yearMonthDuration"
_XSD_HEXBINARY = _XSD + "hexBinary"
_XSD_BASE64BINARY = _XSD + "base64Binary"
_XSD_NORMALIZEDSTRING = _XSD + "normalizedString"
_XSD_TOKEN = _XSD + "token"
_XSD_ANYURI = _XSD + "anyURI"
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

#: XSD datatypes that ``xsd:string``-derive to a plain ``str`` value (RDFLib parity:
#: their ``toPython`` is the identity, so the value is just the lexical form).
_XSD_STRINGLIKE = frozenset(
    (
        _XSD_STRING,
        _XSD_ANYURI,
        _XSD + "normalizedString",
        _XSD + "token",
        _XSD + "language",
        _XSD + "Name",
        _XSD + "NCName",
        _XSD + "NMTOKEN",
    )
)

#: XSD datatypes whose value space applies a ``whiteSpace`` facet to the lexical
#: form on construction (RDFLib parity): ``xsd:normalizedString`` uses ``replace``
#: (each tab/newline/CR → space) and ``xsd:token`` uses ``collapse`` (replace, then
#: collapse space runs and strip). The native :func:`purrdf.xsd_normalize_whitespace`
#: performs the facet; this set gates the FFI call to the two faceted datatypes.
_XSD_WHITESPACE_FACETED = frozenset((_XSD_NORMALIZEDSTRING, _XSD_TOKEN))

#: The RDF 1.2 base-direction vocabulary (closed set).
_DIRECTIONS = frozenset(("ltr", "rtl"))

#: RDFLib's *recognized* string-derived datatypes whose lexical form is always
#: well-formed — ``Literal.ill_typed`` is ``False`` for any lexical here.
_ILL_TYPED_ALWAYS_WELLFORMED = frozenset(
    (
        _XSD_STRING,
        _XSD_ANYURI,
        _XSD + "normalizedString",
        _XSD + "token",
        _XSD + "language",
    )
)

#: Recognized datatypes whose validity the native ``xsd_canonical_lexical`` can
#: decide (it returns the canonical lexical for a well-formed value, else ``None``).
#: The intersection of RDFLib's recognized-datatype set with the native validator's
#: coverage — datatypes outside both (e.g. ``rdf:XMLLiteral``) stay *not checkable*.
_ILL_TYPED_CHECKABLE = _XSD_INTEGERS | frozenset(
    (
        _XSD_DECIMAL,
        _XSD_DOUBLE,
        _XSD_FLOAT,
        _XSD_BOOLEAN,
        _XSD_DATE,
        _XSD_TIME,
        _XSD_DATETIME,
        _XSD_DURATION,
        _XSD_DAYTIMEDURATION,
        _XSD_YEARMONTHDURATION,
        _XSD_HEXBINARY,
        _XSD_BASE64BINARY,
    )
)


def _compute_ill_typed(
    lexical: str, datatype: URIRef | None, language: str | None
) -> bool | None:
    """Return RDFLib's ``Literal.ill_typed`` (``None`` when not checkable).

    ``None`` for a plain/language-tagged literal or a datatype outside RDFLib's
    recognized set; ``False`` for an always-well-formed string type; otherwise the
    native XSD validator decides (``True`` when the lexical form is ill-formed).
    """
    if language is not None or datatype is None:
        return None
    dt = str(datatype)
    if dt in _ILL_TYPED_ALWAYS_WELLFORMED:
        return False
    if dt in _ILL_TYPED_CHECKABLE:
        return purrdf.xsd_canonical_lexical(lexical, dt) is None
    return None


# A restricted ``xsd:dayTimeDuration`` (``-?PnDTnHnMnS``) — the subset RDFLib maps to a
# plain :class:`datetime.timedelta`. Every component is optional but at least one must
# be present; the seconds component may carry a fraction.
_DAYTIME_DURATION_RE = re.compile(
    r"^(?P<sign>-?)P"
    r"(?:(?P<days>\d+)D)?"
    r"(?:T"
    r"(?:(?P<hours>\d+)H)?"
    r"(?:(?P<minutes>\d+)M)?"
    r"(?:(?P<seconds>\d+(?:\.\d+)?)S)?"
    r")?$"
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

    def n3(self, namespace_manager: _SupportsNormalizeUri | None = None) -> str:
        """Return the N3/Turtle form (subclasses override; default = IRI form)."""
        return f"<{self}>"


#: RDFLib's abstract ``Node`` base — collapsed to :class:`Identifier` here.
Node = Identifier


class IdentifiedNode(Identifier):
    """A ``str``-subclass base for URI and blank nodes (mirrors ``rdflib.term.IdentifiedNode``).

    In RDFLib 7.6 this is the shared abstract base class of :class:`URIRef` and
    :class:`BNode`; :class:`Literal` and :class:`Variable` intentionally do *not*
    inherit from it. The shim keeps the same hierarchy so type checks and plugin
    discovery that rely on ``isinstance(x, IdentifiedNode)`` behave identically.
    """

    __slots__ = ()


class URIRef(IdentifiedNode):
    """An IRI term — RDFLib-shaped, backed by :class:`purrdf.NamedNode`."""

    __slots__ = ()

    def __new__(cls, value: str) -> URIRef:
        """Construct from the IRI string (no angle brackets)."""
        return str.__new__(cls, value)

    def n3(self, namespace_manager: _SupportsNormalizeUri | None = None) -> str:
        """Return the N3/Turtle form ``<iri>`` (or ``prefix:local`` via a nsm).

        When ``namespace_manager`` is given, the IRI is abbreviated to
        ``prefix:local`` if a bound prefix matches — RDFLib parity.
        """
        if namespace_manager is not None:
            return namespace_manager.normalizeUri(str(self))
        return f"<{self}>"

    def toPython(self) -> str:  # noqa: N802 - RDFLib API name
        """Return the IRI as a plain ``str`` (RDFLib parity)."""
        return str(self)

    def to_native(self) -> NamedNode:
        """Return the native :class:`purrdf.NamedNode` counterpart."""
        return purrdf.NamedNode(str(self))

    # ── SPARQL property-path operators (RDFLib ``rdflib.paths`` parity) ────────────
    #
    # These build the property-path algebra so ``p1 / p2``, ``p * OneOrMore``,
    # ``p1 | p2``, ``~p``, and ``-p`` work on a ``URIRef`` exactly as in RDFLib.
    # ``__mul__`` shadows ``str``'s repetition operator — matching RDFLib, which
    # likewise repurposes it for cardinality paths.

    def __truediv__(self, other: object) -> Any:
        """``self / other`` — a sequence path."""
        from .paths import SequencePath

        return SequencePath(self, other)  # type: ignore[arg-type]

    def __mul__(self, mod: object) -> Any:
        """``self * mod`` — a cardinality path (``*``/``+``/``?``)."""
        from .paths import MulPath

        return MulPath(self, mod)  # type: ignore[arg-type]

    def __or__(self, other: object) -> Any:
        """``self | other`` — an alternative path."""
        from .paths import AlternativePath

        return AlternativePath(self, other)  # type: ignore[arg-type]

    def __invert__(self) -> Any:
        """``~self`` — an inverse path."""
        from .paths import InvPath

        return InvPath(self)

    def __neg__(self) -> Any:
        """``-self`` — a negated property set."""
        from .paths import NegatedPath

        return NegatedPath(self)


class BNode(IdentifiedNode):
    """A blank-node term — RDFLib-shaped, backed by :class:`purrdf.BlankNode`."""

    __slots__ = ()

    def __new__(cls, value: str | None = None) -> BNode:
        """Construct from a label, or mint a fresh unique label when ``None``."""
        if value is None:
            value = f"N{uuid4().hex}"
        return str.__new__(cls, value)

    def n3(self, namespace_manager: _SupportsNormalizeUri | None = None) -> str:
        """Return the N3/Turtle form ``_:label``."""
        return f"_:{self}"

    def toPython(self) -> str:  # noqa: N802 - RDFLib API name
        """Return the blank-node label as a plain ``str``."""
        return str(self)

    def to_native(self) -> BlankNode:
        """Return the native :class:`purrdf.BlankNode` counterpart."""
        return purrdf.BlankNode(str(self))


def _daytime_duration_to_timedelta(lexical: str) -> datetime.timedelta | str:
    """Map a restricted ``xsd:dayTimeDuration`` to a :class:`datetime.timedelta`.

    Mirrors RDFLib, which yields a plain ``timedelta`` for a day/time duration (an
    ill-formed or unsupported form keeps the lexical string). ``xsd:duration`` and
    ``xsd:yearMonthDuration`` need calendar arithmetic RDFLib delegates to
    ``isodate``; without it we keep the lexical form, so those are not handled here.
    """
    match = _DAYTIME_DURATION_RE.match(lexical)
    # A bare ``P``/``-P`` (no day count, no ``T`` time section at all) has no
    # duration component whatsoever and is ill-typed per RDFLib/isodate — unlike
    # e.g. ``PT`` or ``PT0S``, which isodate accepts as a zero duration. Falling
    # through for the bare form would silently coerce it to ``timedelta(0)``
    # instead of keeping the lexical string, as RDFLib does.
    if match is None or match.group(0) == match.group("sign") + "P":
        return lexical
    days = int(match.group("days") or 0)
    hours = int(match.group("hours") or 0)
    minutes = int(match.group("minutes") or 0)
    seconds = float(match.group("seconds") or 0)
    delta = datetime.timedelta(days=days, hours=hours, minutes=minutes, seconds=seconds)
    return -delta if match.group("sign") == "-" else delta


def _coerce_value(lexical: str, datatype: URIRef | None, language: str | None) -> Any:
    """Map a (lexical, datatype) pair to a Python value (RDFLib ``toPython``).

    Broadened to RDFLib's ``_castLexicalToPython`` breadth. An unknown datatype or an
    ill-formed lexical form always falls back to the lexical string — never raises.
    """
    if language is not None or datatype is None:
        return lexical
    dt = str(datatype)
    # Private-internals consumers (e.g. pyshacl's bool patch) may have mutated the
    # rdflib-style ``_toPythonMapping``; honor that override before falling back
    # to the native-backed coercion table.
    dt_ref = URIRef(dt)
    if dt_ref in _toPythonMapping:
        conv = _toPythonMapping[dt_ref]
        if conv is not None:
            try:
                return conv(lexical)
            except Exception:  # noqa: BLE001 - rdflib parity: bad lexical → lexical string
                return lexical
        return lexical
    if dt in _XSD_STRINGLIKE:
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
    if dt == _XSD_DATE:
        try:
            return datetime.date.fromisoformat(lexical)
        except ValueError:
            return lexical
    if dt == _XSD_TIME:
        try:
            return datetime.time.fromisoformat(lexical)
        except ValueError:
            return lexical
    if dt == _XSD_DATETIME:
        try:
            return datetime.datetime.fromisoformat(lexical)  # noqa: DTZ007
        except ValueError:
            return lexical
    if dt == _XSD_DAYTIMEDURATION:
        return _daytime_duration_to_timedelta(lexical)
    if dt in (_XSD_DURATION, _XSD_YEARMONTHDURATION):
        # RDFLib maps these to an ``isodate`` object (``Duration``/``timedelta``);
        # without ``isodate`` we keep the lexical form (the sanctioned fallback).
        return lexical
    if dt in (_XSD_HEXBINARY, _XSD_BASE64BINARY):
        # Decode to Python ``bytes`` through the native zero-dependency codecs
        # (RUST-FIRST): malformed lexicals fall back to the lexical string, matching
        # RDFLib's ``_castLexicalToPython``.
        decoded = purrdf.xsd_decode_binary(lexical, dt)
        return decoded if decoded is not None else lexical
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

    __slots__ = ("_datatype", "_direction", "_language", "_value")

    _datatype: URIRef | None
    _direction: str | None
    _language: str | None
    _value: Any

    def __new__(
        cls,
        lexical_or_value: object,
        lang: str | None = None,
        datatype: URIRef | str | None = None,
        *,
        direction: str | None = None,
        normalize: bool | None = None,
    ) -> Literal:
        """Construct from a lexical string (with optional lang/datatype) or value.

        ``direction`` is the RDF 1.2 base direction (``"ltr"``/``"rtl"``) of a
        directional language-tagged literal (``dirLangString``); it requires a
        language tag, mirroring the RDF 1.2 ``Literal`` surface.
        """
        dt: URIRef | None
        if isinstance(datatype, str) and not isinstance(datatype, URIRef):
            dt = URIRef(datatype)
        else:
            dt = datatype
        if isinstance(lexical_or_value, Literal):
            # Re-wrapping an existing literal: inherit its lang/datatype/direction
            # unless the caller explicitly overrode them. Literal subclasses str, so
            # without this branch the bare-str case below would silently drop them
            # (RDFLib preserves them — parity).
            lexical = str(lexical_or_value)
            if lang is None and dt is None:
                lang = lexical_or_value._language
                dt = lexical_or_value._datatype
                if direction is None:
                    direction = lexical_or_value._direction
        elif isinstance(lexical_or_value, bytes):
            # A bytes lexical (e.g. ``Literal(b"4b6579", datatype=hexBinary)``) is the
            # UTF-8 encoding of the lexical form — decode it to the str lexical, as
            # RDFLib does, rather than stringifying the ``bytes`` repr.
            lexical = lexical_or_value.decode("utf-8")
        elif isinstance(lexical_or_value, str):
            lexical = str(lexical_or_value)
        else:
            inferred_lexical, inferred_dt = _infer_typed(lexical_or_value)
            lexical = inferred_lexical
            if dt is None and lang is None:
                dt = inferred_dt
        if dt is not None and normalize is not False and str(dt) in _XSD_WHITESPACE_FACETED:
            # Apply the XSD ``whiteSpace`` facet (``replace`` for normalizedString,
            # ``collapse`` for token) to the lexical form itself, so ``str()``, ``n3()``,
            # term equality, and the native round-trip all use the normalized form —
            # RDFLib parity. Delegates to the native facet (RUST-FIRST).
            normalized = purrdf.xsd_normalize_whitespace(lexical, str(dt))
            if normalized is not None:
                lexical = normalized
        if direction is not None:
            if direction not in _DIRECTIONS:
                raise ValueError(
                    f"invalid base direction {direction!r}: expected 'ltr' or 'rtl'"
                )
            if lang is None:
                raise ValueError(
                    "a base direction requires a language tag (RDF 1.2 dirLangString)"
                )
        self = str.__new__(cls, lexical)
        self._language = lang
        self._datatype = dt
        self._direction = direction
        self._value = _coerce_value(lexical, dt, lang)
        return self

    @property
    def language(self) -> str | None:
        """The language tag, or ``None``."""
        return self._language

    @property
    def direction(self) -> str | None:
        """The RDF 1.2 base direction (``"ltr"``/``"rtl"``), or ``None``."""
        return self._direction

    @property
    def ill_typed(self) -> bool | None:
        """Whether the lexical form is invalid for the datatype (RDFLib parity).

        ``True`` when the lexical form is ill-formed for a recognized datatype,
        ``False`` when it is well-formed, and ``None`` when the datatype is not in
        RDFLib's recognized set (so validity is not checkable).
        """
        return _compute_ill_typed(str(self), self._datatype, self._language)

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

    def n3(self, namespace_manager: _SupportsNormalizeUri | None = None) -> str:
        """Return the N3/Turtle form (quoted lexical + lang or ``^^datatype``).

        When ``namespace_manager`` is given, the datatype IRI is abbreviated through
        it (``"v"^^prefix:local``) — RDFLib parity.
        """
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
            tag = f"{body}@{self._language}"
            # RDF 1.2 ``dirLangString``: the base direction rides the language tag as
            # ``@lang--dir`` (the N-Triples 1.2 syntax RDFLib 7.x emits).
            if self._direction is not None:
                return f"{tag}--{self._direction}"
            return tag
        if self._datatype is not None:
            if namespace_manager is not None:
                dt_n3 = namespace_manager.normalizeUri(str(self._datatype))
            else:
                dt_n3 = f"<{self._datatype}>"
            return f"{body}^^{dt_n3}"
        return body

    def to_native(self) -> _NativeLiteral:
        """Return the native :class:`purrdf.Literal` counterpart."""
        if self._language is not None:
            try:
                if self._direction is not None:
                    return purrdf.Literal(
                        str(self),
                        language=self._language,
                        direction=self._direction,
                    )
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
            and self._direction == other._direction
        )

    def __ne__(self, other: object) -> bool:
        """Negate :meth:`__eq__` (``str`` provides its own ``__ne__`` otherwise)."""
        result = self.__eq__(other)
        if result is NotImplemented:
            return NotImplemented
        return not result

    def __hash__(self) -> int:
        """Hash over ``(lexical, datatype, language, direction)`` — follows ``__eq__``."""
        return hash(
            (
                str(self),
                self._datatype,
                _language_hash_key(self._language),
                self._direction,
            )
        )

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

    def n3(self, namespace_manager: _SupportsNormalizeUri | None = None) -> str:
        """Return the SPARQL form ``?name``."""
        return f"?{self}"

    def to_native(self) -> purrdf.Variable:
        """Return the native :class:`purrdf.Variable` counterpart."""
        return purrdf.Variable(str(self))


# ── Private-internals compatibility shims ─────────────────────────────────────
#
# These are NOT public rdflib API. They exist only because downstream consumers
# (notably pyshacl) reach into rdflib's private Python internals and mutate them
# at runtime. The shim keeps the absolute minimum surface needed for those
# consumers to function, while the real value-space work stays in Rust.

#: The XSD namespace prefix used by rdflib's private ``_XSD_PFX`` symbol.
_XSD_PFX: str = _XSD


def _parseBoolean(value: str | bytes) -> bool:  # noqa: N802 - rdflib API name
    """Parse an XSD boolean lexical form (rdflib 7.6 private API parity).

    Lexical space is ``{"true", "false", "1", "0"}``; any other input emits a
    warning and maps to ``False``, matching rdflib's lenient behavior.
    """
    new_value = value.lower()
    if new_value in ("1", "true", b"1", b"true"):
        return True
    if new_value not in ("0", "false", b"0", b"false"):
        warnings.warn(
            f"Parsing weird boolean, {value!r} does not map to True or False",
            category=UserWarning,
            stacklevel=2,
        )
    return False


#: Runtime-mutable datatype → converter table mirroring rdflib's private
#: ``_toPythonMapping``. Consumers such as pyshacl patch the boolean entry.
_toPythonMapping: dict[URIRef, Callable[[str], Any] | None] = {}


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
    value: purrdf.NamedNode | purrdf.BlankNode | purrdf.Literal | purrdf.Triple | None,
) -> URIRef | BNode | Literal | None:
    """Convert a native :mod:`purrdf` term back to a compat term.

    Returns ``None`` for an unbound value. RDF 1.2 quoted-triple (triple) terms have
    no counterpart in rdflib 7.6 (no ``QuotedGraph``/triple-term type), so they are
    surfaced as an explicit :class:`NotImplementedError` rather than mishandled.
    """
    if value is None:
        return None
    if isinstance(value, purrdf.NamedNode):
        return URIRef(value.value)
    if isinstance(value, purrdf.BlankNode):
        return BNode(value.value)
    if isinstance(value, purrdf.Literal):
        if value.language is not None:
            return Literal(value.value, lang=value.language, direction=value.direction)
        datatype = value.datatype.value
        # The native IR expands a plain literal to ``xsd:string``; RDFLib keeps a
        # plain literal datatype-less. Drop ``xsd:string`` on the way back so the
        # compat term matches a plain RDFLib literal (the documented asymmetry).
        if datatype == _XSD_STRING:
            return Literal(value.value)
        return Literal(value.value, datatype=URIRef(datatype))
    raise NotImplementedError(
        "RDF 1.2 triple term has no rdflib 7.6 counterpart and is not "
        f"representable through the compat Graph facade: {value!r}"
    )
