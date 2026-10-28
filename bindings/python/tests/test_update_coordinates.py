# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: these cases exercise installed PyO3 UPDATE arguments, Python
# receiver and RDFLib processor forwarding, and Python exception-before-mutation behavior.
"""Supplied coordinates reach each Python UPDATE receiver without partial adoption."""

from __future__ import annotations

import pytest

import purrdf
from purrdf.compat import rdflib
from purrdf.compat.rdflib.plugins.sparqlprocessor import SPARQLUpdateProcessor

EX = "https://example.org/"
NOW = "2020-01-02T03:04:05.12345678901234567890123456789+05:30"
BCE = "-0001-02-03T04:05:06.98765432109876543210987654321-04:00"
INSERT = """INSERT { <%ss> <%sclock> ?clock ; <%srandom> ?random ;
    <%suuid> ?uuid ; <%stext> ?text }
WHERE { BIND(NOW() AS ?clock) BIND(RAND() AS ?random)
    BIND(UUID() AS ?uuid) BIND(STRUUID() AS ?text) }""" % ((EX,) * 5)
SELECT = f"SELECT ?p ?o WHERE {{ <{EX}s> ?p ?o }} ORDER BY ?p"


def _values(store):
    return tuple(row[1].value for row in store.query(SELECT))


@pytest.mark.parametrize("kind", [purrdf.Store, purrdf.MutableDataset])
@pytest.mark.parametrize("route", ["update", "update_governed"])
def test_update_receivers_preserve_exact_coordinates_and_independent_defaults(kind, route):
    def run(**coordinates):
        store = kind()
        outcome = getattr(store, route)(INSERT, **coordinates)
        if route == "update_governed":
            assert outcome.is_applied and outcome.evidence.is_complete
        else:
            assert outcome.is_applied and outcome.evidence is None and outcome.execution_receipt_json
        return _values(store)

    first = run(now=NOW, entropy_seed=0)
    assert first[0] == NOW and first[3].startswith("urn:uuid:")
    assert run(now=NOW, entropy_seed=0) == first
    changed = run(now=BCE, entropy_seed=0)
    assert changed[0] == BCE and changed[1:] == first[1:]
    changed = run(now=NOW, entropy_seed=2**64 - 1)
    assert changed[0] == NOW and all(a != b for a, b in zip(changed[1:], first[1:]))
    assert run(now=NOW)[0] == NOW
    omitted = run(now=None, entropy_seed=0)
    assert omitted[0] != NOW and omitted[1:] == first[1:]


def test_update_argument_refusals_precede_mutation_and_keep_configuration_precedence():
    for kind in (purrdf.Store, purrdf.MutableDataset):
        for route in ("update", "update_governed"):
            store = kind()
            store.update(f'INSERT DATA {{ <{EX}keep> <{EX}p> "original" }}')
            before = list(store)
            call = getattr(store, route)
            with pytest.raises(ValueError, match="now") as caught:
                call(INSERT, now="invalid")
            assert caught.value.message_id is None and caught.value.presentation is None
            for seed in (-1, 2**64):
                with pytest.raises(OverflowError):
                    call(INSERT, entropy_seed=seed)
            for arguments in ({"now": 3}, {"entropy_seed": "0"}):
                with pytest.raises(TypeError):
                    call(INSERT, **arguments)
            with pytest.raises(ValueError, match="division"):
                call(INSERT, division="invalid", now="invalid")
            with pytest.raises(ValueError, match="now"):
                call("invalid update", now="invalid")
            assert list(store) == before


def test_update_stream_and_governed_rollback_cross_python_boundary():
    request = INSERT + "; " + INSERT.replace(f"<{EX}s>", f"<{EX}second>")
    plain, governed = purrdf.Store(), purrdf.Store()
    coordinates = {"now": NOW, "entropy_seed": 0}
    plain.update(request, **coordinates)
    outcome = governed.update_governed(request, **coordinates)
    assert outcome.is_applied
    assert sorted(map(str, plain)) == sorted(map(str, governed))
    rows = list(plain.query(f"SELECT ?o WHERE {{ ?s <{EX}random> ?o }}"))
    assert len(rows) == 2 and rows[0][0] != rows[1][0]
    before = sorted(map(str, governed))
    tripped = governed.update_governed("CLEAR ALL; " + request, fuel=0, **coordinates)
    assert not tripped.is_applied and tripped.tripped.label == "fuel-exhausted"
    assert sorted(map(str, governed)) == before


def test_rdflib_update_receivers_and_processor_preserve_bindings_and_literals():
    expected = purrdf.Store()
    expected.update(INSERT, now=BCE, entropy_seed=2**64 - 1)
    for receiver_kind in ("graph", "context", "dataset", "processor"):
        dataset = rdflib.Dataset()
        graph = (dataset.get_context(rdflib.URIRef(f"{EX}graph"))
                 if receiver_kind == "context" else dataset if receiver_kind == "dataset"
                 else rdflib.Graph())
        keep = (rdflib.URIRef(f"{EX}keep"), rdflib.URIRef(f"{EX}p"), rdflib.Literal("original"))
        graph.add(keep)
        receiver = SPARQLUpdateProcessor(graph) if receiver_kind == "processor" else graph
        receiver.update(
            INSERT.replace(f"<{EX}s>", "?subject").replace(f"<{EX}clock>", "ex:clock"),
            initBindings={"subject": rdflib.URIRef(f"{EX}s")}, initNs={"ex": EX},
            now=BCE, entropy_seed=2**64 - 1,
        )
        # The compat UPDATE's existing dataset scope is unchanged; inspect its store.
        values = _values(graph._store)
        assert values == _values(expected)
        assert keep in graph
        before = sorted(map(str, graph._store))
        with pytest.raises(ValueError, match="now"):
            receiver.update("CLEAR ALL", now="invalid", entropy_seed=0)
        assert sorted(map(str, graph._store)) == before and keep in graph
