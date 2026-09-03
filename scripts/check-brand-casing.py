# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Reject lowercase ``purrdf`` in PROSE — the project is **PurRDF** in prose.

The brand is **PurRDF** in prose and ``purrdf`` in identifiers (crate names,
Cargo/Python/npm package names, module paths, the CLI binary, URLs). Bare
lowercase ``purrdf`` used as an ordinary English word — "a purrdf extension",
"purrdf follows RDF4J's permissiveness" — is a casing bug, not a citation of an
identifier, and this gate rejects it wherever it appears as prose.

A match is a BARE word: ``purrdf`` with no ``-``, ``_``, ``:``, ``/``, ``.``,
backtick, or alphanumeric character immediately before or after it, and not
inside a Markdown inline-code span or fenced code block. Every one of those
adjacent characters marks the word as part of a longer identifier instead of
prose: ``purrdf-xsd``/``purrdf_xsd`` are crate/module names, ``purrdf::`` is a
Rust path, ``purrdf/`` and ``purrdf.h`` are filesystem/header paths, a
backtick-wrapped `` `purrdf` `` is a code span naming the crate, and an
ASCII-alphanumeric neighbour makes it a fragment of a longer identifier or
plural — ``libpurrdf``, ``purrdfs`` — not the bare word. None of those are
prose and none are rewritten. The neighbour test is ASCII on purpose:
``str.isalnum`` is Unicode-aware (``'本'.isalnum()`` is ``True``), and with it a
bare ``purrdf`` glued to CJK prose — ``本purrdf项目`` — was classified as an
identifier fragment and never flagged, while the same word one half-width
space away (``本 PurRDF 项目``, the house typography) was. Every identifier
this repository mints is ASCII, so a non-ASCII neighbour is prose by
construction. A bare match IS prose, and the fix depends on what it
names: if it names the PROJECT/BRAND, capitalize it to ``PurRDF``; if it names
the ``purrdf`` FACADE CRATE specifically (a legitimate but unmarked mention),
wrap it in backticks instead of capitalizing it — the crate's own name does
not change.

This lint scans:

