# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: this is a differential against the real rdflib 7.6 Python oracle, and the
# behaviour under test lives only in the pure-Python rdflib compat shim.
"""A query that assigns an ``initBindings`` variable answers as rdflib 7.6 does.

PurRDF refuses a query that reassigns a pre-bound variable on every native lane; the
rdflib compat shim alone answers it, as rdflib does. Each query below runs through the
compat ``Graph.query`` and through real rdflib with ``?this`` bound to ``ex:a``, and
must answer the same rows under the same column names, in the same order where the
query orders them. A reassignment in a shape the shim's rewrite does not model is
refused with ``UnmodelledReassignment`` where rdflib answers, beside the same shape
without the reassignment, which still answers as rdflib does. The native
``Store.query`` keeps refusing the same text.
"""

from __future__ import annotations

from types import ModuleType

import pytest

import purrdf
from purrdf.compat.rdflib._rebinding import UnmodelledReassignment

EX = "http://example.org/"

#: Queries that assign the bound ``?this``, and neighbours that do not.
QUERIES = [
    f"SELECT ?value WHERE {{ BIND(<{EX}b> AS ?this) ?this <{EX}p> ?value }}",
    f"SELECT (COUNT(*) AS ?value) WHERE {{ ?s <{EX}p> ?o BIND(<{EX}b> AS ?this) }}",
    f"SELECT ?value WHERE {{ {{ SELECT (<{EX}b> AS ?this) WHERE {{}} }} ?this <{EX}p> ?value }}",
    f"SELECT ?this WHERE {{ BIND(<{EX}b> AS ?this) }}",
    f"SELECT (<{EX}b> AS ?this) WHERE {{}}",
    f"SELECT ?value WHERE {{ ?x <{EX}p> ?value BIND(<{EX}b> AS ?this) FILTER(?this = <{EX}b>) }}",
    f"SELECT ?this ?value WHERE {{ ?this <{EX}p> ?value BIND(<{EX}b> AS ?this) }}",
    f"SELECT (STR(?this) AS ?t) WHERE {{ BIND(<{EX}b> AS ?this) }}",
    f"SELECT (COUNT(*) AS ?this) WHERE {{ ?s <{EX}p> ?o }}",
    # Neighbours: no reassignment, so the shim passes the text through unchanged.
    f"SELECT ?this ?value WHERE {{ ?this <{EX}p> ?value }}",
    f"SELECT ?value WHERE {{ BIND(<{EX}b> AS ?fresh) ?fresh <{EX}p> ?value }}",
    f"SELECT ?value WHERE {{ VALUES ?this {{ <{EX}b> }} ?this <{EX}p> ?value }}",
    f"SELECT ?value WHERE {{ VALUES (?this) {{ (<{EX}a>) (<{EX}b>) }} ?this <{EX}p> ?value }}",
    f"SELECT ?this ?value WHERE {{ ?this <{EX}p> ?value }} ORDER BY ?this",
    # A `VALUES (?this)` row list beside a reassignment: the bracketed variable there
    # is a VALUES column, not an expression read.
    f"SELECT ?value WHERE {{ {{ VALUES (?this) {{ (<{EX}a>) }} ?this <{EX}p> ?value }} "
    f"BIND(<{EX}b> AS ?this) }}",
    # Neighbours of the unmodelled shapes below: the same constructs, no reassignment.
    f"SELECT ?value ?this WHERE {{ ?s <{EX}p> ?value OPTIONAL {{ ?this <{EX}p> ?value }} }}",
    f"SELECT ?value WHERE {{ ?s <{EX}p> ?value MINUS {{ ?this <{EX}p> ?value }} }}",
    f"SELECT ?value WHERE {{ ?s <{EX}p> ?value FILTER EXISTS {{ ?this <{EX}p> ?value }} }}",
    f"SELECT ?this (COUNT(*) AS ?n) WHERE {{ ?this <{EX}p> ?value }} GROUP BY ?this",
    f"SELECT * WHERE {{ ?this <{EX}p> ?value }}",
    f"SELECT ?value WHERE {{ ?s <{EX}p> ?value OPTIONAL {{ BIND(<{EX}b> AS ?fresh) }} }}",
    # The reassignment in a UNION branch.
    f"SELECT ?value ?this WHERE {{ {{ ?s <{EX}p> ?value }} UNION "
    f"{{ BIND(<{EX}b> AS ?this) ?this <{EX}p> ?value }} }}",
    # A reassignment under an `ORDER BY` of the reassigned and of the bound variable.
    f"SELECT ?value WHERE {{ ?s <{EX}p> ?value BIND(?value AS ?this) }} ORDER BY ?this",
    f"SELECT ?value WHERE {{ ?s <{EX}p> ?value BIND(?value AS ?this) }} ORDER BY DESC(?this)",
]

