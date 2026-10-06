# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: drives the xpath_regex keyword of the installed CPython extension's query, update and validation entry points

"""The dated XPath pattern law a caller selects with ``xpath_regex``.

Every Python entry point that evaluates a SPARQL ``REGEX``/``REPLACE``, a SHACL
``sh:pattern`` (in validation, rules, entailment or a node expression) or a ShEx
pattern facet takes ``xpath_regex``, whose value is one
of the stable names in ``purrdf.XPATH_REGEX_PROFILES``. A keyword that was read
but never threaded through would be invisible: the call still succeeds and the
compatibility law answers instead. So each surface is pinned by a behaviour only
the selected law has:

* ``(?:a)b`` is a non-capturing group, which XPath F&O 3.1 admits and XPath F&O
  2.0 does not. Under 3.1 it matches ``"ab"``; under 2.0 it is a pattern syntax
  error, which each host reports through its own pattern channel (an expression
  error drops a SPARQL ``FILTER`` row, a SHACL ``sh:pattern`` result, a
  nonconformant ShEx entry).
* ``^(a)\\1$`` is a backreference. Both dated laws define it, so it matches
  ``"aa"`` under each, while the compatibility law refuses backreferences.

A refusal is pinned from both sides: the three near-miss names are refused and
the exact name beside them is accepted; a pattern one byte over the native
source bound raises ``ValueError`` carrying ``xpath-pattern-bytes``, and the
pattern exactly at the bound is admitted. Every door identifies that refusal the
same way: ``xpath-pattern-bytes`` is the exception's ``message_id`` and its
``presentation``'s, as a SPARQL parse failure's condition is.
"""

from __future__ import annotations

import json
from collections.abc import Callable

import pytest

import purrdf
from purrdf import shapes, shex
from purrdf.compat import rdflib

EX = "https://example.org/"
P = f"<{EX}p>"
XPATH20 = "xpath-2.0-2010-12-14"
XPATH31 = "xpath-3.1-2017-03-21"
LAWS = (XPATH20, XPATH31)
NONCAPTURING = "(?:a)b"
BACKREFERENCE = "^(a)\\1$"
#: The native source bound of ``Limits::new``: 64 KiB of pattern UTF-8.
PATTERN_BYTES = 64 * 1024
REFUSED_NAMES = ("xpath-3.1", "XPATH-3.1-2017-03-21", "")


def assert_refusal_identity(refusal: pytest.ExceptionInfo[ValueError]) -> None:
    """The refusal is identified by its resource's code, not only named in its words."""
    error = refusal.value
    assert "xpath-pattern-bytes" in str(error)
    assert getattr(error, "message_id") == "xpath-pattern-bytes"
    assert getattr(error, "presentation") == {
        "message_id": "xpath-pattern-bytes",
        "parameters": {},
        "detail": None,
    }


def sparql_string(text: str) -> str:
    """``text`` as a SPARQL string literal (JSON escaping is a subset of SPARQL's)."""
    return json.dumps(text)


def regex_select(pattern: str) -> str:
    return (
        f"SELECT ?o WHERE {{ ?s {P} ?o FILTER(REGEX(?o, {sparql_string(pattern)})) }}"
        " ORDER BY ?o"
    )


def loaded_store() -> purrdf.Store:
    store = purrdf.Store()
    store.load(
        f'<{EX}s> {P} "ab" .\n<{EX}s> {P} "aa" .\n<{EX}s> {P} "a" .\n'.encode(),
        purrdf.RdfFormat.N_TRIPLES,
    )
    return store


def lexical(term: object) -> str:
    assert isinstance(term, purrdf.Literal)
    return term.value


def objects(result: object) -> list[str]:
    assert isinstance(result, purrdf.QuerySolutions)
    return [lexical(row["o"]) for row in result]


def outcome_objects(outcome: purrdf.QueryOutcome) -> list[str]:
    assert outcome.is_complete
    return objects(outcome.result)


# ── The accepted names ─────────────────────────────────────────────────────────


def test_profiles_are_the_two_dated_laws_oldest_first() -> None:
    assert purrdf.XPATH_REGEX_PROFILES == LAWS


# ── SPARQL: every query and update door ───────────────────────────────────────

#: (pattern, law) -> the rows a REGEX filter keeps, for the native laws.
SPARQL_EXPECTED = {
    (NONCAPTURING, XPATH31): ["ab"],
    (NONCAPTURING, XPATH20): [],
    (BACKREFERENCE, XPATH31): ["aa"],
    (BACKREFERENCE, XPATH20): ["aa"],
}


