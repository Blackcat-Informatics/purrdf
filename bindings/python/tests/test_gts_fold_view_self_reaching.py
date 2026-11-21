# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""A caller-built term table must not kill the interpreter.

``GtsFoldViewNative.from_parts`` accepts a term dictionary straight from Python. A
triple term that reaches itself (``terms[0].triple == (0, 0, 0)``) states no triple
(GTS-SPEC 7.3): it renders as the blank node of an unbound triple term, so no
accessor walks its components forever. That walk lives in Rust, where a stack overflow
ABORTS the process. A literal whose datatype chain loops is not a triple term; it
raises ``ValueError``.
"""

from __future__ import annotations

import pytest

import purrdf

EX = "http://example.org/"

# (kind, value, datatype, lang, direction, reifier, triple);
# kind 0 = IRI, 1 = literal, 2 = blank node, 3 = quoted triple.
_TermRow = tuple[
    int, str | None, int | None, str | None, str | None, int | None, tuple[int, int, int] | None
]


def _iri(value: str) -> _TermRow:
    return (0, value, None, None, None, None, None)


def _triple(spo: tuple[int, int, int]) -> _TermRow:
    """A self-describing quoted triple (wire ``tt``) naming its own components."""
    return (3, None, None, None, None, None, spo)


# (terms, reifiers, ids that state no triple)
SELF_REACHING = [
    pytest.param([_triple((0, 0, 0))], [], [0], id="a-triple-term-whose-components-are-itself"),
    pytest.param(
        [_iri(f"{EX}p"), _triple((2, 0, 2)), _triple((1, 0, 1))],
        [],
        [1, 2],
        id="two-triple-terms-naming-each-other",
    ),
    pytest.param(
        # The original indirect spelling: term 1 names reifier 2, whose statement-layer
        # binding has term 1 as its own subject.
        [_iri(f"{EX}p"), (3, None, None, None, None, 2, None), _iri(f"{EX}r")],
        [(2, (1, 0, 1), None)],
        [1],
        id="a-reifier-binding-naming-its-own-triple-term",
    ),
    pytest.param(
        # A self-bound triple term names its own id as its reifier, so the binding
        # keyed by that id supplies its components - and here that binding names it.
        [_iri(f"{EX}p"), (3, None, None, None, None, 1, None)],
        [(1, (1, 0, 1), None)],
        [1],
        id="a-self-bound-term-whose-binding-names-it",
    ),
]


@pytest.mark.parametrize(("terms", "reifiers", "unbound"), SELF_REACHING)
def test_a_self_reaching_term_renders_as_an_unbound_triple_and_keeps_its_row(
    terms: list[_TermRow],
    reifiers: list[tuple[int, tuple[int, int, int], int | None]],
    unbound: list[int],
) -> None:
    view = purrdf.GtsFoldViewNative.from_parts(terms, [], reifiers, [])
    for tid in unbound:
        assert view.nq_token(tid) == f"_:unbound_triple_{tid}"
    assert len(view.reifiers()) == len(reifiers)


def test_a_literal_whose_datatype_is_itself_is_a_catchable_exception() -> None:
    """The refusal is a Python exception, and the process keeps working after it."""
    with pytest.raises(ValueError, match="resolves through itself"):
        purrdf.GtsFoldViewNative.from_parts([(1, "x", 0, None, None, None, None)], [], [], [])
    view = purrdf.GtsFoldViewNative.from_parts([_iri(f"{EX}s")], [], [], [])
    assert view.term_count() == 1


def test_a_dangling_component_id_is_still_a_range_error() -> None:
    with pytest.raises(ValueError, match="out of range"):
        purrdf.GtsFoldViewNative.from_parts([_triple((9, 9, 9))], [], [], [])


def test_legitimately_nested_triple_terms_are_still_accepted_and_render() -> None:
    terms = [
        _iri(f"{EX}s"),  # 0
        _iri(f"{EX}p"),  # 1
        _iri(f"{EX}o"),  # 2
        _triple((0, 1, 2)),  # 3 - <<( s p o )>>
        _iri(f"{EX}says"),  # 4
        _triple((0, 4, 3)),  # 5 - <<( s says <<( s p o )>> )>>
    ]
    view = purrdf.GtsFoldViewNative.from_parts(terms, [(0, 1, 5, None)], [], [])
    assert view.nq_token(5) == (
        f"<<( <{EX}s> <{EX}says> <<( <{EX}s> <{EX}p> <{EX}o> )>> )>>"
    )
