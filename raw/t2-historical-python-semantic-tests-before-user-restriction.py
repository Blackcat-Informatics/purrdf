# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: These differentials execute Python Graph and processor objects against the genuine Python RDFLib distribution.
"""Contextual mapping parity through the actual compatibility consumers."""

import json
from collections import Counter
from pathlib import Path

import pytest

EX = "http://example.org/"
FIXTURES = json.loads(
    (
        Path(__file__).resolve().parents[3]
        / "crates/sparql-eval/tests/fixtures/contextual-mappings.json"
    ).read_text()
)
CASES = [
    (fixture, case)
    for fixture in FIXTURES
    for case in fixture["cases"]
    if not case.get("count_callback") or case["oracle"].get("callback_invocations") == 0
]
assert len(CASES) == 198
assert sum(len(fixture["cases"]) for fixture in FIXTURES) == 241


def graph_for(module, fixture, route):
    graph = getattr(module, "Graph" if route == "processor" else route)()
    for triple in fixture["triples"]:
        graph.add(tuple(module.URIRef(EX + term) for term in triple))
    for subject, predicate, obj, name in fixture.get("quads", []):
        terms = tuple(module.URIRef(EX + term) for term in (subject, predicate, obj))
        graph.get_context(module.URIRef(EX + name)).add(terms)
    return graph


def term_key(module, term):
    if term is None:
        return None
    if isinstance(term, module.URIRef):
        return ("iri", str(term))
    if isinstance(term, module.BNode):
        return ("blank", str(term))
    assert isinstance(term, module.Literal), type(term)
    return literal_key(
        str(term), str(term.datatype) if term.datatype else None, term.language
    )


def literal_key(lexical, datatype, language):
    datatype = datatype or (
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString"
        if language
        else "http://www.w3.org/2001/XMLSchema#string"
    )
    return (
        "literal",
        lexical,
        datatype,
        language.lower() if language else None,
    )


def run(module, fixture, case, route):
    graph = graph_for(module, fixture, route)
    bindings = {name: module.URIRef(value) for name, value in case["initial"].items()}
    if route == "processor":
        if module.__name__ == "rdflib":
            from rdflib.plugins.sparql.processor import SPARQLProcessor
        else:
            from purrdf.compat.rdflib.plugins.sparqlprocessor import SPARQLProcessor
        result = SPARQLProcessor(graph).query(case["query"], initBindings=bindings)
        if isinstance(result, dict):
            raw = result
            result = module.query.Result(raw["type_"])
            result.vars = raw.get("vars_")
            result.bindings = raw.get("bindings", [])
            result.askAnswer = raw.get("askAnswer")
            result.graph = raw.get("graph")
    else:
        result = graph.query(case["query"], initBindings=bindings)
    if result.type == "ASK":
        return result.type, bool(result)
    if result.type in ("CONSTRUCT", "DESCRIBE"):
        return result.type, result.graph
    names = [str(name) for name in result.vars]
    # Stable sorting preserves repeated column occurrences and their cells.
    indices = sorted(range(len(names)), key=lambda index: names[index])
    rows = [tuple(term_key(module, row[index]) for index in indices) for row in result]
    return (
        result.type,
        tuple(names[index] for index in indices),
        rows if case.get("ordered") else Counter(rows),
    )


@pytest.mark.parametrize("route", ["Graph", "Dataset", "ConjunctiveGraph", "processor"])
@pytest.mark.parametrize("fixture,case", CASES, ids=[case["name"] for _, case in CASES])
def test_contextual_matrix(compat, oracle, fixture, case, route):
    assert oracle.__version__ == "7.6.0"
    assert compat is not oracle
    if fixture.get("quads") and route in ("Graph", "processor"):
        # Named datasets require a dataset consumer, while all other inputs run
        # through every graph/processor route.
        route = "Dataset"
    expected = run(oracle, fixture, case, route)
    actual = run(compat, fixture, case, route)
    if expected[0] in ("CONSTRUCT", "DESCRIBE"):
        from rdflib.compare import isomorphic

        converted = oracle.Graph()
        for triple in actual[1]:
            converted.add(
                tuple(
                    oracle.URIRef(str(term))
                    if isinstance(term, compat.URIRef)
                    else oracle.BNode(str(term))
                    if isinstance(term, compat.BNode)
                    else oracle.Literal(
                        str(term), lang=term.language, datatype=term.datatype
                    )
                    for term in triple
                )
            )
        assert actual[0] == expected[0]
        assert isomorphic(converted, expected[1])
    else:
        assert actual == expected


