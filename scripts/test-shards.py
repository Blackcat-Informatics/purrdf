#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Target partitions of the workspace test build, preserving feature unification.

Every invocation selects the whole workspace. Package partitions change Cargo's
feature resolution, so these shards partition target kinds and integration names.
The sole library exclusion is the Python extension, whose manifest disables tests.
"""
import argparse
import json

SHARDS = ("lib", "doc", "integration-1", "integration-2", "integration-3", "integration-4", "integration-5")
PATTERNS = ("[a-d]*", "[e-j]*", "[p-r]*", "[!a-r]*", "[k-o]*")


def integration_shard(name: str) -> str:
    """Select one disjoint first-character partition, including non-ASCII names."""
    if not name:
        raise ValueError("an integration test target must have a name")
    first = name[0]
    index = 1 if "a" <= first <= "d" else 2 if "e" <= first <= "j" else 5 if "k" <= first <= "o" else 3 if "p" <= first <= "r" else 4
    return f"integration-{index}"


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--matrix", action="store_true", required=True)
    parser.parse_args()
    print(json.dumps(SHARDS))
