#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Hygiene gate: every conclusion-directed entailment service `purrdf-entail`
publishes must be reachable from ALL FIVE host shapes — Rust, Python, WASM, the
C ABI and the `purrdf` COMMAND LINE.

A capability reachable from one caller shape and dark from another is a defect this
repository has already paid for: nine Description-Logic reasoning services were
compiled into the wasm artifact, budgeted for, and never re-exported from the npm
package root, so they shipped as bytes no consumer could call. `check-wasm-js-exports.py`
closes that hole for ONE host. This closes it across all five, for one surface.

The command line is the fifth row because it was the host this gate's own first draft
left out. `purrdf` is a shipped, first-class product surface, and while it grew
`purrdf reason` — which MATERIALIZES a closure — it had no way to ask whether a premise
entails a conclusion under any regime, by any mechanism. Four rows defined "every host"
in a way that excluded the binary by construction, so nothing would ever have noticed.

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
* the CLI: the FLAG that selects the service, declared on the `Entails` variant of the
  clap command tree in `crates/cli/src/cli.rs`, AND the boundary function inside the
  `use purrdf_validate::regime::{ … };` block of `crates/cli/src/entails.rs`, AND the
  subcommand dispatched from `crates/cli/src/main.rs`. The `use` block is the load-bearing
  needle rather than a call-site regex: this workspace builds with `-D warnings`, so an
  imported name nothing calls does not compile, and a name in that block is therefore proof
  of a call rather than of an import.

# A NAME is not a CAPABILITY: the PARAMETER LIST is checked too

A binding can carry the right name and still be crippled. The `owl:imports` table is
the case that proved it: `purrdf_entail::entails()` has resolved a caller-supplied
import map from the start, the Rust boundary hard-coded an empty one, and every host
therefore refused any premise carrying an `owl:imports` — permanently, with no
parameter to fix it. A name-only gate passed the whole time.

So the Rust boundary's parameter list is read out of `crates/validate/src/regime.rs`
and is the SOURCE OF TRUTH, and every host's parameter list is reconstructed from
`_PARAM_SPELLINGS` and compared to what that host actually declares, in order. Adding
a parameter to the boundary and forgetting one host is therefore a failure that NAMES
the host, and so is a host that grew a parameter the boundary does not have.

`_PARAM_SPELLINGS` is keyed by the BOUNDARY's parameter name and must cover every one
of them — a boundary parameter with no row fails the gate, exactly as a service with
no row in `_HOST_NAMES` does — so the table cannot go quietly stale. One boundary
parameter may map to SEVERAL host parameters where a host has no type for it: the C
ABI has no pair, so an import table is `(import_iris, import_documents, import_count)`
there, and wasm-bindgen has no nested string array, so it is two parallel arrays on
that host too. Each host also declares whatever fixed plumbing it appends
(`out_answer` / `out_certificate` / `out_error` on the C ABI), and nothing else.

The CLI's analogue of a parameter list is its FLAG list, and it is checked in both
directions with one asymmetry the other hosts do not have: three services share ONE
subcommand there, so `purrdf entails` declares the UNION of the three parameter lists
rather than any one of them. So the forward check is per service and a SUBSET one —
every flag that service's boundary parameters spell must be declared — and the reverse
check runs once over the whole variant: every flag it declares must be some service's
parameter spelling, some service's selector, or one of the `_CLI_PLUMBING` names. A
boundary parameter with no CLI flag therefore fails naming the CLI, and a CLI flag
answering to nothing fails too.

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
#
# The `cli` spelling is not a function name but the FLAG that selects the service on
# `purrdf entails`: one subcommand answers all three, because they are one question
# asked three ways and three subcommands would have split the premise, the regime and
# the import table across them. `--conclusion` asks for a verdict, `--conclusion
# --verify` asks for a verdict whose warrant is then re-decided without a reasoner, and
# `--pattern` asks for the certain answers of a basic graph pattern.
_HOST_NAMES: dict[str, dict[str, str]] = {
    "certain_answers": {
        "boundary": "certain_answers_to_string",
        "python": "certain_answers",
        "wasm": "entailCertainAnswers",
        "capi": "purrdf_entail_certain_answers",
        "cli": "--pattern",
    },
    "entails": {
        "boundary": "graph_entails_to_string",
        "python": "graph_entails",
        "wasm": "entailGraphEntails",
        "capi": "purrdf_entail_graph_entails",
        "cli": "--conclusion",
    },
    "verify": {
        "boundary": "verify_entailment_to_string",
        "python": "verify_entailment",
        "wasm": "entailVerifyEntailment",
        "capi": "purrdf_entail_verify_entailment",
        "cli": "--verify",
    },
}

