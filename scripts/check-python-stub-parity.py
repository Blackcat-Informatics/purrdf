#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Hygiene gate: every member the PyO3 extension exposes to Python must be DECLARED in
`bindings/python/python/src/purrdf/__init__.pyi`.

The Python package ships `py.typed`, so that stub is the compiled extension's ONLY
declaration — a type checker reads it and never the `.so`. A `#[pymethods]` method the
stub does not declare is therefore live at runtime and invisible to every checked
caller: the call works, `mypy` rejects it, and a user reading a README that spells the
call sequence is told their own code is wrong.

That is not hypothetical. The incremental SHACL change lane shipped
`PreparedShapes.validate_store_changes`, the `ChangeValidation` it returns, and
`Store.checkpoint` / `Store.change_size` — wired into the npm `index.d.ts` for the same
reason, and then left out of the Python stub, where

    error: "Store" has no attribute "checkpoint"                      [attr-defined]
    error: "_PreparedShapes" has no attribute "validate_store_changes"  [attr-defined]

was the whole of the user-visible symptom. `check-entailment-surface.py` closes this for
ONE surface (the conclusion-directed entailment services) by naming them in a table; this
closes it for the WHOLE extension, by deriving the surface instead of listing it.

# The member set is DERIVED, not listed

Every `#[pyclass]` in `bindings/python/src/**.rs` is read out of the tree, together with
the `#[pymethods]` block that `impl`s it, and every method in that block is a member this
gate requires the stub to declare. A new binding method is therefore a FAILURE of this
gate until its stub line exists, rather than a quiet runtime-only rollout. Nothing here
enumerates classes or methods: a table of them is exactly the thing that goes stale.

The PyO3 spellings that change a member's PYTHON name are honoured, because the Python
name is what the stub must declare:

* `#[new]` is `__init__`;
* `#[pyo3(name = "x")]` renames to `x` (`Store._store_capsule` is the live case) — read
  at the attribute's TOP level only, because inside a `signature = (…)` group the same
  spelling is a parameter called `name` carrying a string default;
* `#[getter]` / `#[setter]` declare a PROPERTY — under `#[getter(x)]`'s explicit name if
  one is given, else the function's, with a `get_` / `set_` prefix stripped exactly as
  PyO3 strips it;
* `#[classattr]` is a class attribute.

A `#[pyclass] enum`'s VARIANTS are checked the same way: they are class attributes PyO3
synthesizes without any `#[pymethods]` block, so deriving from methods alone would leave
`RdfFormat.YAML_LD` — the enum member the stub does have to carry — outside the gate.

# What counts as DECLARED

The stub's class body is found by INDENTATION and the member is looked for inside it, so
a `def` that has drifted to module scope is not a declaration of a method. A member
counts as declared when the class body holds a `def <name>(` (which covers `@property`,
`@staticmethod` and `@overload` alike) or an annotation `<name>: <type>` — both at the
class's OWN member indentation, because a wrapped `def`'s parameter is spelled
`name: type` exactly as an attribute is, and a gate reading the body flat would read
`max_answers` off a query signature as an attribute of `Store`.

# Both directions, for a class the gate RESOLVED

A stub `def` with no `#[pymethods]` behind it is the same defect wearing the other face:
a checked caller writes the call, the type checker accepts it, and it raises
`AttributeError` at runtime. So a resolved class is compared in both directions — every
member the extension exposes must be declared, and every member the class declares must
be exposed. The reverse arm is scoped to classes this gate RESOLVED to a pyclass; the
stub's module-level functions, `TypedDict`s, type aliases and namespace classes have no
pyclass behind them by construction and are not touched by it. It skips DUNDERS, and
only it does: `#[pyclass(eq, ord, hash, str)]` synthesizes `__eq__`, the ordering four,
`__hash__` and `__str__` with no `fn` for this gate to read, and `object` supplies the
rest — so a dunder in the stub declares something that really is there, and a gate
demanding a `#[pymethods]` behind it would refuse a correct stub.

The stub's own naming convention is honoured: an engine class that is re-exported inside
a namespace class carries an underscore-prefixed module-level name (`_PreparedShapes` is
`purrdf.shapes.PreparedShapes`), so the pyclass name `P` resolves to `class P:` if the
stub has one and `class _P:` otherwise.

# A `create_exception!`'s PINNED NAMES are checked too, and DERIVED the same way

A member's EXISTENCE is not the only thing the stub is the only declaration of. The
retrieval surface raises `PlanDocumentError` carrying a pinned kebab-case `.refusal`
name, and both the runtime class and the stub class enumerate the names a caller may
branch on. Two descriptions of one contract drift, and this one did: the stub named
fifteen of nineteen, the four it omitted were the four the last two engine fixes added,
and a member-existence check stayed green through all of it because `refusal: str`
existed the whole time. A caller's type checker reads the stub, so the omitted four did
not exist for anybody reading the declaration.

So the names are checked, and the set is DERIVED at both ends:

* the UNIVERSE is `PlanError::refusal()` in `crates/retrieval/src/error.rs`, whose match
  is exhaustive over a `#[non_exhaustive]` enum — a variant added without a name is a
  compile error there, so the arms' string literals are the whole set by construction;
* the REQUIRED set is whatever the extension's own `create_exception!` docstring names,
  because that docstring is the runtime `__doc__` a caller reads with `help()`.

Nothing here lists a refusal. A pinned list in this script would be a THIRD description
of the same contract, free to drift from the two it is checking.

WHICH `create_exception!` is subject to it is derived from the RAISE SITE, not from the
prose: the boundary is the class built by a function that calls `PlanError::refusal`.
`ShapesProductError` carries a different enum's `dimension_label()` and is left alone
because its raise site says so. Selecting on the docstring instead would have caught it
on the word `truncated`, which both vocabularies happen to contain, and then demanded
twenty `.dimension` labels of a stub that documents none — an over-refusal the
`_ACCEPTED` arm caught while this was being written.

Four ways it fails, each by NAME:

* the runtime docstring names a refusal the stub does not — the live defect;
* the stub names one the runtime docstring does not — the mirror, where a caller is
  told to branch on something that boundary cannot raise;
* either ENUMERATES a hyphenated name in backticks that `refusal()` cannot return — a
  refusal renamed in the engine while the prose still promises the old word, which is
  also what stops this arm going quietly vacuous when a name moves;
* the stub declares no class for the boundary at all, so the whole vocabulary is
  declared nowhere a type checker reads.

The third is SCOPED, and its scope is the rule a writer of these two docstrings needs: a
backticked hyphenated word is read as a promised name only where it is ENUMERATED —
where the backticked token immediately beside it is itself one of the engine's names.
An enumeration is a run of names written next to each other, and that is exactly the
span this arm may read as promises.

Everything outside a run is prose, backticks and all. `plan-document`,
`content-addressed` and `length-framed` are code terms about this very boundary, and
they may be backticked anywhere a run of refusal names does not surround them. Reading
the WHOLE block refused all three, which is worse than having no arm: the message told a
reader their ordinary Markdown was a broken promise, and the only way to satisfy it was
to edit this gate. Both ends of both docstrings are pinned in `_ACCEPTED`. Locality is
also what keeps the arm's purpose — a renamed refusal sits in the middle of the run it
was part of, with a surviving name beside it, so the rename mutation still fires at BOTH
docstrings. One span from the first known name to the last would have missed a rename of
whichever name a docstring enumerates LAST, which is where the newest refusal is written.

