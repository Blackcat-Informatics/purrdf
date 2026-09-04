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

# Term dictionary: (kind, value, datatype, lang, direction, reifier, triple);
# kind 0 = IRI, kind 3 = quoted triple. `triple` names a quoted triple's OWN
# (s, p, o) component ids: `rdf:reifies` is not functional, so one reifier id may
# bind several triples and a triple term cannot borrow its identity from one.
_TermRow = tuple[
    int, str | None, int | None, str | None, str | None, int | None, tuple[int, int, int] | None
]
_TERMS: list[_TermRow] = [
    (0, f"{EX}a", None, None, None, None, None),  # 0
    (0, f"{EX}related", None, None, None, None, None),  # 1
    (0, f"{EX}b", None, None, None, None, None),  # 2
    (0, f"{EX}r1", None, None, None, None, None),  # 3
    (0, f"{EX}source", None, None, None, None, None),  # 4
    (0, f"{EX}ledger", None, None, None, None, None),  # 5
    (0, f"{EX}elsewhere", None, None, None, None, None),  # 6
    (0, f"{EX}g1", None, None, None, None, None),  # 7
    (0, f"{EX}g2", None, None, None, None, None),  # 8
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


def test_a_quoted_triple_terms_own_components_survive_the_fold_view() -> None:
    """A quoted triple states its own `(s, p, o)`; it borrows no reifier binding.

    The same reifier id is deliberately bound to TWO different triples here. A
    view that resolved a triple term through that id could only report one of
    them, silently fusing two distinct terms.
    """
    terms: list[_TermRow] = [
        *_TERMS,
        (3, None, None, None, None, None, (0, 1, 2)),  # 9  = <<( a related b )>>
        (3, None, None, None, None, None, (0, 1, 5)),  # 10 = <<( a related ledger )>>
    ]
    view = purrdf.GtsFoldViewNative.from_parts(
        terms,
        [],
        [(3, (0, 1, 2), None), (3, (0, 1, 5), None)],
        [],
    )
    assert view.term_tuple(9) == (3, None, None, None, None, None, (0, 1, 2))
    assert view.term_tuple(10) == (3, None, None, None, None, None, (0, 1, 5))
    assert view.reifier_count() == 2, "both bindings of the one reifier id are kept"

    rows = view.relational_rows()
    assert rows["terms"][9] == (9, 3, None, None, None, None, (0, 1, 2))
    assert rows["terms"][10] == (10, 3, None, None, None, None, (0, 1, 5))
    # Base direction is a parallel column, and a quoted triple term has none.
    assert rows["directions"][9] is None
    assert rows["directions"][10] is None


def test_an_out_of_range_triple_component_is_refused() -> None:
    """A quoted triple's component ids are validated like every other term id."""
    terms: list[_TermRow] = [*_TERMS, (3, None, None, None, None, None, (0, 1, 99))]
    try:
        purrdf.GtsFoldViewNative.from_parts(terms, [], [], [])
    except ValueError as err:
        assert "terms[9].triple.o" in str(err)
    else:  # pragma: no cover - the call must raise
        raise AssertionError("an out-of-range triple component id must be refused")


def test_gts_round_trip_keeps_every_binding_of_one_reifier() -> None:
    """`RdfDataset.to_gts` → `GtsFoldViewNative.from_bytes` loses no binding.

    `rdf:reifies` is not a functional property, so one reifier id may be declared
    about several different triples — here once in `<g1>` and once in `<g2>`. A
    container that kept only the first would silently discard a caller's claim.
    """
    source = (
        f"<{EX}a> <{EX}related> <{EX}b> <{EX}g1> .\n"
        f"<{EX}a> <{EX}related> <{EX}c> <{EX}g2> .\n"
        f"<{EX}r1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> "
        f"<<( <{EX}a> <{EX}related> <{EX}b> )>> <{EX}g1> .\n"
        f"<{EX}r1> <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> "
        f"<<( <{EX}a> <{EX}related> <{EX}c> )>> <{EX}g2> .\n"
    )
    dataset = purrdf.RdfDataset(source, purrdf.RdfFormat.N_QUADS)
    data = dataset.to_gts()

    # Byte determinism: the same dataset written twice is the same container.
    assert data == dataset.to_gts()

    view = purrdf.GtsFoldViewNative.from_bytes(data)
    rows = view.reifiers()
    assert len(rows) == 2, f"both bindings must survive: {rows}"

    # Content AND graph slot, read back through the term dictionary.
    def tid(iri: str) -> int | None:
        return view.tid_of_iri(iri)

    assert sorted(
        (r, s, p, o, g) for (r, (s, p, o), g) in rows
    ) == sorted(
        [
            (
                tid(f"{EX}r1"),
                tid(f"{EX}a"),
                tid(f"{EX}related"),
                tid(f"{EX}b"),
                tid(f"{EX}g1"),
            ),
            (
                tid(f"{EX}r1"),
                tid(f"{EX}a"),
                tid(f"{EX}related"),
                tid(f"{EX}c"),
                tid(f"{EX}g2"),
            ),
        ]
    )
