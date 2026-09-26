#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Build gate: the asynchronous lane's suspending import is followed, in every
function that calls it, by that function's own stack-pointer restore.

The wasm package runs asynchronous jobs on private shadow-stack regions and
suspends them through one raw import, `purrdf_jspi_suspend` from
`./purrdf_jspi.mjs` (see `crates/rdf-wasm/src/async_query.rs`). While a job is
suspended, the host switches `__stack_pointer` back to its own context. When the
job resumes, the global may hold another context's value, so the resumed code
must write its *own* frame's pointer back before anything can allocate a frame:
the first stack-relevant instruction after the import returns has to be the
frame's `global.set` of the stack pointer. A call, a `global.get` of the stack
pointer (which would read the stale value), a branch that could skip the restore,
or the end of the function (no frame at all) coming first is a latent stack
corruption that no test run can be trusted to hit.

`wasm-opt` inlines the Rust frame function despite `#[inline(never)]`, and the
release artifact carries no name section, so the gate is structural rather than a
call-site count: it disassembles the module with binaryen's `wasm-dis`, resolves
the import's function name from the import section, resolves the stack-pointer
global as the one the function exported as `__wbindgen_add_to_stack_pointer`
sets, and, for every function that calls the import, walks the body in execution
order (folded operands before their operator) from each call site. It also fails
when no function calls the import (the lane was compiled out) and when the import
is referenced any other way (a table entry or `ref.func` would allow an indirect
call the walk cannot see).

The disassembly is read in binaryen's layout: one module field per line, starting
at a one-space indent. Only the functions the gate needs are parsed.

