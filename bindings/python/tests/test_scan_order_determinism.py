# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Multi-process pin: SPARQL solution order through the Python `Store` surface is
reproducible ACROSS PROCESSES, not merely within one.

Background: `Store`/`MutableDataset` wrap `purrdf_core::ir::MutableDataset`, a
copy-on-write suppression-delta layer. Its `freeze()` compaction used to replay
delta-added quads in a plain `std::collections::HashSet`'s hash-iteration order —
and `std`'s default `RandomState` draws a fresh per-construction seed from OS
randomness, so two `MutableDataset`s built from the identical insertion sequence
(even within one process, let alone across separate `python` invocations) could
iterate that set in different orders. `freeze()` sorts the frozen dataset by dense
`TermId`, and those ids are minted in whatever order the delta was replayed in, so
the reordering was directly observable downstream: `GROUP_CONCAT` token order,
`FIRST`/`LAST`/`TOPK`, and plain (no `ORDER BY`) projection order all read that
same scan.

A SINGLE in-process run cannot catch this class of bug — a `HashSet`'s hash seed is
fixed for its own lifetime, so repeating a query against the SAME store looks
perfectly stable. Only a genuinely fresh process (or an equivalent fresh
`MutableDataset` construction) can observe the per-construction seed varying. This
suite spawns many fresh child interpreters running `scan_order_repro.py` and
requires byte-identical output from every one.

The fix (`crates/rdf-core/src/ir/mutable.rs`) replays delta-added quads by an
explicit insertion ordinal (`added_ord`) instead of the `added` set's own
hash-iteration order — the same "sort explicitly, never trust hash order" policy
`purrdf-core::hash` already documents for every other lookup table in the kernel.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

# "at least 10 times" per the regression requirement; a couple extra for margin.
_RUNS = 12

_REPRO_SCRIPT = Path(__file__).resolve().parent / "scan_order_repro.py"


def _run_child() -> dict[str, object]:
    """Run the reproduction script in a brand-new child interpreter and parse its
    one-line JSON result."""
    proc = subprocess.run(
        [sys.executable, str(_REPRO_SCRIPT)],
        cwd=str(_REPRO_SCRIPT.parent),
        env=dict(os.environ),
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, (
        f"scan_order_repro.py must exit 0; stderr:\n{proc.stderr}"
    )
    lines = [line for line in proc.stdout.splitlines() if line.strip()]
    assert len(lines) == 1, f"expected exactly one JSON line, got:\n{proc.stdout}"
    return json.loads(lines[0])


def test_store_scan_order_is_identical_across_fresh_processes() -> None:
    """The SAME query over the SAME dataset yields the SAME solution order in every
    one of many independent processes — plain projection, GROUP_CONCAT, and
    FIRST/LAST alike (all four read the identical underlying BGP scan)."""
    results = [_run_child() for _ in range(_RUNS)]

    distinct_plain = {tuple(r["plain"]) for r in results}
    distinct_group_concat = {r["group_concat"] for r in results}
    distinct_first = {r["first"] for r in results}
    distinct_last = {r["last"] for r in results}

    assert distinct_plain == {("x", "y", "z")}, (
        f"plain projection order must be identical across all {_RUNS} processes "
        f"(and match the source scan order): observed {sorted(distinct_plain)}"
    )
    assert distinct_group_concat == {"x|y|z"}, (
        "GROUP_CONCAT must fold the identical scan order in every process: "
        f"observed {sorted(distinct_group_concat)}"
    )
    assert distinct_first == {"x"}, (
        f"FIRST must read the identical scan order in every process: "
        f"observed {sorted(distinct_first)}"
    )
    assert distinct_last == {"z"}, (
        f"LAST must read the identical scan order in every process: "
        f"observed {sorted(distinct_last)}"
    )

    # Every field within a single process must also be mutually consistent: FIRST/
    # LAST are a faithful fold over the SAME scan `plain`/`group_concat` observed,
    # not an independently (and differently) ordered computation.
    for r in results:
        plain = r["plain"]
        assert r["group_concat"] == "|".join(plain)
        assert r["first"] == plain[0]
        assert r["last"] == plain[-1]
