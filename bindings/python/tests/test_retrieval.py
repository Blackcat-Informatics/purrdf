# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""The ranked-retrieval composition layer on the native surface.

``purrdf.retrieval`` is the layer that turns ONE request into ONE answer across
several ranked producers: it plans the request against what each producer
declares it accepts, admits and compiles the plan into per-stratum SPARQL, runs
those units, and fuses the ranked rows under an exact, content-addressed law.
These tests hold the Python surface to what makes that usable and honest from a
host that writes Python:

* **Producers are DATA, not callables.** ``text_producers`` maps a producer IRI
  to ``(stratum, predicate, graph)``; the engine builds the index and registers
  the relation itself. Nothing it invokes can re-enter the interpreter, which is
  why the whole ladder still runs with the GIL released.
* **Nothing is defaulted.** There is no default producer, stratum, weight,
  smoothing constant or row bound, and ``statistics`` must name its own source
  and revision, because PurRDF mints no vocabulary and invents no measurement.
* **Exactness survives the boundary.** A weight crosses as an ``int`` of raw
  fixed-point units (``retrieval.SCALE`` is one whole unit) and a score comes
  back as its exact decimal ``str``. A float on either leg would be an
  approximation of a number the fusion law computed exactly.
* **Nothing is lost between the stages.** Every fused row carries its
  per-stratum provenance, every applicable producer carries its own terminal
  status, and every request term that reached no producer is named with a typed
  reason rather than quietly producing no rows.
* **A misconfiguration raises where it is supplied**, carrying the engine's own
  diagnostic text — and a refusal never takes down the neighbouring request that
  is valid.
"""

from __future__ import annotations

from typing import Any

import pytest

from purrdf import retrieval

EX = "https://example.org/"

NOTE = f"{EX}note"
TITLE = f"{EX}title"
NOTE_PRODUCER = f"{EX}pf/note-search"
TITLE_PRODUCER = f"{EX}pf/title-search"
NOTE_STRATUM = f"{EX}stratum/note"
TITLE_STRATUM = f"{EX}stratum/title"

DATA = f"""
<{EX}a> <{NOTE}> "the quick brown fox" ;
        <{TITLE}> "quick notes" .
<{EX}b> <{NOTE}> "a quick red fox" ;
        <{TITLE}> "red herrings" .
<{EX}c> <{NOTE}> "an entirely unrelated remark" ;
        <{TITLE}> "quick asides" .
"""

STATISTICS: dict[str, Any] = {"source": "host-statistics", "revision": "r1"}


def _producers(*pairs: tuple[str, str, str]) -> dict[str, tuple[str, str, str]]:
    """``(producer, stratum, predicate)`` triples as the engine's declaration map."""
    return {
        producer: (stratum, predicate, "any") for producer, stratum, predicate in pairs
    }


def _lexical(text: str, predicate: str) -> tuple[Any, ...]:
    """One lexical request term over an indexed predicate."""
    return ("lexical", text, None, predicate)


NOTE_ONLY = _producers((NOTE_PRODUCER, NOTE_STRATUM, NOTE))
BOTH = _producers(
    (NOTE_PRODUCER, NOTE_STRATUM, NOTE),
    (TITLE_PRODUCER, TITLE_STRATUM, TITLE),
)


def test_one_request_fuses_two_producers_into_one_ranking() -> None:
    """Two producers, one answer, with each row's provenance intact."""
    answer = retrieval.search(
        DATA,
        [_lexical("quick fox", NOTE), _lexical("quick", TITLE)],
        text_producers=BOTH,
        weights={NOTE_STRATUM: retrieval.SCALE, TITLE_STRATUM: retrieval.SCALE},
        statistics=STATISTICS,
        k=60,
        top_k=10,
    )

    ranking = [
        (row["entity"], [(c["stratum"], c["rank"]) for c in row["contributions"]])
        for row in answer["rows"]
    ]
    assert ranking[0] == (
        f"<{EX}a>",
        [(NOTE_STRATUM, 1), (TITLE_STRATUM, 1)],
    ), "ex:a holds the needle in both indexed fields, so both producers rank it first"
    reached = {entity for entity, _ in ranking}
    assert f"<{EX}b>" in reached, "ex:b holds the note needle only"
    assert f"<{EX}c>" in reached, "ex:c holds the title needle only"

    single = [entity for entity, provenance in ranking if len(provenance) == 1]
    assert single, "a candidate only one producer reached carries only that producer"


