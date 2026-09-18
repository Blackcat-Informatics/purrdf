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

import re
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

# The two decay rules, by the spelling the binding reads them under. Neither is
# a default: they compute different contributions from the same weights and
# reach different depths, so every call that names a fusion law names one.
TRUNCATED = "reciprocal_rank"
FOLDED = "weighted_reciprocal_rank"


def _producers(*pairs: tuple[str, str, str]) -> dict[str, tuple[str, str, str]]:
    """``(producer, stratum, predicate)`` triples as the engine's declaration map."""
    return {
        producer: (stratum, predicate, "any") for producer, stratum, predicate in pairs
    }


def _lexical(text: str, predicate: str) -> tuple[Any, ...]:
    """One lexical request term over an indexed predicate."""
    return ("lexical", text, None, predicate)


# The deepest depth anything downstream can name: a plan carries a per-stratum
# depth as a 32-bit rank.
PLAN_DEPTH_LIMIT = 2**32 - 1


def _truncated_saturation(k: int) -> int:
    """The exact depth ``"reciprocal_rank"`` separates to, read out of its refusal.

    Derived rather than written down. The truncated rule's wall is a property of
    the layer's declared fixed-point scale and of ``k``, and the refusal names
    the depth the rule does reach, so every boundary these tests probe comes
    from the surface under test rather than from a literal that could quietly
    drift away from it.
    """
    with pytest.raises(ValueError, match="no weight separates ranks to depth") as refused:
        retrieval.weight_for_depth(PLAN_DEPTH_LIMIT, k, decay=TRUNCATED)
    found = re.search(r"separates to depth (\d+)", str(refused.value))
    assert found is not None, str(refused.value)
    return int(found.group(1))


def _measured_depth(depth: int | None) -> int:
    """A tolerated depth that is a measurement rather than a saturation point.

    ``retrieval.deepest_rank_within_width`` answers ``None`` when no depth a
    plan can express ever exceeds the tolerance, exactly as ``"separates_to"``
    does on a ``search`` answer. Every call below that is meant to land inside a
    plan's range says so through here, so a case that silently starts saturating
    fails where it is asked rather than being compared as a number.
    """
    assert depth is not None, (
        "this call was expected to measure a depth inside a plan's range, and "
        "reported that it has no bound there at all"
    )
    return depth


def _counted_width(width: int | None) -> int:
    """A class width that was counted end to end rather than run off the range.

    ``retrieval.class_width`` answers ``None`` when the class is still running
    at the deepest rank a plan can express, which is a saturation point and not
    a count. Every call below that is meant to land on a counted class says so
    through here, so a case that silently starts saturating fails where it is
    asked rather than being compared as a number.
    """
    assert width is not None, (
        "this call was expected to count a class that ends inside a plan's "
        "range, and reported that it has no end there at all"
    )
    return width


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
        decay=TRUNCATED,
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
        decay=TRUNCATED,
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
        decay=TRUNCATED,
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
        decay=TRUNCATED,
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
        decay=TRUNCATED,
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
        decay=TRUNCATED,
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
        decay=TRUNCATED,
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
        decay=TRUNCATED,
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
    sufficient = retrieval.weight_for_depth(depth, 60, decay=TRUNCATED)
    coarse = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=STATISTICS,
        weights={NOTE_STRATUM: sufficient - 1},
        k=60,
        decay=TRUNCATED,
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
        decay=TRUNCATED,
    )
    assert exact["planned_resolution"][NOTE_STRATUM]["fully_separated"] is True


