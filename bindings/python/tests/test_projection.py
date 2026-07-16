# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Thin Python projection bindings stay byte-identical to the Rust carrier."""

from __future__ import annotations

import hashlib

import pytest

import purrdf

_CONFIG = """{
  "profile": "lpg-csv",
  "config": {
    "rdf_type": "https://example.org/type",
    "limits": {
      "max_artifacts": 16,
      "max_artifact_bytes": 1000000,
      "max_total_bytes": 4000000,
      "max_archive_bytes": 5000000,
      "max_term_depth": 16
    },
    "max_records": 1000
  }
}"""

_TURTLE = b"@prefix ex: <https://example.org/> .\nex:s ex:p ex:o .\n"
_RUST_ARCHIVE_SHA256 = "656066450fa23c55976f5434840169452c36324b943435e2f7ae55f8e9b6ef4e"


def test_project_matches_rust_bytes_and_returns_immutable_structured_losses() -> None:
    package = purrdf.project(
        _TURTLE,
        format=purrdf.RdfFormat.TURTLE,
        profile="lpg-csv",
        config=_CONFIG,
    )
    repeated = purrdf.project(
        _TURTLE,
        format=purrdf.RdfFormat.TURTLE,
        profile="lpg-csv",
        config=_CONFIG.encode(),
    )

    assert package.profile == "lpg-csv"
    assert package.archive == repeated.archive
    assert hashlib.sha256(package.archive).hexdigest() == _RUST_ARCHIVE_SHA256
    assert package.losses
    edge_loss = next(
        loss for loss in package.losses if loss.code == "lpg-edge-semantics-lowered"
    )
    assert edge_loss.source == "rdf-1.2-dataset"
    assert edge_loss.target == "lpg"
    assert edge_loss.note
    assert edge_loss.location is not None
    with pytest.raises(AttributeError):
        package.profile = "other"
    with pytest.raises(AttributeError):
        edge_loss.code = "other"


def test_lift_returns_a_frozen_dataset_and_write_only_profiles_fail_typed() -> None:
    package = purrdf.project(
        _TURTLE,
        format=purrdf.RdfFormat.TURTLE,
        profile="lpg-csv",
        config=_CONFIG,
    )
    lifted = purrdf.lift(package.archive, profile="lpg-csv", config=_CONFIG)
    assert lifted.dataset.quad_count() == 1
    assert lifted.losses
    with pytest.raises(AttributeError):
        lifted.losses = []

    with pytest.raises(ValueError, match="not a bidirectional"):
        purrdf.lift(package.archive, profile="skos", config=_CONFIG)
