#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Populate all-platform inputs for the subsequent offline dependency audit."""

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def main() -> None:
    """Fetch every committed lockfile without changing its resolution."""
    tracked = subprocess.check_output(
        ["git", "ls-files", "-z", "--", "Cargo.lock", "**/Cargo.lock"], cwd=ROOT
    )
    for name in tracked.split(b"\0"):
        if name:
            manifest = ROOT / Path(name.decode()).parent / "Cargo.toml"
            subprocess.run(
                ["cargo", "fetch", "--locked", "--manifest-path", str(manifest)],
                cwd=ROOT,
                check=True,
            )


if __name__ == "__main__":
    main()
