#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Refuse a `thread_local!` the interleaving ledger does not account for, and vice versa.

The asynchronous wasm lane suspends a job mid-evaluation and runs other work on the same
thread while it waits. Per-thread state a job left set is read by the next caller as its
own, and a ``RefCell`` held across a suspension panics on the next borrow. Every
``thread_local!`` in the workspace therefore carries a stated reason it is safe under
that interleaving, in ``crates/rdf-wasm/src/interleaving.rs`` (``LEDGER``): the job
swaps it, it is borrowed only inside one call, it is never written while an evaluation
runs, or it is not compiled into the package.

A prose table had held that reasoning, and prose drifts: a thread-local added to the
evaluator is not in the table until someone remembers, and the table keeps naming one
that was deleted. So the ledger is Rust and this gate compares it with the source in both
directions. A static declared in a ``thread_local!`` block under ``crates/`` and absent
from the ledger fails; a ledger entry naming a static no file declares fails. The identity
is ``(file, name)``: the declaring file relative to the repository root, and the static's
identifier.

What is scanned is exactly what ``grep thread_local!`` over ``crates/`` finds, with two
subtractions stated rather than implied: whole-line comments are dropped (a doc comment
that names the macro declares nothing), and string literals are blanked (a message that
quotes the macro declares nothing). Both are proven by ``--self-test``, which also proves
each refusing direction on a fixture tree and a neighbour: one fixture with every
``thread_local!`` listed passes, and the same fixture with one unlisted ``thread_local!``
fails naming it.
"""

from __future__ import annotations

import argparse
import re
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CRATES = REPO_ROOT / "crates"
LEDGER_FILE = REPO_ROOT / "crates" / "rdf-wasm" / "src" / "interleaving.rs"

# `thread_local! {` and `std::thread_local! {`, the opening brace included so the block
# can be walked to its close.
THREAD_LOCAL_OPEN = re.compile(r"\bthread_local!\s*\{")
# `static NAME:` inside a block, `pub(crate)`/`pub` visibility allowed.
STATIC_DECL = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?static\s+([A-Za-z_][A-Za-z0-9_]*)\s*:", re.MULTILINE)
# One ledger entry: the `file:` and `name:` fields, in that order, whatever rustfmt did
# with the line breaks between them.
LEDGER_ENTRY = re.compile(r'ThreadLocal\s*\{\s*file:\s*"([^"]+)"\s*,\s*name:\s*"([^"]+)"\s*,')


def _without_comments_and_strings(text: str) -> str:
    """The source with whole-line comments removed and string literals blanked.

    Line positions are kept: a comment line becomes empty and a string keeps its
    length, so a finding's line number is the source's.
    """
    kept: list[str] = []
    for line in text.splitlines():
        if line.lstrip().startswith("//"):
            kept.append("")
            continue
        kept.append(re.sub(r'"(?:[^"\\\n]|\\.)*"', lambda m: '"' + "_" * (len(m.group(0)) - 2) + '"', line))
    return "\n".join(kept)


def _block_end(text: str, open_brace: int) -> int:
    """The index just past the `}` closing the block opened at `open_brace`."""
    depth = 0
    for index in range(open_brace, len(text)):
        if text[index] == "{":
            depth += 1
        elif text[index] == "}":
            depth -= 1
            if depth == 0:
                return index + 1
    return len(text)


def declared_in(source: str) -> list[str]:
    """Every static a `thread_local!` block in `source` declares, in source order."""
    prepared = _without_comments_and_strings(source)
    found: list[str] = []
    for opening in THREAD_LOCAL_OPEN.finditer(prepared):
        brace = prepared.index("{", opening.start())
        block = prepared[brace : _block_end(prepared, brace)]
        found.extend(STATIC_DECL.findall(block))
    return found


def rust_sources(root: Path) -> list[Path]:
    """Every `.rs` file under `root`, build output excluded."""
    return sorted(
        path
        for path in root.rglob("*.rs")
        if "target" not in path.relative_to(root).parts
    )


def declared_thread_locals(crates: Path, repo_root: Path) -> set[tuple[str, str]]:
    """Every `(file, name)` a `thread_local!` block under `crates` declares."""
    found: set[tuple[str, str]] = set()
    for path in rust_sources(crates):
        relative = path.relative_to(repo_root).as_posix()
        for name in declared_in(path.read_text(encoding="utf-8")):
            found.add((relative, name))
    return found


def ledger_entries(ledger_text: str) -> list[tuple[str, str]]:
    """Every `(file, name)` the ledger lists, in ledger order (duplicates kept)."""
    return [(file, name) for file, name in LEDGER_ENTRY.findall(ledger_text)]


def divergence(declared: set[tuple[str, str]], listed: list[tuple[str, str]], ledger_name: str) -> list[str]:
    """Every disagreement between the source and the ledger, both directions."""
    problems: list[str] = []
    seen: set[tuple[str, str]] = set()
    for entry in listed:
        if entry in seen:
            problems.append(f"{ledger_name} lists {entry[0]}::{entry[1]} twice")
        seen.add(entry)
    for file, name in sorted(declared - seen):
        problems.append(
            f"{file} declares `thread_local!` static `{name}`, and {ledger_name} does not list "
            f"it: add an entry stating why it is safe under the asynchronous lane's interleaving"
        )
    for file, name in sorted(seen - declared):
        problems.append(
            f"{ledger_name} lists {file}::{name}, and no `thread_local!` block in that file "
            f"declares it: the entry is stale, so delete it"
        )
    return problems


def check(crates: Path, repo_root: Path, ledger_file: Path) -> list[str]:
    """The divergence of the real (or a fixture) tree."""
    if not ledger_file.is_file():
        return [f"the ledger {ledger_file.relative_to(repo_root)} is missing"]
    declared = declared_thread_locals(crates, repo_root)
    listed = ledger_entries(ledger_file.read_text(encoding="utf-8"))
    if not listed:
        return [f"{ledger_file.relative_to(repo_root)} lists no `ThreadLocal` entry at all"]
    return divergence(declared, listed, ledger_file.relative_to(repo_root).as_posix())


# ── Self-test fixtures ───────────────────────────────────────────────────────────────

FIXTURE_CRATE = """\
// SPDX-License-Identifier: MIT
//! The doc mentions `thread_local!` and declares nothing by doing so.

