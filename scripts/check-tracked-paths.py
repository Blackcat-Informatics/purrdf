#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
a reviewer cannot see what they are approving.

That ground reaches further than the three characters first named, and the rule now
goes where the ground goes: every C0 control and DEL, plus the bidirectional
overrides U+202A-U+202E and U+2066-U+2069. An ANSI escape can erase and rewrite a
whole rendered line, and a bidi override reorders the characters around it -- the
trojan-source filename case -- so both defeat review strictly harder than ``\\r``
does. Refusing ``\\r`` while accepting ``\\x1b`` was the same unfounded distinction
as refusing ``\\t`` while accepting ``\\r``, one round earlier. Measured rather than
assumed: every tracked path passes, and the success line carries the live count.

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
import unicodedata
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


def _misrendering(component: str) -> str | None:
    """Why *component* does not render as what it is, or None.

    THE RULE GOES WHERE ITS GROUND GOES. The first version refused a newline, a tab and a
    carriage return; the second added the C0 controls and the bidi overrides. Both stopped
    short of the stated ground -- "two distinct tracked paths render identically" and "one
    path becomes two entries in any line-oriented tool" -- which is satisfied strictly
    harder by characters both versions accepted:

    * ``U+200B`` (zero width space), ``U+FEFF`` (zero width no-break space), ``U+3164``
      (Hangul filler) and ``U+2800`` (blank Braille pattern) render as NOTHING, including
      leading, so two paths are indistinguishable in every listing, diff and review;
    * ``U+200E``/``U+200F`` (LRM/RLM) are in Rust's own trojan-source lint set and GitHub's
      advisory, and were accepted while ``U+202E`` was refused;
    * ``U+2028``/``U+2029`` are line and paragraph separators, and ``"a\u2028b".splitlines()``
      returns two entries -- which is the exact reason a newline is refused.

    Judged by Unicode general category where one exists, so the rule is a property rather
    than a list that stops wherever attention ran out. Measured, not assumed: every tracked
    path passes.
    """
    for char in component:
        code = ord(char)
        category = unicodedata.category(char)
        # `Cc` RATHER THAN AN ASCII RANGE. `code < 0x20 or code == 0x7F` stopped exactly
        # where attention ran out: U+0085 NEL is `Cc`, and `"a\u0085b".splitlines()`
        # returns two entries -- the docstring's own stated ground for refusing a newline.
        # U+009B is the 8-bit CSI, the ground for refusing an escape. Both were accepted.
        if category == "Cc":
            return f"the control character U+{code:04X}"
        # A SPACE THAT IS NOT THE SPACE. There was no `Zs` rule at all, so `a b.md` and
        # `a\u00a0b.md` were both accepted and render identically -- the exact cost this
        # gate names. The asymmetry was accidental: `str.strip()` eats a LEADING NBSP while
        # an interior one walked through.
        if category == "Zs" and char != " ":
            return f"U+{code:04X}, a space character that is not the ordinary space"
        if category == "Cf":
            # ZWNJ AND ZWJ ARE ORTHOGRAPHY, NOT DECORATION. They are required in Persian
            # (`می‌روم`) and control Indic conjuncts, and a blanket `Cf` refusal took them
            # -- an over-refusal inside a rule about honesty. Excluded by name, with the
            # reason, rather than by narrowing the category and losing the bidi controls.
            if char in "\u200c\u200d":
                continue
            return f"the invisible format character U+{code:04X} ({category})"
        if category in {"Zl", "Zp"}:
            return f"the line or paragraph separator U+{code:04X} ({category})"
        if char in _BLANK_BUT_NOT_SPACE:
            return f"U+{code:04X}, which renders as nothing at all"
    return None


# Characters that render blank without being a space and without a category that says so,
# so a name containing one is indistinguishable from a name without it.
# CHARACTERS THAT RENDER AS NOTHING ON THEIR OWN. The set is deliberately narrow, and it
# was wrong in both directions when first written.
#
# U+FE0F was in it, and U+FE0F is VARIATION SELECTOR-16: it SELECTS emoji presentation for
# the character before it. It renders nothing by itself because it is not a character on
# its own -- and refusing it rejected `❤️.md`, `⚠️-warning.md` and `🏳️‍🌈.md`, ordinary
# emoji paths. The accept-side fixture was `🐈.md`, which carries no variation selector, so
# it could not observe the hazard the rule introduced: a neighbour that cannot distinguish
# the case it exists for. U+FE00-FE0E are the same block and are equally excluded.
#
# The Hangul and Khmer fillers and U+180E stay: those are format-class blanks with no
# preceding character to modify, and they are what let two paths render identically. U+034F
# (COMBINING GRAPHEME JOINER) also stays -- it is invisible and carries no orthographic
# requirement in any script this repository documents.
_BLANK_BUT_NOT_SPACE = frozenset(
    "\u3164\u2800\u115f\u1160\u17b4\u17b5\uffa0\u034f\u180e"
)


