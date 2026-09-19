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

import abc
import datetime
from types import ModuleType

import pytest

from _shadow_test_utils import _run_in_shadow

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


def test_topython_honors_to_python_mapping_override(compat: ModuleType) -> None:
    """Private ``_toPythonMapping`` overrides are honored with a bare-string key.

    RDFLib's ``_toPythonMapping`` keys are ``URIRef`` instances, but ``URIRef`` is a
    ``str`` subclass with inherited hash/equality. Consumers such as pyshacl patch the
    table with plain strings, so the shim must look up ``dt`` directly rather than
    re-wrapping it in ``URIRef``.
    """
    from purrdf.compat.rdflib.term import _toPythonMapping

    dt = EX + "custom-datatype"
    calls: list[str] = []

    def converter(lexical: str) -> str:
        calls.append(lexical)
        return f"converted:{lexical}"

    original = _toPythonMapping.get(dt)
    try:
        # Register with a plain string key, matching how pyshacl mutates rdflib.
        _toPythonMapping[dt] = converter
        lit = _lit(compat, "hello", dt)
        assert lit.toPython() == "converted:hello"
        assert calls == ["hello"]

        # A converter that raises falls back to the lexical string (rdflib parity).
        _toPythonMapping[dt] = lambda lexical: (_ for _ in ()).throw(ValueError("boom"))
        assert _lit(compat, "world", dt).toPython() == "world"

        # A ``None`` mapping entry also falls back to the lexical string.
        _toPythonMapping[dt] = None
        assert _lit(compat, "none", dt).toPython() == "none"
    finally:
        if original is None:
            _toPythonMapping.pop(dt, None)
        else:
            _toPythonMapping[dt] = original


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


def test_native_direction_participates_in_term_identity() -> None:
    """Native Python equality and container keys preserve base direction."""
    import purrdf

    ltr = purrdf.Literal("x", language="en", direction="ltr")
    rtl = purrdf.Literal("x", language="en", direction="rtl")
    assert ltr != rtl
    assert len({ltr, rtl}) == 2
    assert {ltr: "left", rtl: "right"}[rtl] == "right"
    upper = purrdf.Literal("x", language="EN", direction="rtl")
    assert upper == rtl
    assert hash(upper) == hash(rtl)
    subject = purrdf.NamedNode(EX + "s")
    predicate = purrdf.NamedNode(EX + "p")
    assert purrdf.Triple(subject, predicate, ltr) != purrdf.Triple(subject, predicate, rtl)
    for constructor in (purrdf.Triple, purrdf.Quad):
        lower_row = constructor(subject, predicate, rtl)
        upper_row = constructor(subject, predicate, upper)
        assert lower_row == upper_row
        assert hash(lower_row) == hash(upper_row)
        assert len({lower_row, upper_row}) == 1
    for datatype in ("langString", "dirLangString"):
        with pytest.raises(ValueError, match="requires a language tag"):
            purrdf.Literal("x", datatype=purrdf.NamedNode(
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#" + datatype
            ))


def test_native_row_identity_preserves_opaque_blank_labels() -> None:
    """Opaque blank labels cannot shift a row key's field boundaries."""
    import purrdf

    for constructor in (purrdf.Triple, purrdf.Quad):
        first = constructor(
            purrdf.BlankNode("a"), purrdf.NamedNode("urn:p"),
            purrdf.BlankNode("b\x02urn:q\x02_:c"),
        )
        second = constructor(
            purrdf.BlankNode("a\x02urn:p\x02_:b"), purrdf.NamedNode("urn:q"),
            purrdf.BlankNode("c"),
        )
        assert first != second
        assert len({first, second}) == 2


