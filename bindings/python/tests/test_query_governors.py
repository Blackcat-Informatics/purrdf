# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Caller-supplied execution governors on the native query/update surface.

The consumer's admission law is enforced per answer, so it has to be settable from
the language the consumer writes in. These tests hold that surface to three
promises the Rust tier already makes and the binding must not weaken:

* **A tripped governor is an outcome, not an exception.** ``query_governed``
  returns a ``QueryOutcome`` on both paths, carrying the governor that stopped the
  execution, the evidence, and the rows already reached together with the
  certificate that says what they bound. Raising would throw the paid-for rows away
  and blame the engine for a budget the caller set.
* **A ceiling is inclusive.** An answer whose size equals the cap is complete; one
  unit more is a trip.
* **The GIL is released while the engine runs**, so another thread can cancel a
  running query and a Ctrl-C stops one instead of being swallowed until it finishes.
  A ``KeyboardInterrupt`` is the one stop cause that raises, because it is a Python
  exception the interpreter already raised and swallowing it would lose the signal.
"""

from __future__ import annotations

import signal
import threading
import time

import pytest

import purrdf

EX = "http://example.org/"
RDF_REIFIES = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies"

# Every dimension the kernel governs, by its stable kebab-case label. The evidence
# maps are keyed by these and a caller may match on them.
CALLER_SETTABLE = (
    "fuel",
    "answer-rows",
    "intermediate-cells",
    "scratch-bytes",
    "remote-requests",
)

# A ceiling a governed call meters but never enforces (`2**64 - 2`), versus one no
# dimension carries at all (`2**64 - 1`).
METERED_CEILING = 2**64 - 2
UNBOUNDED_CEILING = 2**64 - 1

SELECT_ALL = f"SELECT ?s ?o WHERE {{ ?s <{EX}p> ?o }} ORDER BY ?s"


def _store(triples: int = 5) -> purrdf.Store:
    """A store of `triples` statements over ``example.org``."""
    store = purrdf.Store()
    store.load(
        "\n".join(f"<{EX}s{i:03}> <{EX}p> <{EX}o{i:03}> ." for i in range(triples)),
        purrdf.RdfFormat.N_TRIPLES,
    )
    return store


def _slow_query(subjects: int = 300) -> tuple[purrdf.Store, str]:
    """A store and a query whose evaluation takes long enough to interrupt.

    A self-join under a ``FILTER NOT EXISTS`` re-enters whole-pattern evaluation
    once per candidate pair, so the work is quadratic in `subjects` while the
    materialized bag stays small — the shape that spends real time inside the
    engine without spending real memory.
    """
    query = (
        f"SELECT ?a ?b WHERE {{ ?a <{EX}p> ?x . ?b <{EX}p> ?y . "
        f"FILTER NOT EXISTS {{ ?a <{EX}q> ?b }} }}"
    )
    return _store(subjects), query


def _rows(solutions: object) -> list[tuple[str, str]]:
    """A SELECT result as `(s, o)` string pairs."""
    return [(str(row[0].value), str(row[1].value)) for row in solutions]  # type: ignore[union-attr]


# ── an ungoverned call is unchanged ───────────────────────────────────────────────


def test_ungoverned_query_is_unchanged() -> None:
    """`Store.query` still returns the results directly, with no outcome wrapper."""
    store = _store()
    solutions = store.query(SELECT_ALL)

    assert isinstance(solutions, purrdf.QuerySolutions)
    assert len(solutions) == 5


def test_governed_query_agrees_with_the_ungoverned_one() -> None:
    """A governor never changes an answer, only an outcome."""
    store = _store()
    governed = store.query_governed(SELECT_ALL)

    assert governed.is_complete
    assert _rows(governed.result) == _rows(store.query(SELECT_ALL))


# ── every governor argument reaches evaluation ────────────────────────────────────


@pytest.mark.parametrize(
    ("keyword", "dimension"),
    [
        ("fuel", "fuel"),
        ("max_answers", "answer-rows"),
        ("max_intermediate_cells", "intermediate-cells"),
        ("max_scratch_bytes", "scratch-bytes"),
        ("max_remote_requests", "remote-requests"),
    ],
)
def test_every_governor_keyword_reaches_evaluation(
    keyword: str, dimension: str
) -> None:
    """Each ceiling arrives at the engine and is echoed by the evidence in force."""
    store = _store()
    outcome = store.query_governed(SELECT_ALL, **{keyword: 9_999})

    assert outcome.evidence.limits[dimension] == 9_999
    assert outcome.evidence.limit_for(dimension) == 9_999


def test_a_governed_call_meters_every_dimension_it_was_not_given() -> None:
    """An unset ceiling is metered, not absent: the evidence has no holes in it."""
    outcome = _store().query_governed(SELECT_ALL)

    assert outcome.is_complete
    for dimension in CALLER_SETTABLE:
        assert outcome.evidence.limits[dimension] == METERED_CEILING
    # The recursion guard is a build ceiling a caller cannot relax, and the paging
    # tier's dimensions are configured elsewhere entirely.
    assert outcome.evidence.limits["udf-depth"] < METERED_CEILING
    assert outcome.evidence.limits["pages"] == UNBOUNDED_CEILING
    assert outcome.evidence.consumed["fuel"] > 0
    assert outcome.evidence.consumed["answer-rows"] == 5
    assert outcome.evidence.consumed_in("answer-rows") == 5


def test_completed_evidence_sizes_the_next_budget() -> None:
    """The measured cost is directly usable as a ceiling — the loop metering exists for."""
    store = _store()
    measured = store.query_governed(SELECT_ALL).evidence.consumed["fuel"]

    assert store.query_governed(SELECT_ALL, fuel=measured).is_complete
    assert not store.query_governed(SELECT_ALL, fuel=measured - 1).is_complete


def test_evidence_rejects_an_unknown_dimension() -> None:
    """A misspelt dimension is a hard error, never a silent zero."""
    evidence = _store().query_governed(SELECT_ALL).evidence

    with pytest.raises(ValueError, match="unknown resource dimension"):
        evidence.consumed_in("fule")
    with pytest.raises(ValueError, match="unknown resource dimension"):
        evidence.limit_for("fule")


def test_governor_arguments_are_keyword_only() -> None:
    """No ceiling can be set by position, where a reordering would silently move it."""
    with pytest.raises(TypeError):
        _store().query_governed(SELECT_ALL, 10)  # type: ignore[misc]


# ── the answer cap, whose boundary is inclusive ───────────────────────────────────


def test_answer_cap_equal_to_the_answer_size_completes() -> None:
    """Consumption equal to the ceiling is admitted: cap == size is a complete answer."""
    outcome = _store().query_governed(SELECT_ALL, max_answers=5)

    assert outcome.is_complete
    assert outcome.tripped is None
    assert outcome.partial is None
    assert len(outcome.result) == 5


def test_answer_cap_one_below_the_answer_size_exhausts() -> None:
    """One unit more than the ceiling trips, and the trip names the answer cap."""
    outcome = _store().query_governed(SELECT_ALL, max_answers=4)

    assert not outcome.is_complete
    assert outcome.result is None
    assert outcome.tripped is not None
    assert outcome.tripped.kind == "budget"
    assert outcome.tripped.label == "answer-cap-exhausted"
    assert outcome.tripped.dimension == "answer-rows"
    assert outcome.tripped.limit == 4
    assert outcome.tripped.consumed == 5
    assert outcome.tripped.estimate is None
    assert outcome.tripped.cause is None


def test_partial_answers_are_a_certified_prefix() -> None:
    """A truncated SELECT hands back its rows plus what they are certified to bound."""
    store = _store()
    complete = _rows(store.query_governed(SELECT_ALL).result)
    outcome = store.query_governed(SELECT_ALL, max_answers=4)

    partial = outcome.partial
    assert partial is not None
    assert partial.certainty == "certain"
    assert partial.is_certain
    assert partial.barrier is None
    assert partial.is_positional_prefix
    # The resumption property, verified rather than asserted: the rows in hand are
    # the true answer's first rows, in order.
    assert _rows(partial.result) == complete[:4]


# ── a trip is an outcome, never an exception ──────────────────────────────────────


@pytest.mark.parametrize(
    "governors",
    [
        {"fuel": 0},
        {"max_answers": 0},
        {"max_intermediate_cells": 0},
        {"deadline_ms": 0},
    ],
    ids=["fuel", "answer-rows", "intermediate-cells", "deadline"],
)
def test_a_tripped_governor_is_returned_not_raised(
    governors: dict[str, int],
) -> None:
    """Every ceiling that can trip returns an outcome object rather than raising."""
    outcome = _store().query_governed(SELECT_ALL, **governors)

    assert isinstance(outcome, purrdf.QueryOutcome)
    assert not outcome.is_complete
    assert outcome.tripped is not None
    assert outcome.evidence.tripped is not None
    assert outcome.evidence.tripped.label == outcome.tripped.label
    assert not outcome.evidence.is_complete


def test_fuel_ceiling_reports_a_budget_governor() -> None:
    """A spent fuel budget reports the dimension, the ceiling, and what was charged."""
    outcome = _store().query_governed(SELECT_ALL, fuel=0)

    assert outcome.tripped is not None
    assert outcome.tripped.label == "fuel-exhausted"
    assert outcome.tripped.dimension == "fuel"
    assert outcome.tripped.limit == 0
    assert outcome.tripped.consumed >= 1
    assert "fuel budget exceeded" in str(outcome.tripped)


def test_deadline_reports_a_stop_cause() -> None:
    """A zero-millisecond deadline expires on the first poll, as a stop signal."""
    outcome = _store().query_governed(SELECT_ALL, deadline_ms=0)

    assert outcome.tripped is not None
    assert outcome.tripped.kind == "stopped"
    assert outcome.tripped.cause == "deadline-exceeded"
    assert outcome.tripped.dimension is None
    assert outcome.tripped.limit is None
    assert outcome.tripped.consumed is None


def test_intermediate_cell_ceiling_is_refused_at_admission() -> None:
    """A ceiling the planner's estimate already exceeds refuses before anything runs.

    The distinct kind is the point: a refusal reports an ESTIMATE, not a
    measurement, because nothing ran to measure.
    """
    outcome = _store().query_governed(SELECT_ALL, max_intermediate_cells=0)

    assert outcome.tripped is not None
    assert outcome.tripped.kind == "refused"
    assert outcome.tripped.label == "cardinality-admission-refused"
    assert outcome.tripped.dimension == "intermediate-cells"
    assert outcome.tripped.limit == 0
    assert outcome.tripped.estimate is not None
    assert outcome.tripped.estimate > 0
    assert outcome.tripped.consumed is None


def test_scratch_byte_ceiling_reports_a_budget_governor() -> None:
    """A value-constructing query is bounded by the arena it mints into."""
    store = _store()
    query = (
        f"SELECT (CONCAT(STR(?s), '{'x' * 64}') AS ?c) WHERE {{ ?s <{EX}p> ?o }}"
    )

    assert store.query_governed(query).is_complete
    outcome = store.query_governed(query, max_scratch_bytes=0)
    assert outcome.tripped is not None
    assert outcome.tripped.dimension == "scratch-bytes"


def test_remote_request_ceiling_reaches_a_query_with_no_remote_call() -> None:
    """A federated ceiling is in force even when the query issues no request."""
    outcome = _store().query_governed(SELECT_ALL, max_remote_requests=0)

    assert outcome.is_complete
    assert outcome.evidence.limits["remote-requests"] == 0
    assert outcome.evidence.consumed["remote-requests"] == 0


# ── the other query forms ─────────────────────────────────────────────────────────


def test_construct_answer_cap_counts_output_statements() -> None:
    """A graph form's cap denominates statements, not solution rows."""
    store = _store()
    query = f"CONSTRUCT {{ ?s <{EX}reaches> ?o }} WHERE {{ ?s <{EX}p> ?o }}"

    complete = store.query_governed(query, max_answers=5)
    assert complete.is_complete
    assert len(complete.result) == 5

    tripped = store.query_governed(query, max_answers=4)
    assert not tripped.is_complete
    assert tripped.tripped is not None
    assert tripped.tripped.label == "answer-cap-exhausted"
    assert tripped.partial is not None
    assert len(tripped.partial.result) == 4