def run_query(store: purrdf.Store, text: str, law: str | None) -> list[str]:
    return objects(store.query(text, xpath_regex=law))


def run_query_governed(store: purrdf.Store, text: str, law: str | None) -> list[str]:
    return outcome_objects(store.query_governed(text, xpath_regex=law))


def run_entailment(store: purrdf.Store, text: str, law: str | None) -> list[str]:
    outcome = store.query_entailment_governed(text, "rdfs", xpath_regex=law)
    assert outcome.outcome is not None
    return outcome_objects(outcome.outcome)


def run_prepared(store: purrdf.Store, text: str, law: str | None) -> list[str]:
    return objects(store.prepare(text, xpath_regex=law).run())


def run_mutable(store: purrdf.Store, text: str, law: str | None) -> list[str]:
    dataset = purrdf.MutableDataset()
    dataset.load(
        f'<{EX}s> {P} "ab" .\n<{EX}s> {P} "aa" .\n<{EX}s> {P} "a" .\n'.encode(),
        purrdf.RdfFormat.N_TRIPLES,
    )
    return objects(dataset.query(text, xpath_regex=law))


def run_compat_graph(store: purrdf.Store, text: str, law: str | None) -> list[str]:
    graph = rdflib.Graph()
    graph.parse(
        data=f'<{EX}s> {P} "ab" .\n<{EX}s> {P} "aa" .\n<{EX}s> {P} "a" .\n',
        format="nt",
    )
    return [str(row[0]) for row in graph.query(text, xpath_regex=law)]


QUERY_DOORS: dict[str, Callable[[purrdf.Store, str, str | None], list[str]]] = {
    "query": run_query,
    "query_governed": run_query_governed,
    "query_entailment_governed": run_entailment,
    "prepare": run_prepared,
    "mutable_dataset_query": run_mutable,
    "compat_graph_query": run_compat_graph,
}


@pytest.mark.parametrize("door", sorted(QUERY_DOORS))
@pytest.mark.parametrize(("pattern", "law"), sorted(SPARQL_EXPECTED))
def test_query_regex_follows_the_selected_law(door: str, pattern: str, law: str) -> None:
    rows = QUERY_DOORS[door](loaded_store(), regex_select(pattern), law)
    assert rows == SPARQL_EXPECTED[(pattern, law)]


@pytest.mark.parametrize("door", sorted(QUERY_DOORS))
def test_query_regex_unselected_keeps_compatibility(door: str) -> None:
    # The compatibility law refuses a backreference: the filter keeps nothing,
    # where either dated law keeps "aa".
    assert QUERY_DOORS[door](loaded_store(), regex_select(BACKREFERENCE), None) == []


@pytest.mark.parametrize("law", LAWS)
def test_replace_follows_the_selected_law(law: str) -> None:
    text = (
        "SELECT ?r WHERE { BIND(REPLACE(\"aa\", "
        f"{sparql_string(BACKREFERENCE)}, \"x\") AS ?r) }}"
    )
    rows = purrdf.Store().query(text, xpath_regex=law)
    assert isinstance(rows, purrdf.QuerySolutions)
    assert [lexical(row["r"]) for row in rows] == ["x"]


def test_replace_unselected_keeps_compatibility() -> None:
    text = (
        "SELECT ?r WHERE { BIND(REPLACE(\"aa\", "
        f"{sparql_string(BACKREFERENCE)}, \"x\") AS ?r) }}"
    )
    rows = purrdf.Store().query(text)
    assert isinstance(rows, purrdf.QuerySolutions)
    assert [row["r"] for row in rows] == [None]


def regex_insert(pattern: str) -> str:
    return (
        f"INSERT {{ ?s <{EX}matched> ?o }} WHERE {{ ?s {P} ?o "
        f"FILTER(REGEX(?o, {sparql_string(pattern)})) }}"
    )


def matched(store: purrdf.Store) -> list[str]:
    return objects(
        store.query(f"SELECT ?o WHERE {{ ?s <{EX}matched> ?o }} ORDER BY ?o")
    )


