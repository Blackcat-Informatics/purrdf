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
