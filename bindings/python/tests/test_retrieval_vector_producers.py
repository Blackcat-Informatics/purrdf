# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Vector producers on the Python retrieval surface, fused with a text producer.

``hnsw_producers`` and ``knn_producers`` register the two shipped nearest-
neighbour relations — the approximate graph search and the exact scan — over a
host's own vectors, beside the text producers, in one request. The
configuration this file measures is the one a multi-modal request exists for,
and the one a bound is hardest to earn in: a text stratum and a vector stratum
that DECLARE ONE SHARED BLOCK and hold disjoint candidates. No candidate either
stream names is final while the other is open, because the other declares the
same block and might still name it; only learning that a stream will NEVER name
a candidate resolves it, and every relation here answers that lookup out of its
own index.

The fixture is the one ``crates/purrdf/tests/multimodal_exclusion_lookup.rs``
and ``multimodal_exact_knn.rs`` build on the Rust surface — the same corpus, the
same vectors, the same law — so the counters pinned here are the counters those
files pin, reached through the public Python API.

Every IRI below is this file's own, in the host's role. PurRDF mints no
vocabulary.
"""

from __future__ import annotations

from typing import Any

import pytest

from purrdf import retrieval

EX = "https://example.org/"

NOTE = f"{EX}note"
TEXT_PRODUCER = f"{EX}pf/search"
HNSW_PRODUCER = f"{EX}pf/neighbours"
KNN_PRODUCER = f"{EX}pf/nearest"
TEXT_STRATUM = f"{EX}stratum/lexical"
VECTOR_STRATUM = f"{EX}stratum/vector"
# The one block both producers declare in the configuration under test. Sharing
# it is what removes the planner's disjointness argument and leaves finality as
# the only thing that can end the read.
SHARED_BLOCK = f"{EX}domain/shared"
# The two blocks the same candidates really do split into, for the control that
# declares them.
TEXT_BLOCK = f"{EX}domain/text"
VECTOR_BLOCK = f"{EX}domain/vector"

NEEDLE = "alpha beta"
# How many documents the needle reaches, and how many rows the vector space
# holds. Deep enough that a full read and a bounded one are different numbers.
CORPUS = 80
DIMS = 8
TOP_K = 5
K = 60
TRUNCATED = "reciprocal_rank"
# An HNSW beam as wide as the corpus, so the approximate read reaches the depth
# the plan asks for and the runs below differ in what was declared rather than in
# how far a beam happened to get.
HNSW_PARAMS = (8, 16, 64, CORPUS)
GUARD = (CORPUS, CORPUS)

WEIGHTS = {TEXT_STRATUM: retrieval.SCALE, VECTOR_STRATUM: retrieval.SCALE}
# Each stratum really holds the rows its producer declares; no selectivity is
# measured.
STATISTICS: dict[str, Any] = {
    "source": "example-statistics",
    "revision": "r1",
    "cardinality": {TEXT_STRATUM: 2 * CORPUS, NOTE: 2 * CORPUS, VECTOR_STRATUM: CORPUS},
}


def _text_subject(at: int) -> str:
    return f"{EX}doc/text/{at}"


def _vector_term(at: int) -> str:
    return f"{EX}doc/vec/{at}"


def _vectors() -> list[list[float]]:
    """``CORPUS`` rows of ``DIMS`` components in [-1, 1), none zero.

    splitmix64 over one running state, row-major — the generator the Rust
    fixtures spell — so the Python space holds bit-identical vectors.
    """
    mask = (1 << 64) - 1
    state = 0x51DE_0000_1234_ABCD
    rows = []
    for _ in range(CORPUS):
        row = []
        for _ in range(DIMS):
            state = (state + 0x9E37_79B9_7F4A_7C15) & mask
            z = state
            z = ((z ^ (z >> 30)) * 0xBF58_476D_1CE4_E5B9) & mask
            z = ((z ^ (z >> 27)) * 0x94D0_49BB_1331_11EB) & mask
            z ^= z >> 31
            value = ((z >> 11) / (1 << 53)) * 2.0 - 1.0
            row.append(0.125 if value == 0.0 else value)
        rows.append(row)
    return rows


ROWS = [(_vector_term(at), vector) for at, vector in enumerate(_vectors())]


def _corpus() -> str:
    """The document the text index is built from.

    ``CORPUS`` documents the needle reaches, and one document per VECTOR term
    whose text shares no term with the needle. The second group makes a
    text-side lookup about a vector candidate a real search of the index's own
    dictionary, and it never makes a vector term a text candidate: a document
    carrying no needle term is in no candidate set at any depth. The vector
    space, symmetrically, holds no row for any text subject.
    """
    lines = [
        f'<{_text_subject(at)}> <{NOTE}> '
        f'"alpha beta gamma {("alpha " * (at % 4 + 1)).strip()}" .'
        for at in range(CORPUS)
    ]
    lines += [
        f'<{_vector_term(at)}> <{NOTE}> "zulu yankee xray whiskey" .'
        for at in range(CORPUS)
    ]
    return "\n".join(lines)


DATA = _corpus()
REQUEST = [("lexical", NEEDLE, None, NOTE), ("entity", f"<{_vector_term(0)}>")]


def _vector_producers(
    kind: str, domains: list[str] | None, rows: list[Any] = ROWS
) -> dict[str, dict[str, tuple[Any, ...]]]:
    """The one vector producer of ``kind``, as the keyword that registers it."""
    if kind == "hnsw":
        spec: tuple[Any, ...] = (
            VECTOR_STRATUM,
            rows,
            "squared_euclidean",
            GUARD,
            HNSW_PARAMS,
            domains,
            (None, None),
            None,
        )
        return {"hnsw_producers": {HNSW_PRODUCER: spec}}
    spec = (
        VECTOR_STRATUM,
        rows,
        "squared_euclidean",
        GUARD,
        domains,
        (None, None),
        (None, None),
    )
    return {"knn_producers": {KNN_PRODUCER: spec}}


def _search(
    kind: str,
    text_domains: list[str] | None,
    vector_domains: list[str] | None,
    top_k: int = TOP_K,
) -> dict[str, Any]:
    """One fused text + vector search, differing between calls only in what the
    two producers declare and in the bound."""
    return retrieval.search(
        DATA,
        REQUEST,
        text_producers={TEXT_PRODUCER: (TEXT_STRATUM, NOTE, "any", text_domains)},
        weights=WEIGHTS,
        statistics=STATISTICS,
        k=K,
        decay=TRUNCATED,
        top_k=top_k,
        data_format="ntriples",
        **_vector_producers(kind, vector_domains),
    )


def _ranking(answer: dict[str, Any]) -> list[tuple[str, str, tuple[Any, ...]]]:
    """An answer's rows, scores and provenance, as one comparable value.

    The threshold witness is left out: it records the threshold in force when a
    row was certified, and certifying earlier is what a shorter read does.
    """
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


def _counters(answer: dict[str, Any]) -> dict[str, tuple[int, int, int]]:
    """``(ranks_pulled, exclusion_lookups, rows_materialised)`` per stratum."""
    return {
        stratum: (
            observed["ranks_pulled"],
            observed["exclusion_lookups"],
            observed["rows_materialised"],
        )
        for stratum, observed in answer["observed_resolution"].items()
    }


@pytest.mark.parametrize("kind", ["hnsw", "knn"])
def test_a_shared_block_read_is_bounded_by_lookups_on_both_modalities(kind: str) -> None:
    """One text stratum and one vector stratum, sharing a block and holding
    disjoint candidates: the full read's answer, out of a strictly shorter read,
    paid for with exclusion lookups on BOTH strata.

    Three runs, one answer:

    * the configuration under test declares one shared block on both producers,
      so only a lookup can settle that a stream will never name a candidate;
    * the lookup-free control declares the two blocks the candidates really lie
      in, so the declaration settles every verdict a lookup would have and
      nothing is asked;
    * the full read declares the same true blocks and asks for every candidate,
      so both streams are read to their ends and nothing is asked — the oracle
      the other two answers are prefixes of.

    All of it is asserted together, because each half alone passes over a defect
    the other catches: "fewer ranks" alone is satisfied by a run that stopped
    early and answered wrongly, "the same answer" alone by lookups nobody made,
    and "lookups happened" alone by a vector producer that was never asked.
    """
    shared = _search(kind, [SHARED_BLOCK], [SHARED_BLOCK])
    control = _search(kind, [TEXT_BLOCK], [VECTOR_BLOCK])
    full = _search(kind, [TEXT_BLOCK], [VECTOR_BLOCK], top_k=2 * CORPUS)

    # 1. One answer, and it is the full read's first five rows — text and vector
    #    candidates interleaved, so both modalities really are in it.
    reference = _ranking(full)[:TOP_K]
    assert len(full["rows"]) == 2 * CORPUS
    assert _ranking(shared) == reference
    assert _ranking(control) == reference
    assert [row["entity"] for row in shared["rows"]] == [
        f"<{_text_subject(1)}>",
        f"<{_vector_term(0)}>",
        f"<{_text_subject(13)}>",
        f"<{_vector_term(46)}>",
        f"<{_text_subject(17)}>",
    ]

    # 2. The shared-block read, exactly. Each stratum is one read taken a row per
    #    pull, and the fusion stops where the fifth row crosses both heads'
    #    threshold, at rank 66: sixty-six ranks, sixty-six rows, and sixty-five
    #    lookups per stratum — one per candidate the other stratum named whose
    #    fate a verdict could still change. The same numbers the Rust surface
    #    pins for this fixture.
    assert _counters(shared) == {
        TEXT_STRATUM: (66, 65, 66),
        VECTOR_STRATUM: (66, 65, 66),
    }, _counters(shared)
    assert {entry["status"] for entry in shared["statuses"].values()} == {
        "ceiling_reached"
    }, shared["statuses"]
    assert shared["exclusion_bases"] == {
        TEXT_STRATUM: "membership",
        VECTOR_STRATUM: "membership",
    }
    assert shared["domains"] == {
        TEXT_STRATUM: [SHARED_BLOCK],
        VECTOR_STRATUM: [SHARED_BLOCK],
    }

    # 3. The lookup-free control asks nothing on either stratum and reads the
    #    four-row prefix the declaration licenses.
    assert _counters(control) == {
        TEXT_STRATUM: (4, 0, 4),
        VECTOR_STRATUM: (4, 0, 4),
    }, _counters(control)
    assert control["domains"] == {
        TEXT_STRATUM: [TEXT_BLOCK],
        VECTOR_STRATUM: [VECTOR_BLOCK],
    }

    # 4. The full read reads every row of both strata and asks nothing. The text
    #    stream ran out; the vector stream was handed the planned depth as its
    #    own neighbour count, which sits on its declared bound, so its read ends
    #    at that bound with the row past it never asked for.
    assert _counters(full) == {
        TEXT_STRATUM: (CORPUS, 0, CORPUS),
        VECTOR_STRATUM: (CORPUS, 0, CORPUS),
    }, _counters(full)
    assert full["statuses"] == {
        TEXT_STRATUM: {"status": "exhausted", "rows_emitted": CORPUS},
        VECTOR_STRATUM: {"status": "row_bound_reached", "rank": CORPUS},
    }

    # 5. And the shared-block read is strictly shorter than the full read on
    #    EACH stratum: the lookups bought ranks on both modalities, not one.
    for stratum in (TEXT_STRATUM, VECTOR_STRATUM):
        assert (
            shared["observed_resolution"][stratum]["ranks_pulled"]
            < full["observed_resolution"][stratum]["ranks_pulled"]
        ), stratum


def test_the_two_vector_producers_differ_in_what_they_promise_and_not_in_what_they_rank() -> (
    None
):
    """The configuration A/B: an approximate and an exact producer over the same
    rows, with a beam as wide as the space, rank the same neighbours — and only
    the approximate one says its search may have missed some.

    The ranking is also held to an oracle computed here, in Python, from the rows
    themselves: squared Euclidean distance from the seed, nearest first, the
    host's row order breaking a tie. A vector producer wired to the wrong rows,
    or not wired at all, cannot pass it.
    """
    hnsw = _search("hnsw", [SHARED_BLOCK], [SHARED_BLOCK], top_k=2 * CORPUS)
    knn = _search("knn", [SHARED_BLOCK], [SHARED_BLOCK], top_k=2 * CORPUS)

    def vector_ranks(answer: dict[str, Any]) -> list[tuple[str, int]]:
        return sorted(
            (
                (row["entity"], c["rank"])
                for row in answer["rows"]
                for c in row["contributions"]
                if c["stratum"] == VECTOR_STRATUM
            ),
            key=lambda pair: pair[1],
        )

    seed = ROWS[0][1]
    distances = sorted(
        (sum((a - b) ** 2 for a, b in zip(vector, seed, strict=True)), at)
        for at, (_, vector) in enumerate(ROWS)
    )
    expected = [
        (f"<{_vector_term(at)}>", rank)
        for rank, (_, at) in enumerate(distances, start=1)
    ]
    assert vector_ranks(knn) == expected
    assert vector_ranks(hnsw) == expected
    assert _ranking(hnsw) == _ranking(knn)

    # What differs is the promise, and it is the relation's own.
    assert knn["exactness"] == {
        "exact": True,
        "deficit": [],
        "inflation": [],
        "unbounded": [],
    }
    assert knn["fidelities"][VECTOR_STRATUM] == {
        "completeness": "complete",
        "order": "faithful",
    }
    assert hnsw["exactness"]["exact"] is False
    assert hnsw["exactness"]["deficit"] == [VECTOR_STRATUM]
    assert hnsw["fidelities"][VECTOR_STRATUM]["completeness"] == "lossy"
    assert hnsw["fidelities"][VECTOR_STRATUM]["completeness_evidence"]
    assert hnsw["fidelities"][TEXT_STRATUM] == knn["fidelities"][TEXT_STRATUM]


@pytest.mark.parametrize("kind", ["hnsw", "knn"])
def test_a_vector_producer_attests_its_space_and_a_host_generation_replaces_it(
    kind: str,
) -> None:
    """Silence on the attestation delegates to the relation, which attests the
    content digest of the space this call built; a declared generation replaces
    it, and an incompleteness is reported beside it."""
    silent = _search(kind, [SHARED_BLOCK], [SHARED_BLOCK])
    digest = silent["attestations"][VECTOR_STRATUM]["generation"]
    assert isinstance(digest, str) and len(digest) == 64
    assert silent["attestations"][VECTOR_STRATUM]["incomplete"] is None
    # The same rows, built again in another call, are the same generation.
    assert (
        _search(kind, [SHARED_BLOCK], [SHARED_BLOCK])["attestations"][VECTOR_STRATUM][
            "generation"
        ]
        == digest
    )

    producers = _vector_producers(kind, [SHARED_BLOCK])
    ((producer, spec),) = next(iter(producers.values())).items()
    attestation_at = 6 if kind == "hnsw" else 5
    attested = list(spec)
    attested[attestation_at] = ("space-7", "shard 2 of 2 is still embedding")
    answer = retrieval.search(
        DATA,
        REQUEST,
        text_producers={TEXT_PRODUCER: (TEXT_STRATUM, NOTE, "any", [SHARED_BLOCK])},
        weights=WEIGHTS,
        statistics=STATISTICS,
        k=K,
        decay=TRUNCATED,
        top_k=TOP_K,
        data_format="ntriples",
        **{next(iter(producers)): {producer: tuple(attested)}},
    )
    assert answer["attestations"][VECTOR_STRATUM] == {
        "generation": "space-7",
        "incomplete": "shard 2 of 2 is still embedding",
    }
    assert VECTOR_STRATUM in answer["exactness"]["deficit"]
    assert _ranking(answer) == _ranking(silent)


def _refusal(
    kind: str, index: int, value: Any
) -> dict[str, dict[str, tuple[Any, ...]]]:
    """The vector producer of ``kind`` with position ``index`` replaced."""
    producers = _vector_producers(kind, None)
    ((keyword, entries),) = producers.items()
    ((producer, spec),) = entries.items()
    changed = list(spec)
    changed[index] = value
    return {keyword: {producer: tuple(changed)}}


def _plan(**producers: Any) -> dict[str, Any]:
    """Plan the fixture request against a text producer and ``producers``."""
    return retrieval.plan(
        DATA,
        REQUEST,
        text_producers=producers.pop(
            "text_producers", {TEXT_PRODUCER: (TEXT_STRATUM, NOTE, "any")}
        ),
        statistics=STATISTICS,
        top_k=TOP_K,
        data_format="ntriples",
        **producers,
    )


@pytest.mark.parametrize("kind", ["hnsw", "knn"])
@pytest.mark.parametrize(
    ("index", "refused", "valid", "message"),
    [
        (1, [], ROWS[:1], "`rows` is empty"),
        (
            1,
            [(_vector_term(0), [1.0, 2.0]), (_vector_term(1), [1.0])],
            [(_vector_term(0), [1.0, 2.0]), (_vector_term(1), [1.0, 3.0])],
            "and row 0 carries 2",
        ),
        (
            1,
            [(_vector_term(0), [1.0, 2.0]), (_vector_term(1), [float("nan"), 3.0])],
            [(_vector_term(0), [1.0, 2.0]), (_vector_term(1), [1e10, 3.0])],
            "non-finite|not finite",
        ),
        (
            1,
            [(_vector_term(0), [1.0, 2.0]), (_vector_term(0), [2.0, 3.0])],
            [(_vector_term(0), [1.0, 2.0]), (_vector_term(1), [1.0, 2.0])],
            "bound to two different rows",
        ),
        (1, [("not an iri", [1.0, 2.0])], [(_vector_term(0), [1.0, 2.0])], "row 0 names"),
        (2, "euclidean", "negative_dot", "unknown metric"),
        (3, (CORPUS, 0), (CORPUS, 1), "can never answer"),
        (3, (CORPUS - 1, CORPUS), (CORPUS, 1), "more than the"),
    ],
)
def test_a_vector_space_refusal_names_the_producer_and_its_neighbour_registers(
    kind: str, index: int, refused: Any, valid: Any, message: str
) -> None:
    """Every refusal a vector space makes, beside the valid neighbour one step
    from it — which must plan, because a refusal that also took the neighbour
    would be an over-refusal nothing else here would catch."""
    subject = HNSW_PRODUCER if kind == "hnsw" else KNN_PRODUCER
    with pytest.raises(ValueError, match=message) as refused_error:
        _plan(**_refusal(kind, index, refused))
    assert subject in str(refused_error.value)
    planned = _plan(**_refusal(kind, index, valid))
    assert planned["plan_id"]


def test_a_zero_vector_is_refused_under_cosine_and_ranked_under_the_other_two() -> None:
    """The origin has no direction, so its cosine distance is undefined rather
    than large; the same rows rank under a metric that divides by nothing."""
    rows = [(_vector_term(0), [0.0, 0.0]), (_vector_term(1), [1.0, 2.0])]
    for kind in ("hnsw", "knn"):
        producers = _refusal(kind, 1, rows)
        ((keyword, entries),) = producers.items()
        ((producer, spec),) = entries.items()
        cosine = list(spec)
        cosine[2] = "cosine"
        with pytest.raises(ValueError, match="zero L2 norm"):
            _plan(**{keyword: {producer: tuple(cosine)}})
        for metric in ("negative_dot", "squared_euclidean"):
            other = list(spec)
            other[2] = metric
            assert _plan(**{keyword: {producer: tuple(other)}})["plan_id"]


def test_an_invalid_graph_parameter_set_is_refused_and_a_valid_one_is_not() -> None:
    with pytest.raises(ValueError, match=HNSW_PRODUCER):
        _plan(**_refusal("hnsw", 4, (1, 16, 64, CORPUS)))
    assert _plan(**_refusal("hnsw", 4, (2, 2, 2, 1)))["plan_id"]


def test_an_empty_order_evidence_is_refused_and_prose_or_none_is_not() -> None:
    with pytest.raises(ValueError, match="supplies no evidence"):
        _plan(**_refusal("hnsw", 7, "  "))
    for order in (None, "vectors were quantised to int8 before indexing"):
        assert _plan(**_refusal("hnsw", 7, order))["plan_id"]


@pytest.mark.parametrize("kind", ["hnsw", "knn"])
def test_a_vector_spec_of_the_wrong_width_is_a_type_error_naming_every_position(
    kind: str,
) -> None:
    producers = _vector_producers(kind, None)
    ((keyword, entries),) = producers.items()
    ((producer, spec),) = entries.items()
    with pytest.raises(TypeError, match="every position written"):
        _plan(**{keyword: {producer: spec[:-1]}})
    with pytest.raises(TypeError, match="every position written"):
        _plan(**{keyword: {producer: (*spec, None)}})
    assert _plan(**producers)["plan_id"]


def test_one_stratum_or_one_iri_across_two_maps_is_refused_by_name() -> None:
    """The three maps are one registry. A stratum claimed by a text and a vector
    producer, or one IRI written into two maps, is refused naming both — and the
    same two producers under distinct IRIs and strata register."""
    clash = _vector_producers("knn", None)
    with pytest.raises(ValueError, match="both claim stratum"):
        _plan(
            text_producers={TEXT_PRODUCER: (VECTOR_STRATUM, NOTE, "any")},
            **clash,
        )
    with pytest.raises(ValueError, match="declared twice"):
        _plan(
            text_producers={KNN_PRODUCER: (TEXT_STRATUM, NOTE, "any")},
            **clash,
        )
    both = {**_vector_producers("knn", None), **_vector_producers("hnsw", None)}
    both["hnsw_producers"] = {
        HNSW_PRODUCER: (f"{EX}stratum/approximate", *both["hnsw_producers"][HNSW_PRODUCER][1:])
    }
    planned = _plan(**both)
    assert planned["plan_id"]
    assert _plan(**clash)["plan_id"]
