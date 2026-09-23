# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""What an answer cost, on the surface a host that writes Python reads.

A fused answer's ``"statuses"`` say how each producer's read ENDED, and every
one of those spellings can be perfectly truthful over a read that cost a
hundred times what a neighbouring one did. ``"exhausted"`` is the sharpest case:
both streams really did run out, and from the status alone a five-row answer
drained out of four hundred rows per stratum is indistinguishable from a
five-row answer over a corpus that held five rows.

The counters in ``"observed_resolution"`` are the only things that tell those
apart, which is why they are on the answer and not behind a verbosity switch:

* ``"ranks_pulled"`` — how far down each producer's ranking the fusion walked;
* ``"exclusion_lookups"`` — the point queries it spent settling finality, a
  different read of a different question and never added into the rank;
* ``"rows_materialised"`` — the rows the stratum's one read produced: each
  stratum is read on demand, a row per pull, so this is the ranks the fusion
  walked plus the probe row where it read past the planned depth;
* ``"collisions_observed"`` — adjacent ranks the fused score could not tell
  apart, counted by observation.

**Every test here is a TWO-RUN test.** A single run's counter is satisfied by a
counter that is always zero, always the corpus size, or always whatever this
fixture happens to produce. Only a pair of runs over the same surface, differing
in one declared thing, can tell a measurement from a constant — so each counter
below is measured twice and the two values must differ in the direction the
declaration promised.

This file is self-contained on purpose: it can be run alone, with
``uv run pytest tests/test_retrieval_cost_counters.py``, without dragging in the
rest of the retrieval suite.
"""

from __future__ import annotations

from typing import Any

from purrdf import retrieval

EX = "https://example.org/"

NOTE = f"{EX}note"
TITLE = f"{EX}title"
NOTE_PRODUCER = f"{EX}pf/note-search"
TITLE_PRODUCER = f"{EX}pf/title-search"
NOTE_STRATUM = f"{EX}stratum/note"
TITLE_STRATUM = f"{EX}stratum/title"

# The two blocks of the candidate universe the disjoint corpus below really does
# split into. They are the HOST's tags: which entities an index names is a fact
# about the corpus that neither the engine nor the relation can see.
NOTE_DOMAIN = f"{EX}domain/notes"
TITLE_DOMAIN = f"{EX}domain/titles"

STATISTICS: dict[str, Any] = {"source": "host-statistics", "revision": "r1"}

# The decay rule every run here fuses under, and its smoothing constant. Neither
# is a default; they are written once so every run below differs in exactly the
# one thing its test says it differs in.
TRUNCATED = "reciprocal_rank"
K = 60

# The bound every run here searches under. Small, because the whole question is
# how much of a forty-row-per-stratum corpus a three-row answer has to read.
TOP_K = 3

# Every key an ``"observed_resolution"`` entry must carry. Written as the whole
# set rather than as four separate lookups: a key that went missing is the
# failure this file exists to prevent, and a test that checked them one at a
# time would report the first and go quiet about the rest.
OBSERVED_KEYS = frozenset(
    {
        "separates_to",
        "ranks_pulled",
        "collisions_observed",
        "exclusion_lookups",
        "rows_materialised",
    }
)


def _lexical(text: str, predicate: str) -> tuple[Any, ...]:
    """One lexical request term over an indexed predicate."""
    return ("lexical", text, None, predicate)


def _corpus(rows: int = 40) -> str:
    """A corpus whose two indexed predicates name entirely disjoint entities.

    ``ex:n0..`` carry notes and nothing else and ``ex:t0..`` carry titles and
    nothing else, so a producer over each really does draw from its own block —
    which is what makes the declaration in ``_declared`` below TRUE rather than
    merely useful. Both fields hold the same needle at varying term counts, so
    both producers rank every one of their own entities and the scores separate.

    Far larger than ``TOP_K``, because a cost only becomes visible where a
    bounded read could have stopped early.
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


def _undeclared() -> dict[str, tuple[Any, ...]]:
    """Two producers that promise nothing about where their candidates lie.

    The widest possible promise — this producer may name anything — and what
    every producer said before domains existed. It is TRUE of this corpus too:
    saying "anything" is never false.
    """
    return {
        NOTE_PRODUCER: (NOTE_STRATUM, NOTE, "any"),
        TITLE_PRODUCER: (TITLE_STRATUM, TITLE, "any"),
    }


def _declared() -> dict[str, tuple[Any, ...]]:
    """The same two producers, each declaring the one block it draws from."""
    return {
        NOTE_PRODUCER: (NOTE_STRATUM, NOTE, "any", [NOTE_DOMAIN]),
        TITLE_PRODUCER: (TITLE_STRATUM, TITLE, "any", [TITLE_DOMAIN]),
    }


