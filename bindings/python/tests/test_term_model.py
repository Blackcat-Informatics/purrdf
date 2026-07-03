# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Differential parity for the term model.

Covers the broadened ``Literal`` value coercion / ``toPython``, ``Literal.ill_typed``,
namespace-manager-aware ``n3``, ``util.from_n3``, RDF 1.2 base direction
(``dirLangString``), and the RDF 1.2 triple-term boundary. Every behavior rdflib 7.6
supports is checked against the ``oracle`` (real rdflib); behaviors it lacks
(base direction, triple terms) are checked against the shim alone or ledgered.
"""

from __future__ import annotations

import datetime
import os
import subprocess
import sys
from pathlib import Path
from types import ModuleType

import pytest

# bindings/python/tests/ -> bindings/ -> python-rdflib-shadow/
_SHADOW_DIR = Path(__file__).resolve().parent.parent.parent / "python-rdflib-shadow"

XSD = "http://www.w3.org/2001/XMLSchema#"
EX = "http://example.org/"


def _lit(module: ModuleType, lexical: str, datatype: str) -> object:
    """Build a typed literal in ``module`` (shim or oracle) from a datatype IRI."""
    return module.Literal(lexical, datatype=module.URIRef(datatype))


# ── toPython value-space breadth ────────────────────────────────────────────────

# Well-formed lexical forms whose ``toPython`` value both libraries agree on
# (identical Python object, not just string-equal).
_WELL_FORMED = [
    ("2002-10-10", "date"),
    ("12:00:00", "time"),
    ("12:00:00Z", "time"),
    ("2002-10-10T12:00:00", "dateTime"),
    ("2002-10-10T12:00:00Z", "dateTime"),
    ("2002-10-10T12:00:00+05:00", "dateTime"),
    ("P1DT2H30M", "dayTimeDuration"),
    ("PT5S", "dayTimeDuration"),
    ("-P1D", "dayTimeDuration"),
    ("PT", "dayTimeDuration"),
    ("PT1H30M", "dayTimeDuration"),
    ("P2DT3H", "dayTimeDuration"),
    ("48656C6C6F", "hexBinary"),
    ("SGVsbG8=", "base64Binary"),
    ("http://example.org/x", "anyURI"),
    ("true", "boolean"),
    ("false", "boolean"),
    ("42", "integer"),
    ("1.5", "decimal"),
    ("1.0E3", "double"),
]


@pytest.mark.parametrize(("lexical", "name"), _WELL_FORMED)
def test_topython_breadth_matches_oracle(
    compat: ModuleType, oracle: ModuleType, lexical: str, name: str
) -> None:
    """``toPython`` on a well-formed lexical form equals rdflib's value object."""
    dt = XSD + name
    c_value = _lit(compat, lexical, dt).toPython()
    o_value = _lit(oracle, lexical, dt).toPython()
    assert c_value == o_value
    assert type(c_value) is type(o_value)


@pytest.mark.parametrize(
    ("lexical", "name"),
    [
        ("abc", "integer"),
        ("notadate", "date"),
        ("ZZ", "hexBinary"),
        ("25:00:00", "time"),
        ("P", "dayTimeDuration"),
        ("-P", "dayTimeDuration"),
    ],
)
def test_topython_illformed_falls_back_to_lexical(
    compat: ModuleType, oracle: ModuleType, lexical: str, name: str
) -> None:
    """An ill-formed lexical form never raises; both keep the lexical string."""
    dt = XSD + name
    c_value = _lit(compat, lexical, dt).toPython()
    # rdflib returns the Literal itself (str-equal to the lexical); the shim returns
    # the lexical string. Compare through ``str`` so the observable form matches.
    assert str(c_value) == str(_lit(oracle, lexical, dt).toPython()) == lexical


@pytest.mark.parametrize("name", ["duration", "yearMonthDuration"])
def test_calendar_duration_falls_back_to_lexical(compat: ModuleType, name: str) -> None:
    """``xsd:duration``/``yearMonthDuration`` keep the lexical form (no ``isodate``)."""
    value = _lit(compat, "P1Y2M", XSD + name).toPython()
    assert value == "P1Y2M"


