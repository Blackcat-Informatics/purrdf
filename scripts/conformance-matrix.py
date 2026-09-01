# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
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
from dataclasses import dataclass, field
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


def _suite_codec() -> SuiteResult:
    """Turtle/TriG/N-Triples/N-Quads/RDF-XML native-codec round-trip."""
    cmd = [
        "cargo", "test", "-p", "purrdf-rdf", "--locked",
        "--test", "native_codec_conformance", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, failed = _cargo_tally(out)
    m = re.search(r"TOTAL: total\s+(\d+)\s+passed\s+(\d+)\s+allowlisted-gap\s+(\d+)", out)
    if m:
        total, passed, gap = (int(m.group(i)) for i in (1, 2, 3))
        detail = f"{passed}/{total} vectors round-trip; {gap} allowlisted gaps"
        return SuiteResult(
            "Syntax codecs (Turtle/TriG/NT/NQ/RDF-XML)", "W3C rdf-tests",
            passed=passed, xskip=gap, failed=(total - passed - gap),
            detail=detail, ok=(rc == 0 and failed == 0), log=out,
        )
    return _no_scoreboard(
        "Syntax codecs (Turtle/TriG/NT/NQ/RDF-XML)", "W3C rdf-tests",
        "`TOTAL: total N passed N allowlisted-gap N`", cmd, out,
    )


def _suite_shacl_w3c() -> SuiteResult:
    cmd = [
        "cargo", "test", "-p", "purrdf-shapes", "--locked",
        "--test", "w3c_conformance", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, failed = _cargo_tally(out)
    m = re.search(r"TOTAL: passed (\d+), xfailed (\d+), ledger (\d+)", out)
    if m:
        passed, xfailed = int(m.group(1)), int(m.group(2))
        detail = f"{passed} pass · {xfailed} ledgered"
        return SuiteResult(
            "SHACL Core + SHACL-SPARQL", "W3C data-shapes",
            passed=passed, xskip=xfailed, failed=0,
            detail=detail, ok=(rc == 0 and failed == 0), log=out,
        )
    return _no_scoreboard(
        "SHACL Core + SHACL-SPARQL", "W3C data-shapes",
        "`TOTAL: passed N, xfailed N, ledger N`", cmd, out,
    )


def _suite_shapes_corpus() -> SuiteResult:
    """First-party SHACL corpus: scrape the harness's per-fixture scoreboard so
    the matrix reports a report-level Pass count, not the single test-function
    tally that ``_suite_cargo`` would yield."""
    cmd = [
        "cargo", "test", "-p", "purrdf-shapes", "--locked",
        "--test", "conformance", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, failed = _cargo_tally(out)
    m = re.search(r"SHAPES-CORPUS: passed (\d+) total (\d+)", out)
    if m:
        passed, total = int(m.group(1)), int(m.group(2))
        detail = f"{passed}/{total} byte-frozen expected reports"
        return SuiteResult(
            "SHACL (first-party corpus)", "first-party frozen reports",
            passed=passed, xskip=0, failed=(total - passed),
            detail=detail, ok=(rc == 0 and failed == 0 and passed == total), log=out,
        )
    return _no_scoreboard(
        "SHACL (first-party corpus)", "first-party frozen reports",
        "`SHAPES-CORPUS: passed N total N`", cmd, out,
    )


def _suite_shacl_rules() -> SuiteResult:
    """SHACL Rules (`sh:rule` inference): scrape the harness's per-fixture
    scoreboard so the matrix reports the inferred-graph fixture count rather than
    the single test-function tally that ``_suite_cargo`` would yield."""
    cmd = [
        "cargo", "test", "-p", "purrdf-shapes", "--locked",
        "--test", "rules_conformance", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, failed = _cargo_tally(out)
    m = re.search(r"RULES: passed (\d+) total (\d+)", out)
    if m:
        passed, total = int(m.group(1)), int(m.group(2))
        detail = f"{passed}/{total} inferred-graph fixtures"
        return SuiteResult(
            "SHACL Rules", "DASH + first-party",
            passed=passed, xskip=(total - passed), failed=0,
            detail=detail, ok=(rc == 0 and failed == 0 and passed == total), log=out,
        )
    return _no_scoreboard(
        "SHACL Rules", "DASH + first-party", "`RULES: passed N total N`", cmd, out,
    )


def _suite_shex_validation() -> SuiteResult:
    cmd = [
        "cargo", "test", "-p", "purrdf-shex", "--locked",
        "--test", "validation_conformance", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, failed = _cargo_tally(out)
    m = re.search(
        r"entries (\d+) \| attempted (\d+) \| pass (\d+) \| xfail (\d+) "
        r"\| fail (\d+) \| skipped (\d+)",
        out,
    )
    if m:
        attempted, passed, xfail, fail, skipped = (int(m.group(i)) for i in (2, 3, 4, 5, 6))
        detail = f"{passed}/{attempted} attempted · {skipped} trait-skips"
        return SuiteResult(
            "ShEx 2.1 validation", "shexTest v2.1.0",
            passed=passed, xskip=xfail + skipped, failed=fail,
            detail=detail, ok=(rc == 0 and failed == 0 and fail == 0), log=out,
        )
    return _no_scoreboard(
        "ShEx 2.1 validation", "shexTest v2.1.0",
        "`entries N | attempted N | pass N | xfail N | fail N | skipped N`", cmd, out,
    )


def _suite_sparql() -> SuiteResult:
    # The datatest harness writes each manifest tally to stderr.  Serialise its
    # cases so libtest progress output cannot splice through those tally lines.
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
            "SPARQL 1.1/1.2 evaluation (full corpus)",
            "W3C sparql11 + sparql12 + first-party",
            passed=passed, xskip=xfail, failed=failed + unexpected,
            detail=detail,
            ok=(rc == 0 and cargo_failed == 0 and failed == 0 and unexpected == 0),
            log=out,
        )
    return _no_scoreboard(
        "SPARQL 1.1/1.2 evaluation (full corpus)",
        "W3C sparql11 + sparql12 + first-party",
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
    cmd = [
        "cargo", "test", "-p", "purrdf-sparql-conformance", "--locked",
        "--test", "construct_corpus", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, failed = _cargo_tally(out)
    m = re.search(r"CONSTRUCT-CORPUS: passed (\d+) total (\d+)", out)
    if m:
        passed, total = int(m.group(1)), int(m.group(2))
        detail = (
            f"{passed}/{total} cases: triple-producing §16.2 + CONSTRUCT GRAPH quads, "
            "paired case for case, incl. the RDF 1.2 statement layer and its "
            "per-graph keying and BOTH terms a subject position refuses (a "
            "literal and a triple term) at BOTH depths it applies (an asserted "
            "subject and the subject of a triple term nested in an object, the "
            "depth at which an unenforced term model emits a document the "
            "engine's own readers refuse); the quad-template grammar bounded "
            "from both sides by 1 positive and 7 negative syntax verdicts"
        )
        return SuiteResult(
            "SPARQL CONSTRUCT (first-party corpus)", "purrdf-construct (first-party)",
            passed=passed, xskip=0, failed=(total - passed),
            detail=detail, ok=(rc == 0 and failed == 0 and passed == total), log=out,
        )
    return _no_scoreboard(
        "SPARQL CONSTRUCT (first-party corpus)", "purrdf-construct (first-party)",
        "`CONSTRUCT-CORPUS: passed N total N`", cmd, out,
    )


def _suite_describe_corpus() -> SuiteResult:
    """First-party DESCRIBE corpus (`crates/sparql-conformance/corpus/describe/`).

    §16.4 leaves the description implementation-defined, so no vendored manifest
    grades a DESCRIBE at all — this row is the only conformance measurement the
    form has, and it pins the engine's documented Symmetric CBD case by case.
    """
    cmd = [
        "cargo", "test", "-p", "purrdf-sparql-conformance", "--locked",
        "--test", "describe_corpus", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, failed = _cargo_tally(out)
    m = re.search(r"DESCRIBE-CORPUS: passed (\d+) total (\d+)", out)
    if m:
        passed, total = int(m.group(1)), int(m.group(2))
        detail = (
            f"{passed}/{total} cases pinning the symmetric CBD, incl. the RDF 1.2 "
            "statement layer on both sides of its subject-or-object disjunction and "
            "its per-graph scope over TriG"
        )
        return SuiteResult(
            "SPARQL DESCRIBE (first-party corpus)", "purrdf-describe (first-party)",
            passed=passed, xskip=0, failed=(total - passed),
            detail=detail, ok=(rc == 0 and failed == 0 and passed == total), log=out,
        )
    return _no_scoreboard(
        "SPARQL DESCRIBE (first-party corpus)", "purrdf-describe (first-party)",
        "`DESCRIBE-CORPUS: passed N total N`", cmd, out,
    )


def _suite_governor_corpus() -> SuiteResult:
    """First-party frozen execution-governor corpus.

    Scrapes the harness's own scoreboard so the matrix reports the *case* count —
    zero / boundary / over-bound per governor, plus the RDF 1.2 statement layer,
    the federated SERVICE seam and the deadline case — rather than the handful of
    test functions ``_suite_cargo`` would count. Every case is graded, so a
    non-zero fail is impossible to reach without the harness itself going red.
    """
    cmd = [
        "cargo", "test", "-p", "purrdf-sparql-conformance", "--locked",
        "--test", "governor_corpus", "--", "--nocapture",
    ]
    rc, out = _run(cmd, _REPO_ROOT)
    _, _, failed = _cargo_tally(out)
    m = re.search(r"GOVERNOR-CORPUS: passed (\d+) total (\d+) bands (\d+)", out)
    if m:
        passed, total, bands = (int(m.group(i)) for i in (1, 2, 3))
        detail = (
            f"{passed}/{total} pinned cases; {bands} zero/boundary/over-bound bands, "
            "frozen and content-addressed"
        )
        return SuiteResult(
            "SPARQL execution governors", "purrdf-sparql-governors (first-party)",
            passed=passed, xskip=0, failed=(total - passed),
            detail=detail, ok=(rc == 0 and failed == 0 and passed == total), log=out,
        )
    return _no_scoreboard(
        "SPARQL execution governors", "purrdf-sparql-governors (first-party)",
        "`GOVERNOR-CORPUS: passed N total N bands N`", cmd, out,
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


def _suite_py_rdflib_gate(build: bool) -> SuiteResult:
    """rdflib's OWN vendored tests run against the purrdf drop-in."""
    log = ""
    if build:
        rc, bout = _run(
            ["uv", "run", "--group", "dev", "maturin", "develop"], _PY_DIR
        )
        log += bout
        if rc != 0:
            return SuiteResult(
                "rdflib LSP drop-in gate", "rdflib 7.6 own tests",
                failed=-1, detail="maturin develop FAILED", ok=False, log=log,
            )
    rc, out = _run(
        ["uv", "run", "python", "-m", "tests.rdflib_suite.runner"], _PY_DIR
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
            ["uv", "run", "--group", "dev", "maturin", "develop"], _PY_DIR
        )
        log += bout
        if rc != 0:
            return SuiteResult(
                "Python binding suite", "first-party (incl. compat differential vs rdflib)",
                failed=-1, detail="maturin develop FAILED", ok=False, log=log,
            )
    rc, out = _run(
        ["uv", "run", "--group", "dev", "pytest", "tests", "-q"], _PY_DIR
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


def enforce_ratchet(results: list[SuiteResult], budget: dict[str, int]) -> None:
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

    orphans = sorted(set(budget) - {r.name for r in results})
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


def native_suites() -> list[SuiteResult]:
    return [
        _suite_cargo(
            "IRI (RFC 3987 / RFC 3986 resolution)", "W3C IRI + RFC vectors",
            ["cargo", "test", "-p", "purrdf-iri", "--locked",
             "--test", "w3c_iri", "--test", "iri_suite", "--test", "resolution"],
            detail="parse/validate/normalize/resolve vectors",
        ),
        _suite_cargo(
            "RDFC-1.0 canonicalization", "W3C rdf-canon",
            ["cargo", "test", "-p", "purrdf-rdf", "--locked", "--test", "rdfc_w3c"],
            detail="65 vectors (64 eval + 1 negative), sharded",
        ),
        _suite_cargo(
            "RDF 1.2 canonicalization profile", "purrdf-rdfc12 v1 (first-party)",
            ["cargo", "test", "-p", "purrdf-rdf", "--locked",
             "--test", "rdf12_canon_profile"],
            detail="17 goldens + 9 refusals, frozen and content-addressed",
        ),
        _suite_codec(),
        _suite_sparql(),
        _suite_construct_corpus(),
        _suite_describe_corpus(),
        _suite_governor_corpus(),
        _suite_entailment(),
        _suite_entailment_rl(),
        _suite_shacl_w3c(),
        _suite_shapes_corpus(),
        _suite_shacl_rules(),
        _suite_shex_validation(),
        _suite_cargo(
            "ShEx syntax + ShExC/ShExJ round-trip", "shexTest v2.1.0",
            ["cargo", "test", "-p", "purrdf-shex", "--locked",
             "--test", "syntax_conformance", "--test", "shexc_roundtrip",
             "--test", "shexj_roundtrip"],
            detail="schemas parse + negative syntax/structure",
        ),
        _suite_gts_vectors(),
    ]


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


# (row name, scraper, specimen lines). One entry per SCRAPED suite; the four
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
        "SPARQL 1.1/1.2 evaluation (full corpus)",
        _suite_sparql,
        (
            _noise("running 1 test"),
            # One manifest tally, because the property under test is "no tally
            # at all is RED". A manifest that drops out entirely is caught by
            # the datatest harness itself — its case fails and cargo goes
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
            _board("  TOTAL: passed 7, xfailed 2, ledger 2"),
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
    return problems


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
        help="skip `maturin develop` before the Python suites (assume prebuilt)",
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
    args = parser.parse_args()

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

    results = native_suites()
    if not args.no_python:
        # Build the native module once (in the rdflib gate); the compat suite
        # then reuses that editable install.
        build = not args.no_build
        results.append(_suite_py_rdflib_gate(build))
        results.append(_suite_py_compat(build=False))

    # Monotone-shrink ratchet: every run suite's ledgered-gap count must equal
    # its committed budget (growth and silent shrink both fail RED).
    enforce_ratchet(results, load_budget())

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
