#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Reject a translated unit that renders a glossary term the way the glossary says not to,
and a translation that drops a keep-English term.

The zh-Hans glossary (``docs/book/po/glossary-zh-Hans.md``) is a gate input, not a page.
Roughly a third of the terms PurRDF's documentation turns on have no established mainland
rendering, and for several of the rest two renderings circulate (蕴涵 and 蕴含 for
*entailment*, 规范化 and 标准化 for *canonicalization*). A reader who meets both concludes
they are two concepts. The glossary settles one rendering per term and lists, per term,
the renderings that are wrong FOR THAT TERM.

A rejected rendering is wrong only when it renders the glossary's English term: 标准化
is wrong for *canonicalization* and the only right word for *standardized* — which the
English book says ("no standardized spelling exists"); 蕴含 is wrong for *entailment*
and right as the ordinary verb *implies*. A substring test over a ``msgstr`` cannot tell
those apart, and the first version of this gate refused all of them. So every rejection is
ANCHORED: it is tested against a ``msgstr`` only when the ``msgid`` carries the row's
English term (the **Anchor** column). The ``.po`` format gives that pairing for free. A
row with no anchor is GLOBAL — its rejections apply wherever they appear — and only the
zh-Hant-register words qualify; the row's note must say so.

Two further rules, each the mirror of an over-refusal or a hole the first version had:

* code spans and fenced blocks inside a ``msgstr`` are never matched (「不要写 `蕴含`」 is
  prose about a spelling, not the spelling);
* a **K** row (keep English) is enforced, not merely stated: when a ``msgid`` carries one
  of its Anchor tokens (case-sensitively, at a word start), the ``msgstr`` must carry it
  verbatim, so 研究物件 for *Research Object* or 吉猫协议 for *GMEOW* is refused although
  no Rejected entry names it.

Translated units are:

* every ``msgstr`` in ``docs/book/po/zh-Hans.po`` that ``mdbook-gettext`` would render —
  non-empty, not fuzzy, not obsolete (a fuzzy or obsolete entry renders as English, so a
  rejected rendering in it is not published and is not refused) — paired with its
  ``msgid``;
* every line of every tracked Markdown file with ``zh-Hans`` in its path (a
  ``README.zh-Hans.md`` sibling, a paragraph-aligned draft under ``docs/book/po/zh-Hans/``
  awaiting its pour into the catalogue), enumerated by ``git ls-files`` like the other
  prose gates — the glossary itself excepted. A file has no ``msgid``, so it is checked
  against the GLOBAL rows only; the pour is where the table is fully enforced.

    python3 scripts/check-i18n-glossary.py               # verify (exit 1 on a hit)
    python3 scripts/check-i18n-glossary.py --self-test   # prove every rule bites both ways
    python3 scripts/check-i18n-glossary.py --po PATH     # a different catalogue

The self-test runs before every scan. For every rejection it executes: the refused form
under an anchored ``msgid`` (refused); the row's own rendering under the same ``msgid``
(passes); the rejected word in its OTHER sense under an unrelated ``msgid`` — an ordinary
Chinese sentence, kept in :data:`NEIGHBOURS` (passes); and the rejected word inside a code
span under the anchored ``msgid`` (passes). For every K token: a translation that drops it
(refused) and one that keeps it (passes). A rejection with no neighbour, a neighbour that
does not actually contain the rejected form, a ``/regex/`` whose derived specimen it does
not match, or a glossary that refuses one of its own renderings is a hard error — so the
table cannot grow a refusal that is proven only one way.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import po_catalog  # noqa: E402 — the sibling module, found via the line above

_REPO = Path(__file__).resolve().parent.parent
PO_PATH = _REPO / "docs" / "book" / "po" / "zh-Hans.po"
GLOSSARY_PATH = _REPO / "docs" / "book" / "po" / "glossary-zh-Hans.md"
TRANSLATED_PATH_MARK = "zh-Hans"

_HEADER_CELLS = ("#", "Term", "Anchor", "Rendering", "Basis", "Rejected", "Note")
_NONE_MARKERS = {"", "—", "-", "–"}
_SEPARATOR = "、"


@dataclass(frozen=True)
class Rejection:
    """One rendering the glossary refuses for one term."""

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
    anchors: tuple[str, ...]
    rendering: str
    basis: str
    rejected: tuple[Rejection, ...]
    note: str

    @property
    def is_global(self) -> bool:
        return not self.anchors

    @property
    def keep_english(self) -> bool:
        return self.basis.strip().startswith("K")

    def anchored_in(self, msgid: str) -> bool:
        """Whether one of the row's anchors appears in ``msgid`` (case-insensitive, at a
        word start, as a prefix — ``entail`` covers ``entailment``)."""
        return any(_anchor_re(a, case_sensitive=False).search(msgid) for a in self.anchors)


