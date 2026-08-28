#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Drift guard for hand-written numbers that restate a generated artifact.

Two documents in this repository already carry machine-generated, drift-guarded
blocks:

  * ``docs/book/src/entailment-rules.md`` — emitted wholesale by
    ``cargo run -p purrdf-entail --example gen_rule_inventory`` from
    ``RuleId`` / ``rules()`` / ``implemented()``, and byte-compared against a
    fresh run by ``scripts/check-generated.sh``.
  * the ``conformance-matrix`` block inside ``docs/CONFORMANCE.md`` — written
    from live harness output by ``scripts/conformance-matrix.py``, which fails
    if the committed block and a fresh full run disagree.

Prose *around* those blocks restates their numbers, and prose is not covered by
either guard. That is exactly how a coverage table came to say ``RDFS 14 / 18``
under a sentence claiming it "cannot fall behind the code": the sentence was
true of the generated block and false of the table above it.

This script closes that hole. Every claim below names a document, the exact
sentence or row it lives in, and the generated artifact it must agree with. A
claim that no longer matches its source is a hard failure naming both values;
a claim whose sentence can no longer be found is *also* a hard failure, so
rewording a row silently drops nothing.

Two of the checks DISCOVER what they cover instead of naming it, because a guard
that names its own scope only guards what someone remembered to register:

  * every rule-coverage table is found by the table's own header row
    (``rule_coverage_table_claims``), wherever it is published;
  * the Python reasoner service table is checked against the set of services the
    type stub declares (``py_service_table_claim``), not against a count.

Both generalizations are load-bearing: ``crates/entail/README.md`` published
``RDF 3 | 1`` and ``RDFS 18 | 14`` while this script read only the book chapter
and printed that every claim agreed.

One check here compares prose against a RULE rather than against a generated
number — ``banned_entailment_overclaims``, the ban on unbounded claims — so
nothing else in this file would notice it answering wrongly, and it did, in both
directions, for every claim that happened to wrap. It therefore SELF-TESTS before
the claims run: ``overclaim_self_test`` injects each banned claim into each swept
unit on one line, wrapped, and wrapped INSIDE the marker the sweep is derived
from, and asserts it is caught every time and exempt every time the scope phrase
is present, wherever the line breaks fall. ``--self-test`` runs that alone.

The set that ban sweeps is DERIVED rather than enumerated, for the same reason the
two checks above discover their scope. It was a hand-written nine-tuple, nothing
asserted the tuple was complete, and the self-test iterated the same tuple — so
cutting it to a single entry left eight documents outside the ban while both the
gate and its preflight printed a green line about the one that remained. The set
is now ``_claim_corpus`` — the Markdown of ``_documented_surface`` plus the
registry descriptions this workspace publishes to crates.io, PyPI and npm —
filtered by the ban's own subject markers, with a coverage arm that names any
document this file knows about which the traversal stops reaching, and a reach arm
that matches every marker-bearing claim across the whole surface so the swept set
is a permission boundary and not a detection boundary.

Membership, the sweep and the reach arm all read ``_reflowed`` text. That is what
makes the derivation total rather than nearly total: the sweep joined wrapped lines
before matching while the other two read raw text, so a claim broken inside its own
subject marker — ``complete OWL`` / ``2 RL entailment`` — joined no set and was
matched by no arm, and the five hand-written wrapped specimens all happened to
break somewhere else. The same held of a whitespace run WITHIN one line, which
the sweep did not normalize at all.

