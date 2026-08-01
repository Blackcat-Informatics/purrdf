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

It is pure text-over-committed-files: no cargo, no network, no test run. The
expensive gates prove the generated artifacts are current; this one proves the
prose agrees with them. Run standalone, or as part of
``scripts/check-generated.sh`` (and therefore ``make check``).
"""

from __future__ import annotations

import re
import sys
import tomllib
from dataclasses import dataclass, field
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
_MAKEFILE = _REPO / "Makefile"
_AGENTS = _REPO / "AGENTS.md"
_RELEASE_CRATES = _REPO / "scripts" / "release-crates.sh"

_INTRODUCTION = _REPO / "docs" / "book" / "src" / "introduction.md"
_RL_SUITE = _REPO / "crates" / "sparql-conformance" / "entailment-suite" / "w3c-owl2-rl"
_CENSUS = _RL_SUITE / "census.tsv"
_RL_LEDGER = _REPO / "crates" / "sparql-conformance" / "src" / "owl2_rl.rs"

_MATRIX_BEGIN = "<!-- BEGIN GENERATED: conformance-matrix -->"
_MATRIX_END = "<!-- END GENERATED: conformance-matrix -->"

# The matrix row name emitted by scripts/conformance-matrix.py for the OWL 2 RL
# entailment lane. Kept as a constant because several claims key off it.
_RL_SUITE_ROW = "Entailment (OWL 2 RL, W3C entailment tests)"


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def _int(text: str) -> int:
    """Parse a documented count, tolerating the thousands separator prose uses."""
    return int(text.replace(",", "").replace(" ", ""))


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


_SPELLED = {
    1: "one", 2: "two", 3: "three", 4: "four", 5: "five", 6: "six", 7: "seven",
    8: "eight", 9: "nine", 10: "ten", 11: "eleven", 12: "twelve", 13: "thirteen",
    14: "fourteen", 15: "fifteen", 16: "sixteen", 17: "seventeen", 18: "eighteen",
}


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


def banned_stale_fragment_names() -> tuple[list[str], int]:
    """The DL fragment has ONE published name; its two superseded spellings are banned.

    The decision core was published as ALCOIQ on nineteen sites and ALCHOIQ in the
    oracle, and both understated what the code decides. The settled name is SHOIQ(D);
    a superseded spelling reappearing anywhere in the documented surface is a
    regression to the two-name state, caught here by name.

    The walk covers ``crates``, ``bindings``, and ``docs`` prose plus ``scripts``' own
    ``.py``/``.json`` — the conformance harness and its ratchet baseline restate the
    same fragment name in their own prose, and a superseded spelling regressed there
    silently until this walk was extended to include it. Returns the problems and the
    number of files scanned, so the script's claim count reports what it really read.
    """
    problems: list[str] = []
    checked = 0
    for root in ("crates", "bindings", "docs"):
        for path in sorted((_REPO / root).rglob("*")):
            if path.suffix not in {".rs", ".md", ".pyi", ".mjs", ".ts"}:
                continue
            if "/pkg/" in str(path) or "node_modules" in str(path):
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
    for path in sorted((_REPO / "scripts").rglob("*")):
        if path.suffix not in {".py", ".json"}:
            continue
        if path.name == "check-doc-claims.py":
            # Names both superseded spellings in its own docstrings/pattern to
            # explain what it bans; that is the ban's definition, not a regression.
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
    checked += 1
    readme = _read(_README)
    for match in re.finditer(r"\bALCH?OIQ\b", readme):
        problems.append(
            f"README.md: superseded fragment spelling `{match.group(0)}`"
        )
    return problems, checked


# The nine documents that carry the entailment story: the repository front page, the
# four published READMEs that restate it, and the four book chapters. They are named
# rather than discovered because the ban below is about what these documents CLAIM, and a
# claim's absence from a document that never makes claims is not evidence of anything —
# sweeping every `.md` in the tree would dilute the ban into a spell-checker over
# changelogs and vendored provenance files.
_ENTAILMENT_CLAIM_DOCS = (
    "README.md",
    "bindings/python/README.md",
    "crates/cli/README.md",
    "crates/entail/README.md",
    "crates/purrdf/README.md",
    "docs/CONFORMANCE.md",
    "docs/book/src/entailment.md",
    "docs/book/src/introduction.md",
    "docs/book/src/project/conformance.md",
)

# The literal phrase that SCOPES a claim to the corpus it was measured on. A sentence
# carrying it is making a bounded statement — "50 / 50 on this vendored W3C corpus" — and
# is exempt; one without it is making the unbounded statement the ban is about.
_CORPUS_SCOPE = "on this vendored W3C corpus"

# Each entry is (compiled pattern, why it is banned). The patterns are deliberately
# narrow: they name the specific unbounded claim rather than the words it is built from,
# so "complete" and "full" remain writable about the things they are true of — a complete
# RULE TABLE, a full closure of one document — and only the sentence that promotes them
# into a claim about a SPECIFICATION is caught.
_BANNED_OVERCLAIMS: tuple[tuple[re.Pattern[str], str], ...] = (
    (
        re.compile(r"\b(complete|full)[a-z]*\s+(?:the\s+)?OWL 2 RDF-Based semantics", re.I),
        "the RDF-Based semantics is not finitely axiomatizable by a rule table; PurRDF "
        "implements a profile's rule table plus five named mechanisms, not the semantics",
    ),
    (
        re.compile(r"\b(complete|full)[a-z]*\s+OWL 2 conformance", re.I),
        "OWL 2 conformance is defined per syntax and per semantics over the whole test "
        "suite; what is measured here is one vendored subset of one corpus",
    ),
    (
        re.compile(r"\b(complete|full)[a-z]*\s+OWL 2 RL entailment", re.I),
        "78 / 78 is RULE-TABLE coverage. Entailment conformance is a different claim, "
        "measured separately and only over the cases actually vendored",
    ),
    (
        re.compile(r"\bfully conformant\b", re.I),
        "conformance is per specification clause and per corpus; `fully conformant` names "
        "neither, so nothing can check it",
    ),
    (
        re.compile(r"\b(faster|fastest|outperform[a-z]*)\b", re.I),
        "a comparative performance claim needs a named competitor, a named workload and a "
        "reproducible measurement; this repository's benches are report-only and assert "
        "no speedup",
    ),
)


def _sentences(text: str) -> list[tuple[int, str]]:
    """`(line number, sentence)` for every sentence of `text`.

    Sentence-scoped rather than line-scoped because the exemption is: the SENTENCE
    carrying the claim also carries the scope phrase. A line-scoped check would exempt a
    claim whose scope sits on the previous wrapped line and refuse one whose scope sits on
    the next, which is a property of the paragraph reflow rather than of the prose.

    Markdown table rows are one "sentence" per cell, because a table cell is a standalone
    statement and the row's other cells are not its context.
    """
    out: list[tuple[int, str]] = []
    for offset, line in enumerate(text.splitlines(), start=1):
        pieces = line.split("|") if line.lstrip().startswith("|") else [line]
        for piece in pieces:
            for sentence in re.split(r"(?<=[.!?])\s+", piece):
                if sentence.strip():
                    out.append((offset, sentence))
    return out


def banned_entailment_overclaims() -> tuple[list[str], int]:
    """The unbounded entailment claims this documentation may not make.

    Modelled on :func:`banned_stale_fragment_names`, and run from the same ``main``, for
    the same reason: some statements are wrong in a way no NUMBER check can see. Every
    other gate in this file compares a documented figure against a generated one, which
    catches a stale count and is blind to a sentence that states no count at all.

    It exists at exactly the moment it is most needed. The documentation these nine files
    carry was rewritten from "here are the known gaps" to "50 / 50, ledger empty", and
    that is precisely when a `complete OWL 2 RL entailment` sentence gets written — the
    numbers really did all reach their ceilings, and the step from "every vendored case
    agrees" to "the implementation is complete" is one short sentence and one large lie.

    Five claims are banned, each with its own reason (see ``_BANNED_OVERCLAIMS``), and
    each is exempt when the SENTENCE carrying it also carries the literal phrase
    ``on this vendored W3C corpus``. The exemption is a literal string rather than a
    pattern on purpose: a bounded claim has to say what it is bounded BY, and a phrase
    a writer must type verbatim is one a reader can search for.

    Returns the problems and the number of files scanned, so the script's headline
    reports what it really read.
    """
    problems: list[str] = []
    for relative in _ENTAILMENT_CLAIM_DOCS:
        path = _REPO / relative
        if not path.is_file():
            raise SystemExit(
                f"check-doc-claims: {relative} is in _ENTAILMENT_CLAIM_DOCS and does not "
                f"exist; the entailment documentation moved, so update the list rather "
                f"than leaving the ban silently narrower"
            )
        for line, sentence in _sentences(_read(path)):
            if _CORPUS_SCOPE in sentence:
                continue
            for pattern, why in _BANNED_OVERCLAIMS:
                match = pattern.search(sentence)
                if match:
                    problems.append(
                        f"{relative}:{line}: banned entailment overclaim "
                        f"`{match.group(0)}` — {why}. Scope the sentence with the literal "
                        f"phrase {_CORPUS_SCOPE!r}, or say the bounded thing instead"
                    )
    return problems, len(_ENTAILMENT_CLAIM_DOCS)


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


def makefile_measured_size_claim() -> list[str]:
    r"""EVERY byte figure and percentage in the Makefile's budget comment must be current.

    `WASM_SIZE_MEASURED_BYTES` is REPORTED by the build rather than enforced by it — an
    exact byte count moved with the builder's username and path, so equality could not hold
    locally and in CI at once. That makes this claim the only thing keeping the constant and
    the prose around it honest, where before the equality gate backstopped it. The prose has
    drifted before: a "Currently 9_313_841" line outlived the constant by 82,660 bytes,
    inside the very comment block that says "a comment is the one part of this file nothing
    checks".

    The first attempt at this claim keyed on the verbs "Currently" and "measures" followed by
    a figure. It inspected ZERO figures, because the comment wraps as `— measures` / newline /
    `# 9_396_501 bytes`, and no amount of `\s+` crosses the `# ` that opens a continuation
    line. It was "verified" against a single-line phrasing introduced to test it, which is the
    error of checking a gate against the shape you wrote rather than the shape in the file.

    So this reads the block with comment markers stripped, keys on nothing, and requires every
    underscored byte figure to be either a current constant or part of a `A -> B` / `between A
    and B` range that the prose marks as historical. Percentages must match the real headroom.
    A vacuity guard asserts a figure was actually inspected, because the whole failure this
    replaces was a check that quietly matched nothing.
    """
    problems: list[str] = []
    text = _read(_MAKEFILE)
    rel = _MAKEFILE.relative_to(_REPO)
    measured = re.search(r"^WASM_SIZE_MEASURED_BYTES := (\d+)$", text, re.MULTILINE)
    budget = re.search(r"^WASM_SIZE_BUDGET_BYTES := (\d+)$", text, re.MULTILINE)
    if not measured or not budget:
        raise SystemExit(
            f"check-doc-claims: {rel} no longer defines both wasm size constants; the "
            f"claim cannot be checked, so do not leave it unchecked"
        )
    measured_bytes = int(measured.group(1))
    budget_bytes = int(budget.group(1))
    current = {measured_bytes, budget_bytes}

    # The comment block, with the `# ` continuation markers removed so a figure that wrapped
    # onto its own line reads as ordinary running prose.
    block = re.sub(r"(?m)^#[ \t]?", "", text[: measured.start()])

    # A figure is HISTORICAL when the prose puts it in a range: `A -> B`, `A to B`,
    # `between A and B`, `above A`, `behind`/`drifted`. Those describe a movement rather than
    # asserting today's size, and the branch's own attribution needs them.
    historical: set[int] = set()
    # A region the prose TAGS as a past measurement. The ablation table records what each
    # capability cost when it landed, which is exactly the evidence the attribution above
    # rests on; those figures are history by construction and must not be forced to today's
    # value. The tag is machine-readable rather than prose, because a gate that has to infer
    # "this paragraph is historical" from wording is the kind of gate that inspects nothing.
    # The region must be TERMINATED. An unbounded `\Z` fallback would let an unclosed tag
    # swallow every figure after it, which is the loophole shape this whole claim exists to
    # close — and moving the tag above the live measurement was exactly how it was exploited.
    for region in re.finditer(
        r"HISTORICAL-MEASUREMENTS:(.*?)END-HISTORICAL", block, re.DOTALL
    ):
        for figure in re.finditer(r"((?:\d{1,3}_)+\d{3})", region.group(1)):
            historical.add(int(figure.group(1).replace("_", "")))
    if "HISTORICAL-MEASUREMENTS:" in block and "END-HISTORICAL" not in block:
        problems.append(
            f"{rel}: the budget comment opens a HISTORICAL-MEASUREMENTS region and never "
            f"closes it with END-HISTORICAL, so every figure after it would be exempt. "
            f"Terminate the region."
        )

    # The two CURRENT constants must each be stated somewhere OUTSIDE a historical region.
    # Without this, tagging a region so that it covers the live measurement exempts the very
    # figures this claim exists to check, and the block reads as documented while asserting
    # nothing about today.
    live_text = re.sub(
        r"HISTORICAL-MEASUREMENTS:.*?END-HISTORICAL", "", block, flags=re.DOTALL
    )
    live_figures = {
        int(m.group(1).replace("_", ""))
        for m in re.finditer(r"((?:\d{1,3}_)+\d{3})", live_text)
    }
    for name, value in (
        ("WASM_SIZE_MEASURED_BYTES", measured_bytes),
        ("WASM_SIZE_BUDGET_BYTES", budget_bytes),
    ):
        if value not in live_figures:
            problems.append(
                f"{rel}: the budget comment never states {name} ({value}) outside a "
                f"HISTORICAL-MEASUREMENTS region. The block must say what the artifact "
                f"measures TODAY; a figure reachable only inside a historical region is a "
                f"record of the past, and tagging a region so it covers the live measurement "
                f"is how this check gets hollowed out"
            )
    for pair in re.finditer(
        r"((?:\d{1,3}_)+\d{3})\s*(?:->|to|and)\s*((?:\d{1,3}_)+\d{3})", block
    ):
        historical.add(int(pair.group(1).replace("_", "")))
        historical.add(int(pair.group(2).replace("_", "")))
    for delta in re.finditer(
        r"(?:above|behind|drifted|between)\D{0,40}?((?:\d{1,3}_)+\d{3})", block
    ):
        historical.add(int(delta.group(1).replace("_", "")))

    inspected = 0
    for figure in re.finditer(r"((?:\d{1,3}_)+\d{3})", block):
        value = int(figure.group(1).replace("_", ""))
        inspected += 1
        if value in current or value in historical:
            continue
        problems.append(
            f"{rel}: the budget comment names {figure.group(1)}, which is neither the "
            f"measured size ({measured_bytes}) nor the ceiling ({budget_bytes}), and is not "
            f"presented as part of a range the prose marks as historical. A figure in this "
            f"block either describes TODAY — in which case it must be one of those two — or "
            f"a movement, in which case write it as `A -> B` or `between A and B`"
        )

    real_headroom = 100.0 * (budget_bytes - measured_bytes) / budget_bytes
    for pct in re.finditer(r"(\d+\.\d+)% headroom", block):
        inspected += 1
        if abs(float(pct.group(1)) - real_headroom) > 0.01:
            problems.append(
                f"{rel}: the budget comment claims {pct.group(1)}% headroom, but "
                f"{measured_bytes} against {budget_bytes} is {real_headroom:.3f}%"
            )

    if inspected == 0:
        raise SystemExit(
            f"check-doc-claims: found no byte figure or percentage in {rel}'s budget "
            f"comment. The first version of this claim inspected nothing for exactly this "
            f"kind of reason; fix the extraction rather than leaving the block unchecked"
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
                r"entailments and correctly withholds on (?P<neg_a>\d+) of "
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
                r"entailments and correctly withholds on (?P<neg_a>\d+) of "
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
            {"consistency": owl2_total - 35, "inconsistency": 35},
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
                "consistency": owl2_total - 35,
                "inconsistency": 35,
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
    ]


def main() -> int:
    inventory = load_rule_inventory()
    matrix = load_matrix()
    crates = load_release_crates()
    census = census_counts()
    lanes = rl_lane_counts()

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
    problems.extend(makefile_measured_size_claim())
    checked += 1
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
    # The ban walk reports how many files it scanned, which is COVERAGE, not a claim
    # count — folding ~1,900 scanned files into the "documented claims" headline would
    # inflate a number readers take as the count of gated statements. The ban is one
    # claim; its reach is printed separately.
    fragment_problems, fragment_files = banned_stale_fragment_names()
    problems.extend(fragment_problems)
    checked += 1
    # The second ban, and the same accounting: one claim, its reach printed separately.
    overclaim_problems, overclaim_files = banned_entailment_overclaims()
    problems.extend(overclaim_problems)
    checked += 1

    for claim in build_claims(inventory, matrix, census, lanes):
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
        f"{overclaim_files} file(s))."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
