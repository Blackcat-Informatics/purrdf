#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Refuse a streamed read whose chunk size is written out instead of named.

Six copies of one number lived under two names across five files, and two had
already drifted -- one to a quarter of the shared size, one to a sixty-fourth. That
is the reason this is a gate rather than only a fix: every chunk size produces a
correct digest, so no run fails and no test reddens, and the only symptom is that
one site issues sixty-four times as many reads as its siblings. The drift had
nothing to report it.

Folding them onto ``scripts/lane_chunk.py`` closes today's copies and does nothing
about the fifth, so this refuses the shape: a literal byte count passed to a
``read()`` in any ``scripts/`` program. What must appear instead is the name
``STREAM_CHUNK_BYTES`` (Python) or ``LANE_STREAM_CHUNK_BYTES`` (the value shell
lanes pass into their embedded blocks).

Deliberately NOT refused, because a gate that over-refuses gets disabled and then
guards nothing:

* ``read()`` with no argument, or with a variable — those already name their size;
* a literal outside a ``read`` call, such as a buffer threshold or a row count —
  ``crates/bench/src/main.rs``'s flush interval is a write-side figure with its
  own name and its own reason, and is not this number;
* small literals inside a ``read`` (``read(1)``, ``read(2)``), which are reading a
  fixed-width field rather than streaming a file. The cutoff is stated rather than
  implied: a chunk size is at least a kibibyte, and nothing reads a 1024-byte
  fixed-width field here.
