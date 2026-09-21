#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Refuse a streamed read whose chunk size is written out instead of named.

Six copies of one number lived under two names across five files, and two had
already drifted -- one to a quarter of the shared size, one to a sixteenth. That
is the reason this is a gate rather than only a fix: every chunk size produces a
correct digest, so no run fails and no test reddens, and the only symptom is that
one site issues sixty-four times as many reads as its siblings. The drift had
nothing to report it.

Folding them onto ``scripts/lane_chunk.py`` closes today's copies and does nothing
about the fifth, so this refuses the shape: a literal byte count passed to a
``read()`` in any ``scripts/`` program. What must appear instead is the name
``STREAM_CHUNK_BYTES`` (Python) or ``LANE_STREAM_CHUNK_BYTES`` (the value shell
lanes pass into their embedded blocks).

Deliberately NOT refused, because a gate that over-refuses gets disabled and then
guards nothing:

* ``read()`` with no argument, or with a variable — those already name their size;
* a literal outside a ``read`` call, such as a buffer threshold or a row count —
  ``crates/bench/src/main.rs``'s flush interval is a write-side figure with its
  own name and its own reason, and is not this number;
* small literals inside a ``read`` (``read(1)``, ``read(2)``), which are reading a
  fixed-width field rather than streaming a file. The cutoff is stated rather than
  implied: a chunk size is at least a kibibyte, and nothing reads a 1024-byte
  fixed-width field here.
