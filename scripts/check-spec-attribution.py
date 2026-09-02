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
   book, the playground worker, the ``.pyi`` stub and the rdflib shim — or one
   of their Simplified Chinese counterparts (the same tuple; see the note on
   CJK spacing at :func:`normalize_cjk_spacing`). The extension phrasing and
   the specification token are invariant across languages (the code example
   is the anchor, and ``SPARQL 1.2`` stays English in Chinese prose), so a
   translated page that names the extension beside the specification with a
   Chinese disclaimer is exactly as honest as the English page and must pass
   exactly as the English page does.

Anything else is a hard failure naming the file, line, the anchor, and the
specification token that made it an attribution.

Scanned surface: every TRACKED ``.rs``, ``.py``, ``.pyi``, ``.md``, ``.mjs``,
``.ts`` and ``.js`` under ``crates/``, ``bindings/``, ``docs/`` and
``scripts/``, plus root ``*.md`` — enumerated by ``git ls-files``, exactly as
``check-brand-casing.py`` and ``check-issue-refs.py`` enumerate, so the four
prose gates agree on what "the tree" is. This one used to walk the
filesystem, and so scanned ``docs/book/book/searchindex.js`` after any local
``mdbook build``: mdBook's search index flattens a page's text, putting the
book's own CONSTRUCT-template sentence beside "SPARQL 1.1" with the
disclaimer out of the window, and the gate failed on a clean commit. The
price is the one its siblings already pay — an UNTRACKED file is not scanned,
so ``git add`` before running the gates. Whole file text is scanned rather
than only comments: the four recurrences this gate exists for lived in a Rust
module doc, a ``#[pyclass]`` doc comment, a Python module docstring and a
crate rustdoc — three different comment shapes and one that ships as runtime
data — and a rule that only saw comments would have missed the day it moves
into a string literal, a README table cell or a generated stub.

    python3 scripts/check-spec-attribution.py                      # verify (exit 1 on a hit)
    python3 scripts/check-spec-attribution.py --self-test          # prove the rule bites
    python3 scripts/check-spec-attribution.py --rendered-tree DIR  # scan a rendered book tree

``--rendered-tree DIR`` scans every ``.md`` under DIR — a rendering of
``docs/book/src/`` with a translation applied (``scripts/check-i18n-render.py``)
— which is build output and untracked, hence outside the default enumeration
twice over, and the only way a gettext translation reaches this gate.

