#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Hygiene gate: every conclusion-directed entailment service `purrdf-entail`
publishes must be reachable from ALL FOUR host shapes — Rust, Python, WASM and the
C ABI.

A capability reachable from one caller shape and dark from another is a defect this
repository has already paid for: nine Description-Logic reasoning services were
compiled into the wasm artifact, budgeted for, and never re-exported from the npm
package root, so they shipped as bytes no consumer could call. `check-wasm-js-exports.py`
closes that hole for ONE host. This closes it across all four, for one surface.

# The service set is DERIVED, not listed

The three services are read out of `crates/entail/src/entails/mod.rs` and
`crates/entail/src/entails/warrant.rs` — every `pub fn` at module scope in the
conclusion-directed service. A fourth service arriving is therefore a FAILURE of this
gate until its four host bindings and its row in `_HOST_NAMES` exist, rather than a
quiet three-out-of-four rollout. The per-host SPELLINGS are a naming convention and
have to be written down; the SET is not, and is not.

# Each needle is SCOPED to that host's block

An unscoped substring search is the trap this gate exists to avoid: `certain_answers`
occurs in a doc comment on every one of these files, and a gate satisfied by prose
proves nothing. So each host is parsed for the construct that actually makes the
capability callable —

* the shared string boundary: a column-0 `pub fn <name>(` in `crates/validate/src/regime.rs`,
  AND that name inside the `pub use regime::{ … };` block of `crates/validate/src/lib.rs`,
  because a `pub fn` in a module nothing re-exports is not on the crate's surface;
* Python: a `fn <name>(` decorated with `#[pyfunction]`, AND a
  `wrap_pyfunction!(<name>, m)` inside the body of `register`, because an unregistered
  `#[pyfunction]` is compiled and unimportable;
* Python types: a `def <name>(` inside the `class entail:` block of the stub, not at
  module scope — a free function in the stub would type-check a call nobody can make;
* WASM: a `#[wasm_bindgen(js_name = <JsName>)]` in `crates/rdf-wasm/src/entail.rs`, AND
  `<JsName>` in BOTH the `import init, { … } from` block and the `export { … };` block
  of `crates/rdf-wasm/js/index.mjs` — the two are separate lists and a name in only one
  of them throws at import time — AND an `export function <JsName>(` in `index.d.ts`;
* the C ABI: a `pub unsafe extern "C" fn <sym>(` under `#[unsafe(no_mangle)]`, AND a
  DECLARATION of `<sym>` in the committed `crates/rdf-capi/include/purrdf.h`, because
  cbindgen does not expand macros and a macro-generated entry point would link into the
  shared object and never reach the header.

Pure text over committed files: no cargo build, no wasm build, no Node, no Python
import. Run standalone or from `make check` / CI.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

_REPO = Path(__file__).resolve().parent.parent

# Where the SET comes from: the conclusion-directed service's own public entry points.
_SERVICE_SOURCES = (
    Path("crates/entail/src/entails/mod.rs"),
    Path("crates/entail/src/entails/warrant.rs"),
)

# The per-host spellings. The KEYS must be exactly the derived set — a service with no
# row here fails the gate, and a row naming no service fails it too, so this table
# cannot drift from `purrdf-entail` in either direction.
#
# `entails` is spelled `graph_entails` on every host because `entails` was already
# taken by the OWL 2 Direct-Semantics tableau's axiom-entailment service, which asks a
# different question of a different calculus and renders a different certificate.
# `verify` is spelled `verify_entailment` because a bare `verify` says nothing about
# what is being verified on a surface that also validates SHACL and ShEx.
_HOST_NAMES: dict[str, dict[str, str]] = {
    "certain_answers": {
        "boundary": "certain_answers_to_string",
        "python": "certain_answers",
        "wasm": "entailCertainAnswers",
        "capi": "purrdf_entail_certain_answers",
    },
    "entails": {
        "boundary": "graph_entails_to_string",
        "python": "graph_entails",
        "wasm": "entailGraphEntails",
        "capi": "purrdf_entail_graph_entails",
    },
    "verify": {
        "boundary": "verify_entailment_to_string",
        "python": "verify_entailment",
        "wasm": "entailVerifyEntailment",
        "capi": "purrdf_entail_verify_entailment",
    },
}

_PUB_FN_RE = re.compile(r"^pub fn (\w+)[(<]", re.MULTILINE)


def _read(relative: str | Path) -> str:
    path = _REPO / relative
    if not path.is_file():
        raise SystemExit(f"check-entailment-surface: {relative} is missing")
    return path.read_text(encoding="utf-8")


def _block(text: str, opener: str, closer: str, what: str) -> str:
    """The body between the first `opener` and the next `closer` after it."""
    start = text.find(opener)
    if start < 0:
        raise SystemExit(
            f"check-entailment-surface: cannot find the {what} block (opener {opener!r}); "
            f"the file's layout moved — update this gate rather than leaving it vacuous"
        )
    end = text.find(closer, start + len(opener))
    if end < 0:
        raise SystemExit(
            f"check-entailment-surface: the {what} block opened at {opener!r} is unterminated"
        )
    return text[start + len(opener) : end]


