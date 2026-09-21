#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Refuse a tracked path that misrepresents itself to the tools that read it.

Two distinct hazards, with two distinct mechanisms, because the first version of
this file collapsed them into one justification that did not survive being
measured — it refused leading whitespace and accepted an interior space on
argument-splitting grounds, and argument splitting does not distinguish them that
way. A refusal is a claim; these are the measurements behind each one.

**A component beginning with ``-`` is read as a FLAG.** Most of this repository's
tooling reaches files through globs, and a glob expands to a bare argument list
with no marker saying "this is a path".

This is not hypothetical. A test stand-in wrote its payload to the last argument
it was given, was pointed at a lane whose final argument is ``--manifest``, and
deposited a 71-byte file called ``--manifest`` at the repository root. It was
committed. At that point ``wc -l *`` in the root died with "unrecognized option",
``head -n1 *`` exited non-zero, and ``tar cf … *`` refused — every one of them
blaming a flag nobody typed. Roughly thirty hygiene scripts swept the tree and
none noticed, because they all walk a file list rather than a glob.

**A name containing a newline, tab, or carriage return renders differently from
what it is.** A newline makes one path two entries in any line-oriented tool; a
tab splits a field in any tab-separated one; a carriage return makes the DISPLAYED
name differ from the real one — a file called ``has\\rcr`` prints as ``hascr``, so
a reviewer cannot see what they are approving. ``\\t`` was refused here and
``\\r`` was not, which had no basis.

**Leading or trailing whitespace is refused for INVISIBILITY, not for splitting.**
Measured in a directory holding ``' lead'``, ``'mid space'`` and ``'trail '``:

* a glob passes all three intact, as three arguments — so globbing justifies no
  whitespace rule at all, and the original derivation was wrong about its own
  central mechanism;
* unquoted ``$(ls)`` and default ``xargs`` turn those three names into four
  words: ``' lead'`` and ``'trail '`` merely lose their space and stay ONE word,
  while ``'mid space'`` SPLITS. The splitting hazard is the interior space, which
  this gate accepts.

So splitting cannot justify either rule, and is not claimed to. What leading and
trailing whitespace really cost is that two distinct tracked paths render
identically in every listing, diff and review, and that a tool round-tripping the
name through word splitting silently addresses a different file.

**An interior space is accepted**, deliberately. It is visible, and the splitting
hazard it poses is real — the answer to unquoted command substitution is to quote
it, not to forbid spaces in filenames. A gate that refused ordinary names would be
removed, and then none of the above would be enforced either.

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
            misrendering = {"\n": "a newline", "\t": "a tab", "\r": "a carriage return"}
            hit = next((name for char, name in misrendering.items() if char in component), None)
            if hit is not None:
                found.append(
                    f"{path!r}: component {component!r} contains {hit}, so the name a tool "
                    f"reads or displays is not the name on disk"
                )
                break
            if component != component.strip():
                found.append(
                    f"{path!r}: component {component!r} begins or ends with whitespace, so it "
                    f"renders identically to a different path and word splitting silently "
                    f"addresses that other one"
                )
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
        # `\r` was ACCEPTED here while `\t` was refused, with no basis for the
        # difference. It is the worst of the three for review: `has\rcr` prints as
        # `hascr`, so the rendered name is not the name.
        "has\rcr",
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
        # AN INTERIOR SPACE IS ACCEPTED, and this is the neighbour that keeps the
        # whitespace rule honest: it is the shape that actually SPLITS under unquoted
        # command substitution, so a gate refusing leading whitespace on splitting
        # grounds would have to refuse this too. It does not, because the answer to
        # unquoted substitution is to quote it.
        "docs/design/a file with spaces.md",
        # A hyphen inside a component is not a leading one.
        "crates/rdf-core/src/dataset_view.rs",
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
    # WHAT WAS INSPECTED, not a broader claim about safety. The previous message read
    # "safe to pass as arguments", which asserts far more than three name-shape rules
    # can establish — an interior space is accepted here and is not safe to pass
    # unquoted. A gate that overstates its own scope is how a reader comes to believe a
    # surface is covered when it is not, which is the failure this file exists inside.
    print(
        f"OK: none of the {len(paths)} tracked paths begins a component with '-', contains a "
        "newline, tab or carriage return, or begins or ends a component with whitespace"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