"""

import argparse
import ast
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = REPO_ROOT / "scripts"

# `read(1 << 22)`, `read(4194304)`, `.read(1<<20)` — a literal count, decimal or
# shifted, handed to a read. `\b` on `read` keeps `handle.read` and bare `read`
# while excluding `spread(`.
# A literal byte count handed to a read, in any spelling this repository could use: a
# shift (`1 << 22`), a plain or underscored decimal (`4194304`, `4_194_304`), hex
# (`0x400000`), or a product (`4 * 1024 * 1024`). The first version matched only a shift
# or four-plus bare digits, so three legal spellings of the same number were accepted --
# and `read(1000)` was refused despite the docstring stating a kibibyte cutoff, because
# the decimal arm counted DIGITS where the rule is about VALUE.
# A `read(...)` call whose argument is a literal expression. The argument text is then
# EVALUATED rather than pattern-matched, because the first version alternated over
# spellings and three legal ones for the very number it exists to catch -- `0o20000000`,
# `0b1000...`, `4 * 1024**2` -- walked straight through. A spelling list is a denylist, and
# a denylist over syntax is the wrong shape.
READ_CALL = re.compile(r"\bread\(\s*([^)\n]+?)\s*\)")

# The one definition, and the shell variable carrying it into an embedded block.
DEFINING_FILE = "lane_chunk.py"

# This gate's own fixtures ARE the shape it refuses — that is what makes the
# self-test non-vacuous — so it does not scan itself. Stated rather than left as a
# quiet exclusion: the alternative is a gate that cannot describe what it refuses.
GATE_FILE = Path(__file__).name


def _literal_value(literal: str) -> tuple[int | None, bool]:
    """`(value, unparseable)` for a read's argument.

    `(None, False)` means "not a literal at all" -- a variable or a name, which already
    names its size and is exactly what this gate wants. `(None, True)` means "a literal
    this gate cannot evaluate", which is reported rather than skipped.

    Evaluated with `ast` rather than matched: the previous alternation missed `0o20000000`,
    `0b1000...` and `4 * 1024**2`, three legal spellings of the number it exists to catch.
    """
    try:
        tree = ast.parse(literal, mode="eval")
    except SyntaxError:
        # `0123` is a syntax error in Python 3 and a plausible typo for a chunk size, so it
        # is a literal that cannot be evaluated rather than a name.
        return None, bool(re.fullmatch(r"[0-9][0-9_]*", literal.strip()))

    def fold(node: ast.expr) -> int | None:
        if isinstance(node, ast.Constant) and isinstance(node.value, int):
            return node.value
        if isinstance(node, ast.BinOp):
            left, right = fold(node.left), fold(node.right)
            if left is None or right is None:
                return None
            if isinstance(node.op, ast.LShift):
                return left << right
            if isinstance(node.op, ast.Mult):
                return left * right
            if isinstance(node.op, ast.Pow):
                return left**right
            if isinstance(node.op, ast.Add):
                return left + right
        return None

    return fold(tree.body), False


def offences(path: Path, text: str) -> list[str]:
    """Every literal-sized streamed read in one file, as a diagnosis per hit."""
    found: list[str] = []
    for number, line in enumerate(text.splitlines(), start=1):
        for match in READ_CALL.finditer(line):
            literal = match.group(1)
            # JUDGED ON VALUE, because the stated rule is a value: a chunk size is at least
            # a kibibyte and nothing here reads a 1024-byte fixed-width field.
            value, unparseable = _literal_value(literal)
            if unparseable:
                # A LITERAL THAT CANNOT BE EVALUATED IS REPORTED, not skipped. `int("0123",
                # 0)` raises on a leading zero, and swallowing that to `None` made
                # `read(0123)` a silent pass -- a gate declining to judge the one shape it
                # was looking at.
                found.append(
                    f"{path.name}:{number}: `read({literal})` is a literal this gate cannot "
                    f"evaluate, so it cannot be judged. Name it instead: "
                    f"`STREAM_CHUNK_BYTES` from scripts/{DEFINING_FILE}."
                )
                continue
            if value is None or value < 1024:
                continue
            found.append(
                f"{path.name}:{number}: `read({literal})` writes the chunk size out. "
                f"Name it: `STREAM_CHUNK_BYTES` from scripts/{DEFINING_FILE}, or "
                f"`${{LANE_STREAM_CHUNK_BYTES}}` passed in by lane-common.sh."
            )
    return found


def scan() -> list[str]:
    """Every offence across the scripts directory, skipping the defining file."""
    found: list[str] = []
    # RECURSIVE. `iterdir()` left any future `scripts/<subdir>/*.py` unscanned, which is
    # the "a surface the gate never inspects" shape this file is one instance of.
    for path in sorted(SCRIPTS.rglob("*")):
        if not path.is_file() or path.name in {DEFINING_FILE, GATE_FILE}:
            continue
        if path.suffix not in {".py", ".sh"}:
            continue
        found.extend(offences(path, path.read_text(encoding="utf-8")))
    return found


def self_test() -> int:
    """The shapes that must be refused, and — the half that matters — those that must not."""
    ok = True
    here = Path("probe.py")

    refused = [
        "        for chunk in iter(lambda: handle.read(1 << 22), b''):",
        "    chunk = sys.stdin.buffer.read(1 << 20)",
        "    data = handle.read(4194304)",
        "    data = handle.read(65536)",
        # Three legal spellings of the same number that the digit-counting rule accepted.
        "    data = handle.read(4_194_304)",
        "    data = handle.read(0x400000)",
        "    data = handle.read(4 * 1024 * 1024)",
        # Three more legal spellings of 4194304 that the alternation accepted outright.
        "    data = handle.read(0o20000000)",
        "    data = handle.read(0b10000000000000000000000)",
        "    data = handle.read(4 * 1024**2)",
        # A literal that cannot be evaluated is reported rather than skipped.
        "    data = handle.read(0123)",
    ]
    for line in refused:
        if not offences(here, line):
            print(f"SELF-TEST FAIL: not refused: {line.strip()}")
            ok = False
    if ok:
        print(f"OK: self-test — all {len(refused)} written-out chunk sizes are refused")

    # THE VALID NEIGHBOURS. Each of these is a real line from this repository or a
    # shape indistinguishable from one, and a rule that refused any of them would
    # pass the block above while making the gate unusable.
    accepted = [
        "        for chunk in iter(lambda: handle.read(STREAM_CHUNK_BYTES), b''):",
        '    chunk = sys.stdin.buffer.read(chunk_bytes)',
        "    text = handle.read()",
        "const FLUSH_EVERY_BYTES: usize = 1 << 20;",
        "_U64 = (1 << 64) - 1",
        "    marker = handle.read(1)",
        "    width = handle.read(2)",
        # Below the stated kibibyte cutoff, so a fixed-width field read rather than a
        # stream. The digit-counting rule refused this one while the docstring said
        # otherwise.
        "    header = handle.read(1000)",
        "DIGEST = 1 << 22  # a bare definition is not a read",
    ]
    wrongly = [line for line in accepted if offences(here, line)]
    if wrongly:
        print(f"SELF-TEST FAIL: these must be accepted and were refused: {wrongly}")
        ok = False
    else:
        print(f"OK: self-test — all {len(accepted)} named or non-streaming reads are accepted")

    # And the tree as it stands, which is the neighbour for the whole gate.
    standing = scan()
    if standing:
        print("SELF-TEST FAIL: the tree as committed is refused:")
        for problem in standing:
            print(f"  {problem}")
        ok = False
    else:
        print("OK: self-test — no script writes the chunk size out (the valid neighbour)")

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()

    found = scan()
    if found:
        sys.exit(
            "FAIL: a streamed read writes its chunk size out instead of naming it:\n  "
            + "\n  ".join(found)
            + "\n  Six copies under two names across five files, two already drifted (one to a "
            "quarter\n  of the shared size, one to a sixteenth), is how this went wrong the first "
            "time —\n  and every chunk size produces a correct digest, so nothing reports it."
        )
    print(f"OK: every streamed read under {SCRIPTS.name}/ names its chunk size")
    return 0


if __name__ == "__main__":
    sys.exit(main())