# What this gate CANNOT see — the stated boundary

* **Parameter lists.** This checks that a member is DECLARED, not that it is declared
  with the right arguments. `check-entailment-surface.py` does the stronger arity check
  for the surface it names, driven by a hand-written `_PARAM_SPELLINGS` table; there is
  no such table here because there is no hand-written member list here either.
* **Return types.** Same reason. A declared member whose annotated type is wrong is a
  defect this gate passes.
* **Module-level `#[pyfunction]`s.** This gate is about pyclass MEMBERS. A free function
  and its registration are what `check-entailment-surface.py` reads, for the surface it
  names.
* **A refusal documented in NEITHER description.** The name arm compares the two
  descriptions of the contract and holds both to `refusal()`'s vocabulary; a new
  document-side refusal that nobody wrote down anywhere passes it. Which variants can
  cross which boundary is a call-graph question and is not decidable from this script's
  text. What it does decide is that the two written descriptions cannot disagree, which
  is the way this contract actually broke.
* **Members added at runtime by the Python shim** (`python/src/purrdf/__init__.py`),
  which are not `#[pymethods]` and are out of this gate's derivation.
* **A name the stub cannot resolve.** Whether `CapsuleType` is imported is a type
  checker's question, not this one's; `mypy` over the stub answers it.
* **Dunders `object` already declares** — `__repr__`, `__str__`, `__eq__`, `__ne__`,
  `__hash__`. Omitting one of those from the stub cannot make a valid call fail to type
  check, because the checker resolves it on `object`; requiring them would report a
  hundred rows that hide nothing. Every OTHER dunder is required: `__init__`,
  `__len__`, `__iter__`, `__next__`, `__bool__` and `__getitem__` all change what a
  checked caller may write, and an omitted one is a real hole.

# The gate MUTATION-TESTS ITSELF, on every run

`_MUTATIONS` is one mutation per arm at least — the construct that arm claims to read,
removed or moved, applied to a STRING and never to a tracked file — and `self_test`
re-runs the whole gate against each and requires it to FAIL. A mutation the gate SURVIVES
is reported by name and is itself a failure of this script. A mutation that can no longer
be APPLIED fails too: its needle is gone because the tree moved, and a self-test quietly
testing nothing is the defect one layer up. `--self-test` runs the suite alone and prints
one line per mutation.

`_ACCEPTED` is the other half, and it is not optional: a refusal is a claim too. The
mutation suite proves only that this gate refuses a BROKEN tree, which a gate refusing
everything also does — so each entry there is a VALID spelling this gate could plausibly
mis-read, and each must leave `gate_problems` empty. The live one is
`#[pyo3(signature = (name = "…", …))]`, where `name` is a PARAMETER with a string
default and not a rename at all; read flat, it would make this gate demand a stub
`def <that default>(` and report the real method as undeclared. An over-refusal is the
mirror of a missed member, and it hides better, because it looks like strictness.