def _anchor_re(anchor: str, *, case_sensitive: bool) -> re.Pattern[str]:
    return re.compile(r"(?<![A-Za-z0-9])" + re.escape(anchor), 0 if case_sensitive else re.I)


def split_cells(line: str) -> list[str]:
    """The cells of a Markdown table row, honouring backtick spans and ``\\|``.

    ``|`` inside a code span is content (the annotation syntax ``{| |}`` sits in one cell
    of the glossary), and ``\\|`` is a literal pipe inside a cell (the regex alternation in
    a Rejected entry), so the split tracks both.
    """
    cells: list[str] = []
    buf: list[str] = []
    in_code = False
    escaped = False
    for c in line.strip():
        if escaped:
            buf.append(c)
            escaped = False
        elif c == "\\":
            escaped = True
        elif c == "`":
            in_code = not in_code
            buf.append(c)
        elif c == "|" and not in_code:
            cells.append("".join(buf).strip())
            buf = []
        else:
            buf.append(c)
    cells.append("".join(buf).strip())
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


def _list_cell(cell: str) -> tuple[str, ...]:
    if cell.strip() in _NONE_MARKERS:
        return ()
    return tuple(p for p in (_strip_code(x) for x in cell.split(_SEPARATOR)) if p not in _NONE_MARKERS)


def parse_rejected(term: str, rendering: str, cell: str) -> tuple[Rejection, ...]:
    out: list[Rejection] = []
    for spelled in _list_cell(cell):
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
            if line.lstrip().startswith("|") and tuple(split_cells(line)) == _HEADER_CELLS
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
        _, term, anchor, rendering, basis, rejected, note = cells
        if not term or not rendering:
            raise SystemExit(
                f"check-i18n-glossary: glossary row with an empty term or rendering: "
                f"{line.strip()!r}"
            )
        rows.append(
            Row(term, _list_cell(anchor), rendering, basis, parse_rejected(term, rendering, rejected), note)
        )
    if not rows:
        raise SystemExit("check-i18n-glossary: the glossary table has no rows")
    return rows


# How a self-test specimen is derived from a ``/regex/`` rejection: the lookarounds are
# removed and the quantified classes the glossary uses are given a concrete value.
# ``check_consistency`` asserts the derived specimen really matches its regex, so a regex
# this table cannot derive a specimen for is a hard error rather than a rejection the
# self-test never exercises.
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
    """The table's own invariants, each a hard error."""
    for row in rows:
        if row.rejected and row.is_global and "global" not in row.note.lower():
            raise SystemExit(
                f"check-i18n-glossary: row {row.term!r} has rejections and no Anchor, which "
                f"makes them GLOBAL, but its Note does not say so or why"
            )
        if row.keep_english and row.is_global:
            raise SystemExit(
                f"check-i18n-glossary: row {row.term!r} is K (keep English) but names no "
                f"Anchor token to keep"
            )
        for rejection in row.rejected:
            probe = specimen(rejection)
            if rejection.find(probe) is None:
                raise SystemExit(
                    f"check-i18n-glossary: row {row.term!r} rejects {rejection.spelled!r}, "
                    f"but the specimen derived for it ({probe!r}) does not match it, so the "
                    f"self-test could never prove it bites. Spell the regex with the "
                    f"constructs the specimen derivation knows (lookarounds, \\d, \\s)"
                )
            for other in rows:
                hit = rejection.find(_strip_code(other.rendering))
                if hit:
                    raise SystemExit(
                        f"check-i18n-glossary: the glossary refuses itself — row "
                        f"{row.term!r} rejects {rejection.spelled!r}, which matches {hit!r} "
                        f"inside row {other.term!r}'s rendering {other.rendering!r}. Spell "
                        f"the rejection as a /regex/ that excludes the right rendering, or "
                        f"drop it"
                    )


# ── Markdown code, which is never prose ───────────────────────────────────────


def _inline_code_spans(line: str) -> list[tuple[int, int]]:
    """``(start, end)`` of every inline code span: a backtick run of N opens a span,
    closed by the next run of exactly N (the same rule the other prose gates apply)."""
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
        run = j - i
        k = j
        while k < n:
            if line[k] != "`":
                k += 1
                continue
            m = k
            while m < n and line[m] == "`":
                m += 1
            if m - k == run:
                spans.append((i, m))
                i = m
                break
            k = m
        else:
            i = j
    return spans


