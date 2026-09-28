#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Fail if a helper the workspace centralized is retyped outside its home.

Every family below was found implemented in two or more crates and collapsed onto
one home: W3C vocabulary constants (``purrdf_iri::vocab``, ``purrdf_xsd::datatype``),
the N-Triples literal and IRIREF escapers (``purrdf_iri::literal_escape``,
``purrdf_iri::iri_escape``), RFC 6901 pointer escapes (``purrdf_iri::json_pointer``),
escape-digit decoding (``purrdf_iri::terminals``), FNV-1a and SplitMix64
(``purrdf_hash::fnv``, ``purrdf_hash::mix``, ``purrdf_testkit::rng``), lowercase
hex (``purrdf_hash::hex``), RDF list walking (``DatasetView::rdf_list_with``), the
term nesting bound (``purrdf_events::MAX_TERM_NESTING_DEPTH``), the wasm host
imports, and the one hashing policy (fixed keys, never the standard library's
unspecified ``DefaultHasher``). Nothing but this gate keeps a second copy from
appearing: each is short enough to retype before its home is found.

This is the ring-fence gate for those homes, in the spirit of
``check-iri-resolver-singleton.py``: a mechanical name-and-shape scan a reviewer
does not have to remember. It reads every first-party ``.rs`` under ``crates/`` and
``bindings/``, blanks comment lines and ``#[cfg(test)]`` modules (prose and tests
may spell anything), skips ``tests/``, ``benches/`` and ``examples/`` targets, and
applies each rule outside that rule's home files.

``ALLOWLIST`` is an explicit, reasoned exemption table, never a silent skip. An
entry that stops matching is reported as STALE so the table cannot rot.

``--list`` prints every finding grouped by rule without failing, which is how
the adoption work tracked what remained.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

IGNORED_DIRS = {".git", ".claude", "target", "node_modules", "tests", "benches", "examples"}
SCANNED_ROOTS = ("crates", "bindings")

W3C_NAMESPACES = (
    "http://www.w3.org/1999/02/22-rdf-syntax-ns#",
    "http://www.w3.org/2000/01/rdf-schema#",
    "http://www.w3.org/2002/07/owl#",
    "http://www.w3.org/ns/shacl#",
    "http://www.w3.org/2001/XMLSchema#",
    "http://www.w3.org/2004/02/skos/core#",
)