Pure text over committed files: no cargo build and no Python import. Run standalone or
from `make check` / CI.
"""

from __future__ import annotations

import re
import sys
from collections.abc import Callable, Generator
from contextlib import contextmanager
from pathlib import Path

_REPO = Path(__file__).resolve().parent.parent

# Where the extension's Python surface is written, and where it is declared.
_BINDING_ROOT = Path("bindings/python/src")
_STUB = Path("bindings/python/python/src/purrdf/__init__.pyi")

# Where the pinned `.refusal` vocabulary is defined. `PlanError::refusal` is an
# exhaustive match over a `#[non_exhaustive]` enum, so its arms' string literals ARE the
# set of names — a variant added without one does not compile.
_ENGINE_ERRORS = Path("crates/retrieval/src/error.rs")
_REFUSAL_SIGNATURE = "pub fn refusal(&self) -> &'static str"

# Dunders `object` already declares. See the module docstring: omitting one cannot make a
# valid call fail to type check, so requiring it would print rows that hide nothing.
_OBJECT_INHERITED = frozenset({"__repr__", "__str__", "__eq__", "__ne__", "__hash__"})

_PYCLASS_ATTRIBUTE = "#[pyclass"
_PYMETHODS_ATTRIBUTE = "#[pymethods]"

# `name = "X"` inside a `#[pyclass(...)]` / `#[pyo3(...)]` attribute.
_NAME_ARGUMENT_RE = re.compile(r'\bname\s*=\s*"([^"]+)"')
# The item a `#[pyclass]` decorates.
_TYPE_DECLARATION_RE = re.compile(r"^(?:pub(?:\([^)]*\))?\s+)?(struct|enum)\s+(\w+)")
# A method declaration at an `impl` block's own indentation.
_METHOD_DECLARATION_RE = re.compile(
    r"^    (?:pub(?:\([^)]*\))?\s+)?(?:const\s+|unsafe\s+|async\s+)*fn\s+(\w+)"
)
# An enum variant at the enum body's own indentation.
_VARIANT_RE = re.compile(r"^    ([A-Za-z_]\w*)\s*(?:[,({]|$)")
# `#[getter]`, `#[getter(x)]`, `#[getter(name = "x")]` — and the same three for setters.
_ACCESSOR_RE = re.compile(r"^#\[(getter|setter)(?:\((.*)\))?\]$")

# A `create_exception!(module, Name, Base, "doc")` invocation, and the arms of
# `PlanError::refusal`. A refusal name is lowercase ASCII with hyphens — `version` has
# none and `non-ascending-keys` has two, and both are names.
_CREATE_EXCEPTION_ATTRIBUTE = "create_exception!("
_CREATE_EXCEPTION_RE = re.compile(r"create_exception!\(\s*\w+\s*,\s*(\w+)\s*,")
_REFUSAL_LITERAL_RE = re.compile(r'"([a-z][a-z0-9]*(?:-[a-z0-9]+)*)"')
# A free function of the binding, and the exception one raises. Which exception carries
# `PlanError::refusal` is read off the CODE that raises it — see `refusal_boundaries`.
_FUNCTION_DECLARATION_RE = re.compile(r"^(?:pub(?:\([^)]*\))?\s+)?fn\s+\w+")
_NEW_ERR_RE = re.compile(r"\b(\w+)::new_err\s*\(")
_REFUSAL_CALL = ".refusal()"
# Anything in backticks, on one line, and the shape a refusal name is SPELLED in prose.
_BACKTICKED_RE = re.compile(r"`([^`\n]+)`")
_HYPHENATED_RE = re.compile(r"^[a-z][a-z0-9]*(?:-[a-z0-9]+)+$")


class Member:
    """One Python-visible member of one pyclass, and where it is declared in Rust."""

    def __init__(self, name: str, kind: str, where: str) -> None:
        self.name = name
        self.kind = kind
        self.where = where


class Exposed:
    """One `#[pyclass]`: its Python name and every member Python can reach on it."""

    def __init__(self, python_name: str, rust_name: str, where: str) -> None:
        self.python_name = python_name
        self.rust_name = rust_name
        self.where = where
        self.members: dict[str, Member] = {}

    def add(self, member: Member) -> None:
        # A `#[getter]`/`#[setter]` pair is ONE property and one stub line, so the first
        # of the pair stands for both rather than being reported twice.
        self.members.setdefault(member.name, member)


# The self-test's ONLY injection point: one file's text, substituted for the committed one
# while a mutation runs. Empty on every ordinary run, so the gate reads the tree and
# nothing else; a mutation is applied to a STRING and no tracked file is ever written.
_OVERLAY: dict[str, str] = {}


def _read(relative: str | Path) -> str:
    if str(relative) in _OVERLAY:
        return _OVERLAY[str(relative)]
    path = _REPO / relative
    if not path.is_file():
        raise SystemExit(f"check-python-stub-parity: {relative} is missing")
    return path.read_text(encoding="utf-8")


@contextmanager
def _mutated(relative: str, text: str) -> Generator[None]:
    """Run the body with `relative` reading as `text`, and restore the tree after."""
    _OVERLAY[relative] = text
    try:
        yield
    finally:
        _OVERLAY.clear()


def binding_sources() -> list[Path]:
    """Every Rust file of the Python binding crate, in a deterministic order."""
    found = sorted(
        path.relative_to(_REPO)
        for path in (_REPO / _BINDING_ROOT).rglob("*.rs")
    )
    if not found:
        raise SystemExit(
            f"check-python-stub-parity: found no Rust sources under {_BINDING_ROOT} — "
            "the crate layout moved; update this gate rather than leaving it vacuous"
        )
    return found


def _attribute_span(lines: list[str], index: int) -> int:
    """The index of the LAST line of the (possibly multi-line) attribute starting at `index`."""
    unclosed = lines[index].count("[") - lines[index].count("]")
    end = index
    while unclosed > 0 and end + 1 < len(lines):
        end += 1
        unclosed += lines[end].count("[") - lines[end].count("]")
    return end


def _decorated_item(lines: list[str], index: int, indent: str) -> tuple[int, list[str]] | None:
    """The item the attribute at `index` DECORATES, with the attributes in between.

    Whether an attribute decorates an item is a different question from whether it occurs
    in the item's file — `#[pyclass]` decorates forty-nine types of this crate — so the
    run between the attribute and its item is walked explicitly. Further attributes, doc
    comments and blank lines may sit in it and NOTHING else: the run stops at the first
    line that is none of those, so an unrelated item in between is an unrelated item.

    Returns `(line index of the item, the attribute lines seen on the way)`, or `None`.
    """
    attributes: list[str] = []
    cursor = _attribute_span(lines, index) + 1
    while cursor < len(lines):
        line = lines[cursor]
        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            cursor += 1
            continue
        if line.startswith(f"{indent}#["):
            end = _attribute_span(lines, cursor)
            attributes.append(" ".join(part.strip() for part in lines[cursor : end + 1]))
            cursor = end + 1
            continue
        if line.startswith(indent) and not line[len(indent) :].startswith(" "):
            return cursor, attributes
        return None
    return None


def _braced_body(lines: list[str], index: int) -> list[str]:
    """The brace-balanced body of the block whose header is at `index`."""
    depth = 0
    body: list[str] = []
    for cursor in range(index, len(lines)):
        line = lines[cursor]
        opened = depth
        depth += line.count("{") - line.count("}")
        if cursor > index:
            body.append(line)
        if opened > 0 and depth <= 0:
            return body[:-1]
        if depth <= 0 and cursor > index:
            return []
    raise SystemExit(
        f"check-python-stub-parity: the block opened at {lines[index]!r} is unterminated; "
        "the file's layout moved — update this gate rather than leaving it vacuous"
    )


_PYO3_ATTRIBUTE = "#[pyo3("


def _top_level_rename(attribute: str) -> str | None:
    """The Python name a `#[pyo3(…)]` attribute renames its item to, or `None`.

    Anchored to the attribute's TOP level, because `name = "x"` is a rename only there.
    Inside a nested group it is something else entirely: `#[pyo3(signature = (name =
    "graph", pretty = false))]` declares a PARAMETER called `name` with a string
    default, an entirely plausible spelling in an RDF library where graphs have names.
    Read flat, that parameter's default would be taken for the method's Python name —
    the gate would demand a stub `def graph(` and report the real method as undeclared,
    refusing a tree that is correct. So every nested parenthesized group is dropped
    before the rename is looked for, and `_ACCEPTED` holds this to it.
    """
    body = attribute.strip()
    if not body.startswith(_PYO3_ATTRIBUTE):
        return None
    depth = 1
    top_level: list[str] = []
    for character in body[len(_PYO3_ATTRIBUTE) :]:
        if character == "(":
            depth += 1
        elif character == ")":
            depth -= 1
            if depth == 0:
                break
        elif depth == 1:
            top_level.append(character)
    renamed = _NAME_ARGUMENT_RE.search("".join(top_level))
    return renamed.group(1) if renamed else None


def _python_member_name(function: str, attributes: list[str]) -> tuple[str, str] | None:
    """The Python name and kind of a `#[pymethods]` function, from what decorates it."""
    kind = "method"
    name = function
    for attribute in attributes:
        body = attribute.strip()
        if body == "#[new]":
            return "__init__", "constructor"
        if body == "#[classattr]":
            kind = "class attribute"
            continue
        accessor = _ACCESSOR_RE.match(body)
        if accessor:
            kind = "property"
            prefix = f"{accessor.group(1)[:3]}_"
            name = function.removeprefix(prefix)
            argument = (accessor.group(2) or "").strip()
            if argument:
                explicit = _NAME_ARGUMENT_RE.search(argument)
                name = explicit.group(1) if explicit else argument.strip('"')
            continue
        renamed = _top_level_rename(body)
        if renamed is not None:
            return renamed, kind
    return name, kind


def exposed_classes() -> dict[str, Exposed]:
    """Every `#[pyclass]` of the binding crate, keyed by its Rust type name.

    Both halves are read here: the type's Python name (and, for an enum, its variants,
    which PyO3 synthesizes as class attributes with no `#[pymethods]` block), and every
    method of every `#[pymethods]` block that `impl`s it.
    """
    classes: dict[str, Exposed] = {}
    pending: list[tuple[str, str, list[str]]] = []
    for source in binding_sources():
        lines = _read(source).splitlines()
        for index, line in enumerate(lines):
            if line.startswith(_PYCLASS_ATTRIBUTE):
                _read_pyclass(classes, lines, index, str(source))
            elif line == _PYMETHODS_ATTRIBUTE:
                pending.append(_read_pymethods(lines, index, str(source)))
    if not classes:
        raise SystemExit(
            "check-python-stub-parity: found no `#[pyclass]` under "
            f"{_BINDING_ROOT} — the crate layout moved; update this gate rather than "
            "leaving it silently vacuous"
        )
    if not pending:
        raise SystemExit(
            f"check-python-stub-parity: found no `{_PYMETHODS_ATTRIBUTE}` block under "
            f"{_BINDING_ROOT} — the crate layout moved; update this gate rather than "
            "leaving it silently vacuous"
        )
    for rust_name, where, body in pending:
        target = classes.get(rust_name)
        if target is None:
            raise SystemExit(
                f"check-python-stub-parity: {where} has a `{_PYMETHODS_ATTRIBUTE}` block "
                f"for `{rust_name}`, which carries no `#[pyclass]` this gate can find; "
                "the crate layout moved — update this gate rather than leaving it vacuous"
            )
        _read_methods(target, body, where)
    return classes


def _read_pyclass(
    classes: dict[str, Exposed], lines: list[str], index: int, source: str
) -> None:
    """Record the `#[pyclass]` at `index`, and an enum's variants with it."""
    end = _attribute_span(lines, index)
    attribute = " ".join(part.strip() for part in lines[index : end + 1])
    decorated = _decorated_item(lines, index, "")
    if decorated is None:
        raise SystemExit(
            f"check-python-stub-parity: the `#[pyclass]` at {source}:{index + 1} "
            "decorates no `struct`/`enum`; the file's layout moved — update this gate "
            "rather than leaving it vacuous"
        )
    item_index, _ = decorated
    declaration = _TYPE_DECLARATION_RE.match(lines[item_index])
    if declaration is None:
        raise SystemExit(
            f"check-python-stub-parity: the `#[pyclass]` at {source}:{index + 1} "
            f"decorates {lines[item_index]!r}, which this gate cannot read as a type; "
            "the file's layout moved — update this gate rather than leaving it vacuous"
        )
    shape, rust_name = declaration.group(1), declaration.group(2)
    renamed = _NAME_ARGUMENT_RE.search(attribute)
    python_name = renamed.group(1) if renamed else rust_name
    where = f"{source}:{item_index + 1}"
    if rust_name in classes:
        raise SystemExit(
            f"check-python-stub-parity: `{rust_name}` carries two `#[pyclass]` "
            f"attributes ({classes[rust_name].where} and {where}); this gate keys its "
            "methods by the Rust type name and cannot tell them apart"
        )
    exposed = Exposed(python_name, rust_name, where)
    classes[rust_name] = exposed
    if shape != "enum":
        return
    # A `#[pyclass] enum`'s variants ARE its Python members, synthesized with no
    # `#[pymethods]` block in sight — so a method-only derivation would leave every enum
    # of this crate outside the gate.
    body = _braced_body(lines, item_index)
    attributes: list[str] = []
    for line in body:
        stripped = line.strip()
        if line.startswith("    #["):
            attributes.append(stripped)
            continue
        if not stripped or stripped.startswith("//"):
            continue
        variant = _VARIANT_RE.match(line)
        if variant:
            renamed_variant = next(
                (
                    renamed
                    for renamed in (
                        _top_level_rename(attribute) for attribute in attributes
                    )
                    if renamed is not None
                ),
                None,
            )
            name = renamed_variant if renamed_variant is not None else variant.group(1)
            exposed.add(Member(name, "enum member", where))
        attributes = []