@pytest.mark.parametrize("governed", [False, True], ids=["update", "update_governed"])
@pytest.mark.parametrize(("pattern", "law"), sorted(SPARQL_EXPECTED))
def test_update_where_follows_the_selected_law(
    governed: bool, pattern: str, law: str
) -> None:
    store = loaded_store()
    if governed:
        assert store.update_governed(regex_insert(pattern), xpath_regex=law).is_applied
    else:
        store.update(regex_insert(pattern), xpath_regex=law)
    assert matched(store) == SPARQL_EXPECTED[(pattern, law)]


def test_compat_graph_update_follows_the_selected_law() -> None:
    for law, expected in ((None, 0), (XPATH20, 1), (XPATH31, 1)):
        graph = rdflib.Graph()
        graph.parse(data=f'<{EX}s> {P} "aa" .\n', format="nt")
        graph.update(regex_insert(BACKREFERENCE), xpath_regex=law)
        assert len(graph) == 1 + expected, law


# ── SPARQL: a native resource refusal raises, it never answers ────────────────


def oversized(pattern_bytes: int) -> str:
    return regex_select("a" * pattern_bytes)


@pytest.mark.parametrize("law", LAWS)
@pytest.mark.parametrize(
    "door",
    [
        "query",
        "query_governed",
        "query_entailment_governed",
        "prepare",
        "mutable_dataset_query",
        "compat_graph_query",
    ],
)
def test_query_resource_refusal_raises(door: str, law: str) -> None:
    store = loaded_store()
    with pytest.raises(ValueError, match="xpath-pattern-bytes") as refusal:
        QUERY_DOORS[door](store, oversized(PATTERN_BYTES + 1), law)
    assert_refusal_identity(refusal)
    # The neighbour exactly at the bound is admitted and answers.
    assert QUERY_DOORS[door](store, oversized(PATTERN_BYTES), law) == []
    assert QUERY_DOORS[door](store, regex_select("a" * 1), law) == ["a", "aa", "ab"]


@pytest.mark.parametrize("law", LAWS)
def test_update_resource_refusal_raises_and_applies_nothing(law: str) -> None:
    store = loaded_store()
    with pytest.raises(ValueError, match="xpath-pattern-bytes") as refusal:
        store.update(regex_insert("a" * (PATTERN_BYTES + 1)), xpath_regex=law)
    assert_refusal_identity(refusal)
    with pytest.raises(ValueError, match="xpath-pattern-bytes") as refusal:
        store.update_governed(regex_insert("a" * (PATTERN_BYTES + 1)), xpath_regex=law)
    assert_refusal_identity(refusal)
    assert len(store) == 3
    store.update(regex_insert("a" * PATTERN_BYTES), xpath_regex=law)
    assert len(store) == 3


# ── SHACL: every validation door ──────────────────────────────────────────────

SH = "http://www.w3.org/ns/shacl#"
PATTERN_COMPONENT = f"{SH}PatternConstraintComponent"


def pattern_shapes(pattern: str) -> str:
    return (
        f"@prefix sh: <{SH}> .\n"
        f"<{EX}S> a sh:NodeShape ; sh:targetNode <{EX}s> ;\n"
        f"  sh:property [ sh:path <{EX}p> ; sh:pattern {json.dumps(pattern)} ] .\n"
    )


def data_nt(value: str) -> str:
    return f"<{EX}s> {P} {json.dumps(value)} .\n"


def shacl_validate(pattern: str, value: str, law: str | None) -> tuple[bool, list[str]]:
    report = shapes.validate(pattern_shapes(pattern), data_nt(value), xpath_regex=law)
    results = report["results"]
    assert isinstance(results, list)
    return bool(report["conforms"]), [str(r["component"]) for r in results]


def shacl_shapes_nt(pattern: str, value: str, law: str | None) -> tuple[bool, list[str]]:
    report = shapes.Shapes(pattern_shapes(pattern)).validate_nt(
        data_nt(value), xpath_regex=law
    )
    return report.conforms, [str(r["component"]) for r in report.results]


def shacl_shapes_store(pattern: str, value: str, law: str | None) -> tuple[bool, list[str]]:
    store = purrdf.Store()
    store.load(data_nt(value).encode(), purrdf.RdfFormat.N_TRIPLES)
    report = shapes.Shapes(pattern_shapes(pattern)).validate_store(store, xpath_regex=law)
    return report.conforms, [str(r["component"]) for r in report.results]


def shacl_prepared_nt(pattern: str, value: str, law: str | None) -> tuple[bool, list[str]]:
    prepared = shapes.Shapes(pattern_shapes(pattern)).prepare()
    report = prepared.validate_nt(data_nt(value), xpath_regex=law)
    return report.conforms, [str(r["component"]) for r in report.results]