def test_every_producer_reports_its_own_terminal_status() -> None:
    """Statuses are per producer and are never reduced to one aggregate flag."""
    answer = retrieval.search(
        DATA,
        [_lexical("quick fox", NOTE), _lexical("quick", TITLE)],
        text_producers=BOTH,
        weights={NOTE_STRATUM: retrieval.SCALE, TITLE_STRATUM: retrieval.SCALE},
        statistics=STATISTICS,
        k=60,
        top_k=10,
    )
    statuses = answer["statuses"]
    assert set(statuses) == {NOTE_STRATUM, TITLE_STRATUM}
    for stratum, status in statuses.items():
        assert status["status"] == "exhausted", stratum
        assert status["rows_emitted"] >= 1, stratum


def test_a_term_no_producer_accepts_is_named_not_dropped() -> None:
    """An armed-but-unserved modality is reported per term, typed."""
    answer = retrieval.search(
        DATA,
        [_lexical("quick fox", NOTE), ("entity", f"<{EX}a>")],
        text_producers=NOTE_ONLY,
        weights={NOTE_STRATUM: retrieval.SCALE},
        statistics=STATISTICS,
        k=60,
        top_k=10,
    )
    assert answer["unserved_terms"] == [
        {"request_term": 1, "reason": "no_producer_accepts"}
    ]
    assert answer["rows"], "the term that WAS served still answers"


def test_scores_are_exact_decimals_that_sum_to_their_provenance() -> None:
    """A score is its contributions, exactly — never a float approximation."""
    answer = retrieval.search(
        DATA,
        [_lexical("quick fox", NOTE), _lexical("quick", TITLE)],
        text_producers=BOTH,
        weights={NOTE_STRATUM: retrieval.SCALE, TITLE_STRATUM: retrieval.SCALE},
        statistics=STATISTICS,
        k=60,
        top_k=10,
    )
    from decimal import Decimal

    for row in answer["rows"]:
        assert isinstance(row["score"], str), "a score crosses as an exact decimal"
        total = sum(Decimal(c["contribution"]) for c in row["contributions"])
        assert total == Decimal(row["score"]), row["entity"]


def test_both_identities_name_the_plan_and_the_law() -> None:
    """An answer names which plan and which fusion law produced it.

    The fusion profile is content-addressed, so the same law is the same id on
    every call and in every process. The plan id is NOT: a plan is pinned to the
    registry INSTANCE it was planned against, registration here is per call (as
    it is everywhere else on this binding), and so every call plans against a
    fresh instance. What is durable across calls is the registry's declared
    shape, and the plan carries that separately — see the next test.
    """
    kwargs: dict[str, Any] = {
        "text_producers": NOTE_ONLY,
        "statistics": STATISTICS,
    }
    answer = retrieval.search(
        DATA,
        [_lexical("quick fox", NOTE)],
        weights={NOTE_STRATUM: retrieval.SCALE},
        k=60,
        top_k=10,
        **kwargs,
    )
    assert len(answer["plan_id"]) == 64
    assert len(answer["profile_id"]) == 64, "a content-addressed law names itself"

    # The same weights under a different smoothing constant are a DIFFERENT law.
    other = retrieval.search(
        DATA,
        [_lexical("quick fox", NOTE)],
        weights={NOTE_STRATUM: retrieval.SCALE},
        k=10,
        top_k=10,
        **kwargs,
    )
    assert other["profile_id"] != answer["profile_id"]

    # …and the same law twice is the same id, because it is its own content.
    again = retrieval.search(
        DATA,
        [_lexical("quick fox", NOTE)],
        weights={NOTE_STRATUM: retrieval.SCALE},
        k=60,
        top_k=10,
        **kwargs,
    )
    assert again["profile_id"] == answer["profile_id"]