* ``.rs`` files under ``crates/`` and ``bindings/`` — only Rust comments (both
  doc comments and ordinary ``//``/``/* */`` comments) are examined, using the
  same conservative string/char/raw-string-literal-skipping scanner
  ``check-issue-refs.py`` uses, so a ``//`` inside a string literal is never
  mistaken for a comment start.
* ``.md`` files under ``crates/``, ``bindings/``, ``docs/``, and root ``*.md``
  files — fenced code blocks are skipped entirely, and Markdown inline-code
  spans (backtick-delimited, including multi-backtick spans) are excluded so a
  ``purrdf`` that is part of a rendered code span is not flagged.

Deliberately narrower than ``check-issue-refs.py``'s five-extension scan:
``.toml``/``.py``/``.yaml``/``.rq`` are not covered. Brand-name prose is a
property of documentation and doc/regular comments; manifest ``description``
fields are covered by their own convention (checked by hand — every
`PurRDF`-branded crate description already opens with the capitalized form),
and ``.py``/``.yaml`` comments in this repository do not carry brand-name
prose today. Widen this list the day one does.

    python3 scripts/check-brand-casing.py                       # verify (exit 1 on a hit)
    python3 scripts/check-brand-casing.py --self-test           # prove the rule bites both ways
    python3 scripts/check-brand-casing.py --rendered-tree DIR   # scan a rendered book tree

The self-test replays glued-CJK, spaced-CJK, code-span and identifier shapes
against the same scanners the tree scan uses, and runs before every scan: a
shape this gate cannot see is a shape that ships with the gate green.

``--rendered-tree DIR`` scans every ``.md`` under DIR — a rendering of
``docs/book/src/`` produced by ``mdbook build`` with the ``markdown`` renderer
and a translation applied (see ``scripts/check-i18n-render.py``). The register
below is consulted through that mapping (``DIR/x.md`` is
``docs/book/src/x.md``), and in that mode it is a CEILING rather than an exact
count: a translation that carries fewer bare mentions than its English source
is an improvement, not stale debt, and the English source's own scan is what
keeps the register exact. Nothing under ``docs/book/book`` or any other build
output is ever scanned by the default mode, so this flag is the only way a
translation reaches this gate at all.

Like ``check-issue-refs.py``'s ``PRE_EXISTING_PROCESS_REFERENCES``, the debt
that predates this rule is carried in a single frozen register,
``PRE_EXISTING_BRAND_CASING``, which may only SHRINK. It differs from that
register's shape for one reason: every ``check-issue-refs.py`` token family
carries a distinguishing payload, so its register keys are distinguishable
per-token phrases — two occurrences in the same file with different payloads
are still kept as separate entries, so a bare ``(file, token)`` pair catches a
genuinely NEW instance even in an already-registered file. This lint's token
is always the same literal word — there is nothing to distinguish one
``purrdf`` from another — so a presence-only register would let an
already-listed file accumulate unlimited NEW bare mentions for free. The
register instead pairs each file with its pre-existing OCCURRENCE COUNT: a
live count above the registered number is new debt (a hard failure, reported
the same as an unregistered file), a live count below it means debt was paid
down (also a hard failure — the entry must be edited down to match, exactly as
a stale entry is elsewhere), and a live count matching the register is the
only way a listed file passes silently.
"""

from __future__ import annotations

import re
import subprocess
import sys
from collections.abc import Iterator
from pathlib import Path

# Files that predate this rule, paired with the exact number of bare-``purrdf``
# prose occurrences they carried when registered. THIS REGISTER MAY ONLY
# SHRINK: a live count that does not match the registered number — whether
# higher (new debt) or lower (debt paid down but the entry left stale) — is a
# hard failure, so the only way to change a file's count here is to edit it to
# match a fresh scan.
PRE_EXISTING_BRAND_CASING: frozenset[tuple[str, int]] = frozenset(
    {
        ("AGENTS.md", 1),
        ("CHANGELOG.md", 7),
        ("CONTRIBUTING.md", 2),
        ("LICENSING.md", 1),
        ("PROVENANCE.md", 5),
        ("bindings/python-rdflib-shadow/README.md", 3),
        ("bindings/python/src/py_store/query.rs", 1),
        ("bindings/python/src/py_store/store.rs", 2),
        ("bindings/python/src/rdf.rs", 1),
        ("bindings/python/tests/README.md", 1),
        ("bindings/python/tests/rdflib_suite/vendor/PROVENANCE.md", 2),
        ("crates/cli/src/cli.rs", 2),
        ("crates/cli/src/query.rs", 1),
        ("crates/cli/src/update.rs", 1),
        ("crates/cli/tests/convert_cli.rs", 2),
        ("crates/iri/src/lib.rs", 1),
        ("crates/purrdf/README.md", 1),
        ("crates/rdf-capi/README.md", 2),
        ("crates/rdf-capi/src/lib.rs", 2),
        ("crates/rdf-capi/src/query.rs", 4),
        ("crates/rdf-core/benches/mutable.rs", 1),
        ("crates/rdf-core/src/backend.rs", 2),
        ("crates/rdf-core/src/dataset_view.rs", 3),
        ("crates/rdf-core/src/ir/canon.rs", 2),
        ("crates/rdf-core/src/ir/dataset.rs", 2),
        ("crates/rdf-core/src/ir/event_sink.rs", 1),
        ("crates/rdf-core/src/ir/global.rs", 1),
        ("crates/rdf-core/src/ir/ingest.rs", 1),
        ("crates/rdf-core/src/ir/mod.rs", 2),
        ("crates/rdf-core/src/ir/mutable.rs", 2),
        ("crates/rdf-core/src/ir/term.rs", 1),
        ("crates/rdf-core/src/lib.rs", 4),
        ("crates/rdf-core/src/turtle.rs", 1),
        ("crates/rdf-core/src/turtle_render.rs", 1),
        ("crates/rdf-events/src/lib.rs", 1),
        ("crates/rdf-wasm/README.md", 3),
        ("crates/rdf-wasm/js/README.md", 1),
        ("crates/rdf-wasm/src/codec.rs", 1),
        ("crates/rdf-wasm/src/convert.rs", 1),
        ("crates/rdf-wasm/src/lib.rs", 4),
        ("crates/rdf-wasm/src/query.rs", 3),
        ("crates/rdf/benches/native_codecs.rs", 1),
        ("crates/rdf/src/gts_compose.rs", 1),
        ("crates/rdf/src/gts_view.rs", 1),
        ("crates/rdf/src/gts_write.rs", 3),
        ("crates/rdf/src/lib.rs", 2),
        ("crates/rdf/src/native_codecs/mod.rs", 1),
        ("crates/rdf/src/native_codecs/text_parse.rs", 2),
        ("crates/rdf/src/statements.rs", 1),
        ("crates/rdf/src/turtle_normalize.rs", 1),
        ("crates/rdf/tests/gts_codec_hygiene.rs", 1),
        ("crates/rdf/tests/never_panic.rs", 1),
        ("crates/rdf/tests/proptest_roundtrip.rs", 1),
        ("crates/shapes/src/constraints.rs", 1),
        ("crates/shapes/src/engine.rs", 1),
        ("crates/shapes/src/expression.rs", 2),
        ("crates/shapes/src/report.rs", 2),
        ("crates/shapes/src/text_ingest.rs", 1),
        ("crates/slice/src/claim_view.rs", 3),
        ("crates/slice/src/dsl_stats_emit.rs", 1),
        ("crates/slice/src/mapping_support.rs", 1),
        ("crates/slice/src/ownership.rs", 1),
        ("crates/slice/src/prefix_lint.rs", 1),
        ("crates/slice/src/standpoint_emit.rs", 1),
        ("crates/slice/tests/ownership_tests.rs", 4),
        ("crates/sparql-algebra/src/ast.rs", 1),
        ("crates/sparql-algebra/src/error.rs", 1),
        ("crates/sparql-algebra/src/lexer.rs", 1),
        ("crates/sparql-algebra/src/lib.rs", 2),
        ("crates/sparql-algebra/src/parser.rs", 2),
        ("crates/sparql-algebra/src/substitute.rs", 1),
        ("crates/sparql-algebra/tests/algebra_snapshots.rs", 1),
        ("crates/sparql-algebra/tests/corpus.rs", 1),
        ("crates/sparql-conformance/tests/corpus_conformance.rs", 1),
        ("crates/sparql-eval/src/engine.rs", 1),
        ("crates/sparql-eval/src/lib.rs", 1),
        ("crates/sparql-eval/src/substitute.rs", 1),
        ("crates/sparql-results/src/lib.rs", 1),
        ("crates/sparql-results/tests/results_corpus.rs", 2),
        ("crates/validate/src/model.rs", 1),
        ("docs/BENCHMARKS.md", 1),
        ("docs/CUTOVER.md", 4),
        ("docs/book/src/interop/rdflib.md", 1),
        ("docs/book/src/project/design-rules.md", 1),
    }
)

# Mirrors ``check-issue-refs.py``'s ``SCAN_DIRS``: ``.rs`` under these plus
# ``.md`` under these plus root ``.md`` files.
SCAN_DIRS = ("crates", "bindings", "docs")

# A bare-word match may not touch one of these characters on either side —
# each one marks the word as part of an identifier rather than prose. An
# ASCII-alphanumeric neighbour is checked separately in ``is_bare`` (it marks
# a longer identifier or plural, e.g. ``libpurrdf``/``purrdfs``, and cannot be
# enumerated as a fixed character set).
_ADJACENT_IDENTIFIER_CHARS = frozenset("-_:/.`")

# Where a rendered book tree's files live in the source tree, for register
# lookups in ``--rendered-tree`` mode.
RENDERED_SOURCE_PREFIX = "docs/book/src/"

BRAND_RE = re.compile(r"purrdf")

# This checker's own path, excluded so its self-documentation (the docstring
# above, which names the very identifiers this lint must not flag) is never
# scanned. Mirrors ``check-issue-refs.py``'s ``SELF_PATH`` exclusion.
SELF_PATH = Path(__file__).resolve()


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def iter_scan_paths(root: Path) -> Iterator[Path]:
    """Yield every tracked ``.rs``/``.md`` file this lint enforces.

    ``.rs`` under ``crates``/``bindings``; ``.md`` under ``crates``/
    ``bindings``/``docs`` plus root ``.md`` files — the same directory
    convention ``check-issue-refs.py`` uses for those two extensions.
    Enumeration is driven by ``git ls-files`` so the scan covers exactly the
    committed first-party source, not untracked build artifacts.
    """
    out = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    for rel in sorted(part for part in out.split("\0") if part):
        suffix = Path(rel).suffix
        if suffix not in (".rs", ".md"):
            continue
        segments = rel.split("/")
        top = segments[0]
        in_scan_dir = top in SCAN_DIRS
        root_file = len(segments) == 1 and rel.endswith(".md")
        if not (in_scan_dir or root_file):
            continue
        path = root / rel
        if path.resolve() == SELF_PATH:
            continue
        if path.is_file():
            yield path
    return


def pos_to_line_col(src: str, pos: int) -> tuple[int, int]:
    """Convert a 0-based source index to 1-based line/column."""
    line = src.count("\n", 0, pos) + 1
    last_nl = src.rfind("\n", 0, pos)
    col = pos - last_nl
    return line, col


def rust_comments(src: str) -> list[tuple[int, int, str]]:
    """Extract Rust comments as ``(start_line, start_col, comment_text)``.

    A small Rust-aware lexer skips string, character, and raw-string literals
    so ``//`` inside ``"http://example.org"`` is not treated as a comment —
    the same scanner ``check-issue-refs.py`` uses, trimmed to the comments it
    returns (this lint has no use for string-literal contents).
    """
    comments: list[tuple[int, int, str]] = []
    n = len(src)
    i = 0
    line, col = 1, 1

    while i < n:
        c = src[i]

        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            if j == -1:
                j = n
            line, col = pos_to_line_col(src, i)
            comments.append((line, col, src[i:j]))
            i = j
            continue

        if c == "/" and i + 1 < n and src[i + 1] == "*":
            j = i + 2
            depth = 1
            while j < n and depth > 0:
                if src[j] == "/" and j + 1 < n and src[j + 1] == "*":
                    depth += 1
                    j += 2
                elif src[j] == "*" and j + 1 < n and src[j + 1] == "/":
                    depth -= 1
                    j += 2
                else:
                    j += 1
            line, col = pos_to_line_col(src, i)
            comments.append((line, col, src[i:j]))
            i = j
            continue

        if c == '"' or (c == "b" and i + 1 < n and src[i + 1] == '"'):
            if c == "b":
                i += 1
            i += 1
            while i < n and src[i] != '"':
                if src[i] == "\\":
                    i += 2
                else:
                    i += 1
            if i < n:
                i += 1
            continue

        if c == "'" or (c == "b" and i + 1 < n and src[i + 1] == "'"):
            if c == "b":
                i += 1
            i += 1
            if i < n:
                if src[i].isalpha() or src[i] == "_":
                    if i + 1 < n and src[i + 1] == "'":
                        i += 2
                        continue
                    while i < n and (src[i].isalnum() or src[i] == "_"):
                        i += 1
                    continue
                while i < n and src[i] != "'":
                    if src[i] == "\\":
                        i += 2
                    else:
                        i += 1
                if i < n:
                    i += 1
                continue
            continue

        if c == "r" or (c == "b" and i + 1 < n and src[i + 1] == "r"):
            start = i
            if c == "b":
                i += 1
            i += 1
            hash_count = 0
            while i < n and src[i] == "#":
                hash_count += 1
                i += 1
            if i < n and src[i] == '"':
                i += 1
                while i < n:
                    if src[i] == '"':
                        k = i + 1
                        matched_hashes = 0
                        while (
                            k < n
                            and src[k] == "#"
                            and matched_hashes < hash_count
                        ):
                            matched_hashes += 1
                            k += 1
                        if matched_hashes == hash_count:
                            i = k
                            break
                    i += 1
                continue
            i = start + 1
            continue

        i += 1

    return comments


def find_inline_code_spans(line: str) -> list[tuple[int, int]]:
    """Return ``(start, end)`` column ranges of inline code spans in ``line``.

    Identical to ``check-issue-refs.py``'s span finder: a backtick run of N
    opens a span, closed by the next backtick run of exactly N.
    """
    spans: list[tuple[int, int]] = []
    i = 0
    n = len(line)

    while i < n:
        if line[i] != "`":
            i += 1
            continue
        j = i
        while j < n and line[j] == "`":
            j += 1
        run_len = j - i
        k = j
        while k < n:
            if line[k] != "`":
                k += 1
                continue
            m = k
            while m < n and line[m] == "`":
                m += 1
            if m - k == run_len:
                spans.append((i, m))
                i = m
                break
            k = m
        else:
            i = j

    return spans


def is_identifier_char(c: str) -> bool:
    """Whether ``c`` can continue an identifier: ASCII letters and digits only.

    Not ``str.isalnum``. That predicate is Unicode-aware — ``'本'.isalnum()`` is
    ``True`` — so a bare ``purrdf`` glued to CJK prose (``本purrdf项目``) read
    as an identifier fragment and passed silently, while ``本 PurRDF 项目``
    one space away was scanned. Every identifier this repository mints (crate
    names, module paths, the binary, package names) is ASCII, so ASCII is the
    exact boundary: a non-ASCII neighbour is prose.
    """
    return c.isascii() and c.isalnum()


def is_bare(text: str, start: int, end: int) -> bool:
    """Whether ``text[start:end]`` (a ``purrdf`` match) is a bare prose word.

    Not bare — i.e. an identifier fragment, not prose — if a ``-``, ``_``,
    ``:``, ``/``, ``.``, or backtick sits immediately before or after it, or if
    an ASCII-alphanumeric character does (see ``is_identifier_char``): that
    shape is a longer identifier (``libpurrdf``) or a plural/suffixed form
    (``purrdfs``), not the bare word. A CJK neighbour is prose, so
    ``本purrdf项目`` IS bare.
    """
    before = text[start - 1] if start > 0 else " "
    after = text[end] if end < len(text) else " "
    if before in _ADJACENT_IDENTIFIER_CHARS or is_identifier_char(before):
        return False
    if after in _ADJACENT_IDENTIFIER_CHARS or is_identifier_char(after):
        return False
    return True


def scan_rust(path: Path) -> list[tuple[int, int, str]]:
    """Return ``(line, col, snippet)`` bare-``purrdf`` hits in a Rust file."""
    return scan_rust_text(path.read_text(encoding="utf-8"))


def scan_rust_text(src: str) -> list[tuple[int, int, str]]:
    """[`scan_rust`] over TEXT — what the self-test calls, so a case is
    measured against the scanner that ships rather than a copy of it."""
    hits: list[tuple[int, int, str]] = []
    for start_line, start_col, text in rust_comments(src):
        for m in BRAND_RE.finditer(text):
            if not is_bare(text, m.start(), m.end()):
                continue
            rel_line = text.count("\n", 0, m.start()) + 1
            last_nl = text.rfind("\n", 0, m.start())
            rel_col = m.start() - last_nl
            line = start_line + rel_line - 1
            col = start_col + rel_col - 1 if rel_line == 1 else rel_col
            snippet = text.split("\n")[rel_line - 1].strip()
            hits.append((line, col, snippet))
    hits.sort()
    return hits


def scan_markdown(path: Path) -> list[tuple[int, int, str]]:
    """Return ``(line, col, snippet)`` bare-``purrdf`` hits in a Markdown file."""
    return scan_markdown_text(path.read_text(encoding="utf-8"))


def scan_markdown_text(src: str) -> list[tuple[int, int, str]]:
    """[`scan_markdown`] over TEXT — see [`scan_rust_text`]."""
    hits: list[tuple[int, int, str]] = []

    in_fence = False
    for line_no, line in enumerate(src.splitlines(), start=1):
        stripped = line.lstrip()
        if re.match(r"(?:```+|~~~+)", stripped):
            in_fence = not in_fence
            continue
        if in_fence:
            continue

        spans = find_inline_code_spans(line)
        for m in BRAND_RE.finditer(line):
            start, end = m.start(), m.end()
            if any(s <= start < e for s, e in spans):
                continue
            if not is_bare(line, start, end):
                continue
            hits.append((line_no, start + 1, line.strip()))

    return hits


# ── This gate's own falsifiability ────────────────────────────────────────────
#
# ``(what, suffix, text, expected hit count)``. Every case is scanned by the
# same function the tree scan uses. The CJK pairs are the point: a tightening
# is a refusal, and a refusal is a claim — so the glued shape that must now be
# FLAGGED sits beside the spaced, code-span and identifier neighbours that
# must still PASS, and both halves are asserted on every run.
_CASES: tuple[tuple[str, str, str, int], ...] = (
    # Must be flagged: bare prose, with and without CJK neighbours.
    ("bare 'purrdf' glued to CJK on both sides", ".md", "本purrdf项目\n", 1),
    (
        "bare 'purrdf' glued to CJK, full-width comma after",
        ".md",
        "本工具包名为purrdf，用于处理RDF。\n",
        1,
    ),
    ("bare 'purrdf' before full-width punctuation", ".md", "purrdf。它是一个工具包。\n", 1),
    ("bare 'purrdf' in English prose", ".md", "the purrdf toolkit\n", 1),
    ("bare 'purrdf' glued to CJK in a Rust comment", ".rs", "// 本purrdf项目\nfn f() {}\n", 1),
    ("bare 'purrdf' glued to CJK in a Rust doc comment", ".rs", "/// 本purrdf项目\nfn f() {}\n", 1),
    # Must pass: the capitalised brand, code spans, identifiers, fences.
    ("the brand, house typography (half-width spaces)", ".md", "本 PurRDF 项目\n", 0),
    ("the brand glued to CJK", ".md", "PurRDF工具包支持SPARQL。\n", 0),
    ("the facade crate in a code span beside CJK", ".md", "`purrdf` 门面 crate\n", 0),
    ("a crate name beside CJK", ".md", "purrdf-core 是内核\n", 0),
    ("a longer identifier and a plural", ".md", "libpurrdf and purrdfs\n", 0),
    ("a Rust path and a module path", ".md", "purrdf::Dataset and purrdf_core\n", 0),
    ("the crate name inside a fenced block", ".md", "```\npurrdf\n```\n", 0),
    ("a string literal in Rust is not a comment", ".rs", 'const N: &str = "purrdf";\n', 0),
)


def self_test(report: bool) -> list[str]:
    """Every case the scanners answer wrongly. An empty list is the only pass."""
    problems: list[str] = []
    for what, suffix, text, expected in _CASES:
        hits = scan_rust_text(text) if suffix == ".rs" else scan_markdown_text(text)
        ok = len(hits) == expected
        if report:
            verdict = "flagged" if hits else "passes "
            print(f"  {'ok' if ok else 'WRONG':5}  {verdict}  {suffix}: {what}")
        if ok:
            continue
        if expected:
            problems.append(
                f"  • {suffix}: {what} is NOT flagged — exactly the bare prose this "
                "gate exists to reject"
            )
        else:
            problems.append(
                f"  • {suffix}: {what} is FLAGGED ({hits}) — a lint that fires on an "
                "identifier or the brand itself is a lint that gets switched off"
            )
    return problems


def iter_rendered_paths(tree: Path) -> Iterator[Path]:
    """Every ``.md`` under a rendered book tree, in a deterministic order."""
    yield from sorted(p for p in tree.rglob("*.md") if p.is_file())


def scan_rendered_tree(tree: Path) -> int:
    """Scan a rendered book tree (see the module doc). Returns the exit code.

    Every hit is reported. A file is spared only up to the count its SOURCE
    page carries in ``PRE_EXISTING_BRAND_CASING`` — a ceiling, since a
    translation that drops a bare mention improved on its source.
    """
    registered_counts = dict(PRE_EXISTING_BRAND_CASING)
    offenders: list[tuple[str, int, int, str]] = []
    scanned = 0
    for path in iter_rendered_paths(tree):
        scanned += 1
        rel = path.relative_to(tree).as_posix()
        found = scan_markdown(path)
        if not found:
            continue
        ceiling = registered_counts.get(RENDERED_SOURCE_PREFIX + rel, 0)
        if len(found) <= ceiling:
            continue
        for line, col, snippet in found:
            offenders.append((rel, line, col, snippet))
    if scanned == 0:
        print(
            f"check-brand-casing: no .md file under {tree} — a rendered tree "
            "with nothing in it is a vacuous pass, not a clean one",
            file=sys.stderr,
        )
        return 1
    for rel, line, col, snippet in offenders:
        print(
            f"{tree / rel}:{line}:{col}: bare 'purrdf' in prose — use 'PurRDF' "
            f"for the project/brand, or wrap `purrdf` in backticks if it names "
            f"the facade crate"
        )
        print(f"    {snippet}")
    if offenders:
        return 1
    print(
        f"OK: no bare 'purrdf' prose in the rendered tree ({scanned} page(s) "
        f"scanned under {tree})."
    )
    return 0


def main(argv: list[str]) -> int:
    rendered: Path | None = None
    alone = False
    args = list(argv[1:])
    while args:
        arg = args.pop(0)
        if arg == "--self-test":
            alone = True
        elif arg == "--rendered-tree" and args:
            rendered = Path(args.pop(0))
        else:
            print(
                f"usage: {Path(argv[0]).name} [--self-test] [--rendered-tree DIR]",
                file=sys.stderr,
            )
            return 2

    if alone:
        print("check-brand-casing: proving the bare-word rule bites, and only bites —")
    # BEFORE the scan, on every run (pure text over a dozen strings): a gate
    # that cannot see the shape it rejects prints OK over exactly that shape.
    blind = self_test(report=alone)
    if blind:
        print(
            "check-brand-casing: this gate answers its own cases wrongly:\n"
            + "\n".join(blind)
            + "\n\nFix the scan, not the case.",
            file=sys.stderr,
        )
        return 1
    if alone:
        print(f"OK: all {len(_CASES)} shapes are flagged or spared as written.")
        return 0

    if rendered is not None:
        if not rendered.is_dir():
            print(f"check-brand-casing: {rendered} is not a directory", file=sys.stderr)
            return 2
        return scan_rendered_tree(rendered)

    root = repo_root()
    registered_counts = dict(PRE_EXISTING_BRAND_CASING)

    offenders: list[tuple[Path, int, int, str]] = []
    mismatches: list[tuple[str, int, int]] = []  # (rel, registered, live)
    live_files: set[str] = set()

    for path in iter_scan_paths(root):
        rel = str(path.relative_to(root))
        found = scan_rust(path) if path.suffix == ".rs" else scan_markdown(path)
        if not found:
            continue
        live_files.add(rel)
        registered = registered_counts.get(rel)
        if registered is None:
            for line, col, snippet in found:
                offenders.append((path, line, col, snippet))
        elif len(found) != registered:
            mismatches.append((rel, registered, len(found)))
            for line, col, snippet in found:
                offenders.append((path, line, col, snippet))

    stale = sorted(set(registered_counts) - live_files)

    if offenders:
        for path, line, col, snippet in offenders:
            print(
                f"{path.relative_to(root)}:{line}:{col}: bare 'purrdf' in prose "
                f"— use 'PurRDF' for the project/brand, or wrap `purrdf` in "
                f"backticks if it names the facade crate"
            )
            print(f"    {snippet}")
    for rel, registered, live in mismatches:
        direction = "more" if live > registered else "fewer"
        print(
            f"scripts/check-brand-casing.py: PRE_EXISTING_BRAND_CASING lists "
            f"{registered} bare 'purrdf' occurrence(s) for {rel!r}, but {live} "
            f"{direction} were found live — update the entry's count to match "
            f"(the register may only shrink, never grow, from an edit)."
        )
    for entry_path in stale:
        print(
            f"scripts/check-brand-casing.py: PRE_EXISTING_BRAND_CASING still "
            f"lists {entry_path!r}, which no longer has a bare 'purrdf' — "
            f"delete the entry so the register keeps shrinking."
        )

    if offenders or stale:
        return 1

    print(
        f"OK: no bare 'purrdf' prose outside identifiers/code spans "
        f"({len(PRE_EXISTING_BRAND_CASING)} pre-existing file(s) registered)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
