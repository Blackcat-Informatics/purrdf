#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Measure the vector work a hot path actually compiles to, from emitted assembly.

A claim that a kernel "vectorizes" is a claim about what LLVM emitted, and the source
does not settle it: a fold written ``a[i * LANES + l]`` compiles to zero vector
instructions while the same fold over ``chunks_exact`` and ``array::from_fn`` compiles
to a packed loop. So this gate reads the emitted asm and counts.

Emission
--------

For each of seven target configurations the audited packages are built with
``--emit=asm`` added to rustc's flags (rustc accumulates emit kinds, so the rlib, the
rmeta and a ``.s`` per crate are all produced), LTO off and one codegen unit::

    x86_64               x86_64-unknown-linux-gnu, -C target-cpu=x86-64 (baseline SSE2)
    x86_64-v3            x86_64-unknown-linux-gnu, -C target-cpu=x86-64-v3
    x86_64-v4            x86_64-unknown-linux-gnu, -C target-cpu=x86-64-v4
    aarch64              aarch64-unknown-linux-gnu (baseline NEON)
    aarch64-neoverse-v1  aarch64-unknown-linux-gnu, -C target-cpu=neoverse-v1 (SVE)
    wasm32               wasm32-unknown-unknown (no SIMD proposal)
    wasm32-simd128       wasm32-unknown-unknown, -C target-feature=+simd128

That emits a ``.s`` for EVERY crate in the graph, dependencies included, so a
dependency's own kernels (memchr's, sha2's) are measured in that dependency's symbols.
The asm is per crate and before LTO; LTO can only inline further, never remove a vector
body a crate already compiled.

The flags travel in ``CARGO_TARGET_<TRIPLE>_RUSTFLAGS``. Cargo gives ``RUSTFLAGS`` and
``CARGO_ENCODED_RUSTFLAGS`` precedence over every target-scoped value, so a caller with
either set would silently measure a build without ``--emit=asm`` or the CPU flag; that
is refused by name. Every rustc command line cargo runs for a target unit is read back
from ``cargo -v`` and must carry ``--emit=asm`` and exactly the configuration's
``target-cpu``/``target-feature`` values, so a flag injected by any other configuration
layer is a failure rather than a different measurement.

Build scripts in the graph compile C for their own crates (blake3's NEON and
assembly backends). Those objects are never measured -- only rustc's emission is -- but
the build scripts must still succeed. For a configuration whose target is not the host,
the gate names one C toolchain for them through cc-rs's per-target variables: ``clang``
(a cross compiler for every target it is given) and ``llvm-ar``, unless the caller has
set ``CC_<triple>``/``AR_<triple>`` itself.

