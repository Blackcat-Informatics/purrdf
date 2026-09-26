#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Fail if a serializer rewinds output it has already produced.

A streaming emitter cannot un-write a byte. Once a window has drained to the
caller's writer it is gone, so any construction that decides what to emit by
emitting it and then taking it back is not merely inelegant here — it is
unimplementable, and it fails only at the scale where the window actually
drains. Below the drain size every one of these spellings works perfectly.

Three of them were live in this workspace, and each looked local and harmless:

* the Turtle writer emitted its prefix header, emitted the body, and
  ``truncate``d the header back off when the body turned out to be empty;
* the SPARQL-Results JSON writer ``pop``ed the root object's closing brace so it
  could splice in a provenance member and re-close;
* the XML escaper ``truncate``d back to its entry length when it met a scalar
  XML 1.0 cannot represent, so the output was "unchanged on failure".

All three were replaced by deciding before emitting. Nothing but this gate keeps
a fourth from arriving: each is the obvious way to write the thing it does, and
an author reaches for it long before they think about the drain.

``TextSink`` removes the affordance where it can — it exposes no ``truncate``,
``pop``, ``clear`` or read-back, so the mistake is not expressible against a
sink. This gate covers the rest of the path: the ``&mut String`` buffers the
per-codec seam still hands around, where the affordance does exist.

Scope is the two trees that emit documents: ``crates/rdf/src/native_codecs/``
and ``crates/sparql-results/src/``, plus ``crates/rdf-core/src/xml_escape.rs``,
which is not a serializer but is called by four of them and held the third
rewind.

A NOTE ON THE MIRROR BUG
------------------------
``pop`` on a stack and ``pop`` on an output buffer are the same six characters.
A gate that refused both would refuse correct code — the XML writer's element
stack, the JSON-LD compiler's remote-context stack, the OKF reader's path
components — and a gate that rejects the correct spelling teaches authors to
work around the gate rather than to keep the law. That failure is the exact
mirror of the one this gate exists to catch, and it is harder to see, because a
refusal looks like strictness.

So every finding is a *candidate*, and ``ALLOWLIST`` is an explicit, reasoned
exemption table naming each receiver that is not an output buffer. It is never a
silent skip: an entry that stops matching is reported as STALE, so the table
cannot rot into a blanket permission the way an ignore-list does.

Run with ``--self-test`` to check the gate detects the three historical rewinds.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# The trees that emit documents, plus the escaper four of them call.
SCANNED: tuple[str, ...] = (
    "crates/rdf/src/native_codecs",
    "crates/sparql-results/src",
    "crates/rdf-core/src/xml_escape.rs",
)

# The four shapes that take back something already produced. `truncate` and
# `clear` shorten; `pop` removes the last thing written; `insert(0`/`remove(0`
# rewrite the front, which a drained stream no longer holds at all.
REWIND = re.compile(r"\.(?:truncate\(|clear\(\)|pop\(\)|insert\(0|remove\(0)")

# `r"`, `r#"`, `r##"` … — a raw string opener, whose closer depends on the hash count.
RAW_STRING_OPEN = re.compile(r'r#*"')
# A char literal (`'a'`, `'\\n'`), as opposed to a lifetime (`'a` with no closing quote).
CHAR_LITERAL = re.compile(r"'(?:\\.|[^\\'])'")

# (path, receiver) -> why this receiver is not an output buffer.
#
# Keyed by receiver rather than by line number so the table survives edits above
# it: a reasoned exemption should not need renewing every time the file moves.
ALLOWLIST: dict[tuple[str, str], str] = {
    (
        "crates/rdf/src/native_codecs/rdfxml.rs",
        "pending",
    ): "The RDF/XML READER's depth-first work stack over the parsed element tree, "
    "which collects a document's `xmlns` declarations. It holds input elements "
    "still to visit, not output.",
    (
        "crates/rdf/src/native_codecs/rdfxml.rs",
        "stack",
    ): "The RDF/XML writer's open-element stack. Popping it closes a tag; the "
    "bytes for that tag were already emitted and are not touched.",
    (
        "crates/rdf/src/native_codecs/jsonld/context/compiler.rs",
        "self.remote_stack",
    ): "The JSON-LD context compiler's remote-context stack, which exists to "
    "detect cyclic `@context` IRIs. It holds IRIs under consideration, not output.",
    (
        "crates/rdf/src/native_codecs/okf/reader.rs",
        "components",
    ): "Path components during OKF reference resolution — the `..` segment loop. "
    "This is input being resolved, not a document being written.",
    (
        "crates/rdf/src/native_codecs/ser_model.rs",
        "left",
    ): "One half of a comparison pair. `cmp_terms`/`cmp_graph` render two terms to "
    "text purely to order them; the rendering is thrown away and never emitted.",
    (
        "crates/rdf/src/native_codecs/ser_model.rs",
        "right",
    ): "The other half of the same comparison pair. See `left`.",
    (
        "crates/rdf/src/native_codecs/ser_model.rs",
        "scratch",
    ): "The TriG graph-name scratch. The name is rendered here so it can be compared "
    "against the currently-open graph BEFORE anything is written — which is the "
    "decide-before-emitting shape this gate exists to enforce, not a violation of "
    "it. Note `out` is a separate parameter and is never rewound.",
    (
        "crates/rdf/src/native_codecs/stream.rs",
        "self.raw",
    ): "The line reader's INPUT buffer, reused across lines. This is the parse path; "
    "there is no document being emitted here to take anything back from.",
}


