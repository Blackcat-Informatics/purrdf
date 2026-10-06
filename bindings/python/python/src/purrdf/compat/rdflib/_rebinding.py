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

That model holds for a ``SELECT`` or ``ASK`` whose reassignment sits, once per group,
in the query's own group, a ``UNION`` branch or a sub-``SELECT``'s own ``SELECT``
clause, and that has no ``OPTIONAL``, ``MINUS``, ``EXISTS``, ``GROUP BY`` or
``SELECT *``. Elsewhere rdflib's answer follows from how its evaluator
merges and re-reads solution mappings around the assignment, which a text rewrite
cannot reproduce (a ``MINUS`` compares the assigned value, an ``OPTIONAL`` arm keeps
the left side's, a nested group's assignment is overwritten by the join). Rather than
answer those differently from rdflib, :func:`rewrite` raises
:class:`UnmodelledReassignment` for them; the differential tests pin each such shape.
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
#: enough that no query names it by accident; a query with a variable that does is
#: left alone.
_FRESH_PREFIX = "__purrdf_rdflib_rebound_"


#: The group kinds a reassignment may sit in and still answer as rdflib does.
_MODELLED_GROUPS = frozenset({"where", "union", "subselect", "template"})

#: Keywords whose presence anywhere in a reassigning query puts it outside the model.
_UNMODELLED_KEYWORDS = ("OPTIONAL", "MINUS", "EXISTS")


class UnmodelledReassignment(NotImplementedError):
    """A query reassigns a bound variable in a shape the shim cannot answer as rdflib.

    Raised instead of an answer that would differ from rdflib's. The native engine
    refuses every such reassignment; the shim's rewrite covers only the shapes in the
    module documentation.
    """


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
    # Only a VARIABLE spelled with the fresh prefix could collide with a fresh name;
    # the prefix inside an IRI or a string (a PREFIX or BASE the query is given, say)
    # names no variable, so it is no reason to leave the reassignment unrewritten.
    if not reassigned or any(
        kind == "var" and value[1:].startswith(_FRESH_PREFIX) for kind, value in tokens
    ):
        return None
    _refuse_unmodelled(tokens, set(reassigned))
    fresh = {name: f"{_FRESH_PREFIX}{name}" for name in reassigned}
    # The `(` that opens a `VALUES ( ?x … )` variable list: a variable there names a
    # column of the data block, not an expression read, so it is left as written.
    values_lists = _values_variable_lists(tokens)

    out: list[str] = []
    # The innermost open bracket at each token: a variable whose innermost bracket is
    # `(` is read by an expression; one inside `{` or at the top is in a pattern or a
    # projection list.
    brackets: list[str] = []
    # Whether the token sits in the outermost SELECT clause (before its WHERE or `{`).
    in_head_select = False
    head_select_done = False
    previous_significant = ""
    for position, (kind, value) in enumerate(tokens):
        if kind == "word" and value.upper() == "SELECT" and not brackets and not head_select_done:
            in_head_select = True
        elif (
            in_head_select
            and not brackets
            and ((kind == "word" and value.upper() == "WHERE") or (kind == "open" and value == "{"))
        ):
            in_head_select = False
            head_select_done = True
        if kind == "open":
            brackets.append("values" if position in values_lists else value)
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


def _previous_significant(tokens: list[tuple[str, str]], index: int) -> int | None:
    for position in range(index - 1, -1, -1):
        if tokens[position][0] not in ("space", "comment"):
            return position
    return None


def _word(tokens: list[tuple[str, str]], position: int | None) -> str:
    """The upper-cased keyword at ``position``, or ``""`` when it is no word."""
    if position is None or tokens[position][0] != "word":
        return ""
    return tokens[position][1].upper()


def _matching_closes(tokens: list[tuple[str, str]]) -> dict[int, int]:
    """Each opening bracket's position, mapped to its closing bracket's."""
    stack: list[int] = []
    closes: dict[int, int] = {}
    for position, (kind, _) in enumerate(tokens):
        if kind == "open":
            stack.append(position)
        elif kind == "close" and stack:
            closes[stack.pop()] = position
    return closes


def _values_variable_lists(tokens: list[tuple[str, str]]) -> set[int]:
    """The positions of each ``(`` opening a ``VALUES ( ?x … )`` variable list."""
    return {
        position
        for position, (kind, value) in enumerate(tokens)
        if kind == "open"
        and value == "("
        and _word(tokens, _previous_significant(tokens, position)) == "VALUES"
    }