@pytest.mark.parametrize("matching", [False, True])
def test_nested_exists_preserves_enclosing_context(compat, oracle, matching):
    fixture = {
        "triples": [
            ["a", "p", "b"],
            ["a", "p", "c"],
            ["a", "q", "z"],
            ["z", "r", "b"],
            ["z" if matching else "other", "r", "c"],
        ]
    }
    case = {
        "initial": {},
        "query": "SELECT ?s WHERE { ?s <http://example.org/q> ?z . "
        "?s <http://example.org/p> ?x FILTER NOT EXISTS { "
        "?s <http://example.org/p> ?y FILTER(?x != ?y) "
        "FILTER NOT EXISTS { ?z <http://example.org/r> ?y } } }",
    }
    expected = run(oracle, fixture, case, "Graph")
    assert sum(expected[2].values()) == 1 + int(matching)
    assert run(compat, fixture, case, "Graph") == expected


@pytest.mark.parametrize("negated", [False, True])
def test_nested_exists_context_survives_local_projection(compat, oracle, negated):
    case = {
        "initial": {},
        "query": "PREFIX ex: <http://example.org/> SELECT ?outer WHERE { "
        "VALUES ?outer { ex:a ex:b } FILTER EXISTS { "
        "{ SELECT ?local WHERE { VALUES ?local { ex:v } } } FILTER "
        + ("NOT " if negated else "")
        + "EXISTS { ?outer ex:p ?local } } }",
    }
    fixture = {"triples": [["a", "p", "v"]]}
    expected = run(oracle, fixture, case, "Graph")
    assert expected[2] == Counter({(("iri", EX + ("b" if negated else "a")),): 1})
    assert run(compat, fixture, case, "Graph") == expected


@pytest.mark.parametrize("nested", [False, True])
def test_exists_filter_group_boundary(compat, oracle, nested):
    tail = "{ FILTER(?x=1) VALUES ?y {1} }" if nested else "FILTER(?x=1) VALUES ?y {1}"
    case = {
        "initial": {},
        "query": "SELECT ?x WHERE { VALUES ?x {1} FILTER EXISTS { FILTER(?x=1) "
        + tail
        + " } }",
    }
    expected = run(oracle, {"triples": []}, case, "Graph")
    assert sum(expected[2].values()) == int(not nested)
    assert run(compat, {"triples": []}, case, "Graph") == expected


@pytest.mark.parametrize("first", ["false", "1/0"])
@pytest.mark.parametrize("reverse", [False, True])
def test_group_filters_evaluate_reached_graph_errors(compat, oracle, first, reverse):
    filters = [
        f"FILTER({first})",
        "FILTER(EXISTS { GRAPH <http://example.org/g> { ?s ?p ?o } })",
    ]
    if reverse:
        filters.reverse()
    query = "SELECT ?x WHERE { VALUES ?x {1} " + " ".join(filters) + " }"
    for module in (oracle, compat):
        with pytest.raises(Exception, match="dataset|single graph"):
            list(module.Graph().query(query))


@pytest.mark.parametrize("initial", [False, True])
@pytest.mark.parametrize("kind", ["labelled", "fresh", "count"])
def test_query_blank_identity(compat, oracle, initial, kind):
    expression = 'BNODE("label")' if kind != "fresh" else "BNODE()"
    query = f"SELECT ?x ?a ?b WHERE {{ VALUES ?x {{1 2 3}} BIND({expression} AS ?a) BIND({expression} AS ?b) }} ORDER BY ?x"
    if kind == "count":
        query = 'SELECT (COUNT(DISTINCT ?a) AS ?n) WHERE { VALUES ?x {1 2 3} BIND(BNODE("label") AS ?a) }'
    answers = []
    for module in (compat, oracle):
        rows = list(
            module.Graph().query(
                query, initBindings={"this": module.URIRef(EX + "a")} if initial else {}
            )
        )
        if kind == "count":
            answers.append([term_key(module, row[0]) for row in rows])
        else:
            assert len(rows) == 3
            blanks = [term for row in rows for term in row[1:]]
            assert all(isinstance(term, module.BNode) for term in blanks)
            answers.append(
                (len(set(blanks)), [term_key(module, row[0]) for row in rows])
            )
    assert answers[0] == answers[1]
    if kind != "count":
        assert answers[0][0] == (1 if kind == "labelled" else 6)


def frozen_result(case):
    answer = case["oracle"]
    if answer["type"] == "ASK":
        return "ASK", answer.get("boolean", answer.get("answer"))
    if answer["type"] in ("CONSTRUCT", "DESCRIBE"):
        return answer["type"], Counter(
            tuple(tuple(term) for term in triple) for triple in answer["triples"]
        )
    names = answer["columns"]
    indices = sorted(range(len(names)), key=lambda index: names[index])

    def term(encoded):
        if encoded is None:
            return None
        return literal_key(*encoded[1:]) if encoded[0] == "literal" else tuple(encoded)

    rows = [tuple(term(row[index]) for index in indices) for row in answer["rows"]]
    return (
        "SELECT",
        tuple(names[index] for index in indices),
        rows if case.get("ordered") else Counter(rows),
    )