The self-test is not decoration. It replays the exact four sentences that were
removed from the tree, asserts each is caught, replays their corrected
counterparts and the four pre-existing correct wordings, and asserts none is
caught — so a future edit that guts the pattern fails here instead of passing
silently and letting the seventh recurrence ship. It does the same for the
Chinese pairs (a translated page with its disclaimer passes; the same page
without it is caught), and it builds a throwaway git repository holding a
tracked violation and an untracked ``book/searchindex.js`` carrying the same
text, asserting the first is enumerated and caught and the second is not
enumerated — the enumeration rule, proven rather than described.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
import tempfile
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
# The two Chinese spellings are the glossary's rendering of "quad template"
# (四元组模板) and the bare "CONSTRUCT template" (CONSTRUCT 模板, with or
# without the house half-width space): a translated page that names the
# extension in prose rather than by its code example would otherwise carry no
# anchor at all, and the attribution would be invisible.
ANCHOR_ALTERNATION = (
    r"quad[\s-]?templates?|CONSTRUCT\s*\{\s*GRAPH"
    r"|CONSTRUCT\s+GRAPH\b|CONSTRUCT\s+templates?\b"
    r"|四元组模板|CONSTRUCT\s*模板"
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
# "SPARQL 1.1's `CONSTRUCT { GRAPH`", 「SPARQL 1.1 的四元组模板」 — 的 is the
# Chinese possessive). One intervening word breaks it, so a contrast sentence
# can never match and the rule needs no distance threshold.
ADJACENT_RE = re.compile(
    r"SPARQL\s*-?\s*1\.[12](?:'s|’s)?[\s*`_“\"的]{0,4}(?:"
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
    # Simplified Chinese, in the house voice (a half-width space between Latin
    # and CJK runs; the window is lower-cased before matching, which is why
    # `PurRDF` reads `purrdf` here). Each is the counterpart of an English
    # wording above, so a translated disclaimer is steered onto an agreed
    # spelling rather than a seventh coinage:
    "并非 sparql 1.2 特性",  # "not a SPARQL 1.2 feature"
    "并非 sparql 1.2",  # "not SPARQL 1.2 ..." (the shorter cut of the same)
    "不是 sparql 1.2 特性",  # "is not a SPARQL 1.2 feature"
    "非 sparql 特性",  # "not a SPARQL feature"
    "sparql 1.2 并未定义",  # "SPARQL 1.2 does not define"
    "sparql 并未定义",  # "SPARQL does not define"
    "并未定义",  # "does not define" — the verb phrase alone, as in English
    "sparql 1.1 与 sparql 1.2 均未定义",  # "neither SPARQL 1.1 nor SPARQL 1.2 defines"
    "均未定义",  # "neither ... defines"
    "第一方扩展",  # "first-party extension"
    "purrdf 扩展",  # "PurRDF extension"
    "purrdf 的扩展",  # "an extension of PurRDF"
    "没有相应的语法",  # "has no syntax (for it)"
    "并无相应的语法",  # "has no syntax (for it)", formal register
)

# A non-ASCII character (in practice a CJK character or full-width punctuation)
# with ASCII whitespace on one side of it. The house typography puts a
# half-width space between a Latin run and a CJK run — 「并非 SPARQL 1.2 特性」
# — and every Chinese disclaimer above is spelled that way; a translator who
# drops the space writes 「并非SPARQL 1.2特性」, which is the same disclaimer.
# The window is normalized by deleting that whitespace before the Chinese
# wordings are looked for, so the gate recognizes the disclaimer rather than
# the typography. Whitespace between two ASCII characters is never touched, so
# the English wordings are matched exactly as before.
_CJK_ADJACENT_SPACE = re.compile(r"(?<=[^\x00-\x7f])\s+|\s+(?=[^\x00-\x7f])")

SCAN_DIRS = ("crates", "bindings", "docs", "scripts")
SCAN_SUFFIXES = (".rs", ".py", ".pyi", ".md", ".mjs", ".ts", ".js")

# Tracked trees whose bytes are not ours to edit: a vendored suite is frozen
# upstream text. (Build output is untracked and so never enumerated at all —
# see ``iter_scan_paths``.)
SKIP_PARTS = frozenset(
    {
        "vendor",
        "node_modules",
        "__pycache__",
        "dist",
        "pkg",
    }
)

SELF_PATH = Path(__file__).resolve()


def repo_root() -> Path:
    return SELF_PATH.parent.parent


def normalize_cjk_spacing(text: str) -> str:
    """``text`` with whitespace beside a non-ASCII character removed.

    See :data:`_CJK_ADJACENT_SPACE`. ASCII-to-ASCII whitespace survives, so
    the English disclaimers are matched exactly as they were.
    """
    return _CJK_ADJACENT_SPACE.sub("", text)


def tracked_files(root: Path) -> list[str]:
    """Every path ``git ls-files`` reports under ``root``, repository-relative."""
    out = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return sorted(part for part in out.split("\0") if part)


def iter_scan_paths(root: Path) -> Iterator[Path]:
    """Every scanned file, in a deterministic order.

    Enumeration is ``git ls-files`` — the convention ``check-brand-casing.py``
    and ``check-issue-refs.py`` follow — so the scan covers exactly the
    committed first-party source: never a build output (``docs/book/book/``),
    never an untracked scratch file. ``root`` is a parameter rather than the
    repository so the self-test can enumerate a throwaway repository and prove
    the rule.
    """
    for rel in tracked_files(root):
        path = Path(rel)
        if path.suffix not in SCAN_SUFFIXES:
            continue
        segments = path.parts
        in_scan_dir = segments[0] in SCAN_DIRS
        root_markdown = len(segments) == 1 and path.suffix == ".md"
        if not (in_scan_dir or root_markdown):
            continue
        if SKIP_PARTS & set(segments):
            continue
        full = root / path
        if full.resolve() == SELF_PATH:
            continue
        if full.is_file():
            yield full


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
        normalized = normalize_cjk_spacing(window)
        return any(
            d in window or normalize_cjk_spacing(d) in normalized for d in DISCLAIMERS
        )

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
    # The translated page. The code example is the anchor and `SPARQL 1.2` is
    # invariant, so only the DISCLAIMER is Chinese — and it must be enough.
    (
        "a Chinese page: the quad template disclaimed in the house voice",
        "四元组模板（`CONSTRUCT { GRAPH ?g { ... } }`）并非 SPARQL 1.2 特性，"
        "而是 PurRDF 的扩展：它让一个 CONSTRUCT 结果为每条语句命名一个图。",
    ),
    (
        "a Chinese page: the same disclaimer with the Latin/CJK spaces dropped",
        "四元组模板（`CONSTRUCT { GRAPH ?g { ... } }`）并非SPARQL 1.2特性，"
        "而是PurRDF的扩展。",
    ),
    (
        "a Chinese page: the negative statement, SPARQL 1.2 as the subject",
        "**SPARQL 1.2 并未定义四元组模板。** SPARQL 1.1 与 SPARQL 1.2 的语法均无此项。",
    ),
    (
        "a Chinese page: the introduction's feature-list wording",
        "可 `CONSTRUCT` 到命名图中的四元组模板（第一方扩展，并非 SPARQL 1.2 特性）、"
        "调用方注册的聚合函数",
    ),
    (
        "a Chinese page: a plain SPARQL 1.1 CONSTRUCT description, no version claim",
        "SPARQL 1.1 §16.2 的 CONSTRUCT 模板把每个解转换为默认图中的三元组。",
    ),
    (
        "a Chinese page: an unrelated SPARQL 1.2 mention with no extension phrasing",
        "SPARQL 1.2 新增了三元组项、具体化节点与注解语法。",
    ),
)

