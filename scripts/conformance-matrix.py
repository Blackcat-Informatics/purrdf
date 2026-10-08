# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
"""PurRDF single conformance matrix.

Runs every conformance-relevant suite — the native Rust W3C harnesses AND the
Python rdflib drop-in gate — and prints ONE scoreboard table with per-suite
pass / xfail-or-skip / fail counts and an overall RED/GREEN verdict.

It is the umbrella that `make conformance` invokes. `make check` (pure-Rust
gate) and `make pytest` (Python gate) stay separate; this script re-runs their
conformance slices together so CI can publish a single matrix.

Design notes:
  * Each suite's own harness already enforces exact fixture totals and XPASS
    ledger discipline internally (AGENTS.md §2). This aggregator does NOT
    re-implement that; it runs each harness, trusts its exit code for the
    RED/GREEN gate, and scrapes the harness's own scoreboard line for the
    richer fixture-level counts shown in the matrix.
  * Deterministic and re-runnable: suites run in a fixed order, output parsing
    is exact-regex, and the process exit code is non-zero iff any suite has an
    unexpected failure (a red cargo/pytest run, an XPASS, or a stale ledger key).
  * When `$GITHUB_STEP_SUMMARY` is set (CI), the matrix is also appended there
    as a Markdown table so it lands in the job summary, not just the log.
  * A scrape that MISSES fails CLOSED. Every scraped row reports the harness's
    own per-case scoreboard; when that line stops being emitted the row cannot
    silently degrade to the handful of Rust test functions `cargo test` counted
    and stay GREEN. `_no_scoreboard` turns the miss into a RED row naming the
    marker that went missing and the command that owed it. A gate that cannot
    fail is not a gate, and a corpus tally nobody measures is not a measurement.
  * `self_test` proves that fail-closed property rather than asserting it: it
    drives every scraper over a specimen of its harness's output through
    `_RUN_STUB`, requires the whole specimen to be RECOGNISED (so a specimen
    gone stale fails loudly instead of testing nothing), then withholds one
    scoreboard line at a time and requires each row to go RED with a non-zero
    fail. It runs BEFORE any harness starts on every invocation — it is pure
    text over strings, so it costs no build and no I/O — and standalone under
    `--self-test`.

Usage:
    python3 scripts/conformance-matrix.py            # full matrix
    python3 scripts/conformance-matrix.py --no-python  # native Rust suites only
    python3 scripts/conformance-matrix.py --self-test  # scrape fail-closed proof

    # The same matrix on several machines, then one verdict (what CI runs):
    python3 scripts/conformance-matrix.py --shard core --emit-results R/core.json
    ...                                   (one run per name in SHARDS)
    python3 scripts/conformance-matrix.py --from-results R
"""

from __future__ import annotations

import argparse
import contextlib
import difflib
import json
import os
import re
import shlex
import subprocess
import sys
from collections.abc import Callable, Iterator
from dataclasses import asdict, dataclass, field, fields
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parent.parent
_PY_DIR = _REPO_ROOT / "bindings" / "python"
_BASELINE_PATH = _REPO_ROOT / "scripts" / "conformance-baseline.json"
_DOC_PATH = _REPO_ROOT / "docs" / "CONFORMANCE.md"
_DOC_BEGIN = "<!-- BEGIN GENERATED: conformance-matrix -->"
_DOC_END = "<!-- END GENERATED: conformance-matrix -->"

# ---------------------------------------------------------------------------
# Result model
# ---------------------------------------------------------------------------


@dataclass
class SuiteResult:
    """One row of the conformance matrix."""

    name: str
    source: str
    passed: int = 0
    xskip: int = 0  # xfailed OR trait-skipped OR allowlisted-gap (never silent)
    failed: int = 0
    detail: str = ""
    ok: bool = False
    budget: int | None = None  # ratchet ceiling from conformance-baseline.json
    # Set only by `_no_scoreboard`: the harness ran but did not emit the
    # per-case scoreboard line this row is scraped from, so `xskip` is not a
    # ledgered-gap count at all and the ratchet must not diagnose it as one.
    scoreboard_missing: bool = False
    log: str = field(default="", repr=False)

    @property
    def status(self) -> str:
        return "GREEN" if self.ok else "RED"


# ---------------------------------------------------------------------------
# Command runner + scoreboard scrapers
# ---------------------------------------------------------------------------


# The self-test's ONLY injection point: one canned (returncode, output) pair
# substituted for the harness a scraper would otherwise spawn. Every scraper
# reaches its harness through `_run`, so setting this drives the real scraping
# code — the regex under test — over specimen text with no build and no cargo.
# `None` outside `_stubbed_run`, which is the only writer.
_RUN_STUB: Callable[[list[str], Path], tuple[int, str]] | None = None


@contextlib.contextmanager
def _stubbed_run(out: str, rc: int = 0) -> Iterator[None]:
    """Answer every `_run` inside this block with (*rc*, *out*)."""
    global _RUN_STUB  # noqa: PLW0603 - the injection point is deliberately global
    previous = _RUN_STUB
    _RUN_STUB = lambda _cmd, _cwd: (rc, out)  # noqa: E731
    try:
        yield
    finally:
        _RUN_STUB = previous


def _run(cmd: list[str], cwd: Path) -> tuple[int, str]:
    """Run *cmd*, return (returncode, combined stdout+stderr)."""
    if _RUN_STUB is not None:
        return _RUN_STUB(cmd, cwd)
    proc = subprocess.run(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        check=False,
    )
    return proc.returncode, proc.stdout


def _cargo_tally(out: str) -> tuple[int, int, int]:
    """Sum every `test result: ...` line into (passed, ignored, failed)."""
    passed = ignored = failed = 0
    seen = False
    for m in re.finditer(
        r"test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored", out
    ):
        seen = True
        passed += int(m.group(1))
        failed += int(m.group(2))
        ignored += int(m.group(3))
    if not seen:
        # No summary line at all (e.g. compile error) — treat as a hard failure.
        return 0, 0, -1
    return passed, ignored, failed


def _suite_cargo(
    name: str, source: str, cmd: list[str], detail: str = ""
) -> SuiteResult:
    rc, out = _run(cmd, _REPO_ROOT)
    passed, ignored, failed = _cargo_tally(out)
    return SuiteResult(
        name=name,
        source=source,
        passed=passed,
        xskip=ignored,
        # Preserve the -1 "no scoreboard / compile error" sentinel so the
        # ratchet skips its budget check (a compile failure is already RED and
        # must not be re-diagnosed as "LEDGER SHRANK"); render/totals already
        # treat failed < 0 as "err".
        failed=failed,
        detail=detail,
        ok=(rc == 0 and failed == 0),
        log=out,
    )


def _no_scoreboard(
    name: str, source: str, marker: str, cmd: list[str], out: str
) -> SuiteResult:
    """A scraped suite whose harness did NOT emit its scoreboard line: hard RED.

    This is the fail-CLOSED replacement for falling back to ``_suite_cargo``.
    The fallback returned whatever `cargo test` reported, which for a corpus
    harness is a handful of Rust test *functions* — so a suite whose per-case
    scoreboard silently stopped being emitted kept printing a plausible small
    number in the Pass column and stayed GREEN. The corpus was no longer being
    measured and nothing said so.

    A missing marker is therefore treated as one failure of the suite's own
    contract: ``failed=1`` so the row renders ``FAIL 1 / RED`` rather than the
    self-contradicting ``FAIL 0 / RED``, and the detail names both the marker
    that went missing and the exact command that owed it, because the next
    person to read this row needs to fix a harness, not re-run a matrix.
    """
    return SuiteResult(
        name=name,
        source=source,
        passed=0,
        xskip=0,
        failed=1,
        detail=(
            f"NO SCOREBOARD: `{shlex.join(cmd)}` did not emit its "
            f"{marker} line, so this row has no per-case measurement behind it. "
            "The cargo tally is NOT a substitute — it counts test functions, not "
            "fixtures. Restore the harness's scoreboard line or re-point the "
            "scraper; do not let the row report a number it did not measure"
        ),
        ok=False,
        scoreboard_missing=True,
        log=out,
    )


def _scrape(
    name: str, source: str, crate: str, test: str, marker: str, pattern: str,
    build: Callable[..., tuple[int, int, int, str, bool]], *extra: str,
) -> SuiteResult:
    """Run one native harness and build its row from the integer groups of its
    scoreboard line `pattern`; `build` returns (pass, xskip, fail, detail, ok).
    A missing line is the hard-RED `_no_scoreboard` row naming `marker`."""
    cmd = ["cargo", "test", "-p", crate, "--locked", "--test", test, "--", "--nocapture", *extra]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, cargo_failed = _cargo_tally(out)
    m = re.search(pattern, out)
    if not m:
        return _no_scoreboard(name, source, marker, cmd, out)
    passed, xskip, failed, detail, ok = build(*(int(g) for g in m.groups()))
    return SuiteResult(
        name, source, passed=passed, xskip=xskip, failed=failed, detail=detail,
        ok=(rc == 0 and cargo_failed == 0 and ok), log=out,
    )


def _passed_total(
    name: str, source: str, package: str, test: str, marker: str,
    detail: Callable[[int, int], str],
) -> SuiteResult:
    """One `<marker>: passed N total N` scoreboard, nothing ledgered: Fail is total - Pass."""
    return _scrape(
        name, source, package, test, f"`{marker}: passed N total N`",
        rf"{marker}: passed (\d+) total (\d+)",
        lambda passed, total: (
            passed, 0, total - passed, detail(passed, total), passed == total,
        ),
    )


def _suite_codec() -> SuiteResult:
    """Turtle/TriG/N-Triples/N-Quads/RDF-XML native-codec round-trip."""
    return _scrape(
        "Syntax codecs (Turtle/TriG/NT/NQ/RDF-XML)", "W3C rdf-tests", "purrdf-rdf",
        "native_codec_conformance", "`TOTAL: total N passed N allowlisted-gap N`",
        r"TOTAL: total\s+(\d+)\s+passed\s+(\d+)\s+allowlisted-gap\s+(\d+)",
        lambda total, passed, gap: (
            passed, gap, total - passed - gap,
            f"{passed}/{total} vectors round-trip; {gap} allowlisted gaps", True,
        ),
    )


def _suite_shacl_w3c() -> SuiteResult:
    """The vendored W3C SHACL 1.0 suite plus the `af/` seam: scrape the harness's
    own `TOTAL` line. Ledgered xfails are counted in the XFail/Skip column."""
    return _scrape(
        "SHACL Core + SHACL-SPARQL", "W3C data-shapes", "purrdf-shapes", "w3c_conformance",
        "`TOTAL: passed N, xfailed N, ledger N`",
        r"TOTAL: passed (\d+), xfailed (\d+), ledger (\d+)",
        lambda passed, xfailed, _ledger: (
            passed, xfailed, 0, f"{passed} pass as approved · {xfailed} ledgered", True,
        ),
    )