def shacl_prepared_changes(
    pattern: str, value: str, law: str | None
) -> tuple[bool, list[str]]:
    # The incremental lane: the checked-in base holds an unrelated statement, and
    # the pending change is the statement the pattern constrains.
    store = purrdf.Store()
    store.load(f'<{EX}other> <{EX}q> "x" .\n'.encode(), purrdf.RdfFormat.N_TRIPLES)
    store.checkpoint()
    store.add(
        purrdf.Quad(
            purrdf.NamedNode(f"{EX}s"), purrdf.NamedNode(f"{EX}p"), purrdf.Literal(value)
        )
    )
    prepared = shapes.Shapes(pattern_shapes(pattern)).prepare()
    outcome = prepared.validate_store_changes(store, xpath_regex=law)
    # A Core pattern has a bounded footprint: the change moves exactly one focus node.
    assert outcome.bounded and outcome.focus_nodes == 1
    report = outcome.report
    return report.conforms, [str(r["component"]) for r in report.results]


SHACL_DOORS: dict[str, Callable[[str, str, str | None], tuple[bool, list[str]]]] = {
    "validate": shacl_validate,
    "shapes_validate_nt": shacl_shapes_nt,
    "shapes_validate_store": shacl_shapes_store,
    "prepared_validate_nt": shacl_prepared_nt,
    "prepared_validate_store_changes": shacl_prepared_changes,
}

#: (pattern, value, law) -> (conforms, result components).
SHACL_EXPECTED = {
    (NONCAPTURING, "ab", XPATH31): (True, []),
    (NONCAPTURING, "ab", XPATH20): (False, [PATTERN_COMPONENT]),
    (BACKREFERENCE, "aa", XPATH31): (True, []),
    (BACKREFERENCE, "aa", XPATH20): (True, []),
}


@pytest.mark.parametrize("door", sorted(SHACL_DOORS))
@pytest.mark.parametrize(("pattern", "value", "law"), sorted(SHACL_EXPECTED))
def test_shacl_pattern_follows_the_selected_law(
    door: str, pattern: str, value: str, law: str
) -> None:
    assert SHACL_DOORS[door](pattern, value, law) == SHACL_EXPECTED[(pattern, value, law)]


@pytest.mark.parametrize("door", sorted(SHACL_DOORS))
def test_shacl_pattern_unselected_keeps_compatibility(door: str) -> None:
    # The compatibility law refuses a backreference, which either dated law
    # matches: the same data is a sh:pattern result here.
    assert SHACL_DOORS[door](BACKREFERENCE, "aa", None) == (False, [PATTERN_COMPONENT])


def test_shacl_sparql_constraint_regex_follows_the_selected_law() -> None:
    shapes_ttl = (
        f"@prefix sh: <{SH}> .\n"
        f"<{EX}S> a sh:NodeShape ; sh:targetNode <{EX}s> ;\n"
        "  sh:sparql [ sh:select "
        + json.dumps(
            f"SELECT $this WHERE {{ $this {P} ?o "
            f"FILTER(!REGEX(?o, {sparql_string(BACKREFERENCE)})) }}"
        )
        + " ] .\n"
    )
    verdicts = {
        law: shapes.validate(shapes_ttl, data_nt("aa"), xpath_regex=law)["conforms"]
        for law in (None, *LAWS)
    }
    # Compatibility: REGEX is an expression error, so `!REGEX` is too and the
    # FILTER drops the row — the constraint reports nothing either way, so only
    # the dated laws are pinned by value here: "aa" matches, nothing is reported.
    assert verdicts[XPATH20] is True
    assert verdicts[XPATH31] is True
    mismatch = shapes.validate(shapes_ttl, data_nt("ab"), xpath_regex=XPATH31)
    assert mismatch["conforms"] is False


def sparql_constraint_shapes(pattern: str) -> str:
    return (
        f"@prefix sh: <{SH}> .\n"
        f"<{EX}S> a sh:NodeShape ; sh:targetNode <{EX}s> ;\n"
        "  sh:sparql [ sh:select "
        + json.dumps(
            f"SELECT $this WHERE {{ $this {P} ?o "
            f"FILTER(REGEX(?o, {sparql_string(pattern)})) }}"
        )
        + " ] .\n"
    )


