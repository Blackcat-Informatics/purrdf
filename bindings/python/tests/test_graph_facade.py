# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Differential parity for the extended ``Graph`` facade surface (Task 4, #11).

Every method that real ``rdflib`` also exposes is compared against the ``oracle``
fixture; behaviours real rdflib does not surface (e.g. the persistence no-op
stubs) are checked structurally against the shim alone.
"""

from __future__ import annotations

from types import ModuleType

EX = "http://example.org/"
XSD = "http://www.w3.org/2001/XMLSchema#"


def _pairs(items: object) -> set[tuple[str, ...]]:
    """Return an iterable of term tuples as a set of stringified tuples."""
    return {tuple(str(x) for x in row) for row in items}  # type: ignore[union-attr]


# ── __getitem__ slicing ─────────────────────────────────────────────────────────


def _bob_graph(mod: ModuleType) -> object:
    """Build the canonical single-triple ``urn:bob rdfs:label "Bob"`` graph."""
    g = mod.Graph()
    g.add((mod.URIRef("urn:bob"), mod.RDFS.label, mod.Literal("Bob")))
    return g


def test_getitem_subject_predicate_objects(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """``g[s]`` yields the same (predicate, object) pairs as rdflib."""
    cg, og = _bob_graph(compat), _bob_graph(oracle)
    cs, os_ = compat.URIRef("urn:bob"), oracle.URIRef("urn:bob")
    assert _pairs(cg[cs]) == _pairs(og[os_])


def test_getitem_slice_variants(compat: ModuleType, oracle: ModuleType) -> None:
    """Every partial slice of ``g`` matches rdflib's accessor semantics."""
    cg, og = _bob_graph(compat), _bob_graph(oracle)
    label_c, label_o = compat.RDFS.label, oracle.RDFS.label
    bob_c, bob_o = compat.Literal("Bob"), oracle.Literal("Bob")
    s_c, s_o = compat.URIRef("urn:bob"), oracle.URIRef("urn:bob")
    # g[:p] -> subject_objects ; g[::o] -> subject_predicates ; g[s::] -> pred_objs
    assert _pairs(cg[:label_c]) == _pairs(og[:label_o])
    assert _pairs(cg[::bob_c]) == _pairs(og[::bob_o])
    assert _pairs(cg[s_c::]) == _pairs(og[s_o::])
    # g[s:p] -> objects ; g[s::o] -> predicates ; g[:p:o] -> subjects
    assert {str(x) for x in cg[s_c:label_c]} == {str(x) for x in og[s_o:label_o]}
    assert {str(x) for x in cg[s_c::bob_c]} == {str(x) for x in og[s_o::bob_o]}
    assert {str(x) for x in cg[:label_c:bob_c]} == {str(x) for x in og[:label_o:bob_o]}
    # fully-specified slice -> containment boolean
    assert (cg[s_c:label_c:bob_c]) == (og[s_o:label_o:bob_o]) is True


# ── skolemize / de_skolemize ──────────────────────────────────────────────────────


