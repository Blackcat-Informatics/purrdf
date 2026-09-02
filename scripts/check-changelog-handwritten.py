#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Refuse to let ``make changelog`` silently destroy hand-authored release notes.

``make changelog`` runs ``git-cliff --output CHANGELOG.md``: it regenerates the
**whole file** from conventional-commit *subjects*. ``cliff.toml``'s body template
emits exactly ``commit.message | split(pat="\\n") | first`` per bullet — one line,
no commit body, no footer, and ``footer = ""``. git-cliff is given no
``--prepend`` and the config declares no keep-region, no fence and no manual
section, so there is **no mechanism by which hand-written prose survives**.

That matters because `CHANGELOG.md`'s `[Unreleased]` section is, today, largely
hand-authored: multi-line bullets that explain what a consumer must DO about a
breaking change, none of which any commit subject contains. Regenerating turns
each of them into a one-line subject, and `cliff.toml` says why that is not
survivable for the breaking ones:

    "Pre-1.0 the suite ships breaking changes under a minor bump, so the
     changelog is the only place a consumer can see that a bump carries one —
     it must never be silent."

So this gate stands at the point of loss — a pre-flight on the `changelog`
target — rather than in `make check`, because the file is *supposed* to carry
hand-authored prose; failing the ordinary gate on it would be backwards. It
fails when regeneration would drop hand-authored content that is not in the
history it regenerates from, and names what would be lost.

Deliberately NOT implemented here: a check that every ``**BREAKING**`` bullet has
a marker-bearing (``type(scope)!:`` / ``BREAKING CHANGE:``) commit behind it.
That check cannot be made non-vacuous. Its only key is the conventional-commit
SCOPE, and a release cycle accumulates breaking commits across many scopes, so
`capi`, `cli` and `rdf` already match something for reasons unrelated to any
particular entry — it would pass entries that are about to vanish. It also could
not run in CI: `.github/workflows/ci.yaml` checks out at `actions/checkout`'s
default depth of 1, so no tag and no history are present to compute a release
range from.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
CHANGELOG = REPO_ROOT / "CHANGELOG.md"

# The longest conventional-commit subject this repository has ever carried is
# ~163 characters, and `scripts/`-adjacent commit linting keeps them one line.
# A bullet longer than this, or one that wraps, cannot have come from a subject.
SUBJECT_CEILING = 200

UNRELEASED = re.compile(r"^## \[Unreleased\]\s*$", re.M)
NEXT_RELEASE = re.compile(r"^## \[", re.M)
BULLET = re.compile(r"^- (.*?)(?=^- |\Z)", re.M | re.S)
BREAKING = re.compile(r"^\*\*BREAKING\*\*")


def has_unreleased(text: str) -> bool:
    """Whether the changelog carries an `[Unreleased]` section at all."""
    return UNRELEASED.search(text) is not None


def unreleased_section(text: str) -> str:
    """The `[Unreleased]` block. Precondition: [`has_unreleased`] is true."""
    start = UNRELEASED.search(text)
    assert start is not None, "call has_unreleased first"
    rest = text[start.end() :]
    end = NEXT_RELEASE.search(rest)
    return rest if end is None else rest[: end.start()]


def unreproducible_bullets(section: str) -> list[str]:
    """Bullets git-cliff's subject-only template could not have produced."""
    found = []
    for match in BULLET.finditer(section):
        bullet = match.group(1).rstrip()
        if "\n" in bullet or len(bullet) > SUBJECT_CEILING:
            found.append(bullet)
    return found


def first_line(bullet: str, width: int = 96) -> str:
    line = bullet.split("\n", 1)[0].strip()
    return line if len(line) <= width else line[: width - 1] + "…"


def main() -> int:
    text = CHANGELOG.read_text(encoding="utf-8")
    if not has_unreleased(text):
        print("OK: CHANGELOG.md has no [Unreleased] section to lose.")
        return 0

    at_risk = unreproducible_bullets(unreleased_section(text))
    if not at_risk:
        print(
            "OK: every [Unreleased] entry is reproducible from a commit subject; "
            "regenerating loses nothing."
        )
        return 0

    breaking = [b for b in at_risk if BREAKING.match(b)]
    print(
        f"Regenerating CHANGELOG.md would DESTROY {len(at_risk)} hand-authored "
        f"[Unreleased] entr{'y' if len(at_risk) == 1 else 'ies'}, "
        f"{len(breaking)} of them **BREAKING**.",
        file=sys.stderr,
    )
    print(
        "\ngit-cliff renders one line per commit SUBJECT. It is given no "
        "--prepend, and cliff.toml declares no keep-region, fence or manual "
        "section, so none of the prose below exists anywhere it regenerates "
        "from. The BREAKING ones are release notes for an incompatible change: "
        "cliff.toml itself states such a change 'must never be silent'.\n",
        file=sys.stderr,
    )
    for bullet in breaking:
        print(f"  BREAKING  {first_line(bullet)}", file=sys.stderr)
    other = len(at_risk) - len(breaking)
    if other:
        print(f"  … and {other} further hand-authored entr(y|ies).", file=sys.stderr)
    print(
        "\nWHAT TO DO\n"
        "  1. Copy the [Unreleased] section somewhere before regenerating.\n"
        "  2. Run `make changelog`, bypassing this gate deliberately:\n"
        "       CHANGELOG_ALLOW_REGENERATE=1 make changelog\n"
        "  3. Re-author the release notes over the regenerated one-line "
        "bullets. A consumer needs what to DO about each break, which no "
        "subject carries.\n"
        "  4. For a FUTURE break, put the marker on the commit as it is "
        "written — `type(scope)!:` or a `BREAKING CHANGE:` footer — so at "
        "least the marker survives regeneration without re-authoring.",
        file=sys.stderr,
    )
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
