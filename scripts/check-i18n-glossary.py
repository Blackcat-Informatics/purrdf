#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Reject a translated unit that renders a glossary term the way the glossary says not to.

The zh-Hans glossary (``docs/book/po/glossary-zh-Hans.md``) is a gate input, not a page.
Roughly a third of the terms PurRDF's documentation turns on have no established mainland
rendering, and for several of the rest two renderings circulate (蕴涵 and 蕴含 for
*entailment*, 规范化 and 标准化 for *canonicalization*). A reader who meets both concludes
they are two concepts. The glossary settles one rendering per term and lists, per term,
the renderings that are WRONG wherever they appear; this gate refuses any translated unit
that uses one.

Translated units are:

* every ``msgstr`` in ``docs/book/po/zh-Hans.po`` that ``mdbook-gettext`` would render —
  non-empty, not fuzzy, not obsolete (a fuzzy or obsolete entry renders as English, so a
  rejected rendering in it is not published and is not refused);
* every line of every tracked Markdown file with ``zh-Hans`` in its path — a
  ``README.zh-Hans.md`` sibling, a paragraph-aligned draft under
  ``docs/book/po/zh-Hans/`` awaiting its pour into the catalogue — enumerated by
  ``git ls-files`` like the other prose gates; the glossary itself excepted, since it
  lists the rejected renderings by design.

The glossary is read by its header row. A **Rejected** entry is a plain substring, or a
``/…/`` regular expression where the wrong rendering is a substring of a right one (bare
闭包 inside 推理闭包). The table is checked for self-consistency before it is applied: a
rejected entry that appears inside any row's own rendering would make the glossary refuse
itself, and is a hard error naming the two rows.

    python3 scripts/check-i18n-glossary.py               # verify (exit 1 on a hit)
    python3 scripts/check-i18n-glossary.py --self-test   # prove the rule bites both ways
    python3 scripts/check-i18n-glossary.py --po PATH     # a different catalogue

The self-test runs before every scan: each rejected rendering the glossary lists is
injected into a ``msgstr`` and must be refused, and the glossary's own rendering for the
same term, an untranslated entry, an English ``msgstr`` and a fuzzy entry carrying the
rejected rendering must all pass. A glossary with no rejected entries at all is a
self-test failure — the gate would then be proving nothing.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

_REPO = Path(__file__).resolve().parent.parent
PO_PATH = _REPO / "docs" / "book" / "po" / "zh-Hans.po"
GLOSSARY_PATH = _REPO / "docs" / "book" / "po" / "glossary-zh-Hans.md"
TRANSLATED_PATH_MARK = "zh-Hans"

_HEADER_CELLS = ("#", "Term", "Rendering", "Basis", "Rejected", "Note")
_NONE_MARKERS = {"", "—", "-", "–"}


sys.path.insert(0, str(Path(__file__).resolve().parent))
import po_catalog  # noqa: E402 — the sibling module, found via the line above


@dataclass(frozen=True)
class Rejection:
    """One rendering the glossary refuses, and the row that refuses it."""

    term: str
    rendering: str
    spelled: str
    pattern: re.Pattern[str]

    def find(self, text: str) -> str | None:
        match = self.pattern.search(text)
        return match.group(0) if match else None


@dataclass(frozen=True)
class Row:
    term: str
    rendering: str
    rejected: tuple[Rejection, ...]


def split_cells(line: str) -> list[str]:
    """The cells of a Markdown table row, honouring backtick spans.

    ``|`` inside a code span is content (the annotation syntax ``{| |}`` sits in
    one cell of the glossary), so the split tracks backtick state.
    """
    cells: list[str] = []
    buf: list[str] = []
    in_code = False
    for c in line.strip():
        if c == "`":
            in_code = not in_code
            buf.append(c)
        elif c == "|" and not in_code:
            cells.append("".join(buf).strip())
            buf = []
        else:
            buf.append(c)
    cells.append("".join(buf).strip())
    # A row is written `| a | b |`, so the first and last cells are empty.
    if cells and cells[0] == "":
        cells = cells[1:]
    if cells and cells[-1] == "":
        cells = cells[:-1]
    return cells


def _strip_code(cell: str) -> str:
    cell = cell.strip()
    if len(cell) >= 2 and cell[0] == "`" and cell[-1] == "`":
        return cell[1:-1]
    return cell


def parse_rejected(term: str, rendering: str, cell: str) -> tuple[Rejection, ...]:
    if cell.strip() in _NONE_MARKERS:
        return ()
    out: list[Rejection] = []
    for piece in cell.split("、"):
        spelled = _strip_code(piece)
        if spelled in _NONE_MARKERS:
            continue
        if len(spelled) >= 2 and spelled[0] == "/" and spelled[-1] == "/":
            pattern = re.compile(spelled[1:-1])
        else:
            pattern = re.compile(re.escape(spelled))
        out.append(Rejection(term, rendering, spelled, pattern))
    return tuple(out)