def test_the_registry_shape_is_durable_where_the_instance_is_not() -> None:
    """Two identical declarations declare the same shape, and two distinct plans.

    This is the honest consequence of per-call registration, stated rather than
    hidden: ``registry_content_fingerprint`` is a pure function of what the
    producers DECLARE, so it survives across calls and processes, while the plan
    id folds in the per-process registry instance a plan is pinned to.
    """
    request = [_lexical("quick fox", NOTE)]
    first = retrieval.plan(
        DATA, request, text_producers=NOTE_ONLY, statistics=STATISTICS
    )
    second = retrieval.plan(
        DATA, request, text_producers=NOTE_ONLY, statistics=STATISTICS
    )
    assert (
        first["registry_content_fingerprint"] == second["registry_content_fingerprint"]
    )
    assert first["plan_id"] != second["plan_id"]


def test_plan_reports_why_a_producer_was_not_selected() -> None:
    """The pure stage says which producers the request reached, and why not."""
    planned = retrieval.plan(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=BOTH,
        statistics=STATISTICS,
    )
    decisions = {d["producer"]: d for d in planned["producer_decisions"]}
    assert decisions[NOTE_PRODUCER]["selected"] is True
    assert decisions[NOTE_PRODUCER]["stratum"] == NOTE_STRATUM
    assert decisions[TITLE_PRODUCER]["selected"] is False
    assert decisions[TITLE_PRODUCER]["reason"] == "no_accepted_term"
    assert planned["statistics"]["source"] == "host-statistics"
    assert planned["statistics"]["revision"] == "r1"


def test_a_measured_cardinality_lowers_the_planned_depth() -> None:
    """Statistics are the host's, and a measurement bounds rather than raises."""
    request = [_lexical("quick fox", NOTE)]
    unbounded = retrieval.plan(
        DATA, request, text_producers=NOTE_ONLY, statistics=STATISTICS
    )
    bounded = retrieval.plan(
        DATA,
        request,
        text_producers=NOTE_ONLY,
        statistics={**STATISTICS, "cardinality": {NOTE_STRATUM: 1}},
    )
    assert bounded["stratum_depths"][NOTE_STRATUM] == 1
    assert (
        bounded["stratum_depths"][NOTE_STRATUM]
        <= unbounded["stratum_depths"][NOTE_STRATUM]
    )
    # The plan records what it assumed, so a replay against moved statistics is
    # a detectable condition rather than a silent replan.
    assert unbounded["statistics"]["entries"] == []
    assert bounded["statistics"]["entries"] == [
        {"subject": NOTE_STRATUM, "cardinality": 1, "selectivity_ppm": None}
    ]


def test_compile_emits_the_sparql_each_stratum_runs() -> None:
    """Admission's value is plain text a host can read, log, or run itself."""
    compiled = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=STATISTICS,
    )
    units = compiled["units"]
    assert len(units) == 1
    assert units[0]["stratum"] == NOTE_STRATUM
    assert '"quick fox"' in units[0]["sparql"], "the needle is a rendered constant"
    assert f"<{NOTE_PRODUCER}>" in units[0]["sparql"], "the unit calls the bound producer"
    assert compiled["plan_id"] == compiled["plan"]["plan_id"]
    assert compiled["planned_resolution"] == {}, (
        "resolution is measured against a fusion law, and this call named none"
    )


