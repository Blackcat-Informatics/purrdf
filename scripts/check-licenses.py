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
import os
import re
import subprocess
import tempfile
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


def _assert_outside_repo(fixture: Path) -> None:
    """Refuse a fixture path inside the repository.

    `tempfile` obeys `TMPDIR`, and `_fixture_repo` runs `git init` in whatever it is given.
    A `TMPDIR` pointing inside the worktree would therefore place a throwaway `.git` in the
    tree this gate judges -- and a nested repository changes what `git ls-files` returns for
    the real one. No tracked write happens today; this is the condition that would make one
    possible, checked rather than assumed.
    """
    root = repo_root().resolve()
    if root in fixture.resolve().parents or fixture.resolve() == root:
        sys.exit(
            f"check-licenses: refusing to build a fixture repository at {fixture} because it "
            f"is inside {root}. Point TMPDIR outside the worktree."
        )


def _fixture_repo(root: Path) -> None:
    """Make *root* a git repository, so `git ls-files` can enumerate a fixture.

    `first_party_header_offenders` walks `git ls-files`, which is the right enumeration for
    the real tree -- it is exactly the set that ships. A fixture therefore has to be a
    repository, and two offline git calls are a smaller price than giving the function a
    second code path that the real run would not take.
    """
    _assert_outside_repo(root)
    for command in (["init", "-q"], ["add", "-A"]):
        subprocess.run(
            ["git", "-C", str(root), *command],
            check=True,
            capture_output=True,
            env={"GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_SYSTEM": "/dev/null", "PATH": os.environ.get("PATH", "")},
        )


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

    # 2. A file at the OLD offer is caught. IN A FIXTURE ROOT, because a gate must not
    #    write the tree it judges. The first version mutated three TRACKED files -- one of
    #    them the root `Cargo.toml`, replacing the workspace licence with "Zlib OR WTFPL"
    #    -- and restored them in a `finally`, which does not run on SIGKILL. On a host whose
    #    background shells are reaped under memory pressure, a killed `make check` left a
    #    32-member workspace declaring the wrong licence: discovered later as a mystery
    #    diff, or published. In the change whose entire subject is not misrepresenting the
    #    offer. Every function here already takes `root` as a parameter, so no mutation was
    #    ever needed.
    with tempfile.TemporaryDirectory(prefix="check-licenses-selftest-") as raw:
        fixture = Path(raw)
        (fixture / "scripts").mkdir()
        (fixture / "scripts" / "probe.py").write_text(
            f"# SPDX-License-Identifier: MIT OR Apache-2.0\n", encoding="utf-8"
        )
        (fixture / "scripts" / "fine.py").write_text(
            f"# SPDX-License-Identifier: {expected}\n", encoding="utf-8"
        )
        _fixture_repo(fixture)
        found = first_party_header_offenders(fixture, expected)
        if any("probe.py" in problem and "MIT OR Apache-2.0'" in problem for problem in found):
            print("OK: self-test — a first-party file left at a narrower offer is refused")
        else:
            print(f"SELF-TEST FAIL: a stale offer was not refused (reported: {found[:2]})")
            ok = False
        if any("fine.py" in problem for problem in found):
            print("SELF-TEST FAIL: a file AT the offer was refused alongside it")
            ok = False

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

    # 7. A PUBLISHED README MUST STATE THE OFFER IN FULL. The gap this closes was named
    #    in two commit messages and closed by neither, and each naming was followed by a
    #    new defect on the same surface: 17 pages with a doubled sentence, then one crate
    #    offering a tri-licensed library under two licences and one publishing no offer at
    #    all. All three shapes execute here.
    # IN A FIXTURE ROOT, for the same reason as case 2: this mutated the tracked
    # `crates/iri/README.md` and relied on a `finally` to put it back.
    with tempfile.TemporaryDirectory(prefix="check-licenses-readme-") as raw:
        fixture = Path(raw)
        crate = fixture / "crates" / "probe"
        crate.mkdir(parents=True)
        (crate / "Cargo.toml").write_text(
            '[package]\nname = "probe"\nreadme = "README.md"\n', encoding="utf-8"
        )
        full = (
            "# probe\n\n## License\n\nLicensed under any one of the following, at your "
            "option:\n\n- MIT license\n- Apache License, Version 2.0\n- Mulan Permissive "
            "Software License, Version 2 (MulanPSL-2.0)\n"
        )
        for mutation, label, needle in (
            (full.replace("## License", "## Notes", 1), "no licence section", "no licence section"),
            (
                full.replace(
                    "- Mulan Permissive Software License, Version 2 (MulanPSL-2.0)\n", "", 1
                ),
                "a proper subset of the offer",
                "proper subset",
            ),
        ):
            (crate / "README.md").write_text(mutation, encoding="utf-8")
            found = published_readme_offenders(fixture, expected)
            if any("crates/probe" in problem and needle in problem for problem in found):
                print(f"OK: self-test — a published README with {label} is refused")
            else:
                print(f"SELF-TEST FAIL: {label} was not refused ({found[:1]})")
                ok = False
        # And the fixture stating the full offer is accepted, so the two refusals above
        # are not satisfied by a rule that refuses every fixture.
        (crate / "README.md").write_text(full, encoding="utf-8")
        if published_readme_offenders(fixture, expected):
            print("SELF-TEST FAIL: a fixture README stating the full offer was refused")
            ok = False
        else:
            print("OK: self-test — a fixture README stating the full offer is accepted")

    # AND THE NEIGHBOUR: every published README as committed is accepted. Without this a
    # rule that refused everything would pass both cases above.
    if published_readme_offenders(root, expected):
        print(
            "SELF-TEST FAIL: the tree as committed has a published README that does not "
            f"state the offer: {published_readme_offenders(root, expected)[:2]}"
        )
        ok = False
    else:
        print("OK: self-test — every published crate README as committed states the full offer")

    # 5. THE EXPRESSION IS READ FROM Cargo.toml, not restated here -- proven by pointing
    #    the reader at a FIXTURE manifest rather than by rewriting the real one. The first
    #    version wrote `license = "Zlib OR WTFPL"` into the root `Cargo.toml` and restored
    #    it in a `finally`; see case 2 for why that is not acceptable in a gate.
    with tempfile.TemporaryDirectory(prefix="check-licenses-expr-") as raw:
        fixture = Path(raw)
        (fixture / "Cargo.toml").write_text(
            '[workspace.package]\nlicense = "Zlib OR WTFPL"\n', encoding="utf-8"
        )
        moved = workspace_license(fixture)
        # And the expectation is a PARAMETER, so the real tree judged against that other
        # expression must offend everywhere -- which is the property the mutation was
        # trying to demonstrate.
        against_other = first_party_header_offenders(root, moved)
        if moved == "Zlib OR WTFPL" and len(against_other) > 100:
            print(
                f"OK: self-test — the expectation follows Cargo.toml ({len(against_other)} "
                f"files offend against {moved!r}), so it is read rather than restated"
            )
        else:
            print(f"SELF-TEST FAIL: the expectation did not follow Cargo.toml (got {moved!r})")
            ok = False

    # 6. A literal-string license parses. `split('"')[1]` raised IndexError here.
    with tempfile.TemporaryDirectory(prefix="check-licenses-toml-") as raw:
        fixture = Path(raw)
        (fixture / "Cargo.toml").write_text(
            "[workspace.package]\nlicense = 'MIT OR Apache-2.0'\n", encoding="utf-8"
        )
        if workspace_license(fixture) == "MIT OR Apache-2.0":
            print("OK: self-test — a TOML literal-string license parses instead of crashing")
        else:
            print("SELF-TEST FAIL: a literal-string license did not parse")
            ok = False

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