# rule id -> (pattern, home files where the shape is allowed, meaning, scope)
# scope "code": prose and test modules blanked; "raw": prose blanked only, because
# the shape is a test-only construct the rule exists to catch.
RULES: dict[str, tuple[re.Pattern[str], tuple[str, ...], str, str]] = {
    "w3c-namespace-literal": (
        re.compile('"(?:' + "|".join(re.escape(ns) for ns in W3C_NAMESPACES) + ")"),
        ("crates/iri/src/vocab.rs", "crates/xsd/src/datatype.rs"),
        "a W3C vocabulary IRI spelled as a string literal instead of the shared constant",
        "code",
    ),
    "literal-escape-fn": (
        re.compile(
            r"\bfn\s+[a-z0-9_]*(?:escape_literal|write_literal_escaped|nt_escape"
            r"|turtle_escape|literal_escape)\s*[(<]"
        ),
        ("crates/iri/src/literal_escape.rs",),
        "an N-Triples/Turtle literal escaper written out again",
        "code",
    ),
    "iri-escape-fn": (
        re.compile(
            r"\bfn\s+[a-z0-9_]*(?:escape_iri|is_iri_forbidden|iri_forbids"
            r"|is_iriref_escape_required|write_iri_escaped)\s*[(<]"
        ),
        ("crates/iri/src/iri_escape.rs",),
        "an IRIREF escaper or its forbidden-set predicate written out again",
        "code",
    ),
    "fnv-offset-basis": (
        re.compile(r"0x_?cbf2_?9ce4_?8422_?2325", re.IGNORECASE),
        ("crates/hash/src/fnv.rs",),
        "the FNV-1a 64-bit offset basis, which marks a retyped FNV-1a",
        "code",
    ),
    "splitmix-increment": (
        re.compile(r"0x_?9e37_?79b9_?7f4a_?7c15", re.IGNORECASE),
        ("crates/hash/src/mix.rs", "crates/testkit/src/rng.rs"),
        "the SplitMix64 increment, which marks a retyped generator",
        "code",
    ),
    "json-pointer-escape": (
        re.compile(r'"~[01]"'),
        ("crates/iri/src/json_pointer.rs",),
        "an RFC 6901 token escape written out again",
        "code",
    ),
    "radix-16-escape": (
        re.compile(r"from_str_radix\([^)]*,\s*16\s*\)"),
        ("crates/iri/src/terminals.rs", "crates/iri/src/percent.rs", "crates/iri/src/json_escape.rs"),
        "hex digits decoded with from_str_radix, which accepts a leading sign",
        "code",
    ),
    "reifier-quad-without-graph": (
        re.compile(r"RdfQuad::new\([^;]*reifier"),
        ("crates/rdf/src/native_quads.rs",),
        "a reifier row rebuilt with RdfQuad::new, which drops the graph slot",
        "code",
    ),
    "default-hasher": (
        re.compile(r"\b(?:DefaultHasher|RandomState)::new\(\)"),
        (),
        "the standard library's unspecified hasher, outside the workspace's fixed-key policy",
        "code",
    ),
    "hex-loop": (
        re.compile(r":02x\}"),
        ("crates/hash/src/hex.rs", "crates/rdf-core/src/hex.rs"),
        "a lowercase-hex rendering loop instead of the shared kernel",
        "code",
    ),
    "rdf-list-walk-fn": (
        re.compile(r"\bfn\s+[a-z0-9_]*(?:walk_rdf_list|rdf_list|collect_list|node_list|list_items)\s*[(<]"),
        ("crates/rdf-core/src/dataset_view.rs", "crates/rdf-core/src/collections.rs"),
        "an rdf:first/rdf:rest walk written out again",
        "code",
    ),
    "term-nesting-depth": (
        re.compile(r"\b(?:MAX_[A-Z_]*TERM_[A-Z_]*(?:NESTING_)?DEPTH)\s*:\s*usize\s*=\s*16\b"),
        ("crates/rdf-events/src/lib.rs",),
        "the RDF 1.2 term nesting bound restated as a literal",
        "code",
    ),
    "cross-package-path-include": (
        re.compile(r'#\[path\s*=\s*"\.\./\.\./'),
        (),
        "a source file included from another package, which a published tarball cannot contain",
        "raw",
    ),
    "wasm-date-now-import": (
        re.compile(r"js_namespace\s*=\s*Date\b"),
        ("crates/sparql-eval/src/wasm_host.rs", "crates/rdf-wasm/src/host.rs"),
        "a second Date.now host import",
        "code",
    ),
    "ascii-whitespace-in-grammar": (
        re.compile(r"\bis_ascii_whitespace\b"),
        (),
        "is_ascii_whitespace, which admits FORM FEED, where a grammar's WS set is four bytes",
        "code",
    ),
}

# (repo-relative path, rule id) -> why this occurrence is NOT a second copy.
ALLOWLIST: dict[tuple[str, str], str] = {
    ("crates/hash/src/fixed/keys.rs", "splitmix-increment"): (
        "the golden-ratio constant used as a fixed hash KEY, not a generator step; "
        "the table hasher's keys are compile-time constants by policy."
    ),
}


def rust_sources() -> list[Path]:
    """Every first-party ``.rs`` file under the scanned roots, in deterministic order,
    skipping test, bench and example targets."""
    found: list[Path] = []
    for root_name in SCANNED_ROOTS:
        root = REPO_ROOT / root_name
        if not root.is_dir():
            continue
        stack = [root]
        while stack:
            directory = stack.pop()
            for entry in sorted(directory.iterdir()):
                if entry.is_dir():
                    if entry.name in IGNORED_DIRS or entry.name.startswith("."):
                        continue
                    stack.append(entry)
                elif entry.suffix == ".rs":
                    found.append(entry)
    return sorted(found)


