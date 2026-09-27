#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Measure how much structure a new Rust module shares with a reference tree.

A clean-room module is written without reading the crate it replaces, and this is
the after-the-fact check that its text agrees: the NEW module (``--new``, files or
directories) is compared with a REFERENCE tree (``--ref``, for example an upstream
crate directory) and the shared structure is reported WITHOUT printing reference
source text. Everything the report quotes is NEW-side: line ranges of the new
files, and identifiers or literals that occur in the new files. The reference
contributes only counts and file paths.

TOKENS. Rust source is lexed with comments dropped and every string, byte-string,
raw-string and character literal reduced to a category token; keywords and
punctuation are kept as they are. Two passes read the tokens; only (a) decides the
exit status, and (b) is a list for a human to read:

(a) STRUCTURE. Identifiers become ``ID``, numbers ``NUM`` and lifetimes ``LT``, so a
    copy with renamed identifiers still matches. Each function signature and each
    ``use`` item is collapsed to one token first (see ``structural``): a drop-in
    replacement reproduces its API's signatures by contract. Every run of k
    consecutive tokens is hashed, and winnowing with a window of w hashes selects
    the fingerprints (the minimum of each window, rightmost on ties). The report
    gives the count and ratio of NEW fingerprints also present in REF, the NEW-side
    line ranges they cover, and the longest run of consecutive shared fingerprints.

