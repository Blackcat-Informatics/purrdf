# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Stream a scoped LPG carrier into an atomically published directory."""

from __future__ import annotations

import argparse
import json
import shutil
import tempfile
from pathlib import Path
from typing import BinaryIO

import purrdf


class DirectorySink:
    """Stage lifecycle-delimited artifact chunks and publish only on commit."""

    def __init__(self, target: Path) -> None:
        self.target = target
        self.staging: Path | None = None
        self.output: BinaryIO | None = None

    def __call__(self, event: str, path: str | None, chunk: bytes) -> None:
        if event == "begin-package":
            self.target.parent.mkdir(parents=True, exist_ok=True)
            self.staging = Path(
                tempfile.mkdtemp(prefix=f".{self.target.name}.", dir=self.target.parent)
            )
        elif event == "begin-artifact":
            if self.staging is None or path is None:
                raise RuntimeError("artifact began outside a package")
            artifact = self.staging / path
            artifact.parent.mkdir(parents=True, exist_ok=True)
            self.output = artifact.open("wb")
        elif event == "chunk":
            if self.output is None:
                raise RuntimeError("chunk arrived outside an artifact")
            self.output.write(chunk)
        elif event == "finish-artifact":
            self._close()
        elif event == "commit-package":
            self._close()
            if self.staging is None:
                raise RuntimeError("package commit arrived without staging")
            if self.target.exists():
                raise FileExistsError(self.target)
            self.staging.rename(self.target)
            self.staging = None
        elif event == "abort-package":
            self._close()
            if self.staging is not None:
                shutil.rmtree(self.staging, ignore_errors=True)
                self.staging = None

    def _close(self) -> None:
        if self.output is not None:
            self.output.close()
            self.output = None


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
    outcome = purrdf.project_artifacts(
        "@prefix ex: <https://example.org/> . ex:alice ex:knows ex:bob .\n",
        format=purrdf.RdfFormat.TURTLE,
        profile="lpg-csv",
        config=config,
        artifact_callback=DirectorySink(args.output),
        progress_callback=lambda row: print(
            row.phase, row.input_records, row.nodes, row.edges, row.bytes
        ),
    )
    print(f"published {outcome.profile} with {outcome.nodes} nodes and {outcome.edges} edges")


if __name__ == "__main__":
    main()