CFG_TEST = re.compile(r"^\s*#\[cfg\(test\)\]\s*$")
MOD_OPEN = re.compile(r"^(\s*)(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{\s*$")


def strip_prose(source: str) -> str:
    """Blank whole-line comments, preserving line count so findings keep their
    line numbers. Prose is not an implementation."""
    out: list[str] = []
    for line in source.split("\n"):
        head = line.lstrip()
        out.append("" if head.startswith(("//", "/*", "*/", "*")) else line)
    return "\n".join(out)


def strip_tests(source: str) -> str:
    """Blank every ``#[cfg(test)] mod name { … }`` block, and the single item that
    follows a bare ``#[cfg(test)]`` attribute, preserving line count.

    A test may spell a constant or a loop to serve as the oracle for the shared
    home, so test modules are not scanned. Only a brace-delimited module is
    blanked to its closing brace (found at the module's own indentation); a
    ``#[cfg(test)]`` on a ``use`` or a ``mod x;`` line blanks that item alone, so
    the rest of the file stays visible.
    """
    lines = source.split("\n")
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if CFG_TEST.match(line):
            out.append("")
            j = i + 1
            while j < len(lines) and not lines[j].strip():
                out.append("")
                j += 1
            if j < len(lines):
                opened = MOD_OPEN.match(lines[j])
                if opened:
                    closing = opened.group(1) + "}"
                    while j < len(lines):
                        out.append("")
                        if lines[j] == closing:
                            break
                        j += 1
                else:
                    out.append("")
            i = j + 1
            continue
        out.append(line)
        i += 1
    return "\n".join(out)


def scan(sources: list[tuple[str, str]]) -> tuple[list[tuple[str, str, int, str]], set[tuple[str, str]]]:
    """Findings as (path, rule, line, meaning) and the allowlist keys that matched."""
    findings: list[tuple[str, str, int, str]] = []
    matched: set[tuple[str, str]] = set()
    for rel, source in sources:
        prose_free = strip_prose(source)
        code_only = strip_tests(prose_free)
        for rule, (pattern, homes, meaning, scope) in RULES.items():
            if rel in homes:
                continue
            text = prose_free if scope == "raw" else code_only
            for hit in pattern.finditer(text):
                key = (rel, rule)
                if key in ALLOWLIST:
                    matched.add(key)
                    continue
                line = text.count("\n", 0, hit.start()) + 1
                findings.append((rel, rule, line, meaning))
    return findings, matched


def load_sources() -> list[tuple[str, str]]:
    return [
        (path.relative_to(REPO_ROOT).as_posix(), path.read_text(encoding="utf-8"))
        for path in rust_sources()
    ]


