#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Fail if a scanner decides a token boundary with a Unicode property.

A scanner's character classes do not decide *membership*, they decide *token
boundaries*. That is why substituting a convenient Unicode property for a
grammar's exact terminal is not the harmless liberality it looks like: it does
not merely widen the accepted language, it **re-tokenizes documents that both
the liberal and the conforming parser accept**, and the two disagree about what
those documents mean. The workspace shipped that bug::

    SELECT ?s WHERE { ?s<NBSP><urn:ex:p> ?o . ?s <urn:ex:q> ?z }

``VARNAME`` was approximated as "ASCII alphanumeric, ``_``, or anything above
U+007F", and U+00A0 is above U+007F, so the greedy name scan swallowed it: four
distinct variables where the author wrote three, a join silently turned into a
cross product, exit zero, no diagnostic. The mirror case, ``ex:a<NBSP>ex:b``,
mints one IRI instead of two and writes it into a data file, where it outlives
any query.

Six instances of this accumulated across three crates in two weeks, because the
substitution is locally reasonable every single time. Nothing but this gate
keeps a seventh from appearing.

**This gate is not "Unicode properties are bad".** It is narrower, and the
distinction is the whole reason it can be trusted: a production that *names* a
Unicode property must be implemented with that property. CommonMark defines its
whitespace as Unicode whitespace, so ``crates/markdown``'s use of
``char::is_whitespace`` is correct and this gate must never flag it. What is
refused is a scanner for a grammar whose production enumerates an exact set —
``WS ::= #x20 | #x9 | #xD | #xA`` (Turtle/SPARQL/ShExC), XML's
``S ::= (#x20 | #x9 | #xD | #xA)+``, JSON's ``ws`` — answering it with a
property that admits 26 code points instead of 4. Getting that boundary wrong in
the *other* direction is the same class of bug: a gate that rejects the correct
fix teaches authors to hand-roll around it.

Scanners are therefore identified **structurally, not by name**: a file that
holds a character cursor (``fn peek`` plus a ``self.pos``-shaped position). Name
-based detection was tried and is wrong in both directions —
``crates/sparql-eval/src/expr.rs`` has ``fn lex_and_dt``, which returns an RDF
term's *lexical form* and scans nothing, while ``crates/shex/src/shapemap.rs``
scans characters without a single ``lex_``-prefixed function.

``ALLOWLIST`` is an explicit, reasoned exemption table — never a silent skip, and
the workspace's deviation ledger for this property. An entry that stops matching
is reported as STALE so the table cannot rot, the same discipline the
conformance harnesses apply to their xfail ledgers.

``--self-test`` proves the rules fire, in both directions. ``--census`` reports
the corpus population a tightening can move, classified by *position* — a
U+00A0 inside an IRIREF body is content and cannot re-tokenize anything, while a
bare one sits at a token boundary. At the time this gate was written the answer
over 4,326 corpus files was **one** occurrence, and it was the instructive kind:
``<urn:ex:\xa0>`` in the frozen W3C RDFC-1.0 fixtures — a LAWFUL IRI that a
tightening aimed at whitespace would break if it reached into IRIREF bodies.
A corpus with no offenders is evidence of no observed regression; it is never
evidence of no over-refusal, which by definition lives in input the corpus does
not contain. That is what the exhaustive per-predicate sweeps are for.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# The one place the W3C terminal productions are spelled out.
HOME = "crates/iri/src/terminals.rs"
HOME_IMPORT = "purrdf_iri::terminals"

# Trees that hold no first-party Rust we govern.
IGNORED_DIRS = {".git", ".claude", "target", "node_modules", ".worktrees"}

# Trees that hold tests and benchmarks rather than shipped scanners.
TEST_DIRS = {"tests", "benches", "examples"}

# `#[cfg(test)]`, and the start of the item it applies to. Test code is exempt
# for a reason this gate cannot afford to get wrong: the vectors that PROVE a
# scanner now matches its production must enumerate the sets it no longer
# accepts, and the only correct way to enumerate "every Unicode whitespace
# character" is `char::is_whitespace`. Hand-picking that set is precisely the
# mistake that let U+205F and U+2029 go unlisted. A gate that flagged the
# derivation would push authors back to hand-picked lists — the mirror of the
# bug it exists to catch.
CFG_TEST = re.compile(r"#\[cfg\(test\)\]")

# A file is scanner-shaped when it carries a character cursor. Both markers are
# required: `fn peek` alone catches token-stream parsers that never see a
# character, and a bare `self.pos` catches binary readers (`crates/gts`,
# `crates/columnar`) whose "position" indexes frames, not scalars.
CURSOR_PEEK = re.compile(r"\bfn\s+peek\w*\s*[(<]")
CURSOR_POS = re.compile(r"\bself\.pos\b")

# Inside a scanner, these substitute a Unicode property for an enumerated
# terminal. `trim`/`trim_start`/`trim_end` are included because a line-oriented
# scanner skips leading whitespace by trimming, and `str::trim` is defined over
# `char::is_whitespace` — the same 26-member set, reached by another spelling.
SCANNER_RULES: dict[str, tuple[re.Pattern[str], str]] = {
    "unicode-whitespace": (
        re.compile(r"\.is_whitespace\(\)|\.trim(?:_start|_end)?\(\)"),
        "Unicode whitespace where the grammar enumerates its WS terminal",
    ),
    "ascii-whitespace": (
        re.compile(r"\.is_ascii_whitespace\(\)"),
        "is_ascii_whitespace, which admits U+000B and U+000C that WS does not",
    ),
    "unicode-name-class": (
        re.compile(r"\.is_alphanumeric\(\)|\.is_alphabetic\(\)"),
        "a Unicode letter property where the grammar enumerates PN_CHARS",
    ),
    # A content class is a terminal too, and `is_control` misses it in BOTH
    # directions: `IRIREF ::= '<' ([^#x00-#x20<>\"{}|^`\\] | UCHAR)* '>'`
    # excludes SPACE (which `is_control` admits) and permits U+007F-U+009F
    # (which `is_control` refuses), so one spelling simultaneously accepts a
    # malformed IRI and rejects a lawful one.
    "unicode-control-class": (
        re.compile(r"\.is_control\(\)"),
        "a Unicode control property where the grammar enumerates its content "
        "class (IRIREF excludes SPACE and admits U+007F-U+009F; is_control "
        "gets both backwards)",
    ),
}

# Unconditional, anywhere: the exact shape the shipped defect took. "Any scalar
# above U+007F is a name character" is never a W3C production; it is what gets
# written when PN_CHARS_BASE's range table looks too long to type.
NON_ASCII_CATCHALL = re.compile(
    r"\bas\s+u32\s*\)\s*(?:>\s*0x7[Ff]|>=\s*0x80)|\bc\s*>=?\s*'\\u\{80\}'",
)
NON_ASCII_RULE = "non-ascii-catchall"
NON_ASCII_MEANING = (
    "'anything above U+007F is a name character', which is not a production"
)

# A local predicate named for a W3C terminal, defined outside the home. The name
# is the claim: `is_pn_chars` asserts it decides PN_CHARS membership, and there
# is exactly one right answer to that, so a body that does not reach the shared
# module is a fork of a table the workspace already owns.
TERMINAL_FN = re.compile(
    r"\bfn\s+is_"
    r"(?:pn_chars(?:_base|_u)?|varname(?:_start|_continue|_char)?|ws)"
    r"\s*[(<]"
)
TERMINAL_FN_RULE = "forked-terminal"
TERMINAL_FN_MEANING = (
    "a W3C terminal predicate retyped instead of reaching the shared module"
)

# What "reaches the shared module" looks like: the module named directly, or one
# of its predicates called through it. Widening this cannot launder the defect —
# NON_ASCII_CATCHALL and the scanner rules are unconditional and no delegation
# clears them.
DELEGATES = re.compile(r"\bpurrdf_iri::terminals\b|\bterminals::is_\w+")

# (repo-relative path, rule id) -> why this occurrence is NOT the defect.
# Every entry must keep matching; a stale one fails this gate. This table is the
# workspace's deviation ledger for terminal predicates: a reason here is a
# design decision on the record, not a skip.
ALLOWLIST: dict[tuple[str, str], str] = {
    ("crates/rdf-core/src/blank_label.rs", TERMINAL_FN_RULE): (
        "deliberately a SECOND, independent transcription, retained as the "
        "oracle the shared module is checked against. The two are not derived "
        "from each other -- one is a binary-searched range table sized for "
        "egress (once per label), the other a `matches!` tree sized for a "
        "tokenizer's inner loop -- so agreement between them is evidence about "
        "the W3C tables themselves, which a single spelling checked against "
        "itself could never provide. The duplication is safe ONLY because "
        "`egress_tables_agree_with_the_shared_scanner_terminals` proves the two "
        "agree on all 1,114,112 scalars; delete that test and this exemption is "
        "void, because a divergence would mint blank node labels the scanner "
        "cannot read back."
    ),
}


def rust_sources() -> list[Path]:
    """Every first-party ``.rs`` file outside the home, in deterministic order."""
    home = REPO_ROOT / HOME
    found: list[Path] = []
    stack = [REPO_ROOT]
    while stack:
        directory = stack.pop()
        for entry in sorted(directory.iterdir()):
            if entry.is_dir():
                if entry.name in IGNORED_DIRS or entry.name.startswith("."):
                    continue
                if entry.name in TEST_DIRS:
                    continue
                stack.append(entry)
            elif entry.suffix == ".rs" and entry != home:
                found.append(entry)
    return sorted(found)


def strip_comment_lines(source: str) -> str:
    """Blank out whole-line comments, preserving every byte offset.

    Prose is not an implementation: the rustdoc at a corrected site must be free
    to say *why* ``char::is_whitespace`` is the wrong answer, and the vectors
    that pin U+1680 must be free to explain it. Blanking rather than deleting
    keeps line numbers exact, so findings still point at the right place.
    """
    out = []
    for line in source.split("\n"):
        head = line.lstrip()
        if head.startswith(("//", "/*", "*/", "*")):
            out.append(" " * len(line))
        else:
            out.append(line)
    return "\n".join(out)


def strip_test_modules(source: str) -> str:
    """Blank out every ``#[cfg(test)]`` item, preserving each byte offset.

    Matched as a brace-balanced span rather than "everything after the first
    attribute", because a file may carry a `#[cfg(test)]` helper above shipped
    code as well as the conventional trailing `mod tests`.
    """
    out = list(source)
    for match in CFG_TEST.finditer(source):
        opening = source.find("{", match.end())
        if opening < 0:
            continue
        depth = 0
        for offset in range(opening, len(source)):
            if source[offset] == "{":
                depth += 1
            elif source[offset] == "}":
                depth -= 1
                if depth == 0:
                    for blank in range(match.start(), offset + 1):
                        if out[blank] != "\n":
                            out[blank] = " "
                    break
    return "".join(out)


def function_body(source: str, start: int) -> str:
    """The brace-delimited body of the ``fn`` whose signature starts at *start*.

    Returns the signature alone when no body follows (a trait method
    declaration), which never delegates and so is never cleared.
    """
    opening = source.find("{", start)
    if opening < 0:
        return source[start : source.find("\n", start)]
    depth = 0
    for offset in range(opening, len(source)):
        char = source[offset]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return source[opening : offset + 1]
    return source[opening:]


def is_scanner(source: str) -> bool:
    """Whether *source* holds a character cursor, and so decides boundaries."""
    return bool(CURSOR_PEEK.search(source)) and bool(CURSOR_POS.search(source))


def findings_for(source: str) -> list[tuple[str, int, str]]:
    """Every (rule, 1-based line, meaning) this file trips."""
    lines = source.splitlines()
    found: list[tuple[str, int, str]] = []

    if is_scanner(source):
        for rule, (pattern, meaning) in SCANNER_RULES.items():
            for index, line in enumerate(lines):
                if pattern.search(line):
                    found.append((rule, index + 1, meaning))

    for index, line in enumerate(lines):
        if NON_ASCII_CATCHALL.search(line):
            found.append((NON_ASCII_RULE, index + 1, NON_ASCII_MEANING))

    for match in TERMINAL_FN.finditer(source):
        if DELEGATES.search(function_body(source, match.start())):
            continue
        line_no = source.count("\n", 0, match.start()) + 1
        found.append((TERMINAL_FN_RULE, line_no, TERMINAL_FN_MEANING))

    return found


def scan() -> tuple[list[str], set[tuple[str, str]]]:
    """Return (offender reports, allowlist keys that actually matched)."""
    offenders: list[str] = []
    matched: set[tuple[str, str]] = set()

    for path in rust_sources():
        source = strip_test_modules(strip_comment_lines(path.read_text(encoding="utf-8")))
        rel = path.relative_to(REPO_ROOT).as_posix()

        for rule, line_no, meaning in sorted(
            findings_for(source), key=lambda hit: (hit[1], hit[0])
        ):
            key = (rel, rule)
            if key in ALLOWLIST:
                matched.add(key)
                continue
            offenders.append(f"{rel}:{line_no}: [{rule}] {meaning}")

    return offenders, matched


SCANNER_SHELL = "impl P {\n    fn peek(&self) -> Option<char> { None }\n    fn f(&self) { let _ = self.pos; %s }\n}\n"

# ``--census``: the corpus population a terminal tightening can possibly move.
# Recorded here, glob and all, so the NEXT tightening re-runs the measurement
# instead of re-inventing it -- or worse, inheriting a number from a previous
# change that measured a different tree with a different entry point.
CENSUS_GLOBS = (
    "vectors/**/*",
    "queries/**/*",
    "crates/**/tests/data/**/*",
    "crates/**/tests/fixtures/**/*",
    "crates/**/corpus/**/*",
    "docs/book/po/*.po",
    "docs/playground/examples/*",
    "bindings/python/tests/**/*",
    "crates/rdf-wasm/js/**/*",
)
CENSUS_EXTS = {
    ".ttl", ".trig", ".nt", ".nq", ".rq", ".ru", ".srx", ".srj", ".shex",
    ".smap", ".shaclc", ".json", ".jsonld", ".po", ".py", ".js", ".ts", ".md",
}

# `WS`, and the Unicode `White_Space` property it is a four-member subset of.
# Spelled out rather than reached through `str.isspace()`, which ALSO counts
# U+001C..U+001F and would invent offenders that do not exist -- the same
# substitute-a-convenient-predicate mistake this gate exists to refuse, made in
# the measurement instead of the scanner.
WS_FOUR = {0x20, 0x09, 0x0D, 0x0A}
UNICODE_WHITE_SPACE = (
    set(range(0x09, 0x0E))
    | {0x20, 0x85, 0xA0, 0x1680, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000}
    | set(range(0x2000, 0x200B))
)
# Invisible, not `White_Space`, and absorbed into names by the old catch-all.
INVISIBLE_FORMAT = {0x200B, 0x200E, 0x200F, 0x2060, 0xFEFF, 0x00AD}
# `PN_CHARS_BASE` admits `[#x200C-#x200D]`: lawful NAME characters, never
# offenders. Sweeping "invisible format characters" hoovers these up and then
# reports a valid query as wrong.
LAWFUL_IN_NAMES = {0x200C, 0x200D}


def census_position(line: str, index: int) -> str:
    """Where ``line[index]`` sits: inside a literal, an IRIREF, or bare.

    "This file contains a U+00A0" is not the question a tightening asks. A
    scalar inside a quoted literal or an IRIREF body is *content*, untouched by
    a scanner whose WS and name classes narrowed; only a BARE one sits at a
    token boundary and can change how the document parses.
    """
    quote: str | None = None
    in_iri = False
    cursor = 0
    while cursor < index:
        char = line[cursor]
        if quote is not None:
            if char == "\\":
                cursor += 2
                continue
            if char == quote:
                quote = None
        elif in_iri:
            if char == ">":
                in_iri = False
        elif char in "\"'":
            quote = char
        elif char == "<":
            in_iri = True
        elif char == "#":
            return "comment"
        cursor += 1
    if quote is not None:
        return "string-body"
    if in_iri:
        return "iriref-body"
    return "BARE"


def census() -> int:
    """Report every offending scalar in the corpora, by position."""
    seen: set[Path] = set()
    counts: dict[tuple[int, str], int] = {}
    bare: list[str] = []
    for pattern in CENSUS_GLOBS:
        for path in REPO_ROOT.glob(pattern):
            if not path.is_file() or path.suffix not in CENSUS_EXTS:
                continue
            if path in seen:
                continue
            seen.add(path)
            try:
                text = path.read_text(encoding="utf-8")
            except (UnicodeDecodeError, OSError):
                continue
            for number, line in enumerate(text.splitlines(), 1):
                for index, char in enumerate(line):
                    code = ord(char)
                    if code in LAWFUL_IN_NAMES:
                        continue
                    offends = (
                        code in UNICODE_WHITE_SPACE and code not in WS_FOUR
                    ) or code in INVISIBLE_FORMAT
                    if not offends:
                        continue
                    where = census_position(line, index)
                    counts[(code, where)] = counts.get((code, where), 0) + 1
                    if where == "BARE":
                        rel = path.relative_to(REPO_ROOT).as_posix()
                        bare.append(f"{rel}:{number}: U+{code:04X}")

    print(f"census: {len(seen)} files over {len(CENSUS_GLOBS)} globs")
    for (code, where), total in sorted(counts.items()):
        print(f"  U+{code:04X}  {where:12s} {total}")
    print(f"bare (tokenization-affecting) occurrences: {len(bare)}")
    for hit in bare:
        print(f"  {hit}")
    # Not a gate. A bare occurrence is a file to READ, not a failure: it may be
    # a vector that deliberately pins the refusal.
    return 0


def self_test() -> None:
    """Prove the gate fires on the shipped defect and stays silent on its fix.

    A gate is a refusal, and a refusal is a claim: showing that the bad shapes
    are caught is only half of it. The half that actually matters here is the
    other one -- this gate's whole risk is that it flags a production that
    legitimately names a Unicode property, teaching the next author to route
    around it.
    """
    rules = lambda body: {rule for rule, _, _ in findings_for(SCANNER_SHELL % body)}

    # The defect, in each spelling it actually shipped as.
    assert "unicode-whitespace" in rules("c.is_whitespace();"), "the WS defect"
    assert "unicode-whitespace" in rules("s.trim_start();"), "trim reaches the same set"
    assert "ascii-whitespace" in rules("c.is_ascii_whitespace();"), "U+000B, U+000C"
    assert "unicode-name-class" in rules("c.is_alphanumeric();"), "the PN_CHARS defect"
    assert "unicode-control-class" in rules("c.is_control();"), "the content-class defect"

    # The verbatim shape the silent misparse took, which no delegation clears.
    was_shipped = "fn is_pn_chars_base(c: char) -> bool { (c as u32) > 0x7F }"
    assert NON_ASCII_RULE in rules(was_shipped), "the > 0x7F catchall"
    assert TERMINAL_FN_RULE in rules(was_shipped), "and the fork it sits in"

    # The corrected form: named for the terminal, but delegating.
    fixed = "fn is_pn_chars_base(c: char) -> bool { terminals::is_pn_chars_base(c) }"
    assert TERMINAL_FN_RULE not in rules(fixed), "delegation must clear the fork"
    assert NON_ASCII_RULE not in rules(fixed), "and leave nothing behind"
    assert not rules("terminals::is_ws(b);"), "the fix itself must be silent"

    # Over-refusal, both axes. A file that is not a scanner decides no token
    # boundary, so `char::is_whitespace` there is ordinary string handling --
    # this is what keeps `crates/markdown` (whose CommonMark production NAMES
    # Unicode whitespace) out of the report.
    not_a_scanner = "fn f(s: &str) { s.trim(); let _ = s.is_whitespace(); }"
    assert not findings_for(not_a_scanner), "a non-scanner must not be flagged"
    assert not findings_for("impl P { fn peek(&self) {} fn f(&self) { s.trim(); } }"), (
        "a cursor-less peek (a token-stream parser) must not be flagged"
    )
    # Test code must stay exempt: the vectors that prove a scanner now matches
    # its production have to enumerate Unicode whitespace to do it, and
    # `char::is_whitespace` is the only correct way to enumerate that set.
    in_test = SCANNER_SHELL % "" + "#[cfg(test)]\nmod t { fn u(c: char) { c.is_whitespace(); } }\n"
    assert not findings_for(strip_test_modules(in_test)), "test code must be exempt"

    # Offsets must survive both blankers, or every finding points at the wrong
    # line and the report is worse than useless.
    for blanker in (strip_comment_lines, strip_test_modules):
        assert len(blanker(in_test)) == len(in_test), f"{blanker.__name__} moved offsets"

    print("check-terminal-predicates.py: self-test OK")


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        self_test()
        return 0
    if "--census" in argv:
        return census()
    offenders, matched = scan()
    stale = sorted(set(ALLOWLIST) - matched)

    if offenders:
        print(
            "A scanner's character classes decide token boundaries, not just "
            "membership, so they must match their production exactly. The "
            "following answer a production with a Unicode property:",
            file=sys.stderr,
        )
        for offender in offenders:
            print(f"  {offender}", file=sys.stderr)
        print(
            f"\nRoute the test through `{HOME_IMPORT}` ({HOME}), where each "
            "production is spelled once with its W3C citation and its ranges "
            "are asserted at compile time. If the grammar here genuinely names "
            "a Unicode property — CommonMark's whitespace does — then this is "
            "not the defect: add it to ALLOWLIST in "
            "scripts/check-terminal-predicates.py, quoting the production.",
            file=sys.stderr,
        )
    if stale:
        print(
            "\nSTALE ALLOWLIST entries in "
            "scripts/check-terminal-predicates.py — they no longer match "
            "anything, so prune them:",
            file=sys.stderr,
        )
        for rel, rule in stale:
            print(f"  {rel}: [{rule}]", file=sys.stderr)

    if offenders or stale:
        return 1
    print(
        f"OK: every scanner's terminals come from {HOME} "
        f"({len(ALLOWLIST)} reasoned exemptions)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
