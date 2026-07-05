# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Differential parity for ``NamespaceManager`` + the standard vocabularies.

Compares ``purrdf.compat.rdflib`` (shim) against real ``rdflib`` (oracle) for the
prefix/qname surface real code leans on: default bindings, ``compute_qname``
(incl. the ``ns1``/``ns2`` auto-generation), ``qname``, ``expand_curie``,
``curie``, ``normalizeUri``, ``bind`` collision semantics, and the base IRIs of
every vocabulary the compat layer re-exports. Fixtures come from ``conftest.py``.
"""

from __future__ import annotations

from types import ModuleType

import pytest

EX = "http://example.org/"

#: Vocabularies both libraries expose, and their canonical base IRIs.
_VOCAB_IRIS = {
    "RDF": "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "RDFS": "http://www.w3.org/2000/01/rdf-schema#",
    "OWL": "http://www.w3.org/2002/07/owl#",
    "XSD": "http://www.w3.org/2001/XMLSchema#",
    "XMLNS": "http://www.w3.org/XML/1998/namespace",
    "SKOS": "http://www.w3.org/2004/02/skos/core#",
    "DC": "http://purl.org/dc/elements/1.1/",
    "DCTERMS": "http://purl.org/dc/terms/",
    "DCAM": "http://purl.org/dc/dcam/",
    "DCMITYPE": "http://purl.org/dc/dcmitype/",
    "FOAF": "http://xmlns.com/foaf/0.1/",
    "DCAT": "http://www.w3.org/ns/dcat#",
    "VOID": "http://rdfs.org/ns/void#",
    "SH": "http://www.w3.org/ns/shacl#",
    "SDO": "https://schema.org/",
    "PROV": "http://www.w3.org/ns/prov#",
    "GEO": "http://www.opengis.net/ont/geosparql#",
    "TIME": "http://www.w3.org/2006/time#",
    "ORG": "http://www.w3.org/ns/org#",
    "QB": "http://purl.org/linked-data/cube#",
    "CSVW": "http://www.w3.org/ns/csvw#",
    "ODRL2": "http://www.w3.org/ns/odrl/2/",
    "PROF": "http://www.w3.org/ns/dx/prof/",
    "VANN": "http://purl.org/vocab/vann/",
    "WGS": "https://www.w3.org/2003/01/geo/wgs84_pos#",
    "BRICK": "https://brickschema.org/schema/Brick#",
    "DOAP": "http://usefulinc.com/ns/doap#",
    "SOSA": "http://www.w3.org/ns/sosa/",
    "SSN": "http://www.w3.org/ns/ssn/",
}


def _nsm(mod: ModuleType) -> object:
    """Return a default ``NamespaceManager`` for the given library."""
    # rdflib requires a Graph argument; the shim accepts none (Graph optional).
    if mod.__name__ == "rdflib":
        return mod.namespace.NamespaceManager(mod.Graph())
    return mod.NamespaceManager()


# ── vocabulary base IRIs ────────────────────────────────────────────────────────


@pytest.mark.parametrize("name", sorted(_VOCAB_IRIS))
def test_vocab_base_iri_matches_oracle(
    name: str, compat: ModuleType, oracle: ModuleType
) -> None:
    """Every re-exported vocabulary carries rdflib's exact base IRI."""
    # Some vocabularies live only under ``rdflib.namespace`` (not top-level rdflib);
    # the compat layer re-exports them all at the top level for drop-in convenience.
    cval = str(getattr(compat, name))
    oval = str(getattr(oracle.namespace, name))
    assert cval == oval == _VOCAB_IRIS[name]


@pytest.mark.parametrize(
    ("name", "term"),
    [("RDF", "type"), ("RDFS", "label"), ("FOAF", "name"), ("SH", "property")],
)
def test_vocab_term_minting_matches_oracle(
    name: str, term: str, compat: ModuleType, oracle: ModuleType
) -> None:
    """Attribute access mints identical term IRIs in both libraries."""
    assert str(getattr(getattr(compat, name), term)) == str(
        getattr(getattr(oracle, name), term)
    )