def _read_pymethods(lines: list[str], index: int, source: str) -> tuple[str, str, list[str]]:
    """The `(Rust type, where, body)` of the `#[pymethods]` block at `index`."""
    decorated = _decorated_item(lines, index, "")
    if decorated is None:
        raise SystemExit(
            f"check-python-stub-parity: the `{_PYMETHODS_ATTRIBUTE}` at "
            f"{source}:{index + 1} decorates no item; the file's layout moved — update "
            "this gate rather than leaving it vacuous"
        )
    item_index, _ = decorated
    target = re.match(r"^impl\s+(?:<[^>]*>\s*)?(\w+)", lines[item_index])
    if target is None:
        raise SystemExit(
            f"check-python-stub-parity: the `{_PYMETHODS_ATTRIBUTE}` at "
            f"{source}:{index + 1} decorates {lines[item_index]!r}, which this gate "
            "cannot read as an `impl`; the file's layout moved"
        )
    return target.group(1), f"{source}:{item_index + 1}", _braced_body(lines, item_index)


def _read_methods(exposed: Exposed, body: list[str], where: str) -> None:
    """Record every Python-visible member declared in one `#[pymethods]` body.

    Every `fn` in a `#[pymethods]` block is exposed — PyO3 admits no private one — so the
    derivation is the block's own items, at the block's own indentation. A `fn` nested in
    a method's body is indented further and is not an item of the block.
    """
    attributes: list[str] = []
    cursor = 0
    while cursor < len(body):
        line = body[cursor]
        stripped = line.strip()
        if line.startswith("    #["):
            end = _attribute_span(body, cursor)
            attributes.append(" ".join(part.strip() for part in body[cursor : end + 1]))
            cursor = end + 1
            continue
        if not stripped or stripped.startswith("//"):
            cursor += 1
            continue
        declaration = _METHOD_DECLARATION_RE.match(line)
        if declaration:
            resolved = _python_member_name(declaration.group(1), attributes)
            if resolved is not None:
                name, kind = resolved
                exposed.add(Member(name, kind, where))
        attributes = []
        cursor += 1


def stub_classes(text: str) -> dict[str, list[str]]:
    """Every `class X:` of the stub, keyed by name, with its body lines.

    By INDENTATION, because a nested class (`class shapes:` re-exports the engine
    classes) is a class too, and because a `def` that has drifted out of a class body is
    exactly the drift this gate is about.
    """
    lines = text.splitlines()
    found: dict[str, list[str]] = {}
    for index, line in enumerate(lines):
        header = re.match(r"^(\s*)class\s+(\w+)\s*(?:\([^)]*\))?\s*:", line)
        if header is None:
            continue
        indent = len(header.group(1))
        body: list[str] = []
        for following in lines[index + 1 :]:
            if following.strip() and (len(following) - len(following.lstrip())) <= indent:
                break
            body.append(following)
        found[header.group(2)] = body
    if not found:
        raise SystemExit(
            f"check-python-stub-parity: found no `class` in {_STUB} — the stub's layout "
            "moved; update this gate rather than leaving it vacuous"
        )
    return found


def _resolves_to(python_name: str, stubs: dict[str, list[str]]) -> tuple[str, list[str]] | None:
    """The stub class declaring `python_name`, under the stub's own naming convention.

    An engine class re-exported inside a namespace class carries an underscore-prefixed
    module-level name (`_PreparedShapes` is `purrdf.shapes.PreparedShapes`), because a
    plain `X = X` in a class body reads as a self-referential type alias.
    """
    for candidate in (python_name, f"_{python_name}"):
        if candidate in stubs:
            return candidate, stubs[candidate]
    return None


def _declares(body: list[str], member: str) -> bool:
    """Whether a stub class body declares `member`.

    A `def <member>(` covers `@property`, `@staticmethod` and `@overload` alike; an
    annotation covers an enum member and a plain attribute.
    """
    return member in _declared_members(body)


_STUB_MEMBER_RE = re.compile(r"(?:def\s+(\w+)\s*\(|(\w+)\s*:)")


