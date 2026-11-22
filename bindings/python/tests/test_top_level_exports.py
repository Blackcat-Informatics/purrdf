# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""Top-level module exports mirror the Rust umbrella crate.

`purrdf` must present the RDF surface at its root and every other engine as a
top-level submodule (`purrdf.shapes`, `purrdf.shex`, `purrdf.entail`,
`purrdf.retrieval`, `purrdf.slice`, `purrdf.gts`) so no caller ever reaches into
`purrdf_native`.
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
    "purrdf.retrieval",
    "purrdf.slice",
    "purrdf.gts",
]


def _import_export(dotted: str):
    module = importlib.import_module(dotted)
    package, _, name = dotted.rpartition(".")
    assert getattr(importlib.import_module(package), name, None) is module, f"{dotted} attribute must match its import"
    return module


@pytest.mark.parametrize("dotted", _ENGINES)
def test_engine_submodule_is_importable(dotted: str) -> None:
    """Import and attribute access resolve to the same engine module."""
    _import_export(dotted)


def test_shapes_is_canonical_name_shacl_is_alias() -> None:
    """SHACL is `purrdf.shapes` (Rust parity) with `purrdf.shacl` as an alias."""
    import purrdf

    assert purrdf.shacl is purrdf.shapes


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
    """Verify installed stub exports, the complete book inventory and PyO3 signatures."""
    import ast
    import re

    import purrdf

    stub = Path(purrdf.__file__).with_name("__init__.pyi")
    text = stub.read_text(encoding="utf-8")
    def declarations(body):
        named = {}
        for node in body:
            if isinstance(node, (ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
                named[node.name] = node
            elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name):
                named[node.target.id] = node.value
            elif isinstance(node, ast.Assign):
                for name in node.targets:
                    if isinstance(name, ast.Name):
                        named[name.id] = node.value
        return named

    def check_exports(module, qualified_name, members, root_members, namespaces=()):
        for name, declaration in members.items():
            if name.startswith("_") or name in namespaces:
                continue
            seen = set()
            while isinstance(declaration, ast.Name) and declaration.id in root_members:
                assert declaration.id not in seen, f"cyclic stub alias for {qualified_name}.{name}"
                seen.add(declaration.id)
                declaration = root_members[declaration.id]
            value = getattr(module, name, None)
            if isinstance(declaration, (ast.FunctionDef, ast.AsyncFunctionDef)):
                assert callable(value), f"{qualified_name}.{name} is declared but not callable"
            elif isinstance(declaration, ast.ClassDef):
                if declaration.name in namespaces:
                    assert value is getattr(module, declaration.name), f"{qualified_name}.{name} must alias {declaration.name}"
                # TypedDict declarations describe the shape of returned dictionaries.
                elif not any(ast.unparse(base) == "TypedDict" for base in declaration.bases):
                    assert isinstance(value, type), f"{qualified_name}.{name} is declared but not a type"

    surface = declarations(ast.parse(text).body)
    assert set(purrdf.__all__) <= surface.keys(), "every public root export needs a canonical declaration"
    namespaces = [dotted.rsplit(".", 1)[1] for dotted in _ENGINES]
    check_exports(purrdf, "purrdf", surface, surface, namespaces)
    for name in namespaces:
        block = surface[name]
        assert isinstance(block, ast.ClassDef), f"expected a stub namespace for purrdf.{name}"
        check_exports(getattr(purrdf, name), f"purrdf.{name}", declarations(block.body), surface)
    assert set(purrdf.gts.__all__) <= set(dir(purrdf.gts))

    reference = Path(__file__).resolve().parents[3] / "docs/book/src/reference/python-api-signatures.md"
    blocks = re.findall(
        r"<!-- python-api-stub: ([^\s]+) -->\n```pyi\n(.*?)\n```",
        reference.read_text(encoding="utf-8"), re.DOTALL,
    )
    package = stub.parent
    canonical = {path.relative_to(package).as_posix(): path for path in package.rglob("*.pyi")}
    assert canonical and len(blocks) == len(dict(blocks)), "empty stub discovery or duplicate inventory"
    assert set(dict(blocks)) == set(canonical), "the book must cover every installed stub"
    for name, declaration in blocks:
        documented = ast.dump(ast.parse(declaration))
        installed = ast.parse(canonical[name].read_text(encoding="utf-8"))
        assert documented == ast.dump(installed), f"complete book signature inventory drifted from {name}"
        if name != "__init__.pyi":
            qualified = "purrdf." + name.removesuffix(".pyi").replace("/", ".").removesuffix(".__init__")
            members = declarations(installed.body)
            check_exports(_import_export(qualified), qualified, members, members)

    def shape(params: str) -> list[str]:
        """Ordered `name` / `name=` tokens, annotations and default VALUES removed."""
        out: list[str] = []
        for raw in re.sub(r"\[[^]]*\]", "", params).split(","):
            param = raw.strip()
            if not param or param in {"self", "$self", "*", "/"}:
                continue
            name = param.split(":")[0].split("=")[0].strip()
            has_default = "=" in param
            out.append(f"{name}=" if has_default else name)
        return out

    engines = ("entail", "shapes", "shex", "retrieval", "GtsFoldViewNative")
    checked: dict[str, int] = dict.fromkeys(engines, 0)
    for engine in engines:
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
            assert callable(fn), f"purrdf.{engine}.{name} is declared in the stub but not callable in the built module"
            if (signature := getattr(fn, "__text_signature__", None)) is None:
                continue  # a non-PyO3 callable: nothing to compare
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
            checked[engine] += 1

    silent = sorted(engine for engine, count in checked.items() if count == 0)
    assert not silent, (
        f"no signature compared for {silent}: the stub block did not parse, or the "
        f"engine's entry points are no longer reachable through `purrdf`"
    )
    total = sum(checked.values())
    assert total >= 15, f"only {total} signature(s) compared; the parser regressed"