def collisions(paths: list[str]) -> list[str]:
    """Every pair of tracked paths that name the same file to a filesystem or a reader.

    THIS IS THE GATE'S OWN STATED GROUND, APPLIED TO PAIRS. Every other rule here refuses a
    single path for rendering as something it is not; two paths differing only by Unicode
    normalisation render IDENTICALLY and were both accepted, because the per-path rules
    cannot see a pair. `café.md` in NFC (U+00E9) and in NFD (e + U+0301) are different byte
    strings, the same rendering, and the SAME FILE on APFS and HFS+ -- so a checkout there
    silently loses one tracked file with no error at all.

    Case-insensitive filesystems collapse `README.md` and `Readme.md` the same way. Both
    checks take the whole list, which is why they live here rather than beside the others.
    """
    found: list[str] = []
    by_normal: dict[str, set[str]] = {}
    by_fold: dict[str, set[str]] = {}
    # DISTINCT PATHS ONLY. A path does not collide with itself, and `git ls-files` emits one
    # entry per index STAGE -- so during an unresolved merge a single conflicted file appears
    # three times and read as a three-way collision with itself. Caught by running the gate
    # mid-merge, which is the one state that produces it.
    for path in sorted(set(paths)):
        by_normal.setdefault(unicodedata.normalize("NFC", path), set()).add(path)
        by_fold.setdefault(unicodedata.normalize("NFC", path).casefold(), set()).add(path)
    for normal, members in sorted(by_normal.items()):
        if len(members) > 1:
            found.append(
                f"{sorted(members)!r} differ only by Unicode normalisation, so they render "
                f"identically and are the SAME FILE on APFS and HFS+ -- a checkout there "
                f"loses one of them with no error. Normalise to NFC: {normal!r}"
            )
    for folded, members in sorted(by_fold.items()):
        distinct = {unicodedata.normalize("NFC", member) for member in members}
        if len(members) > 1 and len(distinct) > 1:
            found.append(
                f"{sorted(members)!r} differ only by case, so they are the same file on a "
                f"case-insensitive filesystem and a checkout there loses one"
            )
    return found


def offences(paths: list[str]) -> list[str]:
    """Every tracked path that a shell cannot hand to a command unambiguously."""
    found: list[str] = collisions(paths)
    for path in paths:
        for component in path.split("/"):
            if component.startswith("-"):
                found.append(f"{path!r}: component {component!r} begins with '-', so a glob "
                            f"expanding to it is read as a FLAG")
                break
            # EVERY CHARACTER THAT MAKES A NAME MISREPRESENT ITSELF, not the three that
            # happened to come to mind. The stated ground for refusing `\r` -- "a reviewer
            # cannot see what they are approving" -- is satisfied strictly harder by an ANSI
            # escape, which can erase and rewrite a whole rendered line, and by a bidi
            # override, which is the trojan-source filename case. Refusing `\r` while
            # accepting `\x1b` is the same unfounded distinction the `\t`-versus-`\r` split
            # was, one round later.
            #
            # Over-refusal risk is measured, not assumed: every tracked path passes, and the success line carries the live count.
            named = {"\n": "a newline", "\t": "a tab", "\r": "a carriage return"}
            hit = next((name for char, name in named.items() if char in component), None)
            if hit is None:
                hit = _misrendering(component)
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
        # An ANSI escape can erase and rewrite the rendered line, which is strictly worse
        # than a carriage return by the docstring's own stated ground.
        "has\x1besc",
        "has\bbackspace",
        "has\x0cformfeed",
        "has\x7fdel",
        # The trojan-source filename case: the name renders in a different order.
        "has\u202eoverride",
        # Invisible: two distinct paths that render identically, which is the stated ground.
        "has\u200bzerowidth",
        "\u200bleading-zero-width",
        "has\ufeffbom",
        "has\u3164hangulfiller",
        "has\u2800blankbraille",
        # In Rust's trojan-source lint set, and accepted while U+202E was refused.
        "has\u200elrm",
        "has\u200frlm",
        # `"a\u2028b".splitlines()` returns two entries, which is why `\n` is refused.
        "has\u2028lineseparator",
        "has\u2029paragraphseparator",
        # `"a\u0085b".splitlines()` returns two entries, which is the stated ground for
        # refusing a newline; U+009B is the 8-bit CSI. Both are `Cc` and both were
        # accepted while `\r` was refused.
        "has\u0085nel",
        "has\u009bcsi",
        # A space that is not the space: these render identically to `a b`.
        "has\u00a0nbsp",
        "has\u3000ideographicspace",
        "has\u202fnarrownbsp",
        # The halfwidth twin of a filler already refused.
        "has\uffa0halfwidthfiller",
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
        # NON-ASCII PATHS ARE ORDINARY. This list gained nothing when the rule was
        # widened, which is how the ZWNJ over-refusal went unnoticed.
        "docs/zh-Hans/说明.md",
        "docs/fr/café.md",
        "docs/fa/می\u200cروم.md",
        "docs/emoji/🐈.md",
        "docs/he/עברית.md",
        # VARIATION SELECTORS ARE NOT BLANKS. Each of these was refused by a rule that
        # called U+FE0F "a character that renders as nothing at all"; the previous accept
        # fixture was a cat with no variation selector, which could not observe it.
        "docs/emoji/❤️.md",
        "docs/emoji/⚠️-warning.md",
        "docs/emoji/🏳️‍🌈.md",
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
        "character that does not render as what it is (a control, an invisible format "
        "character, a line separator), begins or ends a component with whitespace, or "
        "collides with another tracked path under Unicode normalisation or case folding"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
