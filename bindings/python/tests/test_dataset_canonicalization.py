# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""`RdfDataset.to_nquads()` refuses a reserved-vocabulary IRI as a `ValueError`.

`to_nquads()` serializes a frozen dataset through the typed, non-panicking
`try_canonicalize_flat_view` entry point (`bindings/python/src/py_gts_dataset.rs`).
The dataset it serializes is wholly caller-supplied — arbitrary bytes handed to the
`RdfDataset` constructor — so a document carrying an IRI in the canonicalization
overlay's own reserved namespace (`urn:purrdf:rdfc:`; see
`crates/rdf-core/src/ir/canon.rs`) must come back as a `ValueError` naming the
refusal, never abort the process. Before this surface routed through the typed
entry point it did exactly that.

`Dataset.canonicalize()` (`bindings/python/src/py_store/store.rs` +
`bindings/python/src/py_store/canon.rs`) carries the same law over the MUTABLE
quad-store `Dataset`: it used to call panicking `purrdf_core::canonicalize` /
`.expect(...)` directly, so a caller-supplied reserved-vocabulary quad set aborted
the process instead of raising. It now routes through `purrdf_core::try_canonicalize`
and maps the refusal to `ValueError`, exactly like `to_nquads()`.

Not to be confused with `test_entail_regimes.py`, which covers the same refusal on
the entailment-regime closure surface (`purrdf.entail.materialize_nt` and friends) —
a different call site sharing the same underlying law.
"""

from __future__ import annotations

import pytest

import purrdf
from purrdf import RdfDataset, RdfFormat

# `<urn:purrdf:rdfc:reifies>` as a predicate whose OBJECT is a plain IRI (not a
# quoted triple term) is not one of the two exact shapes the canonicalization
# overlay itself lowers, so it is refused rather than silently accepted — accepting
# it would let a structurally different graph canonicalize to the exact bytes a
# genuine reifier binding would.
RESERVED_PREDICATE_PLAIN_OBJECT = (
    "<http://example.org/r> <urn:purrdf:rdfc:reifies> <http://example.org/o> .\n"
)

# The valid neighbour: an ORDINARY predicate adjacent to the reserved namespace (a
# DIFFERENT `urn:purrdf:` sub-namespace, not `urn:purrdf:rdfc:`) in the exact same
# shape. Over-refusal is the mirror bug of silent acceptance, so this must still
# serialize, with the IRI surviving byte-for-byte into the output.
NEIGHBOURING_ORDINARY_PREDICATE = (
    "<http://example.org/r> <urn:purrdf:other:annotation> <http://example.org/o> .\n"
)


def test_to_nquads_raises_value_error_naming_the_reserved_iri() -> None:
    dataset = RdfDataset(RESERVED_PREDICATE_PLAIN_OBJECT, RdfFormat.N_TRIPLES)
    with pytest.raises(ValueError, match="urn:purrdf:rdfc:reifies"):
        dataset.to_nquads()


def test_to_nquads_still_serializes_an_ordinary_neighbouring_iri() -> None:
    """The refusal above is not an over-refusal of an ordinary, unrelated IRI."""
    dataset = RdfDataset(NEIGHBOURING_ORDINARY_PREDICATE, RdfFormat.N_TRIPLES)
    out = dataset.to_nquads()
    assert "urn:purrdf:other:annotation" in out, out


def _quad(subject: str, predicate: str, obj: str) -> purrdf.Quad:
    return purrdf.Quad(
        purrdf.NamedNode(subject), purrdf.NamedNode(predicate), purrdf.NamedNode(obj)
    )


def test_dataset_canonicalize_raises_value_error_naming_the_reserved_iri() -> None:
    """`Dataset.canonicalize()`'s content is wholly caller-supplied too."""
    dataset = purrdf.Dataset(
        [_quad("http://example.org/r", "urn:purrdf:rdfc:reifies", "http://example.org/o")]
    )
    with pytest.raises(ValueError, match="urn:purrdf:rdfc:reifies"):
        dataset.canonicalize(purrdf.CanonicalizationAlgorithm.RDFC_1_0)


def test_dataset_canonicalize_still_admits_an_ordinary_neighbouring_dataset() -> None:
    """The refusal above is not an over-refusal of an ordinary, unrelated IRI.

    The dataset is left USABLE after the refusal above, too: this neighbour reuses
    the same object under construction, one canonicalization call each.
    """
    dataset = purrdf.Dataset(
        [_quad("http://example.org/r", "urn:purrdf:other:annotation", "http://example.org/o")]
    )
    dataset.canonicalize(purrdf.CanonicalizationAlgorithm.RDFC_1_0)
    assert len(dataset) == 1


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


def _canonical_rows(document: str, algorithm: object) -> list[str]:
    """The canonical form of *document* under *algorithm*, as comparable rows.

    `purrdf.parse` rather than a store load, because only it keeps the document's
    blank labels verbatim — and the labels are the whole subject of the exercise.
    """
    dataset = purrdf.Dataset(purrdf.parse(document, RdfFormat.N_TRIPLES))
    dataset.canonicalize(algorithm)
    return sorted(f"{quad.subject} <{quad.predicate}> {quad.object}" for quad in dataset)


def test_two_isomorphic_graphs_canonicalize_to_one_form_under_rdfc_1_0() -> None:
    """Different blank labels, same graph, same canonical bytes.

    This is the property RDFC-1.0 exists to provide and the only one a consumer
    can build on: a digest, a diff or a signature over a graph is meaningless
    unless relabelling the blanks leaves it alone. The canonical labels appear in
    the output too, because a form that kept the document's own labels would be
    stable only by accident of input.
    """
    first = _canonical_rows(ISOMORPHIC_A, purrdf.CanonicalizationAlgorithm.RDFC_1_0)
    second = _canonical_rows(ISOMORPHIC_B, purrdf.CanonicalizationAlgorithm.RDFC_1_0)

    assert first == second, "isomorphic graphs canonicalize identically"
    assert all("_:c14n" in row for row in first), (
        f"every blank carries a canonical label, not the document's: {first}"
    )
    for original in ("_:a", "_:b", "_:x", "_:y"):
        assert not any(f"{original} " in row for row in first), (
            f"{original} is an input label and must not survive: {first}"
        )

    # The neighbouring graph that is a DIFFERENT graph still canonicalizes to a
    # different form. Agreement that swallowed this would be information loss
    # wearing determinism.
    assert (
        _canonical_rows(NON_ISOMORPHIC, purrdf.CanonicalizationAlgorithm.RDFC_1_0)
        != first
    )


def test_the_unstable_algorithm_is_self_consistent() -> None:
    """The unstable algorithm makes no cross-version promise, but it is a function.

    "Unstable" names what it does not promise — that the labels will be the same
    in a later release — not a licence to answer differently twice in one process.
    A caller using it to deduplicate within a run depends on exactly this, and
    nothing else states it.
    """
    algorithm = purrdf.CanonicalizationAlgorithm.UNSTABLE
    first = _canonical_rows(ISOMORPHIC_A, algorithm)
    assert first == _canonical_rows(ISOMORPHIC_A, algorithm)
    assert first, "the fixture canonicalizes to something"

    # And it still tells two different graphs apart, which is the floor any
    # labelling has to clear to be usable for anything.
    assert _canonical_rows(NON_ISOMORPHIC, algorithm) != first
