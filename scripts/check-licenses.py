#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""License-hygiene gate for vendored corpora.

Every vendored test-suite tree in this repo follows the REUSE convention: it
holds a ``LICENSES/`` directory of license texts, and every file it ships is
licensed either by a per-file ``<file>.license`` sidecar (verbatim third-party
material), an inline ``SPDX-License-Identifier`` header (first-party source), or
a ``REUSE.toml`` annotation (first-party files that cannot carry a header, e.g.
harness-authored ``manifest.ttl`` selectors and reconstructed ``.srx`` results).

This gate fails the build if any file under a vendored root is *undeclared* — so
a future re-sync that drops a ``.license`` sidecar cannot slip through silently
(the "no silent skips" doctrine, applied to provenance/licensing). It scales
automatically: any new directory that adds a ``LICENSES/`` subdir becomes a
vendored root and is enforced with zero changes here.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
import tomllib
from pathlib import Path

# Where vendored corpora live. A "vendored root" is any directory below these
# that contains a LICENSES/ subdirectory (the REUSE marker).
SCAN_AREAS = ("crates", "bindings")

# Files that never need their own license declaration.
EXEMPT_NAMES = {"REUSE.toml"}
# Extensions/paths that are documentation or license text, not vendored payload.
EXEMPT_SUFFIXES = (".license",)


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def find_vendored_roots(root: Path) -> list[Path]:
    roots: list[Path] = []
    for area in SCAN_AREAS:
        base = root / area
        if not base.is_dir():
            continue
        for licenses_dir in base.rglob("LICENSES"):
            if licenses_dir.is_dir():
                roots.append(licenses_dir.parent)
    return sorted(set(roots))


def reuse_covered_paths(vendored_root: Path) -> set[Path]:
    """Resolve every file covered by a REUSE.toml annotation in ``vendored_root``."""
    reuse_toml = vendored_root / "REUSE.toml"
    if not reuse_toml.is_file():
        return set()
    data = tomllib.loads(reuse_toml.read_text(encoding="utf-8"))
    covered: set[Path] = set()
    for annotation in data.get("annotations", []):
        patterns = annotation.get("path", [])
        if isinstance(patterns, str):
            patterns = [patterns]
        for pattern in patterns:
            for match in vendored_root.glob(pattern):
                if match.is_file():
                    covered.add(match.resolve())
    return covered


def has_inline_spdx(path: Path) -> bool:
    try:
        with path.open("r", encoding="utf-8", errors="ignore") as handle:
            for _ in range(8):
                line = handle.readline()
                if not line:
                    break
                if "SPDX-License-Identifier" in line:
                    return True
    except OSError:
        return False
    return False


def is_declared(path: Path, reuse_covered: set[Path]) -> bool:
    if path.with_name(path.name + ".license").is_file():
        return True
    if path.resolve() in reuse_covered:
        return True
    return has_inline_spdx(path)


def check_root(vendored_root: Path) -> list[Path]:
    reuse_covered = reuse_covered_paths(vendored_root)
    offenders: list[Path] = []
    for path in sorted(vendored_root.rglob("*")):
        if not path.is_file():
            continue
        # Skip license texts and the sidecars/config themselves.
        if "LICENSES" in path.relative_to(vendored_root).parts:
            continue
        if path.name in EXEMPT_NAMES or path.suffix in EXEMPT_SUFFIXES:
            continue
        if not is_declared(path, reuse_covered):
            offenders.append(path)
    return offenders



MULAN_SHA256 = "eb7a1d713eb919b146787629e22e4c975cb701f529a65d4d7e0fcd417558bf1c"


def check_mulan_text(root: Path) -> list[str]:
    """The committed MulanPSL-2.0 text is a byte contract: its Chinese text
    governs (its own section 6), so a silently drifted copy would change the
    controlling terms. Both committed copies must match the pinned digest."""
    import hashlib

    problems = []
    copies = [root / "LICENSE-MULAN", root / "LICENSES" / "MulanPSL-2.0.txt"]
    for copy in copies:
        if not copy.exists():
            problems.append(f"missing {copy.relative_to(root)}")
            continue
        digest = hashlib.sha256(copy.read_bytes()).hexdigest()
        if digest != MULAN_SHA256:
            problems.append(
                f"{copy.relative_to(root)} digest {digest} != pinned {MULAN_SHA256}"
            )
    return problems