def strip_code(text: str) -> str:
    """``text`` with fenced blocks and inline code spans removed."""
    out: list[str] = []
    fenced = False
    for line in text.splitlines():
        if re.match(r"\s*(?:```+|~~~+)", line):
            fenced = not fenced
            continue
        if fenced:
            continue
        kept = []
        last = 0
        for start, end in _inline_code_spans(line):
            kept.append(line[last:start])
            last = end
        kept.append(line[last:])
        out.append("".join(kept))
    return "\n".join(out)


# ── The rule ──────────────────────────────────────────────────────────────────


def offences(label: str, msgid: str | None, msgstr: str, rows: list[Row]) -> list[str]:
    """Every way ``msgstr`` breaks the glossary for ``msgid`` (``None`` for a file line)."""
    out: list[str] = []
    prose = strip_code(msgstr)
    for row in rows:
        applies = row.is_global or (msgid is not None and row.anchored_in(msgid))
        if applies:
            for rule in row.rejected:
                hit = rule.find(prose)
                if hit is not None:
                    out.append(
                        f"{label}: {hit!r} is a rejected rendering of {rule.term!r} — the "
                        f"glossary says {rule.rendering!r} (docs/book/po/glossary-zh-Hans.md)"
                    )
        if row.keep_english and msgid is not None:
            for token in row.anchors:
                if _anchor_re(token, case_sensitive=True).search(msgid) and token not in msgstr:
                    out.append(
                        f"{label}: the keep-English term {token!r} in the msgid does not "
                        f"survive into the msgstr — glossary row {row.term!r} keeps it "
                        f"verbatim (docs/book/po/glossary-zh-Hans.md)"
                    )
    return out


def po_units(po_path: Path) -> list[tuple[str, str | None, str]]:
    entries = po_catalog.messages(po_catalog.parse_po(po_path.read_text(encoding="utf-8")))
    rel = po_path.relative_to(_REPO) if po_path.is_relative_to(_REPO) else po_path
    return [
        (f"{rel}:{e.line} (msgid {e.msgid[:48]!r})", e.msgid, e.msgstr)
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


def file_units(path: Path) -> list[tuple[str, str | None, str]]:
    rel = path.relative_to(_REPO)
    return [
        (f"{rel}:{number}", None, line)
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1)
    ]


# ── This gate's own falsifiability ────────────────────────────────────────────

# For every rejected rendering, an ordinary Chinese sentence that uses the rejected word
# in its OTHER sense — the sentence a translator writes on a page about something else,
# which the gate must not refuse. Keyed by the Rejected cell's spelling. A global row's
# neighbour is a near miss instead (it must NOT contain the rejected form), because a
# global rejection is wrong wherever it appears and has no other sense to spare.
NEIGHBOURS: dict[str, str] = {
    "具名图": "作者具名发表了这篇文章。",  # global near miss: 具名 alone
    "资料集": "参考资料见附录。",  # global near miss: 资料 alone
    "资料类型": "参考资料见附录。",  # global near miss
    "空白节点": "网页模板中的空白节点会被忽略。",  # an empty DOM node
    "字面值": "该常量的字面值为 42。",  # a constant's literal value
    "语言标记": "编辑器根据语言标记进行语法高亮。",  # an editor's language marker
    "本体论": "本体论是哲学的一个分支。",  # philosophy
    "/知识图(?!谱)/": "这张知识图示意了课程结构。",  # a knowledge diagram
    "蕴含": "这一设计蕴含着一个假设。",  # implies
    "实体化": "该抽象概念被实体化为一个类。",  # reified into a class
    "标准化": "RDF 1.2 已由 W3C 标准化。",  # standardized
    "决定性": "这是决定性因素。",  # decisive
    "出处": "引文出处见脚注。",  # a citation's source
    "三元组术语": "本节解释三元组术语的由来。",  # the terminology of triples
    "三元组词项": "三元组词项在逻辑学教材中另有含义。",
    "基本方向": "设计的基本方向是确定性。",  # basic direction
    "组合数据类型": "C 语言中的结构体是一种组合数据类型。",  # an aggregate type in C
    "/(?<!台)账本/": "区块链是一种分布式账本。",  # a blockchain ledger
    "核外": "核外电子决定了元素的化学性质。",  # extranuclear electrons
    "/研究对象(?![（(]Research Object)/": "本研究的研究对象为大学生。",  # the object of study
    "/表面(?!上|看来|来看)/": "水的表面张力很大。",  # surface tension
    "铸造": "青铜器由铸造而成。",  # bronze casting
    "抵达": "列车准时抵达车站。",  # a train arriving
    "大声": "请勿大声喧哗。",  # loudly
    "全文搜索": "本站提供全文搜索功能。",  # a website's search box
    "校验报告": "文件校验报告显示哈希一致。",  # a checksum verification report
}

