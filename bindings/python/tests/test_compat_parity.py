# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Differential parity: real ``rdflib`` (oracle) vs ``purrdf.compat.rdflib`` (shim).

These tests lock in the current P0 behavior so later tasks cannot silently
regress it. Terms in both libraries are ``str`` subclasses, so we compare
*observable* properties (lexical form, datatype IRI, language, ``toPython``,
``n3``, triple sets) rather than cross-library object identity/equality.

All fixtures come from ``conftest.py``. Fixtures ``compat`` and ``oracle`` are the
shim and the real rdflib respectively; they are never the same object.
"""

from __future__ import annotations

from types import ModuleType

EX = "http://example.org/"
NT_DATA = (
    f"<{EX}s> <{EX}p> <{EX}o> .\n"
    f'<{EX}s> <{EX}name> "Alice" .\n'
    f'<{EX}s> <{EX}age> "42"^^<http://www.w3.org/2001/XMLSchema#integer> .\n'
)


def _triple_strings(graph: object) -> set[tuple[str, str, str]]:
    """Return a graph's triples as ``(str, str, str)`` tuples (library-agnostic)."""
    return {(str(s), str(p), str(o)) for s, p, o in graph}


# ── terms ─────────────────────────────────────────────────────────────────────


def test_uriref_is_str_subclass(compat: ModuleType, oracle: ModuleType) -> None:
    """URIRef is a ``str`` subclass whose value/``n3`` match rdflib."""
    c = compat.URIRef(f"{EX}thing")
    o = oracle.URIRef(f"{EX}thing")
    assert isinstance(c, str) and isinstance(o, str)
    assert str(c) == str(o)
    assert c.n3() == o.n3()


def test_literal_plain_and_typed(compat: ModuleType, oracle: ModuleType) -> None:
    """Plain/typed literal lexical form, datatype and ``toPython`` match rdflib."""
    xsd_int = "http://www.w3.org/2001/XMLSchema#integer"
    for lex, dt in (("Alice", None), ("42", xsd_int)):
        c = compat.Literal(lex, datatype=compat.URIRef(dt)) if dt else compat.Literal(lex)
        o = oracle.Literal(lex, datatype=oracle.URIRef(dt)) if dt else oracle.Literal(lex)
        assert str(c) == str(o)
        assert (str(c.datatype) if c.datatype else None) == (
            str(o.datatype) if o.datatype else None
        )
        assert c.toPython() == o.toPython()


def test_literal_language(compat: ModuleType, oracle: ModuleType) -> None:
    """Language-tagged literals carry a matching language and lexical form."""
    c = compat.Literal("chat", lang="fr")
    o = oracle.Literal("chat", lang="fr")
    assert str(c) == str(o)
    assert c.language == o.language == "fr"


def test_literal_value_inference(compat: ModuleType, oracle: ModuleType) -> None:
    """Inferred datatypes for Python ints/bools match rdflib's."""
    for value in (42, True):
        c = compat.Literal(value)
        o = oracle.Literal(value)
        assert str(c.datatype) == str(o.datatype)
        assert c.toPython() == o.toPython()


def test_literal_term_equality_vs_value_equality(compat: ModuleType) -> None:
    """Term equality (lexical+dt+lang) is distinct from value-space ``.eq()``."""
    xsd = "http://www.w3.org/2001/XMLSchema#"
    a = compat.Literal("1", datatype=compat.URIRef(xsd + "integer"))
    b = compat.Literal("1.0", datatype=compat.URIRef(xsd + "decimal"))
    assert a != b            # different term (lexical + datatype differ)
    assert a.eq(b)           # equal value space


# ── namespaces ──────────────────────────────────────────────────────────────


def test_namespace_attribute_access(compat: ModuleType, oracle: ModuleType) -> None:
    """``RDF.type`` and ``Namespace(...)`` term minting match rdflib strings."""
    assert str(compat.RDF.type) == str(oracle.RDF.type)
    cns = compat.Namespace(EX)
    ons = oracle.Namespace(EX)
    assert str(cns.thing) == str(ons.thing) == f"{EX}thing"


# ── graph ─────────────────────────────────────────────────────────────────────


def test_parse_roundtrip_triple_sets_match(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """Parsing identical N-Triples yields identical triple sets in both libs."""
    cg = compat.Graph()
    cg.parse(data=NT_DATA, format="nt")
    og = oracle.Graph()
    og.parse(data=NT_DATA, format="nt")
    assert len(cg) == len(og) == 3
    assert _triple_strings(cg) == _triple_strings(og)


def test_add_len_contains_and_pattern(compat: ModuleType) -> None:
    """add/len/``in``/wildcard ``triples`` behave as rdflib expects."""
    g = compat.Graph()
    s, p, o = compat.URIRef(f"{EX}s"), compat.URIRef(f"{EX}p"), compat.URIRef(f"{EX}o")
    g.add((s, p, o))
    assert len(g) == 1
    assert (s, p, o) in g
    assert list(g.triples((s, None, None))) == [(s, p, o)]
    assert list(g.subjects(p, o)) == [s]


def test_serialize_turtle_roundtrips(compat: ModuleType) -> None:
    """A graph serialized to Turtle re-parses to the same triple set."""
    g = compat.Graph()
    g.parse(data=NT_DATA, format="nt")
    ttl = g.serialize(format="turtle")
    assert isinstance(ttl, str)
    g2 = compat.Graph()
    g2.parse(data=ttl, format="turtle")
    assert _triple_strings(g) == _triple_strings(g2)


# ── SPARQL ─────────────────────────────────────────────────────────────────────


def test_sparql_select_parity(compat: ModuleType, oracle: ModuleType) -> None:
    """A basic SELECT returns the same bound value in both libraries."""
    query = f"SELECT ?o WHERE {{ <{EX}s> <{EX}name> ?o }}"
    cg = compat.Graph()
    cg.parse(data=NT_DATA, format="nt")
    og = oracle.Graph()
    og.parse(data=NT_DATA, format="nt")
    c_vals = sorted(str(row[0]) for row in cg.query(query))
    o_vals = sorted(str(row[0]) for row in og.query(query))
    assert c_vals == o_vals == ["Alice"]


def test_sparql_ask_parity(compat: ModuleType, oracle: ModuleType) -> None:
    """An ASK query returns the same boolean in both libraries."""
    query = f"ASK {{ <{EX}s> <{EX}p> <{EX}o> }}"
    cg = compat.Graph()
    cg.parse(data=NT_DATA, format="nt")
    og = oracle.Graph()
    og.parse(data=NT_DATA, format="nt")
    assert bool(cg.query(query)) == bool(og.query(query)) is True