# The translated page WITHOUT its disclaimer: the same false claim the four
# English recurrences made, in Chinese, and exactly as much a hard failure.
_MUST_CATCH_ZH: tuple[tuple[str, str], ...] = (
    (
        "a Chinese page attributing the quad template to SPARQL 1.2",
        "SPARQL 1.2 的 `CONSTRUCT { GRAPH ?g { ... } }` 四元组模板为每条语句命名一个图，"
        "因此一个结果可以跨越多个命名图。",
    ),
    (
        "a Chinese page calling the quad template a SPARQL 1.2 feature",
        "四元组模板是 SPARQL 1.2 的一项特性，可将 CONSTRUCT 结果写入命名图。",
    ),
    (
        "a Chinese page: SPARQL 1.2's CONSTRUCT template names a graph per statement",
        "SPARQL 1.2 的 CONSTRUCT 模板为每条语句命名一个图。",
    ),
    (
        "a Chinese page: the quad template attributed to SPARQL 1.1 by the possessive",
        "SPARQL 1.1 的四元组模板可写入命名图。",
    ),
)

# A file name mdBook writes into the (untracked) build output, carrying the
# flattened text that made the filesystem walk fail on a clean commit.
_UNTRACKED_BUILD_OUTPUT = "docs/book/book/searchindex.js"


