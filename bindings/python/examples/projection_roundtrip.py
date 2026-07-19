# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Materialize a deterministic LPG archive and lift it with the native engine."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

import purrdf


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    config = json.dumps(
        {
            "profile": "lpg-csv",
            "config": {
                "rdf_type": "https://example.org/type",
                "scope": {"mode": "all"},
                "limits": {
                    "max_artifacts": 16,
                    "max_artifact_bytes": 1_000_000,
                    "max_total_bytes": 4_000_000,
                    "max_archive_bytes": 5_000_000,
                    "max_term_depth": 16,
                },
                "execution_limits": {
                    "max_input_records": 1_000,
                    "max_model_records": 1_000,
                    "max_nodes": 1_000,
                    "max_edges": 1_000,
                },
            },
        }
    )
    package = purrdf.project(
        "@prefix ex: <https://example.org/> . ex:alice ex:knows ex:bob .\n",
        format=purrdf.RdfFormat.TURTLE,
        profile="lpg-csv",
        config=config,
    )
    args.output.write_bytes(package.archive)
    lifted = purrdf.lift(package.archive, profile="lpg-csv", config=config)
    if lifted.dataset.quad_count() != 1:
        raise RuntimeError("projection round trip changed the RDF dataset")
    print(f"wrote {len(package.archive)} bytes with {len(package.losses)} loss record(s)")


if __name__ == "__main__":
    main()
