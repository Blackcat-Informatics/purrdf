# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""A caller-built term table must raise, not kill the interpreter.

``GtsFoldViewNative.from_parts`` accepts a term dictionary straight from Python. Its
row validation checks that every id is in RANGE — nothing more. The shape those ids
describe was never checked, so a quoted-triple term naming itself
(``terms[0].triple == (0, 0, 0)``) was accepted, and the first accessor that rendered
it walked its components forever.

That walk lives in Rust, where a stack overflow ABORTS the process. It is not a
Python exception and it is not a Rust panic: the interpreter dies with SIGSEGV, taking
the caller's whole program with it. Nothing on the Python side can catch it, so the
only fix is to refuse the table at construction — which is what these tests pin.

Each case is one term dictionary whose ids are all in range, so a range-only check
waves it straight through; ``from_parts`` must raise ``ValueError`` instead.
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


SELF_REACHING = [
    pytest.param(
        [_triple((0, 0, 0))],
        [],
        id="a-triple-term-whose-components-are-itself",
    ),
    pytest.param(
        [_iri(f"{EX}p"), _triple((2, 0, 2)), _triple((1, 0, 1))],
        [],
        id="two-triple-terms-naming-each-other",
    ),
    pytest.param(
        # A literal whose datatype term is the literal itself; the renderer follows
        # the datatype edge exactly as it follows a quoted triple's components.
        [(1, "x", 0, None, None, None, None)],
        [],
        id="a-literal-whose-datatype-is-itself",
    ),
    pytest.param(
        # The original indirect spelling: term 1 names reifier 2, whose statement-layer
        # binding has term 1 as its own subject.
        [_iri(f"{EX}p"), (3, None, None, None, None, 2, None), _iri(f"{EX}r")],
        [(2, (1, 0, 1), None)],
        id="a-reifier-binding-naming-its-own-triple-term",
    ),
    pytest.param(
        # A self-bound triple term may leave its reifier implicit, in which case the
        # binding is keyed by the term's OWN id — and here that binding names the term.
        [_iri(f"{EX}p"), (3, None, None, None, None, None, None)],
        [(1, (1, 0, 1), None)],
        id="an-implicit-self-binding-naming-its-own-term",
    ),
]


@pytest.mark.parametrize(("terms", "reifiers"), SELF_REACHING)
def test_from_parts_refuses_a_self_reaching_term_table(
    terms: list[_TermRow],
    reifiers: list[tuple[int, tuple[int, int, int], int | None]],
) -> None:
    with pytest.raises(ValueError, match="resolves through itself"):
        purrdf.GtsFoldViewNative.from_parts(terms, [], reifiers, [])


def test_the_refusal_is_a_catchable_exception_not_a_process_abort() -> None:
    """The point of the fix, stated as the assertion.

    Before it, this call returned an object and the next ``nq_token`` call killed the
    interpreter. The test therefore asserts that the failure is a Python exception the
    caller can handle and that the process is still alive afterwards to keep working.
    """
    try:
        purrdf.GtsFoldViewNative.from_parts([_triple((0, 0, 0))], [], [], [])
    except ValueError as err:
        assert "gts-self-reaching-term" in str(err) or "resolves through itself" in str(err)
    else:  # pragma: no cover - only reachable if the refusal regresses
        pytest.fail("from_parts accepted a self-reaching term table")

    # Still running, and still usable.
    view = purrdf.GtsFoldViewNative.from_parts([_iri(f"{EX}s")], [], [], [])
    assert view.term_count() == 1


def test_a_dangling_component_id_is_still_a_range_error() -> None:
    """The termination check must not swallow the range check that precedes it."""
    with pytest.raises(ValueError, match="out of range"):
        purrdf.GtsFoldViewNative.from_parts([_triple((9, 9, 9))], [], [], [])


def test_legitimately_nested_triple_terms_are_still_accepted_and_render() -> None:
    """The refusal is not a ban on nesting: an acyclic table still works end to end."""
    terms = [
        _iri(f"{EX}s"),  # 0
        _iri(f"{EX}p"),  # 1
        _iri(f"{EX}o"),  # 2
        _triple((0, 1, 2)),  # 3 — <<( s p o )>>
        _iri(f"{EX}says"),  # 4
        _triple((0, 4, 3)),  # 5 — <<( s says <<( s p o )>> )>>
    ]
    view = purrdf.GtsFoldViewNative.from_parts(terms, [(0, 1, 5, None)], [], [])
    assert view.nq_token(5) == (
        f"<<( <{EX}s> <{EX}says> <<( <{EX}s> <{EX}p> <{EX}o> )>> )>>"
    )