def _member_lines(body: list[str]) -> list[str]:
    """The body lines at the class's OWN member indentation.

    A parameter of a multi-line `def` is spelled `name: type` exactly as an attribute
    is, and a `def` signature that wraps puts one on its own line — so a gate reading
    the body flat sees `Store.max_answers` as an attribute of `Store`. They are told
    apart by INDENTATION: a member sits at the body's outermost level and a parameter
    is indented past it.
    """
    indents = [len(line) - len(line.lstrip()) for line in body if line.strip()]
    if not indents:
        return []
    member_indent = min(indents)
    return [
        line[member_indent:]
        for line in body
        if line.strip() and (len(line) - len(line.lstrip())) == member_indent
    ]


def _is_dunder(name: str) -> bool:
    return name.startswith("__") and name.endswith("__")


def _declared_members(body: list[str]) -> set[str]:
    """Every member name a stub class body declares, at its own indentation."""
    found: set[str] = set()
    for line in _member_lines(body):
        declared = _STUB_MEMBER_RE.match(line)
        if declared:
            found.add(declared.group(1) or declared.group(2))
    return found


# ── the pinned refusal vocabulary, at both ends of the same contract ───────────────────


def refusal_names() -> frozenset[str]:
    """Every pinned name `PlanError::refusal` can return, read out of the engine.

    The match is exhaustive over a `#[non_exhaustive]` enum, so a variant added without a
    name is a compile error in that file and the arms' string literals are the whole
    vocabulary. Deriving it here is the point: a list in this script would be a third
    description of the contract, free to drift from the two it is checking.
    """
    lines = _read(_ENGINE_ERRORS).splitlines()
    for index, line in enumerate(lines):
        if _REFUSAL_SIGNATURE in line:
            names = frozenset(
                _REFUSAL_LITERAL_RE.findall("\n".join(_braced_body(lines, index)))
            )
            if len(names) < 2:
                raise SystemExit(
                    f"check-python-stub-parity: `{_REFUSAL_SIGNATURE}` in "
                    f"{_ENGINE_ERRORS} yields {len(names)} name(s); its arms no longer "
                    "read as string literals — update this gate rather than leaving it "
                    "checking an empty vocabulary"
                )
            return names
    raise SystemExit(
        f"check-python-stub-parity: {_ENGINE_ERRORS} declares no "
        f"`{_REFUSAL_SIGNATURE}`; the engine's refusal vocabulary moved — update this "
        "gate rather than leaving it vacuous"
    )


def _paren_block(lines: list[str], index: int, where: str) -> str:
    """The paren-balanced invocation whose first line is at `index`, as one string."""
    depth = 0
    collected: list[str] = []
    for cursor in range(index, len(lines)):
        line = lines[cursor]
        opened = depth
        depth += line.count("(") - line.count(")")
        collected.append(line)
        if opened > 0 and depth <= 0:
            return "\n".join(collected)
    raise SystemExit(
        f"check-python-stub-parity: the invocation at {where} never closes its "
        "parentheses; the file's layout moved — update this gate rather than leaving "
        "it vacuous"
    )


def exception_classes() -> dict[str, tuple[str, str]]:
    """Every `create_exception!` of the binding: its Python name, where, and its text."""
    found: dict[str, tuple[str, str]] = {}
    for source in binding_sources():
        lines = _read(source).splitlines()
        for index, line in enumerate(lines):
            if not line.startswith(_CREATE_EXCEPTION_ATTRIBUTE):
                continue
            where = f"{source}:{index + 1}"
            block = _paren_block(lines, index, where)
            named = _CREATE_EXCEPTION_RE.search(block)
            if named is None:
                raise SystemExit(
                    f"check-python-stub-parity: the `create_exception!` at {where} does "
                    "not read as `create_exception!(module, Name, Base, …)`; the macro's "
                    "spelling moved — update this gate rather than leaving it vacuous"
                )
            if named.group(1) in found:
                raise SystemExit(
                    f"check-python-stub-parity: `{named.group(1)}` is created twice "
                    f"({found[named.group(1)][0]} and {where}); this gate keys a "
                    "boundary by its Python name and cannot tell them apart"
                )
            found[named.group(1)] = (where, block)
    if not found:
        raise SystemExit(
            "check-python-stub-parity: found no `create_exception!` under "
            f"{_BINDING_ROOT} — the crate layout moved; update this gate rather than "
            "leaving it silently vacuous"
        )
    return found


def _backticked(text: str) -> set[str]:
    """Every backticked token of `text`, exactly as prose spells a pinned name."""
    return set(_BACKTICKED_RE.findall(text))


def _promised_names(text: str, names: frozenset[str]) -> set[str]:
    """Every hyphenated token `text` backticks INSIDE its enumeration of pinned names.

    A docstring that enumerates a vocabulary is still mostly prose, and backticking a
    code term is ordinary Markdown: `plan-document`, `content-addressed` and
    `length-framed` are all words these two docstrings may legitimately acquire. Reading
    every backticked hyphenated token of the whole block as a promised refusal name made
    this arm refuse that — an over-refusal, and one a reader could not satisfy without
    editing this gate, because the message told them a word `refusal()` cannot return is
    a broken promise rather than telling them where the promise is being read from.

    So the scan is scoped to the ENUMERATING REGION, and the region is defined locally
    rather than as one span of the block: a hyphenated token counts as promised when the
    backticked token immediately before or after it is one of the engine's own names.
    That is what an enumeration is — a run of names written next to each other — and
    locality is what keeps the arm's real purpose. A refusal renamed in the engine sits
    in the middle of the run it was part of, with a surviving name on at least one side
    of it, so it is still read as the stale promise it is. One span from the first known
    name to the last would have missed a rename of whichever name the docstring
    enumerates LAST, which is where the stub's run ends and where a reader adding the
    newest refusal writes.

    Prose outside a run is free: a term whose backticked neighbours are not names, or
    which has no backticked neighbour at all, is a code term and not a promise.
    """
    tokens = _BACKTICKED_RE.findall(text)

    def beside_a_name(index: int) -> bool:
        before = tokens[max(index - 1, 0) : index]
        after = tokens[index + 1 : index + 2]
        return any(neighbour in names for neighbour in before + after)

    return {
        token
        for index, token in enumerate(tokens)
        if token not in names and _HYPHENATED_RE.match(token) and beside_a_name(index)
    }


def refusal_boundaries() -> dict[str, str]:
    """Every `create_exception!` class the binding raises carrying `PlanError::refusal`.

    Read off the CODE that raises it — a free function whose body both calls `.refusal()`
    and constructs that exception — and never off the docstring. Which vocabulary a
    boundary carries is a fact about the raise site: `ShapesProductError` is built from a
    different enum's `dimension_label()` and is outside this arm because its raise site
    says so, not because a table here excludes it. Reading the docstring instead would
    have caught `ShapesProductError` on the word `truncated`, which both vocabularies
    happen to contain, and demanded twenty labels of a stub that documents none.
    """
    raised: dict[str, str] = {}
    for source in binding_sources():
        lines = _read(source).splitlines()
        for index, line in enumerate(lines):
            if not _FUNCTION_DECLARATION_RE.match(line):
                continue
            body = "\n".join(_braced_body(lines, index))
            if _REFUSAL_CALL not in body:
                continue
            for exception in _NEW_ERR_RE.findall(body):
                raised.setdefault(exception, f"{source}:{index + 1}")
    boundaries = {
        exception: where
        for exception, where in raised.items()
        if exception in exception_classes()
    }
    if not boundaries:
        raise SystemExit(
            f"check-python-stub-parity: no function under {_BINDING_ROOT} raises a "
            f"`create_exception!` class carrying `{_REFUSAL_CALL}`; the retrieval "
            "binding's refusal boundary moved — update this gate rather than leaving "
            "the name arm checking nothing"
        )
    return boundaries


