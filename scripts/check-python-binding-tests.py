#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Fail if a Rust test module appears in the Python extension crate.

``bindings/python`` builds one library, ``purrdf_native``, and its manifest sets
``test = false`` and ``bench = false`` for a reason that cannot be worked
around: the library is a PyO3 ``extension-module``. It deliberately leaves the
CPython API unresolved so the interpreter supplies those symbols at ``dlopen``
time, which is exactly what makes an ``abi3`` manylinux wheel portable. A test
or bench target is an ordinary executable that links the library with no
interpreter behind it, so it fails at link with dozens of undefined
``pyo3-ffi`` refcount symbols. Cargo therefore builds neither target.

The consequence is easy to miss and was missed: a ``#[cfg(test)]`` module in
this crate is compiled by nothing and run by nothing. It is not a weak test, it
is not a slow test, it is *absent* — and it looks exactly like coverage in a
diff, in a review, and in a file listing. One such module had accumulated a
shadowing error that no gate could ever have reported, because no gate ever
built it. Together they came to roughly fifteen hundred lines of source that
asserted nothing at all.

The textbook remedy is a Cargo feature gating ``extension-module`` so a harness
can link ``libpython``. This workspace forbids Cargo features
(``scripts/check-no-features.py``), and dropping ``extension-module`` outright
would link ``libpython`` into the ``cdylib`` and break the wheel portability
this project ships to PyPI. So ``test = false`` stays, and the gate that really
executes this crate is pytest, against the built extension.

This script is the guard that keeps the dead modules from coming back. It scans
every ``.rs`` file under ``bindings/python/src`` for two shapes:

* a ``test`` predicate inside any ``cfg``/``cfg_attr`` invocation — which covers
  ``#[cfg(test)]``, ``#[cfg(all(test, ...))]`` and ``#[cfg_attr(test, ...)]``
  alike, rather than one spelling of it; and
* the ``#[test]`` attribute itself, for a test function written without a
  gating module around it.

Whole-line comments are blanked before the scan, so prose may name the shapes —
this crate's own source should be free to explain why they are forbidden, and a
doc comment is not a compilation unit. Byte offsets are preserved by blanking
rather than deleting, so a finding still points at the right line.

``--self-test`` proves the gate fires, in both directions: an offending file is
reported and a clean file that merely *describes* the offending shapes is not.
A gate that only ever passes is indistinguishable from a gate that cannot fail.
"""

from __future__ import annotations

import os
import re
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def _scratch_root() -> Path:
    """The build tree's scratch root, ``target/gate-scratch/`` (or under
    ``$CARGO_TARGET_DIR``), the convention ``scripts/build-scratch.sh`` sets out.
    Scratch goes there rather than into the system temporary directory, which is
    not guaranteed to keep a directory for as long as a gate runs."""
    target = os.environ.get("CARGO_TARGET_DIR")
    root = (Path(target) if target else REPO_ROOT / "target") / "gate-scratch"
    root.mkdir(parents=True, exist_ok=True)
    return root

# The crate this gate governs, and the place its coverage belongs instead.
GOVERNED = "bindings/python/src"
HOME = "bindings/python/tests"

# A `cfg(...)`/`cfg_attr(...)` invocation, as an attribute or as the `cfg!`
# macro. The body is read by balancing parentheses rather than by regex, so a
# nested `all(...)`/`any(...)`/`not(...)` is read correctly.
CFG_INVOCATION = re.compile(r"(?:#\s*!?\s*\[\s*cfg(?:_attr)?|\bcfg\s*!)\s*\(")

# A bare `test` predicate inside that body: a whole token, delimited by the
# punctuation a predicate list uses. `feature = "test"` and an identifier such
# as `test_helpers` are both left alone — the first because a named value is not
# the `test` predicate, the second because the token boundaries do not match.
TEST_PREDICATE = re.compile(r"(?:^|[(,\s])test(?:$|[),\s])")

# The test attribute itself, for a `#[test] fn` with no gating module above it.
TEST_ATTRIBUTE = re.compile(r"#\s*\[\s*test\s*\]")

CFG_RULE = "cfg-test"
CFG_MEANING = "a `test` predicate in a cfg invocation"
ATTR_RULE = "test-attr"
ATTR_MEANING = "a `#[test]` attribute"

# The two shapes, written out, for the self-test's offending fixture.
SELF_TEST_OFFENDER = """
#[cfg(test)]
mod tests {
    #[test]
    fn something() {}
}

#[cfg(all(test, unix))]
const HELPER: &str = "x";

#[cfg_attr(test, derive(Debug))]
struct S;
"""

# A file that names both shapes in prose and in a string, and uses a cfg
# invocation that is not the test predicate. None of it is a finding.
SELF_TEST_CLEAN = """
//! This crate sets `test = false`, so a `#[cfg(test)]` module here would be
//! compiled by nothing. A `#[test]` function likewise.

// #[cfg(test)]
// mod tests {
//     #[test]
//     fn something() {}
// }

#[cfg(target_arch = "wasm32")]
const WHERE: &str = "wasm";

#[cfg(not(target_family = "unix"))]
const OTHER: &str = "elsewhere";

const NAME: &str = "test";
const HELPERS: &str = "test_helpers";

