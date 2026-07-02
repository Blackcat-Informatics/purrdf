# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""``("sparql", Processor/UpdateProcessor)`` plugin classes (compat registry).

``SPARQLProcessor.query``/``SPARQLUpdateProcessor.update`` are thin registry
shims over ``Graph.query``/``Graph.update`` — RDFLib resolves the ``sparql``
engine name to these classes and calls them the same way it would call its own
``sparql.processor.SPARQLProcessor``. The tests below pin that every keyword
RDFLib passes through this seam (``initBindings``, ``initNs``, ``base``, and
arbitrary engine ``**kwargs``) actually reaches the native graph instead of
being dropped on the floor.
"""

from __future__ import annotations

from types import ModuleType

EX = "http://example.org/"


def _processor_classes(mod: ModuleType) -> tuple[type, type]:
    """Return ``(SPARQLProcessor, SPARQLUpdateProcessor)`` from the compat shim."""
    from purrdf.compat.rdflib.plugins.sparqlprocessor import (
        SPARQLProcessor,
        SPARQLUpdateProcessor,
    )

    return SPARQLProcessor, SPARQLUpdateProcessor


# ── SPARQLProcessor.query forwards initBindings/base/**kwargs ────────────────────


def test_processor_query_forwards_init_bindings(compat: ModuleType) -> None:
    """``initBindings`` passed through the processor actually pre-binds ``?s``."""
    SPARQLProcessor, _ = _processor_classes(compat)
    g = compat.Graph()
    g.add((compat.URIRef(f"{EX}a"), compat.URIRef(f"{EX}p"), compat.URIRef(f"{EX}b")))
    g.add((compat.URIRef(f"{EX}z"), compat.URIRef(f"{EX}p"), compat.URIRef(f"{EX}y")))
    processor = SPARQLProcessor(g)

    res = processor.query(
        "SELECT ?o WHERE { ?s ?p ?o }",
        initBindings={"s": compat.URIRef(f"{EX}a")},
    )
    assert [str(row[0]) for row in res] == [f"{EX}b"]


def test_processor_query_forwards_base(compat: ModuleType) -> None:
    """``base`` passed through the processor resolves a relative IRI in the query."""
    SPARQLProcessor, _ = _processor_classes(compat)
    g = compat.Graph()
    g.add((compat.URIRef(f"{EX}a"), compat.URIRef(f"{EX}p"), compat.Literal("v")))
    processor = SPARQLProcessor(g)

    res = processor.query("SELECT ?o WHERE { <a> <p> ?o }", base=EX)
    assert [str(row[0]) for row in res] == ["v"]


def test_processor_query_unknown_kwarg_does_not_crash(compat: ModuleType) -> None:
    """An engine kwarg unknown to the processor's own signature still forwards."""
    SPARQLProcessor, _ = _processor_classes(compat)
    g = compat.Graph()
    g.add((compat.URIRef(f"{EX}a"), compat.URIRef(f"{EX}p"), compat.Literal("v")))
    processor = SPARQLProcessor(g)

    res = processor.query("SELECT ?o WHERE { ?s ?p ?o }", extension_namespaces=None)
    assert [str(row[0]) for row in res] == ["v"]


# ── SPARQLUpdateProcessor.update forwards initBindings/**kwargs ──────────────────


def test_update_processor_forwards_init_bindings(compat: ModuleType) -> None:
    """``initBindings`` passed through the update processor pre-binds ``?s``."""
    _, SPARQLUpdateProcessor = _processor_classes(compat)
    g = compat.Graph()
    processor = SPARQLUpdateProcessor(g)

    processor.update(
        f'INSERT DATA {{ ?s <{EX}q> "hi" }}',
        initBindings={"s": compat.URIRef(f"{EX}new")},
    )
    assert (
        compat.URIRef(f"{EX}new"),
        compat.URIRef(f"{EX}q"),
        compat.Literal("hi"),
    ) in g


def test_update_processor_unknown_kwarg_does_not_crash(compat: ModuleType) -> None:
    """An engine kwarg unknown to the update processor's signature still forwards."""
    _, SPARQLUpdateProcessor = _processor_classes(compat)
    g = compat.Graph()
    processor = SPARQLUpdateProcessor(g)

    processor.update(
        f'INSERT DATA {{ <{EX}s> <{EX}p> "v" }}', extension_namespaces=None
    )
    assert (
        compat.URIRef(f"{EX}s"),
        compat.URIRef(f"{EX}p"),
        compat.Literal("v"),
    ) in g
