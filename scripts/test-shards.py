#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Partition test targets without changing Cargo workspace feature unification."""
import argparse
import fnmatch
import json

PATTERNS = ("[a-d]*", "[e-j]*", "[pq]*", "[!a-r]*", "[k-mo]*", "r*", "n*")
SHARDS = ("lib", "doc", *(f"integration-{index}" for index in range(1, len(PATTERNS) + 1)))


def integration_shard(name: str) -> str:
    """Select one disjoint first-character partition, including non-ASCII names."""
    if not name:
        raise ValueError("an integration test target must have a name")
    return next(
        f"integration-{index}"
        for index, pattern in enumerate(PATTERNS, 1)
        if fnmatch.fnmatchcase(name, pattern)
    )


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", action="store_true", required=True)
    parser.parse_args()
    print(json.dumps(SHARDS))