# ── First-party headers must name the whole license offer ─────────────────────
#
# The MulanPSL-2.0 addition rewrote the SPDX identifier in 1707 files. A branch
# in flight at the time merged it without one conflict -- correctly, because its
# new files had no counterpart to conflict with -- and five of them stayed at
# `MIT OR Apache-2.0` while every other file in the tree offered three licenses.
#
# Nothing here reported that. The vendored-root check below asks whether a file
# DECLARES a license; it never asked whether a first-party file declares the
# RIGHT one. So a contributor adding a file during any future license change
# ships a file under a narrower offer than the project makes, and every gate is
# green. That is not a hypothetical: it happened, and it was found by a one-off
# grep, which is exactly the kind of proof this repository does not accept.
#
# The expression is READ FROM `Cargo.toml`, never restated. `[workspace.package]
# license` is what cargo publishes to crates.io and therefore what the project's
# offer actually IS; a copy here would be a second place to change and would
# diverge in precisely the situation this exists to catch.


def workspace_license(root: Path) -> str:
    """The license expression `[workspace.package]` declares, which is the project's offer.

    Parsed, not line-scanned. The first version returned the first line ANYWHERE in
    `Cargo.toml` beginning `license = `, while its own comment claimed to read
    `[workspace.package]` -- so a `[package.metadata.*] license` above line 56 would
    silently have become the gate's expectation. It also did `split('"')[1]`, which
    raises IndexError on `license = 'MIT OR Apache-2.0'`: a legal TOML literal string
    produced a traceback instead of the stated diagnostic. `tomllib` was already
    imported in this file.
    """
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    try:
        return manifest["workspace"]["package"]["license"]
    except (KeyError, TypeError):
        sys.exit(
            "check-licenses: Cargo.toml has no [workspace.package] license; the project's "
            "offer cannot be derived, and this gate will not assume one."
        )


# Suffixes whose first-party files carry a header this gate judges. Kept to the
# languages the tree actually writes: a suffix added here without being checked
# would widen the gate's apparent scope over a surface it never inspects.
HEADER_SUFFIXES = (".rs", ".py", ".pyi", ".sh", ".mjs", ".js", ".toml", ".yaml", ".yml")

# First-party files that DELIBERATELY declare a different license, with the reason.
# This register may only SHRINK: an entry whose file no longer declares something
# else is reported as stale, so an exemption cannot outlive its justification.
#
# The alternative -- dropping the whole rule because one file is different -- is how
# a gate with one awkward case becomes no gate at all.
DELIBERATE_OTHER_LICENSE: dict[str, str] = {
    # The book is PROSE, licensed CC-BY-4.0 so it can be quoted and translated on
    # terms that suit documentation rather than code. Its configuration declares the
    # license of the thing it builds, which is not the license of the tree.
    "docs/book/book.toml": "CC-BY-4.0",
}


def registration_scope_offenders() -> list[str]:
    """Every register entry whose suffix this gate does not inspect.

    "The register may only SHRINK" was true only for the nine checked suffixes.
    `DELIBERATE_OTHER_LICENSE` is keyed by path with no suffix constraint, so an entry
    for a `.md` file was an assertion nothing evaluated -- the main loop skips the
    suffix, so `declared != allowed` never runs and the staleness sweep only asks
    whether the marker is present at all. For every unchecked suffix the register was a
    silent widening.

    This is the same law `check-issue-refs.py` applies one level up: a suffix enumerated
    with no scanner behind it extends a gate's apparent scope over a surface it never
    inspects.
    """
    return [
        f"{rel}: registered as deliberately {allowed!r}, but {Path(rel).suffix!r} is not in "
        f"HEADER_SUFFIXES, so this registration asserts something nothing checks"
        for rel, allowed in sorted(DELIBERATE_OTHER_LICENSE.items())
        if Path(rel).suffix not in HEADER_SUFFIXES
    ]


