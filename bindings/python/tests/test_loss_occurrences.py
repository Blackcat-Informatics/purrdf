# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: this exercises the Python binding's loss dictionaries themselves.
"""A projection loss exposes its occurrence as typed site, subject and detail members."""

from __future__ import annotations

from pathlib import Path

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


def test_loss_occurrences_are_typed_site_subject_and_detail_dictionaries() -> None:
    """The occurrence is never inside `note`; every member is a kinded dictionary."""
    package = purrdf.project(
        _TURTLE,
        format=purrdf.RdfFormat.TURTLE,
        profile="lpg-csv",
        config=_CONFIG,
    )
    assert package.losses
    for loss in package.losses:
        assert loss.site["kind"] in {"unspecified", "document", "gts", "stage"}
        assert loss.subject["kind"] in {
            "absent",
            "term",
            "row",
            "resource",
            "graph",
            "pointer",
            "path",
        }
        for detail in loss.detail:
            assert detail["kind"] == "detail"
            assert detail["name"]
            assert detail["value"]["kind"] in {"count", "identifier", "term", "graph"}
    edge = next(
        loss for loss in package.losses if loss.code == "lpg-edge-semantics-lowered"
    )
    assert edge.subject["kind"] == "resource"
    assert edge.subject["resource_kind"]
    assert edge.subject["identifier"]


def test_a_dropped_row_is_identified_by_kind_and_a_digest_of_bytes() -> None:
    """A source row carries its kind and a 32-byte digest; a graph carries its name."""
    root = Path(__file__).resolve().parents[3] / "crates/rdf/tests/fixtures"
    package = purrdf.project(
        (root / "okf-terms.trig").read_bytes(),
        format=purrdf.RdfFormat.TRIG,
        profile="okf-terms",
        config=(root / "okf-terms.json").read_bytes(),
    )
    rows = [loss.subject for loss in package.losses if loss.subject["kind"] == "row"]
    assert rows
    for row in rows:
        assert row["row_kind"] == "quad"
        assert isinstance(row["digest"], bytes) and len(row["digest"]) == 32
    graphs = [
        loss.subject for loss in package.losses if loss.subject["kind"] == "graph"
    ]
    assert graphs
    assert graphs[0]["graph"] == {"kind": "iri", "value": "https://example.org/source"}
