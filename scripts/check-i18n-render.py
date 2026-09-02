#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Render the zh-Hans book to Markdown and run the prose gates and the SPARQL fence parser
over the rendering.

The book's translation is a gettext catalogue, ``docs/book/po/zh-Hans.po``, applied by the
``mdbook-gettext`` preprocessor at build time. That is the right mechanism — an
untranslated paragraph renders as its English source, so translation lag is visible to
the reader by construction — and it has one consequence this gate exists for: a ``.po``
file is read by NO other gate. ``check-brand-casing.py``, ``check-issue-refs.py``,
``check-spec-attribution.py`` and ``check-doc-claims.py`` enumerate tracked ``.rs``/``.md``
and never open a ``.po``; ``serializer_roundtrip_sweep.rs`` and
``shipped_sparql_examples.rs`` extract fenced SPARQL from ``docs/**/*.md``, never from a
``msgstr``. Without this step a bare ``purrdf`` in Chinese prose, a process token, an
attribution of the quad template (a first-party extension) to SPARQL 1.2, an unscoped
entailment claim, or a broken query inside a fence a translator added would all ship
with every gate green.

So this gate does what the reader's build does — ``mdbook build`` with
``MDBOOK_BOOK__LANGUAGE=zh-Hans`` — with the ``markdown`` renderer added, into a temporary
directory, and then runs over that tree:

1. ``check-brand-casing.py --rendered-tree``
2. ``check-issue-refs.py --rendered-tree``
3. ``check-spec-attribution.py --rendered-tree``
4. ``check-doc-claims.py --rendered-tree`` (the two rule bans; the numeric claims stay
   English by policy — scoreboards are linked, not restated)
5. ``cargo run -p purrdf-sparql-algebra --example sweep_sparql_fences`` — every fenced
   ``sparql`` block must parse as a query or an update.
6. Every translated message must have REACHED the rendering: the longest run of CJK
   characters in each rendered ``msgstr`` must appear in the build output — the
   Markdown tree, or the HTML tree for the messages that exist only in ``SUMMARY.md``
   (part titles, chapter titles), which the ``markdown`` renderer does not emit and
   which land in the HTML sidebar instead. The preprocessor never errors on a message
   it cannot match — it renders the English — so a catalogue entry it silently ignores
   is translation work that was done and is not shown. The everyday case is a STALE
   entry: the English paragraph changed, the catalogue was not re-merged, and a
   hand-edited ``msgid`` no longer exists in the source.

It also asserts BOTH pinned tools: ``mdbook-i18n-helpers`` is installed at exactly
``MDBOOK_I18N_HELPERS_VERSION`` from the Makefile, read from ``cargo install --list``
because the binaries themselves report no version; and ``mdbook`` on PATH reports exactly
``MDBOOK_VERSION`` from ``.github/workflows/docs.yaml`` — the version CI builds with,
whose Markdown re-serialization and search index are what the measurements in
``book.toml`` were taken on. And it reports (report-only, by
design) the catalogue's translated/fuzzy/untranslated counts and how far its msgids have
drifted from a freshly extracted template — the per-release lag numbers the translation
owner reads.

    python3 scripts/check-i18n-render.py              # render and gate (exit 1 on a hit)
    python3 scripts/check-i18n-render.py --self-test  # prove each arm can go red, then gate

The self-test writes NOTHING under the repository. For each arm it builds a throwaway
catalogue containing one poisoned ``msgstr`` — the arm's OWN self-test specimen, imported
from the gate script (a bare ``purrdf`` glued to CJK, a hazard id glued to CJK, the
Chinese attribution of the quad template, the overclaim ban's specimen after a full-width
terminator) plus a fenced ``sparql`` block with a full-width brace written INSIDE a prose
``msgstr``, plus a translation of a ``msgid`` the source does not carry — renders the book
with THAT catalogue (``MDBOOK_PREPROCESSOR__GETTEXT__PO_DIR``), and asserts the arm goes
red. The fence poison takes that shape because
``mdbook-xgettext`` at the pinned version extracts no message for a ``sparql`` fence at
all (only comments and string literals of the languages it lexes), so the English
examples are byte-invariant under translation and the one way a query reaches the
rendering through the catalogue is a translator writing a fence into a paragraph. A gate whose red state has never been observed is a gate
whose green state means nothing.

