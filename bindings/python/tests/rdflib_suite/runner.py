# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Run the vendored rdflib tests against the PurRDF shim in a child interpreter.

Use `python -m tests.rdflib_suite.runner` or execute this file. The child makes
`import rdflib` resolve to the shadow and registers its submodules. The parent
keeps the real rdflib oracle.

Vendored tests run unchanged under pytest with strict xfails. A failure, an
XPASS, a stale ledger entry or too few test outcomes fails the run. The
scoreboard exposes the measured counts to the parent.
"""

from __future__ import annotations

import os
import sys
import tomllib
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
_VENDOR_DIR = _HERE / "vendor"
_LEDGER_PATH = _HERE / "xfail_ledger.toml"
# tests/rdflib_suite/ -> tests/ -> python/ -> bindings/ -> python-rdflib-shadow/
_SHADOW_DIR = _HERE.parent.parent.parent / "python-rdflib-shadow"

_SCOREBOARD_PREFIX = "PURRDF_SCOREBOARD"


def _load_ledger() -> dict[str, str]:
    """Return the ``{nodeid: reason}`` xfail map (empty if the file is absent)."""
    if not _LEDGER_PATH.exists():
        return {}
    data = tomllib.loads(_LEDGER_PATH.read_text(encoding="utf-8"))
    entries = data.get("xfail", {})
    return {str(key): str(reason) for key, reason in entries.items()}


class _LedgerPlugin:
    """Apply strict xfails from the ledger and emit the conformance scoreboard."""

    def __init__(self, ledger: dict[str, str]) -> None:
        self._ledger = ledger
        self._matched: set[str] = set()
        self.stale: set[str] = set()
        self.undersized = True

    def pytest_collection_modifyitems(
        self, config: pytest.Config, items: list[pytest.Item]
    ) -> None:
        collected = {item.nodeid for item in items}
        for item in items:
            reason = self._ledger.get(item.nodeid)
            if reason is not None:
                self._matched.add(item.nodeid)
                item.add_marker(pytest.mark.xfail(reason=reason, strict=True))
        # A ledger key that matches nothing collected is stale — force a failure
        # so the ledger can never drift away from the vendored suite.
        self.stale = set(self._ledger) - collected

    def pytest_terminal_summary(
        self, terminalreporter: pytest.TerminalReporter
    ) -> None:
        stats = terminalreporter.stats
        passed = len(stats.get("passed", []))
        xfailed = len(stats.get("xfailed", []))
        xpassed = len(stats.get("xpassed", []))
        failed = len(stats.get("failed", []))
        errors = len(stats.get("error", []))
        print(
            f"{_SCOREBOARD_PREFIX} passed={passed} xfailed={xfailed} "
            f"xpassed={xpassed} failed={failed} errors={errors} "
            f"ledger_total={len(self._ledger)} ledger_applied={len(self._matched)} "
            f"ledger_stale={len(self.stale)}"
        )
        minimum_passed, minimum_total = 45, 70
        self.undersized = passed < minimum_passed or passed + xfailed < minimum_total
        if self.undersized:
            terminalreporter.write_line(
                f"VENDORED SUITE TOO SMALL ({passed} passed, {passed + xfailed} total). "
                f"Require at least {minimum_passed} passes and {minimum_total} outcomes.",
                red=True,
            )
        if self.stale:
            terminalreporter.write_line(
                "STALE LEDGER ENTRIES (no matching collected test): "
                + ", ".join(sorted(self.stale)),
                red=True,
            )


def main() -> int:
    """Install the shadow, run the vendored suite, return pytest's exit code."""
    # 1. Make `import rdflib` resolve to the purrdf-backed top-level shadow.
    sys.path.insert(0, str(_SHADOW_DIR))
    import rdflib  # noqa: F401  (loads the shadow, registers rdflib.* submodules)

    if not rdflib.Graph.__module__.startswith("purrdf"):  # pragma: no cover
        print(
            "FATAL: `import rdflib` did not resolve to the purrdf shadow "
            f"(got {rdflib.Graph.__module__})",
            file=sys.stderr,
        )
        return 3

    # 2. Run pytest with the vendor dir as the invocation/rootdir so node ids are
    #    stable and ledger-relative (e.g. `test_collection.py::test_scenario`).
    os.chdir(_VENDOR_DIR)
    plugin = _LedgerPlugin(_load_ledger())
    files = sorted(p.name for p in _VENDOR_DIR.glob("test_*.py"))
    args = [
        "-p",
        "no:cacheprovider",
        "-p",
        "no:randomly",
        # Fully isolate from the parent `tests/conftest.py` (its own ledger +
        # real-rdflib `oracle` fixture must not leak into this shadow run).
        f"--rootdir={_VENDOR_DIR}",
        f"--confcutdir={_VENDOR_DIR}",
        "--continue-on-collection-errors",
        "-q",
        *files,
    ]
    exit_code = pytest.main(args, plugins=[plugin])
    if exit_code == 0 and (plugin.stale or plugin.undersized):
        return 4
    return int(exit_code)


if __name__ == "__main__":
    raise SystemExit(main())