def test_compile_refuses_part_of_a_fusion_law_and_accepts_the_whole_one() -> None:
    """Weights, the smoothing constant and the decay rule are ONE law together.

    Any subset of the three names no law at all, and the refusal says which part
    arrived and which did not. Filling the absent part in would report a
    resolution measured against arithmetic the caller never wrote — and the
    decay rule is the part that most changes the measurement, so it is the part
    least defensible to guess.
    """
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
        retrieval.compile(
            DATA, [_lexical("quick fox", NOTE)], k=60, decay=TRUNCATED, **common
        )
    # Two thirds of a law is still not a law, and the message names the third
    # that is missing rather than choosing one.
    with pytest.raises(ValueError, match=r"left the `decay` rule unnamed"):
        retrieval.compile(
            DATA,
            [_lexical("quick fox", NOTE)],
            weights={NOTE_STRATUM: retrieval.SCALE},
            k=60,
            **common,
        )

    # All three together are a law, and the same call answers.
    whole = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        weights={NOTE_STRATUM: retrieval.SCALE},
        k=60,
        decay=TRUNCATED,
        **common,
    )
    assert whole["planned_resolution"][NOTE_STRATUM]["fully_separated"] is True

    # Naming NONE of the three is not a refusal: it compiles without a law and
    # is simply told nothing about resolution.
    lawless = retrieval.compile(DATA, [_lexical("quick fox", NOTE)], **common)
    assert lawless["planned_resolution"] == {}
    assert lawless["units"], "a call that names no law is still a compiled plan"


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
        decay=TRUNCATED,
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
        decay=TRUNCATED,
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
        "decay": TRUNCATED,
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


