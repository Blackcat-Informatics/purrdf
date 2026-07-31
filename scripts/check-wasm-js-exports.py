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
_INDEX_DTS = _REPO / "crates" / "rdf-wasm" / "js" / "index.d.ts"

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


def index_mjs_bound_names() -> set[str]:
    """Every name BOUND in `index.mjs`'s module scope — imported or declared locally.

    An export block only names what the module offers; it does not make the name exist.
    A name exported but never bound makes the whole module fail to load — ES modules
    resolve exports at link time, so `Export 'X' is not defined in module` takes down the
    package root entirely, not just that one symbol.

    This gate's own docstring promised to catch "a JS export whose Rust function no longer
    exists ... a name that throws at import time", and its remediation text tells a reader
    to fix "both the `import` block and the trailing `export` block" — while it read only
    the second. Deleting one name from the import block left it reporting OK against a
    package root that would not load at all.
    """
    text = _read(_INDEX_MJS)
    names: set[str] = set()
    # The `import init, { A, B as C, ... } from "..."` block.
    for block in re.findall(r"^import\s+[\s\S]*?\{([\s\S]*?)\}\s*from\s", text, re.MULTILINE):
        for item in block.split(","):
            item = item.strip()
            if item:
                # `X as Y` binds Y; a bare `X` binds X.
                names.add(item.split(" as ")[-1].strip())
    # The default binding (`import init, {...}`) and any bare default import.
    names.update(re.findall(r"^import\s+(\w+)\s*,", text, re.MULTILINE))
    # Anything declared in the module itself.
    names.update(
        re.findall(
            r"^(?:export\s+)?(?:async\s+)?function\s+(\w+)", text, re.MULTILINE
        )
    )
    names.update(re.findall(r"^(?:export\s+)?class\s+(\w+)", text, re.MULTILINE))
    names.update(
        re.findall(r"^(?:export\s+)?(?:const|let|var)\s+(\w+)", text, re.MULTILINE)
    )
    if not names:
        raise SystemExit(
            f"check-wasm-js-exports: found no bound name in "
            f"{_INDEX_MJS.relative_to(_REPO)} at all — the file was rewritten in a "
            f"way this script's parser does not recognize"
        )
    return names


def class_exports() -> set[str]:
    """Every `#[wasm_bindgen] pub struct` name — a CLASS, reached through an instance.

    Needed by the reverse-drift check rather than the forward one: a class is a
    legitimate package-root export that is not a free function, so without this the
    reverse check would flag `Dataset` and `ReasoningAnswer` as stale.
    """
    names: set[str] = set()
    for path in sorted(_WASM_SRC.rglob("*.rs")):
        lines = _read(path).splitlines()
        for index, line in enumerate(lines):
            if not _ATTR_RE.match(line.strip()):
                continue
            for follower in lines[index + 1 :]:
                stripped = follower.strip()
                if not stripped or stripped.startswith(("#[", "//", "///")):
                    continue
                match = re.match(r"pub struct (\w+)", stripped)
                if match:
                    names.add(match.group(1))
                break
    return names


def index_mjs_local_names() -> set[str]:
    """Names `index.mjs` DEFINES itself, so exporting them is not stale drift.

    The wrapper authors real glue — `ready`, `datasetToStream`, `streamToDataset`,
    the polymorphic `DataFactory` subclass — and those have no Rust counterpart by
    design. Derived from the file rather than hand-listed, so adding glue does not
    require editing this script.
    """
    text = _read(_INDEX_MJS)
    names = set(
        re.findall(r"^(?:export\s+)?(?:async\s+)?function\s+(\w+)", text, re.MULTILINE)
    )
    names |= set(re.findall(r"^(?:export\s+)?class\s+(\w+)", text, re.MULTILINE))
    names |= set(
        re.findall(r"^(?:export\s+)?const\s+(\w+)\s*=", text, re.MULTILINE)
    )
    return names


def index_dts_declared_names() -> set[str]:
    """Every name the TypeScript declaration file declares.

    A JS re-export with no `.d.ts` declaration is reachable from JavaScript and
    invisible to TypeScript, which is the same dark-feature shape one type system
    down: the bytes ship, the export exists, and a TS consumer cannot call it
    without an error.
    """
    text = _read(_INDEX_DTS)
    names = set(
        re.findall(
            r"^export\s+(?:declare\s+)?(?:async\s+)?function\s+(\w+)",
            text,
            re.MULTILINE,
        )
    )
    names |= set(re.findall(r"^export\s+(?:declare\s+)?class\s+(\w+)", text, re.MULTILINE))
    names |= set(re.findall(r"^export\s+(?:declare\s+)?const\s+(\w+)", text, re.MULTILINE))
    for block in re.findall(r"export\s*\{([\s\S]*?)\}\s*;", text):
        for item in block.split(","):
            if item.strip():
                names.add(item.strip().split(" as ")[-1].strip())
    if not names:
        raise SystemExit(
            f"check-wasm-js-exports: found no declaration in "
            f"{_INDEX_DTS.relative_to(_REPO)} — the parser or the file moved; update "
            f"this script rather than leaving the TypeScript surface ungated"
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

    # The REVERSE direction. A name the package root exports whose Rust function no
    # longer exists throws `ReferenceError` at import time and takes the whole module
    # down with it, so it is the more severe of the two drifts, not the lesser.
    # Everything legitimately exported that is not a free function is DERIVED — the
    # wasm-bindgen classes and the wrapper's own glue — so this needs no allowlist to
    # keep in sync.
    legitimate = set(rust_exports) | class_exports() | index_mjs_local_names()
    for name in sorted(js_exports - legitimate):
        problems.append(
            f"{rel_index}: re-exports `{name}` from the package root, but no "
            f"`#[wasm_bindgen]` free function, no `#[wasm_bindgen] pub struct` and no "
            f"local definition in the wrapper provides it — importing the package "
            f"root throws `ReferenceError` on a stale name, taking the whole module "
            f"with it. Either the Rust export was renamed or removed without updating "
            f"the wrapper, or the name is a leftover"
        )

    # THE BINDING CHECK. The two directions above compare the export block against the
    # RUST surface, so a name deleted from the wrapper's `import` block stays "legitimate"
    # — it still exists in Rust — while the module it is exported from can no longer
    # resolve it. ES modules link exports eagerly, so that single missing import makes the
    # ENTIRE package root fail to load, which is strictly worse than one dark symbol and
    # was invisible to this gate until it read the import block too.
    bound = index_mjs_bound_names()
    for name in sorted(js_exports - bound):
        problems.append(
            f"{rel_index}: exports `{name}` but never binds it — the name is in no "
            f"`import {{ ... }} from` block and is declared nowhere in the wrapper. An ES "
            f"module resolves its exports at link time, so this does not merely hide "
            f"`{name}`: importing the package root fails outright with `Export '{name}' "
            f"is not defined in module`, and every other export goes down with it"
        )

    # And the TypeScript surface, which the remediation text below already promises.
    dts_declared = index_dts_declared_names()
    rel_dts = _INDEX_DTS.relative_to(_REPO)
    for name in sorted(set(rust_exports) & js_exports - dts_declared):
        problems.append(
            f"{rel_dts}: `{name}` is re-exported from the package root but has no "
            f"TypeScript declaration, so a TS consumer cannot call it without an "
            f"error — reachable from JavaScript and dark to TypeScript"
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
