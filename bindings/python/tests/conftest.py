# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Shared pytest fixtures + the xfail-ledger XPASS-discipline hook.

The ledger (`xfail_ledger.toml`) maps pytest node ids to reasons for tests that
are expected to fail against the current shim. We apply them as **strict** xfails
so a ledgered test that starts passing fails the run — the ledger only shrinks.
This is the Python analogue of the Rust conformance harnesses (AGENTS.md §2).

Fixtures deliberately keep the *real* rdflib (`oracle`) and the purrdf compat
shim (`compat`) as separate imports: the differential parity suite compares one
against the other, so they must never be the same object. The top-level
`rdflib` shadow package is kept out of this environment entirely (see README.md).
"""

from __future__ import annotations

import tomllib
from pathlib import Path
from types import ModuleType

import pytest

_LEDGER_PATH = Path(__file__).parent / "xfail_ledger.toml"

# The verbatim-vendored rdflib suite under `rdflib_suite/vendor/` is rdflib's
# OWN test tree; it is meant to run ONLY in the shadow subprocess driven by
# `test_rdflib_suite.py` (where `import rdflib` == the purrdf shim). It must
# never be collected by this parent process, which uses the real rdflib as its
# differential oracle. `collect_ignore_glob` keeps pytest away from it.
collect_ignore_glob = ["rdflib_suite/vendor/*", "rdflib_suite/runner.py"]


def _load_ledger() -> dict[str, str]:
    """Return the `{node_id: reason}` map from the xfail ledger (empty if absent)."""
    if not _LEDGER_PATH.exists():
        return {}
    data = tomllib.loads(_LEDGER_PATH.read_text(encoding="utf-8"))
    entries = data.get("xfail", {})
    return {str(key): str(reason) for key, reason in entries.items()}


_LEDGER = _load_ledger()


def pytest_collection_modifyitems(
    config: pytest.Config, items: list[pytest.Item]
) -> None:
    """Apply strict xfail to every collected test named in the ledger."""
    for item in items:
        reason = _LEDGER.get(item.nodeid)
        if reason is not None:
            item.add_marker(pytest.mark.xfail(reason=reason, strict=True))


@pytest.fixture
def compat() -> ModuleType:
    """The purrdf rdflib-compat shim (the implementation under test)."""
    import purrdf.compat.rdflib as compat_rdflib

    return compat_rdflib


@pytest.fixture
def oracle() -> ModuleType:
    """The *real* rdflib — the differential oracle (never the shim)."""
    import rdflib

    return rdflib
