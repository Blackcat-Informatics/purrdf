# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Reject development-process references in PurRDF comments and docs.

Five token families are rejected, over exactly the same scanned surface.

**Issue references** — ``#NNN``. Once an issue is closed the token becomes stale
and misleading, so we do not allow new ones.

**Process references** — ``Task 28``, ``EPIC``, ``this branch``, a phrase
that locates something in the repository's own history (``on origin/main``,
`` on `origin/main` ``), a possessive or section-anchored reference to an
external planning document (``the plan's``, ``the plan §``), and a standalone
acceptance-criterion label (``AC1``). These name the *development effort*
that produced the code rather than the code itself. They go stale the moment
the branch merges: "this branch" has no referent in a merged history,
"Pre-existing on `origin/main`" answers a question only the reviewer who
wrote it was asking, "verbatim from the plan's §6.5" sends a reader to a
document that never shipped, and a reader who meets "task 29" or "AC1" in a
test file cannot look either up. ``#NNN`` was banned for exactly this reason
and these are the same debt spelled differently — which is why ``// ---
task 28: the reasoner façade ---`` sailed through a lint that was already
meant to stop it.

**Hazard / finding labels** — a bare ``H12``, or an ``F6``/``N3``-shaped token
used AS A LABEL (``F6:`` at the start of a clause, or wrapped alone in
parentheses, ``(F1)``). These are identifiers from a review thread — meaningful
only to the reviewer who assigned them, not to the codebase. A bare ``H<N>`` is
banned outright (no legitimate first-party use collides with that shape); an
``F<N>``/``N<N>`` shape is only banned in the specific label positions above,
because those two letters are also legitimate technical vocabulary elsewhere
(``F32``/``F64`` float widths, ``N3`` the RDF serialization, ``N802`` a linter
code) that must not be flagged.

**Development gap/remediation tags** — a bare ``G12:``/``R9:`` label opening a
clause (mirroring the hazard-label shape above but for the ``G<N>``/``R<N>``
letters), the collocation ``gap R9`` (a gap-plan item cited by number), the
collocation ``gap G4``/``Gap G5``/``GAP G4`` (case-insensitive "gap" followed
by a ``G<N>`` id), and ``G12 regression`` (a regression test named after the
gap item it guards). These are session-ephemeral gap-analysis tags — ids from
a review pass's internal numbering, meaningful only to the pass that assigned
them — not stable identifiers a reader can look up once the pass that minted
them is gone. Unlike the hazard letters above, ``G``/``R`` collide with real
technical vocabulary constantly (a query plan's cost `` G ``, `` R1``/``R2``
graph names, register/generation names), so the shape is deliberately narrow:
only a label-opening colon, an explicit "gap"/"regression" collocation, or the
literal ``gap R<N>`` phrase trips it — a bare ``G1`` or ``R2`` used as an
ordinary identifier, and a formal spec-clause cross-reference such as
``crates/rdf-core/src/ir/paged/mod.rs``'s ``G1``/``G3`` (which name clauses
defined in ``docs/design/purrdf-backend-contract.md``, not review-pass items),
are both left alone — the latter via ``AMBIGUOUS_GAP_CLAUSE_FILES`` below.

**Issue-normative spelling** — the phrase "issue-normative", which describes a
grammar choice by pointing at the tracker discussion that settled it rather
than by describing the choice itself.

Process, hazard-label, and gap-tag references carry four frozen registers,
all of which may only SHRINK:

* ``PRE_EXISTING_PROCESS_REFERENCES`` — the debt that predates this rule, one
  entry per ``(file, token)``. A live occurrence with no entry is a hard failure,
  and an entry with no live occurrence is *also* a hard failure, so paying the
  debt down forces the register to be trimmed rather than left to rot.
* ``AMBIGUOUS_BRANCH_PHRASES`` — the files where "this branch" means a
  *control-flow* arm and not a git branch. English overloads the word; these
  places are named so the ban can stay absolute. New code should say "this arm",
  "this case", or "this match arm", which is clearer prose regardless.
