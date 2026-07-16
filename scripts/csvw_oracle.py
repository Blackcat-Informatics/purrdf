#!/usr/bin/env -S uv run --script
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
# /// script
# requires-python = ">=3.11"
# dependencies = ["csvw==4.1.0"]
# ///

"""Validate a PurRDF CSVW package with the pinned independent implementation."""

from __future__ import annotations

import copy
import json
import pathlib
import shutil
import sys
import tempfile
from collections.abc import Callable
from urllib.parse import urlparse

from csvw import CSVW


def load_valid(metadata: pathlib.Path) -> CSVW:
    candidate = CSVW(str(metadata), validate=True, strict=True)
    if not candidate.is_valid:
        raise AssertionError(f"csvw 4.1.0 rejected canonical package: {candidate.warnings}")
    return candidate


def expect_rejected(label: str, operation: Callable[[], object]) -> None:
    try:
        candidate = operation()
        if isinstance(candidate, CSVW) and not candidate.is_valid:
            return
    except (AssertionError, KeyError, TypeError, ValueError):
        return
    raise AssertionError(f"csvw 4.1.0 accepted deliberate {label} corruption")


def localize_package(source: pathlib.Path, destination: pathlib.Path) -> pathlib.Path:
    """Adapt only resource links because csvw 4.1.0 cannot open file IRIs."""
    metadata = json.loads((source / "csvw-metadata.json").read_text(encoding="utf-8"))
    resource_map: dict[str, str] = {}
    for table in metadata["tables"]:
        original = table["url"]
        filename = pathlib.PurePosixPath(urlparse(original).path).name
        localized = f"tables/{filename}"
        resource_map[original] = localized
        table["url"] = localized
        # csvw 4.1.0 exposes this API-only alias but fails to derive it from the
        # normative `headerRowCount: 0`; retain both so the oracle reads the
        # emitted headerless physical form without changing its semantics.
        table["dialect"]["header"] = False
        destination_table = destination / localized
        destination_table.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source / localized, destination_table)
    for table in metadata["tables"]:
        for foreign_key in table["tableSchema"].get("foreignKeys", []):
            reference = foreign_key["reference"]
            if "resource" in reference:
                reference["resource"] = resource_map[reference["resource"]]
    path = destination / "csvw-metadata.json"
    path.write_text(
        json.dumps(metadata, sort_keys=True, separators=(",", ":")),
        encoding="utf-8",
    )
    return path


def main() -> int:
    if len(sys.argv) != 2:
        raise SystemExit("usage: csvw_oracle.py PACKAGE_DIRECTORY")
    root = pathlib.Path(sys.argv[1]).resolve()
    with tempfile.TemporaryDirectory(prefix="purrdf-csvw-oracle-") as tmp:
        oracle_root = pathlib.Path(tmp)
        metadata_path = localize_package(root, oracle_root)
        oracle = load_valid(metadata_path)
        counts = sorted(len(list(table.iterdicts(strict=True))) for table in oracle.tables)
        if counts != [2, 2]:
            raise AssertionError(f"csvw 4.1.0 observed unexpected row counts: {counts}")

        metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        corrupted_metadata = copy.deepcopy(metadata)
        corrupted_metadata["tables"][0]["tableSchema"]["primaryKey"] = "missing"
        bad_metadata_path = oracle_root / "bad-metadata.json"
        bad_metadata_path.write_text(
            json.dumps(corrupted_metadata, sort_keys=True, separators=(",", ":")),
            encoding="utf-8",
        )
        expect_rejected(
            "metadata",
            lambda: CSVW(str(bad_metadata_path), validate=True, strict=True),
        )

        children = oracle_root / "tables" / "children.csv"
        original = children.read_bytes()
        records = original.splitlines(keepends=True)
        if not records:
            raise AssertionError("canonical child table is empty")
        children.write_bytes(original + records[-1])
        try:
            expect_rejected(
                "data",
                lambda: CSVW(str(metadata_path), validate=True, strict=True),
            )
        finally:
            children.write_bytes(original)

    print(
        "OK: csvw==4.1.0 validated the canonical PurRDF table group and rejected "
        "metadata/data corruptions"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
