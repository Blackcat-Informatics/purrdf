# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Shared helpers for tests that exercise the ``rdflib`` shadow distribution.

The shadow distribution re-exports :mod:`purrdf.compat.rdflib` under the
top-level ``rdflib`` package name.  Because the parent pytest process also
has the *real* rdflib installed, any test that needs ``import rdflib`` to
resolve to the shadow must run that code in a subprocess whose
``PYTHONPATH`` is adjusted so the shadow package comes first.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

# bindings/python/tests/ -> bindings/ -> python-rdflib-shadow/
_SHADOW_DIR = Path(__file__).resolve().parent.parent.parent / "python-rdflib-shadow"


def _run_with_shadow(argv: list[str | Path]) -> subprocess.CompletedProcess[str]:
    """Run ``argv`` in a child process with the rdflib shadow prepended to ``PYTHONPATH``.

    The shadow distribution's package root is inserted first so ``import rdflib``
    in the child resolves to the purrdf shadow rather than the real rdflib
    installed in this (parent) environment.  The caller decides how to interpret
    the return code and output.
    """
    env = dict(os.environ)
    existing = env.get("PYTHONPATH", "")
    env["PYTHONPATH"] = (
        f"{_SHADOW_DIR}{os.pathsep}{existing}" if existing else str(_SHADOW_DIR)
    )
    return subprocess.run(
        [str(arg) for arg in argv],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )


def _run_in_shadow(code: str) -> str:
    """Run ``code`` in a child interpreter whose ``import rdflib`` is the shadow."""
    proc = _run_with_shadow([sys.executable, "-c", code])
    assert proc.returncode == 0, (
        f"shadow subprocess failed (rc={proc.returncode})\n"
        f"--- stdout ---\n{proc.stdout}\n--- stderr ---\n{proc.stderr}"
    )
    return proc.stdout
