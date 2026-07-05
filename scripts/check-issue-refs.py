# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Reject new ``#NNN`` GitHub issue-reference tokens in Rust comments and docs.

PurRDF issue references in comments and in-tree markdown documentation are a
form of hidden TODO debt. Once an issue is closed the token becomes stale and
misleading, so we do not allow new ones. This lint scans:

* ``.rs`` files under ``crates/`` and ``bindings/`` — only Rust comments are
  examined. A small Rust-aware lexer skips string, character, and raw-string
  literals so ``//`` inside ``"http://example.org"`` is not treated as a
  comment.
* ``.md`` files under ``crates/``, ``bindings/``, ``docs/``, and root ``*.md``
  files — markdown header anchors (``#101-...``), hex color codes, inline code,
  and fenced code blocks are excluded.
* ``.toml`` files under ``crates/``, ``bindings/``, ``docs/``, and root
  ``*.toml`` files — manifest ``description`` fields and dependency comments are
  scanned line by line; hex colors are excluded by the token pattern itself.
* ``.py`` files under ``scripts/``, ``crates/``, and ``bindings/`` — only ``#``
  line comments are examined. A small Python-aware lexer skips string and
  docstring literals (including ``r``/``b``/``u``/``f`` prefixes and
  triple-quoted strings), exactly as the Rust scan skips string literals, so an
  issue-shaped token inside a string or docstring is never flagged.
* ``.yaml``/``.yml`` workflow files under ``.github/`` — only ``#`` comments are
  examined. A ``#`` is a comment only at line start or after whitespace and only
  when outside a quoted scalar (matching YAML's own comment rule), so a ``#``
  inside a quoted string is treated as data.

The issue token pattern is ``#`` followed by 1–5 decimal digits that is not
followed by another digit, a hex letter, a hyphen, or a decimal fraction
(so ``#3.1`` section numbers are not flagged). This avoids 6-digit hex colors
and markdown anchors while still catching references like ``#16`` or ``#123``.
"""

from __future__ import annotations

import re
import subprocess
from collections.abc import Iterator
from pathlib import Path

ISSUE_RE = re.compile(r"#\d{1,5}(?![\dA-Fa-f-])(?!\.\d)")

SCAN_DIRS = ("crates", "bindings", "docs")

# Directories whose ``.py`` files are first-party enough to lint. ``scripts/``
# is not in ``SCAN_DIRS`` (which governs ``.rs``/``.md``/``.toml``) but is the
# home of the maintenance scripts this lint most wants to cover.
PY_SCAN_DIRS = (*SCAN_DIRS, "scripts")

# Valid Python string-literal prefixes (case-insensitive) that may precede an
# opening quote. ``u`` never combines; ``r`` combines with ``b``/``f``.
PY_STRING_PREFIXES = frozenset({"r", "b", "u", "f", "rb", "br", "rf", "fr"})


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def iter_scan_paths(root: Path) -> Iterator[Path]:
    """Yield every tracked source file the lint enforces.

    Covered: ``.rs``/``.md``/``.toml`` under ``crates``/``bindings``/``docs``
    (plus root ``.md``/``.toml``), ``.py`` under those dirs and ``scripts``, and
    ``.yaml``/``.yml`` GitHub workflow files under ``.github``.

    Enumeration is driven by ``git ls-files`` rather than a filesystem walk so
    the scan covers exactly the committed first-party source. Untracked build
    artifacts and third-party trees (``bindings/python/.venv`` linkml docs,
    ``target/``) are never scanned, keeping the lint deterministic and free of
    "green in CI, red locally" divergence.
    """
    out = subprocess.run(
        ["git", "-C", str(root), "ls-files", "-z"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    for rel in sorted(part for part in out.split("\0") if part):
        suffix = Path(rel).suffix
        if suffix not in (".rs", ".md", ".toml", ".py", ".yaml", ".yml"):
            continue
        segments = rel.split("/")
        top = segments[0]
        if suffix in (".rs", ".md", ".toml"):
            in_scan_dir = top in SCAN_DIRS
            root_file = len(segments) == 1 and (
                rel.endswith(".md") or rel.endswith(".toml")
            )
            if not (in_scan_dir or root_file):
                continue
        elif suffix == ".py":
            if top not in PY_SCAN_DIRS:
                continue
        else:  # .yaml / .yml
            if top != ".github":
                continue
        path = root / rel
        if path.is_file():
            yield path


def pos_to_line_col(src: str, pos: int) -> tuple[int, int]:
    """Convert a 0-based source index to 1-based line/column."""
    line = src.count("\n", 0, pos) + 1
    last_nl = src.rfind("\n", 0, pos)
    col = pos - last_nl
    return line, col


def snippet(text: str, start: int, end: int, window: int = 24) -> str:
    """Return a short snippet of ``text`` surrounding ``text[start:end]``."""
    prefix = text[max(0, start - window) : start].replace("\n", " ")
    matched = text[start:end]
    suffix = text[end : min(len(text), end + window)].replace("\n", " ")
    return f"{prefix}{matched}{suffix}".strip()


def rust_comments(src: str) -> list[tuple[int, int, str]]:
    """Extract Rust comments as ``(start_line, start_col, comment_text)``.

    The scanner is deliberately conservative: it only needs to avoid treating
    ``//`` or ``/*`` inside string/char/raw-string literals as comment
    starters. It understands line comments, block comments (including nested
    ones), byte/regular strings, byte/regular character literals, lifetimes,
    and raw strings with arbitrary hash counts.
    """
    comments: list[tuple[int, int, str]] = []
    n = len(src)
    i = 0

    while i < n:
        c = src[i]

        # Line comment: //, ///, //!
        if c == "/" and i + 1 < n and src[i + 1] == "/":
            j = src.find("\n", i)
            if j == -1:
                j = n
            line, col = pos_to_line_col(src, i)
            comments.append((line, col, src[i:j]))
            i = j
            continue

        # Block comment: /*, /**, /*!
        if c == "/" and i + 1 < n and src[i + 1] == "*":
            j = i + 2
            depth = 1
            while j < n and depth > 0:
                if src[j] == "/" and j + 1 < n and src[j + 1] == "*":
                    depth += 1
                    j += 2
                elif src[j] == "*" and j + 1 < n and src[j + 1] == "/":
                    depth -= 1
                    j += 2
                else:
                    j += 1
            line, col = pos_to_line_col(src, i)
            comments.append((line, col, src[i:j]))
            i = j
            continue

        # String literal or byte string literal.
        if c == '"' or (c == "b" and i + 1 < n and src[i + 1] == '"'):
            if c == "b":
                i += 1
            i += 1  # skip opening quote
            while i < n and src[i] != '"':
                if src[i] == "\\":
                    i += 2
                else:
                    i += 1
            if i < n:
                i += 1  # skip closing quote
            continue

        # Character literal, byte character literal, or lifetime.
        if c == "'" or (c == "b" and i + 1 < n and src[i + 1] == "'"):
            if c == "b":
                i += 1
            i += 1  # skip opening quote
            if i < n:
                if src[i].isalpha() or src[i] == "_":
                    # Could be a lifetime or a single-character literal like 'a'.
                    if i + 1 < n and src[i + 1] == "'":
                        i += 2  # char literal
                        continue
                    # Lifetime: consume the identifier.
                    while i < n and (src[i].isalnum() or src[i] == "_"):
                        i += 1
                    continue
                # Char literal (possibly escaped).
                while i < n and src[i] != "'":
                    if src[i] == "\\":
                        i += 2
                    else:
                        i += 1
                if i < n:
                    i += 1  # skip closing quote
                continue
            continue

        # Raw string literal (possibly byte-prefixed).
        if c == "r" or (c == "b" and i + 1 < n and src[i + 1] == "r"):
            start = i
            if c == "b":
                i += 1
            i += 1  # skip 'r'
            hash_count = 0
            while i < n and src[i] == "#":
                hash_count += 1
                i += 1
            if i < n and src[i] == '"':
                i += 1  # skip opening quote
                while i < n:
                    if src[i] == '"':
                        k = i + 1
                        matched_hashes = 0
                        while (
                            k < n
                            and src[k] == "#"
                            and matched_hashes < hash_count
                        ):
                            matched_hashes += 1
                            k += 1
                        if matched_hashes == hash_count:
                            i = k
                            break
                    i += 1
                continue
            # Not a raw string; resume scanning from just after the 'r'/'b'.
            i = start + 1
            continue

        i += 1

    return comments


def scan_comments(
    comments: list[tuple[int, int, str]],
) -> list[tuple[int, int, str, str]]:
    """Scan extracted ``(start_line, start_col, text)`` comments for tokens.

    Shared by every comment-based scanner (Rust, Python, YAML): each comment
    carries the 1-based line/column of its first character, and match positions
    are translated back into absolute file coordinates.
    """
    violations: list[tuple[int, int, str, str]] = []

    for start_line, start_col, text in comments:
        for match in ISSUE_RE.finditer(text):
            offset = match.start()
            rel_line = text.count("\n", 0, offset) + 1
            last_nl = text.rfind("\n", 0, offset)
            rel_col = offset - last_nl
            line = start_line + rel_line - 1
            col = start_col + rel_col - 1 if rel_line == 1 else rel_col
            violations.append(
                (line, col, match.group(), snippet(text, offset, match.end()))
            )

    return violations


def scan_rust(path: Path) -> list[tuple[int, int, str, str]]:
    """Return violations found in a Rust source file."""
    src = path.read_text(encoding="utf-8")
    return scan_comments(rust_comments(src))


def skip_py_string(src: str, i: int, n: int) -> int:
    """Return the index just past a Python string whose quote is at ``src[i]``.

    Handles triple- and single-quoted strings; a backslash escapes the next
    character for termination purposes in both raw and non-raw strings (a raw
    string still cannot be closed by an escaped quote), so raw/non-raw need no
    separate handling for the purpose of *skipping* the literal.
    """
    quote = src[i]
    if src[i : i + 3] == quote * 3:
        i += 3
        while i < n:
            if src[i] == "\\":
                i += 2
                continue
            if src[i : i + 3] == quote * 3:
                return i + 3
            i += 1
        return n
    i += 1  # skip opening quote
    while i < n:
        c = src[i]
        if c == "\\":
            i += 2
            continue
        if c == quote:
            return i + 1
        if c == "\n":
            return i  # unterminated single-line string
        i += 1
    return n


def python_comments(src: str) -> list[tuple[int, int, str]]:
    """Extract Python ``#`` comments as ``(start_line, start_col, text)``.

    String and docstring literals are skipped so a ``#NNN``-shaped token inside
    a string (or this module's own docstring examples) is never treated as a
    comment. Only ``#`` line comments are returned.
    """
    comments: list[tuple[int, int, str]] = []
    n = len(src)
    i = 0

    while i < n:
        c = src[i]

        # Line comment: everything from '#' to end of line.
        if c == "#":
            j = src.find("\n", i)
            if j == -1:
                j = n
            line, col = pos_to_line_col(src, i)
            comments.append((line, col, src[i:j]))
            i = j
            continue

        # Bare string literal.
        if c in "\"'":
            i = skip_py_string(src, i, n)
            continue

        # Identifier, possibly a string prefix (r"", b'', f"", rb"", ...).
        if c.isalpha() or c == "_":
            j = i
            while j < n and (src[j].isalnum() or src[j] == "_"):
                j += 1
            if (
                j < n
                and src[j] in "\"'"
                and src[i:j].lower() in PY_STRING_PREFIXES
            ):
                i = skip_py_string(src, j, n)
            else:
                i = j
            continue

        i += 1

    return comments


def scan_python(path: Path) -> list[tuple[int, int, str, str]]:
    """Return violations found in a Python source file."""
    src = path.read_text(encoding="utf-8")
    return scan_comments(python_comments(src))


def yaml_comments(src: str) -> list[tuple[int, int, str]]:
    """Extract YAML ``#`` comments as ``(start_line, start_col, text)``.

    A ``#`` opens a comment only at line start or after whitespace and only when
    it is not inside a quoted scalar. Single-quoted scalars escape a quote by
    doubling it (``''``); double-quoted scalars use backslash escapes.
    """
    comments: list[tuple[int, int, str]] = []

    for line_no, line in enumerate(src.splitlines(), start=1):
        n = len(line)
        i = 0
        quote: str | None = None
        while i < n:
            c = line[i]
            if quote == "'":
                if c == "'":
                    if i + 1 < n and line[i + 1] == "'":
                        i += 2  # escaped '' inside a single-quoted scalar
                        continue
                    quote = None
                i += 1
                continue
            if quote == '"':
                if c == "\\":
                    i += 2
                    continue
                if c == '"':
                    quote = None
                i += 1
                continue
            if c in "\"'":
                quote = c
                i += 1
                continue
            if c == "#" and (i == 0 or line[i - 1] in " \t"):
                comments.append((line_no, i + 1, line[i:]))
                break
            i += 1

    return comments


def scan_yaml(path: Path) -> list[tuple[int, int, str, str]]:
    """Return violations found in a YAML source file."""
    src = path.read_text(encoding="utf-8")
    return scan_comments(yaml_comments(src))


def find_inline_code_spans(line: str) -> list[tuple[int, int]]:
    """Return ``(start, end)`` column ranges of inline code spans in ``line``."""
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
        run_len = j - i
        k = j
        while k < n:
            if line[k] != "`":
                k += 1
                continue
            m = k
            while m < n and line[m] == "`":
                m += 1
            if m - k == run_len:
                spans.append((i, m))
                i = m
                break
            k = m
        else:
            i = j

    return spans


def scan_markdown(path: Path) -> list[tuple[int, int, str, str]]:
    """Return violations found in a Markdown file."""
    src = path.read_text(encoding="utf-8")
    violations: list[tuple[int, int, str, str]] = []

    in_fence = False
    for line_no, line in enumerate(src.splitlines(), start=1):
        stripped = line.lstrip()
        if re.match(r"(?:```+|~~~+)", stripped):
            in_fence = not in_fence
            continue
        if in_fence:
            continue

        code_spans = find_inline_code_spans(line)

        for match in ISSUE_RE.finditer(line):
            start = match.start()
            if any(start >= s and start < e for s, e in code_spans):
                continue
            violations.append(
                (
                    line_no,
                    start + 1,
                    match.group(),
                    snippet(line, start, match.end()),
                )
            )

    return violations


def scan_toml(path: Path) -> list[tuple[int, int, str, str]]:
    """Return violations found in a TOML file.

    TOML has no comment/string-lexer subtlety worth modelling here: manifest
    ``description`` strings and ``#`` dependency comments are both plain prose,
    so every ``ISSUE_RE`` match is a real issue reference. Hex color codes are
    already excluded by the token pattern, and after the cleanup there are no
    legitimate ``#NNN`` tokens in these files.
    """
    src = path.read_text(encoding="utf-8")
    violations: list[tuple[int, int, str, str]] = []

    for line_no, line in enumerate(src.splitlines(), start=1):
        for match in ISSUE_RE.finditer(line):
            start = match.start()
            violations.append(
                (
                    line_no,
                    start + 1,
                    match.group(),
                    snippet(line, start, match.end()),
                )
            )

    return violations


def main() -> int:
    root = repo_root()
    violations: list[tuple[Path, int, int, str, str]] = []

    for path in iter_scan_paths(root):
        if path.suffix == ".rs":
            for line, col, token, text in scan_rust(path):
                violations.append((path, line, col, token, text))
        elif path.suffix == ".md":
            for line, col, token, text in scan_markdown(path):
                violations.append((path, line, col, token, text))
        elif path.suffix == ".toml":
            for line, col, token, text in scan_toml(path):
                violations.append((path, line, col, token, text))
        elif path.suffix == ".py":
            for line, col, token, text in scan_python(path):
                violations.append((path, line, col, token, text))
        elif path.suffix in (".yaml", ".yml"):
            for line, col, token, text in scan_yaml(path):
                violations.append((path, line, col, token, text))

    if violations:
        for path, line, col, token, text in violations:
            rel = path.relative_to(root)
            print(f"{rel}:{line}:{col}: {token} {text}")
        return 1

    print("OK: no #NNN issue-reference tokens found in comments or docs.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