def rust_sources() -> list[Path]:
    """Every first-party ``.rs`` file in scope, tests excluded."""
    found: list[Path] = []
    for entry in SCANNED:
        target = REPO_ROOT / entry
        if target.is_file():
            found.append(target)
            continue
        found.extend(
            path
            for path in sorted(target.rglob("*.rs"))
            if not path.name.endswith("tests.rs")
        )
    return found


def receiver(line: str) -> str:
    """The expression a rewind was called on, or ``""`` when unreadable.

    Deliberately textual: the point is to name the receiver in a table a human
    reads, not to resolve it. An unreadable receiver returns ``""``, which
    matches no allowlist key and is therefore reported rather than skipped.
    """
    match = re.search(r"([A-Za-z_][A-Za-z0-9_.]*)\.(?:truncate|clear|pop|insert|remove)\(", line)
    return match.group(1) if match else ""


def strip_noncode(source: str) -> list[str]:
    """`source` with comments and string literals blanked, line structure intact.

    Only real code is scanned. A line comment mentioning a rewind is prose; so is a
    block comment, and so is a string literal that happens to contain one. Reporting
    any of those is the mirror bug this gate is built around — a gate that fails
    ``make check`` on correct code teaches authors to work around it.

    Characters are replaced by spaces rather than removed, so reported line numbers
    and the quoted line still match the file.
    """
    out = list(source)
    index = 0
    end = len(source)
    while index < end:
        char = source[index]
        if char == "/" and index + 1 < end and source[index + 1] == "/":
            while index < end and source[index] != "\n":
                out[index] = " "
                index += 1
        elif char == "/" and index + 1 < end and source[index + 1] == "*":
            # Rust block comments NEST, so track depth rather than finding `*/`.
            depth = 0
            while index < end:
                if source.startswith("/*", index):
                    depth += 1
                    out[index] = out[index + 1] = " "
                    index += 2
                elif source.startswith("*/", index):
                    depth -= 1
                    out[index] = out[index + 1] = " "
                    index += 2
                    if depth == 0:
                        break
                else:
                    if source[index] != "\n":
                        out[index] = " "
                    index += 1
        elif char == "r" and (match := RAW_STRING_OPEN.match(source, index)):
            close = '"' + "#" * (len(match.group(0)) - 2)
            stop = source.find(close, match.end())
            stop = end if stop == -1 else stop + len(close)
            for position in range(index, stop):
                if source[position] != "\n":
                    out[position] = " "
            index = stop
        elif char in {'"', "'"}:
            # A Rust lifetime (`'a`) is not a literal; a char literal is.
            if char == "'" and not CHAR_LITERAL.match(source, index):
                index += 1
                continue
            quote = char
            out[index] = " "
            index += 1
            while index < end:
                if source[index] == "\\":
                    out[index] = " "
                    if index + 1 < end and source[index + 1] != "\n":
                        out[index + 1] = " "
                    index += 2
                    continue
                done = source[index] == quote
                if source[index] != "\n":
                    out[index] = " "
                index += 1
                if done:
                    break
        else:
            index += 1
    return "".join(out).splitlines()


