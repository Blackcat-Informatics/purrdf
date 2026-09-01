#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Fail if RFC 3986 reference resolution is re-implemented outside ``purrdf-iri``.

The workspace once carried five independent hand-rolled resolvers — each with
its own dot-segment loop, its own idea of what a scheme is, and its own set of
bugs. They were collapsed onto one layer, ``purrdf_iri`` (``BaseScope`` for the
§5.1 precedence chain, ``Iri::resolve`` for the §5.2 arithmetic). Nothing but
this gate keeps a sixth from appearing: the arithmetic is short enough to
retype, so a future author reaches for it before they find the crate.

This is the ring-fence gate for that property, in the same spirit as
``make rdf-core-hygiene``: a mechanical, name-and-shape check that a reviewer
does not have to remember. It scans every first-party ``.rs`` source outside
``crates/iri/src/`` for the five shapes an RFC 3986 resolver takes:

* the ``remove_dot_segments`` operation (§5.2.4) by name;
* a ``resolve_relative``/``resolve_reference``-shaped function (§5.2);
* a ``merge``-path-shaped function (§5.2.3);
* the ``".."`` segment-popping loop itself, written out (§5.2.4);
* the scheme grammar retyped as a character-class test (§3.1), which is how a
  local ``is_absolute_iri``/``has_iri_scheme`` gets written.

``ALLOWLIST`` is an explicit, reasoned exemption table — never a silent skip.
An entry that stops matching is reported as STALE so the table cannot rot, the
same discipline the conformance harnesses apply to their xfail ledgers.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# The one place the arithmetic is allowed to live.
HOME = "crates/iri/src"
HOME_IMPORT = "purrdf_iri::BaseScope"

# Trees that hold no first-party Rust we govern.
IGNORED_DIRS = {".git", ".claude", ".coding-ethos", "target", "node_modules"}

# rule id -> (compiled pattern, what it means). A rule is unconditional: any
# match is a finding.
SINGLE_LINE_RULES: dict[str, tuple[re.Pattern[str], str]] = {
    "remove-dot-segments": (
        re.compile(r"\bremove_dot_segments\b"),
        "the RFC 3986 §5.2.4 remove_dot_segments operation",
    ),
    "scheme-grammar": (
        re.compile(r"is_ascii_alphanumeric\(\).*?b?'\+'\s*\|\s*b?'-'\s*\|\s*b?'\.'"),
        "the RFC 3986 §3.1 scheme character class, retyped",
    ),
}

# rule id -> (compiled pattern, what it means). A resolver-shaped FUNCTION is a
# finding only when its own body never reaches the shared layer — the property
# this gate defends is one implementation, not one name, so a thin wrapper that
# delegates to `purrdf_iri` is exactly what we want people to write. The name
# alternatives take a prefix but no suffix, so `resolve_reference_targets` (a
# GraphQL alias-chain walk that has nothing to do with IRIs) is not a near-miss.
FUNCTION_RULES: dict[str, tuple[re.Pattern[str], str]] = {
    "resolver-fn": (
        re.compile(
            r"\bfn\s+[a-z0-9_]*"
            r"(?:resolve_relative|resolve_reference|resolve_iri_reference"
            r"|resolve_against_base|resolve_ref)"
            r"\s*[(<]"
        ),
        "an RFC 3986 §5.2 reference-resolution entry point that never reaches "
        "the shared layer",
    ),
    "merge-path-fn": (
        re.compile(r"\bfn\s+[a-z0-9_]*merge_paths?\s*[(<]"),
        "the RFC 3986 §5.2.3 merge operation",
    ),
    "scheme-fn": (
        re.compile(
            r"\bfn\s+[a-z0-9_]*"
            r"(?:has_iri_scheme|iri_has_scheme|is_absolute_iri|is_iri_absolute)"
            r"\s*[(<]"
        ),
        "a local absolute-IRI/scheme predicate that never reaches the shared "
        "layer",
    ),
}

# What "reaches the shared layer" looks like inside a function body.
DELEGATES = re.compile(r"\bpurrdf_iri\b|\bBaseScope\b")

# The `".."` segment-popping loop, which is `remove_dot_segments` without the
# name. Matched as a shape, not a line: a `..` segment arm or comparison whose
# neighbourhood pops an accumulator, in a file that splits on `/`.
DOT_SEGMENT_ARM = re.compile(r'"\.\."\s*(?:=>|==)|==\s*"\.\."')
DOT_SEGMENT_POP = re.compile(r"\.pop\(\)|\.truncate\(|\.rfind\('/'\)")
PATH_SPLIT = re.compile(r"\.r?split(?:_terminator)?\('/'\)")
DOT_SEGMENT_WINDOW = 4
DOT_SEGMENT_RULE = "dot-segment-loop"
DOT_SEGMENT_MEANING = "the RFC 3986 §5.2.4 dot-segment loop, written out"

