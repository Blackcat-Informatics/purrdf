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
Unicode property must be implemented with that property. CommonMark, for
instance, defines a "Unicode whitespace character" and uses it to decide the
left- and right-flanking delimiter runs of §6.2 emphasis; a scanner
implementing *that* clause with anything narrower would be wrong.

The example is stated precisely because the imprecise version of it was wrong
and sat here as fact: this docstring used to say CommonMark defines *its*
whitespace as Unicode whitespace, and that a scanner in ``crates/markdown`` was
therefore correct to use ``char::is_whitespace``. It is not. §2.1 defines a
blank line as "a line containing only spaces (U+0020) or tabs (U+0009)", and
ATX headings, thematic breaks and GFM table cells all name space-or-tab as
well. One specification can answer this question differently in different
clauses, so the unit of judgement is the CLAUSE a site implements, never the
specification it belongs to.

What is refused is a scanner for a grammar whose production enumerates an exact set —
``WS ::= #x20 | #x9 | #xD | #xA`` (Turtle/SPARQL/ShExC), XML's
``S ::= (#x20 | #x9 | #xD | #xA)+``, JSON's ``ws`` — answering it with a
property that admits 26 code points instead of 4. Getting that boundary wrong in
the *other* direction is the same class of bug: a gate that rejects the correct
fix teaches authors to hand-roll around it.

Scanners are therefore identified **structurally, not by name**: a file that
holds a character cursor (``fn peek`` plus a ``self.pos``/``position``/``cursor``
field). Name-based detection was tried and is wrong in both directions —
``crates/sparql-eval/src/expr.rs`` has ``fn lex_and_dt``, which returns an RDF
term's *lexical form* and scans nothing, while ``crates/shex/src/shapemap.rs``
scans characters without a single ``lex_``-prefixed function.

**A file the structure test cannot reach is a ledger entry, never a silent
pass.** Detection is per FILE, so a crate that puts its entry point in one file
and its cursor in another is invisible to it: ``crates/geo/src/geojson.rs``
carried this exact defect while its scanner lived in ``json.rs``, and reading
found it when this gate did not. Such files are named in ``SCANNERS``, which
rots loudly — an entry whose file is gone, or which the structure test now finds
by itself, fails the gate.

The narrower heuristic once justified itself here with the claim that widening
would "sweep in every config and rendering path those crates own". That was
asserted, never measured, and it was false: widening the cursor field from
``pos`` alone to ``pos|position|cursor`` governs exactly one further file and
produces zero new offenders. The file was ``crates/cdt/src/parse.rs`` — a real
scanner with a ``peek`` and sixty-nine ``self.position`` — invisible the whole
time the claim stood. A cost asserted for a check is a claim like any other, and
this one is now measured rather than argued.

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

``--census-refusals`` measures the populations the other refusals moved — the
comment-emptied bracket pair, the leading byte-order mark, and a name whose
first scalar its head class forbids. Every refusal is owed a count before it
lands, and a count that lives only in a review thread is a count nobody can
re-run. At the time of writing: 0, 0, and 1 — the single hit being
``vectors/shexTest/negativeSyntax/PN_LOCAL-dash-start.shex``, a negative-syntax
vector whose whole purpose is to be refused. Neither census is a gate; a hit is
a file to open.
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
# `position` and `cursor` are spelled as often as `pos`: `crates/cdt/src/parse.rs`
# holds `fn peek(&self) -> Option<u8>` and 69 occurrences of `self.position`, and
# was invisible to every scanner rule until this alternation was added.
CURSOR_POS = re.compile(r"\bself\.(?:pos|position|cursor)\b")

# Scanners the structural test CANNOT see, because detection is per file and
# these keep their cursor in another module. Naming them is the point: a file
# the heuristic misses must be an entry in a ledger that rots loudly, never a
# silent pass. `crates/geo/src/geojson.rs` carried the `str::trim` defect while
# its cursor lived in `crates/geo/src/json.rs`, and nothing here caught it.
SCANNERS: dict[str, str] = {
    "crates/geo/src/geojson.rs": (
        "decides GeoJSON lexical emptiness and hands the rest to the cursor in "
        "crates/geo/src/json.rs, so it scans without holding a cursor itself"
    ),
    "crates/geo/src/json.rs": (
        "IS the GeoJSON cursor -- a thousand lines of self.pos arithmetic -- but "
        "exposes no `fn peek`, so the structure test misses it. Listing only its "
        "caller above named the file that is NOT the scanner and omitted the one "
        "that is. Its whitespace skip spells RFC 8259's `ws = *( %x20 / %x09 / "
        "%x0A / %x0D )` correctly today, and JSON's `ws` is one of the three "
        "enumerated terminals this gate exists to defend, so a one-line edit here "
        "must not pass unseen"
    ),
    "crates/shapes/src/text_ingest.rs": (
        "scans raw document text for Turtle/SPARQL prefix directives and decides "
        "the directive-head boundary itself, with no cursor struct at all. A "
        "Unicode-whitespace boundary here scanned a phantom `@prefix` out of a "
        "string literal and flipped a `purrdf validate` verdict from a hard error "
        "to a reported Violation"
    ),
    "crates/rdf/src/projections/csvw/config.rs": (
        "listed so the reasoned NON-fix below cannot be quietly reversed: two "
        "separate audits reached the same conclusion here independently, and a "
        "third would too unless it is written down"
    ),
    "crates/sparql-conformance/src/rif_xml.rs": (
        "decides where an XML element name ends while walking a manifest. Harness "
        "support rather than a shipped codec, but a harness that mis-reads a "
        "manifest moves a scoreboard, and the scoreboard is this repository's "
        "conformance claim"
    ),
    # The `xsd_regex` module keeps its one character cursor in `scan.rs`, which
    # the structure test DOES see (it holds both `fn peek` and `self.pos`), so
    # `scan.rs` is deliberately absent from this ledger. The files below are the
    # other lexical-decision sites in that module: each folds over the one
    # cursor, or builds the character classes the translator splices, without
    # holding a cursor itself. `classes.rs` is the shipped case -- its `\i`/`\c`
    # bodies were retyped from Turtle's tables with a hand-patched suffix, and
    # the structure test saw nothing because the retyping was a table, not a
    # Unicode-property test. Listing them makes the scanner rules reach the
    # module so a property-substitution in any of them is a finding rather than
    # a silent pass.
    "crates/rdf-core/src/xsd_regex/classes.rs": (
        "builds the `\\i`/`\\I`/`\\c`/`\\C` bracket-class bodies the translator "
        "splices; holds no cursor, so the structure test misses it, but a "
        "Unicode-property approximation here would silently re-tokenize "
        "`sh:pattern`, SPARQL `REGEX` and ShEx `PATTERN` at once"
    ),
    "crates/rdf-core/src/xsd_regex/emit.rs": (
        "folds the token stream from the cursor in scan.rs into the emitted "
        "`regex` source, including the name/space/word class rewrites; a "
        "boundary decision lives here while the cursor lives next door"
    ),
    "crates/rdf-core/src/xsd_regex/ecma.rs": (
        "folds the same token stream to decide which constructs change meaning "
        "when copied into an ECMA-262 slot; no cursor of its own, so the "
        "structure test cannot see it"
    ),
    "crates/rdf-core/src/xsd_regex/xflag.rs": (
        "strips the XPath `x`-flag whitespace ahead of translation and must "
        "exempt character classes exactly; folds over the scanner's token "
        "stream rather than holding a cursor"
    ),
}

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
    r"(?:pn_chars(?:_base|_u)?"
    r"|pn_local(?:_start|_esc)?"
    r"|pn_prefix"
    r"|ncname(?:_start|_char)?"
    r"|blank_node_label_start"
    r"|varname(?:_start|_continue|_char)?"
    r"|xml_name(?:_start)?(?:_char)?"
    r"|ws(?:_char)?"
    r"|iriref_forbidden(?:_byte)?"
    # `hex` is deliberately absent. It looked like a terminal name and is not:
    # `is_hex` validates a Frictionless data-package field in crates/rdf and
    # answers `is_ascii_hexdigit`, which is exact and owns no production. A
    # family list that sweeps in every plausible-sounding name is the same
    # over-refusal this gate refuses in scanners.
    r"|percent|plx|echar)"
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

# A delegating call, for subtraction. Naming the shared predicate is not enough
# on its own: `terminals::is_percent(c) || c == 'x'` mentions it and then widens
# the terminal anyway, which is the defect wearing the fix's clothes.
DELEGATED_CALL = re.compile(r"\b(?:purrdf_iri::)?terminals::is_\w+(?:\s*\([^()]*\))?")
# An alternation surviving that subtraction ADDS an acceptance branch the
# production does not have. `&&` is deliberately not refused: narrowing a shared
# class is how a real production is expressed — `NCNameChar ::= NameChar - ':'`
# is exactly `is_xml_name_char(c) && c != ':'`, and refusing it would push
# authors back to retyping the table, which is the thing being prevented.
WIDENS_DELEGATION = re.compile(r"\|\|")

# (repo-relative path, rule id) -> why this occurrence is NOT the defect.
# Every entry must keep matching; a stale one fails this gate. This table is the
# workspace's deviation ledger for terminal predicates: a reason here is a
# design decision on the record, not a skip.
ALLOWLIST: dict[tuple[str, str], str] = {
    ("crates/rdf/src/projections/csvw/config.rs", "unicode-name-class"): (
        "validates JSON-LD TERM keys, not XML NCNames. JSON-LD 1.1 §3.1: 'Terms "
        "are case sensitive and most valid strings that are not reserved JSON-LD "
        "keywords are valid terms', and CSVW adds no NCName constraint -- so "
        "tightening this to NCName would be an UNSOURCED refusal, the mirror bug "
        "this gate is otherwise here to prevent. The class is deliberately wrong "
        "in both directions relative to NCName: it refuses '.', which NCNameChar "
        "admits, to protect the prefix:local split, and it admits U+00AA, which "
        "NCName excludes -- both lawful JSON-LD terms. And nothing here decides a "
        "token boundary: a prefix arrives as a whole, already-delimited JSON key, "
        "so this is a membership test, which is the one thing a liberal class may "
        "safely be."
    ),
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


def find_item_terminator(source: str, start: int, limit: int) -> int | None:
    """The offset of the first `;` in ``source[start:limit]`` that is real code.

    Skips `//` line comments, `/* */` block comments, string literals and char
    literals, so punctuation inside prose cannot be mistaken for an item's
    terminator. Returns ``None`` when the span holds no such semicolon.
    """
    cursor = start
    while cursor < limit:
        pair = source[cursor : cursor + 2]
        char = source[cursor]
        if pair == "//":
            newline = source.find("\n", cursor)
            cursor = limit if newline < 0 else newline + 1
            continue
        if pair == "/*":
            close = source.find("*/", cursor + 2)
            cursor = limit if close < 0 else close + 2
            continue
        if char in "\"'":
            cursor += 1
            while cursor < limit and source[cursor] != char:
                cursor += 2 if source[cursor] == "\\" else 1
            cursor += 1
            continue
        if char == ";":
            return cursor
        cursor += 1
    return None


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
        # A BODYLESS item -- `#[cfg(test)] use foo;`, `#[cfg(test)] const N: u8
        # = 1;` -- ends at its semicolon and owns no brace. Without this bound
        # the search runs on to the NEXT item's `{` and blanks real scanner
        # code, and the gate reports OK while seeing nothing. That is a silent
        # false negative in the gate built to prevent silent false negatives,
        # and it is reachable by adding one ordinary `use` line to a scanner.
        # Search for the `;` with inline comments masked. `strip_comment_lines`
        # only blanks WHOLE-line comments, so `#[cfg(test)] // ;` would
        # otherwise present a semicolon that is not the item's terminator, and
        # the module would go unstripped — making a test-only probe of a
        # Unicode property fail the gate. That direction is over-refusal rather
        # than blindness, but a gate that fires on correct test code teaches
        # authors to route around it just as surely.
        semicolon = find_item_terminator(source, match.end(), opening)
        if semicolon is not None:
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


def delegates_purely(body: str) -> bool:
    """Whether *body* asks the shared module and does not then widen its answer.

    Mentioning the shared predicate is not the same as deferring to it. Subtract
    the delegating calls and look at what is left: an ``||`` surviving that
    subtraction is an acceptance branch the production does not have, so the
    body has retyped the terminal with extra steps. Narrowing is untouched — a
    real production is often a shared class minus something, and refusing that
    would push authors back to retyping the whole table.
    """
    if not DELEGATES.search(body):
        return False
    return not WIDENS_DELEGATION.search(DELEGATED_CALL.sub("", body))


def is_scanner(source: str, rel: str = "") -> bool:
    """Whether *source* decides token boundaries: it holds a character cursor,
    or it is named in the ``SCANNERS`` ledger because its cursor lives
    elsewhere."""
    if rel in SCANNERS:
        return True
    return bool(CURSOR_PEEK.search(source)) and bool(CURSOR_POS.search(source))


def findings_for(source: str, rel: str = "") -> list[tuple[str, int, str]]:
    """Every (rule, 1-based line, meaning) this file trips."""
    lines = source.splitlines()
    found: list[tuple[str, int, str]] = []

    if is_scanner(source, rel):
        for rule, (pattern, meaning) in SCANNER_RULES.items():
            for index, line in enumerate(lines):
                if pattern.search(line):
                    found.append((rule, index + 1, meaning))

    for index, line in enumerate(lines):
        if NON_ASCII_CATCHALL.search(line):
            found.append((NON_ASCII_RULE, index + 1, NON_ASCII_MEANING))

    for match in TERMINAL_FN.finditer(source):
        if delegates_purely(function_body(source, match.start())):
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
            findings_for(source, rel), key=lambda hit: (hit[1], hit[0])
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
    # The name IS the claim, so every production the shared module owns must be
    # covered. Two of these were added after the first draft of this gate, and a
    # local fork of either would have gone unflagged.
    for owned in (
        "is_pn_chars_base", "is_pn_chars_u", "is_pn_chars", "is_pn_local_start",
        "is_blank_node_label_start", "is_varname_start", "is_varname_continue",
        "is_ws", "is_ws_char", "is_iriref_forbidden_byte",
        "is_xml_name_start_char", "is_xml_name_char",
        "is_ncname_start", "is_ncname_char", "is_pn_prefix",
    ):
        forked = f"fn {owned}(c: char) -> bool {{ c.is_alphanumeric() }}"
        assert TERMINAL_FN_RULE in rules(forked), f"{owned} must be covered"

    fixed = "fn is_pn_chars_base(c: char) -> bool { terminals::is_pn_chars_base(c) }"
    assert TERMINAL_FN_RULE not in rules(fixed), "delegation must clear the fork"

    # Naming the shared predicate is not deferring to it. A body that delegates
    # and THEN adds an acceptance branch has retyped the terminal with extra
    # steps, and it is the shape most likely to be written by someone who read
    # this gate's message and wanted past it.
    for widened in (
        "fn is_percent(c: char) -> bool { terminals::is_percent(c) || c == 'x' }",
        "fn is_ws(c: char) -> bool { terminals::is_ws_char(c) || c == '\\u{a0}' }",
        "fn is_pn_chars(c: char) -> bool { c == '-' || terminals::is_pn_chars(c) }",
    ):
        assert TERMINAL_FN_RULE in rules(widened), f"partial delegation: {widened}"
    # Narrowing is NOT widening, and refusing it would push authors back to
    # retyping the table: `NCNameChar ::= NameChar - ':'` is exactly this shape.
    for narrowed in (
        "fn is_xml_name_char(c: char) -> bool { terminals::is_xml_name_char(c) && c != ':' }",
        "fn is_iriref_forbidden(c: char) -> bool { !terminals::is_iriref_forbidden(c) }",
        "fn is_ws_char(c: char) -> bool { u8::try_from(c).is_ok_and(terminals::is_ws) }",
    ):
        assert TERMINAL_FN_RULE not in rules(narrowed), f"narrowing is lawful: {narrowed}"

    # The bodyless bound must read CODE, not prose. A `;` inside an inline
    # comment is not an item terminator, and treating it as one leaves a test
    # module unstripped — the gate then fires on correct test code, which
    # teaches authors to route around it just as surely as blindness does.
    commented = (
        "#[cfg(test)] // ends here ;\nmod t { fn u(c: char) { c.is_whitespace(); } }\n"
    )
    assert not findings_for(strip_test_modules(commented)), (
        "a `;` inside a comment is not an item terminator"
    )
    assert find_item_terminator("// ; \n x ;", 0, 10) == 9, "comment `;` must be skipped"
    assert find_item_terminator('let s = \";\"; ', 0, 13) == 11, "string `;` must be skipped"
    assert NON_ASCII_RULE not in rules(fixed), "and leave nothing behind"
    assert not rules("terminals::is_ws(b);"), "the fix itself must be silent"

    # Over-refusal, both axes. A file that is not a scanner decides no token
    # boundary, so `char::is_whitespace` there is ordinary string handling --
    # a config path, a rendering path or a report formatter is ordinary string
    # handling, not a token boundary.
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

    # The stripper must be BOUNDED. A bodyless `#[cfg(test)]` item owns no
    # brace, and an unbounded search runs on to the next item's `{` and blanks
    # the scanner -- the gate then reports OK having seen nothing. Exercised
    # with the real offender AFTER the bodyless item, so a regression is a
    # missing finding rather than a crash.
    bodyless = "#[cfg(test)]\nuse std::collections::HashMap;\n\n" + SCANNER_SHELL % "c.is_whitespace();"
    assert "unicode-whitespace" in {
        rule for rule, _, _ in findings_for(strip_test_modules(bodyless))
    }, "a bodyless #[cfg(test)] item must not blank the scanner that follows it"
    for item in ("const N: u8 = 1;", "static S: u8 = 1;", "type T = u8;", "use a::b;"):
        probe = f"#[cfg(test)]\n{item}\n\n" + SCANNER_SHELL % "c.is_alphanumeric();"
        assert "unicode-name-class" in {
            rule for rule, _, _ in findings_for(strip_test_modules(probe))
        }, f"bodyless `{item}` must not blank what follows"
    # ...and the braced form must still be exempt, or the bound has over-fired.
    braced = "#[cfg(test)]\nmod t { fn u(c: char) { c.is_whitespace(); } }\n"
    assert not findings_for(strip_test_modules(braced)), "braced test module stays exempt"

    # Offsets must survive both blankers, or every finding points at the wrong
    # line and the report is worse than useless.
    for blanker in (strip_comment_lines, strip_test_modules):
        assert len(blanker(in_test)) == len(in_test), f"{blanker.__name__} moved offsets"

    # The census patterns are a claim too. A shape that silently matches nothing
    # would report a reassuring zero forever, which is worse than not measuring.
    for bad in ("ex:-a", "ex:.a", "_:-a", "_:.a", "ex:́a", "_:·a", ":-a"):
        assert HEAD_CLASS_VIOLATION.search(bad), f"head-class shape must match {bad!r}"
    for good in ("ex:a", "ex:_a", "ex:0a", "ex::a", "_:a", "_:0a", "ex:a-b", "ex:a.b"):
        assert not HEAD_CLASS_VIOLATION.search(good), f"lawful name matched: {good!r}"
    assert COMMENT_EMPTIED_BRACKET.search("[ # c\n ]"), "comment-emptied bracket"
    assert not COMMENT_EMPTIED_BRACKET.search("[ ?p ?o ]"), "a populated list must not match"

    print("check-terminal-predicates.py: self-test OK")


def stale_scanners() -> list[str]:
    """``SCANNERS`` entries that no longer earn their place.

    An entry is stale when the file is gone, or when the structural test now
    finds it anyway — a ledger that keeps naming what the detector already sees
    teaches the next reader that the detector is weaker than it is.
    """
    stale: list[str] = []
    for rel in sorted(SCANNERS):
        path = REPO_ROOT / rel
        if not path.is_file():
            stale.append(f"{rel}: file no longer exists")
            continue
        source = strip_test_modules(strip_comment_lines(path.read_text(encoding="utf-8")))
        if CURSOR_PEEK.search(source) and CURSOR_POS.search(source):
            stale.append(f"{rel}: the structural test now detects this file on its own")
    return stale


# The refusals this workspace's tightening added BEYOND the whitespace and
# invisible-scalar family `census()` measures. Each is a population question a
# reviewer would otherwise have to take on trust, so each is measured here
# rather than asserted in a review thread that disappears.
#
# Matched as source shapes, not by tokenizing: a Python re-implementation of the
# scanner would be a sixth transcription of the thing this gate exists to keep
# singular. The shapes are deliberately WIDE — they over-report rather than
# under-report, so a non-zero count is a file to open, never a verdict.
COMMENT_EMPTIED_BRACKET = re.compile(r"\[[ \t]*(?:#[^\n]*)?\n(?:\s*#[^\n]*\n)*\s*\]")
LEADING_BOM = "﻿"
# A prefixed name or blank label whose FIRST scalar the head class forbids:
# `-`, `.`, or a combining mark. `PN_LOCAL` heads on PN_CHARS_U | ':' | [0-9] |
# PLX; `BLANK_NODE_LABEL` on PN_CHARS_U | [0-9].
HEAD_CLASS_VIOLATION = re.compile(
    r"(?<![\w:])(?:[A-Za-z_][\w.-]*)?:[-.̀-ͯ·‿⁀]"
    r"|_:[-.̀-ͯ·‿⁀]"
)


def census_refusals() -> int:
    """Report the corpus population of every refusal this change added."""
    seen: set[Path] = set()
    hits: dict[str, list[str]] = {"comment-emptied-bracket": [], "leading-bom": [], "head-class": []}
    for pattern in CENSUS_GLOBS:
        for path in REPO_ROOT.glob(pattern):
            if not path.is_file() or path.suffix not in CENSUS_EXTS or path in seen:
                continue
            seen.add(path)
            try:
                text = path.read_text(encoding="utf-8")
            except (UnicodeDecodeError, OSError):
                continue
            rel = path.relative_to(REPO_ROOT).as_posix()
            if text.startswith(LEADING_BOM):
                hits["leading-bom"].append(rel)
            for match in COMMENT_EMPTIED_BRACKET.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                hits["comment-emptied-bracket"].append(f"{rel}:{line}")
            for match in HEAD_CLASS_VIOLATION.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                hits["head-class"].append(f"{rel}:{line}: {match.group(0)!r}")

    print(f"refusal census: {len(seen)} files over {len(CENSUS_GLOBS)} globs")
    for kind, found in hits.items():
        print(f"  {kind:26s} {len(found)}")
        for hit in found:
            print(f"      {hit}")
    # Not a gate. A hit is a file to READ: a negative-syntax vector SHOULD
    # contain a head-class violation, and finding one there is the corpus
    # working as intended.
    return 0


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        self_test()
        return 0
    if "--census" in argv:
        return census()
    if "--census-refusals" in argv:
        return census_refusals()
    offenders, matched = scan()
    stale = sorted(set(ALLOWLIST) - matched) + [(entry, "") for entry in stale_scanners()]

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
            "a Unicode property — read the CLAUSE, not the specification: one "
            "spec answers this differently in different places — then this is "
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
