# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: exercise dataset and exception ownership through the actual Python extension.

"""GTS generation selection and structured refusals at the dataset boundary."""

import pytest
from purrdf import RdfDataset, RdfFormat


def test_v2_dataset_roundtrip_and_torn_tail() -> None:
    dataset = RdfDataset(
        """
        <https://example.org/g> {
          _:s <https://example.org/p> <<( _:s <https://example.org/p> _:o )>> .
        }
        <https://example.org/empty> {}
        """,
        RdfFormat.TRIG,
    )
    encoded = dataset.to_gts_v2()
    assert encoded[:3] == bytes([0x87, 1, 2])
    restored = RdfDataset.from_gts(encoded)
    assert restored.to_nquads(canonical_search="unlimited").value == dataset.to_nquads(
        canonical_search="unlimited"
    ).value

    with pytest.raises(ValueError, match="the file ends inside an item") as refused:
        RdfDataset.from_gts(encoded[:-1])
    assert "gts-v2-torn-tail" in refused.value.native_cause_json


def test_dataset_loader_keeps_the_profiled_v1_reader() -> None:
    dataset = RdfDataset(
        "<https://example.org/s> <https://example.org/p> <https://example.org/o> .",
        RdfFormat.N_TRIPLES,
    )
    assert RdfDataset.from_gts(dataset.to_gts()).quad_count() == 1
