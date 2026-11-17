# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Python canonical operations preserve explicit policy and owned evidence.

RdfDataset.to_nquads returns an immutable result envelope. Dataset.canonicalize
leaves its original input unchanged on refusal. Both retain the native cause
and finalized receipt JSON when their caller-supplied content is refused.
"""

from __future__ import annotations

import re

import pytest

import purrdf
from purrdf import RdfDataset, RdfFormat

# The 13-node circulant of quadratic residues, in both directions. The labelling
# cannot finish it without a search, so a finite bound of "0" trips the engine's
# typed `limit` refusal; "unlimited" completes it. The two-node cycle below needs
# no search, so the same bound "0" admits it: the refusal is the input's, not the
# bound's.
CIRCULANT = "".join(
    f"_:v{u} <https://example.org/p> _:v{v} .\n_:v{v} <https://example.org/p> _:v{u} .\n"
    for u in range(13)
    for v in range(u + 1, 13)
    if (v - u) % 13 in {1, 3, 4, 9, 10, 12}
)
TWO_CYCLE = "_:a <https://example.org/p> _:b .\n_:b <https://example.org/p> _:a .\n"


def test_to_nquads_raises_value_error_on_the_labelling_limit() -> None:
    dataset = RdfDataset(CIRCULANT, RdfFormat.N_TRIPLES)
    with pytest.raises(ValueError, match="labelling refused") as failure:
        dataset.to_nquads(canonical_search="0").value
    del dataset
    assert failure.value.native_cause.json == failure.value.native_cause_json


def test_to_nquads_still_serializes_inputs_the_bound_admits() -> None:
    """The neighbours of the refusal above: the same bound admits a search-free input,
    and the same input completes under another bound."""
    cycle = RdfDataset(TWO_CYCLE, RdfFormat.N_TRIPLES).to_nquads(canonical_search="0").value
    assert cycle.startswith("_:0 "), cycle
    unlimited = RdfDataset(CIRCULANT, RdfFormat.N_TRIPLES).to_nquads(canonical_search="unlimited").value
    assert unlimited.startswith("_:0 "), unlimited


def test_dataset_canonicalize_raises_value_error_on_the_labelling_limit() -> None:
    dataset = purrdf.Dataset(purrdf.parse(CIRCULANT, RdfFormat.N_TRIPLES))
    original = tuple(str(quad) for quad in dataset)
    with pytest.raises(ValueError, match="labelling refused"):
        dataset.canonicalize(canonical_search="0").value
    assert tuple(str(quad) for quad in dataset) == original, "a refusal leaves the dataset unchanged"
    dataset.canonicalize(canonical_search="unlimited").value
    assert len(dataset) == len(original)


# ── determinism: what canonicalization is FOR ───────────────────────────────────
#
# Everything above is about the refusal. These are about the thing being refused
# on behalf of: a canonical form only earns the name if two documents that are
# the same graph reach the same bytes, and the same document reaches them twice.
# Without a blank node in the fixture no canonical label is ever minted, so the
# refusal tests above cannot witness any of it.

EX = "https://example.org/"

#: Two isomorphic graphs: one two-cycle each, with disjoint blank labels. As RDF
#: graphs they are indistinguishable, and a canonical form has to say so.
ISOMORPHIC_A = f"_:a <{EX}p> _:b .\n_:b <{EX}q> _:a .\n"
ISOMORPHIC_B = f"_:x <{EX}p> _:y .\n_:y <{EX}q> _:x .\n"

#: The neighbouring graph that is NOT isomorphic to either: the same two-cycle
#: shape with one predicate changed, so both edges are `ex:p`. If canonicalization
#: collapsed everything to one form, this would match too — which is how a
#: "deterministic" function that simply discards information passes a
#: same-bytes test.
NON_ISOMORPHIC = f"_:a <{EX}p> _:b .\n_:b <{EX}p> _:a .\n"


def _canonical_rows(document: str, canonical_search: str) -> list[str]:
    """The canonical form of *document* under *canonical_search*, as comparable rows.

    `purrdf.parse` rather than a store load, because only it keeps the document's
    blank labels verbatim — and the labels are the whole subject of the exercise.
    """
    dataset = purrdf.Dataset(purrdf.parse(document, RdfFormat.N_TRIPLES))
    dataset.canonicalize(canonical_search=canonical_search).value
    return sorted(f"{quad.subject} <{quad.predicate}> {quad.object}" for quad in dataset)


def test_two_isomorphic_graphs_canonicalize_to_one_form_under_the_canonical_labelling() -> None:
    """Different blank labels, same graph, same canonical bytes.

    This is the property canonicalization exists to provide and the only one a consumer
    can build on: a digest, a diff or a signature over a graph is meaningless
    unless relabelling the blanks leaves it alone. The canonical labels appear in
    the output too, because a form that kept the document's own labels would be
    stable only by accident of input.
    """
    first = _canonical_rows(ISOMORPHIC_A, "1000000")
    second = _canonical_rows(ISOMORPHIC_B, "1000000")

    assert first == second, "isomorphic graphs canonicalize identically"
    assert all(re.search(r"_:\d+ ", row) for row in first), (
        f"every blank carries a decimal canonical label, not the document's: {first}"
    )
    for original in ("_:a", "_:b", "_:x", "_:y"):
        assert not any(f"{original} " in row for row in first), (
            f"{original} is an input label and must not survive: {first}"
        )

    # The neighbouring graph that is a DIFFERENT graph still canonicalizes to a
    # different form. Agreement that swallowed this would be information loss
    # wearing determinism.
    assert (
        _canonical_rows(NON_ISOMORPHIC, "1000000")
        != first
    )


def test_explicit_policy_grammar_controls_the_python_boundary() -> None:
    """Policy refusal and immutable receipt envelopes belong to this host."""
    dataset = purrdf.Dataset(purrdf.parse(ISOMORPHIC_A, RdfFormat.N_TRIPLES))
    original = tuple(str(quad) for quad in dataset)
    with pytest.raises(TypeError, match="canonical_search"):
        dataset.canonicalize()
    for policy in ("", "01", "-1", "18446744073709551616"):
        with pytest.raises(ValueError) as failure:
            dataset.canonicalize(canonical_search=policy)
        assert failure.value.canonicalization_json == "[]"
        assert tuple(str(quad) for quad in dataset) == original
    for policy in ("1000000", "unlimited"):
        result = dataset.canonicalize(canonical_search=policy)
        assert result.value is None
        with pytest.raises(AttributeError):
            result.canonicalization_json = "[]"
    signal = purrdf.CancellationToken()
    signal.cancel()
    before_stop = tuple(str(quad) for quad in dataset)
    with pytest.raises(ValueError) as failure:
        dataset.canonicalize(canonical_search="unlimited", cancel=signal)
    assert tuple(str(quad) for quad in dataset) == before_stop
    assert failure.value.native_cause.json == failure.value.native_cause_json
