# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Top-level module exports mirror the Rust umbrella crate.

`purrdf` must present the RDF surface at its root and every other engine as a
top-level submodule (`purrdf.shapes`, `purrdf.shex`, `purrdf.entail`,
`purrdf.slice`, `purrdf.gts`) so no caller ever reaches into `purrdf_native`.
Both `import purrdf.<engine>` and attribute access must resolve, and the public
compat/shadow code must never name `purrdf_native`.
"""

from __future__ import annotations

import importlib
from pathlib import Path

import pytest

_ENGINES = [
    "purrdf.shapes",
    "purrdf.shex",
    "purrdf.entail",
    "purrdf.slice",
    "purrdf.gts",
]


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
    assert importlib.import_module("purrdf.entail") is purrdf.entail
    assert importlib.import_module("purrdf.slice") is purrdf.slice
    assert importlib.import_module("purrdf.gts") is purrdf.gts


def test_shapes_is_canonical_name_shacl_is_alias() -> None:
    """SHACL is `purrdf.shapes` (Rust parity) with `purrdf.shacl` as an alias."""
    import purrdf

    assert purrdf.shacl is purrdf.shapes
    assert callable(purrdf.shapes.validate)


def test_regime_entailment_is_not_shacl_rule_entailment() -> None:
    """`purrdf.entail` and `purrdf.shapes.entail` are different mechanisms.

    The names collide by one namespace level and the distinction is load-bearing:
    `purrdf.entail` closes a document under a SPARQL entailment regime's own rule
    table, while `purrdf.shapes.entail` applies the SHACL-AF `sh:rule`s a shapes
    graph declares. Neither may quietly become the other.
    """
    import purrdf

    assert purrdf.entail is not purrdf.shapes.entail
    assert callable(purrdf.shapes.entail)
    assert not callable(purrdf.entail)


def test_engines_expose_expected_surface() -> None:
    """Each engine surfaces its primary entry points off the top-level name."""
    import purrdf

    assert hasattr(purrdf.shapes, "validate")
    assert hasattr(purrdf.shapes, "Shapes")
    assert hasattr(purrdf.shex, "validate")
    assert hasattr(purrdf.entail, "materialize")
    assert hasattr(purrdf.entail, "materialize_nt")
    assert hasattr(purrdf.entail, "rules")
    assert hasattr(purrdf.entail, "implemented_rules")
    assert hasattr(purrdf.entail, "extensions")
    assert hasattr(purrdf.entail, "Regime")
    assert hasattr(purrdf.slice, "SliceCatalog")
    assert hasattr(purrdf.gts, "gts_from_quads")
    assert callable(purrdf.project)
    assert callable(purrdf.project_artifacts)
    assert callable(purrdf.lift)
    assert hasattr(purrdf, "ProjectionPackage")
    assert hasattr(purrdf, "ProjectionProgress")
    assert hasattr(purrdf, "ProjectionStream")
    assert hasattr(purrdf, "ProjectionLoss")
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


def test_stub_signatures_match_the_built_bindings() -> None:
    """The shipped `.pyi` must declare the arity the extension actually accepts.

    A PEP 561 package ships its stub as a CONTRACT: mypy checks caller code against
    the declaration, never against the compiled module. So a stub that declares
    fewer parameters than the binding takes is worse than an unstubbed package —
    the type checker approves a call that raises `TypeError` and rejects the call
    that works. `entail.materialize` and `entail.materialize_nt` drifted exactly
    that way when `program` became a required argument.

    Names alone are not enough (`check-doc-claims.py` already reads those), so this
    compares PARAMETER LISTS: PyO3 publishes the real signature on
    `__text_signature__`, and the stub's is parsed out of the declaration. What is
    compared is the ordered parameter names plus whether each HAS a default — not
    the default's value, because a stub idiomatically writes `= ...` where the
    binding writes the concrete `=0`, and demanding those match would flag correct
    stubs. The `*` keyword-only marker is dropped: a stub may legitimately be
    stricter than the runtime, since that can only reject a working call, never
    approve a failing one.

    The sweep is over each engine's whole surface rather than a hand-listed pair,
    so a future entry point is covered without anyone remembering to add it.
    """
    import re

    import purrdf

    stub = Path(purrdf.__file__).with_name("__init__.pyi")
    text = stub.read_text(encoding="utf-8")

    def shape(params: str) -> list[str]:
        """Ordered `name` / `name=` tokens, annotations and default VALUES removed."""
        out: list[str] = []
        for raw in re.sub(r"\[[^]]*\]", "", params).split(","):
            param = raw.strip()
            if not param or param in {"self", "*", "/"}:
                continue
            name = param.split(":")[0].split("=")[0].strip()
            has_default = "=" in param
            out.append(f"{name}=" if has_default else name)
        return out

    checked = 0
    for engine in ("entail", "shapes", "shex"):
        body = re.search(rf"\nclass {engine}:\n(.*?)(?:\n\S|\Z)", text, re.DOTALL)
        assert body, f"no `class {engine}:` block in {stub}"
        for name, params in re.findall(
            # `[^)]` already matches a newline — a negated class is not narrowed by
            # `re.DOTALL`, which only governs `.` — so the old `(?:[^)]|\n)*?` gave every
            # newline TWO ways to be matched and 2**n paths to backtrack through. Same
            # language, one path.
            r"^    def (\w+)\(\s*\n?([^)]*?)\)\s*->", body.group(1), re.MULTILINE
        ):
            fn = getattr(getattr(purrdf, engine), name, None)
            signature = getattr(fn, "__text_signature__", None)
            if fn is None or signature is None:
                continue  # a class or a non-PyO3 callable: nothing to compare
            declared = shape(params)
            actual = [
                token
                for token in shape(signature.strip("()"))
                if token != "$module"
            ]
            assert declared == actual, (
                f"purrdf.{engine}.{name}: the stub declares {declared} but the "
                f"binding accepts {actual}. mypy checks callers against the stub, "
                f"so this drift approves a call that raises TypeError, or rejects "
                f"one that works."
            )
            checked += 1

    # A parser that silently matched nothing would make every assertion vacuous.
    assert checked >= 15, f"only {checked} signature(s) compared; the parser regressed"
