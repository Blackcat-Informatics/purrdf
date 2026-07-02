# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""``Resource`` must unwrap a ``Resource``-typed predicate/index, not just object.

``Resource`` wraps a graph + subject identifier so callers never see raw
``Identifier`` values back from accessors — every reference value returned is
re-wrapped as a ``Resource``. That means a caller who obtained a predicate via
``resource.predicates()`` (which yields ``Resource`` instances) can naturally
try to pass that same ``Resource`` back in as the predicate/index argument to
``add``/``objects``/``value``/``transitive_objects``/``__getitem__``. Every one
of those call sites must unwrap it to the underlying identifier before handing
it to the graph — a bare ``Resource`` object must never cross into the native
graph layer.
"""

from __future__ import annotations

from types import ModuleType

EX = "http://example.org/"


def _graph_and_terms(mod: ModuleType) -> tuple[object, object, object, object]:
    """A tiny graph plus its subject/predicate/object as native terms."""
    g = mod.Graph()
    s = mod.URIRef(f"{EX}alice")
    p = mod.URIRef(f"{EX}knows")
    o = mod.URIRef(f"{EX}bob")
    g.add((s, p, o))
    return g, s, p, o


def test_add_accepts_resource_typed_predicate(compat: ModuleType) -> None:
    """``resource.add(p, o)`` unwraps a ``Resource``-typed predicate."""
    g, s, p, o = _graph_and_terms(compat)
    subject = compat.resource.Resource(g, s)
    pred_resource = compat.resource.Resource(g, p)
    other_obj = compat.URIRef(f"{EX}carol")

    subject.add(pred_resource, other_obj)

    assert (s, p, other_obj) in g


def test_objects_accepts_resource_typed_predicate(compat: ModuleType) -> None:
    """``resource.objects(predicate)`` unwraps a ``Resource``-typed predicate."""
    g, s, p, o = _graph_and_terms(compat)
    subject = compat.resource.Resource(g, s)
    pred_resource = compat.resource.Resource(g, p)

    results = list(subject.objects(pred_resource))

    assert [r.identifier for r in results] == [o]


def test_value_accepts_resource_typed_predicate(compat: ModuleType) -> None:
    """``resource.value(p=...)`` unwraps a ``Resource``-typed predicate."""
    g, s, p, o = _graph_and_terms(compat)
    subject = compat.resource.Resource(g, s)
    pred_resource = compat.resource.Resource(g, p)

    result = subject.value(p=pred_resource)

    assert result.identifier == o


def test_transitive_objects_accepts_resource_typed_predicate(
    compat: ModuleType,
) -> None:
    """``transitive_objects(predicate)`` unwraps a ``Resource``-typed predicate."""
    g, s, p, o = _graph_and_terms(compat)
    carol = compat.URIRef(f"{EX}carol")
    g.add((o, p, carol))
    subject = compat.resource.Resource(g, s)
    pred_resource = compat.resource.Resource(g, p)

    reached = {r.identifier for r in subject.transitive_objects(pred_resource)}

    assert reached == {s, o, carol}


def test_transitive_subjects_accepts_resource_typed_predicate(
    compat: ModuleType,
) -> None:
    """``transitive_subjects(predicate)`` unwraps a ``Resource``-typed predicate."""
    g, s, p, o = _graph_and_terms(compat)
    target = compat.resource.Resource(g, o)
    pred_resource = compat.resource.Resource(g, p)

    reached = {r.identifier for r in target.transitive_subjects(pred_resource)}

    assert reached == {o, s}


def test_getitem_accepts_resource_typed_index(compat: ModuleType) -> None:
    """``resource[predicate]`` unwraps a ``Resource``-typed direct index."""
    g, s, p, o = _graph_and_terms(compat)
    subject = compat.resource.Resource(g, s)
    pred_resource = compat.resource.Resource(g, p)

    results = list(subject[pred_resource])

    assert [r.identifier for r in results] == [o]


def test_getitem_slice_accepts_resource_typed_predicate(compat: ModuleType) -> None:
    """``resource[predicate:]`` unwraps a ``Resource``-typed predicate in a slice."""
    g, s, p, o = _graph_and_terms(compat)
    subject = compat.resource.Resource(g, s)
    pred_resource = compat.resource.Resource(g, p)

    results = list(subject[pred_resource:])

    assert [r.identifier for r in results] == [o]


def test_subjects_accepts_resource_typed_predicate(compat: ModuleType) -> None:
    """``resource.subjects(predicate)`` unwraps a ``Resource``-typed predicate."""
    g, s, p, o = _graph_and_terms(compat)
    target = compat.resource.Resource(g, o)
    pred_resource = compat.resource.Resource(g, p)

    results = list(target.subjects(pred_resource))

    assert [r.identifier for r in results] == [s]