def refusal_parity_problems(stubs: dict[str, list[str]]) -> list[str]:
    """Every way the two descriptions of the `.refusal` vocabulary disagree."""
    names = refusal_names()
    blocks = exception_classes()
    problems: list[str] = []
    for class_name, raise_site in sorted(refusal_boundaries().items()):
        where, block = blocks[class_name]
        runtime = _backticked(block) & names
        if not runtime:
            raise SystemExit(
                f"check-python-stub-parity: `{class_name}` is raised at {raise_site} "
                f"carrying `{_REFUSAL_CALL}`, and its docstring ({where}) names none of "
                f"the names `{_REFUSAL_SIGNATURE}` returns — the vocabulary or the way "
                "it is written down moved; update this gate rather than leaving the "
                "name arm silently comparing two empty sets"
            )
        resolved = _resolves_to(class_name, stubs)
        if resolved is None:
            problems.append(
                f"  • {class_name} ({where}): the extension creates this exception and "
                f"{_STUB} declares NEITHER `class {class_name}:` nor "
                f"`class _{class_name}:`, so the pinned names it documents are declared "
                "nowhere a type checker reads."
            )
            continue
        stub_name, body = resolved
        declared = _backticked("\n".join(body))
        for missing in sorted(runtime - (declared & names)):
            problems.append(
                f"  • {class_name}.refusal `{missing}`: the extension's own docstring "
                f"({where}) names it and `class {stub_name}:` in {_STUB} does not. The "
                "package ships `py.typed`, so that docstring is what `help()` shows and "
                "the stub's is what a caller's type checker and IDE show — a name only "
                "one of them carries does not exist for whoever reads the other."
            )
        for phantom in sorted((declared & names) - runtime):
            problems.append(
                f"  • {class_name}.refusal `{phantom}`: `class {stub_name}:` in {_STUB} "
                f"names it and the extension's own docstring ({where}) does not, so a "
                "caller is told to branch on a refusal this boundary does not raise."
            )
        for text, site in ((block, where), ("\n".join(body), f"{_STUB} `{stub_name}`")):
            for unknown in sorted(_promised_names(text, names)):
                problems.append(
                    f"  • {class_name}.refusal `{unknown}`: enumerated at {site} beside "
                    f"the pinned names, and `{_REFUSAL_SIGNATURE}` in {_ENGINE_ERRORS} "
                    "cannot return it. Either the engine renamed the refusal and the "
                    "prose still promises the old word, or it is an ordinary code term "
                    "that happens to sit in the run of names — move it into a sentence "
                    "whose backticked neighbours are not refusal names, where a "
                    "hyphenated term is read as prose and promises nothing."
                )
    return problems


def gate_problems() -> list[str]:
    """Every member the extension exposes and the stub does not declare.

    Separate from [`main`] because the self-test runs it once per mutation and reads the
    answer rather than an exit code.
    """
    classes = exposed_classes()
    stubs = stub_classes(_read(_STUB))
    problems: list[str] = []
    for exposed in sorted(classes.values(), key=lambda item: item.python_name):
        resolved = _resolves_to(exposed.python_name, stubs)
        if resolved is None:
            problems.append(
                f"  • {exposed.python_name} ({exposed.where}): the extension exposes this "
                f"class and {_STUB} declares NEITHER `class {exposed.python_name}:` nor "
                f"`class _{exposed.python_name}:`. The package ships `py.typed`, so a "
                "class the stub does not declare is unreachable from every checked caller."
            )
            continue
        stub_name, body = resolved
        for declared in sorted(_declared_members(body)):
            if _is_dunder(declared):
                # The reverse arm skips dunders, and only the reverse arm does. A
                # `#[pyclass(eq, ord, hash, str)]` SYNTHESIZES `__eq__`, the ordering
                # four, `__hash__` and `__str__` with no `fn` anywhere for this gate to
                # read, and `object` supplies the rest — so a dunder in the stub is a
                # declaration of something that really is there, and demanding a
                # `#[pymethods]` for it would refuse a correct stub.
                continue
            if declared not in exposed.members:
                problems.append(
                    f"  • {exposed.python_name}.{declared}: `class {stub_name}:` in "
                    f"{_STUB} declares it and the extension ({exposed.where}) exposes "
                    "NO such member."
                )
        for member in sorted(exposed.members.values(), key=lambda item: item.name):
            if member.name in _OBJECT_INHERITED:
                continue
            if not _declares(body, member.name):
                problems.append(
                    f"  • {exposed.python_name}.{member.name} ({member.kind}, declared at "
                    f"{member.where}): live at runtime, and `class {stub_name}:` in "
                    f"{_STUB} does not declare it. Add a `def {member.name}(` (or an "
                    "annotation) to that class body."
                )
    problems.extend(refusal_parity_problems(stubs))
    return problems


# ── The gate's own falsifiability ──────────────────────────────────────────────────────

# The class and member every mutation below is written against. One specimen is enough
# because the arms are a LOOP over the derived classes: an arm that catches a mutation of
# one catches it for all of them, and an arm that catches none is the tautology this
# section exists to make impossible.
_SPECIMEN_SOURCE = "bindings/python/src/shacl.rs"
_SPECIMEN_CLASS = "ChangeValidation"

# The name arm's specimen: where `PlanDocumentError` is created and documented.
_REFUSAL_SPECIMEN_SOURCE = "bindings/python/src/py_retrieval.rs"


def _cut(text: str, needle: str) -> str:
    """`text` with its first `needle` removed."""
    if needle not in text:
        raise SystemExit(f"{needle!r} is no longer there")
    return text.replace(needle, "", 1)


def _swap(text: str, old: str, new: str) -> str:
    """`text` with its first `old` replaced by `new`."""
    if old not in text:
        raise SystemExit(f"{old!r} is no longer there")
    return text.replace(old, new, 1)


