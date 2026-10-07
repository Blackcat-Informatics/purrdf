# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: These tests invoke Python objects, PyO3 argument extraction, Python result accessors and the isolated Python import shadow.
"""Irreducible Python bindings; query semantic controls live in Rust."""

import pytest

EX = "http://example.org/"


class BindingDoor:
    """Record the Python call while delegating every operation to the real store."""

    def __init__(self, store):
        self.store = store
        self.calls = []

    def __getattr__(self, name):
        return getattr(self.store, name)

    def query_rdflib(self, query, **kwargs):
        self.calls.append((query, kwargs))
        return self.store.query_rdflib(query, **kwargs)


@pytest.mark.parametrize("route", ["Graph", "Dataset", "ConjunctiveGraph", "processor"])
def test_python_contextual_dispatch(compat, route):
    """Only Python can dispatch these Python receiver types and keyword arguments."""
    graph = getattr(compat, "Graph" if route == "processor" else route)()
    graph._store = BindingDoor(graph._store)
    query = f"SELECT ?this WHERE {{ BIND(<{EX}replacement> AS ?this) }}"
    kwargs = {"initBindings": {"this": compat.URIRef(EX + "initial")}}
    if route == "processor":
        from purrdf.compat.rdflib.plugins.sparqlprocessor import SPARQLProcessor

        result = SPARQLProcessor(graph).query(query, **kwargs)
    else:
        result = graph.query(query, **kwargs)
    assert [row.this for row in result] == [compat.URIRef(EX + "replacement")]
    assert len(graph._store.calls) == 1
    _, forwarded = graph._store.calls[0]
    assert forwarded["named_graphs"] == (route in ("Dataset", "ConjunctiveGraph"))


@pytest.mark.parametrize("kind", ["Dataset", "ConjunctiveGraph"])
@pytest.mark.parametrize("union", [False, True])
def test_python_graph_scope_forwarding(compat, kind, union):
    """Python graph views and default_union supply the native call's scope keywords."""
    import purrdf

    dataset = getattr(compat, kind)()
    dataset.default_union = union
    door = BindingDoor(dataset._store)
    dataset._store = door
    list(dataset.query("SELECT ?x WHERE { VALUES ?x {1} }"))
    assert door.calls[-1][1]["named_graphs"] is True
    assert door.calls[-1][1]["default_union"] is union
    graph = dataset.get_context(compat.URIRef(EX + "g"))
    list(graph.query("SELECT ?x WHERE { VALUES ?x {1} }"))
    assert door.calls[-1][1]["named_graphs"] is False
    assert door.calls[-1][1]["default_graph"] == purrdf.NamedNode(EX + "g")
    from purrdf.compat.rdflib.plugins.sparqlprocessor import SPARQLProcessor

    list(SPARQLProcessor(graph).query("SELECT ?x WHERE { VALUES ?x {1} }"))
    assert door.calls[-1][1]["default_graph"] == purrdf.NamedNode(EX + "g")


def test_python_shadow_dispatch():
    """The Python import resolver and shadow package cannot be exercised in Rust."""
    from _shadow_test_utils import _run_in_shadow

    output = _run_in_shadow(
        "import rdflib; "
        "assert rdflib.Graph.__module__.startswith('purrdf'); "
        "g=rdflib.Graph(); "
        "rows=list(g.query('SELECT ?this WHERE { BIND(<http://example.org/replacement> AS ?this) }', "
        "initBindings={'this':rdflib.URIRef('http://example.org/initial')})); "
        "assert rows[0].this==rdflib.URIRef('http://example.org/replacement'); "
        "print('python shadow dispatch')"
    )
    assert output.strip() == "python shadow dispatch"


def test_duplicate_column_cells(compat, oracle):
    """Python tuple positions and named/get/asdict access are Python result APIs."""
    query = "SELECT (?x AS ?v) (1 AS ?v) WHERE { VALUES ?x {2 3} } ORDER BY ?x"
    answers = []
    for module in (compat, oracle):
        rows = list(module.Graph().query(query))
        answers.append(
            [
                (
                    tuple(str(term) if term is not None else None for term in row),
                    row.labels,
                    str(row["v"]),
                    str(row.v),
                    str(row.get("v")),
                    {name: str(value) for name, value in row.asdict().items()},
                )
                for row in rows
            ]
        )
    assert answers[0] == answers[1]


def test_contextual_relation_configuration(compat):
    """Python dictionaries and keyword forwarding exercise the real PyO3 relation boundary."""
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


def test_native_contextual_term_boundary():
    """PyO3 extracts Python term objects and results retain Python-owned values after their iterator drops."""
    import purrdf
    from purrdf.compat.rdflib.namespace import XSD

    store = purrdf.Store()
    terms = [
        purrdf.NamedNode(EX + "a"),
        purrdf.BlankNode("focus"),
        purrdf.Literal("007", datatype=purrdf.NamedNode(str(XSD.integer))),
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
    """Python ResultRow lookup/default/asdict semantics cannot be tested by a native Rust result."""
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
    """Python keyword omission and tuples must reach the existing native configuration door."""
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
