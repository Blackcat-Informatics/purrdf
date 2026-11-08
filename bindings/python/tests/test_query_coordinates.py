# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: these cases exercise installed PyO3 argument extraction, Python
# receiver/processor forwarding, exception attributes and per-run binding dictionaries.
"""Supplied query coordinates cross the Python boundary without losing ownership."""

from __future__ import annotations

import json
import pytest
import purrdf
from purrdf.compat import rdflib
from purrdf.compat.rdflib.plugins.sparqlprocessor import SPARQLProcessor

EX = "https://example.org/"
NOW = "2020-01-02T03:04:05.12345678901234567890123456789+05:30"
OTHER_NOW = "-0001-02-03T04:05:06.98765432109876543210987654321-04:00"
DATA = (f"<{EX}s> <{EX}p> <{EX}o> .\n"
        f"<{EX}s> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <{EX}Cat> .\n"
        f"<{EX}Cat> <http://www.w3.org/2000/01/rdf-schema#subClassOf> <{EX}Animal> .\n")
PROJECTION = "(NOW() AS ?clock) (RAND() AS ?random) (UUID() AS ?uuid) (STRUUID() AS ?text)"
QUERY = f"SELECT {PROJECTION} WHERE {{ ?s <{EX}p> ?o }}"
ENTAILMENT_QUERY = f"SELECT {PROJECTION} WHERE {{ ?s <{EX}p> ?o ; a <{EX}Animal> }}"
ROUTES = ("query", "query_rdflib", "query_governed", "query_entailment_governed")


def _store(kind=purrdf.Store):
    store = kind()
    store.load(DATA, purrdf.RdfFormat.N_TRIPLES)
    return store


def _row(result):
    rows = list(result)
    assert len(rows) == 1 and json.loads(result.execution_receipt_json)["state"] == "started"
    return tuple(rows[0][i].value for i in range(len(result.variables)))


def _answer(store, route, **coordinates):
    if route == "query_entailment_governed":
        answer = store.query_entailment_governed(ENTAILMENT_QUERY, "rdfs", **coordinates)
        assert answer.phase == "answered" and answer.is_complete
        assert "\nregime rdfs\n" in answer.report
        return answer.outcome.result
    if route == "query_governed":
        answer = store.query_governed(QUERY, **coordinates)
        assert answer.is_complete and answer.tripped is None
        return answer.result
    return getattr(store, route)(QUERY, **coordinates)


@pytest.mark.parametrize("kind", [purrdf.Store, purrdf.MutableDataset])
@pytest.mark.parametrize("route", ROUTES)
def test_native_receiver_routes_forward_exact_inputs_and_independent_defaults(kind, route):
    store = _store(kind)
    first = _row(_answer(store, route, now=NOW, entropy_seed=0))
    assert first[0] == NOW and first[2].startswith("urn:uuid:")
    assert _row(_answer(store, route, now=NOW, entropy_seed=0)) == first
    changed = _row(_answer(store, route, now=OTHER_NOW, entropy_seed=0))
    assert changed[0] == OTHER_NOW and changed[1:] == first[1:]
    changed = _row(_answer(store, route, now=NOW, entropy_seed=2**64 - 1))
    assert changed[0] == first[0] and all(a != b for a, b in zip(changed[1:], first[1:]))
    assert _row(_answer(store, route, now=NOW))[0] == NOW
    omitted = _row(_answer(store, route, now=None, entropy_seed=0))
    assert omitted[0] != NOW and omitted[1:] == first[1:]
    retained = _answer(store, route, now=NOW, entropy_seed=0)
    del store
    assert _row(retained) == first


