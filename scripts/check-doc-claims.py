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

It is pure text-over-committed-files: no cargo, no network, no test run. The
expensive gates prove the generated artifacts are current; this one proves the
prose agrees with them. Run standalone, or as part of
``scripts/check-generated.sh`` (and therefore ``make check``).
"""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

_REPO = Path(__file__).resolve().parent.parent

_INVENTORY = _REPO / "docs" / "book" / "src" / "entailment-rules.md"
_CONFORMANCE = _REPO / "docs" / "CONFORMANCE.md"
_ENTAILMENT = _REPO / "docs" / "book" / "src" / "entailment.md"
_BOOK_CONFORMANCE = _REPO / "docs" / "book" / "src" / "project" / "conformance.md"
_README = _REPO / "README.md"
_RELEASE = _REPO / "docs" / "RELEASE.md"
_RELEASE_CRATES = _REPO / "scripts" / "release-crates.sh"

_MATRIX_BEGIN = "<!-- BEGIN GENERATED: conformance-matrix -->"
_MATRIX_END = "<!-- END GENERATED: conformance-matrix -->"


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
# Source 3 — the release crate set
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


def rule_coverage_table_claim(inventory: dict[str, tuple[int, int]]) -> list[str]:
    """The hand-written 'Rule coverage' table in the entailment chapter.

    Checked structurally rather than by regex: the SET of regimes must match the
    inventory too, so a regime added to ``Regime`` cannot be quietly omitted
    from the chapter, and one deleted cannot linger.
    """
    text = _read(_ENTAILMENT)
    rel = _ENTAILMENT.relative_to(_REPO)
    section = re.search(r"## Rule coverage\n(.*?)(?:\n## |\Z)", text, re.DOTALL)
    if not section:
        return [f"{rel}: no '## Rule coverage' section — the coverage table is gone"]
    rows = re.findall(
        r"^\| `([A-Za-z-]+)` \| [^|]* \| (\d+) \| (\d+) \|$",
        section.group(1),
        re.MULTILINE,
    )
    documented = {name: (int(d), int(i)) for name, d, i in rows}
    problems: list[str] = []
    for name in sorted(set(inventory) - set(documented)):
        problems.append(
            f"{rel}: the Rule coverage table has no row for regime `{name}`, "
            f"which docs/book/src/entailment-rules.md defines"
        )
    for name in sorted(set(documented) - set(inventory)):
        problems.append(
            f"{rel}: the Rule coverage table has a row for regime `{name}`, "
            f"which docs/book/src/entailment-rules.md does not define"
        )
    for name in sorted(set(documented) & set(inventory)):
        if documented[name] != inventory[name]:
            d_def, d_impl = documented[name]
            g_def, g_impl = inventory[name]
            problems.append(
                f"{rel}: Rule coverage row `{name}` documents "
                f"{d_def} defined / {d_impl} implemented, but "
                f"rules()/implemented() report {g_def} / {g_impl} "
                f"(docs/book/src/entailment-rules.md)"
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


def build_claims(
    inventory: dict[str, tuple[int, int]], matrix: dict[str, tuple[int, int]]
) -> list[Claim]:
    owl2_pass, owl2_ledger = matrix["Entailment (OWL 2 DL consistency)"]
    owl2_total = owl2_pass + owl2_ledger
    sparql_pass, sparql_xfail = matrix["SPARQL 1.1/1.2 evaluation (full corpus)"]
    shacl_pass, _ = matrix["SHACL Core + SHACL-SPARQL"]
    corpus_pass, _ = matrix["SHACL (first-party corpus)"]
    shex_pass, _ = matrix["ShEx 2.1 validation"]
    codec_pass, _ = matrix["Syntax codecs (Turtle/TriG/NT/NQ/RDF-XML)"]
    rdflib_pass, rdflib_x = matrix["rdflib LSP drop-in gate"]
    compat_pass, compat_x = matrix["purrdf.compat parity"]

    inv = "docs/book/src/entailment-rules.md, generated from rules()/implemented()"
    mat = "the generated conformance-matrix block in docs/CONFORMANCE.md"

    return [
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
        # --- OWL 2 DL consistency, sourced from the matrix block --------------
        Claim(
            "the OWL 2 DL-consistency scoreboard row",
            _CONFORMANCE,
            r"\*\*(?P<passed>\d+) / (?P<total>\d+)\*\* agreeing verdicts · "
            r"(?P<ledgered>\d+) typed-ledger divergences",
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
            r"Every one of the (?P<ledgered>\d+) divergences is named in a typed "
            r"ledger",
            {"ledgered": owl2_ledger},
            mat,
        ),
        Claim(
            "the OWL 2 row in the README conformance table",
            _README,
            r"\*\*(?P<passed>\d+) / (?P<total>\d+)\*\* agreeing, "
            r"(?P<ledgered>\d+) ledgered, 0 unledgered",
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
            r"\(ledgered\) \|\n\| purrdf\.compat parity",
            {"passed": rdflib_pass, "xfail": rdflib_x},
            mat,
        ),
        Claim(
            "the purrdf.compat parity scoreboard row",
            _CONFORMANCE,
            r"first-party differential vs rdflib 7\.6 \| \*\*(?P<passed>\d+)\*\* "
            r"pass · (?P<xfail>\d+) strict-xfail",
            {"passed": compat_pass, "xfail": compat_x},
            mat,
        ),
    ]


def main() -> int:
    inventory = load_rule_inventory()
    matrix = load_matrix()
    crates = load_release_crates()

    problems: list[str] = []
    checked = 0

    problems.extend(rule_coverage_table_claim(inventory))
    checked += 1
    problems.extend(release_crate_list_claim(crates))
    checked += 1

    for claim in build_claims(inventory, matrix):
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

    print(f"OK: {checked} documented claim(s) agree with their generated source.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
