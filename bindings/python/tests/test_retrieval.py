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
  to ``(stratum, predicate, graph)``, or to ``(stratum, predicate, graph,
  domains)`` where the producer can say which blocks of the candidate universe
  it draws from; the engine builds the index and registers the relation itself.
  Nothing it invokes can re-enter the interpreter, which is why the whole ladder
  still runs with the GIL released.
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
* **An answer says what evidence it was produced against.** Every stratum's
  index attests what it can honestly attest — which generation answered, and
  whether that generation was short — and the answer carries those attestations,
  the exactness that follows from them, the candidate-domain declaration each
  stream fused under, and the ``evidence_id`` that is their content identity. An
  absence there is an absence, never a certificate of wholeness.
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

# The row bound the plan-and-compile fixtures ask for.
#
# `plan` and `compile` require it because it is a planning input: it decides how
# deep each stratum is read wherever the producers' own `domains` make that sound.
# This value is above anything the small fixture corpora hold, so the fixtures
# below measure the depth the declarations and the statistics set rather than a
# depth this number cut — the narrowing itself is measured where it is the subject,
# in `test_a_bounded_request_reads_a_flat_prefix_of_each_disjoint_stratum`.
PLAN_TOP_K = 1024

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


# The two blocks of the candidate universe the disjoint-corpus cases below
# declare over. They are the HOST's tags, invented by these tests for their own
# fixture, because which entities an index names is a fact about the corpus that
# neither the engine nor the relation can see.
NOTE_DOMAIN = f"{EX}domain/notes"
TITLE_DOMAIN = f"{EX}domain/titles"

# The seven spellings a terminal status may carry, and nothing else may appear.
# Only the first names no stopper; the other six each name who stopped the read
# and where. Naming no stopper is not on its own a completeness claim -- that is
# what the stratum's ``"fidelities"`` entry says beside it.
STATUS_SPELLINGS = frozenset(
    {
        "exhausted",
        "depth_reached",
        "row_bound_reached",
        "supplied_query_ended",
        "ceiling_reached",
        "execution_failed",
        "terms_rejected",
    }
)


def _disjoint_corpus(rows: int = 40) -> str:
    """A corpus whose two indexed predicates name entirely disjoint entities.

    ``ex:n0..`` carry notes and nothing else, ``ex:t0..`` carry titles and
    nothing else, so a producer over each really does draw from its own block
    and the declaration these tests make about them is TRUE. Both fields hold
    the same needle with varying term counts, so both producers rank every one
    of their own entities and the scores separate.

    It is deliberately far larger than a ``top_k`` of three: what a domain
    declaration buys is visible only where a bounded read could stop early, and
    over a three-document corpus every stream is exhausted before the question
    arises.
    """
    notes = [
        f'<{EX}n{i}> <{NOTE}> "quick {"fox " * (1 + i % 5)}note {i}" .'
        for i in range(rows)
    ]
    titles = [
        f'<{EX}t{i}> <{TITLE}> "quick {"hound " * (1 + i % 7)}title {i}" .'
        for i in range(rows)
    ]
    return "\n".join(notes + titles)


def _declared(
    *entries: tuple[str, str, str, list[str] | None],
) -> dict[str, tuple[Any, ...]]:
    """``(producer, stratum, predicate, domains)`` as the engine's declaration map.

    The four-element spelling of a ``text_producers`` value. ``domains`` of
    ``None`` is the unrestricted promise — the same declaration the three-element
    spelling makes — and a list restricts the producer to those blocks.
    """
    return {
        producer: (stratum, predicate, "any", domains)
        for producer, stratum, predicate, domains in entries
    }


def _emitted_limit(sparql: str) -> int:
    """The row bound a compiled unit's own text carries, read off that text.

    The unit's trailing ``LIMIT`` — the one bounding the whole ``SELECT``, not
    any bound an inner call renders into its own arguments. It is read here so a
    test can compare what the text is allowed to RETURN against the ``"depth"``
    the unit says may be REPORTED; those two numbers differ by the probe row
    wherever the producer's declaration left room for one.
    """
    found = re.search(r"LIMIT (\d+)\s*$", sparql)
    assert found is not None, f"a compiled unit carries a trailing LIMIT: {sparql!r}"
    return int(found.group(1))


def _emitted_limits(sparql: str) -> list[int]:
    """EVERY row bound a compiled unit's text carries, in the order it writes them.

    A unit bounds itself twice: the inner ``SELECT`` around the producer call
    carries the planned depth, and the outer one carries it again. Only the
    second is ``_emitted_limit``, and a test that reads only the second cannot
    see the first go missing or go wrong — which is the bound that actually stops
    the producer's read.

    The two are the same number by construction, and that is exactly why the
    substring they share is not a check: ``"LIMIT 2" in sparql`` stays true when
    the inner one is dropped, because the outer one still spells it. Reading them
    as a LIST makes their count and their positions observable, so an inner bound
    that vanished, doubled or drifted is a different value and not a matching
    substring.
    """
    return [int(found) for found in re.findall(r"LIMIT (\d+)", sparql)]


def _producer_call(sparql: str) -> tuple[str, str]:
    """The producer a compiled unit calls, and the argument list it hands it.

    Read as a pair rather than asserted as two substrings, because a needle that
    appears SOMEWHERE in the text is not a needle the producer was handed: a
    build that rendered a request term into the wrong argument list, or into no
    argument list at all, leaves every substring of the text where it was.
    """
    found = re.search(r"<([^<>]+)> \( ([^()]*) \)", sparql)
    assert found is not None, f"a compiled unit calls one producer: {sparql!r}"
    return found.group(1), found.group(2)


def _ranking(answer: dict[str, Any]) -> list[tuple[str, str, tuple[Any, ...]]]:
    """An answer's rows, scores and provenance, as one comparable value."""
    return [
        (
            row["entity"],
            row["score"],
            tuple(
                (c["stratum"], c["rank"], c["contribution"])
                for c in row["contributions"]
            ),
        )
        for row in answer["rows"]
    ]


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


