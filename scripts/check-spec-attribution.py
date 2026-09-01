# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Reject attributing a PurRDF **extension** to a W3C specification.

PurRDF ships surfaces that no SPARQL specification defines. The largest is the
**quad template** — ``CONSTRUCT { GRAPH ?g { ... } }`` and the whole-template
``CONSTRUCT GRAPH <iri> { ... }`` shorthand — which lets one ``CONSTRUCT``
result name a graph per statement. Neither SPARQL 1.1 nor SPARQL 1.2 has that
grammar: a reader who trusts a doc calling it "a SPARQL 1.2 feature" and takes
the query to another 1.2 engine gets a parse error with nothing to explain it.

That false attribution has been written, removed, and rewritten repeatedly —
including into a ``#[pyclass]`` doc comment, which is the class ``__doc__`` in
the shipped wheel, contradicting the ``.pyi`` stub for the SAME class in the
SAME wheel. Prose review does not catch it, because each recurrence is a
plausible sentence in isolation. A mechanical rule does.

The rule is a **co-occurrence** rule, not a banned phrase: the extension may be
described, and the specification may be named, but not in a way that reads as
the specification defining the extension.

1. Anchor on **extension phrasing** — ``quad template``, ``CONSTRUCT { GRAPH``,
   ``CONSTRUCT GRAPH``, or a bare ``CONSTRUCT template``.
2. If a **specification token** (``SPARQL 1.2``, ``SPARQL 1.1``, or a bare
   ``SPARQL`` immediately followed by a version) appears within
   :data:`WINDOW` characters of the anchor, the text is making an attribution
   claim about the extension.
3. That claim passes ONLY when a **disclaimer** also appears within the same
   window — one of :data:`DISCLAIMERS`, the wordings already established in the
   book, the playground worker, the ``.pyi`` stub and the rdflib shim.

Anything else is a hard failure naming the file, line, the anchor, and the
specification token that made it an attribution.

Scanned surface: ``.rs``, ``.py``, ``.pyi``, ``.md``, ``.mjs``, ``.ts`` and
``.js`` under ``crates/``, ``bindings/``, ``docs/`` and ``scripts/``, plus root
``*.md``. Whole file text is scanned rather than only comments: the four
recurrences this gate exists for lived in a Rust module doc, a ``#[pyclass]``
doc comment, a Python module docstring and a crate rustdoc — three different
comment shapes and one that ships as runtime data — and a rule that only saw
comments would have missed the day it moves into a string literal, a README
table cell or a generated stub.

    python3 scripts/check-spec-attribution.py              # verify (exit 1 on a hit)
    python3 scripts/check-spec-attribution.py --self-test  # prove the rule bites

The self-test is not decoration. It replays the exact four sentences that were
removed from the tree, asserts each is caught, replays their corrected
counterparts and the four pre-existing correct wordings, and asserts none is
caught — so a future edit that guts the pattern fails here instead of passing
silently and letting the seventh recurrence ship.
"""

from __future__ import annotations

import argparse
import re
import sys
from collections.abc import Iterator
from pathlib import Path

# How far from the extension anchor a specification token still reads as an
# attribution of it. Wide enough to span the sentence a doc comment wraps over
# three or four lines, narrow enough that an unrelated mention elsewhere in a
# long Markdown paragraph is not swept in.
WINDOW = 220

# Phrasing that names the extension: the quad-producing template's own
# spellings, plus the generic `CONSTRUCT template` — three of the four
# recurrences never said "quad" at all, they said "a SPARQL 1.2 CONSTRUCT
# template names a graph per statement", which is the same false claim.
ANCHOR_ALTERNATION = (
    r"quad[\s-]?templates?|CONSTRUCT\s*\{\s*GRAPH"
    r"|CONSTRUCT\s+GRAPH\b|CONSTRUCT\s+templates?\b"
)
ANCHOR_RE = re.compile(ANCHOR_ALTERNATION, re.IGNORECASE)

# Naming SPARQL 1.2. A bare `SPARQL` is not enough — the engine is a SPARQL
# engine and says so everywhere; it is the VERSION that turns a description into
# an attribution, and 1.2 is the version every recurrence has named. It is also
# the only version a reader cannot check from memory: SPARQL 1.1's grammar is
# long settled, so every 1.1 mention beside the extension in this tree is an
# explicit CONTRAST ("the SPARQL 1.1 §16.2 form ... and the quad-producing
# form") — exactly the writing this gate wants, and a rule that flagged those
# would be switched off within a week.
SPEC_RE = re.compile(r"SPARQL\s*-?\s*1\.2", re.IGNORECASE)

# The one shape in which naming SPARQL **1.1** is still an attribution: the
# version token DIRECTLY modifying the extension phrase, with nothing but a
# possessive or markup between them ("a SPARQL 1.1 quad template",
# "SPARQL 1.1's `CONSTRUCT { GRAPH`"). One intervening word breaks it, so a
# contrast sentence can never match and the rule needs no distance threshold.
ADJACENT_RE = re.compile(
    r"SPARQL\s*-?\s*1\.[12](?:'s|’s)?[\s*`_“\"]{0,4}(?:"
    + ANCHOR_ALTERNATION
    + r")",
    re.IGNORECASE,
)

# Wordings that make the attribution honest. These are the phrasings already in
# the tree (the book, the introduction's feature list, the playground worker,
# the `.pyi` stub, the rdflib shim), so a new site is steered onto an existing
# spelling rather than inventing a seventh.
DISCLAIMERS: tuple[str, ...] = (
    "not defined by sparql",
    "not a sparql 1.2 feature",
    "not a sparql feature",
    "does not define",
    "do not define",
    "define no",
    "first-party extension",
    "first party extension",
    "purrdf extension",
    "neither sparql 1.1 nor sparql 1.2",
    "nor sparql 1.2",
    "no syntax to ask for",
    "has no syntax",
)

SCAN_DIRS = ("crates", "bindings", "docs", "scripts")
SCAN_SUFFIXES = (".rs", ".py", ".pyi", ".md", ".mjs", ".ts", ".js")

# Trees whose bytes are not ours to edit: a vendored suite is frozen upstream
# text and a build output is a projection of a scanned source.
SKIP_PARTS = frozenset(
    {
        ".git",
        "target",
        "node_modules",
        "vendor",
        "__pycache__",
        "dist",
        "build",
        ".venv",
        "pkg",
    }
)

SELF_PATH = Path(__file__).resolve()


def repo_root() -> Path:
    return SELF_PATH.parent.parent


def iter_scan_paths(root: Path) -> Iterator[Path]:
    """Every scanned file, in a deterministic order."""
    for name in SCAN_DIRS:
        base = root / name
        if not base.is_dir():
            continue
        for path in sorted(base.rglob("*")):
            if not path.is_file() or path.suffix not in SCAN_SUFFIXES:
                continue
            if SKIP_PARTS & set(path.relative_to(root).parts):
                continue
            if path.resolve() == SELF_PATH:
                continue
            yield path
    for path in sorted(root.glob("*.md")):
        if path.is_file():
            yield path


def pos_to_line(src: str, pos: int) -> int:
    return src.count("\n", 0, pos) + 1


def scan_text(src: str) -> list[tuple[int, str, str]]:
    """Return ``(line, extension-phrase, specification-token)`` for every
    unqualified attribution in *src*.

    Two rules, both cleared by a :data:`DISCLAIMERS` phrase in the same window:

    * ``SPARQL 1.2`` anywhere within :data:`WINDOW` of an extension anchor, and
    * ``SPARQL 1.1`` or ``1.2`` DIRECTLY modifying an extension anchor.

    An anchor is reported at most once however many rules or version tokens
    reach it, so one bad sentence is one finding. Findings come back in source
    order, so the report reads top-to-bottom.
    """
    hits: dict[int, tuple[int, str, str]] = {}

    def record(start: int, anchor_text: str, spec_text: str) -> None:
        hits.setdefault(
            start,
            (
                pos_to_line(src, start),
                " ".join(anchor_text.split()),
                " ".join(spec_text.split()),
            ),
        )

    def disclaimed(start: int, end: int) -> bool:
        window = src[max(0, start - WINDOW) : min(len(src), end + WINDOW)].lower()
        return any(d in window for d in DISCLAIMERS)

    for anchor in ANCHOR_RE.finditer(src):
        lo = max(0, anchor.start() - WINDOW)
        hi = min(len(src), anchor.end() + WINDOW)
        spec = SPEC_RE.search(src[lo:hi])
        if spec is None or disclaimed(anchor.start(), anchor.end()):
            continue
        record(anchor.start(), anchor.group(0), spec.group(0))

    for pair in ADJACENT_RE.finditer(src):
        if disclaimed(pair.start(), pair.end()):
            continue
        anchor = ANCHOR_RE.search(src, pair.start(), pair.end())
        spec = SPEC_RE.search(pair.group(0)) or re.search(
            r"SPARQL\s*-?\s*1\.[12]", pair.group(0), re.IGNORECASE
        )
        if anchor is None or spec is None:
            continue
        record(anchor.start(), anchor.group(0), spec.group(0))

    return [hits[k] for k in sorted(hits)]


def scan_path(path: Path) -> list[tuple[int, str, str]]:
    try:
        return scan_text(path.read_text(encoding="utf-8"))
    except UnicodeDecodeError:
        return []


# ── This gate's own falsifiability ────────────────────────────────────────────
#
# The four sentences below are the ones that were live in the tree when this
# gate was written, VERBATIM. If a future edit narrows the pattern until one of
# them slips through, this self-test goes red at the same moment — which is the
# only reason to believe the live scan's silence means anything.

_MUST_CATCH: tuple[tuple[str, str], ...] = (
    (
        "bindings/python/src/py_store/query.rs module rustdoc",
        "//! A SPARQL 1.2 CONSTRUCT template names a graph per statement, so one "
        "result may span\n//! several named graphs and may mix them with "
        "default-graph triples.",
    ),
    (
        "bindings/python/src/py_store/query.rs QueryQuads pyclass __doc__",
        "/// A SPARQL 1.2 CONSTRUCT template carries a graph per STATEMENT: one "
        "template may\n/// write several named graphs, and may mix default-graph "
        "triples with named-graph\n/// quads.",
    ),
    (
        "bindings/python/tests/test_construct_named_graphs.py module docstring",
        '"""A quad-template CONSTRUCT keeps its graph names all the way into '
        "Python.\n\nSPARQL 1.2 lets a CONSTRUCT template name a graph per "
        'statement, so one result may\nwrite several named graphs."""',
    ),
    (
        "crates/sparql-results/src/graph.rs module rustdoc",
        "//! SPARQL 1.2's `CONSTRUCT { GRAPH ?g { … } }` is a quad template: "
        "the graph name\n//! is in the query the caller wrote, one token at a time.",
    ),
)

