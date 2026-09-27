#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Validate ``PROVENANCE.toml`` files: where a first-party module's knowledge came from.

A module written to replace a third-party crate states, beside its source, what it
was written FROM and what proves it right. This validator holds every such file to
one schema. ``scripts/check-licenses.py`` runs it over every tracked
``PROVENANCE.toml``.

SCHEMA (unknown keys anywhere are an error)::

    statement = "Written from RFC 3986 and the answer vectors; the replaced crate's source was not consulted."
    replaced = ["some-crate"]            # optional: crate names this module replaces

    [module]
    paths = ["src/resolve.rs"]           # one or more; relative to this file's directory

    [[sources]]                          # one or more: what the module was written from
    kind = "spec"                        # spec | isa | paper | std-doc | repo | data
    id = "RFC 3986"
    sections = ["5.2", "5.4"]            # one or more

    [[oracles]]                          # one or more: what proves the module right
    kind = "answer-vectors"              # answer-vectors | official-corpus | model | property | node
    path_or_id = "tests/fixtures/resolve.json"

RULES:

* ``[module] paths`` is a non-empty list of non-empty strings, each naming a file or
  directory that exists relative to the ``PROVENANCE.toml``'s own directory.
* ``sources`` and ``oracles`` are non-empty arrays of tables with exactly the keys
  above; ``kind`` is one of the listed values; ``id``/``path_or_id`` are non-empty;
  ``sections`` is a non-empty list of non-empty strings.
* ``replaced``, when present, is a list of non-empty crate names.
* ``statement`` is a non-empty string, and when ``replaced`` is non-empty it must
  contain the literal words ``not consulted``: a module that replaces a crate states
  that the crate's source was not consulted, or it does not claim to be clean-room.

    python3 scripts/cleanroom/provenance.py path/to/PROVENANCE.toml ...
    python3 scripts/cleanroom/provenance.py --self-test
"""

from __future__ import annotations

import argparse
import shutil
import sys
import tomllib
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SOURCE_KINDS = frozenset({"spec", "isa", "paper", "std-doc", "repo", "data"})
ORACLE_KINDS = frozenset({"answer-vectors", "official-corpus", "model", "property", "node"})
TOP_KEYS = frozenset({"module", "sources", "replaced", "oracles", "statement"})
CLEAN_ROOM_PHRASE = "not consulted"


def _non_empty_strings(value: object) -> bool:
    return isinstance(value, list) and bool(value) and all(isinstance(item, str) and item.strip() for item in value)


def _tables(document: dict, key: str, kinds: frozenset[str], fields: tuple[str, ...], problems: list[str]) -> None:
    entries = document.get(key)
    if not isinstance(entries, list) or not entries:
        problems.append(f"`{key}` must be a non-empty array of tables")
        return
    for index, entry in enumerate(entries):
        where = f"{key}[{index}]"
        if not isinstance(entry, dict):
            problems.append(f"{where} must be a table")
            continue
        expected = {"kind", *fields}
        if set(entry) != expected:
            problems.append(f"{where} must have exactly the keys {sorted(expected)}, has {sorted(entry)}")
            continue
        if entry["kind"] not in kinds:
            problems.append(f"{where}.kind {entry['kind']!r} is not one of {sorted(kinds)}")
        for name in fields:
            value = entry[name]
            if name == "sections":
                if not _non_empty_strings(value):
                    problems.append(f"{where}.sections must be a non-empty list of non-empty strings")
            elif not isinstance(value, str) or not value.strip():
                problems.append(f"{where}.{name} must be a non-empty string")


def problems_in(path: Path) -> list[str]:
    """Every way the ``PROVENANCE.toml`` at *path* departs from the schema."""
    try:
        document = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, tomllib.TOMLDecodeError) as error:
        return [f"cannot be read as TOML: {error}"]
    problems: list[str] = []
    unknown = sorted(set(document) - TOP_KEYS)
    if unknown:
        problems.append(f"unknown key(s): {unknown}")

    module = document.get("module")
    if not isinstance(module, dict):
        problems.append("`[module]` table is required")
    elif set(module) != {"paths"}:
        problems.append(f"`[module]` must have exactly the key `paths`, has {sorted(module)}")
    elif not _non_empty_strings(module["paths"]):
        problems.append("`module.paths` must be a non-empty list of non-empty strings")
    else:
        for entry in module["paths"]:
            if not (path.parent / entry).exists():
                problems.append(f"`module.paths` entry {entry!r} does not exist beside this file")

    _tables(document, "sources", SOURCE_KINDS, ("id", "sections"), problems)
    _tables(document, "oracles", ORACLE_KINDS, ("path_or_id",), problems)

    replaced = document.get("replaced", [])
    if not isinstance(replaced, list) or not all(isinstance(item, str) and item.strip() for item in replaced):
        problems.append("`replaced` must be a list of non-empty crate names")
        replaced = []
    statement = document.get("statement")
    if not isinstance(statement, str) or not statement.strip():
        problems.append("`statement` must be a non-empty string")
    elif replaced and CLEAN_ROOM_PHRASE not in statement:
        problems.append(
            f"`replaced` names {replaced}, so `statement` must say the replaced source was "
            f"{CLEAN_ROOM_PHRASE!r}"
        )
    return problems


def check(paths: list[Path]) -> list[str]:
    """Every problem in every file of *paths*, prefixed by the file."""
    return [f"{path}: {problem}" for path in paths for problem in problems_in(path)]


COMPLETE = """\
statement = "Written from RFC 3986 section 5; the replaced crate's source was not consulted."
replaced = ["example-crate"]