# One mutation per arm, at least: a construct exactly one check claims to read, removed or
# moved, in memory. Each must make `gate_problems` non-empty — a mutation the gate
# SURVIVES is a check that proves nothing, and is reported by name.
#
# A mutation that can no longer be APPLIED fails too, and loudly: its needle is gone
# because the tree moved, and a self-test quietly testing nothing is the same defect one
# layer up.
_MUTATIONS: tuple[tuple[str, str, Callable[[str], str]], ...] = (
    # ── a plain method ──
    (
        "a binding grows a method the stub does not declare",
        _SPECIMEN_SOURCE,
        lambda text: _swap(
            text,
            "#[pymethods]\nimpl PyPreparedShapes {",
            "#[pymethods]\nimpl PyPreparedShapes {\n    fn undeclared_lane(&self) {}\n",
        ),
    ),
    (
        "the stub's `def` for a method is renamed out from under the binding",
        str(_STUB),
        lambda text: _swap(
            text,
            "    def validate_store_changes(",
            "    def validate_store_changes_legacy(",
        ),
    ),
    (
        "the stub's `def` is moved out of its class, to module scope",
        str(_STUB),
        lambda text: _swap(
            text, "    def validate_store_changes(", "def validate_store_changes("
        ),
    ),
    # ── a `#[getter]` property ──
    (
        "a `#[getter]` property is dropped from the stub",
        str(_STUB),
        lambda text: _swap(text, "    def focus_nodes(", "    def focus_nodes_gone("),
    ),
    (
        "a binding grows a `#[getter]` the stub does not declare",
        _SPECIMEN_SOURCE,
        lambda text: _swap(
            text,
            "#[pymethods]\nimpl PyChangeValidation {",
            "#[pymethods]\nimpl PyChangeValidation {\n    #[getter]\n"
            "    const fn get_undeclared_scope(&self) -> bool {\n        true\n    }\n",
        ),
    ),
    # ── the reverse direction: a declaration with no binding behind it ──
    (
        "the stub grows a `def` no `#[pymethods]` backs",
        str(_STUB),
        lambda text: _swap(
            text,
            f"class _{_SPECIMEN_CLASS}:\n",
            f"class _{_SPECIMEN_CLASS}:\n    def phantom_lane(self) -> None: ...\n",
        ),
    ),
    # ── `#[new]` is `__init__` ──
    (
        "a `#[new]` constructor's `__init__` is dropped from the stub",
        str(_STUB),
        lambda text: _swap(
            text,
            "class CancellationToken:\n    def __init__(self) -> None: ...",
            "class CancellationToken:",
        ),
    ),
    # ── `#[pyo3(name = …)]` renames ──
    (
        "a `#[pyo3(name = …)]` renames a method away from its stub declaration",
        "bindings/python/src/py_store/store.rs",
        lambda text: _swap(
            text, '#[pyo3(name = "_store_capsule")]', '#[pyo3(name = "_store_capsule_v2")]'
        ),
    ),
    # ── the class itself ──
    (
        "a `#[pyclass]` is renamed and the stub has no class for it",
        _SPECIMEN_SOURCE,
        lambda text: _swap(
            text,
            f'#[pyclass(name = "{_SPECIMEN_CLASS}")]',
            f'#[pyclass(name = "{_SPECIMEN_CLASS}Native")]',
        ),
    ),
    (
        "the stub's class is renamed out from under the binding",
        str(_STUB),
        lambda text: _swap(
            text, f"class _{_SPECIMEN_CLASS}:", f"class _{_SPECIMEN_CLASS}Legacy:"
        ),
    ),
    # ── a `#[pyclass] enum`'s variants ──
    (
        "an enum grows a member the stub does not declare",
        "bindings/python/src/py_store/io.rs",
        lambda text: _swap(
            text, "pub(crate) enum PyRdfFormat {\n", "pub(crate) enum PyRdfFormat {\n    N3,\n"
        ),
    ),
    (
        "an enum member is dropped from the stub",
        str(_STUB),
        lambda text: _swap(text, "    YAML_LD: RdfFormat", "    YAML_LD_GONE: RdfFormat"),
    ),
    # ── the derivation itself ──
    (
        "every `#[pymethods]` block is stripped",
        _SPECIMEN_SOURCE,
        lambda text: _cut(text, "#[pymethods]\n").replace("#[pymethods]\n", ""),
    ),
    (
        "the `#[pymethods]` attribute comes adrift of its `impl`",
        _SPECIMEN_SOURCE,
        lambda text: _swap(
            text,
            "#[pymethods]\nimpl PyChangeValidation {",
            "#[pymethods]\nconst ADRIFT: bool = true;\nimpl PyChangeValidation {",
        ),
    ),
    # ── the pinned `.refusal` vocabulary, at both ends ──
    #
    # The live defect this arm was written for: the stub named fifteen of nineteen, and
    # the four it omitted were reachable from Python against that very build. The first
    # mutation is that defect, reconstructed — a name the extension documents, taken out
    # of the stub and left as prose so nothing else about the docstring changes.
    (
        "a refusal the extension documents is dropped from the stub's docstring",
        str(_STUB),
        lambda text: _swap(
            text,
            "`unconsulted-statistics-subject`. Branch on it",
            "that last one. Branch on it",
        ),
    ),
    (
        "the stub documents a refusal the extension's own docstring does not",
        _REFUSAL_SPECIMEN_SOURCE,
        lambda text: _swap(text, "`unconsulted-statistics-subject`", "that last one"),
    ),
    (
        "the engine renames a refusal and both docstrings promise the old word",
        str(_ENGINE_ERRORS),
        lambda text: _swap(
            text, '"unconsulted-statistics-subject"', '"unconsulted-statistics-row"'
        ),
    ),
    (
        "the stub's class for the refusal boundary is renamed out from under it",
        str(_STUB),
        lambda text: _swap(
            text,
            "class _PlanDocumentError(ValueError):",
            "class _PlanDocumentErrorLegacy(ValueError):",
        ),
    ),
    (
        "the engine's refusal accessor is renamed, leaving the vocabulary underivable",
        str(_ENGINE_ERRORS),
        lambda text: _swap(
            text, _REFUSAL_SIGNATURE, "pub fn refusal_name(&self) -> &'static str"
        ),
    ),
)


# ── the mirror arm: valid trees this gate must ACCEPT ──────────────────────────────────