def test_ask_form_has_no_answer_sequence_to_cap() -> None:
    """A boolean has no sequence, so an answer cap of zero does not bound it."""
    outcome = _store().query_governed(f"ASK {{ ?s <{EX}p> ?o }}", max_answers=0)

    assert outcome.is_complete
    assert bool(outcome.result)


def test_governed_query_over_an_rdf12_reifier() -> None:
    """RDF 1.2 triple terms and reifiers are governed like any other statement."""
    store = purrdf.Store()
    store.load(
        f"_:r <{RDF_REIFIES}> <<( <{EX}a> <{EX}p> <{EX}b> )>> .\n"
        f"_:r <{EX}source> <{EX}doc> .\n"
        f"<{EX}a> <{EX}p> <{EX}b> .\n",
        purrdf.RdfFormat.N_TRIPLES,
    )
    query = (
        f"SELECT ?r ?src WHERE {{ ?r <{RDF_REIFIES}> <<( <{EX}a> <{EX}p> <{EX}b> )>> . "
        f"?r <{EX}source> ?src }}"
    )

    complete = store.query_governed(query, max_answers=1)
    assert complete.is_complete
    assert len(complete.result) == 1

    tripped = store.query_governed(query, max_answers=0)
    assert not tripped.is_complete
    assert tripped.tripped is not None
    assert tripped.tripped.label == "answer-cap-exhausted"

    # The triple term survives a governed CONSTRUCT of the annotation shape.
    constructed = store.query_governed(
        f"CONSTRUCT {{ ?r <{EX}claims> ?tt }} "
        f"WHERE {{ ?r <{RDF_REIFIES}> ?tt }}",
        fuel=100_000,
    )
    assert constructed.is_complete
    assert len(constructed.result) == 1