def test_the_folded_rule_reaches_every_number_in_the_answer() -> None:
    """Naming the folded rule changes the arithmetic, not just the constructor.

    A rule that only reached the profile's constructor would still produce
    truncated-rule contributions, and the difference is invisible unless it is
    computed independently. So the expected raw contribution is derived here
    from the rule's own definition rather than pinned to a literal: the
    truncated rule is ``trunc(w * trunc(S / D) / S)`` and the folded rule is
    ``trunc(w / D)``, with ``D = k + rank``, and at this weight they differ by
    exactly one raw unit at rank one.
    """
    from decimal import Decimal

    scale = retrieval.SCALE
    # A weight of one and a half units: the folded rule's single division keeps
    # a raw unit the truncated rule's inner rounding throws away.
    weight = scale + scale // 2
    denominator = 60 + 1
    expected_truncated = weight * (scale // denominator) // scale
    expected_folded = weight // denominator
    assert expected_folded == expected_truncated + 1, (
        "this fixture is only a witness while the two rules disagree here"
    )

    def _top_row(decay: str) -> dict[str, Any]:
        answer = retrieval.search(
            DATA,
            [_lexical("quick fox", NOTE)],
            text_producers=NOTE_ONLY,
            weights={NOTE_STRATUM: weight},
            statistics=STATISTICS,
            k=60,
            decay=decay,
            top_k=10,
        )
        return answer

    truncated = _top_row(TRUNCATED)
    folded = _top_row(FOLDED)

    # One stratum, so the fused score IS the rank-one contribution.
    assert Decimal(truncated["rows"][0]["score"]) * scale == expected_truncated
    assert Decimal(folded["rows"][0]["score"]) * scale == expected_folded
    assert (
        Decimal(folded["rows"][0]["contributions"][0]["contribution"]) * scale
        == expected_folded
    ), "the per-stratum provenance is computed under the named rule too"

    # …and the law that produced it names itself differently, because the rule
    # is part of what the profile's content identity fixes.
    assert folded["profile_id"] != truncated["profile_id"]
    assert len(folded["profile_id"]) == 64


def test_a_depth_the_truncated_rule_refuses_is_answered_by_the_folded_one() -> None:
    """The remedy the refusal recommends is reachable from Python.

    ``weight_for_depth`` under ``"reciprocal_rank"`` refuses a deep enough
    request because that rule rounds the reciprocal BEFORE the weight lands, so
    past a point no weight parts two adjacent ranks. The error says so and names
    the depth it does reach. That is a diagnosis, and the cure is to fuse under
    ``"weighted_reciprocal_rank"`` instead — so the SAME depth must answer
    there. The boundary is read out of the refusal rather than hardcoded.
    """
    saturates_at = _truncated_saturation(60)
    assert 0 < saturates_at < PLAN_DEPTH_LIMIT

    # The neighbouring VALID case, one rank shallower than the wall: the same
    # rule answers, so the refusal is about the depth and nothing else.
    reachable = retrieval.weight_for_depth(saturates_at, 60, decay=TRUNCATED)
    assert reachable > 0

    # One rank deeper is the wall, and no weight lifts it.
    with pytest.raises(ValueError, match="no weight separates ranks to depth"):
        retrieval.weight_for_depth(saturates_at + 1, 60, decay=TRUNCATED)

    # THE CURE, reachable from the same surface that delivered the diagnosis:
    # the folded rule answers at exactly the depth the truncated one refused.
    cured = retrieval.weight_for_depth(saturates_at + 1, 60, decay=FOLDED)
    assert cured > 0

    # And it keeps answering well past the wall, because its reachable depth
    # grows with the weight rather than being capped by the inner rounding.
    assert retrieval.weight_for_depth(PLAN_DEPTH_LIMIT, 60, decay=FOLDED) > cured

    # The same split shows in the resolution curve, read at the very weight the
    # folded rule quoted. At the truncated rule's own last separating rank that
    # rule has ALREADY put two ranks into one class, while the folded rule still
    # holds them apart — which is the whole difference the cure buys, measured
    # rather than asserted from the refusal alone.
    assert retrieval.class_width(cured, 60, saturates_at, decay=TRUNCATED) == 2
    assert retrieval.class_width(cured, 60, saturates_at, decay=FOLDED) == 1


def test_a_zero_smoothing_constant_is_one_refusal_across_all_three_functions() -> None:
    """``k = 0`` describes no law, and every resolution question says exactly that.

    The three functions are one algebra read from three directions, so they must
    agree about what an invalid law is — and agreement is not "all three raise
    something". Two of them answer from a short-circuit for the smallest
    argument on their own axis: a ``depth`` of one has no adjacent pair to
    separate, and a ``max_width`` of one does no walking. Neither reaches the
    arithmetic that would notice ``k``, so both once handed back a real number
    priced under a rule that cannot be evaluated, while the very same call one
    step along the axis refused — and refused by reporting that the decay rule
    had *saturated* at depth one.

    That is the wrong cause told confidently. Saturation's remedy is the other
    decay rule; a constant of zero is not cured by switching rules, so a caller
    that followed the message would ask the same impossible question again.

    So this pins the cause and not merely the failure: the message names the
    constant and does not tell the saturation story. Both rules, both the
    short-circuiting argument and the walking one, and every case paired with
    ``k = 1`` — the smallest usable law there is — which must still answer.
    """
    weight = 1000 * retrieval.SCALE
    for decay in (TRUNCATED, FOLDED):
        for depth in (1, 2, 64):
            with pytest.raises(ValueError, match="K must be at least 1") as refused:
                retrieval.weight_for_depth(depth, 0, decay=decay)
            assert "no further" not in str(refused.value), (
                f"{decay}, depth {depth}: a malformed law is not a saturated rule"
            )
            assert retrieval.weight_for_depth(depth, 1, decay=decay) > 0, (
                f"{decay}, depth {depth}: the smallest usable constant prices it"
            )

        for rank in (1, 2, 64):
            with pytest.raises(ValueError, match="K must be at least 1") as refused:
                retrieval.class_width(weight, 0, rank, decay=decay)
            assert "no further" not in str(refused.value), (
                f"{decay}, rank {rank}: a malformed law is not a saturated rule"
            )
            assert retrieval.class_width(weight, 1, rank, decay=decay) == 1, (
                f"{decay}, rank {rank}: this weight separates it from both neighbours"
            )

        for max_width in (1, 2, 64):
            with pytest.raises(ValueError, match="K must be at least 1") as refused:
                retrieval.deepest_rank_within_width(weight, 0, max_width, decay=decay)
            assert "no further" not in str(refused.value), (
                f"{decay}, max_width {max_width}: a malformed law is not a "
                "saturated rule"
            )
            assert (
                _measured_depth(
                    retrieval.deepest_rank_within_width(
                        weight, 1, max_width, decay=decay
                    )
                )
                > 1
            ), (
                f"{decay}, max_width {max_width}: the smallest usable constant "
                "reads past a single rank"
            )


def test_the_decay_rule_must_be_named_and_is_named_by_its_own_spelling() -> None:
    """No hidden default, and an unknown spelling names the accepted ones."""
    common: dict[str, Any] = {
        "text_producers": NOTE_ONLY,
        "weights": {NOTE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "top_k": 10,
    }

    # Omitted: refused, naming the argument that is missing.
    with pytest.raises(TypeError, match="decay"):
        retrieval.search(DATA, [_lexical("quick fox", NOTE)], **common)
    with pytest.raises(TypeError, match="decay"):
        retrieval.weight_for_depth(16, 60)
    with pytest.raises(TypeError, match="decay"):
        retrieval.class_width(retrieval.SCALE, 60, 4)
    with pytest.raises(TypeError, match="decay"):
        retrieval.deepest_rank_within_width(retrieval.SCALE, 60, 4)

    # Unknown: refused, naming both accepted spellings.
    for unknown in ("rrf", "", "ReciprocalRank"):
        with pytest.raises(ValueError, match="unknown decay rule") as refused:
            retrieval.search(
                DATA, [_lexical("quick fox", NOTE)], decay=unknown, **common
            )
        assert "reciprocal_rank" in str(refused.value)
        assert "weighted_reciprocal_rank" in str(refused.value)
    with pytest.raises(ValueError, match="unknown decay rule"):
        retrieval.weight_for_depth(16, 60, decay="rrf")
    with pytest.raises(ValueError, match="unknown decay rule"):
        retrieval.class_width(retrieval.SCALE, 60, 4, decay="rrf")
    with pytest.raises(ValueError, match="unknown decay rule"):
        retrieval.deepest_rank_within_width(retrieval.SCALE, 60, 4, decay="rrf")

    # The neighbouring VALID calls: every one of the four answers under either
    # accepted spelling, so the refusals above are about the value and nothing
    # else.
    for named in (TRUNCATED, FOLDED):
        assert retrieval.search(
            DATA, [_lexical("quick fox", NOTE)], decay=named, **common
        )["rows"]
        assert retrieval.weight_for_depth(16, 60, decay=named) > 0
        assert retrieval.class_width(retrieval.SCALE, 60, 4, decay=named) == 1
        assert (
            _measured_depth(
                retrieval.deepest_rank_within_width(retrieval.SCALE, 60, 4, decay=named)
            )
            >= 1
        )


def test_class_width_asks_about_arithmetic_and_needs_no_stratum() -> None:
    """A width is the rule, the constant, the weight and the rank — nothing else.

    No stratum is supplied and none is invented: PurRDF mints no IRIs, and a
    probe IRI conjured to ask a question about arithmetic would be a minted one.
    """
    # Inside the separating range every rank stands alone.
    assert retrieval.class_width(retrieval.SCALE, 60, 1, decay=TRUNCATED) == 1

    # Past it the classes widen, and the curve belongs to the RULE: a weight
    # heavy enough for the fold to buy depth is coarse under the truncated rule
    # and still exact under the folded one at the same rank.
    heavy = 1000 * retrieval.SCALE
    deep = 16 * _truncated_saturation(60)
    coarse = _counted_width(retrieval.class_width(heavy, 60, deep, decay=TRUNCATED))
    fine = _counted_width(retrieval.class_width(heavy, 60, deep, decay=FOLDED))
    assert coarse > 1, "the truncated rule has lost resolution by this depth"
    assert fine < coarse, "and the folded rule has not"

    # The operands it cannot evaluate are refused rather than answered with the
    # most favourable width there is, each beside the neighbour that works.
    with pytest.raises(ValueError, match="rank must be at least 1"):
        retrieval.class_width(retrieval.SCALE, 60, 0, decay=TRUNCATED)
    assert retrieval.class_width(retrieval.SCALE, 60, 1, decay=TRUNCATED) == 1

    with pytest.raises(ValueError, match="K must be at least 1"):
        retrieval.class_width(retrieval.SCALE, 0, 4, decay=TRUNCATED)
    assert retrieval.class_width(retrieval.SCALE, 1, 4, decay=TRUNCATED) == 1

    with pytest.raises(ValueError, match="strictly positive"):
        retrieval.class_width(0, 60, 4, decay=TRUNCATED)
    # A raw weight of one IS evaluable — the refusal above is about the zero and
    # nothing else — and what it evaluates to is the saturating case, because
    # every contribution at that weight has truncated to zero. It is `None`, not
    # a number: see the test below for why that distinction is load-bearing.
    assert retrieval.class_width(1, 60, 4, decay=TRUNCATED) is None


def test_a_class_with_no_end_inside_a_plans_range_is_none_and_not_a_number() -> None:
    """The wall the width curve runs into, told as a wall.

    A plan records a per-stratum depth as a 32-bit rank, so the search for the
    class's far end stops there. A class still running at that rank has no
    counted end, and ``2 ** 32 - 1`` would be the search's own ceiling handed
    back as a measurement — a number a caller can log, plot or divide by, quoted
    where nothing was counted.

    Raw weights of one and fifty are **fifty times apart** and both saturate:
    under either rule every contribution has truncated to the same value, so the
    class at rank one is the whole expressible range in both cases. A bare
    number reported them as the identical width ``4294967295``.
    """
    for raw in (1, 50):
        for decay in (TRUNCATED, FOLDED):
            for rank in (1, 4, 1000):
                assert retrieval.class_width(raw, 60, rank, decay=decay) is None, (
                    f"{decay}, raw {raw}, rank {rank}: this class has no end "
                    "inside a plan's range to count to"
                )

    # The neighbouring weights whose classes DO end inside the range still
    # report counts, so the saturating case is about the measurement and not
    # about light weights in general. Every magnitude here was executed.
    for raw, decay, measured in (
        (99, TRUNCATED, 38),
        (99, FOLDED, 39),
        (100, TRUNCATED, 40),
        (100, FOLDED, 40),
    ):
        assert retrieval.class_width(raw, 60, 1, decay=decay) == measured, (
            f"{decay}, raw {raw}: this class ends inside the range and is counted"
        )


def test_deepest_rank_within_width_asks_about_arithmetic_and_needs_no_stratum() -> None:
    """The third leg of the resolution algebra: name the tolerance, get the depth.

    No stratum is supplied and none is invented, for the same reason
    ``class_width`` takes none: PurRDF mints no IRIs, and a probe IRI conjured
    to ask a question about arithmetic would be a minted one.
    """
    # A tolerance of one is the deepest depth a read can stop at with every rank
    # it actually read separated, and it only grows as the tolerance is relaxed.
    narrow = _measured_depth(
        retrieval.deepest_rank_within_width(retrieval.SCALE, 60, 1, decay=TRUNCATED)
    )
    wide = _measured_depth(
        retrieval.deepest_rank_within_width(retrieval.SCALE, 60, 50, decay=TRUNCATED)
    )
    assert wide > narrow, "a larger tolerance reads deeper, never shallower"

    # The operands it cannot evaluate are refused rather than answered with the
    # most favourable depth there is, each beside the neighbour that works.
    with pytest.raises(ValueError, match="K must be at least 1"):
        retrieval.deepest_rank_within_width(retrieval.SCALE, 0, 4, decay=TRUNCATED)
    assert (
        _measured_depth(
            retrieval.deepest_rank_within_width(retrieval.SCALE, 1, 4, decay=TRUNCATED)
        )
        >= 1
    )

    with pytest.raises(ValueError, match="strictly positive"):
        retrieval.deepest_rank_within_width(0, 60, 4, decay=TRUNCATED)
    assert (
        _measured_depth(
            retrieval.deepest_rank_within_width(1, 60, 4, decay=TRUNCATED)
        )
        >= 1
    )

    with pytest.raises(ValueError, match="strictly positive"):
        retrieval.deepest_rank_within_width(-1, 60, 4, decay=TRUNCATED)


def test_a_tolerance_of_zero_is_refused_and_a_tolerance_of_one_answers() -> None:
    """A class always contains its own rank, so zero describes no class at all.

    That is the same shape as the smoothing constant of zero that describes no
    law, and it is refused on the same terms rather than quietly normalised up
    to one. Reading it as one answered the narrowest REAL tolerance in its
    place, which is the deepest fully-separated depth this algebra reports: the
    most favourable answer there is, returned precisely where nothing was asked.

    The neighbouring valid tolerance is executed in this same test, per the
    repository's rule that a refusal is proved from both sides.
    """
    for decay in (TRUNCATED, FOLDED):
        with pytest.raises(ValueError, match="tolerance must be at least 1") as refused:
            retrieval.deepest_rank_within_width(1000, 60, 0, decay=decay)
        assert "a tolerance of zero is not a tolerance" in str(refused.value), (
            f"{decay}: the refusal must say what is wrong with the operand, not "
            "point at a rank that was never at fault"
        )
        assert "rank must be at least 1" not in str(refused.value), (
            f"{decay}: a tolerance is not a position on the 1-based rank axis"
        )

        # The neighbouring valid case: the narrowest tolerance that does
        # describe a class, at the same weight under the same rule.
        assert (
            _measured_depth(
                retrieval.deepest_rank_within_width(1000, 60, 1, decay=decay)
            )
            >= 1
        ), f"{decay}: a tolerance of one is a tolerance and still answers"


def test_a_depth_that_outruns_every_plan_is_reported_as_saturation() -> None:
    """``None`` is the wall; a number would be the wall wearing a measurement.

    A plan records a per-stratum depth as a 32-bit rank. Two weights fifty times
    apart both read past every depth in that range, and handing back
    ``2**32 - 1`` from each says they reach the SAME depth -- a number a caller
    can log, plot or divide by -- when the only true statement is that neither
    has a bound inside any plan's reach. ``search`` already renders that fact as
    ``"separates_to": None``; this is the same fact from the arithmetic-only
    entry point, and it is rendered the same way.
    """
    fifty_times_apart = (20_000_000 * retrieval.SCALE, 1_000_000_000 * retrieval.SCALE)
    for weight in fifty_times_apart:
        assert (
            retrieval.deepest_rank_within_width(weight, 60, 1, decay=FOLDED) is None
        ), (
            f"weight {weight}: a law with no bound inside any plan's range has "
            "no depth to report, not an enormous one"
        )

    # The neighbouring weight below the wall reports a real depth, so the pair
    # above is about the wall and not about the question being unanswerable.
    below = _measured_depth(
        retrieval.deepest_rank_within_width(
            18_000_000 * retrieval.SCALE, 60, 1, decay=FOLDED
        )
    )
    assert 0 < below < PLAN_DEPTH_LIMIT, (
        "a bound inside a plan's range is a measurement and is reported as one"
    )

    # And the third entry point, handed a REQUESTED depth rather than measuring
    # one, refuses at the identical boundary as a limit of the plan's depth
    # encoding -- with the neighbouring in-range depth still priced.
    with pytest.raises(ValueError, match="deeper than a plan can record"):
        retrieval.weight_for_depth(PLAN_DEPTH_LIMIT + 1, 60, decay=FOLDED)
    assert retrieval.weight_for_depth(PLAN_DEPTH_LIMIT, 60, decay=FOLDED) > 0


def test_deepest_rank_within_width_answers_under_both_rules_and_the_rules_differ() -> None:
    """The curve belongs to the rule, exactly as it does for ``class_width``.

    Read past the truncated rule's wall, the same tolerance buys the folded
    rule a strictly deeper answer, because the folded rule's reachable depth
    grows with the weight while the truncated one's does not.
    """
    heavy = 1000 * retrieval.SCALE
    for max_width in (1, 5, 50):
        truncated_depth = _measured_depth(
            retrieval.deepest_rank_within_width(heavy, 60, max_width, decay=TRUNCATED)
        )
        folded_depth = _measured_depth(
            retrieval.deepest_rank_within_width(heavy, 60, max_width, decay=FOLDED)
        )
        assert folded_depth > truncated_depth, (
            f"max_width {max_width}: the folded rule must read deeper than the "
            "truncated one at the same weight and tolerance"
        )


def test_deepest_rank_within_width_agrees_with_class_width_and_weight_for_depth() -> None:
    """The three functions agree with each other where they must.

    ``deepest_rank_within_width`` answers a DEPTH question: the deepest rank
    reachable by a read truncated there, with every rank it actually reads
    sitting in a class no wider than the tolerance. ``class_width`` answers a
    RANK question over the unbounded curve, which also looks at the one rank a
    truncated read never reaches — so at the reported depth ``D``,
    ``class_width`` finds ``D`` sharing a contribution with ``D + 1`` and
    reports a class of at least ``max_width + 1``, wider than the bounded read
    that stopped at ``D`` ever observes. It is NEVER one — which is what the
    claim "the deepest rank still separated from both its neighbours" would
    predict at a tolerance of one, and that claim was wrong in exactly this way.
    The relation is an inequality and not an equality because the run that ends
    the walk can be longer than the tolerance by more than a rank. At the heavy
    weight below it happens to be exactly ``max_width + 1``, which is why the
    light weights are executed here too: at a raw weight of ``10 ** 2`` a
    tolerance of one lands on depth one and the class there is **forty** ranks
    wide, so equality is the smooth case rather than the law.

    That is the same "depth, not a rank property" distinction this function's
    own documentation draws, and it means the class at ``D`` and at ``D + 1``
    both already exceed the tolerance the bounded read stayed inside of.

    For a tolerance of one this also ties in ``weight_for_depth``, by its own
    exact inverse round trip: ``weight_for_depth`` names the smallest weight
    that separates every adjacent pair up to some depth ``D``, and asking
    ``deepest_rank_within_width`` for the deepest fully-separated rank *at
    that exact weight* must land back on ``D`` — not shallower, because that
    weight was built to reach it, and not deeper, because it is the smallest
    weight that does. ``weight_for_depth`` is not monotone in the weight (a
    heavier weight can fail to reach a depth a lighter one reaches), so this
    round trip through the precise minimum is the honest way to tie the two
    together — anchoring on some larger, arbitrarily-chosen weight is not.
    """
    heavy = 1000 * retrieval.SCALE
    for decay in (TRUNCATED, FOLDED):
        for max_width in (1, 5, 50):
            depth = _measured_depth(
                retrieval.deepest_rank_within_width(heavy, 60, max_width, decay=decay)
            )
            at_depth = _counted_width(
                retrieval.class_width(heavy, 60, depth, decay=decay)
            )
            one_deeper = _counted_width(
                retrieval.class_width(heavy, 60, depth + 1, decay=decay)
            )
            assert at_depth >= max_width + 1, (
                f"{decay}, max_width {max_width}: the reported depth already sits "
                "in an over-tolerance class on the unbounded curve, which also "
                "looks at the one rank the bounded read never reaches"
            )
            assert at_depth == max_width + 1, (
                f"{decay}, max_width {max_width}: and at this weight the run that "
                "ended the walk is exactly one rank longer than the tolerance"
            )
            assert one_deeper > max_width, (
                f"{decay}, max_width {max_width}: one rank deeper must exceed the "
                "tolerance too"
            )

        # The weight_for_depth round trip, at the tolerance where this
        # function agrees exactly with the separating bound.
        for named_depth in (16, 100, 972):
            minimum_weight = retrieval.weight_for_depth(named_depth, 60, decay=decay)
            round_tripped = _measured_depth(
                retrieval.deepest_rank_within_width(minimum_weight, 60, 1, decay=decay)
            )
            assert round_tripped == named_depth, (
                f"{decay}: the minimum weight that separates to depth "
                f"{named_depth} must itself separate to exactly that depth, "
                f"got {round_tripped}"
            )

    # The same law where it is NOT trivially true. These raw weights are four
    # orders of magnitude lighter than `heavy`, their contributions collapse
    # within the first handful of ranks, and a tolerance of one therefore lands
    # on depth ONE with a class tens of ranks wide. ">= max_width + 1" is all
    # that holds here; "two at a tolerance of one" is false by a factor of
    # twenty. Every magnitude below was executed.
    for raw, decay, measured in (
        (100, TRUNCATED, 40),
        (100, FOLDED, 40),
        (121, TRUNCATED, 60),
        (121, FOLDED, 61),
        (200, TRUNCATED, 6),
        (1000, TRUNCATED, 2),
    ):
        depth = _measured_depth(
            retrieval.deepest_rank_within_width(raw, 60, 1, decay=decay)
        )
        assert depth == 1, (
            f"{decay}, raw {raw}: these weights stop separating immediately"
        )
        at_depth = _counted_width(retrieval.class_width(raw, 60, depth, decay=decay))
        assert at_depth == measured, (
            f"{decay}, raw {raw}: the class at the reported depth, measured"
        )
        assert at_depth >= 2, (
            f"{decay}, raw {raw}: the reported depth is never the one a "
            "'separated from both neighbours' reading would predict"
        )