# ── default bindings ────────────────────────────────────────────────────────────


def test_default_bindings_match_oracle(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """A default ``NamespaceManager`` binds rdflib's full default namespace set."""
    cset = {(p, str(n)) for p, n in _nsm(compat).namespaces()}
    oset = {(p, str(n)) for p, n in _nsm(oracle).namespaces()}
    assert cset == oset


# ── compute_qname / qname ───────────────────────────────────────────────────────


@pytest.mark.parametrize(
    "uri",
    [
        "http://xmlns.com/foaf/0.1/name",
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
        "http://www.w3.org/2000/01/rdf-schema#label",
        "https://schema.org/Person",
    ],
)
def test_compute_qname_known_prefixes(
    uri: str, compat: ModuleType, oracle: ModuleType
) -> None:
    """``compute_qname`` yields identical ``(prefix, ns, local)`` for bound IRIs."""
    cp, cn, cl = _nsm(compat).compute_qname(uri)
    op, on, ol = _nsm(oracle).compute_qname(uri)
    assert (cp, str(cn), cl) == (op, str(on), ol)


def test_compute_qname_generates_ns_prefixes(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """Unknown namespaces generate the same ``ns1``/``ns2`` prefixes in order."""
    cnm, onm = _nsm(compat), _nsm(oracle)
    for uri in (f"{EX}a", "http://other.example/b", f"{EX}c"):
        cp, _cn, cl = cnm.compute_qname(uri)
        op, _on, ol = onm.compute_qname(uri)
        assert (cp, cl) == (op, ol)


def test_qname_and_curie(compat: ModuleType, oracle: ModuleType) -> None:
    """``qname`` and ``curie`` render identical CURIEs for bound IRIs."""
    uri = "http://xmlns.com/foaf/0.1/knows"
    assert _nsm(compat).qname(uri) == _nsm(oracle).qname(uri) == "foaf:knows"
    assert _nsm(compat).curie(uri) == _nsm(oracle).curie(uri) == "foaf:knows"


# ── expand_curie ────────────────────────────────────────────────────────────────


@pytest.mark.parametrize("curie", ["rdf:type", "foaf:name", "owl:Class", "sh:node"])
def test_expand_curie(curie: str, compat: ModuleType, oracle: ModuleType) -> None:
    """``expand_curie`` resolves default-bound prefixes identically."""
    assert str(_nsm(compat).expand_curie(curie)) == str(
        _nsm(oracle).expand_curie(curie)
    )


def test_expand_curie_unbound_raises(compat: ModuleType) -> None:
    """An unbound prefix raises ``ValueError`` (rdflib parity)."""
    with pytest.raises(ValueError, match="not bound"):
        _nsm(compat).expand_curie("nope:thing")


# ── normalizeUri ────────────────────────────────────────────────────────────────


@pytest.mark.parametrize(
    "uri",
    [
        "http://xmlns.com/foaf/0.1/name",
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
        f"{EX}unbound",
    ],
)
def test_normalize_uri(uri: str, compat: ModuleType, oracle: ModuleType) -> None:
    """``normalizeUri`` shortens bound IRIs and falls back to ``<uri>`` otherwise."""
    assert _nsm(compat).normalizeUri(uri) == _nsm(oracle).normalizeUri(uri)


# ── bind collision semantics ────────────────────────────────────────────────────


def test_bind_collision_autosuffix(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """Re-binding a prefix to a new namespace auto-suffixes (``ex1``) identically."""
    cnm, onm = _nsm(compat), _nsm(oracle)
    for nm in (cnm, onm):
        nm.bind("ex", "http://example.org/one#")
        nm.bind("ex", "http://example.org/two#")  # collides -> ex1
    cmap = {(p, str(n)) for p, n in cnm.namespaces()}
    omap = {(p, str(n)) for p, n in onm.namespaces()}
    assert ("ex", "http://example.org/one#") in cmap
    assert ("ex1", "http://example.org/two#") in cmap
    assert cmap == omap


def test_bind_replace(compat: ModuleType, oracle: ModuleType) -> None:
    """``replace=True`` overwrites the prefix in place, no auto-suffix."""
    cnm, onm = _nsm(compat), _nsm(oracle)
    for nm in (cnm, onm):
        nm.bind("ex", "http://example.org/one#")
        nm.bind("ex", "http://example.org/two#", replace=True)
    cmap = {(p, str(n)) for p, n in cnm.namespaces()}
    omap = {(p, str(n)) for p, n in onm.namespaces()}
    assert ("ex", "http://example.org/two#") in cmap
    assert cmap == omap


# ── parse wires document prefixes into the graph ────────────────────────────────


def test_parse_binds_document_prefixes(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """Parsing Turtle records the document's prefixes on the graph (rdflib parity)."""
    ttl = (
        "@prefix ex: <http://example.org/> .\n"
        "@prefix foaf: <http://xmlns.com/foaf/0.1/> .\n"
        'ex:s foaf:name "Alice" .\n'
    )
    cg, og = compat.Graph(), oracle.Graph()
    cg.parse(data=ttl, format="turtle")
    og.parse(data=ttl, format="turtle")
    cmap = {(p, str(n)) for p, n in cg.namespaces()}
    omap = {(p, str(n)) for p, n in og.namespaces()}
    assert ("ex", "http://example.org/") in cmap
    assert ("ex", "http://example.org/") in omap


def test_parse_sparql_style_prefix(compat: ModuleType) -> None:
    """SPARQL-style ``PREFIX`` declarations are also captured."""
    ttl = "PREFIX ex: <http://example.org/>\nex:s ex:p ex:o .\n"
    g = compat.Graph()
    g.parse(data=ttl, format="turtle")
    assert ("ex", "http://example.org/") in {(p, str(n)) for p, n in g.namespaces()}


def test_parse_binds_jsonld_context_prefixes(compat: ModuleType) -> None:
    """JSON-LD ``@context`` prefixes are extracted after parse by walking the JSON context."""
    jsonld = (
        '{"@context": {"ex": "http://example.org/"}, '
        '"@id": "http://example.org/s", '
        '"http://example.org/p": {"@id": "http://example.org/o"}}'
    )
    g = compat.Graph()
    g.parse(data=jsonld, format="json-ld")
    assert ("ex", "http://example.org/") in {(p, str(n)) for p, n in g.namespaces()}


def test_parse_binds_rdfxml_xmlns_prefixes(compat: ModuleType) -> None:
    """RDF/XML ``xmlns:`` prefixes are recorded on the graph (rdflib parity)."""
    rdfxml = (
        '<?xml version="1.0"?>\n'
        '<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" '
        'xmlns:ex="http://example.org/">\n'
        '  <ex:Thing rdf:about="http://example.org/s">\n'
        '    <ex:p>o</ex:p>\n'
        '  </ex:Thing>\n'
        '</rdf:RDF>'
    )
    g = compat.Graph()
    g.parse(data=rdfxml, format="xml")
    namespaces = {(p, str(n)) for p, n in g.namespaces()}
    assert ("ex", "http://example.org/") in namespaces
    # The reserved xml/xmlns prefixes must not be re-bound by the scan.
    assert ("xml", "http://www.w3.org/XML/1998/namespace") in namespaces


# ── the closed/defined namespace machinery ──────────────────────────────────────


def test_closed_namespace_membership(compat: ModuleType) -> None:
    """``ClosedNamespace`` mints listed terms and rejects unknown ones."""
    ns = compat.ClosedNamespace("http://example.org/", ["a", "b"])
    assert str(ns.a) == "http://example.org/a"
    assert ns["b"] in ns
    with pytest.raises((AttributeError, KeyError)):
        _ = ns.c


def test_defined_namespace_subclass(compat: ModuleType) -> None:
    """A ``DefinedNamespace`` subclass mints terms off its ``_NS`` base IRI."""

    class EXNS(compat.DefinedNamespace):
        _NS = compat.Namespace("http://example.org/")
        _fail = True
        thing: object

    assert str(EXNS.thing) == "http://example.org/thing"
    assert "thing" in EXNS
    with pytest.raises(AttributeError):
        _ = EXNS.missing
