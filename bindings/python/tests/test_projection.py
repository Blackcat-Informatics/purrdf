# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Thin Python projection bindings stay byte-identical to the Rust carrier."""

from __future__ import annotations

import hashlib
import io
import json
import tarfile
from pathlib import Path

import pytest

import purrdf

_CONFIG = """{
  "profile": "lpg-csv",
  "config": {
    "rdf_type": "https://example.org/type",
    "scope": {"mode": "all"},
    "limits": {
      "max_artifacts": 16,
      "max_artifact_bytes": 1000000,
      "max_total_bytes": 4000000,
      "max_archive_bytes": 5000000,
      "max_term_depth": 16
    },
    "execution_limits": {
      "max_input_records": 1000,
      "max_model_records": 1000,
      "max_nodes": 1000,
      "max_edges": 1000
    }
  }
}"""

_TURTLE = b"@prefix ex: <https://example.org/> .\nex:s ex:p ex:o .\n"
_RUST_ARCHIVE_SHA256 = "656066450fa23c55976f5434840169452c36324b943435e2f7ae55f8e9b6ef4e"
_REPO = Path(__file__).resolve().parents[3]
_RESEARCH_FIXTURES = _REPO / "crates/rdf/tests/fixtures/research-objects/carrier"
_CSVW_TERMS_CONFIG = _REPO / "crates/rdf/tests/fixtures/csvw-terms.json"
_RESEARCH_PROFILES = (
    "croissant-1.1",
    "ro-crate-1.3",
    "datacite-4.6",
    "dcat-3",
    "frictionless-data-package-1",
)


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


def test_project_artifacts_streams_archive_identical_chunks_and_typed_progress() -> None:
    events: list[tuple[str, str | None, bytes]] = []
    artifacts: dict[str, bytearray] = {}
    progress: list[purrdf.ProjectionProgress] = []

    def accept_artifact(event: str, path: str | None, chunk: bytes) -> None:
        events.append((event, path, chunk))
        if event == "begin-artifact":
            assert path is not None
            artifacts[path] = bytearray()
        elif event == "chunk":
            assert path is not None
            artifacts[path].extend(chunk)

    streamed = purrdf.project_artifacts(
        _TURTLE,
        format=purrdf.RdfFormat.TURTLE,
        profile="lpg-csv",
        config=_CONFIG,
        artifact_callback=accept_artifact,
        progress_callback=progress.append,
    )
    materialized = purrdf.project(
        _TURTLE,
        format=purrdf.RdfFormat.TURTLE,
        profile="lpg-csv",
        config=_CONFIG,
    )
    with tarfile.open(fileobj=io.BytesIO(materialized.archive), mode="r:") as archive:
        expected = {
            member.name: archive.extractfile(member).read()
            for member in archive.getmembers()
        }

    assert {path: bytes(body) for path, body in artifacts.items()} == expected
    assert events[0] == ("begin-package", None, b"")
    assert events[-1] == ("commit-package", None, b"")
    assert [path for event, path, _ in events if event == "begin-artifact"] == sorted(
        artifacts
    )
    assert streamed.profile == "lpg-csv"
    assert streamed.input_records == 1
    assert streamed.nodes == 2
    assert streamed.edges == 1
    assert streamed.model_records > 0
    assert streamed.losses
    assert progress[0].phase == "scanning"
    assert progress[-1].phase == "complete"
    assert all(
        before.input_records <= after.input_records
        and before.model_records <= after.model_records
        and before.nodes <= after.nodes
        and before.edges <= after.edges
        and before.artifacts <= after.artifacts
        and before.bytes <= after.bytes
        for before, after in zip(progress, progress[1:], strict=False)
    )
    with pytest.raises(AttributeError):
        progress[-1].phase = "other"


def test_project_artifacts_aborts_and_preserves_callback_errors() -> None:
    events: list[str] = []

    def reject_chunk(event: str, _path: str | None, _chunk: bytes) -> None:
        events.append(event)
        if event == "chunk":
            raise RuntimeError("injected artifact callback failure")

    with pytest.raises(RuntimeError, match="injected artifact callback failure"):
        purrdf.project_artifacts(
            _TURTLE,
            format=purrdf.RdfFormat.TURTLE,
            profile="lpg-csv",
            config=_CONFIG,
            artifact_callback=reject_chunk,
        )
    assert events[-1] == "abort-package"
    assert "commit-package" not in events

    missing_scope = json.loads(_CONFIG)
    del missing_scope["config"]["scope"]
    with pytest.raises(ValueError, match="missing field.*scope"):
        purrdf.project_artifacts(
            _TURTLE,
            format=purrdf.RdfFormat.TURTLE,
            profile="lpg-csv",
            config=json.dumps(missing_scope),
            artifact_callback=lambda *_args: None,
        )


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


def test_all_research_object_profiles_execute_through_the_shared_carrier() -> None:
    source = (_RESEARCH_FIXTURES / "shared.ttl").read_bytes()
    for profile in _RESEARCH_PROFILES:
        config = (_RESEARCH_FIXTURES / f"{profile}.json").read_bytes()
        first = purrdf.project(
            source,
            format=purrdf.RdfFormat.TURTLE,
            profile=profile,
            config=config,
        )
        second = purrdf.project(
            source,
            format=purrdf.RdfFormat.TURTLE,
            profile=profile,
            config=config,
        )
        assert first.profile == profile
        assert first.archive == second.archive
        lifted = purrdf.lift(first.archive, profile=profile, config=config)
        assert lifted.dataset.quad_count() > 0


def test_curated_csvw_terms_projects_and_is_structurally_write_only() -> None:
    source = b"""@prefix ex: <https://example.org/> .
ex:term ex:label "Term" ; ex:other ex:value .
"""
    config = _CSVW_TERMS_CONFIG.read_bytes()
    first = purrdf.project(
        source,
        format=purrdf.RdfFormat.TURTLE,
        profile="csvw-terms",
        config=config,
    )
    second = purrdf.project(
        source,
        format=purrdf.RdfFormat.TURTLE,
        profile="csvw-terms",
        config=config,
    )
    assert first.profile == "csvw-terms"
    assert first.archive == second.archive
    with tarfile.open(fileobj=io.BytesIO(first.archive), mode="r:") as archive:
        assert [member.name for member in archive.getmembers()] == [
            "csvw-metadata.json",
            "terms.csv",
        ]
    assert any(loss.code == "csvw-terms-predicate-unmapped" for loss in first.losses)
    assert all(loss.location is not None for loss in first.losses)
    with pytest.raises(ValueError, match="not a bidirectional"):
        purrdf.lift(first.archive, profile="csvw-terms", config=config)
