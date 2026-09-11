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

Not to be confused with `test_entail_regimes.py`, which covers the same refusal on
the entailment-regime closure surface (`purrdf.entail.materialize_nt` and friends) —
a different call site sharing the same underlying law.
"""

from __future__ import annotations

import pytest

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