def test_daytime_duration_is_timedelta(compat: ModuleType) -> None:
    """``xsd:dayTimeDuration`` maps to a ``datetime.timedelta`` (rdflib parity)."""
    value = _lit(compat, "P1DT2H", XSD + "dayTimeDuration").toPython()
    assert value == datetime.timedelta(days=1, hours=2)


@pytest.mark.parametrize(
    ("lexical", "expected"),
    [
        ("PT1H30M", datetime.timedelta(hours=1, minutes=30)),
        ("P2DT3H", datetime.timedelta(days=2, hours=3)),
        ("PT", datetime.timedelta(0)),
    ],
)
def test_daytime_duration_well_formed_values(
    compat: ModuleType, lexical: str, expected: datetime.timedelta
) -> None:
    """A well-formed day/time duration — including the all-zero ``PT`` form — yields
    the matching ``timedelta``, never a stringly-typed fallback.
    """
    literal = _lit(compat, lexical, XSD + "dayTimeDuration")
    value = literal.toPython()
    assert value == expected
    assert isinstance(value, datetime.timedelta)


@pytest.mark.parametrize("lexical", ["P", "-P"])
def test_daytime_duration_bare_form_falls_back_to_lexical(
    compat: ModuleType, lexical: str
) -> None:
    """A bare ``P``/``-P`` has no duration component at all and is ill-typed: the
    shim must keep the raw lexical string, not silently coerce it to a zero
    ``timedelta``.
    """
    literal = _lit(compat, lexical, XSD + "dayTimeDuration")
    value = literal.toPython()
    assert value == lexical
    assert not isinstance(value, datetime.timedelta)
    assert str(literal) == lexical


# ── ill_typed ───────────────────────────────────────────────────────────────────

# Curated (datatype, lexical) pairs whose well-formedness the native validator and
# rdflib agree on (the shim is stricter on a few lenient rdflib quirks — whitespace,
# decimal exponents, base64 padding — which are deliberately excluded here).
_ILL_TYPED_CASES = [
    ("integer", "1"),
    ("integer", "abc"),
    ("integer", "1.5"),
    ("byte", "1"),
    ("byte", "999"),
    ("nonNegativeInteger", "0"),
    ("nonNegativeInteger", "-1"),
    ("decimal", "1.5"),
    ("decimal", "abc"),
    ("double", "1.5E3"),
    ("double", "abc"),
    ("boolean", "true"),
    ("boolean", "maybe"),
    ("date", "2002-10-10"),
    ("date", "notadate"),
    ("dateTime", "2002-10-10T12:00:00"),
    ("dateTime", "nope"),
    ("time", "12:00:00"),
    ("hexBinary", "0A"),
    ("hexBinary", "ZZ"),
]


@pytest.mark.parametrize(("name", "lexical"), _ILL_TYPED_CASES)
def test_ill_typed_matches_oracle(
    compat: ModuleType, oracle: ModuleType, name: str, lexical: str
) -> None:
    """``Literal.ill_typed`` agrees with rdflib for recognized datatypes."""
    dt = XSD + name
    assert _lit(compat, lexical, dt).ill_typed == _lit(oracle, lexical, dt).ill_typed