Usage: `check-wasm-jspi-frame.py <module.wasm | module.wat>`, or `--self-test` to run
the fixture suite in `scripts/test_check_wasm_jspi_frame.py`.
"""

from __future__ import annotations

import bisect
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

IMPORT_MODULE = "./purrdf_jspi.mjs"
IMPORT_NAME = "purrdf_jspi_suspend"
STACK_POINTER_EXPORT = "__wbindgen_add_to_stack_pointer"

# Wasm text identifier characters (the `idchar` production).
_IDCHARS = r"0-9A-Za-z!#$%&'*+\-./:<=>?@\\^_`|~"

_TOKEN = re.compile(
    r'"(?:[^"\\]|\\.)*"'  # string
    r"|;;[^\n]*"  # line comment
    r"|\(;.*?;\)"  # block comment
    r"|\(|\)"
    r"|[^\s()\";]+",
    re.DOTALL,
)

_FIELD_START = re.compile(r"^ \(", re.MULTILINE)

# Instructions after which the walk cannot vouch for what runs next.
_CALLS = frozenset(
    {
        "call",
        "call_indirect",
        "call_ref",
        "return_call",
        "return_call_indirect",
        "return_call_ref",
    }
)
_TRANSFERS = frozenset(
    {
        "br",
        "br_if",
        "br_table",
        "br_on_null",
        "br_on_non_null",
        "br_on_cast",
        "br_on_cast_fail",
        "return",
        "if",
        "else",
        "throw",
        "throw_ref",
        "rethrow",
        "delegate",
    }
)
# Function header fields that are not instructions.
_FUNC_HEADER = frozenset({"param", "result", "local", "type", "export", "import"})
# Block-type annotations on a structured instruction.
_BLOCK_HEADER = frozenset({"param", "result", "type"})


class GateError(Exception):
    """The module could not be checked (malformed or unexpected input)."""


@dataclass(frozen=True)
class Instruction:
    """One instruction in execution order: its opcode and immediates."""

    op: str
    args: tuple[str, ...]


@dataclass(frozen=True)
class Violation:
    """A call site whose first stack-relevant successor is not the restore."""

    function: str
    call_index: int
    offender: str


@dataclass(frozen=True)
class Report:
    """The result of checking one module."""

    import_function: str
    stack_pointer: str
    call_sites: int
    functions: tuple[str, ...]
    violations: tuple[Violation, ...]


# ---------------------------------------------------------------------------
# Parsing
# ---------------------------------------------------------------------------


type SExpr = list[SExpr | str]
"""A parsed WAT form: an operator atom followed by atoms and nested forms."""


def _tokens(text: str) -> list[str]:
    return [
        token
        for token in _TOKEN.findall(text)
        if not token.startswith(";;") and not token.startswith("(;")
    ]


def _parse(text: str) -> SExpr:
    """Parse one module field's text into nested lists of atoms.

    The text holds the field's form, optionally followed by the `)` that closes the
    module (the last field runs to the end of the disassembly); anything else after the
    form is an unexpected layout.
    """
    tokens = _tokens(text)
    if not tokens or tokens[0] != "(":
        raise GateError(f"expected a module field, found {text[:80]!r}")
    stack: list[SExpr] = []
    for position, token in enumerate(tokens):
        if token == "(":
            stack.append([])
        elif token == ")":
            if not stack:
                raise GateError("unbalanced ')' in the disassembly")
            done = stack.pop()
            if not stack:
                if any(rest != ")" for rest in tokens[position + 1 :]):
                    raise GateError(f"more than one form in the module field {text[:80]!r}")
                return done
            stack[-1].append(done)
        else:
            if not stack:
                raise GateError(f"an atom outside any form in {text[:80]!r}")
            stack[-1].append(token)
    raise GateError("unbalanced '(' in the disassembly")


def _arm_children(arm: SExpr) -> list[SExpr]:
    """The instructions of an `if` arm, which holds forms only."""
    children: list[SExpr] = []
    for item in arm[1:]:
        if not isinstance(item, list):
            raise GateError(f"an atom inside an if arm: {item!r:.80}")
        children.append(item)
    return children


def _flatten(node: SExpr, out: list[Instruction]) -> None:
    """Append `node`'s instructions to `out` in execution order."""
    if not node or not isinstance(node[0], str):
        raise GateError(f"expected an instruction, found {node!r:.80}")
    op = node[0]
    atoms = tuple(item for item in node[1:] if isinstance(item, str))
    lists = [item for item in node[1:] if isinstance(item, list)]
    if op in {"block", "loop", "try_table"}:
        out.append(Instruction(op, atoms))
        for child in lists:
            if child and child[0] in _BLOCK_HEADER:
                continue
            if op == "try_table" and child and child[0] in {"catch", "catch_ref", "catch_all", "catch_all_ref"}:
                continue
            _flatten(child, out)
        out.append(Instruction("end", ()))
    elif op == "if":
        arms: dict[str, SExpr | None] = {"then": None, "else": None}
        for child in lists:
            if child and child[0] in _BLOCK_HEADER:
                continue
            if child and child[0] in arms:
                arms[child[0]] = child
            else:
                _flatten(child, out)  # the condition
        out.append(Instruction("if", atoms))
        then_arm = arms["then"]
        for child in _arm_children(then_arm) if then_arm is not None else []:
            _flatten(child, out)
        else_arm = arms["else"]
        if else_arm is not None:
            out.append(Instruction("else", ()))
            for child in _arm_children(else_arm):
                _flatten(child, out)
        out.append(Instruction("end", ()))
    elif op == "try":
        out.append(Instruction(op, atoms))
        for child in lists:
            if child and child[0] in _BLOCK_HEADER:
                continue
            if child and child[0] in {"do", "catch", "catch_all", "delegate"}:
                if child[0] != "do":
                    out.append(Instruction(child[0], tuple(a for a in child[1:] if isinstance(a, str))))
                for grandchild in child[1:]:
                    if isinstance(grandchild, list):
                        _flatten(grandchild, out)
            else:
                _flatten(child, out)
        out.append(Instruction("end", ()))
    else:
        for child in lists:
            _flatten(child, out)
        out.append(Instruction(op, atoms))


