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

# It checks the WHOLE file, not just `[Unreleased]`

The first version of this gate inspected `[Unreleased]` alone, and reported
"regenerating loses nothing" while regeneration deleted the entire `## [1.0.0]`
section — 601 lines, including the prose explaining why 1.0.0 republished the
0.13.0 tree. The statement was true of the section it read and false of the file
it was guarding. A RELEASED section carries strictly more hand-authored prose
than `[Unreleased]` ever does, because release notes are written at release time.

The structural half of that failure is now checked by COMPARISON rather than
prediction: git-cliff is run to a temporary file and the section headings of the
result are compared against the committed ones. A heading that exists now and
would not exist after is a hard refusal, named. That is a cheap invariant, it
needs no prose analysis, and it is exactly what was missed.

The prose half stays a WARNING for released sections and an ERROR for
`[Unreleased]`. Regeneration replaces prose in every released section by
construction, so erroring on that would make this gate fail every single time and
train the operator to reach past it — the failure mode of a gate nobody can
satisfy. `[Unreleased]` keeps erroring because it is the section a release is
about to consume.

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


SECTION = re.compile(r"^## \[([^\]]+)\]", re.M)


def section_names(text: str) -> list[str]:
    """Every `## [name]` heading, in file order."""
    return SECTION.findall(text)


def sections(text: str) -> list[tuple[str, str]]:
    """`(name, body)` for every `## [name]` section, in file order."""
    out = []
    marks = list(SECTION.finditer(text))
    for i, m in enumerate(marks):
        end = marks[i + 1].start() if i + 1 < len(marks) else len(text)
        out.append((m.group(1), text[m.end() : end]))
    return out


def regenerate() -> str | None:
    """What `make changelog` would write, without writing it.

    Mirrors the Makefile's invocation. Returns `None` when git-cliff cannot be
    run at all — the caller then falls back to the prose-only checks rather than
    passing silently, because a gate that no-ops when its tool is missing is a
    gate that reports success for the wrong reason.
    """
    import subprocess
    import tomllib

    try:
        version = tomllib.load((REPO_ROOT / "Cargo.toml").open("rb"))["workspace"][
            "package"
        ]["version"]
        done = subprocess.run(
            ["git-cliff", "--config", "cliff.toml", "--tag", f"rust-v{version}"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            timeout=180,
            check=False,
        )
    except (OSError, KeyError, subprocess.SubprocessError):
        return None
    return done.stdout if done.returncode == 0 and done.stdout.strip() else None


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


def vanishing_sections(text: str) -> list[str] | None:
    """Headings the committed file has that a regeneration would not.

    `None` means the comparison could not be made (git-cliff absent or failing),
    which is reported rather than treated as a pass.
    """
    regenerated = regenerate()
    if regenerated is None:
        return None
    after = set(section_names(regenerated))
    return [name for name in section_names(text) if name not in after]


def main() -> int:
    text = CHANGELOG.read_text(encoding="utf-8")

    # 1. STRUCTURAL, by comparison: a section that exists now and would not exist
    #    after regeneration is deleted content, whatever its prose looks like.
    #    This is the check whose absence let the whole 1.0.0 section vanish under
    #    a green gate.
    vanishing = vanishing_sections(text)
    if vanishing is None:
        print(
            "WARNING: could not run git-cliff to compare, so the section-loss "
            "check was skipped; only the prose checks below ran.",
            file=sys.stderr,
        )
    elif vanishing:
        print(
            f"Regenerating CHANGELOG.md would DELETE {len(vanishing)} released "
            f"section(s) outright:",
            file=sys.stderr,
        )
        for name in vanishing:
            print(f"  ## [{name}]", file=sys.stderr)
        print(
            "\ngit-cliff rebuilds the whole file from the tag range it can see. A "
            "section it does not emit is not merged in — it is gone, along with "
            "every hand-written release note under it.\n"
            "\nWHAT TO DO\n"
            "  1. Keep the committed CHANGELOG.md.\n"
            "  2. Regenerate to a SCRATCH file, take only the new version's "
            "section from it, and splice that in above the newest existing one.\n"
            "  3. Re-check that every heading present before is present after.",
            file=sys.stderr,
        )
        return 1

    # The prose heuristic (`unreproducible_bullets`) stays scoped to
    # `[Unreleased]`, where it was designed to work. Applied to released sections
    # it mis-fires: its bullet regex runs to the next bullet, so the last bullet in
    # a section swallows the trailing blank lines and the next heading and looks
    # multi-line. Run against git-cliff's OWN output it flagged 99 entries as
    # "would be rewritten by regeneration" — entries that were the regeneration.
    # A warning that cries wolf on correct input is worse than no warning; the
    # structural comparison above is the check this gate was missing.

    if not has_unreleased(text):
        print("OK: CHANGELOG.md has no [Unreleased] section to lose.")
        return 0

    at_risk = unreproducible_bullets(unreleased_section(text))
    if not at_risk:
        print(
            "OK: every [Unreleased] entry is reproducible from a commit subject; "
            "no section would be deleted."
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