# ── cancellation, from Python, while the engine runs ──────────────────────────────


def test_cancellation_token_reaches_the_evaluator() -> None:
    """A token cancelled before the call stops it on the first poll."""
    token = purrdf.CancellationToken()
    assert not token.cancelled
    token.cancel()
    assert token.cancelled

    outcome = _store().query_governed(SELECT_ALL, cancel=token)

    assert not outcome.is_complete
    assert outcome.tripped is not None
    assert outcome.tripped.kind == "stopped"
    assert outcome.tripped.cause == "cancelled"


def test_cancellation_is_idempotent_and_never_reversible() -> None:
    """Cancelling twice is one cancellation, and nothing clears the bit."""
    token = purrdf.CancellationToken()
    token.cancel()
    token.cancel()

    assert token.cancelled
    assert repr(token) == "<CancellationToken cancelled=True>"


def test_cancellation_from_another_thread_stops_a_running_query() -> None:
    """A second thread cancels a query already inside the engine.

    This is also the test that the GIL is genuinely released: a thread that could
    not run while the engine ran could never flip the token, and the query would
    return complete.
    """
    store, query = _slow_query()
    token = purrdf.CancellationToken()

    def cancel_soon() -> None:
        time.sleep(0.05)
        token.cancel()

    canceller = threading.Thread(target=cancel_soon)
    canceller.start()
    try:
        outcome = store.query_governed(query, cancel=token)
    finally:
        canceller.join()

    assert not outcome.is_complete
    assert outcome.tripped is not None
    assert outcome.tripped.cause == "cancelled"
    # A cancellation is still a governed outcome: the rows already reached come
    # back with the certificate that says what they bound.
    assert outcome.partial is not None
    assert outcome.evidence.consumed["fuel"] > 0