@pytest.mark.parametrize("law", LAWS)
def test_shacl_sparql_constraint_resource_refusal_carries_the_code(law: str) -> None:
    # The constraint's own query refuses the pattern: the SPARQL engine's diagnostic
    # is identified by the same code the sh:pattern matcher's refusal is.
    with pytest.raises(ValueError, match="xpath-pattern-bytes") as refusal:
        shapes.validate(
            sparql_constraint_shapes("a" * (PATTERN_BYTES + 1)), data_nt("b"), xpath_regex=law
        )
    assert_refusal_identity(refusal)
    # The neighbour exactly at the bound is admitted: "b" does not match, so the
    # constraint selects nothing and the data conforms.
    report = shapes.validate(
        sparql_constraint_shapes("a" * PATTERN_BYTES), data_nt("b"), xpath_regex=law
    )
    assert report["conforms"] is True


@pytest.mark.parametrize("law", LAWS)
@pytest.mark.parametrize("door", sorted(SHACL_DOORS))
def test_shacl_resource_refusal_raises(door: str, law: str) -> None:
    with pytest.raises(ValueError, match="xpath-pattern-bytes") as refusal:
        SHACL_DOORS[door]("a" * (PATTERN_BYTES + 1), "a", law)
    assert_refusal_identity(refusal)
    # The neighbour exactly at the bound is admitted and reports the mismatch.
    assert SHACL_DOORS[door]("a" * PATTERN_BYTES, "a", law) == (False, [PATTERN_COMPONENT])


# ── ShEx ───────────────────────────────────────────────────────────────────────


def shexj_schema(pattern: str) -> str:
    # ShExC's REGEXP escapes admit no `\1`, so the schema is ShExJ, derived from
    # the canonical form of a ShExC schema with a placeholder pattern.
    canonical = shex.parse(f"PREFIX ex: <{EX}> ex:S {{ ex:p /PLACEHOLDER/ }}")
    assert canonical.count('"PLACEHOLDER"') == 1
    return canonical.replace('"PLACEHOLDER"', json.dumps(pattern))


def shex_conformant(pattern: str, value: str, law: str | None) -> bool:
    results = shex.validate(
        shexj_schema(pattern),
        data_nt(value),
        [(f"{EX}s", f"{EX}S")],
        schema_format="shexj",
        data_format="ntriples",
        xpath_regex=law,
    )
    assert len(results) == 1
    return results[0]["conformant"]


@pytest.mark.parametrize(
    ("pattern", "value", "law", "conformant"),
    [
        (NONCAPTURING, "ab", XPATH31, True),
        (NONCAPTURING, "ab", XPATH20, False),
        (BACKREFERENCE, "aa", XPATH31, True),
        (BACKREFERENCE, "aa", XPATH20, True),
        (BACKREFERENCE, "aa", None, False),
    ],
)
def test_shex_pattern_follows_the_selected_law(
    pattern: str, value: str, law: str | None, conformant: bool
) -> None:
    assert shex_conformant(pattern, value, law) is conformant


@pytest.mark.parametrize("law", LAWS)
def test_shex_resource_refusal_raises(law: str) -> None:
    with pytest.raises(ValueError, match="xpath-pattern-bytes") as refusal:
        shex_conformant("a" * (PATTERN_BYTES + 1), "a", law)
    assert_refusal_identity(refusal)
    assert shex_conformant("a" * PATTERN_BYTES, "a", law) is False


# ── SHACL tools: entailment, rules, node expressions ──────────────────────────

HIT = f"<{EX}hit>"


def rule_shapes(pattern: str) -> str:
    construct = (
        f"CONSTRUCT {{ $this <{EX}hit> ?v }} WHERE {{ $this {P} ?v "
        f"FILTER(REGEX(?v, {sparql_string(pattern)})) }}"
    )
    return (
        f"@prefix sh: <{SH}> .\n"
        f"<{EX}S> a sh:NodeShape ; sh:targetSubjectsOf {P} ;\n"
        f"  sh:rule [ a sh:SPARQLRule ; sh:construct {json.dumps(construct)} ] .\n"
    )


def rule_set(pattern: str) -> str:
    return (
        f"PREFIX : <{EX}>\n"
        f"RULE {{ ?s :hit ?v }} WHERE {{ ?s :p ?v FILTER(REGEX(?v, {sparql_string(pattern)})) }}\n"
    )