def enumeration_self_test(report: bool) -> list[str]:
    """The enumeration rule, proven on a throwaway repository.

    Two files carry the same unqualified attribution: a TRACKED ``docs/x.md``
    and an UNTRACKED ``docs/book/book/searchindex.js``. The first must be
    enumerated and caught; the second must not be enumerated at all — the
    build output that used to fail this gate after any local ``mdbook build``.
    """
    failures: list[str] = []
    violation = _MUST_CATCH[3][1]
    with tempfile.TemporaryDirectory(prefix="check-spec-attribution-") as tmp:
        root = Path(tmp)
        subprocess.run(
            ["git", "init", "--quiet", str(root)], check=True, capture_output=True
        )
        tracked = root / "docs" / "x.md"
        tracked.parent.mkdir(parents=True)
        tracked.write_text(violation + "\n", encoding="utf-8")
        untracked = root / _UNTRACKED_BUILD_OUTPUT
        untracked.parent.mkdir(parents=True)
        untracked.write_text(violation + "\n", encoding="utf-8")
        subprocess.run(
            ["git", "-C", str(root), "add", "docs/x.md"],
            check=True,
            capture_output=True,
        )
        enumerated = [p.relative_to(root).as_posix() for p in iter_scan_paths(root)]
        if "docs/x.md" not in enumerated:
            failures.append(
                "NOT ENUMERATED: a tracked docs/x.md carrying a violation "
                f"(enumerated: {enumerated})"
            )
        elif not scan_path(tracked):
            failures.append("NOT CAUGHT: the violation in the tracked docs/x.md")
        elif report:
            print("  caught   a violation in a TRACKED file (throwaway repository)")
        if _UNTRACKED_BUILD_OUTPUT in enumerated:
            failures.append(
                f"ENUMERATED: the untracked {_UNTRACKED_BUILD_OUTPUT} — build output "
                "is outside the tree this gate scans"
            )
        elif not scan_path(untracked):
            failures.append(
                f"the untracked {_UNTRACKED_BUILD_OUTPUT} no longer carries a match, "
                "so its exclusion proves nothing about the enumeration"
            )
        elif report:
            print(
                f"  spared   the untracked {_UNTRACKED_BUILD_OUTPUT} (it matches, and "
                "is not enumerated)"
            )
    return failures


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

    for name, text in _MUST_CATCH_ZH:
        if not scan_text(text):
            failures.append(f"NOT CAUGHT: {name}")
        elif report:
            line, anchor, spec = scan_text(text)[0]
            print(f"  caught   {name}: line {line}, `{anchor}` near `{spec}`")

    failures.extend(enumeration_self_test(report))
    return failures


def scan_rendered_tree(tree: Path) -> int:
    """Scan every ``.md`` under a rendered book tree. Returns the exit code."""
    findings: list[str] = []
    scanned = 0
    for path in sorted(p for p in tree.rglob("*.md") if p.is_file()):
        scanned += 1
        for line, anchor, spec in scan_path(path):
            findings.append(f"{path}:{line}: `{anchor}` attributed to `{spec}`")
    if scanned == 0:
        print(
            f"check-spec-attribution: no .md file under {tree} — a rendered tree "
            "with nothing in it is a vacuous pass, not a clean one",
            file=sys.stderr,
        )
        return 1
    if findings:
        for f in findings:
            print(f"  {f}", file=sys.stderr)
        print(f"\n{len(findings)} unqualified attribution(s).", file=sys.stderr)
        return 1
    print(
        f"check-spec-attribution: no unqualified specification attributions in the "
        f"rendered tree ({scanned} page(s) scanned under {tree})."
    )
    return 0


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument(
        "--self-test",
        action="store_true",
        help="replay the removed sentences and their corrections, then scan",
    )
    ap.add_argument(
        "--rendered-tree",
        type=Path,
        metavar="DIR",
        help="scan every .md under DIR (a rendered, translated book) instead of the tree",
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

    if args.rendered_tree is not None:
        if not args.rendered_tree.is_dir():
            print(
                f"check-spec-attribution: {args.rendered_tree} is not a directory",
                file=sys.stderr,
            )
            return 2
        return scan_rendered_tree(args.rendered_tree)

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
