# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: These cases exercise CPython keyword dispatch, frozen PyO3 owners and host database/file return values.
"""Actual Python policy dispatch and report ownership across host I/O."""

from dataclasses import FrozenInstanceError
from pathlib import Path

import purrdf
import pytest


def test_policy_is_required_and_every_limit_is_explicit():
    data = purrdf.gts_from_quads(b"", format=purrdf.RdfFormat.TURTLE)
    for function in (
        purrdf.gts_columnar_rows_from_bytes,
        purrdf.gts_columnar_parquet_from_bytes,
    ):
        with pytest.raises(TypeError):
            function(data)
        with pytest.raises(TypeError):
            function(data, blob_limits="automatic")
        _, report = function(data, blob_limits="unbounded")
        assert report.omissions == report.losses == []
    with pytest.raises(TypeError):
        purrdf.GtsBlobLimits(max_encoded_bytes=1)
    policy = purrdf.GtsBlobLimits.unbounded()
    with pytest.raises(AttributeError):
        policy.max_encoded_bytes = 1
    assert purrdf.gts.GtsBlobLimits is purrdf.GtsBlobLimits


@pytest.mark.parametrize(
    "writer,suffix",
    [
        ("gts_to_sqlite", ".sqlite"),
        ("gts_to_duckdb", ".duckdb"),
        ("gts_to_parquet", "-parquet"),
    ],
)
def test_host_result_retains_original_native_report(writer, suffix, tmp_path):
    data = (
        Path(__file__).resolve().parents[3] / "vectors/22-inline-blob.gts"
    ).read_bytes()
    policy = purrdf.GtsBlobLimits(
        max_encoded_bytes=0,
        max_decoded_bytes=None,
        max_total_decoded_bytes=None,
        max_metadata_bytes=None,
        max_frame_decoded_bytes=None,
    )
    result = getattr(purrdf, writer)(
        data, str(tmp_path / ("out" + suffix)), blob_limits=policy
    )
    assert isinstance(result.paths, tuple)
    assert all(Path(path).exists() for path in result.paths)
    assert len(result.report.losses) == len(result.report.omissions) == 1
    omission = result.report.omissions[0]
    assert omission.reason == "blob-byte-budget"
    assert omission.frame_range[0] < omission.frame_range[1]
    assert len(omission.frame_id) == len(omission.segment_head) == 32
    with pytest.raises(FrozenInstanceError):
        result.paths = ()
    with pytest.raises(AttributeError):
        omission.reason = "changed"
    del result, data
    cause = omission.native_cause
    assert cause["code"] == "gts-blob-retention-refusal"
    assert cause["fields"]["encoded_bytes"]["kind"] == "byte-count"
    assert cause["fields"]["limits"]["kind"] == "cause"
    assert omission.limits.max_encoded_bytes == 0