def expression_shapes(pattern: str) -> str:
    return (
        f"@prefix sh: <{SH}> .\n"
        "@prefix shnex: <http://www.w3.org/ns/shacl-node-expr#> .\n"
        f"_:e shnex:filterShape [ sh:pattern {json.dumps(pattern)} ] ; "
        'shnex:nodes [ shnex:var "focusNode" ] .\n'
    )


def tool_entail(pattern: str, value: str, law: str | None) -> bool:
    out = shapes.entail(rule_shapes(pattern), data_nt(value), xpath_regex=law)
    return HIT in str(out["ntriples"])


def tool_apply_rules(pattern: str, value: str, law: str | None) -> bool:
    out = shapes.apply_rules(data_nt(value), rule_shapes(pattern), xpath_regex=law)
    return HIT in str(out["inferred"])


def tool_apply_srl(pattern: str, value: str, law: str | None) -> bool:
    out = shapes.apply_rules(data_nt(value), srl=rule_set(pattern), xpath_regex=law)
    return HIT in str(out["inferred"])


def tool_eval_node_expr(pattern: str, value: str, law: str | None) -> bool:
    out = shapes.eval_node_expr(
        expression_shapes(pattern), data_nt(value), "_:e", json.dumps(value), xpath_regex=law
    )
    outputs = out["outputs"]
    assert isinstance(outputs, list)
    return outputs == [json.dumps(value)]


TOOL_DOORS: dict[str, Callable[[str, str, str | None], bool]] = {
    "entail": tool_entail,
    "apply_rules": tool_apply_rules,
    "apply_rules_srl": tool_apply_srl,
    "eval_node_expr": tool_eval_node_expr,
}

#: (pattern, value, law) -> whether the run reads the pattern as matching.
TOOL_EXPECTED = {
    (NONCAPTURING, "ab", XPATH31): True,
    (NONCAPTURING, "ab", XPATH20): False,
    (BACKREFERENCE, "aa", XPATH31): True,
    (BACKREFERENCE, "aa", XPATH20): True,
    (BACKREFERENCE, "aa", None): False,
}


@pytest.mark.parametrize("door", sorted(TOOL_DOORS))
@pytest.mark.parametrize(
    ("pattern", "value", "law"), sorted(TOOL_EXPECTED, key=lambda case: repr(case))
)
def test_shacl_tool_follows_the_selected_law(
    door: str, pattern: str, value: str, law: str | None
) -> None:
    assert TOOL_DOORS[door](pattern, value, law) is TOOL_EXPECTED[(pattern, value, law)]


@pytest.mark.parametrize("law", LAWS)
@pytest.mark.parametrize("door", sorted(TOOL_DOORS))
def test_shacl_tool_resource_refusal_raises(door: str, law: str) -> None:
    with pytest.raises(ValueError, match="xpath-pattern-bytes") as refusal:
        TOOL_DOORS[door]("a" * (PATTERN_BYTES + 1), "a", law)
    assert_refusal_identity(refusal)
    # The neighbour exactly at the bound is admitted and matches nothing.
    assert TOOL_DOORS[door]("a" * PATTERN_BYTES, "a", law) is False


# ── Unknown names are refused; the exact name beside them is accepted ─────────

REFUSING_DOORS: dict[str, Callable[[str | None], object]] = {
    "query": lambda law: loaded_store().query("ASK {}", xpath_regex=law),
    "query_governed": lambda law: loaded_store().query_governed("ASK {}", xpath_regex=law),
    "query_entailment_governed": lambda law: loaded_store().query_entailment_governed(
        "ASK {}", "rdfs", xpath_regex=law
    ),
    "update": lambda law: loaded_store().update(
        f"INSERT DATA {{ <{EX}s> {P} 1 }}", xpath_regex=law
    ),
    "update_governed": lambda law: loaded_store().update_governed(
        f"INSERT DATA {{ <{EX}s> {P} 1 }}", xpath_regex=law
    ),
    "prepare": lambda law: loaded_store().prepare("ASK {}", xpath_regex=law),
    "compat_graph_query": lambda law: rdflib.Graph().query("ASK {}", xpath_regex=law),
    "shacl_validate": lambda law: shapes.validate(
        pattern_shapes("a"), data_nt("a"), xpath_regex=law
    ),
    "shacl_shapes_validate_nt": lambda law: shapes.Shapes(pattern_shapes("a")).validate_nt(
        data_nt("a"), xpath_regex=law
    ),
    "shacl_shapes_validate_store": lambda law: shapes.Shapes(
        pattern_shapes("a")
    ).validate_store(loaded_store(), xpath_regex=law),
    "shacl_prepared_validate_nt": lambda law: shapes.Shapes(pattern_shapes("a"))
    .prepare()
    .validate_nt(data_nt("a"), xpath_regex=law),
    "shacl_prepared_validate_store_changes": lambda law: shapes.Shapes(pattern_shapes("a"))
    .prepare()
    .validate_store_changes(loaded_store(), xpath_regex=law),
    "shacl_entail": lambda law: tool_entail("a", "a", law),
    "shacl_apply_rules": lambda law: tool_apply_rules("a", "a", law),
    "shacl_apply_rules_srl": lambda law: tool_apply_srl("a", "a", law),
    "shacl_eval_node_expr": lambda law: tool_eval_node_expr("a", "a", law),
    "shex_validate": lambda law: shex.validate(
        shexj_schema("a"),
        data_nt("a"),
        [(f"{EX}s", f"{EX}S")],
        schema_format="shexj",
        data_format="ntriples",
        xpath_regex=law,
    ),
}