#: Reassignments in shapes whose rdflib answer comes from how its evaluator merges
#: solution mappings around the assignment, which the shim's text rewrite does not
#: reproduce: the shim raises rather than answer them differently.
UNMODELLED = [
    # A reassignment inside a sub-SELECT's WHERE: rdflib projects the assigned value
    # through the inner SELECT, alone and under an outer FILTER on it.
    f"SELECT ?this ?value WHERE {{ {{ SELECT ?this ?value WHERE {{ ?this <{EX}p> ?value "
    f"BIND(<{EX}z> AS ?this) }} }} }}",
    f"SELECT ?value WHERE {{ {{ SELECT ?this ?value WHERE {{ ?this <{EX}p> ?value "
    f"BIND(<{EX}z> AS ?this) }} }} FILTER(?this = <{EX}z>) }}",
    # A CONSTRUCT template reads the assigned value.
    f"CONSTRUCT {{ ?this <{EX}r> ?value }} WHERE {{ ?x <{EX}p> ?value BIND(<{EX}z> AS ?this) }}",
    # Two assignments in one group: rdflib keeps the later one.
    f"SELECT ?this WHERE {{ ?this <{EX}p> ?value BIND(<{EX}z> AS ?this) "
    f"BIND(<{EX}w> AS ?this) }}",
    f"SELECT ?value WHERE {{ ?x <{EX}p> ?value OPTIONAL {{ BIND(<{EX}b> AS ?this) }} }}",
    f"SELECT ?value ?this WHERE {{ ?s <{EX}p> ?value "
    f"OPTIONAL {{ ?this <{EX}p> ?value BIND(<{EX}b> AS ?this) }} }}",
    f"SELECT ?value WHERE {{ ?s <{EX}p> ?value BIND(<{EX}b> AS ?this) "
    f"OPTIONAL {{ ?this <{EX}p> ?value }} }}",
    f"SELECT ?value WHERE {{ ?s <{EX}p> ?value "
    f"MINUS {{ BIND(<{EX}b> AS ?this) ?this <{EX}p> ?value }} }}",
    f"SELECT ?value WHERE {{ ?s <{EX}p> ?value BIND(<{EX}b> AS ?this) "
    f"MINUS {{ ?this <{EX}p> ?value }} }}",
    f"SELECT ?value WHERE {{ ?s <{EX}p> ?value "
    f"FILTER EXISTS {{ BIND(<{EX}b> AS ?this) ?this <{EX}p> ?value }} }}",
    f"SELECT ?value ?this WHERE {{ ?this <{EX}p> ?value {{ BIND(<{EX}b> AS ?this) }} }}",
    f"SELECT ?this (COUNT(*) AS ?n) WHERE {{ ?s <{EX}p> ?value BIND(<{EX}b> AS ?this) }} "
    f"GROUP BY ?this",
    f"SELECT * WHERE {{ ?s <{EX}p> ?value BIND(<{EX}b> AS ?this) }}",
]

#: Queries whose row ORDER is part of the answer: compared unsorted.
ORDERED = [
    f"SELECT ?value WHERE {{ ?s <{EX}p> ?value BIND(?value AS ?this) }} ORDER BY ?this",
    f"SELECT ?value WHERE {{ ?s <{EX}p> ?value BIND(?value AS ?this) }} ORDER BY DESC(?this)",
    f"SELECT ?value ?this WHERE {{ ?s <{EX}p> ?value BIND(?value AS ?this) }} "
    f"ORDER BY DESC(STR(?this))",
    f"SELECT ?this ?value WHERE {{ ?this <{EX}p> ?value }} ORDER BY DESC(?value)",
]


def _answer(
    mod: ModuleType, query: str, *, ordered: bool = False
) -> tuple[list[str], list[tuple[str, ...]]]:
    graph = mod.Graph()
    for subject, obj in [("a", "o1"), ("a", "o2"), ("b", "o3")]:
        graph.add(
            (
                mod.URIRef(f"{EX}{subject}"),
                mod.URIRef(f"{EX}p"),
                mod.URIRef(f"{EX}{obj}"),
            )
        )
    result = graph.query(query, initBindings={"this": mod.URIRef(f"{EX}a")})
    if result.type == "CONSTRUCT":
        return [], sorted(tuple(str(term) for term in triple) for triple in result.graph)
    names = [str(variable) for variable in result.vars]
    rows = [tuple(str(cell) for cell in row) for row in result]
    if "SELECT *" in query:
        # rdflib orders a `SELECT *` projection by set iteration, which varies from run
        # to run, so only the column SET is compared, each row read in name order.
        order = sorted(range(len(names)), key=names.__getitem__)
        names = [names[index] for index in order]
        rows = [tuple(row[index] for index in order) for row in rows]
    return names, rows if ordered else sorted(rows)


@pytest.mark.parametrize("query", QUERIES)
def test_the_shim_answers_a_reassignment_as_rdflib_does(
    compat: ModuleType, oracle: ModuleType, query: str
) -> None:
    assert _answer(compat, query) == _answer(oracle, query)


@pytest.mark.parametrize("query", ORDERED)
def test_the_shim_orders_a_reassignment_as_rdflib_does(
    compat: ModuleType, oracle: ModuleType, query: str
) -> None:
    assert _answer(compat, query, ordered=True) == _answer(oracle, query, ordered=True)


@pytest.mark.parametrize("query", UNMODELLED)
def test_an_unmodelled_reassignment_is_refused_where_rdflib_answers(
    compat: ModuleType, oracle: ModuleType, query: str
) -> None:
    # rdflib answers the query, so the refusal is the shim's own limit, never a claim
    # that the query is invalid; its neighbour without the reassignment is in QUERIES.
    _answer(oracle, query)
    with pytest.raises(UnmodelledReassignment, match="cannot answer that as rdflib does"):
        _answer(compat, query)


def test_a_prefix_iri_spelling_the_fresh_prefix_still_rewrites(
    compat: ModuleType, oracle: ModuleType
) -> None:
    # Only a variable named with the shim's fresh prefix suppresses the rewrite; the
    # same characters inside a PREFIX IRI name no variable.
    query = (
        f"PREFIX q: <{EX}__purrdf_rdflib_rebound_this/> "
        f"SELECT ?value WHERE {{ BIND(<{EX}b> AS ?this) ?this <{EX}p> ?value }}"
    )
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
    answer = store.query(fresh, substitutions={purrdf.Variable("this"): purrdf.NamedNode(f"{EX}a")})
    assert [row[0] for row in answer] == [purrdf.NamedNode(f"{EX}o1")]