def shadow_matrix():
    import rdflib

    assert rdflib.Graph.__module__.startswith("purrdf")
    executed = 0
    for fixture, case in CASES:
        actual = run(
            rdflib, fixture, case, "Dataset" if fixture.get("quads") else "Graph"
        )
        if actual[0] in ("CONSTRUCT", "DESCRIBE"):
            actual = (
                actual[0],
                Counter(
                    tuple(term_key(rdflib, term) for term in triple)
                    for triple in actual[1]
                ),
            )
        assert actual == frozen_result(case), case["name"]
        executed += 1
    assert executed == 198
    print(f"shadow contextual inputs: {executed}")


def test_shadow_contextual_matrix():
    from _shadow_test_utils import _run_in_shadow

    assert (
        _run_in_shadow(
            f"import sys; sys.path.insert(0, {str(Path(__file__).resolve().parent)!r}); from test_contextual_mappings import shadow_matrix; shadow_matrix()"
        ).strip()
        == "shadow contextual inputs: 198"
    )


def test_contextual_relation_configuration(compat):
    import purrdf

    graph = compat.Graph()
    rows = list(
        graph.query(
            f"SELECT ?this ?label WHERE {{ ?this <{EX}rel> ?label BIND(<{EX}replacement> AS ?this) }}",
            initBindings={"this": compat.URIRef(EX + "a")},
            relations={
                EX + "rel": (
                    1,
                    1,
                    [
                        [
                            purrdf.NamedNode(EX + "a"),
                            purrdf.Literal("bonjour", language="fr"),
                        ]
                    ],
                )
            },
        )
    )
    assert len(rows) == 1
    assert rows[0].this == compat.URIRef(EX + "replacement")
    assert rows[0].label == compat.Literal("bonjour", lang="fr")
    with pytest.raises(ValueError):
        graph.query(
            f"SELECT ?label WHERE {{ ?this <{EX}rel> ?label }}",
            relations={EX + "rel": (1, 1, [[]])},
        )


def test_duplicate_column_cells(compat, oracle):
    query = "SELECT (?x AS ?v) (1 AS ?v) WHERE { VALUES ?x {2 3} } ORDER BY ?x"
    fixture = {"triples": []}
    case = {"query": query, "initial": {}, "ordered": True}
    assert run(compat, fixture, case, "Graph") == run(oracle, fixture, case, "Graph")
    answers = []
    for module in (compat, oracle):
        rows = list(module.Graph().query(query))
        answers.append(
            [
                (
                    tuple(term_key(module, term) for term in row),
                    row.labels,
                    term_key(module, row["v"]),
                    term_key(module, row.v),
                    term_key(module, row.get("v")),
                    {
                        name: term_key(module, value)
                        for name, value in row.asdict().items()
                    },
                )
                for row in rows
            ]
        )
    assert answers[0] == answers[1]


def test_native_contextual_term_boundary():
    import purrdf

    store = purrdf.Store()
    terms = [
        purrdf.NamedNode(EX + "a"),
        purrdf.BlankNode("focus"),
        purrdf.Literal(
            "007", datatype=purrdf.NamedNode("http://www.w3.org/2001/XMLSchema#integer")
        ),
        purrdf.Literal("مرحبا", language="ar", direction="rtl"),
        purrdf.Triple(
            purrdf.BlankNode("subject"),
            purrdf.NamedNode(EX + "p"),
            purrdf.Literal("object"),
        ),
    ]
    for term in terms:
        result = store.query_rdflib(
            "SELECT ?this ?carrier WHERE { VALUES ?carrier {1} }",
            substitutions={purrdf.Variable("this"): term},
        )
        row = next(result)
        del result
        assert row["this"] == term
    with pytest.raises(TypeError, match="substitution keys must be Variable"):
        store.query_rdflib("ASK {}", substitutions={"this": terms[0]})
    store.load(f"_:focus <{EX}p> <{EX}o> .", purrdf.RdfFormat.N_TRIPLES)
    scoped = next(store.query(f"SELECT ?focus WHERE {{ ?focus <{EX}p> <{EX}o> }}"))[0]
    row = next(
        store.query_rdflib(
            f"SELECT ?focus ?o WHERE {{ ?focus <{EX}p> ?o }}",
            substitutions={purrdf.Variable("focus"): scoped},
        )
    )
    assert row["focus"] == scoped
    assert row["o"] == purrdf.NamedNode(EX + "o")


