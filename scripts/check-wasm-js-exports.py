#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Hygiene gate: every `#[wasm_bindgen]` FREE FUNCTION compiled into
`crates/rdf-wasm/src/` must be re-exported from the npm package root,
`crates/rdf-wasm/js/index.mjs`.

The wasm size budget (`WASM_SIZE_BUDGET_BYTES` in the Makefile) is a reviewed
decision paid for exactly once, at the point a new `#[wasm_bindgen]` export links a
capability into the shipped artifact. That payment is worthless if the JS wrapper
module never re-exports the symbol: the bytes ship, the budget was raised to make
room for them, and no consumer of `@blackcatinformatics/purrdf` (whose `exports` map
in `crates/rdf-wasm/js/package.json` refuses a deep `./pkg/` import) can ever call
it. That is exactly how nine Description-Logic reasoning services — `entailClassify`,
`entailConsistency`, `entailEntails`, `entailExplainConclusion`,
`entailExtractModule`, `entailInstances`, `entailJustify`, `entailProfile`,
`entailRealize` — came to be a dark feature: compiled in, budgeted for, and
unreachable.

This script is a structural gate, not a doc-claims one (contrast
`scripts/check-doc-claims.py`, which restates generated NUMBERS): it discovers every
free function `#[wasm_bindgen]` decorates in the crate and every name
`crates/rdf-wasm/js/index.mjs` exports, and fails naming exactly the gap, in either
direction — a Rust export the JS module forgets, or a JS export whose Rust function
no longer exists (the reverse drift, which would mean the module is exporting a name
that throws `ReferenceError` at import time, or a stale name a rename left behind).

A "free function" is a `pub fn` at module scope, as opposed to a method inside a
`#[wasm_bindgen] impl Foo { ... }` block (reached through a class instance, e.g.
`dataset.serialize()` — those are out of this gate's scope: they are exercised
through the class the RDF/JS API already returns, not a bare import) or a
`#[wasm_bindgen] pub struct Foo` (a class, not a function; `ReasoningAnswer` and
`RegimeClosure` are checked by hand, the same way the JS test suite drives them by
calling the functions that return them). The distinction is structural: a module-scope
attribute (column 0, so not indented inside an `impl` block) that decorates a `pub fn`
rather than a `pub struct`/`impl`.

Pure text-over-committed-files: no cargo build, no wasm build, no Node. Run
standalone, or as part of `make check` / CI (see the Makefile and
`.github/workflows/ci.yaml`).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

_REPO = Path(__file__).resolve().parent.parent
_WASM_SRC = _REPO / "crates" / "rdf-wasm" / "src"
_INDEX_MJS = _REPO / "crates" / "rdf-wasm" / "js" / "index.mjs"