Host-CPU C flags are stripped from cross builds, and only from them. cc-rs 1.2
(``src/lib.rs``) reads a flag variable through ``envflags``, which walks
``target_envs`` -- ``CFLAGS_<target>``, ``CFLAGS_<target_underscored>``,
``TARGET_CFLAGS``, ``CFLAGS`` -- and appends EVERY one that is set ("Collect from all
environment variables, in reverse order as in ``getenv_with_target_prefixes``
precedence"), so a plain ``CFLAGS`` always reaches a cross compile line and no
target-scoped variable can remove a token from it. A host-tuned ``-march=native`` is
not a valid flag for a foreign target (clang: "unsupported argument 'native' to option
'-march='"), and a later flag cannot repair an invalid one. So for a configuration
whose triple is not the host's, the child's copy of ``CFLAGS`` and ``CXXFLAGS`` loses
exactly ``-march=native``, ``-mtune=native`` and ``-mcpu=native`` (also split from
``native``), every other token kept in order, and each removal is printed. Host
configurations inherit both variables byte-for-byte, and the gate's own environment is
never modified. The ``-D warnings`` that
``build.rustflags`` would carry is restated in the per-target flags, which replace it. A
missing target standard library or a missing tool is a hard failure, locally and in CI;
nothing here skips.

Parsing
-------

Each ``.s`` is split into functions: ELF ``.type NAME,@function`` ... ``.size NAME``,
and wasm ``NAME:`` / ``.functype NAME`` ... ``end_function``. ``.L`` labels are never
functions. LLVM clones (``.llvm.N``, ``.cold``, ``.constprop.N``, ``.isra.N``,
``.part.N``) are folded into their parent: a kernel whose loop was split into a cold
clone is still one kernel.

Names are demangled by a small built-in demangler for both the v0 (``_R...``) and the
legacy (``_ZN...E``) schemes, enough to recover the path and the crate. A manifest
entry matches by that crate plus a substring of the demangled path. A private or
inlined site is measured in the enclosing hot symbol, which its entry names.

Counting
--------

*Vector work* is packed arithmetic, compare, mask-extract, conversion, logical and
shuffle-for-reduction instructions on vector registers (see ``VECTOR_*`` below).
Moves, loads, stores, broadcasts and constants are not work. Same-register zeroing and
all-ones idioms (``xorps %xmm0, %xmm0``, ``pcmpeqd %xmm1, %xmm1``,
``eor v0.16b, v0.16b, v0.16b``) are excluded explicitly, because LLVM emits them in
scalar code too. *FMA* is any fused multiply-add, scalar or packed. *Relaxed* is any
``relaxed_`` wasm instruction.

Manifest (``scripts/simd-asm-manifest.toml``)
---------------------------------------------

``[build] packages`` names the packages to build. ``[build] rlib_packages`` names
packages that are also a ``cdylib``: linking a cdylib needs a linker for the target,
which a cross configuration does not have and the asm does not need, so each of those
is built on its own by ``cargo rustc --crate-type rlib``, against the same dependency
units. Each ``[[site]]`` has an ``id`` and a
list of ``[[site.measure]]`` tables, which between them cover all seven
configurations. A measure names ``configs``, ``crate`` and ``symbol`` and carries:

* ``min_vector_ops`` -- every matched copy has at least this much vector work;
* ``require_mnemonics`` -- instructions that prove the shape, each present in every
  matched copy. ``"pcmpeqb"`` names a mnemonic; ``"fmla .2d"`` also requires an operand
  containing ``.2d``. A vector mnemonic only counts when it is real work, so an idiom
  cannot satisfy it;
* exactly one of ``max_fma`` (exact arithmetic: 0) or ``min_fma`` (fast arithmetic);
* ``forbid_relaxed`` -- stated explicitly on every measure;
* ``single_copy`` -- the symbol exists exactly once, out of line, in every build graph
  that emits it. A configuration is one ``cargo build`` plus one ``cargo rustc`` per
  ``rlib_packages`` entry, and each is its own graph: a crate a second graph rebuilds
  under different dependency features is a second, separate artifact, not a second
  copy inside one program. Two copies in ONE graph (a generic body instantiated in two
  downstream crates, or an inlined clone kept out of line) is the failure.

A measure that matches no function fails: the symbol was renamed or inlined away, and
either way the evidence is gone.

Static checks
-------------

* The identity-path scan: the fast float arithmetic (``algebraic_add`` and its
  siblings, and the ``Reassociated`` arithmetic) may appear only under
  ``crates/rdf-core/src/distance/``, ``crates/sparql-eval/src/knn/``,
  ``crates/hnsw/src/`` and those crates' ``tests/`` and ``benches/``. Content identity,
  canonical serialization, GTS and RDFC-1.0 are exact by contract.
* The multiplier scan: no speedup multiplier or percentage in the audit document or
  the unreleased CHANGELOG section. Evidence is an instruction count.

Document mode (``--doc``)
-------------------------

``docs/design/purrdf-simd.md`` carries two generated regions:

* ``<!-- simd-asm:sites:begin -->`` ... ``<!-- simd-asm:sites:end -->``: the site table.
  Its header names ``id``, ``crate``, ``fn``, ``covers``, ``verdict`` and ``reason`` and
  one column per configuration name; further descriptive columns are free-form and
  kept as written. The configuration cells are generated.
* ``<!-- simd-asm:benches:begin -->`` ... ``<!-- simd-asm:benches:end -->``: the bench
  table, with ``bench`` (a ``crates/<crate>/benches/<file>.rs`` path) and ``sites``.

Parity: every manifest id is a row; every row that names a function has a manifest
entry, whatever its verdict; a row that names no function is crate-level, its verdict
is ``leave``, and its reason cites ``crates/<crate>/benches/``, which must not exist;
the generated cells equal the measured counts (``--write-doc`` regenerates them).
Coverage: every workspace member has a row; every bench file has a row naming only
known site ids; every roster site below is in the ``covers`` cell of a row that names a
function with a manifest entry. ``--doc`` with no document is a failure, not a skip.

``--self-test`` exercises every refusal against an embedded fixture, and the valid
neighbour of each, without building anything.
"""

from __future__ import annotations

import argparse
import dataclasses
import json
import os
import re
import shlex
import shutil
import signal
import subprocess
import sys
import tomllib
from collections import Counter
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
MANIFEST = REPO_ROOT / "scripts" / "simd-asm-manifest.toml"
DOC = REPO_ROOT / "docs" / "design" / "purrdf-simd.md"
CHANGELOG = REPO_ROOT / "CHANGELOG.md"
BUILD_SCRATCH = REPO_ROOT / "scripts" / "build-scratch.sh"


# --------------------------------------------------------------------------- configs


@dataclasses.dataclass(frozen=True)
class Config:
    """One target configuration: a triple plus the codegen flags that define it."""

    name: str
    triple: str
    flags: tuple[str, ...]
    arch: str  # "x86", "aarch64" or "wasm"

    def target_cpus(self) -> set[str]:
        return {f.split("=", 1)[1] for f in self.flags if f.startswith("target-cpu=")}

    def target_features(self) -> set[str]:
        return {f.split("=", 1)[1] for f in self.flags if f.startswith("target-feature=")}

    def rustflags(self) -> str:
        # `-D warnings` restates the workspace bar: a target-scoped RUSTFLAGS replaces
        # `build.rustflags`, so it would otherwise be lost for exactly these builds.
        return " ".join(["--emit=asm", "-D warnings", *(f"-C {flag}" for flag in self.flags)])


CONFIGS: tuple[Config, ...] = (
    Config("x86_64", "x86_64-unknown-linux-gnu", ("target-cpu=x86-64",), "x86"),
    Config("x86_64-v3", "x86_64-unknown-linux-gnu", ("target-cpu=x86-64-v3",), "x86"),
    Config("x86_64-v4", "x86_64-unknown-linux-gnu", ("target-cpu=x86-64-v4",), "x86"),
    Config("aarch64", "aarch64-unknown-linux-gnu", (), "aarch64"),
    Config("aarch64-neoverse-v1", "aarch64-unknown-linux-gnu", ("target-cpu=neoverse-v1",), "aarch64"),
    Config("wasm32", "wasm32-unknown-unknown", (), "wasm"),
    Config("wasm32-simd128", "wasm32-unknown-unknown", ("target-feature=+simd128",), "wasm"),
)
CONFIG_NAMES = tuple(config.name for config in CONFIGS)
CONFIG_BY_NAME = {config.name: config for config in CONFIGS}


class GateError(Exception):
    """A failure the gate reports by name and exits non-zero on."""


# --------------------------------------------------------------------------- demangling


class _DemangleError(Exception):
    pass


_BASIC_TYPES = {
    "a": "i8", "b": "bool", "c": "char", "d": "f64", "e": "str", "f": "f32",
    "h": "u8", "i": "isize", "j": "usize", "l": "i32", "m": "u32", "n": "i128",
    "o": "u128", "s": "i16", "t": "u16", "u": "()", "v": "...", "x": "i64",
    "y": "u64", "z": "!", "p": "_",
}
_B62 = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"


class _V0:
    """A recursive-descent reader of the v0 mangling grammar (RFC 2603).

    It prints paths the way ``rustc-demangle`` does in its alternate (hash-free) form:
    ``memchr::arch::x86_64::memchr::memchr2_raw::find_avx2``, ``<T>::method``,
    ``<T as Trait>::method`` and ``path::<Args>``.
    """

    def __init__(self, text: str) -> None:
        self.s = text
        self.pos = 0
        self.depth = 0

    # -- lexical
    def peek(self) -> str:
        return self.s[self.pos] if self.pos < len(self.s) else ""

    def next(self) -> str:
        if self.pos >= len(self.s):
            raise _DemangleError("unexpected end")
        c = self.s[self.pos]
        self.pos += 1
        return c

    def eat(self, c: str) -> bool:
        if self.peek() == c:
            self.pos += 1
            return True
        return False

    def base62(self) -> int:
        if self.eat("_"):
            return 0
        value = 0
        while True:
            c = self.next()
            if c == "_":
                return value + 1
            digit = _B62.find(c)
            if digit < 0:
                raise _DemangleError(f"bad base-62 digit {c!r}")
            value = value * 62 + digit

    def opt_tagged(self, tag: str) -> int:
        return self.base62() + 1 if self.eat(tag) else 0

    def decimal(self) -> int:
        start = self.pos
        if self.peek() == "0":
            self.pos += 1
            return 0
        while self.peek().isdigit():
            self.pos += 1
        if start == self.pos:
            raise _DemangleError("expected a decimal number")
        return int(self.s[start:self.pos])

    def ident(self) -> tuple[int, str]:
        dis = self.opt_tagged("s")
        punycode = self.eat("u")
        length = self.decimal()
        self.eat("_")
        name = self.s[self.pos:self.pos + length]
        if len(name) != length:
            raise _DemangleError("identifier runs past the end")
        self.pos += length
        return dis, (f"punycode{{{name}}}" if punycode else name)

    def backref(self, parse):
        target = self.base62()
        if target >= self.pos:
            raise _DemangleError("forward backref")
        saved = self.pos
        self.pos = target
        self.depth += 1
        if self.depth > 200:
            raise _DemangleError("backref recursion too deep")
        try:
            return parse()
        finally:
            self.depth -= 1
            self.pos = saved

    # -- grammar
    def path(self, in_type: bool = False) -> tuple[str, str | None]:
        """A path as ``(text, crate)``; ``crate`` is the crate that defines it."""
        c = self.next()
        if c == "C":
            _dis, name = self.ident()
            return name, name
        if c == "N":
            ns = self.next()
            inner, crate = self.path(in_type)
            dis, name = self.ident()
            if ns.isupper():
                label = {"C": "closure", "S": "shim"}.get(ns, ns)
                shown = f"{{{label}:{name}#{dis}}}" if name else f"{{{label}#{dis}}}"
                return f"{inner}::{shown}", crate
            return f"{inner}::{name}", crate
        if c == "M":
            self.opt_tagged("s")
            _impl, crate = self.path()
            self_ty = self.type_()
            return f"<{self_ty}>", crate
        if c == "X":
            self.opt_tagged("s")
            _impl, crate = self.path()
            self_ty = self.type_()
            trait, _trait_crate = self.path(in_type=True)
            return f"<{self_ty} as {trait}>", crate
        if c == "Y":
            self_ty = self.type_()
            trait, crate = self.path(in_type=True)
            return f"<{self_ty} as {trait}>", crate
        if c == "I":
            inner, crate = self.path(in_type)
            args = []
            while not self.eat("E"):
                args.append(self.generic_arg())
            sep = "<" if in_type else "::<"
            return f"{inner}{sep}{', '.join(args)}>", crate
        if c == "B":
            return self.backref(lambda: self.path(in_type))
        raise _DemangleError(f"bad path tag {c!r}")

    def generic_arg(self) -> str:
        if self.eat("L"):
            self.base62()
            return "'_"
        if self.eat("K"):
            return self.const()
        return self.type_()

    def type_(self) -> str:
        c = self.peek()
        if c in _BASIC_TYPES:
            self.pos += 1
            return _BASIC_TYPES[c]
        if c in "CNMXYI":
            return self.path(in_type=True)[0]
        self.pos += 1
        if c == "A":
            inner = self.type_()
            return f"[{inner}; {self.const()}]"
        if c == "S":
            return f"[{self.type_()}]"
        if c == "T":
            items = []
            while not self.eat("E"):
                items.append(self.type_())
            return "(" + ", ".join(items) + ("," if len(items) == 1 else "") + ")"
        if c in "RQ":
            if self.eat("L"):
                self.base62()
            return ("&" if c == "R" else "&mut ") + self.type_()
        if c in "PO":
            return ("*const " if c == "P" else "*mut ") + self.type_()
        if c == "F":
            self.opt_tagged("G")
            unsafe = self.eat("U")
            abi = ""
            if self.eat("K"):
                abi = "C" if self.eat("C") else self.ident()[1]
            params = []
            while not self.eat("E"):
                params.append(self.type_())
            ret = self.type_()
            prefix = ("unsafe " if unsafe else "") + (f'extern "{abi}" ' if abi else "")
            return f"{prefix}fn({', '.join(params)}) -> {ret}"
        if c == "D":
            self.opt_tagged("G")
            traits = []
            while not self.eat("E"):
                trait = self.path(in_type=True)[0]
                while self.eat("p"):
                    _dis, name = self.ident()
                    trait += f"<{name} = {self.type_()}>"
                traits.append(trait)
            if self.eat("L"):
                self.base62()
            return "dyn " + " + ".join(traits)
        if c == "B":
            return self.backref(self.type_)
        raise _DemangleError(f"bad type tag {c!r}")

    def const(self) -> str:
        if self.eat("B"):
            return self.backref(self.const)
        if self.eat("p"):
            return "_"
        c = self.next()
        if c in "RQ":
            return "&" + self.const()
        if c == "A":
            items = []
            while not self.eat("E"):
                items.append(self.const())
            return "[" + ", ".join(items) + "]"
        if c == "T":
            items = []
            while not self.eat("E"):
                items.append(self.const())
            return "(" + ", ".join(items) + ")"
        if c == "V":
            name = self.path(in_type=True)[0]
            shape = self.next()
            if shape == "U":
                return name
            fields = []
            while not self.eat("E"):
                if shape == "S":
                    self.ident()
                fields.append(self.const())
            return f"{name}{{{', '.join(fields)}}}"
        negative = self.eat("n")
        start = self.pos
        while self.peek() and self.peek() != "_":
            self.pos += 1
        digits = self.s[start:self.pos]
        self.next()
        value = int(digits, 16) if digits else 0
        if c == "b":
            return "true" if value else "false"
        if c == "e":
            return repr(bytes.fromhex(digits).decode("utf-8", "replace"))
        return f"{'-' if negative else ''}{value}"


_LEGACY_ESCAPES = {
    "SP": "@", "BP": "*", "RF": "&", "LT": "<", "GT": ">", "LP": "(", "RP": ")", "C": ",",
}


def _legacy_segment(seg: str) -> str:
    if seg.startswith("_$"):
        seg = seg[1:]
    out = []
    i = 0
    while i < len(seg):
        if seg[i] == "$":
            end = seg.find("$", i + 1)
            if end < 0:
                out.append(seg[i:])
                break
            code = seg[i + 1:end]
            if code in _LEGACY_ESCAPES:
                out.append(_LEGACY_ESCAPES[code])
            elif code.startswith("u") and re.fullmatch(r"u[0-9a-f]+", code):
                out.append(chr(int(code[1:], 16)))
            else:
                out.append(seg[i:end + 1])
            i = end + 1
        elif seg.startswith("..", i):
            out.append("::")
            i += 2
        else:
            out.append(seg[i])
            i += 1
    return "".join(out)


def _demangle_legacy(symbol: str) -> tuple[str, str | None, str] | None:
    """``_ZN<len><seg>...E[suffix]`` -> (path, crate, suffix), the hash segment dropped."""
    body = symbol[3:]
    pos = 0
    segments = []
    while pos < len(body) and body[pos] != "E":
        match = re.match(r"\d+", body[pos:])
        if not match:
            return None
        length = int(match.group(0))
        pos += len(match.group(0))
        segments.append(body[pos:pos + length])
        pos += length
    if pos >= len(body):
        return None
    suffix = body[pos + 1:]
    if segments and re.fullmatch(r"h[0-9a-f]{16}", segments[-1]):
        segments.pop()
    if not segments:
        return None
    parts = [_legacy_segment(s) for s in segments]
    path = "::".join(parts)
    first = parts[0]
    crate = first if re.fullmatch(r"[A-Za-z_][\w]*", first) else None
    if crate is None:
        match = re.match(r"<+&?(?:mut )?([A-Za-z_]\w*)::", first)
        crate = match.group(1) if match else None
    return path, crate, suffix


@dataclasses.dataclass(frozen=True)
class Demangled:
    path: str
    crate: str | None
    base: str  # the symbol with any LLVM clone suffix removed


def demangle(symbol: str) -> Demangled:
    """Demangle a v0 or legacy Rust symbol; anything else is returned unchanged.

    The clone suffix (``.llvm.N``, ``.cold`` ...) is cut off first, so a clone and its
    parent share one ``base``.
    """
    if symbol.startswith("_R"):
        base = symbol.split(".", 1)[0]
        reader = _V0(base[2:])
        try:
            if reader.peek().isdigit():
                reader.decimal()
            # Backrefs are offsets from the start of the text after `_R`.
            path, crate = reader.path()
        except (_DemangleError, RecursionError, ValueError):
            return Demangled(base, None, base)
        return Demangled(path, crate, base)
    if symbol.startswith("_ZN"):
        parsed = _demangle_legacy(symbol)
        if parsed is not None:
            path, crate, suffix = parsed
            base = symbol[: len(symbol) - len(suffix)] if suffix else symbol
            return Demangled(path, crate, base)
    base = strip_clone_suffix(symbol)
    return Demangled(base, None, base)


_CLONE_SUFFIX = re.compile(r"(?:\.(?:llvm\.\d+|cold(?:\.\d+)?|constprop\.\d+|isra\.\d+|part\.\d+|specialized\.\d+))+$")


def strip_clone_suffix(symbol: str) -> str:
    return _CLONE_SUFFIX.sub("", symbol)


# --------------------------------------------------------------------------- asm parsing


@dataclasses.dataclass
class Instruction:
    mnemonic: str
    operands: tuple[str, ...]


@dataclasses.dataclass
class Function:
    """One out-of-line function, with its LLVM clones folded in."""

    symbol: str  # the base (clone-free) symbol
    path: str
    crate: str | None
    unit: str  # which .s it came from
    instructions: list[Instruction] = dataclasses.field(default_factory=list)
    graph: int = 0  # which cargo invocation of the configuration emitted the unit


def _split_operands(text: str) -> tuple[str, ...]:
    parts, depth, current = [], 0, []
    for ch in text:
        if ch in "([{":
            depth += 1
        elif ch in ")]}":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append("".join(current).strip())
            current = []
        else:
            current.append(ch)
    tail = "".join(current).strip()
    if tail:
        parts.append(tail)
    return tuple(parts)


def _strip_comment(line: str, arch: str) -> str:
    marker = "//" if arch == "aarch64" else "#"
    cut = line.find(marker)
    return line if cut < 0 else line[:cut]


_ELF_TYPE = re.compile(r"^\s*\.type\s+([^,\s]+)\s*,\s*[@%]function")
_SIZE = re.compile(r"^\s*\.size\s+([^,\s]+)\s*,")
_FUNCTYPE = re.compile(r"^\s*\.functype\s+(\S+)")
_LABEL = re.compile(r"^([^\s:#]+):\s*(?:#.*)?$")


def parse_asm(text: str, arch: str, unit: str, want=None) -> list[tuple[str, list[Instruction]]]:
    """Split one ``.s`` into ``(raw symbol, instructions)`` pairs, clones not yet folded.

    A function begins at its label when the symbol was declared ``@function`` (ELF and
    wasm both emit ``.type``) or when the label is followed by ``.functype`` (wasm). It
    ends at ``.size`` (ELF) or ``end_function`` (wasm). ``.L`` labels are block labels,
    never functions. ``want(symbol)``, when given, decides at the label whether a
    function's instructions are collected at all.
    """
    lines = text.splitlines()
    declared = {m.group(1) for line in lines if (m := _ELF_TYPE.match(line))}
    functions: list[tuple[str, list[Instruction]]] = []
    current: tuple[str, list[Instruction]] | None = None
    for index, raw in enumerate(lines):
        label = _LABEL.match(raw)
        if label and not raw.startswith((" ", "\t")):
            name = label.group(1)
            if name.startswith(".L") and name not in declared:
                continue
            is_function = name in declared
            if not is_function:
                following = next((l for l in lines[index + 1:index + 4] if l.strip()), "")
                ft = _FUNCTYPE.match(following)
                is_function = ft is not None and ft.group(1) == name
            if is_function:
                current = (name, [])
                if want is None or want(name):
                    functions.append(current)
                else:
                    current = (name, None)
            continue
        if current is None:
            continue
        stripped = raw.strip()
        if arch == "wasm" and stripped == "end_function":
            current = None
            continue
        size = _SIZE.match(raw)
        if size:
            if size.group(1) == current[0]:
                current = None
            continue
        if current[1] is None:
            continue
        body = _strip_comment(raw, arch).strip()
        if not body or body.startswith(".") or body.endswith(":"):
            continue
        head, *rest = re.split(r"\s+", body, maxsplit=1)
        current[1].append(Instruction(head.lower(), _split_operands(rest[0] if rest else "")))
    return functions


def collect_functions(units, arch: str, keep=None, select=None) -> list[Function]:
    """Every function of every unit, with LLVM clones folded under their parent.

    ``units`` yields ``(unit name, asm text)`` or ``(unit name, asm text, graph)``, where
    ``graph`` numbers the cargo invocation that emitted the unit (0 when omitted).
    ``select(Demangled)`` decides from the name alone whether a function is parsed, and
    ``keep(Function)`` filters what was parsed, so a whole-graph build is never held in
    memory -- only the functions something reads.
    """
    out: list[Function] = []
    want = None if select is None else (lambda raw: select(demangle(raw)))
    for unit, text, *rest in units:
        graph = rest[0] if rest else 0
        by_base: dict[str, Function] = {}
        order: list[str] = []
        for raw, instructions in parse_asm(text, arch, unit, want):
            demangled = demangle(raw)
            func = by_base.get(demangled.base)
            if func is None:
                func = Function(demangled.base, demangled.path, demangled.crate, unit, graph=graph)
                by_base[demangled.base] = func
                order.append(demangled.base)
            func.instructions.extend(instructions)
        out.extend(by_base[b] for b in order if keep is None or keep(by_base[b]))
    return out


# --------------------------------------------------------------------------- counting

_X86_VREG = re.compile(r"%[xyz]mm\d+|%k[0-7]")
_ARM_VREG = re.compile(r"\b[vz]\d+\.\d*[bhsdq]\b|\bz\d+\.[bhsdq]\b")

# x86 vector work, by mnemonic. Moves (mov*), broadcasts, inserts/extracts of one lane,
# loads and stores are deliberately absent.
VECTOR_X86 = re.compile(
    r"^(?:"
    r"v?(?:add|sub|mul|div|min|max|sqrt|and|andn|or|xor|hadd|hsub|addsub)p[sd]"
    r"|v?cmp[a-z]*p[sd]"
    r"|v?p(?:add|sub)(?:u?s)?[bwdq]"
    r"|v?pcmp(?:eq|gt)[bwdq]|vpcmp[a-z]*"
    r"|v?p(?:min|max)[su][bwdq]"
    r"|v?p(?:and|andn|or|xor)[dq]?|vpternlog[dq]"
    r"|v?pmovmskb|v?movmskp[sd]|v?ptest|vptestn?m[bwdq]"
    r"|v?pmul(?:l[wdq]|h[u]?w|udq|dq|hrsw)|v?pmadd(?:wd|ubsw)|v?psadbw"
    r"|v?ps(?:ll|rl|ra)[wdq]|v?ps(?:ll|rl)dq|vps(?:ll|rl|ra)v[wdq]"
    r"|v?pshuf[bd]|v?pshuf[hl]w|v?palignr|v?punpck[hl](?:bw|wd|dq|qdq)|v?unpck[hl]p[sd]"
    r"|v?shufp[sd]|vpermil(?:p[sd])|vperm2[fi]128|vperm[dq]|vpermp[sd]|vperm[it]2[a-z]+"
    r"|vextract[fi](?:128|32x4|64x2|32x8|64x4)"
    r"|v?pack[su]s(?:wb|dw)|v?pmov[sz]x[bwd][wdq]"
    r"|v?pblendvb|v?pblend[wd]|v?blendv?p[sd]"
    r"|v?cvt(?:ps2pd|pd2ps|dq2pd|dq2ps|ps2dq|pd2dq|tps2dq|tpd2dq)"
    r"|v?pabs[bwdq]|vpopcnt[bwdq]|vpcompress[bwdq]|vpexpand[bwdq]"
    r"|sha256(?:rnds2|msg1|msg2)|sha1(?:rnds4|nexte|msg1|msg2)|v?aes(?:enc|enclast|dec|declast)|v?pclmulqdq"
    r")$"
)
FMA_X86 = re.compile(r"^vf(?:n?m(?:add|sub))")
# Families whose same-source form is a zeroing or all-ones idiom, never work.
_IDIOM_X86 = re.compile(r"^v?(?:p?xor|pandn|andnp[sd]|xorp[sd]|psub[bwdq]|pcmpeq[bwdq]|subp[sd])[dq]?$")

VECTOR_ARM = frozenset(
    """
    fadd fsub fmul fdiv fmax fmin fmaxnm fminnm fabd fabs fneg fsqrt faddp fmaxp fminp
    fmaxv fminv fmaxnmp fminnmp fmaxnmv fminnmv faddv fadda fcmeq fcmgt fcmge fcmlt fcmle
    fcvtl fcvtl2 fcvtn fcvtn2 scvtf ucvtf fcvtzs fcvtzu frintn frintm frintp frintz
    add sub mul mla mls addp addv umaxv uminv smaxv sminv umaxp uminp smaxp sminp
    umax umin smax smin uaddlv saddlv uaddlp saddlp uaddw uaddw2 uaddl uaddl2 usubl usubl2
    uaddv saddv andv orv eorv
    cmeq cmhi cmhs cmgt cmge cmle cmlt cmtst cmpeq cmpne cmphi cmphs cmpgt cmpge cmplo cmpls
    and orr eor bic orn bsl bit bif not mvn eor3 bcax
    tbl tbx ext zip1 zip2 uzp1 uzp2 trn1 trn2 rev16 rev32 rev64
    shrn shrn2 rshrn rshrn2 ushr sshr shl ushl sshl usra ssra sli sri xtn xtn2 uxtl uxtl2 sxtl sxtl2
    cnt abs neg uqadd uqsub sqadd sqsub umull umull2 smull smull2 umlal umlal2 pmull pmull2
    aese aesd aesmc aesimc sha1c sha1m sha1p sha1h sha1su0 sha1su1
    sha256h sha256h2 sha256su0 sha256su1 sha512h sha512h2 sha512su0 sha512su1 rax1 xar match
    """.split()
)
FMA_ARM = frozenset("fmla fmls fmad fmsb fnmla fnmls fnmad fnmsb fmadd fmsub fnmadd fnmsub".split())
_IDIOM_ARM = frozenset({"eor", "sub", "cmeq", "bic"})

_WASM_VECTOR_PREFIX = re.compile(r"^(?:v128|i8x16|i16x8|i32x4|i64x2|f32x4|f64x2)\.")
_WASM_NOT_WORK = re.compile(r"(?:^v128\.(?:load|store|const)|\.load|\.store|\.const$|\.splat$|extract_lane|replace_lane)")
FMA_WASM = re.compile(r"relaxed_n?madd")


def is_vector_work(inst: Instruction, arch: str) -> bool:
    m = inst.mnemonic
    if arch == "x86":
        if not VECTOR_X86.match(m) or not any(_X86_VREG.search(op) for op in inst.operands):
            return False
        regs = [op for op in inst.operands if _X86_VREG.fullmatch(op)]
        if _IDIOM_X86.match(m) and len(regs) >= 2 and len(inst.operands) in (2, 3):
            # AT&T order: 2-operand `op src, dst`; 3-operand VEX `op src2, src1, dst`.
            if inst.operands[0] == inst.operands[1]:
                return False
        return True
    if arch == "aarch64":
        if m not in VECTOR_ARM or not any(_ARM_VREG.search(op) for op in inst.operands):
            return False
        if m in _IDIOM_ARM and len(inst.operands) == 3 and inst.operands[1] == inst.operands[2]:
            return False
        return True
    if not _WASM_VECTOR_PREFIX.match(m) or _WASM_NOT_WORK.search(m):
        return False
    return True


def is_fma(inst: Instruction, arch: str) -> bool:
    if arch == "x86":
        return bool(FMA_X86.match(inst.mnemonic))
    if arch == "aarch64":
        return inst.mnemonic in FMA_ARM
    return bool(FMA_WASM.search(inst.mnemonic))


def is_relaxed(inst: Instruction) -> bool:
    return "relaxed_" in inst.mnemonic


@dataclasses.dataclass(frozen=True)
class Counts:
    vector_ops: int
    fma: int
    relaxed: int
    mnemonics: Counter
    work_mnemonics: frozenset


def count(func: Function, arch: str) -> Counts:
    vector = fma = relaxed = 0
    mnemonics: Counter = Counter()
    work: set[str] = set()
    for inst in func.instructions:
        mnemonics[inst.mnemonic] += 1
        if is_vector_work(inst, arch):
            vector += 1
            work.add(inst.mnemonic)
        if is_fma(inst, arch):
            fma += 1
        if is_relaxed(inst):
            relaxed += 1
    return Counts(vector, fma, relaxed, mnemonics, frozenset(work))


def _is_vector_mnemonic(mnemonic: str, arch: str) -> bool:
    if arch == "x86":
        return bool(VECTOR_X86.match(mnemonic))
    if arch == "aarch64":
        return mnemonic in VECTOR_ARM
    return bool(_WASM_VECTOR_PREFIX.match(mnemonic)) and not _WASM_NOT_WORK.search(mnemonic)


def satisfies(func: Function, spec: str, arch: str) -> bool:
    """Whether ``func`` holds a real instance of ``spec`` (``mnemonic[ operand-substring]``)."""
    mnemonic, _, operand = spec.strip().partition(" ")
    operand = operand.strip()
    vector_class = _is_vector_mnemonic(mnemonic, arch)
    for inst in func.instructions:
        if inst.mnemonic != mnemonic:
            continue
        if vector_class and not is_vector_work(inst, arch):
            continue
        if operand and not any(operand in op for op in inst.operands):
            continue
        return True
    return False


# --------------------------------------------------------------------------- manifest


@dataclasses.dataclass(frozen=True)
class Measure:
    configs: tuple[str, ...]
    crate: str
    symbol: str
    label: str
    min_vector_ops: int
    require_mnemonics: tuple[str, ...]
    max_fma: int | None
    min_fma: int | None
    forbid_relaxed: bool
    single_copy: bool


@dataclasses.dataclass(frozen=True)
class Site:
    id: str
    summary: str
    measures: tuple[Measure, ...]


@dataclasses.dataclass(frozen=True)
class Manifest:
    packages: tuple[str, ...]
    sites: tuple[Site, ...]
    rlib_packages: tuple[str, ...] = ()

    def ids(self) -> set[str]:
        return {site.id for site in self.sites}


_MEASURE_KEYS = {
    "configs", "crate", "symbol", "label", "min_vector_ops", "require_mnemonics", "max_fma",
    "min_fma", "forbid_relaxed", "single_copy",
}
_SITE_KEYS = {"id", "summary", "measure"}


class _Fields:
    """Typed reads of one manifest table.

    Each reader returns a value of exactly its field's type. A missing required key or a
    value of the wrong type is recorded as a named manifest problem and a typed
    placeholder is returned, so validation reports every problem at once and nothing
    downstream ever sees a coerced value. ``bool`` is refused where a count is expected,
    because TOML ``true`` is a Python ``int``.
    """

    def __init__(self, table: dict, where: str, problems: list[str]) -> None:
        self.table = table
        self.where = where
        self.problems = problems

    def _bad(self, key: str, expected: str) -> None:
        value = self.table[key]
        self.problems.append(
            f"{self.where}: `{key}` must be {expected}, not {type(value).__name__} {value!r}"
        )

    def _missing(self, key: str, required: bool) -> bool:
        if key in self.table:
            return False
        if required:
            self.problems.append(f"{self.where}: `{key}` is required")
        return True

    def text(self, key: str, required: bool = False) -> str:
        if self._missing(key, required):
            return ""
        value = self.table[key]
        if not isinstance(value, str) or (required and not value):
            self._bad(key, "a non-empty string" if required else "a string")
            return ""
        return value

    def count(self, key: str, required: bool = False) -> int | None:
        if self._missing(key, required):
            return None
        value = self.table[key]
        if isinstance(value, bool) or not isinstance(value, int) or value < 0:
            self._bad(key, "a non-negative integer")
            return None
        return value

    def flag(self, key: str, required: bool = False) -> bool:
        if self._missing(key, required):
            return False
        value = self.table[key]
        if not isinstance(value, bool):
            self._bad(key, "a boolean (true or false)")
            return False
        return value

    def str_list(self, key: str, required: bool = False, non_empty: bool = False) -> tuple[str, ...]:
        if self._missing(key, required):
            return ()
        value = self.table[key]
        if (
            not isinstance(value, list)
            or not all(isinstance(v, str) and v.strip() for v in value)
            or (non_empty and not value)
        ):
            self._bad(key, "a non-empty list of strings" if non_empty else "a list of non-empty strings")
            return ()
        return tuple(value)


def load_manifest(data: dict) -> Manifest:
    """Validate and load a parsed manifest. Every omission a reader could misread is refused."""
    problems: list[str] = []
    unknown_top = set(data) - {"build", "site"}
    if unknown_top:
        problems.append(f"unknown top-level keys {sorted(unknown_top)}")
    build = data.get("build", {})
    for key in set(build) - {"packages", "rlib_packages"}:
        problems.append(f"`[build]`: unknown key `{key}`")
    packages = build.get("packages")
    if not isinstance(packages, list) or not packages or not all(isinstance(p, str) for p in packages):
        problems.append("`[build] packages` must be a non-empty list of package names")
        packages = []
    rlib_packages = build.get("rlib_packages", [])
    if not isinstance(rlib_packages, list) or not all(isinstance(p, str) and p for p in rlib_packages):
        problems.append("`[build] rlib_packages` must be a list of package names")
        rlib_packages = []
    for package in sorted(set(rlib_packages) & set(packages)):
        problems.append(f"`{package}` is in both `packages` and `rlib_packages`; a package is built one way")
    sites: list[Site] = []
    seen: set[str] = set()
    for raw in data.get("site", []):
        sid = raw.get("id")
        if not isinstance(sid, str) or not re.fullmatch(r"[a-z0-9][a-z0-9._-]*", sid or ""):
            problems.append(f"site id {sid!r} is not a lowercase identifier")
            continue
        if sid in seen:
            problems.append(f"site `{sid}` is declared twice")
        seen.add(sid)
        for key in set(raw) - _SITE_KEYS:
            problems.append(f"site `{sid}`: unknown key `{key}`")
        if not isinstance(raw.get("summary"), str) or not raw.get("summary"):
            problems.append(f"site `{sid}`: `summary` is required")
        measures: list[Measure] = []
        covered: Counter = Counter()
        raw_measures = raw.get("measure", [])
        if not isinstance(raw_measures, list):
            problems.append(f"site `{sid}`: `measure` must be an array of tables")
            raw_measures = []
        for m in raw_measures:
            if not isinstance(m, dict):
                problems.append(f"site `{sid}`: every `[[site.measure]]` must be a table")
                continue
            fields = _Fields(m, f"site `{sid}` measure {m.get('label') or m.get('symbol')!r}", problems)
            for key in set(m) - _MEASURE_KEYS:
                problems.append(f"{fields.where}: unknown key `{key}`")
            configs = fields.str_list("configs", required=True, non_empty=True)
            for c in configs:
                if c not in CONFIG_BY_NAME:
                    problems.append(f"{fields.where}: unknown configuration `{c}` (known: {', '.join(CONFIG_NAMES)})")
                covered[c] += 1
            if ("max_fma" in m) == ("min_fma" in m):
                problems.append(f"{fields.where}: exactly one of `max_fma` (exact) or `min_fma` (fast) is required")
            measures.append(
                Measure(
                    configs=configs,
                    crate=fields.text("crate", required=True).replace("-", "_"),
                    symbol=fields.text("symbol", required=True),
                    label=fields.text("label"),
                    min_vector_ops=fields.count("min_vector_ops", required=True) or 0,
                    require_mnemonics=fields.str_list("require_mnemonics"),
                    max_fma=fields.count("max_fma"),
                    min_fma=fields.count("min_fma"),
                    forbid_relaxed=fields.flag("forbid_relaxed", required=True),
                    single_copy=fields.flag("single_copy"),
                )
            )
        missing = [c for c in CONFIG_NAMES if covered[c] == 0]
        if not measures:
            problems.append(f"site `{sid}` has no `[[site.measure]]`")
        elif missing:
            problems.append(
                f"site `{sid}` measures no symbol on {', '.join(missing)}; every site records its "
                f"counts on all {len(CONFIG_NAMES)} configurations (a floor of 0 is allowed, a gap is not)"
            )
        labels = Counter((c, mm.label) for mm in measures for c in mm.configs)
        for (c, label), n in labels.items():
            if n > 1:
                problems.append(
                    f"site `{sid}`: {n} measures on `{c}` share the label {label!r}; give each a distinct `label`"
                )
        sites.append(Site(sid, str(raw.get("summary", "")), tuple(measures)))
    if problems:
        raise GateError("manifest is invalid:\n  " + "\n  ".join(problems))
    return Manifest(tuple(packages), tuple(sites), tuple(rlib_packages))


# --------------------------------------------------------------------------- evaluation


@dataclasses.dataclass(frozen=True)
class Result:
    """What one measure found on one configuration."""

    site: str
    config: str
    measure: Measure
    copies: int
    vector_ops: int  # the minimum over matched copies
    fma: int  # the maximum over matched copies
    relaxed: int  # the maximum over matched copies
    problems: tuple[str, ...]


def matches(func: Function, measure: Measure) -> bool:
    return func.crate == measure.crate and measure.symbol in func.path


def _is_nested_closure(func: Function, symbol: str) -> bool:
    """Whether `func` is a closure nested inside the function `symbol` names.

    A `|_| unreachable!(..)` handed to `unwrap_or_else`, or any other closure a
    measured function defines, is its own out-of-line symbol whose path extends the
    parent's with `::{closure#N}`. It carries none of the parent's evidence, so when
    the parent itself is matched the closure is not a second copy of the site.
    """
    at = func.path.find(symbol)
    return at >= 0 and "{closure" in func.path[at + len(symbol):] and "{closure" not in symbol


def measured(functions: list[Function], measure: Measure) -> list[Function]:
    """The functions a measure judges: its matches, less closures nested in a match.

    Closures are dropped only when a non-closure match remains, so a site whose
    evidence genuinely lives in a closure is still measured there, never reported as
    absent.
    """
    matched = [f for f in functions if matches(f, measure)]
    parents = [f for f in matched if not _is_nested_closure(f, measure.symbol)]
    return parents or matched


def evaluate(site: Site, measure: Measure, config: Config, functions: list[Function]) -> Result:
    matched = measured(functions, measure)
    problems: list[str] = []
    where = f"`{site.id}`{' [' + measure.label + ']' if measure.label else ''} on {config.name}"
    if not matched:
        problems.append(
            f"{where}: no function of crate `{measure.crate}` has `{measure.symbol}` in its path -- "
            f"the symbol was renamed or inlined away, and the evidence with it"
        )
        return Result(site.id, config.name, measure, 0, 0, 0, 0, tuple(problems))
    counted = [(f, count(f, config.arch)) for f in matched]
    if measure.single_copy:
        by_graph = Counter(f.graph for f in matched)
        for graph, copies in sorted(by_graph.items()):
            if copies != 1:
                units = ", ".join(sorted({f"{f.path} ({Path(f.unit).name})" for f in matched if f.graph == graph}))
                problems.append(
                    f"{where}: `single_copy` requires exactly one out-of-line copy per build graph, "
                    f"found {copies} in graph {graph}: {units}"
                )
    for func, c in counted:
        name = f"{where}: `{func.path}`"
        if c.vector_ops < measure.min_vector_ops:
            problems.append(f"{name} has {c.vector_ops} vector ops, below the floor of {measure.min_vector_ops}")
        for spec in measure.require_mnemonics:
            if not satisfies(func, spec, config.arch):
                problems.append(f"{name} lacks the required `{spec}`")
        if measure.max_fma is not None and c.fma > measure.max_fma:
            problems.append(f"{name} contains {c.fma} fused multiply-add(s); the exact contract allows {measure.max_fma}")
        if measure.min_fma is not None and c.fma < measure.min_fma:
            problems.append(f"{name} contains {c.fma} fused multiply-add(s), below the fast floor of {measure.min_fma}")
        if measure.forbid_relaxed and c.relaxed:
            problems.append(f"{name} contains {c.relaxed} `relaxed_` instruction(s), which this site forbids")
    return Result(
        site.id,
        config.name,
        measure,
        len(matched),
        min(c.vector_ops for _f, c in counted),
        max(c.fma for _f, c in counted),
        max(c.relaxed for _f, c in counted),
        tuple(problems),
    )


def evaluate_config(manifest: Manifest, config: Config, functions: list[Function]) -> list[Result]:
    return [
        evaluate(site, measure, config, functions)
        for site in manifest.sites
        for measure in site.measures
        if config.name in measure.configs
    ]


def render_cell(results: list[Result]) -> str:
    """The generated document cell for one site on one configuration."""
    parts = []
    for r in results:
        req = "·req✓" if r.measure.require_mnemonics and not r.problems else ""
        body = f"v{r.vector_ops}·f{r.fma}·r{r.relaxed}{req}"
        parts.append(f"{r.measure.label} {body}" if r.measure.label else body)
    return "<br>".join(parts) if parts else "—"


# --------------------------------------------------------------------------- building


def refuse_overriding_env(env: dict[str, str]) -> None:
    """Refuse an environment in which cargo would drop the configuration's flags."""
    for var in ("RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"):
        if env.get(var):
            raise GateError(
                f"{var} is set. Cargo gives it precedence over CARGO_TARGET_<TRIPLE>_RUSTFLAGS, so "
                f"the measurement would silently lose --emit=asm and the configuration's CPU flags. "
                f"Unset it for this gate."
            )


def _env_triple(triple: str) -> str:
    return triple.upper().replace("-", "_")


def host_triple() -> str:
    out = subprocess.run(["rustc", "-vV"], capture_output=True, text=True, check=False)
    if out.returncode != 0:
        raise GateError(f"`rustc -vV` failed: {out.stderr.strip()}")
    for line in out.stdout.splitlines():
        if line.startswith("host: "):
            return line.split(": ", 1)[1].strip()
    raise GateError("`rustc -vV` reported no host triple")


def missing_targets(configs, sysroot: Path, is_dir=Path.is_dir) -> list[str]:
    """The target triples whose standard library is absent from ``sysroot``."""
    return sorted({c.triple for c in configs if not is_dir(sysroot / "lib" / "rustlib" / c.triple / "lib")})


def require_targets(configs: tuple[Config, ...]) -> None:
    """Every target's standard library must be installed. Missing is a failure, never a skip."""
    out = subprocess.run(["rustc", "--print", "sysroot"], capture_output=True, text=True, check=False)
    if out.returncode != 0:
        raise GateError(f"`rustc --print sysroot` failed: {out.stderr.strip()}")
    missing = missing_targets(configs, Path(out.stdout.strip()))
    if missing:
        raise GateError(
            "the standard library is not installed for: " + ", ".join(missing)
            + f" (`rustup target add {' '.join(missing)}`); every configuration is measured, none is skipped"
        )


HOST_CPU_FLAGS = ("-march", "-mtune", "-mcpu")


def strip_host_cpu_flags(value: str) -> tuple[str, list[str]]:
    """``value`` without its host-CPU tokens, and the tokens removed, in order.

    A host-CPU token is ``-march=native``, ``-mtune=native`` or ``-mcpu=native``, or the
    same flag split from ``native`` across two tokens (``-march native``). Every other
    token is kept in its original order. A value with nothing to remove is returned
    byte-for-byte unchanged, whitespace included.
    """
    tokens = value.split()
    kept: list[str] = []
    removed: list[str] = []
    i = 0
    while i < len(tokens):
        token = tokens[i]
        if token in {f"{flag}=native" for flag in HOST_CPU_FLAGS}:
            removed.append(token)
            i += 1
        elif token in HOST_CPU_FLAGS and i + 1 < len(tokens) and tokens[i + 1] == "native":
            removed.append(f"{token} native")
            i += 2
        else:
            kept.append(token)
            i += 1
    return (" ".join(kept), removed) if removed else (value, [])


def config_env(config: Config, base: dict[str, str], host: str, which=shutil.which, log=print) -> dict[str, str]:
    """The child environment for one configuration's build: a copy, never ``os.environ``."""
    env = dict(base)
    env[f"CARGO_TARGET_{_env_triple(config.triple)}_RUSTFLAGS"] = config.rustflags()
    if config.triple != host:
        for var in ("CFLAGS", "CXXFLAGS"):
            if var not in env:
                continue
            stripped, removed = strip_host_cpu_flags(env[var])
            if removed:
                env[var] = stripped
                log(
                    f"simd-asm: {config.triple}: removed host-CPU flag(s) {' '.join(removed)} "
                    f"from {var} (cross build)"
                )
        suffix = config.triple.replace("-", "_")
        for var, tool in (("CC", "clang"), ("AR", "llvm-ar")):
            key = f"{var}_{suffix}"
            if env.get(key) or env.get(f"{var}_{config.triple}"):
                continue
            if which(tool) is None:
                raise GateError(
                    f"`{tool}` is not on PATH. Build scripts in the graph compile C for "
                    f"{config.triple}, and this gate names `{tool}` for that (or set {key})."
                )
            env[key] = tool
    return env


_RUNNING = re.compile(r"^\s*Running `(.*)`\s*$")


def verify_command_lines(stderr: str, config: Config) -> list[str]:
    """Every rustc invocation for a target unit carries the configuration's flags, and only those."""
    problems = []
    for line in stderr.splitlines():
        match = _RUNNING.match(line)
        if not match:
            continue
        try:
            words = shlex.split(match.group(1))
        except ValueError:
            continue
        if "--crate-name" not in words or "--target" not in words:
            continue
        if words[words.index("--target") + 1] != config.triple:
            continue
        crate = words[words.index("--crate-name") + 1]
        emits: list[str] = []
        denies_warnings = any(
            (w == "-D" and i + 1 < len(words) and words[i + 1] == "warnings") or w in ("-Dwarnings", "--deny=warnings")
            for i, w in enumerate(words)
        )
        cpus: set[str] = set()
        features: set[str] = set()
        for i, w in enumerate(words):
            if w.startswith("--emit="):
                emits.extend(w.split("=", 1)[1].split(","))
            elif w == "--emit" and i + 1 < len(words):
                emits.extend(words[i + 1].split(","))
            value = None
            if w == "-C" and i + 1 < len(words):
                value = words[i + 1]
            elif w.startswith("-C") and len(w) > 2:
                value = w[2:]
            if value is not None:
                if value.startswith("target-cpu="):
                    cpus.add(value.split("=", 1)[1])
                elif value.startswith("target-feature="):
                    features.update(v for v in value.split("=", 1)[1].split(",") if v)
        if "asm" not in emits:
            problems.append(f"{config.name}: rustc for `{crate}` was not given --emit=asm")
        if not denies_warnings:
            problems.append(f"{config.name}: rustc for `{crate}` was not given -D warnings")
        if cpus != config.target_cpus():
            problems.append(f"{config.name}: rustc for `{crate}` got target-cpu {sorted(cpus)}, expected {sorted(config.target_cpus())}")
        if features != config.target_features():
            problems.append(f"{config.name}: rustc for `{crate}` got target-feature {sorted(features)}, expected {sorted(config.target_features())}")
    return problems


def asm_paths(stdout: str, config: Config, is_file=Path.is_file) -> list[Path]:
    """The ``.s`` of every target library unit, located from cargo's artifact messages.

    rustc writes the ``.s`` beside the unit's own rlib/rmeta. For a plain library that
    file carries cargo's hash suffix (``libname-<16 hex>.rmeta``). A package that is also
    a ``cdylib`` cannot take the suffix, so its unit files are unhashed, in the unit's
    own output directory, next to the unhashed copy cargo uplifts. So the hashed files
    are tried first and every other rlib/rmeta after them, and the first with a ``.s``
    beside it is the unit's asm.
    """
    paths: list[Path] = []
    problems: list[str] = []
    for line in stdout.splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") != "compiler-artifact":
            continue
        kinds = set(message["target"]["kind"])
        if not kinds & {"lib", "rlib"}:
            continue
        files = [Path(f) for f in message.get("filenames", [])]
        if not any(f"/{config.triple}/" in str(f) for f in files):
            continue
        libs = [f for f in files if f.suffix in (".rmeta", ".rlib")]
        if not libs:
            problems.append(f"{message['package_id']}: no rlib/rmeta among {files}")
            continue
        hashed = [f for f in libs if re.search(r"-[0-9a-f]{16}$", f.stem)]
        candidates = [f.with_name(f.stem.removeprefix("lib") + ".s") for f in (*hashed, *(f for f in libs if f not in hashed))]
        asm = next((a for a in candidates if is_file(a)), None)
        if asm is None:
            problems.append(
                f"`{message['target']['name']}` has no .s beside any of {[f.name for f in libs]}; "
                f"the build did not emit asm for it (a cache that restores outputs without the .s, "
                f"or a flag layer that dropped --emit=asm)"
            )
            continue
        paths.append(asm)
    if problems:
        raise GateError(f"{config.name}: " + "\n  ".join(problems))
    if not paths:
        raise GateError(f"{config.name}: cargo reported no library artifact for {config.triple}")
    return paths


def failure_tail(stderr: str, lines: int = 40) -> str:
    """The last ``lines`` of a failed build's stderr, without cargo's progress lines.

    ``-v`` prints every rustc command line and a ``Fresh`` line per unit, so a plain
    tail is the failing command and none of the reason for it.
    """
    kept = [l for l in stderr.splitlines() if not re.match(r"^\s*(?:Running `|Fresh |Compiling |Dirty |Checking )", l)]
    return "\n".join(kept[-lines:])


def build_commands(config: Config, packages: tuple[str, ...], rlib_packages: tuple[str, ...]) -> list[list[str]]:
    """The cargo invocations for one configuration: one build, then one per rlib-only package."""
    common = [
        "-v", "--message-format=json-render-diagnostics", "--release", "--locked",
        "--lib", "--target", config.triple,
        "--config", "profile.release.lto=false", "--config", "profile.release.codegen-units=1",
    ]
    cmds = [["cargo", "build", *common, *(arg for p in packages for arg in ("-p", p))]]
    cmds += [["cargo", "rustc", *common, "-p", p, "--crate-type", "rlib"] for p in rlib_packages]
    return cmds


def build_config(config: Config, manifest: Manifest, scratch: Path, host: str, chooser) -> list[Function]:
    """Build one configuration and return the emitted functions ``chooser`` selects."""
    print(f"== {config.name}: cargo build --target {config.triple} ({config.rustflags()})", flush=True)
    env = config_env(config, dict(os.environ), host, log=lambda line: print(line, flush=True))
    env["CARGO_TARGET_DIR"] = str(scratch)
    paths: dict[Path, int] = {}
    for graph, cmd in enumerate(build_commands(config, manifest.packages, manifest.rlib_packages)):
        proc = subprocess.run(cmd, cwd=REPO_ROOT, env=env, capture_output=True, text=True, check=False)
        if proc.returncode != 0:
            raise GateError(f"{config.name}: `{' '.join(cmd[:2])}` failed (exit {proc.returncode}):\n{failure_tail(proc.stderr)}")
        problems = verify_command_lines(proc.stderr, config)
        if problems:
            raise GateError("the build was not the configuration it claims:\n  " + "\n  ".join(problems))
        # A unit an earlier graph already emitted (same crate, same hash) is that graph's.
        for p in asm_paths(proc.stdout, config):
            paths.setdefault(p, graph)
    units = ((str(p), p.read_text(encoding="utf-8", errors="replace"), graph) for p, graph in paths.items())
    return collect_functions(units, config.arch, chooser.keep, chooser.select)


class Scratch:
    """A scratch build directory from ``scripts/build-scratch.sh``, removed on every exit path."""

    def __enter__(self) -> Path:
        out = subprocess.run(
            ["bash", "-c", f'source "{BUILD_SCRATCH}" && build_scratch_dir simd-asm'],
            capture_output=True, text=True, check=False,
        )
        if out.returncode != 0 or not out.stdout.strip():
            raise GateError(f"build_scratch_dir failed: {out.stderr.strip()}")
        self.path = Path(out.stdout.strip())
        self._previous = signal.signal(signal.SIGTERM, self._terminate)
        return self.path

    @staticmethod
    def _terminate(_signum, _frame):
        raise SystemExit(143)

    def __exit__(self, *_exc) -> None:
        signal.signal(signal.SIGTERM, self._previous)
        shutil.rmtree(self.path, ignore_errors=True)


@dataclasses.dataclass(frozen=True)
class Chooser:
    """Which emitted functions one configuration's reader parses (``select``) and keeps."""

    select: object  # Callable[[Demangled], bool] | None
    keep: object  # Callable[[Function], bool] | None


def measure_all(manifest: Manifest, configs: tuple[Config, ...], keep_for) -> dict[str, list[Function]]:
    """Build every configuration; ``keep_for(config)`` is its ``Chooser``."""
    refuse_overriding_env(dict(os.environ))
    require_targets(configs)
    host = host_triple()
    out: dict[str, list[Function]] = {}
    with Scratch() as scratch:
        for config in configs:
            out[config.name] = build_config(config, manifest, scratch, host, keep_for(config))
    return out


def manifest_keep(manifest: Manifest):
    """For each configuration, parse exactly the functions some measure on it can match."""
    def keep_for(config: Config) -> Chooser:
        measures = [m for site in manifest.sites for m in site.measures if config.name in m.configs]
        return Chooser(
            select=lambda d: any(d.crate == m.crate and m.symbol in d.path for m in measures),
            keep=None,
        )
    return keep_for


# --------------------------------------------------------------------------- static scans

IDENTITY_TOKEN = re.compile(r"\balgebraic_(?:add|sub|mul|div|rem)\b|\bReassociated\b")
IDENTITY_ALLOWED = (
    "crates/rdf-core/src/distance/",
    "crates/sparql-eval/src/knn/",
    "crates/hnsw/src/",
    "crates/rdf-core/tests/",
    "crates/rdf-core/benches/",
    "crates/sparql-eval/tests/",
    "crates/sparql-eval/benches/",
    "crates/hnsw/tests/",
    "crates/hnsw/benches/",
)


def identity_scan(files: dict[str, str]) -> list[str]:
    """Fast float arithmetic outside the three ranked-distance homes is refused."""
    problems = []
    for rel, text in sorted(files.items()):
        if rel.startswith(IDENTITY_ALLOWED):
            continue
        for lineno, line in enumerate(text.splitlines(), 1):
            for m in IDENTITY_TOKEN.finditer(line):
                problems.append(
                    f"{rel}:{lineno}: `{m.group(0)}` outside the ranked-distance modules; identity, "
                    f"canonical and serialization paths are exact by contract"
                )
    return problems


def rust_sources() -> dict[str, str]:
    out = subprocess.run(
        ["git", "ls-files", "-z", "--", "crates", "bindings"], cwd=REPO_ROOT,
        capture_output=True, check=False,
    )
    if out.returncode != 0:
        raise GateError(f"`git ls-files` failed: {out.stderr.decode(errors='replace').strip()}")
    files = {}
    for rel in out.stdout.decode().split("\0"):
        if rel.endswith(".rs") and (REPO_ROOT / rel).is_file():
            files[rel] = (REPO_ROOT / rel).read_text(encoding="utf-8", errors="replace")
    return files


MULTIPLIER = re.compile(r"\d+(?:\.\d+)?\s*(?:x|×|times)(?!\w)|\d+(?:\.\d+)?\s*%")
SPEED_WORD = re.compile(r"faster|speedup|speed-up|quicker|slower", re.IGNORECASE)


def multiplier_scan(name: str, text: str) -> list[str]:
    """A multiplier or percentage within 40 characters of a speed word is a claim, not evidence."""
    problems = []
    for m in MULTIPLIER.finditer(text):
        window = text[max(0, m.start() - 40): m.end() + 40]
        if SPEED_WORD.search(window):
            line = text.count("\n", 0, m.start()) + 1
            problems.append(
                f"{name}:{line}: `{m.group(0).strip()}` beside a speed word -- state the asm "
                f"evidence (instruction counts), not a multiplier"
            )
    return problems


def unreleased_changelog(text: str) -> str:
    head, sep, rest = text.partition("## [Unreleased]")
    if not sep:
        raise GateError("CHANGELOG.md has no `## [Unreleased]` section to scan")
    return re.split(r"\n## \[", rest, maxsplit=1)[0]


# --------------------------------------------------------------------------- document


SITES_BEGIN, SITES_END = "<!-- simd-asm:sites:begin -->", "<!-- simd-asm:sites:end -->"
BENCHES_BEGIN, BENCHES_END = "<!-- simd-asm:benches:begin -->", "<!-- simd-asm:benches:end -->"
SITE_COLUMNS = ("id", "crate", "fn", "covers", "verdict", "reason")
VERDICTS = {"covered", "rewrite", "leave", "regime-3", "dispatch", "fast-variant"}
EMPTY_CELL = {"", "—", "-", "n/a"}

# The minimum-coverage roster: each must be in the `covers` cell of a row that names a
# function with a manifest entry.
ROSTER = (
    "intern", "pack_bits", "pack_index_compare", "pack_query", "ir_layout",
    "bgp-join-probe", "solution_row", "regex_eval", "paged_cross_page_bgp", "datalog-seminaive",
    "entail-chase", "entail-classify",
    "shapes-pattern_lookup", "shapes-pattern_validate", "shex-pattern_validate",
    "text-search", "retrieval-fusion",
    "columnar-codec", "geo-relate",
    "rdf-line-split", "rdf-escape", "sparql-tokenizer", "iri-parse", "xsd-lexical", "json-ordered",
)


def _cells(row: str) -> list[str]:
    body = row.strip()
    if body.startswith("|"):
        body = body[1:]
    if body.endswith("|") and not body.endswith("\\|"):
        body = body[:-1]
    return [c.strip() for c in re.split(r"(?<!\\)\|", body)]


def _plain(cell: str) -> str:
    return cell.strip().strip("`").strip()


def _region(text: str, begin: str, end: str) -> tuple[int, int]:
    start, stop = text.find(begin), text.find(end)
    if start < 0 or stop < 0 or stop < start:
        raise GateError(f"the document lacks the `{begin}` ... `{end}` region")
    return start + len(begin), stop


def parse_table(text: str, begin: str, end: str) -> tuple[list[str], list[list[str]], tuple[int, int]]:
    lo, hi = _region(text, begin, end)
    rows = [line for line in text[lo:hi].splitlines() if line.strip().startswith("|")]
    if len(rows) < 2:
        raise GateError(f"the `{begin}` region holds no table")
    header = [_plain(c).lower() for c in _cells(rows[0])]
    body = [_cells(r) for r in rows[2:]]
    for r in body:
        if len(r) != len(header):
            raise GateError(f"a row in `{begin}` has {len(r)} cells, the header has {len(header)}: {r}")
    return header, body, (lo, hi)


@dataclasses.dataclass(frozen=True)
class DocWorld:
    """Everything the document checks read besides the document: pure data, so fixtures can stand in."""

    members: tuple[str, ...]  # workspace package names
    member_dirs: dict  # package name -> relative crate dir
    bench_files: tuple[str, ...]  # crates/<crate>/benches/<file>.rs
    bench_dirs: frozenset  # relative crate dirs that have a benches/ directory


def workspace_world() -> DocWorld:
    root = tomllib.loads((REPO_ROOT / "Cargo.toml").read_text(encoding="utf-8"))
    members, dirs = [], {}
    for rel in root["workspace"]["members"]:
        name = tomllib.loads((REPO_ROOT / rel / "Cargo.toml").read_text(encoding="utf-8"))["package"]["name"]
        members.append(name)
        dirs[name] = rel
    benches = sorted(str(p.relative_to(REPO_ROOT)) for p in REPO_ROOT.glob("crates/*/benches/*.rs"))
    bench_dirs = frozenset(rel for rel in dirs.values() if (REPO_ROOT / rel / "benches").is_dir())
    return DocWorld(tuple(members), dirs, tuple(benches), bench_dirs)


def doc_checks(doc: str, manifest: Manifest, cells: dict, world: DocWorld) -> list[str]:
    """Parity and coverage between the document, the manifest and the measured cells.

    ``cells`` maps ``(site id, config name)`` to the generated cell text.
    """
    problems: list[str] = []
    header, rows, _span = parse_table(doc, SITES_BEGIN, SITES_END)
    for col in (*SITE_COLUMNS, *CONFIG_NAMES):
        if col not in header:
            problems.append(f"the site table has no `{col}` column")
    if problems:
        return problems
    col = {name: header.index(name) for name in (*SITE_COLUMNS, *CONFIG_NAMES)}
    ids = manifest.ids()
    row_ids: dict[str, list[str]] = {}
    for row in rows:
        rid = _plain(row[col["id"]])
        if rid in row_ids:
            problems.append(f"site row `{rid}` appears twice")
        row_ids[rid] = row
        crate = _plain(row[col["crate"]])
        verdict = _plain(row[col["verdict"]]).lower()
        names_fn = _plain(row[col["fn"]]) not in EMPTY_CELL
        if verdict not in VERDICTS:
            problems.append(f"row `{rid}`: verdict `{verdict}` is not one of {sorted(VERDICTS)}")
        if crate not in world.members:
            problems.append(f"row `{rid}`: `{crate}` is not a workspace member")
        if names_fn:
            if rid not in ids:
                problems.append(
                    f"row `{rid}` names a function but has no manifest entry; a `{verdict}` verdict "
                    f"still records its measured counts"
                )
        else:
            if rid in ids:
                problems.append(f"row `{rid}` has a manifest entry but names no function")
            crate_dir = world.member_dirs.get(crate, "")
            cite = f"{crate_dir}/benches/"
            if verdict != "leave":
                problems.append(f"crate-level row `{rid}` must be `leave`, not `{verdict}`")
            if cite not in row[col["reason"]]:
                problems.append(f"crate-level row `{rid}` must cite that `{cite}` does not exist")
            if crate_dir in world.bench_dirs:
                problems.append(
                    f"crate-level row `{rid}`: `{cite}` exists, so `{crate}` has a hot path to "
                    f"measure -- give it a function row with a manifest entry"
                )
        for name in CONFIG_NAMES:
            expected = cells.get((rid, name), "—")
            if row[col[name]] != expected:
                problems.append(
                    f"row `{rid}` `{name}` reads {row[col[name]]!r}, measured {expected!r} "
                    f"(run `python3 scripts/check-simd-asm.py --write-doc`)"
                )
    for sid in sorted(ids - set(row_ids)):
        problems.append(f"manifest site `{sid}` has no row in the site table")
    # (a) crate coverage
    covered_crates = {_plain(r[col["crate"]]) for r in rows}
    for member in world.members:
        if member not in covered_crates:
            problems.append(f"workspace member `{member}` has no site row")
    # (b) bench coverage
    bheader, brows, _ = parse_table(doc, BENCHES_BEGIN, BENCHES_END)
    if "bench" not in bheader or "sites" not in bheader:
        problems.append("the bench table needs `bench` and `sites` columns")
        return problems
    bcol, scol = bheader.index("bench"), bheader.index("sites")
    mapped: dict[str, list[str]] = {}
    for row in brows:
        bench = _plain(row[bcol])
        mapped[bench] = [_plain(s) for s in row[scol].split(",") if _plain(s) not in EMPTY_CELL]
    for bench in world.bench_files:
        if bench not in mapped:
            problems.append(f"bench `{bench}` has no row in the bench table")
        elif not mapped[bench]:
            problems.append(f"bench `{bench}` maps to no site id")
    for bench, sids in sorted(mapped.items()):
        if bench not in world.bench_files:
            problems.append(f"bench row `{bench}` names no bench file that exists")
        for sid in sids:
            if sid not in row_ids:
                problems.append(f"bench `{bench}` maps to unknown site id `{sid}`")
    # (c) roster
    for key in ROSTER:
        if not any(
            key in [_plain(c) for c in row[col["covers"]].split(",")]
            and _plain(row[col["fn"]]) not in EMPTY_CELL
            and rid in ids
            for rid, row in row_ids.items()
        ):
            problems.append(f"roster site `{key}` is not covered by a function row with a manifest entry")
    problems.extend(multiplier_scan("docs/design/purrdf-simd.md", doc))
    return problems


def require_doc(path: Path) -> None:
    """``--doc`` on a missing document is a failure, never a skip."""
    if not path.is_file():
        shown = path.relative_to(REPO_ROOT) if path.is_relative_to(REPO_ROOT) else path
        raise GateError(f"{shown} does not exist; `--doc` checks it, it does not skip it")


def write_doc(doc: str, cells: dict) -> str:
    header, rows, (lo, hi) = parse_table(doc, SITES_BEGIN, SITES_END)
    idx = header.index("id")
    region = doc[lo:hi]
    out_lines = []
    table_seen = 0
    for line in region.splitlines(keepends=True):
        if not line.strip().startswith("|"):
            out_lines.append(line)
            continue
        table_seen += 1
        if table_seen <= 2:
            out_lines.append(line)
            continue
        row = _cells(line)
        rid = _plain(row[idx])
        for name in CONFIG_NAMES:
            row[header.index(name)] = cells.get((rid, name), "—")
        newline = "\n" if line.endswith("\n") else ""
        out_lines.append("| " + " | ".join(row) + " |" + newline)
    return doc[:lo] + "".join(out_lines) + doc[hi:]


# --------------------------------------------------------------------------- driver


def run(args: argparse.Namespace) -> int:
    manifest = load_manifest(tomllib.loads(MANIFEST.read_text(encoding="utf-8")))
    problems: list[str] = []
    problems += identity_scan(rust_sources())
    problems += multiplier_scan("CHANGELOG.md [Unreleased]", unreleased_changelog(CHANGELOG.read_text(encoding="utf-8")))
    if args.doc or args.write_doc:
        require_doc(DOC)

    if args.probe:
        wanted = tuple(CONFIG_BY_NAME[c] for c in (args.config or CONFIG_NAMES))

        def probe_hit(f: Function) -> bool:
            pattern = re.compile(args.probe)
            hit = (
                any(pattern.search(op) for i in f.instructions for op in i.operands)
                if args.probe_operands
                else bool(pattern.search(f.path))
            )
            return hit and (not args.crate or f.crate == args.crate.replace("-", "_"))

        crate = args.crate.replace("-", "_") if args.crate else None
        by_name = None if args.probe_operands else (
            lambda d: bool(re.search(args.probe, d.path)) and (crate is None or d.crate == crate)
        )
        functions = measure_all(manifest, wanted, lambda _config: Chooser(by_name, probe_hit))
        for config in wanted:
            for f in functions[config.name]:
                c = count(f, config.arch)
                top = ", ".join(
                    f"{m}×{n}" for m, n in sorted(c.mnemonics.items())
                    if _is_vector_mnemonic(m, config.arch) or is_fma(Instruction(m, ()), config.arch)
                )
                print(f"{config.name:20} {f.crate}  {f.path}\n{'':20} v{c.vector_ops} f{c.fma} r{c.relaxed}  [{Path(f.unit).name}]  {top}")
                if args.dump:
                    for inst in f.instructions:
                        print(f"{'':24}{inst.mnemonic} {', '.join(inst.operands)}")
        return 0

    functions = measure_all(manifest, CONFIGS, manifest_keep(manifest))
    results: list[Result] = []
    for config in CONFIGS:
        results += evaluate_config(manifest, config, functions[config.name])
    for r in results:
        problems.extend(r.problems)
    cells = {}
    for site in manifest.sites:
        for config in CONFIGS:
            cells[(site.id, config.name)] = render_cell([r for r in results if r.site == site.id and r.config == config.name])

    width = max(len(s.id) for s in manifest.sites)
    print(f"\n{'site'.ljust(width)}  " + "  ".join(CONFIG_NAMES))
    for site in manifest.sites:
        print(f"{site.id.ljust(width)}  " + "  ".join(cells[(site.id, c)].replace("<br>", " / ") for c in CONFIG_NAMES))

    if args.write_doc:
        text = DOC.read_text(encoding="utf-8")
        updated = write_doc(text, cells)
        if updated != text:
            DOC.write_text(updated, encoding="utf-8")
            print(f"wrote the measured cells into {DOC.relative_to(REPO_ROOT)}")
    if args.doc or args.write_doc:
        problems += doc_checks(DOC.read_text(encoding="utf-8"), manifest, cells, workspace_world())
    else:
        print("document parity and coverage: not requested (`--doc`)")

    if problems:
        print(f"\nFAIL: {len(problems)} problem(s):", file=sys.stderr)
        for p in problems:
            print(f"  - {p}", file=sys.stderr)
        return 1
    print(f"\nOK: {len(manifest.sites)} site(s) measured on {len(CONFIGS)} configurations")
    return 0


# --------------------------------------------------------------------------- self-test


_X86_SCALAR = """\
	.section	.text.scalar,"ax",@progbits
	.type	_ZN4demo6kernel3dot17h0123456789abcdefE,@function
_ZN4demo6kernel3dot17h0123456789abcdefE:
	xorps	%xmm0, %xmm0
.LBB0_1:
	movaps	(%rdi), %xmm1
	addsd	%xmm1, %xmm0
	jne	.LBB0_1
	retq
.Lfunc_end0:
	.size	_ZN4demo6kernel3dot17h0123456789abcdefE, .Lfunc_end0-_ZN4demo6kernel3dot17h0123456789abcdefE
"""
_X86_PACKED = _X86_SCALAR.replace("addsd\t%xmm1, %xmm0", "addpd\t%xmm1, %xmm0\n\tmulpd\t%xmm2, %xmm1")
_X86_FMA = _X86_PACKED.replace("mulpd\t%xmm2, %xmm1", "vfmadd231pd\t%ymm2, %ymm1, %ymm0")
_ARM_FMA = """\
	.type	_RNvNtCs1234_4demo6kernel3dot,@function
_RNvNtCs1234_4demo6kernel3dot:
	movi	v0.2d, #0000000000000000
	fmla	v0.2d, v1.2d, v2.2d
	fadd	v0.2d, v0.2d, v3.2d
	ret
.Lfunc_end0:
	.size	_RNvNtCs1234_4demo6kernel3dot, .Lfunc_end0-_RNvNtCs1234_4demo6kernel3dot
"""
_ARM_EXACT = _ARM_FMA.replace("fmla\tv0.2d, v1.2d, v2.2d", "fmul\tv4.2d, v1.2d, v2.2d")
_WASM_RELAXED = """\
	.section	.text._RNvNtCs1234_4demo6kernel3dot,"",@
	.type	_RNvNtCs1234_4demo6kernel3dot,@function
_RNvNtCs1234_4demo6kernel3dot:
	.functype	_RNvNtCs1234_4demo6kernel3dot (i32, i32) -> (f64)
	local.get	0
	v128.load	0
	f64x2.relaxed_madd
	f64x2.add
	f64x2.mul
	v128.const	0, 0
	end_function
"""
_WASM_EXACT = _WASM_RELAXED.replace("\tf64x2.relaxed_madd\n", "")
_X86_FAST = """\
	.type	_ZN4demo4fast3dot17h1111111111111111E,@function
_ZN4demo4fast3dot17h1111111111111111E:
	vaddpd	%ymm1, %ymm0, %ymm0
	vmulpd	%ymm2, %ymm1, %ymm1
	retq
.Lfunc_end0:
	.size	_ZN4demo4fast3dot17h1111111111111111E, .Lfunc_end0-_ZN4demo4fast3dot17h1111111111111111E
	.type	_ZN4demo4fast3dot17h1111111111111111E.cold.1,@function
_ZN4demo4fast3dot17h1111111111111111E.cold.1:
	vaddpd	%ymm3, %ymm0, %ymm0
	retq
.Lfunc_end1:
	.size	_ZN4demo4fast3dot17h1111111111111111E.cold.1, .Lfunc_end1-_ZN4demo4fast3dot17h1111111111111111E.cold.1
"""
_X86_FAST_FMA = _X86_FAST.replace("vmulpd\t%ymm2, %ymm1, %ymm1", "vfmadd231pd\t%ymm2, %ymm1, %ymm0")
_X86_FAST_DUP = _X86_FAST_FMA + _X86_FAST_FMA.replace("1111111111111111", "2222222222222222")


def _measure(**kw) -> Measure:
    base = dict(
        configs=CONFIG_NAMES, crate="demo", symbol="kernel::dot", label="", min_vector_ops=1,
        require_mnemonics=(), max_fma=0, min_fma=None, forbid_relaxed=True, single_copy=False,
    )
    base.update(kw)
    return Measure(**base)


def _problems(asm: str, arch: str, measure: Measure) -> tuple[str, ...]:
    return _problems_in([("fixture.s", asm, 0)], arch, measure)


def _problems_in(units: list[tuple[str, str, int]], arch: str, measure: Measure) -> tuple[str, ...]:
    config = next(c for c in CONFIGS if c.arch == arch)
    funcs = collect_functions(units, arch)
    return evaluate(Site("fixture", "fixture", (measure,)), measure, config, funcs).problems


def _manifest_dict(**site_overrides) -> dict:
    site = {
        "id": "demo.dot", "summary": "fixture",
        "measure": [{
            "configs": list(CONFIG_NAMES), "crate": "demo", "symbol": "kernel::dot",
            "min_vector_ops": 0, "max_fma": 0, "forbid_relaxed": True,
        }],
    }
    site.update(site_overrides)
    return {"build": {"packages": ["demo"]}, "site": [site]}


_FIXTURE_WORLD = DocWorld(
    members=("demo-core", "demo-cli"),
    member_dirs={"demo-core": "crates/demo-core", "demo-cli": "crates/demo-cli"},
    bench_files=("crates/demo-core/benches/dot.rs",),
    bench_dirs=frozenset({"crates/demo-core"}),
)


def _fixture_doc(cell: str = "v0·f0·r0", *, rows: str | None = None, benches: str | None = None) -> str:
    cfg_head = " | ".join(CONFIG_NAMES)
    cfg_sep = " | ".join("---" for _ in CONFIG_NAMES)
    cfg_vals = " | ".join(cell for _ in CONFIG_NAMES)
    dash = " | ".join("—" for _ in CONFIG_NAMES)
    covers = ", ".join(ROSTER)
    if rows is None:
        rows = (
            f"| `demo.dot` | `demo-core` | `kernel::dot` | {covers} | leave | a loop-carried sum | {cfg_vals} |\n"
            f"| `demo-cli` | `demo-cli` | — | — | leave | no hot path; `crates/demo-cli/benches/` does not exist | {dash} |\n"
        )
    if benches is None:
        benches = "| `crates/demo-core/benches/dot.rs` | demo.dot |\n"
    return (
        "# Fixture\n\n" + SITES_BEGIN + "\n"
        f"| id | crate | fn | covers | verdict | reason | {cfg_head} |\n"
        f"|---|---|---|---|---|---|{cfg_sep}|\n" + rows + SITES_END + "\n\n"
        + BENCHES_BEGIN + "\n| bench | sites |\n|---|---|\n" + benches + BENCHES_END + "\n"
    )


def self_test() -> int:
    failures: list[str] = []

    def expect(cond: bool, what: str) -> None:
        if not cond:
            failures.append(what)

    # -- demangling
    expect(demangle("_ZN4demo6kernel3dot17h0123456789abcdefE").path == "demo::kernel::dot", "legacy path")
    expect(demangle("_ZN4demo6kernel3dot17h0123456789abcdefE").crate == "demo", "legacy crate")
    legacy_impl = demangle("_ZN46_$LT$demo..Bar$u20$as$u20$core..fmt..Debug$GT$3fmt17h0123456789abcdefE")
    expect(legacy_impl.path == "<demo::Bar as core::fmt::Debug>::fmt", f"legacy impl path: {legacy_impl.path}")
    expect(legacy_impl.crate == "demo", "legacy impl crate")
    v0 = demangle("_RNvNvNtNtNtCslcQ1kWjaiiR_6memchr4arch6x86_646memchr11memchr2_raw9find_avx2")
    expect(v0.path == "memchr::arch::x86_64::memchr::memchr2_raw::find_avx2", f"v0 nested path: {v0.path}")
    expect(v0.crate == "memchr", "v0 crate")
    v0_impl = demangle("_RNvMNtNtNtNtCslcQ1kWjaiiR_6memchr4arch6x86_644avx210packedpairNtB2_6Finder9find_impl")
    expect(v0_impl.path == "<memchr::arch::x86_64::avx2::packedpair::Finder>::find_impl", f"v0 impl + backref: {v0_impl.path}")
    v0_generic = demangle("_RINvNtCs1234_4demo6kernel3dotdfEB4_")
    expect(v0_generic.path == "demo::kernel::dot::<f64, f32>", f"v0 generic: {v0_generic.path}")
    v0_trait = demangle("_RNvXs_NtCs1234_4demo6kernelNtB4_6KernelNtNtCs5678_4core3fmt5Debug3fmt")
    expect(v0_trait.path == "<demo::kernel::Kernel as core::fmt::Debug>::fmt", f"v0 trait impl: {v0_trait.path}")
    expect(v0_trait.crate == "demo", "v0 trait impl crate is the impl's crate")
    clone = demangle("_RNvNtCs1234_4demo6kernel3dot.llvm.123456")
    expect(clone.base == "_RNvNtCs1234_4demo6kernel3dot" and clone.path == "demo::kernel::dot", "v0 clone suffix")
    expect(demangle("_ZN4demo3dot17h0123456789abcdefE.cold.1").base == "_ZN4demo3dot17h0123456789abcdefE", "legacy clone")

    # -- parsing: block labels are not functions; clones fold under the parent
    funcs = collect_functions([("f.s", _X86_FAST)], "x86")
    expect(len(funcs) == 1 and len(funcs[0].instructions) == 5, f"cold clone folds under its parent: {[(f.path, len(f.instructions)) for f in funcs]}")
    expect(all(".LBB" not in f.symbol for f in collect_functions([("f.s", _X86_SCALAR)], "x86")), ".LBB is never a function")
    picked = collect_functions([("f.s", _X86_FAST + _X86_PACKED)], "x86", select=lambda d: d.path == "demo::fast::dot")
    expect([f.path for f in picked] == ["demo::fast::dot"] and len(picked[0].instructions) == 5, "the name prefilter keeps only the selected function, clones included")
    # -- a closure nested in a measured function is not a second copy of the site
    parent = collect_functions([("f.s", _X86_PACKED)], "x86")[0]
    stub = Function(parent.symbol + "c", parent.path + "::{closure#0}", parent.crate, parent.unit)
    judged = measured([parent, stub], _measure())
    expect(judged == [parent], "a scalar closure nested in a vectorized match is not judged against its floor")
    expect(measured([stub], _measure()) == [stub], "a closure that is the only match is still measured, never dropped")
    expect(any("below the floor" in p for p in evaluate(Site("fixture", "fixture", (_measure(),)), _measure(), next(c for c in CONFIGS if c.arch == "x86"), [stub]).problems), "a closure-only match still fails its floor")
    named = _measure(symbol=parent.path + "::{closure#0}")
    expect(measured([parent, stub], named) == [stub], "a symbol that names the closure measures the closure")
    wasm_funcs = collect_functions([("w.s", _WASM_EXACT)], "wasm")
    expect(len(wasm_funcs) == 1 and wasm_funcs[0].path == "demo::kernel::dot", "wasm function split")

    # -- min_vector_ops: scalar with a zeroing idiom fails; packed neighbour passes
    expect(any("below the floor" in p for p in _problems(_X86_SCALAR, "x86", _measure())), "scalar xorps+movaps+addsd must fail min_vector_ops >= 1")
    expect(not _problems(_X86_PACKED, "x86", _measure()), "addpd in the loop must pass")
    # -- an idiom cannot satisfy a required mnemonic; real work can
    idiom = _X86_SCALAR.replace("addsd\t%xmm1, %xmm0", "pcmpeqb\t%xmm3, %xmm3\n\tpmovmskb\t%xmm3, %eax")
    real = _X86_SCALAR.replace("addsd\t%xmm1, %xmm0", "pcmpeqb\t%xmm1, %xmm3\n\tpmovmskb\t%xmm3, %eax")
    req = _measure(require_mnemonics=("pcmpeqb", "pmovmskb"), min_vector_ops=0)
    expect(any("lacks the required `pcmpeqb`" in p for p in _problems(idiom, "x86", req)), "same-register pcmpeqb must not satisfy the requirement")
    expect(not _problems(real, "x86", req), "a real pcmpeqb + pmovmskb must satisfy it")
    expect(any("lacks" in p for p in _problems(_ARM_EXACT, "aarch64", _measure(require_mnemonics=("fadd .4s",)))), "arrangement qualifier must be honoured")
    expect(not _problems(_ARM_EXACT, "aarch64", _measure(require_mnemonics=("fadd .2d", "fmul .2d"))), "fadd/fmul .2d present")
    arm_zero = _ARM_EXACT.replace("fadd\tv0.2d, v0.2d, v3.2d", "eor\tv0.16b, v0.16b, v0.16b").replace("fmul\tv4.2d, v1.2d, v2.2d", "mov\tv4.16b, v1.16b")
    expect(any("below the floor" in p for p in _problems(arm_zero, "aarch64", _measure())), "aarch64 eor zeroing + mov is not vector work")

    # -- max_fma = 0: FMA fails (x86 and aarch64); the neighbour without FMA passes
    expect(any("fused multiply-add" in p for p in _problems(_X86_FMA, "x86", _measure())), "vfmadd231pd must fail max_fma = 0")
    expect(any("fused multiply-add" in p for p in _problems(_ARM_FMA, "aarch64", _measure())), "fmla must fail max_fma = 0")
    expect(not _problems(_ARM_EXACT, "aarch64", _measure()), "aarch64 without FMA must pass max_fma = 0")
    # -- forbid_relaxed
    expect(any("relaxed_" in p for p in _problems(_WASM_RELAXED, "wasm", _measure(max_fma=None, min_fma=0))), "f64x2.relaxed_madd must fail forbid_relaxed")
    expect(not _problems(_WASM_EXACT, "wasm", _measure(require_mnemonics=("f64x2.add", "f64x2.mul"))), "f64x2.add + f64x2.mul without relaxed must pass")
    # -- min_fma on a fast fn
    fast = _measure(crate="demo", symbol="fast::dot", max_fma=None, min_fma=1, single_copy=True)
    expect(any("below the fast floor" in p for p in _problems(_X86_FAST, "x86", fast)), "a fast fn without FMA must fail min_fma >= 1")
    expect(not _problems(_X86_FAST_FMA, "x86", fast), "a fast fn with FMA (and a folded .cold clone) must pass")
    # -- single_copy
    expect(any("single_copy" in p for p in _problems(_X86_FAST_DUP, "x86", fast)), "a duplicated fast symbol must fail single_copy")
    # Two units of one graph each holding a copy (a generic body instantiated in two
    # downstream crates) is two copies in one program; the same crate rebuilt by a second
    # graph is a separate artifact, and its one copy there is its own.
    two_crates = [("demo_a-1111111111111111.s", _X86_FAST_FMA, 0), ("demo_b-2222222222222222.s", _X86_FAST_FMA, 0)]
    expect(any("single_copy" in p for p in _problems_in(two_crates, "x86", fast)), "one copy in each of two units of one graph must fail single_copy")
    two_graphs = [("demo-1111111111111111.s", _X86_FAST_FMA, 0), ("demo-2222222222222222.s", _X86_FAST_FMA, 1)]
    expect(not _problems_in(two_graphs, "x86", fast), "one copy in each of two build graphs must pass single_copy")
    expect(
        any("single_copy" in p for p in _problems_in([("demo.s", _X86_FAST_DUP, 0), ("demo-2.s", _X86_FAST_FMA, 1)], "x86", fast)),
        "a duplicate inside one graph fails even when another graph is clean",
    )
    # -- presence
    expect(any("renamed or inlined away" in p for p in _problems(_X86_PACKED, "x86", _measure(symbol="kernel::gone"))), "a missing symbol must fail")
    expect(any("renamed or inlined away" in p for p in _problems(_X86_PACKED, "x86", _measure(crate="other"))), "the crate must match, not just the path")

    # -- manifest validation
    try:
        load_manifest(_manifest_dict())
    except GateError as err:
        failures.append(f"a complete manifest must load: {err}")
    for bad, what in (
        (_manifest_dict(measure=[{**_manifest_dict()["site"][0]["measure"][0], "configs": ["x86_64"]}]), "a site missing configurations"),
        (_manifest_dict(measure=[{**_manifest_dict()["site"][0]["measure"][0], "configs": [*CONFIG_NAMES, "riscv"]}]), "an unknown configuration"),
        (_manifest_dict(measure=[{k: v for k, v in _manifest_dict()["site"][0]["measure"][0].items() if k != "forbid_relaxed"}]), "an implicit forbid_relaxed"),
        (_manifest_dict(measure=[{**_manifest_dict()["site"][0]["measure"][0], "min_fma": 1}]), "both max_fma and min_fma"),
        (_manifest_dict(measure=[{**_manifest_dict()["site"][0]["measure"][0], "floor": 1}]), "an unknown key"),
        (_manifest_dict(measure=[{**_manifest_dict()["site"][0]["measure"][0], "note": "x"}]), "a `note` key (annotations are TOML comments)"),
    ):
        try:
            load_manifest(bad)
            failures.append(f"{what} must be refused")
        except GateError:
            pass

    # -- a `note` is refused by name, like any key the gate would read and then ignore;
    # the same measure without it loads
    try:
        load_manifest(_manifest_dict(measure=[{**_manifest_dict()["site"][0]["measure"][0], "note": "x"}]))
    except GateError as err:
        expect("unknown key `note`" in str(err), f"a `note` must be refused as an unknown key: {err}")
    try:
        load_manifest(_manifest_dict(measure=[dict(_manifest_dict()["site"][0]["measure"][0])]))
    except GateError as err:
        failures.append(f"the same measure without `note` must load: {err}")

    # -- `rlib_packages`: each cdylib package is built alone as an rlib; a package listed
    # both ways, a wrong type or an unknown `[build]` key is refused
    rlib_ok = {**_manifest_dict(), "build": {"packages": ["demo"], "rlib_packages": ["demo-wasm"]}}
    try:
        expect(load_manifest(rlib_ok).rlib_packages == ("demo-wasm",), "rlib_packages load as written")
    except GateError as err:
        failures.append(f"a manifest with rlib_packages must load: {err}")
    for build, what in (
        ({"packages": ["demo"], "rlib_packages": ["demo"]}, "a package in both lists"),
        ({"packages": ["demo"], "rlib_packages": "demo-wasm"}, "a string rlib_packages"),
        ({"packages": ["demo"], "features": ["x"]}, "an unknown [build] key"),
    ):
        try:
            load_manifest({**_manifest_dict(), "build": build})
            failures.append(f"{what} must be refused")
        except GateError:
            pass
    cmds = build_commands(CONFIG_BY_NAME["aarch64"], ("demo",), ("demo-wasm",))
    expect(len(cmds) == 2 and cmds[0][:2] == ["cargo", "build"] and cmds[1][:2] == ["cargo", "rustc"], f"one build plus one rustc per rlib package: {cmds}")
    expect(cmds[1][-4:] == ["-p", "demo-wasm", "--crate-type", "rlib"] and "--target" in cmds[1] and "--lib" in cmds[1], f"the rlib build names its package and crate type: {cmds[1]}")
    expect(len(build_commands(CONFIG_BY_NAME["x86_64"], ("demo",), ())) == 1, "no rlib package, one command")

    # -- a wrong-typed field is a named manifest error, never a coercion; its neighbour loads
    base_measure = _manifest_dict()["site"][0]["measure"][0]
    for key, value in (
        ("min_vector_ops", "3"), ("min_vector_ops", True), ("min_vector_ops", -1),
        ("max_fma", 0.0), ("crate", 5), ("symbol", ""), ("label", 7),
        ("forbid_relaxed", "yes"), ("single_copy", 1),
        ("require_mnemonics", "pcmpeqb"), ("require_mnemonics", ["pcmpeqb", 3]), ("configs", "x86_64"),
    ):
        try:
            load_manifest(_manifest_dict(measure=[{**base_measure, key: value}]))
            failures.append(f"`{key} = {value!r}` must be refused")
        except GateError as err:
            expect(f"`{key}` must be" in str(err), f"`{key} = {value!r}` must name the field and its type: {err}")
    typed_neighbour = {
        **base_measure, "min_vector_ops": 3, "label": "sse2", "single_copy": False,
        "require_mnemonics": ["pcmpeqb", "pmovmskb"],
    }
    try:
        loaded = load_manifest(_manifest_dict(measure=[typed_neighbour])).sites[0].measures[0]
        expect(loaded.min_vector_ops == 3 and loaded.require_mnemonics == ("pcmpeqb", "pmovmskb"), "typed fields load as written")
    except GateError as err:
        failures.append(f"the correctly typed neighbour must load: {err}")

    # -- environment and command-line verification
    try:
        refuse_overriding_env({"RUSTFLAGS": "-C target-cpu=native"})
        failures.append("RUSTFLAGS must be refused")
    except GateError:
        pass
    try:
        refuse_overriding_env({"RUSTFLAGS": "", "PATH": "/usr/bin"})
    except GateError:
        failures.append("an empty or absent RUSTFLAGS must be accepted")
    v3 = CONFIG_BY_NAME["x86_64-v3"]
    good = "     Running `kache rustc --crate-name memchr --emit=dep-info,metadata,link -C opt-level=3 --target x86_64-unknown-linux-gnu --emit=asm -D warnings -C target-cpu=x86-64-v3`"
    expect(not verify_command_lines(good, v3), "a correctly flagged unit must pass")
    expect(verify_command_lines(good.replace(" --emit=asm", ""), v3), "a unit without --emit=asm must fail")
    expect(verify_command_lines(good.replace(" -D warnings", ""), v3), "a unit without -D warnings must fail")
    expect("-D warnings" in v3.rustflags() and "--emit=asm" in v3.rustflags(), "the per-target rustflags carry --emit=asm and -D warnings")
    expect(verify_command_lines(good[:-1] + " -C target-cpu=native`", v3), "a second target-cpu must fail")
    expect(not verify_command_lines(good.replace("--crate-name memchr", "--crate-name build_script_build").replace(" --target x86_64-unknown-linux-gnu", ""), v3), "host units are out of scope")
    cross = config_env(
        CONFIG_BY_NAME["aarch64"], {"CFLAGS": "-march=native", "CC_aarch64_unknown_linux_gnu": "my-cc"},
        "x86_64-unknown-linux-gnu", which=lambda tool: f"/bin/{tool}", log=lambda _line: None,
    )
    expect(cross.get("CC_aarch64_unknown_linux_gnu") == "my-cc", "a caller's CC_<triple> is kept")
    logged: list[str] = []
    cross_cfg = CONFIG_BY_NAME["aarch64"]
    host_triple_name = "x86_64-unknown-linux-gnu"
    tools = lambda tool: f"/bin/{tool}"  # noqa: E731
    base_env = {"CFLAGS": "-O2 -march=native -pipe", "CXXFLAGS": "-O2 -pipe"}
    stripped = config_env(cross_cfg, base_env, host_triple_name, which=tools, log=logged.append)
    expect(stripped["CFLAGS"] == "-O2 -pipe", f"(a) cross CFLAGS loses -march=native: {stripped['CFLAGS']!r}")
    expect(logged == ["simd-asm: aarch64-unknown-linux-gnu: removed host-CPU flag(s) -march=native from CFLAGS (cross build)"], f"(a) the removal is logged once, CXXFLAGS not at all: {logged}")
    expect(base_env["CFLAGS"] == "-O2 -march=native -pipe", "(a) the caller's environment mapping is not mutated")
    logged.clear()
    host_env = config_env(CONFIG_BY_NAME["x86_64"], {"CFLAGS": "-O2 -march=native -pipe"}, host_triple_name, log=logged.append)
    expect(host_env["CFLAGS"] == "-O2 -march=native -pipe" and not logged, "(b) a host configuration keeps CFLAGS byte-for-byte")
    logged.clear()
    untouched = config_env(cross_cfg, {"CFLAGS": "-O2  -pipe"}, host_triple_name, which=tools, log=logged.append)
    expect(untouched["CFLAGS"] == "-O2  -pipe" and not logged, "(c) cross CFLAGS without a native token is unchanged, byte-for-byte, and silent")
    logged.clear()
    kept = config_env(cross_cfg, {"CFLAGS": "-march=armv8-a -O2"}, host_triple_name, which=tools, log=logged.append)
    expect(kept["CFLAGS"] == "-march=armv8-a -O2" and not logged, "(d) a non-native -march is kept")
    expect(strip_host_cpu_flags("-O2 -march native -mtune=native -mcpu=native -g") == ("-O2 -g", ["-march native", "-mtune=native", "-mcpu=native"]), "split and every host-CPU flag form is removed")
    expect(strip_host_cpu_flags("-mcpu=cortex-a72 -mtune=generic")[1] == [], "non-native -mcpu/-mtune are kept")
    wasm_env = config_env(CONFIG_BY_NAME["wasm32"], {"CXXFLAGS": "-mtune=native -O2"}, host_triple_name, which=tools, log=logged.append)
    expect(wasm_env["CXXFLAGS"] == "-O2", "wasm32 is a cross configuration too")
    try:
        config_env(CONFIG_BY_NAME["aarch64"], {}, "x86_64-unknown-linux-gnu", which=lambda _tool: None)
        failures.append("a cross build with no clang/llvm-ar must be refused")
    except GateError:
        pass
    named = config_env(CONFIG_BY_NAME["aarch64"], {}, "x86_64-unknown-linux-gnu", which=lambda tool: f"/bin/{tool}")
    expect(named.get("CC_aarch64_unknown_linux_gnu") == "clang" and named.get("AR_aarch64_unknown_linux_gnu") == "llvm-ar", "clang + llvm-ar are named for a cross target when present")
    everything = frozenset(Path("/sysroot/lib/rustlib") / c.triple / "lib" for c in CONFIGS)
    expect(not missing_targets(CONFIGS, Path("/sysroot"), is_dir=lambda p: p in everything), "every installed target passes")
    expect(missing_targets(CONFIGS, Path("/sysroot"), is_dir=lambda p: "aarch64" not in str(p) and p in everything) == ["aarch64-unknown-linux-gnu"], "a missing aarch64 std is named, not skipped")
    native = config_env(CONFIG_BY_NAME["x86_64"], {"CFLAGS": "-O2"}, "x86_64-unknown-linux-gnu")
    expect(native.get("CFLAGS") == "-O2" and "CC_x86_64_unknown_linux_gnu" not in native, "a host-target build keeps the host C configuration")

    # -- locating each unit's asm: a hashed rlib's `.s`; an unhashed cdylib+rlib unit's
    # `.s` in its own output directory; a unit with no `.s` anywhere is refused
    def artifact(name: str, filenames: list[str], kind: list[str]) -> str:
        return json.dumps({
            "reason": "compiler-artifact", "package_id": name,
            "target": {"name": name, "kind": kind}, "filenames": filenames,
        })
    x86 = CONFIG_BY_NAME["x86_64"]
    unit = "/t/build/x86_64-unknown-linux-gnu/release/build/demo/0123456789abcdef/out"
    plain = artifact("demo", ["/t/x86_64-unknown-linux-gnu/release/deps/libdemo-0123456789abcdef.rlib", "/t/x86_64-unknown-linux-gnu/release/deps/libdemo-0123456789abcdef.rmeta"], ["lib"])
    cdylib = artifact("demo_wasm", ["/t/x86_64-unknown-linux-gnu/release/libdemo_wasm.so", "/t/x86_64-unknown-linux-gnu/release/libdemo_wasm.rlib", f"{unit}/libdemo_wasm.rmeta"], ["cdylib", "rlib"])
    present = {Path("/t/x86_64-unknown-linux-gnu/release/deps/demo-0123456789abcdef.s"), Path(f"{unit}/demo_wasm.s")}
    try:
        found = asm_paths(plain + "\n" + cdylib, x86, is_file=lambda p: p in present)
        expect(found == [Path("/t/x86_64-unknown-linux-gnu/release/deps/demo-0123456789abcdef.s"), Path(f"{unit}/demo_wasm.s")], f"hashed and cdylib+rlib units both locate their .s: {found}")
    except GateError as err:
        failures.append(f"a cdylib+rlib unit with a .s in its output directory must be found: {err}")
    try:
        asm_paths(cdylib, x86, is_file=lambda _p: False)
        failures.append("a unit with no .s beside any rlib/rmeta must be refused")
    except GateError as err:
        expect("has no .s beside any" in str(err), f"the missing .s is named: {err}")

    # -- a failed build names the compiler's error, not only the command that failed
    noisy = "     Running `rustc --crate-name demo`\n       Fresh memchr v2\n   Compiling demo v1\nerror: linking with `cc` failed\n"
    expect(failure_tail(noisy) == "error: linking with `cc` failed", f"progress lines are dropped, the error kept: {failure_tail(noisy)!r}")

    # -- identity scan
    expect(identity_scan({"crates/rdf-core/src/canon.rs": "let s = a.algebraic_add(b);"}), "algebraic_add in canon.rs must fail")
    expect(not identity_scan({"crates/rdf-core/src/distance/exact.rs": "let s = a.algebraic_add(b);"}), "algebraic_add under distance/ must pass")
    expect(identity_scan({"crates/gts/src/lib.rs": "type A = Reassociated;"}), "Reassociated in gts must fail")
    expect(not identity_scan({"crates/rdf-core/src/canon.rs": "fn algebraic_class(&self) -> AlgebraicClass;"}), "algebraic_class (an aggregate property) must pass")
    expect(not identity_scan({"crates/hnsw/tests/fast.rs": "Reassociated"}), "hnsw tests may name Reassociated")

    # -- multiplier scan
    for claim in ("3× faster", "2x speedup", "40% faster", "a 1.5x speed-up"):
        expect(bool(multiplier_scan("t", claim)), f"{claim!r} must fail the multiplier scan")
    for fine in ("4×u32 masked compare is faster to write", "256×8 LUT", "`i32x4`", "`f64x2.add`", "the loop is 3x unrolled"):
        expect(not multiplier_scan("t", fine), f"{fine!r} must pass the multiplier scan")

    # -- document parity and coverage
    manifest = load_manifest(_manifest_dict())
    cells = {("demo.dot", c): "v0·f0·r0" for c in CONFIG_NAMES}
    fresh = doc_checks(_fixture_doc(), manifest, cells, _FIXTURE_WORLD)
    expect(not fresh, f"the fully mapped fixture must pass: {fresh}")
    expect(any("measured" in p for p in doc_checks(_fixture_doc("v9·f0·r0"), manifest, cells, _FIXTURE_WORLD)), "a stale count must fail parity")
    expect(write_doc(_fixture_doc("v9·f0·r0"), cells) == _fixture_doc(), "--write-doc regenerates the stale cells exactly")
    no_entry = load_manifest({"build": {"packages": ["demo"]}, "site": []})
    expect(any("has no manifest entry" in p for p in doc_checks(_fixture_doc(), no_entry, {}, _FIXTURE_WORLD)), "a leave row naming a function without an entry must fail")
    orphan = load_manifest({"build": {"packages": ["demo"]}, "site": [_manifest_dict()["site"][0], {**_manifest_dict()["site"][0], "id": "demo.orphan"}]})
    orphan_cells = {**cells, **{("demo.orphan", c): "v0·f0·r0" for c in CONFIG_NAMES}}
    expect(any("has no row" in p for p in doc_checks(_fixture_doc(), orphan, orphan_cells, _FIXTURE_WORLD)), "a manifest id with no row must fail")
    dash = " | ".join("—" for _ in CONFIG_NAMES)
    vals = " | ".join("v0·f0·r0" for _ in CONFIG_NAMES)
    covers = ", ".join(ROSTER)
    exempt_with_benches = (
        f"| `demo.dot` | `demo-cli` | `kernel::dot` | {covers} | leave | r | {vals} |\n"
        f"| `demo-core` | `demo-core` | — | — | leave | no hot path; `crates/demo-core/benches/` does not exist | {dash} |\n"
    )
    expect(any("exists, so" in p for p in doc_checks(_fixture_doc(rows=exempt_with_benches), manifest, cells, _FIXTURE_WORLD)), "an exempt row for a crate with benches/ must fail")
    function_row_instead = (
        f"| `demo.dot` | `demo-core` | `kernel::dot` | {covers} | leave | r | {vals} |\n"
        f"| `demo-cli` | `demo-cli` | — | — | leave | no hot path; `crates/demo-cli/benches/` does not exist | {dash} |\n"
    )
    expect(not doc_checks(_fixture_doc(rows=function_row_instead), manifest, cells, _FIXTURE_WORLD), "the same crate given a function row with an entry must pass")
    uncited = f"| `demo.dot` | `demo-core` | `kernel::dot` | {covers} | leave | r | {vals} |\n| `demo-cli` | `demo-cli` | — | — | leave | no hot path | {dash} |\n"
    expect(any("must cite" in p for p in doc_checks(_fixture_doc(rows=uncited), manifest, cells, _FIXTURE_WORLD)), "an exempt row that cites no benches/ must fail")
    missing_crate = f"| `demo.dot` | `demo-core` | `kernel::dot` | {covers} | leave | r | {vals} |\n"
    expect(any("`demo-cli` has no site row" in p for p in doc_checks(_fixture_doc(rows=missing_crate), manifest, cells, _FIXTURE_WORLD)), "a missing crate row must fail coverage")
    expect(any("has no row in the bench table" in p for p in doc_checks(_fixture_doc(benches=""), manifest, cells, _FIXTURE_WORLD)), "a missing bench row must fail coverage")
    expect(any("maps to no site id" in p for p in doc_checks(_fixture_doc(benches="| `crates/demo-core/benches/dot.rs` | — |\n"), manifest, cells, _FIXTURE_WORLD)), "a bench mapped to no id must fail")
    expect(any("unknown site id" in p for p in doc_checks(_fixture_doc(benches="| `crates/demo-core/benches/dot.rs` | demo.nope |\n"), manifest, cells, _FIXTURE_WORLD)), "a bench mapped to an unknown id must fail")
    short_roster = function_row_instead.replace(covers, ", ".join(ROSTER[1:]))
    expect(any(f"roster site `{ROSTER[0]}`" in p for p in doc_checks(_fixture_doc(rows=short_roster), manifest, cells, _FIXTURE_WORLD)), "a roster site with no row must fail")
    boast = _fixture_doc().replace("a loop-carried sum", "a loop-carried sum, 3× faster")
    expect(any("speed word" in p for p in doc_checks(boast, manifest, cells, _FIXTURE_WORLD)), "a multiplier in the document must fail")
    try:
        doc_checks("# no regions\n", manifest, cells, _FIXTURE_WORLD)
        failures.append("a document without the generated regions must fail")
    except GateError:
        pass

    # -- every other parity refusal, each against the passing fixture it differs from
    def refuses(doc: str, needle: str, what: str, man: Manifest = manifest, cl: dict = cells) -> None:
        found = doc_checks(doc, man, cl, _FIXTURE_WORLD)
        expect(any(needle in p for p in found), f"{what} must fail ({needle!r} not in {found})")
    fn_row = f"| `demo.dot` | `demo-core` | `kernel::dot` | {covers} | leave | r | {vals} |\n"
    exempt_row = f"| `demo-cli` | `demo-cli` | — | — | leave | no hot path; `crates/demo-cli/benches/` does not exist | {dash} |\n"
    refuses(_fixture_doc(rows=fn_row + fn_row + exempt_row), "appears twice", "a duplicated row id")
    refuses(_fixture_doc(rows=fn_row.replace("| leave |", "| faster-ish |") + exempt_row), "is not one of", "an unknown verdict")
    refuses(_fixture_doc(rows=fn_row.replace("`demo-core`", "`demo-gone`") + exempt_row), "is not a workspace member", "a row for a crate outside the workspace")
    refuses(_fixture_doc(rows=fn_row.replace("`kernel::dot`", "—") + exempt_row), "has a manifest entry but names no function", "a manifest site whose row names no function")
    refuses(_fixture_doc(rows=fn_row + exempt_row.replace("| leave |", "| rewrite |")), "must be `leave`", "a crate-level row with a non-leave verdict")
    no_covers = _fixture_doc().replace("| covers |", "| scope |", 1)
    refuses(no_covers, "has no `covers` column", "a site table without a `covers` column")
    refuses(_fixture_doc(benches="| `crates/demo-core/benches/dot.rs` | demo.dot |\n| `crates/demo-core/benches/gone.rs` | demo.dot |\n"), "names no bench file that exists", "a bench row for a file that does not exist")
    bad_bench_head = _fixture_doc().replace("| bench | sites |", "| bench | rows |")
    refuses(bad_bench_head, "needs `bench` and `sites` columns", "a bench table without a `sites` column")
    # the valid neighbour of the column checks: extra descriptive columns are accepted,
    # and `--write-doc` keeps them while regenerating only the configuration cells
    extra = _fixture_doc().replace("| reason |", "| reason | path |").replace("|---|---|---|---|---|---|", "|---|---|---|---|---|---|---|")
    extra = extra.replace("| a loop-carried sum |", "| a loop-carried sum | `src/k.rs:1` |").replace("does not exist |", "does not exist | — |")
    found = doc_checks(extra, manifest, cells, _FIXTURE_WORLD)
    expect(not found, f"extra descriptive columns must be accepted: {found}")
    stale_extra = extra.replace("v0·f0·r0", "v9·f0·r0")
    expect(write_doc(stale_extra, cells) == extra, "--write-doc keeps extra columns and regenerates only the configuration cells")
    # `--doc` on a missing document is a failure; an existing one passes the precondition
    try:
        require_doc(REPO_ROOT / "docs" / "design" / "no-such-simd-audit.md")
        failures.append("a missing document must fail --doc")
    except GateError as err:
        expect("does not skip it" in str(err), f"the missing document is named: {err}")
    try:
        require_doc(Path(__file__).resolve())
    except GateError as err:
        failures.append(f"an existing document must pass the --doc precondition: {err}")

    if failures:
        print("FAIL: the simd-asm gate's self-test found refusals that do not fire or neighbours that do not pass:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print("OK: simd-asm self-test -- every refusal fires and every valid neighbour passes")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument("--self-test", action="store_true", help="exercise every refusal and neighbour on fixtures; builds nothing")
    parser.add_argument("--doc", action="store_true", help="also check the audit document's parity and coverage")
    parser.add_argument("--write-doc", action="store_true", help="regenerate the document's measured cells, then check it")
    parser.add_argument("--probe", metavar="PATH", help="list every emitted function whose demangled path matches the regex PATH (exploration; not a gate)")
    parser.add_argument("--probe-operands", action="store_true", help="with --probe: match PATH against instruction operands (call targets) instead of function paths")
    parser.add_argument("--dump", action="store_true", help="with --probe: print every instruction of each match")
    parser.add_argument("--crate", help="with --probe: only this crate")
    parser.add_argument("--config", action="append", choices=CONFIG_NAMES, help="with --probe: only these configurations")
    args = parser.parse_args()
    if args.self_test:
        return self_test()
    try:
        return run(args)
    except GateError as err:
        print(f"FAIL: {err}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