def parse_glossary(text: str) -> list[Row]:
    """Every row of the glossary table, located by its header row."""
    lines = text.splitlines()
    header_at = next(
        (
            i
            for i, line in enumerate(lines)
            if line.lstrip().startswith("|")
            and tuple(split_cells(line)) == _HEADER_CELLS
        ),
        None,
    )
    if header_at is None:
        raise SystemExit(
            f"check-i18n-glossary: no table with the header row {list(_HEADER_CELLS)} in "
            f"{GLOSSARY_PATH.relative_to(_REPO)} — the gate reads the glossary by that row"
        )
    rows: list[Row] = []
    for line in lines[header_at + 2 :]:
        if not line.lstrip().startswith("|"):
            break
        cells = split_cells(line)
        if len(cells) != len(_HEADER_CELLS):
            raise SystemExit(
                f"check-i18n-glossary: glossary row has {len(cells)} cells, expected "
                f"{len(_HEADER_CELLS)}: {line.strip()!r}"
            )
        _, term, rendering, _basis, rejected, _note = cells
        if not term or not rendering:
            raise SystemExit(
                f"check-i18n-glossary: glossary row with an empty term or rendering: "
                f"{line.strip()!r}"
            )
        rows.append(Row(term, rendering, parse_rejected(term, rendering, rejected)))
    if not rows:
        raise SystemExit("check-i18n-glossary: the glossary table has no rows")
    return rows


# How a self-test specimen is derived from a ``/regex/`` rejection: the
# lookarounds are removed and the two quantified classes the glossary uses are
# given a concrete value. ``check_consistency`` asserts the derived specimen
# really matches its regex, so a regex this table cannot derive a specimen for
# is a hard error rather than a rejection the self-test never exercises.
_SPECIMEN_SUBSTITUTIONS = (
    (re.compile(r"\(\?<?[!=].*?\)"), ""),
    (re.compile(r"\\d\+"), "0"),
    (re.compile(r"\\d"), "0"),
    (re.compile(r"\\s\*"), " "),
    (re.compile(r"\\s\+"), " "),
)


def specimen(rule: Rejection) -> str:
    """Text the self-test injects to prove ``rule`` is refused."""
    spelled = rule.spelled
    if not (len(spelled) >= 2 and spelled[0] == "/" and spelled[-1] == "/"):
        return spelled
    text = spelled[1:-1]
    for pattern, replacement in _SPECIMEN_SUBSTITUTIONS:
        text = pattern.sub(replacement, text)
    return text


def check_consistency(rows: list[Row]) -> None:
    """A rejected rendering may not appear inside ANY row's rendering, and every
    ``/regex/`` rejection must match the specimen derived for it."""
    for row in rows:
        for rejection in row.rejected:
            probe = specimen(rejection)
            if rejection.find(probe) is None:
                raise SystemExit(
                    f"check-i18n-glossary: row {row.term!r} rejects {rejection.spelled!r}, "
                    f"but the specimen derived for it ({probe!r}) does not match it, so "
                    f"the self-test could never prove it bites. Spell the regex with the "
                    f"constructs the specimen derivation knows (lookarounds, \\d, \\s)"
                )
            for other in rows:
                hit = rejection.find(_strip_code(other.rendering))
                if hit:
                    raise SystemExit(
                        f"check-i18n-glossary: the glossary refuses itself — row "
                        f"{row.term!r} rejects {rejection.spelled!r}, which matches "
                        f"{hit!r} inside row {other.term!r}'s rendering "
                        f"{other.rendering!r}. Spell the rejection as a /regex/ that "
                        f"excludes the right rendering, or drop it"
                    )


def rejections(rows: list[Row]) -> list[Rejection]:
    return [r for row in rows for r in row.rejected]


def offences(label: str, text: str, rules: list[Rejection]) -> list[str]:
    out: list[str] = []
    for rule in rules:
        hit = rule.find(text)
        if hit is None:
            continue
        out.append(
            f"{label}: {hit!r} is a rejected rendering of {rule.term!r} — the glossary "
            f"says {rule.rendering!r} (docs/book/po/glossary-zh-Hans.md)"
        )
    return out


def po_units(po_path: Path) -> list[tuple[str, str]]:
    entries = po_catalog.messages(po_catalog.parse_po(po_path.read_text(encoding="utf-8")))
    rel = po_path.relative_to(_REPO) if po_path.is_relative_to(_REPO) else po_path
    return [
        (f"{rel}:{e.line} (msgid {e.msgid[:48]!r})", e.msgstr)
        for e in entries
        if e.translated
    ]