def test_prepared_run_keeps_bindings_separate_and_coordinates_local_to_each_call():
    store = _store()
    prepared = store.prepare(
        "SELECT ?now ?entropy_seed ?bindings (NOW() AS ?clock) (RAND() AS ?random) {}",
        parameters=["now", "entropy_seed", "bindings"],
    )
    bindings = {name: purrdf.Literal(name) for name in prepared.parameters}
    first = _row(prepared.run(bindings=bindings, now=NOW, entropy_seed=0))
    assert first[:4] == ("now", "entropy_seed", "bindings", NOW)
    changed = _row(prepared.run(bindings, now=OTHER_NOW, entropy_seed=0))
    assert changed[:3] == first[:3] and changed[3] == OTHER_NOW and changed[4] == first[4]
    changed = _row(prepared.run(bindings, now=NOW, entropy_seed=1))
    assert changed[:4] == first[:4] and changed[4] != first[4]
    omitted = _row(prepared.run(bindings, entropy_seed=0))
    assert omitted[3] != NOW and omitted[4] == first[4]
    with pytest.raises(ValueError):
        prepared.run(now=NOW, entropy_seed=0)
    with pytest.raises(TypeError):
        prepared.run(this=purrdf.NamedNode(f"{EX}s"))
    retained = prepared.run(bindings, now=NOW, entropy_seed=0)
    del prepared, store
    assert _row(retained) == first


def test_compat_graph_context_dataset_and_processor_forward_coordinates():
    expected = _row(_store().query(QUERY, now=NOW, entropy_seed=0))
    graph = rdflib.Graph()
    graph.parse(data=DATA, format="nt")
    dataset = rdflib.Dataset()
    context = dataset.get_context(rdflib.URIRef(f"{EX}graph"))
    context.parse(data=DATA, format="nt")
    dataset.default_union = True
    for receiver in (graph, context, dataset, SPARQLProcessor(graph)):
        rows = list(receiver.query(QUERY, now=NOW, entropy_seed=0))
        assert len(rows) == 1
        assert tuple(str(cell) for cell in rows[0]) == expected


def test_coordinate_argument_refusals_are_python_errors_before_execution():
    store = _store()
    prepared = store.prepare(QUERY)
    calls = [lambda route=route, **kw: _answer(store, route, **kw) for route in ROUTES]
    calls.append(prepared.run)
    for call in calls:
        with pytest.raises(ValueError, match="now") as caught:
            call(now="invalid", entropy_seed=0)
        assert caught.value.message_id is None and caught.value.presentation is None
        for seed in (-1, 2**64):
            with pytest.raises(OverflowError):
                call(entropy_seed=seed)
        for arguments in ({"now": 3}, {"entropy_seed": "0"}):
            with pytest.raises(TypeError):
                call(**arguments)
        assert _row(call(now=NOW, entropy_seed=0))[0] == NOW
    with pytest.raises(ValueError, match="now"):
        store.query("invalid query", now="invalid")


def test_coordinates_preserve_python_governor_outcomes_and_opaque_provenance():
    store = _store()
    query = f"SELECT {PROJECTION} WHERE {{ ?s ?p ?o }}"
    coordinates = {"now": NOW, "entropy_seed": 0}
    complete = store.query_governed(query, **coordinates)
    capped = store.query_governed(query, max_answers=1, **coordinates)
    assert not capped.is_complete and capped.result is None
    assert capped.tripped.label == "answer-cap-exhausted"
    assert capped.partial.is_certain and capped.partial.is_positional_prefix
    assert list(capped.partial.result)[0][0].value == NOW
    assert len(capped.partial.result) == 1 and len(complete.result) == 3
    token = purrdf.CancellationToken()
    token.cancel()
    stopped = store.query_governed(query, cancel=token, **coordinates)
    assert not stopped.is_complete and stopped.tripped.kind == "stopped"
    refused = store.query_governed(query, fuel=0, **coordinates)
    assert not refused.is_complete and refused.tripped is not None
    result = rdflib.Graph().query(f"SELECT {PROJECTION} {{}}", **coordinates)
    namespace = ("prov", f"{EX}provenance#")
    encoded = result.serialize(format="json", provenance_namespace=namespace, query_hash="caller-identity")
    decoded = purrdf.provenance_from_json(encoded.encode(), *namespace)
    assert decoded["query_hash"] == "caller-identity"