_PUB_FN_RE = re.compile(r"^pub fn (\w+)[(<]", re.MULTILINE)

# How each BOUNDARY parameter is spelled on each host. The keys must be exactly the
# boundary's own parameter names — a boundary parameter with no row fails the gate,
# and a row naming no boundary parameter fails it too — so this table cannot drift
# from `crates/validate/src/regime.rs` in either direction.
#
# A value is a TUPLE because one boundary parameter can be several host parameters
# where the host has no type for it. `imports` is an ordered list of
# `(ontology-iri, document)` pairs; C has no pair, so it is two parallel arrays plus a
# count there, and wasm-bindgen has no ABI for a nested string array (`Vec<Vec<String>>`
# does not implement `VectorFromWasmAbi`), so it is two parallel arrays on that host too.
#
# Four host keys, because a host's Rust spelling and its published spelling are
# different files that can disagree: `python` is the `#[pyo3(signature = …)]` list AND
# the `.pyi` stub, `wasm` is the `#[wasm_bindgen]` function, `dts` is `index.d.ts`
# (camelCase, hand-written), and `capi` is the Rust entry point AND the cbindgen header.
_PARAM_SPELLINGS: dict[str, dict[str, tuple[str, ...]]] = {
    "regime": {
        "python": ("regime",),
        "wasm": ("regime",),
        "dts": ("regime",),
        "capi": ("regime",),
        "cli": ("--regime",),
    },
    # The premise, under the two names the boundary gives it: `document` for the
    # pattern-shaped question, `premise` for the conclusion-shaped one.
    "document": {
        "python": ("data",),
        "wasm": ("document",),
        "dts": ("document",),
        "capi": ("document",),
        "cli": ("--premise",),
    },
    "premise": {
        "python": ("premise",),
        "wasm": ("premise",),
        "dts": ("premise",),
        "capi": ("premise",),
        "cli": ("--premise",),
    },
    "pattern": {
        "python": ("pattern",),
        "wasm": ("pattern",),
        "dts": ("pattern",),
        "capi": ("pattern",),
        "cli": ("--pattern",),
    },
    "conclusion": {
        "python": ("conclusion",),
        "wasm": ("conclusion",),
        "dts": ("conclusion",),
        "capi": ("conclusion",),
        "cli": ("--conclusion",),
    },
    "imports": {
        "python": ("imports",),
        "wasm": ("import_iris", "import_documents"),
        "dts": ("importIris", "importDocuments"),
        "capi": ("import_iris", "import_documents", "import_count"),
        "cli": ("--import",),
    },
}

# What each host appends AFTER the boundary's own parameters, and nothing else. The C
# ABI returns through out-params and reports through an error handle; every other host
# returns a value.
_HOST_PLUMBING: dict[str, tuple[str, ...]] = {
    "capi": ("out_answer", "out_certificate", "out_error"),
}

# The `purrdf entails` flags that answer to no boundary parameter and to no service, and
# are therefore the subcommand's own plumbing. `--report` is the certificate target every
# reasoning subcommand carries; `--from` and `--base` are the CLI's own format resolution,
# which runs in FRONT of a boundary that parses one media type; `OUT` is positional and so
# is not a flag at all. A flag outside this list and outside the two tables above is a
# capability with no boundary behind it, and fails the gate.
_CLI_PLUMBING: frozenset[str] = frozenset({"--report", "--from", "--base"})

# Where the CLI's three needles live.
_CLI_COMMAND_TREE = Path("crates/cli/src/cli.rs")
_CLI_MODULE = Path("crates/cli/src/entails.rs")
_CLI_DISPATCH = Path("crates/cli/src/main.rs")

# The clap variant that carries the conclusion-directed surface.
_CLI_VARIANT = "Entails"

# Matched pairs the parameter splitter must not split inside. Angle brackets are here
# for `&ImportList<'_>` and `Vec<(String, String)>`; the splitter only ever runs over
# the text BETWEEN a function's own parentheses, so a `->` return arrow is out of reach.
_BRACKETS = {"(": ")", "[": "]", "{": "}", "<": ">"}


def _params_between_parens(text: str, open_paren: int, what: str) -> list[str]:
    """The top-level, comma-separated chunks inside the parens starting at `open_paren`."""
    if text[open_paren] != "(":
        raise SystemExit(f"check-entailment-surface: {what} does not open with `(`")
    depth: list[str] = []
    chunks: list[str] = []
    current = ""
    for index in range(open_paren, len(text)):
        char = text[index]
        if char in _BRACKETS:
            depth.append(_BRACKETS[char])
            if len(depth) == 1:
                continue
        elif depth and char == depth[-1]:
            depth.pop()
            if not depth:
                if current.strip():
                    chunks.append(current.strip())
                return chunks
        if len(depth) == 1 and char == ",":
            if current.strip():
                chunks.append(current.strip())
            current = ""
            continue
        current += char
    raise SystemExit(
        f"check-entailment-surface: the parameter list of {what} is unterminated; the "
        "file's layout moved — update this gate rather than leaving it vacuous"
    )