def _search(
    corpus: str,
    producers: dict[str, tuple[Any, ...]],
    weight: int,
) -> dict[str, Any]:
    """One run of the whole ladder, with everything but the two bits under test
    held fixed."""
    return retrieval.search(
        corpus,
        [_lexical("quick", NOTE), _lexical("quick", TITLE)],
        text_producers=producers,
        weights={NOTE_STRATUM: weight, TITLE_STRATUM: weight},
        statistics=STATISTICS,
        k=K,
        decay=TRUNCATED,
        top_k=TOP_K,
    )


def _ranking(answer: dict[str, Any]) -> list[tuple[str, str]]:
    """An answer's rows and scores, as one comparable value."""
    return [(row["entity"], row["score"]) for row in answer["rows"]]


def _counter(answer: dict[str, Any], stratum: str, name: str) -> int:
    """One counter of one stratum, with the absence refused.

    ``"rows_materialised"`` is ``None`` for a stream with no materialised read
    behind it, which is never one this module builds — every stratum here is read
    through the executor. An absence reaching this helper means the surface
    stopped reporting something it can report, so it fails here rather than
    being compared as a zero.
    """
    observed = answer["observed_resolution"][stratum]
    assert OBSERVED_KEYS <= set(observed), (
        f"the observed resolution for {stratum} is missing "
        f"{sorted(OBSERVED_KEYS - set(observed))}"
    )
    value = observed[name]
    assert value is not None, (
        f"{stratum} reported no {name}, and every stratum of a run this module "
        "builds has a read to count"
    )
    return int(value)


def _probe(answer: dict[str, Any], stratum: str) -> int:
    """The probe row a stratum's read produced: one where the read reached past
    its planned depth — its status says the depth stopped it — and none
    otherwise."""
    return 1 if answer["statuses"][stratum]["status"] == "depth_reached" else 0


def test_the_read_counters_move_between_a_cheap_run_and_a_corpus_cost_run() -> None:
    """The same answer, twice, for two very different prices.

    Two runs over one corpus with one request, differing in exactly one thing:
    whether the two producers DECLARE that their candidates lie in disjoint
    blocks. The declaration is true either way — "anything" is never false — so
    the answer cannot move, and the only question is what each run paid.

    Without the declaration a candidate one producer named is not final while
    the other, which might still name it, is open. With it, fusion can certify
    without waiting. That is the difference, and the counters are where it shows
    up: ``"ranks_pulled"`` for how far the fusion walked, and
    ``"rows_materialised"`` for what the reads behind it returned.

    A counter that reported the same number for both runs would not be measuring
    the read at all, and every claim anybody made from it would be a claim about
    a constant.
    """
    corpus = _corpus()
    cheap = _search(corpus, _declared(), retrieval.SCALE)
    costly = _search(corpus, _undeclared(), retrieval.SCALE)

    # The premise: the two runs return the identical answer, so nothing below is
    # a difference in what was asked for.
    assert _ranking(cheap) == _ranking(costly), (
        "a declaration licenses a shorter read, and never a different answer"
    )
    assert len(cheap["rows"]) == TOP_K

    for stratum in (NOTE_STRATUM, TITLE_STRATUM):
        cheap_ranks = _counter(cheap, stratum, "ranks_pulled")
        costly_ranks = _counter(costly, stratum, "ranks_pulled")
        assert cheap_ranks > 0 and costly_ranks > 0, (
            f"{stratum}: a run that pulled no rank measured nothing"
        )
        assert cheap_ranks < costly_ranks, (
            f"{stratum}: `ranks_pulled` did not move with the cost of the "
            f"answer ({cheap_ranks} against {costly_ranks}), so it is not "
            "measuring the read"
        )

        cheap_rows = _counter(cheap, stratum, "rows_materialised")
        costly_rows = _counter(costly, stratum, "rows_materialised")
        assert cheap_rows > 0 and costly_rows > 0, (
            f"{stratum}: a read that returned no row is not a read this fixture "
            "takes"
        )
        assert cheap_rows < costly_rows, (
            f"{stratum}: the read-work figure did not move with the cost of the "
            f"answer ({cheap_rows} against {costly_rows}) — which is exactly the "
            "number a headline about materialised rows is a headline about"
        )

        # The read produced exactly what the fusion walked: each stratum is one
        # read taken a row per pull, so its rows are its ranks, plus the probe
        # row only where the fusion read past the planned depth. A read begun
        # again, or read ahead of the fusion, would show here as rows the ranks
        # do not account for.
        assert cheap_rows == cheap_ranks + _probe(cheap, stratum), stratum
        assert costly_rows == costly_ranks + _probe(costly, stratum), stratum

    # And the exact values, per configuration, as a pair: the declared run reads
    # three rows per stratum for a three-row answer, and the undeclared run —
    # which settles finality by asking — reads each stratum's forty once.
    assert [
        (
            _counter(cheap, stratum, "ranks_pulled"),
            _counter(cheap, stratum, "rows_materialised"),
        )
        for stratum in (NOTE_STRATUM, TITLE_STRATUM)
    ] == [(3, 3), (3, 3)]
    assert [
        (
            _counter(costly, stratum, "ranks_pulled"),
            _counter(costly, stratum, "rows_materialised"),
        )
        for stratum in (NOTE_STRATUM, TITLE_STRATUM)
    ] == [(40, 40), (40, 40)]


