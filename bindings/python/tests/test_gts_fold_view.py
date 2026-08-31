# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""The GTS fold view's statement layer keeps the graph it was asserted in.

The RDF 1.2 statement layer is keyed per graph: a reifier declaration and a
statement annotation each carry the named graph they were asserted in. One reifier
id may therefore be declared and annotated independently in two graphs, and those
are distinct rows.

Both fold-view row accessors and the relational projection carry that column.
Dropping it would fuse rows that belong to different graphs into one
indistinguishable heap — a Python consumer could no longer tell which graph made
which claim, and could not reconstruct the dataset it was handed.
"""

from __future__ import annotations

import purrdf

EX = "http://example.org/"

# Term dictionary: (kind, value, datatype, lang, direction, reifier); kind 0 = IRI.
_TERMS: list[tuple[int, str | None, int | None, str | None, str | None, int | None]] = [
    (0, f"{EX}a", None, None, None, None),  # 0
    (0, f"{EX}related", None, None, None, None),  # 1
    (0, f"{EX}b", None, None, None, None),  # 2
    (0, f"{EX}r1", None, None, None, None),  # 3
    (0, f"{EX}source", None, None, None, None),  # 4
    (0, f"{EX}ledger", None, None, None, None),  # 5
    (0, f"{EX}elsewhere", None, None, None, None),  # 6
    (0, f"{EX}g1", None, None, None, None),  # 7
    (0, f"{EX}g2", None, None, None, None),  # 8
]


def _two_graph_view() -> purrdf.GtsFoldViewNative:
    """One reifier id (`3`), declared and annotated in both `<g1>` and `<g2>`."""
    return purrdf.GtsFoldViewNative.from_parts(
        _TERMS,
        [(0, 1, 2, 7), (0, 1, 2, 8)],
        [(3, (0, 1, 2), 7), (3, (0, 1, 2), 8)],
        [(3, 4, 5, 7), (3, 4, 6, 8)],
    )


def test_fold_view_rows_carry_the_graph_they_were_asserted_in() -> None:
    view = _two_graph_view()
    assert view.reifier_count() == 2
    assert view.annotation_count() == 2
    assert view.reifiers() == [(3, (0, 1, 2), 7), (3, (0, 1, 2), 8)]
    assert view.annotations() == [(3, 4, 5, 7), (3, 4, 6, 8)]


def test_relational_rows_carry_the_statement_layer_graph_column() -> None:
    rows = _two_graph_view().relational_rows()
    assert rows["reifiers"] == [(3, 0, 1, 2, 7), (3, 0, 1, 2, 8)]
    assert rows["annotations"] == [(3, 4, 5, 7), (3, 4, 6, 8)]


def test_default_graph_statement_rows_keep_a_none_graph_slot() -> None:
    """`None` names the default graph and stays distinguishable from a named one."""
    view = purrdf.GtsFoldViewNative.from_parts(
        _TERMS,
        [(0, 1, 2, None)],
        [(3, (0, 1, 2), None)],
        [(3, 4, 5, None)],
    )
    assert view.reifiers() == [(3, (0, 1, 2), None)]
    assert view.annotations() == [(3, 4, 5, None)]
    assert view.relational_rows()["reifiers"] == [(3, 0, 1, 2, None)]


def test_an_out_of_range_statement_graph_id_is_refused() -> None:
    """The graph slot is validated like every other term id — no silent acceptance."""
    try:
        purrdf.GtsFoldViewNative.from_parts(
            _TERMS,
            [],
            [(3, (0, 1, 2), 99)],
            [],
        )
    except ValueError as err:
        assert "reifiers[0].g" in str(err)
    else:  # pragma: no cover - the call must raise
        raise AssertionError("an out-of-range reifier graph id must be refused")