def test_keyboard_interrupt_stops_a_running_query() -> None:
    """Ctrl-C interrupts the engine instead of being swallowed until it finishes.

    The one stop cause that RAISES: the interpreter has already turned the pending
    signal into an exception, and dropping it would make the Ctrl-C disappear.
    """
    store, query = _slow_query()

    def interrupt_soon() -> None:
        time.sleep(0.05)
        signal.raise_signal(signal.SIGINT)

    interrupter = threading.Thread(target=interrupt_soon)
    interrupter.start()
    try:
        with pytest.raises(KeyboardInterrupt):
            store.query_governed(query)
    finally:
        interrupter.join()

    # The signal was consumed by the call that raised, so the next query is clean.
    assert store.query_governed(SELECT_ALL).is_complete


# ── governed UPDATE ───────────────────────────────────────────────────────────────


def test_governed_update_applies_and_reports_evidence() -> None:
    """An applied request reports the cost of applying it."""
    store = _store()
    outcome = store.update_governed(f"INSERT DATA {{ <{EX}a> <{EX}p> <{EX}b> }}")

    assert isinstance(outcome, purrdf.UpdateOutcome)
    assert outcome.is_applied
    assert outcome.tripped is None
    assert outcome.evidence.is_complete
    assert len(store) == 6