def scan() -> tuple[list[str], set[tuple[str, str]]]:
    """Findings, and which allowlist keys were actually used."""
    findings: list[str] = []
    used: set[tuple[str, str]] = set()
    for path in rust_sources():
        rel = path.relative_to(REPO_ROOT).as_posix()
        source = path.read_text(encoding="utf-8")
        raw = source.splitlines()
        for number, code in enumerate(strip_noncode(source), 1):
            line = raw[number - 1]
            if not REWIND.search(code):
                continue
            key = (rel, receiver(code))
            if key in ALLOWLIST:
                used.add(key)
                continue
            findings.append(
                f"{rel}:{number}: rewind on `{key[1] or '<unreadable receiver>'}`: "
                f"{line.strip()}"
            )
    return findings, used


def self_test() -> int:
    """The gate detects each of the three rewinds that were actually removed."""
    historical = (
        ("turtle header retraction", "        out.truncate(start);"),
        ("SRJ root-brace splice", "    if out.pop() != Some('}') {"),
        ("xml_escape failure rewind", "            output.truncate(original_length);"),
    )
    failures = 0
    for name, line in historical:
        if not REWIND.search(line.split("//", 1)[0]):
            print(f"SELF-TEST FAIL: would not have caught the {name}: {line.strip()}")
            failures += 1
    # And it does NOT fire on a receiver the table exempts, which is the half a
    # gate like this gets wrong: a rule that catches everything catches the
    # element stack too, and then it gets disabled.
    exempt = "        while let Some(step) = stack.pop() {"
    if receiver(exempt.split("//", 1)[0]) != "stack":
        print(f"SELF-TEST FAIL: receiver not read from an exempt line: {exempt.strip()}")
        failures += 1
    # Prose mentioning a rewind is not a rewind. Each of these is a shape that
    # appears in real Rust and that a naive `//`-only strip would report — the
    # over-refusal this gate is built to avoid, which would fail `make check` on
    # correct code and teach authors to route around it.
    prose = (
        ("line comment", "        // the old spelling called out.truncate(start) here"),
        ("block comment", "        /* out.truncate(start) */ let x = 1;"),
        ("nested block comment", "        /* outer /* out.pop() */ still comment */"),
        ("string literal", '        let message = "call out.truncate(0) to reset";'),
        ("raw string", '        let pattern = r"out.pop() is not a rewind here";'),
        ("hashed raw string", '        let pattern = r#"out.clear() inside "quotes""#;'),
        ("trailing comment after code", "        let n = v.len(); // v.pop() explained"),
    )
    for name, line in prose:
        stripped = strip_noncode(line)
        if stripped and REWIND.search(stripped[0]):
            print(f"SELF-TEST FAIL: fired on a {name}: {line.strip()}")
            failures += 1

    # And the other direction: stripping must not blind the gate to real code on a
    # line that ALSO carries prose, or a rewind hides behind a comment.
    mixed = '        out.truncate(start); // retract the header'
    stripped = strip_noncode(mixed)
    if not (stripped and REWIND.search(stripped[0])):
        print(f"SELF-TEST FAIL: stripping hid real code: {mixed.strip()}")
        failures += 1

    # A lifetime is not a char literal; treating `\'a` as opening a string would
    # swallow the rest of the line and hide anything after it.
    lifetime = "        fn f<'a>(out: &'a mut String) { out.truncate(0); }"
    stripped = strip_noncode(lifetime)
    if not (stripped and REWIND.search(stripped[0])):
        print(f"SELF-TEST FAIL: a lifetime blinded the scan: {lifetime.strip()}")
        failures += 1
    if failures:
        return 1
    print("OK: check-serializer-rewinds self-test")
    return 0


def main() -> int:
    if "--self-test" in sys.argv:
        return self_test()

    findings, used = scan()
    stale = sorted(set(ALLOWLIST) - used)

    for finding in findings:
        print(f"REWIND: {finding}")
    for key in stale:
        print(
            f"STALE ALLOWLIST: {key[0]} no longer rewinds `{key[1]}`; "
            f"delete the entry rather than leaving a standing exemption"
        )

    if findings:
        print(
            f"\n{len(findings)} rewind(s) on an emitted document. A streaming "
            f"emitter cannot un-write a drained byte, so this works only while "
            f"the document stays under the drain window. Decide before emitting, "
            f"or add a reasoned ALLOWLIST entry naming why the receiver is not "
            f"output."
        )
    if stale:
        print(f"\n{len(stale)} stale allowlist entr(ies).")
    if findings or stale:
        return 1

    print(f"OK: no serializer rewinds ({len(ALLOWLIST)} reasoned exemptions, all live)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