* ``AMBIGUOUS_PLAN_PHRASES`` — the files where "the plan's"/"the plan §" names
  a runtime query, compaction, or execution *plan value* (``the plan's
  pre-order``, ``the plan's transform chain``) and not a development-planning
  document. "Plan" is both a domain noun in this codebase and the word the
  banned phrase uses, so these places are named so the ban can stay absolute
  without flagging every doc comment that talks about a query plan's shape.
* ``AMBIGUOUS_GAP_CLAUSE_FILES`` — the files where a bare ``G<N>:`` names a
  formal clause of a shipped design contract (e.g. ``docs/design/purrdf-
  backend-contract.md``'s numbered ``G0``-``G9`` clauses) rather than a
  review-pass gap-tracking id. The clause and the tag are lexically identical;
  only the file's own cross-reference to the contract document distinguishes
  them, so these places are named so the ban can stay absolute.

This lint scans:

* ``.rs`` files under ``crates/`` and ``bindings/`` — only Rust comments are
  examined. A small Rust-aware lexer skips string, character, and raw-string
  literals so ``//`` inside ``"http://example.org"`` is not treated as a
  comment.
* ``.md`` files under ``crates/``, ``bindings/``, ``docs/``, and root ``*.md``
  files — markdown header anchors (``#101-...``), hex color codes, inline code,
  and fenced code blocks are excluded.
* ``.toml`` files under ``crates/``, ``bindings/``, ``docs/``, and root
  ``*.toml`` files — manifest ``description`` fields and dependency comments are
  scanned line by line; hex colors are excluded by the token pattern itself.
* ``.py`` files under ``scripts/``, ``crates/``, and ``bindings/`` — both ``#``
  line comments and documentation strings are examined. A small Python-aware
  lexer skips string and docstring literals (including ``r``/``b``/``u``/``f``
  prefixes and triple-quoted strings) for the ``#``-comment pass, exactly as the
  Rust scan skips string literals; an ``ast`` walk then scans every
  module/class/function docstring as prose (mirroring the ``.md``/``.toml``
  scans), so an issue reference hiding in a docstring is caught while an
  issue-shaped token inside an ordinary string literal is not flagged. The
  checker's own file is excluded so its detection examples are not matched.
* ``.yaml``/``.yml`` workflow files under ``.github/`` — only ``#`` comments are
  examined. A ``#`` is a comment only at line start or after whitespace and only
  when outside a quoted scalar (matching YAML's own comment rule), so a ``#``
  inside a quoted string is treated as data.

The issue token pattern is ``#`` followed by 1–5 decimal digits that is not
followed by another digit, a hex letter, a hyphen, or a decimal fraction
(so ``#3.1`` section numbers are not flagged). This avoids 6-digit hex colors
and markdown anchors while still catching references like ``#16`` or ``#123``.

The process token patterns are ``Task``/``task`` followed by an optional ``#``
and a number, the bare uppercase acronym ``EPIC`` (so ``EPIC #906`` and
``(EPIC \\`text_parse\\`)`` are both caught, while the ordinary English word
"epic" in a changelog entry is not), the phrase "this branch" in any case,
"on origin/main" / "on `origin/main`" (release documentation that says a
branch is *synchronized with* `origin/main` is unaffected — only the "on"
collocation that locates a piece of code in history is banned), "the plan's"
(the apostrophe matches either the ASCII ``'`` or the typographic U+2019
``’`` form, since a prose editor or pasted review comment may use either)
/ "the plan §" (a possessive or section-anchored reference to an external
planning document — matched literally, since every occurrence in this
repository's history has been lowercase mid-sentence prose), and a standalone
acceptance-criterion label ``AC`` followed by exactly one digit (``AC1``);
``AC12`` is two digits and does not match, since the shape is specifically the
single-digit labels this repository's planning documents have used.

The hazard-label token patterns are a bare ``H`` followed by 1-3 digits
(``H12``), and an ``F``/``N`` followed by 1-3 digits either immediately
followed by a colon (``F6:``) or wrapped alone in parentheses (``(F1)``,
``(N3)``) — the two shapes every real finding label in this repository's
history has taken. The narrower ``F``/``N`` shape leaves every other use of
those letters (a float width, a serialization name, a linter code) alone.

The gap-tag token patterns are, all case-insensitive on the word "gap": the
literal collocation ``gap R`` followed by 1-2 digits (``gap R9``), the
collocation ``gap G`` followed by 1-2 digits (``gap G4``, ``Gap G5``,
``GAP G4``), a bare ``G`` followed by 1-2 digits and the word ``regression``
(``G12 regression``), and a bare ``G`` or ``R`` followed by 1-2 digits
immediately followed by a colon (``G12:``, ``R9:``) — the same label shape
the hazard tokens use, applied to the two letters a gap-analysis pass names
its own items and remediation-regression tests with. Every one of these
shapes requires either the literal word "gap"/"regression" alongside the
digits, or the label-opening colon; a bare ``G1``/``R2`` used as an ordinary
identifier (a loop variable, a graph name, a generation counter) triggers
none of them.

The "issue-normative" pattern is that literal phrase, case-sensitive, since it
has exactly one spelling in this repository's history and any capitalized
variant would already read as a proper noun rather than as this phrase.
"""

from __future__ import annotations

import ast
import re
import subprocess
from collections.abc import Iterator
from pathlib import Path

ISSUE_PATTERN = r"#\d{1,5}(?![\dA-Fa-f-])(?!\.\d)"

# Every rejected token family in ONE pattern, so a file is lexed once no matter
# how many families there are. ``match.lastgroup`` names the family, which is what
# separates an issue reference from a process reference in the report and what
# keeps the inline-code exclusion applying to the former alone.
TOKEN_RE = re.compile(
    rf"(?P<issue>{ISSUE_PATTERN})"
    r"|(?P<task>\b[Tt]ask\s+#?\d+\b)"
    r"|(?P<epic>\bEPIC\b)"
    r"|(?P<branch>(?i:\bthis\ branch\b))"
    r"|(?P<history_ref>\bon\s+`?origin/main`?\b)"
    r"|(?P<issue_normative>\bissue-normative\b)"
    r"|(?P<hazard>\bH\d{1,3}\b)"
    r"|(?P<hazard_label>\b[FN]\d{1,3}:|\([FHN]\d{1,3}\))"
    r"|(?P<plan_ref>\bthe plan(?:[\'’]s\b|\s+§))"
    r"|(?P<ac_label>\bAC\d\b)"
    r"|(?P<gap_tag>(?i:gap)\s+R\d{1,2}\b"
    r"|(?i:gap)\s+G\d{1,2}\b"
    r"|\bG\d{1,2}\s+regression\b"
    r"|\bG\d{1,2}:"
    r"|\bR\d{1,2}:)"
)

# The prose fix each process family's message suggests.
PROCESS_REMEDY: dict[str, str] = {
    "task": "name what the code does, not the work item that produced it",
    "epic": "name the capability, not the work item that produced it",
    "branch": (
        'say "this arm" / "this case" for a control-flow branch, and state the '
        "constraint itself rather than the branch that imposed it"
    ),
    "history_ref": (
        "state the constraint or behaviour itself rather than where in the "
        "repository's history it was introduced, fixed, or scoped"
    ),
    "issue_normative": (
        "restate as the grammar/behaviour choice itself, with no reference to the "
        "review thread that settled it"
    ),
    "hazard": "restate as the behaviour it describes, with no review-thread hazard id",
    "hazard_label": (
        "restate as the behaviour it describes, with no review-thread finding label"
    ),
    "plan_ref": (
        "describe what the code/table/text itself does or covers, not a planning "
        "document's section number or item count"
    ),
    "ac_label": (
        "name the requirement itself, not the acceptance-criterion label that "
        "tracked it"
    ),
    "gap_tag": (
        "restate as the technical fact the code/test establishes, with no "
        "gap-analysis-pass id or regression-tag number"
    ),
}

# Process and hazard-label references that predate this rule, as ``(path,
# matched token)``. Every occurrence of that token in that file is covered by
# one entry. THIS REGISTER MAY ONLY SHRINK: an entry whose token no longer
# appears in its file is reported as stale, so paying a debt down forces the
# line to be deleted here rather than leaving a permanent licence behind.
PRE_EXISTING_PROCESS_REFERENCES: frozenset[tuple[str, str]] = frozenset(
    {
        ("bindings/python/src/py_gts.rs", "Task 8"),
        ("bindings/python/src/rdf.rs", "Task 8"),
        ("bindings/python/src/rdf.rs", "Task 9"),
        ("crates/gts/src/compact.rs", "Task 6"),
        ("crates/gts/tests/compaction_signatures.rs", "Task 4"),
        ("crates/rdf-core/benches/ir_layout.rs", "Task 7"),
        ("crates/rdf-core/src/diagnostic.rs", "Task 12"),
        ("crates/rdf-core/src/ir/global.rs", "Task 4"),
        ("crates/rdf-core/src/sssom.rs", "Task 7"),
        ("crates/rdf-core/tests/paged_backend.rs", "(F1)"),
        ("crates/rdf-wasm/src/dataset.rs", "Task 5"),
        ("crates/rdf-wasm/src/factory.rs", "Task 5"),
        ("crates/rdf/src/gts.rs", "Task 4"),
        ("crates/rdf/src/gts_certify.rs", "Task 5"),
        ("crates/rdf/src/native_codecs/mod.rs", "EPIC"),
        ("crates/rdf/src/native_codecs/mod.rs", "Task 1"),
        ("crates/rdf/src/turtle_normalize.rs", "Task 5"),
        ("crates/rdf/tests/gts_authorship_census.rs", "this branch"),
        ("crates/rdf/tests/gts_certify.rs", "Task 5"),
        ("crates/rdf/tests/gts_certify.rs", "Task 6"),
        ("crates/rdf/tests/gts_certify.rs", "the plan's"),
        ("crates/shapes/src/instance.rs", "Task 6"),
        ("crates/shapes/src/json_schema.rs", "F6:"),
        ("crates/shapes/src/json_schema.rs", "Task 3"),
        ("crates/shapes/src/json_schema.rs", "Task 4"),
        ("crates/shapes/src/json_schema.rs", "Task 6"),
        ("crates/shapes/src/shapes.rs", "Task 2"),
        ("crates/shapes/tests/rules_conformance.rs", "Task 6"),
        ("crates/sparql-conformance/tests/owl2_rl_conformance.rs", "the plan's"),
        ("crates/sparql-eval/src/parallel_determinism_gate.rs", "Task 7"),
        ("crates/sparql-eval/src/stat_agg.rs", "the plan's"),
        ("bindings/python/src/py_gts.rs", "gap G4"),
        ("bindings/python/src/py_slice.rs", "gap G5"),
        ("crates/gts/tests/event_bridge.rs", "R9:"),
        ("crates/rdf/tests/gts_certify.rs", "GAP G2"),
        ("crates/rdf/tests/gts_certify.rs", "GAP G4"),
        ("crates/rdf/tests/gts_certify.rs", "GAP G6"),
        ("crates/shapes/src/json_schema.rs", "Gap G5"),
        ("crates/shapes/src/json_schema.rs", "R3:"),
        ("crates/shapes/src/json_schema.rs", "R4:"),
        ("crates/shapes/src/json_schema.rs", "R5:"),
        ("crates/shapes/src/json_schema.rs", "R7:"),
        ("crates/slice/src/catalog.rs", "G8:"),
        ("crates/slice/tests/ownership_tests.rs", "G12:"),
        ("crates/slice/tests/ownership_tests.rs", "G13:"),
        ("crates/slice/tests/ownership_tests.rs", "G14:"),
        ("crates/sparql-algebra/src/lexer.rs", "G1 regression"),
        ("crates/sparql-algebra/src/lexer.rs", "G2 regression"),
        ("crates/sparql-algebra/src/parser.rs", "G3 regression"),
        ("crates/sparql-algebra/src/parser.rs", "gap G4"),
    }
)

# Files where "this branch" denotes a CONTROL-FLOW arm — an `if`/`match`
# alternative — and not a git branch. The phrase is banned outright rather than
# guessed at, because English gives no reliable signal: "it only appears on this
# branch" is a match arm and "it moved on this branch" is a work item, and both
# read identically to a regex. Naming the code-sense sites keeps the ban absolute
# while costing nothing in precision. Like the register above, this may only
# shrink: an entry whose file no longer says "this branch" is reported as stale.
AMBIGUOUS_BRANCH_PHRASES: frozenset[str] = frozenset(
    {
        "bindings/python/python/src/purrdf/compat/rdflib/term.py",
        "bindings/python/tests/test_entail_reasoning.py",
        "crates/gts/tests/replication_diff.rs",
        "crates/rdf-core/src/dataset_view.rs",
        "crates/rdf-core/src/turtle_render.rs",
        "crates/validate/src/regime.rs",
    }
)

# Files where "the plan's"/"the plan §" denotes a runtime QUERY, COMPACTION, or
# EXECUTION plan VALUE — "the plan's pre-order", "the plan's transform chain" —
# and not a development-planning document. The phrase is banned outright rather
# than guessed at, because English gives no reliable signal: "arrived at through
# the plan's soundness certificate" reads identically whether "plan" is a query
# plan or a design document, and both shapes occur in this codebase. Naming the
# code-sense sites keeps the ban absolute while costing nothing in precision.
# Like the registers above, this may only shrink: an entry whose file no longer
# says "the plan's"/"the plan §" is reported as stale.
AMBIGUOUS_PLAN_PHRASES: frozenset[str] = frozenset(
    {
        "crates/datalog/src/seminaive.rs",
        "crates/gts/src/compact.rs",
        "crates/gts/tests/pinned_dict_compaction.rs",
        "crates/rdf-core/src/ir/dataset.rs",
        "crates/rdf/tests/dict_vectors.rs",
        "crates/sparql-eval/src/governor/ledger.rs",
        "crates/sparql-eval/src/property_fn.rs",
        "crates/sparql-eval/tests/governed_query.rs",
    }
)

# Files where a bare ``G<N>:`` cross-references a formal clause of
# ``docs/design/purrdf-backend-contract.md`` (whose ``### G0`` .. ``### G9``
# headings define durable, shipped identity-and-generation rules for the paged
# layer) rather than naming a review-pass gap-tracking item. The label shape is
# identical in both uses — only the topic each site's ``G<N>:`` comment
# describes (matching the contract clause's own title) tells them apart, so
# these places are named so the ban can stay absolute. Like the registers
# above, this may only shrink: an entry whose file no longer says ``G<N>:`` is
# reported as stale.
AMBIGUOUS_GAP_CLAUSE_FILES: frozenset[str] = frozenset(
    {
        "crates/rdf-core/src/ir/paged/mod.rs",
    }
)

SCAN_DIRS = ("crates", "bindings", "docs")

# Directories whose ``.py`` files are first-party enough to lint. ``scripts/``
# is not in ``SCAN_DIRS`` (which governs ``.rs``/``.md``/``.toml``) but is the
# home of the maintenance scripts this lint most wants to cover.
PY_SCAN_DIRS = (*SCAN_DIRS, "scripts")

# Valid Python string-literal prefixes (case-insensitive) that may precede an
# opening quote. ``u`` never combines; ``r`` combines with ``b``/``f``.
PY_STRING_PREFIXES = frozenset({"r", "b", "u", "f", "rb", "br", "rf", "fr"})

# This checker's own path. Its module/function docstrings and strings carry
# issue-number-shaped *example* tokens (e.g. ``#16``, ``#123``) that document
# what the lint detects; scanning them would flag the documentation of the lint
# itself. Exclude the checker from the scan so its self-documentation examples
# are never matched, without weakening detection anywhere else.
SELF_PATH = Path(__file__).resolve()


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def iter_scan_paths(root: Path) -> Iterator[Path]:
    """Yield every tracked source file the lint enforces.

    Covered: ``.rs``/``.md``/``.toml`` under ``crates``/``bindings``/``docs``
    (plus root ``.md``/``.toml``), ``.py`` under those dirs and ``scripts``, and
    ``.yaml``/``.yml`` GitHub workflow files under ``.github``.

    Enumeration is driven by ``git ls-files`` rather than a filesystem walk so
    the scan covers exactly the committed first-party source. Untracked build
    artifacts and third-party trees (``bindings/python/.venv`` linkml docs,
    ``target/``) are never scanned, keeping the lint deterministic and free of
    "green in CI, red locally" divergence.
    """
    out = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    for rel in sorted(part for part in out.split("\0") if part):
        suffix = Path(rel).suffix
        if suffix not in (".rs", ".md", ".toml", ".py", ".pyi", ".rq", ".yaml", ".yml"):
            continue
        segments = rel.split("/")
        top = segments[0]
        if suffix in (".rs", ".md", ".toml"):
            in_scan_dir = top in SCAN_DIRS
            root_file = len(segments) == 1 and (
                rel.endswith(".md") or rel.endswith(".toml")
            )
            if not (in_scan_dir or root_file):
                continue
        elif suffix == ".rq":
            # SPARQL is SHIPPED text wherever it lives: a tracker token in a committed
            # query is published exactly as one in a printed string is, and six of these
            # carried one while every gate reported clean. `generated/` holds emitted
            # artifacts, `queries/` first-party ones; both are read by users.
            if top not in ("generated", "queries"):
                continue
        elif suffix in (".py", ".pyi"):
            # `.pyi` is SHIPPED: it is the PEP 561 stub inside every published wheel,
            # so a process reference in it is published to PyPI. It was outside this
            # scan while `.py` was inside, which is how eight of them accumulated in
            # the one file a typing consumer reads most.
            if top not in PY_SCAN_DIRS:
                continue
        else:  # .yaml / .yml
            if top != ".github":
                continue
        path = root / rel
        if path.resolve() == SELF_PATH:
            continue
        if path.is_file():
            yield path


def pos_to_line_col(src: str, pos: int) -> tuple[int, int]:
    """Convert a 0-based source index to 1-based line/column."""
    line = src.count("\n", 0, pos) + 1
    last_nl = src.rfind("\n", 0, pos)
    col = pos - last_nl
    return line, col


def snippet(text: str, start: int, end: int, window: int = 24) -> str:
    """Return a short snippet of ``text`` surrounding ``text[start:end]``."""
    prefix = text[max(0, start - window) : start].replace("\n", " ")
    matched = text[start:end]
    suffix = text[end : min(len(text), end + window)].replace("\n", " ")
    return f"{prefix}{matched}{suffix}".strip()


def rust_comments(src: str) -> list[tuple[int, int, str]]:
    """Extract Rust comments as ``(start_line, start_col, comment_text)``."""
    return rust_comments_and_literals(src)[0]


def rust_string_literals(src: str) -> list[tuple[int, int, str]]:
    """Extract Rust string-literal CONTENTS as ``(start_line, start_col, text)``.

    A literal is not a comment, and for most lints that is the end of it — but an
    issue-reference token inside one is not a note to a developer, it is text the
    program PRINTS or RETURNS. Three shipped surfaces carried such tokens while this
    lint reported clean, because the lexer that avoids reading ``//`` inside
    ``"http://…"`` as a comment discarded the literal instead of scanning it. They are
    scanned now; the URL exemption is unaffected, because it is the ``//`` that is
    exempt, not the ``#NNN``.
    """
    return rust_comments_and_literals(src)[1]


def rust_comments_and_literals(
    src: str,
) -> tuple[list[tuple[int, int, str]], list[tuple[int, int, str]]]:
    """One pass over Rust source, returning its comments and its string literals.

    The scanner is deliberately conservative: it only needs to avoid treating
    ``//`` or ``/*`` inside string/char/raw-string literals as comment
    starters. It understands line comments, block comments (including nested
    ones), byte/regular strings, byte/regular character literals, lifetimes,
    and raw strings with arbitrary hash counts.
    """
    comments: list[tuple[int, int, str]] = []
    literals: list[tuple[int, int, str]] = []
    n = len(src)
    i = 0
    # Seeded so a source whose first token of interest is a literal rather than a
    # comment still has a position to report. Every file in this tree opens with a
    # licence header, so the comment arm has always run first and bound these — but
    # that is a property of the corpus, not of the scanner, and a file without one
    # would otherwise abort the whole hygiene run.
    line, col = 1, 1

    while i < n:
        c = src[i]

        # Line comment: //, ///, //!
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            if j == -1:
                j = n
            line, col = pos_to_line_col(src, i)
            comments.append((line, col, src[i:j]))
            i = j
            continue

        # Block comment: /*, /**, /*!
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

        # String literal or byte string literal.
        if c == '"' or (c == "b" and i + 1 < n and src[i + 1] == '"'):
            if c == "b":
                i += 1
            i += 1  # skip opening quote
            start_line, start_col = line, col
            start = i
            while i < n and src[i] != '"':
                if src[i] == "\\":
                    i += 2
                else:
                    i += 1
            literals.append((start_line, start_col, src[start:i]))
            if i < n:
                i += 1  # skip closing quote
            continue

        # Character literal, byte character literal, or lifetime.
        if c == "'" or (c == "b" and i + 1 < n and src[i + 1] == "'"):
            if c == "b":
                i += 1
            i += 1  # skip opening quote
            if i < n:
                if src[i].isalpha() or src[i] == "_":
                    # Could be a lifetime or a single-character literal like 'a'.
                    if i + 1 < n and src[i + 1] == "'":
                        i += 2  # char literal
                        continue
                    # Lifetime: consume the identifier.
                    while i < n and (src[i].isalnum() or src[i] == "_"):
                        i += 1
                    continue
                # Char literal (possibly escaped).
                while i < n and src[i] != "'":
                    if src[i] == "\\":
                        i += 2
                    else:
                        i += 1
                if i < n:
                    i += 1  # skip closing quote
                continue
            continue

        # Raw string literal (possibly byte-prefixed).
        if c == "r" or (c == "b" and i + 1 < n and src[i + 1] == "r"):
            start = i
            if c == "b":
                i += 1
            i += 1  # skip 'r'
            hash_count = 0
            while i < n and src[i] == "#":
                hash_count += 1
                i += 1
            if i < n and src[i] == '"':
                i += 1  # skip opening quote
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
            # Not a raw string; resume scanning from just after the 'r'/'b'.
            i = start + 1
            continue

        i += 1

    return comments, literals


def is_rust_doc_comment(text: str) -> bool:
    """Return whether a Rust comment renders as Markdown (i.e. is a doc comment).

    Only doc comments are Markdown, so only they get inline-code-span exclusion.
    Outer/inner line docs are ``///`` and ``//!``; block docs are ``/**`` and
    ``/*!``. Rust treats ``////`` (four-plus slashes), ``/***`` and the empty
    ``/**/`` as *ordinary* comments, not docs — so those must NOT be excluded and
    are matched out here.
    """
    if text.startswith("///") and not text.startswith("////"):
        return True
    if text.startswith("//!"):
        return True
    if text.startswith("/*!"):
        return True
    if text.startswith("/**") and not text.startswith(("/***", "/**/")):
        return True
    return False


def scan_comments(
    comments: list[tuple[int, int, str]],
    *,
    exclude_inline_code: bool = False,
) -> list[tuple[int, int, str, str, str]]:
    """Scan extracted ``(start_line, start_col, text)`` comments for tokens.

    Shared by every comment-based scanner (Rust, Python, YAML): each comment
    carries the 1-based line/column of its first character, and match positions
    are translated back into absolute file coordinates.

    When ``exclude_inline_code`` is set, matches inside a Markdown inline-code
    span (backtick-delimited) are skipped **only for Rust doc comments**
    (``///``/``//!``/``/**``/``/*!``), which render as Markdown, mirroring
    ``scan_markdown``. In a doc comment an issue-shaped token inside a code span
    like ```term#3``` is a code literal — the exact output
    ``RdfLocation::display`` emits — not a stale issue reference. Ordinary
    ``//``/``/* */`` comments are NOT Markdown, so backticks carry no special
    meaning there and a ``#NNN`` token inside them is still flagged.
    """
    violations: list[tuple[int, int, str, str, str]] = []

    for start_line, start_col, text in comments:
        text_lines = text.split("\n")
        doc_comment = exclude_inline_code and is_rust_doc_comment(text)
        for match in TOKEN_RE.finditer(text):
            kind = match.lastgroup or "issue"
            offset = match.start()
            rel_line = text.count("\n", 0, offset) + 1
            last_nl = text.rfind("\n", 0, offset)
            rel_col = offset - last_nl
            if doc_comment and kind == "issue":
                col0 = rel_col - 1
                spans = find_inline_code_spans(text_lines[rel_line - 1])
                if any(s <= col0 < e for s, e in spans):
                    continue
            line = start_line + rel_line - 1
            col = start_col + rel_col - 1 if rel_line == 1 else rel_col
            violations.append(
                (line, col, match.group(), snippet(text, offset, match.end()), kind)
            )

    return violations


def literal_token_is_a_reference(text: str, token: str) -> bool:
    """Whether a ``#NNN`` inside a Rust string literal is an ISSUE reference.

    Two shapes are legitimately `#`-with-digits inside a string and are not tracker
    references, so scanning literals without distinguishing them would trade a real
    hole for false positives:

    * a format specifier — ``{value:#04x}``, ``{n:#b}`` — where the `#` is the
      alternate-form flag and the digits are a width;
    * an identifier or IRI fragment where the `#` is preceded by a word character,
      as in ``unit#7`` or ``origin-set#5``.

    A tracker reference stands as its own word: preceded by nothing, whitespace, or an
    opening delimiter. That is the only shape flagged here.
    """
    index = text.find(token)
    while index != -1:
        before = text[index - 1] if index > 0 else " "
        after_pos = index + len(token)
        after = text[after_pos] if after_pos < len(text) else " "
        standalone = not (before.isalnum() or before == "_")
        format_flag = after in "xXbBoOeE?}" or before == ":"
        if standalone and not format_flag:
            return True
        index = text.find(token, index + 1)
    return False


def scan_rust(path: Path) -> list[tuple[int, int, str, str, str]]:
    """Return violations found in a Rust source file.

    ``exclude_inline_code`` is requested, but ``scan_comments`` applies the
    inline-code-span exclusion only to Rust *doc* comments (which render as
    Markdown) — an issue-shaped token inside backticks in a doc comment is a code
    literal, not a stale issue reference. Ordinary ``//``/``/* */`` comments are
    not Markdown, so a ``#NNN`` inside backticks there is still flagged.
    """
    src = path.read_text(encoding="utf-8")
    comments, literals = rust_comments_and_literals(src)
    found = scan_comments(comments, exclude_inline_code=True)
    # String literals are scanned for ISSUE tokens only. A process reference like
    # "this branch" is ordinary prose a program may legitimately print, whereas a
    # `#NNN` in a printed or returned string publishes a tracker id to a caller —
    # which `.baseline` bans outright, and which three shipped surfaces carried while
    # this lint reported clean.
    found.extend(
        hit
        for hit in scan_comments(literals, exclude_inline_code=False)
        if hit[4] == "issue" and literal_token_is_a_reference(hit[3], hit[2])
    )
    found.sort(key=lambda hit: (hit[0], hit[1]))
    return found


def skip_py_string(src: str, i: int, n: int) -> int:
    """Return the index just past a Python string whose quote is at ``src[i]``.

    Handles triple- and single-quoted strings; a backslash escapes the next
    character for termination purposes in both raw and non-raw strings (a raw
    string still cannot be closed by an escaped quote), so raw/non-raw need no
    separate handling for the purpose of *skipping* the literal.
    """
    quote = src[i]
    if src[i : i + 3] == quote * 3:
        i += 3
        while i < n:
            if src[i] == "\\":
                i += 2
                continue
            if src[i : i + 3] == quote * 3:
                return i + 3
            i += 1
        return n
    i += 1  # skip opening quote
    while i < n:
        c = src[i]
        if c == "\\":
            i += 2
            continue
        if c == quote:
            return i + 1
        if c == "\n":
            return i  # unterminated single-line string
        i += 1
    return n


def python_comments(src: str) -> list[tuple[int, int, str]]:
    """Extract Python ``#`` comments as ``(start_line, start_col, text)``.

    String and docstring literals are skipped so a ``#NNN``-shaped token inside
    a string (or this module's own docstring examples) is never treated as a
    comment. Only ``#`` line comments are returned.
    """
    comments: list[tuple[int, int, str]] = []
    n = len(src)
    i = 0

    while i < n:
        c = src[i]

        # Line comment: everything from '#' to end of line.
        if c == "#":
            j = src.find("\n", i)
            if j == -1:
                j = n
            line, col = pos_to_line_col(src, i)
            comments.append((line, col, src[i:j]))
            i = j
            continue

        # Bare string literal.
        if c in "\"'":
            i = skip_py_string(src, i, n)
            continue

        # Identifier, possibly a string prefix (r"", b'', f"", rb"", ...).
        if c.isalpha() or c == "_":
            j = i
            while j < n and (src[j].isalnum() or src[j] == "_"):
                j += 1
            if (
                j < n
                and src[j] in "\"'"
                and src[i:j].lower() in PY_STRING_PREFIXES
            ):
                i = skip_py_string(src, j, n)
            else:
                i = j
            continue

        i += 1

    return comments


def python_docstrings(src: str) -> list[tuple[int, int, str]]:
    """Extract module/class/function docstrings as ``(line, col, text)``.

    The Python ``#``-comment lexer deliberately skips string and docstring
    literals, so docstring *prose* (module/class/function documentation) was an
    uncovered surface. Docstrings are documentation just like ``.md``/``.toml``
    prose and must be scanned the same way: an issue reference buried in a
    module docstring is exactly the stale-TODO debt this lint rejects.

    An ``ast`` walk locates every docstring — the first statement of a module,
    class, or (async) function body when it is a bare string literal — and the
    original source segment (quotes included) is returned so ``scan_comments``
    reports precise file coordinates. Only genuine documentation strings are
    returned; ordinary string expressions elsewhere in the code are not, so a
    ``#NNN``-shaped token inside a real string literal (e.g. a URL fragment) is
    still not treated as an issue reference.
    """
    try:
        tree = ast.parse(src)
    except SyntaxError:
        # A file that does not parse cannot carry a docstring we can trust;
        # the ``#``-comment lexer still covers it. Do not swallow other errors.
        return []

    docstrings: list[tuple[int, int, str]] = []
    for node in ast.walk(tree):
        if not isinstance(
            node,
            (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef),
        ):
            continue
        body = node.body
        if not body:
            continue
        first = body[0]
        if not (
            isinstance(first, ast.Expr)
            and isinstance(first.value, ast.Constant)
            and isinstance(first.value.value, str)
        ):
            continue
        segment = ast.get_source_segment(src, first.value)
        if segment is None:
            continue
        docstrings.append((first.value.lineno, first.value.col_offset + 1, segment))

    return docstrings


def scan_python(path: Path) -> list[tuple[int, int, str, str, str]]:
    """Return violations found in a Python source file.

    Both ``#`` line comments and documentation strings (module/class/function
    docstrings) are scanned; the docstring pass closes the prose gap that let
    an issue reference hide inside a module docstring.
    """
    src = path.read_text(encoding="utf-8")
    return scan_comments(python_comments(src)) + scan_comments(
        python_docstrings(src)
    )


def yaml_comments(src: str) -> list[tuple[int, int, str]]:
    """Extract YAML ``#`` comments as ``(start_line, start_col, text)``.

    A ``#`` opens a comment only at line start or after whitespace and only when
    it is not inside a quoted scalar. Single-quoted scalars escape a quote by
    doubling it (``''``); double-quoted scalars use backslash escapes.
    """
    comments: list[tuple[int, int, str]] = []

    for line_no, line in enumerate(src.splitlines(), start=1):
        n = len(line)
        i = 0
        quote: str | None = None
        while i < n:
            c = line[i]
            if quote == "'":
                if c == "'":
                    if i + 1 < n and line[i + 1] == "'":
                        i += 2  # escaped '' inside a single-quoted scalar
                        continue
                    quote = None
                i += 1
                continue
            if quote == '"':
                if c == "\\":
                    i += 2
                    continue
                if c == '"':
                    quote = None
                i += 1
                continue
            if c in "\"'":
                quote = c
                i += 1
                continue
            if c == "#" and (i == 0 or line[i - 1] in " \t"):
                comments.append((line_no, i + 1, line[i:]))
                break
            i += 1

    return comments


def scan_yaml(path: Path) -> list[tuple[int, int, str, str, str]]:
    """Return violations found in a YAML source file."""
    src = path.read_text(encoding="utf-8")
    return scan_comments(yaml_comments(src))


def find_inline_code_spans(line: str) -> list[tuple[int, int]]:
    """Return ``(start, end)`` column ranges of inline code spans in ``line``."""
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


def scan_markdown(path: Path) -> list[tuple[int, int, str, str, str]]:
    """Return violations found in a Markdown file."""
    src = path.read_text(encoding="utf-8")
    violations: list[tuple[int, int, str, str, str]] = []

    in_fence = False
    for line_no, line in enumerate(src.splitlines(), start=1):
        stripped = line.lstrip()
        if re.match(r"(?:```+|~~~+)", stripped):
            in_fence = not in_fence
            continue
        if in_fence:
            continue

        code_spans = find_inline_code_spans(line)

        for match in TOKEN_RE.finditer(line):
            kind = match.lastgroup or "issue"
            start = match.start()
            if kind == "issue" and any(
                start >= s and start < e for s, e in code_spans
            ):
                continue
            violations.append(
                (
                    line_no,
                    start + 1,
                    match.group(),
                    snippet(line, start, match.end()),
                    kind,
                )
            )

    return violations


def scan_toml(path: Path) -> list[tuple[int, int, str, str, str]]:
    """Return violations found in a TOML file.

    TOML has no comment/string-lexer subtlety worth modelling here: manifest
    ``description`` strings and ``#`` dependency comments are both plain prose,
    so every ``TOKEN_RE`` match is a real reference. Hex color codes are
    already excluded by the token pattern, and after the cleanup there are no
    legitimate ``#NNN`` tokens in these files.
    """
    src = path.read_text(encoding="utf-8")
    violations: list[tuple[int, int, str, str, str]] = []

    for line_no, line in enumerate(src.splitlines(), start=1):
        for match in TOKEN_RE.finditer(line):
            start = match.start()
            violations.append(
                (
                    line_no,
                    start + 1,
                    match.group(),
                    snippet(line, start, match.end()),
                    match.lastgroup or "issue",
                )
            )

    return violations


def scan_path(path: Path) -> list[tuple[int, int, str, str, str]]:
    """Scan one file with the scanner its suffix calls for."""
    if path.suffix == ".rs":
        return scan_rust(path)
    if path.suffix == ".md":
        return scan_markdown(path)
    if path.suffix == ".toml":
        return scan_toml(path)
    if path.suffix == ".rq":
        # A `#` opens a comment in SPARQL, so the comment scanner reads these directly.
        return scan_comments(
            [
                (n + 1, line.index("#") + 1, line[line.index("#") :])
                for n, line in enumerate(path.read_text(encoding="utf-8").splitlines())
                if "#" in line
            ],
            exclude_inline_code=False,
        )
    if path.suffix in (".py", ".pyi"):
        # A stub is Python syntax, so the Python scanner reads its comments correctly.
        # Listing `.pyi` in the path iterator without adding it HERE would extend the
        # gate's apparent scope while it inspected nothing — the same silent no-op this
        # script exists to prevent in prose.
        return scan_python(path)
    if path.suffix in (".yaml", ".yml"):
        return scan_yaml(path)
    return []


def main() -> int:
    root = repo_root()

    issues: list[tuple[Path, int, int, str, str]] = []
    process: list[tuple[Path, int, int, str, str, str]] = []
    # Every ``(path, token)`` a register entry could be covering, so an entry
    # that no longer has one can be reported as stale.
    live_process: set[tuple[str, str]] = set()
    live_branch_files: set[str] = set()
    live_plan_files: set[str] = set()
    live_gap_clause_files: set[str] = set()

    for path in iter_scan_paths(root):
        rel = str(path.relative_to(root))
        for line, col, token, text, kind in scan_path(path):
            if kind == "issue":
                issues.append((path, line, col, token, text))
                continue
            key = (rel, token)
            live_process.add(key)
            if kind == "branch" and rel in AMBIGUOUS_BRANCH_PHRASES:
                live_branch_files.add(rel)
                continue
            if kind == "plan_ref" and rel in AMBIGUOUS_PLAN_PHRASES:
                live_plan_files.add(rel)
                continue
            if kind == "gap_tag" and rel in AMBIGUOUS_GAP_CLAUSE_FILES:
                live_gap_clause_files.add(rel)
                continue
            if key in PRE_EXISTING_PROCESS_REFERENCES:
                continue
            process.append((path, line, col, token, text, PROCESS_REMEDY[kind]))

    stale = sorted(
        entry for entry in PRE_EXISTING_PROCESS_REFERENCES if entry not in live_process
    )
    stale_branch = sorted(AMBIGUOUS_BRANCH_PHRASES - live_branch_files)
    stale_plan = sorted(AMBIGUOUS_PLAN_PHRASES - live_plan_files)
    stale_gap_clause = sorted(AMBIGUOUS_GAP_CLAUSE_FILES - live_gap_clause_files)

    if issues:
        for path, line, col, token, text in issues:
            print(f"{path.relative_to(root)}:{line}:{col}: {token} {text}")
    if process:
        for path, line, col, token, text, remedy in process:
            rel = path.relative_to(root)
            print(f"{rel}:{line}:{col}: process reference {token!r} — {remedy}")
            print(f"    {text}")
    for entry_path, token in stale:
        print(
            f"scripts/check-issue-refs.py: PRE_EXISTING_PROCESS_REFERENCES still "
            f"lists {(entry_path, token)}, which no longer occurs — the debt was "
            f"paid; delete the entry so the register keeps shrinking."
        )
    for entry_path in stale_branch:
        print(
            f"scripts/check-issue-refs.py: AMBIGUOUS_BRANCH_PHRASES still lists "
            f"{entry_path!r}, which no longer says 'this branch' — delete the "
            f"entry so the register keeps shrinking."
        )
    for entry_path in stale_plan:
        print(
            f"scripts/check-issue-refs.py: AMBIGUOUS_PLAN_PHRASES still lists "
            f"{entry_path!r}, which no longer says 'the plan's'/'the plan §' — "
            f"delete the entry so the register keeps shrinking."
        )
    for entry_path in stale_gap_clause:
        print(
            f"scripts/check-issue-refs.py: AMBIGUOUS_GAP_CLAUSE_FILES still lists "
            f"{entry_path!r}, which no longer says 'G<N>:' — delete the entry so "
            f"the register keeps shrinking."
        )

    if issues or process or stale or stale_branch or stale_plan or stale_gap_clause:
        return 1

    print(
        f"OK: no #NNN issue-reference tokens and no new process references in "
        f"comments or docs "
        f"({len(PRE_EXISTING_PROCESS_REFERENCES)} pre-existing process "
        f"reference(s), {len(AMBIGUOUS_BRANCH_PHRASES)} control-flow "
        f"'this branch' site(s), {len(AMBIGUOUS_PLAN_PHRASES)} runtime-plan "
        f"'the plan's' site(s), and {len(AMBIGUOUS_GAP_CLAUSE_FILES)} spec-clause "
        f"'G<N>:' site(s) registered)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