def _function_body(form: SExpr) -> tuple[str, list[Instruction]]:
    """A parsed `(func …)` form's name and instructions in execution order."""
    if not form or form[0] != "func":
        raise GateError("expected a function")
    name = form[1] if len(form) > 1 and isinstance(form[1], str) else "<unnamed>"
    body: list[Instruction] = []
    for item in form[1:]:
        if isinstance(item, str):
            continue
        if item and item[0] in _FUNC_HEADER:
            continue
        _flatten(item, body)
    return name, body


# ---------------------------------------------------------------------------
# The check
# ---------------------------------------------------------------------------


def _fields(text: str) -> list[tuple[int, int]]:
    """The `[start, end)` offsets of every module field."""
    starts = [match.start() for match in _FIELD_START.finditer(text)]
    if not starts:
        raise GateError("no module fields found (expected binaryen wasm-dis layout)")
    ends = starts[1:] + [len(text)]
    return list(zip(starts, ends))


def _name_pattern(name: str) -> re.Pattern[str]:
    return re.compile(rf"(?<![{_IDCHARS}])" + re.escape(name) + rf"(?![{_IDCHARS}])")


def _find_import(text: str) -> str:
    pattern = re.compile(
        r'^ \(import\s+"' + re.escape(IMPORT_MODULE) + r'"\s+"' + re.escape(IMPORT_NAME)
        + rf'"\s+\(func\s+(\$[{_IDCHARS}]+)',
        re.MULTILINE,
    )
    found = pattern.findall(text)
    if len(found) != 1:
        raise GateError(
            f"expected exactly one import of {IMPORT_MODULE!r} {IMPORT_NAME!r}, found {len(found)}"
        )
    return found[0]


def _field_named(text: str, fields: list[tuple[int, int]], kind: str, name: str) -> SExpr:
    header = re.compile(rf" \({kind}\s+{re.escape(name)}(?![{_IDCHARS}])")
    for start, end in fields:
        if header.match(text, start):
            return _parse(text[start:end])
    raise GateError(f"no {kind} named {name}")


def _find_stack_pointer(text: str, fields: list[tuple[int, int]]) -> str:
    pattern = re.compile(
        r'^ \(export\s+"' + re.escape(STACK_POINTER_EXPORT) + rf'"\s+\(func\s+(\$?[{_IDCHARS}]+)\)',
        re.MULTILINE,
    )
    found = pattern.findall(text)
    if len(found) != 1:
        raise GateError(f"expected exactly one export named {STACK_POINTER_EXPORT!r}, found {len(found)}")
    _, body = _function_body(_field_named(text, fields, "func", found[0]))
    globals_set = {ins.args[0] for ins in body if ins.op == "global.set" and ins.args}
    if len(globals_set) != 1:
        raise GateError(
            f"{STACK_POINTER_EXPORT} ({found[0]}) sets {len(globals_set)} globals; expected exactly one"
        )
    return globals_set.pop()


def _first_stack_relevant(body: list[Instruction], after: int, sp: str) -> str | None:
    """None when the restore comes first after `body[after]`; else the offender."""
    for ins in body[after + 1 :]:
        if ins.op == "global.set" and ins.args and ins.args[0] == sp:
            return None
        if ins.op == "global.get" and ins.args and ins.args[0] == sp:
            return f"global.get {sp}"
        if ins.op in _CALLS:
            return " ".join((ins.op, *ins.args))
        if ins.op in _TRANSFERS or ins.op.startswith("br_on_"):
            return " ".join((ins.op, *ins.args))
    return "the end of the function"


