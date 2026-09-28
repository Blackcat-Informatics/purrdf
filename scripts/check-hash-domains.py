#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Hold every hash-domain string to a committed registry.

A domain string is the constant a digest is separated by: the bytes a crate feeds a
hasher first so that the same content hashed for two purposes yields two identities.
Every one of them is a published identity law — a content id, a plan id, a proof
digest, a cache key on disk — and renaming one changes what a consumer already holds.
The workspace spells them six ways (`purrdf.purremb.v1.…\\0`, `purrdf-datalog-…-vN`,
`purrdf-geo/…/vN`, `purrdf:…:v1`, unversioned registry names, bare names owned by
the GTS specification), and none can be renamed. So the rule is not one spelling; it
is one ledger: ``scripts/hash-domains.txt`` lists every domain string in the tree,
and this gate fails when the tree and the ledger disagree in either direction. A new
domain is added to the ledger in the same change that mints it, so the diff shows a
reviewer that a new identity law was declared on purpose; a domain that vanishes
from the tree is a stale ledger line, so an identity law cannot disappear silently.

What counts as a domain string, mechanically (non-test source under ``crates/`` and
``bindings/``, comment lines blanked):

* a ``const``/``static`` string or byte-string whose NAME says so (``DOMAIN`` or a
  ``D_`` prefix; vocabulary constants such as ``RDFS_DOMAIN`` are excluded by their
  namespace prefix) or whose VALUE carries a version suffix (``-v1``, ``/v2``,
  ``.v1``, `` v1``) or a trailing NUL;
* a literal fed straight into a hasher: ``.update(b"…")``, ``.update("…".as_bytes())``,
  ``derive_key("…")``, ``frame_le(…, b"…")``.

``--write`` regenerates the ledger from the tree (for the change that mints a domain);
``--self-test`` exercises the classifier.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
LEDGER = REPO_ROOT / "scripts" / "hash-domains.txt"
IGNORED_DIRS = {".git", ".claude", "target", "node_modules", "tests", "benches", "examples"}
SCANNED_ROOTS = ("crates", "bindings")

CONST = re.compile(
    r"\b(?:const|static)\s+([A-Z][A-Z0-9_]*)\s*:\s*&(?:'static\s+)?"
    r"(?:str|\[u8(?:;\s*\d+)?\])\s*=\s*(b?\"(?:[^\"\\]|\\.)*\")"
)
NAME_SAYS_DOMAIN = re.compile(r"^D_|^DOMAIN|_DOMAIN$|DOMAIN_")
VOCABULARY_CONST = re.compile(r"^(?:RDF|RDFS|OWL|SH|SHNEX|XSD|SKOS|DCAT|DCTERMS|PROV|FOAF)_")
VALUE_SAYS_DOMAIN = re.compile(r"(?:[-/. ]v\d+|\\0)\"$")
INLINE = re.compile(
    r"(?:\.update\(|derive_key\(|frame_le\([^,]+,\s*)(b?\"(?:[^\"\\]|\\.)*\")(?:\.as_bytes\(\))?\)"
)
CFG_TEST = re.compile(r"^\s*#\[cfg\(test\)\]\s*$")
MOD_OPEN = re.compile(r"^(\s*)(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{\s*$")


def rust_sources() -> list[Path]:
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


def strip(source: str) -> str:
    """Blank comment lines and ``#[cfg(test)] mod … { }`` blocks, keeping line count."""
    lines = source.split("\n")
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        head = line.lstrip()
        if head.startswith(("//", "/*", "*/", "*")):
            out.append("")
            i += 1
            continue
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


def domains_in(rel: str, source: str) -> set[str]:
    text = strip(source)
    found: set[str] = set()
    for name, literal in CONST.findall(text):
        if VOCABULARY_CONST.match(name):
            continue
        if NAME_SAYS_DOMAIN.search(name) or VALUE_SAYS_DOMAIN.search(literal):
            found.add(f"{literal}\t{rel}")
    for literal in INLINE.findall(text):
        if len(literal) > 4:
            found.add(f"{literal}\t{rel}")
    return found


def scan_tree() -> set[str]:
    found: set[str] = set()
    for path in rust_sources():
        rel = path.relative_to(REPO_ROOT).as_posix()
        found |= domains_in(rel, path.read_text(encoding="utf-8"))
    return found


def read_ledger() -> set[str]:
    if not LEDGER.exists():
        return set()
    return {
        line.rstrip("\n")
        for line in LEDGER.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.startswith("#")
    }


def write_ledger(entries: set[str]) -> None:
    header = (
        "# The hash-domain ledger: every digest-separation string in the tree, with the\n"
        "# file that declares it. Regenerated by `python3 scripts/check-hash-domains.py\n"
        "# --write`; `make check` fails when tree and ledger disagree. See the script's\n"
        "# docstring for what counts and why none of these may be renamed.\n"
    )
    LEDGER.write_text(header + "\n".join(sorted(entries)) + "\n", encoding="utf-8")


def self_test() -> int:
    sample = '\n'.join([
        'const D_TARGET_SET: &[u8] = b"purrdf.purremb.v1.target-set\\0";',
        'const CACHE_KEY: &str = "purrdf-datalog-cache-v3";',
        'const ENGINE: &str = "purrdf-sparql-eval";',
        'const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";',
        'f.field("args")',
        'hasher.update(b"purrdf-geo/index-source/v1");',
        'hasher.update(payload);',
        '// const IGNORED_DOMAIN: &str = "in-a-comment-v1";',
        '#[cfg(test)]',
        'mod tests {',
        '    const T_DOMAIN: &str = "test-only-v9";',
        '}',
    ])
    got = {line.split("\t")[0] for line in domains_in("crates/x/src/lib.rs", sample)}
    want = {'b"purrdf.purremb.v1.target-set\\0"', '"purrdf-datalog-cache-v3"', 'b"purrdf-geo/index-source/v1"'}
    if got != want:
        print(f"self-test: classifier answered {sorted(got)}, wanted {sorted(want)}", file=sys.stderr)
        return 1
    print("check-hash-domains self-test: OK")
    return 0


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return self_test()
    tree = scan_tree()
    if "--write" in argv:
        write_ledger(tree)
        print(f"check-hash-domains: ledger written with {len(tree)} entries")
        return 0
    ledger = read_ledger()
    minted = sorted(tree - ledger)
    stale = sorted(ledger - tree)
    if minted:
        print("check-hash-domains: domain strings in the tree but not in the ledger "
              "(a new identity law needs a deliberate ledger line: run --write and review the diff):",
              file=sys.stderr)
        for entry in minted:
            print(f"  {entry}", file=sys.stderr)
    if stale:
        print("check-hash-domains: ledger lines with no domain string in the tree "
              "(an identity law moved or vanished; if intended, run --write):", file=sys.stderr)
        for entry in stale:
            print(f"  {entry}", file=sys.stderr)
    if minted or stale:
        return 1
    print(f"check-hash-domains: OK ({len(ledger)} domain strings on the ledger)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
