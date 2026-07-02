# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Task 8 (#9 / #11): the rdflib LSP conformance gate.

This is the #9 "rdflib LSP gate": run rdflib's OWN, verbatim-vendored test suite
against ``purrdf.compat.rdflib`` and require a GREEN result — every vendored test
either passes or is a *known, ledgered, strict* xfail, with zero unexpected
failures and zero XPASS.

Isolation (hard constraint): this parent pytest process has the REAL rdflib 7.6
installed as the differential oracle, and the Task 7 shadow claims the same
``rdflib`` import name; the two must never co-inhabit one interpreter. So the
whole vendored suite runs in a **subprocess** (``tests/rdflib_suite/runner.py``)
whose ``import rdflib`` resolves to the shadow. This process' ``sys.modules`` /
``sys.path`` are never mutated — asserted by ``test_rdflib_shadow.py``.

The scoreboard (passed / xfailed counts) is parsed from the runner's stdout and
surfaced here so the conformance numbers are visible in the parent run.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
from pathlib import Path

_TESTS_DIR = Path(__file__).resolve().parent
_RUNNER = _TESTS_DIR / "rdflib_suite" / "runner.py"
_SCOREBOARD_RE = re.compile(
    r"PURRDF_SCOREBOARD passed=(?P<passed>\d+) xfailed=(?P<xfailed>\d+) "
    r"xpassed=(?P<xpassed>\d+) failed=(?P<failed>\d+) errors=(?P<errors>\d+) "
    r"ledger_total=(?P<ledger_total>\d+) ledger_applied=(?P<ledger_applied>\d+) "
    r"ledger_stale=(?P<ledger_stale>\d+)"
)


def _run_suite() -> tuple[int, str, dict[str, int]]:
    """Execute the vendored suite in a child interpreter; return (rc, out, board)."""
    # Run the runner as a script. The child prepends the shadow dir to sys.path
    # itself; the parent env is left untouched (no rdflib shadowing leaks here).
    proc = subprocess.run(
        [sys.executable, str(_RUNNER)],
        cwd=str(_TESTS_DIR),
        env=dict(os.environ),
        capture_output=True,
        text=True,
        check=False,
    )
    combined = proc.stdout + "\n" + proc.stderr
    match = _SCOREBOARD_RE.search(combined)
    board = {k: int(v) for k, v in match.groupdict().items()} if match else {}
    return proc.returncode, combined, board


def test_rdflib_conformance_gate() -> None:
    """rdflib's own vendored tests pass against the shim modulo the ledger.

    GREEN == returncode 0: every vendored test passed or is a known strict xfail;
    any real failure, any XPASS (a ledgered test that started passing), any
    collection error, or any stale ledger key turns this RED and shrinks/edits
    the ledger.
    """
    returncode, output, board = _run_suite()
    assert board, f"could not find scoreboard in runner output:\n{output}"

    # Report the conformance scoreboard on the parent run for visibility.
    print(
        "\nrdflib LSP conformance scoreboard: "
        f"{board['passed']} passed / {board['xfailed']} xfailed "
        f"(xpassed={board['xpassed']}, failed={board['failed']}, "
        f"errors={board['errors']}, ledger {board['ledger_applied']}/"
        f"{board['ledger_total']} applied, stale={board['ledger_stale']})"
    )

    assert board["xpassed"] == 0, (
        "XPASS discipline: a ledgered test now passes — remove it from "
        f"xfail_ledger.toml so the ledger shrinks.\n{output}"
    )
    assert board["failed"] == 0, f"unexpected vendored-test failures:\n{output}"
    assert board["errors"] == 0, f"vendored-test collection/setup errors:\n{output}"
    assert board["ledger_stale"] == 0, (
        "stale ledger key(s) match no collected test — prune xfail_ledger.toml.\n"
        f"{output}"
    )
    assert returncode == 0, f"conformance gate is RED (rc={returncode}):\n{output}"


def test_conformance_suite_is_substantial() -> None:
    """Guard against the gate silently shrinking to a trivial suite.

    A green gate is only meaningful if it actually exercises a broad slice of
    rdflib's public API. Assert a floor on the number of vendored test outcomes.
    """
    _returncode, output, board = _run_suite()
    assert board, f"could not find scoreboard in runner output:\n{output}"
    total = board["passed"] + board["xfailed"]
    assert total >= 70, f"vendored suite unexpectedly small ({total} outcomes)"
    assert board["passed"] >= 45, f"too few genuine passes ({board['passed']})"