# (repo-relative path, rule id) -> why this occurrence is NOT a second resolver.
# Every entry must keep matching; a stale one fails this gate.
ALLOWLIST: dict[tuple[str, str], str] = {
    ("crates/gts/src/files.rs", DOT_SEGMENT_RULE): (
        "archive member/symlink target normalization, not IRI resolution. It "
        "must REFUSE an escape above the extraction root; RFC 3986 §5.2.4 "
        "clamps at the root instead, so routing it through purrdf-iri would "
        "convert a rejected zip-slip into a silently rewritten path."
    ),
    ("crates/rdf/src/native_codecs/okf/reader.rs", DOT_SEGMENT_RULE): (
        "OKF bundle-relative Markdown link targets, resolved inside the "
        "bundle's file tree. Same refuse-don't-clamp requirement as the "
        "archive normalizer above; these are member paths, not IRI references."
    ),
    ("crates/rdf/src/projections/research_object/config.rs", "resolver-fn"): (
        "not a resolver: validate_relative_identifier() first rejects every "
        "input a resolver would have to reason about (absolute, dot-segment, "
        "query, fragment, backslash), leaving a plain concatenation under a "
        "caller-owned base. Nothing of RFC 3986 §5.2 is reimplemented."
    ),
}


def rust_sources() -> list[Path]:
    """Every first-party ``.rs`` file outside the one crate that owns the
    arithmetic, in deterministic order."""
    home = REPO_ROOT / HOME
    found: list[Path] = []
    stack = [REPO_ROOT]
    while stack:
        directory = stack.pop()
        for entry in sorted(directory.iterdir()):
            if entry.is_dir():
                if entry.name in IGNORED_DIRS or entry.name.startswith("."):
                    continue
                if entry == home:
                    continue
                stack.append(entry)
            elif entry.suffix == ".rs":
                found.append(entry)
    return sorted(found)


def strip_comment_lines(source: str) -> str:
    """Blank out whole-line comments, preserving every byte offset.

    Prose is not an implementation: a doc comment that *names*
    ``remove_dot_segments`` — which the ``purrdf-iri`` tests must, since that is
    the operation under test — is not a second copy of it. Blanking rather than
    deleting keeps line numbers and byte offsets exact, so findings still point
    at the right place.
    """
    out = []
    for line in source.split("\n"):
        head = line.lstrip()
        if head.startswith(("//", "/*", "*/", "*")):
            out.append(" " * len(line))
        else:
            out.append(line)
    return "\n".join(out)


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


def dot_segment_hits(lines: list[str], source: str) -> list[int]:
    """Line numbers (1-based) where the segment-popping loop appears."""
    if not PATH_SPLIT.search(source):
        return []
    hits: list[int] = []
    for index, line in enumerate(lines):
        if not DOT_SEGMENT_ARM.search(line):
            continue
        low = max(0, index - DOT_SEGMENT_WINDOW)
        high = min(len(lines), index + DOT_SEGMENT_WINDOW + 1)
        if any(DOT_SEGMENT_POP.search(near) for near in lines[low:high]):
            hits.append(index + 1)
    return hits


def scan() -> tuple[list[str], set[tuple[str, str]]]:
    """Return (offender reports, allowlist keys that actually matched)."""
    offenders: list[str] = []
    matched: set[tuple[str, str]] = set()

    for path in rust_sources():
        source = strip_comment_lines(path.read_text(encoding="utf-8"))
        rel = path.relative_to(REPO_ROOT).as_posix()
        lines = source.splitlines()

        found: list[tuple[str, int, str]] = []
        for rule, (pattern, meaning) in SINGLE_LINE_RULES.items():
            for index, line in enumerate(lines):
                if pattern.search(line):
                    found.append((rule, index + 1, meaning))
        for rule, (pattern, meaning) in FUNCTION_RULES.items():
            for match in pattern.finditer(source):
                if DELEGATES.search(function_body(source, match.start())):
                    continue
                found.append((rule, source.count("\n", 0, match.start()) + 1, meaning))
        for line_no in dot_segment_hits(lines, source):
            found.append((DOT_SEGMENT_RULE, line_no, DOT_SEGMENT_MEANING))

        for rule, line_no, meaning in sorted(found, key=lambda hit: (hit[1], hit[0])):
            key = (rel, rule)
            if key in ALLOWLIST:
                matched.add(key)
                continue
            offenders.append(f"{rel}:{line_no}: [{rule}] {meaning}")

    return offenders, matched


def main() -> int:
    offenders, matched = scan()
    stale = sorted(set(ALLOWLIST) - matched)

    if offenders:
        print(
            "RFC 3986 reference resolution has exactly one home in this "
            f"workspace: {HOME}. The following re-implement it:",
            file=sys.stderr,
        )
        for offender in offenders:
            print(f"  {offender}", file=sys.stderr)
        print(
            f"\nDelete the local arithmetic and route through `{HOME_IMPORT}` "
            "(the §5.1 base-precedence chain) or `purrdf_iri::Iri::resolve` "
            "(the §5.2 reference arithmetic). If an occurrence genuinely is "
            "not a resolver, add it to ALLOWLIST in "
            "scripts/check-iri-resolver-singleton.py with the reason.",
            file=sys.stderr,
        )
    if stale:
        print(
            "\nSTALE ALLOWLIST entries in "
            "scripts/check-iri-resolver-singleton.py — they no longer match "
            "anything, so prune them:",
            file=sys.stderr,
        )
        for rel, rule in stale:
            print(f"  {rel}: [{rule}]", file=sys.stderr)

    if offenders or stale:
        return 1
    print(
        f"OK: RFC 3986 reference resolution lives only in {HOME} "
        f"({len(ALLOWLIST)} reasoned non-resolver exemptions)"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