(b) REVIEW TOKENS. Every identifier of at least 5 characters, every non-trivial
    numeric literal (not 0-16 and not a power of two up to 2^16, compared by VALUE,
    so ``0xFF_FF_FF_FF`` and ``4294967295`` are one literal) and every string literal
    of at least 4 characters that occurs in NEW and also in REF -- minus Rust's
    primitive types and prelude names, minus the calibrated GENERIC identifiers
    (below), and minus the allowlist file (``--spec-allow``: one token per line,
    ``#`` starts a comment; tokens that legitimately come from a specification).
    They are reported under ``review``, sorted, each with its NEW-side count and
    locations, and NEVER change the exit status. Independent first-party crates
    still share dozens of such tokens after the generic lists (ordinary words such
    as ``chunk`` or ``canonical``, small constants), so a shared token is a lead to
    look at, not evidence; a gate on it would refuse unrelated code.

THE THRESHOLDS ARE MEASURED, NOT CHOSEN. ``k``, ``w``, the run limit, the ratio limit
and the generic-identifier list are read from ``similarity-calibration.toml`` beside
this file, which ``--calibrate`` writes from two populations in this repository:

* KNOWN-INDEPENDENT pairs: first-party crates of which neither depends on the other
  (checked from their manifests), compared in both directions. Their maxima are the
  noise floor -- structure two independently written Rust modules share anyway.
* PLANTED DERIVATIONS: three first-party modules, each (a) with every identifier
  renamed, (b) renamed and with adjacent independent statements swapped, and
  (c) renamed and with statement runs extracted into helper functions, each
  compared with its own crate. Built under ``target/`` at calibration time only.

For each k in {8, 12, 16, 20, 25} (w = k / 2) the limits are
``run_limit = ceil(1.5 * independent max longest run)`` and
``ratio_limit = 2 * independent max ratio``; a comparison is flagged when its longest
run EXCEEDS the run limit or its ratio EXCEEDS the ratio limit. The chosen k is the
smallest one at which

* both limits are FALSIFIABLE: the ratio limit is below 1.0, since no comparison's
  ratio can exceed 1.0 and a limit at or above it is a test that cannot fire;
* those limits flag every planted derivation and pass every independent pair, and
  the next k in the sweep separates them too, so the choice is not on a knife edge;
* the independent maximum longest run is STABLE: the next k moves it by at most
  ``STABLE_STEP`` of its value. A bigger step says the noise floor at k is still
  short structure every Rust file shares, and a threshold set on it is noise.

If no k qualifies, calibration fails and says so; it does not weaken the check.
Identifiers and literals shared by three or more independent pairs are GENERIC
(std, Rust idiom, common English, everyday constants) and drop out of the review
list (b).

Output: one JSON document on stdout. The exit status is the structural pass (a)
alone: 1 when the longest shared run exceeds the run limit or the shared ratio
exceeds the ratio limit, else 0 -- whatever the review list (b) holds. Exit 2 when
the calibration file is missing, malformed or out of range, or an input cannot be
read. Python standard library only.

    python3 scripts/cleanroom/similarity.py --new crates/x/src/fast --ref /path/upstream/src
    python3 scripts/cleanroom/similarity.py --calibrate      # rewrite the calibration file
    python3 scripts/cleanroom/similarity.py --self-test
"""

from __future__ import annotations

import argparse
import contextlib
import hashlib
import io
import json
import math
import re
import shutil
import sys
import tomllib
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
CALIBRATION = Path(__file__).resolve().parent / "similarity-calibration.toml"
SCRATCH = REPO_ROOT / "target" / "cleanroom-selftest"
MIN_IDENT = 5
MIN_STRING = 4
TRIVIAL_NUMBERS = frozenset(range(17)) | frozenset(1 << bit for bit in range(17))

# Calibration populations. Every pair is re-checked for independence from the two
# manifests before it is measured.
SWEEP = (8, 12, 16, 20, 25)
INDEPENDENT_PAIRS = (
    ("iri", "xsd"),
    ("iri", "text"),
    ("xsd", "rdf-events"),
    ("xsd", "text"),
    ("cdt", "columnar"),
    ("cdt", "gts"),
    ("geo", "text"),
    ("json", "markdown"),
    ("json", "geo"),
    ("hnsw", "shex"),
    ("hnsw", "columnar"),
    ("hnsw", "sparql-results"),
    ("slice", "text"),
    ("markdown", "rdf-events"),
    ("markdown", "datalog"),
    ("shex", "columnar"),
    ("iri", "columnar"),
)
PLANTED_MODULES = (
    "crates/hnsw/src/search.rs",
    "crates/text/src/score.rs",
    "crates/json/src/parse.rs",
)
GENERIC_PAIR_COUNT = 3
# The independent maximum longest run is STABLE at k when the next k in the sweep
# moves it by at most this fraction. A larger step means the maximum at k is still
# dominated by short structure every Rust file shares, not by the pairs' content.
STABLE_STEP = 0.25
# The review-list kinds a calibrated generic list is kept for, in file order.
GENERIC_KINDS = ("identifier", "number", "string")
GENERATED_MARKERS = ("@generated", "do not edit", "automatically generated")

KEYWORDS = frozenset(
    "as async await break const continue crate dyn else enum extern false fn for if impl in "
    "let loop match mod move mut pub ref return self Self static struct super trait true type "
    "union unsafe use where while abstract become box do final gen macro override priv try "
    "typeof unsized virtual yield macro_rules raw safe".split()
)
# Rust's primitive types and the std prelude: every module names these, so a shared
# occurrence carries no information about where the module came from.
LANGUAGE_ALLOW = frozenset(
    "bool char str u8 u16 u32 u64 u128 usize i8 i16 i32 i64 i128 isize f32 f64 "
    "Copy Send Sized Sync Unpin Drop Fn FnMut FnOnce drop Box ToOwned Clone PartialEq "
    "PartialOrd Eq Ord AsRef AsMut Into From Default Iterator Extend IntoIterator "
    "DoubleEndedIterator ExactSizeIterator Option Some None Result Ok Err String ToString "
    "Vec TryFrom TryInto FromIterator".split()
)
PUNCTUATION = sorted(
    "<<= >>= ... ..= :: -> => == != <= >= && || += -= *= /= %= ^= &= |= << >> .. "
    "+ - * / % ^ ! & | = < > @ . , ; : # $ ? ~ { } [ ] ( )".split(),
    key=len,
    reverse=True,
)
ASSIGNMENTS = frozenset("= += -= *= /= %= ^= &= |= <<= >>=".split())
IDENT = re.compile(r"(?:r#)?[A-Za-z_][A-Za-z0-9_]*")
NUMBER = re.compile(
    r"(?:0x[0-9A-Fa-f_]+|0o[0-7_]+|0b[01_]+|[0-9][0-9_]*(?:\.[0-9][0-9_]*)?(?:[eE][+-]?[0-9_]+)?)"
    r"(?:[iu](?:8|16|32|64|128|size)|f32|f64)?"
)
RAW_STRING = re.compile(r'(?:b|c)?r(#*)"')
CHAR_LITERAL = re.compile(r"'(?:\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]+\}|.)|[^\\'\n])'")
LIFETIME = re.compile(r"'[A-Za-z_][A-Za-z0-9_]*")


class CalibrationError(RuntimeError):
    """The calibration file is missing or malformed, or calibration cannot separate."""


@dataclass(frozen=True)
class Token:
    """One lexed token: structural form, exact text, kind, line and source span."""

    shape: str
    text: str
    kind: str  # ident | keyword | number | string | char | lifetime | punct | collapsed
    line: int
    start: int
    end: int


@dataclass(frozen=True)
class Settings:
    """The calibrated parameters of a comparison."""

    k: int
    w: int
    run_limit: int
    ratio_limit: float
    generic: frozenset[tuple[str, str]]


def _skip_block_comment(src: str, index: int) -> int:
    depth = 0
    while index < len(src):
        if src.startswith("/*", index):
            depth += 1
            index += 2
        elif src.startswith("*/", index):
            depth -= 1
            index += 2
            if depth == 0:
                return index
        else:
            index += 1
    return index


def _string_end(src: str, index: int) -> int:
    """Index after a quoted string starting at the quote at *index*."""
    index += 1
    while index < len(src):
        if src[index] == "\\":
            index += 2
        elif src[index] == '"':
            return index + 1
        else:
            index += 1
    return index


def tokenize(src: str) -> list[Token]:
    """Lex Rust source into ``Token``s, dropping comments."""
    tokens: list[Token] = []
    index = 0
    line = 1
    length = len(src)
    while index < length:
        char = src[index]
        start_line = line
        if char == "\n":
            line += 1
            index += 1
            continue
        if char.isspace():
            index += 1
            continue
        if src.startswith("//", index):
            end = src.find("\n", index)
            index = length if end == -1 else end
            continue
        if src.startswith("/*", index):
            end = _skip_block_comment(src, index)
            line += src.count("\n", index, end)
            index = end
            continue
        raw = RAW_STRING.match(src, index) if char in "bcr" else None
        if raw:
            closing = '"' + raw.group(1)
            body_start = raw.end()
            end = src.find(closing, body_start)
            end = length if end == -1 else end + len(closing)
            text = src[body_start : max(body_start, end - len(closing))]
            tokens.append(Token("STR", text, "string", start_line, index, end))
            line += src.count("\n", index, end)
            index = end
            continue
        if char == '"' or (char in "bc" and src.startswith('"', index + 1)):
            quote = index if char == '"' else index + 1
            end = _string_end(src, quote)
            tokens.append(Token("STR", src[quote + 1 : end - 1], "string", start_line, index, end))
            line += src.count("\n", index, end)
            index = end
            continue
        if char == "'" or (char == "b" and src.startswith("'", index + 1)):
            quote = index if char == "'" else index + 1
            literal = CHAR_LITERAL.match(src, quote)
            if literal:
                tokens.append(Token("CHAR", literal.group(0), "char", start_line, index, literal.end()))
                index = literal.end()
                continue
            lifetime = LIFETIME.match(src, index) if char == "'" else None
            if lifetime:
                tokens.append(Token("LT", lifetime.group(0), "lifetime", start_line, index, lifetime.end()))
                index = lifetime.end()
                continue
        if char.isdigit():
            number = NUMBER.match(src, index)
            assert number is not None
            tokens.append(Token("NUM", number.group(0), "number", start_line, index, number.end()))
            index = number.end()
            continue
        ident = IDENT.match(src, index)
        if ident:
            word = ident.group(0)
            bare = word[2:] if word.startswith("r#") else word
            if bare in KEYWORDS and not word.startswith("r#"):
                tokens.append(Token(bare, bare, "keyword", start_line, index, ident.end()))
            else:
                tokens.append(Token("ID", bare, "ident", start_line, index, ident.end()))
            index = ident.end()
            continue
        for punct in PUNCTUATION:
            if src.startswith(punct, index):
                tokens.append(Token(punct, punct, "punct", start_line, index, index + len(punct)))
                index += len(punct)
                break
        else:
            tokens.append(Token(char, char, "punct", start_line, index, index + 1))
            index += 1
    return tokens


def structural(tokens: list[Token]) -> list[Token]:
    """*tokens* with each function signature and each ``use`` item collapsed to one token.

    A drop-in replacement MUST reproduce the signatures of the API it replaces, and
    its imports are dictated by what it calls, so neither is evidence of copying --
    yet a single shared signature such as ``pub fn f(x: &[T]) -> U {`` is fourteen
    identical structural tokens, and a module of drop-in functions would be flagged
    by its contract alone. So a signature (``fn`` up to the ``{`` or ``;`` that ends it
    at bracket depth 0) becomes one ``SIG`` token and a ``use`` item one ``USE`` token;
    the body that follows is still compared token by token.
    """
    out: list[Token] = []
    index = 0
    while index < len(tokens):
        token = tokens[index]
        if token.kind == "keyword" and token.text in ("fn", "use"):
            depth = 0
            end = index + 1
            while end < len(tokens):
                text = tokens[end].text if tokens[end].kind == "punct" else ""
                if text in ("(", "["):
                    depth += 1
                elif text in (")", "]"):
                    depth -= 1
                elif depth <= 0 and (text == ";" or (text == "{" and token.text == "fn")):
                    break
                end += 1
            label = "SIG" if token.text == "fn" else "USE"
            out.append(Token(label, label, "collapsed", token.line, token.start, token.end))
            index = end + 1 if token.text == "use" else end
            continue
        out.append(token)
        index += 1
    return out


def _hash(shapes: tuple[str, ...]) -> int:
    digest = hashlib.blake2b("\x1f".join(shapes).encode("utf-8"), digest_size=8).digest()
    return int.from_bytes(digest, "big")


def fingerprints(tokens: list[Token], k: int, w: int) -> list[tuple[int, int]]:
    """Winnowed ``(hash, k-gram start)`` fingerprints of *tokens*, in position order."""
    shapes = [token.shape for token in tokens]
    grams = [_hash(tuple(shapes[i : i + k])) for i in range(len(shapes) - k + 1)]
    if not grams:
        return []
    if len(grams) < w:
        low = min(range(len(grams)), key=lambda i: (grams[i], -i))
        return [(grams[low], low)]
    selected: list[tuple[int, int]] = []
    last = -1
    for start in range(len(grams) - w + 1):
        low = min(range(start, start + w), key=lambda i: (grams[i], -i))
        if low != last:
            selected.append((grams[low], low))
            last = low
    return selected


def number_value(text: str) -> str:
    """A numeric literal's value as a canonical string (integers exactly)."""
    body = re.sub(r"(?:[iu](?:8|16|32|64|128|size)|f32|f64)$", "", text.replace("_", ""))
    try:
        if body.startswith(("0x", "0X")):
            return str(int(body[2:], 16))
        if body.startswith(("0o", "0O")):
            return str(int(body[2:], 8))
        if body.startswith(("0b", "0B")):
            return str(int(body[2:], 2))
        if re.fullmatch(r"[0-9]+", body):
            return str(int(body))
        return repr(float(body))
    except ValueError:
        return body


def exact_keys(
    tokens: list[Token], generic: frozenset[tuple[str, str]] = frozenset()
) -> dict[tuple[str, str], list[int]]:
    """The pass-(b) candidates of *tokens*: ``(kind, key)`` to the lines they occur on.

    *generic* holds calibrated ``(kind, key)`` pairs that are never candidates.
    """
    keys: dict[tuple[str, str], list[int]] = {}
    for token in tokens:
        key: tuple[str, str] | None = None
        if token.kind == "ident" and len(token.text) >= MIN_IDENT and token.text not in LANGUAGE_ALLOW:
            key = ("identifier", token.text)
        elif token.kind == "number":
            value = number_value(token.text)
            if not (value.isdigit() and int(value) in TRIVIAL_NUMBERS):
                key = ("number", value)
        elif token.kind == "string" and len(token.text) >= MIN_STRING:
            key = ("string", token.text)
        if key is not None and key not in generic:
            keys.setdefault(key, []).append(token.line)
    return keys


def is_generated(path: Path) -> bool:
    """A file under a ``generated`` directory, or one whose head declares it generated."""
    if "generated" in path.parts:
        return True
    try:
        head = "\n".join(path.read_text(encoding="utf-8", errors="replace").splitlines()[:8]).lower()
    except OSError:
        return False
    return any(marker in head for marker in GENERATED_MARKERS)


def rust_files(paths: list[Path], skip_generated: bool = False) -> list[Path]:
    """Every ``.rs`` file named by or under *paths*, sorted."""
    files: set[Path] = set()
    for path in paths:
        if path.is_dir():
            files.update(candidate for candidate in path.rglob("*.rs") if candidate.is_file())
        elif path.is_file():
            files.add(path)
        else:
            raise FileNotFoundError(f"no such file or directory: {path}")
    return sorted(path for path in files if not (skip_generated and is_generated(path)))


def load_allow(path: Path | None) -> set[tuple[str, str]]:
    """The ``--spec-allow`` tokens, keyed the way pass (b) keys NEW tokens."""
    allowed: set[tuple[str, str]] = set()
    if path is None:
        return allowed
    for raw in path.read_text(encoding="utf-8").splitlines():
        entry = raw.split("#", 1)[0].strip() if not raw.lstrip().startswith('"') else raw.strip()
        if not entry:
            continue
        if entry.startswith('"') and entry.endswith('"') and len(entry) >= 2:
            allowed.add(("string", entry[1:-1]))
            continue
        allowed.add(("identifier", entry))
        allowed.add(("string", entry))
        if re.fullmatch(r"[0-9][0-9A-Za-z_.+-]*", entry):
            allowed.add(("number", number_value(entry)))
    return allowed


def load_settings(path: Path) -> Settings:
    """The calibrated settings in *path*; ``CalibrationError`` when absent or malformed."""
    if not path.is_file():
        raise CalibrationError(f"calibration file {path} is missing; run --calibrate")
    try:
        settings = tomllib.loads(path.read_text(encoding="utf-8"))["settings"]
        k, w = settings["k"], settings["window"]
        run_limit, ratio_limit = settings["run_limit"], settings["ratio_limit"]
        lists = {kind: settings[f"generic_{kind}s"] for kind in GENERIC_KINDS}
    except (OSError, tomllib.TOMLDecodeError, KeyError, TypeError) as error:
        raise CalibrationError(f"calibration file {path} is malformed: {error!r}") from error
    if not (
        isinstance(k, int) and isinstance(w, int) and 1 <= w <= k
        and isinstance(run_limit, int) and run_limit >= 0
        and isinstance(ratio_limit, (int, float)) and 0 <= ratio_limit < 1
        and all(
            isinstance(items, list) and all(isinstance(item, str) for item in items)
            for items in lists.values()
        )
    ):
        raise CalibrationError(f"calibration file {path} has out-of-range settings")
    generic = frozenset((kind, item) for kind, items in lists.items() for item in items)
    return Settings(k, w, run_limit, float(ratio_limit), generic)


def _ranges(lines: list[int]) -> list[list[int]]:
    merged: list[list[int]] = []
    for line in sorted(set(lines)):
        if merged and line <= merged[-1][1] + 1:
            merged[-1][1] = line
        else:
            merged.append([line, line])
    return merged


def structure_stats(
    new_shaped: list[tuple[str, list[Token]]], ref_prints: dict[int, set[str]], k: int, w: int
) -> tuple[int, int, int, list[dict]]:
    """``(total, shared, longest run, runs)`` of NEW fingerprints against REF's."""
    total = shared = longest = 0
    runs: list[dict] = []
    for name, shaped in new_shaped:
        prints = fingerprints(shaped, k, w)
        total += len(prints)
        run: list[tuple[int, int]] = []
        for item in [*prints, None]:
            if item is not None and item[0] in ref_prints:
                shared += 1
                run.append(item)
                continue
            if run:
                longest = max(longest, len(run))
                lines = [token.line for _, pos in run for token in shaped[pos : pos + k]]
                runs.append(
                    {
                        "file": name,
                        "fingerprints": len(run),
                        "new_lines": _ranges(lines),
                        "ref_files": sorted(set().union(*(ref_prints[value] for value, _ in run))),
                    }
                )
                run = []
    return total, shared, longest, runs


def ref_index(ref_shaped: list[tuple[str, list[Token]]], k: int, w: int) -> dict[int, set[str]]:
    """Fingerprint hash to the REF files holding it."""
    index: dict[int, set[str]] = {}
    for name, shaped in ref_shaped:
        for value, _ in fingerprints(shaped, k, w):
            index.setdefault(value, set()).add(name)
    return index


def _read_tokens(path: Path) -> list[Token]:
    return tokenize(path.read_text(encoding="utf-8", errors="replace"))


def compare(new_paths: list[Path], ref_paths: list[Path], allow_path: Path | None, settings: Settings) -> tuple[dict, int]:
    """The similarity report and its exit status."""
    new_files = rust_files(new_paths)
    ref_files = rust_files(ref_paths)
    ref_tokens = [(str(ref), _read_tokens(ref)) for ref in ref_files]
    ref_prints = ref_index([(name, structural(tokens)) for name, tokens in ref_tokens], settings.k, settings.w)
    ref_keys: set[tuple[str, str]] = set()
    for _, tokens in ref_tokens:
        ref_keys.update(exact_keys(tokens, settings.generic))
    allowed = load_allow(allow_path)

    new_tokens = [(str(new), _read_tokens(new)) for new in new_files]
    total, shared, longest, runs = structure_stats(
        [(name, structural(tokens)) for name, tokens in new_tokens], ref_prints, settings.k, settings.w
    )
    ratio = shared / total if total else 0.0
    shared_tokens: dict[tuple[str, str], list[str]] = {}
    for name, tokens in new_tokens:
        for key, lines in exact_keys(tokens, settings.generic).items():
            if key in ref_keys and key not in allowed:
                shared_tokens.setdefault(key, []).extend(f"{name}:{line}" for line in lines)

    structure_flagged = longest > settings.run_limit or ratio > settings.ratio_limit
    report = {
        "new_files": [str(path) for path in new_files],
        "ref_files": len(ref_files),
        "structure": {
            "k": settings.k,
            "window": settings.w,
            "run_limit": settings.run_limit,
            "ratio_limit": settings.ratio_limit,
            "new_fingerprints": total,
            "shared_fingerprints": shared,
            "shared_ratio": round(ratio, 6),
            "longest_shared_run": longest,
            "runs_over_limit": [entry for entry in runs if entry["fingerprints"] > settings.run_limit],
            "flagged": structure_flagged,
        },
        # Pass (b): shared non-generic, non-allowlisted tokens for a human to read.
        # Never part of the exit status (see the module doc).
        "review": [
            {
                "kind": kind,
                "token": token,
                "count": len(locations),
                "new_locations": sorted(set(locations)),
            }
            for (kind, token), locations in sorted(shared_tokens.items())
        ],
    }
    status = 1 if structure_flagged else 0
    report["verdict"] = "flagged" if status else "clean"
    return report, status


# ── Planted derivations ──────────────────────────────────────────────────────


def _statements(tokens: list[Token]) -> list[dict]:
    """Every block's statements: ``{"fn_body", "segments": [(start, end, simple)]}``.

    A segment runs from the token after a block's ``{`` or previous ``;`` to the next
    ``;`` at bracket depth 0 of that block; it is SIMPLE when it holds no brace.
    """
    blocks: list[dict] = []
    stack: list[dict] = []
    pending_fn = False
    for index, token in enumerate(tokens):
        text = token.text if token.kind == "punct" else ""
        if token.kind == "keyword" and token.text == "fn":
            pending_fn = True
        if text == "{":
            if stack:
                stack[-1]["braced"] = True
            block = {"fn_body": pending_fn, "segments": [], "seg_start": index + 1, "braced": False, "depth": 0}
            pending_fn = False
            stack.append(block)
            blocks.append(block)
        elif text == "}":
            if stack:
                stack.pop()
        elif not stack:
            continue
        elif text in ("(", "["):
            stack[-1]["depth"] += 1
        elif text in (")", "]"):
            stack[-1]["depth"] -= 1
        elif text == ";" and stack[-1]["depth"] == 0:
            block = stack[-1]
            block["segments"].append((block["seg_start"], index, not block["braced"]))
            block["seg_start"] = index + 1
            block["braced"] = False
    return blocks


def _movable(tokens: list[Token], start: int, end: int) -> bool:
    """A simple statement with no control flow and no macro call."""
    for index in range(start, end + 1):
        token = tokens[index]
        if token.kind == "keyword" and token.text in ("return", "break", "continue"):
            return False
        if token.text == "?" and token.kind == "punct":
            return False
        if token.kind == "ident" and index + 1 <= end and tokens[index + 1].text == "!":
            return False
    return True


def _reads_writes(tokens: list[Token], start: int, end: int) -> tuple[set[str], set[str]]:
    reads = {tokens[i].text for i in range(start, end + 1) if tokens[i].kind == "ident"}
    writes: set[str] = set()
    for index in range(start, end + 1):
        if tokens[index].kind == "punct" and tokens[index].text in ASSIGNMENTS:
            writes = {tokens[i].text for i in range(start, index) if tokens[i].kind == "ident"}
            break
    else:
        first = next((tokens[i].text for i in range(start, end + 1) if tokens[i].kind == "ident"), None)
        if first is not None:
            writes = {first}
    return reads, writes


def _independent(tokens: list[Token], one: tuple[int, int, bool], two: tuple[int, int, bool]) -> bool:
    reads_one, writes_one = _reads_writes(tokens, one[0], one[1])
    reads_two, writes_two = _reads_writes(tokens, two[0], two[1])
    return not (writes_one & reads_two) and not (writes_two & reads_one)


def _rename_map(tokens: list[Token]) -> dict[str, str]:
    names: dict[str, str] = {}
    for token in tokens:
        if token.kind == "ident" and token.text not in LANGUAGE_ALLOW and token.text not in names:
            names[token.text] = f"v{len(names)}"
    return names


def _render(src: str, tokens: list[Token], order: list[int | str], names: dict[str, str]) -> str:
    out: list[str] = []
    for item in order:
        if isinstance(item, str):
            text = item
        else:
            token = tokens[item]
            text = names.get(token.text, token.text) if token.kind == "ident" else src[token.start : token.end]
        out.append(text)
        out.append("\n" if text in ("{", "}", ";") else " ")
    return "".join(out)


def planted_variants(src: str) -> dict[str, tuple[str, int]]:
    """The three planted derivations of *src*, each with its count of edits."""
    tokens = tokenize(src)
    names = _rename_map(tokens)
    identity: list[int | str] = list(range(len(tokens)))
    variants = {"renamed": (_render(src, tokens, identity, names), len(names))}

    blocks = _statements(tokens)
    order: list[int | str] = list(range(len(tokens)))
    swaps = 0
    for block in blocks:
        segments = block["segments"]
        index = 0
        while index + 1 < len(segments):
            one, two = segments[index], segments[index + 1]
            if (
                one[2] and two[2]
                and _movable(tokens, one[0], one[1]) and _movable(tokens, two[0], two[1])
                and _independent(tokens, one, two)
            ):
                first = list(range(one[0], one[1] + 1))
                second = list(range(two[0], two[1] + 1))
                order[one[0] : two[1] + 1] = second + first
                swaps += 1
                index += 2
            else:
                index += 1
    variants["reordered"] = (_render(src, tokens, order, names), swaps)

    replaced: dict[int, list[int | str]] = {}
    helpers: list[int | str] = []
    extracted = 0
    for block in blocks:
        if not block["fn_body"]:
            continue
        segments = block["segments"]
        run: list[tuple[int, int, bool]] = []
        for segment in [*segments, None]:
            if segment is not None and segment[2] and _movable(tokens, segment[0], segment[1]) and (
                not run or run[-1][1] + 1 == segment[0]
            ):
                run.append(segment)
                continue
            if len(run) >= 4:
                chunk = run[1 : 1 + max(2, len(run) // 2)]
                name = f"helper_{extracted}"
                replaced[chunk[0][0]] = [name, "(", "&", "mut", "state", ")", ";"]
                for skip in range(chunk[0][0] + 1, chunk[-1][1] + 1):
                    replaced[skip] = []
                helpers += ["fn", name, "(", "state", ":", "&", "mut", "State", ")", "{"]
                helpers += list(range(chunk[0][0], chunk[-1][1] + 1))
                helpers += ["}"]
                extracted += 1
                break
            run = [segment] if segment is not None and segment[2] else []
    body: list[int | str] = []
    for index in range(len(tokens)):
        body.extend(replaced.get(index, [index]))
    variants["extracted"] = (_render(src, tokens, body + helpers, names), extracted)
    return variants


# ── Calibration ──────────────────────────────────────────────────────────────


def _crate_deps(crate: str) -> set[str]:
    manifest = tomllib.loads((REPO_ROOT / "crates" / crate / "Cargo.toml").read_text(encoding="utf-8"))
    names: set[str] = set()

    def collect(table: dict) -> None:
        for key in ("dependencies", "dev-dependencies", "build-dependencies"):
            for name, spec in table.get(key, {}).items():
                names.add(spec.get("package", name) if isinstance(spec, dict) else name)

    collect(manifest)
    for target in manifest.get("target", {}).values():
        collect(target)
    return names


def _crate_name(crate: str) -> str:
    manifest = tomllib.loads((REPO_ROOT / "crates" / crate / "Cargo.toml").read_text(encoding="utf-8"))
    return manifest["package"]["name"]


def calibrate(out_path: Path) -> int:
    """Measure both populations, choose k and the limits, write *out_path*."""
    for one, two in INDEPENDENT_PAIRS:
        if _crate_name(two) in _crate_deps(one) or _crate_name(one) in _crate_deps(two):
            raise CalibrationError(f"{one} and {two} are not independent: one depends on the other")

    crates = sorted({crate for pair in INDEPENDENT_PAIRS for crate in pair})
    crates += sorted({Path(module).parts[1] for module in PLANTED_MODULES} - set(crates))
    tokens: dict[str, list[tuple[str, list[Token]]]] = {}
    for crate in crates:
        files = rust_files([REPO_ROOT / "crates" / crate / "src"], skip_generated=True)
        tokens[crate] = [(str(path.relative_to(REPO_ROOT)), _read_tokens(path)) for path in files]
    shaped = {crate: [(name, structural(toks)) for name, toks in items] for crate, items in tokens.items()}

    planted_dir = SCRATCH / "similarity-calibration"
    if planted_dir.exists():
        shutil.rmtree(planted_dir)
    planted_dir.mkdir(parents=True)
    planted: list[dict] = []
    for module in PLANTED_MODULES:
        src = (REPO_ROOT / module).read_text(encoding="utf-8")
        for variant, (text, edits) in planted_variants(src).items():
            path = planted_dir / f"{Path(module).parts[1]}-{Path(module).stem}-{variant}.rs"
            path.write_text(text, encoding="utf-8")
            planted.append(
                {"module": module, "variant": variant, "edits": edits, "crate": Path(module).parts[1],
                 "shaped": structural(tokenize(text))}
            )
        if not all(item["edits"] > 0 for item in planted[-3:]):
            raise CalibrationError(f"{module}: a planted variant made no edit, so it tests nothing")

    sweep: list[dict] = []
    per_k: dict[int, dict] = {}
    for k in SWEEP:
        w = k // 2
        index = {crate: ref_index(items, k, w) for crate, items in shaped.items()}
        pairs = []
        for one, two in INDEPENDENT_PAIRS:
            forward = structure_stats(shaped[one], index[two], k, w)
            backward = structure_stats(shaped[two], index[one], k, w)
            pairs.append(
                {
                    "k": k,
                    "pair": f"{one}<->{two}",
                    "longest_run": max(forward[2], backward[2]),
                    "ratio": round(max(forward[1] / max(forward[0], 1), backward[1] / max(backward[0], 1)), 6),
                }
            )
        max_run = max(pair["longest_run"] for pair in pairs)
        max_ratio = max(pair["ratio"] for pair in pairs)
        run_limit = math.ceil(1.5 * max_run)
        ratio_limit = round(2 * max_ratio, 6)
        positives = []
        for item in planted:
            total, shared, longest, _ = structure_stats(
                [(item["module"], item["shaped"])], index[item["crate"]], k, w
            )
            ratio = round(shared / total, 6) if total else 0.0
            positives.append(
                {
                    "k": k,
                    "module": item["module"],
                    "variant": item["variant"],
                    "edits": item["edits"],
                    "longest_run": longest,
                    "ratio": ratio,
                    "flagged": longest > run_limit or ratio > ratio_limit,
                }
            )
        independent_pass = all(
            pair["longest_run"] <= run_limit and pair["ratio"] <= ratio_limit for pair in pairs
        )
        separates = independent_pass and all(item["flagged"] for item in positives)
        per_k[k] = {"pairs": pairs, "positives": positives}
        sweep.append(
            {
                "k": k,
                "window": w,
                "independent_max_run": max_run,
                "independent_max_ratio": max_ratio,
                "run_limit": run_limit,
                "ratio_limit": ratio_limit,
                "planted_min_run": min(item["longest_run"] for item in positives),
                "planted_min_ratio": min(item["ratio"] for item in positives),
                "separates": separates,
            }
        )

    verdicts = sweep_verdicts(sweep)
    for row, verdict in zip(sweep, verdicts):
        row["verdict"] = verdict
        print(
            f"k={row['k']:>2} w={row['window']:>2}  independent max run {row['independent_max_run']:>4} "
            f"ratio {row['independent_max_ratio']:.4f}  ->  run_limit {row['run_limit']:>4} "
            f"ratio_limit {row['ratio_limit']:.4f}  planted min run {row['planted_min_run']:>4} "
            f"ratio {row['planted_min_ratio']:.4f}  {verdict}"
        )
    chosen = choose_k(sweep)

    k = chosen["k"]
    pairs, positives = per_k[k]["pairs"], per_k[k]["positives"]
    # How far past its limit the weakest planted derivation is (>1 means flagged), and
    # how far under its limits the closest independent pair stays (>=1 means passes).
    positive_margin = min(
        max(item["longest_run"] / chosen["run_limit"], item["ratio"] / chosen["ratio_limit"])
        for item in positives
    )
    independent_headroom = min(
        min(
            chosen["run_limit"] / pair["longest_run"] if pair["longest_run"] else math.inf,
            chosen["ratio_limit"] / pair["ratio"] if pair["ratio"] else math.inf,
        )
        for pair in pairs
    )

    keys = {
        crate: {key for _, toks in items for key in exact_keys(toks)}
        for crate, items in tokens.items()
    }
    shared_by_pair = {f"{one}<->{two}": keys[one] & keys[two] for one, two in INDEPENDENT_PAIRS}
    occurrences: dict[tuple[str, str], int] = {}
    for shared in shared_by_pair.values():
        for key in shared:
            occurrences[key] = occurrences.get(key, 0) + 1
    generic = {key for key, count in occurrences.items() if count >= GENERIC_PAIR_COUNT}
    exact_rows = []
    for pair, shared in shared_by_pair.items():
        remaining = shared - generic
        exact_rows.append(
            {
                "pair": pair,
                "shared_identifiers": sum(1 for key in shared if key[0] == "identifier"),
                "shared_identifiers_after_generic": sum(1 for key in remaining if key[0] == "identifier"),
                "shared_literals": sum(1 for key in shared if key[0] != "identifier"),
                "shared_literals_after_generic": sum(1 for key in remaining if key[0] != "identifier"),
            }
        )

    lines = [
        "# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>",
        "# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0",
        "#",
        "# WRITTEN BY `python3 scripts/cleanroom/similarity.py --calibrate`; do not edit by hand.",
        "# `[settings]` is what a comparison reads. `[calibration]` is the evidence behind it:",
        "# the k sweep, the known-independent first-party pairs (the noise floor) and the",
        "# planted derivations (renamed; renamed + reordered; renamed + extracted) at every",
        "# k, the margins at the chosen k, and the exact pass's shared-token counts before",
        "# and after the generic lists.",
        "",
        "[settings]",
        f"k = {k}",
        f"window = {chosen['window']}",
        "# Flag when a comparison's longest shared-fingerprint run EXCEEDS this.",
        f"run_limit = {chosen['run_limit']}",
        "# Flag when a comparison's shared-fingerprint ratio EXCEEDS this.",
        f"ratio_limit = {chosen['ratio_limit']}",
    ]
    for kind in GENERIC_KINDS:
        lines.append(f"# {kind.capitalize()}s shared by >= {GENERIC_PAIR_COUNT} independent pairs; not listed for review.")
        lines.append(f"generic_{kind}s = [")
        lines += [f"    {json.dumps(value)}," for value in sorted(v for kd, v in generic if kd == kind)]
        lines.append("]")
    lines += [
        "",
        "[calibration]",
        "run_limit_rule = \"ceil(1.5 * independent max longest run)\"",
        "ratio_limit_rule = \"2 * independent max ratio\"",
        "choice_rule = \"smallest k whose ratio limit is below 1.0, that separates, whose next k "
        f"separates, and whose independent max run moves by at most {STABLE_STEP} to the next k\"",
        "# The weakest planted derivation's largest (value / limit); above 1.0 means flagged.",
        f"planted_min_margin = {round(positive_margin, 4)}",
        "# The closest independent pair's smallest (limit / value); 1.0 or more means it passes.",
        f"independent_min_headroom = {round(independent_headroom, 4)}",
        "",
    ]
    for row in sweep:
        lines.append("[[calibration.sweep]]")
        lines += [f"{key} = {json.dumps(value)}" for key, value in row.items()]
        lines.append("")
    for value in SWEEP:
        for pair in per_k[value]["pairs"]:
            lines.append("[[calibration.independent]]")
            lines += [f"{key} = {json.dumps(item)}" for key, item in pair.items()]
            lines.append("")
    for value in SWEEP:
        for item in per_k[value]["positives"]:
            lines.append("[[calibration.planted]]")
            lines += [f"{key} = {json.dumps(entry)}" for key, entry in item.items()]
            lines.append("")
    for row in exact_rows:
        lines.append("[[calibration.exact]]")
        lines += [f"{key} = {json.dumps(value)}" for key, value in row.items()]
        lines.append("")
    out_path.write_text("\n".join(lines).rstrip("\n") + "\n", encoding="utf-8")
    load_settings(out_path)
    counts = ", ".join(f"{sum(1 for kd, _ in generic if kd == kind)} {kind}s" for kind in GENERIC_KINDS)
    print(
        f"chosen k={k} w={chosen['window']} run_limit={chosen['run_limit']} "
        f"ratio_limit={chosen['ratio_limit']} planted margin x{positive_margin:.2f} "
        f"independent headroom x{independent_headroom:.2f}; generic: {counts}"
    )
    for row in exact_rows:
        print(
            f"exact {row['pair']:<24} shared identifiers {row['shared_identifiers']:>4} -> "
            f"{row['shared_identifiers_after_generic']:>4}; shared literals {row['shared_literals']:>3} -> "
            f"{row['shared_literals_after_generic']:>3} after generic"
        )
    print(f"wrote {out_path}")
    return 0


def sweep_verdicts(sweep: list[dict]) -> list[str]:
    """Per sweep row: ``qualifies`` or the first reason it does not (see the module doc)."""
    verdicts = []
    for position, row in enumerate(sweep):
        following = sweep[position + 1] if position + 1 < len(sweep) else None
        if row["ratio_limit"] >= 1:
            verdicts.append("ratio limit >= 1.0 cannot fire")
        elif not row["separates"]:
            verdicts.append("does not separate")
        elif following is None:
            verdicts.append("no next k to confirm it")
        elif not following["separates"]:
            verdicts.append("next k does not separate")
        elif abs(following["independent_max_run"] - row["independent_max_run"]) > (
            STABLE_STEP * row["independent_max_run"]
        ):
            verdicts.append("independent max run not stable to the next k")
        else:
            verdicts.append("qualifies")
    return verdicts


def choose_k(sweep: list[dict]) -> dict:
    """The smallest qualifying sweep row; ``CalibrationError`` when none qualifies."""
    for row, verdict in zip(sweep, sweep_verdicts(sweep)):
        if verdict == "qualifies":
            return row
    raise CalibrationError(
        "no k in the sweep has a falsifiable ratio limit, separates every planted derivation "
        "from every independent pair at it and the next k, and has a stable independent "
        "maximum; the check is not weakened to make one"
    )


# ── Self-test ────────────────────────────────────────────────────────────────


REFERENCE_FUNCTION = """\
pub fn upstream_checksum_{n}(bytes: &[u8]) -> u64 {{
    let mut accumulator: u64 = 0xcbf2_9ce4_8422_2325;
    let mut position = 0usize;
    for &byte in bytes.iter() {{
        accumulator ^= u64::from(byte);
        accumulator = accumulator.wrapping_mul(0x0100_0000_01b3);
        if accumulator & 1 == 0 {{
            accumulator = accumulator.rotate_left(5);
        }} else {{
            accumulator = accumulator.rotate_right(3);
        }}
        position += 1;
        if position % 7 == 0 {{
            accumulator = accumulator ^ (accumulator >> 29);
        }}
    }}
    match bytes.len() {{
        0 => accumulator,
        length if length < 8 => accumulator.wrapping_add(length as u64),
        _ => accumulator.wrapping_sub(bytes[0] as u64),
    }}
}}
"""

INDEPENDENT = """\
/// Counts the entries above three, written from the description alone.
pub fn above_three(list: &[u32]) -> usize {
    let mut n = 0;
    let mut i = 0;
    while i < list.len() {
        n += usize::from(list[i] > 3);
        i += 1;
    }
    n
}
"""

SHARES_CONSTANT = """\
pub const OFFSET: u64 = 14695981039346656037;

pub fn seeded(seed: u64) -> u64 {
    match seed {
        0 => OFFSET,
        other => OFFSET ^ other,
    }
}
"""


def self_test(calibration: Path) -> int:
    """(i) renamed copy flagged, (ii) independent passes, (iii) shared constant listed
    for review unless allowlisted without changing the exit status, (iv) a generic
    number is not listed where a distinctive one is -- all under the committed
    calibration -- (v) a missing, unfalsifiable or
    incomplete calibration file is exit 2, and (vi) the k choice refuses an unfiring
    ratio limit, an unstable noise floor and a sweep with no qualifying k."""
    scratch = SCRATCH / "similarity"
    if scratch.exists():
        shutil.rmtree(scratch)
    (scratch / "ref").mkdir(parents=True)
    ok = True

    with contextlib.redirect_stderr(io.StringIO()):
        missing = run_cli(["--new", str(scratch), "--ref", str(scratch), "--calibration", str(scratch / "absent.toml")])
    if missing == 2:
        print("OK: self-test — a missing calibration file is exit 2, not a comparison")
    else:
        print("SELF-TEST FAIL: a missing calibration file did not exit 2")
        ok = False
    try:
        settings = load_settings(calibration)
    except CalibrationError as error:
        print(f"SELF-TEST FAIL: {error}")
        return 1

    reference = "\n".join(REFERENCE_FUNCTION.format(n=n) for n in range(6))
    (scratch / "ref" / "lib.rs").write_text(reference, encoding="utf-8")
    near_copy = planted_variants(reference)["renamed"][0]

    def run(name: str, text: str, allow: str | None = None) -> tuple[dict, int]:
        new = scratch / f"{name}.rs"
        new.write_text(text, encoding="utf-8")
        allow_path = None
        if allow is not None:
            allow_path = scratch / f"{name}.allow"
            allow_path.write_text(allow, encoding="utf-8")
        return compare([new], [scratch / "ref"], allow_path, settings)

    report, status = run("near_copy", near_copy)
    if status == 1 and report["structure"]["flagged"]:
        print(
            "OK: self-test — (i) a renamed near-copy is flagged by structure "
            f"(longest run {report['structure']['longest_shared_run']} > {settings.run_limit})"
        )
    else:
        print(f"SELF-TEST FAIL: (i) near-copy not flagged: {report['structure']}")
        ok = False
    rendered = json.dumps(report)
    if "upstream_checksum" in rendered or "accumulator" in rendered:
        print("SELF-TEST FAIL: the report quotes reference-only text")
        ok = False
    else:
        print("OK: self-test — the report quotes no reference-only identifier")

    report, status = run("independent", INDEPENDENT)
    if status == 0:
        print(
            "OK: self-test — (ii) an independent implementation of a same-shaped function passes "
            f"(shared ratio {report['structure']['shared_ratio']})"
        )
    else:
        print(f"SELF-TEST FAIL: (ii) independent implementation flagged: {report}")
        ok = False

    fnv = number_value("0xcbf2_9ce4_8422_2325")

    def reviewed(report: dict, value: str) -> bool:
        return any(entry["kind"] == "number" and entry["token"] == value for entry in report["review"])

    report, status = run("constant", SHARES_CONSTANT)
    if reviewed(report, fnv) and status == 0 and not report["structure"]["flagged"]:
        print(
            "OK: self-test — (iii) a distinctive constant shared in another spelling is listed "
            "for review, and a review hit alone leaves the exit status 0"
        )
    else:
        print(f"SELF-TEST FAIL: (iii) shared constant not in review, or it changed the status {status}: {report}")
        ok = False
    report, status = run("constant_allowed", SHARES_CONSTANT, "# FNV-1a 64-bit offset basis\n0xcbf29ce484222325\n")
    if status == 0 and not reviewed(report, fnv):
        print("OK: self-test — (iii) the same constant listed in --spec-allow is not listed for review")
    else:
        print(f"SELF-TEST FAIL: allowlisted constant still listed: {report['review']}")
        ok = False

    generic_numbers = sorted(value for kind, value in settings.generic if kind == "number")
    if generic_numbers:
        (scratch / "ref_literal").mkdir()
        # A value no calibrated list holds, beside one that is generic by measurement.
        for label, value in (("generic", generic_numbers[0]), ("distinctive", "999983")):
            (scratch / "ref_literal" / "lib.rs").write_text(f"pub const A: u64 = {value};\n", encoding="utf-8")
            new = scratch / f"literal_{label}.rs"
            new.write_text(f"pub const B: u64 = {value};\n", encoding="utf-8")
            report, status = compare([new], [scratch / "ref_literal"], None, settings)
            listed = reviewed(report, number_value(value))
            if listed == (label == "distinctive") and status == 0:
                print(f"OK: self-test — a shared {label} number is {'' if listed else 'not '}listed for review")
            else:
                print(f"SELF-TEST FAIL: shared {label} number {value} (status {status}): {report['review']}")
                ok = False
    else:
        print("SELF-TEST FAIL: the calibration lists no generic number, so the generic pass is untested")
        ok = False

    text = calibration.read_text(encoding="utf-8")
    unfalsifiable = scratch / "unfalsifiable.toml"
    unfalsifiable.write_text(
        re.sub(r"(?m)^ratio_limit = .*$", "ratio_limit = 1.2", text, count=1), encoding="utf-8"
    )
    truncated = scratch / "truncated.toml"
    truncated.write_text(re.sub(r"(?m)^generic_strings = \[[^\]]*\]\n", "", text, count=1), encoding="utf-8")
    common = ["--new", str(scratch / "independent.rs"), "--ref", str(scratch / "ref")]
    statuses = {}
    for name, path in (("unfalsifiable", unfalsifiable), ("truncated", truncated), ("committed", calibration)):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            statuses[name] = run_cli([*common, "--calibration", str(path)])
    if statuses == {"unfalsifiable": 2, "truncated": 2, "committed": 0}:
        print("OK: self-test — a ratio limit >= 1.0 or a missing generic list is exit 2; the committed file runs")
    else:
        print(f"SELF-TEST FAIL: calibration validation exit statuses {statuses}")
        ok = False

    def row(k: int, run: int, ratio_limit: float, separates: bool = True) -> dict:
        return {"k": k, "independent_max_run": run, "ratio_limit": ratio_limit, "separates": separates}

    unstable = [row(8, 120, 1.1), row(12, 110, 0.6), row(16, 52, 0.35), row(20, 50, 0.2), row(25, 46, 0.1)]
    stable = [row(8, 120, 1.1), row(12, 110, 0.6), row(16, 100, 0.35), row(20, 95, 0.2), row(25, 90, 0.1)]
    if choose_k(unstable)["k"] == 16 and choose_k(stable)["k"] == 12:
        print(
            "OK: self-test — k is refused for an unfiring ratio limit or an unstable noise floor, "
            "and the smallest stable separating k is chosen"
        )
    else:
        print(f"SELF-TEST FAIL: choose_k chose {choose_k(unstable)['k']} and {choose_k(stable)['k']}")
        ok = False
    try:
        choose_k([row(8, 120, 0.6), row(12, 110, 0.5, separates=False), row(16, 100, 0.4, separates=False)])
    except CalibrationError:
        print("OK: self-test — a sweep with no stable separating k is a calibration failure, not a pick")
    else:
        print("SELF-TEST FAIL: choose_k picked a k from a sweep in which none qualifies")
        ok = False

    lexed = tokenize('let s = "a // not a comment"; /* x /* nested */ y */ let c = \'"\'; r#"raw "q""#;')
    kinds = [token.kind for token in lexed]
    if kinds.count("string") == 2 and "char" in kinds and all(token.text not in ("x", "y") for token in lexed):
        print("OK: self-test — strings, nested comments and a quote char lex as one token each")
    else:
        print(f"SELF-TEST FAIL: lexer: {[(t.kind, t.text) for t in lexed]}")
        ok = False

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def run_cli(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--new", type=Path, nargs="+", help="new module files or directories")
    parser.add_argument("--ref", type=Path, nargs="+", help="reference files or directories")
    parser.add_argument("--spec-allow", type=Path, help="allowlisted tokens, one per line")
    parser.add_argument("--calibration", type=Path, default=CALIBRATION)
    parser.add_argument("--calibrate", action="store_true", help="measure and rewrite the calibration file")
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args(argv)
    if args.self_test:
        return self_test(args.calibration)
    if args.calibrate:
        try:
            return calibrate(args.calibration)
        except CalibrationError as error:
            print(f"similarity: {error}", file=sys.stderr)
            return 2
    if not args.new or not args.ref:
        parser.error("--new and --ref are required")
    try:
        settings = load_settings(args.calibration)
        report, status = compare(args.new, args.ref, args.spec_allow, settings)
    except CalibrationError as error:
        print(f"similarity: {error}", file=sys.stderr)
        return 2
    except OSError as error:
        print(f"similarity: {error}", file=sys.stderr)
        return 2
    if not report["new_files"]:
        print("similarity: --new names no .rs file", file=sys.stderr)
        return 2
    print(json.dumps(report, indent=2, sort_keys=True))
    return status


if __name__ == "__main__":
    raise SystemExit(run_cli(sys.argv[1:]))
