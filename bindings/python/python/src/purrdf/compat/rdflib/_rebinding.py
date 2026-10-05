# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# Why not Rust: rdflib's initBindings quirks are reproduced inside the pure-Python rdflib
# compat shim on purpose; the native purrdf module and every Rust lane must never see them.
"""rdflib's ``initBindings`` reassignment, reproduced for the compat shim only.

PurRDF refuses a query that assigns a pre-bound variable (``BIND(... AS ?x)`` or
``(... AS ?x)`` over a bound ``?x``): a pre-bound variable is one value for the whole
evaluation. rdflib 7.6 answers such a query instead, and the compat shim must answer
as rdflib does. Measured against rdflib 7.6.0, with ``?this`` bound to ``ex:a``:

* a triple pattern still matches ``?this`` as ``ex:a`` after a ``BIND`` assigns it;
* an expression after the ``BIND`` (a ``FILTER``, a later ``BIND``, a ``SELECT``
  expression) reads the assigned value; without one it reads ``ex:a``;
* the projected ``?this`` is the assigned value where the ``BIND`` ran, ``ex:a``
  elsewhere, and a ``(... AS ?this)`` in the ``SELECT`` clause names its column.

The shim reproduces that by renaming each assignment to a fresh variable before the
native engine sees the text, reading it back through ``COALESCE(fresh, ?this)`` in
expression positions, and folding the fresh column into ``?this`` in the result. The
native engine still pre-binds ``?this`` and still refuses a reassignment; it never
sees one from here.
"""

from __future__ import annotations

import re
from dataclasses import dataclass

#: One lexical token of a SPARQL query, by kind. Strings, IRIs and comments are
#: tokens of their own so a ``?`` or ``AS`` inside one is never read as syntax.
_TOKEN_RE = re.compile(
    r"""
    (?P<comment>\#[^\n]*)
  | (?P<string>'''(?:[^'\\]|\\.|'(?!''))*'''|\"\"\"(?:[^"\\]|\\.|"(?!""))*\"\"\"
               |'(?:[^'\\\n]|\\.)*'|"(?:[^"\\\n]|\\.)*")
  | (?P<iri><[^<>"{}|^`\\\x00-\x20]*>)
  | (?P<var>[?$][A-Za-z0-9_·À-￿]+)
  | (?P<word>[A-Za-z_][A-Za-z0-9_\-]*(?::[^\s(){}\[\],;.]*)?)
  | (?P<open>[({\[])
  | (?P<close>[)}\]])
  | (?P<space>\s+)
  | (?P<other>.)
    """,
    re.VERBOSE | re.DOTALL,
)

#: The prefix of a fresh variable carrying a reassigned value. Long and specific
#: enough that no query names it by accident; a query that does is left alone.
_FRESH_PREFIX = "__purrdf_rdflib_rebound_"


@dataclass(frozen=True)
class Rebinding:
    """A query rewritten for rdflib's reassignment, and how to fold its result."""

    text: str
    #: ``(bound variable name, fresh variable name)`` for each reassigned variable.
    renamed: tuple[tuple[str, str], ...]


def _tokens(text: str) -> list[tuple[str, str]]:
    return [(match.lastgroup or "other", match.group()) for match in _TOKEN_RE.finditer(text)]


def rewrite(text: str, bound: set[str]) -> Rebinding | None:
    """Rewrite ``text`` when it assigns a variable in ``bound``; ``None`` otherwise.

    ``None`` means the query reaches the native engine unchanged, which is every query
    that does not reassign a bound variable.
    """
    tokens = _tokens(text)
    reassigned: list[str] = []
    for index, (kind, value) in enumerate(tokens):
        if kind != "word" or value.upper() != "AS":
            continue
        target = _next_significant(tokens, index)
        if target is None:
            continue
        target_kind, target_value = tokens[target]
        name = target_value[1:]
        if target_kind == "var" and name in bound and name not in reassigned:
            reassigned.append(name)
    if not reassigned or _FRESH_PREFIX in text:
        return None
    fresh = {name: f"{_FRESH_PREFIX}{name}" for name in reassigned}

    out: list[str] = []
    # The innermost open bracket at each token: a variable whose innermost bracket is
    # `(` is read by an expression; one inside `{` or at the top is in a pattern or a
    # projection list.
    brackets: list[str] = []
    # Whether the token sits in the outermost SELECT clause (before its WHERE or `{`).
    in_head_select = False
    head_select_done = False
    previous_significant = ""
    for kind, value in tokens:
        if kind == "word" and value.upper() == "SELECT" and not brackets and not head_select_done:
            in_head_select = True
        elif in_head_select and not brackets and (
            (kind == "word" and value.upper() == "WHERE") or (kind == "open" and value == "{")
        ):
            in_head_select = False
            head_select_done = True
        if kind == "open":
            brackets.append(value)
        elif kind == "close" and brackets:
            brackets.pop()
        if kind == "var" and value[1:] in fresh:
            name = value[1:]
            if previous_significant.upper() == "AS":
                # The assignment itself: it writes the fresh variable instead.
                value = f"?{fresh[name]}"
            elif brackets and brackets[-1] == "(":
                # An expression read: the assigned value where the assignment ran,
                # the bound value elsewhere.
                value = f"COALESCE(?{fresh[name]}, {value})"
            elif in_head_select and not brackets:
                # A bare projection: project the fresh column beside it, to fold.
                value = f"{value} ?{fresh[name]}"
        out.append(value)
        if kind not in ("space", "comment"):
            previous_significant = value if kind == "word" else kind
    return Rebinding(
        text="".join(out),
        renamed=tuple((name, fresh[name]) for name in reassigned),
    )


def _next_significant(tokens: list[tuple[str, str]], index: int) -> int | None:
    for position in range(index + 1, len(tokens)):
        if tokens[position][0] not in ("space", "comment"):
            return position
    return None


def fold_columns(
    names: list[str], rows: list[list[object]], renamed: tuple[tuple[str, str], ...]
) -> tuple[list[str], list[list[object]]]:
    """Fold each fresh column into its bound variable's, as rdflib's projection does.

    Where both columns are projected, a row's value is the fresh one if it is bound and
    the bound one otherwise; a fresh column projected alone (a ``(... AS ?x)`` in the
    ``SELECT`` clause, or a ``SELECT *``) is renamed to the bound variable's name.
    """
    names = list(names)
    rows = [list(row) for row in rows]
    for bound_name, fresh_name in renamed:
        if fresh_name not in names:
            continue
        fresh_at = names.index(fresh_name)
        if bound_name in names:
            bound_at = names.index(bound_name)
            for row in rows:
                if row[fresh_at] is not None:
                    row[bound_at] = row[fresh_at]
            for row in rows:
                del row[fresh_at]
            del names[fresh_at]
        else:
            names[fresh_at] = bound_name
    return names, rows