The guards AROUND that ban have the same problem the ban had: a conditional nothing
ever satisfies is the same green light as the check it was written to protect. Seven
levers could be pulled with every gate green — a description field path re-pointed by
one letter, a subject marker made OPTIONAL rather than deleted (which leaves the
literal in the pattern's source, so a substring test passes it), a subject marker made
one BRANCH of an alternation (which a witness that deletes the marker from one specimen
answers correctly and which lets ``complete OWL2 RL entailment`` escape the whole ban),
either of two arms with its walk removed, one arm of the documented-surface walk
dropped, which took that surface from 798 files to 785 without an error, the one scoped
specimen that crosses a line written on one line instead, and the table of mutations
below narrowed to a single entry. Each guard is now a function that takes what it judges
as an argument, and ``mutation_self_test`` hands each the shape the defect really had
and requires it to fail (``_MUTATIONS``, floored so it cannot be narrowed back). It runs
first, on every invocation.

The alternation lever is closed by construction rather than by a guard: a banned claim is
written as PARTS and ``_banned_pattern`` composes the subject marker into the pattern as a
concatenated literal, so a pattern that does not require its marker cannot be spelled, and
the two legitimate ways to broaden the ban — a second entry with its own marker, or a
marker re-pointed at a literal both spellings share — are written out where the table is.

It is pure text-over-committed-files: no cargo, no network, no test run. The
expensive gates prove the generated artifacts are current; this one proves the
prose agrees with them. Run standalone, or as part of
``scripts/check-generated.sh`` (and therefore ``make check``).
"""

from __future__ import annotations

import hashlib
import json
import re
import string
import sys
import tomllib
from collections.abc import Callable
from dataclasses import dataclass, field
from functools import lru_cache
from pathlib import Path

_REPO = Path(__file__).resolve().parent.parent

_INVENTORY = _REPO / "docs" / "book" / "src" / "entailment-rules.md"
_CONFORMANCE = _REPO / "docs" / "CONFORMANCE.md"
_ENTAILMENT = _REPO / "docs" / "book" / "src" / "entailment.md"
_BOOK_CONFORMANCE = _REPO / "docs" / "book" / "src" / "project" / "conformance.md"
_README = _REPO / "README.md"
# The three CRATE READMEs that state the 78-rule table. They are published to
# crates.io on their own, so a reader may meet the number there and nowhere else;
# each therefore carries the qualifier, and each is gated here. This script did
# not cover crate READMEs before, which is how all three came to state rule-table
# coverage as though it were entailment conformance. Their rule-COVERAGE tables
# need no entry: those are discovered by header row, not by path.
_ENTAIL_README = _REPO / "crates" / "entail" / "README.md"
_PURRDF_README = _REPO / "crates" / "purrdf" / "README.md"
_CLI_README = _REPO / "crates" / "cli" / "README.md"
# The PyPI front page. It is the highest-traffic non-Rust surface and states the
# rule-table numbers, the OWL 2 RL lane split, and the reasoner's service set;
# none of it was gated here, which is how it came to claim four missing RDFS
# rules and a refusal for two regimes that both materialize.
_PY_README = _REPO / "bindings" / "python" / "README.md"
_PY_STUB = _REPO / "bindings" / "python" / "python" / "src" / "purrdf" / "__init__.pyi"
_RELEASE = _REPO / "docs" / "RELEASE.md"
_AGENTS = _REPO / "AGENTS.md"
_RELEASE_CRATES = _REPO / "scripts" / "release-crates.sh"

_GOVERNOR_PROFILE = _REPO / "docs" / "SPARQL-GOVERNOR-PROFILE.md"
_GOVERNOR_MANIFEST = _REPO / "vectors" / "sparql-governors" / "manifest.tsv"
_GOVERNOR_SOURCE = _REPO / "crates" / "sparql-eval" / "src" / "governor" / "mod.rs"

_INTRODUCTION = _REPO / "docs" / "book" / "src" / "introduction.md"
_RL_SUITE = _REPO / "crates" / "sparql-conformance" / "entailment-suite" / "w3c-owl2-rl"
_CENSUS = _RL_SUITE / "census.tsv"
_RL_LEDGER = _REPO / "crates" / "sparql-conformance" / "src" / "owl2_rl.rs"
# The harness test that pins `OWL2-RL-MECHANISMS` verbatim. The line is RECOMPUTED
# from the corpus on every run and the assertion fails if it moved, so the pinned
# string is a measurement this script may quote — unlike prose, which is a number
# someone typed a second time.
_RL_MECHANISM_PIN = (
    _REPO / "crates" / "sparql-conformance" / "tests" / "owl2_rl_conformance.rs"
)

# The first-party `purrdf-extend` manifest. `docs/CONFORMANCE.md`'s SPARQL
# 1.1/1.2 scoreboard row hand-counts three of its case families in prose
# ("nine temporal", "six LATERAL", "eight SEP-0007" as originally written) —
# each a restatement of `mf:entries`'s own list, and none of the three was
# derived from it, which is how the SEP-0007 figure fell one short of the
# nine cases actually shipped (:existsScopeProjectionViolation went uncounted).
_EXTEND_MANIFEST = (
    _REPO / "crates" / "sparql-conformance" / "suite" / "purrdf-extend" / "manifest.ttl"
)

_MATRIX_BEGIN = "<!-- BEGIN GENERATED: conformance-matrix -->"
_MATRIX_END = "<!-- END GENERATED: conformance-matrix -->"

# The matrix row name emitted by scripts/conformance-matrix.py for the OWL 2 RL
# entailment lane. Kept as a constant because several claims key off it.
_RL_SUITE_ROW = "Entailment (OWL 2 RL, W3C entailment tests)"


# The self-test's ONLY injection point: a document's text, substituted for the committed one
# while one mutation runs. Empty on every ordinary run, so the gate reads the tree and nothing
# else; an injected claim is written to a STRING and never to a tracked file.
_OVERLAY: dict[str, str] = {}


@lru_cache(maxsize=None)
def _committed(path: Path) -> str:
    """`path` as it stands on disk.

    Cached because this gate never writes: a file's committed text is the same string for
    the whole run, and the derived sweep is re-derived once per injected sentence to prove
    it re-derives. Reading the tree that many times cost more than every check in this file
    put together. The self-test's injections do NOT go through here — they go through the
    overlay in ``_read``, which is consulted first and never cached.
    """
    return path.read_text(encoding="utf-8")


def _read(path: Path) -> str:
    if str(path) in _OVERLAY:
        return _OVERLAY[str(path)]
    return _committed(path)


def _int(text: str) -> int:
    """Parse a documented count, tolerating the separator and spelling prose uses.

    A sentence that OPENS on a count spells it (`Fifteen of those 50`), so a
    reader that only accepted digits would leave exactly those sentences
    ungated. The word forms are `_SPELLED` read backwards, so there is one
    table rather than two that can fall out of step.
    """
    cleaned = text.replace(",", "").replace(" ", "")
    spelled = _CARDINAL.get(cleaned.lower())
    return spelled if spelled is not None else int(cleaned)


# A run of whitespace, and the one thing every arm of the entailment-overclaim ban does to
# text before it looks at it. The ban's patterns spell their subjects with SINGLE SPACES
# (`OWL 2 RL entailment`), so a paragraph reflow that moves the line break INSIDE one of
# those spaces must not change any arm's answer. It did: `_overclaims_in` joined wrapped
# lines before matching while the membership test and the reach arm read raw text, so
# `complete OWL` / `2 RL entailment` across a wrap was a claim the sweep would have caught
# in a document that never joined the swept set and that no other arm was reading either.
_WHITESPACE = re.compile(r"\s+")

# Big enough for the whole prose corpus and every manifest, plus the injected variants one
# self-test run produces, so the derived sweep can be re-derived once per injected sentence
# — which is the point of it — without re-scanning units whose text has not changed.
_MARKER_CACHE = 256


def _reflowed(text: str) -> str:
    """`text` with every whitespace run collapsed to one space.

    The shared definition of what an OCCURRENCE is. Membership in the swept set, the
    sentence sweep and the reach arm all read text through this, so all three agree, and
    the derivation's guarantee — a unit that carries a banned claim carries that claim's
    subject marker — holds at every wrap point rather than at the ones nobody tried.
    """
    return _WHITESPACE.sub(" ", text)


def _reflowed_stripped(text: str) -> str:
    """``_reflowed(text).strip()``, by the faster route.

    ``str.split()`` with no argument splits on exactly the runs ``\\s+`` matches — both ask
    ``Py_UNICODE_ISSPACE`` — so this is the same normalization, not a second one. It is
    spelled separately because the sweep calls it once per LINE of every document it reads,
    where the regex cost is the largest single line item in this gate, and
    :func:`_check_specimens` asserts the two agree on every specimen form rather than
    leaving "the same normalization" as a claim in a comment.
    """
    return " ".join(text.split())


def _reflowed_offsets(text: str) -> list[int]:
    """For each character of ``_reflowed(text)``, its offset in `text`.

    Built only when a reach-arm hit has to be REPORTED with a line number, because it is
    the one per-character mapping in this file and the reach arm walks the whole documented
    surface. A passing run never calls it.
    """
    offsets: list[int] = []
    cursor = 0
    for run in _WHITESPACE.finditer(text):
        offsets.extend(range(cursor, run.start()))
        offsets.append(run.start())
        cursor = run.end()
    offsets.extend(range(cursor, len(text)))
    return offsets


def _reflowed_line(text: str, index: int) -> int:
    """The 1-based line of `text` holding the character at ``_reflowed(text)`` offset `index`."""
    offsets = _reflowed_offsets(text)
    raw = offsets[index] if index < len(offsets) else len(text)
    return text.count("\n", 0, raw) + 1


# ---------------------------------------------------------------------------
# Source 1 — the generated entailment rule inventory
# ---------------------------------------------------------------------------


def load_rule_inventory() -> dict[str, tuple[int, int]]:
    """Regime -> (defined, implemented), read from the generated inventory.

    The inventory is byte-identical to a fresh ``gen_rule_inventory`` run (that
    is what ``check-generated.sh`` asserts immediately before calling this
    script), so reading it here is equivalent to reading ``rules()`` and
    ``implemented()`` without paying for a cargo build.
    """
    text = _read(_INVENTORY)
    section = re.search(
        r"## Coverage by regime\n(.*?)(?:\n## |\Z)", text, re.DOTALL
    )
    if not section:
        raise SystemExit(
            f"check-doc-claims: no 'Coverage by regime' table in "
            f"{_INVENTORY.relative_to(_REPO)}"
        )
    rows = re.findall(
        r"^\| ([A-Za-z-]+) \| `[a-z-]+` \| (\d+) \| (\d+) \|$",
        section.group(1),
        re.MULTILINE,
    )
    if not rows:
        raise SystemExit(
            f"check-doc-claims: could not parse any regime row out of "
            f"{_INVENTORY.relative_to(_REPO)}"
        )
    return {name: (int(defined), int(impl)) for name, defined, impl in rows}


_COMPAT_LEDGER = _REPO / "bindings" / "python" / "tests" / "xfail_ledger.toml"
_RDFLIB_LEDGER = (
    _REPO / "bindings" / "python" / "tests" / "rdflib_suite" / "xfail_ledger.toml"
)


def load_xfail_ledger_sizes() -> dict[str, int]:
    """How many strict xfails each Python ledger actually holds.

    The scoreboard rows carrying these numbers were already gated; the PROSE naming
    the ledger files was not, and drifted to 5 and 24 against real values of 4 and 1
    — inside the same document, a few lines from the correct figures. Deriving both
    from the TOML makes the two statements one measurement.
    """
    sizes: dict[str, int] = {}
    for key, path in (("compat", _COMPAT_LEDGER), ("rdflib", _RDFLIB_LEDGER)):
        if not path.is_file():
            raise SystemExit(
                f"check-doc-claims: {path.relative_to(_REPO)} is missing; the ledger "
                f"prose claim cannot be checked, so do not leave it unchecked"
            )
        with path.open("rb") as handle:
            entries = tomllib.load(handle).get("xfail", {})
        sizes[key] = len(entries)
    return sizes


def load_rule_extensions() -> list[str]:
    """Every rule this workspace fires that no specification table states.

    Read from the same generated inventory as ``load_rule_inventory``, so the two
    cannot disagree about what is normative. An empty result is legitimate — a
    build that extends nothing — and the disclosure claim below is then vacuous
    rather than failing.
    """
    text = _read(_INVENTORY)
    section = re.search(r"\n## Extensions\n(.*?)(?:\n## |\Z)", text, re.DOTALL)
    if not section:
        raise SystemExit(
            f"check-doc-claims: no '## Extensions' section in "
            f"{_INVENTORY.relative_to(_REPO)}; the generator always emits one "
            f"(empty when nothing is extended), so a missing section means the "
            f"generator changed shape and this claim would silently pass"
        )
    return re.findall(
        r"^\| [A-Za-z-]+ \| `[a-z-]+` \| `([a-z0-9-]+)` \|$",
        section.group(1),
        re.MULTILINE,
    )


def load_projection_profile_count() -> int:
    """How many projection profiles the carrier enum defines."""
    source = _read(_REPO / "crates" / "rdf" / "src" / "projections" / "carrier.rs")
    body = re.search(r"pub enum ProjectionProfile \{(.*?)\n\}", source, re.DOTALL)
    if not body:
        raise SystemExit(
            "check-doc-claims: cannot find ProjectionProfile in carrier.rs; the profile-count "
            "claim cannot be checked, so do not leave it unchecked"
        )
    count = len(re.findall(r"^\s{4}[A-Z][A-Za-z0-9]*,", body.group(1), re.MULTILINE))
    if count == 0:
        raise SystemExit("check-doc-claims: ProjectionProfile parsed to zero variants")
    return count


def load_never_published() -> list[str]:
    """The workspace members whose manifests say `publish = false`, by crate name."""
    names: list[str] = []
    for manifest in sorted(_REPO.glob("crates/*/Cargo.toml")) + sorted(
        _REPO.glob("bindings/*/Cargo.toml")
    ):
        text = _read(manifest)
        if re.search(r"^publish\s*=\s*false", text, re.MULTILINE):
            name = re.search(r'^name\s*=\s*"([^"]+)"', text, re.MULTILINE)
            if name:
                names.append(name.group(1))
    if not names:
        raise SystemExit(
            "check-doc-claims: found no publish=false manifest; the never-published claim "
            "cannot be checked, so do not leave it unchecked"
        )
    return sorted(names)


# Case-id prefix -> the family name the prose hand-counts it under. Order matters:
# `notExists` is checked before `exists` would ever be tried against it, but since
# no id starts with both, checked in listed order with the first match winning.
# A prefix here is deliberately a case-ID convention, not a `mf:` type: SEP-0007
# ships both `mf:QueryEvaluationTest` and `mf:NegativeSyntaxTest` cases under one
# family, so counting by RDF type would split what the prose counts as one figure.
_EXTEND_FAMILY_PREFIXES: tuple[tuple[str, str], ...] = (
    ("temporal", "temporal"),
    ("lateral", "LATERAL"),
    ("notExists", "SEP-0007"),
    ("exists", "SEP-0007"),
)


def load_extend_manifest_family_counts() -> dict[str, int]:
    """How many `purrdf-extend` manifest cases each hand-counted family actually holds.

    Counted from `mf:entries`'s own list — the manifest's real structure, not a
    hand-maintained tally — by each entry id's own prefix (`_EXTEND_FAMILY_PREFIXES`),
    so a case added to or removed from the manifest changes this count without
    anyone updating a second number to match. This is the ground truth
    `docs/CONFORMANCE.md`'s prose restates; see `_EXTEND_MANIFEST`'s own comment
    for the drift this closes.
    """
    text = _read(_EXTEND_MANIFEST)
    entries_block = re.search(r"mf:entries\s*\((.*?)\)\s*\.", text, re.DOTALL)
    if not entries_block:
        raise SystemExit(
            f"check-doc-claims: no 'mf:entries ( ... )' list in "
            f"{_EXTEND_MANIFEST.relative_to(_REPO)}; the family-count claim "
            f"cannot be checked, so do not leave it unchecked"
        )
    ids = re.findall(r":([A-Za-z0-9]+)", entries_block.group(1))
    if not ids:
        raise SystemExit(
            f"check-doc-claims: the mf:entries list in "
            f"{_EXTEND_MANIFEST.relative_to(_REPO)} parsed to zero case ids"
        )
    # Ids outside all three prefixes (VERSION/AGG/base-direction/etc.) are not
    # hand-counted anywhere in prose, so they are simply not tallied — this
    # function's contract is the three families it names, not every id.
    counts: dict[str, int] = {family: 0 for _, family in _EXTEND_FAMILY_PREFIXES}
    for entry_id in ids:
        for prefix, family in _EXTEND_FAMILY_PREFIXES:
            if entry_id.startswith(prefix):
                counts[family] += 1
                break
    return counts


_SPELLED = {
    1: "one", 2: "two", 3: "three", 4: "four", 5: "five", 6: "six", 7: "seven",
    8: "eight", 9: "nine", 10: "ten", 11: "eleven", 12: "twelve", 13: "thirteen",
    14: "fourteen", 15: "fifteen", 16: "sixteen", 17: "seventeen", 18: "eighteen",
}

# The same table read backwards, for `_int`. Derived rather than written out, so a
# cardinal added above is one both directions gain at once.
_CARDINAL = {word: value for value, word in _SPELLED.items()}


def never_published_claim() -> list[str]:
    """The documents naming the never-published set must name ALL of it, spelled right.

    `publish = false` is the ground truth, and two documents restate it. Both once said
    "two" while four manifests carried the flag — the flag had grown and the prose had
    not, because nothing derived one from the other.
    """
    problems: list[str] = []
    names = load_never_published()
    spelled = _SPELLED.get(len(names))
    for path in (_REPO / "docs" / "book" / "src" / "project" / "releases.md", _AGENTS):
        if not path.is_file():
            continue
        text = _read(path)
        rel = path.relative_to(_REPO)
        for crate in names:
            if crate not in text:
                problems.append(
                    f"{rel}: names the never-published set but omits `{crate}`, whose "
                    f"manifest says `publish = false`"
                )
        stated = re.search(
            r"(?:^|\s)([A-Z]?[a-z]+) (?:workspace )?(?:crates|members) (?:are deliberately never published|never reach)",
            text,
        )
        if stated and spelled and stated.group(1).lower() != spelled:
            problems.append(
                f"{rel}: says `{stated.group(0).strip()}`, but {len(names)} manifests "
                f"({', '.join(names)}) say `publish = false`"
            )
    return problems


def profile_count_claim() -> list[str]:
    """Any 'All <word> profiles' sentence must equal the carrier enum's variant count."""
    problems: list[str] = []
    count = load_projection_profile_count()
    spelled = _SPELLED.get(count)
    for path in (_REPO / "crates" / "rdf" / "README.md",):
        text = _read(path)
        rel = path.relative_to(_REPO)
        found = re.findall(r"All ([a-z]+) profiles", text)
        if not found:
            problems.append(
                f"{rel}: no 'All <word> profiles' sentence found — the profile-count claim "
                f"was reworded or removed; update the pattern so it stays checked"
            )
        for word in found:
            if spelled and word != spelled:
                problems.append(
                    f"{rel}: says 'All {word} profiles', but ProjectionProfile has "
                    f"{count} variants ({spelled})"
                )
    return problems


def program_regime_dts_claim() -> list[str]:
    """index.d.ts's 'needs an extra INPUT' sentence must match PROGRAM_REGIME_NAMES."""
    problems: list[str] = []
    boundary = _read(_REPO / "crates" / "validate" / "src" / "regime.rs")
    program = re.search(
        r"PROGRAM_REGIME_NAMES: \[&str; (\d+)\] = \[(.*?)\]", boundary, re.DOTALL
    )
    if not program:
        raise SystemExit(
            "check-doc-claims: cannot read PROGRAM_REGIME_NAMES; the program-regime claim "
            "cannot be checked, so do not leave it unchecked"
        )
    count = int(program.group(1))
    names = re.findall(r'"([a-z-]+)"', program.group(2))
    dts = _REPO / "crates" / "rdf-wasm" / "js" / "index.d.ts"
    text = _read(dts)
    rel = dts.relative_to(_REPO)
    spelled = _SPELLED.get(count, str(count))
    sentence = re.search(r"([A-Z][a-z]+) of them(?:[^.]*?)extra INPUT", text)
    if not sentence:
        problems.append(
            f"{rel}: no '<word> of them ... extra INPUT' sentence found — the "
            f"program-regime claim was reworded; update the pattern so it stays checked"
        )
    elif sentence.group(1).lower() != spelled:
        problems.append(
            f"{rel}: says `{sentence.group(1)} of them` need a program, but "
            f"PROGRAM_REGIME_NAMES has {count} ({', '.join(names)})"
        )
    else:
        for name in names:
            if f'`"{name}"`' not in text:
                problems.append(
                    f"{rel}: the program-taking regime `{name}` is never named"
                )
    return problems


# This script names both superseded fragment spellings and every banned overclaim in its
# own docstrings and patterns, because that is where the bans are DEFINED. It is therefore
# the one file both bans must skip: a ban that fails on its own definition is a ban nobody
# can write down.
_GATE_SCRIPT = Path(__file__).resolve()

# Directories that are not this repository's documented surface: build output, a Python
# virtualenv that may or may not exist on a given machine, and vendored JavaScript. They
# are skipped by PATH SEGMENT rather than by substring so a crate legitimately named
# `pkg-something` is not silently dropped.
_UNDOCUMENTED_SEGMENTS = frozenset({"pkg", "node_modules", ".venv", "target"})

# The ratchet baseline the conformance harness writes, and one of the two files that put
# `scripts` on the documented surface in the first place: it restates the fragment name in
# its own prose, which is where a superseded spelling once regressed unseen.
_CONFORMANCE_BASELINE = _REPO / "scripts" / "conformance-baseline.json"

# One file per ARM of the walk below, each of which must still be reached. An arm is a root
# or a suffix, and a dropped arm narrows both bans at once while every count in this gate
# stays plausible: removing `scripts` took the surface from 798 files to 785 with the gate
# green, because a walk that returns fewer files returns no error. The coverage arm catches a
# dropped ROOT — its registered documents live under `crates`, `bindings` and `docs` — and
# caught nothing at all for `scripts`, which holds no Markdown.
#
# Landmarks rather than counts: a file this gate already depends on by name, so the failure
# says which arm stopped running rather than that a number moved.
_SURFACE_LANDMARKS: tuple[tuple[Path, str], ...] = (
    (_ENTAIL_README, "the `crates` root"),
    (_PY_README, "the `bindings` root"),
    (_ENTAILMENT, "the `docs` root"),
    (_GATE_SCRIPT, "the `scripts` arm's `.py` suffix"),
    (_CONFORMANCE_BASELINE, "the `scripts` arm's `.json` suffix"),
    (_AGENTS, "the repository-root Markdown arm"),
)


def _check_surface_landmarks(found: set[Path]) -> None:
    """Every arm of the documented-surface walk must still reach its landmark.

    Takes the walk's result rather than performing it, so :data:`_MUTATIONS` can hand it a
    narrowed surface and require it to fail.
    """
    for path, arm in _SURFACE_LANDMARKS:
        if path not in found:
            raise SystemExit(
                f"check-doc-claims: the documented surface no longer reaches "
                f"{path.relative_to(_REPO)}, so {arm} contributes nothing. Both bans walk "
                f"this one traversal, so an arm that stops running narrows the stale-name "
                f"ban and the entailment-overclaim ban's reach together, silently and by "
                f"however many files that arm carried. Restore the arm, or drop it "
                f"deliberately in the same commit and say why"
            )


def _documented_surface() -> list[Path]:
    """Every file in this repository's own documented surface, in path order.

    ONE traversal, shared by both bans below, so neither can be narrowed without
    narrowing the other and neither can drift into covering a different tree than it
    claims — and each ARM of it must reach a named landmark
    (:func:`_check_surface_landmarks`), because a walk that stops running one arm returns
    fewer files rather than an error. It covers ``crates``, ``bindings`` and ``docs``
    prose, ``scripts``' own
    ``.py``/``.json`` — the conformance harness and its ratchet baseline restate the
    fragment name in their own prose, and a superseded spelling regressed there silently
    until this walk was extended to include it — and every Markdown file at the
    repository root, which is where the contract (``AGENTS.md``), the release notes
    (``CHANGELOG.md``) and the provenance and licensing statements live.

    What it does NOT cover is stated here rather than left to be discovered, because both
    bans below describe their reach as total over this walk and "total" has to name the
    thing it is total over:

      * REGISTRY METADATA is covered, but not by this walk — a manifest is not prose, and
        only one of its fields is published. See :func:`_registry_prose`, which the
        entailment-overclaim ban sweeps alongside this walk's Markdown.
      * ``.html`` is deliberately out. Two HTML files are tracked (one vendored RO-Crate
        fixture and the playground shell), and ``docs/book/book`` holds hundreds more that
        are mdBook OUTPUT — a re-rendering of Markdown this walk already reads. Including
        the suffix would sweep the same prose twice and make the gate's answer depend on
        whether ``make book`` had been run on the machine. The suffix set is what keeps
        this walk independent of the build.
      * ``.sh``, ``.yml`` and the ``Makefile`` are deliberately out: their comments are build
        mechanics addressed to whoever edits the build, published to no reader. The
        ``scripts`` arm takes ``.py``/``.json`` because those two carry the conformance
        harness's own PROSE about the fragment and the corpus, which regressed once.
      * The many ``.ttl``/``.rq``/``.srx`` fixture suffixes are corpus data, not
        statements this project makes.
    """
    found: list[Path] = []
    for root in ("crates", "bindings", "docs"):
        for path in sorted((_REPO / root).rglob("*")):
            if path.suffix not in {".rs", ".md", ".pyi", ".mjs", ".ts"}:
                continue
            if _UNDOCUMENTED_SEGMENTS.intersection(path.parts):
                continue
            found.append(path)
    for path in sorted((_REPO / "scripts").rglob("*")):
        if path.suffix in {".py", ".json"}:
            found.append(path)
    found.extend(sorted(_REPO.glob("*.md")))
    # Subsumes "the walk returned nothing": every arm must reach its landmark, so a walk
    # that returns nothing fails by naming the first arm that stopped running rather than
    # by reporting a zero that says nothing about which arm produced it.
    _check_surface_landmarks(set(found))
    return sorted(found)


# The manifest file names that carry registry metadata, and the table path to the one field
# each of them PUBLISHES as prose. Three registries, because this repository releases to
# three: crates.io, PyPI and npm.
#
# A field path is a bare tuple, and a tuple agrees with nothing on its own: re-pointed by
# ONE LETTER — `("package", "descriptions")` — every `Cargo.toml` yields nothing, all 23
# crates.io descriptions leave both the swept set and the reach arm, and the release-lane
# cross-check below does not notice because it counts ENTRIES rather than what they read. So
# each declared kind is required to yield, and to yield PROSE; see
# :func:`_check_registry_yield`.
_REGISTRY_DESCRIPTION: tuple[tuple[str, tuple[str, ...]], ...] = (
    ("Cargo.toml", ("package", "description")),
    ("pyproject.toml", ("project", "description")),
    ("package.json", ("description",)),
)

# A value with no whitespace in it: a crate name, a version, an SPDX expression, a URL.
# Every OTHER field of the three tables above holds one of those, and every description this
# workspace publishes is a sentence — so this is what separates "the field a registry
# renders as prose" from "some neighbouring field the path landed on instead".
_BARE_IDENTIFIER = re.compile(r"^\S+$")


@lru_cache(maxsize=1)
def _registry_manifest_paths() -> tuple[Path, ...]:
    """Every manifest in this workspace that carries registry metadata, in path order.

    Memoized because it is a pure function of the tree's LAYOUT and the walk is a second
    pass over the same two trees :func:`_documented_surface` reads. Contents are never
    cached: every reader goes back through ``_read``, so the self-test's overlay is seen.

    Cross-checked against the workspace's own ``members`` list, which is the ground truth
    for what reaches crates.io. A member whose manifest this walk does not reach is a
    narrowed traversal, and it fails here by name rather than by silently publishing an
    ungated description.
    """
    # How many registries this repository actually publishes to, read off the release
    # lanes rather than counted here. The lanes are maintained for an entirely different
    # reason, so dropping a manifest kind from `_REGISTRY_DESCRIPTION` — which would also
    # stop this walk from LOOKING for that kind, and so could never notice its own gap —
    # fails against them instead. A fourth lane fails this on the day it is added.
    lanes = sorted(
        path.name for path in (_REPO / ".github" / "workflows").glob("release-*.y*ml")
    )
    if len(_REGISTRY_DESCRIPTION) < len(lanes):
        raise SystemExit(
            f"check-doc-claims: this repository has {len(lanes)} release lane(s) ({lanes}) "
            f"but only {len(_REGISTRY_DESCRIPTION)} registry description field(s) are "
            f"declared, so at least one registry's front page is published ungated. Add "
            f"the manifest and the field it publishes"
        )
    names = sorted({name for name, _ in _REGISTRY_DESCRIPTION})
    found: list[Path] = []
    for root in ("crates", "bindings"):
        for name in names:
            for path in (_REPO / root).rglob(name):
                if not _UNDOCUMENTED_SEGMENTS.intersection(path.parts):
                    found.append(path)
    found.extend(_REPO / name for name in names if (_REPO / name).is_file())
    reached = set(found)
    members = (
        tomllib.loads(_read(_REPO / "Cargo.toml")).get("workspace", {}).get("members", [])
    )
    if not members:
        raise SystemExit(
            "check-doc-claims: the root manifest lists no workspace members, so the "
            "registry-metadata walk has nothing to check itself against"
        )
    missing = sorted(
        member for member in members if _REPO / member / "Cargo.toml" not in reached
    )
    if missing:
        raise SystemExit(
            f"check-doc-claims: the registry-metadata walk does not reach the manifest of "
            f"workspace member(s) {missing}, whose descriptions are published to crates.io. "
            f"The traversal was narrowed; restore it rather than leaving a crate's front "
            f"page outside the entailment-overclaim ban"
        )
    return tuple(sorted(found))


@lru_cache(maxsize=_MARKER_CACHE)
def _manifest_data(name: str, text: str) -> object:
    """`text` parsed as the manifest `name` — TOML, or JSON for ``package.json``.

    Keyed on the TEXT, so the self-test's overlaid manifest is a different key and is
    parsed for real rather than served from the committed one. Callers READ the result and
    never mutate it; a mutation would be shared, and there is nothing here to mutate.
    """
    return json.loads(text) if name.endswith(".json") else tomllib.loads(text)


def _check_registry_yield(harvest: dict[str, list[tuple[str, str]]]) -> None:
    """Every declared manifest KIND must publish a description, and it must read as prose.

    The field path beside each manifest name in ``_REGISTRY_DESCRIPTION`` is a bare tuple,
    and until this check existed nothing compared it with the manifests it reads. Re-pointed
    by one letter — ``("package", "descriptions")`` — every ``Cargo.toml`` yielded nothing:
    all 23 crates.io descriptions left the swept set AND the reach arm in one edit, with
    every gate green, because the release-lane cross-check counts declared ENTRIES rather
    than what they read and the emptiness test below only fired when all three kinds went
    dark at once.

    So the floor is PER KIND, which is the granularity the defect had. And the yielded text
    must read as prose, because re-pointing the same path at ``("package", "name")`` yields
    something for every manifest — the crate's identifier — and a gate that then swept 23
    identifiers for banned sentences would be just as dark while looking twice as busy.

    Takes the harvest rather than reading the tree, so :data:`_MUTATIONS` can hand it the
    two shapes the defect took and require this to fail.
    """
    for name, keys in _REGISTRY_DESCRIPTION:
        field = ".".join(keys)
        published = harvest.get(name, [])
        if not published:
            raise SystemExit(
                f"check-doc-claims: no `{name}` in this workspace yields a `{field}`, so "
                f"every description that manifest kind publishes is outside the "
                f"entailment-overclaim ban — neither swept nor reachable — while the ban "
                f"still reports a green line about the kinds that remain. Re-point the "
                f"field at what the manifest really spells, or drop the kind and say why "
                f"in the same commit"
            )
        for label, prose in published:
            if _BARE_IDENTIFIER.match(prose.strip()):
                raise SystemExit(
                    f"check-doc-claims: {label} reads {prose.strip()!r}, which is an "
                    f"identifier rather than the sentence a registry renders. The "
                    f"`{field}` path is pointing at a neighbouring field — a name, a "
                    f"version, a licence, a URL — so the ban would sweep identifiers and "
                    f"never see the prose it exists to read"
                )


def _registry_prose() -> list[tuple[str, str]]:
    """``(label, prose)`` for every registry description this workspace publishes.

    A crate's ``description`` is the first sentence a reader meets on crates.io — above the
    README, and alone in every search result — and ``project.description`` and npm's
    ``description`` are the same sentence on PyPI and npm. That is precisely the
    highest-traffic-front-page argument this script already uses to gate
    ``bindings/python/README.md``, so the metadata beside that README cannot sit outside
    the ban: ``crates/entail/Cargo.toml`` carries the ``owl 2`` subject marker today, and
    until this walk existed it was neither swept nor reachable.

    DESCRIPTIONS, not whole manifests, and that boundary is the decision rather than a
    convenience. What a registry renders is the description; a manifest's COMMENTS are
    addressed to whoever edits the build and are published nowhere, so sweeping them would
    put the ban in the position of refusing ``crates/sparql-conformance/Cargo.toml``'s
    comment explaining what the OWL 2 RL lane renders — prose ABOUT the ban's subject
    rather than a claim. Keywords and categories are published too, but they are a
    controlled vocabulary rather than sentences, and ``_sentences`` has nothing to say
    about them.

    No TOML writer and no new dependency: ``tomllib`` and ``json`` are both standard
    library, and this repository's Python floor is 3.13.

    What each declared manifest KIND yielded is kept as it is read and handed to
    :func:`_check_registry_yield`, which is where "the field path is the one a registry
    renders" stops being a comment beside a tuple.
    """
    units: list[tuple[str, str]] = []
    harvest: dict[str, list[tuple[str, str]]] = {}
    for path in _registry_manifest_paths():
        table = next(keys for name, keys in _REGISTRY_DESCRIPTION if name == path.name)
        data = _manifest_data(path.name, _read(path))
        for key in table:
            data = data.get(key, {}) if isinstance(data, dict) else {}
        if isinstance(data, str) and data.strip():
            unit = (f"{path.relative_to(_REPO)} [{'.'.join(table)}]", data)
            units.append(unit)
            harvest.setdefault(path.name, []).append(unit)
    # Subsumes "no description at all was read": a kind that yields nothing fails by NAME,
    # and all three going dark is that failure three times rather than a different one.
    _check_registry_yield(harvest)
    return units


def banned_stale_fragment_names(surface: list[Path]) -> tuple[list[str], int]:
    """The DL fragment has ONE published name; its two superseded spellings are banned.

    The decision core was published as ALCOIQ on nineteen sites and ALCHOIQ in the
    oracle, and both understated what the code decides. The settled name is SHOIQ(D);
    a superseded spelling reappearing anywhere in the documented surface is a
    regression to the two-name state, caught here by name.

    Walks the shared :func:`_documented_surface`. Returns the problems and the number of
    files scanned, so the script's claim count reports what it really read.
    """
    problems: list[str] = []
    checked = 0
    for path in surface:
        if path == _GATE_SCRIPT:
            continue
        checked += 1
        text = _read(path)
        for match in re.finditer(r"\bALCH?OIQ\b", text):
            line = text.count("\n", 0, match.start()) + 1
            problems.append(
                f"{path.relative_to(_REPO)}:{line}: superseded fragment spelling "
                f"`{match.group(0)}` — the decision core's one published name is "
                f"SHOIQ(D)"
            )
    return problems, checked


# The literal phrase that SCOPES a claim to the corpus it was measured on. A sentence
# carrying it is making a bounded statement — "50 / 50 on this vendored W3C corpus" — and
# is exempt; one without it is making the unbounded statement the ban is about.
_CORPUS_SCOPE = "on this vendored W3C corpus"

# Each entry is (before, subject marker, after, why it is banned) — PARTS rather than a
# finished pattern, and that is the whole point rather than a style choice. `_banned_pattern`
# composes them as `(?:before)marker(?:after)`, so a pattern that does not REQUIRE its marker
# cannot be spelled here: the marker is a literal in a top-level concatenation of three atoms,
# and neither half can reach around it because each must be a regular expression on its own
# before it is wrapped.
#
# The patterns are deliberately narrow: they name the specific unbounded claim rather than the
# words it is built from, so "complete" and "full" remain writable about the things they are
# true of — a complete RULE TABLE, a full closure of one document — and only the sentence that
# promotes them into a claim about a SPECIFICATION is caught.
#
# The SUBJECT MARKER is the lower-cased literal a unit must already contain before the
# pattern beside it can possibly match, and it is what DERIVES the swept set below. It is
# not a judgement about which documents "carry the entailment story" — that judgement is
# what let the set be hand-written, and a hand-written set can be narrowed. It is read off
# the pattern itself: `complete OWL 2 RL entailment` cannot appear in a unit that never
# writes `OWL 2`, so sweeping the units that write `OWL 2` is total for that pattern rather
# than merely generous.
#
# TO BAN A SECOND SPELLING of a claim already here — `OWL2` beside `OWL 2` — there are two
# correct edits and one wrong one. Correct: add a SECOND ENTRY with its own marker (`owl2`)
# and its own specimen, so the new spelling derives its own swept set; or RE-POINT this
# entry's marker at a literal both spellings share (`rl entailment`) and put the alternation
# in `before`, so the sweep follows the marker that moved. Wrong: widen `before`/`after` into
# an alternation that makes the marker one branch among several — the pattern would then
# match prose that never writes the marker, and prose that never writes the marker joins no
# swept set, fires no reach-arm probe and is reported by no arm. `_banned_pattern` refuses
# that edit rather than trusting a reader to notice it.
#
# "Total" is an implication with a precondition, and the precondition is that membership
# and detection read the SAME TEXT. They did not. The sweep joined wrapped lines into
# paragraphs before matching and the membership test read raw text, so a claim wrapped
# inside the marker's own space — `complete OWL` / `2 RL entailment` — was a claim the
# sweep would have caught in a unit that never joined the set. Both now read
# `_reflowed` text, which makes the implication hold at every wrap point:
#
#   if a banned pattern matches the reflowed text of a unit,
#   then the pattern's marker is a substring of that same reflowed text (`_check_ban_table`
#   asserts each marker really does appear in its pattern's source, single-spaced),
#   therefore the unit is in the swept set.
#
# `_check_specimens` asserts each specimen sentence carries its marker, and does so in the
# form wrapped INSIDE the marker as well as on one line — so a unit ACQUIRES membership in
# the swept set at the same instant it acquires the claim, and the ban cannot be dodged by
# writing the claim somewhere unswept or by pressing return in the middle of it.
#
# The performance claim declares no marker, and that is deliberate rather than an
# oversight. Its pattern names a WORD rather than a claim, and the word is used honestly
# and often in prose whose subject IS measurement: `docs/BENCHMARKS.md` reports "2.1-2.2x
# faster than real `rdflib` at 100k triples" with a named competitor, a named workload and
# a reproducible harness, which is exactly the bounded form the ban asks for, and
# `docs/book/src/project/performance.md` disclaims '"Nx faster."' in as many words.
# Deriving a document set from that word would sweep both and refuse both, so the ban
# would be arguing with the two documents in the repository that already agree with it.
# It is therefore enforced over the surface the marker-bearing patterns define — the
# conformance and entailment story, where a comparative brag has no business appearing —
# and not repo-wide.
_BANNED_PARTS: tuple[tuple[str, str | None, str, str], ...] = (
    (
        r"\b(complete|full)[a-z]*\s+(?:the\s+)?",
        "owl 2",
        r" RDF-Based semantics",
        "the RDF-Based semantics is not finitely axiomatizable by a rule table; PurRDF "
        "implements a profile's rule table plus five named mechanisms, not the semantics",
    ),
    (
        r"\b(complete|full)[a-z]*\s+",
        "owl 2",
        r" conformance",
        "OWL 2 conformance is defined per syntax and per semantics over the whole test "
        "suite; what is measured here is one vendored subset of one corpus",
    ),
    (
        r"\b(complete|full)[a-z]*\s+",
        "owl 2",
        r" RL entailment",
        "78 / 78 is RULE-TABLE coverage. Entailment conformance is a different claim, "
        "measured separately and only over the cases actually vendored",
    ),
    (
        r"\bfully ",
        "conformant",
        r"\b",
        "conformance is per specification clause and per corpus; `fully conformant` names "
        "neither, so nothing can check it",
    ),
    (
        r"\b(faster|fastest|outperform[a-z]*)\b",
        None,
        "",
        "a comparative performance claim needs a named competitor, a named workload and a "
        "reproducible measurement; this repository's benches are report-only and assert "
        "no speedup",
    ),
)


# A subject marker's permitted SPELLING, and it is narrow on purpose. The marker is inserted
# into its pattern as a PLAIN LITERAL, so a metacharacter would stop it being one; it names
# the text a document must carry, so an upper-case letter would be a marker the ban's own
# lower-cased reasoning could not follow; and it is searched for in `_reflowed` text, where
# every whitespace run is a single space, so a tab or a double space could never be found
# there and the marker would silently sweep nothing. One spelling rule settles all three:
# lower-case ASCII words, single spaces between them, nothing else.
_MARKER_SPELLING = re.compile(r"[a-z0-9]+(?: [a-z0-9]+)*")


def _banned_pattern(before: str, subject: str | None, after: str) -> re.Pattern[str]:
    """One banned claim, composed so it CANNOT match prose that omits its subject marker.

    Requirement as a property of the PATTERN, which is what a witness could not give. The
    old guard deleted the marker from that pattern's own specimen and required the match to
    stop — which proves the pattern cannot match THAT STRING without the marker, not that it
    cannot match ANY string without it. Adding a branch (``(?:OWL 2|OWL2) RL entailment``)
    passed that witness while a document writing the unspaced spelling joined no swept set,
    fired no reach-arm probe and was reported by no arm of the ban. The natural trigger was
    benign: a maintainer BROADENING the ban would have punched the hole.

    So the marker is not written into a pattern, it is composed into one. Each half is
    required to be a regular expression ON ITS OWN and is then wrapped as ``(?:…)``, and the
    marker goes between them as a literal. The result is a top-level concatenation of three
    atoms, so every string the pattern matches contains the marker's characters
    consecutively — case-insensitively, which is exactly what :data:`_MARKER_LITERALS` then
    asks of a document. There is no alternation, quantifier or group either half can spell
    that reaches around a concatenated atom: an added branch lands INSIDE its own ``(?:…)``,
    and a half that would only balance once the marker sat between the two — ``…(?:`` and
    ``|OWL2 …)`` — is refused here, by name, because that is the one shape that could put the
    marker inside a group this composition does not control.

    ``None`` is the marker-less case (see the performance claim below): its pattern is
    written whole in ``before``, it derives no swept set, and it is enforced over the surface
    the marked patterns define.
    """
    if subject is None:
        if after:
            raise SystemExit(
                f"check-doc-claims: a banned overclaim with no subject marker spells its "
                f"whole pattern in one part, and this one also carries {after!r}. There is "
                f"no marker to compose around, so the second part would be silently "
                f"concatenated and the reader would be told a marker bounds it"
            )
        return re.compile(before, re.I)
    if not _MARKER_SPELLING.fullmatch(subject):
        raise SystemExit(
            f"check-doc-claims: the subject marker {subject!r} is not spelled as lower-case "
            f"words separated by single spaces. It is inserted into its pattern as a plain "
            f"literal and searched for in `_reflowed` text, so anything else is either not "
            f"a literal there or not findable here, and the derived sweep would silently "
            f"sweep nothing for this pattern"
        )
    for role, part in (("before", before), ("after", after)):
        try:
            re.compile(part)
        except re.error as broken:
            raise SystemExit(
                f"check-doc-claims: the {role} half of the banned claim whose subject "
                f"marker is {subject!r} is not a regular expression on its own ({broken}): "
                f"{part!r}. Each half is wrapped as `(?:…)` and composed around the marker, "
                f"and that wrapping is the only reason an added alternation branch cannot "
                f"reach around it — a half that balances only once the marker sits between "
                f"the two would put the marker inside a group this composition does not "
                f"control, and `(?:OWL 2|OWL2) RL entailment` matches prose that never "
                f"writes the marker, joins no swept set and is reported by no arm. To ban a "
                f"second spelling, add a second entry with its own marker and its own "
                f"specimen, or re-point this entry's marker at a literal both spellings "
                f"share and put the alternation in `before`"
            ) from broken
    return re.compile(f"(?:{before}){subject}(?:{after})", re.I)


# The ban table the rest of this file reads: (compiled pattern, subject marker, why). Derived
# from the parts rather than written beside them, so the composition above is the only way a
# banned claim gets compiled at all.
_BANNED_OVERCLAIMS: tuple[tuple[re.Pattern[str], str | None, str], ...] = tuple(
    (_banned_pattern(before, subject, after), subject, why)
    for before, subject, after, why in _BANNED_PARTS
)


# Each subject marker as a compiled literal, so MEMBERSHIP asks the same question DETECTION
# does, in the same currency. The ban's patterns match case-insensitively and require their
# marker's characters consecutively (`_banned_pattern`); a lower-cased substring test is not
# the same question, because `re.IGNORECASE` folds pairs `str.lower` does not move — `ſ`
# matches `s`, `K` matches `k`, `İ` matches `i` — so a pattern could match text a `.lower()`
# containment test said carried no marker, and the derivation's implication (matched here,
# therefore swept there) would hold for every document anyone had tried and not in general.
# Derived from the same markers, so re-pointing one re-points this in the same edit.
_MARKER_LITERALS: tuple[tuple[str, re.Pattern[str]], ...] = tuple(
    (subject, re.compile(re.escape(subject), re.I))
    for subject in sorted({subject for _, subject, _ in _BANNED_OVERCLAIMS if subject})
)


# The subject markers as a RAW-TEXT probe: each marker with its internal spaces relaxed to
# `\s+`, so it matches whatever a paragraph reflow did to the words. The reach arm walks the
# whole documented surface and would otherwise reflow ten megabytes to find nothing, so it
# asks this first and stops if the answer is no.
#
# Sound rather than merely fast, and this is why: a banned pattern that matches reflowed text
# necessarily contains that pattern's marker single-spaced (`_check_ban_table`), and a
# single-spaced marker in reflowed text was a run of whitespace between the same words in the
# raw text — which is exactly what this matches. Derived from the same markers rather than
# written out, so it cannot go stale, and the self-test injects through it in both wrap forms.
_MARKER_PROBE = re.compile(
    "|".join(
        sorted(
            r"\s+".join(re.escape(word) for word in subject.split())
            for _, subject, _ in _BANNED_OVERCLAIMS
            if subject
        )
    ),
    re.I,
)


def _table_pairs() -> tuple[
    tuple[tuple[re.Pattern[str], str | None, str], tuple[str, str]], ...
]:
    """``(banned claim, specimen)`` for the whole ban table, asserted index-aligned first.

    The two tables are written apart — the ban beside its reason, the specimens beside the
    wrap forms they exercise — and every reader of one wants the other. Pairing them in one
    place is what lets the alignment be asserted ONCE for every reader rather than by
    whichever of them remembered to: a `zip` over mismatched tables silently drops the tail,
    and the tail is a banned claim with no specimen, which is a pattern whose marker
    requirement and whose wrap forms both go untested while the self-test still prints a
    number.

    ``_OVERCLAIM_SPECIMENS`` is defined further down this file, beside the wrap forms it is
    written to exercise; this is a function, so the name is resolved when it is CALLED and
    the two tables can each sit where they read best.
    """
    if len(_OVERCLAIM_SPECIMENS) != len(_BANNED_OVERCLAIMS):
        raise SystemExit(
            f"check-doc-claims: {len(_OVERCLAIM_SPECIMENS)} specimen(s) for "
            f"{len(_BANNED_OVERCLAIMS)} banned claim(s) — they are index-aligned, so a "
            f"banned claim would go untested. Add the missing specimen"
        )
    return tuple(zip(_BANNED_OVERCLAIMS, _OVERCLAIM_SPECIMENS))


def _check_marker_bounds(
    pattern: re.Pattern[str], subject: str, specimen: str
) -> None:
    """`subject` must really BOUND `pattern` — a WITNESS, in the currency of text.

    The second opinion, and deliberately not the proof. Requirement is a property of the
    PATTERN and is settled where the pattern is built (:func:`_banned_pattern`): the marker
    is a literal in a concatenation, so no string the pattern matches can omit it. This
    function asks the same question of one string instead — delete the marker from the
    pattern's own specimen and the match must stop — which is strictly weaker, and saying so
    is the point: a witness that stands in for the proof is what let an added alternation
    branch pass while ``complete OWL2 RL entailment`` escaped the whole ban.

    What it is still FOR is the route that goes around the composition: a pattern compiled by
    hand into the table, where nothing checked that the marker is required and nothing
    checked that it is even spelled. Both of those it can see, and :data:`_MUTATIONS` hands
    it the shape they take — a marker wrapped as ``(?:OWL 2 )?``, which leaves the literal in
    ``pattern.pattern`` verbatim so a source check passes it, while a document saying
    ``complete RL entailment`` carries no marker, joins no swept set, fires no marker probe
    and is reported by no arm of the ban.

    Takes its three arguments rather than reading the table, so that mutation can be applied
    without editing one.
    """
    if subject not in pattern.pattern.lower():
        raise SystemExit(
            f"check-doc-claims: the banned overclaim {pattern.pattern!r} does not "
            f"contain its own subject marker {subject!r}, so a document without the "
            f"marker could still match it and the derived sweep would not be total. "
            f"Re-point the marker at what the pattern really requires"
        )
    # A space in place of the marker rather than nothing, so the deletion cannot glue two
    # words into a third; reflowed afterwards, because reflowed text is what all three arms
    # of the ban read.
    without = _reflowed_stripped(
        re.sub(re.escape(subject), " ", specimen, count=1, flags=re.I)
    )
    if pattern.search(without):
        raise SystemExit(
            f"check-doc-claims: the banned overclaim {pattern.pattern!r} still matches "
            f"{without!r} — its own specimen with the subject marker {subject!r} deleted "
            f"— so the pattern does not REQUIRE the marker it declares. The marker is "
            f"what derives the swept set and what the reach arm's probe asks for first, "
            f"so a claim written without it would join no set and be reported by no arm. "
            f"Make the marker mandatory in the pattern, or re-point it at what the "
            f"pattern really requires"
        )


def _check_ban_table() -> None:
    """The ban table must keep enough markers, and each must really bound its pattern.

    A marker that does not appear in its pattern's source is a marker that no longer proves
    anything about which documents the pattern can match, and the derived sweep would
    silently stop being total. A pattern edited so it no longer REQUIRES its marker — the
    literal deleted, wrapped as ``(?:OWL 2 )?``, or made one branch of an alternation — is
    not caught here at all: it cannot be written. The table holds parts, and
    :func:`_banned_pattern` composes the marker into every pattern as a concatenated literal,
    so the edit fails at import, on the line that spells it, in the same commit. What is left
    for this function is the FLOOR (a marker deleted outright, by setting it to ``None``) and
    :func:`_check_marker_bounds`, the witness that still answers if a pattern is ever
    compiled by hand around the composition.

    The specimens are read through :func:`_table_pairs`, which is also what asserts the two
    tables are still index-aligned; a pattern with no specimen beside it would otherwise be
    a pattern whose marker requirement went untested.
    """
    marked = sum(1 for _, subject, _ in _BANNED_OVERCLAIMS if subject)
    if marked < 4:
        raise SystemExit(
            f"check-doc-claims: only {marked} banned overclaim(s) declare a subject "
            f"marker, below this table's floor of 4. Setting a marker to None narrows "
            f"BOTH the swept set and the reach arm in one edit — which is the shape of "
            f"narrowing this ban already had to have removed once. Restore the marker, "
            f"or lower the floor deliberately in the same commit and say why"
        )
    for (pattern, subject, _), (sentence, _wrapped) in _table_pairs():
        if subject is None:
            continue
        _check_marker_bounds(pattern, subject, sentence)


@lru_cache(maxsize=_MARKER_CACHE)
def _markers_in(text: str) -> tuple[str, ...]:
    """The banned claims' subject markers `text` carries, read from its REFLOWED form.

    The whole membership test, in one place, so the sweep's derivation and the coverage
    arm's independent re-check cannot drift apart. Reflowed because that is what makes the
    derivation total: a claim broken inside `OWL 2` carries the marker just as much as one
    that is not, and the raw-text test said otherwise.
    """
    reflowed = _reflowed(text)
    return tuple(
        subject for subject, literal in _MARKER_LITERALS if literal.search(reflowed)
    )


def _claim_corpus(surface: list[Path]) -> list[tuple[str, str]]:
    """``(label, prose)`` for every unit the entailment-overclaim ban may sweep.

    The Markdown of :func:`_documented_surface` — the ban judges SENTENCES, and
    ``_sentences`` reads Markdown (fences, tables, wrapped paragraphs), not source code —
    plus every registry description (:func:`_registry_prose`), which is prose a reader
    meets on crates.io, PyPI and npm without opening the repository at all.
    """
    units = [
        (str(path.relative_to(_REPO)), _read(path))
        for path in surface
        if path.suffix == ".md" and path != _GATE_SCRIPT
    ]
    units.extend(_registry_prose())
    return units


def _entailment_claim_units(surface: list[Path]) -> list[tuple[str, str]]:
    """The prose units the entailment-overclaim ban sweeps, DERIVED not enumerated.

    This set used to be a hand-written nine-tuple. Nothing asserted the tuple was
    complete, and the self-test iterated the same tuple, so cutting it from nine entries
    to one left eight documents outside the ban and BOTH the gate and its preflight
    printing a green line about the one that remained. That is a gate that cannot fail,
    one level up from the gate the preflight was written to keep honest.

    So the set is derived the way :func:`banned_stale_fragment_names` derives its own:
    from :func:`_claim_corpus`, filtered to the units that carry a banned claim's subject
    marker.

    Membership is computed from the same REFLOWED text the ban then reads, through
    ``_read`` and ``_reflowed``, so a unit that acquires a banned claim acquires
    membership in the same edit — including when the claim wraps inside its own marker,
    which is the one wrap point the earlier raw-text test could not see.
    """
    return [(label, text) for label, text in _claim_corpus(surface) if _markers_in(text)]


# A line that opens a block of its OWN rather than continuing the one above it: a heading, a
# list item, a table row, a fence. Two of those are two statements, not one reflowed one, so
# joining them would let one line's scope phrase exempt the next line's claim — which is the
# same defect as the line-scoped split, arriving from the other side.
_BLOCK_OPENER = re.compile(r"^\s*(?:#{1,6}\s|[-*+]\s|\d+[.)]\s|\|)")
_FENCE = re.compile(r"^\s*(?:```|~~~)")
# The sentence terminator, and the whitespace after it that a paragraph join has already
# normalized to one space.
_TERMINATOR = re.compile(r"(?<=[.!?])\s+")


def _paragraph_sentences(lines: list[tuple[int, str]]) -> list[tuple[int, str]]:
    """The sentences of `lines` joined into one paragraph, each at the line it STARTS on.

    The start line rather than the line the claim's banned phrase lands on, so a wrapped
    claim is reported where a reader will find its first word.

    Each line is ``_reflowed`` before it is joined, so the sweep and the membership test
    that derives the swept set agree about what an OCCURRENCE is right down to the run of
    spaces inside one line: `OWL  2 RL entailment` is the same claim as `OWL 2 RL
    entailment`, and a table cell indented with a tab is the same sentence as one that is
    not.
    """
    joined = ""
    starts: list[tuple[int, int]] = []
    for number, piece in lines:
        if joined:
            joined += " "
        starts.append((len(joined), number))
        joined += _reflowed_stripped(piece)

    spans: list[tuple[int, int]] = []
    opened = 0
    for separator in _TERMINATOR.finditer(joined):
        spans.append((opened, separator.start()))
        opened = separator.end()
    spans.append((opened, len(joined)))

    out: list[tuple[int, str]] = []
    for begin, end in spans:
        sentence = joined[begin:end]
        if not sentence.strip():
            continue
        number = next(number for offset, number in reversed(starts) if offset <= begin)
        out.append((number, sentence))
    return out


# Big enough to hold the whole derived sweep, so one run never splits a document's sentences
# twice; `banned_entailment_overclaims` fails if the swept set outgrows it rather than
# quietly paying for the re-parse. A document's sentences are a pure function of its text,
# and the returned list is read and never mutated, here or by the ban walk.
_SENTENCE_CACHE = 64


@lru_cache(maxsize=_SENTENCE_CACHE)
def _sentences(text: str) -> list[tuple[int, str]]:
    """`(line number, sentence)` for every sentence of `text`, with WRAPPED LINES JOINED.

    Sentence-scoped rather than line-scoped because the exemption is: the SENTENCE
    carrying the claim also carries the scope phrase. A line-scoped check would exempt a
    claim whose scope sits on the previous wrapped line and refuse one whose scope sits on
    the next, which is a property of the paragraph reflow rather than of the prose.

    Splitting each LINE on sentence terminators is line-scoped by another name, and that is
    what this did: a sentence that wrapped became two half-sentences, neither of which was
    the sentence anyone wrote. Both failure modes it was written to prevent were live —
    ``the complete OWL 2 RDF-Based semantics.`` was caught and the same words wrapped after
    ``RDF-Based`` were not, so every banned claim was one line wrap from being unsayable to
    being invisible; and a correctly scoped claim whose ``on this vendored W3C corpus`` sat
    on the next line was refused for a scope it did carry.

    So the lines are joined into paragraphs FIRST. A blank line ends one, and so does any
    line that opens a block of its own (see ``_BLOCK_OPENER``). Inside a fenced code block
    nothing is joined at all: code is not reflowed prose, and two adjacent code lines are
    two statements.

    Markdown table rows are one "sentence" per cell, because a table cell is a standalone
    statement and the row's other cells are not its context.
    """
    out: list[tuple[int, str]] = []
    paragraph: list[tuple[int, str]] = []

    def flush() -> None:
        if paragraph:
            out.extend(_paragraph_sentences(paragraph))
            paragraph.clear()

    fenced = False
    for offset, line in enumerate(text.splitlines(), start=1):
        if _FENCE.match(line):
            flush()
            fenced = not fenced
            continue
        if fenced or not line.strip():
            flush()
            if line.strip():
                out.extend(_paragraph_sentences([(offset, line)]))
            continue
        if line.lstrip().startswith("|"):
            flush()
            for cell in line.split("|"):
                out.extend(_paragraph_sentences([(offset, cell)]))
            continue
        if _BLOCK_OPENER.match(line):
            flush()
        paragraph.append((offset, line))
    flush()
    return out


def _overclaims_in(relative: str, text: str) -> list[str]:
    """Every banned overclaim `text` makes, reported against `relative`.

    One document at a time, so nothing that exercises this walk has to re-read the rest of
    the swept set to do it. The self-test injects a sentence into one document at a time
    and used to re-run the WHOLE sweep for each injection, which made its cost the square
    of the swept set — a shape that quietly punishes widening the very set this gate now
    derives rather than enumerates.
    """
    problems: list[str] = []
    for line, sentence in _sentences(text):
        if _CORPUS_SCOPE in sentence:
            continue
        for pattern, _subject, why in _BANNED_OVERCLAIMS:
            match = pattern.search(sentence)
            if match:
                problems.append(
                    f"{relative}:{line}: banned entailment overclaim "
                    f"`{match.group(0)}` — {why}. Scope the sentence with the literal "
                    f"phrase {_CORPUS_SCOPE!r}, or say the bounded thing instead"
                )
    return problems


def _reach_hits(label: str, text: str, *, locate: bool) -> list[str]:
    """Every marker-bearing banned claim `text` makes, for a unit OUTSIDE the swept set.

    The ban's third arm, extracted so it can be injected into rather than only walked. It
    matches over ``_reflowed`` text, which is the same text the membership test reads: a
    claim wrapped inside its own subject marker either joins the swept set and is caught
    there, or it is not in the swept set and is caught here. Before both arms read reflowed
    text it could do neither, and the "permission boundary, not a detection boundary"
    property held everywhere except the one wrap point that mattered.

    Deliberately coarser than :func:`_overclaims_in`: it has no sentence structure and no
    exemption, because a unit outside the swept set has nowhere to put a scope phrase. A
    hit here is either a claim to reword or a claim to move into documentation the ban
    reads, and both are loud.

    `locate` asks for a line number, which costs the offset map; registry descriptions are
    single strings with no line to name.
    """
    hits: list[str] = []
    if not _MARKER_PROBE.search(text):
        return hits
    flat = _reflowed(text)
    for pattern, subject, why in _BANNED_OVERCLAIMS:
        if subject is None:
            continue
        for match in pattern.finditer(flat):
            where = f"{label}:{_reflowed_line(text, match.start())}" if locate else label
            hits.append(
                f"{where}: banned entailment overclaim `{match.group(0)}` OUTSIDE the "
                f"swept set — {why}. This unit is not one the ban sweeps sentence by "
                f"sentence, so the {_CORPUS_SCOPE!r} exemption cannot be applied to it: "
                f"say the bounded thing instead, or move the claim to documentation the "
                f"ban reads"
            )
    return hits


def _reach_arm(
    surface: list[Path], swept_set: set[str]
) -> tuple[list[str], int]:
    """The reach arm's WALK: every unit outside `swept_set`, and how many it visited.

    A function rather than a loop inside the ban so the self-test can run it over a surface
    of ONE with an injected claim and see the whole arm answer — the filter, the read and
    the match — for the price of one file instead of eight hundred. Injecting into
    :func:`_reach_hits` alone proved the matcher answered and left the walk itself untested,
    which is how the arm could have been deleted with every gate still green.
    """
    problems: list[str] = []
    visited = 0
    for path in surface:
        if path == _GATE_SCRIPT:
            continue
        relative = str(path.relative_to(_REPO))
        if relative in swept_set:
            continue
        visited += 1
        problems.extend(_reach_hits(relative, _read(path), locate=True))
    for label, text in _registry_prose():
        if label in swept_set:
            continue
        visited += 1
        problems.extend(_reach_hits(label, text, locate=False))
    return problems, visited


def _check_visited(arm: str, visited: int, expected: int, why: str) -> None:
    """`arm` must really have WALKED the units it reports nothing about.

    An arm whose loop is deleted reports zero problems, which reads exactly like a clean
    tree. The reach arm has counted its visits since it was extracted; the sentence sweep
    and the coverage arm did not, and either loop could be removed with every gate green —
    the sweep because a swept set nobody sweeps yields no problems, the coverage arm because
    a check nobody runs names no missing document. All three now count, against a total the
    DERIVATION produced rather than against anything the loop itself computed.

    Takes the two counts rather than deriving either, so :data:`_MUTATIONS` can hand it a
    deleted loop's answer — zero visits of a non-empty set — and require it to fail.
    """
    if visited < expected:
        raise SystemExit(
            f"check-doc-claims: {arm} visited {visited} of the {expected} unit(s) it must "
            f"read. It was narrowed or skipped — {why}"
        )


def _registered_prose_documents() -> list[Path]:
    """Every Markdown document this script names as a module-level constant.

    An INDEPENDENT reference for the ban's reach, read out of this module's own globals.
    These paths are maintained for entirely different reasons — each is the subject of a
    numeric claim elsewhere in this file — so narrowing the ban's traversal cannot narrow
    them in the same edit, and the coverage arm below can name exactly which documents
    left. A hand-written list of "documents the ban must reach" would be the defect this
    change removes, wearing a different hat.
    """
    found: set[Path] = set()
    for value in list(globals().values()):
        for item in value if isinstance(value, tuple) else (value,):
            if isinstance(item, Path) and item.suffix == ".md":
                found.add(item)
    if len(found) < 8:
        raise SystemExit(
            f"check-doc-claims: only {len(found)} Markdown document(s) are reachable as "
            f"module-level constants, so the entailment-overclaim ban's coverage arm has "
            f"almost nothing to check itself against. The document constants were inlined "
            f"or renamed; restore them rather than leaving the ban's reach unchecked"
        )
    return sorted(found)


def banned_entailment_overclaims(
    surface: list[Path],
) -> tuple[list[str], int, int, int]:
    """The unbounded entailment claims this documentation may not make.

    Modelled on :func:`banned_stale_fragment_names`, and run from the same ``main``, for
    the same reason: some statements are wrong in a way no NUMBER check can see. Every
    other gate in this file compares a documented figure against a generated one, which
    catches a stale count and is blind to a sentence that states no count at all.

    It exists at exactly the moment it is most needed. The documentation this repository
    carries was rewritten from "here are the known gaps" to "50 / 50, ledger empty", and
    that is precisely when a `complete OWL 2 RL entailment` sentence gets written — the
    numbers really did all reach their ceilings, and the step from "every vendored case
    agrees" to "the implementation is complete" is one short sentence and one large lie.

    Five claims are banned, each with its own reason (see ``_BANNED_OVERCLAIMS``), and
    each is exempt when the SENTENCE carrying it also carries the literal phrase
    ``on this vendored W3C corpus``. The exemption is a literal string rather than a
    pattern on purpose: a bounded claim has to say what it is bounded BY, and a phrase
    a writer must type verbatim is one a reader can search for.

    The exemption is a property of the SENTENCE, so the sentence has to be the unit — which
    means joining wrapped lines back into paragraphs before splitting on terminators; see
    ``_sentences``, which did not, and ``overclaim_self_test``, which now injects every banned
    claim into every swept unit in every form its whitespace can take, and asserts the
    answer in both directions.

    THREE arms, because the swept set is where this ban was last hollowed out:

      * the SWEEP itself, sentence-scoped and exemption-aware, over the derived set;
      * a COVERAGE arm — every Markdown document this script names as a module-level
        constant must be reachable by the traversal, and must be SWEPT if it carries a
        subject marker. Narrow the traversal, the suffix filter or the derivation itself
        and this names the documents that left, whether or not any of them carries a
        claim today;
      * a REACH arm (:func:`_reach_hits`) — every marker-bearing pattern is matched against
        the WHOLE documented surface, source files and registry descriptions included, and
        a hit outside the swept set is a failure. The swept set is then a permission
        boundary and not a detection boundary: narrowing it cannot hide a claim, it can
        only take away the place a scoped claim may be written, which fails loudly rather
        than silently.

    All three read ``_reflowed`` text, which is what makes that last sentence true at every
    wrap point. While the sweep joined wrapped lines and the other two arms read raw text,
    a claim broken inside its own subject marker joined no set and matched no arm.

    And all three COUNT what they visited (:func:`_check_visited`), because an arm that
    reports nothing and an arm that never ran print the same green line. The reach arm
    counted from the day it was extracted; the other two did not, so either loop could be
    deleted with every gate green. The counts are checked against what the derivation
    produced, not against anything the loops compute for themselves.

    Returns the problems, the number of units swept, the size of the corpus they were
    drawn from and the number of units the reach arm visited, so the script's headline
    reports what each arm really read rather than a number from somewhere nearby.
    """
    _check_ban_table()
    problems: list[str] = []
    corpus = _claim_corpus(surface)
    units = _entailment_claim_units(surface)
    if not units:
        raise SystemExit(
            "check-doc-claims: the entailment-overclaim ban derived an EMPTY set from a "
            f"corpus of {len(corpus)} prose unit(s). Either the subject markers no longer "
            "match any documentation or the traversal broke; a ban that sweeps nothing "
            "passes everything"
        )
    if len(units) > _SENTENCE_CACHE:
        raise SystemExit(
            f"check-doc-claims: the swept set has grown to {len(units)} units, past "
            f"the {_SENTENCE_CACHE}-entry sentence cache, so the sweep would re-parse "
            f"documents it has already read. Raise _SENTENCE_CACHE"
        )
    swept = 0
    for label, text in units:
        swept += 1
        problems.extend(_overclaims_in(label, text))
    _check_visited(
        "the sentence sweep",
        swept,
        len(units),
        "and the sweep is the arm that reads the derived set sentence by sentence, so a "
        "banned claim in a unit the derivation produced would be matched by nothing",
    )

    swept_set = {label for label, _ in units}
    reachable = {label for label, _ in corpus}
    registered = _registered_prose_documents()
    covered = 0
    for document in registered:
        covered += 1
        relative = str(document.relative_to(_REPO))
        if relative not in reachable:
            problems.append(
                f"{relative}: this gate already names the document elsewhere, and the "
                f"entailment-overclaim ban's traversal does not reach it. The traversal "
                f"was narrowed — a document outside it cannot be swept, and a claim "
                f"written there would never be read"
            )
            continue
        if relative in swept_set:
            continue
        carried = list(_markers_in(_read(document)))
        if carried:
            problems.append(
                f"{relative}: this gate already names the document elsewhere, it carries "
                f"the banned claims' subject marker(s) {carried}, and the "
                f"entailment-overclaim ban does not sweep it. The derivation dropped it — "
                f"restore it rather than leaving one of this gate's own documents outside "
                f"the ban"
            )
    _check_visited(
        "the coverage arm",
        covered,
        len(registered),
        "and the coverage arm is what names the documents this gate knows about that the "
        "traversal or the derivation stopped reaching",
    )

    hits, reached = _reach_arm(surface, swept_set)
    problems.extend(hits)
    # What the reach arm VISITED, not what it found. An arm whose walk is skipped reports
    # zero hits exactly like a clean tree, so the count is checked rather than assumed:
    # everything outside the swept set must be visited, and this says so out loud.
    _check_visited(
        "the reach arm",
        reached,
        len(surface) - 1 + len(_registry_prose()) - len(swept_set),
        "and that arm is the only reason the swept set is a permission boundary rather than "
        "a detection boundary, so a claim written in the gap would be reported by nothing",
    )
    return problems, swept, len(corpus), reached


# One specimen sentence per banned claim, in two written forms: on one line, and wrapped
# INSIDE the banned phrase itself. The wrap is where the ban used to end — every one of these
# claims was sayable by pressing return in the middle of it — so the pair is the falsifiable
# form of the rule that the exemption is a property of the SENTENCE and not of the reflow.
#
# The last one wraps before the word rather than inside it: a one-word pattern cannot be split
# by a line break, and what a wrap moves there is the SCOPE, which `_scoped` then tests.
#
# These two are written; the rest of the matrix is DERIVED. `_specimen_forms` adds the scoped
# variants and, for a marker with an internal space, the forms whose whitespace falls inside
# the marker itself — which is the wrap point all five written ones happen to miss, and the
# one the derivation used to be blind to.
_OVERCLAIM_SPECIMENS: tuple[tuple[str, str], ...] = (
    (
        "PurRDF implements the complete OWL 2 RDF-Based semantics.",
        "PurRDF implements the complete OWL 2 RDF-Based\nsemantics.",
    ),
    (
        "PurRDF reaches complete OWL 2 conformance.",
        "PurRDF reaches complete\nOWL 2 conformance.",
    ),
    (
        "PurRDF has complete OWL 2 RL entailment.",
        "PurRDF has complete OWL 2 RL\nentailment.",
    ),
    ("PurRDF is fully conformant.", "PurRDF is fully\nconformant."),
    (
        "PurRDF is faster than every alternative.",
        "PurRDF is\nfaster than every alternative.",
    ),
)


def _scoped(sentence: str) -> str:
    """`sentence` with the scope phrase on the NEXT line — the claim the ban must permit.

    The other half of the same defect: a line-scoped check refuses this, because the scope it
    carries is one wrap away from the claim it scopes. Every correctly bounded claim in these
    documents is one paragraph reflow from being rejected for saying exactly what it should.
    """
    return f"{sentence.rstrip('.')}\n{_CORPUS_SCOPE}."


def _broken_in_marker(
    sentence: str, subject: str | None, separator: str
) -> str | None:
    """`sentence` with the subject marker's own internal space replaced by `separator`.

    The specimen forms the other five could not see. Every hand-written wrapped specimen
    breaks somewhere ELSE — after ``RDF-Based``, before ``OWL 2``, after ``OWL 2 RL``,
    after ``fully``, before ``faster`` — so 525 injections all passed while
    ``complete OWL`` / ``2 RL entailment`` joined no set and matched no arm. A specimen
    that only breaks where someone thought to break it tests the wrap points that were
    already safe.

    DERIVED from the marker rather than written beside it, so it cannot drift away from the
    literal the membership test searches for: re-point a marker and these forms re-point
    with it, in the same edit. ``None`` when the marker is a single word — no whitespace run
    lives inside ``conformant``, so there is no such form and nothing to test.
    """
    if subject is None or " " not in subject:
        return None
    match = re.search(re.escape(subject), sentence, re.I)
    if not match:
        return None
    return (
        sentence[: match.start()]
        + match.group(0).replace(" ", separator, 1)
        + sentence[match.end() :]
    )


# Every whitespace character this repository's prose can hold: the ASCII set Python names,
# plus the no-break and narrow-no-break spaces its own numeric prose uses — `_int` above
# strips the narrow one out of documented counts. The two spellings of the normalization
# must agree on all of them, because `_reflowed_stripped` is what the sweep reads and
# `_reflowed` is what membership and the reach arm read.
_WHITESPACE_PROBES = tuple(sorted(set(string.whitespace) | {" ", " "}))


# The two whitespace runs the full specimen matrix breaks a marker with. Two rather than
# all eight because the matrix runs over every swept unit and every banned claim, and what
# a third ASCII space character would add there is cost; `overclaim_self_test` runs the
# whole probe set over one unit instead. `_check_separators` asserts these two still cover
# both KINDS of run, since that is what the pair is for: one crosses a line boundary, which
# only the paragraph join can survive, and one does not, which only the within-line
# normalization can.
_BROKEN_SEPARATORS = ("\n", "  ")


def _check_separators() -> None:
    """The matrix must break a marker both ACROSS a line and WITHIN one."""
    if not any("\n" in run for run in _BROKEN_SEPARATORS):
        raise SystemExit(
            "check-doc-claims: no specimen separator crosses a line boundary, so the "
            "paragraph join the sweep depends on would go untested"
        )
    if not any("\n" not in run for run in _BROKEN_SEPARATORS):
        raise SystemExit(
            "check-doc-claims: every specimen separator crosses a line boundary, so a "
            "claim whose whitespace falls WITHIN one line — `OWL  2 RL entailment` — would "
            "go untested, and the sweep normalizes between lines and within one for "
            "exactly that reason"
        )
    stray = sorted(set("".join(_BROKEN_SEPARATORS)) - set(_WHITESPACE_PROBES))
    if stray:
        raise SystemExit(
            f"check-doc-claims: the specimen separators use {stray}, which are not "
            f"whitespace this gate normalizes. A separator that is not whitespace tests "
            f"nothing about the reflow"
        )


def _check_reflow_agreement(form: str, text: str) -> None:
    """The two spellings of the normalization must produce the same string."""
    if _reflowed_stripped(text) != _reflowed(text).strip():
        raise SystemExit(
            f"check-doc-claims: `_reflowed_stripped` and `_reflowed` disagree about the "
            f"specimen form {form!r} ({text!r}). The sweep uses the first and the "
            f"membership test and the reach arm use the second, so they would be back to "
            f"reading two different texts and a claim could sit in the gap"
        )


def _broken_forms(sentence: str, subject: str | None) -> list[str]:
    """Every form of `sentence` whose whitespace falls INSIDE its own subject marker.

    Empty for a single-word marker. One list, so the three places that inject a claim into
    something that does not yet carry one — an unswept document, an unswept registry
    description and a file outside the swept set entirely — all test the same forms, and
    adding a form covers all three.
    """
    return [
        form
        for separator in _BROKEN_SEPARATORS
        if (form := _broken_in_marker(sentence, subject, separator)) is not None
    ]


def _specimen_forms(
    sentence: str, wrapped: str, subject: str | None
) -> list[tuple[str, str, bool]]:
    """``(name, text, must be caught)`` for every form of one specimen the ban must answer.

    Eight forms where the marker has an internal space and five where it does not, in both
    directions: the claim must be caught however the whitespace inside it falls, and the
    same claim scoped must be exempt however it falls. The scoped variants are not
    decoration — a writer whose correctly bounded sentence happens to reflow inside
    ``OWL 2`` must not be refused for it.

    The last two pairs are the ones that were missing. A line break inside the marker is
    what the membership test could not see; a DOUBLE SPACE inside it is what the sweep
    could not see, because the sweep normalizes between lines and, until this commit, not
    within one.
    """
    forms = [
        ("one line", sentence, True),
        ("wrapped", wrapped, True),
        ("one line, scoped", _scoped(sentence).replace("\n", " "), False),
        ("scope on the next line", _scoped(sentence), False),
        ("wrapped, scoped", _scoped(wrapped), False),
    ]
    for separator in _BROKEN_SEPARATORS:
        name = "wrapped" if "\n" in separator else "double-spaced"
        inside = _broken_in_marker(sentence, subject, separator)
        if inside is None:
            continue
        forms.append((f"{name} inside the subject marker", inside, True))
        forms.append((f"{name} inside the subject marker, scoped", _scoped(inside), False))
    return forms


def _acquiring(host: Path, text: str, sentence: str) -> str:
    """`text` rewritten so `host` really carries `sentence`, in `host`'s OWN format.

    A Markdown document acquires a claim by gaining a paragraph. A manifest acquires one
    only inside the field a registry publishes, and only if the file still parses — an
    injection that produced unreadable TOML would prove nothing about the walk that reads
    it, and an injection appended as a comment would prove the opposite of what is wanted.

    The form wrapped inside the marker arrives with a real newline, which a single-line
    manifest string cannot hold, so it is written as the escape both TOML and JSON decode
    back to one. The parser, not this function, decides whether that worked.
    """
    if host.suffix == ".md":
        return f"{text}\n\n{sentence}\n"
    table = next(keys for name, keys in _REGISTRY_DESCRIPTION if name == host.name)
    data: object = _manifest_data(host.name, text)
    for key in table:
        data = data[key] if isinstance(data, dict) else ""
    if not isinstance(data, str) or data not in text:
        raise SystemExit(
            f"check-doc-claims: {host.relative_to(_REPO)} does not spell its "
            f"`{'.'.join(table)}` verbatim, so the self-test cannot make it acquire a "
            f"claim and the registry surface's own direction would go untested"
        )
    escaped = (
        sentence.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
    )
    return text.replace(data, f"{data} {escaped}", 1)


def _check_scope_placement(pattern: re.Pattern[str], scoped: str) -> None:
    """The scoped specimen must carry its scope phrase on a LATER LINE than its claim.

    The exemption direction is the half of this ban that refuses a sentence a writer is
    entitled to write, and the only form that tests it across a wrap is the one
    :func:`_scoped` builds. Written on ONE line it tests nothing the plain scoped form does
    not: a line-scoped ``_sentences`` — the defect this file removed — answers every
    same-line form correctly and every cross-line form wrongly.

    Takes the scoped text rather than scoping it here, so :data:`_MUTATIONS` can hand it the
    same-line form and require it to fail.
    """
    claim = pattern.search(scoped)
    if claim is None or _CORPUS_SCOPE not in scoped:
        raise SystemExit(
            f"check-doc-claims: the scoped specimen {scoped!r} does not carry both the "
            f"banned claim ({pattern.pattern!r}) and the literal scope phrase "
            f"{_CORPUS_SCOPE!r}, so the exemption it exists to test is not being tested"
        )
    if scoped.count("\n", 0, claim.end()) == scoped.count(
        "\n", 0, scoped.index(_CORPUS_SCOPE)
    ):
        raise SystemExit(
            f"check-doc-claims: the scoped specimen {scoped!r} carries its scope phrase on "
            f"the SAME line as the claim it scopes, so the one form that proves a correctly "
            f"bounded sentence survives a paragraph reflow has stopped proving it. A "
            f"line-scoped sweep would answer this form correctly and still refuse every "
            f"writer whose scope wrapped"
        )


def _check_specimens() -> None:
    """Each specimen must match ITS OWN banned claim, and carry that claim's marker.

    Four assertions, and three of them are what make the derived sweep total. The specimens are
    index-aligned with ``_BANNED_OVERCLAIMS`` — asserted by :func:`_table_pairs`, which
    every reader of either table goes through — and a specimen that matches some OTHER
    pattern would leave its own pattern untested while the self-test still printed a number.
    A marker-bearing specimen that does NOT contain its marker would be a counter-example to
    the whole derivation: it would be a real instance of the claim that a document could
    carry without joining the swept set.

    And the same has to hold of the specimen WRAPPED INSIDE ITS MARKER, which is where the
    derivation stopped being total: reflowed, that form must still carry the marker (so it
    joins the set) and must still match the pattern (so it is caught once it has). Asserted
    over the reflowed text rather than the raw text, because reflowed text is what all
    three arms now read — and the two spellings of that normalization
    (:func:`_reflowed` and :func:`_reflowed_stripped`) are asserted equal on every form,
    so "the same normalization by the faster route" stays a fact rather than a comment.

    The fourth is the other direction: the scoped form must put its scope phrase on a later
    LINE than the claim (:func:`_check_scope_placement`), because a scoped form written on
    one line is a form a line-scoped sweep answers correctly.
    """
    _check_separators()
    for (pattern, subject, _), (sentence, wrapped) in _table_pairs():
        _check_scope_placement(pattern, _scoped(sentence))
        if not pattern.search(sentence):
            raise SystemExit(
                f"check-doc-claims: the self-test's specimen {sentence!r} does not match "
                f"the banned overclaim beside it ({pattern.pattern!r}) — the ban was "
                f"reworded, so re-point the specimen rather than leaving the self-test "
                f"proving nothing."
            )
        if subject is not None and subject not in sentence.lower():
            raise SystemExit(
                f"check-doc-claims: the specimen {sentence!r} does not carry its claim's "
                f"subject marker {subject!r}, so a document could make this claim without "
                f"joining the swept set the marker derives. Either the marker or the "
                f"specimen is wrong"
            )
        for form, text, _catch in _specimen_forms(sentence, wrapped, subject):
            _check_reflow_agreement(form, text)
            if subject is None:
                continue
            # The same form with every whitespace character this repository's prose can
            # hold, one at a time, dropped inside the marker. The specimens carry spaces
            # and newlines and nothing else, so asserting the two normalizations agree on
            # the specimens alone would leave a tab free to make them disagree.
            for character in _WHITESPACE_PROBES:
                probe = _broken_in_marker(sentence, subject, character)
                if probe is not None:
                    _check_reflow_agreement(f"{form}, split by {character!r}", probe)
        for separator in _BROKEN_SEPARATORS:
            inside = _broken_in_marker(sentence, subject, separator)
            if subject is not None and " " in subject and inside is None:
                raise SystemExit(
                    f"check-doc-claims: the subject marker {subject!r} has an internal "
                    f"space, so whitespace can fall inside a claim that carries it — and "
                    f"no such form could be derived from the specimen {sentence!r}. That "
                    f"is exactly where this derivation stopped being total once; do not "
                    f"leave it untested"
                )
            # `subject is None` already implies `inside is None`, because that is the
            # first thing `_broken_in_marker` tests. Saying both here makes the invariant
            # LOCAL: the reader (and the type checker) sees why the `in` below is safe
            # without holding a distant helper's contract in their head.
            if inside is None or subject is None:
                continue
            if subject not in _reflowed(inside).lower() or not pattern.search(
                _reflowed(inside)
            ):
                raise SystemExit(
                    f"check-doc-claims: the specimen {inside!r}, broken inside its own "
                    f"subject marker, does not both carry {subject!r} and match "
                    f"{pattern.pattern!r} once reflowed. Membership and detection have "
                    f"stopped reading the same text, so a claim broken there would join "
                    f"no set and be matched by no arm"
                )


def overclaim_self_test(surface: list[Path], report: bool) -> list[str]:
    """Every injected claim the ban does not answer correctly. Empty is the passing answer.

    Two directions, over every swept document and every banned claim: the unscoped sentence
    must be CAUGHT in both its forms, and the same sentence scoped — with the scope phrase on
    the following line — must be exempt. The first commit to carry this ban verified it with
    45 single-line injections; every one of them exercised the one case that worked, and both
    of the cases that did not were live in the same file.

    A third direction was added with the derived document set: a marker-bearing claim
    injected into a corpus unit the ban does NOT currently sweep must pull that unit into
    the swept set and be caught there. That is the falsifiable form of the derivation — "a
    unit acquires membership at the same instant it acquires the claim" — and without it
    the widened set would be a story rather than a property. It runs against an unswept
    Markdown document AND an unswept registry description, because those are two different
    readers of the corpus and only one of them existed when the property was first written
    down. Each is injected in the plain form and in the form WRAPPED INSIDE ITS OWN MARKER,
    which is the form that used to join nothing.

    A fourth direction covers the reach arm, which nothing exercised: a marker-bearing
    claim written into a file the ban cannot sweep sentence by sentence — the first Rust
    source file on the surface — must still be REPORTED, in every form its whitespace can
    take, and must NOT be reported for a unit the sweep already reads. That arm is what
    makes the swept set a permission boundary rather than a detection boundary, and it read
    raw text until this commit, so the form broken inside the marker escaped it too.

    The count of injections is derived from the same call that derives the sweep, so it
    cannot be read as evidence that the sweep is wide: a narrowed sweep narrows this number
    with it, and it is the coverage and reach arms in
    :func:`banned_entailment_overclaims`, not this count, that refuse the narrowing.

    Nothing is written: each unit is read once and the injected copy lives in a string.
    """
    _check_ban_table()
    _check_specimens()
    units = _entailment_claim_units(surface)
    wrong: list[str] = []
    checked = 0
    for label, committed in units:
        for (_pattern, subject, _why), (sentence, wrapped) in _table_pairs():
            for form, injected, must_catch in _specimen_forms(
                sentence, wrapped, subject
            ):
                problems = _overclaims_in(label, f"{committed}\n\n{injected}\n")
                checked += 1
                if bool(problems) is must_catch:
                    continue
                wrong.append(
                    f"{label}: {'NOT CAUGHT' if must_catch else 'FALSELY CAUGHT'} "
                    f"({form}) — {injected!r}"
                )

    # EVERY whitespace character, once, in one unit. The matrix above breaks a marker with
    # two runs over every swept unit, which is where the cost is; this asks whether the
    # answer depends on WHICH whitespace fell inside the claim, which is a property of the
    # normalization and needs one document to settle.
    probe_label, probe_text = units[0]
    for (_pattern, subject, _why), (sentence, _wrapped) in _table_pairs():
        for character in _WHITESPACE_PROBES:
            injected = _broken_in_marker(sentence, subject, character)
            if injected is None:
                continue
            for form, text, must_catch in (
                (f"split by {character!r}", injected, True),
                (f"split by {character!r}, scoped", _scoped(injected), False),
            ):
                problems = _overclaims_in(probe_label, f"{probe_text}\n\n{text}\n")
                checked += 1
                if bool(problems) is must_catch:
                    continue
                wrong.append(
                    f"{probe_label}: {'NOT CAUGHT' if must_catch else 'FALSELY CAUGHT'} "
                    f"({form}) — {text!r}"
                )

    # The derivation itself, injected: an UNSWEPT corpus unit that acquires a
    # marker-bearing claim must both join the swept set and be caught in it.
    already = {label for label, _ in units}
    unswept = [
        path
        for path in surface
        if path.suffix == ".md"
        and path != _GATE_SCRIPT
        and str(path.relative_to(_REPO)) not in already
    ]
    if not unswept:
        raise SystemExit(
            "check-doc-claims: every Markdown document in the corpus is already swept, so "
            "the derivation's own direction — an unswept document that acquires a claim "
            "joins the set — cannot be tested. Widen the corpus or narrow nothing"
        )
    registry = _registry_prose()
    unswept_registry = [
        path
        for path in _registry_manifest_paths()
        if any(
            label.startswith(f"{path.relative_to(_REPO)} [") and label not in already
            for label, _ in registry
        )
    ]
    if not unswept_registry:
        raise SystemExit(
            "check-doc-claims: every registry description is already swept, so the "
            "derivation's own direction cannot be tested on the registry surface. Widen "
            "the corpus or narrow nothing"
        )
    # One unswept unit of each kind is enough, and each is the first in path order rather
    # than a chosen one: the property under test belongs to the MARKER, not to the unit,
    # and re-deriving the whole set once per unit per claim would cost more than the sweep.
    for host, relative in (
        (unswept[0], str(unswept[0].relative_to(_REPO))),
        (unswept_registry[0], str(unswept_registry[0].relative_to(_REPO))),
    ):
        host_text = _read(host)
        for (_pattern, subject, _why), (sentence, _wrapped) in _table_pairs():
            if subject is None:
                continue
            for injected in (sentence, *_broken_forms(sentence, subject)):
                _OVERLAY[str(host)] = _acquiring(host, host_text, injected)
                try:
                    joined = [
                        (label, body)
                        for label, body in _entailment_claim_units(surface)
                        if label == relative or label.startswith(f"{relative} [")
                    ]
                    caught = bool(joined) and bool(_overclaims_in(*joined[0]))
                finally:
                    _OVERLAY.clear()
                checked += 1
                if joined and caught:
                    continue
                wrong.append(
                    f"{relative}: NOT SWEPT ON ACQUIRING THE CLAIM — {injected!r} carries "
                    f"the subject marker {subject!r}, so this unit must join the derived "
                    f"set and be caught in it"
                )

    # The reach arm, injected: a marker-bearing claim in a file the ban cannot sweep
    # sentence by sentence must still be reported, however it wraps.
    outside = next(
        (path for path in surface if path.suffix == ".rs"),
        None,
    )
    if outside is None:
        raise SystemExit(
            "check-doc-claims: the documented surface holds no source file outside the "
            "swept set, so the reach arm — the one that makes the swept set a permission "
            "boundary rather than a detection boundary — cannot be tested"
        )
    outside_relative = str(outside.relative_to(_REPO))
    outside_text = _read(outside)
    for (_pattern, subject, _why), (sentence, _wrapped) in _table_pairs():
        if subject is None:
            continue
        for injected in (sentence, *_broken_forms(sentence, subject)):
            _OVERLAY[str(outside)] = f"{outside_text}\n// {injected}\n"
            try:
                # Through the whole arm, over a surface of one, and in both directions: a
                # unit outside the swept set is reported, and the same unit inside it is
                # left to the sentence sweep instead of being reported twice.
                reported, _ = _reach_arm([outside], set())
                permitted, _ = _reach_arm([outside], {outside_relative})
            finally:
                _OVERLAY.clear()
            checked += 2
            if any(entry.startswith(outside_relative) for entry in reported):
                if not any(
                    entry.startswith(outside_relative) for entry in permitted
                ):
                    continue
                wrong.append(
                    f"{outside_relative}: REACHED INSIDE THE SWEPT SET — {injected!r} was "
                    f"reported for a unit the sweep already reads sentence by sentence, so "
                    f"the swept set has stopped being a permission boundary"
                )
                continue
            wrong.append(
                f"{outside_relative}: NOT REACHED OUTSIDE THE SWEPT SET — {injected!r} "
                f"carries the subject marker {subject!r}, so the reach arm must report it "
                f"wherever it is written"
            )
    if report:
        print(
            f"check-doc-claims: the entailment-overclaim ban answered {checked} injected "
            f"sentence(s) — every banned claim in every wrap form over all {len(units)} "
            f"swept unit(s), each split by every one of the "
            f"{len(_WHITESPACE_PROBES)} whitespace characters over one of them, each "
            f"marker-bearing claim over one unswept document "
            f"({unswept[0].relative_to(_REPO)}) and one unswept registry description "
            f"({unswept_registry[0].relative_to(_REPO)}), and each over one unswept source "
            f"file ({outside_relative}) — "
            f"{checked - len(wrong)} correctly, {len(wrong)} not."
        )
    return wrong


def _marker_made_optional(before: str, subject: str, after: str) -> re.Pattern[str]:
    """The banned claim's parts recompiled with the subject marker OPTIONAL.

    The pattern a hand-compiled table entry could hold and a source-text check could not
    see: the marker's literal is still spelled in ``pattern.pattern``, so
    ``subject in pattern.pattern.lower()`` passes — while the pattern now matches prose
    that never writes the marker, and prose that never writes the marker joins no swept set
    and fires no reach-arm probe.

    Built from the PARTS rather than by rewriting a composed pattern's source, because the
    composition puts the marker's neighbouring space on the far side of a group boundary and
    a mutation that left that space mandatory would be a different, harmless pattern — a
    mutation that does not reproduce the defect proves nothing about the guard that refuses
    it.
    """
    return re.compile(
        f"(?:{before})(?:{subject} ?)?"
        f"(?:{after[1:] if after.startswith(' ') else after})",
        re.I,
    )


def _mutated_optional_marker() -> None:
    """A marker made optional rather than deleted. ``_check_marker_bounds`` must refuse."""
    for (before, subject, after, _why), (_entry, (sentence, _wrapped)) in zip(
        _BANNED_PARTS, _table_pairs()
    ):
        if subject is None:
            continue
        _check_marker_bounds(
            _marker_made_optional(before, subject, after), subject, sentence
        )
        return
    raise SystemExit(
        "check-doc-claims: no banned overclaim declares a subject marker, so the mutation "
        "that proves a marker must be REQUIRED cannot be applied"
    )


def _mutated_alternating_marker() -> None:
    """A second spelling smuggled in as an ALTERNATION BRANCH. ``_banned_pattern`` refuses.

    The escape the single-specimen witness could not see, in the shape a maintainer would
    have reached for: broadening `OWL 2 RL entailment` to `(?:OWL 2|OWL2) RL entailment`
    leaves the marker in the pattern's source and leaves the deletion witness satisfied,
    while `complete OWL2 RL entailment` joins no swept set and is reported by no arm. Spelled
    against the composition it must go through, the same edit splits into two halves that are
    not regular expressions on their own, and that is what is refused.

    The mutation proves the door is locked. The reason there is no second door is structural
    and lives in :func:`_banned_pattern`: an alternation written INSIDE either half stays
    inside its own ``(?:…)``, so the marker remains a concatenated atom whatever the halves
    say.
    """
    for before, subject, after, _why in _BANNED_PARTS:
        if subject is None:
            continue
        _banned_pattern(f"{before}(?:", subject, f"|OWL2{after})")
        return
    raise SystemExit(
        "check-doc-claims: no banned overclaim declares a subject marker, so the mutation "
        "that proves an alternation cannot reach around one cannot be applied"
    )


def _mutated_narrowed_mutations() -> None:
    """This table itself, cut to one entry. ``_check_mutation_floor`` must refuse."""
    _check_mutation_floor(_MUTATIONS[:1])


def _mutated_same_line_scope() -> None:
    """A scoped specimen written on ONE line. ``_check_scope_placement`` must refuse."""
    for (pattern, _subject, _), (sentence, _wrapped) in _table_pairs():
        _check_scope_placement(pattern, _scoped(sentence).replace("\n", " "))
        return
    raise SystemExit(
        "check-doc-claims: the ban table is empty, so the mutation that proves the scoped "
        "specimen still crosses a line cannot be applied"
    )


def _mutated_dropped_arm() -> None:
    """An arm of the surface walk contributing nothing. ``_check_surface_landmarks`` refuses."""
    _check_surface_landmarks({path for path, _ in _SURFACE_LANDMARKS[1:]})


def _mutated_empty_kind() -> None:
    """A declared manifest kind that yields nothing. ``_check_registry_yield`` must refuse."""
    harvest = {
        name: [(f"{name} [{'.'.join(keys)}]", "A sentence a registry renders.")]
        for name, keys in _REGISTRY_DESCRIPTION
    }
    harvest.pop(_REGISTRY_DESCRIPTION[0][0])
    _check_registry_yield(harvest)


def _mutated_identifier_field() -> None:
    """A field path re-pointed at an identifier. ``_check_registry_yield`` must refuse."""
    harvest = {
        name: [(f"{name} [{'.'.join(keys)}]", "A sentence a registry renders.")]
        for name, keys in _REGISTRY_DESCRIPTION
    }
    name, keys = _REGISTRY_DESCRIPTION[0]
    harvest[name] = [(f"{name} [{'.'.join(keys)}]", "purrdf-entail")]
    _check_registry_yield(harvest)


def _mutated_skipped_walk() -> None:
    """A deleted loop's answer: zero visits of a set the derivation filled.

    ONE entry for the three arms rather than three, because all three route through the same
    guard and three calls differing only in a label would claim a coverage this does not
    have. What each arm passes IN is checked where it is passed — against ``len(units)``,
    ``len(registered)`` and the surface arithmetic — and none of those is a number its own
    loop computed.
    """
    _check_visited("a deleted walk", 0, 3, "this is the mutation, not the tree")


# One mutation per lever this file has had to have removed, applied to the GUARD'S INPUT
# rather than to the tree. A guard is a conditional, and a conditional nothing ever
# satisfies is the same green light as the loop it was written to protect — `_check_visited`
# compared against a total the loop itself computed would pass forever, and
# `_check_registry_yield` reading the tree it validates could never be shown the tree it
# exists to refuse. So each guard takes its subject as an argument and each mutation below
# hands it the shape the defect really had.
#
# Every entry must raise `SystemExit`. A mutation the guards SURVIVE is a guard that cannot
# see the thing it is named for, and `mutation_self_test` says so by name.
_MUTATIONS: tuple[tuple[str, Callable[[], None]], ...] = (
    (
        "an arm of the documented-surface walk that contributes nothing",
        _mutated_dropped_arm,
    ),
    (
        "a manifest kind whose declared field path yields no description",
        _mutated_empty_kind,
    ),
    (
        "a description field path re-pointed at a bare identifier",
        _mutated_identifier_field,
    ),
    (
        "a subject marker made OPTIONAL in its own pattern",
        _mutated_optional_marker,
    ),
    (
        "a subject marker made one BRANCH of an alternation",
        _mutated_alternating_marker,
    ),
    (
        "a scoped specimen that no longer crosses a line",
        _mutated_same_line_scope,
    ),
    (
        "an arm of the entailment-overclaim ban with its walk deleted",
        _mutated_skipped_walk,
    ),
    (
        "this mutation table itself, narrowed",
        _mutated_narrowed_mutations,
    ),
)

# The floor under the table above, in the same shape as `_BANNED_OVERCLAIMS`'s own floor and
# for the same reason: every table in this file that can be narrowed has been, and the two
# gates one level down both printed a green line about whatever was left. Narrowing this one
# to a single entry — or to `()` — left every gate at exit 0, because the guards still ran and
# only the evidence that they SEE anything went away.
#
# One entry per defect that was live with every gate green, so the floor is the count itself:
# none of these shapes stops being possible, and a table below the floor is a table that has
# forgotten one. Adding a mutation raises it in the same commit.
_MUTATION_FLOOR = 8


def _check_mutation_floor(
    mutations: tuple[tuple[str, Callable[[], None]], ...],
) -> None:
    """The mutation table must keep its entries, and they must be DIFFERENT entries.

    A count alone would be a floor a copy-paste satisfies, so distinctness is asserted with
    it: eight copies of one mutation is one lever proved and seven claimed. Both halves are
    the same argument the ban table's own floor makes — this table is narrowable, narrowing
    it costs no gate its exit code, and what it takes away is the only evidence that any of
    these guards can see the shape it is named for.

    Takes the table rather than reading it, so :data:`_MUTATIONS` can hand it a narrowed one.
    """
    if len(mutations) < _MUTATION_FLOOR:
        raise SystemExit(
            f"check-doc-claims: {len(mutations)} mutation(s) of this gate's own guards, "
            f"below the floor of {_MUTATION_FLOOR}. Each entry is a lever that was live "
            f"with every gate green, and deleting one leaves every gate at exit 0 while the "
            f"guard it exercised stops being exercised at all. Restore it, or lower the "
            f"floor deliberately in the same commit and say which defect stopped being "
            f"possible"
        )
    for kind, seen in (
        ("descriptions", {what for what, _ in mutations}),
        ("callables", {mutate for _, mutate in mutations}),
    ):
        if len(seen) != len(mutations):
            raise SystemExit(
                f"check-doc-claims: the mutation table has {len(mutations)} entries and "
                f"{len(seen)} distinct {kind}, so it is claiming more levers than it "
                f"exercises. A floor a repeated entry satisfies is the hand-written tuple "
                f"this file has already had to remove once"
            )


def mutation_self_test(report: bool) -> list[str]:
    """Every mutation this gate's guards do NOT refuse. An empty list is the passing answer.

    The preflight beside :func:`overclaim_self_test`, and the same argument one level down.
    That one proves the ban answers injected PROSE; this one proves the guards around it
    answer injected STATE — an arm of the surface walk that contributes nothing, a manifest
    kind that publishes nothing, a field path that publishes an identifier, a pattern that
    no longer requires its marker (made optional, or made one branch of an alternation), a
    scoped specimen that stopped crossing a line, a ban arm whose walk was removed, and this
    table itself narrowed to one entry. Every one of those shapes was live, with every gate
    green.

    The table is floored before it is run (:func:`_check_mutation_floor`), because a table of
    mutations is the one table in this file whose narrowing costs no gate its exit code:
    every guard still runs, and only the evidence that any of them can SEE anything goes
    away. That is the same shape as the hand-written tuple this file already had to remove,
    one meta-level up.

    It costs microseconds: no tree is read and no file is written, because each guard takes
    what it judges as an argument.
    """
    _check_mutation_floor(_MUTATIONS)
    survived = [
        what
        for what, mutate in _MUTATIONS
        if _survives(mutate)
    ]
    if report and not survived:
        print(
            f"check-doc-claims: all {len(_MUTATIONS)} mutations of this gate's own guards are "
            f"refused by them."
        )
    return survived


def _survives(mutate: Callable[[], None]) -> bool:
    """Whether `mutate` gets past the guard it is aimed at. ``True`` is the failing answer."""
    try:
        mutate()
    except SystemExit:
        return False
    return True


# The codec's own short `id` has no fixed relationship to the prose name a front page
# uses, so the mapping is spelled out once here rather than guessed per document. A new
# `FormatDescriptor` whose `id` is missing from this map is a `KeyError` in
# `codec_table_claim`, not a silently-ungated format — the same "a tenth format fails this
# on the day it is added" property the media-type table already has.
_CODEC_DISPLAY_NAMES = {
    "turtle": "Turtle",
    "trig": "TriG",
    "ntriples": "N-Triples",
    "nquads": "N-Quads",
    "rdfxml": "RDF/XML",
    "trix": "TriX",
    "hextuples": "HexTuples",
    "jsonld": "JSON-LD",
    "yamlld": "YAML-LD",
}

# The document set `codec_table_claim` gates, the way `extension_disclosure_claim` gates
# the union of rule-coverage documents: the codecs chapter carries the full per-format
# table, and these four front pages restate the format list in prose without a table. All
# four said "seven" (Turtle, TriG, N-Triples, N-Quads, RDF/XML, JSON-LD, YAML-LD) while
# TriX and HexTuples were both full codecs already, which is how a reader could read any
# of the highest-traffic pages in the repository and come away thinking of nine formats
# as seven.
_CODEC_FRONT_PAGES = (
    _README,
    _PURRDF_README,
    _REPO / "crates" / "rdf" / "README.md",
    _INTRODUCTION,
    # The playground is the page a reader is most likely to MEET the codecs on, and its
    # own format lists are what the console renders — so a count there is a claim about
    # behaviour, not only about prose. It was outside this set while the comment above
    # named exactly this failure.
    _REPO / "docs" / "playground" / "engine.worker.mjs",
    _REPO / "docs" / "playground" / "index.html",
)


def codec_listings(text: str) -> list[tuple[str, str]]:
    """Every `const NAME = [ … ];` array in `text` whose members look like format ids.

    A prose page has none and is checked by mention alone; a code page declares the
    format set it actually offers, and each such declaration is a separate claim.
    """
    listings: list[tuple[str, str]] = []
    for match in re.finditer(
        r"const (\w*FORMATS\w*) = \[([\s\S]*?)\];", text
    ):
        listings.append((match.group(1), match.group(2)))
    return listings


def reasoning_session_hosts_claim() -> list[str]:
    """The reasoning session must be reachable from every host, with the same services.

    A capability that exists, is reachable one way and not another, and looks complete
    from each surface on its own is the defect this session was built to remove — the DL
    services were reachable one-shot from four hosts and as a session from none. Fixing it
    on one host would have recreated the same shape one level down.

    So the service set is derived from the shared boundary's own `impl ReasonerSession`
    and required of all four hosts: Rust (the facade re-export), Python (`entail.Reasoner`),
    WASM (the `#[wasm_bindgen] impl` and the published `.d.ts`) and C (the committed ABI
    header). A service added to the boundary and wired to three hosts fails here.
    """
    problems: list[str] = []
    boundary = (_REPO / "crates" / "validate" / "src" / "regime.rs").read_text()

    # The service methods of `impl ReasonerSession`, taken from the block that defines
    # them rather than from a list, so this cannot go stale against the boundary.
    start = boundary.index("impl ReasonerSession {\n    /// See [`consistency_to_string`]")
    end = boundary.index("// \u2500\u2500 The services", start)
    services = sorted(set(re.findall(r"\n    pub fn ([a-z_]+)\(", boundary[start:end])))
    if len(services) < 5:
        raise SystemExit(
            "reasoning_session_hosts_claim found only "
            f"{len(services)} session services, so it is not reading the boundary and "
            "would pass no matter which host omitted one"
        )

    def camel(name: str) -> str:
        head, *rest = name.split("_")
        return head + "".join(part.title() for part in rest)

    def block(text: str, opener: str, closer: str) -> str:
        """The text between `opener` and the next `closer`.

        Every needle below is scoped to the host's session BLOCK rather than run over the
        whole file. Unscoped, a service that exists as a free function and NOT as a method
        would satisfy its own check — which is precisely the defect this claim exists to
        catch, so an unscoped needle would make the gate dark.
        """
        start = text.index(opener)
        return text[start : text.index(closer, start + len(opener))]

    py_native = (_REPO / "bindings" / "python" / "src" / "py_entail.rs").read_text()
    py_stub = (
        _REPO / "bindings" / "python" / "python" / "src" / "purrdf" / "__init__.pyi"
    ).read_text()
    wasm = (_REPO / "crates" / "rdf-wasm" / "src" / "entail.rs").read_text()
    dts = (_REPO / "crates" / "rdf-wasm" / "js" / "index.d.ts").read_text()

    hosts = {
        "Rust facade (crates/purrdf/src/reasoning.rs)": (
            (_REPO / "crates" / "purrdf" / "src" / "reasoning.rs").read_text(),
            lambda n: "pub use purrdf_validate::regime::ReasonerSession;",
        ),
        "Python (bindings/python/src/py_entail.rs)": (
            block(py_native, "impl PyReasoner {", "\n}\n"),
            lambda n: f"fn {n}(",
        ),
        "Python stub (__init__.pyi)": (
            block(py_stub, "class Reasoner:", "\n# "),
            lambda n: f"def {n}(",
        ),
        "WASM (crates/rdf-wasm/src/entail.rs)": (
            block(wasm, "impl Reasoner {", "\n}\n"),
            lambda n: f"fn {n}(",
        ),
        "WASM types (crates/rdf-wasm/js/index.d.ts)": (
            block(dts, "export class Reasoner {", "\n}\n"),
            lambda n: f"{camel(n)}(",
        ),
        "C ABI header (crates/rdf-capi/include/purrdf.h)": (
            (_REPO / "crates" / "rdf-capi" / "include" / "purrdf.h").read_text(),
            lambda n: f"purrdf_reasoner_{n}(",
        ),
    }

    for host, (text, needle) in hosts.items():
        missing = [service for service in services if needle(service) not in text]
        if missing:
            problems.append(
                f"{host}: the reasoning session omits {missing} — every host must reach "
                f"every service the shared boundary defines ({services}), or the capability "
                "is reachable from one caller shape and dark from another"
            )
    return problems


def codec_table_claim() -> tuple[list[str], int]:
    """Every document that spells out the codec list must list every `NativeRdfFormat`.

    The book's codec table listed seven of nine, omitting TriX and HexTuples, and its
    prose said "seven" — internally consistent and wrong about the code. Both are full
    codecs with media types, `classify` aliases and dispatch entries. Derived from
    `FormatDescriptor`'s own table rather than from a count, so a tenth format fails this
    on the day it is added.

    That was one document. Four more front pages (`README.md`, `crates/purrdf/README.md`,
    `crates/rdf/README.md`, `docs/book/src/introduction.md`) enumerate the same list in
    prose, by name rather than by media type, and none of them was checked — so all four
    went stale identically. This walks the whole document SET, the way
    `extension_disclosure_claim` does for the rule-coverage front pages: the codecs
    chapter is checked against the full table (media type presence and star capability),
    and the front pages are checked for the display name of every registered format plus
    any spelled-out format count.
    """
    problems: list[str] = []
    source = _read(
        _REPO / "crates" / "rdf" / "src" / "native_codecs" / "media_type.rs"
    )
    # Each descriptor's id, media type and star capability together, so the table's
    # CONTENT is checked and not merely its row set. Listing every format while
    # mislabelling one is a document that satisfies a presence check and still tells a
    # reader something false.
    descriptors = re.findall(
        r'id: "([^"]+)",[\s\S]*?media_type: "([^"]+)",[\s\S]*?carries_star: (true|false),',
        source,
    )
    if not descriptors:
        raise SystemExit(
            "check-doc-claims: no FormatDescriptor entries found in media_type.rs; the "
            "codec-table claim cannot be checked, so do not leave it unchecked"
        )
    media = [media_type for _, media_type, _ in descriptors]
    unknown_ids = [fid for fid, _, _ in descriptors if fid not in _CODEC_DISPLAY_NAMES]
    if unknown_ids:
        raise SystemExit(
            f"check-doc-claims: media_type.rs registers format id(s) {unknown_ids} with "
            f"no entry in _CODEC_DISPLAY_NAMES — add the prose name a front page uses "
            f"before this claim can check them"
        )

    path = _REPO / "docs" / "book" / "src" / "concepts" / "codecs.md"
    text = _read(path)
    rel = path.relative_to(_REPO)
    for media_type in sorted(set(media)):
        if f"`{media_type}`" not in text:
            problems.append(
                f"{rel}: the codec table omits `{media_type}`, which "
                f"crates/rdf/src/native_codecs/media_type.rs registers as a first-party "
                f"format with its own codec and `classify` aliases"
            )
    # The row's own star column, read from the table and compared with the descriptor.
    for _, media_type, star in descriptors:
        row = re.search(
            r"^\|[^|]*\|\s*`" + re.escape(media_type) + r"`\s*\|\s*([^|]+?)\s*\|$",
            text,
            re.MULTILINE,
        )
        if not row:
            continue  # absence is already reported above
        documented = row.group(1).strip().lower()
        expected = "yes" if star == "true" else "no"
        if documented != expected:
            problems.append(
                f"{rel}: the codec table says `{media_type}` is star-capable "
                f"`{documented}`, but media_type.rs records `carries_star: {star}`"
            )

    spelled = {
        7: "seven", 8: "eight", 9: "nine", 10: "ten", 11: "eleven", 12: "twelve",
    }.get(len(set(media)))

    def _check_spelled_count(doc_text: str, doc_rel: Path) -> None:
        for claim_match in re.finditer(r"for ([a-z]+) formats", doc_text):
            if spelled and claim_match.group(1) != spelled:
                problems.append(
                    f"{doc_rel}: says `{claim_match.group(0)}`, but media_type.rs "
                    f"registers {len(set(media))} ({spelled})"
                )

    _check_spelled_count(text, rel)
    checked = 1

    # Each format is named acceptably by either its DISPLAY name (prose pages) or its
    # format id (code, where the id is the string the engine is actually handed) — the
    # claim is that the page accounts for every registered format, not that it spells it
    # one particular way.
    spellings = sorted(
        (_CODEC_DISPLAY_NAMES[fid], fid) for fid, _, _ in descriptors
    )
    for front_page in _CODEC_FRONT_PAGES:
        front_text = _read(front_page)
        front_rel = front_page.relative_to(_REPO)
        checked += 1
        # A code page declares its formats as ARRAYS, and a format present in one array
        # but missing from another is exactly the gap a whole-file mention test cannot
        # see: the console would offer a format it could parse and not serialize. Each
        # declared list is therefore checked for completeness on its own.
        for list_name, listing in codec_listings(front_text):
            missing_from_list = sorted(
                fid for _, fid in spellings if f'"{fid}"' not in listing
            )
            if missing_from_list:
                problems.append(
                    f"{front_page.relative_to(_REPO)}: the `{list_name}` list omits "
                    f"{', '.join(missing_from_list)}, which "
                    f"crates/rdf/src/native_codecs/media_type.rs registers"
                )
        for name, fid in spellings:
            if name not in front_text and f'"{fid}"' not in front_text:
                problems.append(
                    f"{front_rel}: never names `{name}`, a first-party format "
                    f"crates/rdf/src/native_codecs/media_type.rs registers"
                )
        _check_spelled_count(front_text, front_rel)
    return problems, checked



def regime_count_claim() -> list[str]:
    """A document that names two or more regimes must name all of them.

    `REGIME_NAMES` in `crates/validate/src/regime.rs` is the one accepted set and every host
    routes through it. Two npm READMEs advertised four and five of the seven, understating the
    property that matters most about the surface: materialization is TOTAL over the regimes, and
    no regime is refused for being the regime it is.

    The first attempt matched a `/`-separated run of backticked names. It reached two of the
    four documents it named: the CLI README lists regimes in a MARKDOWN TABLE and the Python
    README in a COMMA run, and neither is a slash run. Rather than enumerate shapes — there is
    always another shape — this counts the regimes a document mentions ANYWHERE and requires
    the set to be empty, a single regime, or complete. A page discussing one regime is fine; a
    page that enumerates is not allowed to enumerate a subset.

    Also checks the spelled-out count, so removing a regime from `REGIME_NAMES` fails every
    document that advertises "all seven", and a vacuity guard, because the version this
    replaces silently checked almost nothing.
    """
    problems: list[str] = []
    boundary = _read(_REPO / "crates" / "validate" / "src" / "regime.rs")
    names = re.search(
        r"pub const REGIME_NAMES: \[&str; \d+\] = \[(.*?)\];", boundary, re.DOTALL
    )
    if not names:
        raise SystemExit(
            "check-doc-claims: cannot read REGIME_NAMES out of crates/validate/src/regime.rs; "
            "the regime-enumeration claim cannot be checked, so do not leave it unchecked"
        )
    accepted = re.findall(r'"([a-z-]+)"', names.group(1))
    spelled = {
        1: "one", 2: "two", 3: "three", 4: "four", 5: "five",
        6: "six", 7: "seven", 8: "eight", 9: "nine", 10: "ten",
    }.get(len(accepted))

    documents = [
        _REPO / "crates" / "rdf-wasm" / "README.md",
        _REPO / "crates" / "rdf-wasm" / "js" / "README.md",
        _CLI_README,
        _PY_README,
        _README,
        _ENTAIL_README,
        _PURRDF_README,
        _CONFORMANCE,
        _ENTAILMENT,
    ]
    # The documents whose PURPOSE is to enumerate the regime surface. A page here that
    # stops recognising at least two regimes has not gone clean — it has been reworded
    # out of this claim's reach, which is how two of them came to advertise four and
    # five of the seven. De-registering one is a deliberate edit to this list.
    must_enumerate = {
        _REPO / "crates" / "rdf-wasm" / "README.md",
        _REPO / "crates" / "rdf-wasm" / "js" / "README.md",
        _CLI_README,
        _PY_README,
    }
    enumerating = 0
    for path in documents:
        if not path.is_file():
            continue
        text = _read(path)
        rel = path.relative_to(_REPO)
        mentioned = {
            regime
            for regime in accepted
            if re.search(r'`"?' + re.escape(regime) + r'"?`', text)
        }
        if path in must_enumerate and len(mentioned) < 2:
            problems.append(
                f"{rel}: recognises {len(mentioned)} regime name(s), below this "
                f"document's floor of 2 — its regime enumeration has been reworded out "
                f"of this claim's reach. Restore the backticked names, or de-register "
                f"the document from must_enumerate in the same commit and say why"
            )
        if len(mentioned) >= 2:
            enumerating += 1
            missing = sorted(set(accepted) - mentioned)
            if missing:
                problems.append(
                    f"{rel}: names {len(mentioned)} of the {len(accepted)} entailment "
                    f"regimes and omits {', '.join(missing)}. A document that enumerates "
                    f"regimes may not enumerate a subset — REGIME_NAMES accepts all of them "
                    f"and every one materializes"
                )
        # A spelled-out total must match the real one, so removing a regime fails here too.
        for claim in re.finditer(
            r"all (?:the )?([A-Z]+|[a-z]+) (?:SPARQL )?(?:entailment )?regimes", text
        ):
            word = claim.group(1).lower()
            if word in {
                "one", "two", "three", "four", "five", "six", "seven", "eight",
                "nine", "ten",
            } and word != spelled:
                problems.append(
                    f"{rel}: claims `{claim.group(0)}`, but REGIME_NAMES has "
                    f"{len(accepted)} ({spelled})"
                )

    if enumerating == 0:
        raise SystemExit(
            "check-doc-claims: no document was found to enumerate regimes at all. The first "
            "version of this claim reached two of four documents for exactly this kind of "
            "reason; fix the extraction rather than leaving the enumerations unchecked"
        )
    return problems


def xfail_ledger_prose_claim(sizes: dict[str, int]) -> list[str]:
    """The prose naming each Python xfail ledger must state its real size.

    The scoreboard ROWS carrying these counts were already gated, so the document
    held the correct numbers and, a hundred lines later, two wrong ones describing
    the same ledgers. A reader has no way to know which of the two the gate covers.
    Both are now derived from `len(ledger["xfail"])`.
    """
    problems: list[str] = []
    text = _read(_CONFORMANCE)
    rel = _CONFORMANCE.relative_to(_REPO)
    expectations = (
        (
            "compat",
            r"`purrdf\.compat` parity ledger \(\*\*(?P<n>\d+)\*\* strict xfails?\)",
            _COMPAT_LEDGER,
        ),
        (
            "rdflib",
            r"vendored tests \(\*\*(?P<n>\d+)\*\* strict\s+xfails?\)",
            _RDFLIB_LEDGER,
        ),
    )
    for key, pattern, ledger in expectations:
        found = re.findall(pattern, text)
        if len(found) != 1:
            problems.append(
                f"{rel}: the {key} xfail-ledger prose — expected exactly one match, "
                f"found {len(found)}. The sentence was reworded or removed; update "
                f"the pattern in scripts/check-doc-claims.py so the claim stays "
                f"checked (pattern: {pattern})"
            )
            continue
        if _int(found[0]) != sizes[key]:
            problems.append(
                f"{rel}: the {key} xfail-ledger prose says {found[0]} strict xfail(s) "
                f"but {ledger.relative_to(_REPO)} holds {sizes[key]}"
            )
    return problems


def extension_disclosure_claim(extensions: list[str]) -> list[str]:
    """A document that publishes the coverage table must NAME what exceeds it.

    The coverage claim above compels every such document to restate the defined
    and implemented counts, and nothing compelled it to mention a rule that fires
    outside both. That combination is worse than an unchecked document: a gate
    holds the number continuously true while the sentence around it stays
    materially incomplete, so a reader who trusts the gate is misled by exactly
    the part it verifies. Requiring the extension NAMES rather than a count keeps
    this structural — a second extension fails this the day it lands.

    The document set is the UNION of the tables discovered structurally and the
    registered READMEs that state the rule-table claim in prose. Restricting it to
    table-carriers alone would leave the front pages that say "all 78 rules"
    without publishing a table — the highest-traffic prose in the repository —
    ungated.
    """
    problems: list[str] = []
    documents = sorted(
        set(rule_coverage_documents())
        | {_README, _ENTAIL_README, _PURRDF_README, _CLI_README, _PY_README}
    )
    for path in documents:
        rel = path.relative_to(_REPO)
        text = _read(path)
        for rule in extensions:
            if f"`{rule}`" not in text:
                problems.append(
                    f"{rel}: states the rule-table coverage but never names "
                    f"`{rule}`, a rule this build fires that no specification "
                    f"table states. A document that publishes the coverage "
                    f"counts must also disclose what fires beyond them, or it is "
                    f"a continuously-verified number wrapped in an incomplete "
                    f"sentence (docs/book/src/entailment-rules.md "
                    f"'## Extensions')"
                )
    return problems


# ---------------------------------------------------------------------------
# Source 2 — the generated conformance-matrix block
# ---------------------------------------------------------------------------


def load_matrix() -> dict[str, tuple[int, int]]:
    """Suite name -> (pass, xfail/skip), read from the generated matrix block.

    ``conformance-matrix.py`` refuses to pass unless this block equals a fresh
    full harness run, so the block is the committed measurement.
    """
    text = _read(_CONFORMANCE)
    if _MATRIX_BEGIN not in text or _MATRIX_END not in text:
        raise SystemExit(
            f"check-doc-claims: matrix markers not found in "
            f"{_CONFORMANCE.relative_to(_REPO)}"
        )
    inner = text[
        text.index(_MATRIX_BEGIN) + len(_MATRIX_BEGIN) : text.index(_MATRIX_END)
    ]
    suites: dict[str, tuple[int, int]] = {}
    for line in inner.splitlines():
        cells = [c.strip() for c in line.strip().strip("|").split("|")]
        if len(cells) != 7 or not cells[2].isdigit():
            continue
        suites[cells[0]] = (int(cells[2]), int(cells[3]))
    if not suites:
        raise SystemExit(
            f"check-doc-claims: could not parse any suite row out of the "
            f"generated block in {_CONFORMANCE.relative_to(_REPO)}"
        )
    return suites


def load_governor_corpus_counts() -> dict[str, int]:
    """Case counts derived from the frozen governor-corpus manifest.

    ``manifest.tsv`` is byte-frozen (``scripts/check-corpus-frozen.py``), and its
    ``band`` column already distinguishes a zero/boundary/over-bound case from a
    seam case (``n/a`` — see the corpus's own README). Deriving the counts from
    here, rather than restating them a third and fourth time, is what lets
    CONFORMANCE.md's hand-written scoreboard prose and SPARQL-GOVERNOR-PROFILE.md
    §11 disagree LOUDLY the moment a lane is added, instead of just going stale:
    `--write-doc` regenerates CONFORMANCE.md's matrix block but touches neither of
    those prose sites.
    """
    lines = [
        line
        for line in _read(_GOVERNOR_MANIFEST).splitlines()
        if line and not line.startswith("#")
    ]
    if not lines:
        raise SystemExit(
            f"check-doc-claims: no case rows in "
            f"{_GOVERNOR_MANIFEST.relative_to(_REPO)}"
        )
    total = len(lines)
    band = sum(
        1
        for line in lines
        if line.split("\t")[5] in {"zero", "boundary", "over-bound"}
    )
    return {"total": total, "band": band, "seam": total - band}


def governor_corpus_count_claim(matrix: dict[str, tuple[int, int]]) -> list[str]:
    """The governor-corpus case count, restated by hand in two documents.

    CONFORMANCE.md's per-engine scoreboard row and SPARQL-GOVERNOR-PROFILE.md §11
    each restate the corpus's total case count and its zero/boundary/over-bound
    (band) vs. seam split in prose that `conformance-matrix.py --write-doc` never
    touches — it only rewrites CONFORMANCE.md's generated matrix block. This claim
    checks the generated block's own governor-suite pass count AND both prose
    restatements against the one source that cannot drift on its own: the frozen
    manifest itself.
    """
    counts = load_governor_corpus_counts()
    problems: list[str] = []

    matrix_pass, _ = matrix["SPARQL execution governors"]
    if matrix_pass != counts["total"]:
        problems.append(
            f"{_CONFORMANCE.relative_to(_REPO)}: the generated matrix block "
            f"reports {matrix_pass} governor cases, but "
            f"{_GOVERNOR_MANIFEST.relative_to(_REPO)} has {counts['total']}"
        )

    total_band_sites: list[tuple[Path, str]] = [
        (
            _CONFORMANCE,
            _flow(
                r"\*\*(?P<total>\d+) / (?P<total2>\d+)\*\* pinned cases · 0 ledgered\. "
                r"\*\*(?P<band>\d+) of them are band cases\*\*"
            ),
        ),
        (
            _GOVERNOR_PROFILE,
            _flow(
                r"Across the corpus there are (?P<total>\d+) cases total, of which "
                r"(?P<band>\d+) form zero, boundary, or over-bound lanes"
            ),
        ),
    ]
    for path, pattern in total_band_sites:
        rel = path.relative_to(_REPO)
        matches = list(re.finditer(pattern, _read(path)))
        if len(matches) != 1:
            problems.append(
                f"{rel}: expected exactly one governor-corpus case-count claim "
                f"matching the pattern, found {len(matches)}.\n    pattern: {pattern}"
            )
            continue
        found = matches[0].groupdict()
        for group in ("total", "total2"):
            if group in found and _int(found[group]) != counts["total"]:
                problems.append(
                    f"{rel}: documented {found[group]} governor cases, "
                    f"{_GOVERNOR_MANIFEST.relative_to(_REPO)} has {counts['total']}"
                )
        if _int(found["band"]) != counts["band"]:
            problems.append(
                f"{rel}: documented {found['band']} governor band cases, "
                f"{_GOVERNOR_MANIFEST.relative_to(_REPO)} has {counts['band']}"
            )

    seam_sites: list[tuple[Path, str]] = [
        (_CONFORMANCE, _flow(r"The remaining (?P<seam>\d+) cases name seams")),
        (
            _GOVERNOR_PROFILE,
            _flow(
                r"the\s+remaining (?P<seam>\d+) are transport, relation, "
                r"charge-seam, `EXISTS`-evidence, and wall-clock cases"
            ),
        ),
    ]
    for path, pattern in seam_sites:
        rel = path.relative_to(_REPO)
        matches = list(re.finditer(pattern, _read(path)))
        if len(matches) != 1:
            problems.append(
                f"{rel}: expected exactly one governor-corpus seam-count claim "
                f"matching the pattern, found {len(matches)}.\n    pattern: {pattern}"
            )
            continue
        got = _int(matches[0].group("seam"))
        if got != counts["seam"]:
            problems.append(
                f"{rel}: documented {got} seam cases, "
                f"{_GOVERNOR_MANIFEST.relative_to(_REPO)} has {counts['seam']}"
            )
    return problems


def load_governor_schedule_source() -> tuple[str, int, list[tuple[str, int]]]:
    """``(GOVERNOR_PROFILE_ID, GOVERNOR_PROFILE_VERSION, CHARGE_SCHEDULE)`` as the
    engine defines them, in ``crates/sparql-eval/src/governor/mod.rs``.

    Parsed straight from the Rust source rather than restated, so a schedule
    change — a point added, removed, renamed, or repriced, or the version bumped
    without it — cannot leave SPARQL-GOVERNOR-PROFILE.md §10's recipe or its
    pinned digest comment behind without this check noticing. That is exactly
    the failure mode that let §10 print a v6 preimage and a v6 digest under a
    v7 heading listing 16 charge points.
    """
    text = _read(_GOVERNOR_SOURCE)
    rel = _GOVERNOR_SOURCE.relative_to(_REPO)

    id_match = re.search(r'pub const GOVERNOR_PROFILE_ID: &str = "([^"]+)";', text)
    if not id_match:
        raise SystemExit(f"check-doc-claims: no `GOVERNOR_PROFILE_ID` constant in {rel}")

    version_match = re.search(r"pub const GOVERNOR_PROFILE_VERSION: u32 = (\d+);", text)
    if not version_match:
        raise SystemExit(
            f"check-doc-claims: no `GOVERNOR_PROFILE_VERSION` constant in {rel}"
        )

    body_match = re.search(
        r"pub const CHARGE_SCHEDULE: \[\(&str, u64\); \d+\] = \[(.*?)\n\];",
        text,
        re.DOTALL,
    )
    if not body_match:
        raise SystemExit(f"check-doc-claims: no `CHARGE_SCHEDULE` table in {rel}")
    body = body_match.group(1)
    entries = re.findall(r'\(\s*"([^"]+)"\s*,\s*(\d+)\s*\)', body)
    # Every tuple entry in the body must have been read. Counting the construct
    # separately is what keeps "the table shrank" from looking like "the table
    # is fine but the pattern missed some rows".
    declared = len(re.findall(r'\(\s*"', body))
    if declared != len(entries):
        raise SystemExit(
            f"check-doc-claims: {rel} declares {declared} `CHARGE_SCHEDULE` "
            f"entries but only {len(entries)} parsed; the entry shape changed, "
            f"so update the pattern in load_governor_schedule_source()"
        )
    schedule = [(label, int(cost)) for label, cost in entries]
    return id_match.group(1), int(version_match.group(1)), schedule


def load_governor_profile_recipe() -> tuple[str, int, list[str], str]:
    """``(id, version, label list, pinned digest)`` from SPARQL-GOVERNOR-PROFILE.md
    §10's fenced recipe — the one place a consumer is told "a consumer can
    therefore recompute it from this document alone".

    Parsed rather than restated, so the recipe cannot silently drift from what
    it claims to reproduce.
    """
    text = _read(_GOVERNOR_PROFILE)
    rel = _GOVERNOR_PROFILE.relative_to(_REPO)
    section_match = re.search(
        r"## 10\. Profile identity.*?```sh\n(.*?)\n```", text, re.DOTALL
    )
    if not section_match:
        raise SystemExit(
            f"check-doc-claims: no fenced recipe under the '## 10. Profile "
            f"identity' heading in {rel}"
        )
    block = section_match.group(1)

    header_match = re.search(r"printf '([^\\']+)\\n(\d+)\\n'", block)
    if not header_match:
        raise SystemExit(
            f"check-doc-claims: could not find the `printf '<id>\\n<version>\\n'` "
            f"line in the §10 recipe in {rel}"
        )
    doc_id = header_match.group(1)
    doc_version = int(header_match.group(2))

    labels_match = re.search(r"printf '%s\\t1\\n'(.*?)\n\}", block, re.DOTALL)
    if not labels_match:
        raise SystemExit(
            f"check-doc-claims: could not find the `printf '%s\\t1\\n' ...` "
            f"label line in the §10 recipe in {rel}"
        )
    doc_labels = labels_match.group(1).replace("\\\n", " ").split()

    digest_match = re.search(r"\n#\s*([0-9a-f]{64})\s*\n```", section_match.group(0))
    if not digest_match:
        raise SystemExit(
            f"check-doc-claims: could not find the pinned `# <hex digest>` "
            f"comment closing the §10 recipe in {rel}"
        )
    doc_digest = digest_match.group(1)

    return doc_id, doc_version, doc_labels, doc_digest


def _schedule_digest(id_: str, version: int, schedule: list[tuple[str, int]]) -> str:
    """The lowercase-hex SHA-256 of the canonical preimage, mirroring
    ``schedule_preimage``/``schedule_digest`` in
    ``crates/sparql-eval/src/governor/mod.rs``: the id, then the version, then
    one ``label\\tcost`` line per schedule entry, every line ``\\n``-terminated.
    """
    preimage = f"{id_}\n{version}\n"
    for label, cost in schedule:
        preimage += f"{label}\t{cost}\n"
    return hashlib.sha256(preimage.encode()).hexdigest()


def governor_profile_digest_claim() -> list[str]:
    """SPARQL-GOVERNOR-PROFILE.md §10's recipe must reproduce
    ``GOVERNOR_PROFILE_DIGEST`` exactly: the same id, the same version, the
    same label list in the same order, and the same pinned hex digest as
    ``crates/sparql-eval/src/governor/mod.rs``, the one place those four
    constants are defined.

    A stale version, a stale label list, and a stale digest are reported as
    three DISTINCT failures — each is independently fixable, and folding them
    into one "the recipe disagrees" message would hide which of the three
    moved. §10's own recipe is also re-run (not just re-derived) against its
    own pinned comment, which is what catches a digest comment that was hand-
    edited to the wrong value even when the id/version/labels above it are
    all current.
    """
    rel = _GOVERNOR_PROFILE.relative_to(_REPO)
    src = _GOVERNOR_SOURCE.relative_to(_REPO)
    problems: list[str] = []

    engine_id, engine_version, engine_schedule = load_governor_schedule_source()
    doc_id, doc_version, doc_labels, doc_digest = load_governor_profile_recipe()

    # The recipe's label `printf` hardcodes a literal `1` cost for every entry. That
    # is only a valid shorthand for the engine's actual schedule while every entry
    # really does cost 1 — the moment one does not, the recipe's shell one-liner can
    # no longer reproduce the digest at all, which is a defect this claim must catch
    # rather than silently mis-price.
    non_unit = [(label, cost) for label, cost in engine_schedule if cost != 1]
    if non_unit:
        problems.append(
            f"{src}: `CHARGE_SCHEDULE` prices {non_unit} at other than 1, but "
            f"the §10 recipe in {rel} hardcodes `1` for every label via "
            f"`printf '%s\\t1\\n'` — the recipe needs a per-label cost, not "
            f"just a label list, or it can no longer reproduce the digest"
        )

    if doc_id != engine_id:
        problems.append(
            f"{rel}: the §10 recipe's id is {doc_id!r}, but `GOVERNOR_PROFILE_ID` "
            f"in {src} is {engine_id!r}"
        )

    if doc_version != engine_version:
        problems.append(
            f"{rel}: the §10 recipe prints version {doc_version}, but "
            f"`GOVERNOR_PROFILE_VERSION` in {src} is {engine_version} — the "
            f"recipe's preimage no longer matches the version this document's "
            f"own '## 10. Profile identity' table declares"
        )

    engine_labels = [label for label, _ in engine_schedule]
    if doc_labels != engine_labels:
        problems.append(
            f"{rel}: the §10 recipe's label list is stale — it names "
            f"{doc_labels} ({len(doc_labels)} label(s)), but `CHARGE_SCHEDULE` "
            f"in {src} is, in table order, {engine_labels} "
            f"({len(engine_labels)} label(s))"
        )

    # The digest is checked against BOTH the recipe's own inputs (does the recipe,
    # run as written, actually produce the comment pinned under it?) and the
    # freshly-parsed engine schedule (does that comment match what the engine ships
    # today?). The two can disagree independently: a hand-edited digest comment
    # fails only the first, and a self-consistent recipe frozen at an old version
    # fails only the second — which is exactly how §10 went stale, since its v6
    # recipe and v6 digest comment agreed with EACH OTHER the whole time.
    recomputed = _schedule_digest(
        doc_id, doc_version, [(label, 1) for label in doc_labels]
    )
    if doc_digest != recomputed:
        problems.append(
            f"{rel}: the §10 recipe's pinned digest comment is {doc_digest}, but "
            f"running the recipe exactly as written (id={doc_id!r}, "
            f"version={doc_version}, labels={doc_labels}) gives {recomputed}"
        )
    engine_digest = _schedule_digest(engine_id, engine_version, engine_schedule)
    if doc_digest != engine_digest:
        problems.append(
            f"{rel}: the §10 recipe's pinned digest comment is {doc_digest}, but "
            f"`GOVERNOR_PROFILE_DIGEST` recomputed from the current "
            f"`CHARGE_SCHEDULE` in {src} is {engine_digest}"
        )

    return problems


# ---------------------------------------------------------------------------
# Source 3 — the frozen upstream OWL 2 census
# ---------------------------------------------------------------------------


def load_census() -> list[dict[str, str]]:
    """Every upstream ``otest:TestCase``, one row each, from ``census.tsv``.

    The census is *derived* from the W3C manifest
    (<https://www.w3.org/2009/11/owl-test/all.rdf>), byte-frozen by
    ``scripts/check-corpus-frozen.py``, and cross-checked against both vendored
    corpora's directory listings by
    ``owl2_rl_conformance.rs::census_accounts_for_every_upstream_case`` — so a
    row cannot claim a case is graded while the payload is absent, nor the
    reverse. That makes it a legitimate generated source for every count the
    prose makes about *what W3C published* as opposed to what PurRDF scored.
    """
    lines = _read(_CENSUS).splitlines()
    header = lines[0].split("\t")
    rows = [dict(zip(header, line.split("\t"), strict=True)) for line in lines[1:] if line]
    if not rows:
        raise SystemExit(f"check-doc-claims: no rows in {_CENSUS.relative_to(_REPO)}")
    return rows


def census_counts() -> dict[str, int]:
    """The upstream tallies the documentation restates, all from the census."""
    rows = load_census()
    types: dict[str, int] = {}
    for row in rows:
        for kind in row["otest_types"].split(";"):
            types[kind] = types.get(kind, 0) + 1
    probe: dict[str, int] = {}
    for row in rows:
        probe[row["dl_probe"]] = probe.get(row["dl_probe"], 0) + 1
    rl: dict[str, int] = {}
    for row in rows:
        rl[row["rl_corpus"]] = rl.get(row["rl_corpus"], 0) + 1

    consistency_shaped = types.get("ConsistencyTest", 0) + types.get(
        "InconsistencyTest", 0
    )
    dl_graded = sum(1 for r in rows if r["dl_corpus"] == "graded")
    decides = probe.get("decides-consistent", 0) + probe.get("decides-inconsistent", 0)
    withholds = probe.get("withholds-reasoner", 0) + probe.get("withholds-parse", 0)
    return {
        "upstream_cases": len(rows),
        "positive_entailment": types.get("PositiveEntailmentTest", 0),
        "negative_entailment": types.get("NegativeEntailmentTest", 0),
        "consistency_shaped": consistency_shaped,
        "dl_graded": dl_graded,
        "dl_excluded": consistency_shaped - dl_graded,
        "dl_decides": decides,
        "dl_decides_consistent": probe.get("decides-consistent", 0),
        "dl_decides_inconsistent": probe.get("decides-inconsistent", 0),
        "dl_non_terminating": probe.get("non-terminating", 0),
        "dl_withholds": withholds,
        "dl_withholds_reasoner": probe.get("withholds-reasoner", 0),
        "dl_withholds_parse": probe.get("withholds-parse", 0),
        "dl_no_premise": probe.get("no-rdfxml-premise", 0),
        "rl_positive": rl.get("graded-positive", 0),
        "rl_negative": rl.get("graded-negative", 0),
    }


# ---------------------------------------------------------------------------
# Source 4 — the OWL 2 RL divergence ledger
# ---------------------------------------------------------------------------

# `RlGap` variant -> (census token it must be a divergence of, actionable?).
# Mirrors `RlGap::is_actionable` in crates/sparql-conformance/src/owl2_rl.rs;
# a variant added there without a row here is a hard failure below, so the two
# cannot drift apart silently.
_RL_GAP_ACTIONABLE: dict[str, bool] = {
    "MissingRule": True,
    "UnsoundDerivation": True,
    "SchemaConclusion": False,
    "NegativeConclusion": False,
    "ConstructOutsideRl": False,
    "ImportsUnresolved": False,
    "Refused": False,
}


def load_rl_ledger() -> list[tuple[str, str]]:
    """``(case, RlGap variant)`` for every entry of ``owl2_rl::LEDGER``.

    The harness asserts ``unledgered == 0`` and ``stale == 0``, so this table is
    *exactly* the set of vendored entailment cases whose verdict diverges from
    W3C's. That is what lets the per-lane split below be derived rather than
    asserted.

    **An EMPTY ledger is a legitimate answer**, and telling it apart from a
    parse failure is this function's only subtlety. The table is now empty — all
    50 vendored cases agree — so a blanket "could not parse any entry" refusal
    would fail the gate on the very state the gate exists to reach. What is still
    fatal is the table being *missing* (a rename this script did not follow), or
    the body holding ``LedgerEntry`` text that the entry pattern cannot read (a
    reshaped entry, which would silently under-count the ledger). Both are
    checked; an empty body is not.
    """
    text = _read(_RL_LEDGER)
    body = re.search(
        r"pub const LEDGER: &\[LedgerEntry\] = &\[(.*?)\n\];", text, re.DOTALL
    )
    if not body:
        raise SystemExit(
            f"check-doc-claims: no `LEDGER` table in {_RL_LEDGER.relative_to(_REPO)}"
        )
    entries = re.findall(
        r"case:\s*\"([^\"]+)\",\s*\n?\s*gap:\s*RlGap::(\w+)", body.group(1)
    )
    # Every `LedgerEntry` in the body must have been read. Counting the construct
    # separately is what keeps "the table is empty" and "the entries changed shape"
    # from looking alike.
    declared = len(re.findall(r"\bLedgerEntry\s*\{", body.group(1)))
    if declared != len(entries):
        raise SystemExit(
            f"check-doc-claims: {_RL_LEDGER.relative_to(_REPO)} declares {declared} "
            f"`LedgerEntry` value(s) but only {len(entries)} parsed; the entry shape "
            f"changed, so update the pattern in load_rl_ledger()"
        )
    return entries


def rl_lane_counts() -> dict[str, int]:
    """Per-lane agreement and the typed-gap tally, derived from ledger × census.

    Every divergence is a ledger entry and every ledger entry is a divergence
    (the harness fails otherwise), and the census says which lane each case is
    in — so ``agreeing = graded - ledgered`` per lane is a derivation from two
    gate-verified artifacts, not an assumption that the negative lane is clean.
    """
    ledger = load_rl_ledger()
    lane = {r["case"]: r["rl_corpus"] for r in load_census()}
    counts = census_counts()

    unknown = sorted({gap for _, gap in ledger} - set(_RL_GAP_ACTIONABLE))
    if unknown:
        raise SystemExit(
            f"check-doc-claims: LEDGER uses RlGap variant(s) {unknown} that "
            f"scripts/check-doc-claims.py does not classify; add them to "
            f"_RL_GAP_ACTIONABLE (mirroring RlGap::is_actionable)"
        )

    gaps: dict[str, int] = {}
    ledgered_positive = ledgered_negative = 0
    for case, gap in ledger:
        gaps[gap] = gaps.get(gap, 0) + 1
        where = lane.get(case)
        if where == "graded-positive":
            ledgered_positive += 1
        elif where == "graded-negative":
            ledgered_negative += 1
        else:
            raise SystemExit(
                f"check-doc-claims: LEDGER names {case!r}, which the census "
                f"records as {where!r} rather than a graded entailment case"
            )

    positive_agree = counts["rl_positive"] - ledgered_positive
    negative_agree = counts["rl_negative"] - ledgered_negative
    return {
        "positive_total": counts["rl_positive"],
        "negative_total": counts["rl_negative"],
        "positive_agree": positive_agree,
        "negative_agree": negative_agree,
        "ledgered": len(ledger),
        "actionable": sum(n for g, n in gaps.items() if _RL_GAP_ACTIONABLE[g]),
        "structural": sum(n for g, n in gaps.items() if not _RL_GAP_ACTIONABLE[g]),
        "missing_rule": gaps.get("MissingRule", 0),
        "schema_conclusion": gaps.get("SchemaConclusion", 0),
        "negative_conclusion": gaps.get("NegativeConclusion", 0),
        "construct_outside_rl": gaps.get("ConstructOutsideRl", 0),
        "imports_unresolved": gaps.get("ImportsUnresolved", 0),
    }


# The bucket of the normative rule table itself. Every OTHER bucket on the mechanism
# line is a lane that exists because the table decides no conclusion of that shape —
# what the prose calls "the five" — so "beyond the table" is this name's complement
# rather than a list repeated here that could fall behind `EntailmentMechanism::ALL`.
_RL_TABLE_MECHANISM = "strict-table"


def rl_mechanism_counts() -> dict[str, int]:
    """The `OWL2-RL-MECHANISMS` split, read from the line the harness pins verbatim.

    The harness RECOMPUTES that line from the corpus on every run and its `assert_eq!`
    fails the moment a case moves between two mechanisms, so the pinned string is a
    measurement rather than a second copy of a number — which is what makes it a
    legitimate source for a claim, unlike the prose the claim checks.

    Returns the corpus total, and the POSITIVE count reached by a mechanism other
    than the rule table. A positive count on such a lane is a conclusion that lane
    ESTABLISHED; a negative one is an admission it could not read the construct, so
    summing the two would not be the number the prose is about.
    """
    text = _read(_RL_MECHANISM_PIN)
    literal = re.search(r'"(OWL2-RL-MECHANISMS:[^"]*)"', text, re.DOTALL)
    if not literal:
        raise SystemExit(
            f"check-doc-claims: no pinned `OWL2-RL-MECHANISMS` string literal in "
            f"{_RL_MECHANISM_PIN.relative_to(_REPO)}; the harness assertion was "
            f"reworded or removed, so update rl_mechanism_counts()"
        )
    # A Rust string literal wraps with a trailing backslash and re-indents, which is
    # whitespace the line itself does not have.
    line = re.sub(r"\\\s*", " ", literal.group(1))
    buckets = {
        name: (int(positive), int(negative))
        for name, positive, negative in re.findall(
            r"([a-z][a-z-]*) (\d+)/(\d+)", line.split(") ", 1)[-1]
        )
    }
    withheld = re.search(r"withheld (\d+)", line)
    if _RL_TABLE_MECHANISM not in buckets or not withheld:
        raise SystemExit(
            f"check-doc-claims: the pinned `OWL2-RL-MECHANISMS` line in "
            f"{_RL_MECHANISM_PIN.relative_to(_REPO)} has no "
            f"{_RL_TABLE_MECHANISM!r} bucket or no `withheld` residue; its shape "
            f"changed, so update rl_mechanism_counts()"
        )
    return {
        "total": sum(p + n for p, n in buckets.values()) + int(withheld.group(1)),
        "beyond_table_positive": sum(
            positive
            for name, (positive, _) in buckets.items()
            if name != _RL_TABLE_MECHANISM
        ),
    }


# ---------------------------------------------------------------------------
# Source 5 — the release crate set
# ---------------------------------------------------------------------------


def load_release_crates() -> list[str]:
    """The publish-ordered release set defined in scripts/release-crates.sh."""
    text = _read(_RELEASE_CRATES)
    body = re.search(r"PURRDF_RELEASE_CRATES=\((.*?)\)", text, re.DOTALL)
    if not body:
        raise SystemExit(
            f"check-doc-claims: no PURRDF_RELEASE_CRATES array in "
            f"{_RELEASE_CRATES.relative_to(_REPO)}"
        )
    return [line.strip() for line in body.group(1).split() if line.strip()]


# ---------------------------------------------------------------------------
# Claims
# ---------------------------------------------------------------------------


@dataclass
class Claim:
    """One documented number that must equal a generated one."""

    what: str
    path: Path
    pattern: str
    expected: dict[str, int]
    source: str
    failures: list[str] = field(default_factory=list)

    def check(self) -> bool:
        text = _read(self.path)
        matches = list(re.finditer(self.pattern, text))
        rel = self.path.relative_to(_REPO)
        # A capture group with no expected value is a number that LOOKS gated
        # and is not — the precise failure mode this whole script exists to
        # prevent. An expected key with no group is a claim checking nothing.
        groups = set(re.compile(self.pattern).groupindex)
        if groups != set(self.expected):
            self.failures.append(
                f"{rel}: {self.what} — the pattern captures {sorted(groups)} but "
                f"expects {sorted(self.expected)}; every captured number must "
                f"have a measured value and vice versa"
            )
            return False
        if len(matches) != 1:
            self.failures.append(
                f"{rel}: {self.what} — expected exactly one match for the "
                f"documented claim, found {len(matches)}. The row was reworded "
                f"or removed; update the pattern in scripts/check-doc-claims.py "
                f"so the claim stays checked.\n    pattern: {self.pattern}"
            )
            return False
        found = matches[0].groupdict()
        ok = True
        for group, want in self.expected.items():
            got = _int(found[group])
            if got != want:
                ok = False
                self.failures.append(
                    f"{rel}: {self.what} — documented {group}={got}, "
                    f"measured {want} ({self.source})"
                )
        return ok


# The header row of a hand-written rule-coverage table. Documents are found by
# THIS, not by a heading and not by a hard-coded path list: a coverage table
# cannot exist without its own header, so renaming the section above it (or
# adding a table to a crate README nobody remembered to register here) cannot
# drop the check. That generality is not decorative — `crates/entail/README.md`
# stated `RDF 3 | 1` and `RDFS 18 | 14` for as long as this function read only
# `docs/book/src/entailment.md`, while the script printed that every claim agreed.
_COVERAGE_HEADER = "| Regime | Rule table | Defined | Implemented |"

# Where a coverage table may live: the book, the published crate READMEs, the
# binding READMEs, and the repository front page. Every `.md` under `docs/` is
# swept, so a new chapter is covered without an edit here.
def _coverage_candidates() -> list[Path]:
    return sorted(
        {
            _REPO / "README.md",
            *(_REPO / "crates").glob("*/README.md"),
            *(_REPO / "bindings").glob("*/README.md"),
            *(_REPO / "docs").rglob("*.md"),
        }
    )


def rule_coverage_documents() -> list[Path]:
    """Every in-tree document that restates the generated rule-coverage table."""
    found = [
        path
        for path in _coverage_candidates()
        if path.is_file() and _COVERAGE_HEADER in _read(path)
    ]
    if not found:
        raise SystemExit(
            f"check-doc-claims: no document carries the rule-coverage header "
            f"{_COVERAGE_HEADER!r}. Either every coverage table was deleted or "
            f"the header was reworded; update _COVERAGE_HEADER rather than "
            f"leaving the tables ungated."
        )
    return found


def _coverage_tables(text: str) -> list[dict[str, tuple[int, int]]]:
    """Every ``regime -> (defined, implemented)`` table in one document.

    A table starts at ``_COVERAGE_HEADER`` and runs to the first line that is not
    a table row, so the alignment row is skipped and trailing prose ends it.
    """
    lines = text.splitlines()
    tables: list[dict[str, tuple[int, int]]] = []
    for index, line in enumerate(lines):
        if line.strip() != _COVERAGE_HEADER:
            continue
        rows: dict[str, tuple[int, int]] = {}
        for row in lines[index + 1 :]:
            if not row.startswith("|"):
                break
            match = re.match(
                r"^\| `([A-Za-z-]+)` \| [^|]* \| (\d+) \| (\d+) \|$", row
            )
            if match:
                rows[match.group(1)] = (int(match.group(2)), int(match.group(3)))
        tables.append(rows)
    return tables


def rule_coverage_table_claims(
    inventory: dict[str, tuple[int, int]],
) -> tuple[list[str], int]:
    """Every hand-written 'Rule coverage' table, wherever it is published.

    Checked structurally rather than by regex: the SET of regimes must match the
    inventory too, so a regime added to ``Regime`` cannot be quietly omitted from
    a table, and one deleted cannot linger. Returns the problems and the number
    of tables checked, so the script's claim count reports what it really read.
    """
    problems: list[str] = []
    checked = 0
    for path in rule_coverage_documents():
        rel = path.relative_to(_REPO)
        tables = _coverage_tables(_read(path))
        for table in tables:
            checked += 1
            if not table:
                problems.append(
                    f"{rel}: a rule-coverage table's rows could not be parsed — "
                    f"every row must read ``| `Regime` | table | defined | "
                    f"implemented |``"
                )
                continue
            for name in sorted(set(inventory) - set(table)):
                problems.append(
                    f"{rel}: the Rule coverage table has no row for regime "
                    f"`{name}`, which docs/book/src/entailment-rules.md defines"
                )
            for name in sorted(set(table) - set(inventory)):
                problems.append(
                    f"{rel}: the Rule coverage table has a row for regime "
                    f"`{name}`, which docs/book/src/entailment-rules.md does not "
                    f"define"
                )
            for name in sorted(set(table) & set(inventory)):
                if table[name] != inventory[name]:
                    d_def, d_impl = table[name]
                    g_def, g_impl = inventory[name]
                    problems.append(
                        f"{rel}: Rule coverage row `{name}` documents "
                        f"{d_def} defined / {d_impl} implemented, but "
                        f"rules()/implemented() report {g_def} / {g_impl} "
                        f"(docs/book/src/entailment-rules.md)"
                    )
    return problems, checked


# The `purrdf.entail` entry points that are the CHASE lane rather than the OWL 2
# Direct-Semantics reasoner. Everything else the type stub declares on
# `class entail` is a reasoning service, and the Python README documents each one
# in its service table. Mirrors the stub's own two-block layout.
_CHASE_ENTRY_POINTS = frozenset(
    {"materialize", "materialize_nt", "rules", "implemented_rules", "extensions"}
)


def load_py_entail_services() -> list[str]:
    """Every Description-Logic service `purrdf.entail` declares, from the stub.

    ``bindings/python/python/src/purrdf/__init__.pyi`` is the committed, typed
    declaration of the Python surface — the artifact mypy checks call sites
    against — so it is what "reachable from Python" means in-tree. Reading the
    stub keeps this script text-over-committed-files: no import of a built wheel.
    """
    text = _read(_PY_STUB)
    body = re.search(r"\nclass entail:\n(.*?)(?:\n\S|\Z)", text, re.DOTALL)
    if not body:
        raise SystemExit(
            f"check-doc-claims: no `class entail:` block in "
            f"{_PY_STUB.relative_to(_REPO)}"
        )
    declared = re.findall(r"^    def (\w+)\(", body.group(1), re.MULTILINE)
    if not declared:
        raise SystemExit(
            f"check-doc-claims: `class entail:` in {_PY_STUB.relative_to(_REPO)} "
            f"declares no methods"
        )
    unknown = _CHASE_ENTRY_POINTS - set(declared)
    if unknown:
        raise SystemExit(
            f"check-doc-claims: _CHASE_ENTRY_POINTS names {sorted(unknown)}, "
            f"which {_PY_STUB.relative_to(_REPO)} no longer declares; update the "
            f"chase/reasoner split rather than leaving the service table ungated"
        )
    return [name for name in declared if name not in _CHASE_ENTRY_POINTS]


def py_service_table_claim(services: list[str]) -> list[str]:
    """The Python README must document every reasoning service, and no other.

    Structural rather than numeric: the README carries no service COUNT to go
    stale, and a service added to the binding fails this check on the day it is
    added rather than the day someone notices the front page never mentioned it.
    """
    text = _read(_PY_README)
    rel = _PY_README.relative_to(_REPO)
    documented = set(re.findall(r"\| `entail\.(\w+)\(", text))
    expected = set(services)
    problems: list[str] = []
    for name in sorted(expected - documented):
        problems.append(
            f"{rel}: the Description-Logic service table has no row for "
            f"`entail.{name}(...)`, which {_PY_STUB.relative_to(_REPO)} declares"
        )
    for name in sorted(documented - expected):
        problems.append(
            f"{rel}: the Description-Logic service table has a row for "
            f"`entail.{name}(...)`, which is not a reasoning service "
            f"{_PY_STUB.relative_to(_REPO)} declares"
        )
    return problems


def release_crate_list_claim(crates: list[str]) -> list[str]:
    """The Trusted-Publisher crate bullet list in docs/RELEASE.md.

    A crate present in the publish array but absent from this list is how a
    crate reaches the release lane without anyone configuring a publisher — or a
    crates.io record — for it.
    """
    text = _read(_RELEASE)
    rel = _RELEASE.relative_to(_REPO)
    section = re.search(
        r"`scripts/check-doc-claims\.py` checks this list against:\n\n(.*?)\n\n",
        text,
        re.DOTALL,
    )
    if not section:
        return [f"{rel}: the Trusted Publisher crate list could not be located"]
    listed = re.findall(r"^- `([a-z0-9-]+)`$", section.group(1), re.MULTILINE)
    if listed != crates:
        return [
            f"{rel}: the Trusted Publisher crate list disagrees with "
            f"scripts/release-crates.sh\n"
            f"    documented: {listed}\n"
            f"    release set: {crates}"
        ]
    return []


def _flow(pattern: str) -> str:
    """Make a claim pattern independent of how the prose happens to be wrapped.

    Every literal space becomes ``\\s+``, so re-flowing a paragraph (which a
    documentation edit does constantly) does not silently drop a claim from this
    gate by making its pattern unmatchable. The numbers stay exact; only the
    whitespace between them is elastic. Use ``ANY`` for a gap that spans
    sentences, since ``.`` does not cross a newline without ``re.DOTALL``.
    """
    return pattern.replace(" ", r"\s+")


# A run of arbitrary text, newlines included, between two anchored fragments.
ANY = r"[\s\S]*?"


def rl_matrix_agreement_claim(
    matrix: dict[str, tuple[int, int]], lanes: dict[str, int]
) -> list[str]:
    """The RL row of the generated matrix must equal the ledger×census derivation.

    Two independent artifacts measure the same run: the matrix block (scraped
    from the harness's `OWL2-RL-ENTAILMENT` scoreboard line) and the LEDGER
    cross-referenced against the census. If they disagree, one of them is stale
    and every per-lane number in the prose below is unsourced — so this is
    checked before any of those claims are.
    """
    row = matrix.get(_RL_SUITE_ROW)
    if row is None:
        return [
            f"docs/CONFORMANCE.md: the generated matrix block has no "
            f"{_RL_SUITE_ROW!r} row; regenerate with "
            f"`python3 scripts/conformance-matrix.py --write-doc`"
        ]
    passed, ledgered = row
    derived_pass = lanes["positive_agree"] + lanes["negative_agree"]
    problems: list[str] = []
    if passed != derived_pass:
        problems.append(
            f"the matrix block's {_RL_SUITE_ROW!r} row reports {passed} passing, "
            f"but LEDGER × census.tsv derive {derived_pass} "
            f"({lanes['positive_agree']} positive + {lanes['negative_agree']} negative)"
        )
    if ledgered != lanes["ledgered"]:
        problems.append(
            f"the matrix block's {_RL_SUITE_ROW!r} row reports {ledgered} ledgered, "
            f"but owl2_rl.rs::LEDGER holds {lanes['ledgered']} entries"
        )
    return problems


def build_claims(
    inventory: dict[str, tuple[int, int]],
    matrix: dict[str, tuple[int, int]],
    census: dict[str, int],
    lanes: dict[str, int],
    mechanisms: dict[str, int],
    extend_families: dict[str, int],
) -> list[Claim]:
    owl2_pass, owl2_ledger = matrix["Entailment (OWL 2 DL consistency)"]
    owl2_total = owl2_pass + owl2_ledger
    sparql_pass, sparql_xfail = matrix["SPARQL 1.1/1.2 evaluation (full corpus)"]
    shacl_pass, _ = matrix["SHACL Core + SHACL-SPARQL"]
    corpus_pass, _ = matrix["SHACL (first-party corpus)"]
    shex_pass, _ = matrix["ShEx 2.1 validation"]
    codec_pass, _ = matrix["Syntax codecs (Turtle/TriG/NT/NQ/RDF-XML)"]
    rdflib_pass, rdflib_x = matrix["rdflib LSP drop-in gate"]
    compat_pass, compat_x = matrix["Python binding suite"]

    rl_pass, rl_ledger = matrix[_RL_SUITE_ROW]
    rl_total = rl_pass + rl_ledger

    inv = "docs/book/src/entailment-rules.md, generated from rules()/implemented()"
    mat = "the generated conformance-matrix block in docs/CONFORMANCE.md"
    cen = (
        "crates/sparql-conformance/entailment-suite/w3c-owl2-rl/census.tsv, derived "
        "from the W3C manifest and byte-frozen"
    )
    led = (
        "crates/sparql-conformance/src/owl2_rl.rs::LEDGER cross-referenced against "
        "census.tsv (the harness asserts 0 unledgered and 0 stale)"
    )
    ext = (
        "crates/sparql-conformance/suite/purrdf-extend/manifest.ttl's `mf:entries` "
        "list, counted per family by each case id's own prefix"
    )
    mech = (
        "the `OWL2-RL-MECHANISMS` line pinned in "
        "crates/sparql-conformance/tests/owl2_rl_conformance.rs, which the harness "
        "recomputes from the corpus on every run"
    )

    # Every prose site that states the RL lane split, in the exact words it uses.
    # Sourced from `led`: the split is derived, not scraped, because the
    # scoreboard line reports only the combined agreement count.
    lane_sites: list[tuple[str, Path, str]] = [
        (
            "the OWL 2 RL lane split in the CONFORMANCE scoreboard row",
            _CONFORMANCE,
            _flow(r"Negative lane \*\*(?P<neg_a>\d+) / (?P<neg_t>\d+)\*\*")
            + ANY
            + _flow(r"Positive lane \*\*(?P<pos_a>\d+) / (?P<pos_t>\d+)\*\*"),
        ),
        (
            "the OWL 2 RL lane split in the CONFORMANCE rule-table row",
            _CONFORMANCE,
            _flow(
                r"entailment tests score (?P<pos_a>\d+) of (?P<pos_t>\d+) positive "
                r"and (?P<neg_a>\d+) of (?P<neg_t>\d+) negative"
            ),
        ),
        (
            "the OWL 2 RL lane split in the CONFORMANCE known-gaps item",
            _CONFORMANCE,
            _flow(r"negative lane is (?P<neg_a>\d+) / (?P<neg_t>\d+): no unsoundness")
            + ANY
            + _flow(r"positive lane is \*\*(?P<pos_a>\d+) of (?P<pos_t>\d+)\*\*"),
        ),
        (
            "the OWL 2 RL lane split in the entailment chapter's rule-table bullet",
            _ENTAILMENT,
            _flow(
                r"reaches (?P<pos_a>\d+) of (?P<pos_t>\d+) published positive "
                r"entailments, and agrees with W3C on (?P<neg_a>\d+) of "
                r"(?P<neg_t>\d+) negative ones"
            ),
        ),
        (
            "the OWL 2 RL lane split in the entailment chapter's conformance bullet",
            _ENTAILMENT,
            _flow(
                r"negative lane is (?P<neg_a>\d+) of (?P<neg_t>\d+): no unsoundness\.\*\*"
            )
            + ANY
            + _flow(r"positive lane is \*\*(?P<pos_a>\d+) of (?P<pos_t>\d+)\*\*"),
        ),
        (
            "the OWL 2 RL lane split in the README feature bullet",
            _README,
            _flow(
                r"scores \*\*(?P<pos_a>\d+) of (?P<pos_t>\d+) positive and "
                r"(?P<neg_a>\d+) of (?P<neg_t>\d+) negative\*\*"
            ),
        ),
        (
            "the OWL 2 RL lane split in the README conformance table",
            _README,
            _flow(
                r"negative lane \*\*(?P<neg_a>\d+) / (?P<neg_t>\d+)\*\* "
                r"\(no unsoundness\), positive lane "
                r"\*\*(?P<pos_a>\d+) / (?P<pos_t>\d+)\*\*"
            ),
        ),
        (
            "the OWL 2 RL lane split in the purrdf-entail crate README",
            _ENTAIL_README,
            _flow(
                r"entailment tests this chase scores \*\*(?P<pos_a>\d+) of "
                r"(?P<pos_t>\d+)\s+positive and (?P<neg_a>\d+) of (?P<neg_t>\d+)\s*"
                r"negative\*\*"
            ),
        ),
        (
            "the OWL 2 RL lane split in the purrdf umbrella crate README",
            _PURRDF_README,
            _flow(
                r"entailment tests this chase scores \*\*(?P<pos_a>\d+) of "
                r"(?P<pos_t>\d+)\s+positive and (?P<neg_a>\d+) of (?P<neg_t>\d+)\s*"
                r"negative\*\*"
            ),
        ),
        (
            "the OWL 2 RL lane split in the purrdf-cli crate README",
            _CLI_README,
            _flow(
                r"entailment tests this chase scores \*\*(?P<pos_a>\d+) of "
                r"(?P<pos_t>\d+)\s+positive and (?P<neg_a>\d+) of (?P<neg_t>\d+)\s*"
                r"negative\*\*"
            ),
        ),
        (
            "the OWL 2 RL lane split in the Python binding README",
            _PY_README,
            _flow(
                r"reaches (?P<pos_a>\d+) of (?P<pos_t>\d+) published positive "
                r"entailments, and agrees with W3C on (?P<neg_a>\d+) of "
                r"(?P<neg_t>\d+) negative ones"
            ),
        ),
        (
            "the OWL 2 RL lane split in the book's introduction",
            _INTRODUCTION,
            _flow(
                r"tests score (?P<pos_a>\d+) of (?P<pos_t>\d+) positive and "
                r"(?P<neg_a>\d+) of (?P<neg_t>\d+) negative"
            ),
        ),
        (
            # The GENERATED inventory. It restates the lane split in prose that
            # `crates/entail/examples/gen_rule_inventory.rs` carries as a literal, so
            # `scripts/check-generated.sh` can only prove the two copies agree with each
            # other — never that either agrees with the harness. This entry is the only
            # thing that ties them to a measurement.
            "the OWL 2 RL lane split in the generated rule inventory",
            _INVENTORY,
            _flow(
                r"(?P<pos_a>\d+) of (?P<pos_t>\d+) positive and (?P<neg_a>\d+) of "
                r"(?P<neg_t>\d+) negative"
            ),
        ),
        (
            "the OWL 2 RL lane split in the book's conformance chapter",
            _BOOK_CONFORMANCE,
            _flow(
                r"being (?P<pos_a>\d+) of (?P<pos_t>\d+) positive and (?P<neg_a>\d+) "
                r"of (?P<neg_t>\d+) negative"
            ),
        ),
    ]
    lane_expected = {
        "pos_a": lanes["positive_agree"],
        "pos_t": lanes["positive_total"],
        "neg_a": lanes["negative_agree"],
        "neg_t": lanes["negative_total"],
    }

    # Every prose site that states the DL corpus's subset/exclusion tallies.
    exclusion_sites: list[tuple[str, Path, str]] = [
        (
            "the DL subset/exclusion tally in the CONFORMANCE scoreboard row",
            _CONFORMANCE,
            r"(?P<graded>\d+) of the (?P<shaped>\d+) consistency-shaped cases upstream — "
            r"and of the (?P<excluded>\d+) it leaves out, \*\*(?P<decides>\d+) the tableau "
            r"decided when the exclusion was measured\*\* \((?P<dc>\d+) consistent \+ "
            r"(?P<di>\d+) inconsistent\), (?P<nonterm>\d+) did not terminate under a 40 s "
            r"ceiling, (?P<withheld>\d+) were withheld \((?P<wr>\d+) reasoner, "
            r"(?P<wp>\d+) parse\) and (?P<nopremise>\d+) carry no RDF/XML premise",
        ),
        (
            "the DL subset/exclusion tally in the CONFORMANCE known-gaps item",
            _CONFORMANCE,
            _flow(
                r"vendors (?P<graded>\d+) of the \*\*(?P<shaped>\d+)\*\* "
                r"consistency-shaped cases upstream"
            )
            + ANY
            + _flow(
                r"reports what the other \*\*(?P<excluded>\d+)\*\* did when the probe ran — "
                r"\*\*(?P<decides>\d+) the tableau decided\*\* "
                r"\((?P<dc>\d+) consistent, (?P<di>\d+) inconsistent\), "
                r"(?P<nonterm>\d+) that did not terminate under a 40 s ceiling"
            )
            + ANY
            + _flow(
                r"(?P<withheld>\d+) withheld \((?P<wr>\d+) reasoner, "
                r"(?P<wp>\d+) parse\), and (?P<nopremise>\d+) with no RDF/XML premise"
            ),
        ),
        (
            "the DL subset/exclusion tally in the entailment chapter",
            _ENTAILMENT,
            _flow(
                r"(?P<graded>\d+) of the (?P<shaped>\d+) consistency-shaped cases "
                r"upstream\. Of the (?P<excluded>\d+) it leaves out, "
                r"\*\*(?P<decides>\d+) the tableau decided when the exclusion was "
                r"measured\*\* "
                r"\((?P<dc>\d+) consistent, (?P<di>\d+) inconsistent\), "
                r"(?P<nonterm>\d+) did not terminate under a 40 s ceiling, "
                r"(?P<withheld>\d+) were withheld \((?P<wr>\d+) reasoner, "
                r"(?P<wp>\d+) parse\), and (?P<nopremise>\d+) carry no RDF/XML premise"
            ),
        ),
    ]
    exclusion_expected = {
        "graded": census["dl_graded"],
        "shaped": census["consistency_shaped"],
        "excluded": census["dl_excluded"],
        "decides": census["dl_decides"],
        "dc": census["dl_decides_consistent"],
        "di": census["dl_decides_inconsistent"],
        "nonterm": census["dl_non_terminating"],
        "withheld": census["dl_withholds"],
        "wr": census["dl_withholds_reasoner"],
        "wp": census["dl_withholds_parse"],
        "nopremise": census["dl_no_premise"],
    }

    # Every prose site that tallies the typed RL divergence ledger.
    gap_sites: list[tuple[str, Path, str]] = [
        (
            "the OWL 2 RL gap tally in the CONFORMANCE scoreboard row",
            _CONFORMANCE,
            # The ledger is EMPTY, so the row states the tally as zeros rather than as
            # a split of a nonzero total. Every typed count is still checked: an
            # entry arriving in any class moves the number the prose has to state.
            r"the typed-divergence ledger is EMPTY \((?P<schema>\d+) schema-conclusion, "
            r"(?P<neg>\d+) negative-conclusion, (?P<outside>\d+) construct-outside-rl, "
            r"(?P<imports>\d+) imports-unresolved\); \*\*(?P<actionable>\d+) (?:is|are) "
            r"actionable\*\* \((?P<missing>\d+) missing-rule\)",
        ),
        (
            "the OWL 2 RL gap tally in the CONFORMANCE known-gaps item",
            _CONFORMANCE,
            _flow(
                r"is EMPTY — (?P<schema>\d+) `schema-conclusion`, (?P<neg>\d+) "
                r"`negative-conclusion`, (?P<outside>\d+) `construct-outside-rl`, "
                r"(?P<imports>\d+) `imports-unresolved`, and "
                r"\*\*(?P<actionable>\d+) (?:is|are) actionable\*\* "
                r"\((?P<missing>\d+) `missing-rule`\)"
            ),
        ),
        (
            "the OWL 2 RL gap tally in the entailment chapter",
            _ENTAILMENT,
            _flow(
                r"is EMPTY — (?P<schema>\d+) `schema-conclusion`, (?P<neg>\d+) "
                r"`negative-conclusion`, (?P<outside>\d+) `construct-outside-rl`, "
                r"(?P<imports>\d+) `imports-unresolved`, and "
                r"\*\*(?P<actionable>\d+) (?:is|are) actionable\*\* "
                r"\((?P<missing>\d+) `missing-rule`\)"
            ),
        ),
    ]
    gap_expected = {
        "structural": lanes["structural"],
        "ledgered": lanes["ledgered"],
        "schema": lanes["schema_conclusion"],
        "neg": lanes["negative_conclusion"],
        "outside": lanes["construct_outside_rl"],
        "imports": lanes["imports_unresolved"],
        "actionable": lanes["actionable"],
        "missing": lanes["missing_rule"],
    }

    claims = [
        Claim(
            what,
            path,
            pattern,
            {k: v for k, v in lane_expected.items() if f"?P<{k}>" in pattern},
            led,
        )
        for what, path, pattern in lane_sites
    ]
    claims += [
        Claim(
            what,
            path,
            pattern,
            {k: v for k, v in exclusion_expected.items() if f"?P<{k}>" in pattern},
            cen,
        )
        for what, path, pattern in exclusion_sites
    ]
    claims += [
        Claim(
            "the root README's OWL 2 RL rule-count headline",
            _README,
            _flow(r"\*\*all (?P<owlrl>\d+) OWL 2 RL rules\*\*"),
            {"owlrl": inventory["OWL-RL"][0]},
            inv,
        ),
        Claim(
            "the root README's RDF+RDFS pattern-count headline",
            _README,
            _flow(r"all (?P<rdfs>\d+) RDF \+ RDFS patterns"),
            {"rdfs": inventory["RDFS"][0]},
            inv,
        ),
        Claim(
            "the SHACL corpus size in the benchmarks doc",
            _REPO / "docs" / "BENCHMARKS.md",
            _flow(r"All (?P<total>\d+) committed first-party conformance cases"),
            {"total": corpus_pass},
            mat,
        ),
    ]
    claims += [
        Claim(
            what,
            path,
            pattern,
            {k: v for k, v in gap_expected.items() if f"?P<{k}>" in pattern},
            led,
        )
        for what, path, pattern in gap_sites
    ]
    # --- the generated rule inventory's two remaining entailment numbers --------
    #
    # Both are prose that `crates/entail/examples/gen_rule_inventory.rs` emits as a
    # HARDCODED literal, so the document and its generator agree by construction and
    # `scripts/check-generated.sh` cannot tell either of them from the truth. Each is
    # therefore sourced here from the artifact that MEASURES it — the matrix block and
    # the harness's pinned mechanism line — and not from the generator.
    claims += [
        Claim(
            "the corpus agreement total in the generated rule inventory",
            _INVENTORY,
            _flow(
                r"entailment corpus: (?P<agreed>\d+) of (?P<total>\d+) cases agree"
            ),
            {"agreed": rl_pass, "total": rl_total},
            mat,
        ),
        Claim(
            "the beyond-the-table tally in the generated rule inventory",
            _INVENTORY,
            _flow(
                r"(?P<beyond>[A-Za-z]+|\d+) of those (?P<total>\d+) are reached by a "
                r"mechanism"
            ),
            {
                "beyond": mechanisms["beyond_table_positive"],
                "total": mechanisms["total"],
            },
            mech,
        ),
    ]

    return claims + [
        # --- the OWL 2 RL entailment lane, sourced from the matrix block ------
        Claim(
            "the OWL 2 RL scoreboard row",
            _CONFORMANCE,
            _flow(
                r"\*\*(?P<passed>\d+) / (?P<total>\d+)\*\* agreeing · "
                r"(?P<ledgered>\d+) typed-ledger divergences?"
            ),
            {"passed": rl_pass, "total": rl_total, "ledgered": rl_ledger},
            mat,
        ),
        Claim(
            "the OWL 2 RL row in the README conformance table",
            _README,
            _flow(
                r"\*\*(?P<passed>\d+) / (?P<total>\d+)\*\* agreeing, "
                r"(?P<ledgered>\d+) ledgered, 0 unledgered — negative lane"
            ),
            {"passed": rl_pass, "total": rl_total, "ledgered": rl_ledger},
            mat,
        ),
        Claim(
            "the OWL 2 RL conformance paragraph in the entailment chapter",
            _ENTAILMENT,
            _flow(
                r"W3C OWL 2 RL entailment tests — (?P<passed>\d+) of (?P<total>\d+) "
                r"cases agree, (?P<ledgered>\d+) ledgered"
            ),
            {"passed": rl_pass, "total": rl_total, "ledgered": rl_ledger},
            mat,
        ),
        Claim(
            "the OWL 2 RL snapshot in the book's conformance chapter",
            _BOOK_CONFORMANCE,
            _flow(r"scores (?P<passed>\d+)/(?P<total>\d+), being"),
            {"passed": rl_pass, "total": rl_total},
            mat,
        ),
        # --- what W3C published, sourced from the frozen census ---------------
        Claim(
            "the upstream entailment-test counts in the CONFORMANCE known-gaps item",
            _CONFORMANCE,
            _flow(
                r"nodes\) holds \*\*(?P<positive>\d+) `otest:PositiveEntailmentTest` "
                r"and (?P<negative>\d+) `otest:NegativeEntailmentTest`\*\* cases"
            ),
            {
                "positive": census["positive_entailment"],
                "negative": census["negative_entailment"],
            },
            cen,
        ),
        Claim(
            "the upstream manifest size in the CONFORMANCE known-gaps item",
            _CONFORMANCE,
            _flow(r"(?P<cases>\d+) `otest:TestCase` nodes\)"),
            {"cases": census["upstream_cases"]},
            cen,
        ),
        Claim(
            "the upstream entailment-test counts in the entailment chapter",
            _ENTAILMENT,
            _flow(
                r"manifest holds \*\*(?P<positive>\d+) positive and "
                r"(?P<negative>\d+) negative entailment tests\*\*"
            ),
            {
                "positive": census["positive_entailment"],
                "negative": census["negative_entailment"],
            },
            cen,
        ),
        Claim(
            "the RL corpus composition in the CONFORMANCE suite inventory",
            _CONFORMANCE,
            _flow(
                r"(?P<total>\d+) cases \((?P<positive>\d+) positive RL-profile "
                r"RDF-Based entailments plus all (?P<negative>\d+) negative "
                r"entailments\)"
            ),
            {
                "total": census["rl_positive"] + census["rl_negative"],
                "positive": census["rl_positive"],
                "negative": census["rl_negative"],
            },
            cen,
        ),
        Claim(
            "the upstream census size in the CONFORMANCE suite inventory",
            _CONFORMANCE,
            _flow(r"one row per upstream `otest:TestCase` \((?P<cases>\d+) rows\)"),
            {"cases": census["upstream_cases"]},
            cen,
        ),
        Claim(
            "the DL subset restatement in the book's conformance chapter",
            _BOOK_CONFORMANCE,
            _flow(
                r"(?P<graded>\d+) of the (?P<shaped>\d+) consistency-shaped cases "
                r"W3C published"
            ),
            {"graded": census["dl_graded"], "shaped": census["consistency_shaped"]},
            cen,
        ),
        Claim(
            "the RL corpus composition in the CONFORMANCE known-gaps item",
            _CONFORMANCE,
            _flow(
                r"(?P<positive>\d+) positive cases W3C places inside the RL profile "
                r"under RDF-Based semantics, plus \*\*all\*\* (?P<negative>\d+) "
                r"negative cases"
            ),
            {"positive": census["rl_positive"], "negative": census["rl_negative"]},
            cen,
        ),
        # --- rule tables, sourced from the generated inventory ----------------
        Claim(
            "the 'Entailment rule tables' scoreboard row",
            _CONFORMANCE,
            r"`OWL-RL` \*\*(?P<owlrl_i>\d+) / (?P<owlrl_d>\d+)\*\*[^|]*?"
            r"`RDFS` \*\*(?P<rdfs_i>\d+) / (?P<rdfs_d>\d+)\*\*[^|]*?"
            r"`RDF` \*\*(?P<rdf_i>\d+) / (?P<rdf_d>\d+)\*\*[^|]*?"
            r"`D` \*\*(?P<d_i>\d+) / (?P<d_d>\d+)\*\*",
            {
                "owlrl_i": inventory["OWL-RL"][1],
                "owlrl_d": inventory["OWL-RL"][0],
                "rdfs_i": inventory["RDFS"][1],
                "rdfs_d": inventory["RDFS"][0],
                "rdf_i": inventory["RDF"][1],
                "rdf_d": inventory["RDF"][0],
                "d_i": inventory["D"][1],
                "d_d": inventory["D"][0],
            },
            inv,
        ),
        # The bare rule count in each published crate README. It is the number a
        # crates.io reader meets first, so it is gated against the same generated
        # inventory the book's table is.
        Claim(
            "the OWL-RL rule count in the purrdf-entail crate README",
            _ENTAIL_README,
            _flow(r"`OWL-RL` fires all (?P<owlrl_d>\d+)\s+rules"),
            {"owlrl_d": inventory["OWL-RL"][0]},
            inv,
        ),
        Claim(
            "the OWL-RL rule count in the purrdf umbrella crate README",
            _PURRDF_README,
            _flow(
                r"all (?P<owlrl_d>\d+) OWL 2 RL rules of OWL 2 Profiles §4.3 "
                r"Tables 4–9"
            ),
            {"owlrl_d": inventory["OWL-RL"][0]},
            inv,
        ),
        # The three rule-table numbers on the PyPI front page: the runnable
        # snippet's two comments and the qualifier sentence beneath it. The
        # snippet is executable, so a wrong comment is a wrong EXAMPLE — the
        # readme claimed four RDFS rules were missing while the wheel returned
        # none — and the qualifier is the only thing that stops 78 / 78 being
        # read as entailment conformance.
        Claim(
            "the OWL-RL rule-table snippet in the Python binding README",
            _PY_README,
            _flow(
                r'defined = entail\.rules\("owl-rl"\) # (?P<owlrl_d>\d+) — OWL 2 '
                r"Profiles §4\.3 Tables 4–9\n"
                r'fired = entail\.implemented_rules\("owl-rl"\) # (?P<owlrl_i>\d+)'
            ),
            {"owlrl_d": inventory["OWL-RL"][0], "owlrl_i": inventory["OWL-RL"][1]},
            inv,
        ),
        Claim(
            "the RDFS gap comment in the Python binding README",
            _PY_README,
            _flow(
                r"# \[\] — RDFS fires (?P<rdfs_i>\d+) of its (?P<rdfs_d>\d+) rules"
            ),
            {"rdfs_i": inventory["RDFS"][1], "rdfs_d": inventory["RDFS"][0]},
            inv,
        ),
        Claim(
            "the rule-table-coverage qualifier in the Python binding README",
            _PY_README,
            _flow(
                r"\*\*(?P<owlrl_i>\d+) / (?P<owlrl_d>\d+) is rule-table coverage"
            ),
            {"owlrl_i": inventory["OWL-RL"][1], "owlrl_d": inventory["OWL-RL"][0]},
            inv,
        ),
        Claim(
            "the rule counts in the purrdf-cli crate README",
            _CLI_README,
            _flow(
                r"`rdfs` fires (?P<rdfs_i>\d+) of the (?P<rdfs_d>\d+) RDF \+ RDFS "
                r"patterns; `owl-rl` fires all (?P<owlrl_d>\d+) rules"
            ),
            {
                "rdfs_i": inventory["RDFS"][1],
                "rdfs_d": inventory["RDFS"][0],
                "owlrl_d": inventory["OWL-RL"][0],
            },
            inv,
        ),
        # --- OWL 2 DL consistency, sourced from the matrix block --------------
        Claim(
            "the rdflib vendor scoreboard in its PROVENANCE",
            _REPO / "bindings" / "python" / "tests" / "rdflib_suite" / "vendor"
            / "PROVENANCE.md",
            _flow(r"\*\*(?P<passed>\d+) passed / (?P<xfail>\d+) xfailed\*\*"),
            {"passed": rdflib_pass, "xfail": rdflib_x},
            mat,
        ),
        Claim(
            "the rdflib scoreboard in the Python test README",
            _REPO / "bindings" / "python" / "tests" / "README.md",
            _flow(r"Scoreboard: \*\*(?P<passed>\d+) passed / (?P<xfail>\d+) xfailed\*\*"),
            {"passed": rdflib_pass, "xfail": rdflib_x},
            mat,
        ),
        Claim(
            "the OWL 2 DL-consistency figure in the book's entailment chapter",
            _ENTAILMENT,
            _flow(r'"(?P<passed>\d+) of (?P<total>\d+)" is a number over a corpus'),
            {"passed": owl2_pass, "total": owl2_total},
            mat,
        ),
        Claim(
            "the SHACL first-party corpus size in the CONFORMANCE command block",
            _CONFORMANCE,
            _flow(r"# the (?P<total>\d+)-case frozen corpus"),
            {"total": corpus_pass},
            mat,
        ),
        Claim(
            "the SHACL first-party corpus size in the book's validation chapter",
            _REPO / "docs" / "book" / "src" / "validation" / "shacl.md",
            _flow(r"a first-party frozen corpus of (?P<total>\d+) cases"),
            {"total": corpus_pass},
            mat,
        ),
        Claim(
            "the OWL 2 DL-consistency scoreboard row",
            _CONFORMANCE,
            r"\*\*(?P<passed>\d+) / (?P<total>\d+)\*\* agreeing verdicts · "
            r"(?P<ledgered>\d+) typed-ledger divergences?",
            {"passed": owl2_pass, "total": owl2_total, "ledgered": owl2_ledger},
            mat,
        ),
        Claim(
            "the OWL 2 corpus composition (ConsistencyTest + InconsistencyTest)",
            _CONFORMANCE,
            r"\((?P<consistency>\d+) `otest:ConsistencyTest` \+ "
            r"(?P<inconsistency>\d+) `otest:InconsistencyTest`",
            # The two case kinds must account for the whole corpus.
            {"consistency": owl2_total - 36, "inconsistency": 36},
            f"{mat} (agreed + ledgered = the vendored corpus size)",
        ),
        Claim(
            "the OWL 2 conformance paragraph in the entailment chapter",
            _ENTAILMENT,
            r"W3C OWL 2 test suite — (?P<passed>\d+) of (?P<total>\d+) cases "
            r"agree, (?P<ledgered>\d+) ledgered",
            {"passed": owl2_pass, "total": owl2_total, "ledgered": owl2_ledger},
            mat,
        ),
        Claim(
            "the OWL 2 case-kind split in the entailment chapter",
            _ENTAILMENT,
            r"all (?P<total>\d+) vendored cases are\n  `otest:ConsistencyTest` "
            r"\((?P<consistency>\d+)\) or `otest:InconsistencyTest` "
            r"\((?P<inconsistency>\d+)\)",
            {
                "total": owl2_total,
                "consistency": owl2_total - 36,
                "inconsistency": 36,
            },
            mat,
        ),
        Claim(
            "the OWL 2 divergence count in the entailment chapter",
            _ENTAILMENT,
            _flow(
                r"Every one of the (?P<ledgered>\d+) divergences is named in a typed "
                r"ledger"
            ),
            {"ledgered": owl2_ledger},
            mat,
        ),
        Claim(
            "the OWL 2 row in the README conformance table",
            _README,
            # Anchored to the end of the table cell: the OWL 2 RL row directly
            # below has the same shape and would otherwise match too.
            r"\*\*(?P<passed>\d+) / (?P<total>\d+)\*\* agreeing, "
            r"(?P<ledgered>\d+) ledgered, 0 unledgered \|",
            {"passed": owl2_pass, "total": owl2_total, "ledgered": owl2_ledger},
            mat,
        ),
        Claim(
            "the OWL 2 snapshot in the book's conformance chapter",
            _BOOK_CONFORMANCE,
            r"(?P<passed>\d+)/(?P<total>\d+) agreeing\nverdicts on the vendored "
            r"W3C OWL 2 DL-consistency corpus",
            {"passed": owl2_pass, "total": owl2_total},
            mat,
        ),
        Claim(
            "the ledgered-gap summary in the book's conformance chapter",
            _BOOK_CONFORMANCE,
            r"non-canonical XSD lexicals; (?P<ledgered>\d+) typed OWL 2 "
            r"divergences\)",
            {"ledgered": owl2_ledger},
            mat,
        ),
        # --- the remaining scoreboard rows the matrix block can source --------
        Claim(
            "the SPARQL 1.1/1.2 scoreboard row",
            _CONFORMANCE,
            r"\| \*\*(?P<passed>\d+)\*\* pass · (?P<xfail>\d+) typed xfail · 0 fail",
            {"passed": sparql_pass, "xfail": sparql_xfail},
            mat,
        ),
        Claim(
            "the 'N ledgered xfails' sentence in the SPARQL known-gaps item",
            _CONFORMANCE,
            # Sentence-shaped, not line-anchored: a "5→4" typo drifted from every
            # other count in this same document (the matrix row, the scoreboard
            # row, and the sentence's own five-fixture enumeration) and nothing
            # caught it because this specific sentence was never derived from
            # anything. Matched by the "N ledgered xfails" shape rather than a
            # fixed count, so a sixth xfail landing tomorrow fails this until the
            # prose is updated, and re-wording elsewhere in the paragraph cannot
            # silently widen the match.
            _flow(r"remaining non-passes are the \*\*(?P<xfail>\d+) ledgered xfails\*\*"),
            {"xfail": sparql_xfail},
            mat,
        ),
        Claim(
            "the W3C SHACL scoreboard row",
            _CONFORMANCE,
            r"W3C data-shapes, `core/` \+ `sparql/` \+ `af/` \| "
            r"\*\*(?P<passed>\d+) / (?P<total>\d+)\*\* · (?P<ledgered>\d+) ledgered",
            {"passed": shacl_pass, "total": shacl_pass, "ledgered": 0},
            mat,
        ),
        Claim(
            "the first-party SHACL corpus scoreboard row",
            _CONFORMANCE,
            r"\*\*(?P<passed>\d+) / (?P<total>\d+)\*\* frozen expected reports",
            {"passed": corpus_pass, "total": corpus_pass},
            mat,
        ),
        Claim(
            "the ShEx 2.1 validation scoreboard row",
            _CONFORMANCE,
            r"\*\*(?P<passed>[\d,]+) / (?P<total>[\d,]+)\*\* attempted · "
            r"(?P<xfail>\d+) xfail",
            {"passed": shex_pass, "total": shex_pass, "xfail": 0},
            mat,
        ),
        Claim(
            "the syntax-codec scoreboard row",
            _CONFORMANCE,
            r"\*\*(?P<passed>\d+) / (?P<total>\d+)\*\* round-trip \(nquads "
            r"(?P<nq>\d+), ntriples (?P<nt>\d+), rdfxml (?P<rx>\d+), trig "
            r"(?P<tg>\d+), turtle (?P<tt>\d+)\)",
            {
                "passed": codec_pass,
                "total": codec_pass,
                # The per-format split must account for the whole suite.
                "nq": 27,
                "nt": 29,
                "rx": 31,
                "tg": 60,
                "tt": codec_pass - (27 + 29 + 31 + 60),
            },
            mat,
        ),
        Claim(
            "the rdflib drop-in scoreboard row",
            _CONFORMANCE,
            r"\*\*(?P<passed>\d+)\*\* pass · (?P<xfail>\d+) strict-xfail "
            r"\(ledgered\) \|\n\| Python binding suite",
            {"passed": rdflib_pass, "xfail": rdflib_x},
            mat,
        ),
        Claim(
            "the Python binding suite scoreboard row",
            _CONFORMANCE,
            r"compat differential vs rdflib 7\.6 included \| "
            r"\*\*(?P<passed>\d+)\*\* pass · (?P<xfail>\d+) strict-xfail",
            {"passed": compat_pass, "xfail": compat_x},
            mat,
        ),
        # --- purrdf-extend manifest per-family case counts, hand-counted in the
        # SPARQL 1.1/1.2 scoreboard row's prose. Each is sourced from the manifest's
        # own `mf:entries` list (`load_extend_manifest_family_counts`), not from a
        # second hand-maintained number, which is how "eight SEP-0007" survived
        # shipping a ninth case.
        Claim(
            "the purrdf-extend temporal-family case count in the SPARQL 1.1/1.2 row",
            _CONFORMANCE,
            _flow(
                r"purrdf-extend` manifest additionally walks "
                r"(?P<temporal>[A-Za-z]+|\d+) temporal arithmetic cases"
            ),
            {"temporal": extend_families["temporal"]},
            ext,
        ),
        Claim(
            "the purrdf-extend LATERAL-family case count in the SPARQL 1.1/1.2 row",
            _CONFORMANCE,
            _flow(
                r"the same manifest also walks (?P<lateral>[A-Za-z]+|\d+) "
                r"`LATERAL` \(SEP-0006\) cases"
            ),
            {"lateral": extend_families["LATERAL"]},
            ext,
        ),
        Claim(
            "the purrdf-extend SEP-0007-family case count in the SPARQL 1.1/1.2 row",
            _CONFORMANCE,
            _flow(
                r"the same manifest also walks (?P<exists>[A-Za-z]+|\d+) SEP-0007 "
                r"`EXISTS`/`NOT EXISTS` cases"
            ),
            {"exists": extend_families["SEP-0007"]},
            ext,
        ),
    ]


def main(argv: list[str]) -> int:
    unknown = [argument for argument in argv[1:] if argument != "--self-test"]
    if unknown:
        print(f"usage: {Path(argv[0]).name} [--self-test]", file=sys.stderr)
        return 2
    alone = "--self-test" in argv[1:]

    # ONE traversal of the documented surface, shared by both bans, so neither can be
    # narrowed without narrowing the other.
    surface = _documented_surface()

    # BEFORE the claims, on every run, and before the ban's own preflight: a guard that
    # cannot refuse the state it is named for makes every line printed after it worth less.
    survived = mutation_self_test(report=alone)
    if survived:
        print(
            "check-doc-claims: this gate's own guards do not refuse:\n"
            + "\n".join(f"  - {entry}" for entry in survived)
            + "\n\nEach line above is a lever that can be pulled with every gate green. Fix "
            "the guard, not the mutation.",
            file=sys.stderr,
        )
        return 1

    # The ban is the one check here that compares prose against a RULE rather than against a
    # generated number, so nothing else in this file would notice it answering wrongly — and
    # it answered wrongly in both directions for every claim that happened to wrap.
    wrong = overclaim_self_test(surface, report=alone)
    if wrong:
        print(
            "check-doc-claims: the entailment-overclaim ban does not answer its own "
            "injected sentences:\n"
            + "\n".join(f"  - {entry}" for entry in wrong)
            + "\n\nA claim it cannot see is a claim it does not ban, and a scoped claim it "
            "refuses is a sentence a writer cannot say. Fix `_sentences`, not the "
            "specimens.",
            file=sys.stderr,
        )
        return 1
    if alone:
        return 0

    inventory = load_rule_inventory()
    matrix = load_matrix()
    crates = load_release_crates()
    census = census_counts()
    lanes = rl_lane_counts()
    mechanisms = rl_mechanism_counts()
    extend_families = load_extend_manifest_family_counts()

    problems: list[str] = []
    checked = 0

    coverage_problems, coverage_checked = rule_coverage_table_claims(inventory)
    problems.extend(coverage_problems)
    checked += coverage_checked
    problems.extend(release_crate_list_claim(crates))
    checked += 1
    problems.extend(rl_matrix_agreement_claim(matrix, lanes))
    checked += 1
    problems.extend(py_service_table_claim(load_py_entail_services()))
    checked += 1
    problems.extend(extension_disclosure_claim(load_rule_extensions()))
    checked += 1
    problems.extend(xfail_ledger_prose_claim(load_xfail_ledger_sizes()))
    checked += 2
    problems.extend(regime_count_claim())
    checked += 1
    codec_problems, codec_checked = codec_table_claim()
    problems.extend(codec_problems)
    checked += codec_checked
    problems.extend(never_published_claim())
    checked += 1
    problems.extend(profile_count_claim())
    checked += 1
    problems.extend(program_regime_dts_claim())
    checked += 1
    problems.extend(reasoning_session_hosts_claim())
    checked += 1
    problems.extend(governor_corpus_count_claim(matrix))
    checked += 1
    problems.extend(governor_profile_digest_claim())
    checked += 1
    # The ban walk reports how many files it scanned, which is COVERAGE, not a claim
    # count — folding ~1,900 scanned files into the "documented claims" headline would
    # inflate a number readers take as the count of gated statements. The ban is one
    # claim; its reach is printed separately.
    fragment_problems, fragment_files = banned_stale_fragment_names(surface)
    problems.extend(fragment_problems)
    checked += 1
    # The second ban, and the same accounting: one claim, its reach printed separately.
    overclaim_problems, overclaim_swept, overclaim_corpus, overclaim_reached = (
        banned_entailment_overclaims(surface)
    )
    problems.extend(overclaim_problems)
    checked += 1

    for claim in build_claims(
        inventory, matrix, census, lanes, mechanisms, extend_families
    ):
        claim.check()
        problems.extend(claim.failures)
        checked += 1

    if problems:
        print(
            "Documented claims disagree with their generated source:\n",
            file=sys.stderr,
        )
        for problem in problems:
            print(f"  - {problem}", file=sys.stderr)
        print(
            "\nEvery number above is restated prose. Fix the prose to match the\n"
            "generated artifact — do not edit the generated artifact to match\n"
            "the prose. Regenerate the sources with `make metadata` (rule\n"
            "inventory) and `python3 scripts/conformance-matrix.py --write-doc`\n"
            "(conformance matrix).",
            file=sys.stderr,
        )
        return 1

    print(
        f"OK: {checked} documented claim(s) agree with their generated source "
        f"(stale-name ban swept {fragment_files} file(s); entailment-overclaim ban swept "
        f"{overclaim_swept} of {overclaim_corpus} prose unit(s) — Markdown documents and "
        f"registry descriptions — sentence by sentence, and matched every marker-bearing "
        f"claim across the {overclaim_reached} unit(s) outside that set)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