Run locally as ``make check-i18n``; CI runs it in the docs workflow before building
the zh-Hans book into the Pages artifact.
"""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

_REPO = Path(__file__).resolve().parent.parent
_SCRIPTS = _REPO / "scripts"
_BOOK = _REPO / "docs" / "book"
_PO_DIR = _BOOK / "po"
_PO = _PO_DIR / "zh-Hans.po"
_MAKEFILE = _REPO / "Makefile"
_DOCS_WORKFLOW = _REPO / ".github" / "workflows" / "docs.yaml"
LANGUAGE = "zh-Hans"

# The four prose gates, each with a `--rendered-tree` mode, in the order the
# report reads.
PROSE_GATES = (
    "check-brand-casing.py",
    "check-issue-refs.py",
    "check-spec-attribution.py",
    "check-doc-claims.py",
)
FENCE_SWEEP = (
    "cargo",
    "run",
    "-q",
    "-p",
    "purrdf-sparql-algebra",
    "--example",
    "sweep_sparql_fences",
    "--locked",
    "--",
)


sys.path.insert(0, str(_SCRIPTS))
import po_catalog  # noqa: E402 — the sibling module, found via the line above


def pinned_version() -> str:
    match = re.search(r"^MDBOOK_I18N_HELPERS_VERSION := (\S+)$", _MAKEFILE.read_text(), re.M)
    if not match:
        raise SystemExit("check-i18n-render: MDBOOK_I18N_HELPERS_VERSION is not in the Makefile")
    return match.group(1)


def pinned_mdbook_version() -> str:
    match = re.search(r"^\s*MDBOOK_VERSION:\s*(\S+)\s*$", _DOCS_WORKFLOW.read_text(), re.M)
    if not match:
        raise SystemExit("check-i18n-render: MDBOOK_VERSION is not in .github/workflows/docs.yaml")
    return match.group(1)


def require_tools() -> str:
    """mdbook and mdbook-i18n-helpers on PATH, both at their pins; returns the helpers pin."""
    pin = pinned_version()
    mdbook_pin = pinned_mdbook_version()
    install = f"cargo install mdbook-i18n-helpers --version {pin} --locked"
    for binary in ("mdbook", "mdbook-gettext", "mdbook-xgettext", "cargo"):
        if shutil.which(binary) is None:
            raise SystemExit(
                f"check-i18n-render: `{binary}` is not on PATH — install mdBook and the "
                f"pinned helpers:\n  {install}"
            )
    reported = subprocess.run(["mdbook", "--version"], check=True, capture_output=True, text=True).stdout
    found_mdbook = re.search(r"v?(\d+\.\d+\.\d+)", reported)
    if found_mdbook is None or found_mdbook.group(1) != mdbook_pin:
        raise SystemExit(
            f"check-i18n-render: mdbook version mismatch — `mdbook --version` says "
            f"{reported.strip()!r}, docs.yaml pins {mdbook_pin}. Install the pin, e.g.\n  "
            f"curl -sSfL https://github.com/rust-lang/mdBook/releases/download/v{mdbook_pin}/"
            f"mdbook-v{mdbook_pin}-x86_64-unknown-linux-musl.tar.gz | tar -xz -C ~/.local/bin"
        )
    listed = subprocess.run(
        ["cargo", "install", "--list"], check=True, capture_output=True, text=True
    ).stdout
    found = re.search(r"^mdbook-i18n-helpers v(\S+):$", listed, re.M)
    if found is None:
        raise SystemExit(
            f"check-i18n-render: `cargo install --list` does not report "
            f"mdbook-i18n-helpers (the binaries carry no version of their own) — install "
            f"the pin:\n  {install}"
        )
    if found.group(1) != pin:
        raise SystemExit(
            f"check-i18n-render: mdbook-i18n-helpers version mismatch — found "
            f"{found.group(1)}, expected {pin}:\n  {install}"
        )
    return pin


def render(out: Path, po_dir: Path | None = None) -> Path:
    """Build the zh-Hans book into ``out`` (``html/`` and ``markdown/``); returns the
    Markdown tree, ``out / "markdown"``."""
    env = dict(os.environ)
    env["MDBOOK_BOOK__LANGUAGE"] = LANGUAGE
    env["MDBOOK_OUTPUT__MARKDOWN"] = "{}"
    env["MDBOOK_OUTPUT__HTML__SEARCH__ENABLE"] = "false"
    if po_dir is not None:
        env["MDBOOK_PREPROCESSOR__GETTEXT__PO_DIR"] = str(po_dir)
    run = subprocess.run(
        ["mdbook", "build", "-d", str(out), str(_BOOK)],
        env=env,
        capture_output=True,
        text=True,
    )
    tree = out / "markdown"
    if run.returncode != 0 or not tree.is_dir():
        raise SystemExit(
            f"check-i18n-render: `mdbook build` of the {LANGUAGE} book failed "
            f"(exit {run.returncode}):\n{run.stdout}{run.stderr}"
        )
    if not any(tree.rglob("*.md")):
        raise SystemExit(f"check-i18n-render: the markdown renderer wrote nothing under {tree}")
    return tree


def extract_template(out: Path) -> list:
    """A fresh ``.pot`` from the English source, parsed."""
    env = dict(os.environ)
    env["MDBOOK_OUTPUT"] = json.dumps({"xgettext": {"pot-file": "messages.pot"}})
    run = subprocess.run(
        ["mdbook", "build", "-d", str(out), str(_BOOK)],
        env=env,
        capture_output=True,
        text=True,
    )
    pot = out / "messages.pot"
    if run.returncode != 0 or not pot.is_file():
        raise SystemExit(
            f"check-i18n-render: mdbook-xgettext failed (exit {run.returncode}):\n"
            f"{run.stdout}{run.stderr}"
        )
    return po_catalog.messages(po_catalog.parse_po(pot.read_text(encoding="utf-8")))


def gate(name: str, tree: Path) -> int:
    cmd = (
        list(FENCE_SWEEP) + [str(tree)]
        if name == "sweep_sparql_fences"
        else [sys.executable, str(_SCRIPTS / name), "--rendered-tree", str(tree)]
    )
    run = subprocess.run(cmd, cwd=_REPO, capture_output=True, text=True)
    for stream in (run.stdout, run.stderr):
        if stream.strip():
            print("    " + stream.strip().replace("\n", "\n    "))
    return run.returncode


def gate_all(tree: Path, catalogue: Path) -> list[str]:
    """Run every arm over ``tree``; the names of the arms that went red."""
    red: list[str] = []
    for name in (*PROSE_GATES, "sweep_sparql_fences"):
        print(f"  {name}:")
        if gate(name, tree) != 0:
            red.append(name)
    if gate_reached(catalogue, tree) != 0:
        red.append("reached-the-rendering")
    return red


def report_catalogue(template: list) -> None:
    entries = po_catalog.messages(po_catalog.parse_po(_PO.read_text(encoding="utf-8")))
    live = [e for e in entries if not e.obsolete]
    translated = sum(e.translated for e in live)
    fuzzy = sum(e.fuzzy for e in live)
    obsolete = len(entries) - len(live)
    source_ids = {(e.msgctxt, e.msgid) for e in template}
    catalogue_ids = {(e.msgctxt, e.msgid) for e in live}
    missing = len(source_ids - catalogue_ids)
    stale = len(catalogue_ids - source_ids)
    print(
        f"  catalogue {_PO.relative_to(_REPO)}: {len(live)} message(s) — {translated} "
        f"translated, {fuzzy} fuzzy, {len(live) - translated - fuzzy} untranslated, "
        f"{obsolete} obsolete."
    )
    print(
        f"  drift against a fresh template: {missing} source message(s) absent from the "
        f"catalogue, {stale} catalogue message(s) no longer in the source"
        + (" — `make book-po-update` refreshes it." if missing or stale else ".")
    )


# ── This gate's own falsifiability ────────────────────────────────────────────


def _catalogue(pairs: list[tuple[str, str]]) -> str:
    """A throwaway catalogue: the real scaffold's header, then ``pairs``.

    The header is copied rather than written because ``mdbook-gettext``'s PO
    reader requires the full metadata block (it panics on a header that
    carries only ``Content-Type``), and the scaffold's is known to load.
    """
    text = _PO.read_text(encoding="utf-8")
    head = text[: text.index("\n\n") + 1]
    body = "".join(
        f'\nmsgid "{po_catalog.escape(msgid)}"\nmsgstr "{po_catalog.escape(msgstr)}"\n'
        for msgid, msgstr in pairs
    )
    return head + body


# Runs of CJK ideographs, four or more: the part of a translated message that
# survives Markdown re-serialization untouched, so its presence in the rendered
# tree is the honest test of whether the message was applied.
_CJK_RUN = re.compile(r"[\u4e00-\u9fff\u3400-\u4dbf]{4,}")


def _prose_msgid(template: list) -> str:
    """A plain body paragraph: one line, no code span, a full sentence — so it lives
    in a page (not only in ``SUMMARY.md``) and its poison reaches the Markdown tree."""
    for e in template:
        if "\n" not in e.msgid and "`" not in e.msgid and len(e.msgid) > 60 and e.msgid.endswith("."):
            return e.msgid
    raise SystemExit("check-i18n-render: the template has no plain prose message to poison")


# A msgid no English page carries: the stale entry arm 6 exists to refuse.
_STALE_MSGID = "This sentence exists in the catalogue and nowhere in the English source."


def unreached(catalogue: Path, tree: Path) -> list[str]:
    """Translated messages whose CJK text is absent from the build output (arm 6).

    ``tree`` is the Markdown tree; its sibling ``html/`` is read as well, because a
    ``SUMMARY.md``-only message (a part or chapter title) is rendered into the sidebar
    and the page ``<title>`` and into no Markdown file. CJK text is not entity-encoded
    in mdBook's HTML, so the same probe works on both.
    """
    files = list(tree.rglob("*.md"))
    html = tree.parent / "html"
    files += list(html.rglob("*.html")) + list(html.glob("*.js"))
    rendered = "".join(p.read_text(encoding="utf-8") for p in sorted(files))
    entries = po_catalog.messages(po_catalog.parse_po(catalogue.read_text(encoding="utf-8")))
    missing: list[str] = []
    for e in entries:
        if not e.translated:
            continue
        runs = _CJK_RUN.findall(e.msgstr)
        if not runs:
            continue
        probe = max(runs, key=len)
        if probe not in rendered:
            missing.append(
                f"{catalogue.name}:{e.line}: translated message never reached the rendering "
                f"(msgid {e.msgid[:60]!r}; probe {probe!r}) — the preprocessor did not "
                f"match its msgid; the English source no longer carries it "
                f"(`make book-po-update` marks such entries fuzzy or obsolete)"
            )
    return missing


def gate_reached(catalogue: Path, tree: Path) -> int:
    print("  translated messages reached the rendering:")
    missing = unreached(catalogue, tree)
    for line in missing:
        print(f"    {line}")
    if missing:
        return 1
    print("    every rendered msgstr's CJK text is present in the tree.")
    return 0


def load_gate(name: str):  # noqa: ANN202 — a module object
    """Import a sibling gate script as a module, for its own self-test specimens."""
    path = _SCRIPTS / name
    spec = importlib.util.spec_from_file_location(path.stem.replace("-", "_"), path)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def poisons(template: list) -> tuple[tuple[str, str, str, str], ...]:
    """``(arm, what, msgid, msgstr)`` — one poisoned message per arm.

    Each poison is the arm's OWN self-test specimen, imported from the gate
    script rather than restated here: the gates scan ``scripts/*.py`` too, and
    a specimen restated in this file is a claim this file makes. Reusing the
    gate's case also makes this the pipeline proof it is meant to be — the
    same text the gate already refuses on a string must still be refused once
    it has travelled through a ``.po`` file, ``mdbook-gettext`` and the
    ``markdown`` renderer.
    """
    prose = _prose_msgid(template)
    brand = load_gate("check-brand-casing.py")
    refs = load_gate("check-issue-refs.py")
    spec = load_gate("check-spec-attribution.py")
    claims = load_gate("check-doc-claims.py")
    glued_brand = next(text for what, _s, text, n in brand._CASES if n and "glued" in what)
    glued_hazard = next(
        src for what, _s, src, token in refs._DETECTION_CASES if token == "H12" and "glued" in what
    )
    zh_attribution = spec._MUST_CATCH_ZH[0][1]
    overclaim = claims._OVERCLAIM_SPECIMENS[2][0]
    return (
        ("check-brand-casing.py", "a bare 'purrdf' glued to CJK", prose, glued_brand.strip()),
        ("check-issue-refs.py", "a hazard id glued to CJK", prose, glued_hazard.strip()),
        (
            "check-spec-attribution.py",
            "the quad template (a first-party extension, not defined by SPARQL 1.2) "
            "attributed to it in Chinese",
            prose,
            zh_attribution,
        ),
        (
            "check-doc-claims.py",
            "the overclaim ban's own specimen after a full-width terminator, unscoped",
            prose,
            f"在此语料上的结果如下。{overclaim}",
        ),
        (
            "sweep_sparql_fences",
            "a fenced sparql block with a full-width brace, written inside a paragraph",
            prose,
            "示例：\n\n```sparql\nSELECT ?s WHERE ｛ ?s ?p ?o ｝\n```\n",
        ),
    )


def self_test(template: list, scratch: Path) -> list[str]:
    """Each arm must go red on a catalogue poisoned for it. Empty is the only pass.

    Every poison is ALSO required to reach the rendering (arm 6), so a poison that
    the preprocessor silently dropped would fail here as "not reached" rather than
    pass as "not caught" — the two are different defects and the report must say
    which. The last case is the mirror: a translation the preprocessor is known to
    drop, which arm 6 must refuse.
    """
    failures: list[str] = []
    cases = list(poisons(template))
    if any(e.msgid == _STALE_MSGID for e in template):
        raise SystemExit("check-i18n-render: the stale-msgid specimen exists in the source")
    cases.append(
        (
            "reached-the-rendering",
            "a translation of a msgid the English source does not carry (a stale entry)",
            _STALE_MSGID,
            "这段译文没有对应的英文段落。",
        )
    )
    for index, (arm, what, msgid, msgstr) in enumerate(cases):
        po_dir = scratch / f"poison-{index}"
        po_dir.mkdir()
        catalogue = po_dir / f"{LANGUAGE}.po"
        catalogue.write_text(_catalogue([(msgid, msgstr)]), encoding="utf-8")
        tree = render(scratch / f"render-{index}", po_dir=po_dir)
        print(f"  {arm} on {what}:")
        if arm == "reached-the-rendering":
            code = gate_reached(catalogue, tree)
        else:
            if unreached(catalogue, tree):
                failures.append(f"the poison for {arm} never reached the rendering — {msgstr!r}")
                continue
            code = gate(arm, tree)
        if code == 0:
            failures.append(f"{arm} stayed GREEN on {what} — {msgstr!r}")
        else:
            print(f"    -> red (exit {code}), as required")
    return failures


def main(argv: list[str]) -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true", help="prove each arm can go red first")
    args = ap.parse_args(argv)

    pin = require_tools()
    print(f"check-i18n-render: mdbook {pinned_mdbook_version()} and mdbook-i18n-helpers {pin}, both at their pins.")
    if not _PO.is_file():
        raise SystemExit(f"check-i18n-render: no catalogue at {_PO}")

    with tempfile.TemporaryDirectory(prefix="check-i18n-render-") as tmp:
        scratch = Path(tmp)
        template = extract_template(scratch / "template")
        if args.self_test:
            print("check-i18n-render: proving each arm goes red on a poisoned catalogue —")
            failures = self_test(template, scratch)
            if failures:
                print(
                    "check-i18n-render: an arm cannot go red:\n"
                    + "\n".join(f"  - {f}" for f in failures),
                    file=sys.stderr,
                )
                return 1
            print("check-i18n-render: every arm went red on its poison.")

        print(f"check-i18n-render: rendering the {LANGUAGE} book with {_PO.relative_to(_REPO)} —")
        tree = render(scratch / "render")
        pages = sum(1 for _ in tree.rglob("*.md"))
        print(f"  {pages} page(s) rendered to Markdown under a temporary directory.")
        red = gate_all(tree, _PO)
        report_catalogue(template)
        if red:
            print(
                f"check-i18n-render: the {LANGUAGE} rendering fails {len(red)} gate(s): "
                f"{', '.join(red)}",
                file=sys.stderr,
            )
            return 1
    print(f"OK: the {LANGUAGE} rendering passes all {len(PROSE_GATES) + 2} gates.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