def _named_params(text: str, opener: str, what: str, trailing: bool) -> list[str]:
    """The parameter NAMES of the declaration `opener` introduces.

    `trailing` picks the identifier off the END of each chunk (C, whose declarators put
    the name last: `const char *const *import_iris`) rather than off the front
    (Rust/Python/TypeScript, which are all `name: type`).
    """
    start = text.find(opener)
    if start < 0:
        raise SystemExit(
            f"check-entailment-surface: cannot find {opener!r} for {what}; the file's "
            "layout moved — update this gate rather than leaving it vacuous"
        )
    chunks = _params_between_parens(text, start + len(opener) - 1, what)
    names: list[str] = []
    for chunk in chunks:
        pattern = r"(\w+)\s*(?:\[\s*\])?\s*$" if trailing else r"^(?:mut\s+)?(\w+)"
        found = re.search(pattern, chunk) if trailing else re.match(pattern, chunk)
        if not found:
            raise SystemExit(
                f"check-entailment-surface: cannot read a parameter name out of "
                f"{chunk!r} in {what}"
            )
        names.append(found.group(1))
    return names


def _pyo3_signature(text: str, function: str) -> list[str]:
    """The `#[pyo3(signature = (…))]` list attached to `fn <function>(`.

    The signature attribute rather than the Rust parameter list, because the attribute
    is what Python actually sees: it excludes the `py: Python<'_>` token and it is the
    thing that would carry a default if one were ever (wrongly) added.
    """
    # Further attribute lines may sit between — `#[allow(clippy::needless_pass_by_value)]`
    # is where this repository puts the binding-ABI waiver — so the run of them is
    # skipped rather than assumed absent. What is NOT allowed between is another `fn`.
    found = re.search(
        rf"#\[pyo3\(signature = \((?P<params>[^()]*)\)\)\]\s*"
        rf"(?:#\[[^\n]*\][^\n]*\s*)*fn {function}\(",
        text,
    )
    if not found:
        raise SystemExit(
            f"check-entailment-surface: `fn {function}` carries no `#[pyo3(signature = …)]` "
            "above it; without one the Python call shape is whatever PyO3 infers, which "
            "this gate cannot check"
        )
    return [chunk.strip().split("=")[0].strip() for chunk in found.group("params").split(",") if chunk.strip()]


def _wasm_params(text: str, js_name: str) -> list[str]:
    """The parameter names of the `pub fn` under `#[wasm_bindgen(js_name = <js_name>)]`.

    The attribute is what names the function on the JS side, so the binding is found by
    it rather than by the Rust identifier, which is free to differ and does.
    """
    attribute = f"#[wasm_bindgen(js_name = {js_name})]"
    start = text.find(attribute)
    if start < 0:
        raise SystemExit(
            f"check-entailment-surface: cannot find {attribute!r}; the file's layout moved"
        )
    tail = text[start:]
    found = re.search(r"pub fn (\w+)\(", tail)
    if not found:
        raise SystemExit(
            f"check-entailment-surface: {attribute!r} decorates no `pub fn`; the file's "
            "layout moved — update this gate rather than leaving it vacuous"
        )
    return _named_params(
        tail, found.group(0), f"the wasm binding {js_name}", trailing=False
    )