# ── A publishable README must state the offer, not a subset of it ─────────────
#
# THE GAP WAS NAMED TWICE AND CLOSED NEITHER TIME, and both times the naming was
# followed immediately by a new defect on the same surface. The first commit said
# "nothing yet gates the human-readable offer, so this is a fix without a guard";
# it shipped a doubled sentence to 17 crates.io pages. The second said "Naming the
# gap did not substitute for having the guard"; it left `crates/rdf-capi` offering
# a tri-licensed crate under two licences, and `crates/retrieval` -- publishable,
# `readme` declared -- with no licence offer at all.
#
# A crate whose `Cargo.toml` names a README is publishing that file to crates.io
# beside three-license metadata. So the README must carry a licence section, and
# that section must name every term of the offer. Naming a PROPER SUBSET is the
# defect that matters: the metadata says three, the page a human reads says two,
# and the page is the one they believe.
# `Licensing` as well as `License`/`Licence`, and a single `#` as well as `##`. The first
# pattern was `^##+\s+Licen[cs]e`, which refused a section headed `## Licensing` -- the name
# of this repository's own LICENSING.md -- and a top-level `# License`.
# `Licensing`/`Licence`/`License`, and the zh-Hans heading, which an English-only pattern
# could not match -- so `README.zh-Hans.md` would be mis-diagnosed as "publishes with no
# licence section" the moment this rule reaches a non-crate README.
LICENCE_HEADING = re.compile(r"^(#{1,6})\s+(?:Licen[cs]|许可|授权)", re.MULTILINE)

# HOW EACH TERM MAY BE WRITTEN. `term.split("-")[0]` reduced the needles to `MIT`,
# `Apache` and `MulanPSL`, so the VERSION was never checked: a section naming
# `Apache-1.1 OR MulanPSL-1.0` passed, and so did one reading "MIT only. NOT offered under
# Apache-2.0 or MulanPSL-2.0" -- a section explicitly REFUSING two of the three, accepted
# as stating all three. A term with no entry here is a hard failure rather than a fallback
# to substring matching, so a future licence cannot be silently unchecked.
# WHITESPACE-TOLERANT, because Markdown wraps. The root README spells its own offer as
# `[Apache License\n2.0]` across a line break, and a pattern with a literal space matched
# neither alias -- so the gate would have false-refused this repository's own landing page
# the moment the rule reached it. `\s+` rather than a space in every multi-word alias.
LICENCE_ALIASES: dict[str, tuple[str, ...]] = {
    "MIT": (r"\bMIT\b",),
    "Apache-2.0": (
        r"Apache-2\.0",
        r"Apache\s+License,?\s+Version\s+2\.0",
        r"Apache\s+License\s+2\.0",
    ),
    "MulanPSL-2.0": (
        r"MulanPSL-2\.0",
        r"Mulan\s+Permissive\s+Software\s+License,?\s+Version\s+2",
    ),
}