# A msgid no row anchors — asserted, not assumed, in ``self_test``.
_UNRELATED_MSGID = "The weather was fine today."


def _po_text(msgid: str, msgstr: str, *, fuzzy: bool = False, obsolete: bool = False) -> str:
    prefix = "#~ " if obsolete else ""
    flag = "#, fuzzy\n" if fuzzy else ""
    return (
        'msgid ""\nmsgstr ""\n"Content-Type: text/plain; charset=UTF-8\\n"\n\n'
        f'{flag}{prefix}msgid "{po_catalog.escape(msgid)}"\n'
        f'{prefix}msgstr "{po_catalog.escape(msgstr)}"\n'
    )


def _units_of(po_text: str) -> list[tuple[str, str | None, str]]:
    entries = po_catalog.messages(po_catalog.parse_po(po_text))
    return [("probe", e.msgid, e.msgstr) for e in entries if e.translated]


def self_test(rows: list[Row], report: bool) -> list[str]:
    """Every rule, both ways, executed through :func:`offences`. Empty is the only pass."""
    problems: list[str] = []
    rules = [r for row in rows for r in row.rejected]
    if not rules:
        return ["the glossary lists no rejected rendering, so this gate refuses nothing"]
    if any(row.anchored_in(_UNRELATED_MSGID) for row in rows):
        return [f"the unrelated msgid {_UNRELATED_MSGID!r} is anchored by a row — pick another"]

    def verdict(what: str, po_text: str, must_refuse: bool) -> None:
        found = [o for label, msgid, msgstr in _units_of(po_text) for o in offences(label, msgid, msgstr, rows)]
        ok = bool(found) is must_refuse
        if report:
            print(f"  {'ok' if ok else 'WRONG':5}  {'refused' if found else 'passes '}  {what}")
        if not ok:
            problems.append(
                f"{'NOT REFUSED' if must_refuse else 'FALSELY REFUSED'}: {what}"
                + (f" -> {found}" if found else "")
            )

    for row in rows:
        anchored = _UNRELATED_MSGID if row.is_global else f"This paragraph is about {row.anchors[0]}."
        for rule in row.rejected:
            probe = specimen(rule)
            neighbour = NEIGHBOURS.get(rule.spelled)
            if neighbour is None:
                problems.append(
                    f"NO NEIGHBOUR: {row.term} rejects {rule.spelled!r} and NEIGHBOURS has no "
                    f"ordinary sentence for it — a refusal proven only one way"
                )
                continue
            if row.is_global:
                if rule.find(neighbour) is not None:
                    problems.append(
                        f"BAD NEIGHBOUR: {row.term} is global, so its neighbour must be a near "
                        f"miss, but {neighbour!r} contains {rule.spelled!r}"
                    )
                    continue
                verdict(
                    f"{row.term} (GLOBAL): {probe!r} under an unrelated msgid",
                    _po_text(_UNRELATED_MSGID, f"本页使用{probe}一词。"),
                    True,
                )
                verdict(
                    f"{row.term} (GLOBAL): the near miss {neighbour!r}",
                    _po_text(_UNRELATED_MSGID, neighbour),
                    False,
                )
            else:
                if rule.find(neighbour) is None:
                    problems.append(
                        f"BAD NEIGHBOUR: {neighbour!r} does not contain {rule.spelled!r}, so it "
                        f"proves nothing about {row.term}'s rejection"
                    )
                    continue
                verdict(
                    f"{row.term}: {probe!r} under an anchored msgid ({row.anchors[0]!r})",
                    _po_text(anchored, f"本页使用{probe}一词。"),
                    True,
                )
                verdict(
                    f"{row.term}: the other sense — {neighbour!r} under an unrelated msgid",
                    _po_text(_UNRELATED_MSGID, neighbour),
                    False,
                )
            # A K row's probe keeps the invariant token beside the code span, so the
            # survival arm has nothing to say and only the code-span rule is tested.
            keep = f"（{row.anchors[0]}）" if row.keep_english else ""
            verdict(
                f"{row.term}: {probe!r} inside a code span under the anchored msgid",
                _po_text(anchored, f"不要写 `{probe}`{keep}。"),
                False,
            )
        if row.rejected:
            rendering = _strip_code(row.rendering)
            if rendering != "as written":
                verdict(
                    f"{row.term}: the glossary rendering {rendering!r} under the anchored msgid",
                    _po_text(anchored, f"本页使用 {rendering} 一词。"),
                    False,
                )
        if row.keep_english:
            for token in row.anchors:
                verdict(
                    f"{row.term}: {token!r} in the msgid, DROPPED from the msgstr",
                    _po_text(f"The {token} toolkit is here.", "该工具包在此。"),
                    True,
                )
                verdict(
                    f"{row.term}: {token!r} in the msgid, kept in the msgstr",
                    _po_text(f"The {token} toolkit is here.", f"该 {token} 工具包在此。"),
                    False,
                )
            verdict(
                f"{row.term}: no token in the msgid, nothing to keep",
                _po_text("The toolkit is here.", "该工具包在此。"),
                False,
            )

    probe = specimen(rules[0])
    anchored = f"This paragraph is about {next(r for r in rows if r.rejected and not r.is_global).anchors[0]}."
    verdict("an untranslated entry (empty msgstr)", _po_text(anchored, ""), False)
    verdict("an English msgstr", _po_text(anchored, anchored), False)
    verdict(
        f"a FUZZY entry carrying {probe!r} (not rendered, so not refused)",
        _po_text(anchored, f"本页使用{probe}一词。", fuzzy=True),
        False,
    )
    verdict(
        f"an OBSOLETE entry carrying {probe!r} (not rendered, so not refused)",
        _po_text(anchored, f"本页使用{probe}一词。", obsolete=True),
        False,
    )
    # The sentences from the review that the first version refused, and the book's own.
    for what, msgid, msgstr in (
        (
            "the book's own sentence: 'no standardized spelling exists'",
            "Other engines already ship a form of it, but no standardized spelling exists.",
            "其他引擎已经提供了某种形式，但不存在标准化的拼写。",
        ),
        (
            "a faithful sentence keeping every invariant",
            "PurRDF is an RDF 1.2 toolkit in Rust with Python and WebAssembly bindings.",
            "PurRDF 是一个用 Rust 编写的 RDF 1.2 工具包，提供 Python 与 WebAssembly 绑定。",
        ),
        (
            "the gloss form with the acronym",
            "Research Object projections",
            "研究对象（Research Object，RO）投影",
        ),
    ):
        verdict(what, _po_text(msgid, msgstr), False)
    for what, msgid, msgstr in (
        ("Research Object rendered as 研究物件", "Research Object projections", "研究物件（RO）投影"),
        ("GMEOW translated", "The GMEOW ontology.", "吉猫协议本体。"),
        ("IRI translated in running text", "Every IRI is absolute.", "每个国际化资源标识符都是绝对的。"),
    ):
        verdict(what, _po_text(msgid, msgstr), True)
    return problems


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true", help="prove the rules, then stop")
    ap.add_argument("--po", type=Path, default=PO_PATH, help="the catalogue to check")
    ap.add_argument("--glossary", type=Path, default=GLOSSARY_PATH)
    args = ap.parse_args(argv)

    rows = parse_glossary(args.glossary.read_text(encoding="utf-8"))
    check_consistency(rows)
    rules = [r for row in rows for r in row.rejected]
    tokens = [t for row in rows if row.keep_english for t in row.anchors]

    if args.self_test:
        print("check-i18n-glossary: proving every rule bites, and only bites —")
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
            f"OK: {len(rules)} rejected rendering(s) across {len(rows)} glossary row(s) "
            f"({sum(1 for r in rows if r.rejected and r.is_global)} global), each refused "
            f"under its anchor and spared in its other sense; {len(tokens)} keep-English "
            f"token(s), each refused when dropped."
        )
        return 0

    if not args.po.is_file():
        print(f"check-i18n-glossary: no catalogue at {args.po}", file=sys.stderr)
        return 1
    units = po_units(args.po)
    files = tracked_translated_files()
    for path in files:
        units.extend(file_units(path))

    found = [o for label, msgid, msgstr in units for o in offences(label, msgid, msgstr, rows)]
    if found:
        print(
            "check-i18n-glossary: translated text breaks the glossary:\n"
            + "\n".join(f"  - {o}" for o in found),
            file=sys.stderr,
        )
        return 1
    po_label = args.po.relative_to(_REPO) if args.po.is_relative_to(_REPO) else args.po
    print(
        f"OK: {len(rules)} rejected rendering(s) and {len(tokens)} keep-English token(s) from "
        f"{len(rows)} glossary row(s) respected by {len(units)} translated unit(s) ({po_label} "
        f"plus {len(files)} tracked {TRANSLATED_PATH_MARK} Markdown file(s), global rows only)."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