def _braced_block(text: str, opener: str, what: str) -> str:
    """The brace-balanced body that `opener` (which must end in `{`) introduces."""
    start = text.find(opener)
    if start < 0:
        raise SystemExit(
            f"check-entailment-surface: cannot find {opener!r} for {what}; the file's "
            "layout moved — update this gate rather than leaving it vacuous"
        )
    depth = 0
    for index in range(start + len(opener) - 1, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return text[start + len(opener) : index]
    raise SystemExit(
        f"check-entailment-surface: the {what} block opened at {opener!r} is unterminated"
    )


def _cli_flags(command_tree: str) -> list[str]:
    """Every long flag the `Entails` clap variant declares, in declaration order.

    clap derives a long name from the field name unless the `#[arg]` attribute spells one,
    so both forms are read: `long = "import"` on a field named `imports` is `--import`, and
    a bare `long` on `regime` is `--regime`. A field with no `long` at all is positional
    (`OUT`) and is not a flag.
    """
    block = _braced_block(
        command_tree, f"    {_CLI_VARIANT} {{", f"the clap `{_CLI_VARIANT}` variant"
    )
    flags: list[str] = []
    attributes = ""
    for line in block.splitlines():
        stripped = line.strip()
        if stripped.startswith("#["):
            attributes += stripped
            continue
        if stripped.startswith("///") or not stripped:
            continue
        field = re.match(r"^(\w+):", stripped)
        if field is None:
            # A continuation line of a multi-line attribute, or of a field's type.
            if attributes:
                attributes += stripped
            continue
        named = re.search(r'\blong\s*=\s*"([\w-]+)"', attributes)
        if named:
            flags.append(f"--{named.group(1)}")
        elif re.search(r"\blong\b(?!\s*=)", attributes):
            flags.append("--" + field.group(1).replace("_", "-"))
        attributes = ""
    if not flags:
        raise SystemExit(
            f"check-entailment-surface: the clap `{_CLI_VARIANT}` variant declares no long "
            "flag; the file's layout moved — update this gate rather than leaving it vacuous"
        )
    return flags


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
    cli_tree = _read(_CLI_COMMAND_TREE)
    cli_module = _read(_CLI_MODULE)
    cli_dispatch = _read(_CLI_DISPATCH)
    cli_flags = _cli_flags(cli_tree)
    cli_imports = _block(
        cli_module, "use purrdf_validate::regime::{", "};", "cli boundary import"
    )

    problems: list[str] = []
    problems.extend(_cli_wiring_problems(cli_tree, cli_dispatch, cli_flags))
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
            (
                "cli flag",
                f"{_CLI_COMMAND_TREE}::{_CLI_VARIANT}",
                names["cli"] in cli_flags,
            ),
            (
                "cli boundary call",
                f"{_CLI_MODULE}::use purrdf_validate::regime",
                re.search(rf"\b{names['boundary']}\b", cli_imports) is not None,
            ),
        ]
        crippled = False
        for host, where, present in checks:
            if not present:
                problems.append(
                    f"  • {service}: NO {host} — expected {names} to be reachable from {where}"
                )
                crippled = True
        # A name that is not there has no parameter list to read, and a second message
        # about it would say nothing the first did not.
        if not crippled:
            problems.extend(
                _arity_problems(
                    service,
                    names,
                    boundary=boundary,
                    py_source=py_source,
                    py_stub=py_stub,
                    wasm_source=wasm_source,
                    index_dts=index_dts,
                    capi_source=capi_source,
                    capi_header=capi_header,
                    cli_flags=cli_flags,
                )
            )
    return problems


def _cli_wiring_problems(
    cli_tree: str, cli_dispatch: str, cli_flags: list[str]
) -> list[str]:
    """The CLI checks that are about the SUBCOMMAND rather than about one service.

    Two of them. A variant clap parses and `main` never routes is a subcommand that
    cannot run, so the dispatch arm and the module call are checked once. And every flag
    the variant declares must answer to something — a boundary parameter, a service, or
    the subcommand's own plumbing — which is the reverse of the per-service subset check
    and is what stops a flag from existing with no boundary behind it.
    """
    problems: list[str] = []
    for what, where, present in (
        (
            "dispatch arm",
            f"{_CLI_DISPATCH}::Command::{_CLI_VARIANT}",
            f"Command::{_CLI_VARIANT} {{" in cli_dispatch,
        ),
        (
            "dispatch call",
            f"{_CLI_DISPATCH}::entails::run",
            "entails::run(" in cli_dispatch,
        ),
        (
            "subcommand variant",
            f"{_CLI_COMMAND_TREE}::Command::{_CLI_VARIANT}",
            f"    {_CLI_VARIANT} {{" in cli_tree,
        ),
    ):
        if not present:
            problems.append(
                f"  • the conclusion-directed CLI surface has NO {what} — expected it at "
                f"{where}. A subcommand the binary does not route is a capability its "
                "users cannot reach."
            )
    answerable = set(_CLI_PLUMBING)
    answerable.update(names["cli"] for names in _HOST_NAMES.values())
    for spellings in _PARAM_SPELLINGS.values():
        answerable.update(spellings["cli"])
    stray = [flag for flag in cli_flags if flag not in answerable]
    if stray:
        problems.append(
            f"  • the `purrdf entails` subcommand declares {stray}, which is neither a "
            "boundary parameter's CLI spelling, nor a service's selector, nor declared "
            f"plumbing ({sorted(_CLI_PLUMBING)}). A flag with no boundary behind it is a "
            "capability this gate cannot check: add a row, or drop the flag."
        )
    return problems