fn detest() {}
"""


def strip_comment_lines(source: str) -> str:
    """Blank out whole-line comments, preserving every byte offset.

    Prose is not a compilation unit: the source of the very crate this gate
    governs should be able to explain why a test module cannot live there, and
    naming the shape is how one explains it. Blanking rather than deleting keeps
    line numbers and byte offsets exact, so findings still point at the right
    place.
    """
    out = []
    for line in source.split("\n"):
        head = line.lstrip()
        if head.startswith(("//", "/*", "*/", "*")):
            out.append(" " * len(line))
        else:
            out.append(line)
    return "\n".join(out)


def cfg_test_offsets(source: str):
    """Yield offsets of `test` predicates inside cfg invocations."""
    for invocation in CFG_INVOCATION.finditer(source):
        opening = source.find("(", invocation.start(), invocation.end())
        depth = 0
        for offset in range(opening, len(source)):
            char = source[offset]
            if char == "(":
                depth += 1
            elif char == ")":
                depth -= 1
                if depth == 0:
                    body = source[opening + 1 : offset]
                    if predicate := TEST_PREDICATE.search(body):
                        yield opening + 1 + predicate.start()
                    break


def rust_sources(root: Path) -> list[Path]:
    """Every ``.rs`` file under *root*, in deterministic order."""
    return sorted(root.rglob("*.rs"))


def scan(root: Path, base: Path) -> list[str]:
    """Findings under *root*, reported relative to *base*."""
    findings: list[str] = []
    for path in rust_sources(root):
        source = strip_comment_lines(path.read_text(encoding="utf-8"))
        rel = path.relative_to(base).as_posix()
        hits: list[tuple[int, str, str]] = []
        for offset in cfg_test_offsets(source):
            hits.append((source.count("\n", 0, offset) + 1, CFG_RULE, CFG_MEANING))
        for match in TEST_ATTRIBUTE.finditer(source):
            hits.append(
                (source.count("\n", 0, match.start()) + 1, ATTR_RULE, ATTR_MEANING)
            )
        for line_no, rule, meaning in sorted(hits):
            findings.append(f"{rel}:{line_no}: [{rule}] {meaning}")
    return findings


def self_test() -> int:
    """Prove the gate fires on the offending shapes and not on prose about them."""
    with tempfile.TemporaryDirectory(dir=_scratch_root()) as raw:
        root = Path(raw)
        (root / "offender.rs").write_text(SELF_TEST_OFFENDER, encoding="utf-8")
        offending = scan(root, root)
        expected_rules = {finding.split("[", 1)[1].split("]", 1)[0] for finding in offending}
        if len(offending) != 4 or expected_rules != {CFG_RULE, ATTR_RULE}:
            print(
                "SELF-TEST FAILED: the offending fixture carries three cfg `test` "
                "predicates and one `#[test]` attribute, and the scan reported "
                f"{len(offending)} finding(s) across rules {sorted(expected_rules)}:",
                file=sys.stderr,
            )
            for finding in offending:
                print(f"  {finding}", file=sys.stderr)
            return 1

        (root / "offender.rs").unlink()
        (root / "clean.rs").write_text(SELF_TEST_CLEAN, encoding="utf-8")
        clean = scan(root, root)
        if clean:
            print(
                "SELF-TEST FAILED: a file that only DESCRIBES the forbidden "
                "shapes, and uses cfg predicates that are not `test`, was "
                "reported. That is the over-refusal half of this gate, and it "
                "would teach authors to stop explaining the rule:",
                file=sys.stderr,
            )
            for finding in clean:
                print(f"  {finding}", file=sys.stderr)
            return 1

    print("OK: the Rust-test-module gate fires on both shapes and on nothing else")
    return 0


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return self_test()

    governed = REPO_ROOT / GOVERNED
    if not governed.is_dir():
        print(
            f"{GOVERNED} is missing: this gate governs the PyO3 extension crate "
            "and cannot confirm anything about a tree that is not there.",
            file=sys.stderr,
        )
        return 1

    findings = scan(governed, REPO_ROOT)
    if findings:
        print(
            "The Python extension crate must carry no Rust test module. Its "
            "manifest sets `test = false` because the library is a PyO3 "
            "`extension-module`: it leaves the CPython API unresolved for the "
            "interpreter to supply at dlopen time, so a test executable — which "
            "has no interpreter — fails at link. A `#[cfg(test)]` module here is "
            "therefore compiled by nothing and run by nothing, while looking "
            "exactly like coverage. The following would never run:",
            file=sys.stderr,
        )
        for finding in findings:
            print(f"  {finding}", file=sys.stderr)
        print(
            f"\nMove what each one asserts into {HOME}, which is the gate that "
            "really executes this crate (`make pytest`, against the built "
            "extension). Where the property has no Python-visible behaviour at "
            "all, the logic itself belongs behind the binding, in the crate that "
            "owns it, where an ordinary `cargo test` reaches it. Restoring the "
            "module is not an option: Cargo features are forbidden here, and "
            "dropping `extension-module` would link libpython into the cdylib "
            "and break the abi3 wheel this project publishes.",
            file=sys.stderr,
        )
        return 1

    print(
        f"OK: no Rust test module under {GOVERNED} "
        f"({len(rust_sources(governed))} sources scanned)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
