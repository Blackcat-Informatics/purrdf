# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: Python object mutation, lifetime and detached thread calls require the actual Python binding.
"""Retained retrieval uses the original Python snapshot on concurrent calls."""

from concurrent.futures import ThreadPoolExecutor
from threading import Barrier

import pytest

from purrdf.retrieval import RetrievalSession
from purrdf import retrieval


EX = "https://example.org/"
NOTE = f"{EX}note"
STRATUM = f"{EX}stratum/note"
PRODUCER = f"{EX}pf/note"
DATA = f'<{EX}a> <{NOTE}> "red fox" .\n<{EX}b> <{NOTE}> "blue hare" .'


def _without_instance(result):
    return {
        key: _without_instance(value) if key == "plan" else value
        for key, value in result.items()
        if key not in ("plan_id", "canonical_bytes")
    }


def test_retrieval_session_snapshots_producers_and_isolates_requests():
    spec = {"lexicon": "empty"}
    producers = {PRODUCER: (STRATUM, NOTE, "any", spec)}
    with pytest.raises(ValueError):
        RetrievalSession("not a Turtle document", text_producers=producers)
    session = RetrievalSession(DATA, text_producers=producers)
    other = RetrievalSession(DATA, text_producers=producers)

    request = [("lexical", "red fox", None, NOTE)]
    planning = {"statistics": {"source": "host", "revision": "r1"}, "top_k": 1}
    law = {
        "weights": {STRATUM: 10**12},
        "k": 60,
        "decay": "reciprocal_rank",
        "scale_digits": 12,
    }
    plan = session.plan(request, **planning)
    compiled = session.compile(request, **planning, **law)
    assert plan["plan_id"] == compiled["plan_id"]
    assert plan == compiled["plan"]
    assert other.plan(request, **planning)["plan_id"] != plan["plan_id"]

    baseline = retrieval.plan(DATA, request, text_producers=producers, **planning)
    assert baseline["plan_id"] != plan["plan_id"]
    assert _without_instance(baseline) == _without_instance(plan)
    baseline = retrieval.compile(
        DATA, request, text_producers=producers, **planning, **law
    )
    assert _without_instance(baseline) == _without_instance(compiled)

    queries = ("red fox", "blue hare")
    expected = [
        retrieval.search(
            DATA,
            [("lexical", query, None, NOTE)],
            text_producers=producers,
            **planning,
            **law,
        )
        for query in queries
    ]
    assert expected[0]["rows"] != expected[1]["rows"]

    # The registry must outlive both mutation and removal of the original
    # producer declarations. Independent per-call args cross the interpreter.
    spec["lexicon"] = "invalid"
    producers.clear()
    barrier = Barrier(2)

    def run(query):
        statistics = {"source": "host", "revision": "r1"}
        barrier.wait(timeout=15)
        return session.search(
            [("lexical", query, None, NOTE)],
            statistics=statistics,
            top_k=1,
            **law,
        )

    with ThreadPoolExecutor(max_workers=2) as pool:
        actual = list(pool.map(run, queries))

    assert actual[0]["plan_id"] == plan["plan_id"]
    for fresh, retained in zip(expected, actual, strict=True):
        assert fresh["plan_id"] != retained["plan_id"]
        assert _without_instance(fresh) == _without_instance(retained)
    assert session.plan(request, **planning) == plan