def test_the_exclusion_lookup_counter_moves_with_what_the_producers_declared() -> None:
    """Asking is a cost, and it is counted apart from the ranks.

    An exclusion lookup is a point query: *do you hold this one*, asked of a
    producer that has not named the candidate in hand. It settles finality where
    a block declaration cannot, and it is a read the answer paid for — so it is
    counted, in its own field, and never folded into ``"ranks_pulled"``.

    The two runs are the same pair as above. The undeclared run has no way to
    settle finality except by asking, so it asks; the declared run has the
    answer in the declaration and asks nothing. A counter that reported the same
    number for both — or zero for both — would be measuring nothing.
    """
    corpus = _corpus()
    asking = _search(corpus, _undeclared(), retrieval.SCALE)
    settled = _search(corpus, _declared(), retrieval.SCALE)

    asked = sum(
        _counter(asking, stratum, "exclusion_lookups")
        for stratum in (NOTE_STRATUM, TITLE_STRATUM)
    )
    unasked = sum(
        _counter(settled, stratum, "exclusion_lookups")
        for stratum in (NOTE_STRATUM, TITLE_STRATUM)
    )

    assert asked > 0, (
        "the undeclared run cannot settle finality from a declaration, so it "
        "must have asked — a zero here is a counter that is not counting"
    )
    assert unasked == 0, (
        "the declared run has its answer in the declaration, so it must ask "
        f"nothing, and it reports {unasked}"
    )
    assert asked != unasked

    # And the lookups really are counted APART from the ranks: the point queries
    # are not in the rank counter, which would report this stratum as having been
    # read deeper than it was.
    for stratum in (NOTE_STRATUM, TITLE_STRATUM):
        ranks = _counter(asking, stratum, "ranks_pulled")
        rows = _counter(asking, stratum, "rows_materialised")
        assert ranks <= rows, (
            f"{stratum}: the ranks walked ({ranks}) exceed the rows the read "
            f"returned ({rows}), which is what folding lookups into the rank "
            "would look like"
        )


def test_the_collision_counter_moves_with_the_law_and_not_with_the_corpus() -> None:
    """The counter that is a fact about the profile, measured by observation.

    ``"collisions_observed"`` counts adjacent ranks whose contributions this run
    could not tell apart. It is counted on the comparison the fusion actually
    performs, not inferred from a depth against a bound — so the two runs here
    hold the corpus, the request and the bound fixed and change only the WEIGHT,
    which is the thing the law's resolution depends on.

    At one whole unit the truncated rule separates adjacent ranks far past
    anything this corpus can reach, so nothing collides. At one raw unit — the
    smallest weight there is — every contribution truncates to the same value
    from the first pair on, so every adjacent pair collides. Nothing is wrong in
    the second run: those ranks are ordered by the tie-break's later keys
    instead. What would be wrong is a counter that said the same thing about
    both.
    """
    corpus = _corpus()
    separating = _search(corpus, _declared(), retrieval.SCALE)
    colliding = _search(corpus, _declared(), 1)

    for stratum in (NOTE_STRATUM, TITLE_STRATUM):
        # The premise: both runs walked far enough for an adjacent pair to
        # exist, or neither could have collided and the pair proves nothing.
        assert _counter(separating, stratum, "ranks_pulled") >= 2, stratum
        assert _counter(colliding, stratum, "ranks_pulled") >= 2, stratum

        assert _counter(separating, stratum, "collisions_observed") == 0, (
            f"{stratum}: this law separates every rank this run reached, so an "
            "observed collision would contradict it"
        )
        assert _counter(colliding, stratum, "collisions_observed") > 0, (
            f"{stratum}: at one raw unit every contribution truncates to the "
            "same value, so a zero here is a counter stuck at zero"
        )

    # And the separating run's own `"separates_to"` agrees with the zero above:
    # the law it fused under really does separate past what it read.
    for stratum in (NOTE_STRATUM, TITLE_STRATUM):
        separates_to = separating["observed_resolution"][stratum]["separates_to"]
        pulled = _counter(separating, stratum, "ranks_pulled")
        assert separates_to is None or separates_to >= pulled, (
            f"{stratum}: the law stops separating at {separates_to} inside a "
            f"read of {pulled} ranks, so the zero above is not derivable"
        )