use std::cell::Cell;

thread_local! {
    /// A counter.
    static COUNTER: Cell<u64> = const { Cell::new(0) };
    pub(crate) static FLAG: Cell<bool> = const { Cell::new(false) };
}

fn message() -> &'static str {
    "a string that says thread_local! { static NOT_A_STATIC: () = (); } and declares nothing"
}

#[cfg(test)]
std::thread_local! {
    static PROBE: Cell<Option<usize>> = const { Cell::new(None) };
}
"""

FIXTURE_LEDGER = """\
pub const LEDGER: &[ThreadLocal] = &[
    ThreadLocal {
        file: "crates/fixture/src/lib.rs",
        name: "COUNTER",
        safety: Safety::PerCall,
        reason: "read inside one call",
    },
    ThreadLocal { file: "crates/fixture/src/lib.rs", name: "FLAG", safety: Safety::PerCall, reason: "one call" },
    ThreadLocal {
        file: "crates/fixture/src/lib.rs",
        name: "PROBE",
        safety: Safety::NotCompiledIn,
        reason: "test only",
    },
];
"""

NEIGHBOUR_UNLISTED = """
std::thread_local! {
    static UNLISTED: Cell<u8> = const { Cell::new(0) };
}
"""


def self_test() -> int:
    """Both refusing directions, the prose exclusions, and the valid neighbours."""
    ok = True

    # The tree as committed is the first valid neighbour: a gate that refused
    # unconditionally would pass every refusal case below.
    standing = check(CRATES, REPO_ROOT, LEDGER_FILE)
    if standing:
        print("SELF-TEST FAIL: the tree as committed is reported as divergent:")
        for problem in standing:
            print(f"  {problem}")
        ok = False
    else:
        print("OK: self-test — the tree as committed has no divergence (the valid neighbour)")

    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)
        crates = root / "crates"
        source = crates / "fixture" / "src" / "lib.rs"
        source.parent.mkdir(parents=True)
        ledger = crates / "ledger" / "src" / "interleaving.rs"
        ledger.parent.mkdir(parents=True)
        # Build output under `crates/` is never scanned.
        stray = crates / "fixture" / "target" / "debug" / "gen.rs"
        stray.parent.mkdir(parents=True)
        stray.write_text("thread_local! { static FROM_BUILD_OUTPUT: () = (); }\n", encoding="utf-8")

        # The fixture neighbour: every declared static listed, prose and strings ignored.
        source.write_text(FIXTURE_CRATE, encoding="utf-8")
        ledger.write_text(FIXTURE_LEDGER, encoding="utf-8")
        declared = declared_thread_locals(crates, root)
        expected = {
            ("crates/fixture/src/lib.rs", "COUNTER"),
            ("crates/fixture/src/lib.rs", "FLAG"),
            ("crates/fixture/src/lib.rs", "PROBE"),
        }
        if declared != expected:
            print(
                "SELF-TEST FAIL: the fixture declares three statics (the doc comment, the "
                f"string literal and the build output declare none), and the scan found {sorted(declared)}"
            )
            ok = False
        else:
            print("OK: self-test — a `thread_local!` in a comment, a string or build output declares nothing")
        found = check(crates, root, ledger)
        if found:
            print(f"SELF-TEST FAIL: the fully listed fixture is reported as divergent: {found}")
            ok = False
        else:
            print("OK: self-test — a fixture whose every `thread_local!` is listed passes (the neighbour)")

        # Direction 1: an unlisted `thread_local!` fails, naming the file and the static.
        source.write_text(FIXTURE_CRATE + NEIGHBOUR_UNLISTED, encoding="utf-8")
        found = check(crates, root, ledger)
        if any("crates/fixture/src/lib.rs" in p and "`UNLISTED`" in p and "does not list" in p for p in found) and len(found) == 1:
            print("OK: self-test — the same fixture with one unlisted `thread_local!` is refused, naming it")
        else:
            print(f"SELF-TEST FAIL: an unlisted `thread_local!` was not refused as the one problem (reported: {found})")
            ok = False

        # Direction 2: a listed static that no longer exists fails, naming the entry.
        source.write_text(FIXTURE_CRATE.replace("static COUNTER", "static RENAMED"), encoding="utf-8")
        found = check(crates, root, ledger)
        stale = [p for p in found if "COUNTER" in p and "stale" in p]
        unlisted = [p for p in found if "RENAMED" in p and "does not list" in p]
        if len(stale) == 1 and len(unlisted) == 1 and len(found) == 2:
            print("OK: self-test — a ledger entry whose static is gone is refused as stale")
        else:
            print(f"SELF-TEST FAIL: a stale ledger entry was not refused (reported: {found})")
            ok = False

        # A duplicate entry is refused.
        source.write_text(FIXTURE_CRATE, encoding="utf-8")
        ledger.write_text(
            FIXTURE_LEDGER.replace(
                '    ThreadLocal { file: "crates/fixture/src/lib.rs", name: "FLAG",',
                '    ThreadLocal { file: "crates/fixture/src/lib.rs", name: "FLAG", safety: Safety::PerCall, reason: "twice" },\n'
                '    ThreadLocal { file: "crates/fixture/src/lib.rs", name: "FLAG",',
                1,
            ),
            encoding="utf-8",
        )
        found = check(crates, root, ledger)
        if len(found) == 1 and "twice" in found[0] and "FLAG" in found[0]:
            print("OK: self-test — a `thread_local!` listed twice is refused")
        else:
            print(f"SELF-TEST FAIL: a duplicate ledger entry was not refused (reported: {found})")
            ok = False

        # An empty ledger is refused rather than read as "nothing to compare".
        ledger.write_text("pub const LEDGER: &[ThreadLocal] = &[];\n", encoding="utf-8")
        found = check(crates, root, ledger)
        if len(found) == 1 and "no `ThreadLocal` entry" in found[0]:
            print("OK: self-test — an empty ledger is refused")
        else:
            print(f"SELF-TEST FAIL: an empty ledger was not refused (reported: {found})")
            ok = False

    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--self-test", action="store_true", help="prove the gate fails in both directions")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    problems = check(CRATES, REPO_ROOT, LEDGER_FILE)
    if problems:
        print("FAIL: the thread-local ledger and the source disagree:", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1
    listed = ledger_entries(LEDGER_FILE.read_text(encoding="utf-8"))
    print(f"OK: every `thread_local!` under crates/ is in the interleaving ledger ({len(listed)} statics)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