# A module-scope (column-0) `#[wasm_bindgen]` attribute, bare or with arguments
# (`#[wasm_bindgen(js_name = entailMaterialize)]`). Column 0 is what excludes every
# method inside a `#[wasm_bindgen] impl Foo { ... }` block, which this codebase always
# indents four spaces under the `impl`.
_ATTR_RE = re.compile(r"^#\[wasm_bindgen\b.*\]$")
_JS_NAME_RE = re.compile(r"js_name\s*=\s*(\w+)")
_PUB_FN_RE = re.compile(r"^pub fn (\w+)")


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def free_function_exports() -> dict[str, tuple[Path, int, str]]:
    """JS export name -> (file, attribute line, Rust fn name), one per free function.

    Walks each source line; on a column-0 `#[wasm_bindgen...]` attribute, follows the
    contiguous run of column-0 attribute (`#[...]`) and doc-comment (`///`) lines that
    follow it — collecting `js_name` from any of them — until it reaches the
    decorated item. If that item is `pub fn NAME`, the function is exported under its
    `js_name` if one was given, or under `NAME` verbatim otherwise (wasm-bindgen's own
    default: it does not case-convert an un-renamed free function). If the item is
    anything else (`pub struct`, `impl`), nothing is recorded — that free-function
    scoping is the whole point (see module docstring).
    """
    exports: dict[str, tuple[Path, int, str]] = {}
    for path in sorted(_WASM_SRC.glob("*.rs")):
        lines = _read(path).splitlines()
        n = len(lines)
        i = 0
        while i < n:
            if not _ATTR_RE.match(lines[i]):
                i += 1
                continue
            attr_line = i + 1
            js_name: str | None = None
            match = _JS_NAME_RE.search(lines[i])
            if match:
                js_name = match.group(1)
            j = i + 1
            while j < n and (lines[j].startswith("#[") or lines[j].startswith("///")):
                nested = _JS_NAME_RE.search(lines[j])
                if nested:
                    js_name = nested.group(1)
                j += 1
            if j < n:
                fn_match = _PUB_FN_RE.match(lines[j])
                if fn_match:
                    rust_name = fn_match.group(1)
                    name = js_name or rust_name
                    exports[name] = (path, attr_line, rust_name)
            i = j
    if not exports:
        raise SystemExit(
            f"check-wasm-js-exports: found no `#[wasm_bindgen]` free function under "
            f"{_WASM_SRC.relative_to(_REPO)} — the parser or the crate layout moved; "
            f"update this script rather than leaving the gate silently vacuous"
        )
    return exports


def index_mjs_export_names() -> set[str]:
    """Every name `crates/rdf-wasm/js/index.mjs` exports from the package root.

    Covers both re-export forms the file uses: the trailing `export { A, B, ... };`
    block (an `X as Y` rename, if one is ever added, counts under `Y`, the name a
    caller actually imports) and any top-level `export (async )?function NAME` /
    `export class NAME` declared directly in the wrapper (`ready`,
    `datasetToStream`, `streamToDataset` today).
    """
    text = _read(_INDEX_MJS)
    names: set[str] = set()
    for block in re.findall(r"export\s*\{([\s\S]*?)\}\s*;", text):
        for item in block.split(","):
            item = item.strip()
            if not item:
                continue
            names.add(item.split(" as ")[-1].strip())
    names.update(re.findall(r"^export\s+(?:async\s+)?function\s+(\w+)", text, re.MULTILINE))
    names.update(re.findall(r"^export\s+class\s+(\w+)", text, re.MULTILINE))
    if not names:
        raise SystemExit(
            f"check-wasm-js-exports: found no exported name in "
            f"{_INDEX_MJS.relative_to(_REPO)} at all — the file was rewritten in a "
            f"way this script's parser does not recognize"
        )
    return names


def main() -> int:
    rust_exports = free_function_exports()
    js_exports = index_mjs_export_names()
    rel_index = _INDEX_MJS.relative_to(_REPO)

    missing = sorted(set(rust_exports) - js_exports)
    problems: list[str] = []
    for name in missing:
        path, line, rust_name = rust_exports[name]
        rel = path.relative_to(_REPO)
        problems.append(
            f"{rel}:{line}: `{rust_name}` is exported to JS as `{name}` and compiled "
            f"into the wasm binary, but {rel_index} does not re-export `{name}` from "
            f"the package root — it ships bytes no consumer of the published "
            f"package can reach (the npm `exports` map refuses a deep `./pkg/` "
            f"import)"
        )

    if problems:
        print(
            "The wasm-bindgen free-function surface and the npm package root have "
            "drifted apart:\n",
            file=sys.stderr,
        )
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        print(
            f"\nAdd the missing name(s) to both the `import` block and the trailing "
            f"`export {{ ... }}` block of {rel_index}, in the file's existing "
            f"case-insensitive alphabetical order, and declare its/their TypeScript "
            f"signature(s) in crates/rdf-wasm/js/index.d.ts.",
            file=sys.stderr,
        )
        return 1

    print(
        f"OK: all {len(rust_exports)} #[wasm_bindgen] free function(s) in "
        f"{_WASM_SRC.relative_to(_REPO)} are re-exported from {rel_index}."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