def test_skolemize_deskolemize_roundtrip(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """Skolemization mints the rdflib genid IRI and de-skolemizes back."""
    for mod in (compat, oracle):
        g = mod.Graph()
        b = mod.BNode("abc")
        g.add((b, mod.URIRef(f"{EX}p"), mod.Literal("v")))
        sk = g.skolemize()
        subjects = [str(s) for s, _p, _o in sk]
        assert subjects == [
            "https://rdflib.github.io/.well-known/genid/rdflib/abc"
        ]
        de = sk.de_skolemize()
        assert [str(s) for s, _p, _o in de] == ["abc"]


def test_skolemize_specific_bnode_only(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """Passing ``bnode`` skolemizes only that node, leaving others blank."""
    for mod in (compat, oracle):
        g = mod.Graph()
        b1, b2 = mod.BNode("one"), mod.BNode("two")
        g.add((b1, mod.URIRef(f"{EX}p"), b2))
        sk = g.skolemize(bnode=b1)
        (s, _p, o), = list(sk)
        assert str(s).endswith("/rdflib/one")
        assert str(o) == "two"  # untouched blank node


# ── cbd / connected / all_nodes ───────────────────────────────────────────────────


def test_cbd_matches_oracle(compat: ModuleType, oracle: ModuleType) -> None:
    """The Concise Bounded Description matches rdflib triple-for-triple."""
    for mod, key in ((compat, "compat"), (oracle, "oracle")):
        g = mod.Graph()
        a = mod.URIRef(f"{EX}a")
        b = mod.BNode()
        g.add((a, mod.URIRef(f"{EX}has"), b))
        g.add((b, mod.URIRef(f"{EX}v"), mod.Literal("1")))
        g.add((mod.URIRef(f"{EX}other"), mod.URIRef(f"{EX}z"), mod.Literal("x")))
        cbd = g.cbd(a)
        assert len(cbd) == 2, key


def test_connected_and_all_nodes(compat: ModuleType, oracle: ModuleType) -> None:
    """``all_nodes`` count and ``connected`` verdict match rdflib."""
    for mod in (compat, oracle):
        g = mod.Graph()
        g.add((mod.URIRef(f"{EX}a"), mod.URIRef(f"{EX}p"), mod.URIRef(f"{EX}b")))
        g.add((mod.URIRef(f"{EX}b"), mod.URIRef(f"{EX}p"), mod.URIRef(f"{EX}c")))
        assert len(g.all_nodes()) == 3
        assert g.connected() is True
        g.add((mod.URIRef(f"{EX}x"), mod.URIRef(f"{EX}p"), mod.URIRef(f"{EX}y")))
        assert g.connected() is False


# ── resource ──────────────────────────────────────────────────────────────────────


def test_resource_surface(compat: ModuleType, oracle: ModuleType) -> None:
    """``graph.resource`` exposes identifier/graph/value/objects like rdflib."""
    for mod in (compat, oracle):
        g = mod.Graph()
        s, friend = mod.URIRef(f"{EX}s"), mod.URIRef(f"{EX}f")
        g.add((s, mod.URIRef(f"{EX}knows"), friend))
        g.add((s, mod.RDFS.label, mod.Literal("Bob")))
        r = g.resource(s)
        assert str(r.identifier) == f"{EX}s"
        assert r.graph is g
        assert str(r.value(mod.RDFS.label)) == "Bob"
        refs = sorted(str(x.identifier) for x in r.objects(mod.URIRef(f"{EX}knows")))
        assert refs == [f"{EX}f"]


# ── name resolution / hashing / persistence stubs ─────────────────────────────────


def test_qname_and_compute_qname(compat: ModuleType, oracle: ModuleType) -> None:
    """``qname``/``compute_qname`` delegate to the namespace manager like rdflib."""
    cg, og = compat.Graph(), oracle.Graph()
    cg.bind("ex", compat.Namespace(EX))
    og.bind("ex", oracle.Namespace(EX))
    assert cg.qname(f"{EX}thing") == og.qname(f"{EX}thing") == "ex:thing"
    cp, cn, cl = cg.compute_qname(f"{EX}thing")
    op, on, ol = og.compute_qname(f"{EX}thing")
    assert (cp, str(cn), cl) == (op, str(on), ol)


def test_graph_hash_and_eq_contract(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """``__hash__``/``__eq__`` key on the identifier — matching rdflib's contract."""
    for mod in (compat, oracle):
        ident = mod.URIRef(f"{EX}g")
        a = mod.Graph(identifier=ident)
        b = mod.Graph(identifier=ident)
        assert a == b
        assert hash(a) == hash(b)
        # distinct default identifiers are unequal (rdflib mints a fresh BNode)
        assert mod.Graph() != mod.Graph()


def test_persistence_stubs_are_noops(compat: ModuleType) -> None:
    """open/close/commit/rollback are RDFLib-shaped no-ops on the native store."""
    g = compat.Graph()
    g.add((compat.URIRef(f"{EX}s"), compat.URIRef(f"{EX}p"), compat.Literal("v")))
    assert g.open("config") is None
    assert g.commit() is g
    assert g.rollback() is g
    assert g.close() is None
    assert len(g) == 1  # data survived the no-op transaction calls


# ── BatchAddGraph / Seq ───────────────────────────────────────────────────────────


def test_batch_add_graph(compat: ModuleType, oracle: ModuleType) -> None:
    """Buffered adds flush on batch-full and on context exit, matching rdflib."""
    for mod in (compat, oracle):
        g = mod.Graph()
        with mod.graph.BatchAddGraph(g, batch_size=2) as batch:
            for i in range(5):
                batch.add(
                    (mod.URIRef(f"{EX}s{i}"), mod.URIRef(f"{EX}p"), mod.Literal(i))
                )
        assert len(g) == 5
        assert batch.count == 5


def test_seq_ordering(compat: ModuleType, oracle: ModuleType) -> None:
    """``Seq`` orders members by the ``rdf:_N`` container-membership index."""
    for mod in (compat, oracle):
        g = mod.Graph()
        s = mod.URIRef(f"{EX}seq")
        g.add((s, mod.URIRef(str(mod.RDF) + "_2"), mod.Literal("two")))
        g.add((s, mod.URIRef(str(mod.RDF) + "_1"), mod.Literal("one")))
        g.add((s, mod.URIRef(str(mod.RDF) + "_3"), mod.Literal("three")))
        seq = mod.graph.Seq(g, s)
        assert [str(x) for x in seq] == ["one", "two", "three"]
        assert len(seq) == 3
        assert str(seq[0]) == "one"


# ── literal provenance hardening (#11 fault line) ─────────────────────────────────


def test_literal_provenance_survives_unrelated_update(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """A plain/``xsd:string`` literal pair survives an unrelated SPARQL update."""
    for mod in (compat, oracle):
        g = mod.Graph()
        s, p = mod.URIRef(f"{EX}s"), mod.URIRef(f"{EX}p")
        g.add((s, p, mod.Literal("foo")))
        g.add((s, p, mod.Literal("foo", datatype=mod.URIRef(XSD + "string"))))
        assert len(g) == 2
        g.update(f"INSERT DATA {{ <{EX}s2> <{EX}p2> <{EX}o2> }}")
        assert len(g) == 3
        dts = sorted(str(o.datatype) for o in g.objects(s, p))
        assert dts == ["None", XSD + "string"]


def test_literal_provenance_pruned_on_update_delete(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """Deleting a literal via SPARQL prunes its shim-side provenance too."""
    for mod in (compat, oracle):
        g = mod.Graph()
        s, p = mod.URIRef(f"{EX}s"), mod.URIRef(f"{EX}p")
        g.add((s, p, mod.Literal("foo")))
        g.add((s, p, mod.Literal("foo", datatype=mod.URIRef(XSD + "string"))))
        g.update(f"DELETE WHERE {{ <{EX}s> <{EX}p> ?o }}")
        assert len(g) == 0


def test_parse_string_datatype_collapse_matches_oracle(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """Parsing a plain + ``xsd:string`` literal keeps both terms (rdflib parity).

    This is the residual provenance gap: ``parse`` loads straight into the native
    store (bypassing the literal-variant map), so the plain/``xsd:string`` collapse
    cannot be re-expanded. Ledgered strict-xfail under ``#11``.
    """
    nt = (
        f'<{EX}s> <{EX}p> "foo" .\n'
        f'<{EX}s> <{EX}p> "foo"^^<{XSD}string> .\n'
    )
    cg, og = compat.Graph(), oracle.Graph()
    cg.parse(data=nt, format="nt")
    og.parse(data=nt, format="nt")
    assert len(cg) == len(og)