@pytest.mark.parametrize("name", REFUSED_NAMES)
@pytest.mark.parametrize("door", sorted(REFUSING_DOORS))
def test_unknown_profile_name_is_refused(door: str, name: str) -> None:
    with pytest.raises(ValueError) as refusal:
        REFUSING_DOORS[door](name)
    message = str(refusal.value)
    assert "xpath_regex" in message
    assert repr(name).replace("'", '"') in message
    for accepted in LAWS:
        assert f'"{accepted}"' in message


@pytest.mark.parametrize("door", sorted(REFUSING_DOORS))
def test_exact_profile_name_beside_the_refused_ones_is_accepted(door: str) -> None:
    REFUSING_DOORS[door](XPATH31)
    REFUSING_DOORS[door](XPATH20)
    REFUSING_DOORS[door](None)


# ── Counted repetitions answer at the production defaults ─────────────────────

PROSE_WORDS = (
    "gamma", "graph", "beta", "rdf", "pattern", "shape", "delta", "alpha", "node", "sparql",
)


@pytest.fixture(scope="module")
def counted_values() -> dict[str, str]:
    """The inputs each counted shape was refused over, by name, and their completions.

    Deterministic lowercase prose joins words by single spaces; ``mixed`` is a run of
    ``a`` and ``b`` that is not a repetition of ``ab``.
    """
    words: list[str] = []
    length = 0
    while length < 8 << 20:
        index = len(words)
        words.append(PROSE_WORDS[(index * 7 + index // 3) % len(PROSE_WORDS)])
        length += len(words[-1]) + 1
    prose = " ".join(words)
    pairs = "ab" * (1 << 19)
    mixed = "abbaab" * ((4 << 20) // 6)
    plain = {
        "pairs_128k": (pairs[: 128 << 10], "c"),
        "pairs": (pairs, "c"),
        "quads": ("abcd" * (1 << 18), "e"),
        "mixed_400k": (mixed[: 400 << 10], "c"),
        "mixed_800k": (mixed[: 800 << 10], "c"),
        "mixed": (mixed, "c"),
        "prose_4m": (prose[: prose[: 4 << 20].rfind(" ")], " zzz"),
        "prose": (prose, " zzz"),
        "run": ("a" * (1 << 20), "b"),
    }
    values = {"empty": ""}
    for name, (value, suffix) in plain.items():
        values[name] = value
        values[f"{name}_completed"] = value + suffix
    return values


#: (native pattern, compatibility pattern with the same matches, plain subject).
#: Each was refused by the native matcher at its subject's size: a counted group
#: repeated from every start kept one thread per distinct count. A count the
#: compatibility law refuses to build is compared through its equivalent.
COUNTED = (
    ("(ab){1,1000}c", "(ab){1,1000}c", "pairs_128k"),
    ("(ab){2,50}c", "(ab){2,50}c", "pairs"),
    ("(ab){1,100}c", "(ab){1,100}c", "pairs"),
    ("(ab|cd){1,20}e", "(ab|cd){1,20}e", "quads"),
    ("((a|b){3}){5,9}c", "((a|b){3}){5,9}c", "mixed_400k"),
    ("((a|b){2}){2,5}c", "((a|b){2}){2,5}c", "mixed_800k"),
    ("(a|b){1,30}c", "(a|b){1,30}c", "mixed"),
    ("(a|b){3,9}c", "(a|b){3,9}c", "mixed"),
    ("(\\w+\\s){3,5}zzz", "(\\w+\\s){3,5}zzz", "prose_4m"),
    ("node.*graph.*zzz", "node.*graph.*zzz", "prose"),
    ("(a|aa){1,1000}b", "(a|aa){1,1000}b", "run"),
    ("(ab){1,100000}c", "abc", "pairs_128k"),
)


def counted_answers(values: dict[str, str], law: str | None) -> list[str]:
    """Each counted pattern decided on its plain and completed subjects, in one query."""
    store = purrdf.Store()
    store.load(
        "".join(
            f"<{EX}{name}> {P} {json.dumps(value)} .\n" for name, value in values.items()
        ).encode(),
        purrdf.RdfFormat.N_TRIPLES,
    )
    rows = []
    for index, (pattern, compatible, subject) in enumerate(COUNTED):
        literal = sparql_string(pattern if law else compatible)
        rows += [f"({index} <{EX}{subject}{tail}> {literal})" for tail in ("", "_completed")]
    nullable = "^(a?){18446744073709551616}$" if law else "^a*$"
    rows += [f"({len(COUNTED)} <{EX}{name}> {sparql_string(nullable)})" for name in ("empty", "pairs")]
    text = (
        f"SELECT ?r WHERE {{ VALUES (?i ?s ?p) {{ {' '.join(rows)} }} "
        f"?s {P} ?v BIND(REGEX(?v, ?p) AS ?r) }} ORDER BY ?i ?s"
    )
    result = store.query(text, xpath_regex=law)
    assert isinstance(result, purrdf.QuerySolutions)
    return [lexical(row["r"]) for row in result]


@pytest.mark.parametrize("law", LAWS)
def test_counted_repetitions_answer_like_the_compatibility_law(
    counted_values: dict[str, str], law: str
) -> None:
    # Plain subjects fail, completed ones match, and the empty string meets a
    # nullable whole required more than u64 times.
    expected = ["false", "true"] * len(COUNTED) + ["true", "false"]
    assert counted_answers(counted_values, None) == expected
    assert counted_answers(counted_values, law) == expected


@pytest.mark.parametrize("law", LAWS)
@pytest.mark.parametrize(
    ("pattern", "compatible", "subject"),
    [COUNTED[0], COUNTED[4], COUNTED[6], COUNTED[8], COUNTED[10], COUNTED[11]],
)
def test_counted_repetitions_validate_like_the_compatibility_law(
    counted_values: dict[str, str], law: str, pattern: str, compatible: str, subject: str
) -> None:
    for name, conforms in ((subject, False), (f"{subject}_completed", True)):
        value = counted_values[name]
        assert shacl_validate(compatible, value, None)[0] is conforms
        assert shacl_validate(pattern, value, law)[0] is conforms, name
        assert shex_conformant(compatible, value, None) is conforms
        assert shex_conformant(pattern, value, law) is conforms, name


@pytest.mark.parametrize("law", LAWS)
@pytest.mark.parametrize("pattern", ["(a|aa){1,1000}b", "((a|aa){1,4})(b|c)"])
def test_counted_ambiguous_replacement_over_a_mebibyte_like_the_compatibility_law(
    law: str, pattern: str
) -> None:
    # Short runs closed by `b` or `c`, after a long run no `b` closes within a
    # thousand iterations: one replacement rewrites thousands of matches.
    runs = ["a" * 100_000]
    length, index = 100_000, 0
    while length < 1 << 20:
        run = "a" * (1 + (index * 7 + index // 5) % 13) + ("c" if index % 3 == 0 else "b")
        runs.append(run)
        length += len(run)
        index += 1
    store = purrdf.Store()
    store.load(
        f"<{EX}s> {P} {json.dumps(''.join(runs))} .\n".encode(), purrdf.RdfFormat.N_TRIPLES
    )
    text = (
        f"SELECT ?r WHERE {{ <{EX}s> {P} ?v "
        f'BIND(REPLACE(?v, {sparql_string(pattern)}, "[$1]") AS ?r) }}'
    )

    def replaced(selected: str | None) -> str:
        result = store.query(text, xpath_regex=selected)
        assert isinstance(result, purrdf.QuerySolutions)
        return lexical(next(iter(result))["r"])

    expected = replaced(None)
    assert "[a]" in expected
    assert replaced(law) == expected
