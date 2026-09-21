#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

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
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = REPO_ROOT / "scripts"

# `read(1 << 22)`, `read(4194304)`, `.read(1<<20)` — a literal count, decimal or
# shifted, handed to a read. `\b` on `read` keeps `handle.read` and bare `read`
# while excluding `spread(`.
LITERAL_READ = re.compile(r"\bread\(\s*(\d+\s*<<\s*\d+|\d{4,})\s*\)")

# The one definition, and the shell variable carrying it into an embedded block.
DEFINING_FILE = "lane_chunk.py"

# This gate's own fixtures ARE the shape it refuses — that is what makes the
# self-test non-vacuous — so it does not scan itself. Stated rather than left as a
# quiet exclusion: the alternative is a gate that cannot describe what it refuses.
GATE_FILE = Path(__file__).name


def offences(path: Path, text: str) -> list[str]:
    """Every literal-sized streamed read in one file, as a diagnosis per hit."""
    found: list[str] = []
    for number, line in enumerate(text.splitlines(), start=1):
        for match in LITERAL_READ.finditer(line):
            literal = match.group(1)
            # A fixed-width field read is not a streamed chunk. `\d{4,}` already
            # excludes `read(1)`; a shift form is judged on its value.
            if "<<" in literal:
                base, shift = (part.strip() for part in literal.split("<<"))
                if int(base) << int(shift) < 1024:
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
    for path in sorted(SCRIPTS.iterdir()):
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