_SHACL12_NAME = "SHACL 1.2 (Core, SPARQL, node expressions, rules, SPARQL RL)"
_SHACL12_SOURCE = "W3C shacl12-test-suite"
_SHACL12_UNLISTED_NAME = "SHACL 1.2 unlisted vendored files"
_SHACL12_UNLISTED_SOURCE = "W3C shacl12-test-suite files no manifest includes"


def _suite_shacl12_w3c() -> SuiteResult:
    """The vendored W3C SHACL 1.2 suite, every test type: scrape the harness's
    own `W3C12 TOTAL` line so the row counts suite ENTRIES (sht:Validate,
    sht:EvalNodeExpr, sht:Infer and the seven srlt: types), not the handful of
    Rust test functions the cargo tally would report.

    Only the APPROVED suite — the entries an upstream manifest lists — is this
    row. Pass is the entries that agree with their approved expectation exactly.
    The entries whose approved result spells a computed decimal non-canonically
    (graded by substituting the XSD 1.1 canonical spelling) are counted in the
    XFail/Skip column and named in the detail, never folded into Pass. The entries of vendored files no manifest includes are
    their own row, `_suite_shacl12_unlisted`."""
    return _scrape(
        _SHACL12_NAME, _SHACL12_SOURCE, "purrdf-shapes", "w3c12_conformance",
        "`W3C12 TOTAL: passed N, non-canonical-expected-decimal N, xfailed N, ledger N`",
        r"W3C12 TOTAL: passed (\d+), non-canonical-expected-decimal (\d+), "
        r"xfailed (\d+), ledger (\d+)",
        lambda passed, noncanonical, xfailed, _ledger: (
            passed, noncanonical + xfailed, 0,
            (
                f"{passed} pass as approved · {noncanonical} non-canonical expected "
                f"decimals (graded by the XSD 1.1 canonical spelling) · {xfailed} ledgered"
            ),
            True,
        ),
    )


def _suite_shacl12_unlisted() -> SuiteResult:
    """The entries of vendored SHACL 1.2 files that NO upstream manifest
    includes: graded exactly against their own file (one with a proven delta)
    by their own test, and reported as their own row so they are never counted
    among the approved suite's passes."""
    return _scrape(
        _SHACL12_UNLISTED_NAME, _SHACL12_UNLISTED_SOURCE, "purrdf-shapes", "w3c12_conformance",
        "`W3C12 UNLISTED: passed N, exact N, with-delta N, total N`",
        r"W3C12 UNLISTED: passed (\d+), exact (\d+), with-delta (\d+), total (\d+)",
        lambda passed, exact, with_delta, total: (
            passed, 0, total - passed,
            (
                f"{total} entries of vendored files no upstream manifest includes, graded "
                f"apart from the approved suite: {exact} exactly as written · {with_delta} "
                "with a proven delta"
            ),
            passed == total,
        ),
        "--exact", "w3c_shacl12_unlisted_vendored_files",
    )


def _suite_shapes_corpus() -> SuiteResult:
    """First-party SHACL corpus: scrape the harness's per-fixture scoreboard so
    the matrix reports a report-level Pass count, not the single test-function
    tally that ``_suite_cargo`` would yield."""
    return _passed_total(
        "SHACL (first-party corpus)", "first-party frozen reports", "purrdf-shapes",
        "conformance", "SHAPES-CORPUS",
        lambda passed, total: f"{passed}/{total} byte-frozen expected reports",
    )


def _suite_community_shacl() -> SuiteResult:
    """Community SHACL corpus: every applicable dated execution through the fresh,
    prepared and restored-product complete-report routes, graded by the
    conformance kit. The harness counts unsupported profiles apart from failures
    and asserts there are none; here an unsupported execution is not a pass."""
    return _passed_total(
        "Community SHACL dated profiles (REC 2017 / WD 2026)",
        "community-proposed corpus, agent-reviewed", "purrdf-sparql-conformance",
        "community_conformance", "COMMUNITY-SHACL",
        lambda passed, total: f"{passed}/{total} applicable dated executions",
    )


