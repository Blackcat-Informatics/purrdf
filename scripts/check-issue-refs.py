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


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def iter_scan_paths(root: Path) -> Iterator[Path]:
    """Yield every tracked ``.rs`` and ``.md`` file the lint enforces.

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
        if Path(rel).suffix not in (".rs", ".md"):
            continue
        segments = rel.split("/")
        in_scan_dir = segments[0] in SCAN_DIRS
        root_md = len(segments) == 1 and rel.endswith(".md")
        if not (in_scan_dir or root_md):
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


def scan_rust(path: Path) -> list[tuple[int, int, str, str]]:
    """Return violations found in a Rust source file."""
    src = path.read_text(encoding="utf-8")
    violations: list[tuple[int, int, str, str]] = []

    for start_line, start_col, text in rust_comments(src):
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

    if violations:
        for path, line, col, token, text in violations:
            rel = path.relative_to(root)
            print(f"{rel}:{line}:{col}: {token} {text}")
        return 1

    print("OK: no #NNN issue-reference tokens found in comments or docs.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
