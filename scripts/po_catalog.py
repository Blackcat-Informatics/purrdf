# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""A minimal reader for gettext ``.po``/``.pot`` catalogues.

Shared by ``check-i18n-glossary.py`` and ``check-i18n-render.py``, which read
the book's translation catalogue (``docs/book/po/zh-Hans.po``) and the
template ``mdbook-xgettext`` extracts from the English source. Only the parts
of the format those gates need are modelled: the ``msgctxt``/``msgid``/
``msgstr`` triple with its continuation lines, the ``fuzzy`` flag, and the
``#~`` obsolete marker. Plural forms are read but never produced by
``mdbook-xgettext``, so they are folded into ``msgstr`` rather than modelled.

No third-party dependency (``polib`` is not installed here and a gate should
not need a package to read a text file), and the escapes are the four the
tool emits: ``\\\\``, ``\\"``, ``\\n``, ``\\t``.
"""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class Entry:
    """One catalogue entry, at the 1-based line its first keyword sits on."""

    line: int
    msgctxt: str | None
    msgid: str
    msgstr: str
    fuzzy: bool
    obsolete: bool

    @property
    def translated(self) -> bool:
        """Whether ``mdbook-gettext`` would render this entry's ``msgstr``.

        A fuzzy entry is not rendered (the preprocessor keeps the source text
        for it), and neither is an obsolete one, so both are "untranslated"
        to a reader even when ``msgstr`` is non-empty.
        """
        return bool(self.msgstr) and not self.fuzzy and not self.obsolete


_ESCAPES = {"\\": "\\", '"': '"', "n": "\n", "t": "\t"}


def unescape(text: str) -> str:
    """The value of a quoted PO string's body (between the quotes)."""
    out: list[str] = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c == "\\" and i + 1 < n and text[i + 1] in _ESCAPES:
            out.append(_ESCAPES[text[i + 1]])
            i += 2
            continue
        out.append(c)
        i += 1
    return "".join(out)


def escape(text: str) -> str:
    """The body of a quoted PO string for ``text`` (inverse of :func:`unescape`)."""
    return (
        text.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\t", "\\t")
    )


def _quoted_body(line: str) -> str:
    """The body of a ``"..."`` token, which is the whole of ``line``."""
    line = line.strip()
    if len(line) < 2 or line[0] != '"' or line[-1] != '"':
        raise ValueError(f"not a quoted PO string: {line!r}")
    return unescape(line[1:-1])


def parse_po(text: str) -> list[Entry]:
    """Every entry of a catalogue, including the header (``msgid ""``).

    An entry ends at a blank line or at the next non-continuation keyword
    after a ``msgstr``. Lines beginning ``#~`` are obsolete entries and are
    read the same way with the marker stripped.
    """
    entries: list[Entry] = []
    fields: dict[str, list[str]] = {}
    current: str | None = None
    fuzzy = False
    obsolete = False
    start = 0

    def flush() -> None:
        nonlocal fields, current, fuzzy, obsolete
        if "msgid" in fields:
            msgstr = "".join(fields.get("msgstr", []))
            if "msgstr" not in fields:
                # Plural forms: the first form stands for the entry.
                plural = [k for k in fields if k.startswith("msgstr[")]
                if plural:
                    msgstr = "".join(fields[sorted(plural)[0]])
            ctxt = fields.get("msgctxt")
            entries.append(
                Entry(
                    line=start,
                    msgctxt="".join(ctxt) if ctxt is not None else None,
                    msgid="".join(fields["msgid"]),
                    msgstr=msgstr,
                    fuzzy=fuzzy,
                    obsolete=obsolete,
                )
            )
        fields = {}
        current = None
        fuzzy = False
        obsolete = False

    for number, raw in enumerate(text.splitlines(), start=1):
        line = raw.strip()
        if not line:
            flush()
            continue
        entry_obsolete = False
        if line.startswith("#~"):
            entry_obsolete = True
            line = line[2:].strip()
            if not line:
                continue
        if line.startswith("#"):
            if current is not None:
                # A comment after a msgstr opens the next entry.
                flush()
            if line.startswith("#,") and "fuzzy" in [f.strip() for f in line[2:].split(",")]:
                fuzzy = True
            continue
        if line.startswith('"'):
            if current is None:
                raise ValueError(f"line {number}: continuation string with no keyword")
            fields[current].append(_quoted_body(line))
            continue
        keyword, _, rest = line.partition(" ")
        if keyword in ("msgctxt", "msgid", "msgid_plural") or keyword.startswith(
            "msgstr"
        ):
            if keyword in ("msgctxt", "msgid") and (
                keyword in fields or "msgstr" in fields
            ):
                flush()
            if not fields:
                start = number
                obsolete = entry_obsolete
            current = keyword
            fields[current] = [_quoted_body(rest)]
            continue
        raise ValueError(f"line {number}: unrecognized PO line {line!r}")
    flush()
    return entries


def messages(entries: list[Entry]) -> list[Entry]:
    """The entries that are messages: everything but the header."""
    return [e for e in entries if e.msgid or e.msgctxt]