# Wordings that MUST pass: the four corrections that replaced the strings above,
# and the four pre-existing correct sites the corrections were modelled on.
_MUST_PASS: tuple[tuple[str, str], ...] = (
    (
        "the corrected module rustdoc",
        "//! A quad-template CONSTRUCT (`CONSTRUCT { GRAPH ?g { … } }` — a "
        "first-party\n//! extension, NOT defined by SPARQL 1.2) names a graph per "
        "statement.",
    ),
    (
        "the corrected pyclass __doc__",
        "/// A quad-template CONSTRUCT (`CONSTRUCT { GRAPH ?g { … } }` — a "
        "first-party\n/// extension, NOT defined by SPARQL 1.2) carries a graph per "
        "STATEMENT.",
    ),
    (
        "the corrected Python module docstring",
        '"""A quad template (`CONSTRUCT { GRAPH ?g { ... } }` — a first-party '
        'extension, NOT\ndefined by SPARQL 1.2) names a graph per statement."""',
    ),
    (
        "the corrected sparql-results rustdoc",
        "//! `CONSTRUCT { GRAPH ?g { … } }` — a first-party extension, NOT "
        "defined by SPARQL\n//! 1.2 — is a quad template.",
    ),
    (
        "the book's negative statement",
        "**SPARQL 1.2 does not define the quad template.** Neither the 1.1 nor the "
        "1.2 grammar has it.",
    ),
    (
        "the introduction's feature list",
        "quad templates that `CONSTRUCT` into named graphs (a first-party "
        "extension, not a SPARQL 1.2 feature), caller-registered aggregates",
    ),
    (
        "a plain SPARQL 1.1 CONSTRUCT description carrying no version claim",
        "The SPARQL 1.1 §16.2 CONSTRUCT template turns each solution into "
        "triples in the default graph.",
    ),
    (
        "an unrelated SPARQL 1.2 mention with no extension phrasing nearby",
        "SPARQL 1.2 adds triple terms, reifiers and annotation syntax to the "
        "query language.",
    ),
)


