# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Task 10: the downstream-acceptance matrix.

Proves that real third-party rdflib consumers — SPARQLWrapper, pyshacl, sssom —
import and run their core paths against ``purrdf.compat.rdflib`` (the shim), with
their ``import rdflib`` / plugin lookups resolving to purrdf rather than the
genuine rdflib.

Mechanism (identical in spirit to the Task 8 conformance gate): the ``acceptance``
dependency group installs each consumer, which drags in the *real* rdflib. To make
a consumer run on purrdf WITHOUT mutating this parent process (whose in-process
rdflib is the differential oracle), every row runs in a SUBPROCESS whose
``PYTHONPATH`` prepends ``bindings/python-rdflib-shadow``; the child's
``import rdflib`` then resolves to the purrdf shadow (Task 7), shadowing the
installed real rdflib for that child only. The parent's ``sys.modules`` /
``sys.path`` are never touched.

Each row's driver lives in ``tests/acceptance/driver_<package>.py`` and prints a
single ``ACCEPT_RESULT <json>`` line this module parses. Outcomes:

* ``pass``          — core path ran, lookups resolved  → the test passes.
* ``fail``          — installed but a genuine compat gap → ledgered strict-xfail.
* ``unavailable``   — package not installed             → the test SKIPS.
* ``misconfigured`` — the shadow was not in force        → hard error (never silent).

The ledgered rows (pyshacl, sssom) are strict xfails in ``xfail_ledger.toml``: if
either starts passing, the XPASS fails the run and forces the ledger to shrink —
the same XPASS discipline as the Rust conformance harnesses (AGENTS.md §2).
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest

_TESTS_DIR = Path(__file__).resolve().parent
_ACCEPTANCE_DIR = _TESTS_DIR / "acceptance"
# tests/ -> python/ -> bindings/ -> python-rdflib-shadow/
_SHADOW_DIR = _TESTS_DIR.parent.parent / "python-rdflib-shadow"

_RESULT_PREFIX = "ACCEPT_RESULT "


def _run_driver(package: str) -> tuple[int, dict[str, Any], str]:
    """Run ``driver_<package>.py`` in a shadow subprocess; return (rc, record, raw).

    Prepends the shadow distribution to ``PYTHONPATH`` so the child's
    ``import rdflib`` resolves to purrdf, then parses the driver's single
    ``ACCEPT_RESULT`` line into a structured record.
    """
    driver = _ACCEPTANCE_DIR / f"driver_{package}.py"
    assert driver.is_file(), f"missing acceptance driver: {driver}"

    env = dict(os.environ)
    existing = env.get("PYTHONPATH", "")
    env["PYTHONPATH"] = (
        f"{_SHADOW_DIR}{os.pathsep}{existing}" if existing else str(_SHADOW_DIR)
    )
    proc = subprocess.run(
        [sys.executable, str(driver)],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    raw = f"--- stdout ---\n{proc.stdout}\n--- stderr ---\n{proc.stderr}"
    record: dict[str, Any] = {}
    for line in proc.stdout.splitlines():
        if line.startswith(_RESULT_PREFIX):
            record = json.loads(line[len(_RESULT_PREFIX) :])
    assert record, f"driver emitted no ACCEPT_RESULT line:\n{raw}"
    assert record.get("outcome") != "misconfigured", (
        "acceptance shadow was not in force — the child imported real rdflib, "
        f"so the row proves nothing:\n{raw}"
    )
    return proc.returncode, record, raw


def _require_core_path(package: str) -> dict[str, Any]:
    """Skip when the package is absent; otherwise require its core path to pass."""
    _rc, record, raw = _run_driver(package)
    if record["outcome"] == "unavailable":
        pytest.skip(record.get("reason", f"{package} not installed"))
    assert record["outcome"] == "pass", (
        f"{package} acceptance core path did not run green "
        f"(stage={record.get('stage')}, error={record.get('error')}):\n{raw}"
    )
    return record


def test_shadow_distribution_present() -> None:
    """The acceptance mechanism depends on the Task 7 shadow distribution."""
    assert (_SHADOW_DIR / "rdflib" / "__init__.py").is_file()


def test_sparqlwrapper_result_conversion() -> None:
    """SPARQLWrapper's rdflib-backed result parsing resolves to purrdf.

    Drives its offline SPARQL-Results JSON parser and its RDF/XML CONSTRUCT
    conversion (``from rdflib import ConjunctiveGraph; graph.parse(...)``), which
    dispatches through the shim's parser-plugin lookup to a purrdf-backed graph.
    """
    record = _require_core_path("sparqlwrapper")
    assert record["graph_type"].startswith("purrdf"), record


def test_pyshacl_core_path() -> None:
    """pyshacl.validate runs its core path against a purrdf-backed graph.

    Ledgered strict-xfail: pyshacl fails to import against the shim (first a
    missing ``rdflib.term.IdentifiedNode`` base class, then version-gated
    monkeypatching of rdflib's private ``rdflib.term`` / ``rdflib.plugins.sparql``
    internals), all upstream of the public surface the shim exposes.
    """
    _require_core_path("pyshacl")


def test_sssom_core_path() -> None:
    """sssom serializes a mapping set to RDF via its rdflib-backed writer.

    Ledgered strict-xfail: sssom's linkml dependency deep-imports rdflib's private
    serializer module (``rdflib.plugins.serializers.turtle``), which the shim does
    not provide, so ``import sssom`` fails before purrdf code is reached.
    """
    _require_core_path("sssom")


@pytest.mark.skip(
    reason="SPARQLWrapper's live-query transport needs a running SPARQL "
    "endpoint; the offline acceptance environment has none, so only the "
    "rdflib-backed result-conversion path is exercised (see "
    "test_sparqlwrapper_result_conversion). Not a shim gap."
)
def test_sparqlwrapper_live_endpoint_query() -> None:  # pragma: no cover
    """The network query path is not evaluated offline (explicit skip)."""


def test_acceptance_matrix_summary() -> None:
    """Print the per-package matrix for visibility and guard the harness itself.

    Not a pass/fail gate on the ledgered rows (those are owned by their own
    strict-xfail tests); this asserts every driver produced a parseable,
    known-outcome record under an in-force shadow, and that at least one consumer
    was actually exercised (never a silently all-unavailable green).
    """
    packages = ("sparqlwrapper", "pyshacl", "sssom")
    rows: list[tuple[str, dict[str, Any]]] = []
    for package in packages:
        _rc, record, _raw = _run_driver(package)
        rows.append((package, record))

    print("\ndownstream acceptance matrix:")
    for package, record in rows:
        outcome = record["outcome"]
        version = record.get("version", "-")
        note = record.get("detail") or record.get("reason") or record.get("error", "")
        print(f"  {package:<16} {outcome:<12} v{version:<10} {note}")

    known = {"pass", "fail", "unavailable"}
    for package, record in rows:
        assert record["outcome"] in known, (package, record)
    exercised = [p for p, r in rows if r["outcome"] in {"pass", "fail"}]
    assert exercised, (
        "no downstream consumer was exercised — the acceptance dependency group "
        "is not installed; sync it to evaluate the matrix"
    )
