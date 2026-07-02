# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Differential parity for the ``Dataset`` named-graph facade.

Real ``rdflib``'s ``Dataset.contexts`` (and ``Dataset.identifier``) emit
``DeprecationWarning``; the suite silences those so the differential comparison
stays focused on observable behaviour rather than deprecation noise.
"""

from __future__ import annotations

import warnings
from types import ModuleType

EX = "http://example.org/"
XSD = "http://www.w3.org/2001/XMLSchema#"
DEFAULT = "urn:x-rdflib:default"


def _quad_strings(quads: object) -> set[tuple[str, str, str, str | None]]:
    """Return quads as ``(s, p, o, graph|None)`` string tuples."""
    return {
        (str(s), str(p), str(o), None if g is None else str(g))
        for s, p, o, g in quads  # type: ignore[union-attr]
    }


def _seed(mod: ModuleType, *, default_union: bool = False) -> object:
    """Build a two-graph dataset: one default triple + one named triple."""
    ds = mod.Dataset(default_union=default_union)
    ds.add((mod.URIRef(f"{EX}a"), mod.URIRef(f"{EX}b"), mod.Literal("foo")))
    g = ds.graph(mod.URIRef(f"{EX}gr"))
    g.add((mod.URIRef(f"{EX}x"), mod.URIRef(f"{EX}y"), mod.Literal("bar")))
    return ds


# ── named-graph views ─────────────────────────────────────────────────────────────


def test_named_graph_view_is_readable(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """``Dataset.graph(id)`` returns a graph scoped to that slot for read+write."""
    for mod in (compat, oracle):
        ds = _seed(mod)
        g = ds.graph(mod.URIRef(f"{EX}gr"))
        assert len(g) == 1
        assert {(str(s), str(p), str(o)) for s, p, o in g} == {
            (f"{EX}x", f"{EX}y", "bar")
        }
        # writing through the view lands in that named graph only
        g.add((mod.URIRef(f"{EX}x2"), mod.URIRef(f"{EX}y"), mod.Literal("baz")))
        assert len(ds.graph(mod.URIRef(f"{EX}gr"))) == 2


def test_default_graph_view(compat: ModuleType, oracle: ModuleType) -> None:
    """The default-graph view sees only unnamed triples (identifier is default id)."""
    for mod in (compat, oracle):
        ds = _seed(mod)
        default = ds.graph(mod.URIRef(DEFAULT))
        assert str(default.identifier) == DEFAULT
        assert {(str(s), str(o)) for s, _p, o in default} == {(f"{EX}a", "foo")}


def test_get_context_reads_without_registering(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """``get_context`` returns a readable view over an existing named graph."""
    for mod in (compat, oracle):
        ds = _seed(mod)
        gc = ds.get_context(mod.URIRef(f"{EX}gr"))
        assert len(gc) == 1


# ── quads / iteration parity ──────────────────────────────────────────────────────


def test_quads_parity(compat: ModuleType, oracle: ModuleType) -> None:
    """``quads((None,)*4)`` yields identical quad sets (default graph named)."""
    cds, ods = _seed(compat), _seed(oracle)
    assert _quad_strings(cds.quads((None, None, None, None))) == _quad_strings(
        ods.quads((None, None, None, None))
    )


def test_quads_restricted_to_named_graph(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """Restricting the graph slot yields only that graph's quads."""
    for mod in (compat, oracle):
        ds = _seed(mod)
        restricted = _quad_strings(
            ds.quads((None, None, None, mod.URIRef(f"{EX}gr")))
        )
        assert restricted == {(f"{EX}x", f"{EX}y", "bar", f"{EX}gr")}


def test_dataset_iter_yields_quads(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """Iterating a dataset yields quads (RDFLib ``Dataset.__iter__``)."""
    cds, ods = _seed(compat), _seed(oracle)
    assert _quad_strings(cds) == _quad_strings(ods)


# ── contexts / graphs ─────────────────────────────────────────────────────────────


def test_graphs_enumerates_default_plus_named(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """``graphs()`` yields the default graph plus every named graph."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        cds, ods = _seed(compat), _seed(oracle)
        c_names = sorted(str(g.identifier) for g in cds.graphs())
        o_names = sorted(str(g.identifier) for g in ods.graphs())
    assert c_names == o_names == [f"{EX}gr", DEFAULT]


def test_add_graph_registers_empty_graph(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """A registered (still-empty) named graph is enumerated by ``graphs()``."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        for mod in (compat, oracle):
            ds = mod.Dataset()
            ds.add_graph(mod.URIRef(f"{EX}empty"))
            names = sorted(str(g.identifier) for g in ds.graphs())
            assert f"{EX}empty" in names


def test_remove_graph(compat: ModuleType, oracle: ModuleType) -> None:
    """``remove_graph`` drops a named graph's triples and unregisters it."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        for mod in (compat, oracle):
            ds = _seed(mod)
            ds.remove_graph(mod.URIRef(f"{EX}gr"))
            assert len(ds.get_context(mod.URIRef(f"{EX}gr"))) == 0
            names = sorted(str(g.identifier) for g in ds.graphs())
            assert f"{EX}gr" not in names
            # the default graph and its triple persist
            assert len(ds) == 1


# ── default_union semantics ───────────────────────────────────────────────────────


def test_default_union_triples(compat: ModuleType, oracle: ModuleType) -> None:
    """``triples`` spans the default graph only, or every graph under union."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        for mod in (compat, oracle):
            strict = _seed(mod, default_union=False)
            union = _seed(mod, default_union=True)
            strict_objs = sorted(
                str(o) for _s, _p, o in strict.triples((None, None, None))
            )
            union_objs = sorted(
                str(o) for _s, _p, o in union.triples((None, None, None))
            )
            assert strict_objs == ["foo"]
            assert union_objs == ["bar", "foo"]


# ── literal provenance across the dataset ─────────────────────────────────────────


def test_named_graph_literal_provenance_survives_update(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """Plain/``xsd:string`` provenance in a named graph survives an update."""
    with warnings.catch_warnings():
        warnings.simplefilter("ignore", DeprecationWarning)
        for mod in (compat, oracle):
            ds = mod.Dataset()
            g = ds.graph(mod.URIRef(f"{EX}gr"))
            s, p = mod.URIRef(f"{EX}s"), mod.URIRef(f"{EX}p")
            g.add((s, p, mod.Literal("foo")))
            g.add((s, p, mod.Literal("foo", datatype=mod.URIRef(XSD + "string"))))
            assert len(g) == 2
            # A GRAPH-scoped INSERT DATA (real rdflib rejects a bare triple on a
            # Dataset) must not wipe the unrelated named-graph literal provenance.
            ds.update(
                f"INSERT DATA {{ GRAPH <{EX}other> {{ <{EX}s2> <{EX}p2> <{EX}o2> }} }}"
            )
            assert len(ds.graph(mod.URIRef(f"{EX}gr"))) == 2
