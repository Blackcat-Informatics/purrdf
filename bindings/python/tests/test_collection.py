# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Regression tests for :class:`purrdf.compat.rdflib.collection.Collection`."""

from __future__ import annotations

import importlib
from types import ModuleType

EX = "http://example.org/"


def _collection_module(mod: ModuleType) -> ModuleType:
    """Import the package's ``collection`` submodule (shim or oracle)."""
    return importlib.import_module(f"{mod.__name__}.collection")


def _items(mod: ModuleType, *local_names: str) -> list[object]:
    """Build a list of ``URIRef`` items from local names."""
    return [mod.URIRef(f"{EX}{name}") for name in local_names]


def _collection(mod: ModuleType, *local_names: str) -> tuple[object, object]:
    """Create a graph, an anchor, and a Collection seeded with ``local_names``."""
    g = mod.Graph()
    anchor = mod.URIRef(f"{EX}list")
    c = _collection_module(mod).Collection(g, anchor, _items(mod, *local_names))
    return g, c


def test_del_first_preserves_remaining_items(compat: ModuleType) -> None:
    """Deleting the first item of a multi-item list leaves the rest intact."""
    g, c = _collection(compat, "a", "b", "c")
    anchor = compat.URIRef(f"{EX}list")

    del c[0]

    assert list(c) == _items(compat, "b", "c")
    assert len(c) == 2
    # The anchor still carries the list head.
    assert (anchor, compat.RDF.first, compat.URIRef(f"{EX}b")) in g
    rest = g.value(anchor, compat.RDF.rest)
    assert rest is not None and rest != compat.RDF.nil
    assert g.value(rest, compat.RDF.first) == compat.URIRef(f"{EX}c")


def test_del_first_two_item_list(compat: ModuleType) -> None:
    """Deleting the first item of a two-item list leaves a single-item list."""
    g, c = _collection(compat, "a", "b")
    anchor = compat.URIRef(f"{EX}list")

    del c[0]

    assert list(c) == _items(compat, "b")
    assert len(c) == 1
    assert (anchor, compat.RDF.first, compat.URIRef(f"{EX}b")) in g
    assert (anchor, compat.RDF.rest, compat.RDF.nil) in g


def test_del_first_matches_rdflib(compat: ModuleType, oracle: ModuleType) -> None:
    """The shim's ``del collection[0]`` matches real rdflib item-for-item."""
    cg, cc = _collection(compat, "a", "b", "c")
    og, oc = _collection(oracle, "a", "b", "c")

    del cc[0]
    del oc[0]

    assert list(cc) == list(oc)
    assert len(cc) == len(oc) == 2


def test_getitem_negative_index(compat: ModuleType) -> None:
    """c[-1] and c[-2] return the expected tail items for lists of length >= 2."""
    _, cc = _collection(compat, "a", "b", "c")

    assert cc[-1] == compat.URIRef(f"{EX}c")
    assert cc[-2] == compat.URIRef(f"{EX}b")


def test_setitem_negative_index(compat: ModuleType) -> None:
    """c[-1] = x replaces the last item for lists of length >= 2."""
    _, cc = _collection(compat, "a", "b", "c")
    new_c = compat.URIRef(f"{EX}new-c")

    cc[-1] = new_c

    assert list(cc) == _items(compat, "a", "b", "new-c")
    assert cc[-1] == new_c


def test_del_negative_step_slice_all(compat: ModuleType) -> None:
    """del c[::-1] deletes every item without index instability."""
    _, cc = _collection(compat, "a", "b", "c")

    del cc[::-1]

    assert list(cc) == []
    assert len(cc) == 0


def test_del_negative_step_slice_stride_two(compat: ModuleType) -> None:
    """del c[::-2] removes indices 2, 0 for a length-3 list."""
    _, cc = _collection(compat, "a", "b", "c")

    del cc[::-2]

    assert list(cc) == _items(compat, "b")
    assert len(cc) == 1


def test_del_positive_step_slice(compat: ModuleType) -> None:
    """del c[1:3] removes the middle two items of a length-4 list."""
    _, cc = _collection(compat, "a", "b", "c", "d")

    del cc[1:3]

    assert list(cc) == _items(compat, "a", "d")
    assert len(cc) == 2


def test_empty_collection_wires_anchor_to_nil(compat: ModuleType) -> None:
    """An empty Collection explicitly wires its anchor to rdf:nil."""
    g = compat.Graph()
    anchor = compat.URIRef(f"{EX}list")
    c = _collection_module(compat).Collection(g, anchor, [])

    assert list(c) == []
    assert len(c) == 0
    assert (anchor, compat.RDF.rest, compat.RDF.nil) in g
    assert (anchor, compat.RDF.first, None) not in g
