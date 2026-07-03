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

Usage:
    python3 scripts/conformance-matrix.py            # full matrix
    python3 scripts/conformance-matrix.py --no-python  # native Rust suites only
"""

from __future__ import annotations

import argparse
import difflib
import json
import os
import re
import subprocess
import sys
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
    log: str = field(default="", repr=False)

    @property
    def status(self) -> str:
        return "GREEN" if self.ok else "RED"


# ---------------------------------------------------------------------------
# Command runner + scoreboard scrapers
# ---------------------------------------------------------------------------


def _run(cmd: list[str], cwd: Path) -> tuple[int, str]:
    """Run *cmd*, return (returncode, combined stdout+stderr)."""
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
        failed=max(failed, 0),
        detail=detail,
        ok=(rc == 0 and failed == 0),
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
    return _suite_cargo(
        "Syntax codecs (Turtle/TriG/NT/NQ/RDF-XML)", "W3C rdf-tests", cmd
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
    return _suite_cargo("SHACL Core + SHACL-SPARQL", "W3C data-shapes", cmd)


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
    return _suite_cargo("ShEx 2.1 validation", "shexTest v2.1.0", cmd)


def _suite_sparql() -> SuiteResult:
    cmd = [
        "cargo", "test", "-p", "purrdf-sparql-conformance", "--locked",
        "--", "--nocapture",
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
    return _suite_cargo(
        "SPARQL 1.1/1.2 evaluation (full corpus)",
        "W3C sparql11 + sparql12 + first-party",
        cmd,
    )


def _suite_py_rdflib_gate(build: bool) -> SuiteResult:
    """rdflib's OWN vendored tests run against the purrdf drop-in."""
    log = ""
    if build:
        rc, bout = _run(
            ["uv", "run", "--group", "acceptance", "maturin", "develop"], _PY_DIR
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
    """The full first-party compat parity pytest suite."""
    log = ""
    if build:
        rc, bout = _run(
            ["uv", "run", "--group", "acceptance", "maturin", "develop"], _PY_DIR
        )
        log += bout
        if rc != 0:
            return SuiteResult(
                "purrdf.compat parity", "first-party (differential vs rdflib)",
                failed=-1, detail="maturin develop FAILED", ok=False, log=log,
            )
    rc, out = _run(
        ["uv", "run", "--group", "acceptance", "pytest", "tests", "-q"], _PY_DIR
    )
    log += out
    passed = _int(re.search(r"(\d+) passed", out))
    xfailed = _int(re.search(r"(\d+) xfailed", out))
    failed = _int(re.search(r"(\d+) failed", out))
    xpassed = _int(re.search(r"(\d+) xpassed", out))
    errors = _int(re.search(r"(\d+) error", out))
    detail = f"{passed} pass · {xfailed} strict-xfail (ledgered)"
    return SuiteResult(
        "purrdf.compat parity", "first-party (differential vs rdflib)",
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
      * a run suite with no budget entry fails RED.

    Suites that could not emit a scoreboard (``failed < 0`` — a compile error or
    aborted harness) keep their own failure and are not re-diagnosed here.
    """
    for r in results:
        r.budget = budget.get(r.name)
        if r.failed < 0:
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
        _suite_codec(),
        _suite_sparql(),
        _suite_shacl_w3c(),
        _suite_cargo(
            "SHACL (first-party corpus)", "first-party frozen reports",
            ["cargo", "test", "-p", "purrdf-shapes", "--locked", "--test", "conformance"],
            detail="48 byte-frozen expected reports",
        ),
        _suite_shex_validation(),
        _suite_cargo(
            "ShEx syntax + ShExC/ShExJ round-trip", "shexTest v2.1.0",
            ["cargo", "test", "-p", "purrdf-shex", "--locked",
             "--test", "syntax_conformance", "--test", "shexc_roundtrip",
             "--test", "shexj_roundtrip"],
            detail="schemas parse + negative syntax/structure",
        ),
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


def check_doc_block(block: str) -> bool:
    """True iff the committed matrix block equals the freshly measured one."""
    _, inner, _ = _split_doc(_DOC_PATH.read_text(encoding="utf-8"))
    if inner.strip() == block.strip():
        return True
    print(
        f"\n{_DOC_PATH.relative_to(_REPO_ROOT)} conformance-matrix block is stale; "
        "regenerate with `python3 scripts/conformance-matrix.py --write-doc`.",
        file=sys.stderr,
    )
    diff = difflib.unified_diff(
        inner.strip().splitlines(),
        block.strip().splitlines(),
        fromfile="committed",
        tofile="measured",
        lineterm="",
    )
    print("\n".join(diff), file=sys.stderr)
    return False


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
    args = parser.parse_args()

    if args.write_doc and args.no_python:
        # The committed doc block reflects the full 10-row matrix; a native-only
        # run cannot reproduce it.
        parser.error("--write-doc requires the full suite (do not pass --no-python)")

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