def test_ill_typed_none_when_not_checkable(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """``ill_typed`` is ``None`` for plain/lang/unrecognized-datatype literals."""
    assert compat.Literal("hi").ill_typed is oracle.Literal("hi").ill_typed is None
    assert (
        compat.Literal("hi", lang="en").ill_typed
        is oracle.Literal("hi", lang="en").ill_typed
        is None
    )
    unknown = EX + "myType"
    assert (
        _lit(compat, "hi", unknown).ill_typed
        is _lit(oracle, "hi", unknown).ill_typed
        is None
    )


# ── n3 with a namespace manager ─────────────────────────────────────────────────


def _oracle_nsm(oracle: ModuleType) -> object:
    """A real rdflib ``NamespaceManager`` bound to xsd/ex."""
    graph = oracle.Graph()
    nsm = graph.namespace_manager
    nsm.bind("xsd", oracle.URIRef(XSD))
    nsm.bind("ex", oracle.URIRef(EX))
    return nsm


def _compat_nsm(compat: ModuleType) -> object:
    """A compat ``NamespaceManager`` bound to xsd/ex."""
    nsm = compat.NamespaceManager()
    nsm.bind("xsd", XSD)
    nsm.bind("ex", EX)
    return nsm


def test_uriref_n3_abbreviates_via_nsm(compat: ModuleType, oracle: ModuleType) -> None:
    """``URIRef.n3(nsm)`` yields ``prefix:local`` (or ``<iri>`` when unbound)."""
    c_nsm, o_nsm = _compat_nsm(compat), _oracle_nsm(oracle)
    for iri in (XSD + "int", EX + "foo", "http://other.example/x"):
        assert compat.URIRef(iri).n3(c_nsm) == oracle.URIRef(iri).n3(o_nsm)
    # Without a nsm the plain angle-bracket form is unchanged.
    assert compat.URIRef(EX + "foo").n3() == oracle.URIRef(EX + "foo").n3()


def test_literal_n3_abbreviates_datatype_via_nsm(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """``Literal.n3(nsm)`` abbreviates the datatype IRI to ``prefix:local``."""
    c_nsm, o_nsm = _compat_nsm(compat), _oracle_nsm(oracle)
    c = _lit(compat, "3", XSD + "integer").n3(c_nsm)
    o = _lit(oracle, "3", XSD + "integer").n3(o_nsm)
    assert c == o == '"3"^^xsd:integer'


# ── util.from_n3 ────────────────────────────────────────────────────────────────

_FROM_N3_CASES = [
    "<http://example.org/thing>",
    "_:b1",
    '"hi"@en',
    '"hi"',
    f'"3"^^<{XSD}integer>',
    "ex:foo",
    '"v"^^ex:dt',
    "42",
    "1.5",
    "1e3",
    "true",
    "false",
]


@pytest.mark.parametrize("text", _FROM_N3_CASES)
def test_from_n3_matches_oracle(
    compat: ModuleType, oracle: ModuleType, text: str
) -> None:
    """``util.from_n3`` parses each term to the same observable form as rdflib."""
    from purrdf.compat.rdflib.util import from_n3

    c_nsm = _compat_nsm(compat)
    o_nsm = _oracle_nsm(oracle)
    c_term = from_n3(text, nsm=c_nsm)
    o_term = from_n3(text, nsm=o_nsm)
    assert type(c_term).__name__ == type(o_term).__name__
    assert str(c_term) == str(o_term)
    assert (getattr(c_term, "language", None)) == (getattr(o_term, "language", None))
    c_dt = getattr(c_term, "datatype", None)
    o_dt = getattr(o_term, "datatype", None)
    assert (str(c_dt) if c_dt else None) == (str(o_dt) if o_dt else None)


def test_from_n3_empty_returns_default(compat: ModuleType) -> None:
    """An empty term string returns the supplied default (rdflib parity)."""
    from purrdf.compat.rdflib.util import from_n3

    sentinel = compat.URIRef(EX + "default")
    assert from_n3("", default=sentinel) is sentinel


# ── RDF 1.2 base direction (dirLangString) ──────────────────────────────────────
#
# rdflib 7.6 has no base-direction surface (``Literal(..., direction=...)`` is a
# TypeError there), so these assert the shim alone rather than against the oracle.


def test_direction_accessors_and_n3(compat: ModuleType) -> None:
    """A ``dirLangString`` exposes ``.language`` + ``.direction`` and an n3 form."""
    lit = compat.Literal("مرحبا", lang="ar", direction="rtl")
    assert lit.language == "ar"
    assert lit.direction == "rtl"
    assert lit.n3() == '"مرحبا"@ar--rtl'


def test_direction_requires_language_and_valid_token(compat: ModuleType) -> None:
    """A base direction requires a language tag and a valid ``ltr``/``rtl`` token."""
    with pytest.raises(ValueError, match="requires a language tag"):
        compat.Literal("x", direction="ltr")
    with pytest.raises(ValueError, match="invalid base direction"):
        compat.Literal("x", lang="en", direction="sideways")


def test_direction_round_trips_through_native(compat: ModuleType) -> None:
    """``to_native``/``from_native`` preserve the base direction."""
    from purrdf.compat.rdflib import term as compat_term

    lit = compat.Literal("chat", lang="fr", direction="ltr")
    native = lit.to_native()
    assert native.direction == "ltr"
    back = compat_term.from_native(native)
    assert isinstance(back, compat.Literal)
    assert back.language == "fr"
    assert back.direction == "ltr"


def test_direction_participates_in_term_identity(compat: ModuleType) -> None:
    """Two literals differing only in base direction are distinct terms."""
    ltr = compat.Literal("x", lang="en", direction="ltr")
    rtl = compat.Literal("x", lang="en", direction="rtl")
    none = compat.Literal("x", lang="en")
    assert ltr != rtl
    assert ltr != none
    assert hash(ltr) != hash(rtl)


# ── RDF 1.2 triple term boundary ────────────────────────────────────────────────


def test_rdf12_triple_term_has_no_rdflib_counterpart(compat: ModuleType) -> None:
    """A native RDF 1.2 triple term should map to an rdflib-representable term.

    Ledgered strict-xfail: rdflib 7.6 has no triple-term/``QuotedGraph`` type, so
    ``from_native`` raises rather than producing one. This test documents the
    boundary and will flip to passing once rdflib gains an RDF 1.2 counterpart.
    """
    import purrdf

    from purrdf.compat.rdflib import term as compat_term

    inner = purrdf.Triple(
        purrdf.NamedNode(EX + "s"),
        purrdf.NamedNode(EX + "p"),
        purrdf.NamedNode(EX + "o"),
    )
    term = compat_term.from_native(inner)
    assert term is not None


# ── IdentifiedNode hierarchy (rdflib 7.6 parity) ────────────────────────────────


def test_identified_node_hierarchy(compat: ModuleType) -> None:
    """URIRef and BNode inherit from IdentifiedNode; Literal and Variable do not."""
    from purrdf.compat.rdflib.term import IdentifiedNode

    assert issubclass(compat.URIRef, IdentifiedNode)
    assert issubclass(compat.BNode, IdentifiedNode)
    assert not issubclass(compat.Literal, IdentifiedNode)
    assert not issubclass(compat.Variable, IdentifiedNode)
    # IdentifiedNode itself is still a str subclass and an Identifier.
    assert issubclass(IdentifiedNode, compat.Identifier)
    assert issubclass(IdentifiedNode, str)


def test_identified_node_importable_from_compat_term() -> None:
    """``from purrdf.compat.rdflib.term import IdentifiedNode`` resolves."""
    from purrdf.compat.rdflib.term import IdentifiedNode

    assert IdentifiedNode.__name__ == "IdentifiedNode"
    assert IdentifiedNode.__module__ == "purrdf.compat.rdflib.term"


def _run_in_shadow(code: str) -> str:
    """Run ``code`` in a child interpreter whose ``import rdflib`` is the shadow."""
    env = dict(os.environ)
    existing = env.get("PYTHONPATH", "")
    env["PYTHONPATH"] = (
        f"{_SHADOW_DIR}{os.pathsep}{existing}" if existing else str(_SHADOW_DIR)
    )
    proc = subprocess.run(
        [sys.executable, "-c", code],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, (
        f"shadow subprocess failed (rc={proc.returncode})\n"
        f"--- stdout ---\n{proc.stdout}\n--- stderr ---\n{proc.stderr}"
    )
    return proc.stdout


def test_identified_node_resolves_through_shadow() -> None:
    """Under the shadow distribution, ``rdflib.term.IdentifiedNode`` is the shim class."""
    code = (
        "from purrdf.compat.rdflib.term import IdentifiedNode as CompatIdentifiedNode\n"
        "from rdflib.term import IdentifiedNode as ShadowIdentifiedNode\n"
        "assert ShadowIdentifiedNode is CompatIdentifiedNode, "
        "f'{ShadowIdentifiedNode} is not {CompatIdentifiedNode}'\n"
        "print('OK')\n"
    )
    assert _run_in_shadow(code).strip() == "OK"