@pytest.mark.parametrize("store_name", ["Store", "MutableDataset"])
@pytest.mark.parametrize("direction", [None, "rtl"])
@pytest.mark.parametrize("nested", [False, True])
def test_native_language_identity_survives_store_operations(
    store_name: str, direction: str | None, nested: bool,
) -> None:
    """Insertion, lookup, prebinding and removal share canonical language identity."""
    import purrdf

    store = getattr(purrdf, store_name)()
    subject, predicate = purrdf.NamedNode(EX + "s"), purrdf.NamedNode(EX + "p")
    upper = purrdf.Literal("x", language="EN", direction=direction)
    lower = purrdf.Literal("x", language="en", direction=direction)
    if nested:
        upper = purrdf.Triple(subject, predicate, upper)
        lower = purrdf.Triple(subject, predicate, lower)
    upper_quad = purrdf.Quad(subject, predicate, upper)
    lower_quad = purrdf.Quad(subject, predicate, lower)
    store.add(upper_quad)
    if store_name == "Store":
        assert lower_quad in store
    else:
        assert store.contains(lower_quad)
        assert len(store.quads_for_pattern(object=lower)) == 1
    for value in (upper, lower):
        assert bool(store.query(
            "ASK { ?s ?p ?o }", substitutions={purrdf.Variable("o"): value},
        ))
    store.remove(lower_quad)
    assert not bool(store.query("ASK { ?s ?p ?o }"))


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
    assert issubclass(compat.URIRef, compat.IdentifiedNode)
    assert issubclass(compat.BNode, compat.IdentifiedNode)
    assert not issubclass(compat.Literal, compat.IdentifiedNode)
    assert not issubclass(compat.Variable, compat.IdentifiedNode)
    # IdentifiedNode itself is still a str subclass and an Identifier.
    assert issubclass(compat.IdentifiedNode, compat.Identifier)
    assert issubclass(compat.IdentifiedNode, str)
    # MRO parity with rdflib 7.6: IdentifiedNode -> Identifier -> Node -> ABC -> str -> object.
    assert compat.IdentifiedNode.__mro__ == (
        compat.IdentifiedNode,
        compat.Identifier,
        compat.Node,
        abc.ABC,
        str,
        object,
    )


def test_identified_node_importable_from_compat_term() -> None:
    """``from purrdf.compat.rdflib.term import IdentifiedNode`` resolves."""
    from purrdf.compat.rdflib.term import IdentifiedNode

    assert IdentifiedNode.__name__ == "IdentifiedNode"
    assert IdentifiedNode.__module__ == "purrdf.compat.rdflib.term"


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


# ── the NATIVE term model, on its own terms ─────────────────────────────────────
#
# Everything above compares the compat shim against the oracle. These compare the
# native `purrdf` term constructors against the RDF abstract syntax directly,
# because the shim re-implements some of this in Python — its own base-direction
# validation, its own `xsd:string` collapse — and so a shim test can pass while
# the native surface underneath it is wrong. A host that imports `purrdf` rather
# than `purrdf.compat.rdflib` gets exactly these terms.

RDF = "http://www.w3.org/1999/02/22-rdf-syntax-ns#"


def test_a_plain_and_an_explicit_xsd_string_literal_are_one_native_term() -> None:
    """RDF 1.1 abolished the plain literal: its datatype IS ``xsd:string``.

    So the two spellings name the same term, and a term model that told them
    apart would report two distinct nodes where RDF has one — visible as a
    duplicate row in any set, dict or graph keyed by terms. Equality, hash and
    set cardinality are asserted together because a type can get any two of the
    three right and still be broken as a key.
    """
    import purrdf

    plain = purrdf.Literal("Alice")
    explicit = purrdf.Literal("Alice", datatype=purrdf.NamedNode(XSD + "string"))

    assert plain == explicit
    assert hash(plain) == hash(explicit)
    assert len({plain, explicit}) == 1, "one term, so one key"
    assert {plain: "first", explicit: "second"} == {plain: "second"}

    # And the getter answers the same way round: a plain literal reports the
    # datatype it has rather than an absence.
    assert plain.datatype.value == XSD + "string"
    assert explicit.datatype.value == XSD + "string"


def test_a_native_language_tagged_literal_reports_rdf_langstring() -> None:
    """A language tag fixes the datatype, and the getter says which one.

    ``rdf:langString`` is not a datatype a caller may supply — it is implied by
    the tag — so this getter is the only place the pairing is visible, and a
    literal that reported ``xsd:string`` here would be claiming to be a different
    term from the one it is.
    """
    import purrdf

    lit = purrdf.Literal("hi", language="en")
    assert lit.language == "en"
    assert lit.datatype.value == RDF + "langString"

    # The neighbouring untagged literal is the contrast, and it is a different
    # term with a different datatype.
    plain = purrdf.Literal("hi")
    assert plain.language is None
    assert plain.datatype.value == XSD + "string"
    assert lit != plain


