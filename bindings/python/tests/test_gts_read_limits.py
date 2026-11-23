# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: this exercises Python keyword admission and typed native exception fields that Rust tests cannot observe.
"""Python GTS byte readers expose the native whole-read contract."""

import pytest

import purrdf


def _container() -> bytes:
    dataset = purrdf.RdfDataset(
        "<https://example.org/s> <https://example.org/p> <https://example.org/o> .\n",
        purrdf.RdfFormat.N_TRIPLES,
    )
    return dataset.to_gts()


@pytest.mark.parametrize(
    "read",
    [purrdf.GtsFoldViewNative.from_bytes, purrdf.gts_relational_rows_from_bytes],
)
def test_byte_readers_accept_exact_input_and_refuse_adjacent_excess(read) -> None:
    data = _container()
    exact = purrdf.GtsReadLimits(encoded_input_bytes=len(data))
    assert read(data, read_limits=exact) is not None
    with pytest.raises(ValueError) as caught:
        read(data, read_limits=purrdf.GtsReadLimits(encoded_input_bytes=len(data) - 1))
    assert caught.value.code == "rdf-ir-gts-read-limit"
    assert caught.value.diagnostic_json


def test_limits_are_discoverable_under_the_gts_namespace() -> None:
    assert purrdf.gts.GtsReadLimits is purrdf.GtsReadLimits
    finite = purrdf.GtsReadLimits().encoded_input_bytes
    assert finite < purrdf.GtsReadLimits.unbounded().encoded_input_bytes


@pytest.mark.parametrize(
    "read",
    [purrdf.gts_columnar_rows_from_bytes, purrdf.gts_columnar_parquet_from_bytes],
)
def test_columnar_byte_readers_apply_explicit_whole_read_limits(read) -> None:
    data = _container()
    with pytest.raises(ValueError) as caught:
        read(
            data,
            blob_limits=purrdf.GtsBlobLimits.unbounded(),
            read_limits=purrdf.GtsReadLimits(encoded_input_bytes=len(data) - 1),
        )
    assert caught.value.code == "rdf-ir-gts-read-limit"
    assert caught.value.diagnostic_json