def _group_kinds(tokens: list[tuple[str, str]]) -> dict[int, str]:
    """The kind of each ``{`` group, by the position of its opening brace."""
    closes = _matching_closes(tokens)
    kinds: dict[int, str] = {}
    depth = 0
    seen_top_group = False
    first_word = next((value.upper() for kind, value in tokens if kind == "word"), "")
    for position, (kind, value) in enumerate(tokens):
        if kind == "close" and value == "}":
            depth -= 1
            continue
        if kind != "open" or value != "{":
            continue
        before = _previous_significant(tokens, position)
        word = _word(tokens, before)
        two_before = _word(tokens, _previous_significant(tokens, before) if before else None)
        after_close = _next_significant(tokens, closes[position]) if position in closes else None
        # A `VALUES` data block holds no assignment, so it needs no kind of its own.
        if word in ("OPTIONAL", "MINUS", "EXISTS"):
            group = word.lower()
        elif two_before in ("GRAPH", "SERVICE") or word in ("GRAPH", "SERVICE"):
            group = "graph"
        elif word == "WHERE":
            group = "where"
        elif depth == 0 and not seen_top_group:
            group = "template" if first_word == "CONSTRUCT" else "where"
        elif word == "UNION" or _word(tokens, after_close) == "UNION":
            group = "union"
        elif _word(tokens, _next_significant(tokens, position)) == "SELECT":
            group = "subselect"
        else:
            group = "plain"
        if depth == 0:
            seen_top_group = True
        kinds[position] = group
        depth += 1
    return kinds


def _refuse_unmodelled(tokens: list[tuple[str, str]], reassigned: set[str]) -> None:
    """Raise :class:`UnmodelledReassignment` for a shape the rewrite does not model."""
    words = [value.upper() for kind, value in tokens if kind == "word"]
    for keyword in _UNMODELLED_KEYWORDS:
        if keyword in words:
            raise UnmodelledReassignment(
                f"the query reassigns a bound variable and contains {keyword}; the rdflib "
                f"compat shim cannot answer that as rdflib does"
            )
    for position, word in enumerate(words):
        if word == "GROUP" and words[position + 1 : position + 2] == ["BY"]:
            raise UnmodelledReassignment(
                "the query reassigns a bound variable and has a GROUP BY; the rdflib "
                "compat shim cannot answer that as rdflib does"
            )
    for position, (kind, value) in enumerate(tokens):
        if kind == "word" and value.upper() == "SELECT":
            following = _next_significant(tokens, position)
            while _word(tokens, following) in ("DISTINCT", "REDUCED"):
                following = _next_significant(tokens, following)
            if following is not None and tokens[following][1] == "*":
                raise UnmodelledReassignment(
                    "the query reassigns a bound variable under SELECT *; the rdflib "
                    "compat shim cannot answer that as rdflib does"
                )
    # A CONSTRUCT or DESCRIBE answers with a graph, which the fresh-column fold never
    # reaches: its template would keep reading the bound value.
    form = next(
        (word for word in words if word in ("SELECT", "ASK", "CONSTRUCT", "DESCRIBE")), ""
    )
    if form in ("CONSTRUCT", "DESCRIBE"):
        raise UnmodelledReassignment(
            f"the query reassigns a bound variable in a {form}; the rdflib compat shim "
            "cannot answer that as rdflib does"
        )
    kinds = _group_kinds(tokens)
    # Each open group as (kind, position of its brace): the position tells two groups
    # of one kind apart.
    open_groups: list[tuple[str, int]] = []
    assigned: set[tuple[str, tuple[int, ...]]] = set()
    for position, (kind, value) in enumerate(tokens):
        if kind == "open" and value == "{":
            open_groups.append((kinds.get(position, "plain"), position))
        elif kind == "close" and value == "}" and open_groups:
            open_groups.pop()
        elif kind == "word" and value.upper() == "AS":
            target = _next_significant(tokens, position)
            if target is None or tokens[target][0] != "var":
                continue
            name = tokens[target][1][1:]
            if name not in reassigned:
                continue
            group_kinds = [group for group, _ in open_groups]
            outside = [group for group in group_kinds if group not in _MODELLED_GROUPS]
            if outside:
                raise UnmodelledReassignment(
                    f"the query reassigns {tokens[target][1]} inside a {outside[-1]} group; "
                    f"the rdflib compat shim cannot answer that as rdflib does"
                )
            # Inside a sub-SELECT's own WHERE group the assigned value has to survive
            # the inner projection, which projects the bound name rather than the
            # fresh one. A `(… AS ?x)` in the sub-SELECT's own SELECT clause does not
            # have that problem: it IS the projection.
            if "subselect" in group_kinds and "where" in group_kinds[
                group_kinds.index("subselect") + 1 :
            ]:
                raise UnmodelledReassignment(
                    f"the query reassigns {tokens[target][1]} inside a sub-SELECT's WHERE "
                    "clause; the rdflib compat shim cannot answer that as rdflib does"
                )
            # Two assignments of one name in one group: SPARQL refuses the second as a
            # rebinding of an in-scope variable, while rdflib keeps the later value.
            key = (name, tuple(brace for _, brace in open_groups))
            if key in assigned:
                raise UnmodelledReassignment(
                    f"the query assigns {tokens[target][1]} twice in one group; the rdflib "
                    "compat shim cannot answer that as rdflib does"
                )
            assigned.add(key)


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