def test_duplicate_unbound_row_access(compat, oracle):
    from purrdf.compat.rdflib.query import ResultRow
    from rdflib.query import ResultRow as OracleRow

    bound = compat.URIRef(EX + "a")
    actual = ResultRow((bound, None), ("v", "v"))
    # The genuine row constructor reads a mapping; construct the same exposed
    # tuple with its final-label table to exercise duplicate-position lookup.
    expected = tuple.__new__(OracleRow, (oracle.URIRef(EX + "a"), None))
    expected.labels = {"v": 1}
    assert actual.labels == expected.labels
    assert actual.asdict() == expected.asdict() == {}
    assert actual["v"] is expected["v"] is None
    assert actual.get("v", bound) is expected.get("v", bound) is None
    assert actual.get("missing", bound) == bound


def test_contextual_aggregate_and_standpoint_configuration(compat):
    graph = compat.Graph()
    namespace = EX + "aggregate/"
    query = (
        f"SELECT (AGG(<{namespace}MEDIAN>, ?v) AS ?m) WHERE {{ VALUES ?v {{1 2 3}} }}"
    )
    assert str(next(iter(graph.query(query, aggregate_namespace=namespace))).m) == "2"
    extension = EX + "extension/"
    held = f"ASK {{ FILTER(<{extension}heldIn>(<{EX}r>, <{EX}s>)) }}"
    with pytest.raises(ValueError, match="standpoint predicate configuration"):
        graph.query(held, extension_namespaces=[extension])
    assert not bool(
        graph.query(
            held,
            extension_namespaces=[extension],
            standpoint_predicates=(EX + "accordingTo", EX + "sharpens"),
        )
    )


@pytest.mark.parametrize("kind", ["Dataset", "ConjunctiveGraph"])
@pytest.mark.parametrize("union", [False, True])
def test_dataset_query_scope(compat, oracle, kind, union):
    answers = []
    for module in (compat, oracle):
        dataset = getattr(module, kind)()
        dataset.default_union = union
        predicate = module.URIRef(EX + "p")
        subject = module.URIRef(EX + "s")
        dataset.add((subject, predicate, module.URIRef(EX + "default")))
        for name in ("g", "h"):
            dataset.get_context(module.URIRef(EX + name)).add(
                (subject, predicate, module.URIRef(EX + "named"))
            )
        query = f"SELECT ?o WHERE {{ ?s <{EX}p> ?o }}"
        answers.append(sorted(str(row.o) for row in dataset.query(query)))
        graph = dataset.get_context(module.URIRef(EX + "g"))
        assert [str(row.o) for row in graph.query(query)] == [EX + "named"]
        if module is compat:
            from purrdf.compat.rdflib.plugins.sparqlprocessor import SPARQLProcessor

            assert [str(row.o) for row in SPARQLProcessor(graph).query(query)] == [
                EX + "named"
            ]
        with pytest.raises(Exception, match="dataset|Dataset"):
            list(
                graph.query(f"SELECT ?o WHERE {{ GRAPH <{EX}h> {{ ?s <{EX}p> ?o }} }}")
            )
        with pytest.raises(Exception, match="dataset|Dataset"):
            list(
                graph.query(
                    f"ASK {{ FILTER EXISTS {{ GRAPH <{EX}h> {{ ?s <{EX}p> ?o }} }} }}"
                )
            )
        assert [
            str(row.o)
            for row in dataset.query(
                f"SELECT ?o FROM <{EX}g> WHERE {{ ?s <{EX}p> ?o }}"
            )
        ] == [EX + "named"]
        assert [
            str(row.o)
            for row in dataset.query(
                f"SELECT ?o FROM NAMED <{EX}g> WHERE {{ GRAPH <{EX}g> {{ ?s <{EX}p> ?o }} }}"
            )
        ] == [EX + "named"]
    assert (
        answers[0]
        == answers[1]
        == ([EX + "default", EX + "named"] if union else [EX + "default"])
    )


def test_single_graph_lazy_dataset_error(compat, oracle):
    graph_body = f"GRAPH <{EX}g> {{ ?x ?p ?y }}"
    empty_queries = [
        f"SELECT ?s WHERE {{ ?s <{EX}p> ?o OPTIONAL {{ {graph_body} }} }}",
        f"SELECT ?s WHERE {{ ?s <{EX}p> ?o FILTER EXISTS {{ {graph_body} }} }}",
    ]
    for module in (compat, oracle):
        graph = module.Graph()
        for query in empty_queries:
            assert list(graph.query(query)) == []
        query = (
            f"SELECT ?x WHERE {{ BIND(IF(true, 1, EXISTS {{ {graph_body} }}) AS ?x) }}"
        )
        assert str(next(iter(graph.query(query))).x) == "1"
        for query in (
            f"SELECT ?s WHERE {{ ?s <{EX}p> ?o MINUS {{ {graph_body} }} }}",
            f"SELECT ?x WHERE {{ BIND(IF(false, 1, EXISTS {{ {graph_body} }}) AS ?x) }}",
        ):
            with pytest.raises(Exception, match="dataset|Dataset"):
                list(graph.query(query))