def test_an_answer_names_the_evidence_the_indexes_attested() -> None:
    """The third identity, and the attestations it is the content identity of.

    An answer carries three identities, and this is the one the other two cannot
    stand in for: ``plan_id`` pins the question, ``profile_id`` pins the law, and
    ``evidence_id`` pins the index generations that answered. Rebuilding an index
    moves none of the fields a caller can otherwise see — not the dataset it
    passed, not the request, not the registry fingerprint — so without this
    there is no way to tell two answers assembled from different index states
    apart. Two runs over the same corpus are the same evidence, and say so.

    The shipped text producer attests the content fingerprint of the index that
    answered: a digest of the configuration, the ranking law, the analyzer's
    Unicode tables, the documents, the dictionary and every posting. It is
    derived from content and never from a clock or a counter, so two runs over
    one corpus agree on it and a corpus with one more document does not.
    """
    kwargs: dict[str, Any] = {
        "text_producers": NOTE_ONLY,
        "weights": {NOTE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": 10,
    }
    first = retrieval.search(DATA, [_lexical("quick fox", NOTE)], **kwargs)
    second = retrieval.search(DATA, [_lexical("quick fox", NOTE)], **kwargs)

    attestation = first["attestations"][NOTE_STRATUM]
    assert set(attestation) == {"generation", "incomplete"}
    generation = attestation["generation"]
    assert isinstance(generation, str) and generation, (
        "the shipped text producer names the generation of the index that "
        "answered, verbatim; an absence here would make two answers over two "
        "index states indistinguishable"
    )
    assert len(generation) == 64 and all(c in "0123456789abcdef" for c in generation), (
        f"a 32-byte content digest in lowercase hex, got {generation!r}"
    )
    assert first["attestations"] == second["attestations"], (
        "the same index in the same state attests the same thing twice"
    )

    # Rebuilt with one more document: a row the earlier index could not return,
    # and a corpus size every surviving row is scored against. The request, the
    # producers, the weights and the law are identical, so the only thing that
    # changed is the index — and the generation moves with it.
    grown = retrieval.search(
        DATA + f'<{EX}d> <{NOTE}> "another quick fox" .\n',
        [_lexical("quick fox", NOTE)],
        **kwargs,
    )
    reached = {row["entity"] for row in grown["rows"]}
    assert f"<{EX}d>" in reached and reached != {
        row["entity"] for row in first["rows"]
    }, "the added document really is retrievable, so the returnable rows changed"
    grown_generation = grown["attestations"][NOTE_STRATUM]["generation"]
    assert grown_generation != generation, (
        "an added document changes which rows can be returned, so it must "
        "change the generation the producer attests"
    )

    assert len(first["evidence_id"]) == 64, "rendered like its two sibling ids"
    assert first["evidence_id"] == second["evidence_id"], (
        "two runs over one index state carry one evidence id"
    )
    assert grown["evidence_id"] != first["evidence_id"], (
        "and two answers from two index states do not"
    )
    # And it is its own identity, not a restatement of the other two: the answer
    # carries all three, and a reader compares the triple.
    assert first["plan_id"] != first["evidence_id"]
    assert first["profile_id"] != first["evidence_id"]


def test_an_attestation_has_two_independently_absent_axes() -> None:
    """Which generation answered and whether it was whole are separate facts.

    They are two keys rather than one fused flag because they are separately
    knowable and separately absent: a generation without a service level says
    which index answered but not whether it was all there, and a service level
    without a generation says an index was short without saying which one to
    rebuild. Each is ``None`` on its own when its producer said nothing.

    ``None`` under ``"incomplete"`` is the one that must never be read as good
    news. There is no "whole" value for it to be the opposite of — a producer
    stopped at the engine's row ceiling never looked at the rows it was licensed
    to skip, so it could not certify wholeness even if it were asked — and the
    seam therefore asks only the narrower question that has an honest answer on
    every path: was your index NOT whole?
    """
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
    assert set(answer["attestations"]) == {NOTE_STRATUM, TITLE_STRATUM}, (
        "every stream fusion was handed attests, and nothing else is keyed"
    )
    for stratum, attestation in answer["attestations"].items():
        assert set(attestation) == {"generation", "incomplete"}, stratum
        for axis in ("generation", "incomplete"):
            value = attestation[axis]
            assert value is None or (isinstance(value, str) and value), (
                f"{stratum}: {axis} is a verbatim string or an absence"
            )

    # Exactness is derived from exactly those attestations, and no producer here
    # declared a short index — which is the narrow true thing, not a certificate
    # that every index was whole.
    assert answer["exactness"] == {
        "exact": True,
        "deficit": [],
        "inflation": [],
        "unbounded": [],
    }


# ── what only the HOST can attest about the index behind a producer ──────────
#
# The relation this surface builds indexes the document it was handed, so it can
# attest which index answered — the content digest of that index — and nothing
# else. Whether the corpus that document was assembled from was WHOLE is a fact
# that never crosses this boundary in any other value: a corpus read out of a
# search index mid-rebuild is the same document as one read out of a whole index.
# Only the host knows, so a `text_producers` value may carry one trailing
# `(generation, incompleteness)` attestation, in the same shape and with the same
# refusals the SPARQL lane's relation declarations use.
#
# It is the FIFTH position, after an explicitly written `domains`, because a
# four-element tail is already a domains list: `("a", "b")` is a well-formed
# two-tag restriction and a well-formed attestation at once, and guessing which
# the host meant would report one back as the other.

#: The host's own name for the index version that produced a producer's rows.
INDEX_GENERATION = "notes-index-7"

#: The host's own reason its index was not whole, verbatim — a shard name and a
#: phase, because that is what an operator can act on and ``True`` is not.
REBUILDING = "shard 3 of 4 is still rebuilding"

#: The fusion law and the request the attestation cases below all share, so the
#: only thing that differs between any two of them is what a producer attested.
ATTESTED_COMMON: dict[str, Any] = {
    "weights": {NOTE_STRATUM: retrieval.SCALE, TITLE_STRATUM: retrieval.SCALE},
    "statistics": STATISTICS,
    "k": 60,
    "decay": TRUNCATED,
    "top_k": 10,
}

#: The request every attestation case runs, reaching both producers.
ATTESTED_REQUEST = [_lexical("quick fox", NOTE), _lexical("quick", TITLE)]


def _note_attesting(
    attestation: tuple[str | None, str | None],
) -> dict[str, tuple[Any, ...]]:
    """``BOTH``, with only the note producer carrying the host's attestation.

    The five-element spelling writes its ``domains`` position explicitly as
    ``None`` — the unrestricted promise, which is the same declaration the
    three-element title producer beside it makes — because the attestation is
    the fifth position. Only one of the two attests, so every assertion below is
    about a per-stratum fact rather than about a flag the answer carries once,
    and the two widths in one dict are the proof that a producer which declares
    nothing keeps its own reading.
    """
    return {
        NOTE_PRODUCER: (NOTE_STRATUM, NOTE, "any", None, attestation),
        TITLE_PRODUCER: (TITLE_STRATUM, TITLE, "any"),
    }


def test_an_attested_incompleteness_makes_that_stratums_scores_lower_bounds() -> None:
    """The host says its index was short, and the answer says so all the way down.

    This is the whole point of the position: without it no producer this surface
    registers can ever say it was short, so ``"exact"`` could only ever be
    ``True`` and the ``"incomplete"`` key could only ever be ``None`` — a
    structurally present receipt with no reachable content. With it, the reason
    the host wrote arrives verbatim under the stratum that declared it, the
    exactness derived from it names that stratum and no other, and the evidence
    identity moves because the evidence did.

    What does NOT change is the ranking. An attestation labels the bag; it never
    reaches the rows, the scores or the provenance, and the run below is compared
    against the identical one whose producers said nothing.
    """
    silent = retrieval.search(
        DATA, ATTESTED_REQUEST, text_producers=BOTH, **ATTESTED_COMMON
    )
    short = retrieval.search(
        DATA,
        ATTESTED_REQUEST,
        text_producers=_note_attesting((None, REBUILDING)),
        **ATTESTED_COMMON,
    )

    assert short["attestations"][NOTE_STRATUM]["incomplete"] == REBUILDING, (
        "recorded verbatim: an operator acts on the shard and the phase, and "
        "nothing here parses or summarises either"
    )
    assert short["exactness"] == {
        "exact": False,
        "deficit": [NOTE_STRATUM],
        "inflation": [NOTE_STRATUM],
        "unbounded": [],
    }, (
        "every score in this answer is an ESTIMATE rather than a value, and the "
        "error runs both ways: a row the short index never named is summed too "
        "low, and every row behind it moved up a rank and is summed too high. "
        "The lists name which index to rebuild, on each side"
    )

    # Per stratum, not per answer: the producer that said nothing still says
    # nothing, and its silence is not upgraded to a shortfall by its neighbour's.
    assert short["attestations"][TITLE_STRATUM]["incomplete"] is None
    for side in ("deficit", "inflation"):
        assert NOTE_STRATUM in short["exactness"][side], side
        assert TITLE_STRATUM not in short["exactness"][side], side

    # The rows are untouched — a short answer is still a real answer in this
    # fusion's own certified order — and so is the generation the relation
    # attests, because an axis the host left silent delegates to the relation.
    assert _ranking(short) == _ranking(silent), (
        "an attestation labels the answer and never changes it"
    )
    assert (
        short["attestations"][NOTE_STRATUM]["generation"]
        == silent["attestations"][NOTE_STRATUM]["generation"]
    ), "declaring an incompleteness does not cost the index its own digest"

    # And the third identity moved, because what the indexes attested moved —
    # while the law it was fused under, which an attestation says nothing about,
    # is the same content-addressed law it always was.
    assert short["evidence_id"] != silent["evidence_id"]
    assert short["profile_id"] == silent["profile_id"]


def test_a_declared_generation_is_reported_and_is_not_a_shortfall() -> None:
    """An absence is not a shortfall, and naming a version is not declaring one.

    The two axes are independent, so a host that knows which version of its index
    answered but has no reason to think it was short declares exactly that, and
    the answer stays EXACT. Reading a named generation as a shortfall would be
    the over-refusal mirror of the silent drop: nothing is wrong, and an answer
    that called itself a lower bound would send an operator to rebuild an index
    that was fine.

    The generation the host names REPLACES the content digest the shipped
    relation would otherwise attest, because exactly one generation is pinned per
    invocation. That is asserted here rather than left to be discovered: a host
    choosing its own spelling is choosing to identify the index by it.
    """
    silent = retrieval.search(
        DATA, ATTESTED_REQUEST, text_producers=BOTH, **ATTESTED_COMMON
    )
    named = retrieval.search(
        DATA,
        ATTESTED_REQUEST,
        text_producers=_note_attesting((INDEX_GENERATION, None)),
        **ATTESTED_COMMON,
    )

    assert named["attestations"][NOTE_STRATUM] == {
        "generation": INDEX_GENERATION,
        "incomplete": None,
    }, "the host's own spelling, verbatim, and no shortfall declared beside it"
    assert named["exactness"] == {
        "exact": True,
        "deficit": [],
        "inflation": [],
        "unbounded": [],
    }, "naming which index answered says nothing about whether it was short"
    assert _ranking(named) == _ranking(silent)

    assert named["attestations"][NOTE_STRATUM]["generation"] != (
        silent["attestations"][NOTE_STRATUM]["generation"]
    ), "one generation is pinned per invocation, so the host's replaces the digest"
    assert (
        named["attestations"][TITLE_STRATUM]
        == silent["attestations"][TITLE_STRATUM]
    ), "the producer that declared nothing attests exactly what it always did"


def test_a_producer_that_declares_no_attestation_is_unchanged() -> None:
    """The valid neighbour: three widths, one answer, down to the evidence id.

    A position that changed what a spec without it means would be a silent
    migration of every host already using this surface. So the three-element
    spelling, the four-element one with an explicit ``None`` domains, and the
    five-element one attesting ``(None, None)`` must all be the same declaration
    — silence — and must produce the same answer, the same attestations and the
    same content identity for them.

    ``(None, None)`` is the case that makes silence genuinely silence rather than
    a third declaration with a meaning of its own.
    """
    omitted = retrieval.search(
        DATA, ATTESTED_REQUEST, text_producers=BOTH, **ATTESTED_COMMON
    )
    unrestricted = retrieval.search(
        DATA,
        ATTESTED_REQUEST,
        text_producers=_declared(
            (NOTE_PRODUCER, NOTE_STRATUM, NOTE, None),
            (TITLE_PRODUCER, TITLE_STRATUM, TITLE, None),
        ),
        **ATTESTED_COMMON,
    )
    silent = retrieval.search(
        DATA,
        ATTESTED_REQUEST,
        text_producers=_note_attesting((None, None)),
        **ATTESTED_COMMON,
    )

    for answer in (unrestricted, silent):
        assert _ranking(answer) == _ranking(omitted)
        assert answer["attestations"] == omitted["attestations"]
        assert answer["evidence_id"] == omitted["evidence_id"]
        assert answer["exactness"] == {
            "exact": True,
            "deficit": [],
            "inflation": [],
            "unbounded": [],
        }
        assert answer["domains"] == omitted["domains"]

    # And what a producer declares to the PLANNER is untouched by what it attests
    # about its index, which is why the position can be read by all three entry
    # points while only `search` reports it: an attestation names no arity, no
    # mode and no ranked order, so the registry's durable content fingerprint is
    # the same whether one is declared or not.
    def _fingerprint(producers: dict[str, Any]) -> str:
        planned = retrieval.plan(
            DATA, ATTESTED_REQUEST, text_producers=producers, statistics=STATISTICS
        ,
            top_k=PLAN_TOP_K,
)
        return planned["registry_content_fingerprint"]

    assert _fingerprint(_note_attesting((INDEX_GENERATION, REBUILDING))) == (
        _fingerprint(BOTH)
    )


def test_a_malformed_attestation_is_refused_and_its_neighbours_are_not() -> None:
    """Each refusal, paired with the well-formed declaration one line away from it.

    A refusal here is only evidence about what it excludes if the neighbouring
    valid case is executed too — otherwise a shape check that swept up a legal
    spelling would look exactly like correct strictness until a host wrote the
    declaration that should work and did not. So every case below is a pair.
    """
    well_formed = _note_attesting((INDEX_GENERATION, REBUILDING))

    def _search(producers: dict[str, Any]) -> dict[str, Any]:
        return retrieval.search(
            DATA, ATTESTED_REQUEST, text_producers=producers, **ATTESTED_COMMON
        )

    # The neighbour, first: the declaration all four refusals below are one
    # mistake away from really does answer, and answers with both axes.
    accepted = _search(well_formed)
    assert accepted["attestations"][NOTE_STRATUM] == {
        "generation": INDEX_GENERATION,
        "incomplete": REBUILDING,
    }
    assert accepted["exactness"] == {
        "exact": False,
        "deficit": [NOTE_STRATUM],
        "inflation": [NOTE_STRATUM],
        "unbounded": [],
    }

    # A member of the wrong type names the member, because both are recorded
    # verbatim and neither has a spelling this binding could coerce one into.
    with pytest.raises(TypeError, match="`generation` must be a str or None"):
        _search(_note_attesting((7, REBUILDING)))  # type: ignore[arg-type]
    with pytest.raises(TypeError, match="`incompleteness` must be a str or None"):
        _search(_note_attesting((INDEX_GENERATION, 7)))  # type: ignore[arg-type]

    # A bare string in the fifth position is NOT destructured into its own two
    # characters. `"ab"` extracts as a perfectly well-formed two-member sequence,
    # so accepting it would have reported `generation="a"` back to an operator as
    # though the host had said it. It is a shape error, like any other value in a
    # position this spec does not have.
    with pytest.raises(TypeError, match="an attestation is the fifth position"):
        _search({NOTE_PRODUCER: (NOTE_STRATUM, NOTE, "any", None, "ab")})

    # As is a sequence of the wrong width — three axes is not this declaration.
    with pytest.raises(TypeError, match="an attestation is the fifth position"):
        _search(
            {
                NOTE_PRODUCER: (
                    NOTE_STRATUM,
                    NOTE,
                    "any",
                    None,
                    (INDEX_GENERATION, REBUILDING, "extra"),
                )
            }
        )

    # And a value of the wrong number of positions altogether.
    with pytest.raises(TypeError, match="an attestation is the fifth position"):
        _search({NOTE_PRODUCER: (NOTE_STRATUM, NOTE)})

    # The fourth position is `domains` and stays `domains`, even when what was
    # written there would have been a well-formed attestation: the two are
    # genuinely ambiguous at that width, and this binding refuses to guess. A
    # host that wrote one there is told which list it landed in.
    with pytest.raises(ValueError, match="domain tag"):
        _search(
            {NOTE_PRODUCER: (NOTE_STRATUM, NOTE, "any", (INDEX_GENERATION, REBUILDING))}
        )

    # None of which disturbed the neighbouring producer or the next call: the
    # same map, once more, with the same answer.
    assert _ranking(_search(well_formed)) == _ranking(accepted)


def test_a_terminal_status_carries_one_of_the_three_spellings_reachable_here() -> None:
    """Three of the seven endings, executed — and the suite says which three those are.

    ``"exhausted"`` says the producer emitted every row its SEARCH produced --
    not that everything matching was returned, which is what the stratum's
    ``"fidelities"`` entry says beside it. The other six
    each name who stopped the read: ``"depth_reached"`` the producer stopping at
    the depth the plan gave it, ``"row_bound_reached"`` the producer stopping at
    the row count it declared it can serve per invocation — a read whose ending
    nobody could observe, because the row past it could not be asked for —
    ``"ceiling_reached"`` a contribution bound (a
    fusion the caller's ``top_k`` stopped writes this over the streams it
    stopped), ``"supplied_query_ended"`` a unit running a query text the host
    wrote rather than one the layer rendered — whose own internal bound the layer
    cannot see, so what it left unread was not observable either —
    ``"execution_failed"`` a producer that could not run at all, and
    ``"terms_rejected"`` one that declined the terms it was handed. Reading any
    of the other six as "that was all of it" is the mistake the seven spellings
    exist to prevent.

    What this test executes is ``"exhausted"``, ``"depth_reached"`` and
    ``"ceiling_reached"``, and it claims nothing about the other four: they are
    unreachable through this binding, not untested by oversight, and
    ``py_retrieval.rs``'s header records why beside the refusals in the same
    position. ``"row_bound_reached"`` needs a self-bounding producer — one whose
    declaration places the depth as an argument the producer reads — and the one
    relation this surface registers places none, so every stratum a Python host
    can configure is bounded by the unit's own emitted ``LIMIT``.
    ``"supplied_query_ended"`` needs a unit carrying a query text a caller wrote,
    and this surface compiles every unit it runs and accepts no bundle from a
    caller. ``"terms_rejected"`` is a receipt a producer writes for itself, and
    ``"execution_failed"`` needs a unit whose text could not be prepared or run;
    both belong to a host driving the Rust surface with a bundle of its own, and
    none of the four can come out of a call that compiles its own units from a
    text index.

    The spellings are not asserted against the call's documentation. A docstring
    that contains the word proves nothing about which string the mapping emits,
    and would have passed with the mapping deleted. What is checked below is the
    emitted value and the payload each spelling owes.
    """
    # ``"exhausted"``: a corpus small enough that this query drains it, so the
    # producer's search really did run out of rows.
    exhausted = retrieval.search(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        weights={NOTE_STRATUM: retrieval.SCALE},
        statistics=STATISTICS,
        k=60,
        decay=TRUNCATED,
        top_k=10,
    )["statuses"][NOTE_STRATUM]
    assert exhausted["status"] == "exhausted"
    assert exhausted["rows_emitted"] >= 1

    # A measured cardinality bounds the planned depth, and the producer stops
    # there with rows still beneath it — which is NOT a claim the rows ran out.
    corpus = _disjoint_corpus()
    bounded = retrieval.search(
        corpus,
        [_lexical("quick", NOTE)],
        text_producers=NOTE_ONLY,
        weights={NOTE_STRATUM: retrieval.SCALE},
        statistics={**STATISTICS, "cardinality": {NOTE_STRATUM: 2}},
        k=60,
        decay=TRUNCATED,
        top_k=10,
    )["statuses"][NOTE_STRATUM]
    assert bounded["status"] == "depth_reached"
    assert bounded["rank"] == 2, "ranks one and two were read; nothing below was"

    stopped = retrieval.search(
        corpus,
        [_lexical("quick", NOTE), _lexical("quick", TITLE)],
        text_producers=_declared(
            (NOTE_PRODUCER, NOTE_STRATUM, NOTE, [NOTE_DOMAIN]),
            (TITLE_PRODUCER, TITLE_STRATUM, TITLE, [TITLE_DOMAIN]),
        ),
        weights={NOTE_STRATUM: retrieval.SCALE, TITLE_STRATUM: retrieval.SCALE},
        statistics=STATISTICS,
        k=60,
        decay=TRUNCATED,
        top_k=3,
    )["statuses"]
    assert {entry["status"] for entry in stopped.values()} == {"ceiling_reached"}
    for stratum, entry in stopped.items():
        from decimal import Decimal

        assert Decimal(entry["bound"]) > 0, stratum

    # Whatever a status says, it says it with one of the seven spellings and with
    # the payload that spelling owes — no aggregate flag, and no eighth word.
    for entry in (exhausted, bounded, *stopped.values()):
        assert entry["status"] in STATUS_SPELLINGS, entry
        assert ("rows_emitted" in entry) == (entry["status"] == "exhausted"), (
            "only the ending that names no stopper carries a row count; every "
            "other spelling names its stopper instead"
        )

    # And the coverage this test does and does not have, stated rather than
    # implied: exactly three endings came out of the three calls above. A fourth
    # appearing is news — some registration this surface offers now reaches an
    # ending the module header says it cannot — and a third going missing is a
    # fixture that stopped exercising what it was written for.
    reached = {entry["status"] for entry in (exhausted, bounded, *stopped.values())}
    assert reached == {"exhausted", "depth_reached", "ceiling_reached"}, reached
    assert reached < STATUS_SPELLINGS, (
        "the seven spellings are the whole vocabulary, and this surface reaches "
        "strictly fewer than all of them"
    )


def test_a_declared_domain_changes_the_reading_and_not_the_answer() -> None:
    """The Python-visible proof that a declaration buys a read, not an answer.

    Fusion certifies a candidate only when every stream that COULD still name it
    has. With nothing declared, that is every open stream — so over two strata
    whose candidate sets do not overlap, a top-three drains both, because the
    confirmation it waits for is never coming. Telling fusion which blocks each
    producer draws from lets it skip the streams that provably cannot name a
    candidate, and only those: the finality test does not get weaker, its
    quantifier gets smaller.

    So the two runs below must agree on every row, every score and every
    contribution, and disagree only on how far they read to get there. The
    declaration is reported back on the answer, because it is an input the
    answer cannot otherwise be audited against: it decides which streams fusion
    was allowed to skip when it certified a row.
    """
    corpus = _disjoint_corpus()
    request = [_lexical("quick", NOTE), _lexical("quick", TITLE)]
    common: dict[str, Any] = {
        "weights": {NOTE_STRATUM: retrieval.SCALE, TITLE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": 3,
    }
    undeclared = retrieval.search(corpus, request, text_producers=BOTH, **common)
    declared = retrieval.search(
        corpus,
        request,
        text_producers=_declared(
            (NOTE_PRODUCER, NOTE_STRATUM, NOTE, [NOTE_DOMAIN]),
            (TITLE_PRODUCER, TITLE_STRATUM, TITLE, [TITLE_DOMAIN]),
        ),
        **common,
    )

    assert _ranking(declared) == _ranking(undeclared), (
        "a declaration licenses a shorter read, and never a different answer"
    )
    assert len(declared["rows"]) == 3

    # The reading is where they differ, and it differs in the direction the
    # declaration promised.
    assert any(
        entry["status"] == "ceiling_reached" for entry in declared["statuses"].values()
    ), "the declared run stopped at a bound instead of draining its streams"
    assert all(
        entry["status"] == "exhausted" for entry in undeclared["statuses"].values()
    ), "with nothing declared there is no stream fusion may skip, so both drain"
    for stratum in (NOTE_STRATUM, TITLE_STRATUM):
        assert (
            declared["observed_resolution"][stratum]["ranks_pulled"]
            < undeclared["observed_resolution"][stratum]["ranks_pulled"]
        ), stratum

    # And the answer says whose word it was certified on.
    assert declared["domains"] == {
        NOTE_STRATUM: [NOTE_DOMAIN],
        TITLE_STRATUM: [TITLE_DOMAIN],
    }
    assert undeclared["domains"] == {NOTE_STRATUM: None, TITLE_STRATUM: None}, (
        "None is the widest promise — this producer may name anything — and it "
        "is what every answer this engine produced before domains carried"
    )

    # The shorter read is still an exact one: nothing here attested a short
    # index, so the scores are sums of every contribution that was due.
    assert declared["exactness"] == {
        "exact": True,
        "deficit": [],
        "inflation": [],
        "unbounded": [],
    }


def test_a_bounded_request_reads_a_flat_prefix_of_each_disjoint_stratum() -> None:
    """The materialized read is flat in corpus size, not merely walked less far.

    The defect this test exists for: a declaration used to buy a shorter walk of a
    stream that had already been materialized in full. Fusion stopped early and
    said so, and the compiled unit was still ``LIMIT <corpus>`` — so the elapsed
    time of a top-five answer grew with the corpus while the instrument reported a
    handful of ranks pulled. The bound arrived at ``fuse``, four stages after the
    depth was chosen.

    The bound is a planning input now, so the same ``top_k`` over a corpus and over
    a corpus four times its size compiles to the SAME per-stratum ``LIMIT`` and
    records the SAME depth. Both numbers are asserted, because either alone would
    leave the defect expressible: a narrowed ``LIMIT`` beside an unnarrowed depth
    is a plan recording a read nobody took, and an unnarrowed ``LIMIT`` beside a
    narrowed depth is the corpus still being materialized.

    And the answer does not move. At both sizes the bounded run's rows, scores and
    provenance are identical to the draining run's — the run that reads every row
    of every stream because nothing was declared. That is the soundness gate: a
    depth rule that fired too eagerly would show up here as a truncated ranked
    list, which is the mirror of the defect above and worse than it.
    """
    top_k = 5
    disjoint = _declared(
        (NOTE_PRODUCER, NOTE_STRATUM, NOTE, [NOTE_DOMAIN]),
        (TITLE_PRODUCER, TITLE_STRATUM, TITLE, [TITLE_DOMAIN]),
    )
    request = [_lexical("quick", NOTE), _lexical("quick", TITLE)]
    law: dict[str, Any] = {
        "weights": {NOTE_STRATUM: retrieval.SCALE, TITLE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": top_k,
    }

    def compiled_bounds(rows: int) -> dict[str, tuple[int, int]]:
        """Each stratum's recorded depth and the ``LIMIT`` its unit carries."""
        compiled = retrieval.compile(
            _disjoint_corpus(rows),
            request,
            text_producers=disjoint,
            statistics=STATISTICS,
            top_k=top_k,
        )
        return {
            unit["stratum"]: (unit["depth"], _emitted_limit(unit["sparql"]))
            for unit in compiled["units"]
        }

    small, large = compiled_bounds(400), compiled_bounds(1600)
    assert small == large, (
        "the materialized read is flat in corpus size, so neither the recorded "
        f"depth nor the emitted LIMIT may move with it: {small} vs {large}"
    )
    assert small == {
        NOTE_STRATUM: (top_k, top_k + 1),
        TITLE_STRATUM: (top_k, top_k + 1),
    }, (
        "each stratum is read to the bound and no deeper, with the probe row one "
        f"past it exactly as at any other depth: {small}"
    )

    # The answer is the same answer, at both sizes, as the run that drains
    # everything. Nothing declared means no stream fusion may skip, so the
    # undeclared run is the oracle here rather than a second engine.
    for rows in (400, 1600):
        corpus = _disjoint_corpus(rows)
        bounded = retrieval.search(corpus, request, text_producers=disjoint, **law)
        draining = retrieval.search(corpus, request, text_producers=BOTH, **law)
        assert _ranking(bounded) == _ranking(draining), (
            f"over {rows} rows per predicate a declaration licensed a shorter "
            "read, and it must never license a different answer"
        )
        assert len(bounded["rows"]) == top_k

        # And the reading really was flat, measured on the rows themselves: the
        # draining run pulled its whole corpus per stratum, the bounded one pulled
        # at most what its depth allowed.
        for stratum in (NOTE_STRATUM, TITLE_STRATUM):
            assert draining["observed_resolution"][stratum]["ranks_pulled"] == rows, (
                "the undeclared run has no stream it may skip, so it reads to the "
                "end of both"
            )
            assert bounded["observed_resolution"][stratum]["ranks_pulled"] <= top_k, (
                stratum
            )


def test_an_unrestricted_or_overlapping_stratum_keeps_the_declared_depth() -> None:
    """The neighbours the prefix rule must NOT fire on, and they are not refused.

    The proof the prefix rests on needs one premise: each candidate has at most
    one naming stratum, so its fused score is a single weighted contribution that
    falls with rank. The producers' own declarations are what supply that premise,
    and two shapes supply none of it — a stratum that says it may name anything,
    and two strata that say they draw from the same block, where a candidate's
    score is a SUM across both.

    Neither is refused and neither answers differently. What they keep is the
    depth, which is the declared-or-measured bound and exactly the depth every such
    plan has always carried. Reading more than the answer needs is the safe
    direction; reading less would truncate a ranked list with nothing saying so.
    """
    top_k = 5
    rows = 40
    corpus = _disjoint_corpus(rows)
    request = [_lexical("quick", NOTE), _lexical("quick", TITLE)]

    def depths(producers: dict[str, tuple[Any, ...]]) -> dict[str, int]:
        compiled = retrieval.compile(
            corpus, request, text_producers=producers, statistics=STATISTICS, top_k=top_k
        )
        return {unit["stratum"]: unit["depth"] for unit in compiled["units"]}

    # The declared bound, read off the shape that licenses no prefix at all: both
    # producers unrestricted, which is what every producer said before domains
    # carried.
    declared = depths(BOTH)
    assert declared == {NOTE_STRATUM: rows, TITLE_STRATUM: rows}, (
        "each producer's own index bounds it at the rows it holds"
    )

    # One unrestricted stratum beside one restricted one: the unrestricted one has
    # made no promise about which candidates it will not name, so there is no
    # premise and the bound stands for BOTH.
    mixed = _declared(
        (NOTE_PRODUCER, NOTE_STRATUM, NOTE, [NOTE_DOMAIN]),
        (TITLE_PRODUCER, TITLE_STRATUM, TITLE, None),
    )
    assert depths(mixed) == declared, (
        "an unrestricted stratum licenses no prefix, for itself or for its siblings"
    )

    # Two producers declaring the SAME block. Every candidate either could name is
    # in a block both declared, so scores sum across strata and the merge argument
    # has nothing to stand on.
    overlapping = _declared(
        (NOTE_PRODUCER, NOTE_STRATUM, NOTE, [NOTE_DOMAIN]),
        (TITLE_PRODUCER, TITLE_STRATUM, TITLE, [NOTE_DOMAIN]),
    )
    assert depths(overlapping) == declared, (
        "declarations that meet license no prefix"
    )

    # And every one of them answers, identically. The depth is the only thing that
    # moved anywhere in this family.
    law: dict[str, Any] = {
        "weights": {NOTE_STRATUM: retrieval.SCALE, TITLE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": top_k,
    }
    widest = retrieval.search(corpus, request, text_producers=BOTH, **law)
    for producers in (mixed, overlapping):
        assert _ranking(
            retrieval.search(corpus, request, text_producers=producers, **law)
        ) == _ranking(widest), (
            "a declaration changes how much is read, never what is returned"
        )


def test_an_empty_domain_declaration_is_refused_and_its_neighbours_are_not() -> None:
    """An empty restriction is a promise to name nothing, and is refused by name.

    It sits one line away from the unrestricted declaration and means the
    opposite of it, so reading it as "no restriction" would be the silent repair
    that registers a producer whose every row contradicts its own declaration.
    The refusal names the producer that declared it, because a ``text_producers``
    map holds several and the message has to say which one to fix.

    Both neighbours are valid and both are checked here, which is the point of
    the test: a refusal that also swept up ``None`` or a one-tag list would be
    the mirror failure — strictness that reads as correctness until a host
    writes the declaration that should work and does not.
    """
    common: dict[str, Any] = {
        "weights": {NOTE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": 10,
    }
    request = [_lexical("quick fox", NOTE)]

    with pytest.raises(ValueError, match=re.escape(NOTE_PRODUCER)) as refused:
        retrieval.search(
            DATA,
            request,
            text_producers=_declared((NOTE_PRODUCER, NOTE_STRATUM, NOTE, [])),
            **common,
        )
    assert "empty list" in str(refused.value)

    unrestricted = retrieval.search(
        DATA,
        request,
        text_producers=_declared((NOTE_PRODUCER, NOTE_STRATUM, NOTE, None)),
        **common,
    )
    assert unrestricted["rows"], "None is a declaration, and a perfectly good one"
    assert unrestricted["domains"] == {NOTE_STRATUM: None}

    restricted = retrieval.search(
        DATA,
        request,
        text_producers=_declared((NOTE_PRODUCER, NOTE_STRATUM, NOTE, [NOTE_DOMAIN])),
        **common,
    )
    assert _ranking(restricted) == _ranking(unrestricted)
    assert restricted["domains"] == {NOTE_STRATUM: [NOTE_DOMAIN]}

    # The three-element spelling is the same declaration as an explicit None, so
    # a host that never heard of domains keeps exactly the answer it had.
    omitted = retrieval.search(DATA, request, text_producers=NOTE_ONLY, **common)
    assert _ranking(omitted) == _ranking(unrestricted)
    assert omitted["domains"] == unrestricted["domains"]

    # A tag that is not an IRI is refused where it is written, naming the tag.
    with pytest.raises(ValueError, match="domain tag"):
        retrieval.search(
            DATA,
            request,
            text_producers=_declared(
                (NOTE_PRODUCER, NOTE_STRATUM, NOTE, ["not an iri"])
            ),
            **common,
        )


def test_two_producers_under_one_stratum_are_refused_by_name_and_two_strata_are_not() -> None:
    """One stratum carries one producer, and the refusal is a ``ValueError``.

    The registry underneath enforces this with a panic, which is the right shape
    for Rust code assembling a registry and the wrong one at this boundary: a
    panic arrives in Python as ``PanicException``, which derives from
    ``BaseException`` and slips past a host's ``except Exception``. Every other
    misconfiguration on this surface raises ``ValueError`` where it is supplied,
    naming what to fix, and so does this one — before the registry is touched.

    The neighbour is the configuration a host actually wants: the same two
    producers under two strata, which fuse.
    """
    common: dict[str, Any] = {
        "weights": {NOTE_STRATUM: retrieval.SCALE, TITLE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": 10,
    }
    request = [_lexical("quick", NOTE), _lexical("quick", TITLE)]

    # Written title-first so the names reported are visibly a function of the
    # declarations rather than of the dict's insertion order.
    with pytest.raises(ValueError) as refused:
        retrieval.search(
            DATA,
            request,
            text_producers=_producers(
                (TITLE_PRODUCER, NOTE_STRATUM, TITLE),
                (NOTE_PRODUCER, NOTE_STRATUM, NOTE),
            ),
            **common,
        )
    message = str(refused.value)
    assert f"<{NOTE_PRODUCER}> and <{TITLE_PRODUCER}>" in message, message
    assert f"claim stratum <{NOTE_STRATUM}>" in message, message
    assert "ONE producer" in message and "two strata" in message, message

    fused = retrieval.search(
        DATA,
        request,
        text_producers=_producers(
            (TITLE_PRODUCER, TITLE_STRATUM, TITLE),
            (NOTE_PRODUCER, NOTE_STRATUM, NOTE),
        ),
        **common,
    )
    assert fused["rows"], "two producers under two strata is the configuration that fuses"
    assert set(fused["statuses"]) == {NOTE_STRATUM, TITLE_STRATUM}


def test_a_false_domain_declaration_is_refused_and_names_who_collided() -> None:
    """A declaration is a promise, and the rows are checked against it.

    A domain tag names a block of a PARTITION of the candidate universe, so a
    candidate lies in exactly one block: two producers whose declarations put
    one entity in blocks with nothing in common cannot both be telling the
    truth about it. The consumer cannot know which of the two is wrong, so it
    reports the contradiction rather than picking a side — and it cannot widen
    the declaration silently instead, because the declaration has already been
    USED: rows were certified early on the strength of it, and merging the late
    contribution would hand back a score its own provenance contradicts.

    The message names all three parties, and each answers a different question:
    the item says what, the stratum says who broke its promise, and the stratum
    that had already named the candidate says against whose declaration.

    This is NOT a refusal of producers that overlap. The corpus here has one
    entity in both indexed fields, and the neighbouring declarations that say so
    — one shared tag, or none at all — fuse it into one row carrying both
    contributions, which the second half of this test holds.
    """
    common: dict[str, Any] = {
        "weights": {NOTE_STRATUM: retrieval.SCALE, TITLE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": 10,
    }
    request = [_lexical("quick", NOTE), _lexical("quick", TITLE)]

    with pytest.raises(ValueError) as refused:
        retrieval.search(
            DATA,
            request,
            text_producers=_declared(
                (NOTE_PRODUCER, NOTE_STRATUM, NOTE, [NOTE_DOMAIN]),
                (TITLE_PRODUCER, TITLE_STRATUM, TITLE, [TITLE_DOMAIN]),
            ),
            **common,
        )
    message = str(refused.value)
    assert "declared candidate domains cannot reach" in message
    assert NOTE_STRATUM in message and TITLE_STRATUM in message, (
        "the refusal names the stratum that broke its promise AND the one whose "
        "declaration it collided with; neither half is actionable alone"
    )
    assert f"<{EX}a>" in message, "and the candidate the two disagree about"

    # The one-line fix, and the proof it is not over-refusal: producers that
    # really do rank the same entities declare a tag they share, and fuse.
    shared = retrieval.search(
        DATA,
        request,
        text_producers=_declared(
            (NOTE_PRODUCER, NOTE_STRATUM, NOTE, [NOTE_DOMAIN]),
            (TITLE_PRODUCER, TITLE_STRATUM, TITLE, [NOTE_DOMAIN]),
        ),
        **common,
    )
    both = [row for row in shared["rows"] if len(row["contributions"]) == 2]
    assert both, "one entity holds both needles, and one row carries both ranks"

    # …as do the same two producers declaring nothing at all, to the same answer.
    assert _ranking(shared) == _ranking(
        retrieval.search(DATA, request, text_producers=BOTH, **common)
    )


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
    ,
        top_k=PLAN_TOP_K,
)
    second = retrieval.plan(
        DATA, request, text_producers=NOTE_ONLY, statistics=STATISTICS
    ,
        top_k=PLAN_TOP_K,
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
            top_k=PLAN_TOP_K,
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
    ,
        top_k=PLAN_TOP_K,
)
    bounded = retrieval.plan(
        DATA,
        request,
        text_producers=NOTE_ONLY,
        statistics={**STATISTICS, "cardinality": {NOTE_STRATUM: 1}},
            top_k=PLAN_TOP_K,
)
    assert bounded["stratum_depths"][NOTE_STRATUM] == 1
    assert (
        bounded["stratum_depths"][NOTE_STRATUM]
        <= unbounded["stratum_depths"][NOTE_STRATUM]
    )
    # The plan records what it assumed, so a replay against moved statistics is
    # a detectable condition rather than a silent replan. The stratum a depth is
    # derived for is recorded beside that depth, with everything it came from.
    assert unbounded["stratum_derivations"][NOTE_STRATUM]["cardinality"] is None, (
        "a provider that reported nothing is recorded as having reported nothing, "
        "not omitted: a subject the plan does not name is one a replay cannot check"
    )
    assert bounded["stratum_derivations"][NOTE_STRATUM]["cardinality"] == 1
    assert bounded["stratum_derivations"][NOTE_STRATUM]["cause"] == "cardinality"
    assert unbounded["stratum_derivations"][NOTE_STRATUM]["cause"] == "declaration"

    # The snapshot names every subject planning consulted: the request predicate,
    # which derives no depth and about which this host says nothing, AND the
    # stratum, whose row is the projection of the derivation above. The two rows
    # differ in the cardinality, so a swap of the two -- the mis-mapping this
    # assertion exists to catch -- fails on the number.
    assert bounded["statistics"]["entries"] == [
        {
            "subject": NOTE,
            "cardinality": None,
            "selectivity_ppm": None,
            "selectivity_terms": [],
        },
        {
            "subject": NOTE_STRATUM,
            "cardinality": 1,
            "selectivity_ppm": None,
            "selectivity_terms": [],
        },
    ]


def test_a_selectivity_only_statistic_is_recorded_on_the_plan() -> None:
    """A selectivity with no cardinality narrows the depth, so the plan records it.

    This is the whole of the defect it pins: the depth moved and the evidence did
    not, so a replay against moved statistics could derive a different depth with
    nothing in the plan to compare against. All three facts are asserted
    together, because each alone passes over a different bug -- the depth is what
    a plan already got right, and the record alone would pass for a build that
    recorded a statistic it never applied.
    """
    request = [_lexical("quick fox", NOTE)]
    selectivity_only = {**STATISTICS, "selectivity": {(NOTE_STRATUM, 0): 0}}

    planned = retrieval.plan(
        DATA,
        request,
        text_producers=NOTE_ONLY,
        statistics=selectivity_only,
        top_k=PLAN_TOP_K,
    )
    assert planned["stratum_depths"][NOTE_STRATUM] == 1, (
        "a selectivity of zero narrows the read, floored at one probing row so "
        "the producer is the thing that reports the emptiness"
    )

    derivation = planned["stratum_derivations"][NOTE_STRATUM]
    assert derivation["selectivity_ppm"] == 0, (
        "the statistic that narrowed the depth is the statistic recorded"
    )
    assert derivation["cardinality"] is None, (
        "absent is not zero; the provider reported no cardinality at all"
    )
    assert derivation["selectivity_terms"] == [0]
    assert derivation["cause"] == "floor"

    # And the plan's own snapshot names that stratum, so a caller asking which
    # statistics this was planned against is told about it without having to
    # know that a depth happened to be derived for it. The cardinality is `None`
    # -- absence, not the zero the selectivity beside it really is.
    entries = {entry["subject"]: entry for entry in planned["statistics"]["entries"]}
    assert entries[NOTE_STRATUM] == {
        "subject": NOTE_STRATUM,
        "cardinality": None,
        "selectivity_ppm": 0,
        "selectivity_terms": [0],
    }, (
        "a stratum consulted for a selectivity alone is named on the snapshot, "
        "with the measurement it gave and the absence it did not"
    )
    assert entries[NOTE_STRATUM]["cardinality"] is None, (
        "`None` and `0` are different facts, and this row carries one of each: "
        "the host measured a selectivity of zero and no cardinality at all"
    )
    # The neighbour, in the same snapshot: the request predicate the host is
    # equally silent about carries no selectivity either, so the `0` above is the
    # stratum's own measurement rather than a value smeared across every row.
    assert entries[NOTE] == {
        "subject": NOTE,
        "cardinality": None,
        "selectivity_ppm": None,
        "selectivity_terms": [],
    }

    compiled = retrieval.compile(
        DATA,
        request,
        text_producers=NOTE_ONLY,
        statistics=selectivity_only,
        top_k=PLAN_TOP_K,
    )
    unit = compiled["units"][0]
    assert unit["depth"] == 1
    # THE DEPTH-BEARING BOUNDS, both of them, read as values rather than as a
    # substring. The unit writes its bound twice -- once around the producer call
    # and once around the whole SELECT -- and the two are the same number by
    # construction: the compiler derives each from the same depth, so no fixture
    # can make them differ, and a host that reads a depth as an argument gets no
    # inner LIMIT at all rather than a different one. That is precisely why
    # `"LIMIT 2" in sparql` guarded nothing here: it is satisfied by the outer
    # bound alone, so the INNER one -- the bound that stops the producer's read,
    # which is the whole subject of this test -- could go missing under it.
    assert _emitted_limits(unit["sparql"]) == [2, 2], (
        "the planned depth of one, plus the one probe row, at BOTH bounds the "
        f"unit writes: {unit['sparql']}"
    )
    assert _emitted_limit(unit["sparql"]) == unit["depth"] + 1, (
        "one row past the planned depth, so a read this bound cuts is reported "
        f"as such rather than as an exhausted stratum: {unit['sparql']}"
    )

    # THE CONTROL, and the reason the numbers above are an observation. A
    # selectivity of half narrows the same three declared rows to two rather than
    # to one, so the same producer over the same corpus emits a DIFFERENT pair of
    # bounds. A build that had baked in a constant, or that emitted the
    # declaration instead of the planned depth, agrees with one of these two rows
    # and fails on the other.
    narrowed = retrieval.compile(
        DATA,
        request,
        text_producers=NOTE_ONLY,
        statistics={**STATISTICS, "selectivity": {(NOTE_STRATUM, 0): 500_000}},
        top_k=PLAN_TOP_K,
    )["units"][0]
    assert narrowed["depth"] == 2, (
        "half of the three rows this producer declares, rounded up, is two"
    )
    assert narrowed["declared_rows"] == 3, (
        "the declaration did not move between the two rows; the statistic did"
    )
    assert _emitted_limits(narrowed["sparql"]) == [3, 3], (
        f"a different planned depth is a different emitted bound: {narrowed['sparql']}"
    )


def test_a_subject_the_host_reports_but_planning_never_consults_is_absent() -> None:
    """A cardinality for a subject no depth is derived for stays out of the plan.

    The control has an oracle: the host is willing to speak about the unconsulted
    subject, so a build that recorded every subject it could reach rather than
    every subject it asked about would name it. The consulted subject is asserted
    present in the same test with a different value, so this cannot pass for a
    plan that recorded nothing.
    """
    unconsulted = f"{EX}stratum/nobody-asks"
    planned = retrieval.plan(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics={
            **STATISTICS,
            "cardinality": {NOTE_STRATUM: 3, unconsulted: 9999},
        },
        top_k=PLAN_TOP_K,
    )

    assert planned["stratum_derivations"][NOTE_STRATUM]["cardinality"] == 3, (
        "the consulted subject carries its own value, not the other one"
    )
    assert unconsulted not in planned["stratum_derivations"]
    assert all(
        entry["subject"] != unconsulted for entry in planned["statistics"]["entries"]
    ), "a subject planning never consulted is named nowhere"


def test_compile_emits_the_sparql_each_stratum_runs() -> None:
    """Admission's value is plain text a host can read, log, or run under an obligation.

    The text is runnable, and running it is not the same as reporting its rows:
    a unit is emitted exactly one row deeper than the plan reads, so a host that
    executes the text itself keeps at most ``"depth"`` rows. The unit carries
    that bound beside the text, which is the only reason the obligation is
    dischargeable here — ``"planned_resolution"`` is empty on a call that names
    no law, so it is no fallback source for the number.
    """
    compiled = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=STATISTICS,
            top_k=PLAN_TOP_K,
)
    units = compiled["units"]
    assert len(units) == 1
    assert units[0]["stratum"] == NOTE_STRATUM
    # Read as a call rather than as two substrings: a needle that appears
    # somewhere in the text is not a needle the producer was handed, and a term
    # rendered into no argument list at all would leave both substrings where
    # they were.
    producer, arguments = _producer_call(units[0]["sparql"])
    assert producer == NOTE_PRODUCER, "the unit calls the bound producer"
    assert '"quick fox"' in arguments, (
        f"the needle is rendered into that producer's own arguments: {arguments!r}"
    )
    depth = units[0]["depth"]
    assert isinstance(depth, int) and depth >= 1, (
        "the reportable bound travels with the text it bounds"
    )
    assert _emitted_limits(units[0]["sparql"]) == [depth + 1, depth + 1], (
        "the emitted bound is the depth plus the one probe row, at every depth "
        "and at BOTH bounds the unit writes: a text bounded at exactly the depth "
        "could not tell an exhausted producer from a read the depth cut short, "
        "and an inner bound nothing reads is an inner bound nothing checks"
    )
    assert compiled["plan_id"] == compiled["plan"]["plan_id"]
    assert compiled["planned_resolution"] == {}, (
        "resolution is measured against a fusion law, and this call named none"
    )


def test_a_compiled_unit_is_emitted_one_probe_row_deeper_than_it_reports() -> None:
    """The emitted ``LIMIT`` is not the reportable bound, and the unit says both.

    A text bounded at exactly the depth cannot tell the two endings apart that
    a consumer has to distinguish: a producer that ran out of rows, and a read
    the planned depth cut short. So wherever the producer's declared row bound
    leaves room, the unit is emitted one row deeper and that last row is a
    probe — a READ and never a value. Here a measured cardinality lowers the
    depth below what the producer declared, which is exactly the room the probe
    needs, so the emitted ``LIMIT`` is ``depth + 1`` while ``"depth"`` stays the
    number of rows a host may keep.

    That room is asserted rather than assumed — ``"declared_rows"`` strictly above
    ``"depth"`` — because it is the only thing separating this case from the one
    its sibling covers, where the depth has reached the declaration.
    """
    compiled = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics={**STATISTICS, "cardinality": {NOTE_STRATUM: 1}},
            top_k=PLAN_TOP_K,
)
    unit = compiled["units"][0]
    assert unit["depth"] == 1, "the host's measurement lowered the depth to one row"
    assert unit["declared_rows"] > unit["depth"], (
        "the room the probe row needs, read off the declaration the unit carries: "
        f"declared {unit['declared_rows']}, depth {unit['depth']}"
    )
    assert _emitted_limits(unit["sparql"]) == [unit["depth"] + 1, unit["depth"] + 1], (
        "the extra row is the probe, and a host that runs this text reports "
        "only the first `depth` rows -- at both bounds, since the inner one is "
        "what actually stops the producer's read"
    )


def test_a_probe_row_is_emitted_even_where_the_declaration_leaves_no_room() -> None:
    """The emitted ``LIMIT`` is one past the depth even here, and that is the point.

    With no statistic to narrow it, the depth is already the producer's whole
    declared row bound. A read that stopped exactly there could not tell a producer
    that ran out from one the bound cut, so it would report the one ending that
    names no stopper on the strength of a number nobody checked.
    The unit asks for one row more instead: if that row arrives the producer
    contradicted its own registration and the read is refused by name, and if it
    does not, the exhaustion is verified rather than believed.

    The extra row lives in the ``LIMIT`` only — a ceiling the evaluator applies to
    a cursor the producer never hears about, so probing costs nothing. A producer
    that reads a depth argument is never asked to exceed what it registered.

    The bound a host may report is still ``"depth"`` and never the text's
    ``LIMIT``, which is now always the larger of the two.

    The premise is the whole test, so it is asserted and not assumed: the
    ``"declared_rows"`` the unit carries must EQUAL its ``"depth"`` here. Without
    that, this configuration would be an ordinary one with room to spare — its
    sibling's case under a second name — and a bound wrongly capped at the
    declaration would sail through, because such a cap only ever bites where the
    depth has arrived at the declaration.
    """
    compiled = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=STATISTICS,
            top_k=PLAN_TOP_K,
)
    unit = compiled["units"][0]
    planned = retrieval.plan(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=STATISTICS,
            top_k=PLAN_TOP_K,
)
    assert unit["depth"] == planned["stratum_depths"][NOTE_STRATUM], (
        "the unit reports the depth the plan recorded, not a number of its own"
    )
    assert unit["declared_rows"] == unit["depth"], (
        "this test's premise: no statistic narrowed the depth, so it has risen to "
        "the producer's whole declared row bound and there is no room under it — "
        f"declared {unit['declared_rows']}, depth {unit['depth']}"
    )
    assert _emitted_limits(unit["sparql"]) == [unit["depth"] + 1, unit["depth"] + 1], (
        "the probe slot exists even on the declaration, so exhaustion is checked "
        "rather than assumed -- and it exists at the inner bound too, which is "
        "the one a producer's read actually runs into"
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
            top_k=PLAN_TOP_K,
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
    called, handed = _producer_call(compiled["units"][0]["sparql"])
    assert called == NOTE_PRODUCER
    assert '"quick fox"' in handed, (
        f"and the request term is still in the call's own arguments: {handed!r}"
    )

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
            top_k=PLAN_TOP_K,
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
            top_k=PLAN_TOP_K,
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
                    top_k=PLAN_TOP_K,
)
    with pytest.raises(ValueError, match="smoothing constant"):
        retrieval.compile(
            DATA, [_lexical("quick fox", NOTE)], k=60, decay=TRUNCATED, **common
        ,
            top_k=PLAN_TOP_K,
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
                    top_k=PLAN_TOP_K,
)

    # All three together are a law, and the same call answers.
    whole = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        weights={NOTE_STRATUM: retrieval.SCALE},
        k=60,
        decay=TRUNCATED,
        **common,
            top_k=PLAN_TOP_K,
)
    assert whole["planned_resolution"][NOTE_STRATUM]["fully_separated"] is True

    # Naming NONE of the three is not a refusal: it compiles without a law and
    # is simply told nothing about resolution.
    lawless = retrieval.compile(DATA, [_lexical("quick fox", NOTE)], **common, top_k=PLAN_TOP_K)
    assert lawless["planned_resolution"] == {}
    assert lawless["units"], "a call that names no law is still a compiled plan"


def test_an_answer_reports_planned_and_observed_resolution_apart() -> None:
    """Both altitudes are on the answer, under names that cannot be confused.

    ``planned_resolution`` is what the admission waist said the depths this plan
    records would cost; ``observed_resolution`` is what the rows this run really
    pulled did cost. They disagree here exactly as they should: a three-document
    corpus exhausts after a handful of ranks, while the plan reserved the depth
    the producer declared it could fill, and a depth nothing reached cost nothing.
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
            top_k=PLAN_TOP_K,
)
    assert compiled["planned_resolution"] == answer["planned_resolution"]


def test_no_producer_is_refused_by_name() -> None:
    """An empty producer set is a refusal; there is no default to fall back on."""
    with pytest.raises(ValueError, match="no ranked producers"):
        retrieval.plan(
            DATA, [_lexical("quick fox", NOTE)], text_producers={}, statistics=STATISTICS
        ,
            top_k=PLAN_TOP_K,
)


def test_statistics_must_name_their_own_revision() -> None:
    """The planner records the revision verbatim and invents none."""
    with pytest.raises(ValueError, match="revision"):
        retrieval.plan(
            DATA,
            [_lexical("quick fox", NOTE)],
            text_producers=NOTE_ONLY,
            statistics={"source": "host-statistics"},
                    top_k=PLAN_TOP_K,
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
        ,
            top_k=PLAN_TOP_K,
)

    # The neighbouring valid case: one language is one partition, and it answers.
    one_language = (
        f'<{EX}a> <{NOTE}> "the quick fox"@en .\n'
        f'<{EX}b> <{NOTE}> "a quick hound"@en .\n'
    )
    planned = retrieval.plan(
        one_language, [_lexical("quick", NOTE)], text_producers=NOTE_ONLY, statistics=STATISTICS
    ,
        top_k=PLAN_TOP_K,
)
    assert planned["producer_bindings"], "a single-partition index declares a ranked order"

    # …and the declaration is worth having: the whole ladder runs over it and both
    # documents holding the needle come back ranked. "It planned" alone would be
    # satisfied by a declaration nothing could execute.
    answer = retrieval.search(
        one_language,
        [_lexical("quick", NOTE)],
        text_producers=NOTE_ONLY,
        weights={NOTE_STRATUM: retrieval.SCALE},
        statistics=STATISTICS,
        k=60,
        decay=TRUNCATED,
        top_k=10,
    )
    assert sorted(row["entity"] for row in answer["rows"]) == [f"<{EX}a>", f"<{EX}b>"], (
        "one partition, one ranking, and every document holding the needle in it"
    )


def test_an_unknown_request_kind_names_what_is_accepted() -> None:
    """A malformed request is refused where it is written, not later."""
    with pytest.raises(ValueError, match="unknown kind"):
        retrieval.plan(
            DATA,
            [("keyword", "quick fox", None, NOTE)],
            text_producers=NOTE_ONLY,
            statistics=STATISTICS,
                    top_k=PLAN_TOP_K,
)


def test_an_unknown_graph_selector_names_what_is_accepted() -> None:
    """A producer's graph selector is "any", "default", or a named-graph IRI."""
    with pytest.raises(ValueError, match="unknown graph selector"):
        retrieval.plan(
            DATA,
            [_lexical("quick fox", NOTE)],
            text_producers={NOTE_PRODUCER: (NOTE_STRATUM, NOTE, "every")},
            statistics=STATISTICS,
                    top_k=PLAN_TOP_K,
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
    # one, refuses at the identical boundary as a limit of the depth encoding a
    # plan carries -- with the neighbouring in-range depth still priced.
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


# ---------------------------------------------------------------------------
# What each producer promises about its own rows, and what an answer may be
# read to claim because of it.
#
# The fidelity term is host-supplied for the reason ``domains`` is: BM25 over
# the index the relation holds is exhaustive, and whether that index covers what
# the host means by its corpus is a fact the relation cannot see. A host that
# indexed a sample says so here or nowhere.
# ---------------------------------------------------------------------------

LOSS = "approximate: a 10% sample of the corpus; an empty result proves nothing"

# The other axis. Only a producer comparing APPROXIMATED values ranks a row
# better than it earned, and the shipped text relation never does -- BM25 over
# the document this call was handed compares exact scores. A host whose text was
# transliterated or machine-translated before it arrived is the party who knows
# otherwise, and this is the only place it can say so.
PERTURBATION = "machine-translated before indexing; ranks compare glosses"


def _is_zero(lexical: str) -> bool:
    """Whether a rendered fixed-point value is zero.

    Compared numerically rather than as text: a `Fixed` renders in its full
    decimal lexical (`"0.000000000000"`), and pinning the spelling here would
    make this test fail if the scale ever changed, which is not what it is about.
    """
    return float(lexical) == 0.0


def _with_fidelity(
    *entries: tuple[str, str, str, tuple[str | None, str | None]],
) -> dict[str, tuple[Any, ...]]:
    """``(producer, stratum, predicate, (completeness, order))`` as the map.

    The six-element spelling, with ``domains`` left unrestricted and the
    attestation left silent so the fidelity term is the only thing under test.

    Fidelity is the SIXTH position, not the fifth. The fifth is an attestation,
    and the two are about different objects: an attestation says which version
    of an index answered and whether that version was whole, while a fidelity
    says whether the producer's own search over it names every row it should and
    ranks them as they were due. A host can be silent on one and explicit on the
    other, which is why neither position can stand in for the other.
    """
    return {
        producer: (stratum, predicate, "any", None, (None, None), fidelity)
        for producer, stratum, predicate, fidelity in entries
    }


def _answer(producers: dict[str, Any], **overrides: Any) -> dict[str, Any]:
    """One fused answer over ``producers``, with the fixture's usual inputs."""
    kwargs: dict[str, Any] = {
        "text_producers": producers,
        "weights": {NOTE_STRATUM: retrieval.SCALE, TITLE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": 10,
    }
    kwargs.update(overrides)
    return retrieval.search(
        DATA,
        [_lexical("quick fox", NOTE), _lexical("quick", TITLE)],
        **kwargs,
    )


def test_every_stratum_reports_its_fidelity_on_both_axes() -> None:
    answer = _answer(BOTH)
    assert set(answer["fidelities"]) == set(answer["attestations"]), (
        "keyed like the maps beside it: every stream fusion was handed, and "
        "nothing else"
    )
    for stratum, fidelity in answer["fidelities"].items():
        assert fidelity["completeness"] == "complete", stratum
        assert fidelity["order"] == "faithful", stratum
        # Absent, not null. A `None` would be a third state to interpret, which
        # is the ambiguity this surface exists to remove.
        assert "completeness_evidence" not in fidelity, stratum
        assert "order_evidence" not in fidelity, stratum


def test_an_exhaustive_answer_is_exact_and_wholly_certain() -> None:
    answer = _answer(BOTH)
    assert answer["exactness"] == {
        "exact": True,
        "deficit": [],
        "inflation": [],
        "unbounded": [],
    }
    for row in answer["rows"]:
        assert row["interval"]["bounded"] is True
        assert _is_zero(row["interval"]["deficit"])
        assert _is_zero(row["interval"]["inflation"])
    assert answer["certain_prefix"] == len(answer["rows"]), (
        "nothing was withheld and nothing was promoted, so every row keeps its "
        "place -- which is the answer this engine always gave"
    )


def test_a_declared_loss_reaches_the_answer_verbatim_and_on_both_sides() -> None:
    answer = _answer(
        _with_fidelity(
            (NOTE_PRODUCER, NOTE_STRATUM, NOTE, (LOSS, None)),
            (TITLE_PRODUCER, TITLE_STRATUM, TITLE, (None, None)),
        )
    )

    degraded = answer["fidelities"][NOTE_STRATUM]
    assert degraded["completeness"] == "lossy"
    assert degraded["completeness_evidence"] == LOSS, (
        "the string the host published is the string a consumer reads: not a "
        "boolean derived downstream, not a summary, not a re-wording"
    )
    assert degraded["order"] == "faithful", (
        "a partial index still ranks what it holds truly; only a producer "
        "comparing approximated values is order-perturbed"
    )

    # The neighbour that must not be swept up with it.
    assert answer["fidelities"][TITLE_STRATUM]["completeness"] == "complete"

    # Named on BOTH sides, and only the responsible stratum.
    assert answer["exactness"]["exact"] is False
    assert answer["exactness"]["deficit"] == [NOTE_STRATUM]
    assert answer["exactness"]["inflation"] == [NOTE_STRATUM], (
        "fusion scores by rank, so a stratum that misses a row also promotes "
        "every row behind it -- the error runs in both directions"
    )
    assert answer["exactness"]["unbounded"] == []


def test_a_status_alone_is_not_a_completeness_claim() -> None:
    answer = _answer(
        _with_fidelity(
            (NOTE_PRODUCER, NOTE_STRATUM, NOTE, (LOSS, None)),
            (TITLE_PRODUCER, TITLE_STRATUM, TITLE, (None, None)),
        )
    )
    # The status is exactly what an exhaustive producer reports. That is the
    # whole problem, and why the fidelity has to sit beside it.
    assert answer["statuses"][NOTE_STRATUM]["status"] in STATUS_SPELLINGS
    assert answer["fidelities"][NOTE_STRATUM]["completeness"] == "lossy"


def test_a_declared_loss_bounds_each_row_and_can_shorten_the_certain_prefix() -> None:
    exhaustive = _answer(BOTH)
    degraded = _answer(
        _with_fidelity(
            (NOTE_PRODUCER, NOTE_STRATUM, NOTE, (LOSS, None)),
            (TITLE_PRODUCER, TITLE_STRATUM, TITLE, (None, None)),
        )
    )

    named = [
        row
        for row in degraded["rows"]
        if any(c["stratum"] == NOTE_STRATUM for c in row["contributions"])
    ]
    assert named, "the fixture has rows the degraded stratum named"
    for row in named:
        assert row["interval"]["inflation"] != "0", (
            "a row the lossy stratum named may have been promoted by a row it "
            "missed, so its whole contribution from that stratum is suspect"
        )

    # Both directions, and the evidence behind them. `degraded <= exhaustive`
    # alone is satisfied by `0 <= n` for ANY implementation -- including one that
    # always answered zero -- so it is the exhaustive side that has to be pinned
    # to a real number for the comparison to mean anything.
    assert exhaustive["certain_prefix"] == len(exhaustive["rows"]), (
        "nothing was withheld and nothing was promoted, so every row keeps its "
        "place and the whole answer is certain"
    )
    assert degraded["certain_prefix"] < exhaustive["certain_prefix"], (
        "and a declared loss really does cost certainty rather than being "
        "recorded and ignored"
    )

    # The bound the verdict rests on, on both answers. Exhaustive: every stream
    # ran out and none could have missed a row, so nothing outside the answer
    # can be worth anything at all. Degraded: the lossy stratum could have owed
    # an unnamed candidate a rank-one contribution, so the ceiling is a real
    # number and that is exactly what shortens the prefix.
    assert _is_zero(exhaustive["unemitted_ceiling"]), (
        "no candidate outside an exhaustive answer can be worth anything"
    )
    assert not _is_zero(degraded["unemitted_ceiling"]), (
        "a lossy stratum's whole failure mode is not naming things, so a "
        "candidate it never named is owed the most it could have held back"
    )

    assert _ranking(degraded) == _ranking(exhaustive), (
        "and the rows are identical either way: a declaration changes what the "
        "answer may be read to claim, never what the answer is"
    )


def test_an_empty_evidence_is_refused_on_either_axis_and_its_neighbours_are_not() -> (
    None
):
    """A degradation with nothing behind it is refused; real prose is not.

    The refusal is per AXIS, because the two fail independently and a host may
    know about one and not the other. Both sides are executed: over-refusal is
    the mirror of the silent-drop bug and it hides perfectly, since a test that
    only checks the refusals passes for an implementation that refuses
    everything.
    """

    def search(fidelity: tuple[Any, Any]) -> dict[str, Any]:
        return _answer(
            _with_fidelity((NOTE_PRODUCER, NOTE_STRATUM, NOTE, fidelity)),
            weights={NOTE_STRATUM: retrieval.SCALE},
        )

    # Refused, on either axis: a declared degradation that discloses nothing
    # reports a degraded stratum while saying nothing a reader can act on, and
    # is indistinguishable in a rendered answer from one that declared none.
    for empty in ("", "    ", "\n\t "):
        with pytest.raises(ValueError, match="`completeness` but supplies no evidence"):
            search((empty, None))
        with pytest.raises(ValueError, match="`order` but supplies no evidence"):
            search((None, empty))

    # Refused: a member that is not a string names the axis it was written for.
    with pytest.raises(TypeError, match="`completeness` must be a str or None"):
        search((7, None))
    with pytest.raises(TypeError, match="`order` must be a str or None"):
        search((None, 7))

    # Refused: a bare string is NOT destructured into its own two characters.
    # `"ab"` extracts as a well-formed two-member sequence, so accepting it would
    # report `"a"` back as completeness evidence the host never wrote.
    with pytest.raises(TypeError, match="a fidelity is the sixth"):
        search("ab")  # type: ignore[arg-type]
    # As is a sequence of the wrong width: three axes is not this declaration.
    with pytest.raises(TypeError, match="a fidelity is the sixth"):
        search((LOSS, None, None))  # type: ignore[arg-type]

    # ── the neighbours that must still work, which is the point of the test ──
    silent = search((None, None))["fidelities"][NOTE_STRATUM]
    assert silent == {"completeness": "complete", "order": "faithful"}, (
        "silence on both axes is the top of the lattice, and it is reported "
        "rather than left absent"
    )

    lossy = search((LOSS, None))["fidelities"][NOTE_STRATUM]
    assert lossy["completeness"] == "lossy"
    assert lossy["completeness_evidence"] == LOSS
    assert lossy["order"] == "faithful", "one axis degraded is not both"

    # The order axis, which no shipped producer can reach on its own: a host
    # whose text was transliterated or machine-translated before it arrived is
    # ranking over approximations of the values the ranking is meant to be over.
    perturbed = search((None, PERTURBATION))["fidelities"][NOTE_STRATUM]
    assert perturbed["order"] == "perturbed"
    assert perturbed["order_evidence"] == PERTURBATION
    assert perturbed["completeness"] == "complete", "and the other way round"

    both = search((LOSS, PERTURBATION))["fidelities"][NOTE_STRATUM]
    assert both == {
        "completeness": "lossy",
        "completeness_evidence": LOSS,
        "order": "perturbed",
        "order_evidence": PERTURBATION,
    }


def test_a_perturbed_order_leaves_every_score_it_touched_without_a_finite_bound() -> (
    None
):
    """The axis that breaks the bound, end to end on the shipped surface.

    Every score bound this engine computes rests on one inequality: a row a
    producer NAMED has a true rank at least as bad as the rank it was emitted
    at. A producer comparing approximated values breaks exactly that -- it can
    rank a row it found BETTER than the row was due -- so there is no number to
    report, and reporting one anyway would be the fabrication this channel
    exists to prevent.

    So the interval says so instead of guessing, and it says it per row: only
    rows the perturbed stratum actually named lose their bound. The strata
    responsible are named, in both places, so the fact stays actionable.
    """
    answer = _answer(
        _with_fidelity(
            (NOTE_PRODUCER, NOTE_STRATUM, NOTE, (None, PERTURBATION)),
            (TITLE_PRODUCER, TITLE_STRATUM, TITLE, (None, None)),
        )
    )

    assert answer["fidelities"][NOTE_STRATUM] == {
        "completeness": "complete",
        "order": "perturbed",
        "order_evidence": PERTURBATION,
    }, "a perturbed order is not a completeness claim, and is not derived from one"

    # The answer-level verdict names it under `unbounded` and NOT under the two
    # finite sides: this producer names every row that was due, so nothing is
    # withheld and nothing is promoted by absence. It is the ORDER that is wrong.
    assert answer["exactness"] == {
        "exact": False,
        "deficit": [],
        "inflation": [],
        "unbounded": [NOTE_STRATUM],
    }

    named = [
        row
        for row in answer["rows"]
        if any(c["stratum"] == NOTE_STRATUM for c in row["contributions"])
    ]
    assert named, "the fixture has rows the perturbed stratum named"
    for row in named:
        assert row["interval"] == {"bounded": False, "perturbed": [NOTE_STRATUM]}, (
            "no finite bound exists for a row whose rank a perturbed producer "
            "decided, and the responsible stratum is named rather than implied"
        )

    # The neighbour: a faithful stratum's declaration is untouched, and a row
    # only the faithful stratum named keeps a finite bound.
    assert answer["fidelities"][TITLE_STRATUM]["order"] == "faithful"
    for row in answer["rows"]:
        if row not in named:
            assert row["interval"]["bounded"] is True

    assert answer["certain_prefix"] == 0, (
        "and nothing is certain: a prefix is a claim about relative order, and "
        "a perturbed stratum is precisely a producer whose order is not evidence"
    )


def test_the_evidence_a_host_writes_is_the_evidence_a_consumer_reads() -> None:
    """Byte for byte, on both axes, through the whole ladder.

    The string is the host's disclosure and a consumer acts on it, so anything
    this layer does to it -- trimming, tagging, re-wrapping -- is the layer
    editing a statement it did not make. The characters here are the ones a
    delimiter scheme or a `strip()` would corrupt: leading and trailing
    whitespace, a colon, a semicolon, a control character, and a newline. The
    Rust leg is proved against the same shapes, and this is the claim that the
    Python leg is not weaker.
    """
    awkward = "  lossy: 10% sample; seethe note\nand the second line  "
    answer = _answer(
        _with_fidelity((NOTE_PRODUCER, NOTE_STRATUM, NOTE, (awkward, awkward))),
        weights={NOTE_STRATUM: retrieval.SCALE},
    )
    declared = answer["fidelities"][NOTE_STRATUM]
    assert declared["completeness_evidence"] == awkward, (
        "not trimmed, not parsed, not re-worded -- the bytes the host published"
    )
    assert declared["order_evidence"] == awkward
    # And in particular the surrounding whitespace survived, which is the part a
    # `.trim()` would silently take and no other assertion here would notice.
    assert declared["completeness_evidence"].startswith("  ")
    assert declared["completeness_evidence"].endswith("  ")


def test_the_four_producer_spellings_agree_where_they_overlap() -> None:
    """Every accepted width, saying the same thing, answers the same thing.

    There are four: the bare triple, plus a ``domains`` position, plus an
    attestation position, plus a fidelity position. Each added position has a
    value that means SILENCE, and a spec that writes the silent value must be
    indistinguishable from one that stops short of the position entirely —
    otherwise writing a position down would itself be a declaration, and a host
    that spelled out a default would get a different answer from one that did
    not.
    """
    common: dict[str, Any] = {"weights": {NOTE_STRATUM: retrieval.SCALE}}
    three = _answer(NOTE_ONLY, **common)
    four = _answer(_declared((NOTE_PRODUCER, NOTE_STRATUM, NOTE, None)), **common)
    five = _answer(
        {NOTE_PRODUCER: (NOTE_STRATUM, NOTE, "any", None, (None, None))}, **common
    )
    six = _answer(
        _with_fidelity((NOTE_PRODUCER, NOTE_STRATUM, NOTE, (None, None))), **common
    )

    widths = (three, four, five, six)
    for answer in widths[1:]:
        assert _ranking(answer) == _ranking(three), (
            "the same declaration, four ways to write it, one ranking"
        )
        assert answer["fidelities"] == three["fidelities"], (
            "the shorter spellings are the same declaration, so they declare the "
            "same thing: omitting the term means exhaustive, not unstated"
        )
        assert answer["exactness"] == three["exactness"]
        assert answer["attestations"] == three["attestations"], (
            "a written-out silence is silence, not a declaration"
        )
    # Not `plan_id`: a plan identity folds the registry's per-process INSTANCE
    # id alongside its content, so two separately built registries never share
    # one even when they declare identically. The content is what these three
    # spellings share, and `canonical_description`'s injectivity test in
    # `crates/sparql-eval` is where that is pinned.


def test_the_shipped_docstring_teaches_the_channel_it_actually_returns() -> None:
    """``help(retrieval.search)`` is the only spec most hosts will ever read.

    A docstring is not decoration on this surface: it ships inside the wheel, it
    is what ``help()`` prints, and it is the only description of the answer a
    host has at the interpreter. One that names a key the function does not
    return is worse than one that says nothing, because a reader acts on it --
    and the key this replaced, ``lower_bounds_for``, taught the one-sided reading
    the two-sided interval exists to correct. A consumer that believed it would
    treat every score as a floor and be confidently wrong in the one direction
    the name told it not to look.

    So the prose is pinned against the dict the function really builds, rather
    than reviewed. Both halves matter: the stale key must be gone, and every key
    a caller must know about must be named. Checking only the first would pass
    for a docstring that had been emptied.
    """
    doc = retrieval.search.__doc__
    assert doc, "the shipped function carries its own documentation"
    flat = " ".join(doc.split())

    assert "LOWER BOUND" not in doc, (
        "the one-sided reading is gone: a score is an estimate whose error runs "
        "both ways, and a floor is the wrong shape for it"
    )
    # The key may be NAMED, but only to deny it. A reader who learned the old
    # shape elsewhere needs to be told it is gone, so the disclaimer earns its
    # place -- but it must be the only mention, or the denial sits beside a
    # promise of the same key.
    assert flat.count("lower_bounds_for") == 1, (
        "the only mention of the retired key is the sentence retiring it"
    )
    assert 'there is no `"lower_bounds_for"` key' in flat, (
        "and that sentence says outright that it does not exist, rather than "
        "leaving a reader to notice its absence"
    )

    # Every key the answer really carries for this channel, named where a host
    # can find it. Derived from a real answer rather than transcribed, so a key
    # that is renamed without the prose following fails here.
    answer = _answer(
        _with_fidelity(
            (NOTE_PRODUCER, NOTE_STRATUM, NOTE, (LOSS, None)),
            (TITLE_PRODUCER, TITLE_STRATUM, TITLE, (None, None)),
        )
    )
    for key in ("fidelities", "certain_prefix", "exactness", "statuses"):
        assert key in answer, f"the fixture does not exercise {key}"
        assert f'"{key}"' in doc, f"`{key}` is returned but never documented"
    for key in ("deficit", "inflation", "unbounded"):
        assert key in answer["exactness"], f"the fixture does not exercise {key}"
        assert f'"{key}"' in doc, f"`exactness[{key}]` is returned but undocumented"
    assert "interval" in answer["rows"][0], "the fixture does not exercise interval"
    assert '"interval"' in doc, "a per-row interval is returned but undocumented"


# ── the declarations a call is assembled from ──────────────────────────────────


def test_the_planned_resolution_reports_the_depth_the_plan_itself_recorded() -> None:
    """The cost is quoted against the depth this plan records, not one of its own.

    ``planned_resolution`` answers "what will this plan cost", so the depth it
    prices has to be the depth recorded beside it. A resolution derived from some
    other number — the producer's declared bound, say, or a fresh re-derivation —
    would be a true statement about a plan that is not the one being compiled, and
    the two agree on a small fixture often enough that nothing would look wrong.
    """
    compiled = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=STATISTICS,
        weights={NOTE_STRATUM: retrieval.SCALE},
        k=60,
        decay=TRUNCATED,
            top_k=PLAN_TOP_K,
)
    assert (
        compiled["planned_resolution"][NOTE_STRATUM]["requested_depth"]
        == compiled["plan"]["stratum_depths"][NOTE_STRATUM]
    ), "the evidence names the depth the plan recorded"

    # And it follows the plan when the plan moves: a measured cardinality lowers
    # the recorded depth, and the priced depth goes with it.
    bounded = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics={**STATISTICS, "cardinality": {NOTE_STRATUM: 1}},
        weights={NOTE_STRATUM: retrieval.SCALE},
        k=60,
        decay=TRUNCATED,
            top_k=PLAN_TOP_K,
)
    assert bounded["plan"]["stratum_depths"][NOTE_STRATUM] == 1
    assert bounded["planned_resolution"][NOTE_STRATUM]["requested_depth"] == 1
    assert (
        bounded["planned_resolution"][NOTE_STRATUM]["requested_depth"]
        < compiled["planned_resolution"][NOTE_STRATUM]["requested_depth"]
    ), "the priced depth moved with the plan rather than staying put"


def test_a_non_positive_weight_is_refused_naming_the_stratum() -> None:
    """A weight of zero or less orders nothing, and the refusal says whose.

    A weight is a stratum's share of every fused score. Zero contributes nothing
    at any rank, so a stratum weighted zero is a stream read and discarded — and a
    negative weight orders the ranking backwards. Neither is a weight, and a
    ``weights`` map holds several, so the message names the one to fix.

    The neighbouring positive weights are executed here because "positive" reaches
    all the way down to one raw unit: an implementation that refused anything it
    considered too small to matter would be discarding exactly the low-weight
    strata a host tunes by hand.
    """
    common: dict[str, Any] = {
        "text_producers": NOTE_ONLY,
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": 10,
    }
    request = [_lexical("quick fox", NOTE)]

    for refused_weight in (0, -1, -retrieval.SCALE):
        with pytest.raises(ValueError, match="non-positive weight") as refused:
            retrieval.search(DATA, request, weights={NOTE_STRATUM: refused_weight}, **common)
        assert NOTE_STRATUM in str(refused.value), (
            f"the refusal names the stratum whose weight was rejected: {refused.value}"
        )

    # The neighbouring valid weights, down to the smallest positive one there is.
    for accepted in (1, retrieval.SCALE // 2, retrieval.SCALE, 1000 * retrieval.SCALE):
        answer = retrieval.search(DATA, request, weights={NOTE_STRATUM: accepted}, **common)
        assert answer["rows"], f"a raw weight of {accepted} is a weight and answers"


def test_every_graph_selector_spelling_selects_a_different_reading() -> None:
    """``"any"``, ``"default"`` and a graph IRI are three readings of one corpus.

    A selector accepted and then ignored is the failure this catches, and it needs
    a corpus where the three disagree: one row in a named graph and one in the
    default graph, so ``"default"`` reaches exactly the second, the graph IRI
    reaches exactly the first, and each is the wrong answer for the other. A
    single-graph fixture makes all three identical.

    ``"any"`` is the third reading and is exercised over a single-graph corpus,
    because a graph is a partition of the index and a rank is computed within one
    partition — so ``"any"`` over two graphs cannot honestly declare a ranking at
    all, which is the neighbouring refusal held below.
    """
    common: dict[str, Any] = {
        "weights": {NOTE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": 10,
        "data_format": "nquads",
    }
    request = [_lexical("quick fox", NOTE)]
    named_graph = f"{EX}g"
    split = (
        f'<{EX}a> <{NOTE}> "the quick brown fox" <{named_graph}> .\n'
        f'<{EX}b> <{NOTE}> "a quick red fox" .\n'
    )

    def _entities(selector: str, corpus: str) -> list[str]:
        answer = retrieval.search(
            corpus,
            request,
            text_producers={NOTE_PRODUCER: (NOTE_STRATUM, NOTE, selector)},
            **common,
        )
        return [row["entity"] for row in answer["rows"]]

    assert _entities("default", split) == [f"<{EX}b>"], (
        "the default graph holds ex:b and nothing else"
    )
    assert _entities(named_graph, split) == [f"<{EX}a>"], (
        "and the named graph holds ex:a — so the selector really routes"
    )

    # "any" over ONE graph is the widest reading and reaches both rows.
    one_graph = (
        f'<{EX}a> <{NOTE}> "the quick brown fox" <{named_graph}> .\n'
        f'<{EX}b> <{NOTE}> "a quick red fox" <{named_graph}> .\n'
    )
    assert sorted(_entities("any", one_graph)) == [f"<{EX}a>", f"<{EX}b>"]

    # A fourth spelling is refused naming what a selector may be — and the
    # refusal names the producer that carried it, since a map holds several.
    with pytest.raises(ValueError, match="unknown graph selector") as refused:
        retrieval.search(
            split,
            request,
            text_producers={NOTE_PRODUCER: (NOTE_STRATUM, NOTE, "every")},
            **common,
        )
    message = str(refused.value)
    assert NOTE_PRODUCER in message, message
    for accepted in ('"any"', '"default"'):
        assert accepted in message, f"the refusal names {accepted}: {message}"


def test_each_data_format_name_routes_the_document_and_an_unknown_one_is_refused() -> None:
    """Three document syntaxes, named, with no default beyond ``"turtle"``.

    Each is proved by syntax only its own codec reads, so a name routed to the
    wrong codec fails on its own document rather than being masked by a grammar
    that happens to accept both.
    """
    common: dict[str, Any] = {
        "text_producers": NOTE_ONLY,
        "weights": {NOTE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": 10,
    }
    request = [_lexical("quick fox", NOTE)]
    turtle = f'PREFIX ex: <{EX}> ex:a ex:note "the quick brown fox" .\n'
    ntriples = f'<{EX}a> <{NOTE}> "the quick brown fox" .\n'
    nquads = f'<{EX}a> <{NOTE}> "the quick brown fox" <{EX}g> .\n'

    for document, data_format in (
        (turtle, "turtle"),
        (ntriples, "ntriples"),
        (nquads, "nquads"),
    ):
        answer = retrieval.search(document, request, data_format=data_format, **common)
        assert [row["entity"] for row in answer["rows"]] == [f"<{EX}a>"], data_format

    # …and each syntax the others cannot read is refused, so the three above are
    # routing rather than one lenient grammar read three times.
    with pytest.raises(ValueError):
        retrieval.search(nquads, request, data_format="ntriples", **common)
    with pytest.raises(ValueError):
        retrieval.search(turtle, request, data_format="nquads", **common)

    with pytest.raises(ValueError, match="unknown data format") as refused:
        retrieval.search(ntriples, request, data_format="trix", **common)
    message = str(refused.value)
    for accepted in ('"turtle"', '"ntriples"', '"nquads"'):
        assert accepted in message, f"the refusal names {accepted}: {message}"


def test_every_partial_fusion_law_names_what_arrived_and_what_did_not() -> None:
    """All six proper subsets of the three, each told apart from the others.

    A law is ``weights``, ``k`` and ``decay`` together. There are six ways to name
    some and not the rest, and a refusal that described them with one sentence
    would be useless in exactly the case a caller needs it: they wrote two of the
    three and cannot see which one is missing. So each message is checked to name
    the parts that arrived AND the parts that did not, and the two lists are
    checked against each other — a message that named all three on both sides
    would match either half alone.
    """
    common: dict[str, Any] = {
        "text_producers": NOTE_ONLY,
        "statistics": STATISTICS,
    }
    request = [_lexical("quick fox", NOTE)]
    weights = {NOTE_STRATUM: retrieval.SCALE}
    labels = {
        "weights": "`weights`",
        "k": "the smoothing constant `k`",
        "decay": "the `decay` rule",
    }
    whole = {"weights": weights, "k": 60, "decay": TRUNCATED}

    # The three parts in the order the message lists them, so an expectation can
    # be derived rather than transcribed six times.
    order = ("weights", "k", "decay")
    for supplied in (
        ("weights",),
        ("k",),
        ("decay",),
        ("weights", "k"),
        ("weights", "decay"),
        ("k", "decay"),
    ):
        absent = tuple(part for part in order if part not in supplied)
        with pytest.raises(ValueError) as refused:
            retrieval.compile(
                DATA, request, **{part: whole[part] for part in supplied}, **common
            ,
                top_k=PLAN_TOP_K,
)
        message = str(refused.value)
        arrived = " and ".join(labels[part] for part in order if part in supplied)
        missing = " and ".join(labels[part] for part in absent)
        assert f"named {arrived}, and left {missing} unnamed" in message, (
            f"supplied {supplied}: {message}"
        )

    # The two neighbouring cases that are NOT refusals: all three name a law, and
    # none of them names no law — which is compiled without one, not refused.
    named = retrieval.compile(DATA, request, **whole, **common, top_k=PLAN_TOP_K)
    assert named["planned_resolution"][NOTE_STRATUM]["fully_separated"] is True
    lawless = retrieval.compile(DATA, request, **common, top_k=PLAN_TOP_K)
    assert lawless["planned_resolution"] == {}
    assert lawless["units"], "a call that names no law is still a compiled plan"


def test_a_several_block_declaration_is_refused_where_it_is_registered() -> None:
    """A restriction no row can back is refused at registration, not at row one.

    A one-block declaration needs nothing per row: it has already said where every
    candidate of this producer lies, so a consumer reads the block off the
    declaration. A SEVERAL-block declaration says only that the candidates lie
    somewhere in the set, which obliges the producer to say which block each row
    came from — and neither ranked relation this surface can build declares a
    column such a block could be read out of, because a domain tag describes how a
    host's corpora partition and only the host knows that.

    So a several-block list from Python is unsatisfiable by construction, and the
    fusion used to say so at the first row it pulled: a registration-time defect
    reported as a query-time failure, after the plan, the compile and the first
    read had all been paid for. It is refused where the caller wrote it instead,
    naming the producer and the three exits that do work.

    The blocks are quoted in canonical order, not the host's. A declaration is a
    SET, so the order it was written in carries no information and must not reach
    anything a reader compares: two hosts that named the same two blocks in
    different orders named the same declaration and read the same refusal.

    The valid neighbours are held below — one tag, one tag written twice, and
    `None` — and that the one-tag declaration still shortens the read it bounds
    is held by ``test_a_declared_domain_changes_the_reading_and_not_the_answer``.
    """
    common: dict[str, Any] = {
        "weights": {NOTE_STRATUM: retrieval.SCALE},
        "statistics": STATISTICS,
        "k": 60,
        "decay": TRUNCATED,
        "top_k": 10,
    }
    request = [_lexical("quick fox", NOTE)]
    assert NOTE_DOMAIN < TITLE_DOMAIN, "canonical order is over the tag IRIs"

    for declared in ([NOTE_DOMAIN, TITLE_DOMAIN], [TITLE_DOMAIN, NOTE_DOMAIN]):
        with pytest.raises(ValueError, match=re.escape(NOTE_PRODUCER)) as refused:
            retrieval.search(
                DATA,
                request,
                text_producers=_declared(
                    (NOTE_PRODUCER, NOTE_STRATUM, NOTE, declared)
                ),
                **common,
            )
        message = str(refused.value)
        assert "names 2 blocks" in message, message
        assert f"[<{NOTE_DOMAIN}>, <{TITLE_DOMAIN}>]" in message, (
            f"declared as {declared}, reported canonically: {message}"
        )
        # The exits, because a refusal a caller cannot act on is only a stop.
        assert "exactly ONE domain tag" in message
        assert "one producer per block" in message
        assert "`domains=None`" in message

    # It is refused before anything is planned, so the cheapest stage sees it too
    # — a host that never calls `search` still learns at the same point.
    with pytest.raises(ValueError, match="names 2 blocks"):
        retrieval.plan(
            DATA,
            request,
            text_producers=_declared(
                (NOTE_PRODUCER, NOTE_STRATUM, NOTE, [NOTE_DOMAIN, TITLE_DOMAIN])
            ),
            statistics=STATISTICS,
                    top_k=PLAN_TOP_K,
)

    # ── the valid neighbours, which is the point of the test ──────────────────
    # One block leaves nothing to disambiguate, so it registers AND still bounds
    # the read: the refusal above is about the ambiguity, not about declaring
    # domains at all.
    restricted = retrieval.search(
        DATA,
        request,
        text_producers=_declared((NOTE_PRODUCER, NOTE_STRATUM, NOTE, [NOTE_DOMAIN])),
        **common,
    )
    assert restricted["domains"] == {NOTE_STRATUM: [NOTE_DOMAIN]}
    assert restricted["rows"]

    # A list that spells one block twice names ONE block, so the count that
    # decides is taken over the set and not over the list the host wrote.
    repeated = retrieval.search(
        DATA,
        request,
        text_producers=_declared(
            (NOTE_PRODUCER, NOTE_STRATUM, NOTE, [NOTE_DOMAIN, NOTE_DOMAIN])
        ),
        **common,
    )
    assert repeated["domains"] == {NOTE_STRATUM: [NOTE_DOMAIN]}
    assert _ranking(repeated) == _ranking(restricted)

    # And `None` — the widest promise — registers exactly as it always did.
    unrestricted = retrieval.search(
        DATA,
        request,
        text_producers=_declared((NOTE_PRODUCER, NOTE_STRATUM, NOTE, None)),
        **common,
    )
    assert unrestricted["domains"] == {NOTE_STRATUM: None}
    assert _ranking(unrestricted) == _ranking(restricted)


# ── the plan document: a plan that leaves the process and comes back ─────────
#
# `plan(...)["canonical_bytes"]` is the plan's canonical, length-framed encoding
# — the bytes `"plan_id"` is the digest of — and `retrieval.certify_plan` is what
# reads one back in. The pair is what makes a recorded depth a CHECKABLE claim
# rather than an asserted one: the plan records every input its depths were
# derived from, so a document that arrives from somewhere a host does not control
# can have its arithmetic re-run against its own evidence.
#
# The tests below tamper with real documents. That is the only way to exercise
# the check at all: a freshly planned plan derives its own depths by
# construction, so certifying one re-proves a tautology and would pass over a
# `certify` that did nothing.

# The measured cardinality every plan-document fixture is planned under.
#
# It is load-bearing, not decoration. It makes each stratum's recorded DEPTH (2)
# differ from its recorded DECLARATION (3), and those two numbers sit in adjacent
# sections of the encoding under the same stratum key. Planned without it the
# two are equal, the four-byte depth and the low half of the eight-byte
# declaration read the same, and a splice aiming at one would be editing whichever
# came first. `_splice` asserts the target is unique rather than trusting this
# note.
PLAN_DOCUMENT_STATISTICS: dict[str, Any] = {
    **STATISTICS,
    "cardinality": {NOTE_STRATUM: 2},
}


def _framed(text: str) -> bytes:
    """One length-framed UTF-8 string, as the canonical writer writes it."""
    encoded = text.encode()
    return len(encoded).to_bytes(8, "little") + encoded


def _splice(document: bytes, old: bytes, new: bytes, what: str) -> bytes:
    """``document`` with its one occurrence of ``old`` replaced by ``new``.

    The uniqueness assertion is the test's own oracle. These splices address a
    field by the bytes around it rather than by a computed offset, so a fixture
    that made the pattern ambiguous -- or an encoding change that moved it --
    fails here, naming the field, instead of quietly editing a neighbouring one
    and asserting a refusal that came from the wrong place.
    """
    found = document.count(old)
    assert found == 1, (
        f"the {what} must appear exactly once in the document for this splice to "
        f"be addressing it, and it appears {found} time(s)"
    )
    return document.replace(old, new)


def _depth_entry(stratum: str, depth: int) -> bytes:
    """One entry of the per-stratum depths section: the key, then its 32-bit depth."""
    return _framed(stratum) + depth.to_bytes(4, "little")


def _snapshot_row(subject: str, cardinality: int) -> bytes:
    """One statistics-snapshot row: a measured cardinality, no selectivity.

    The shape the fixtures below produce: a present cardinality, an absent
    selectivity, and an empty term domain. Absence is a discriminant byte rather
    than a sentinel, because every ``u64`` is a legal measurement.
    """
    return (
        _framed(subject)
        + b"\x01"
        + cardinality.to_bytes(8, "little")
        + b"\x00"
        + (0).to_bytes(8, "little")
    )


def _one_stratum_document() -> tuple[dict[str, Any], bytes]:
    """A planned one-stratum plan and its canonical bytes."""
    planned = retrieval.plan(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=PLAN_DOCUMENT_STATISTICS,
        top_k=PLAN_TOP_K,
    )
    assert planned["stratum_depths"] == {NOTE_STRATUM: 2}
    assert planned["stratum_derivations"][NOTE_STRATUM]["declared"] == 3, (
        "the depth and the declaration must differ, or the splices below cannot "
        "tell which of the two they are addressing"
    )
    return planned, planned["canonical_bytes"]


def _two_stratum_document() -> tuple[dict[str, Any], bytes]:
    """A planned two-stratum plan and its canonical bytes.

    Two strata are what makes an ORDER exist to break. The keyed sections of the
    encoding carry one entry each here, and a single-entry section is ascending
    however it is written.
    """
    planned = retrieval.plan(
        DATA,
        [_lexical("quick fox", NOTE), _lexical("quick", TITLE)],
        text_producers=BOTH,
        statistics=PLAN_DOCUMENT_STATISTICS,
        top_k=PLAN_TOP_K,
    )
    assert planned["stratum_depths"] == {NOTE_STRATUM: 2, TITLE_STRATUM: 3}
    return planned, planned["canonical_bytes"]


def _depths_section(planned: dict[str, Any]) -> bytes:
    """The whole two-entry depths section: the count, then both entries ascending."""
    return (
        (2).to_bytes(8, "little")
        + _depth_entry(NOTE_STRATUM, planned["stratum_depths"][NOTE_STRATUM])
        + _depth_entry(TITLE_STRATUM, planned["stratum_depths"][TITLE_STRATUM])
    )


def test_a_planned_plan_round_trips_through_its_canonical_bytes() -> None:
    """The valid neighbour every refusal below is measured against.

    A document this surface produced decodes, certifies, and renders EQUAL to the
    plan it came from -- identity, depths, derivations, snapshot and all. Without
    this the refusals prove nothing: a `certify_plan` that refused every document
    would pass every one of them, and a round trip that lost a field would make
    the refusals look like strictness rather than breakage.
    """
    planned, document = _one_stratum_document()
    assert isinstance(document, bytes)

    received = retrieval.certify_plan(document)
    assert received == planned, (
        "the decoded plan renders exactly as the plan it was encoded from; a "
        "field that did not survive the round trip would show up here"
    )
    assert received["canonical_bytes"] == document, (
        "and re-encoding the decoded plan reproduces the document byte for byte, "
        "which is what makes `plan_id` the digest of the bytes in hand"
    )
    assert received["plan_id"] == planned["plan_id"]


def test_a_tampered_depth_is_refused_as_not_derivable() -> None:
    """A depth edited away from its own recorded inputs is refused BY NAME.

    This is what the record exists for. The document's recorded inputs derive a
    depth of 2; the splice rewrites the recorded depth to 3 and changes nothing
    else, so the plan now claims a read one row deeper than its own evidence
    licenses. Nothing about the document is malformed -- it decodes -- and the
    refusal comes from re-running the planner's arithmetic over the plan's own
    numbers.

    The control is the untampered document, which certifies in the test above and
    is asserted to differ from this one here, so a splice that silently did
    nothing could not pass.
    """
    _planned, document = _one_stratum_document()
    forged = _splice(
        document,
        _depth_entry(NOTE_STRATUM, 2),
        _depth_entry(NOTE_STRATUM, 3),
        "recorded depth",
    )
    assert forged != document, "the splice must actually have edited the document"

    with pytest.raises(retrieval.PlanDocumentError) as refused:
        retrieval.certify_plan(forged)
    assert refused.value.refusal == "depth-not-derivable"
    assert "records depth 3" in str(refused.value)
    assert "derive depth 2" in str(refused.value)

    # And the neighbour that must still be admitted: the same document, unedited.
    assert retrieval.certify_plan(document)["stratum_depths"] == {NOTE_STRATUM: 2}


def test_a_document_written_under_another_layout_is_refused_by_version() -> None:
    """A version this build does not write is refused rather than reinterpreted."""
    _planned, document = _one_stratum_document()
    assert document[:2] == (4).to_bytes(2, "little"), (
        "the fixture's own header, read rather than assumed: the splice below "
        "moves it to a version this build does not write"
    )
    older = (3).to_bytes(2, "little") + document[2:]

    with pytest.raises(retrieval.PlanDocumentError) as refused:
        retrieval.certify_plan(older)
    assert refused.value.refusal == "version"
    assert "version 3" in str(refused.value)

    # The neighbour: the same bytes under the header this build does write.
    assert retrieval.certify_plan(document)["version"] == 4


def test_a_snapshot_row_contradicting_its_derivation_is_refused() -> None:
    """A stratum's two records of one measurement must agree.

    The snapshot row is written as a projection of the derivation, so they agree
    in any plan this surface emitted. The splice moves the SNAPSHOT's cardinality
    and leaves the derivation alone, which is why the depth still derives: the
    document is refused for the disagreement itself and not for an arithmetic
    that stopped working.
    """
    _planned, document = _one_stratum_document()
    forged = _splice(
        document,
        _snapshot_row(NOTE_STRATUM, 2),
        _snapshot_row(NOTE_STRATUM, 7),
        "snapshot row for the stratum",
    )

    with pytest.raises(retrieval.PlanDocumentError) as refused:
        retrieval.certify_plan(forged)
    assert refused.value.refusal == "statistics-entry-contradicts-derivation"
    assert "cardinality 7" in str(refused.value)

    # The neighbour: the untouched row, whose two records say the same number.
    entries = {
        entry["subject"]: entry
        for entry in retrieval.certify_plan(document)["statistics"]["entries"]
    }
    assert entries[NOTE_STRATUM]["cardinality"] == 2


def test_a_derivation_the_snapshot_does_not_name_is_refused() -> None:
    """A depth derived for a stratum the snapshot omits is refused.

    The splice renames the snapshot's row for the stratum to a subject of the same
    length that still sorts after the request predicate beside it -- so the
    document remains ascending and decodes, and the only thing wrong with it is
    that the stratum a depth was derived for is now named nowhere in the evidence.
    """
    _planned, document = _one_stratum_document()
    renamed = f"{EX}stratum/zote"
    assert len(renamed) == len(NOTE_STRATUM) and renamed > NOTE, (
        "the replacement subject keeps the snapshot ascending, so this document "
        "is refused for the missing stratum rather than for an unordered section"
    )
    forged = _splice(
        document,
        _snapshot_row(NOTE_STRATUM, 2),
        _snapshot_row(renamed, 2),
        "snapshot row for the stratum",
    )

    with pytest.raises(retrieval.PlanDocumentError) as refused:
        retrieval.certify_plan(forged)
    assert refused.value.refusal == "derivation-without-statistics-entry"
    assert NOTE_STRATUM in str(refused.value)

    # The neighbour: the same document with the stratum named, which certifies.
    assert any(
        entry["subject"] == NOTE_STRATUM
        for entry in retrieval.certify_plan(document)["statistics"]["entries"]
    )


def test_a_section_that_does_not_ascend_is_refused() -> None:
    """A keyed section out of order is not an encoding of any plan.

    The two depth entries are swapped and nothing else is touched, so the
    document describes exactly the plan it did before -- which is the point. A
    decoder that accepted it would admit as many distinct encodings of one plan
    as the section has permutations, each digesting to that plan's single id.
    """
    planned, document = _two_stratum_document()
    section = _depths_section(planned)
    descending = (
        (2).to_bytes(8, "little")
        + _depth_entry(TITLE_STRATUM, planned["stratum_depths"][TITLE_STRATUM])
        + _depth_entry(NOTE_STRATUM, planned["stratum_depths"][NOTE_STRATUM])
    )
    forged = _splice(document, section, descending, "per-stratum depths section")

    with pytest.raises(retrieval.PlanDocumentError) as refused:
        retrieval.certify_plan(forged)
    assert refused.value.refusal == "non-ascending-keys"
    assert "stratum depth" in str(refused.value)

    # The neighbour: the ascending original, which carries the same two depths.
    assert retrieval.certify_plan(document)["stratum_depths"] == {
        NOTE_STRATUM: 2,
        TITLE_STRATUM: 3,
    }


def _selectivity_run(terms: list[int]) -> bytes:
    """One length-framed run of 32-bit request-term indices."""
    return len(terms).to_bytes(8, "little") + b"".join(
        term.to_bytes(4, "little") for term in terms
    )


def _derivation_head(
    stratum: str, declared: int, selectivity_ppm: int, terms: list[int]
) -> bytes:
    """A derivation record up to and including its selectivity-term run.

    Stops at the run rather than spanning the whole record, so the splices below
    do not have to spell the licensed prefix that follows it. It still begins
    with the stratum key and the declaration, which is what tells it apart from
    the snapshot row for the same stratum -- the row carries no declaration.
    """
    return (
        _framed(stratum)
        + declared.to_bytes(8, "little")
        + b"\x00"
        + b"\x01"
        + selectivity_ppm.to_bytes(8, "little")
        + _selectivity_run(terms)
    )


def _selective_snapshot_row(
    subject: str, selectivity_ppm: int, terms: list[int]
) -> bytes:
    """One snapshot row: no cardinality, a measured selectivity, and its domain."""
    return (
        _framed(subject)
        + b"\x00"
        + b"\x01"
        + selectivity_ppm.to_bytes(8, "little")
        + _selectivity_run(terms)
    )


def _selective_one_stratum_document() -> tuple[dict[str, Any], bytes]:
    """A one-term plan whose stratum records a selectivity over term 0.

    One request term, so index 0 is the ONLY index that addresses this request --
    which is what makes the forge below a term that names nothing rather than a
    term that names another one.
    """
    planned = retrieval.plan(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics={**STATISTICS, "selectivity": {(NOTE_STRATUM, 0): 500_000}},
        top_k=PLAN_TOP_K,
    )
    assert [binding["request_terms"] for binding in planned["producer_bindings"]] == [
        [0]
    ], "one term, so index 0 is the only index this request has"
    assert planned["stratum_derivations"][NOTE_STRATUM]["selectivity_terms"] == [0]
    return planned, planned["canonical_bytes"]


def _two_term_selectivity_document() -> tuple[dict[str, Any], bytes]:
    """A two-term plan whose request predicate aggregates over BOTH terms.

    A run of two indices is what makes an ORDER exist to break: a run of one is
    ascending however it is written. The row is the note PREDICATE's, which
    derives no depth -- so a forge of its run is refused for the run itself
    rather than for disagreeing with a derivation it does not have.
    """
    planned = retrieval.plan(
        DATA,
        [_lexical("quick fox", NOTE), _lexical("quick", TITLE)],
        text_producers=BOTH,
        statistics={**STATISTICS, "selectivity": {(NOTE, 0): 100_000, (NOTE, 1): 200_000}},
        top_k=PLAN_TOP_K,
    )
    assert sorted(
        binding["request_terms"] for binding in planned["producer_bindings"]
    ) == [[0], [1]], "two terms, so indices 0 and 1 address this request and 2 does not"
    entries = {entry["subject"]: entry for entry in planned["statistics"]["entries"]}
    assert entries[NOTE]["selectivity_terms"] == [0, 1]
    assert entries[NOTE]["selectivity_ppm"] == 300_000, (
        "the aggregate is the SUM of the two terms the host answered for, which "
        "is the number the run beside it is the domain of"
    )
    return planned, planned["canonical_bytes"]


def test_a_selectivity_domain_that_addresses_no_request_term_is_refused() -> None:
    """A recorded selectivity domain indexes the plan's OWN request, or nothing.

    The run says which request terms a selectivity aggregate was summed over. An
    index past the end of the request names no term at all, so the aggregate's
    domain becomes unreadable and the record that exists to make the sum
    checkable stops being one. The forge moves the index in BOTH of the plan's
    records of it, so the document is refused for the index rather than for the
    two records disagreeing.

    The over-refusal guard is the boundary: this request carries one term, so 0
    addresses it and 1 does not, and both are executed. The empty run -- what a
    subject nothing contributed a selectivity to records -- is asserted legal in
    the same test, because a rule that refused a run field rather than an
    unaddressable index would take every plan the planner writes with it.
    """
    planned, document = _selective_one_stratum_document()
    declared = planned["stratum_derivations"][NOTE_STRATUM]["declared"]

    for forged_index in (1, 99):
        forged = _splice(
            document,
            _derivation_head(NOTE_STRATUM, declared, 500_000, [0]),
            _derivation_head(NOTE_STRATUM, declared, 500_000, [forged_index]),
            "derivation's selectivity-term run",
        )
        forged = _splice(
            forged,
            _selective_snapshot_row(NOTE_STRATUM, 500_000, [0]),
            _selective_snapshot_row(NOTE_STRATUM, 500_000, [forged_index]),
            "snapshot row's selectivity-term run",
        )
        assert forged != document, "the splices must actually have edited the document"

        with pytest.raises(retrieval.PlanDocumentError) as refused:
            retrieval.certify_plan(forged)
        assert refused.value.refusal == "selectivity-term-out-of-range"
        assert f"selectivity term {forged_index}" in str(refused.value)
        assert "carries 1 request term(s)" in str(refused.value)

    # The neighbour: the same document with the one index that DOES address this
    # request, which certifies and arrives with its domain intact.
    received = retrieval.certify_plan(document)
    assert received["stratum_derivations"][NOTE_STRATUM]["selectivity_terms"] == [0]
    entries = {entry["subject"]: entry for entry in received["statistics"]["entries"]}
    assert entries[NOTE_STRATUM]["selectivity_terms"] == [0]

    # And the empty run, which is what a subject no selectivity was reported for
    # records. This document's every run is empty and it certifies untouched, so
    # the refusals above are about the index and not about the field.
    _empty, without_selectivity = _one_stratum_document()
    certified = retrieval.certify_plan(without_selectivity)
    assert all(
        entry["selectivity_terms"] == []
        for entry in certified["statistics"]["entries"]
    )
    assert certified["stratum_derivations"][NOTE_STRATUM]["selectivity_terms"] == []


def test_a_selectivity_domain_out_of_order_or_repeated_is_refused() -> None:
    """A selectivity domain is a SET, so its encoding ascends and never repeats.

    The run is the domain of a sum, so `[1, 0]` states exactly what `[0, 1]`
    states -- which is precisely why the order has to be required rather than
    tolerated. Accepting both would give one plan two encodings, each digesting
    to that plan's single id, and the identity a host compares would stop being a
    function of the content. A repeat is the other clause: one term counted twice
    into a total the arithmetic reached once.

    Three forges and two neighbours over ONE fixture. The neighbours are a run
    that is genuinely different from the control -- a single index, not the pair
    -- so a decoder that had silently sorted or emptied the run could not pass
    them.
    """
    planned, document = _two_term_selectivity_document()
    honest = _selective_snapshot_row(NOTE, 300_000, [0, 1])

    for run, refusal, fragment in (
        ([1, 0], "non-ascending-selectivity-terms", "selectivity term 0 after 1"),
        ([1, 1], "duplicate-selectivity-term", "selectivity term 1 twice"),
        ([0, 2], "selectivity-term-out-of-range", "carries 2 request term(s)"),
    ):
        forged = _splice(
            document,
            honest,
            _selective_snapshot_row(NOTE, 300_000, run),
            "snapshot row's selectivity-term run",
        )
        with pytest.raises(retrieval.PlanDocumentError) as refused:
            retrieval.certify_plan(forged)
        assert refused.value.refusal == refusal, f"the run {run} is refused by name"
        assert fragment in str(refused.value)
        assert NOTE in str(refused.value), (
            "and the refusal names the record that carries the run, since a plan "
            "holds one per stratum and one per snapshot row"
        )

    # The neighbours, both over the same fixture. The first is a DIFFERENT run
    # from the control -- ascending, distinct, and in range -- so it proves the
    # decoder reads the run rather than waving a known pattern through.
    narrowed = _splice(
        document,
        honest,
        _selective_snapshot_row(NOTE, 300_000, [1]),
        "snapshot row's selectivity-term run",
    )
    entries = {
        entry["subject"]: entry
        for entry in retrieval.certify_plan(narrowed)["statistics"]["entries"]
    }
    assert entries[NOTE]["selectivity_terms"] == [1], (
        "an ascending, addressable run arrives exactly as it was written"
    )

    # The second is the untouched document, whose run is the pair.
    received = retrieval.certify_plan(document)
    assert received == planned
    entries = {entry["subject"]: entry for entry in received["statistics"]["entries"]}
    assert entries[NOTE]["selectivity_terms"] == [0, 1]


def test_a_stratum_recorded_twice_in_the_depths_is_refused() -> None:
    """Two depths for one stratum are two answers to one question.

    Refused rather than resolved: the map this section fills would silently keep
    whichever arrived last, which is exactly how a forged document would choose
    its own depth. The splice writes the note stratum's entry twice in place of
    the ascending pair, so the count the section declares still matches what it
    carries and the only thing wrong with it is the repeat.
    """
    planned, document = _two_stratum_document()
    entry = _depth_entry(NOTE_STRATUM, planned["stratum_depths"][NOTE_STRATUM])
    forged = _splice(
        document,
        _depths_section(planned),
        (2).to_bytes(8, "little") + entry + entry,
        "per-stratum depths section",
    )

    with pytest.raises(retrieval.PlanDocumentError) as refused:
        retrieval.certify_plan(forged)
    assert refused.value.refusal == "duplicate-stratum-depth"
    assert NOTE_STRATUM in str(refused.value)

    # The neighbour: the section naming each stratum once, which certifies.
    assert sorted(retrieval.certify_plan(document)["stratum_depths"]) == sorted(
        [NOTE_STRATUM, TITLE_STRATUM]
    )


def test_explain_depth_names_the_leg_that_bound_a_received_depth() -> None:
    """A host can ask which input bound a stratum's depth on a plan it received.

    The answer is read off the DOCUMENT rather than off the dict that produced it,
    which is the whole question: a host holding bytes someone else sent has no
    dict. It is the same closed vocabulary the plan renders under `"cause"`, from
    the same engine call, so the two are asserted to agree -- and the fixture
    makes them a real measurement by planning a cardinality that is narrower than
    the declaration, so a build that answered `"declaration"` for everything would
    fail here.
    """
    planned, document = _one_stratum_document()
    assert retrieval.explain_depth(document, NOTE_STRATUM) == "cardinality"
    assert (
        retrieval.explain_depth(document, NOTE_STRATUM)
        == planned["stratum_derivations"][NOTE_STRATUM]["cause"]
    )

    # A stratum this plan derives no depth for has no explanation, and says so
    # rather than naming a leg of a derivation that never ran.
    assert retrieval.explain_depth(document, f"{EX}stratum/nobody-asks") is None

    # The control that makes the first assertion an observation rather than a
    # constant: the same corpus planned with no cardinality at all derives its
    # depth from the declaration, and the answer moves with it.
    unmeasured = retrieval.plan(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=STATISTICS,
        top_k=PLAN_TOP_K,
    )
    assert (
        retrieval.explain_depth(unmeasured["canonical_bytes"], NOTE_STRATUM)
        == "declaration"
    )


def test_explain_depth_refuses_a_stratum_that_is_not_an_iri() -> None:
    """The label is parsed, not trusted, and the refusal says which dimension failed."""
    _planned, document = _one_stratum_document()
    with pytest.raises(retrieval.PlanDocumentError) as refused:
        retrieval.explain_depth(document, "not an iri")
    assert refused.value.refusal == "invalid-iri"

    # The neighbour: a well-formed IRI on the same document still answers.
    assert retrieval.explain_depth(document, NOTE_STRATUM) == "cardinality"


def test_a_compiled_plans_document_is_the_document_of_the_admitted_plan() -> None:
    """`compile` hands out the same plan document, for the plan it admitted.

    A host that compiled can store or certify the document without planning
    again, and the document it stores is the one the emitted units were compiled
    from. Both identities are asserted, because either alone would pass for a
    stage that rendered someone else's plan under the right digest: `"plan_id"`
    on the answer is the ADMITTED plan's, and the document must digest to it.

    The identities are compared within one call. Two calls plan against two
    registries, and the registry a plan was planned against is part of what a
    plan IS -- two registries that declare byte-identical contents can still
    register one IRI to two implementations that answer differently -- so two
    documents from two calls are not expected to be the same document.
    """
    compiled = retrieval.compile(
        DATA,
        [_lexical("quick fox", NOTE)],
        text_producers=NOTE_ONLY,
        statistics=PLAN_DOCUMENT_STATISTICS,
        top_k=PLAN_TOP_K,
    )
    document = compiled["plan"]["canonical_bytes"]
    assert compiled["plan"]["plan_id"] == compiled["plan_id"]

    received = retrieval.certify_plan(document)
    assert received == compiled["plan"]
    # The oracle that keeps this from passing over an empty document: the depth
    # the certified plan records is the depth the compiled unit was keyed to.
    assert received["stratum_depths"][NOTE_STRATUM] == compiled["units"][0]["depth"]