def first_party_header_offenders(root: Path, expected: str) -> list[str]:
    """Every first-party file whose SPDX identifier is not the project's offer.

    A file with NO identifier is not reported: requiring one everywhere is a
    different rule with a different scope, and adding it here silently would be
    the over-refusal that gets a gate disabled. What is reported is a file that
    states an offer and states the wrong one.

    Vendored payload is excluded by the same `LICENSES/` marker the rest of this
    file uses, because a vendored file's identifier is UPSTREAM's statement and
    must not be rewritten to match ours.
    """
    vendored = find_vendored_roots(root)
    listing = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        capture_output=True,
        text=True,
        check=False,
    )
    if listing.returncode != 0:
        sys.exit(f"check-licenses: git ls-files failed: {listing.stderr.strip()}")

    offenders: list[str] = []
    for rel in sorted(part for part in listing.stdout.split("\0") if part):
        path = root / rel
        if path.suffix not in HEADER_SUFFIXES or not path.is_file():
            continue
        # EXCLUDED BY WHOSE STATEMENT IT IS, not by which directory it sits in.
        #
        # Excluding whole vendored roots protected ZERO upstream headers and hid FOUR of
        # ours: the six vendored roots contain exactly four files with a checked suffix,
        # and all four are first-party `REUSE.toml` carrying the project's own offer. So
        # the stated rationale -- "a vendored identifier is UPSTREAM's statement and must
        # not be rewritten to match ours" -- described a case that does not exist in this
        # tree, while the exclusion created the one it was written to prevent: the next
        # license change that missed those four would pass green.
        #
        # A file is upstream's if its copyright line is not ours. That is the property
        # the rationale was actually about.
        if any(vendored_root in path.parents for vendored_root in vendored):
            try:
                opening = path.read_text(encoding="utf-8").splitlines()[:8]
            except (UnicodeDecodeError, OSError):
                continue
            if not any("Blackcat Informatics" in line for line in opening):
                continue
        try:
            head = path.read_text(encoding="utf-8").splitlines()[:8]
        except (UnicodeDecodeError, OSError):
            continue
        for line in head:
            marker = "SPDX-License-Identifier:"
            if marker in line:
                declared = line.split(marker, 1)[1].strip()
                allowed = DELIBERATE_OTHER_LICENSE.get(rel)
                if allowed is not None:
                    if declared != allowed:
                        offenders.append(
                            f"{rel}: is registered as deliberately {allowed!r} but now declares "
                            f"{declared!r} — the registration is stale, so re-judge it rather "
                            f"than widening it"
                        )
                elif declared != expected:
                    offenders.append(f"{rel}: declares {declared!r}, the project offers {expected!r}")
                break

    # A REGISTERED EXEMPTION THAT NO LONGER APPLIES IS ITSELF A FINDING. Without this
    # the register only ever grows, and a path that was deleted or brought back into
    # line keeps a permanent hole open behind it.
    for registered, allowed in sorted(DELIBERATE_OTHER_LICENSE.items()):
        candidate = root / registered
        if not candidate.is_file():
            offenders.append(
                f"{registered}: registered as deliberately {allowed!r} but the file is gone"
            )
        elif "SPDX-License-Identifier:" not in candidate.read_text(
            encoding="utf-8", errors="replace"
        ):
            offenders.append(
                f"{registered}: registered as deliberately {allowed!r} but declares no license"
            )
    return offenders


