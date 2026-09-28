#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Refuse a silent drop over a shapes-graph read in the SHACL shapes parser.

The SHACL shapes parser reads a shape's parameter values with ``objects_of``.
Two spellings of "read them" quietly discard every value of the wrong kind:

* ``self.objects_of(id, sh::CLASS).into_iter().filter_map(|t| match t {
  Term::NamedNode(n) => Some(n), _ => None })`` — a literal ``sh:class`` value
  vanishes, and the shape validates as if the author had not written it;
* ``for t in self.objects_of(id, sh::NODE_KIND) { if let Term::NamedNode(n) = &t
  { … } else { continue } }`` — the same drop, one statement at a time.

Neither fails a test, because a test fixture writes well-formed shapes. The
ill-formed shapes graph loads green and checks less than it says — the silent
drop this workspace treats as a first-class defect. Every value the parser reads
must be matched exhaustively, with a load error for the kind it cannot take.

What is refused, in ``crates/shapes/src/shapes/parser/`` and in the parser's
home ``crates/shapes/src/shapes.rs``:

* ``.filter_map(`` applied, in the same statement, to a chain rooted at an
  ``objects_of(`` call;
* a ``for`` loop over an ``objects_of(`` result — directly, or through a ``let``
  binding of one — whose body skips an element with ``else { continue }``.

Comments and string literals are blanked before scanning, so prose that quotes
the pattern (this docstring included, if it were Rust) is not code.

``--self-test`` proves both rules fire on planted drops and stay quiet on the
exhaustive spellings that replaced them.
"""

import argparse
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SCOPE_DIRS = [REPO_ROOT / "crates" / "shapes" / "src" / "shapes" / "parser"]
SCOPE_FILES = [REPO_ROOT / "crates" / "shapes" / "src" / "shapes.rs"]


def blank_comments_and_strings(source: str) -> str:
    """Replace comment and string-literal contents with spaces, keeping offsets
    and newlines, so a scan sees only code."""
    out = []
    i = 0
    n = len(source)
    while i < n:
        c = source[i]
        if source.startswith("//", i):
            end = source.find("\n", i)
            end = n if end == -1 else end
            out.append(" " * (end - i))
            i = end
            continue
        if source.startswith("/*", i):
            end = source.find("*/", i + 2)
            end = n if end == -1 else end + 2
            out.append("".join(ch if ch == "\n" else " " for ch in source[i:end]))
            i = end
            continue
        raw = re.match(r'r(#*)"', source[i:])
        if raw and (i == 0 or not (source[i - 1].isalnum() or source[i - 1] == "_")):
            hashes = raw.group(1)
            close = '"' + hashes
            end = source.find(close, i + len(raw.group(0)))
            end = n if end == -1 else end + len(close)
            out.append("".join(ch if ch == "\n" else " " for ch in source[i:end]))
            i = end
            continue
        if c == '"':
            j = i + 1
            while j < n and source[j] != '"':
                j += 2 if source[j] == "\\" else 1
            end = min(j + 1, n)
            out.append("".join(ch if ch == "\n" else " " for ch in source[i:end]))
            i = end
            continue
        if c == "'" and i + 2 < n and (source[i + 2] == "'" or source[i + 1] == "\\"):
            # a char literal ('x' or '\n'), not a lifetime
            end = source.find("'", i + 2) + 1
            out.append(" " * (end - i))
            i = end
            continue
        out.append(c)
        i += 1
    return "".join(out)


def statement_span(code: str, start: int) -> str:
    """The code from ``start`` to the end of its statement: the first ``;``, or the
    first ``{`` opening a block, at parenthesis depth zero."""
    depth = 0
    i = start
    while i < len(code):
        c = code[i]
        if c in "([":
            depth += 1
        elif c in ")]":
            depth -= 1
            if depth < 0:
                break
        elif depth == 0 and c in ";{":
            break
        i += 1
    return code[start:i]


def block_body(code: str, open_brace: int) -> str:
    """The text of the block whose ``{`` is at ``open_brace``."""
    depth = 0
    for i in range(open_brace, len(code)):
        if code[i] == "{":
            depth += 1
        elif code[i] == "}":
            depth -= 1
            if depth == 0:
                return code[open_brace : i + 1]
    return code[open_brace:]


SKIP = re.compile(r"else\s*\{\s*continue\s*;?\s*\}")
LET_BOUND = re.compile(r"\blet\s+(?:mut\s+)?([A-Za-z_][A-Za-z0-9_]*)\b[^=;]*=")
FOR_LOOP = re.compile(r"\bfor\s+[^{};]*?\s+in\s+")


def find_drops(source: str) -> list[tuple[int, str]]:
    """Every silent drop in ``source``, as ``(line, description)``."""
    code = blank_comments_and_strings(source)
    drops: list[tuple[int, str]] = []

    def line_of(offset: int) -> int:
        return code.count("\n", 0, offset) + 1

    # Rule 1: filter_map on an objects_of chain.
    for match in re.finditer(r"\bobjects_of\s*\(", code):
        span = statement_span(code, match.start())
        if re.search(r"\.\s*filter_map\s*\(", span):
            drops.append((line_of(match.start()), "filter_map over an objects_of(...) result"))

    # Names bound to an objects_of result.
    bound: set[str] = set()
    for match in LET_BOUND.finditer(code):
        span = statement_span(code, match.start())
        if re.search(r"\bobjects_of\s*\(", span):
            bound.add(match.group(1))

    # Rule 2: a for loop over an objects_of result that skips with else-continue.
    for match in FOR_LOOP.finditer(code):
        head_end = match.end()
        brace = code.find("{", head_end)
        if brace == -1:
            continue
        iterated = code[head_end:brace]
        over_objects = re.search(r"\bobjects_of\s*\(", iterated) is not None or any(
            re.search(rf"(?<![A-Za-z0-9_]){re.escape(name)}\b", iterated) for name in bound
        )
        if over_objects and SKIP.search(block_body(code, brace)):
            drops.append(
                (line_of(match.start()), "else { continue } over an objects_of(...) result")
            )
    return drops


def scoped_files() -> list[Path]:
    files: list[Path] = []
    for directory in SCOPE_DIRS:
        files.extend(sorted(directory.rglob("*.rs")))
    files.extend(SCOPE_FILES)
    return files


def run() -> int:
    offenders: list[str] = []
    files = scoped_files()
    if not files:
        print("check-shapes-parser-drops: no files in scope — the gate reads nothing")
        return 1
    for path in files:
        for line, what in find_drops(path.read_text(encoding="utf-8")):
            offenders.append(f"{path.relative_to(REPO_ROOT)}:{line}: {what}")
    if offenders:
        print("check-shapes-parser-drops: silent drops over shapes-graph reads:")
        for offender in offenders:
            print(f"  {offender}")
        print(
            "Match every value exhaustively and refuse the kind the parameter cannot take; "
            "a skipped value is a constraint that silently stopped constraining."
        )
        return 1
    print(f"check-shapes-parser-drops: {len(files)} files, no silent drop")
    return 0


PLANTED = {
    "filter_map on a chain": """
        let classes: Vec<NamedNode> = self
            .objects_of(id, sh::CLASS)
            .into_iter()
            .filter_map(|t| match t {
                Term::NamedNode(n) => Some(n),
                _ => None,
            })
            .collect();
    """,
    "else-continue in a direct loop": """
        for t in self.objects_of(id, sh::NODE_KIND) {
            let Term::NamedNode(n) = &t else { continue };
            push(n);
        }
    """,
    "if-let else-continue in a direct loop": """
        for t in self.objects_of(id, sh::NODE_KIND) {
            if let Term::NamedNode(n) = &t { push(n); } else { continue; }
        }
    """,
    "else-continue through a let binding": """
        let mut values = self.objects_of(id, sh::PATTERN);
        values.sort();
        for value in &values {
            let Term::Literal(lit) = value else {
                continue;
            };
            push(lit);
        }
    """,
    "filter_map after a let binding in one statement": """
        let flags = objects_of(self.data, id, sh::FLAGS).into_iter().filter_map(literal).min();
    """,
}

CLEAN = {
    "exhaustive match with a load error": """
        for value in self.objects_of(id, sh::CLASS) {
            match value {
                Term::NamedNode(n) => classes.push(n),
                other => return Err(format!("not an IRI: {other}")),
            }
        }
    """,
    "filter_map over something else": """
        let ids: IdSet = PREDICATES.iter().filter_map(|iri| data.term_id_by_iri(iri)).collect();
    """,
    "continue in a loop that is not over objects_of": """
        for (param, component) in unimplemented_component_params() {
            let Some(x) = lookup(param) else { continue };
            use_it(x);
        }
    """,
    "the pattern in a comment and a string": """
        // .objects_of(id, p).into_iter().filter_map(|t| t.ok())
        let doc = "for t in self.objects_of(id, p) { if let A = t {} else { continue } }";
    """,
}


def self_test() -> int:
    failures: list[str] = []
    for name, snippet in PLANTED.items():
        if not find_drops(snippet):
            failures.append(f"planted drop NOT flagged: {name}")
    for name, snippet in CLEAN.items():
        found = find_drops(snippet)
        if found:
            failures.append(f"clean code flagged: {name}: {found}")
    if failures:
        print("check-shapes-parser-drops --self-test FAILED:")
        for failure in failures:
            print(f"  {failure}")
        return 1
    print(
        f"check-shapes-parser-drops --self-test: {len(PLANTED)} planted drops flagged, "
        f"{len(CLEAN)} clean spellings accepted"
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(
        description=(__doc__ or "Refuse silent drops in the SHACL shapes parser.").splitlines()[0]
    )
    parser.add_argument("--self-test", action="store_true", help="prove the rules fire")
    args = parser.parse_args()
    return self_test() if args.self_test else run()


if __name__ == "__main__":
    sys.exit(main())
