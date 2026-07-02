# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Shared plumbing for the downstream-acceptance subprocess drivers.

Each ``driver_<package>.py`` next to this file is a standalone script executed in
a CHILD interpreter whose ``PYTHONPATH`` prepends ``bindings/python-rdflib-shadow``
so a plain ``import rdflib`` resolves to the purrdf shadow (Task 7), *shadowing*
the genuine rdflib that the acceptance dependency group also installed. The
driver then imports a real third-party rdflib consumer and drives its core path.

A driver never raises to the shell uncaught: it prints exactly one machine-
readable ``ACCEPT_RESULT <json>`` line and exits with a coded status the parent
test (:mod:`tests.test_acceptance_matrix`) maps to a pytest outcome:

* ``0`` / ``outcome="pass"``  — the consumer imported and ran its core path,
  and its rdflib / plugin lookups resolved to purrdf.
* ``2`` / ``outcome="fail"``  — the consumer is installed but its core path did
  not run green against the shim (a genuine, ledgered compat gap).
* ``3`` / ``outcome="unavailable"`` — the consumer is not installed in this
  environment; the parent SKIPS the row (explicit, never silent).
* ``4`` / ``outcome="misconfigured"`` — ``import rdflib`` did NOT resolve to the
  purrdf shadow (the harness would otherwise be silently testing real rdflib);
  the parent treats this as a hard error.

Because the driver dir is ``sys.path[0]`` for the script, ``import _harness``
resolves here; the shadow (from ``PYTHONPATH``) still wins for ``import rdflib``.
"""

from __future__ import annotations

import importlib.util
import json
import traceback
from typing import Any, NoReturn

_PREFIX = "ACCEPT_RESULT"


def _emit(record: dict[str, Any]) -> None:
    """Print the single machine-readable result line the parent test parses."""
    print(f"{_PREFIX} {json.dumps(record, sort_keys=True)}")


def unavailable(package: str, reason: str) -> NoReturn:
    """Report that ``package`` is not installed; the parent skips the row."""
    _emit({"package": package, "outcome": "unavailable", "reason": reason})
    raise SystemExit(3)


def misconfigured(package: str, reason: str) -> NoReturn:
    """Report that the shadow is not in force — a hard harness error."""
    _emit({"package": package, "outcome": "misconfigured", "reason": reason})
    raise SystemExit(4)


def passed(package: str, **detail: Any) -> NoReturn:
    """Report that the consumer's core path ran green against the shim."""
    _emit({"package": package, "outcome": "pass", **detail})
    raise SystemExit(0)


def failed(package: str, stage: str, exc: BaseException) -> NoReturn:
    """Report a genuine (ledgered) compat gap, with the concrete error + trace."""
    _emit(
        {
            "package": package,
            "outcome": "fail",
            "stage": stage,
            "error": f"{type(exc).__name__}: {exc}",
            "traceback": traceback.format_exc(),
        }
    )
    raise SystemExit(2)


def require_installed(package: str) -> None:
    """Skip (never silently pass) when the consumer package is absent.

    Uses :func:`importlib.util.find_spec` so a *missing* distribution is cleanly
    distinguished from a distribution that is present but fails to import against
    the shim — the former is "not evaluated", the latter a real acceptance result.
    """
    if importlib.util.find_spec(package) is None:
        unavailable(
            package,
            f"{package} is not installed in the acceptance environment "
            "(sync the `acceptance` dependency group to evaluate this row)",
        )


def require_shadow(package: str) -> None:
    """Assert ``import rdflib`` resolved to the purrdf shadow, not real rdflib.

    Guards the whole mechanism: without the shadow on ``PYTHONPATH`` the child
    would import the genuine rdflib the acceptance group also installed, and the
    row would prove nothing. A miss is a hard harness error, never a silent pass.
    """
    import rdflib

    module = rdflib.Graph.__module__
    if not module.startswith("purrdf"):
        misconfigured(
            package,
            f"import rdflib resolved to {module!r}, not the purrdf shadow "
            "(PYTHONPATH must prepend bindings/python-rdflib-shadow)",
        )