def derived_services() -> set[str]:
    """Every `pub fn` at module scope in the conclusion-directed service."""
    found: set[str] = set()
    for source in _SERVICE_SOURCES:
        text = _read(source)
        # Only the file's own top-level items: everything under `#[cfg(test)]` is
        # indented inside `mod tests`, so a column-0 `pub fn` cannot be a test helper.
        found.update(_PUB_FN_RE.findall(text))
    if not found:
        raise SystemExit(
            "check-entailment-surface: found no `pub fn` in "
            f"{', '.join(str(p) for p in _SERVICE_SOURCES)} — the crate layout moved; "
            "update this gate rather than leaving it silently vacuous"
        )
    return found


def _indented_class_block(text: str, header: str) -> str:
    """The body of an indented `class`/`impl`-style block, by indentation."""
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if line.strip() == header:
            indent = len(line) - len(line.lstrip())
            body: list[str] = []
            for following in lines[index + 1 :]:
                if following.strip() and (len(following) - len(following.lstrip())) <= indent:
                    break
                body.append(following)
            return "\n".join(body)
    raise SystemExit(
        f"check-entailment-surface: cannot find the block headed {header!r}; the file's "
        f"layout moved — update this gate rather than leaving it vacuous"
    )


def missing_bindings(services: set[str]) -> list[str]:
    """One message per host binding a service is missing, scoped to that host's block."""
    boundary = _read("crates/validate/src/regime.rs")
    boundary_reexports = _block(
        _read("crates/validate/src/lib.rs"), "pub use regime::{", "};", "validate re-export"
    )
    py_source = _read("bindings/python/src/py_entail.rs")
    py_register = _block(
        py_source, "pub(crate) fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {", "\n}\n", "python register"
    )
    py_stub = _indented_class_block(
        _read("bindings/python/python/src/purrdf/__init__.pyi"), "class entail:"
    )
    wasm_source = _read("crates/rdf-wasm/src/entail.rs")
    index_mjs = _read("crates/rdf-wasm/js/index.mjs")
    mjs_import = _block(index_mjs, "import init, {", "} from", "index.mjs import")
    mjs_export = _block(index_mjs, "\nexport {", "\n};", "index.mjs export")
    index_dts = _read("crates/rdf-wasm/js/index.d.ts")
    capi_source = _read("crates/rdf-capi/src/entail.rs")
    capi_header = _read("crates/rdf-capi/include/purrdf.h")

    problems: list[str] = []
    for service in sorted(services):
        names = _HOST_NAMES[service]
        checks = [
            (
                "rust boundary",
                "crates/validate/src/regime.rs",
                f"pub fn {names['boundary']}(" in boundary,
            ),
            (
                "rust re-export",
                "crates/validate/src/lib.rs",
                re.search(rf"\b{names['boundary']}\b", boundary_reexports) is not None,
            ),
            (
                "python function",
                "bindings/python/src/py_entail.rs",
                f"#[pyfunction]" in py_source and f"\nfn {names['python']}(" in py_source,
            ),
            (
                "python registration",
                "bindings/python/src/py_entail.rs::register",
                f"wrap_pyfunction!({names['python']}, m)" in py_register,
            ),
            (
                "python stub",
                "bindings/python/python/src/purrdf/__init__.pyi::class entail",
                f"def {names['python']}(" in py_stub,
            ),
            (
                "wasm binding",
                "crates/rdf-wasm/src/entail.rs",
                f"#[wasm_bindgen(js_name = {names['wasm']})]" in wasm_source,
            ),
            (
                "npm import binding",
                "crates/rdf-wasm/js/index.mjs",
                re.search(rf"^\s*{names['wasm']},$", mjs_import, re.MULTILINE) is not None,
            ),
            (
                "npm re-export",
                "crates/rdf-wasm/js/index.mjs",
                re.search(rf"^\s*{names['wasm']},$", mjs_export, re.MULTILINE) is not None,
            ),
            (
                "npm type declaration",
                "crates/rdf-wasm/js/index.d.ts",
                f"export function {names['wasm']}(" in index_dts,
            ),
            (
                "c abi entry point",
                "crates/rdf-capi/src/entail.rs",
                f'pub unsafe extern "C" fn {names["capi"]}(' in capi_source,
            ),
            (
                "c abi header declaration",
                "crates/rdf-capi/include/purrdf.h",
                re.search(rf"^int32_t {names['capi']}\(", capi_header, re.MULTILINE) is not None,
            ),
        ]
        for host, where, present in checks:
            if not present:
                problems.append(
                    f"  • {service}: NO {host} — expected {names} to be reachable from {where}"
                )
    return problems


def main() -> int:
    services = derived_services()
    unmapped = services - _HOST_NAMES.keys()
    if unmapped:
        print(
            "check-entailment-surface: purrdf-entail publishes "
            f"{sorted(unmapped)} with no host spellings. A capability reachable from Rust "
            "and dark from Python, WASM and C is the defect this gate exists for: add the "
            "four bindings and a row in _HOST_NAMES.",
            file=sys.stderr,
        )
        return 1
    stale = _HOST_NAMES.keys() - services
    if stale:
        print(
            f"check-entailment-surface: _HOST_NAMES names {sorted(stale)}, which "
            "purrdf-entail no longer publishes — the host bindings are calling a service "
            "that is gone, or the derivation moved.",
            file=sys.stderr,
        )
        return 1

    problems = missing_bindings(services)
    if problems:
        print(
            "check-entailment-surface: the conclusion-directed entailment surface is not "
            "reachable from every host:\n" + "\n".join(problems),
            file=sys.stderr,
        )
        return 1

    print(
        f"OK: all {len(services)} conclusion-directed entailment service(s) "
        f"({', '.join(sorted(services))}) reach Rust, Python, WASM and the C ABI."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