[module]
paths = ["src/resolve.rs"]

[[sources]]
kind = "spec"
id = "RFC 3986"
sections = ["5.2"]

[[oracles]]
kind = "answer-vectors"
path_or_id = "vectors.json"
"""


def self_test() -> int:
    """Each refusal beside the complete neighbour it must accept."""
    scratch = REPO_ROOT / "target" / "cleanroom-selftest" / "provenance"
    if scratch.exists():
        shutil.rmtree(scratch)
    (scratch / "src").mkdir(parents=True)
    (scratch / "src" / "resolve.rs").write_text("", encoding="utf-8")
    target = scratch / "PROVENANCE.toml"
    no_sources = "\n".join(
        line for line in COMPLETE.split("\n\n") if not line.startswith("[[sources]]")
    )
    cases = [
        ("a complete file", COMPLETE, None),
        ("a file without `sources`", no_sources, "`sources` must be"),
        (
            "`replaced` without the clean-room statement",
            COMPLETE.replace("was not consulted", "was read closely"),
            "must say the replaced source",
        ),
        (
            "the same statement with `replaced` empty",
            COMPLETE.replace("was not consulted", "was read closely").replace('["example-crate"]', "[]"),
            None,
        ),
        ("a module path that does not exist", COMPLETE.replace("src/resolve.rs", "src/gone.rs"), "does not exist"),
        ("an unknown source kind", COMPLETE.replace('kind = "spec"', 'kind = "blog"'), "is not one of"),
    ]
    ok = True
    for label, text, needle in cases:
        target.write_text(text, encoding="utf-8")
        found = problems_in(target)
        if needle is None and not found:
            print(f"OK: self-test — {label} is accepted")
        elif needle is not None and any(needle in problem for problem in found):
            print(f"OK: self-test — {label} is refused")
        else:
            print(f"SELF-TEST FAIL: {label}: {found}")
            ok = False
    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("files", nargs="*", type=Path)
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    if not args.files:
        parser.error("name at least one PROVENANCE.toml")
    problems = check(args.files)
    for problem in problems:
        print(problem, file=sys.stderr)
    if problems:
        return 1
    print(f"OK: {len(args.files)} PROVENANCE.toml file(s) follow the schema")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
