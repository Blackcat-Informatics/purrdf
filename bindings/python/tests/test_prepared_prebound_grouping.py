# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: verifies that Python dictionary bindings reach prepared
# execution and that the grouped result returns Python term objects.
"""Check parameter and grouped row transport through the Python binding."""

from __future__ import annotations

import purrdf

EX = "http://example.org/"


def test_a_projected_parameter_survives_an_implicit_group() -> None:
    bound = purrdf.NamedNode(f"{EX}a")
    count = purrdf.Literal(
        "2", datatype=purrdf.NamedNode("http://www.w3.org/2001/XMLSchema#integer")
    )
    store = purrdf.Store()
    store.load(
        f"<{EX}a> <{EX}p> <{EX}o1> .\n"
        f"<{EX}a> <{EX}p> <{EX}o2> .\n"
        f"<{EX}b> <{EX}p> <{EX}o3> .\n",
        purrdf.RdfFormat.N_TRIPLES,
    )
    prepared = store.prepare(
        f"SELECT $this (COUNT(*) AS ?c) WHERE {{ $this <{EX}p> ?o }}",
        parameters=["this"],
    )
    solutions = prepared.run({"this": bound})
    assert [variable.value for variable in solutions.variables] == ["this", "c"]
    (row,) = list(solutions)
    assert (row[0], row[1]) == (bound, count)
