# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Top-level module exports mirror the Rust umbrella crate.

`purrdf` must present the RDF surface at its root and every other engine as a
top-level submodule (`purrdf.shapes`, `purrdf.shex`, `purrdf.slice`, `purrdf.gts`)
so no caller ever reaches into `purrdf_native`. Both `import purrdf.<engine>` and
attribute access must resolve, and the public compat/shadow code must never name
`purrdf_native`.
"""

from __future__ import annotations

import importlib
from pathlib import Path

import pytest

_ENGINES = ["purrdf.shapes", "purrdf.shex", "purrdf.slice", "purrdf.gts"]


@pytest.mark.parametrize("dotted", _ENGINES)
def test_engine_submodule_is_importable(dotted: str) -> None:
    """`import purrdf.<engine>` resolves (not just attribute access)."""
    module = importlib.import_module(dotted)
    assert module is not None


def test_attribute_access_matches_import() -> None:
    """Attribute access and `import` yield the same submodule objects."""
    import purrdf

    assert importlib.import_module("purrdf.shapes") is purrdf.shapes
    assert importlib.import_module("purrdf.shex") is purrdf.shex
    assert importlib.import_module("purrdf.slice") is purrdf.slice
    assert importlib.import_module("purrdf.gts") is purrdf.gts


def test_shapes_is_canonical_name_shacl_is_alias() -> None:
    """SHACL is `purrdf.shapes` (Rust parity) with `purrdf.shacl` as an alias."""
    import purrdf

    assert purrdf.shacl is purrdf.shapes
    assert callable(purrdf.shapes.validate)


def test_engines_expose_expected_surface() -> None:
    """Each engine surfaces its primary entry points off the top-level name."""
    import purrdf

    assert hasattr(purrdf.shapes, "validate")
    assert hasattr(purrdf.shapes, "Shapes")
    assert hasattr(purrdf.shex, "validate")
    assert hasattr(purrdf.slice, "SliceCatalog")
    assert hasattr(purrdf.gts, "gts_from_quads")
    assert set(purrdf.gts.__all__) <= set(dir(purrdf.gts))


def test_no_purrdf_native_leak_in_public_code() -> None:
    """The public compat/shadow code paths never name `purrdf_native`.

    Only the package `__init__.py` shim may reference the native cdylib; every
    caller-facing module goes through the top-level `purrdf` surface.
    """
    pkg_root = Path(__file__).resolve().parent.parent / "python" / "src" / "purrdf"
    public_dirs = [pkg_root / "compat"]
    offenders: list[str] = []
    for base in public_dirs:
        for path in base.rglob("*.py"):
            if "purrdf_native" in path.read_text(encoding="utf-8"):
                offenders.append(str(path.relative_to(pkg_root)))
    assert not offenders, f"purrdf_native leaked into public code: {offenders}"