def test_compile_says_what_a_plan_costs_without_running_it() -> None:
    """The waist's own question: what does this plan cost, before paying for it?

    Naming the law the host means to fuse under is enough. Nothing is executed —
    ``compile`` never touches the producers' rows — and the answer is already
    there: how deep each stratum plans to read, how deep the law still tells
    adjacent ranks apart, and the one bit that follows from comparing them.
    """
    compiled = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=STATISTICS,
        weights={NOTE_STRATUM: retrieval.SCALE},
        k=60,
    )
    planned = compiled["planned_resolution"][NOTE_STRATUM]
    assert planned["fully_separated"] is True, (
        "a whole unit of weight separates every rank this plan reads"
    )
    assert planned["requested_depth"] >= 1
    assert planned["separates_to"] is None or (
        planned["separates_to"] >= planned["requested_depth"]
    )
    # The units are still the units: asking what the plan costs did not run it.
    assert f"<{NOTE_PRODUCER}>" in compiled["units"][0]["sparql"]

    # The neighbouring case that is NOT fully separated, derived rather than
    # guessed: `weight_for_depth` reports the TRUE minimum weight that still
    # separates every rank this plan reads, so one raw unit less cannot reach it.
    depth = planned["requested_depth"]
    assert depth >= 2, "a single rank has no adjacent pair to lose"
    sufficient = retrieval.weight_for_depth(depth, 60)
    coarse = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=STATISTICS,
        weights={NOTE_STRATUM: sufficient - 1},
        k=60,
    )
    coarse_planned = coarse["planned_resolution"][NOTE_STRATUM]
    assert coarse_planned["requested_depth"] == depth
    assert coarse_planned["fully_separated"] is False, (
        "a depth past where this law stops separating is reported, not refused"
    )
    assert coarse_planned["separates_to"] < depth
    assert coarse["units"], "a coarse plan is still a compiled plan"

    # …and the exact minimum, one unit up, separates it. The boundary is
    # measured, not approximated, and neither side of it is a refusal.
    exact = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=STATISTICS,
        weights={NOTE_STRATUM: sufficient},
        k=60,
    )
    assert exact["planned_resolution"][NOTE_STRATUM]["fully_separated"] is True


def test_compile_refuses_half_a_fusion_law_and_accepts_the_whole_one() -> None:
    """Weights and the smoothing constant are one law between them."""
    common: dict[str, Any] = {
        "text_producers": NOTE_ONLY,
        "statistics": STATISTICS,
    }
    with pytest.raises(ValueError, match="smoothing constant"):
        retrieval.compile(
            DATA,
            [_lexical("quick fox", NOTE)],
            weights={NOTE_STRATUM: retrieval.SCALE},
            **common,
        )
    with pytest.raises(ValueError, match="smoothing constant"):
        retrieval.compile(DATA, [_lexical("quick fox", NOTE)], k=60, **common)

    # Both halves together are a law, and the same call answers.
    whole = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        weights={NOTE_STRATUM: retrieval.SCALE},
        k=60,
        **common,
    )
    assert whole["planned_resolution"][NOTE_STRATUM]["fully_separated"] is True


def test_an_answer_reports_planned_and_observed_resolution_apart() -> None:
    """Both altitudes are on the answer, under names that cannot be confused.

    ``planned_resolution`` is what the admission waist said the plan's depths
    would cost; ``observed_resolution`` is what the rows this run really pulled
    did cost. They disagree here exactly as they should: a three-document corpus
    exhausts after a handful of ranks, while the plan reserved the depth the
    producer declared it could fill, and a depth nothing reached cost nothing.
    """
    request = [_lexical("quick fox", NOTE)]
    answer = retrieval.search(
        DATA,
        request,
        text_producers=NOTE_ONLY,
        weights={NOTE_STRATUM: retrieval.SCALE},
        statistics=STATISTICS,
        k=60,
        top_k=10,
    )

    planned = answer["planned_resolution"][NOTE_STRATUM]
    observed = answer["observed_resolution"][NOTE_STRATUM]
    assert planned["fully_separated"] is True
    assert observed["collisions_observed"] == 0
    assert observed["ranks_pulled"] <= planned["requested_depth"], (
        "a run cannot pull deeper than the plan it descends from"
    )

    # And it is the WAIST's value, not a second derivation: compiling the same
    # request under the same law, executing nothing, reports the same thing.
    compiled = retrieval.compile(
        DATA,
        request,
        text_producers=NOTE_ONLY,
        statistics=STATISTICS,
        weights={NOTE_STRATUM: retrieval.SCALE},
        k=60,
    )
    assert compiled["planned_resolution"] == answer["planned_resolution"]


