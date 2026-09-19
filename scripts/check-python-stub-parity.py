#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

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
    return problems


# ── The gate's own falsifiability ──────────────────────────────────────────────────────

# The class and member every mutation below is written against. One specimen is enough
# because the arms are a LOOP over the derived classes: an arm that catches a mutation of
# one catches it for all of them, and an arm that catches none is the tautology this
# section exists to make impossible.
_SPECIMEN_SOURCE = "bindings/python/src/shacl.rs"
_SPECIMEN_CLASS = "ChangeValidation"


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
            text, "pub(crate) enum PyRdfFormat {\n", "pub(crate) enum PyRdfFormat {\n    RDF_XML,\n"
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
    print(
        f"OK: all {members} Python-visible member(s) of {len(classes)} `#[pyclass]`(es) "
        f"are declared in {_STUB}; all {len(_MUTATIONS)} mutations of that tree fail this "
        f"gate; and all {len(_ACCEPTED)} valid spelling(s) of it pass."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