def self_test(report: bool) -> list[str]:
    """Prove the rule bites, and prove it does not over-bite."""
    failures: list[str] = []

    for name, text in _MUST_CATCH:
        if not scan_text(text):
            failures.append(f"NOT CAUGHT: {name}")
        elif report:
            line, anchor, spec = scan_text(text)[0]
            print(f"  caught   {name}: line {line}, `{anchor}` near `{spec}`")

    for name, text in _MUST_PASS:
        hits = scan_text(text)
        if hits:
            failures.append(f"FALSE POSITIVE: {name} -> {hits}")
        elif report:
            print(f"  passes   {name}")

    # A quad-template spelling is caught whatever version it names: no SPARQL
    # grammar defines it, so 1.1 is exactly as false a claim as 1.2.
    downgraded = _MUST_CATCH[3][1].replace("1.2", "1.1")
    if not scan_text(downgraded):
        failures.append("NOT CAUGHT: the quad template attributed to SPARQL 1.1")
    elif report:
        print("  caught   the quad template attributed to SPARQL 1.1")

    return failures


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--self-test",
        action="store_true",
        help="replay the removed sentences and their corrections, then scan",
    )
    args = ap.parse_args(argv)

    if args.self_test:
        print("check-spec-attribution self-test:")
        failures = self_test(report=True)
        if failures:
            print("\nSELF-TEST FAILED:", file=sys.stderr)
            for f in failures:
                print(f"  {f}", file=sys.stderr)
            return 1
        print("  self-test OK")

    root = repo_root()
    findings: list[str] = []
    for path in iter_scan_paths(root):
        rel = path.relative_to(root).as_posix()
        for line, anchor, spec in scan_path(path):
            findings.append(f"{rel}:{line}: `{anchor}` attributed to `{spec}`")

    if findings:
        print(
            "ERROR: a PurRDF extension is attributed to a SPARQL specification.\n"
            "The quad-producing CONSTRUCT template (`CONSTRUCT { GRAPH ?g { ... } }`\n"
            "and `CONSTRUCT GRAPH <iri> { ... }`) is a first-party extension: no\n"
            "SPARQL grammar defines it, and a reader who believes otherwise gets a\n"
            "parse error on another engine with nothing to explain it.\n\n"
            "Fix by naming it as an extension where it is described, e.g.\n"
            '  "a quad template (`CONSTRUCT { GRAPH ?g { ... } }` - a first-party\n'
            '   extension, NOT defined by SPARQL 1.2)"\n',
            file=sys.stderr,
        )
        for f in findings:
            print(f"  {f}", file=sys.stderr)
        print(f"\n{len(findings)} unqualified attribution(s).", file=sys.stderr)
        return 1

    print("check-spec-attribution: no unqualified specification attributions.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
