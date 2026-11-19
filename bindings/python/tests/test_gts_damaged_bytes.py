# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: this exercises the Python exception surface (ValueError with code and diagnostic_json attributes) of the compiled extension.
"""Byte readers of a GTS container refuse a damaged file instead of returning a partial view."""

from __future__ import annotations

import pytest

import purrdf

EX = "https://example.org/"
CBOR_TEXT_STRAY = b"\x65stray"  # CBOR text string "stray": an item that is no header and no frame

READERS = [
    purrdf.GtsFoldViewNative.from_bytes,
    purrdf.gts_relational_rows_from_bytes,
]


def _container() -> bytes:
    dataset = purrdf.RdfDataset(
        f"<{EX}a> <{EX}p> <{EX}b> <{EX}g> .\n", purrdf.RdfFormat.N_QUADS
    )
    return dataset.to_gts()


@pytest.mark.parametrize("read", READERS)
def test_valid_container_still_reads(read) -> None:
    assert read(_container()) is not None


@pytest.mark.parametrize("read", READERS)
@pytest.mark.parametrize("position", ["prepended", "appended"])
def test_damaged_container_raises_with_native_fields(read, position: str) -> None:
    data = _container()
    damaged = CBOR_TEXT_STRAY + data if position == "prepended" else data + CBOR_TEXT_STRAY
    with pytest.raises(ValueError) as caught:
        read(damaged)
    assert caught.value.code
    assert caught.value.diagnostic_json
