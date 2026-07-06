#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Fail if any first-party workspace package declares Cargo features.

The sole sanctioned exception is the ``capi`` feature on ``purrdf-capi``:
``cargo capi`` (cargo-c >= 0.10) unconditionally enables a feature named
``capi`` and hard-errors if the crate does not declare it, so the C-ABI header
regeneration/drift gate (``make capi-header``/``capi-check`` and the CI ``capi``
job) cannot run without that marker. It gates no code — the whole C-ABI surface
is always compiled. The allowlist is keyed on the exact ``(package, feature)``
pair, so any *other* feature on ``purrdf-capi`` — and any feature on any other
crate — still fails this gate.
"""

import json
import subprocess
import sys

# Exact per-package allowlist of permitted feature names. Keep this as tight as
# the cargo-c requirement demands and no tighter — do NOT broaden it to unblock
# ordinary optionality (that is exactly what this gate exists to forbid).
ALLOWED_FEATURES = {
    "purrdf-capi": {"capi"},
}


def main() -> int:
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1", "--locked"],
            text=True,
        )
    )
    offenders = []
    for package in metadata["packages"]:
        declared = set(package.get("features", {}))
        disallowed = declared - ALLOWED_FEATURES.get(package["name"], set())
        if disallowed:
            offenders.append(f"{package['name']}: {', '.join(sorted(disallowed))}")

    if offenders:
        print(
            "First-party workspace crates must not declare Cargo features "
            "(except the allowlisted purrdf-capi:capi cargo-c marker).",
            file=sys.stderr,
        )
        print("\n".join(offenders), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
