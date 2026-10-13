# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: Python iterator dispatch, PyO3 integer extraction, reference lifetime and Python tuple accessors require the actual Python runtime.
"""Host protocol checks; fixed queries only obtain real native row wrappers."""

import operator

import pytest

import purrdf
from purrdf.compat import rdflib as compat
from purrdf.compat.rdflib.query import ResultRow


def _row(query="SELECT ?a ?b ?c WHERE { VALUES (?a ?b ?c) { (1 UNDEF 3) } }"):
    result = purrdf.Store().query(query)
    rows = list(result)
    assert len(rows) == 1
    return rows[0]


def test_native_iteration_is_repeatable_independent_and_retains_row():
    row = _row()
    assert len(row) == 3
    expected = (row[0], row[1], row[2])
    assert expected[1] is None
    assert tuple(row) == expected
    assert list(row) == list(expected)
    a, b, c = row
    assert (a, b, c) == expected
    first, second = iter(row), iter(row)
    assert first is not second
    assert iter(first) is first
    assert operator.length_hint(first) == 3
    assert next(first) == expected[0]
    assert operator.length_hint(first) == 2
    assert next(second) == expected[0]
    del row
    assert next(first) is None
    assert next(first) == expected[2]
    assert operator.length_hint(first) == 0
    with pytest.raises(StopIteration):
        next(first)
    with pytest.raises(StopIteration):
        next(first)
    assert list(second) == list(expected[1:])


def test_native_named_signed_integer_and_type_neighbors():
    row = _row()
    assert row["a"] == row[purrdf.Variable("a")] == row[0] == row[-3]
    assert row["b"] is row[1] is row[-2] is None
    assert row["c"] == row[2] == row[-1]
    assert row[False] == row[0]
    assert row[True] is None

    class Position(int):
        pass

    assert row[Position(-1)] == row[2]
    for missing in ("missing", purrdf.Variable("missing")):
        with pytest.raises(KeyError):
            row[missing]
    for unsupported in (object(), [], slice(0, 1)):
        with pytest.raises(TypeError):
            row[unsupported]


@pytest.mark.parametrize("position", [3, -4, 2**100, -(2**100), -(2**63), 2**63])
def test_native_outside_positions_raise_index_error(position):
    row = _row()
    with pytest.raises(IndexError):
        row[position]
    assert tuple(row) == (row[0], row[1], row[2])


def test_zero_width_native_and_compat_tuple_protocol():
    row = _row("SELECT * WHERE {}")
    assert len(row) == 0
    assert list(row) == []
    assert tuple(row) == ()
    assert operator.length_hint(iter(row)) == 0
    for position in (0, -1):
        with pytest.raises(IndexError):
            row[position]
    empty = ResultRow((), ())
    assert len(empty) == 0 and tuple(empty) == () and empty.asdict() == {}
    # Contextual compatibility presentation deliberately suppresses empty mappings.
    assert list(compat.Graph().query("SELECT * WHERE {}")) == []


def test_select_star_projects_actual_public_column_order():
    query = "SELECT * WHERE { VALUES (?a ?b ?c) { (1 UNDEF 3) } }"
    native = purrdf.Store().query(query)
    variables = native.variables
    row = next(native)
    assert len(row) == len(variables) == 3
    assert tuple(row) == tuple(row[variable] for variable in variables)
    result = compat.Graph().query(query)
    rows = list(result)
    assert len(rows) == 1
    assert len(rows[0]) == len(result.vars) == 3
    assert tuple(rows[0]) == tuple(rows[0][variable] for variable in result.vars)


def test_native_duplicate_projection_retains_every_position():
    result = purrdf.Store().query_rdflib("SELECT ?v ?v WHERE { VALUES ?v { 1 } }")
    row = next(result)
    assert len(result.variables) == len(row) == 2
    assert tuple(row) == (row[0], row[1])
    assert row["v"] == row[purrdf.Variable("v")] == row[0]


@pytest.mark.parametrize("last_bound", [False, True])
def test_compat_duplicate_positions_keep_last_label_access(last_bound):
    first = compat.URIRef("http://example.org/first")
    last = compat.URIRef("http://example.org/last") if last_bound else None
    row = ResultRow((first, last), ("v", "v"))
    assert len(row) == 2 and tuple(row) == (first, last)
    assert row[0] == row[-2] == first
    assert row[1] is row[-1] is last
    assert row[:] == (first, last)
    assert row["v"] is row[compat.Variable("v")] is row.v is last
    assert row.get("v", first) is last
    assert row.get("missing", first) == first
    assert row.asdict() == ({"v": last} if last_bound else {})
    labels = row.labels
    assert labels == {"v": 1}
    labels["v"] = 0
    assert row.labels == {"v": 1}