def self_test() -> int:
    """The four directions the header rule needs, executed rather than described.

    This gate shipped its new refusal with no self-test and no argument parsing at all:
    `--self-test` and `--this-flag-does-not-exist` both printed the normal OK line and
    exited 0. That is worse than a missing self-test, because `check-gate-parity.py`
    makes arguments part of a gate's identity -- so wiring `check-licenses.py
    --self-test` into both lists would have produced a green no-op the parity gate
    certified as an agreeing rule.
    """
    root = repo_root()
    ok = True
    real = (root / "Cargo.toml").read_text(encoding="utf-8")
    expected = workspace_license(root)

    # 1. THE TREE AS COMMITTED IS CLEAN. First, because it is the neighbour a
    #    refusal-only suite cannot distinguish: a rule that reported everything would
    #    pass every case below and reject the repository.
    standing = first_party_header_offenders(root, expected) + registration_scope_offenders()
    if standing:
        print("SELF-TEST FAIL: the tree as committed is reported as offending:")
        for problem in standing[:5]:
            print(f"  {problem}")
        ok = False
    else:
        print(f"OK: self-test — the tree as committed declares {expected!r} throughout")

    # 2. A file at the OLD offer is caught. Checked against a real tracked file so the
    #    scan's own path handling is exercised, not a fixture the scan never walks.
    probe = root / "scripts" / "lane_chunk.py"
    original = probe.read_text(encoding="utf-8")
    try:
        probe.write_text(original.replace(expected, "MIT OR Apache-2.0", 1), encoding="utf-8")
        found = first_party_header_offenders(root, expected)
        if any("lane_chunk.py" in problem and "MIT OR Apache-2.0'" in problem for problem in found):
            print("OK: self-test — a first-party file left at a narrower offer is refused")
        else:
            print(f"SELF-TEST FAIL: a stale offer was not refused (reported: {found[:2]})")
            ok = False
    finally:
        probe.write_text(original, encoding="utf-8")

    # 3. A STALE REGISTRATION is caught. The register may only shrink, so an entry whose
    #    file has come back into line must be reported rather than silently honoured.
    registered = next(iter(DELIBERATE_OTHER_LICENSE))
    DELIBERATE_OTHER_LICENSE[registered] = "Zlib"
    try:
        found = first_party_header_offenders(root, expected)
        if any(registered in problem and "stale" in problem for problem in found):
            print("OK: self-test — a registration that no longer matches its file is refused")
        else:
            print(f"SELF-TEST FAIL: a stale registration was not refused (reported: {found[:2]})")
            ok = False
    finally:
        DELIBERATE_OTHER_LICENSE[registered] = "CC-BY-4.0"

    # 4. A REGISTRATION OUTSIDE THE CHECKED SUFFIXES is refused, so the register cannot
    #    widen into a surface nothing inspects.
    DELIBERATE_OTHER_LICENSE["docs/NOTES.md"] = "Proprietary-DoNotDistribute"
    try:
        found = registration_scope_offenders()
        if any("NOTES.md" in problem and "HEADER_SUFFIXES" in problem for problem in found):
            print("OK: self-test — a registration whose suffix is unchecked is refused")
        else:
            print(f"SELF-TEST FAIL: an out-of-scope registration was not refused ({found})")
            ok = False
    finally:
        del DELIBERATE_OTHER_LICENSE["docs/NOTES.md"]

    # 5. THE EXPRESSION IS READ FROM Cargo.toml, not restated here. This is the property
    #    the whole design rests on: a copy in this file would be a second place to change
    #    and would diverge in exactly the situation the gate exists to catch.
    try:
        (root / "Cargo.toml").write_text(
            real.replace(f'license = "{expected}"', 'license = "Zlib OR WTFPL"', 1),
            encoding="utf-8",
        )
        moved = workspace_license(root)
        found = first_party_header_offenders(root, moved)
        if moved == "Zlib OR WTFPL" and len(found) > 100:
            print(
                f"OK: self-test — changing Cargo.toml's field moves the expectation "
                f"({len(found)} files then offend), so it is read rather than restated"
            )
        else:
            print(f"SELF-TEST FAIL: the expectation did not follow Cargo.toml (got {moved!r})")
            ok = False
    finally:
        (root / "Cargo.toml").write_text(real, encoding="utf-8")

    # 6. A literal-string license parses. `split('"')[1]` raised IndexError here.
    scratch = root / "target" / "check-licenses-selftest"
    scratch.mkdir(parents=True, exist_ok=True)
    (scratch / "Cargo.toml").write_text(
        "[workspace.package]\nlicense = 'MIT OR Apache-2.0'\n", encoding="utf-8"
    )
    if workspace_license(scratch) == "MIT OR Apache-2.0":
        print("OK: self-test — a TOML literal-string license parses instead of crashing")
    else:
        print("SELF-TEST FAIL: a literal-string license did not parse")
        ok = False

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def main() -> int:
    # ARGUMENTS ARE PARSED, and an unknown one is refused. This file used to ignore argv
    # entirely, so every flag was a silent no-op that still printed OK.
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    if parser.parse_args().self_test:
        return self_test()
    root = repo_root()
    mulan_problems = check_mulan_text(root)
    if mulan_problems:
        print("License hygiene FAILED: MulanPSL-2.0 text drifted:", file=sys.stderr)
        for problem in mulan_problems:
            print(f"  {problem}", file=sys.stderr)
        return 1
    roots = find_vendored_roots(root)
    if not roots:
        print("check-licenses: no vendored roots found (LICENSES/ marker).", file=sys.stderr)
        return 0

    all_offenders: list[Path] = []
    for vendored_root in roots:
        offenders = check_root(vendored_root)
        all_offenders.extend(offenders)

    if all_offenders:
        print(
            "License hygiene FAILED: vendored files without a `.license` sidecar,\n"
            "inline SPDX header, or REUSE.toml annotation:",
            file=sys.stderr,
        )
        for path in all_offenders:
            print(f"  {path.relative_to(root)}", file=sys.stderr)
        return 1

    expected = workspace_license(root)
    scope_offenders = registration_scope_offenders()
    if scope_offenders:
        print(
            "License hygiene FAILED: the deliberate-exemption register asserts something\n"
            "this gate does not check:",
            file=sys.stderr,
        )
        for problem in scope_offenders:
            print(f"  {problem}", file=sys.stderr)
        return 1
    header_offenders = first_party_header_offenders(root, expected)
    if header_offenders:
        print(
            "License hygiene FAILED: first-party file(s) declare a different license offer\n"
            f"than Cargo.toml's {expected!r}. A file added during a license change merges\n"
            "cleanly and keeps the old offer, which is how this went unnoticed once already:",
            file=sys.stderr,
        )
        for problem in header_offenders:
            print(f"  {problem}", file=sys.stderr)
        return 1

    total = sum(1 for _ in roots)
    print(
        f"OK: {total} vendored root(s) license-clean; MulanPSL-2.0 text matches its pin; "
        f"every first-party SPDX header declares {expected!r}."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