"""

import argparse
import ast
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPTS = REPO_ROOT / "scripts"

# `read(1 << 22)`, `read(4194304)`, `.read(1<<20)` — a literal count, decimal or
# shifted, handed to a read. `\b` on `read` keeps `handle.read` and bare `read`
# while excluding `spread(`.
# A literal byte count handed to a read, in any spelling this repository could use: a
# shift (`1 << 22`), a plain or underscored decimal (`4194304`, `4_194_304`), hex
# (`0x400000`), or a product (`4 * 1024 * 1024`). The first version matched only a shift
# or four-plus bare digits, so three legal spellings of the same number were accepted --
# and `read(1000)` was refused despite the docstring stating a kibibyte cutoff, because
# the decimal arm counted DIGITS where the rule is about VALUE.
# A `read(...)` call whose argument is a literal expression. The argument text is then
# EVALUATED rather than pattern-matched, because the first version alternated over
# spellings and three legal ones for the very number it exists to catch -- `0o20000000`,
# `0b1000...`, `4 * 1024**2` -- walked straight through. A spelling list is a denylist, and
# a denylist over syntax is the wrong shape.
# A LINE SCAN IS THE FALLBACK, NOT THE METHOD. `[^)\n]+?` truncated at the first `)`, so
# every call that matters was invisible -- and invisible as a SILENT SKIP, because the
# truncated text fails to parse and an unparseable fragment reads the same as a name:
#
#   handle.read((4194304))          -> group `(4194304`     skipped
#   handle.read(int(4194304))       -> group `int(4194304`  skipped
#   os.read(fd, 4194304)            -> group `fd, 4194304`  skipped
#   handle.read(\n    4 * 1024 * 1024\n)                    not matched at all
#   scripts/lane-common.sh:310, `handle.read(int(sys.argv[2]))` -- this repository's own
#   non-trivial read site, skipped
#
# Python files are therefore parsed and walked. Shell files keep the line scan, because
# their embedded Python lives inside a single-quoted shell string and is not a module --
# stated rather than left as a silent difference in coverage.
READ_CALL = re.compile(r"\bread\(\s*([^)\n]+?)\s*\)")

# A shift or exponent large enough to hang the fold. `read(2**10**10)` made `make check`
# compute a number with ten billion digits; the gate's own denial-of-service.
_MAX_EXPONENT = 64

# The one definition, and the shell variable carrying it into an embedded block.
DEFINING_FILE = "lane_chunk.py"

# This gate's own fixtures ARE the shape it refuses — that is what makes the
# self-test non-vacuous — so it does not scan itself. Stated rather than left as a
# quiet exclusion: the alternative is a gate that cannot describe what it refuses.
GATE_FILE = Path(__file__).name


def _literal_value(literal: str) -> tuple[int | None, bool]:
    """`(value, unparseable)` for a read's argument.

    `(None, False)` means "not a literal at all" -- a variable or a name, which already
    names its size and is exactly what this gate wants. `(None, True)` means "a literal
    this gate cannot evaluate", which is reported rather than skipped.

    Evaluated with `ast` rather than matched: the previous alternation missed `0o20000000`,
    `0b1000...` and `4 * 1024**2`, three legal spellings of the number it exists to catch.
    """
    try:
        tree = ast.parse(literal, mode="eval")
    except SyntaxError:
        # `0123` is a syntax error in Python 3 and a plausible typo for a chunk size, so it
        # is a literal that cannot be evaluated rather than a name.
        return None, bool(re.fullmatch(r"[0-9][0-9_]*", literal.strip()))

    too_large = False

    def fold(node: ast.expr) -> int | None:
        nonlocal too_large
        if isinstance(node, ast.Constant) and isinstance(node.value, int):
            return node.value
        # `int(4194304)` is a chunk size written out with a no-op conversion around it.
        if (
            isinstance(node, ast.Call)
            and getattr(node.func, "id", "") in {"int", "round"}
            and len(node.args) == 1
        ):
            return fold(node.args[0])
        if isinstance(node, ast.BinOp):
            left, right = fold(node.left), fold(node.right)
            if left is None or right is None:
                return None
            if isinstance(node.op, ast.Mult):
                return left * right
            if isinstance(node.op, ast.Add):
                return left + right
            if isinstance(node.op, ast.Sub):
                return left - right
            if isinstance(node.op, ast.FloorDiv) and right != 0:
                return left // right
            # BOUNDED, because an unbounded fold is the gate hanging itself: `2**10**10`
            # asked `make check` for a number with ten billion digits.
            # BOUNDED, AND REPORTED RATHER THAN SKIPPED. An unbounded fold is the gate
            # hanging itself -- `2**10**10` asked `make check` for a number with ten
            # billion digits -- and returning None for it would be a silent pass for a
            # shape that is unmistakably a written-out size.
            if isinstance(node.op, (ast.LShift, ast.Pow)) and right > _MAX_EXPONENT:
                too_large = True
                return None
            if isinstance(node.op, ast.LShift):
                return left << right
            if isinstance(node.op, ast.Pow):
                return left**right
        return None

    value = fold(tree.body)
    return value, too_large


def python_offences(path: Path, text: str) -> list[str]:
    """Every literal-sized read in a Python file, found by parsing rather than matching."""
    try:
        tree = ast.parse(text)
    except SyntaxError as error:
        return [f"{path.name}: cannot be parsed, so its reads cannot be judged ({error})"]
    found: list[str] = []
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        name = node.func.attr if isinstance(node.func, ast.Attribute) else getattr(node.func, "id", "")
        if name != "read":
            continue
        # `os.read(fd, n)` puts the count second; `handle.read(n)` first.
        arguments = node.args[1:] if name == "read" and len(node.args) == 2 else node.args
        for argument in arguments:
            value, unparseable = _literal_value(ast.unparse(argument))
            if unparseable:
                found.append(
                    f"{path.name}:{node.lineno}: `read({ast.unparse(argument)})` is a literal "
                    f"this gate cannot evaluate, so it cannot be judged. Name it instead."
                )
            elif value is not None and value >= 1024:
                found.append(
                    f"{path.name}:{node.lineno}: `read({ast.unparse(argument)})` writes the "
                    f"chunk size out. Name it: `STREAM_CHUNK_BYTES` from "
                    f"scripts/{DEFINING_FILE}, or `${{LANE_STREAM_CHUNK_BYTES}}` passed "
                    f"in by lane-common.sh."
                )
    return found


# EVERY WAY A SHELL SCRIPT LAUNCHES AN EMBEDDED PROGRAM, not the one form that came to
# mind. The first version matched only `python3 -c '` followed immediately by a newline,
# which opened 3 of the 12 `-c` sites and NONE of the five heredoc sites
# (`python3 - "$arg" <<'PY'`) in three files. A survey of the tracked shell scripts is
# what settled that; a narrower pattern had reported the heredocs absent.
#
# Two launchers, two delimiters:
#   python3 -c '<body>'          -- the body is the single-quoted argument
#   python3 - [args] <<'DELIM'   -- the body runs to a line that is exactly DELIM
# THE CLAIM "A SHELL SINGLE-QUOTED STRING CANNOT CONTAIN A SINGLE QUOTE" IS TRUE OF THE
# STRING AND FALSE OF THE WORD. `'\''` is exactly how shell puts a quote inside one, and
# `[^']*` truncated at it -- which produced BOTH failure directions from one mis-extraction:
# the truncated fragment fails to parse and is reported as an offence, so a legitimate script
# is refused with a nonsense diagnostic, while the genuine nested read later in the same body
# is never seen at all.
#
# This is the fourth pattern here. The first three were progressively wider guesses at the
# syntax (`-c '` plus a newline; `^'` for the closing quote; `[^']*`); this one encodes the
# escape rule, which is the thing that was actually being got wrong.
_QUOTE_ESCAPE = "'\\''"
_QUOTED_BODY = re.compile(r"python3 -c '((?:[^']|'\\'')*)'")
# `python3 -c "..."` is a third launcher spelling. No tracked script uses it, and the comment
# that used to say "two launchers, two delimiters" was a claim about this repository dressed
# as a claim about shell.
_DQUOTED_BODY = re.compile(r'python3 -c "((?:[^"\\]|\\.)*)"')

_HEREDOC_OPEN = re.compile(r"python3 -[^\n]*<<(-?)'?(\w+)'?[^\n]*$", re.MULTILINE)


def embedded_programs(text: str) -> list[tuple[int, str, str]]:
    """Every embedded Python program as `(line_offset, raw, body)`.

    `line_offset` is the 0-based line the body starts on in the shell file, so a finding can
    name the shell line rather than a line number inside a fragment nobody can locate.
    `raw` is the text exactly as it appears (used for masking); `body` is the program the
    shell would actually hand to Python, with that launcher's escapes undone.

    THE UNESCAPING IS PER LAUNCHER, because the launchers do not share an escape rule.
    Applying the single-quoted rule to a double-quoted body corrupted it, and applying
    neither to a double-quoted body left `\"` in the source -- so the ordinary spelling
    `python3 -c "print(\"hi\"); …"` was reported as a parse failure. That is both failure
    directions from one mis-translation: a legitimate script refused with a nonsense
    diagnostic, and a body that DOES write the chunk size out reported as unparseable rather
    than as the offence, losing the literal and the line number.
    """
    programs: list[tuple[int, str, str]] = []

    def _commented(at: int) -> bool:
        # A SHELL COMMENT MENTIONING THE IDIOM IS NOT AN INVOCATION. Only the line the match
        # starts on matters: a `#` inside a body is Python, judged by the walker.
        line_start = text.rfind("\n", 0, at) + 1
        return text[line_start:at].lstrip().startswith("#")

    for match in _QUOTED_BODY.finditer(text):
        if _commented(match.start()):
            continue
        raw = match.group(1)
        programs.append((text[: match.start(1)].count("\n"), raw, raw.replace(_QUOTE_ESCAPE, "'")))

    for match in _DQUOTED_BODY.finditer(text):
        if _commented(match.start()):
            continue
        raw = match.group(1)
        # INSIDE DOUBLE QUOTES ONLY FIVE ESCAPES EXIST. The shell removes a backslash
        # before `$`, a backtick, `"`, a backslash and a newline, and KEEPS it before
        # anything else -- verified against bash: `a\_b` keeps it, `a\$b` and `a\"b` drop
        # it. Unescaping everything corrupted `msg = '\''` into three quotes, an
        # unterminated triple quote, and refused a legitimate body. Caught by the fixture
        # written for the opposite defect, which is why both directions are fixtures.
        body = re.sub(r"\\\n", "", raw)
        body = re.sub(r"\\([$`\"\\])", r"\1", body)
        programs.append((text[: match.start(1)].count("\n"), raw, body))

    # A HEREDOC OPENER INSIDE AN ALREADY-EXTRACTED BODY IS PYTHON, NOT SHELL. A `-c` body
    # that merely MENTIONS one -- `print("run: python3 - <<EOF")` -- matched the opener
    # pattern, and the phantom body ran to the end of the file because its delimiter never
    # appeared. The ordinary shell that followed is not Python, so the whole script was
    # refused as unparseable: a false refusal triggered by a program describing the idiom.
    consumed = [
        (text.index(raw), text.index(raw) + len(raw)) for _offset, raw, _body in programs
    ]

    def _inside_a_body(at: int) -> bool:
        return any(start <= at < end for start, end in consumed)

    lines = text.splitlines()
    for match in _HEREDOC_OPEN.finditer(text):
        if _commented(match.start()) or _inside_a_body(match.start()):
            continue
        strips_tabs = match.group(1) == "-"
        delimiter = match.group(2)
        opened = text[: match.start()].count("\n")
        body: list[str] = []
        for line in lines[opened + 1 :]:
            # `<<-` STRIPS LEADING TABS, from the body AND from the delimiter line; plain
            # `<<` requires the delimiter to be the whole line. Accepting `<<-` while
            # keeping its tabs made the body `unexpected indent` -- a spelling the pattern
            # went out of its way to admit and could then not judge. And comparing with
            # `.strip()` on a plain heredoc terminated on an indented look-alike inside a
            # triple-quoted string.
            candidate = line.lstrip("\t") if strips_tabs else line
            if candidate == delimiter if strips_tabs else line == delimiter:
                break
            body.append(candidate)
        if body:
            raw = "\n".join(lines[opened + 1 : opened + 1 + len(body)])
            programs.append((opened + 1, raw, "\n".join(body)))
    return programs


def shell_offences(path: Path, text: str) -> list[str]:
    """Every literal-sized read in a shell script, including its embedded Python.

    THE WEAKER PATH WAS THE ONE THAT MATTERED. `.py` files are parsed and walked; `.sh`
    files kept a line scan whose regex truncates at the first `)`, so a truncated fragment
    failed to parse and read as a name -- a silent pass. And the lanes' embedded Python is
    exactly where the non-trivial reads live: `scripts/lane-common.sh`'s own
    `handle.read(int(sys.argv[2]))` was invisible to the gate, which this file's own comment
    block cites as a reason for the AST rewrite it then did not apply here.
    """
    found: list[str] = []
    for line_offset, _raw, body in embedded_programs(text):
        for problem in python_offences(path, body):
            found.append(_rebase_line(problem, line_offset))
    # The shell's own reads. Embedded bodies are blanked LINE-FOR-LINE rather than deleted:
    # `sub("")` removed their lines outright and shifted every later line number, so a
    # shell-native finding was reported up to 119 lines early in `lane-common.sh`.
    masked = text
    # MASKED ON THE RAW TEXT, not the translated body: once an escape is undone the body no
    # longer appears in the file, so `replace` found nothing and the same read was reported
    # twice on one line.
    for _line_offset, raw, _body in embedded_programs(text):
        masked = masked.replace(raw, "\n" * raw.count("\n"), 1)
    # A WHOLE-LINE SHELL COMMENT IS DOCUMENTATION, not a read. Without this, a comment
    # mentioning the idiom this gate enforces -- `# see python3 -c 'h.read(4194304)'` -- was
    # itself an offence: an over-refusal against prose describing the rule. Blanked rather
    # than dropped, so every later line number survives, and only in the SHELL path, where a
    # line-initial `#` is unambiguous. A `#` inside an extracted body is Python and is judged
    # by the walker, which is why this runs after the bodies are removed.
    masked = "\n".join(
        "" if line.lstrip().startswith("#") else line for line in masked.split("\n")
    )
    found.extend(offences(path, masked))
    return found


def _rebase_line(problem: str, offset: int) -> str:
    """Shift a `name:LINE:` prefix by *offset*, so an embedded finding names the shell line."""
    parts = problem.split(":", 2)
    if len(parts) == 3 and parts[1].isdigit():
        return f"{parts[0]}:{int(parts[1]) + offset}:{parts[2]}"
    return problem


def offences(path: Path, text: str) -> list[str]:
    """Every literal-sized streamed read in one file, as a diagnosis per hit."""
    found: list[str] = []
    for number, line in enumerate(text.splitlines(), start=1):
        for match in READ_CALL.finditer(line):
            literal = match.group(1)
            # JUDGED ON VALUE, because the stated rule is a value: a chunk size is at least
            # a kibibyte and nothing here reads a 1024-byte fixed-width field.
            value, unparseable = _literal_value(literal)
            if unparseable:
                # A LITERAL THAT CANNOT BE EVALUATED IS REPORTED, not skipped. `int("0123",
                # 0)` raises on a leading zero, and swallowing that to `None` made
                # `read(0123)` a silent pass -- a gate declining to judge the one shape it
                # was looking at.
                found.append(
                    f"{path.name}:{number}: `read({literal})` is a literal this gate cannot "
                    f"evaluate, so it cannot be judged. Name it instead: "
                    f"`STREAM_CHUNK_BYTES` from scripts/{DEFINING_FILE}."
                )
                continue
            if value is None or value < 1024:
                continue
            found.append(
                f"{path.name}:{number}: `read({literal})` writes the chunk size out. "
                f"Name it: `STREAM_CHUNK_BYTES` from scripts/{DEFINING_FILE}, or "
                f"`${{LANE_STREAM_CHUNK_BYTES}}` passed in by lane-common.sh."
            )
    return found


def scan() -> list[str]:
    """Every offence across the scripts directory, skipping the defining file."""
    found: list[str] = []
    # RECURSIVE. `iterdir()` left any future `scripts/<subdir>/*.py` unscanned, which is
    # the "a surface the gate never inspects" shape this file is one instance of.
    for path in sorted(SCRIPTS.rglob("*")):
        # BY RELATIVE PATH, not by bare name: `scripts/vendor/lane_chunk.py` carrying a
        # drifted constant would have been fully exempt while its neighbour was flagged.
        if not path.is_file() or path.relative_to(SCRIPTS) in {Path(DEFINING_FILE), Path(GATE_FILE)}:
            continue
        if path.suffix not in {".py", ".sh"}:
            continue
        text = path.read_text(encoding="utf-8")
        found.extend(
            python_offences(path, text) if path.suffix == ".py" else shell_offences(path, text)
        )
    return found


def self_test() -> int:
    """The shapes that must be refused, and — the half that matters — those that must not."""
    ok = True
    here = Path("probe.py")

    refused = [
        "        for chunk in iter(lambda: handle.read(1 << 22), b''):",
        "    chunk = sys.stdin.buffer.read(1 << 20)",
        "    data = handle.read(4194304)",
        "    data = handle.read(65536)",
        # Three legal spellings of the same number that the digit-counting rule accepted.
        "    data = handle.read(4_194_304)",
        "    data = handle.read(0x400000)",
        "    data = handle.read(4 * 1024 * 1024)",
        # Three more legal spellings of 4194304 that the alternation accepted outright.
        "    data = handle.read(0o20000000)",
        "    data = handle.read(0b10000000000000000000000)",
        "    data = handle.read(4 * 1024**2)",
        # A literal that cannot be evaluated is reported rather than skipped.
        "    data = handle.read(0123)",
        # Every shape the regex finder could not see. Each is a chunk size written out.
        "    data = handle.read((4194304))",
        "    data = handle.read(int(4194304))",
        "    data = os.read(fd, 4194304)",
        "    data = handle.read(8388608 // 2)",
        # A fold this gate refuses to compute is reported, not skipped: it asked for a
        # number with ten billion digits and hung `make check`.
        "    data = handle.read(2**10**10)",
    ]
    for line in refused:
        # THROUGH THE PATH A PYTHON FILE ACTUALLY TAKES. These fixtures used to be judged
        # by the line scanner while the gate walks the AST for `.py` -- a self-test
        # proving a code path the gate does not use on the files it mostly reads.
        if not python_offences(here, line.strip()):
            print(f"SELF-TEST FAIL: not refused: {line.strip()}")
            ok = False
    if ok:
        print(f"OK: self-test — all {len(refused)} written-out chunk sizes are refused")

    # THE VALID NEIGHBOURS, split by the path that actually judges them. A `for` clause
    # and a Rust line are not standalone Python, so they belong to the line scanner that
    # reads `.sh` files; the rest go through the AST walker that reads `.py`. Running them
    # all through one path is how the first version of this split reported a correct
    # fixture as unparseable.
    accepted_python = [
        "chunk = iter(lambda: handle.read(STREAM_CHUNK_BYTES), b'')",
        "chunk = sys.stdin.buffer.read(chunk_bytes)",
        "text = handle.read()",
        "_U64 = (1 << 64) - 1",
        "marker = handle.read(1)",
        "width = handle.read(2)",
        "DIGEST = 1 << 22  # a bare definition is not a read",
        # Below the stated kibibyte cutoff, so a fixed-width field read, not a stream.
        "header = handle.read(1000)",
        "rest = handle.read(-1)",
    ]
    accepted_other = [
        # Not Python at all: a write-side figure in Rust with its own name and reason.
        "const FLUSH_EVERY_BYTES: usize = 1 << 20;",
        "    chunk = sys.stdin.buffer.read(chunk_bytes)",
    ]
    wrongly = [line for line in accepted_python if python_offences(here, line)]
    wrongly += [line for line in accepted_other if offences(here, line)]
    if wrongly:
        print(f"SELF-TEST FAIL: these must be accepted and were refused: {wrongly}")
        ok = False
    else:
        total = len(accepted_python) + len(accepted_other)
        print(f"OK: self-test — all {total} named or non-streaming reads are accepted")

    # SHELL FIXTURES, DRIVEN THROUGH `shell_offences`. Not one existed: every fixture above
    # is a Python line, so the entire `.sh` path -- the subject of the change that added it,
    # and where the lanes' embedded programs live -- could be stubbed to `return []` with
    # this self-test green and the bare run at exit 0. The CI step is named "Check the
    # stream-chunk gate can still fail", which it could not.
    shell_refused = {
        "a literal read in a `-c` body": "python3 -c '\nimport sys\nh.read(4194304)\n'\n",
        # A NESTED CALL, so this fixture can only pass through the AST walker. A plain
        # `read(4194304)` here proved nothing: the line scanner sees it whether or not the
        # heredoc was ever extracted, so the fixture passed with the extractor disabled --
        # a control that cannot distinguish the case it exists for. Caught by mutating the
        # heredoc pattern to never match and watching the test stay green.
        "a literal read the line scanner cannot see, in a heredoc body": (
            "python3 - \"$1\" <<'PY'\nimport sys\nh.read(int(4194304))\nPY\n"
        ),
        "a shape the line scanner could not see": "python3 -c '\nh.read(int(4194304))\n'\n",
        # SINGLE-LINE BODIES, which every other fixture here is not. Reverting the body
        # pattern to the previous `^'` form -- which silently required a MULTI-line body --
        # left this self-test fully green, so the repair could not be observed to fail. Each
        # of these is NESTED on purpose: a plain `read(4194304)` passes through the line
        # scanner whether or not extraction happened, so it would not distinguish.
        "a single-line `-c` body, nested": "python3 -c 'h.read(int(4194304))'\n",
        "a single-line `-c` body, two-arg": "python3 -c 'os.read(fd, 4194304)'\n",
        "a single-line `-c` body, parenthesised": "python3 -c 'h.read((4194304))'\n",
        # A double-quoted body is a third launcher spelling.
        "a double-quoted `-c` body, nested": 'python3 -c "h.read(int(4194304))"\n',
        # THE POSIX QUOTE ESCAPE continues the body rather than closing it. Truncating at it
        # produced a false refusal (the fragment does not parse) AND missed the real read.
        "a nested read after the POSIX quote escape": (
            "python3 -c 'msg = '\\''go'\\''; h.read(int(4194304))'\n"
        ),
        # A DOUBLE-QUOTED BODY WITH ESCAPED QUOTES is the ordinary spelling of that
        # launcher, not a corner. Handing it to the parser with the backslashes still in it
        # refused a legitimate script AND lost the literal -- the same both-directions
        # mis-translation as the single-quoted case, committed by the repair for it.
        "a double-quoted body with escaped quotes": (
            'python3 -c "print(\\"hi\\"); h.read(int(4194304))"\n'
        ),
        # `<<-` strips leading tabs. Accepting the spelling and keeping the tabs made every
        # such body `unexpected indent`.
        # A PLAIN HEREDOC ENDS ONLY ON AN EXACT DELIMITER LINE. Comparing with `.strip()`
        # terminated the body at an indented look-alike inside a triple-quoted string, so
        # everything after it -- including this read -- was never parsed. Verified against
        # bash: a tab-indented `PY` does NOT close a plain `<<'PY'`.
        "a delimiter look-alike inside a triple-quoted string": (
            'python3 - <<\'PY\'\ndoc = """\n\tPY\n"""\nh.read(int(4194304))\nPY\n'
        ),
        "a tab-indented `<<-` heredoc body": (
            "python3 - <<-'PY'\n\th.read(int(4194304))\n\tPY\n"
        ),
        "a shell-native read": '    chunk = "$(dd bs=4194304)"\n    h.read(4194304)\n',
    }
    # A REFUSAL MUST NAME THE LITERAL, not merely be a refusal. Every one of these bodies
    # writes `4194304` out, and a mis-extraction reports "cannot be parsed" instead -- which
    # is still non-empty, so a fixture asserting only "something was reported" passes on the
    # very defect it exists to catch. Two fixtures fell into that trap before this.
    missed = [
        label
        for label, body in shell_refused.items()
        if not any("4194304" in problem for problem in shell_offences(here, body))
    ]
    if missed:
        print(
            f"SELF-TEST FAIL: these shell shapes were not refused, or were refused without "
            f"naming the literal: {missed}"
        )
        ok = False
    else:
        print(
            f"OK: self-test — all {len(shell_refused)} shell shapes are refused, each naming "
            "the literal rather than a parse failure"
        )

    shell_accepted = {
        "a named read in a `-c` body": "python3 -c '\nh.read(int(sys.argv[2]))\n'\n",
        "a named read in a heredoc body": (
            "python3 - \"$1\" <<'PY'\nh.read(chunk_bytes)\nPY\n"
        ),
        "a small fixed-width field": "python3 -c '\nh.read(2)\n'\n",
        "a single-line `-c` body, named": "python3 -c 'h.read(int(sys.argv[2]))'\n",
        "a double-quoted body with escaped quotes, named": (
            'python3 -c "print(\\"hi\\"); h.read(chunk_bytes)"\n'
        ),
        # Inside DOUBLE quotes those four characters are Python source, not a shell escape.
        # Applying the single-quoted rule to them corrupted the body into an unterminated
        # string.
        "a double-quoted body containing the quote-escape characters": (
            'python3 -c "msg = \'\\\'\'; h.read(chunk_bytes)"\n'
        ),
        "a tab-indented `<<-` heredoc body, named": (
            "python3 - <<-'PY'\n\th.read(chunk_bytes)\n\tPY\n"
        ),
        "the quote escape with a named read": (
            "python3 -c 'msg = '\\''go'\\''; h.read(chunk_bytes)'\n"
        ),
        # A COMMENT MENTIONING THE IDIOM IS DOCUMENTATION, not an invocation. Deleting both
        # halves of that fix left the self-test green, because it shipped with no fixture.
        "a comment mentioning the idiom": "# see python3 -c 'h.read(4194304)' for the idiom\n",
        "an indented comment mentioning it": "    # python3 -c 'h.read(4194304)'\n",
        "a comment mentioning a bare read": "# h.read(4194304) is the wrong spelling\n",
        # A PROGRAM THAT MENTIONS A HEREDOC OPENER IS STILL ONE PROGRAM. The opener pattern
        # matched inside an already-extracted body, and the phantom body ran to the end of
        # the file because its delimiter never appeared -- so the ordinary shell that
        # followed was parsed as Python and the whole script refused. A false refusal
        # triggered by a program describing the idiom this gate enforces.
        "a `-c` body mentioning a heredoc opener": (
            "python3 -c '\nprint(\"run: python3 - <<EOF\")\nh.read(chunk_bytes)\n'\n"
            'if [[ -n "${x}" ]]; then\n  echo hi\nfi\n'
        ),
        "shell with no read at all": 'echo "no reads here"\n',
    }
    wrongly = [label for label, body in shell_accepted.items() if shell_offences(here, body)]
    if wrongly:
        print(f"SELF-TEST FAIL: these shell shapes must be accepted: {wrongly}")
        ok = False
    else:
        print(f"OK: self-test — all {len(shell_accepted)} named or non-streaming shell reads are accepted")

    # AND THE LINE NUMBER MUST NAME THE SHELL LINE. Blanking an embedded body with `sub("")`
    # deleted its lines and shifted every later number, so a shell-native finding was
    # reported up to 119 lines early.
    # ONE READ IS ONE FINDING. The embedded bodies are blanked out before the shell scan,
    # and the blanking must use the RAW text: once an escape is translated the body no longer
    # appears in the file, `replace` finds nothing, and the same read is reported twice on one
    # line. Nothing observed that until this fixture.
    duplicated = shell_offences(here, "python3 -c 'msg = '\\''go'\\''; h.read(4194304)'\n")
    if len(duplicated) != 1:
        print(
            f"SELF-TEST FAIL: one read in an escaped body produced {len(duplicated)} findings, "
            f"not one -- the body was masked after translation rather than as written"
        )
        ok = False
    else:
        print("OK: self-test -- a read in an escaped body is reported exactly once")

    # THE LINE NUMBER OF AN EMBEDDED FINDING must name the shell line, and the number of a
    # SHELL-NATIVE finding AFTER an embedded body must survive the blanking. The first
    # version of this probe had nothing after the body, so it could not observe the defect
    # it was written for: reverting the line-for-line blanking to `replace(body, "", 1)` --
    # which deleted the lines and shifted everything below -- left this self-test fully
    # green while reporting real findings up to 119 lines early in `lane-common.sh`.
    probe = "\n" * 9 + "python3 -c '\nh.read(4194304)\n'\n" + "\nh.read(65536)\n"
    reported = sorted(shell_offences(here, probe))
    embedded = [problem for problem in reported if "4194304" in problem]
    native = [problem for problem in reported if "65536" in problem]
    if not embedded or ":11:" not in embedded[0]:
        print(f"SELF-TEST FAIL: the embedded finding does not name the shell line: {embedded}")
        ok = False
    elif not native or ":14:" not in native[0]:
        print(
            f"SELF-TEST FAIL: a shell-native finding AFTER an embedded body does not name its "
            f"own line — the embedded body was deleted rather than blanked, so every later "
            f"line number is short: {native}"
        )
        ok = False
    else:
        print(
            "OK: self-test — an embedded finding names its shell line, and a shell-native "
            "finding after one keeps its own"
        )

    # And the tree as it stands, which is the neighbour for the whole gate.
    standing = scan()
    if standing:
        print("SELF-TEST FAIL: the tree as committed is refused:")
        for problem in standing:
            print(f"  {problem}")
        ok = False
    else:
        print("OK: self-test — no script writes the chunk size out (the valid neighbour)")

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        return self_test()

    found = scan()
    if found:
        sys.exit(
            "FAIL: a streamed read writes its chunk size out instead of naming it:\n  "
            + "\n  ".join(found)
            + "\n  Six copies under two names across five files, two already drifted (one to a "
            "quarter\n  of the shared size, one to a sixty-fourth), is how this went wrong the first "
            "time —\n  and every chunk size produces a correct digest, so nothing reports it."
        )
    print(f"OK: every streamed read under {SCRIPTS.name}/ names its chunk size")
    return 0


if __name__ == "__main__":
    sys.exit(main())