def tracked_translated_files() -> list[Path]:
    out = subprocess.run(
        ["git", "-C", str(_REPO), "ls-files", "-z"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    paths = []
    for rel in sorted(p for p in out.split("\0") if p):
        if not (rel.endswith(".md") and TRANSLATED_PATH_MARK in rel):
            continue
        path = _REPO / rel
        if path == GLOSSARY_PATH or not path.is_file():
            continue
        paths.append(path)
    return paths


def file_units(path: Path) -> list[tuple[str, str]]:
    rel = path.relative_to(_REPO)
    return [
        (f"{rel}:{number}", line)
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1)
    ]


# ── This gate's own falsifiability ────────────────────────────────────────────


def _po_with(msgstr: str, *, fuzzy: bool = False, obsolete: bool = False) -> str:
    prefix = "#~ " if obsolete else ""
    flag = "#, fuzzy\n" if fuzzy else ""
    return (
        'msgid ""\nmsgstr ""\n"Content-Type: text/plain; charset=UTF-8\\n"\n\n'
        f"{flag}{prefix}msgid \"Entailment regimes\"\n"
        f"{prefix}msgstr \"{po_catalog.escape(msgstr)}\"\n"
    )


def self_test(rows: list[Row], report: bool) -> list[str]:
    """Every rejected rendering must be refused; every right one must pass."""
    rules = rejections(rows)
    problems: list[str] = []
    if not rules:
        return ["the glossary lists no rejected rendering, so this gate refuses nothing"]

    def units_of(po_text: str) -> list[tuple[str, str]]:
        entries = po_catalog.messages(po_catalog.parse_po(po_text))
        return [("probe", e.msgstr) for e in entries if e.translated]

    def verdict(what: str, po_text: str, must_refuse: bool) -> None:
        found = [o for label, text in units_of(po_text) for o in offences(label, text, rules)]
        ok = bool(found) is must_refuse
        if report:
            print(f"  {'ok' if ok else 'WRONG':5}  {'refused' if found else 'passes '}  {what}")
        if not ok:
            problems.append(
                f"{'NOT REFUSED' if must_refuse else 'FALSELY REFUSED'}: {what}"
                + (f" -> {found}" if found else "")
            )

    for row in rows:
        for rule in row.rejected:
            probe = specimen(rule)
            verdict(
                f"{rule.term}: the rejected rendering {probe!r} in a msgstr",
                _po_with(f"本页使用{probe}一词。"),
                True,
            )
        rendering = _strip_code(row.rendering)
        verdict(
            f"{row.term}: the glossary rendering {rendering!r} in a msgstr",
            _po_with(f"本页使用 {rendering} 一词。"),
            False,
        )
    probe = specimen(rules[0])
    verdict("an untranslated entry (empty msgstr)", _po_with(""), False)
    verdict("an English msgstr", _po_with("Entailment regimes"), False)
    verdict(
        f"a FUZZY entry carrying {probe!r} (not rendered, so not refused)",
        _po_with(f"本页使用{probe}一词。", fuzzy=True),
        False,
    )
    verdict(
        f"an OBSOLETE entry carrying {probe!r} (not rendered, so not refused)",
        _po_with(f"本页使用{probe}一词。", obsolete=True),
        False,
    )
    return problems


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true", help="prove the rule, then stop")
    ap.add_argument("--po", type=Path, default=PO_PATH, help="the catalogue to check")
    ap.add_argument("--glossary", type=Path, default=GLOSSARY_PATH)
    args = ap.parse_args(argv)

    rows = parse_glossary(args.glossary.read_text(encoding="utf-8"))
    check_consistency(rows)
    rules = rejections(rows)

    if args.self_test:
        print("check-i18n-glossary: proving every rejected rendering is refused —")
    problems = self_test(rows, report=args.self_test)
    if problems:
        print(
            "check-i18n-glossary: this gate answers its own cases wrongly:\n"
            + "\n".join(f"  - {p}" for p in problems),
            file=sys.stderr,
        )
        return 1
    if args.self_test:
        print(
            f"OK: {len(rules)} rejected rendering(s) across {len(rows)} glossary row(s), "
            "each refused, each row's own rendering spared."
        )
        return 0

    units: list[tuple[str, str]] = []
    if args.po.is_file():
        units.extend(po_units(args.po))
    else:
        print(f"check-i18n-glossary: no catalogue at {args.po}", file=sys.stderr)
        return 1
    files = tracked_translated_files()
    for path in files:
        units.extend(file_units(path))

    found = [o for label, text in units for o in offences(label, text, rules)]
    if found:
        print(
            "check-i18n-glossary: translated text uses a rendering the glossary rejects:\n"
            + "\n".join(f"  - {o}" for o in found),
            file=sys.stderr,
        )
        return 1
    print(
        f"OK: {len(rules)} rejected rendering(s) from {len(rows)} glossary row(s) absent "
        f"from {len(units)} translated unit(s) ({args.po.relative_to(_REPO) if args.po.is_relative_to(_REPO) else args.po} "
        f"plus {len(files)} tracked {TRANSLATED_PATH_MARK} Markdown file(s))."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