def self_test() -> int:
    """Each rule fires on its shape, stays quiet in its home and in prose/tests, and a
    stale allowlist entry is detected."""
    cases = {
        "w3c-namespace-literal": 'const X: &str = "http://www.w3.org/2002/07/owl#sameAs";',
        "literal-escape-fn": "fn escape_literal(s: &str) -> String {",
        "iri-escape-fn": "fn is_iri_forbidden(c: char) -> bool {",
        "fnv-offset-basis": "let mut h: u64 = 0xcbf2_9ce4_8422_2325;",
        "splitmix-increment": "state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);",
        "json-pointer-escape": 'out.push_str("~1");',
        "radix-16-escape": "u32::from_str_radix(digits, 16)",
        "reifier-quad-without-graph": "quads.push(RdfQuad::new(reifier.reifier, RDF_REIFIES, statement));",
        "default-hasher": "let mut h = std::collections::hash_map::DefaultHasher::new();",
        "hex-loop": 'write!(out, "{byte:02x}").unwrap();',
        "rdf-list-walk-fn": "fn walk_rdf_list(&self, head: Term) -> Vec<Term> {",
        "term-nesting-depth": "pub const MAX_TERM_NESTING_DEPTH: usize = 16;",
        "cross-package-path-include": '#[path = "../../rdf-core/tests/support/term_fixture.rs"]',
        "wasm-date-now-import": "#[wasm_bindgen(js_namespace = Date, js_name = now)]",
        "ascii-whitespace-in-grammar": "while bytes[i].is_ascii_whitespace() {",
    }
    failures = 0
    for rule, snippet in cases.items():
        findings, _ = scan([("crates/x/src/lib.rs", snippet)])
        if [f[1] for f in findings] != [rule]:
            print(f"self-test: rule {rule} did not fire alone on its shape: {findings}", file=sys.stderr)
            failures += 1
        homes = RULES[rule][1]
        if homes:
            findings, _ = scan([(homes[0], snippet)])
            if findings:
                print(f"self-test: rule {rule} fired inside its home {homes[0]}", file=sys.stderr)
                failures += 1
        findings, _ = scan([("crates/x/src/lib.rs", "// " + snippet)])
        if findings:
            print(f"self-test: rule {rule} fired on a comment line", file=sys.stderr)
            failures += 1
        findings, _ = scan([("crates/x/src/lib.rs", "#[cfg(test)]\nmod tests {\n" + snippet + "\n}")])
        if findings and RULES[rule][3] != "raw":
            print(f"self-test: rule {rule} fired inside a test module", file=sys.stderr)
            failures += 1
    # A bare `#[cfg(test)]` on one item hides that item only, never the rest of the file.
    later = "#[cfg(test)]\nuse x::y;\nfn escape_literal(s: &str) -> String {"
    findings, _ = scan([("crates/x/src/lib.rs", later)])
    if [f[1] for f in findings] != ["literal-escape-fn"]:
        print(f"self-test: a bare cfg(test) attribute hid later code: {findings}", file=sys.stderr)
        failures += 1
    # An allowlisted occurrence is matched, not reported; a missing one is stale.
    findings, matched = scan([("crates/hash/src/fixed/keys.rs", "const PHI: u64 = 0x9E37_79B9_7F4A_7C15;")])
    if findings or ("crates/hash/src/fixed/keys.rs", "splitmix-increment") not in matched:
        print("self-test: the allowlist did not absorb its own entry", file=sys.stderr)
        failures += 1
    _, matched = scan([("crates/hash/src/fixed/keys.rs", "const PHI: u64 = 1;")])
    if set(ALLOWLIST) - matched != set(ALLOWLIST):
        print("self-test: a stale allowlist entry was not detected", file=sys.stderr)
        failures += 1
    if failures:
        return 1
    print(f"check-shared-helpers self-test: OK ({len(RULES)} rules)")
    return 0


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return self_test()
    findings, matched = scan(load_sources())
    stale = sorted(set(ALLOWLIST) - matched)
    if "--list" in argv:
        by_rule: dict[str, list[tuple[str, int]]] = {}
        for rel, rule, line, _ in findings:
            by_rule.setdefault(rule, []).append((rel, line))
        for rule in RULES:
            hits = by_rule.get(rule, [])
            print(f"{rule}: {len(hits)}")
            for rel, line in hits:
                print(f"  {rel}:{line}")
        if stale:
            print("STALE allowlist entries:")
            for rel, rule in stale:
                print(f"  {rel}: [{rule}]")
        return 0
    if findings:
        print("check-shared-helpers: a centralized helper is retyped outside its home:", file=sys.stderr)
        for rel, rule, line, meaning in findings:
            print(f"  {rel}:{line}: [{rule}] {meaning}", file=sys.stderr)
        print(
            "\nCall the shared home instead (see the module docstring). If an occurrence "
            "genuinely is not a copy, add it to ALLOWLIST with its reason.",
            file=sys.stderr,
        )
    if stale:
        print("\nSTALE ALLOWLIST entries in scripts/check-shared-helpers.py (delete them):", file=sys.stderr)
        for rel, rule in stale:
            print(f"  {rel}: [{rule}]", file=sys.stderr)
    if findings or stale:
        return 1
    print(f"check-shared-helpers: OK ({len(RULES)} rules, {len(ALLOWLIST)} reasoned exemptions)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