def published_readme_offenders(root: Path, expected: str) -> list[str]:
    """Every crate that publishes a README not stating the full licence offer."""
    terms = [term.strip() for term in expected.split(" OR ")]
    offenders: list[str] = []
    for manifest in sorted((root / "crates").glob("*/Cargo.toml")):
        data = tomllib.loads(manifest.read_text(encoding="utf-8"))
        package = data.get("package", {})
        # `publish = []` is cargo's other spelling of "never publish", and it was missed.
        if package.get("publish") in (False, []):
            continue
        readme = manifest.parent / "README.md"
        name = manifest.parent.name
        # WHAT CARGO PUBLISHES, not what the manifest spells out. The first version
        # required an explicit `readme = "README.md"`, and cargo AUTO-DISCOVERS `README.md`
        # when the key is absent -- so `crates/hnsw` and `crates/json` published READMEs
        # with no licence offer at all and the gate could not see either. The gate checked
        # a proxy for the property (an explicit key) instead of the property (does a README
        # reach crates.io), which is the same mistake as reading a self-test's result for a
        # gate's result.
        key = package.get("readme")
        if key is False:
            continue
        if key is None:
            if not readme.is_file():
                continue
        elif key != "README.md":
            continue
        if not readme.is_file():
            offenders.append(f"crates/{name}: publishes README.md and it does not exist")
            continue
        body = readme.read_text(encoding="utf-8")
        heading = LICENCE_HEADING.search(body)
        if heading is None:
            offenders.append(
                f"crates/{name}/README.md: publishes to crates.io with no licence section, "
                f"so the page carries the metadata's offer and no human-readable one"
            )
            continue
        # SCOPED TO THE SECTION, not the whole file. Reading the body let the file's own
        # SPDX header -- which names all three by construction -- satisfy a section that
        # named two, so the subset case the gate exists for could never fire.
        #
        # The section ends at a heading of the SAME OR SHALLOWER depth. `^##+\s` matched
        # `###` too, so a legitimate sub-heading inside a licence section truncated it to
        # nothing -- and the diagnostic then reported all three terms missing, telling the
        # author the opposite of the truth about a page that stated the offer in full.
        depth = len(heading.group(1))
        section = body[heading.end():]
        next_heading = re.search(rf"^#{{1,{depth}}}\s", section, re.MULTILINE)
        if next_heading is not None:
            section = section[: next_heading.start()]
        unknown = [term for term in terms if term not in LICENCE_ALIASES]
        if unknown:
            sys.exit(
                f"check-licenses: no spelling is recorded for licence term(s) {unknown}, so "
                "a README section naming them cannot be checked. Add them to "
                "LICENCE_ALIASES rather than letting the check fall back to a substring."
            )
        missing = [
            term
            for term in terms
            if not any(re.search(alias, section) for alias in LICENCE_ALIASES[term])
        ]
        # WITHDRAWAL IS NOT DETECTABLE BY PRESENCE MATCHING, and the attempt was worse
        # than nothing. A phrase list keyed on `only` / `not offered` / `except under`
        # refused all four of these legitimate tri-licence sentences --
        #
        #   "Pick only one of the three; you do not need to comply with all"
        #   "The vendored W3C corpora are not licensed under these terms"
        #   "No warranty is provided and support is not offered"
        #   "You may not remove the notice except under the terms"
        #
        # -- while missing two of three real withdrawals ("do NOT apply to this crate",
        # "were withdrawn"). `\bonly\b(?=[^.]*\bnot\b)` was the worst of it: `[^.]`
        # matches newlines, so any `only` before any `not` with no intervening period fired.
        #
        # It passed the four neighbours I wrote for it because I chose neighbours that did
        # not contain the trigger words -- a control that cannot distinguish the case it
        # exists for, which is the shape this whole change is about. A rule refusing more
        # good prose than bad makes the gate worse than its absence, so it is deleted
        # rather than tuned. What remains is the checkable claim: every term of the offer
        # must be NAMED in the section.
        if missing:
            offenders.append(
                f"crates/{name}/README.md: its licence section does not name {missing}; the "
                f"crate's metadata offers {expected!r}, so the page states a proper subset "
                f"of what the crate actually offers"
            )
    return offenders


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
    readme_offenders = published_readme_offenders(root, expected)
    if readme_offenders:
        print(
            "License hygiene FAILED: a crate publishes a README that does not state the\n"
            f"full offer {expected!r}:",
            file=sys.stderr,
        )
        for problem in readme_offenders:
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
        f"every first-party SPDX header declares {expected!r}; every published crate "
        f"README states that offer in full."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