def test_a_native_typed_literal_keeps_its_datatype_and_differs_from_the_plain_one() -> None:
    """A datatype is part of the term, so ``"1"^^xsd:integer`` is not ``"1"``.

    The pair is asserted together on purpose: a model that stored the datatype
    but left it out of the identity would pass the getter check and still merge
    two distinct terms.
    """
    import purrdf

    typed = purrdf.Literal("1", datatype=purrdf.NamedNode(XSD + "integer"))
    plain = purrdf.Literal("1")

    assert typed.datatype.value == XSD + "integer"
    assert typed != plain
    assert hash(typed) != hash(plain)
    assert len({typed, plain}) == 2


def test_the_native_literal_refuses_a_direction_without_a_language_and_an_unknown_token() -> None:
    """A ``dirLangString`` is a language tag AND a direction from a closed set.

    Both refusals are on the native constructor rather than the shim, which
    validates directions again in Python and so cannot witness this. Each is
    paired with the neighbouring valid call, because an over-refusal here would
    reject the RDF 1.2 literals this surface exists to express and would read as
    correct strictness right up to the point a host wrote one.
    """
    import purrdf

    # A direction with no language is not a dirLangString and is not anything
    # else either, so it is refused rather than silently dropped.
    with pytest.raises(ValueError, match="requires a language tag"):
        purrdf.Literal("x", direction="ltr")
    # …and the neighbouring call that adds the tag is accepted.
    assert purrdf.Literal("x", language="en", direction="ltr").direction == "ltr"

    # The vocabulary is closed: two tokens, and no third spelling is read as
    # either of them.
    with pytest.raises(ValueError, match="invalid base direction") as refused:
        purrdf.Literal("x", language="en", direction="up")
    message = str(refused.value)
    assert '"ltr"' in message and '"rtl"' in message, (
        f"the refusal names both accepted tokens: {message}"
    )
    for token in ("ltr", "rtl"):
        assert purrdf.Literal("x", language="en", direction=token).direction == token

    # A literal that carries no direction reports its absence, which is what
    # makes the getter readable at all: `None` here means "no base direction",
    # never "the default one".
    assert purrdf.Literal("x").direction is None
    assert purrdf.Literal("x", language="en").direction is None
    assert purrdf.Literal("x", datatype=purrdf.NamedNode(XSD + "integer")).direction is None


def test_a_quoted_triple_is_an_object_and_never_a_subject() -> None:
    """RDF 1.2 puts a triple term in the object slot only.

    This is the line RDF 1.2 draws that the obsolete RDF-star proposal did not: a
    subject is an IRI or a blank node, full stop. A model that accepted a triple
    term as a subject would let a caller build a row no RDF syntax can write and
    no consumer can read back, and the refusal has to come from the constructor
    rather than from a type stub, since a stub governs no runtime call.
    """
    import purrdf

    inner = purrdf.Triple(
        purrdf.NamedNode(EX + "s"),
        purrdf.NamedNode(EX + "p"),
        purrdf.NamedNode(EX + "o"),
    )
    reifies = purrdf.NamedNode(RDF + "reifies")

    # The object slot: accepted, and it comes back as a triple term rather than
    # as some flattened rendering of one.
    for constructor in (purrdf.Triple, purrdf.Quad):
        row = constructor(purrdf.NamedNode(EX + "r"), reifies, inner)
        assert isinstance(row.object, purrdf.Triple)
        assert row.object == inner
        assert isinstance(row.subject, purrdf.NamedNode)

    # The subject slot: refused, for both row types, with the reason.
    for constructor in (purrdf.Triple, purrdf.Quad):
        with pytest.raises(TypeError, match="must be a NamedNode or BlankNode"):
            constructor(inner, reifies, purrdf.NamedNode(EX + "o"))

    # The neighbouring valid subjects, which the refusal must not sweep up.
    for subject in (purrdf.NamedNode(EX + "r"), purrdf.BlankNode("r")):
        assert purrdf.Quad(subject, reifies, inner).subject == subject
