# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Reject development-process references in PurRDF comments and docs.

Six token families are rejected, over exactly the same scanned surface.

**Issue references** — ``#NNN``. Once an issue is closed the token becomes stale
and misleading, so we do not allow new ones. The same debt hides in two further
shapes. One is an issue number baked into a fixture IRI's path segment, such as
``https://example.org/187-lateral-graph#p``. A reader who meets that host
after the issue closes has no more of a referent than they would from a bare
``#187`` — the digits just moved from a comment into a string literal, which
is exactly the surface the plain ``#NNN`` scan of Rust source does not reach
(string, not comment). The other is the URL form, ``.../issues/31`` or
``.../pull/42``: ``#NNN`` is merely the shorthand for it, and a doc comment
reading ``proposed at <https://github.com/an-org/a-repo/issues/31>`` shipped
in the SPARQL algebra while this lint ran, looked, and was structurally unable
to see it — the digits sat behind a ``/`` rather than a ``#``. Any owner/repo
matches, not just this project's: an upstream tracker thread goes stale for a
reader exactly as fast as our own, and the rule bans the reference, not the
repository it points into. All three shapes are banned outright, with no
grandfather register, because the codebase carries zero legitimate occurrences
of any of them.

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

**Issue-provenance phrasing** — ``the issue``, ``this issue``, ``the issue's``,
and ``the issue-author('s)``, naming the GitHub issue that requested a design or
measured a prototype without ever printing its number. This is the same debt as
the bare ``#NNN`` shape wearing prose instead of a symbol, and it shipped past
every family above: ``the issue's own width``, ``the issue that offered this
crate``, and ``the issue-author's prototype digests`` all read as ordinary
prose to the ``#NNN``/``Task``/``EPIC`` patterns, none of which this shape
resembles. A trailing hyphen is excluded from the match, so ``the issue-named
function``, ``the issue-reference ban``, and the ``issue-normative`` phrase
above are left alone as the unrelated compound words they are; only the bare
phrase and its ``'s``/``-author`` forms are rejected.

**Hazard / finding labels** — a bare ``H12``, or an ``F6``/``N3``-shaped token
used AS A LABEL (``F6:`` opening a clause, ``F6 —`` opening one with a dash
instead of a colon, or wrapped alone in parentheses, ``(F1)``). These are
identifiers from a review thread — meaningful only to the reviewer who assigned
them, not to the codebase. Every one of these letters is also legitimate
technical vocabulary — ``F32``/``F64`` float widths, ``N3`` the RDF
serialization, ``N802`` a linter code, ``H1``–``H6`` the Markdown heading levels
this repository ships a codec for — so a label is recognised by the LABEL SHAPE
(a position plus its punctuation), never by the letter alone.

The DASH form is the one this gate was blind to, and the blindness is the worst
kind: the rule was written, the scan ran, and eleven labels reading ``F1 —``
sat in two tracked test files while the gate printed OK, because the separator
was an em-dash rather than the colon the pattern required. A punctuation mark
is not what makes a label a label. All three separators are matched now — the
colon, an em/en dash, and the ASCII ``-``/``--`` a plain-text comment uses for
the same purpose.

The correction to that fix is the mirror failure, and it is recorded here
because it is the one this rule will keep drifting toward. Widening the rule to
read the label's POSITION *alone* — any comment line opening with ``F``/``N``
and digits — made the gate refuse ``// N3 is a Turtle superset; we parse Turtle
only.`` and ``// F64 values round-trip exactly through the writer.``, and it
refused them on CONTINUATION lines too, so re-wrapping a paragraph could turn
valid prose into a gate failure. It passed only because no line in the tree
happened to begin with one of those tokens. A refusal is a claim, and that one
was false: position and punctuation are BOTH required, together. The label
shapes are enumerated at ``LABEL_OPENING_RE``, and the prose that must stay
spared is asserted case by case in ``_DETECTION_CASES``.

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
* ``.ttl``/``.nt``/``.nq``/``.rq`` under ``generated/``, ``queries/`` and the
  first-party ``crates/*/corpus/`` trees. RDF and SPARQL text is SHIPPED text
  wherever it lives, and the conformance corpora carry prose comments,
  ``mf:name`` strings and fixture IRIs — every surface this lint polices in a
  Rust file — while being scanned for none of them. One lexer reads all four
  grammars: ``#`` opens a comment except inside a quoted literal (data) or an
  ``<IRIREF>`` (a fragment). That second exception is load-bearing in both
  directions — almost every line of a fixture carries ``<http://…/ns#p>``, so
  splitting on the first ``#`` of the line reads the rest of the line as a
  comment and never reaches the real comment after it, and an IRI's own ``#123``
  fragment is not a tracker reference. Comments are scanned for every family;
  literals and IRIs for the issue families only, exactly as Rust literals are.
* ``.yaml``/``.yml`` workflow files under ``.github/`` — only ``#`` comments are
  examined. A ``#`` is a comment only at line start or after whitespace and only
  when outside a quoted scalar (matching YAML's own comment rule), so a ``#``
  inside a quoted string is treated as data.

Vendored payload is excluded by PATH, never by pattern: the byte-frozen W3C
suites (which are outside the corpus scoping above) and the verbatim RDFLib test
files under ``bindings/python/tests/rdflib_suite/vendor/``. Upstream material
legitimately carries tracker URLs as provenance — W3C's own ``rdfs:seeAlso``
links, RDFLib's own issue citations — and this repository runs it unmodified, so
a lint firing there would demand an edit ``check-corpus-frozen.py`` forbids and
the drop-in oracle depends on not having. First-party sidecars in those
directories (this repository's own ``README.md``/``PROVENANCE.md``) stay in
scope.

# The gate proves it can SEE, on every run

A lint's failure mode is not a false positive, it is a hole: it runs, it looks,
it cannot match the thing it exists to reject, and it prints OK — which reads
exactly like a clean tree. Two such holes shipped here, one per axis (a shape
no pattern matched, a surface no scanner read). So ``self_test`` asserts every
rejected shape against the scanners as they actually run, asserts that each
declared RDF/SPARQL surface holds first-party files rather than being nominal,
and asserts that each vendored control both MATCHES the patterns and sits
outside the scan — so the exclusion is proven to be what spares it rather than
blindness. It runs before the scan on every invocation, and ``--self-test`` runs
it alone, printing one line per case.

The issue token pattern is ``#`` followed by 1–5 decimal digits that is not
followed by another digit, a hex letter, a hyphen, or a decimal fraction
(so ``#3.1`` section numbers are not flagged). This avoids 6-digit hex colors
and markdown anchors while still catching references like ``#16`` or ``#123``.

The issue-in-IRI token pattern is the literal text ``example.org/`` followed
by 2–4 decimal digits and a hyphen (``example.org/187-...``), matched as a
plain substring rather than gated by the Rust string/comment lexer's usual
literal-only filtering: unlike the bare ``#NNN`` shape (which collides with
format-spec flags and identifier fragments and so needs
``literal_token_is_a_reference`` to disambiguate), ``example.org/<digits>-``
has no legitimate non-tracker reading anywhere in this repository's fixture
vocabulary, so every match is reported. It is scanned everywhere ``TOKEN_RE``
already runs — Rust comments AND string literals (fixture IRIs live in
literals, which is the surface the plain issue pattern skips there),
Markdown/TOML prose, and Python/YAML comments and docstrings — reusing the
existing scan surfaces rather than adding a new one, since the class differs
from a comment-borne issue reference only in *shape*, not in *location*.

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
(``H12``), and an ``F``/``N`` followed by 1-3 digits that is either immediately
followed by a colon (``F6:``), separated by whitespace from an em/en dash
(``F1 —``), separated by whitespace from an ASCII ``--`` that is itself
followed by whitespace (``F1 -- ``), or wrapped alone in parentheses
(``(F1)``, ``(N3)``). The narrower ``F``/``N`` shape leaves every other use of
those letters (a float width, a serialization name, a linter code) alone, and
the trailing-whitespace requirement on the ASCII form keeps ``F1 -1`` — a label
shape followed by a negative number — out of the match.

The bare ``H<N>`` shape carries ONE exemption, and it is an exemption in the
prose's own terms rather than a register of files: ``H1``–``H6`` (only those
six — ``H7`` and ``H12`` are never spared) on a line that also says
"heading"/"header" is naming a Markdown heading LEVEL, not a review-thread
hazard. This repository ships a Markdown codec, so ``Uses H1 through H6 for
Markdown heading levels.`` and ``H2 headings become sections`` are prose it will
keep writing; refusing them is the over-refusal mirror of the miss above. A
hazard id on a line with no heading vocabulary (``risk H3 is unresolved``) still
fires, and so does ``H12`` on a line about headings.

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

The issue-provenance pattern is ``the``/``this`` (either case) followed by
``issue``, with an optional ``-author`` and an optional ``'s`` (matching either
the ASCII or the typographic apostrophe), rejected only when nothing
alphanumeric or a hyphen immediately follows. The trailing-hyphen exclusion is
load-bearing: without it, ``the issue-named function`` and ``the
issue-reference ban`` — unrelated compound words — would also match, because a
hyphen is not a word character and would otherwise satisfy the same boundary
the bare phrase needs.
"""

from __future__ import annotations

import ast
import re
import subprocess
import sys
from collections.abc import Iterator
from pathlib import Path

ISSUE_PATTERN = r"#\d{1,5}(?![\dA-Fa-f-])(?!\.\d)"

# A tracker reference spelled as a URL. ``#NNN`` is the SHORTHAND for this, and a
# lint that saw only the shorthand had a hole exactly one paste wide: a doc
# comment reading ``proposed at <https://github.com/an-org/a-repo/issues/31>``
# shipped in the SPARQL algebra while this gate ran, looked, and was structurally
# unable to see it — the digits were behind a ``/`` instead of a ``#``. Any
# owner/repo is matched, not just this project's: an upstream tracker thread goes
# stale for a reader exactly as fast as our own, and the standing rule bans the
# reference, not the repository it points into. Vendored trees carry these
# legitimately as provenance and are excluded by path, not by pattern.
ISSUE_URL_PATTERN = r"github\.com/[\w.-]+/[\w.-]+/(?:issues|pull)/\d+"

# The word boundary every process-token family anchors on — spelled as explicit
# ASCII lookarounds rather than ``\b``. Python places ``\b`` only between a
# ``\w`` and a ``\W`` character, and CJK characters are ``\w``, so ``\bH12\b``
# matched ``风险 H12 点`` and did NOT match ``风险H12点``: every family below went
# blind the moment a token was glued to Chinese prose, and the gate depended on
# a typography rule (a half-width space between Latin and CJK runs) that it
# does not check. A process token is an ASCII word; "not preceded or followed
# by an ASCII word character" is the boundary that was meant, and it reads a
# CJK neighbour as the prose it is. ``#NNN`` and the URL form were never
# ``\b``-anchored and were not affected.
_NOT_AFTER_WORD = r"(?<![A-Za-z0-9_])"
_NOT_BEFORE_WORD = r"(?![A-Za-z0-9_])"

# Every rejected token family in ONE pattern, so a file is lexed once no matter
# how many families there are. ``match.lastgroup`` names the family, which is what
# separates an issue reference from a process reference in the report and what
# keeps the inline-code exclusion applying to the former alone.
TOKEN_RE = re.compile(
    rf"(?P<issue_url>{ISSUE_URL_PATTERN})"
    rf"|(?P<issue>{ISSUE_PATTERN})"
    r"|(?P<issue_iri>example\.org/\d{2,4}-)"
    rf"|(?P<task>{_NOT_AFTER_WORD}[Tt]ask\s+#?\d+{_NOT_BEFORE_WORD})"
    rf"|(?P<epic>{_NOT_AFTER_WORD}EPIC{_NOT_BEFORE_WORD})"
    rf"|(?P<branch>(?i:{_NOT_AFTER_WORD}this\ branch{_NOT_BEFORE_WORD}))"
    rf"|(?P<history_ref>{_NOT_AFTER_WORD}on\s+`?origin/main`?{_NOT_BEFORE_WORD})"
    rf"|(?P<issue_prose>(?i:{_NOT_AFTER_WORD}(?:the|this)\ issue(?:-author)?"
    r"(?:['’]s)?(?![A-Za-z0-9_-])))"
    rf"|(?P<issue_normative>{_NOT_AFTER_WORD}issue-normative{_NOT_BEFORE_WORD})"
    rf"|(?P<hazard>{_NOT_AFTER_WORD}H\d{{1,3}}{_NOT_BEFORE_WORD})"
    rf"|(?P<hazard_label>{_NOT_AFTER_WORD}[FN]\d{{1,3}}:"
    rf"|{_NOT_AFTER_WORD}[FN]\d{{1,3}}\s+[—–]"
    rf"|{_NOT_AFTER_WORD}[FN]\d{{1,3}}\s+--(?=\s)"
    rf"|\([FHN]\d{{1,3}}\))"
    rf"|(?P<plan_ref>{_NOT_AFTER_WORD}the plan(?:[\'’]s{_NOT_BEFORE_WORD}|\s+§))"
    rf"|(?P<ac_label>{_NOT_AFTER_WORD}AC\d{_NOT_BEFORE_WORD})"
    rf"|(?P<gap_tag>(?i:gap)\s+R\d{{1,2}}{_NOT_BEFORE_WORD}"
    rf"|(?i:gap)\s+G\d{{1,2}}{_NOT_BEFORE_WORD}"
    rf"|{_NOT_AFTER_WORD}G\d{{1,2}}\s+regression{_NOT_BEFORE_WORD}"
    rf"|{_NOT_AFTER_WORD}G\d{{1,2}}:"
    rf"|{_NOT_AFTER_WORD}R\d{{1,2}}:)"
)

# A finding label OPENING a comment line — after nothing but the comment's own
# punctuation — AND closed, on that same line, by the punctuation a label wears.
# BOTH halves are required, and the second half is the correction to a real
# over-refusal: reading the position alone made every comment line that merely
# BEGAN with `N3`, `F32` or `F64` a gate failure, including a continuation line
# produced by nothing worse than re-wrapping a paragraph.
#
# The closing is an immediate colon, a whitespace-separated em/en dash, a
# whitespace-separated ASCII `--`, or a single bare word closed by a colon
# (`F2 regression:`, `F1 rows:`) — any of which may be preceded by a
# PARENTHETICAL qualifying the label itself (`F1 (value-restriction variant):`,
# `F1 (THE FINDING ITSELF) —`). The parenthetical is optional rather than its
# own alternative on purpose: enumerating `label + parenthetical + colon` while
# forgetting `label + parenthetical + dash` is the same enumeration hole the
# colon-only rule had, and one of the eleven labels this branch removed wore
# exactly that second shape.
#
# A SINGLE `-` is deliberately not a separator here: `N3 - a Turtle superset -
# is out of scope.` is prose, and thirty `# noqa: CODE - reason` suppressions
# ship in the RDFLib compatibility layer.
#
# The dash and colon closings are also matched by `TOKEN_RE` wherever they
# occur, not just at a line's start; this rule adds the qualifier shapes, which
# have no unambiguous reading anywhere else.
LABEL_OPENING_RE = re.compile(
    r"^[ \t/*#!>-]*([FHN]\d{1,3})"
    r"(?:[ \t]*\([^)]*\))?"
    r"(?::"
    r"|[ \t]+[—–]"
    r"|[ \t]+--(?=[ \t])"
    r"|[ \t]+[A-Za-z][A-Za-z0-9_-]*:"
    r")"
)

# ``H1``–``H6`` are the Markdown HEADING LEVELS as well as the shape a review
# thread's hazard ids wear, and this repository ships a Markdown codec — so
# ``Uses H1 through H6 for Markdown heading levels.`` and ``H2 headings become
# sections`` are prose it will keep writing, and a gate that refuses them is the
# over-refusal mirror of the label miss above. The exemption is stated in the
# prose's own terms rather than as a register of files, and it is doubly narrow:
# only the six levels Markdown actually has (``H7`` and ``H12`` are never
# spared), and only on a line that also says "heading"/"header". A hazard id
# with no heading vocabulary beside it still fires.
HEADING_LEVEL_RE = re.compile(r"\AH[1-6]\Z")
HEADING_PROSE_RE = re.compile(r"(?i)head(?:ing|er)s?")


def is_markdown_heading_level(kind: str, token: str, line: str) -> bool:
    """Whether an ``H<N>`` hit names a Markdown heading level, not a hazard id."""
    return (
        kind == "hazard"
        and HEADING_LEVEL_RE.match(token) is not None
        and HEADING_PROSE_RE.search(line) is not None
    )

# Where a rendered book tree's files live in the source tree, for register
# lookups in ``--rendered-tree`` mode (see ``main``).
RENDERED_SOURCE_PREFIX = "docs/book/src/"

# The families that name a TRACKER ITEM — a bare number, a fixture host carrying
# one, or the URL form of the same thing. They are banned outright with no
# register, are the only families read out of non-comment text (string literals,
# fixture IRIs), and are reported as issue references rather than as process
# references, so none of them has a ``PROCESS_REMEDY`` row.
ISSUE_FAMILIES = ("issue", "issue_iri", "issue_url")

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
    "issue_prose": (
        "state the fact, design, or measurement itself, with no reference to "
        "the tracker issue that requested or produced it"
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
        ("crates/rdf/tests/gts_certify.rs", "Task 5"),
        ("crates/rdf/tests/gts_certify.rs", "Task 6"),
        ("crates/rdf/tests/gts_certify.rs", "the plan's"),
        ("crates/shapes/src/instance.rs", "Task 6"),
        # Surfaced by the clause-opening label rule (see ``LABEL_OPENING_RE``),
        # which reads the label's POSITION rather than the punctuation after it.
        # `F6:` in this same file was already registered; these are the same
        # debt spelled `F1 (variant)`, `F1 rows:` and `F<N> regression:`, which
        # a colon-or-dash pattern could not see.
        ("crates/shapes/src/json_schema.rs", "F1"),
        ("crates/shapes/src/json_schema.rs", "F6:"),
        ("crates/shapes/src/json_schema.rs", "Task 3"),
        ("crates/shapes/src/json_schema.rs", "Task 4"),
        ("crates/shapes/src/json_schema.rs", "Task 6"),
        ("crates/shapes/src/shapes.rs", "Task 2"),
        ("crates/shapes/tests/rules_conformance.rs", "Task 6"),
        ("crates/sparql-conformance/tests/owl2_rl_conformance.rs", "the plan's"),
        ("crates/sparql-eval/src/parallel_determinism_gate.rs", "Task 7"),
        ("crates/sparql-eval/src/stat_agg.rs", "the plan's"),
        ("crates/xsd/src/temporal.rs", "F1"),
        ("crates/xsd/src/temporal.rs", "F2"),
        ("crates/xsd/src/temporal.rs", "F3"),
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

# Files where "the plan's"/"the plan §" denotes a runtime QUERY, COMPACTION,
# EXECUTION, or RETRIEVAL plan VALUE — "the plan's pre-order", "the plan's
# transform chain", "the plan's canonical identity" —
# and not a development-planning document. The phrase is banned outright rather
# than guessed at, because English gives no reliable signal: "arrived at through
# the plan's soundness certificate" reads identically whether "plan" is a query
# plan or a design document, and both shapes occur in this codebase. Naming the
# code-sense sites keeps the ban absolute while costing nothing in precision.
# Like the registers above, this may only shrink: an entry whose file no longer
# says "the plan's"/"the plan §" is reported as stale.
AMBIGUOUS_PLAN_PHRASES: frozenset[str] = frozenset(
    {
        "bindings/python/python/src/purrdf/__init__.pyi",
        "bindings/python/src/py_retrieval.rs",
        "bindings/python/tests/test_retrieval.py",
        "crates/datalog/src/seminaive.rs",
        "crates/gts/src/compact.rs",
        "crates/gts/tests/pinned_dict_compaction.rs",
        "crates/rdf-core/src/ir/dataset.rs",
        "crates/rdf/tests/dict_vectors.rs",
        "crates/retrieval/README.md",
        "crates/retrieval/src/admission.rs",
        "crates/retrieval/src/canonical.rs",
        "crates/retrieval/src/compile.rs",
        "crates/retrieval/src/error.rs",
        "crates/retrieval/src/lib.rs",
        "crates/retrieval/src/matching.rs",
        "crates/retrieval/src/plan.rs",
        "crates/retrieval/tests/admission_tests.rs",
        "crates/retrieval/tests/plan.rs",
        "crates/retrieval/tests/planner_golden.rs",
        "crates/retrieval/tests/search.rs",
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

# RDF/SPARQL text: Turtle, N-Triples, N-Quads and SPARQL queries. All four share
# one comment rule (``#`` to end of line, outside a string literal and outside an
# ``<IRIREF>``), so one lexer reads all four.
RDF_SUFFIXES = (".ttl", ".nt", ".nq", ".rq")

# The first-party RDF/SPARQL text this lint reads, as ``git ls-files`` path
# shapes. Two families:
#
# * ``generated/`` and ``queries/`` — emitted and first-party SPARQL, already
#   covered because a committed query is SHIPPED text.
# * ``crates/<crate>/corpus/`` — the first-party conformance corpora (the frozen
#   SHACL corpus and the CONSTRUCT/DESCRIBE corpora). These carry prose comments,
#   ``mf:name`` strings and fixture IRIs — every surface this lint polices in a
#   Rust file — and were unguarded entirely.
#
# Deliberately NOT covered, and by PATH so no pattern has to guess: the vendored
# W3C suites under ``crates/*/suite/``, ``crates/*/entailment-suite/`` and
# ``crates/*/tests/corpus/``, and the vendored vectors under ``vectors/``. Those
# are byte-frozen upstream material this repository may not edit, and upstream
# manifests legitimately carry tracker URLs as provenance (W3C's own
# ``rdfs:seeAlso`` links, for one). A lint that fires on them demands an edit
# that ``check-corpus-frozen.py`` forbids.
CORPUS_TOP_DIRS = ("generated", "queries")


def _is_first_party_rdf_text(segments: list[str]) -> bool:
    """Whether a ``git ls-files`` path is first-party RDF/SPARQL text."""
    if segments[0] in CORPUS_TOP_DIRS:
        return True
    return (
        len(segments) > 3 and segments[0] == "crates" and segments[2] == "corpus"
    )


# Vendored trees whose payload is verbatim upstream text. ``rdflib_suite/vendor/``
# holds byte-for-byte copies of RDFLib's own test files (see its PROVENANCE.md);
# they are run UNMODIFIED as the drop-in conformance oracle, so an upstream
# comment citing an upstream issue thread is provenance, not this repository's
# debt — and it cannot be paid down here without breaking the very property the
# corpus exists to have. First-party sidecars in the same directory (this
# repository's own README/PROVENANCE prose) stay in scope, mirroring the payload/
# sidecar split ``check-corpus-frozen.py`` already draws.
VENDORED_TREES = ("bindings/python/tests/rdflib_suite/vendor/",)
VENDORED_FIRST_PARTY_SIDECARS = frozenset({"README.md", "PROVENANCE.md"})


def _is_vendored_payload(rel: str) -> bool:
    if not rel.startswith(VENDORED_TREES):
        return False
    return rel.rsplit("/", 1)[-1] not in VENDORED_FIRST_PARTY_SIDECARS

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
    (plus root ``.md``/``.toml``), ``.py`` under those dirs and ``scripts``,
    ``.ttl``/``.nt``/``.nq``/``.rq`` under ``generated``/``queries`` and under
    the first-party ``crates/*/corpus/`` trees, and ``.yaml``/``.yml`` GitHub
    workflow files under ``.github``.

    Enumeration is driven by ``git ls-files`` rather than a filesystem walk so
    the scan covers exactly the committed first-party source. Untracked build
    artifacts and third-party trees (``bindings/python/.venv`` linkml docs,
    ``target/``) are never scanned, keeping the lint deterministic and free of
    "green in CI, red locally" divergence. Vendored payload that IS tracked is
    excluded by path (see ``VENDORED_TREES``).
    """
    out = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    for rel in sorted(part for part in out.split("\0") if part):
        suffix = Path(rel).suffix
        if suffix not in (
            ".rs", ".md", ".toml", ".py", ".pyi", ".yaml", ".yml", *RDF_SUFFIXES
        ):
            continue
        if _is_vendored_payload(rel):
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
        elif suffix in RDF_SUFFIXES:
            # RDF and SPARQL text is SHIPPED text wherever it lives: a tracker token
            # in a committed query or fixture is published exactly as one in a printed
            # string is, and six queries carried one while every gate reported clean.
            # `generated/` holds emitted artifacts and `queries/` first-party ones;
            # `crates/*/corpus/` holds the first-party conformance corpora, whose
            # comments, `mf:name` strings and fixture IRIs were unguarded entirely.
            if not _is_first_party_rdf_text(segments):
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
    seen_labels: set[tuple[int, int]] = set()

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
            if is_markdown_heading_level(kind, match.group(), text_lines[rel_line - 1]):
                continue
            line = start_line + rel_line - 1
            col = start_col + rel_col - 1 if rel_line == 1 else rel_col
            if kind == "hazard_label":
                seen_labels.add((line, col))
            violations.append(
                (line, col, match.group(), snippet(text, offset, match.end()), kind)
            )

        # A FINDING LABEL OPENS A CLAUSE AND IS CLOSED BY LABEL PUNCTUATION, and
        # it takes BOTH to be one. A pattern written around the marks alone
        # catches whichever ones it happened to enumerate — eleven `F1 —` labels
        # shipped past a rule that required a colon. A pattern written around
        # the POSITION alone refuses ordinary prose: `// N3 is a Turtle
        # superset` and `// F64 values round-trip exactly` are sentences, and on
        # a wrapped comment they are sentences that only START a line because of
        # where the paragraph happened to break. Requiring the two together
        # catches `F1 (value-restriction variant):` and `F2 regression:` — the
        # qualifier shapes no punctuation rule reaches — while leaving every
        # sentence that merely begins with a float width or a serialization name
        # alone. See `LABEL_OPENING_RE` for the enumerated closings.
        for rel_line, line_text in enumerate(text_lines, start=1):
            opening = LABEL_OPENING_RE.match(line_text)
            if opening is None:
                continue
            rel_col = opening.start(1) + 1
            line = start_line + rel_line - 1
            col = start_col + rel_col - 1 if rel_line == 1 else rel_col
            if (line, col) in seen_labels:
                continue
            violations.append(
                (
                    line,
                    col,
                    opening.group(1),
                    line_text.strip()[:60],
                    "hazard_label",
                )
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


def scan_rust(src: str) -> list[tuple[int, int, str, str, str]]:
    """Return violations found in Rust source text.

    ``exclude_inline_code`` is requested, but ``scan_comments`` applies the
    inline-code-span exclusion only to Rust *doc* comments (which render as
    Markdown) — an issue-shaped token inside backticks in a doc comment is a code
    literal, not a stale issue reference. Ordinary ``//``/``/* */`` comments are
    not Markdown, so a ``#NNN`` inside backticks there is still flagged.
    """
    comments, literals = rust_comments_and_literals(src)
    found = scan_comments(comments, exclude_inline_code=True)
    found.extend(literal_issue_hits(scan_comments(literals, exclude_inline_code=False)))
    found.sort(key=lambda hit: (hit[0], hit[1]))
    return found


def literal_issue_hits(
    hits: list[tuple[int, int, str, str, str]],
) -> list[tuple[int, int, str, str, str]]:
    """The reportable hits from a NON-comment span (a string literal, an IRI).

    Only the ISSUE families survive here. A process reference like "this branch"
    is ordinary prose a program may legitimately print, whereas a ``#NNN``, an
    ``example.org/<digits>-`` fixture host or a tracker URL in a printed,
    returned, or fixture string publishes a tracker id to a caller — which
    ``.baseline`` bans outright, and which three shipped surfaces (and, for the
    IRI shape, two fixture-heavy test files) carried while this lint reported
    clean.

    The bare ``#NNN`` shape is additionally disambiguated by
    ``literal_token_is_a_reference``, because inside a literal it collides with
    format specifiers and IRI fragments. The URL and fixture-host shapes need no
    such filter: neither has a legitimate non-tracker reading.
    """
    return [
        hit
        for hit in hits
        if hit[4] in ("issue_iri", "issue_url")
        or (hit[4] == "issue" and literal_token_is_a_reference(hit[3], hit[2]))
    ]


def rdf_comments_and_text(
    src: str,
) -> tuple[list[tuple[int, int, str]], list[tuple[int, int, str]]]:
    """One pass over Turtle/N-Triples/N-Quads/SPARQL text: its comments and its text.

    All four grammars share one comment rule — ``#`` runs to end of line — with
    two exceptions that a naive "everything after the first ``#``" split gets
    backwards in both directions:

    * a ``#`` inside an ``<IRIREF>`` is a FRAGMENT, not a comment. Almost every
      line of an RDF fixture carries one (``<http://example.org/ns#p>``), so the
      naive split reads the rest of every such line as a comment and never
      reaches the real trailing comment after it.
    * a ``#`` inside a quoted literal is DATA.

    An ``<IRIREF>`` is recognised by the grammar's own rule rather than by
    guessing: it contains no whitespace and none of ``<>"{}|^`\\``. That is what
    keeps SPARQL's ``FILTER(?a < 3 && ?b > 4)`` from being mistaken for an IRI
    and swallowing whatever follows.

    Returns ``(comments, text)`` where *text* is the contents of every quoted
    literal and every IRIREF — the ``mf:name`` prose and fixture IRIs a corpus
    publishes, which are exactly as shipped as a Rust string literal is.
    """
    comments: list[tuple[int, int, str]] = []
    text: list[tuple[int, int, str]] = []
    n = len(src)
    i = 0

    while i < n:
        c = src[i]

        if c == "#":
            j = src.find("\n", i)
            if j == -1:
                j = n
            line, col = pos_to_line_col(src, i)
            comments.append((line, col, src[i:j]))
            i = j
            continue

        if c == "<":
            end = _iriref_end(src, i, n)
            if end is not None:
                line, col = pos_to_line_col(src, i + 1)
                text.append((line, col, src[i + 1 : end]))
                i = end + 1
                continue
            i += 1
            continue

        if c in "\"'":
            triple = src[i : i + 3] == c * 3
            quote = c * 3 if triple else c
            start = i + len(quote)
            j = start
            while j < n:
                if src[j] == "\\":
                    j += 2
                    continue
                if src[j : j + len(quote)] == quote:
                    break
                if not triple and src[j] == "\n":
                    break  # unterminated single-line literal
                j += 1
            line, col = pos_to_line_col(src, start)
            text.append((line, col, src[start : min(j, n)]))
            i = min(j, n) + (len(quote) if j < n else 0)
            continue

        i += 1

    return comments, text


def _iriref_end(src: str, start: int, n: int) -> int | None:
    """Index of the ``>`` closing the IRIREF opening at ``src[start]``, or None.

    ``None`` means the ``<`` is not an IRIREF opener — in SPARQL it is then the
    less-than operator. The test is the grammar's: an IRIREF admits no whitespace
    and none of ``<>"{}|^`\\``.
    """
    j = start + 1
    while j < n:
        c = src[j]
        if c == ">":
            return j
        if c.isspace() or c in '<"{}|^`\\':
            return None
        j += 1
    return None


def scan_rdf(src: str) -> list[tuple[int, int, str, str, str]]:
    """Return violations found in Turtle/N-Triples/N-Quads/SPARQL text.

    Comments are prose and are scanned for every token family. Literals and
    IRIREFs are scanned for the ISSUE families only, exactly as Rust string
    literals are — a fixture IRI or an ``mf:name`` is published text, but the
    process-reference families describe developer prose and have no bearing on
    a data file's payload.
    """
    comments, text = rdf_comments_and_text(src)
    found = scan_comments(comments, exclude_inline_code=False)
    found.extend(literal_issue_hits(scan_comments(text, exclude_inline_code=False)))
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


def scan_python(src: str) -> list[tuple[int, int, str, str, str]]:
    """Return violations found in Python source text.

    Both ``#`` line comments and documentation strings (module/class/function
    docstrings) are scanned; the docstring pass closes the prose gap that let
    an issue reference hide inside a module docstring.
    """
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


def scan_yaml(src: str) -> list[tuple[int, int, str, str, str]]:
    """Return violations found in YAML source text."""
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


def scan_markdown(src: str) -> list[tuple[int, int, str, str, str]]:
    """Return violations found in Markdown text."""
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
            if is_markdown_heading_level(kind, match.group(), line):
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


def scan_toml(src: str) -> list[tuple[int, int, str, str, str]]:
    """Return violations found in TOML text.

    TOML has no comment/string-lexer subtlety worth modelling here: manifest
    ``description`` strings and ``#`` dependency comments are both plain prose,
    so every ``TOKEN_RE`` match is a real reference. Hex color codes are
    already excluded by the token pattern, and after the cleanup there are no
    legitimate ``#NNN`` tokens in these files.
    """
    violations: list[tuple[int, int, str, str, str]] = []

    for line_no, line in enumerate(src.splitlines(), start=1):
        for match in TOKEN_RE.finditer(line):
            start = match.start()
            kind = match.lastgroup or "issue"
            if is_markdown_heading_level(kind, match.group(), line):
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


def scan_source(suffix: str, src: str) -> list[tuple[int, int, str, str, str]]:
    """Scan one file's TEXT with the scanner its suffix calls for.

    Separate from [`scan_path`] because the self-test scans strings: a
    falsifiability case that had to write a file to be measured would either
    touch the tree or measure something other than what ships.

    A suffix listed in ``iter_scan_paths`` and absent HERE would extend the
    gate's apparent scope while it inspected nothing, so an unknown suffix is a
    hard failure rather than an empty list.
    """
    if suffix == ".rs":
        return scan_rust(src)
    if suffix == ".md":
        return scan_markdown(src)
    if suffix == ".toml":
        return scan_toml(src)
    if suffix in RDF_SUFFIXES:
        return scan_rdf(src)
    if suffix in (".py", ".pyi"):
        # A stub is Python syntax, so the Python scanner reads its comments correctly.
        # `.pyi` is SHIPPED (the PEP 561 stub inside every published wheel), which is
        # why it is enumerated at all.
        return scan_python(src)
    if suffix in (".yaml", ".yml"):
        return scan_yaml(src)
    raise SystemExit(
        f"check-issue-refs: {suffix!r} is enumerated for scanning but no scanner "
        "reads it — the gate would report clean over a surface it never inspected."
    )


def scan_path(path: Path) -> list[tuple[int, int, str, str, str]]:
    """Scan one file with the scanner its suffix calls for."""
    return scan_source(path.suffix, path.read_text(encoding="utf-8"))


# ── This gate's own falsifiability ────────────────────────────────────────────
#
# A lint's failure mode is not a false positive, it is a hole: it runs, it looks,
# and it is structurally unable to see the thing it is written to reject — and it
# prints OK, which reads exactly like a clean tree. Two such holes shipped here.
# The cases below are the shapes that got through, each asserted against the
# scanners as they actually run, plus the clean shapes that must NOT be flagged
# (a lint no one can live with is removed, and then there is no lint at all).

# The token SHAPE that shipped in a first-party Rust doc comment while this gate
# reported clean: the digits were behind a `/`, so the `#NNN` scan could not see
# them. The owner/repo is a placeholder for the same reason fixtures say
# `example.org` — a specimen must exercise the pattern without becoming the very
# reference it is written to reject.
_SHIPPED_URL = "https://github.com/an-org/a-repo/issues/31"
_SHIPPED_URL_TOKEN = "github.com/an-org/a-repo/issues/31"

# ``(what, suffix, source text, token that must be reported — or None for
# "must report nothing")``. Every one is text this gate scans for real.
_DETECTION_CASES: tuple[tuple[str, str, str, str | None], ...] = (
    (
        "an issue URL in a Rust doc comment (the shape that shipped)",
        ".rs",
        f"/// The form proposed at <{_SHIPPED_URL}> and already implemented.\n",
        _SHIPPED_URL_TOKEN,
    ),
    (
        "an issue URL in a Rust string literal",
        ".rs",
        f'const NOTE: &str = "see {_SHIPPED_URL}";\n',
        _SHIPPED_URL_TOKEN,
    ),
    (
        "a pull-request URL in Markdown prose",
        ".md",
        "The rewrite landed in https://github.com/example-org/example/pull/42 .\n",
        "github.com/example-org/example/pull/42",
    ),
    (
        "an issue URL in a Turtle comment",
        ".ttl",
        f"@prefix ex: <http://example.org/ns#> .\n# tracked at {_SHIPPED_URL}\n",
        _SHIPPED_URL_TOKEN,
    ),
    (
        "an issue URL in an mf:name string",
        ".ttl",
        f'[] mf:name "regression for {_SHIPPED_URL}" .\n',
        _SHIPPED_URL_TOKEN,
    ),
    (
        "an issue URL baked into a fixture IRI",
        ".ttl",
        f"<{_SHIPPED_URL}> a ex:Case .\n",
        _SHIPPED_URL_TOKEN,
    ),
    (
        "an issue URL in a SPARQL comment",
        ".rq",
        f"SELECT * WHERE {{ ?s ?p ?o }}  # {_SHIPPED_URL}\n",
        _SHIPPED_URL_TOKEN,
    ),
    (
        "a bare #NNN in an N-Quads comment",
        ".nq",
        "# regression for #31\n<http://example.org/s> <http://example.org/p> "
        '"o" <http://example.org/g> .\n',
        "#31",
    ),
    (
        "an issue-number fixture host in a Turtle IRI",
        ".ttl",
        "<https://example.org/187-lateral-graph#p> a ex:Case .\n",
        "example.org/187-",
    ),
    (
        # The trailing comment is reachable ONLY if the `#` inside the IRI is
        # read as a fragment. Splitting on the first `#` of the line — which is
        # what this scan used to do for `.rq` — swallows the IRI and never
        # reaches the reference after it.
        "a reference AFTER an IRI that itself contains a # fragment",
        ".ttl",
        "ex:a <http://example.org/ns#label> ex:b .  # tracked at "
        f"{_SHIPPED_URL}\n",
        _SHIPPED_URL_TOKEN,
    ),
    (
        # `<` is SPARQL's less-than as well as an IRI opener. Treating every `<`
        # as an IRI start swallows the rest of the line and hides the comment.
        "a reference after a SPARQL less-than comparison",
        ".rq",
        "SELECT * WHERE { FILTER(?a < 3 && ?b > 4) }  # "
        f"{_SHIPPED_URL}\n",
        _SHIPPED_URL_TOKEN,
    ),
    (
        "an ordinary IRI fragment that merely looks like an issue number",
        ".ttl",
        "ex:a <http://example.org/ns#123> ex:b .\n",
        None,
    ),
    # Issue-provenance phrasing: the shape that got past every other family here,
    # because it is prose rather than a symbol.
    (
        "a possessive issue-provenance reference, spelled the issue's",
        ".rs",
        "/// This crate ships the issue's own width, not a value it picked.\n",
        "the issue's",
    ),
    (
        "a bare issue-provenance reference, spelled this issue",
        ".rs",
        "/// No consumer exists in this issue, so the parameter is not added.\n",
        "this issue",
    ),
    (
        "an issue-author possessive reference",
        ".rs",
        "// The issue-author's prototype numbers are not this crate's evidence.\n",
        "The issue-author's",
    ),
    (
        "an issue-derived compound word that is a different phrase (spared)",
        ".rs",
        "/// Local name (the issue-named function): listLength.\n",
        None,
    ),
    # The process-token families glued to CJK prose. Each is a token this gate
    # already rejected in English and reported clean over when the neighbouring
    # character was Chinese, because `\b` is not a boundary between a Latin
    # letter and a CJK character. The English control beside each proves the
    # ASCII lookaround is the same boundary `\b` was wherever `\b` worked, and
    # the clean Chinese sentence at the end proves the widened match fires on
    # tokens, not on Chinese.
    # The finding-label separators. The colon form was matched from the start;
    # the dash forms were not, and eleven labels spelled with an em-dash sat in
    # tracked Rust comments while this gate reported clean. Each separator is
    # asserted here, with the technical vocabulary that shares those letters
    # asserted clean beside it.
    (
        "a finding label opening a clause with a colon",
        ".rs",
        "// F6: the manifest is written before the corpus exists.\n",
        "F6:",
    ),
    (
        "a finding label opening a clause with an em-dash (the shape that shipped)",
        ".rs",
        "// F1 — the lane certifies output it never produced.\n",
        "F1 —",
    ),
    (
        "a finding label opening a clause with an en-dash",
        ".md",
        "N3 – the shard manifest outlives its shard.\n",
        "N3 –",
    ),
    (
        "a finding label opening a clause with an ASCII double hyphen",
        ".rs",
        "// F4 -- the stamp survives a failed run.\n",
        "F4 --",
    ),
    (
        "a finding label wrapped alone in parentheses",
        ".rs",
        "// The guard added for (F1) is exercised below.\n",
        "(F1)",
    ),
    (
        # The shape no punctuation rule reaches: the label OPENS the clause and
        # what follows it is a parenthetical, not a separator.
        "a finding label opening a clause with a parenthetical",
        ".rs",
        "// F1 (the value-restriction variant): the guard rejects a bare node.\n",
        "F1",
    ),
    (
        "a finding label opening a clause with a bare word",
        ".rs",
        "// F2 regression: magnitudes past the old wrap point.\n",
        "F2",
    ),
    (
        # The counter-check to the two above. Mid-sentence, these letters are
        # ordinary technical vocabulary and the rule must leave them alone.
        "a serialization name and a float width mid-sentence (spared)",
        ".rs",
        "// The N3 grammar and the F64 lane are both handled here.\n",
        None,
    ),
    (
        "a float width followed by a dash is not a finding label",
        ".rs",
        "// The F64-to-F32 narrowing is checked by the caller.\n",
        None,
    ),
    (
        "a label shape followed by a negative number is not a finding label",
        ".md",
        "The F1 -1 offset is the sentinel for an absent frame.\n",
        None,
    ),
    (
        # The reason the ASCII separator is the DOUBLE hyphen and not the single
        # one. A linter suppression spells its code, a single hyphen, and a
        # reason -- which is lexically a label followed by a dash, and thirty of
        # them ship in the RDFLib compatibility layer. Rejecting those would be
        # the mirror of missing the em-dash: a rule nobody can live with.
        "a linter suppression code followed by a single hyphen (spared)",
        ".py",
        "    value = obj  # noqa: N802 - RDFLib API name\n",
        None,
    ),
    # ── Position is not enough, and the over-refusal that proved it ────────────
    #
    # Reading the label's POSITION alone — any comment line opening with an
    # `F`/`N`/`H` and digits — refuses ordinary prose in an RDF library, and
    # refuses it on a CONTINUATION line too, so re-wrapping a paragraph could
    # turn a valid comment into a gate failure. Each sentence below is prose
    # this repository is entitled to write; each has a firing control beside it,
    # because a gate whose self-test only proves it refuses is half a test.
    (
        "a serialization name opening a comment line (spared)",
        ".rs",
        "// N3 is a Turtle superset; we parse Turtle only.\n",
        None,
    ),
    (
        "a float width opening a comment line (spared)",
        ".rs",
        "// F64 values round-trip exactly through the writer.\n",
        None,
    ),
    (
        "two float widths opening a comment line (spared)",
        ".rs",
        "// F32 and F64 are both accepted by the numeric cast.\n",
        None,
    ),
    (
        "a serialization name opening a line with SINGLE hyphens (spared)",
        ".rs",
        "// N3 - a Turtle superset - is out of scope.\n",
        None,
    ),
    (
        # The landmine shape: nothing is wrong with the comment, the paragraph
        # merely broke before the token.
        "a doc-comment continuation line beginning with a float width (spared)",
        ".rs",
        "/// The decimal path is the one the reader tolerates, which is why\n"
        "/// F64 values are compared after normalisation rather than before.\n",
        None,
    ),
    (
        "an ordinary-comment continuation line beginning with a serialization name (spared)",
        ".rs",
        "// The parser accepts Turtle and, by the superset relation,\n"
        "// N3 documents whose subset is Turtle.\n",
        None,
    ),
    (
        "a continuation line beginning with a narrow float width (spared)",
        ".rs",
        "// The cast widens on the way in and narrows on the way out, so\n"
        "// F32 inputs are stored at the width the caller handed over.\n",
        None,
    ),
    (
        "a label shape mid-sentence after a section word (spared)",
        ".rs",
        "// Section F2 of the contract covers the numeric cast.\n",
        None,
    ),
    (
        "a label shape mid-sentence after an imperative (spared)",
        ".rs",
        "// Use F1 when the frame is absent and F2 when it is not.\n",
        None,
    ),
    # The controls for the four spared shapes above: the SAME letters, in the
    # same opening position, wearing the punctuation that makes them labels.
    (
        "a finding label opening a clause with a colon (control)",
        ".rs",
        "// F1: the lane certifies output it never produced.\n",
        "F1:",
    ),
    (
        "a serialization-letter label with an em-dash (control)",
        ".rs",
        "// N3 — the shard manifest outlives its shard.\n",
        "N3 —",
    ),
    (
        "a float-width-letter label with an ASCII double hyphen (control)",
        ".rs",
        "// F4 -- the stamp survives a failed run.\n",
        "F4 --",
    ),
    (
        "a label wrapped alone in parentheses on a continuation line (control)",
        ".rs",
        "// The guard below closes the finding recorded as\n// (N2) in the review.\n",
        "(N2)",
    ),
    (
        "an issue number in a comment (control)",
        ".rs",
        "// Tracked as #332 until the writer is rewritten.\n",
        "#332",
    ),
    (
        # The eleventh of the eleven. Its qualifier is a parenthetical and its
        # separator is a dash, and a rule that enumerated `parenthetical + colon`
        # alone let it through — the same enumeration hole, one shape further on.
        "a finding label with a parenthetical qualifier and a dash (control)",
        ".rs",
        "// F1 (THE FINDING ITSELF) — the lane certified a digest for a corpus "
        "that did not exist.\n",
        "F1",
    ),
    (
        # A wrapped comment whose continuation line IS a label still fires: the
        # rule that spares the sentences above is punctuation, not line count.
        "a label opening a continuation line with a dash (control)",
        ".rs",
        "// The guard covers the wide lane, and the finding it closes is\n"
        "// F1 — the lane certifies output it never produced.\n",
        "F1 —",
    ),
    # ── Markdown heading levels are not hazard ids ─────────────────────────────
    #
    # `H1`..`H6` are what a Markdown codec's prose calls its heading levels. The
    # bare `H<N>` rule refused that prose outright; it is spared now, on the two
    # conditions stated at `is_markdown_heading_level`, with the controls that
    # prove each condition is load-bearing.
    (
        "a heading-level range in a comment about Markdown (spared)",
        ".rs",
        "// Uses H1 through H6 for Markdown heading levels.\n",
        None,
    ),
    (
        "heading-level prose in Markdown (spared)",
        ".md",
        "H2 headings become sections in the exported document.\n",
        None,
    ),
    (
        "a heading level beside the word header (spared)",
        ".rs",
        "// The H3 header is emitted as a nested section.\n",
        None,
    ),
    (
        # Condition one: the line must be ABOUT headings. A hazard id keeps its
        # number wherever it appears.
        "a single-digit hazard id with no heading vocabulary (control)",
        ".rs",
        "// The H3 risk is unresolved and blocks the writer.\n",
        "H3",
    ),
    (
        # Condition two: only the six levels Markdown has. `H12` is not one.
        "a two-digit hazard id on a line about headings (control)",
        ".md",
        "H12 headings are not a level; this is a review-thread id.\n",
        "H12",
    ),
    (
        "a heading-shaped id outside the Markdown range (control)",
        ".rs",
        "// The H7 heading hazard is tracked separately.\n",
        "H7",
    ),
    ("a hazard id glued to CJK", ".md", "风险H12点已处理。\n", "H12"),
    ("a hazard id spaced from CJK (house typography)", ".md", "风险 H12 点已处理。\n", "H12"),
    ("a hazard id in English prose", ".md", "risk H12 handled.\n", "H12"),
    ("EPIC glued to CJK", ".md", "此为EPIC的一部分。\n", "EPIC"),
    ("a task reference glued to CJK", ".md", "见Task 28的描述。\n", "Task 28"),
    ("an acceptance-criterion label glued to CJK", ".md", "满足AC1要求。\n", "AC1"),
    ("a bare #NNN glued to CJK", ".md", "参见#31。\n", "#31"),
    (
        "a Chinese sentence with Latin acronyms and no process token",
        ".md",
        "本节描述三元组项（triple term）的处理方式，见 RDF 1.2 与 SHACL。\n",
        None,
    ),
    (
        "an ASCII word that merely ends in a token shape stays spared",
        ".md",
        "the MAC1 register and the SHA256 digest and ACID transactions\n",
        None,
    ),
    (
        "an ordinary N-Triples fixture with no reference at all",
        ".nt",
        "<http://example.org/s> <http://example.org/p> "
        '"a plain literal"@en .\n',
        None,
    ),
)

# ``(what, path, suffix)`` — vendored payload that CARRIES a reference the
# widened patterns match. Each must be outside the scan (upstream text this
# repository runs verbatim and may not edit) AND must be reported when scanned
# directly, so the exclusion is proven to be what spares it rather than the
# patterns being blind.
_VENDORED_CONTROLS: tuple[tuple[str, str], ...] = (
    (
        "W3C's own rdfs:seeAlso issue link in a vendored SPARQL manifest",
        "crates/sparql-conformance/suite/w3c-sparql11/functions/manifest.ttl",
    ),
    (
        "RDFLib's own issue reference in a verbatim vendored test",
        "bindings/python/tests/rdflib_suite/vendor/test_optional.py",
    ),
)


def self_test(report: bool) -> list[str]:
    """Every way this gate is blind. An empty list is the only passing answer."""
    problems: list[str] = []

    for what, suffix, src, expected in _DETECTION_CASES:
        tokens = [hit[2] for hit in scan_source(suffix, src)]
        if report:
            verdict = (
                (expected in tokens) if expected is not None else (not tokens)
            )
            print(f"  {'ok' if verdict else 'BLIND':6}  {suffix}: {what}")
        if expected is None:
            if tokens:
                problems.append(
                    f"  • {suffix}: {what} is FLAGGED ({tokens}) — a lint that "
                    "fires on ordinary data is a lint that gets switched off"
                )
        elif expected not in tokens:
            problems.append(
                f"  • {suffix}: {what} is NOT reported (found {tokens or 'nothing'}) "
                f"— {expected!r} is exactly the shape this gate exists to reject"
            )

    root = repo_root()
    scanned = {str(path.relative_to(root)) for path in iter_scan_paths(root)}

    for suffix in RDF_SUFFIXES:
        covered = [
            rel
            for rel in scanned
            if rel.endswith(suffix) and _is_first_party_rdf_text(rel.split("/"))
        ]
        if report:
            print(
                f"  {'ok' if covered else 'BLIND':6}  {suffix}: "
                f"{len(covered)} first-party file(s) in the scan"
            )
        if not covered:
            problems.append(
                f"  • no first-party {suffix} file is in the scan — the surface is "
                "declared and inspects nothing"
            )

    for what, rel in _VENDORED_CONTROLS:
        path = root / rel
        if not path.is_file():
            raise SystemExit(
                f"check-issue-refs: the vendored control {rel} is gone, so nothing "
                f"proves {what} is still spared by PATH rather than by blindness. "
                "Re-point the control rather than leaving the gate untested."
            )
        reported = bool(scan_source(path.suffix, path.read_text(encoding="utf-8")))
        excluded = rel not in scanned
        if report:
            print(
                f"  {'ok' if (reported and excluded) else 'BROKEN':6}  {rel}: {what}"
            )
        if not reported:
            problems.append(
                f"  • {rel}: {what} is no longer matched at all, so its exclusion "
                "proves nothing about the patterns"
            )
        if not excluded:
            problems.append(
                f"  • {rel}: vendored payload is INSIDE the scan — upstream text "
                "this repository runs verbatim would have to be edited to satisfy "
                "a lint about this repository's own debt"
            )

    return problems


def scan_rendered_tree(tree: Path) -> int:
    """Scan every ``.md`` under a rendered book tree. Returns the exit code.

    A rendering of ``docs/book/src/`` — ``mdbook build`` with the ``markdown``
    renderer and a translation applied (see ``scripts/check-i18n-render.py``)
    — is outside the default enumeration twice over: it is build output, and
    it is untracked. The same scanners run over it here; the registers are
    consulted through the source mapping (``DIR/x.md`` is
    ``docs/book/src/x.md``), and no stale-entry report is made, because the
    tree carries only the book and the English scan is what keeps the
    registers exact.
    """
    issues: list[str] = []
    process: list[str] = []
    scanned = 0
    for path in sorted(p for p in tree.rglob("*.md") if p.is_file()):
        scanned += 1
        rel = path.relative_to(tree).as_posix()
        source_rel = RENDERED_SOURCE_PREFIX + rel
        for line, col, token, text, kind in scan_path(path):
            if kind in ISSUE_FAMILIES:
                issues.append(f"{path}:{line}:{col}: {token} {text}")
                continue
            if kind == "branch" and source_rel in AMBIGUOUS_BRANCH_PHRASES:
                continue
            if kind == "plan_ref" and source_rel in AMBIGUOUS_PLAN_PHRASES:
                continue
            if kind == "gap_tag" and source_rel in AMBIGUOUS_GAP_CLAUSE_FILES:
                continue
            if (source_rel, token) in PRE_EXISTING_PROCESS_REFERENCES:
                continue
            process.append(
                f"{path}:{line}:{col}: process reference {token!r} — "
                f"{PROCESS_REMEDY[kind]}\n    {text}"
            )
    if scanned == 0:
        print(
            f"check-issue-refs: no .md file under {tree} — a rendered tree with "
            "nothing in it is a vacuous pass, not a clean one",
            file=sys.stderr,
        )
        return 1
    for entry in issues + process:
        print(entry)
    if issues or process:
        return 1
    print(
        f"OK: no issue-reference tokens and no process references in the rendered "
        f"tree ({scanned} page(s) scanned under {tree})."
    )
    return 0


def main(argv: list[str]) -> int:
    rendered: Path | None = None
    alone = False
    args = list(argv[1:])
    while args:
        argument = args.pop(0)
        if argument == "--self-test":
            alone = True
        elif argument == "--rendered-tree" and args:
            rendered = Path(args.pop(0))
        else:
            print(
                f"usage: {Path(argv[0]).name} [--self-test] [--rendered-tree DIR]",
                file=sys.stderr,
            )
            return 2

    if alone:
        print(
            "check-issue-refs: checking that this gate can SEE each shape it "
            "rejects, and that it spares vendored text by path —"
        )
    # BEFORE the scan itself, on every run: this gate's whole failure mode is
    # printing OK over a surface it never inspected or a shape it cannot match,
    # and it has done both. Pure text over strings plus one `git ls-files`, so it
    # costs a fraction of the scan it precedes.
    blind = self_test(report=alone)
    if blind:
        print(
            "check-issue-refs: this gate reports clean over shapes it exists to "
            "reject:\n" + "\n".join(blind)
            + "\n\nEach line above is a reference that ships with every gate "
            "green. Fix the scan, not the case.",
            file=sys.stderr,
        )
        return 1
    if alone:
        print(
            f"OK: all {len(_DETECTION_CASES)} shapes are matched or spared as "
            f"written, all {len(RDF_SUFFIXES)} RDF/SPARQL surfaces hold "
            f"first-party files, and all {len(_VENDORED_CONTROLS)} vendored "
            "controls are excluded by path."
        )
        return 0

    if rendered is not None:
        if not rendered.is_dir():
            print(f"check-issue-refs: {rendered} is not a directory", file=sys.stderr)
            return 2
        return scan_rendered_tree(rendered)

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
            if kind in ISSUE_FAMILIES:
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
    raise SystemExit(main(sys.argv))
