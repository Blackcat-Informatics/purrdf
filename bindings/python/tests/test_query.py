# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""SPARQL query surface for the compat shim.

Differential against the *real* rdflib oracle: native ``initBindings`` (via the
engine ``substitutions`` kwarg), ``DESCRIBE`` result form, and aggregation. The
custom-function and ``SERVICE`` gaps (the native engine cannot call back into a
Python function, and has no federation source) are ledgered strict xfails.
"""

from __future__ import annotations

from types import ModuleType

EX = "http://example.org/"


def _social_graph(mod: ModuleType) -> object:
    """A small social graph: a knows b/c, b knows d; names on a and b."""
    g = mod.Graph()
    knows = mod.URIRef(f"{EX}knows")
    name = mod.URIRef(f"{EX}name")
    g.add((mod.URIRef(f"{EX}a"), knows, mod.URIRef(f"{EX}b")))
    g.add((mod.URIRef(f"{EX}a"), knows, mod.URIRef(f"{EX}c")))
    g.add((mod.URIRef(f"{EX}b"), knows, mod.URIRef(f"{EX}d")))
    g.add((mod.URIRef(f"{EX}a"), name, mod.Literal("Ann")))
    g.add((mod.URIRef(f"{EX}b"), name, mod.Literal("Bob")))
    return g


# ── initBindings via native substitutions ─────────────────────────────────────────


def test_initbindings_select_matches_oracle(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """A SELECT pre-bound via ``initBindings`` matches rdflib's results."""

    def run(mod: ModuleType) -> list[str]:
        g = _social_graph(mod)
        res = g.query(
            "SELECT ?o WHERE { ?s ?p ?o }",
            initBindings={"s": mod.URIRef(f"{EX}a"), "p": mod.URIRef(f"{EX}knows")},
        )
        return sorted(str(row[0]) for row in res)

    assert run(compat) == run(oracle) == [f"{EX}b", f"{EX}c"]


def test_initbindings_projects_bound_variable(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """A pre-bound variable stays projectable (native substitution semantics)."""

    def run(mod: ModuleType) -> list[tuple[str, str]]:
        g = _social_graph(mod)
        res = g.query(
            "SELECT ?s ?o WHERE { ?s ?p ?o }",
            initBindings={"s": mod.URIRef(f"{EX}a"), "p": mod.URIRef(f"{EX}knows")},
        )
        return sorted((str(row[0]), str(row[1])) for row in res)

    assert run(compat) == run(oracle)


def test_initbindings_ask_matches_oracle(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """An ASK pre-bound via ``initBindings`` matches rdflib's boolean."""

    def run(mod: ModuleType) -> bool:
        g = _social_graph(mod)
        return bool(
            g.query(
                "ASK { ?s ?p ?o }",
                initBindings={"s": mod.URIRef(f"{EX}a"), "o": mod.URIRef(f"{EX}b")},
            )
        )

    assert run(compat) == run(oracle) is True


def test_initbindings_construct_matches_oracle(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """A CONSTRUCT pre-bound via ``initBindings`` builds the same triples."""

    def run(mod: ModuleType) -> list[tuple[str, str, str]]:
        g = _social_graph(mod)
        res = g.query(
            f"CONSTRUCT {{ ?s <{EX}reaches> ?o }} WHERE {{ ?s ?p ?o }}",
            initBindings={"s": mod.URIRef(f"{EX}a"), "p": mod.URIRef(f"{EX}knows")},
        )
        return sorted((str(s), str(p), str(o)) for s, p, o in res)

    assert run(compat) == run(oracle)


# ── DESCRIBE result form ──────────────────────────────────────────────────────────


def test_describe_result_form_and_triples(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """DESCRIBE returns a graph result whose triples match rdflib's."""

    def run(mod: ModuleType) -> tuple[str, list[tuple[str, str, str]]]:
        g = _social_graph(mod)
        res = g.query(f"DESCRIBE <{EX}a>")
        triples = sorted((str(s), str(p), str(o)) for s, p, o in res)
        return (res.type, triples)

    c_type, c_triples = run(compat)
    o_type, o_triples = run(oracle)
    assert c_type == o_type == "DESCRIBE"
    assert c_triples == o_triples


# ── aggregation (attempted; lands green) ──────────────────────────────────────────


def test_group_concat_aggregation_matches_oracle(
    compat: ModuleType, oracle: ModuleType
) -> None:
    """GROUP_CONCAT aggregation produces the same multiset of tokens as rdflib."""

    def run(mod: ModuleType) -> set[str]:
        g = _social_graph(mod)
        res = g.query(
            f"SELECT (GROUP_CONCAT(?o; SEPARATOR=',') AS ?g) "
            f"WHERE {{ <{EX}a> <{EX}knows> ?o }}"
        )
        (row,) = list(res)
        return set(str(row[0]).split(","))

    assert run(compat) == run(oracle) == {f"{EX}b", f"{EX}c"}


# ── ledgered runtime gaps (strict xfail) ──────────────────────────────────────────


def test_register_custom_function_executes(compat: ModuleType) -> None:
    """A registered Python custom function is invoked during evaluation.

    Deferred: the native engine evaluates in Rust and cannot call an arbitrary
    Python callable, so registration records the function but does not execute it.
    """
    from purrdf.compat.rdflib.plugins.sparql import register_custom_function

    def to_upper(x: object) -> object:
        return compat.Literal(str(x).upper())

    register_custom_function(compat.URIRef(f"{EX}fn/upper"), to_upper, override=True)
    g = compat.Graph()
    g.add((compat.URIRef(f"{EX}s"), compat.URIRef(f"{EX}p"), compat.Literal("hi")))
    res = g.query(
        f"SELECT (<{EX}fn/upper>(?o) AS ?u) WHERE {{ <{EX}s> <{EX}p> ?o }}"
    )
    assert [str(row[0]) for row in res] == ["HI"]


def test_service_federation_matches_oracle(compat: ModuleType) -> None:
    """A SERVICE clause federates to a remote endpoint.

    Deferred: the native engine has no remote query source configured, so a
    SERVICE clause is a hard error rather than a federated fetch.
    """
    g = compat.Graph()
    g.add((compat.URIRef(f"{EX}s"), compat.URIRef(f"{EX}p"), compat.Literal("hi")))
    res = g.query(
        f"SELECT ?o WHERE {{ SERVICE <{EX}sparql> {{ <{EX}s> <{EX}p> ?o }} }}"
    )
    assert [str(row[0]) for row in res] == ["hi"]
