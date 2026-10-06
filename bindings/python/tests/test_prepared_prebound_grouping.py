# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: this pins the Python binding's own Store.prepare/run surface end to end;
# the engine rewrite it exercises is covered in Rust by
# crates/sparql-eval/tests/prebound_grouping.rs, and only Python can call this surface.
"""A prepared query's parameter survives ``GROUP BY`` and aggregation.

A prepared parameter is bound before every run, so the engine reads it as a
constant of the evaluation: an aggregate query may project it, or read it in a
``SELECT`` expression, without grouping by it. That is only sound if the value is
carried past the group, which keeps nothing but its keys and its aggregates. These
tests hold the Python surface to it: the bound value appears in every grouped row,
beside the aggregate it was grouped with, never as an unbound cell.

The neighbouring case that already worked — grouping BY the parameter — is pinned
beside them.
"""

from __future__ import annotations

import purrdf

EX = "http://example.org/"
XSD_INTEGER = purrdf.NamedNode("http://www.w3.org/2001/XMLSchema#integer")
TWO = purrdf.Literal("2", datatype=XSD_INTEGER)


def _store() -> purrdf.Store:
    store = purrdf.Store()
    store.load(
        "\n".join(
            [
                f"<{EX}a> <{EX}p> <{EX}o1> .",
                f"<{EX}a> <{EX}p> <{EX}o2> .",
                f"<{EX}b> <{EX}p> <{EX}o3> .",
            ]
        ),
        purrdf.RdfFormat.N_TRIPLES,
    )
    return store


def _rows(query: str) -> list[tuple]:
    prepared = _store().prepare(query, parameters=["this"])
    solutions = prepared.run(this=purrdf.NamedNode(f"{EX}a"))
    width = len(solutions.variables)
    return sorted(
        (tuple(row[i] for i in range(width)) for row in solutions),
        key=repr,
    )


def test_a_projected_parameter_survives_an_implicit_group() -> None:
    rows = _rows(f"SELECT $this (COUNT(*) AS ?c) WHERE {{ $this <{EX}p> ?o }}")
    assert rows == [(purrdf.NamedNode(f"{EX}a"), TWO)]


def test_a_select_expression_reads_the_parameter_above_the_group() -> None:
    rows = _rows(
        f"SELECT (STR($this) AS ?t) (COUNT(*) AS ?c) WHERE {{ $this <{EX}p> ?o }}"
    )
    assert rows == [(purrdf.Literal(f"{EX}a"), TWO)]


def test_a_parameter_survives_a_group_by_another_key() -> None:
    rows = _rows(f"SELECT $this ?o WHERE {{ $this <{EX}p> ?o }} GROUP BY ?o")
    assert rows == [
        (purrdf.NamedNode(f"{EX}a"), purrdf.NamedNode(f"{EX}o1")),
        (purrdf.NamedNode(f"{EX}a"), purrdf.NamedNode(f"{EX}o2")),
    ]


def test_a_group_keyed_by_the_parameter_is_unchanged() -> None:
    rows = _rows(
        f"SELECT $this (COUNT(*) AS ?c) WHERE {{ $this <{EX}p> ?o }} GROUP BY $this"
    )
    assert rows == [(purrdf.NamedNode(f"{EX}a"), TWO)]


def _values(query: str) -> set:
    """The ``?value`` column of ``query`` run with ``$this`` bound to ``ex:a``."""
    prepared = _store().prepare(query, parameters=["this"])
    solutions = prepared.run(this=purrdf.NamedNode(f"{EX}a"))
    column = [variable.value for variable in solutions.variables].index("value")
    return {row[column] for row in solutions if row[column] is not None}


TRUE = purrdf.Literal(
    "true", datatype=purrdf.NamedNode("http://www.w3.org/2001/XMLSchema#boolean")
)


def test_a_parameter_survives_a_sub_select_group() -> None:
    # The same answers `crates/shapes/tests/prebinding_lanes.rs` requires of every lane.
    assert _values(
        f"SELECT ?value WHERE {{ {{ SELECT $this (COUNT(*) AS ?c) "
        f"WHERE {{ $this <{EX}p> ?o }} }} "
        f"BIND((sameTerm($this, <{EX}a>) && ?c = 2) AS ?value) }}"
    ) == {TRUE}
    assert _values(
        f"SELECT (STR($this) AS ?value) WHERE {{ {{ SELECT $this ?o "
        f"WHERE {{ $this <{EX}p> ?o }} GROUP BY ?o }} }}"
    ) == {purrdf.Literal(f"{EX}a")}
    assert _values(
        f"SELECT ?value WHERE {{ {{ SELECT $this (COUNT(*) AS ?value) "
        f"WHERE {{ $this <{EX}p> ?o }} }} FILTER(BOUND($this)) }}"
    ) == {TWO}


def test_having_order_by_and_an_empty_group_read_the_parameter() -> None:
    assert _values(
        f"SELECT (COUNT(*) AS ?value) WHERE {{ $this <{EX}p> ?o }} "
        f"HAVING (sameTerm($this, <{EX}a>))"
    ) == {TWO}
    one = purrdf.Literal("1", datatype=XSD_INTEGER)
    assert _values(
        f"SELECT (COUNT(*) AS ?value) WHERE {{ $this <{EX}p> ?o }} GROUP BY ?o ORDER BY $this"
    ) == {one}
    assert _values(
        f"SELECT ((sameTerm($this, <{EX}a>) && COUNT(?z) = 0) AS ?value) "
        f"WHERE {{ $this <{EX}none> ?z }}"
    ) == {TRUE}


def test_an_assignment_out_of_the_parameter_s_scope_answers_by_join() -> None:
    # Where `$this` is not yet in scope a BIND to it is SPARQL (§18.2.1), and the
    # assigned value joins with the bound one as any two bindings do: `ex:b` against
    # `ex:a` is no row; after the triple pattern the group's COUNT reads 3 rows.
    disjoint = _store().prepare(
        f"SELECT ?value WHERE {{ BIND(<{EX}b> AS $this) $this <{EX}p> ?value }}",
        parameters=["this"],
    )
    # No row at all, not merely no bound ?value.
    assert list(disjoint.run(this=purrdf.NamedNode(f"{EX}a"))) == []
    assert _values(
        f"SELECT (COUNT(*) AS ?value) WHERE {{ ?s <{EX}p> ?o BIND(<{EX}b> AS $this) }}"
    ) == {purrdf.Literal("3", datatype=purrdf.NamedNode("http://www.w3.org/2001/XMLSchema#integer"))}
    # A sub-SELECT assigning it without projecting it binds a variable of its own.
    assert _values(
        f"SELECT ?value WHERE {{ {{ SELECT ?value WHERE {{ ?s <{EX}p> ?value "
        f"BIND(?s AS $this) FILTER(?s = $this) }} }} }}"
    ) == {purrdf.NamedNode(f"{EX}o{n}") for n in (1, 2, 3)}
    assert _values(
        # Reads the parameter, so the binding refuses no unmentioned declaration.
        f"SELECT ?value WHERE {{ BIND(<{EX}b> AS ?fresh) ?fresh <{EX}p> ?value "
        f"FILTER(BOUND($this)) }}"
    ) == {purrdf.NamedNode(f"{EX}o3")}