def test_no_producer_is_refused_by_name() -> None:
    """An empty producer set is a refusal; there is no default to fall back on."""
    with pytest.raises(ValueError, match="no ranked producers"):
        retrieval.plan(
            DATA, [_lexical("quick fox", NOTE)], text_producers={}, statistics=STATISTICS
        )


def test_statistics_must_name_their_own_revision() -> None:
    """The planner records the revision verbatim and invents none."""
    with pytest.raises(ValueError, match="revision"):
        retrieval.plan(
            DATA,
            [_lexical("quick fox", NOTE)],
            text_producers=NOTE_ONLY,
            statistics={"source": "host-statistics"},
        )


def test_a_float_weight_is_refused_and_an_exact_one_is_not() -> None:
    """The refusal is about exactness, and the neighbouring exact case still works."""
    common: dict[str, Any] = {
        "text_producers": NOTE_ONLY,
        "statistics": STATISTICS,
        "k": 60,
        "top_k": 10,
    }
    with pytest.raises(TypeError, match="raw fixed-point units"):
        retrieval.search(
            DATA, [_lexical("quick fox", NOTE)], weights={NOTE_STRATUM: 0.5}, **common
        )

    answer = retrieval.search(
        DATA,
        [_lexical("quick fox", NOTE)],
        weights={NOTE_STRATUM: retrieval.SCALE // 2},
        **common,
    )
    assert answer["rows"], "half a unit, stated exactly, is a perfectly good weight"


def test_a_multi_partition_index_refuses_to_claim_a_ranking() -> None:
    """A rank is computed within one partition, so two languages declare none."""
    tagged = (
        f'<{EX}a> <{NOTE}> "the quick fox"@en .\n'
        f'<{EX}b> <{NOTE}> "le renard vif"@fr .\n'
    )
    with pytest.raises(ValueError, match="partition"):
        retrieval.plan(
            tagged, [_lexical("quick", NOTE)], text_producers=NOTE_ONLY, statistics=STATISTICS
        )

    # The neighbouring valid case: one language is one partition, and it answers.
    one_language = (
        f'<{EX}a> <{NOTE}> "the quick fox"@en .\n'
        f'<{EX}b> <{NOTE}> "a quick hound"@en .\n'
    )
    planned = retrieval.plan(
        one_language, [_lexical("quick", NOTE)], text_producers=NOTE_ONLY, statistics=STATISTICS
    )
    assert planned["producer_bindings"], "a single-partition index declares a ranked order"


def test_an_unknown_request_kind_names_what_is_accepted() -> None:
    """A malformed request is refused where it is written, not later."""
    with pytest.raises(ValueError, match="unknown kind"):
        retrieval.plan(
            DATA,
            [("keyword", "quick fox", None, NOTE)],
            text_producers=NOTE_ONLY,
            statistics=STATISTICS,
        )


def test_an_unknown_graph_selector_names_what_is_accepted() -> None:
    """A producer's graph selector is "any", "default", or a named-graph IRI."""
    with pytest.raises(ValueError, match="unknown graph selector"):
        retrieval.plan(
            DATA,
            [_lexical("quick fox", NOTE)],
            text_producers={NOTE_PRODUCER: (NOTE_STRATUM, NOTE, "every")},
            statistics=STATISTICS,
        )
