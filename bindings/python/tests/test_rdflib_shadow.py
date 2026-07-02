# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Task 7 (#11): the opt-in top-level ``rdflib`` shadow distribution.

``purrdf-rdflib`` (``bindings/python-rdflib-shadow``) ships a top-level
``rdflib`` package that re-exports :mod:`purrdf.compat.rdflib`, so third-party
code doing a literal ``import rdflib`` transparently runs on purrdf.

HARD CONSTRAINT — this pytest process has the *real* rdflib 7.6 installed as the
differential oracle, and the shadow ALSO claims the ``rdflib`` import name; the
two must never co-inhabit one interpreter. So every assertion here runs in a
**subprocess** with the shadow dir prepended to ``PYTHONPATH`` (child
``import rdflib`` picks the shadow), and the parent's ``sys.modules`` /
``sys.path`` are never mutated. The ``oracle`` fixture below re-checks that the
parent still sees the genuine rdflib, untouched.
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path
from types import ModuleType

import pytest

# bindings/python/tests/ -> bindings/ -> python-rdflib-shadow/
_SHADOW_DIR = Path(__file__).resolve().parent.parent.parent / "python-rdflib-shadow"


def _run_in_shadow(code: str) -> str:
    """Run ``code`` in a child interpreter whose ``import rdflib`` is the shadow.

    Prepends the shadow distribution's package root to ``PYTHONPATH`` so a plain
    ``import rdflib`` in the child resolves to the purrdf shadow rather than the
    real rdflib installed in this (parent) environment.
    """
    env = dict(os.environ)
    existing = env.get("PYTHONPATH", "")
    env["PYTHONPATH"] = (
        f"{_SHADOW_DIR}{os.pathsep}{existing}" if existing else str(_SHADOW_DIR)
    )
    proc = subprocess.run(
        [sys.executable, "-c", code],
        env=env,
        capture_output=True,
        text=True,
        check=False,
    )
    assert proc.returncode == 0, (
        f"shadow subprocess failed (rc={proc.returncode})\n"
        f"--- stdout ---\n{proc.stdout}\n--- stderr ---\n{proc.stderr}"
    )
    return proc.stdout


def test_shadow_dir_exists() -> None:
    """The separate shadow distribution ships a top-level ``rdflib`` package."""
    assert (_SHADOW_DIR / "rdflib" / "__init__.py").is_file()
    assert (_SHADOW_DIR / "rdflib" / "py.typed").is_file()
    assert (_SHADOW_DIR / "pyproject.toml").is_file()


def test_shadow_resolves_to_purrdf() -> None:
    """``import rdflib`` in the child loads the shadow (a purrdf-backed package)."""
    out = _run_in_shadow(
        "import rdflib; print(rdflib.__file__); "
        "print(rdflib.Graph.__module__)"
    )
    lines = out.strip().splitlines()
    assert str(_SHADOW_DIR) in lines[0]
    assert lines[1].startswith("purrdf")


def test_shadow_parse_and_serialize() -> None:
    """A round-trip: parse N-Triples, count quads, serialize Turtle."""
    code = (
        "import rdflib\n"
        "g = rdflib.Graph()\n"
        "g.parse(data='<http://example.org/s> <http://example.org/p> "
        "<http://example.org/o> .', format='nt')\n"
        "assert len(g) == 1, len(g)\n"
        "out = g.serialize(format='turtle')\n"
        "assert 'example.org' in out, out\n"
        "print('OK')\n"
    )
    assert _run_in_shadow(code).strip() == "OK"


def test_shadow_namespace_and_term_imports() -> None:
    """``from rdflib.namespace import RDF`` / ``from rdflib import URIRef`` work."""
    code = (
        "from rdflib.namespace import RDF\n"
        "from rdflib import URIRef, Literal, BNode\n"
        "assert URIRef.__module__.startswith('purrdf'), URIRef.__module__\n"
        "assert Literal.__module__.startswith('purrdf'), Literal.__module__\n"
        "assert BNode.__module__.startswith('purrdf'), BNode.__module__\n"
        "assert type(RDF).__module__.startswith('purrdf'), type(RDF).__module__\n"
        "print('OK')\n"
    )
    assert _run_in_shadow(code).strip() == "OK"


def test_shadow_plugins_sparql_importable() -> None:
    """``import rdflib.plugins.sparql`` resolves through the shadow."""
    code = (
        "import rdflib.plugins.sparql as s\n"
        "assert s.__name__.startswith('purrdf'), s.__name__\n"
        "import sys\n"
        "assert 'rdflib.plugins.sparql' in sys.modules\n"
        "assert 'rdflib.plugins' in sys.modules\n"
        "print('OK')\n"
    )
    assert _run_in_shadow(code).strip() == "OK"


def test_shadow_submodule_identity() -> None:
    """Shadow submodules are the very same objects as the purrdf compat ones."""
    code = (
        "import rdflib\n"
        "import rdflib.namespace\n"
        "import purrdf.compat.rdflib as c\n"
        "import purrdf.compat.rdflib.namespace as cn\n"
        "assert rdflib.namespace is cn\n"
        "assert rdflib.Graph is c.Graph\n"
        "print('OK')\n"
    )
    assert _run_in_shadow(code).strip() == "OK"


def test_parent_oracle_is_real_rdflib(oracle: ModuleType) -> None:
    """The parent process keeps the genuine rdflib oracle — never the shadow.

    Guards the hard constraint: exercising the shadow (only ever in a child)
    must not leak into or shadow this differential-oracle interpreter.
    """
    assert not oracle.URIRef.__module__.startswith("purrdf")
    assert str(_SHADOW_DIR) not in (getattr(oracle, "__file__", "") or "")


@pytest.mark.parametrize("dotted", ["rdflib", "rdflib.namespace", "rdflib.term"])
def test_parent_sys_modules_untouched(dotted: str) -> None:
    """No shadow module ever entered the parent's ``sys.modules``."""
    mod = sys.modules.get(dotted)
    if mod is None:
        return
    assert str(_SHADOW_DIR) not in (getattr(mod, "__file__", "") or "")
    assert not getattr(mod, "__name__", "").startswith("purrdf")
