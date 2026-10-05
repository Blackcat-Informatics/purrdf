# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: this is a differential against the real rdflib 7.6 Python oracle, and the
# behaviour under test lives only in the pure-Python rdflib compat shim.
"""A query that assigns an ``initBindings`` variable answers as rdflib 7.6 does.

PurRDF refuses a query that reassigns a pre-bound variable on every native lane; the
rdflib compat shim alone answers it, as rdflib does. Each query below runs through the
compat ``Graph.query`` and through real rdflib with ``?this`` bound to ``ex:a``, and
must answer the same rows under the same column names. The native ``Store.query``
keeps refusing the same text.
"""

from __future__ import annotations

from types import ModuleType

import pytest

import purrdf

EX = "http://example.org/"

#: Queries that assign the bound ``?this``, and neighbours that do not.
QUERIES = [
    f"SELECT ?value WHERE {{ BIND(<{EX}b> AS ?this) ?this <{EX}p> ?value }}",
    f"SELECT (COUNT(*) AS ?value) WHERE {{ ?s <{EX}p> ?o BIND(<{EX}b> AS ?this) }}",
    f"SELECT ?value WHERE {{ {{ SELECT (<{EX}b> AS ?this) WHERE {{}} }} "
    f"?this <{EX}p> ?value }}",
    f"SELECT ?this WHERE {{ BIND(<{EX}b> AS ?this) }}",
    f"SELECT (<{EX}b> AS ?this) WHERE {{}}",
    f"SELECT ?value WHERE {{ ?x <{EX}p> ?value BIND(<{EX}b> AS ?this) "
    f"FILTER(?this = <{EX}b>) }}",
    f"SELECT ?this ?value WHERE {{ ?this <{EX}p> ?value BIND(<{EX}b> AS ?this) }}",
    f"SELECT (STR(?this) AS ?t) WHERE {{ BIND(<{EX}b> AS ?this) }}",
    f"SELECT (COUNT(*) AS ?this) WHERE {{ ?s <{EX}p> ?o }}",
    # Neighbours: no reassignment, so the shim passes the text through unchanged.
    f"SELECT ?this ?value WHERE {{ ?this <{EX}p> ?value }}",
    f"SELECT ?value WHERE {{ BIND(<{EX}b> AS ?fresh) ?fresh <{EX}p> ?value }}",
    f"SELECT ?value WHERE {{ VALUES ?this {{ <{EX}b> }} ?this <{EX}p> ?value }}",
]


def _answer(mod: ModuleType, query: str) -> tuple[list[str], list[tuple[str, ...]]]:
    graph = mod.Graph()
    for subject, obj in [("a", "o1"), ("a", "o2"), ("b", "o3")]:
        graph.add(
            (mod.URIRef(f"{EX}{subject}"), mod.URIRef(f"{EX}p"), mod.URIRef(f"{EX}{obj}"))
        )
    result = graph.query(query, initBindings={"this": mod.URIRef(f"{EX}a")})
    names = [str(variable) for variable in result.vars]
    rows = sorted(tuple(str(cell) for cell in row) for row in result)
    return names, rows


@pytest.mark.parametrize("query", QUERIES)
def test_the_shim_answers_a_reassignment_as_rdflib_does(
    compat: ModuleType, oracle: ModuleType, query: str
) -> None:
    assert _answer(compat, query) == _answer(oracle, query)


def test_native_query_keeps_refusing_the_reassignment() -> None:
    store = purrdf.Store()
    store.load(f"<{EX}a> <{EX}p> <{EX}o1> .", purrdf.RdfFormat.N_TRIPLES)
    reassigning = f"SELECT ?value WHERE {{ BIND(<{EX}b> AS ?this) ?this <{EX}p> ?value }}"
    with pytest.raises(Exception, match="pre-bound"):
        store.query(
            reassigning,
            substitutions={purrdf.Variable("this"): purrdf.NamedNode(f"{EX}a")},
        )
    # The neighbour that assigns a fresh variable answers.
    fresh = f"SELECT ?value WHERE {{ ?this <{EX}p> ?value BIND(<{EX}b> AS ?fresh) }}"
    answer = store.query(
        fresh, substitutions={purrdf.Variable("this"): purrdf.NamedNode(f"{EX}a")}
    )
    assert [row[0] for row in answer] == [purrdf.NamedNode(f"{EX}o1")]