def _arity_problems(
    service: str,
    names: dict[str, str],
    *,
    boundary: str,
    py_source: str,
    py_stub: str,
    wasm_source: str,
    index_dts: str,
    capi_source: str,
    capi_header: str,
    cli_flags: list[str],
) -> list[str]:
    """One message per host whose parameter list is not the boundary's.

    The boundary is the source of truth and every host is RECONSTRUCTED from it through
    `_PARAM_SPELLINGS`, so a parameter added to the boundary and forgotten on one host
    fails here naming that host — which is the whole reason this function exists. A
    name-only gate certified the `owl:imports` table as reachable from four hosts while
    it had a parameter on none of them.
    """
    boundary_params = _named_params(
        boundary,
        f"pub fn {names['boundary']}(",
        f"the boundary {names['boundary']}",
        trailing=False,
    )
    unmapped = [param for param in boundary_params if param not in _PARAM_SPELLINGS]
    if unmapped:
        return [
            f"  • {service}: the boundary takes {unmapped} and _PARAM_SPELLINGS says how "
            "NO host spells them. A parameter the Rust boundary has and the hosts do not "
            "is the defect this gate exists for: add it to every binding and a row here."
        ]

    def expected(host: str) -> list[str]:
        spelled: list[str] = []
        for param in boundary_params:
            spelled.extend(_PARAM_SPELLINGS[param][host])
        return spelled + list(_HOST_PLUMBING.get(host, ()))

    actual: list[tuple[str, str, str, list[str]]] = [
        (
            "python function",
            "bindings/python/src/py_entail.rs",
            "python",
            _pyo3_signature(py_source, names["python"]),
        ),
        (
            "python stub",
            "bindings/python/python/src/purrdf/__init__.pyi",
            "python",
            _named_params(
                py_stub,
                f"def {names['python']}(",
                f"the stub for {names['python']}",
                trailing=False,
            ),
        ),
        (
            "wasm binding",
            "crates/rdf-wasm/src/entail.rs",
            "wasm",
            _wasm_params(wasm_source, names["wasm"]),
        ),
        (
            "npm type declaration",
            "crates/rdf-wasm/js/index.d.ts",
            "dts",
            _named_params(
                index_dts,
                f"export function {names['wasm']}(",
                f"the .d.ts declaration of {names['wasm']}",
                trailing=False,
            ),
        ),
        (
            "c abi entry point",
            "crates/rdf-capi/src/entail.rs",
            "capi",
            _named_params(
                capi_source,
                f'pub unsafe extern "C" fn {names["capi"]}(',
                f"the C entry point {names['capi']}",
                trailing=False,
            ),
        ),
        (
            "c abi header declaration",
            "crates/rdf-capi/include/purrdf.h",
            "capi",
            _named_params(
                capi_header,
                f"int32_t {names['capi']}(",
                f"the header declaration of {names['capi']}",
                trailing=True,
            ),
        ),
    ]

    problems: list[str] = []
    for host, where, key, declared in actual:
        wanted = expected(key)
        if declared != wanted:
            problems.append(
                f"  • {service}: the {host} does NOT take the boundary's parameters — "
                f"{where} declares {declared}, and {names['boundary']}"
                f"{boundary_params} means it must declare {wanted}"
            )

    # The CLI is a SUBSET check rather than an equality one, and only here: `purrdf
    # entails` answers all three services from one subcommand, so it declares the union
    # of the three parameter lists. The reverse direction — a flag answering to nothing —
    # is checked once, in `_cli_wiring_problems`.
    wanted_flags = [flag for param in boundary_params for flag in _PARAM_SPELLINGS[param]["cli"]]
    absent = [flag for flag in wanted_flags if flag not in cli_flags]
    if absent:
        problems.append(
            f"  • {service}: the cli subcommand does NOT take the boundary's parameters — "
            f"{_CLI_COMMAND_TREE}::{_CLI_VARIANT} declares {cli_flags}, and "
            f"{names['boundary']}{boundary_params} means it must also declare {absent}"
        )
    return problems


def main() -> int:
    services = derived_services()
    unmapped = services - _HOST_NAMES.keys()
    if unmapped:
        print(
            "check-entailment-surface: purrdf-entail publishes "
            f"{sorted(unmapped)} with no host spellings. A capability reachable from Rust "
            "and dark from Python, WASM, C and the command line is the defect this gate "
            "exists for: add the five bindings and a row in _HOST_NAMES.",
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
        f"({', '.join(sorted(services))}) reach Rust, Python, WASM, the C ABI and the "
        "`purrdf` command line, each with the boundary's whole parameter list."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
