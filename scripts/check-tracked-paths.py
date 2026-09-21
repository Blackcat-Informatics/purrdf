#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Refuse tracked paths that a shell cannot safely pass to a command.

A path is not just a name: it is an argument. Most of this repository's tooling —
its own scripts, `make` recipes, and whatever an operator types — reaches files
through globs, and a glob expands to a bare argument list with no marker saying
"this is a path". So a tracked file whose name begins with ``-`` is read as a
FLAG by every command it reaches.

This is not hypothetical. A test stand-in wrote its payload to the last argument
it was given, was pointed at a lane whose final argument is ``--manifest``, and
deposited a 71-byte file called ``--manifest`` at the repository root. It was
committed. At that point ``wc -l *`` in the root died with "unrecognized option",
``head -n1 *`` exited non-zero, and ``tar cf … *`` refused — every one of them
blaming a flag nobody typed. Roughly thirty hygiene scripts swept the tree and
none of them noticed, because they all walk a file list rather than a glob.

The rule is therefore about the shape of the NAME, independent of contents:

* a path component may not begin with ``-``;
* nor may it begin or end with whitespace, or contain a newline or a tab, which
  break argument splitting and line-oriented tooling the same way.

``--`` would end option parsing for a well-written caller, and most callers here
are well written. The point is that a repository should not require every future
caller to be.
"""

import argparse
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def tracked_paths() -> list[str]:
    """Every path git tracks, one per line, with names taken literally.

    ``-z`` is used so a name containing a newline arrives intact rather than as
    two entries — a check for unsafe names must not be defeated by one.
    """
    result = subprocess.run(
        ["git", "-C", str(REPO_ROOT), "ls-files", "-z"],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        sys.exit(f"FAIL: git ls-files failed: {result.stderr.strip()}")
    return [entry for entry in result.stdout.split("\0") if entry]


def offences(paths: list[str]) -> list[str]:
    """Every tracked path that a shell cannot hand to a command unambiguously."""
    found: list[str] = []
    for path in paths:
        for component in path.split("/"):
            if component.startswith("-"):
                found.append(f"{path!r}: component {component!r} begins with '-', so a glob "
                            f"expanding to it is read as a FLAG")
                break
            if component != component.strip():
                found.append(f"{path!r}: component {component!r} begins or ends with whitespace")
                break
            if "\n" in component or "\t" in component:
                found.append(f"{path!r}: component {component!r} contains a newline or tab")
                break
    return found


def self_test() -> int:
    """Both directions: the shapes that must be refused, and the ones that must not."""
    ok = True

    refused = [
        "--manifest",
        "-rf",
        "scripts/--out",
        "crates/bench/-weird",
        " leading-space",
        "trailing-space ",
        "has\nnewline",
        "has\ttab",
    ]
    for path in refused:
        if not offences([path]):
            print(f"SELF-TEST FAIL: {path!r} should be refused and was not")
            ok = False
    if ok:
        print(f"OK: self-test — all {len(refused)} unsafe name shapes are refused")

    # THE VALID NEIGHBOUR, and it is the half that matters: a check that refused
    # every path would pass the list above and reject the whole repository.
    accepted = [
        "scripts/check-tracked-paths.py",
        "crates/bench/tests/make_bench_lanes.rs",
        "docs/design/purrdf-bench-lane-laws.md",
        "README.md",
        "a-file-with-hyphens.txt",
        "dir-with-hyphens/inner-file.rs",
        "Cargo.toml",
    ]
    wrongly = [path for path in accepted if offences([path])]
    if wrongly:
        print(f"SELF-TEST FAIL: ordinary paths were refused: {wrongly}")
        ok = False
    else:
        print(f"OK: self-test — all {len(accepted)} ordinary paths are accepted")

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()

    paths = tracked_paths()
    found = offences(paths)
    if found:
        sys.exit(
            "FAIL: tracked path(s) a shell cannot pass to a command safely:\n  "
            + "\n  ".join(found)
            + "\n  A glob expanding to one of these is read as a flag or split wrongly by "
            "every\n  command it reaches. Rename or remove them."
        )
    print(f"OK: all {len(paths)} tracked paths are safe to pass as arguments")
    return 0


if __name__ == "__main__":
    sys.exit(main())
