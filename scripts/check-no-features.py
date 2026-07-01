#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Fail if any first-party workspace package declares Cargo features."""

import json
import subprocess
import sys


def main() -> int:
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
            text=True,
        )
    )
    offenders = []
    for package in metadata["packages"]:
        features = package.get("features", {})
        if features:
            offenders.append(f"{package['name']}: {', '.join(sorted(features))}")

    if offenders:
        print("First-party workspace crates must not declare Cargo features.", file=sys.stderr)
        print("\n".join(offenders), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