def _suite_product_equivalence() -> SuiteResult:
    """Prepared-shapes-product equivalence over BOTH SHACL corpora.

    Not a second SHACL grading -- the two rows above already decide whether the
    engine's answer is right. This one decides whether the prepared-product
    codec CHANGES that answer: every shapes graph in `vectors/shacl/` and
    `crates/shapes/corpus/` is validated three ways (parsed from source, packed
    and admitted, packed and rebuilt from the carried dataset) and the three
    reports must be byte-identical as N-Triples.

    The Pass column is shapes graphs whose three lanes agreed; the ledger column
    is shapes graphs the product writer REFUSES, and its budget is the thing
    that makes over-refusal visible -- a codec that quietly stopped packing a
    construct would otherwise move those cases out of the comparison and leave
    every remaining comparison passing."""
    cmd = [
        "cargo", "test", "-p", "purrdf-shapes", "--locked",
        "--test", "product_corpus_equivalence", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, failed = _cargo_tally(out)
    m = re.search(
        r"PRODUCT-EQUIVALENCE: passed (\d+) ledgered (\d+) unparsable (\d+) "
        r"disagreed (\d+) total (\d+)",
        out,
    )
    if m:
        passed, ledgered, unparsable, disagreed, total = (
            int(m.group(i)) for i in (1, 2, 3, 4, 5)
        )
        detail = (
            f"{passed}/{total} shapes graphs agree across parse/admit/rebuild; "
            f"{ledgered} refused by the product writer; {unparsable} whose shapes graph "
            "the conformance harnesses require the loader to refuse (a declared "
            "sht:Failure)"
        )
        return SuiteResult(
            "SHACL prepared-product equivalence",
            "W3C data-shapes + shacl12-test-suite + first-party corpus",
            passed=passed, xskip=ledgered,
            failed=(disagreed + total - passed - ledgered - unparsable),
            detail=detail, ok=(rc == 0 and failed == 0 and disagreed == 0), log=out,
        )
    return _no_scoreboard(
        "SHACL prepared-product equivalence",
        "W3C data-shapes + shacl12-test-suite + first-party corpus",
        "`PRODUCT-EQUIVALENCE: passed N ledgered N unparsable N disagreed N total N`",
        cmd, out,
    )


def _suite_xsd_regex_corpus() -> SuiteResult:
    """First-party XSD/XPath `regExp` corpus: scrape the harness's per-case
    scoreboard so the matrix reports the case count rather than the handful of
    test functions that ``_suite_cargo`` would count.

    The suite grades ``purrdf_core::xsd_regex::compile`` -- the one shared
    dialect translation ``sh:pattern``, SPARQL ``REGEX``/``REPLACE`` and ShEx
    ``PATTERN`` all route through -- so a dialect regression shows up here
    once rather than three times, or not at all."""
    return _passed_total(
        "XSD/XPath regExp (first-party corpus)", "first-party, XSD G + F&O 5.6",
        "purrdf-core", "xsd_regex_conformance", "XSD-REGEX-CORPUS",
        lambda passed, total: f"{passed}/{total} hand-derived XSD/XPath regExp cases",
    )


def _suite_shacl_rules() -> SuiteResult:
    """SHACL Rules (`sh:rule` inference): scrape the harness's per-fixture
    scoreboard so the matrix reports the inferred-graph fixture count rather than
    the single test-function tally that ``_suite_cargo`` would yield."""
    return _scrape(
        "SHACL Rules", "DASH + first-party", "purrdf-shapes", "rules_conformance",
        "`RULES: passed N total N`", r"RULES: passed (\d+) total (\d+)",
        lambda passed, total: (
            passed, total - passed, 0, f"{passed}/{total} inferred-graph fixtures",
            passed == total,
        ),
    )


def _suite_shex_validation() -> SuiteResult:
    return _scrape(
        "ShEx 2.1 validation", "shexTest v2.1.0", "purrdf-shex", "validation_conformance",
        "`entries N | attempted N | pass N | xfail N | fail N | skipped N`",
        r"entries (\d+) \| attempted (\d+) \| pass (\d+) \| xfail (\d+) "
        r"\| fail (\d+) \| skipped (\d+)",
        lambda _entries, attempted, passed, xfail, fail, skipped: (
            passed, xfail + skipped, fail,
            f"{passed}/{attempted} attempted · {skipped} trait-skips", fail == 0,
        ),
    )


_SPARQL_NAME = "SPARQL 1.0/1.1/1.2 evaluation (full corpus)"
_SPARQL10_UNLISTED_NAME = "SPARQL 1.0 unlisted vendored files"
_SPARQL10_SUPERSEDED_NAME = "SPARQL 1.0 cases superseded by RDF 1.2"
_SPARQL10_UNLISTED_SOURCE = "W3C data-r2 files no upstream manifest lists"


def _suite_sparql10_unlisted() -> SuiteResult:
    """The vendored data-r2 files no upstream manifest lists, accounted file by
    file by their own native test: the one described entry the optional-filter
    group leaves out, graded against the SPARQL 1.1 reading that replaced it;
    the orphan `sameTerm-manifest.ttl` and the files only it names, whose
    anonymous untyped entries the loader refuses; and the payloads no manifest
    names. A new unlisted file, or one of these becoming listed, fails the test.
    Only the graded entry is a Pass; the other files declare no test."""
    return _scrape(
        _SPARQL10_UNLISTED_NAME, _SPARQL10_UNLISTED_SOURCE, "purrdf-sparql-conformance",
        "sparql10_inventory",
        "`W3C10 UNLISTED: files N, orphan-manifest-files N, unreferenced-payloads N, "
        "unlisted-entry-results N`",
        r"W3C10 UNLISTED: files (\d+), orphan-manifest-files (\d+), "
        r"unreferenced-payloads (\d+), unlisted-entry-results (\d+)",
        lambda files, orphan, unreferenced, entries: (
            entries, 0, 0,
            (
                f"{files} vendored files no upstream manifest lists: {entries} described "
                "entry the group leaves out, graded against the SPARQL 1.1 reading that "
                f"replaced it · {orphan} files of the orphan sameTerm manifest, whose "
                f"anonymous untyped entries cannot load · {unreferenced} payloads no "
                "manifest names"
            ),
            True,
        ),
        "--exact", "every_vendored_data_r2_file_is_listed_or_an_exact_unlisted_remainder",
    )


def _suite_sparql10_superseded() -> SuiteResult:
    """Data-r2 cases whose frozen answer RDF 1.2 makes unreachable, graded apart:
    `dawg-sort-11` puts a simple literal before the same-spelled `xsd:string`, one
    term in RDF 1.2, a rule SPARQL 1.2 §15.1 dropped. Its test asserts the SPARQL
    1.2 order over the frozen exact terms, and that only the frozen order differs."""
    return _scrape(
        _SPARQL10_SUPERSEDED_NAME, "W3C data-r2 extended evaluation root",
        "purrdf-sparql-conformance", "sparql10_inventory",
        "`W3C10 SUPERSEDED: graded N, sparql12-order N`",
        r"W3C10 SUPERSEDED: graded (\d+), sparql12-order (\d+)",
        lambda graded, ordered: (
            ordered, 0, graded - ordered,
            (
                f"{ordered}/{graded} graded against the SPARQL 1.2 order (§15.1, RDF 1.2 "
                "simple literal = xsd:string): the frozen terms exactly, the frozen order not"
            ),
            ordered == graded,
        ),
        "--exact", "the_extended_sort_case_is_graded_against_the_sparql12_order",
    )


def _suite_sparql() -> SuiteResult:
    # Each manifest case writes its tally to stderr.  Serialise the cases so the
    # runner's progress output cannot splice through those tally lines.
    cmd = [
        "cargo",
        "test",
        "-p",
        "purrdf-sparql-conformance",
        "--locked",
        "--test",
        "sparql_conformance",
        "--",
        "--nocapture",
        "--test-threads=1",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, cargo_failed = _cargo_tally(out)
    passed = xfail = unexpected = failed = 0
    matched = False
    for m in re.finditer(
        r"\] (\d+) passed, (\d+) xfail, (\d+) unexpected-pass, (\d+) failed, (\d+) unmodeled",
        out,
    ):
        matched = True
        passed += int(m.group(1))
        xfail += int(m.group(2))
        unexpected += int(m.group(3))
        failed += int(m.group(4))
    if matched:
        detail = f"{passed} pass · {xfail} xfail (ledgered)"
        return SuiteResult(
            _SPARQL_NAME,
            "W3C sparql10 + sparql11 + sparql12 + first-party",
            passed=passed, xskip=xfail, failed=failed + unexpected,
            detail=detail,
            ok=(rc == 0 and cargo_failed == 0 and failed == 0 and unexpected == 0),
            log=out,
        )
    return _no_scoreboard(
        _SPARQL_NAME,
        "W3C sparql10 + sparql11 + sparql12 + first-party",
        "per-manifest `[<manifest>] N passed, N xfail, N unexpected-pass, "
        "N failed, N unmodeled`",
        cmd,
        out,
    )


def _suite_construct_corpus() -> SuiteResult:
    """First-party CONSTRUCT corpus (`crates/sparql-conformance/corpus/construct/`).

    Its own row rather than a fold into the SPARQL row: the corpus exists so a
    consumer can read CONSTRUCT coverage — both the triple-producing §16.2 form
    and the quad-producing `CONSTRUCT GRAPH <iri>` form — off the scoreboard, and
    a number folded into a four-digit total answers nobody's question. Every case
    is graded and there is no xfail ledger, so a non-zero fail cannot appear
    without the harness going red.
    """
    return _passed_total(
        "SPARQL CONSTRUCT (first-party corpus)", "purrdf-construct (first-party)",
        "purrdf-sparql-conformance", "construct_corpus", "CONSTRUCT-CORPUS",
        lambda passed, total: (
            f"{passed}/{total} cases: triple-producing §16.2 + CONSTRUCT GRAPH quads, "
            "paired case for case, incl. the RDF 1.2 statement layer and its "
            "per-graph keying and BOTH terms a subject position refuses (a "
            "literal and a triple term) at BOTH depths it applies (an asserted "
            "subject and the subject of a triple term nested in an object, the "
            "depth at which an unenforced term model emits a document the "
            "engine's own readers refuse); the quad-template grammar bounded "
            "from both sides by 1 positive and 7 negative syntax verdicts"
        ),
    )


def _suite_describe_corpus() -> SuiteResult:
    """First-party DESCRIBE corpus (`crates/sparql-conformance/corpus/describe/`).

    §16.4 leaves the description implementation-defined, so no vendored manifest
    grades a DESCRIBE at all — this row is the only conformance measurement the
    form has, and it pins the engine's documented Symmetric CBD case by case.
    """
    return _passed_total(
        "SPARQL DESCRIBE (first-party corpus)", "purrdf-describe (first-party)",
        "purrdf-sparql-conformance", "describe_corpus", "DESCRIBE-CORPUS",
        lambda passed, total: (
            f"{passed}/{total} cases pinning the symmetric CBD, incl. the RDF 1.2 "
            "statement layer on both sides of its subject-or-object disjunction and "
            "its per-graph scope over TriG"
        ),
    )


def _suite_cdt_corpus() -> SuiteResult:
    """Vendored SEP-0009 SPARQL Composite Datatypes corpus (`vectors/sparql-cdt/`).

    The INDEPENDENT oracle for `cdt:List`/`cdt:Map`, `FOLD` and `UNFOLD`. Its own
    row rather than a fold into the SPARQL row for the reason the CONSTRUCT and
    DESCRIBE corpora have theirs: a consumer asking whether this engine does
    SEP-0009 must be able to read the answer off the scoreboard instead of out of
    a four-digit total, and the answer here carries a documented lexical-space
    divergence that a merged number would bury.
    """
    cmd = [
        "cargo", "test", "-p", "purrdf-sparql-conformance", "--locked",
        "--test", "cdt_corpus", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, failed = _cargo_tally(out)
    m = re.search(r"SPARQL-CDT-CORPUS: passed (\d+) xfail (\d+) total (\d+)", out)
    if m:
        passed, xfail, total = (int(m.group(i)) for i in (1, 2, 3))
        detail = (
            f"{passed}/{total} vendored upstream cases (awslabs/SPARQL-CDTs, commit "
            "e0a7465) across six groups: the list and map function libraries, the "
            "FOLD aggregate and the UNFOLD graph pattern, ORDER BY over composite "
            "values, and blank-node identity inside composite lexical forms. "
            "PurRDF reads TWO forms outside the published lexical space — an RDF "
            "1.2 triple term and a directional language-tagged literal as list "
            "elements or map values — which a conformant SEP-0009 reader calls "
            "ill-formed; `crates/sparql-conformance/tests/cdt_lexical_divergence.rs` "
            "grades every composite literal in every corpus this workspace ships "
            "and proves not one of them needs either"
        )
        return SuiteResult(
            "SPARQL CDT (SEP-0009, vendored corpus)", "awslabs/SPARQL-CDTs",
            passed=passed, xskip=xfail, failed=(total - passed - xfail),
            detail=detail, ok=(rc == 0 and failed == 0 and passed + xfail == total),
            log=out,
        )
    return _no_scoreboard(
        "SPARQL CDT (SEP-0009, vendored corpus)", "awslabs/SPARQL-CDTs",
        "`SPARQL-CDT-CORPUS: passed N xfail X total M`", cmd, out,
    )


def _suite_governor_corpus() -> SuiteResult:
    """First-party frozen execution-governor corpus.

    Scrapes the harness's own scoreboard so the matrix reports the *case* count —
    zero / boundary / over-bound per governor, plus the RDF 1.2 statement layer,
    the federated SERVICE seam and the deadline case — rather than the handful of
    test functions ``_suite_cargo`` would count. Every case is graded, so a
    non-zero fail is impossible to reach without the harness itself going red.
    """
    return _scrape(
        "SPARQL execution governors", "purrdf-sparql-governors (first-party)",
        "purrdf-sparql-conformance", "governor_corpus",
        "`GOVERNOR-CORPUS: passed N total N bands N`",
        r"GOVERNOR-CORPUS: passed (\d+) total (\d+) bands (\d+)",
        lambda passed, total, bands: (
            passed, 0, total - passed,
            f"{passed}/{total} pinned cases; {bands} zero/boundary/over-bound bands, "
            "frozen and content-addressed",
            passed == total,
        ),
    )


_GEO_GOLDEN = (
    _REPO_ROOT / "crates" / "geo" / "tests" / "determinism.rs"
)


def _suite_geo_determinism() -> SuiteResult:
    """`purrdf-geo`'s exact-arithmetic determinism corpus, against its pinned golden.

    This row deliberately does NOT count `cargo test` test functions, and the
    reason is worth recording because the first attempt at it did. A cargo tally
    over this crate's five integration binaries measured 33 on one machine and 37
    on the CI runner from byte-identical source, which made the doc drift-guard
    red for a reason that had nothing to do with GeoSPARQL. That is the failure
    `_no_scoreboard` exists to name: a test-function tally is not a measurement of
    a corpus, and a number that moves with the build environment is not a
    measurement at all.

    What IS stable, and what this row reports, is the lane's own fixture-level
    claim: the `purrdf_geo::determinism::CORPUS` geometries whose SERIALIZED bytes
    — WKT and GeoJSON renderings, DE-9IM matrix strings, exact decimal measures
    and IEEE bit patterns — fold into one `u64`, compared against `GOLDEN_DIGEST`.
    `Pass` is the corpus size; a digest that disagrees with the golden is the
    whole suite failing, because the digest is one value over the whole corpus and
    there is no per-geometry verdict to partially credit.

    The golden is read out of the test source rather than restated here, so
    there stays one copy in the tree. This complete corpus runs natively;
    release-crate WASM compilation is checked separately by `make wasm`.
    """
    name = "GeoSPARQL 1.1 determinism corpus"
    source = "purrdf-geo (first-party; OGC 22-047r1)"
    cmd = [
        "cargo", "run", "-p", "purrdf-geo", "--locked",
        "--example", "geo_digest",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    digest = re.search(r"^digest=([0-9a-f]{16})$", out, re.MULTILINE)
    corpus = re.search(r"^corpus_len=(\d+)$", out, re.MULTILINE)
    if not digest or not corpus:
        return _no_scoreboard(
            name, source, "`digest=<16 hex>` / `corpus_len=N`", cmd, out,
        )
    try:
        golden_text = _GEO_GOLDEN.read_text(encoding="utf-8")
    except OSError:
        golden_text = ""
    golden = re.search(
        r"const GOLDEN_DIGEST: u64 = 0x([0-9a-f_]+);", golden_text
    )
    if not golden:
        # The golden is the ORACLE. Without it this row would be the crate
        # agreeing with itself, which is not a conformance measurement.
        return _no_scoreboard(
            name, source,
            "`const GOLDEN_DIGEST: u64 = 0x…;` in crates/geo/tests/determinism.rs",
            cmd, out,
        )
    want = golden.group(1).replace("_", "")
    got = digest.group(1)
    total = int(corpus.group(1))
    agrees = got == want
    detail = (
        f"{total} geometries folded byte-wise into one u64 digest, compared with "
        f"the pinned `GOLDEN_DIGEST` in crates/geo/tests/determinism.rs "
        f"(`{want}`): {'agrees' if agrees else f'DISAGREES (computed {got})'}. "
        "Counts CORPUS GEOMETRIES, not test functions. NO OGC conformance suite "
        "is vendored and none is claimed — the crate's SHACL shapes are "
        "first-party `example.org` mirrors of the shipped OGC 22-047r1 validator, "
        "because PurRDF mints no vocabulary IRIs, so the evaluator-seam and "
        "shape cases grade the implementation against its own reading of the "
        "specification and are gated by `make check` rather than counted here. "
        "This complete corpus runs natively; `make wasm` separately builds the release crate."
    )
    return SuiteResult(
        name, source,
        passed=(total if agrees else 0), xskip=0, failed=(0 if agrees else total),
        detail=detail,
        ok=(rc == 0 and agrees),
        log=out,
    )


def _suite_gts_vectors() -> SuiteResult:
    """The frozen cross-language GTS vector corpus (`vectors/*.gts`).

    Its own row because nothing else in this matrix reads that corpus: the wire
    format is governed upstream in `gmeow-gts` and this repository never
    regenerates the vectors, so the only thing purrdf can measure is whether its
    production reader folds each `<id>.gts` into exactly the `<id>.expected.json`
    the corpus ships. That agreement used to be recorded only inside the
    harness's own divergence ledger, which meant the umbrella gate could not see
    the corpus at all and a shipped doc could go on calling it unqualifiedly
    byte-exact.

    `Pass` is the vectors that agree byte-for-byte and `XFail/Skip` is the
    harness's `KNOWN_DIVERGENCES` ledger — vectors whose committed expectation
    this reader knowingly contradicts, each pinned on both sides by a dedicated
    test and held to XPASS discipline (a listed vector that starts agreeing fails
    the harness). `Fail` can only be non-zero if the corpus changed size, which
    the harness asserts separately.
    """
    cmd = [
        "cargo", "test", "-p", "purrdf-rdf", "--locked",
        "--test", "gts_corpus_expected_fold", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, failed = _cargo_tally(out)
    name = "GTS transport (frozen vectors)"
    source = "gmeow-gts frozen corpus, vectors/"
    m = re.search(r"GTS-VECTORS: agreed (\d+) total (\d+) diverging (\d+)", out)
    if not m:
        return _no_scoreboard(
            name, source, "`GTS-VECTORS: agreed N total N diverging N`", cmd, out,
        )
    agreed, total, diverging = (int(m.group(i)) for i in (1, 2, 3))
    plural = "divergence" if diverging == 1 else "divergences"
    detail = (
        f"{agreed}/{total} frozen vectors fold byte-exactly into their committed "
        f"`.expected.json`; {diverging} ledgered {plural} from an upstream "
        "expectation this reader contradicts. The corpus is governed upstream and "
        "is never regenerated here"
    )
    return SuiteResult(
        name, source,
        passed=agreed, xskip=diverging, failed=(total - agreed - diverging),
        detail=detail,
        ok=(rc == 0 and failed == 0 and agreed + diverging == total),
        log=out,
    )


def _suite_entailment() -> SuiteResult:
    """W3C OWL 2 suite graded against the native `OWL-Direct` SHOIQ(D) tableau.

    This row is CONSISTENCY-shaped and says so: all 261 vendored cases are
    `otest:ConsistencyTest` / `otest:InconsistencyTest`, so it measures the
    DL/tableau lane's satisfiability verdicts. It does NOT measure the OWL 2 RL
    rule table; that lane has its own row (`_suite_entailment_rl`), graded
    against W3C's own entailment tests. Entailment used to fold silently into the
    SPARQL row, where a regression in it was invisible.

    The corpus is also a SUBSET of what W3C published — 261 of the 482
    consistency-shaped upstream cases — so the harness emits a second line,
    `OWL2-DL-EXCLUDED`, tallying what the other 221 would do. It is scraped into
    this row's note so the pass count is never read as the whole upstream
    material: most of the exclusions are cases the tableau decided when the
    exclusion was probed (a recorded measurement in census.tsv's dl_probe
    column, not a live run — the harness reads the column and cannot detect a
    regression among the excluded cases).
    """
    cmd = [
        "cargo", "test", "-p", "purrdf-sparql-conformance", "--locked",
        "--test", "owl2_conformance", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, cargo_failed = _cargo_tally(out)
    m = re.search(
        r"OWL2-ENTAILMENT: agreed (\d+) ledgered (\d+) unledgered (\d+) "
        r"stale (\d+) total (\d+)",
        out,
    )
    if m:
        agreed, ledgered, unledgered, stale, total = (int(m.group(i)) for i in range(1, 6))
        detail = f"{agreed}/{total} DL consistency verdicts · {ledgered} ledgered"
        excluded = re.search(
            r"OWL2-DL-EXCLUDED: total (\d+) non-terminating (\d+) decides (\d+) "
            r"withholds (\d+) no-premise (\d+)",
            out,
        )
        if excluded:
            ex_total, non_term, decides, withholds, no_premise = (
                int(excluded.group(i)) for i in range(1, 6)
            )
            detail = _augment(
                detail,
                f"corpus is a subset: {ex_total} more consistency-shaped cases "
                f"upstream are NOT vendored ({decides} the tableau decided when "
                f"probed, {non_term} non-terminating, {withholds} withheld, "
                f"{no_premise} with no RDF/XML premise)",
            )
        else:
            # The exclusion line is part of this harness's contract; losing it
            # would silently restore "N of 261" as an unqualified headline. It
            # is a missing scoreboard line like any other, so it goes down the
            # same path: FAIL 1 / RED, naming the marker and the command. The
            # arm used to keep the scraped `agreed`/`ledgered` counts and set
            # `ok=False`, which rendered `FAIL 0 / RED` — a row asserting both
            # that nothing failed and that the suite is red.
            return _no_scoreboard(
                "Entailment (OWL 2 DL consistency)", "W3C OWL 2 test suite",
                "`OWL2-DL-EXCLUDED: total N non-terminating N decides N "
                "withholds N no-premise N`",
                cmd,
                out,
            )
        return SuiteResult(
            "Entailment (OWL 2 DL consistency)", "W3C OWL 2 test suite",
            passed=agreed, xskip=ledgered, failed=unledgered + stale,
            detail=detail,
            ok=(rc == 0 and cargo_failed == 0 and unledgered == 0 and stale == 0),
            log=out,
        )
    return _no_scoreboard(
        "Entailment (OWL 2 DL consistency)", "W3C OWL 2 test suite",
        "`OWL2-ENTAILMENT: agreed N ledgered N unledgered N stale N total N`",
        cmd,
        out,
    )


def _suite_entailment_rl() -> SuiteResult:
    """W3C's own OWL 2 **entailment** tests, graded through the OWL 2 RL chase.

    The independent oracle for the RL rule table. Until it existed, the table was
    scored only by fixtures authored alongside the rules themselves, and the
    `OWL-RL 78 / 78` rule-table headline stood in for entailment conformance.
    They are different claims: this row measures the second one.

    `Pass` is the agreeing verdicts across both lanes (positive: the closure
    contains the published conclusion; negative: it does not), and `XFail/Skip`
    is the typed divergence ledger in
    `crates/sparql-conformance/src/owl2_rl.rs::LEDGER`. The scoreboard's
    `actionable` count — divergences naming a sound rule of RL's own shape, as
    opposed to a structural limit of the profile — is carried into the note so
    the ledger's size is never mistaken for a defect count.
    """
    cmd = [
        "cargo", "test", "-p", "purrdf-sparql-conformance", "--locked",
        "--test", "owl2_rl_conformance", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, cargo_failed = _cargo_tally(out)
    name = "Entailment (OWL 2 RL, W3C entailment tests)"
    source = "W3C OWL 2 entailment tests"
    m = re.search(
        r"OWL2-RL-ENTAILMENT: agreed (\d+) ledgered (\d+) unledgered (\d+) "
        r"stale (\d+) total (\d+) actionable (\d+)",
        out,
    )
    if not m:
        return _no_scoreboard(
            name,
            source,
            "`OWL2-RL-ENTAILMENT: agreed N ledgered N unledgered N stale N "
            "total N actionable N`",
            cmd,
            out,
        )
    agreed, ledgered, unledgered, stale, total, actionable = (
        int(m.group(i)) for i in range(1, 7)
    )
    detail = f"{agreed}/{total} agreeing · {ledgered} ledgered · {actionable} actionable"
    split = re.search(
        r"\[w3c-owl2-rl\] (\d+) positive \+ (\d+) negative entailment cases", out
    )
    if split:
        # The corpus composition only — NOT a per-lane agreement split, which
        # this line does not report and which is therefore not invented here.
        # The lane split is derived from the LEDGER and the census, and gated,
        # in scripts/check-doc-claims.py.
        detail = _augment(
            detail, f"corpus: {split.group(1)} positive + {split.group(2)} negative"
        )
    return SuiteResult(
        name, source,
        passed=agreed, xskip=ledgered, failed=unledgered + stale,
        detail=detail,
        ok=(rc == 0 and cargo_failed == 0 and unledgered == 0 and stale == 0),
        log=out,
    )


# The rows the two Python gates produce, which `--no-python` does not run.
PYTHON_SUITES = frozenset({"rdflib LSP drop-in gate", "Python binding suite"})


def _suite_py_rdflib_gate(build: bool) -> SuiteResult:
    """rdflib's OWN vendored tests run against the purrdf drop-in."""
    log = ""
    if build:
        rc, bout = _run(
            ["uv", "sync", "--locked", "--group", "dev"], _PY_DIR
        )
        log += bout
        if rc != 0:
            return SuiteResult(
                "rdflib LSP drop-in gate", "rdflib 7.6 own tests",
                failed=-1, detail="editable uv sync FAILED", ok=False, log=log,
            )
    rc, out = _run(
        ["uv", "run", "--locked", "python", "-m", "tests.rdflib_suite.runner"], _PY_DIR
    )
    log += out
    m = re.search(
        r"PURRDF_SCOREBOARD passed=(\d+) xfailed=(\d+) xpassed=(\d+) "
        r"failed=(\d+) errors=(\d+) ledger_total=(\d+) ledger_applied=(\d+) "
        r"ledger_stale=(\d+)",
        out,
    )
    if not m:
        return SuiteResult(
            "rdflib LSP drop-in gate", "rdflib 7.6 own tests",
            failed=-1, detail="no scoreboard emitted", ok=False, log=log,
        )
    passed, xfailed, xpassed, failed, errors, _lt, _la, stale = (
        int(m.group(i)) for i in range(1, 9)
    )
    detail = f"{passed} pass · {xfailed} strict-xfail (ledgered)"
    if xpassed:
        detail += f" · {xpassed} XPASS!"
    if stale:
        detail += f" · {stale} stale ledger keys!"
    return SuiteResult(
        "rdflib LSP drop-in gate", "rdflib 7.6 own tests",
        passed=passed, xskip=xfailed, failed=failed + errors + xpassed + stale,
        detail=detail, ok=(rc == 0), log=log,
    )


def _suite_py_compat(build: bool) -> SuiteResult:
    """The whole Python binding pytest suite, compat-parity differential included.

    Named for what it RUNS rather than for one of its parts: the command is
    `pytest tests`, so the count covers every binding test — entailment, GTS,
    projections, shapes — and not only the rdflib differential. A row labelled for
    the differential alone reports a number that is not the differential's.
    """
    log = ""
    if build:
        rc, bout = _run(
            ["uv", "sync", "--locked", "--group", "dev"], _PY_DIR
        )
        log += bout
        if rc != 0:
            return SuiteResult(
                "Python binding suite", "first-party (incl. compat differential vs rdflib)",
                failed=-1, detail="editable uv sync FAILED", ok=False, log=log,
            )
    rc, out = _run(
        ["uv", "run", "--locked", "--group", "dev", "pytest", "tests", "-q"], _PY_DIR
    )
    log += out
    passed = _int(re.search(r"(\d+) passed", out))
    xfailed = _int(re.search(r"(\d+) xfailed", out))
    failed = _int(re.search(r"(\d+) failed", out))
    xpassed = _int(re.search(r"(\d+) xpassed", out))
    errors = _int(re.search(r"(\d+) error", out))
    detail = f"{passed} pass · {xfailed} strict-xfail (ledgered)"
    return SuiteResult(
        "Python binding suite", "first-party (incl. compat differential vs rdflib)",
        passed=passed, xskip=xfailed, failed=failed + xpassed + errors,
        detail=detail, ok=(rc == 0), log=log,
    )


def _int(m: re.Match[str] | None) -> int:
    return int(m.group(1)) if m else 0


# ---------------------------------------------------------------------------
# Monotone-shrink ratchet
# ---------------------------------------------------------------------------


def load_budget() -> dict[str, int]:
    """Load the ratchet budget: suite name -> allowed ledgered-gap count."""
    data = json.loads(_BASELINE_PATH.read_text(encoding="utf-8"))
    return {name: entry["ledgered"] for name, entry in data["suites"].items()}


def _augment(detail: str, msg: str) -> str:
    return f"{detail} · {msg}" if detail else msg


def enforce_ratchet(
    results: list[SuiteResult], budget: dict[str, int], check_orphans: bool = True,
    not_run: frozenset[str] = frozenset(),
) -> None:
    """Gate each suite's ledgered count against its committed budget.

    The budget in ``conformance-baseline.json`` is authoritative and may only
    ever be edited DOWNWARD. The live ledgered count must EQUAL its budget:

      * a count ABOVE budget (a regressed or newly-ledgered gap) fails RED — fix
        the gap, do not raise the budget;
      * a count BELOW budget (a fixed gap) also fails RED until the budget is
        lowered here, which locks the gain in — this is the ratchet, by design;
      * a run suite with no budget entry fails RED;
      * a budget entry NO SUITE PRODUCES fails RED — the reverse direction. The
        loop below is over ``results``, so an orphan key is simply never read: it
        would sit in the baseline forever, silently guarding nothing, and a suite
        later renamed INTO that spelling would inherit a stale ceiling. Renaming a
        suite is exactly when this happens, which is why it is checked rather than
        trusted.

    Suites that could not emit a scoreboard keep their own failure and are not
    re-diagnosed here — a compile error or aborted harness (``failed < 0``), and
    a harness that ran but withheld its scoreboard line
    (``scoreboard_missing``). Both already fail RED for a reason the row states,
    and neither has a ledgered-gap count to gate: their ``xskip`` is zero
    because nothing was measured, not because a gap was fixed, so gating it
    would print "LEDGER SHRANK — lower the budget to lock the gain" over a
    broken harness and invite someone to ratchet a measurement away. Their
    names still count as produced, so a broken harness does not also read as an
    orphan key.
    """
    for r in results:
        r.budget = budget.get(r.name)
        if r.failed < 0 or r.scoreboard_missing:
            continue
        if r.budget is None:
            r.ok = False
            r.detail = _augment(
                r.detail,
                f'NO BUDGET: add "{r.name}" to scripts/conformance-baseline.json',
            )
        elif r.xskip > r.budget:
            r.ok = False
            r.detail = _augment(
                r.detail,
                f"LEDGER GREW: {r.xskip} > budget {r.budget} — a gap regressed; "
                "fix it, do not raise the budget",
            )
        elif r.xskip < r.budget:
            r.ok = False
            r.detail = _augment(
                r.detail,
                f"LEDGER SHRANK: {r.xskip} < budget {r.budget} — lower it in "
                "scripts/conformance-baseline.json to lock the gain",
            )

    # One shard of the matrix (`--shard`) produces only some of the budgeted
    # suites, so it cannot tell an orphan key from a suite another shard runs.
    # `--from-results` judges the combined rows with this check on.
    orphans = sorted(set(budget) - {r.name for r in results} - not_run) if check_orphans else []
    if orphans:
        raise SystemExit(
            "conformance-matrix: scripts/conformance-baseline.json budgets a suite "
            f"no run produced: {', '.join(orphans)}. A key nothing reads guards "
            "nothing — either the suite was renamed and the key was not, or the "
            "suite was removed and its budget outlived it. Fix the spelling or "
            "delete the entry; do not leave a ceiling with no suite under it."
        )


# ---------------------------------------------------------------------------
# Orchestration
# ---------------------------------------------------------------------------


def _native_registry() -> list[tuple[str, Callable[[], SuiteResult]]]:
    """Every native suite, in matrix order, with the shard that runs it.

    Nothing runs here: each entry is a thunk. The order is the order the matrix
    prints, and the shard is only where CI runs a suite (see `SHARDS`).
    """
    return [
        ("core", lambda: _suite_cargo(
            "IRI (RFC 3987 / RFC 3986 resolution)", "W3C IRI + RFC vectors",
            ["cargo", "test", "-p", "purrdf-iri", "--locked",
             "--test", "w3c_iri", "--test", "iri_suite", "--test", "resolution"],
            detail="parse/validate/normalize/resolve vectors",
        )),
        ("core", lambda: _suite_cargo(
            "RDFC-1.0 canonicalization", "W3C rdf-canon",
            ["cargo", "test", "-p", "purrdf-rdf", "--locked", "--test", "rdfc_w3c"],
            detail="65 vectors (64 eval + 1 negative), sharded",
        )),
        ("core", lambda: _suite_cargo(
            "RDF 1.2 canonicalization profile", "purrdf-rdfc12 v2 (first-party)",
            ["cargo", "test", "-p", "purrdf-rdf", "--locked",
             "--test", "rdf12_canon_profile"],
            detail="19 goldens + 7 refusals, frozen and content-addressed",
        )),
        ("core", _suite_codec),
        ("sparql", _suite_sparql),
        ("sparql", _suite_sparql10_unlisted),
        ("sparql", _suite_sparql10_superseded),
        ("sparql", _suite_construct_corpus),
        ("sparql", _suite_describe_corpus),
        ("sparql", _suite_cdt_corpus),
        ("sparql", _suite_governor_corpus),
        # The two lanes below are `_suite_cargo` rows for the same reason the
        # four above/below them are: neither grades a CORPUS. Each is a
        # first-party test lane over inline fixtures and pinned literals, so the
        # only per-case unit that exists is the test function, and the row says
        # so rather than inventing a fixture count. Both shipped without a matrix
        # row at all, which is strictly worse: a lane nothing reports is a lane
        # whose regression the umbrella gate cannot see.
        ("core", lambda: _suite_cargo(
            "SPARQL embedding kNN (first-party)",
            "purrdf-embedding-knn (first-party)",
            ["cargo", "test", "-p", "purrdf-sparql-eval", "--locked",
             "--test", "embedding_knn_e2e", "--test", "knn_wasm_determinism"],
            detail=(
                "the PURREMB kNN property-function seam end to end — rank order, "
                "join-back, the LIMIT prefix law, cross-artifact byte identity, and "
                "the `property-function-work` governor charge point the governor "
                "corpus deliberately does not band (its relations report zero work). "
                "The `knn_wasm_determinism` case pins five neighbours and their exact "
                "`xsd:double` distance lexicals as a literal, so a reassociated sum or "
                "a fused multiply-add that swaps two near-tied neighbours fails here; "
                "these numeric expectations run natively; `make wasm-test` separately "
                "selects actual WASM dispatch and SIMD kernels"
            ),
        )),
        ("core", lambda: _suite_cargo(
            "HNSW approximate kNN (first-party)",
            "purrdf-hnsw (first-party)",
            ["cargo", "test", "-p", "purrdf-hnsw", "--locked",
             "--test", "conformance", "--test", "invariants",
             "--test", "oracle_contract", "--test", "sparql_e2e",
             "--test", "purremb_roundtrip", "--test", "adversarial_payload",
             "--test", "vector_query", "--test", "determinism",
             "--test", "corpus_geometry"],
            detail=(
                "the approximate half of the retrieval pair, graded against the exact "
                "path as its oracle: every offered row compared to the exact scan's "
                "own `(distance, row)` order, the graph invariants that make an offer "
                "possible at all (every row reachable from the entry point, degree "
                "bounds, sorted adjacency, the fixed level formula), the approximation "
                "contract (an offer is never a proof of absence), the PURREMB guard "
                "round-trip through the real INDEX_GUARDS and INDEX_PAYLOAD sections, "
                "the SPARQL property-function seam from query text, and the "
                "decoder-hostility cases. It also grades the CORPUS the recall figures "
                "are taken over, because on this crate the distribution decides the "
                "recall, so a generator that drifted toward uniform would quietly turn "
                "every recall number into a statement about the fixture: effective "
                "dimension against a uniform control, and the achieved within-cluster "
                "cosine -- measured against the centroid each row was drawn around, "
                "pinned across a ladder of intended values, pinned again for the default "
                "shape every figure in this crate uses, and held flat across a sixty-fourfold "
                "range of widths, which is the property that separates an intended cosine "
                "from a fixed noise amplitude whose tightness collapses as the width "
                "grows. This complete corpus runs natively; focused WASM tests "
                "exercise actual dispatch, SIMD kernels and path admission"
            ),
        )),
        ("core", _suite_geo_determinism),
        ("sparql", _suite_entailment),
        ("sparql", _suite_entailment_rl),
        ("shapes", _suite_shacl_w3c),
        ("shapes", _suite_shacl12_w3c),
        ("shapes", _suite_shacl12_unlisted),
        ("shapes", _suite_shapes_corpus),
        ("shapes", _suite_community_shacl),
        ("shapes", _suite_product_equivalence),
        ("core", _suite_xsd_regex_corpus),
        ("shapes", _suite_shacl_rules),
        ("shapes", _suite_shex_validation),
        ("shapes", lambda: _suite_cargo(
            "ShEx syntax + ShExC/ShExJ round-trip", "shexTest v2.1.0",
            ["cargo", "test", "-p", "purrdf-shex", "--locked",
             "--test", "syntax_conformance", "--test", "shexc_roundtrip",
             "--test", "shexj_roundtrip"],
            detail="schemas parse + negative syntax/structure",
        )),
        # A `_suite_cargo` row that IS a per-case measurement: the suite
        # harness is `harness = false` with one libtest case per suite test, so
        # the tally counts suite cases, and its `suite-inventory` case pins the
        # file, group, case and remote counts, so the corpus cannot shrink
        # under this number without the row going RED.
        ("core", lambda: _suite_cargo(
            "JSON Schema draft 2020-12 (official suite)",
            "JSON-Schema-Test-Suite 5b0ee16",
            ["cargo", "test", "-p", "purrdf-jsonschema", "--locked", "--test", "suite"],
            detail=(
                "every draft 2020-12 test file, optional/ included and optional/format/ "
                "run with format assertion on, one case per suite test: all 1,463 "
                "validation cases, all 874 format cases and all 4 output-format cases "
                "pass, plus the "
                "suite-inventory case. optional/cross-draft.json evaluates a $ref into a "
                "draft 2019-09 document under 2019-09 rules"
            ),
        )),
        ("core", lambda: _suite_cargo(
            "JSON Schema draft 2019-09 (official suite)",
            "JSON-Schema-Test-Suite 5b0ee16",
            ["cargo", "test", "-p", "purrdf-jsonschema", "--locked", "--test",
             "suite_draft2019_09"],
            detail=(
                "every draft 2019-09 test file, optional/ included and optional/format/ "
                "run with format assertion on, one case per suite test: all 1,419 "
                "validation cases, all 874 format cases and all 4 output-format cases "
                "pass, plus the "
                "suite-inventory case. $recursiveRef, array-form items, and "
                "optional/cross-draft.json's $refs into 2020-12 and draft-07 documents, "
                "each evaluated under its own draft"
            ),
        )),
        ("core", lambda: _suite_cargo(
            "JSON Schema draft-07 (official suite)",
            "JSON-Schema-Test-Suite 5b0ee16",
            ["cargo", "test", "-p", "purrdf-jsonschema", "--locked", "--test",
             "suite_draft7"],
            detail=(
                "every draft-07 test file, optional/ included and optional/format/ run "
                "with format assertion on, one case per suite test: all 1,047 validation "
                "cases and all 793 format cases pass, plus the suite-inventory case; "
                "draft-07 has no "
                "output-format tests"
            ),
        )),
        ("core", _suite_gts_vectors),
    ]


def native_suites() -> list[SuiteResult]:
    return [run() for _shard, run in _native_registry()]


# ---------------------------------------------------------------------------
# Shards: the same matrix on several runners, judged once
# ---------------------------------------------------------------------------

# Where CI runs each suite. A shard is a runner, not a different gate: every
# suite belongs to exactly one shard, `--shard` runs that shard's suites and
# writes their rows UNJUDGED, and `--from-results` reassembles the rows in
# matrix order and judges them exactly as a single full run does (the ratchet
# with its orphan check, the scoreboard, the job summary, the document drift
# check). Suites that compile the same crates share a shard, so no runner builds
# a graph another one already paid for.
SHARDS = ("core", "sparql", "shapes", "python")
_RESULTS_FORMAT = "purrdf-conformance-shard/1"


def full_registry(build: bool) -> list[tuple[str, Callable[[], SuiteResult]]]:
    """The whole matrix -- the native suites, then the two Python gates -- in print order.

    The Python pair shares one shard: the compat suite reuses the module the
    rdflib gate builds, exactly as in a single run.
    """
    return [
        *_native_registry(),
        ("python", lambda: _suite_py_rdflib_gate(build)),
        ("python", lambda: _suite_py_compat(build=False)),
    ]


def registry_shards() -> list[str]:
    """The shard of every matrix row, by row index. Builds nothing, runs nothing."""
    return [shard for shard, _run in full_registry(build=False)]


def shard_payload(shard: str, indexed: list[tuple[int, SuiteResult]]) -> str:
    """One shard's rows as measured -- before the ratchet -- for `--from-results`.

    The ratchet runs once, over the whole matrix, in the judging run: its orphan
    half can only be decided with every shard in hand, and applying the rest
    twice would annotate a red row twice.
    """
    rows = [{"index": index, **asdict(result)} for index, result in indexed]
    return json.dumps(
        {"format": _RESULTS_FORMAT, "shard": shard, "rows": rows},
        indent=1, ensure_ascii=False,
    ) + "\n"


def merge_shards(files: dict[str, str], shards_by_index: list[str]) -> list[SuiteResult]:
    """Every shard's rows in matrix order; anything but the whole matrix once is refused.

    `files` maps a file name to its text. Exactly one `<shard>.json` per name in
    `SHARDS` must be present, each recording its own shard, and each must hold
    exactly the row indices that shard owns in `shards_by_index`. Because the
    shards partition the indices, that makes the combined rows complete and
    disjoint by construction: a missing shard, a stray file, a shard that ran
    another's suite or skipped one of its own, and a row missing a field are all
    refused by name rather than judged as a smaller matrix.
    """
    def refuse(message: str) -> SystemExit:
        return SystemExit(f"conformance-matrix: --from-results: {message}")

    expected = {f"{shard}.json" for shard in SHARDS}
    missing = sorted(expected - set(files))
    extra = sorted(set(files) - expected)
    if missing or extra:
        raise refuse(
            f"needs exactly {sorted(expected)}; missing {missing}, unexpected {extra}. "
            "A shard with no results is a slice of the matrix nobody measured."
        )
    field_names = {f.name for f in fields(SuiteResult)}
    by_index: dict[int, SuiteResult] = {}
    for shard in SHARDS:
        name = f"{shard}.json"
        try:
            data = json.loads(files[name])
        except json.JSONDecodeError as err:
            raise refuse(f"{name} is not JSON: {err}") from None
        if not isinstance(data, dict) or data.get("format") != _RESULTS_FORMAT:
            raise refuse(f"{name} is not a `{_RESULTS_FORMAT}` results file")
        if data.get("shard") != shard:
            raise refuse(f"{name} records shard {data.get('shard')!r}")
        rows = data.get("rows")
        if not isinstance(rows, list) or not all(isinstance(row, dict) for row in rows):
            raise refuse(f"{name} has no list of rows")
        owned = [index for index, owner in enumerate(shards_by_index) if owner == shard]
        reported = [row.get("index") for row in rows]
        if reported != owned:
            raise refuse(
                f"{name} reports rows {reported}, but shard `{shard}` runs rows {owned}"
            )
        for row in rows:
            values = {key: value for key, value in row.items() if key != "index"}
            if set(values) != field_names:
                raise refuse(
                    f"{name} row {row['index']} has fields {sorted(values)}, "
                    f"a matrix row has {sorted(field_names)}"
                )
            by_index[row["index"]] = SuiteResult(**values)
    return [by_index[index] for index in range(len(shards_by_index))]


def read_results_dir(directory: Path) -> dict[str, str]:
    """Every entry of `directory` by name (a non-file entry reads as unexpected)."""
    if not directory.is_dir():
        raise SystemExit(f"conformance-matrix: --from-results: {directory} is not a directory")
    return {
        entry.name: entry.read_text(encoding="utf-8") if entry.is_file() else ""
        for entry in sorted(directory.iterdir())
    }


def _shard_self_test() -> list[str]:
    """The shard split and its reassembly, each refusal beside its valid neighbour."""
    problems: list[str] = []
    shards_by_index = registry_shards()
    if sorted(set(shards_by_index)) != sorted(SHARDS):
        problems.append(
            f"  • shards: the registry uses {sorted(set(shards_by_index))}, SHARDS names "
            f"{sorted(SHARDS)}; a shard no suite uses, or a suite no shard runs, is a gap"
        )
    if shards_by_index[-2:] != ["python", "python"] or shards_by_index.count("python") != 2:
        problems.append("  • shards: the two Python gates must be the only `python` rows, last")

    def row(index: int) -> SuiteResult:
        return SuiteResult(
            name=f"suite {index}", source="specimen", passed=index, xskip=index % 3,
            failed=index % 2, detail=f"d{index}", ok=index % 2 == 0, log=f"log {index}",
        )

    def payloads() -> dict[str, str]:
        return {
            f"{shard}.json": shard_payload(
                shard, [(i, row(i)) for i, owner in enumerate(shards_by_index) if owner == shard]
            )
            for shard in SHARDS
        }

    try:
        merged = merge_shards(payloads(), shards_by_index)
        if merged != [row(i) for i in range(len(shards_by_index))]:
            problems.append("  • shards: the reassembled rows differ from the rows each shard wrote")
    except SystemExit as err:
        problems.append(f"  • shards: a complete set of shard results is refused: {err}")

    def refused(files: dict[str, str], needle: str, what: str) -> None:
        try:
            merge_shards(files, shards_by_index)
        except SystemExit as err:
            if needle not in str(err):
                problems.append(f"  • shards: {what} is refused for the wrong reason: {err}")
            return
        problems.append(f"  • shards: {what} is accepted")

    first, second = SHARDS[0], SHARDS[1]
    complete = payloads()
    refused({k: v for k, v in complete.items() if k != f"{first}.json"}, "missing", "a missing shard")
    refused({**complete, "stray.json": "{}"}, "unexpected", "a stray results file")
    refused({**complete, f"{first}.json": complete[f"{second}.json"]}, "records shard", "a shard's file under another name")
    refused({**complete, f"{first}.json": "not json"}, "is not JSON", "a malformed file")
    refused({**complete, f"{first}.json": json.dumps({"shard": first, "rows": []})}, "results file", "a file of another format")
    owned = [i for i, owner in enumerate(shards_by_index) if owner == first]
    other = next(i for i, owner in enumerate(shards_by_index) if owner != first)
    refused({**complete, f"{first}.json": shard_payload(first, [(i, row(i)) for i in owned[:-1]])}, "reports rows", "a shard that skipped one of its suites")
    refused({**complete, f"{first}.json": shard_payload(first, [(i, row(i)) for i in [*owned, other]])}, "reports rows", "a shard that ran another shard's suite")
    stripped = json.loads(complete[f"{first}.json"])
    del stripped["rows"][0]["failed"]
    refused({**complete, f"{first}.json": json.dumps(stripped)}, "has fields", "a row without its fail count")
    return problems


def render(results: list[SuiteResult]) -> str:
    name_w = max(len(r.name) for r in results)
    src_w = max(len(r.source) for r in results)
    header = (
        f"  {'SUITE':<{name_w}}  {'SOURCE':<{src_w}}  "
        f"{'PASS':>6}  {'XF/SKIP':>7}  {'BUDGET':>6}  {'FAIL':>5}  STATUS"
    )
    lines = ["", "PurRDF conformance matrix", "=" * len(header), header, "-" * len(header)]
    for r in results:
        fail_cell = "err" if r.failed < 0 else str(r.failed)
        budget_cell = "-" if r.budget is None else str(r.budget)
        lines.append(
            f"  {r.name:<{name_w}}  {r.source:<{src_w}}  "
            f"{r.passed:>6}  {r.xskip:>7}  {budget_cell:>6}  {fail_cell:>5}  {r.status}"
        )
    tot_pass = sum(r.passed for r in results)
    tot_xskip = sum(r.xskip for r in results)
    tot_budget = sum(r.budget or 0 for r in results)
    tot_fail = sum(max(r.failed, 0) for r in results)
    lines.append("-" * len(header))
    lines.append(
        f"  {'TOTAL':<{name_w}}  {'':<{src_w}}  "
        f"{tot_pass:>6}  {tot_xskip:>7}  {tot_budget:>6}  {tot_fail:>5}"
    )
    lines.append("")
    notes = [r for r in results if r.detail]
    if notes:
        lines.append("Notes:")
        for r in notes:
            lines.append(f"  - {r.name}: {r.detail}")
        lines.append("")
    green = all(r.ok for r in results)
    verdict = "GREEN — all conformance suites pass or are ledgered" if green else "RED"
    lines.append(f"VERDICT: {verdict}")
    if not green:
        for r in results:
            if not r.ok:
                lines.append(f"  RED: {r.name} — see log above")
    lines.append("")
    return "\n".join(lines)


def render_matrix_table(results: list[SuiteResult]) -> str:
    """The Markdown matrix table only (no title, no verdict) — the canonical
    block embedded in both the CI job summary and docs/CONFORMANCE.md."""
    rows = [
        "| Suite | Source | Pass | XFail/Skip | Budget | Fail | Status |",
        "| --- | --- | ---: | ---: | ---: | ---: | :---: |",
    ]
    for r in results:
        fail_cell = "err" if r.failed < 0 else str(r.failed)
        budget_cell = "—" if r.budget is None else str(r.budget)
        badge = "GREEN" if r.ok else "RED"
        rows.append(
            f"| {r.name} | {r.source} | {r.passed} | {r.xskip} | "
            f"{budget_cell} | {fail_cell} | {badge} |"
        )
    return "\n".join(rows)


def render_markdown(results: list[SuiteResult]) -> str:
    green = all(r.ok for r in results)
    return "\n".join(
        [
            "## PurRDF conformance matrix",
            "",
            render_matrix_table(results),
            "",
            f"**Verdict: {'GREEN' if green else 'RED'}**",
            "",
        ]
    )


# ---------------------------------------------------------------------------
# Generated doc block (drift guard over docs/CONFORMANCE.md's matrix table)
# ---------------------------------------------------------------------------


def _split_doc(text: str) -> tuple[str, str, str]:
    """Return (head-through-BEGIN, current inner, END-through-tail)."""
    if _DOC_BEGIN not in text or _DOC_END not in text:
        raise SystemExit(
            f"conformance-matrix: markers not found in {_DOC_PATH.relative_to(_REPO_ROOT)} "
            f"({_DOC_BEGIN} / {_DOC_END})"
        )
    i = text.index(_DOC_BEGIN) + len(_DOC_BEGIN)
    j = text.index(_DOC_END)
    return text[:i], text[i:j], text[j:]


def write_doc_block(block: str) -> None:
    head, _, tail = _split_doc(_DOC_PATH.read_text(encoding="utf-8"))
    _DOC_PATH.write_text(f"{head}\n{block}\n{tail}", encoding="utf-8")


def _normalize(text: str) -> str:
    """Strip surrounding whitespace and fold CRLF to LF so a Windows/autocrlf
    checkout does not read as drift against the LF-rendered block."""
    return text.replace("\r\n", "\n").strip()


def check_doc_block(block: str) -> bool:
    """True iff the committed matrix block equals the freshly measured one."""
    _, inner, _ = _split_doc(_DOC_PATH.read_text(encoding="utf-8"))
    inner, block = _normalize(inner), _normalize(block)
    if inner == block:
        return True
    print(
        f"\n{_DOC_PATH.relative_to(_REPO_ROOT)} conformance-matrix block is stale; "
        "regenerate with `python3 scripts/conformance-matrix.py --write-doc`.",
        file=sys.stderr,
    )
    diff = difflib.unified_diff(
        inner.splitlines(),
        block.splitlines(),
        fromfile="committed",
        tofile="measured",
        lineterm="",
    )
    print("\n".join(diff), file=sys.stderr)
    return False


# ---------------------------------------------------------------------------
# Self-test: every scraped row must go RED without its scoreboard line
# ---------------------------------------------------------------------------
#
# The failure this proves against is not hypothetical. Every scraped suite used
# to fall back to `_suite_cargo` when its regex missed, so a harness that
# stopped printing its per-case scoreboard kept a plausible small number in the
# Pass column and a GREEN badge, and the corpus behind it stopped being measured
# with nothing to say so. `_no_scoreboard` closes that; the specimens below are
# what keep it closed, because a fail-closed claim nobody exercises decays into
# a fail-open one the first time a regex is edited.
#
# The numbers in the specimens are DELIBERATELY small and synthetic. They are
# not measurements and must never be read as any: only the SHAPE of each line is
# under test, and a specimen wearing real-looking totals is a specimen someone
# eventually quotes. The shapes are copied from the harnesses that print them.

_CARGO_OK = "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"


def _board(line: str) -> tuple[str, bool]:
    """A scoreboard line: withheld one at a time, each withholding must go RED."""
    return (line, True)


def _noise(line: str) -> tuple[str, bool]:
    """Surrounding harness chatter, kept so each scrape is exercised over a log
    of the shape it really reads and not over a bare isolated marker."""
    return (line, False)


# (row name, scraper, specimen lines). One entry per SCRAPED suite; the six
# `_suite_cargo` rows in `native_suites` scrape nothing and have no scoreboard
# line to withhold, and the two Python rows already fail closed on a missing
# scoreboard by construction.
_SPECIMENS: tuple[tuple[str, Callable[[], SuiteResult], tuple[tuple[str, bool], ...]], ...] = (
    (
        "Syntax codecs (Turtle/TriG/NT/NQ/RDF-XML)",
        _suite_codec,
        (
            _noise("=== W3C RDF 1.2 native-codec round-trip conformance ==="),
            _noise("vendored corpus: crates/rdf/tests/corpus/w3c"),
            _noise("     turtle: total   5  passed   4  allowlisted-gap  1"),
            _board("      TOTAL: total   9  passed   7  allowlisted-gap  2"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        _SPARQL10_SUPERSEDED_NAME,
        _suite_sparql10_superseded,
        (_board("W3C10 SUPERSEDED: graded 1, sparql12-order 1"), _noise(_CARGO_OK)),
    ),
    (
        _SPARQL10_UNLISTED_NAME,
        _suite_sparql10_unlisted,
        (
            _noise("running 1 test"),
            _board(
                "W3C10 UNLISTED: files 20, orphan-manifest-files 8, "
                "unreferenced-payloads 11, unlisted-entry-results 1"
            ),
            _noise(_CARGO_OK),
        ),
    ),
    (
        _SPARQL_NAME,
        _suite_sparql,
        (
            _noise("running 1 test"),
            # One manifest tally, because the property under test is "no tally
            # at all is RED". A manifest that drops out entirely is caught by
            # the conformance harness itself — its case fails and cargo goes
            # non-zero — not by counting lines here, which would need this
            # script to hold a second copy of the manifest list.
            _board(
                "[w3c-sparql11/aggregates] 12 passed, 1 xfail, "
                "0 unexpected-pass, 0 failed, 0 unmodeled"
            ),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "SPARQL CONSTRUCT (first-party corpus)",
        _suite_construct_corpus,
        (
            _noise("running 1 test"),
            _board("CONSTRUCT-CORPUS: passed 29 total 29"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "SPARQL DESCRIBE (first-party corpus)",
        _suite_describe_corpus,
        (
            _noise("running 1 test"),
            _board("DESCRIBE-CORPUS: passed 7 total 7"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "SPARQL CDT (SEP-0009, vendored corpus)",
        _suite_cdt_corpus,
        (
            _noise("running 2 tests"),
            _board("SPARQL-CDT-CORPUS: passed 9 xfail 1 total 10"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "SPARQL execution governors",
        _suite_governor_corpus,
        (
            _noise("running 1 test"),
            _board("GOVERNOR-CORPUS: passed 12 total 12 bands 4"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "Entailment (OWL 2 DL consistency)",
        _suite_entailment,
        (
            _noise("running 1 test"),
            _board("OWL2-ENTAILMENT: agreed 9 ledgered 2 unledgered 0 stale 0 total 11"),
            _board(
                "OWL2-DL-EXCLUDED: total 8 non-terminating 1 decides 5 "
                "withholds 1 no-premise 1"
            ),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "Entailment (OWL 2 RL, W3C entailment tests)",
        _suite_entailment_rl,
        (
            _noise(
                "[w3c-owl2-rl] 6 positive + 5 negative entailment cases, graded "
                "through the OWL 2 RL chase"
            ),
            _board(
                "OWL2-RL-ENTAILMENT: agreed 9 ledgered 2 unledgered 0 stale 0 "
                "total 11 actionable 1"
            ),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "SHACL Core + SHACL-SPARQL",
        _suite_shacl_w3c,
        (
            _noise("W3C SHACL conformance scoreboard (9 tests):"),
            _noise("  core/node                     passed   4  xfailed   1"),
            _board("  TOTAL: passed 6, xfailed 2, ledger 2"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        _SHACL12_NAME,
        _suite_shacl12_w3c,
        (
            _noise(
                "W3C SHACL 1.2 conformance scoreboard (9 approved tests; 1 entries of "
                "unlisted vendored files reported apart):"
            ),
            _noise(
                "  core/node                            passed   4  "
                "non-canonical-expected-decimal  1  xfailed   1"
            ),
            _board(
                "  W3C12 TOTAL: passed 5, non-canonical-expected-decimal 1, xfailed 2, "
                "ledger 2"
            ),
            _noise(_CARGO_OK),
        ),
    ),
    (
        _SHACL12_UNLISTED_NAME,
        _suite_shacl12_unlisted,
        (
            _noise("W3C SHACL 1.2 entries of unlisted vendored files:"),
            _noise("  core/node/example-001                        sht:Validate"),
            _board("  W3C12 UNLISTED: passed 2, exact 1, with-delta 1, total 2"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "SHACL (first-party corpus)",
        _suite_shapes_corpus,
        (
            _noise("first-party SHACL corpus:"),
            _board("SHAPES-CORPUS: passed 9 total 9"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "Community SHACL dated profiles (REC 2017 / WD 2026)",
        _suite_community_shacl,
        (
            _noise("COMMUNITY SHACL TOTAL: cases 64 executions 115 passed 115 failed 0"),
            _board("COMMUNITY-SHACL: passed 115 total 115"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "SHACL prepared-product equivalence",
        _suite_product_equivalence,
        (
            _board(
                "PRODUCT-EQUIVALENCE: passed 9 ledgered 1 unparsable 2 disagreed 0 total 12"
            ),
            _noise("  evidence: 9 agreed on a report, of which 7 carried at least one "
                   "validation result"),
            _noise("  refusal dimensions: none — every loadable shapes graph packs"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "XSD/XPath regExp (first-party corpus)",
        _suite_xsd_regex_corpus,
        (
            _noise("first-party XSD/XPath regExp corpus:"),
            _board("XSD-REGEX-CORPUS: passed 9 total 9"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "SHACL Rules",
        _suite_shacl_rules,
        (
            _noise("SHACL rules corpus:"),
            _board("RULES: passed 6 total 6"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "ShEx 2.1 validation",
        _suite_shex_validation,
        (
            _noise("shexTest validation scoreboard:"),
            _board(
                "  entries 40 | attempted 32 | pass 32 | xfail 0 | fail 0 | skipped 8"
            ),
            _noise("  skip[Greedy] = 8"),
            _noise("  trait[Cardinality] = 6/6"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        "GTS transport (frozen vectors)",
        _suite_gts_vectors,
        (
            _noise("running 1 test"),
            _board("GTS-VECTORS: agreed 8 total 9 diverging 1"),
            _noise(_CARGO_OK),
        ),
    ),
    (
        # Two scoreboard lines rather than one, because this row needs BOTH: a
        # digest with no corpus size is a number over an unknown amount of
        # material, and a corpus size with no digest is a count of geometries
        # nothing graded. Withholding either must go RED, which is exactly what
        # the self-test drives.
        #
        # The digest here is the REAL `GOLDEN_DIGEST`, and it is the one specimen
        # value in this file that is not synthetic: the scraper compares what the
        # example printed against what the test source pins, so a made-up digest
        # would make the specimen row disagree and test the failure path while
        # claiming to test the success one.
        "GeoSPARQL 1.1 determinism corpus",
        _suite_geo_determinism,
        (
            _noise("    Finished `dev` profile [unoptimized + debuginfo] target(s)"),
            _noise("     Running `target/debug/examples/geo_digest`"),
            _board("digest=9667c2ee2cd3ad4b"),
            _board("corpus_len=20"),
        ),
    ),
)

_SCOREBOARD_LINES = sum(
    1 for _, _, lines in _SPECIMENS for _, is_board in lines if is_board
)


def _specimen(lines: tuple[tuple[str, bool], ...], withhold: int | None = None) -> str:
    """The specimen as harness output, optionally without line *withhold*."""
    return "".join(
        f"{text}\n" for i, (text, _) in enumerate(lines) if i != withhold
    )


def self_test(report: bool) -> list[str]:
    """Every scraped row that survives losing its scoreboard line, plus every
    specimen that no longer looks like its harness. An empty list is the only
    passing answer.

    Two properties, and the second is what stops the first from rotting:

      * WITHHOLDING — dropping one scoreboard line from an otherwise green
        specimen must leave the row RED with at least one counted failure. This
        is the fail-closed claim itself.
      * RECOGNITION — the UNMUTATED specimen must come back GREEN. Every scraper
        returns a RED `_no_scoreboard` row when its regex misses, so a green
        unmutated row proves the specimen was actually parsed. Without this
        check a specimen left behind by a harness that changed its wording would
        miss the regex in every arm, each withholding would still go RED, and
        the whole suite would pass while testing nothing at all.
    """
    problems: list[str] = []
    for name, scraper, lines in _SPECIMENS:
        boards = [i for i, (_, is_board) in enumerate(lines) if is_board]
        if not boards:
            raise SystemExit(
                f"conformance-matrix: the specimen for {name!r} marks no scoreboard "
                "line, so it withholds nothing and proves nothing. Mark the line the "
                "scraper reads with `_board(...)`."
            )

        with _stubbed_run(_specimen(lines)):
            whole = scraper()
        if whole.name != name:
            raise SystemExit(
                f"conformance-matrix: the self-test lists {name!r}, but that scraper "
                f"now produces the row {whole.name!r}. The suite was renamed and this "
                "table was not — fix the spelling rather than leaving a row whose "
                "fail-closed behaviour nothing here reports on."
            )
        if report:
            print(f"  {'ok' if whole.ok else 'STALE':9}  {name}: whole specimen recognised")
        if not whole.ok:
            problems.append(
                f"  • {name}: the UNMUTATED specimen is not recognised — the row "
                f"comes back {whole.status} ({whole.detail or 'no detail'}). The "
                "specimen no longer looks like what the harness prints, so every "
                "withholding below would go RED for the wrong reason and this "
                "suite would test nothing. Re-point the specimen at the harness."
            )
            continue

        for i in boards:
            with _stubbed_run(_specimen(lines, withhold=i)):
                row = scraper()
            caught = (not row.ok) and row.failed >= 1
            if report:
                print(
                    f"  {'caught' if caught else 'SURVIVED':9}  {name}: "
                    f"without {lines[i][0].strip()!r}"
                )
            if not caught:
                problems.append(
                    f"  • {name}: withholding {lines[i][0].strip()!r} leaves the row "
                    f"{row.status} with fail {row.failed} and pass {row.passed} — the "
                    "per-case scoreboard stopped being measured and this matrix still "
                    "reports a number for it"
                )
    shard_problems = _shard_self_test()
    if report:
        print(
            f"  {'ok' if not shard_problems else 'BROKEN':9}  shards: every suite in one of "
            f"{len(SHARDS)} shards; a missing, stray, mislabelled, short, overreaching or "
            "field-less shard result is refused and the complete set reassembles"
        )
    return problems + shard_problems


def main() -> int:
    parser = argparse.ArgumentParser(description="PurRDF conformance matrix")
    parser.add_argument(
        "--no-python",
        action="store_true",
        help="run only the native Rust conformance suites (skip the rdflib gate)",
    )
    parser.add_argument(
        "--no-build",
        action="store_true",
        help="skip editable `uv sync` before the Python suites (assume prebuilt)",
    )
    parser.add_argument(
        "--write-doc",
        action="store_true",
        help="rewrite the generated matrix block in docs/CONFORMANCE.md from the "
        "measured results (instead of drift-checking it)",
    )
    parser.add_argument(
        "--self-test",
        action="store_true",
        help="run only the fail-closed proof (no harness, no build): every scraped "
        "row must go RED when its scoreboard line is withheld",
    )
    parser.add_argument(
        "--shard",
        choices=SHARDS,
        help="run only this shard's suites and write their rows to --emit-results "
        "(a slice of the matrix for a later --from-results; judges nothing)",
    )
    parser.add_argument(
        "--emit-results",
        type=Path,
        metavar="FILE",
        help="with --shard: the file the shard's measured rows are written to",
    )
    parser.add_argument(
        "--from-results",
        type=Path,
        metavar="DIR",
        help="run no suite: judge the whole matrix from DIR/<shard>.json for every "
        "shard, exactly as a full run judges it (ratchet, scoreboard, document check)",
    )
    args = parser.parse_args()

    if (args.shard is None) != (args.emit_results is None):
        parser.error("--shard and --emit-results go together")
    if args.shard and (args.no_python or args.write_doc):
        # A shard is a slice: `--no-python` would redefine which slice, and the
        # document block needs the whole matrix, which only --from-results has.
        parser.error("--shard runs one slice; it takes neither --no-python nor --write-doc")
    if args.from_results and (args.shard or args.no_python or args.no_build):
        parser.error("--from-results runs no suite; it takes no run options")
    if args.write_doc and args.no_python:
        # The committed doc block reflects the full matrix (every native Rust
        # suite PLUS the two Python gates); a native-only run cannot reproduce it,
        # and writing it from one would silently delete the Python rows.
        parser.error("--write-doc requires the full suite (do not pass --no-python)")

    if args.self_test:
        print(
            f"conformance-matrix: withholding each of the {_SCOREBOARD_LINES} "
            f"scoreboard lines across {len(_SPECIMENS)} scraped suites, every one of "
            "which must turn its row RED —"
        )
    # BEFORE any harness starts, on every invocation: these rows report corpus
    # tallies nothing else measures, and for a whole branch each of them fell
    # back to a `cargo test` count and stayed GREEN when its scoreboard line went
    # missing. Pure text over strings through `_RUN_STUB`, so it costs no build
    # and no cargo — a rounding error against the matrix it precedes.
    problems = self_test(report=args.self_test)
    if problems:
        print(
            "conformance-matrix: this matrix reports corpus tallies it did not "
            "measure:\n" + "\n".join(problems)
            + "\n\nEach line above is a row that stays GREEN, or a specimen that "
            "checks nothing, while the corpus behind it goes unmeasured. Fix the "
            "scraper, not the specimen.",
            file=sys.stderr,
        )
        return 1
    if args.self_test:
        print(
            f"OK: all {len(_SPECIMENS)} scraped suites recognise their specimen, and "
            f"withholding any of the {_SCOREBOARD_LINES} scoreboard lines turns the "
            "row RED with a counted failure."
        )
        return 0

    # Build the native module once (in the rdflib gate); the compat suite then
    # reuses that editable install.
    registry = full_registry(build=not args.no_build)
    if args.shard:
        indexed = [
            (index, run()) for index, (shard, run) in enumerate(registry) if shard == args.shard
        ]
        args.emit_results.parent.mkdir(parents=True, exist_ok=True)
        args.emit_results.write_text(shard_payload(args.shard, indexed), encoding="utf-8")
        results = [result for _index, result in indexed]
        # Judged here only to name a red row in this runner's log; the verdict
        # that gates is the --from-results run over every shard.
        enforce_ratchet(results, load_budget(), check_orphans=False)
        print(f"conformance-matrix: shard `{args.shard}` ({len(results)} of {len(registry)} suites)")
        print(render(results))
        for r in results:
            if not r.ok:
                print(f"\n----- captured log: {r.name} -----", file=sys.stderr)
                print(r.log, file=sys.stderr)
        print(
            f"wrote the shard's rows to {args.emit_results}; `--from-results` judges "
            "the whole matrix"
        )
        return 0 if all(r.ok for r in results) else 1
    if args.from_results:
        results = merge_shards(read_results_dir(args.from_results), registry_shards())
    else:
        results = [
            run() for shard, run in registry if not (args.no_python and shard == "python")
        ]
    budget = load_budget()
    not_run = PYTHON_SUITES if args.no_python else frozenset()

    # Monotone-shrink ratchet: every run suite's ledgered-gap count must equal
    # its committed budget (growth and silent shrink both fail RED).
    enforce_ratchet(results, budget, not_run=not_run)

    text = render(results)
    print(text)

    # On a red suite, surface its captured log so CI shows the actual failure.
    for r in results:
        if not r.ok:
            print(f"\n----- captured log: {r.name} -----", file=sys.stderr)
            print(r.log, file=sys.stderr)

    summary_path = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary_path:
        with open(summary_path, "a", encoding="utf-8") as fh:
            fh.write(render_markdown(results))
            fh.write("\n")

    # Keep the published ledger honest: regenerate or drift-check the matrix
    # block in docs/CONFORMANCE.md against the freshly measured results. Only in
    # a full run (a native-only run cannot reproduce the whole table).
    doc_ok = True
    if not args.no_python:
        block = render_matrix_table(results)
        if args.write_doc:
            write_doc_block(block)
            print(f"wrote matrix block to {_DOC_PATH.relative_to(_REPO_ROOT)}")
        else:
            doc_ok = check_doc_block(block)

    return 0 if (all(r.ok for r in results) and doc_ok) else 1


if __name__ == "__main__":
    raise SystemExit(main())