def check_text(text: str) -> Report:
    """Check a module's `wasm-dis` text."""
    fields = _fields(text)
    imported = _find_import(text)
    sp = _find_stack_pointer(text, fields)
    name = _name_pattern(imported)
    call = re.compile(r"\b(?:return_)?call\s+" + re.escape(imported) + rf"(?![{_IDCHARS}])")

    references = len(name.findall(text))
    textual_calls = len(call.findall(text))
    if references != textual_calls + 1:
        raise GateError(
            f"{imported} is referenced {references - 1 - textual_calls} time(s) other than by a "
            "direct call; an indirect call could not be checked"
        )

    starts = [start for start, _ in fields]
    callers = sorted({bisect.bisect_right(starts, match.start()) - 1 for match in call.finditer(text)})
    violations: list[Violation] = []
    functions: list[str] = []
    parsed_calls = 0
    for field in callers:
        if field < 0:
            raise GateError(f"a call of {imported} lies outside every module field")
        start, end = fields[field]
        chunk = text[start:end]
        if not chunk.startswith(" (func"):
            raise GateError(f"a call of {imported} lies in a module field that is not a function")
        function, body = _function_body(_parse(chunk))
        functions.append(function)
        for index, ins in enumerate(body):
            if ins.op not in {"call", "return_call"} or not ins.args or ins.args[0] != imported:
                continue
            parsed_calls += 1
            if ins.op == "return_call":
                violations.append(Violation(function, index, "a tail call leaves no frame to restore"))
                continue
            offender = _first_stack_relevant(body, index, sp)
            if offender is not None:
                violations.append(Violation(function, index, offender))
    if parsed_calls != textual_calls:
        raise GateError(
            f"found {textual_calls} textual call(s) of {imported} but parsed {parsed_calls}; "
            "the disassembly layout is not the one this gate reads"
        )
    if parsed_calls == 0:
        raise GateError(
            f"no function calls {IMPORT_NAME} ({imported}); the asynchronous lane was compiled out"
        )
    return Report(imported, sp, parsed_calls, tuple(functions), tuple(violations))


def disassemble(path: Path) -> str:
    """The module's text: read as is for `.wat`, disassembled by `wasm-dis` otherwise."""
    if path.suffix == ".wat":
        return path.read_text(encoding="utf-8")
    try:
        result = subprocess.run(
            ["wasm-dis", str(path)], check=True, capture_output=True, text=True
        )
    except FileNotFoundError as error:
        raise GateError("wasm-dis (binaryen) not found; it is a required wasm build dependency") from error
    except subprocess.CalledProcessError as error:
        raise GateError(f"wasm-dis failed on {path}: {error.stderr.strip()}") from error
    return result.stdout


def self_test() -> int:
    """Run the gate's fixture suite (`test_check_wasm_jspi_frame.py`): a good frame and an
    inlined good frame pass, and every way of reaching a call or the stale pointer before
    the restore — or having no caller at all — fails."""
    import unittest

    suite = unittest.defaultTestLoader.discover(
        str(Path(__file__).resolve().parent), pattern="test_check_wasm_jspi_frame.py"
    )
    result = unittest.TextTestRunner(verbosity=1).run(suite)
    if not result.wasSuccessful() or result.testsRun == 0:
        print("FAIL: the JSPI frame gate's self-test failed", file=sys.stderr)
        return 1
    print(f"OK: JSPI frame gate self-test ({result.testsRun} fixtures)")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(f"usage: {argv[0]} <module.wasm | module.wat> | --self-test", file=sys.stderr)
        return 2
    if argv[1] == "--self-test":
        return self_test()
    path = Path(argv[1])
    try:
        report = check_text(disassemble(path))
    except GateError as error:
        print(f"FAIL: JSPI frame gate on {path}: {error}", file=sys.stderr)
        return 1
    if report.violations:
        print(
            f"FAIL: JSPI frame gate on {path}: {len(report.violations)} call site(s) of "
            f"{IMPORT_NAME} ({report.import_function}) do not restore the stack pointer "
            f"({report.stack_pointer}) first:",
            file=sys.stderr,
        )
        for violation in report.violations:
            print(
                f"  function {violation.function}, instruction {violation.call_index}: "
                f"{violation.offender} comes before the restore",
                file=sys.stderr,
            )
        return 1
    print(
        f"OK: JSPI frame gate: {report.call_sites} call site(s) of {IMPORT_NAME} "
        f"({report.import_function}) in {len(report.functions)} function(s) "
        f"({', '.join(report.functions)}), each restoring the stack pointer "
        f"({report.stack_pointer}) before any call"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