def test_a_tripped_update_applies_nothing() -> None:
    """A stopped request leaves the store exactly as it found it — no partial write."""
    store = _store()
    before = len(store)
    outcome = store.update_governed(
        f"INSERT DATA {{ <{EX}a> <{EX}p> <{EX}b> }}", fuel=0
    )

    assert not outcome.is_applied
    assert outcome.tripped is not None
    assert outcome.tripped.label == "fuel-exhausted"
    assert len(store) == before
    assert not list(store.query(f"SELECT ?o WHERE {{ <{EX}a> <{EX}p> ?o }}"))


def test_a_cancelled_update_applies_nothing() -> None:
    """The all-or-nothing guarantee holds for a stop signal, not only for a ceiling."""
    store = _store()
    token = purrdf.CancellationToken()
    token.cancel()

    outcome = store.update_governed(
        f"INSERT DATA {{ <{EX}a> <{EX}p> <{EX}b> }}", cancel=token
    )

    assert not outcome.is_applied
    assert outcome.tripped is not None
    assert outcome.tripped.cause == "cancelled"
    assert len(store) == 5


def test_update_governed_has_no_answer_cap() -> None:
    """`max_answers` is refused on the update path: an UPDATE has no answer sequence."""
    with pytest.raises(TypeError):
        _store().update_governed(  # type: ignore[call-arg]
            f"INSERT DATA {{ <{EX}a> <{EX}p> <{EX}b> }}", max_answers=1
        )


# ── the same surface on MutableDataset ────────────────────────────────────────────


def test_mutable_dataset_governed_query_and_update() -> None:
    """The COW dataset carries the identical governed surface, not a narrower one."""
    dataset = purrdf.MutableDataset()
    dataset.load(
        "\n".join(f"<{EX}s{i:03}> <{EX}p> <{EX}o{i:03}> ." for i in range(5)),
        purrdf.RdfFormat.N_TRIPLES,
    )

    assert dataset.query_governed(SELECT_ALL, max_answers=5).is_complete

    tripped = dataset.query_governed(SELECT_ALL, max_answers=4)
    assert not tripped.is_complete
    assert tripped.tripped is not None
    assert tripped.tripped.label == "answer-cap-exhausted"
    assert tripped.partial is not None
    assert len(tripped.partial.result) == 4

    applied = dataset.update_governed(f"INSERT DATA {{ <{EX}a> <{EX}p> <{EX}b> }}")
    assert applied.is_applied
    assert len(dataset) == 6

    stopped = dataset.update_governed(
        f"INSERT DATA {{ <{EX}c> <{EX}p> <{EX}d> }}", fuel=0
    )
    assert not stopped.is_applied
    assert len(dataset) == 6