# A refusal is a claim too. `_MUTATIONS` proves this gate REFUSES a broken tree and
# proves nothing whatsoever about what it accepts — and an over-refusal is the mirror of
# the silent drop: every mutation still fails, the gate still reads as strict, and a
# correct tree is rejected with a message indistinguishable from a real defect. So each
# entry below is a VALID spelling this gate could plausibly mis-read, applied the same
# way, and each must leave `gate_problems` EMPTY. An entry that comes back with problems
# is an over-refusal and fails this script exactly as a survived mutation does.
_ACCEPTED: tuple[tuple[str, str, Callable[[str], str]], ...] = (
    (
        'a `signature = (name = "…")` parameter default is not a rename',
        _SPECIMEN_SOURCE,
        lambda text: _swap(
            text,
            "    fn to_sarif(&self, py: Python<'_>) -> String {",
            '    #[pyo3(signature = (name = "report", pretty = false))]\n'
            "    fn to_sarif(&self, py: Python<'_>, name: &str, pretty: bool) -> String {",
        ),
    ),
    # The name arm's own over-refusal, and the one it actually committed once: a
    # `create_exception!` that carries some OTHER enum's pinned labels. Its docstring
    # already shares the word `truncated` with `PlanError`'s vocabulary, so a gate
    # selecting on prose demands this stub document twenty `.dimension` labels it
    # deliberately summarizes. A label added there must change nothing here.
    (
        "a pinned label of a boundary that carries no `PlanError::refusal`",
        _SPECIMEN_SOURCE,
        lambda text: _swap(
            text, "`depth-limit`, `malformed`", "`depth-limit`, `malformed`, `newly-added`"
        ),
    ),
    # Presence, not sequence: the stub is free to group and order the names however it
    # reads best, and a gate comparing two lists positionally would refuse this.
    (
        "the stub names the same refusals in another order",
        str(_STUB),
        lambda text: _swap(
            text, "`truncated`, `trailing-bytes`", "`trailing-bytes`, `truncated`"
        ),
    ),
    # Prose hyphenation is prose. Only a BACKTICKED hyphenated word is read as a name,
    # which is what makes the unknown-name arm above safe to run over prose at all.
    (
        "a hyphenated word in the stub's prose, unbackticked",
        str(_STUB),
        lambda text: _swap(
            text,
            "Branch on it, never on `str(exc)`.",
            "Branch on it at the plan-document boundary, never on `str(exc)`.",
        ),
    ),
    # Backticking a code term is ordinary Markdown, and these are the two docstrings
    # most likely to acquire one: `plan-document`, `content-addressed`, `length-framed`
    # are all words about this very boundary. The unknown-name arm once read ANY
    # backticked hyphenated token of either block as a promised refusal name and refused
    # all four spellings below — a gate a reader could not satisfy without editing the
    # gate. Both ENDS of both docstrings are held here, because the enumeration has two
    # of them and prose sits on each side.
    (
        "a backticked code term in the stub's prose, before the enumeration",
        str(_STUB),
        lambda text: _swap(
            text,
            "A refusal from the plan-document boundary: `certify_plan`, `explain_depth`.",
            "A refusal from the `content-addressed` plan-document boundary:\n"
            "    `certify_plan`, `explain_depth`.",
        ),
    ),
    (
        "a backticked code term in the stub's prose, after the enumeration",
        str(_STUB),
        lambda text: _swap(
            text,
            "`unconsulted-statistics-subject`. Branch on it, never on `str(exc)`.",
            "`unconsulted-statistics-subject`. Branch on it, never on `str(exc)`. The\n"
            "    bytes it refused are `length-framed`.",
        ),
    ),
    (
        "a backticked code term in the `create_exception!` prose, before the enumeration",
        _REFUSAL_SPECIMEN_SOURCE,
        lambda text: _swap(
            text,
            "A refusal from the plan-document boundary —",
            "A refusal from the `content-addressed` plan-document boundary —",
        ),
    ),
    (
        "a backticked code term in the `create_exception!` prose, after the enumeration",
        _REFUSAL_SPECIMEN_SOURCE,
        lambda text: _swap(
            text,
            '`ValueError` keeps working."',
            "`ValueError` keeps working.\\n\\\n      \\n\\\n"
            '      The bytes it refused are `length-framed`."',
        ),
    ),
)


def accepts_valid_trees(report: bool) -> list[str]:
    """Every VALID tree this gate refuses. An empty list is the only passing answer."""
    refused: list[str] = []
    for what, relative, spell in _ACCEPTED:
        try:
            text = spell(_read(relative))
        except SystemExit as stale:
            raise SystemExit(
                f"check-python-stub-parity: the self-test cannot apply its valid "
                f"spelling {what!r} to {relative}: {stale}. The tree moved — update the "
                "arm rather than leaving the self-test proving nothing."
            ) from stale
        with _mutated(relative, text):
            try:
                problems = gate_problems()
            except SystemExit as refusal:
                problems = [f"  • the gate refused to read the tree: {refusal}"]
        if report:
            print(f"  {'accepted' if not problems else 'REFUSED':8}  {relative}: {what}")
        if problems:
            refused.append(f"  • {relative}: {what}\n" + "\n".join(problems))
    return refused


def self_test(report: bool) -> list[str]:
    """Every mutation this gate does NOT catch. An empty list is the only passing answer."""
    survived: list[str] = []
    for what, relative, mutate in _MUTATIONS:
        try:
            text = mutate(_read(relative))
        except SystemExit as stale:
            raise SystemExit(
                f"check-python-stub-parity: the self-test cannot apply its mutation "
                f"{what!r} to {relative}: {stale}. The tree moved — update the mutation "
                "rather than leaving the self-test proving nothing."
            ) from stale
        with _mutated(relative, text):
            try:
                caught = "caught" if gate_problems() else ""
            except SystemExit:
                # A mutation the gate REFUSES to read is still a mutation the gate does
                # not pass: it exits non-zero naming the file whose layout moved.
                caught = "refused"
        if report:
            print(f"  {caught or 'SURVIVED':8}  {relative}: {what}")
        if not caught:
            survived.append(f"  • {relative}: {what}")
    return survived


def main(argv: list[str]) -> int:
    unknown = [argument for argument in argv[1:] if argument != "--self-test"]
    if unknown:
        print(f"usage: {Path(argv[0]).name} [--self-test]", file=sys.stderr)
        return 2
    alone = "--self-test" in argv[1:]

    if alone:
        print(
            f"check-python-stub-parity: mutating the committed tree {len(_MUTATIONS)} "
            "ways, each of which must fail this gate —"
        )
    # BEFORE the gate's own verdict, on every run: a green light this script cannot
    # withhold is worth nothing. It is pure text over the same files, so the whole suite
    # costs a fraction of a second.
    survived = self_test(report=alone)
    if survived:
        print(
            "check-python-stub-parity: this gate PASSES a tree it is written to refuse:\n"
            + "\n".join(survived)
            + "\n\nEach line above is a mutation that makes a member undeclared and leaves "
            "this script exiting 0 — a green light with nothing behind it. Fix the check, "
            "not the mutation.",
            file=sys.stderr,
        )
        return 1

    if alone:
        print(
            f"check-python-stub-parity: and spelling it {len(_ACCEPTED)} valid way(s), "
            "each of which must PASS —"
        )
    # The other half of the same obligation: refusing everything is not strictness.
    refused = accepts_valid_trees(report=alone)
    if refused:
        print(
            "check-python-stub-parity: this gate REFUSES a tree that is correct:\n"
            + "\n".join(refused)
            + "\n\nEach block above is a valid spelling this gate reports as a defect — an "
            "over-refusal, which is the mirror of a missed member and just as much a bug "
            "in the check. Fix the check, not the spelling.",
            file=sys.stderr,
        )
        return 1

    if alone:
        print(
            f"OK: all {len(_MUTATIONS)} mutations of the committed tree fail this gate, "
            f"and all {len(_ACCEPTED)} valid spelling(s) of it pass."
        )
        return 0

    problems = gate_problems()
    if problems:
        print(
            "check-python-stub-parity: the PyO3 extension exposes members the type stub "
            "does not declare:\n" + "\n".join(problems),
            file=sys.stderr,
        )
        return 1

    classes = exposed_classes()
    members = sum(len(exposed.members) for exposed in classes.values())
    blocks = exception_classes()
    boundaries = refusal_boundaries()
    documented = sum(
        len(_backticked(blocks[name][1]) & refusal_names()) for name in boundaries
    )
    print(
        f"OK: all {members} Python-visible member(s) of {len(classes)} `#[pyclass]`(es) "
        f"are declared in {_STUB}, as are all {documented} pinned `.refusal` name(s) of "
        f"{len(boundaries)} `create_exception!` boundary(ies); all {len(_MUTATIONS)} "
        f"mutations of that tree fail this gate; and all {len(_ACCEPTED)} valid "
        "spelling(s) of it pass."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
