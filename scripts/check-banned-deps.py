#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

"""Fail if an eliminated third-party package re-enters the dependency surface.

``BANNED_ANY_EDGE`` records the complete removed dependency inventory,
including the ox-family and ``hex``. Every one is forbidden on runtime,
build, dev/test and transitive
edges. The check reads **every git-tracked Cargo.lock**, including the lock of
an excluded test fixture, and names the violating lockfile in its error. A
root-only scan would miss a stale resolution in an excluded workspace.

The independent manifest scan reads every git-tracked Cargo.toml and every
manifest declared by the root workspace, including new members not yet added
to git. It checks dependency aliases by their ``package`` name as well as
ordinary keys, across normal, build, dev/test, target and workspace tables.
This catches a forbidden direct declaration even before an excluded workspace
has a lockfile. A committed first-party manifest is not necessarily a member
of the root workspace: ``crates/gts/fuzz`` is excluded.

The manifest set is the union of the git-tracked set and the root workspace's
declared member/excluded-root set. The repository vendors no third-party Rust
source, so each tracked manifest is a first-party declaration. If vendored
Rust source is ever committed, its provenance must be handled explicitly; the
scan must not silently narrow to workspace members.

**Substitution.** A ``[patch]``/``[replace]`` override that redirects a
banned name to a different source is scanned for too: ``[[patch.unused]]``
blocks and ``replace = "name:version"`` lines in ``Cargo.lock`` are matched
against ``BANNED_ANY_EDGE`` the same as a normally resolved package.

What this CANNOT see: a fork published under a DIFFERENT package name (for
example a ``[patch]`` table remapping ``package = "my-oxigraph-fork"``).
Cargo.lock would then record that fork under its own name, indistinguishable
from any other unrelated crate to a name-based scan. Catching that requires a
source-URL allowlist or a checksum/provenance check, neither of which this
gate performs; a rename or an under-a-different-name fork is a real gap, not
one this script claims to close.

Each ban list entry names the replacement so the failure message teaches the
fix. A dependency joins a list in the change that removes it, so this gate
never lies about the present.

**The allowed half** (``dependency-ledger.toml``, ``[packages]``): the ban
lists say what may not come back; the ledger says what may stay. Every
external (non-workspace) package in every committed ``Cargo.lock`` has an
entry with a category — ``spec-codec``, ``crypto``, ``runtime``, ``oracle``,
``bench`` or ``binding`` for a direct dependency of a workspace member,
``transitive:<direct>`` for a package only reached through that direct
dependency — a one-line ``reason``, and a ``metrics`` table computed from
``cargo metadata --locked --offline`` (never the network):

* ``direct`` — a workspace member names it directly, on any edge kind;
* ``exclusive_closure`` — for a direct dependency, how many external packages
  leave the graph if every workspace edge to it is cut (itself included); 0
  for a transitive one;
* ``release`` — reached over normal and build edges from a crate whose
  ``publish`` is not false;
* ``wasm`` — the same, in the ``--filter-platform wasm32-unknown-unknown``
  resolution;
* ``build_script`` / ``proc_macro`` — the package has a build script / is a
  procedural macro.

The gate fails on an unlisted package, on a listed package no lock resolves,
on a name resolved at two versions without a ``duplicate_reason``, on an
uncategorized skeleton, on a ``transitive:<direct>`` category whose direct
dependency does not reach the package, and on metrics drift.
``--update-metrics`` rewrites the ``[packages]`` section deterministically
(adding flagged skeletons for new packages and reporting, never dropping,
entries whose package vanished); ``--report`` prints the graph counts.
"""

from __future__ import annotations

import json
import posixpath
import re
import subprocess
import sys
import tomllib
from collections.abc import Iterable
from dataclasses import dataclass, field
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

# Package name -> first-party replacement. Banned on ANY edge (runtime, build,
# dev/test, transitive) because a first-party replacement exists for every
# edge kind; reintroduction anywhere in the resolved Cargo.lock closure fails.
BANNED_ANY_EDGE: dict[str, str] = {
    "oxilangtag": "purrdf_iri::langtag (RFC 5646 well-formedness)",
    "oxiri": "purrdf-iri",
    "oxsdatatypes": "purrdf-xsd",
    "oxrdf": "purrdf-core",
    "oxigraph": "the native purrdf engine",
    "petgraph": "purrdf_core::graph::tarjan_scc (the first-party iterative Tarjan SCC)",
    "thiserror": "plain error types and std::error::Error implementations",
    "thiserror-impl": "plain error types and std::error::Error implementations (thiserror macro backend)",
    "tempfile": "purrdf_testkit::{TempDir, NamedTempFile} (temp_dir!/temp_file!, for_unit_test)",
    "insta": "purrdf_testkit::assert_golden! (byte-exact checked-in goldens)",
    # The previous insta snapshot UI's own transitive closure.
    "console": "purrdf_testkit::assert_golden! (no interactive snapshot UI)",
    "encode_unicode": "purrdf_testkit::assert_golden! (no terminal Unicode encoder)",
    "similar": "purrdf_testkit::assert_golden! (no snapshot diff engine)",
    "pretty_assertions": "ordinary Rust assertions and purrdf_testkit goldens",
    "diff": "ordinary Rust assertions (no assertion diff engine)",
    "yansi": "ordinary Rust assertions (no coloured assertion output)",
    "proptest": "purrdf_testkit::prop (the choice-sequence property harness, prop_test!)",
    # proptest's own closure: its RNG stack, its fork/timeout runner and its
    # array helpers. Nothing else in the graph pulled any of them in.
    "rand": "purrdf_testkit::prop's in-house SplitMix64/xoshiro256** stream",
    "rand_chacha": "purrdf_testkit::prop's in-house SplitMix64/xoshiro256** stream",
    "rand_xorshift": "purrdf_testkit::prop's in-house SplitMix64/xoshiro256** stream",
    "rand_core": "purrdf_testkit::prop's in-house SplitMix64/xoshiro256** stream (old signature optional edge also left)",
    "ppv-lite86": "purrdf_testkit::prop's in-house SplitMix64/xoshiro256** stream",
    "unarray": "purrdf_testkit::prop (no array strategies are needed)",
    "rusty-fork": "purrdf_testkit::prop (cases run in-process under catch_unwind)",
    "wait-timeout": "purrdf_testkit::prop (cases run in-process under catch_unwind)",
    "quick-error": "purrdf_testkit::prop (cases run in-process under catch_unwind)",
    "fnv": "purrdf_testkit::prop (cases run in-process under catch_unwind)",
    # Pulled in by proptest's bit-set strategies and by fancy-regex; with both
    # gone nothing else in the graph resolves them.
    "bit-set": "purrdf_testkit::prop (no bit-set strategies are needed)",
    "bit-vec": "purrdf_testkit::prop (no bit-set strategies are needed)",
    "datatest-stable": "purrdf_testkit::harness fed by purrdf_sparql_conformance::paths::suite_manifests",
    # datatest-stable's own closure: its UTF-8 path type, its libtest runner,
    # the runner's JSON string escaper, and the backtracking regex engine its
    # file pattern was matched with. Nothing else in the graph pulled any of them in.
    "camino": "std::path (case names are the UTF-8-checked, /-joined relative path)",
    "libtest-mimic": "purrdf_testkit::harness (the libtest-compatible harness = false runner)",
    "escape8259": "purrdf_testkit::harness (the libtest-compatible harness = false runner)",
    "fancy-regex": "purrdf_sparql_conformance::paths::suite_manifests (an exact file-name match)",
    "boon": "purrdf-jsonschema (native JSON Schema 2020-12, 2019-09 and draft-07 validation)",
    # boon's own closure: its URL/IDNA stack (url, idna and the ICU4X Unicode
    # data it normalizes with), its URI parser and its append-only list. Nothing
    # else in the graph pulled any of them in; purrdf-jsonschema resolves every
    # reference through purrdf-iri.
    "appendlist": "purrdf-jsonschema (compiled subschemas live in one arena)",
    "fluent-uri": "purrdf-iri (the workspace's one RFC 3986 resolver)",
    "borrow-or-share": "purrdf-iri (the workspace's one RFC 3986 resolver)",
    "ref-cast": "purrdf-iri (the workspace's one RFC 3986 resolver)",
    "ref-cast-impl": "purrdf-iri (the workspace's one RFC 3986 resolver)",
    "base64": "purrdf-jsonschema (the draft-07 contentEncoding check decodes RFC 4648 base64 in-house)",
    "url": "purrdf-iri (RFC 3986/3987 parsing and reference resolution)",
    "form_urlencoded": "purrdf-iri (RFC 3986/3987 parsing and reference resolution)",
    "percent-encoding": "purrdf-iri (RFC 3986/3987 parsing and reference resolution)",
    "idna": "purrdf-iri (RFC 3987 IRIs carry Unicode hosts without IDNA mapping)",
    "idna_adapter": "purrdf-iri (RFC 3987 IRIs carry Unicode hosts without IDNA mapping)",
    "utf8_iter": "purrdf-iri (RFC 3987 IRIs carry Unicode hosts without IDNA mapping)",
    "icu_collections": "purrdf-iri (no Unicode normalization on the IRI path)",
    "icu_locale_core": "purrdf-iri (no Unicode normalization on the IRI path)",
    "icu_normalizer": "purrdf-iri (no Unicode normalization on the IRI path)",
    "icu_normalizer_data": "purrdf-iri (no Unicode normalization on the IRI path)",
    "icu_properties": "purrdf-iri (no Unicode normalization on the IRI path)",
    "icu_properties_data": "purrdf-iri (no Unicode normalization on the IRI path)",
    "icu_provider": "purrdf-iri (no Unicode normalization on the IRI path)",
    "litemap": "purrdf-iri (no Unicode normalization on the IRI path)",
    "potential_utf": "purrdf-iri (no Unicode normalization on the IRI path)",
    "tinystr": "purrdf-iri (no Unicode normalization on the IRI path)",
    "writeable": "purrdf-iri (no Unicode normalization on the IRI path)",
    "yoke": "purrdf-iri (no Unicode normalization on the IRI path)",
    "yoke-derive": "purrdf-iri (no Unicode normalization on the IRI path)",
    "zerofrom": "purrdf-iri (no Unicode normalization on the IRI path)",
    "zerofrom-derive": "purrdf-iri (no Unicode normalization on the IRI path)",
    "zerotrie": "purrdf-iri (no Unicode normalization on the IRI path)",
    "zerovec": "purrdf-iri (no Unicode normalization on the IRI path)",
    "zerovec-derive": "purrdf-iri (no Unicode normalization on the IRI path)",
    "stable_deref_trait": "purrdf-iri (no Unicode normalization on the IRI path)",
    "displaydoc": "purrdf-iri (no Unicode normalization on the IRI path)",
    "synstructure": "purrdf-iri (no Unicode normalization on the IRI path)",
    "wasm-bindgen-test": "purrdf_testkit::harness run on wasm32 by scripts/wasm-test-runner.sh",
    "js-sys": "direct wasm-bindgen imports of Date.now and Math.random",
    # wasm-bindgen-test's own closure: its attribute macro and shared descriptor
    # crate, its async test executor, its coverage hook and its console colours.
    # Nothing else in the graph pulled any of them in.
    "wasm-bindgen-test-macro": "purrdf_testkit::harness_main! (cases named by their functions)",
    "wasm-bindgen-test-shared": "purrdf_testkit::harness run on wasm32 by scripts/wasm-test-runner.sh",
    "wasm-bindgen-futures": "purrdf_testkit::harness (wasm32 cases run synchronously)",
    "async-trait": "purrdf_testkit::harness (wasm32 cases run synchronously)",
    "scoped-tls": "purrdf_testkit::harness (wasm32 cases run synchronously)",
    "minicov": "purrdf_testkit::harness (no coverage hook in the wasm32 test binaries)",
    "nu-ansi-term": "purrdf_testkit::harness (libtest's own ANSI colour codes)",
    # js-sys's `std` feature pulled futures-util and its closure in for a stream
    # adapter nothing here uses; the workspace takes js-sys with default features
    # off, so nothing else resolves them.
    "futures-util": "js-sys with default-features = false (its `std` feature pulled it in)",
    "futures-core": "js-sys with default-features = false (its `std` feature pulled it in)",
    "futures-task": "js-sys with default-features = false (its `std` feature pulled it in)",
    "pin-project-lite": "js-sys with default-features = false (its `std` feature pulled it in)",
    "slab": "js-sys with default-features = false (its `std` feature pulled it in)",
    "md-5": "purrdf_hash::md5 (RFC 1321)",
    "hex": 'core::fmt::LowerHex formatting (`format!("{digest:x}")`)',
    "sha1": "purrdf_hash::sha1 (FIPS 180-4)",
    "sha3": "purrdf_hash::sha3 (FIPS 202)",
    # sha3's permutation crate; nothing else in the graph pulled it in.
    "keccak": "purrdf_hash::sha3::keccak_f1600 (FIPS 202 Keccak-f[1600])",
    "ahash": "purrdf_hash::fixed::FixedHasher",
    # The old hashbrown 0.15 resolution pulled these in; the current
    # hashbrown resolution does not. Banning every removed lockfile name also
    # keeps the old hashbrown closure from silently returning.
    "allocator-api2": "current hashbrown resolution (no allocator-api2 edge)",
    "foldhash": "purrdf_hash::fixed::FixedHasher (old hashbrown 0.15 closure)",
    "version_check": "const and build-time declarations (old ahash/generic-array version probe)",
    # RustCrypto 0.11 and ed25519-dalek 3 removed the older 0.10/0.7 crypto
    # trait and key-encoding closure from the published graph.
    "generic-array": "RustCrypto 0.11 hybrid-array-backed traits",
    "opaque-debug": "current RustCrypto GHash/POLYVAL implementation",
    "pkcs8": "ed25519-dalek 3 native key handling (no PKCS#8 import path)",
    "spki": "ed25519-dalek 3 native key handling (no SPKI import path)",
    "der": "ed25519-dalek 3 native key handling (no DER import path)",
    "base64ct": "ed25519-dalek 3 native key handling (old SPKI closure)",
    # These old optional resolutions vanished along with the replaced test
    # and RNG stacks. They remain forbidden even on target-specific edges.
    "libm": "current num-traits configuration and purrdf_testkit wasm harness",
    "wasi": "purrdf_testkit::rng (old getrandom 0.2 target closure)",
    "filetime": "std::fs::File::set_times (purrdf-gts restores file and directory mtimes)",
    "memmap2": "purrdf_cli::mmap::Mmap (read-only mmap/munmap through libc)",
    "rustix": "purrdf_cli::mmap (memfd_create and fcntl F_ADD_SEALS/F_GET_SEALS through libc)",
    # rustix's own closure: its raw-syscall backend and its error and flag
    # helpers. The committed Cargo.lock listed rustix as the only dependent of
    # each before it left.
    "linux-raw-sys": "purrdf_cli::mmap (libc system-call bindings)",
    "errno": "std::io::Error::last_os_error (in purrdf_cli::mmap)",
    "bitflags": "plain libc::c_int seal masks (in purrdf_cli::mmap)",
    "smallvec": "purrdf_core::SmallVec (an in-house small-vector type)",
    "flate2": "purrdf-deflate (native RFC 1951 DEFLATE and RFC 1952 gzip)",
    # flate2's pure-Rust backend and its closure. The committed Cargo.lock listed
    # flate2 as the only dependent of miniz_oxide and crc32fast, and miniz_oxide as
    # the only dependent of adler2 and simd-adler32, before they left.
    "miniz_oxide": "purrdf-deflate (native RFC 1951 DEFLATE and RFC 1952 gzip)",
    "adler2": "purrdf-deflate (gzip carries no Adler-32; no zlib framing is used)",
    "simd-adler32": "purrdf-deflate (gzip carries no Adler-32; no zlib framing is used)",
    "crc32fast": "purrdf_hash::crc32 (CRC-32/ISO-HDLC with pclmulqdq and Armv8 CRC32 paths)",
    "csv": "purrdf_core::csv (the W3C CSVW dialect reader/writer)",
    # csv's own field-scanning engine; nothing else in the graph pulled it in.
    "csv-core": "purrdf_core::csv (the W3C CSVW dialect reader/writer)",
    "time": "purrdf_gts::rfc3339 (RFC 3339 §5.6-5.8 parsed and formatted directly)",
    # time's own closure: its internal core types, its compile-time format-description
    # macros, and the ranged-integer/formatting crates its date validation used.
    # Nothing else in the graph pulled any of them in.
    "time-core": "purrdf_gts::rfc3339 (RFC 3339 §5.6-5.8 parsed and formatted directly)",
    "time-macros": "purrdf_gts::rfc3339 (RFC 3339 §5.6-5.8 parsed and formatted directly)",
    "deranged": "purrdf_gts::rfc3339 (RFC 3339 §5.6-5.8 parsed and formatted directly)",
    "num-conv": "purrdf_gts::rfc3339 (RFC 3339 §5.6-5.8 parsed and formatted directly)",
    "powerfmt": "purrdf_gts::rfc3339 (RFC 3339 §5.6-5.8 parsed and formatted directly)",
    # The analyzer's Unicode layer, generated from the vendored database in
    # crates/iri/unicode/; `cargo tree --locked --target all -i` showed each
    # held only by purrdf-text (tinyvec through unicode-normalization alone).
    "caseless": "purrdf_text::unicode::case_fold (CaseFolding.txt C + F)",
    "unicode-normalization": "purrdf_text::unicode::{nfd, nfc, nfkd, nfkc} (UAX 15)",
    "unicode-segmentation": "purrdf_text::unicode::{word_bounds, word_indices} (UAX 29)",
    "tinyvec": "purrdf_text::unicode (no inline-buffer crate is needed)",
    "tinyvec_macros": "purrdf_text::unicode (no inline-buffer crate is needed)",
}

# Reserved for a future direct-only removal, if one is ever authorized.
# Every current removal is an any-edge ban, including hex and rand_core.
BANNED_DIRECT_ONLY: dict[str, str] = {
}

DEP_TABLE_KEYS = ("dependencies", "dev-dependencies", "build-dependencies")

PACKAGE_NAME_RE = re.compile(r'^name = "([^"]+)"$', re.MULTILINE)
REPLACE_KEY_RE = re.compile(r'^replace = "([^:"]+):', re.MULTILINE)
PATCH_UNUSED_BLOCK_RE = re.compile(
    r"^\[\[patch\.unused\]\]\n((?:(?!^\[\[).)*)", re.MULTILINE | re.DOTALL
)


class TrackedFileDiscoveryError(RuntimeError):
    """Raised when the set of git-tracked files cannot be established.

    Never downgraded to "scan the root manifest/lock and hope": a gate that
    silently narrows its own scope is the exact failure this discovery step
    exists to fix, so an unusable ``git`` is a hard error, not a degraded mode.
    """


def paths_with_basename(listing: str, basename: str) -> list[str]:
    """Paths in a NUL-separated ``git ls-files -z`` listing whose final
    component is exactly ``basename``.

    Matching is on the **basename**, so near-miss names that are not actually
    the file in question (``Cargo.lock.bak``, a ``Cargo.lock.md`` write-up, a
    ``Cargo.toml.orig``) are not scanned. Returned in sorted order so the
    failure output is deterministic regardless of git's listing order.
    """
    return sorted(
        path
        for path in listing.split("\0")
        if path and path.rsplit("/", 1)[-1] == basename
    )


def lockfile_paths_from_ls_files(listing: str) -> list[str]:
    """The ``Cargo.lock`` paths in a NUL-separated ``git ls-files -z`` listing."""
    return paths_with_basename(listing, "Cargo.lock")


def manifest_paths_from_ls_files(listing: str) -> list[str]:
    """The ``Cargo.toml`` paths in a NUL-separated ``git ls-files -z`` listing."""
    return paths_with_basename(listing, "Cargo.toml")


def tracked_listing(root: Path) -> str:
    """The raw ``git ls-files -z`` listing for ``root``.

    Uses git rather than a filesystem glob on purpose: "committed" is the
    property that matters (an untracked manifest or lock in a scratch
    directory or a ``target/`` tree is not something a reviewer can be held
    to), and git is the only authority on it.
    """
    try:
        completed = subprocess.run(
            ["git", "-C", str(root), "ls-files", "-z"],
            capture_output=True,
            check=True,
            text=True,
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        raise TrackedFileDiscoveryError(
            f"could not list git-tracked files under {root} to find every "
            f"committed Cargo.lock and Cargo.toml: {exc}"
        ) from exc
    return completed.stdout


def committed_lockfiles(root: Path) -> list[Path]:
    """Every ``Cargo.lock`` tracked by git under ``root``."""
    return [root / path for path in lockfile_paths_from_ls_files(tracked_listing(root))]


def lock_failures(display_path: str, lock_text: str) -> list[str]:
    """Tier-1 failure messages for one lockfile, each naming ``display_path``.

    ``display_path`` is repository-relative so the message points at the file
    to fix — which lock a banned name came back through is the whole question
    once more than one is scanned.
    """
    any_edge = set(any_edge_offenders(lock_text))
    substitution = set(substitution_offenders(lock_text))
    failures: list[str] = []
    for name in sorted(any_edge | substitution):
        replacement = BANNED_ANY_EDGE[name]
        if name in any_edge:
            failures.append(
                f"FAIL: `{name}` is back in {display_path}; it was replaced by "
                f"{replacement} and must not be reintroduced through any "
                "runtime, build, or test edge"
            )
        else:
            failures.append(
                f"FAIL: `{name}` reappears via a patch/replace override in "
                f"{display_path} with no plain resolved entry; it was replaced "
                f"by {replacement} and must not be reintroduced under any source"
            )
    return failures


def any_edge_offenders(lock_text: str) -> list[str]:
    """Tier-1 names resolved anywhere in the lock (any ``name = "..."`` line).

    The regex is not scoped to ``[[package]]`` tables, so it already matches a
    banned name sitting inside a ``[[patch.unused]]`` block the same as an
    ordinary resolved package.
    """
    resolved = set(PACKAGE_NAME_RE.findall(lock_text))
    return sorted(resolved & BANNED_ANY_EDGE.keys())


def substitution_offenders(lock_text: str) -> list[str]:
    """Tier-1 names smuggled back via a ``[patch]``/``[replace]`` override
    that keeps the same package name but redirects its source.

    Two constructs are checked explicitly (on top of, and overlapping with,
    ``any_edge_offenders``, so a future narrowing of ``PACKAGE_NAME_RE`` to
    ``[[package]]`` tables only cannot silently drop this coverage):

    * ``[[patch.unused]]`` blocks — a declared-but-inactive patch still names
      the package it would have replaced.
    * ``replace = "name:version"`` keys — the deprecated ``[replace]``
      feature's pointer, which names the package in a value string rather
      than a ``name = "..."`` line, so ``any_edge_offenders`` alone would miss
      it.
    """
    names: set[str] = set()
    names.update(REPLACE_KEY_RE.findall(lock_text))
    for block in PATCH_UNUSED_BLOCK_RE.finditer(lock_text):
        names.update(PACKAGE_NAME_RE.findall(block.group(1)))
    return sorted(names & BANNED_ANY_EDGE.keys())


def direct_dependency_names(manifest: dict) -> set[str]:
    """Every real package name a manifest declares directly.

    Covers the top-level ``dependencies``/``dev-dependencies``/
    ``build-dependencies`` tables plus the same three tables nested under any
    ``[target.'cfg(...)'.*]`` platform gate. A renamed dependency's ``package``
    value is the resolved package name; its local alias must not hide a ban.
    """
    names: set[str] = set()

    def add_table(table: dict) -> None:
        for alias, declaration in table.items():
            if isinstance(declaration, dict):
                names.add(declaration.get("package", alias))
            else:
                names.add(alias)

    for key in DEP_TABLE_KEYS:
        add_table(manifest.get(key, {}))
    for target_table in manifest.get("target", {}).values():
        for key in DEP_TABLE_KEYS:
            add_table(target_table.get(key, {}))
    return names


def manifest_override_names(manifest: dict) -> set[str]:
    """Real package names declared by manifest ``[patch]``/``[replace]``.

    The lockfile scan catches a resolved or unused patch. This independent
    scan also covers an excluded workspace with no committed lock yet.
    """
    names: set[str] = set()
    for patches in manifest.get("patch", {}).values():
        for alias, declaration in patches.items():
            names.add(
                declaration.get("package", alias)
                if isinstance(declaration, dict)
                else alias
            )
    for key, declaration in manifest.get("replace", {}).items():
        names.add(key.split(":", 1)[0])
        if isinstance(declaration, dict) and "package" in declaration:
            names.add(declaration["package"])
    return names


def manifest_banned_declarations(
    manifest: dict, banned: dict[str, str]
) -> tuple[set[str], set[str]]:
    """Banned package names in workspace and ordinary manifest declarations."""
    workspace_deps = direct_dependency_names(
        {"dependencies": manifest.get("workspace", {}).get("dependencies", {})}
    )
    declared = direct_dependency_names(manifest) | manifest_override_names(manifest)
    return workspace_deps & banned.keys(), declared & banned.keys()


def manifest_path_for(directory: str) -> str:
    """The repository-relative ``Cargo.toml`` path for a crate directory.

    ``directory`` comes straight out of a ``members``/``exclude`` list, so it
    may be ``"."`` (the single-package workspace ``crates/gts/fuzz`` declares)
    or carry a trailing ``.`` component; ``normpath`` folds both so the set
    never holds ``a/./Cargo.toml`` and ``a/Cargo.toml`` as two entries.
    """
    normalized = posixpath.normpath(directory)
    return "Cargo.toml" if normalized == "." else f"{normalized}/Cargo.toml"


def declared_member_manifests(root: Path, base: str, workspace_table: dict) -> set[str]:
    """Manifest paths for every member a single ``[workspace]`` table declares.

    ``base`` is the repository-relative directory that owns the table (``""``
    for the root manifest), because an excluded root's ``members`` entries are
    relative to *it*, not to the repository root.

    A member entry may be a glob (``crates/*``); those are expanded against
    the filesystem, since a glob names whatever is on disk and there is no
    manifest text to read otherwise. A table with no ``members`` key is not an
    error — Cargo then treats the manifest's own package as the sole member,
    and that manifest is added by the caller.
    """
    manifests: set[str] = set()
    for entry in workspace_table.get("members", []):
        relative = posixpath.normpath(f"{base}/{entry}" if base else entry)
        if any(character in entry for character in "*?["):
            manifests.update(
                path.relative_to(root).as_posix()
                for path in root.glob(f"{relative}/Cargo.toml")
            )
        else:
            manifests.add(manifest_path_for(relative))
    return manifests


def declared_manifests(root: Path) -> set[str]:
    """Manifest paths the root ``Cargo.toml`` declares, directly or via an
    excluded root's own ``[workspace]`` table.

    This is the half of the tier-2 manifest set that does not depend on git,
    so a first-party crate that is already a workspace member but not yet
    ``git add``-ed is still scanned.
    """
    root_manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    workspace_table = root_manifest.get("workspace", {})

    manifests = {"Cargo.toml"}
    manifests.update(declared_member_manifests(root, "", workspace_table))

    for excluded in workspace_table.get("exclude", []):
        excluded_manifest_path = manifest_path_for(excluded)
        excluded_file = root / excluded_manifest_path
        if not excluded_file.is_file():
            # `exclude` may legitimately name a directory that holds no crate
            # at all (build scratch, fixtures). Nothing to read, nothing to
            # ban — this is not a narrowing of the scan.
            continue
        manifests.add(excluded_manifest_path)
        excluded_manifest = tomllib.loads(excluded_file.read_text(encoding="utf-8"))
        manifests.update(
            declared_member_manifests(
                root,
                posixpath.normpath(excluded),
                excluded_manifest.get("workspace", {}),
            )
        )

    return manifests


def first_party_manifests(root: Path) -> list[str]:
    """Every first-party ``Cargo.toml`` tier 2 must read, repository-relative.

    The union of the git-tracked manifests and the declared ones (see the
    module docstring): either derivation alone has a blind spot the other
    covers, and a union can only ever widen, so neither can quietly shrink the
    gate. Declared paths with no file on disk are dropped — a manifest that
    does not exist cannot declare a dependency, and Cargo itself is the right
    thing to complain about a broken ``members`` entry.
    """
    manifests = set(manifest_paths_from_ls_files(tracked_listing(root)))
    manifests.update(declared_manifests(root))
    return sorted(path for path in manifests if (root / path).is_file())


def direct_edge_offenders(
    root: Path, banned: dict[str, str] | None = None
) -> dict[str, list[str]]:
    """Banned declarations mapped to their first-party manifest paths.

    The default checks all current any-edge bans and any future direct-only
    bans. ``banned`` permits a synthetic direct-only self-test without adding
    a fake package to the production ban list.
    """
    if banned is None:
        banned = BANNED_ANY_EDGE | BANNED_DIRECT_ONLY
    offenders: dict[str, list[str]] = {}

    for manifest_path in first_party_manifests(root):
        manifest = tomllib.loads((root / manifest_path).read_text(encoding="utf-8"))
        workspace, declared = manifest_banned_declarations(manifest, banned)
        for name in sorted(workspace):
            offenders.setdefault(name, []).append(
                f"{manifest_path} [workspace.dependencies]"
            )
        for name in sorted(declared):
            offenders.setdefault(name, []).append(manifest_path)

    return offenders


# ---------------------------------------------------------------------------
# The allowed half: dependency-ledger.toml's [packages] section.
# ---------------------------------------------------------------------------

LEDGER_PATH = REPO_ROOT / "dependency-ledger.toml"

# Everything above this line in the ledger is hand-authored and preserved
# byte-for-byte by --update-metrics; everything below it is re-rendered.
PACKAGES_MARKER = (
    "# ==== [packages] — rendered by `python3 scripts/check-banned-deps.py "
    "--update-metrics` ===="
)
PACKAGES_PREAMBLE = """\
# Every external (non-workspace) package in every committed Cargo.lock, with
# why it stays. `category` and `reason` (and `duplicate_reason`, required when
# a name resolves at more than one version) are authored; `metrics` is computed
# from `cargo metadata --locked --offline` and must match it exactly. This
# section is re-rendered by --update-metrics, so a comment written here is
# lost: put the rationale in `reason`.
#
# category: spec-codec | crypto | runtime | oracle | bench | binding for a
# direct dependency of a workspace member, transitive:<direct> for a package
# only reached through that direct dependency."""

DIRECT_CATEGORIES = ("spec-codec", "crypto", "runtime", "oracle", "bench", "binding")
TRANSITIVE_PREFIX = "transitive:"
UNCATEGORIZED = "UNCATEGORIZED"
METRIC_KEYS = (
    "direct",
    "exclusive_closure",
    "release",
    "wasm",
    "build_script",
    "proc_macro",
)
WASM_TARGET = "wasm32-unknown-unknown"


class MetadataError(RuntimeError):
    """Raised when ``cargo metadata`` cannot produce the resolved graph.

    Never downgraded to "skip the metrics": the ledger is only as honest as
    the graph it is checked against.
    """


@dataclass
class ResolvedGraph:
    """One lockfile's resolved package graph, as ``cargo metadata`` reports it."""

    names: dict[str, str]  # package id -> name
    external: set[str]  # ids of non-workspace packages
    workspace: set[str]
    published: set[str]  # workspace ids whose `publish` is not false
    build_script: set[str]
    proc_macro: set[str]
    # id -> [(dependency id, edge kinds)], kinds drawn from normal/build/dev.
    edges: dict[str, list[tuple[str, frozenset[str]]]] = field(default_factory=dict)


def graph_from_metadata(meta: dict) -> ResolvedGraph:
    """Build a ``ResolvedGraph`` from ``cargo metadata --format-version 1`` JSON."""
    workspace = set(meta["workspace_members"])
    names: dict[str, str] = {}
    build_script: set[str] = set()
    proc_macro: set[str] = set()
    published: set[str] = set()
    for package in meta["packages"]:
        pid = package["id"]
        names[pid] = package["name"]
        kinds = {kind for target in package["targets"] for kind in target["kind"]}
        if "custom-build" in kinds:
            build_script.add(pid)
        if "proc-macro" in kinds:
            proc_macro.add(pid)
        # `publish = false` surfaces as an empty registry list.
        if pid in workspace and package.get("publish") != []:
            published.add(pid)
    edges: dict[str, list[tuple[str, frozenset[str]]]] = {}
    for node in meta["resolve"]["nodes"]:
        edges[node["id"]] = [
            (
                dep["pkg"],
                frozenset(kind["kind"] or "normal" for kind in dep["dep_kinds"]),
            )
            for dep in node["deps"]
        ]
    return ResolvedGraph(
        names=names,
        external=set(names) - workspace,
        workspace=workspace,
        published=published,
        build_script=build_script,
        proc_macro=proc_macro,
        edges=edges,
    )


def reach(
    graph: ResolvedGraph,
    roots: Iterable[str],
    kinds: frozenset[str],
    cut_name: str | None = None,
) -> set[str]:
    """Package ids reachable from ``roots`` over edges carrying any of ``kinds``.

    ``cut_name`` removes every edge from a workspace member to a package of that
    name — the "cut the direct edge" of the exclusive-closure metric.
    """
    seen = set(roots)
    stack = list(seen)
    while stack:
        current = stack.pop()
        for dep, dep_kinds in graph.edges.get(current, ()):
            if dep in seen or not (dep_kinds & kinds):
                continue
            if (
                cut_name is not None
                and current in graph.workspace
                and graph.names[dep] == cut_name
            ):
                continue
            seen.add(dep)
            stack.append(dep)
    return seen


ALL_KINDS = frozenset({"normal", "build", "dev"})
RELEASE_KINDS = frozenset({"normal", "build"})


@dataclass
class LedgerFacts:
    """What the committed locks and their resolved graphs say, merged."""

    lock_package_count: int = 0
    # external name -> set of versions, per lock (a duplicate is two versions
    # of one name inside ONE lock, never across locks).
    versions: dict[str, set[str]] = field(default_factory=dict)
    duplicated: set[str] = field(default_factory=set)
    metrics: dict[str, dict[str, object]] = field(default_factory=dict)
    # direct name -> external names its closure reaches (any edge kind).
    closure_of_direct: dict[str, set[str]] = field(default_factory=dict)
    reachable: int = 0
    release: int = 0
    wasm: int = 0


def lock_external_versions(lock_text: str) -> tuple[int, dict[str, set[str]]]:
    """(``[[package]]`` count, external name -> versions) for one lockfile.

    External means the entry records a ``source``: workspace and path packages
    have none.
    """
    packages = tomllib.loads(lock_text).get("package", [])
    versions: dict[str, set[str]] = {}
    for package in packages:
        if "source" in package:
            versions.setdefault(package["name"], set()).add(package["version"])
    return len(packages), versions


def merge_facts(
    facts: LedgerFacts,
    lock_text: str,
    full: ResolvedGraph,
    wasm: ResolvedGraph,
) -> None:
    """Fold one lockfile and its two resolved graphs into ``facts``."""
    count, versions = lock_external_versions(lock_text)
    facts.lock_package_count += count
    for name, found in versions.items():
        facts.versions.setdefault(name, set()).update(found)
        if len(found) > 1:
            facts.duplicated.add(name)

    reachable = reach(full, full.workspace, ALL_KINDS)
    release = reach(full, full.published, RELEASE_KINDS)
    wasm_release = reach(wasm, wasm.published, RELEASE_KINDS)
    facts.reachable += len(reachable & full.external)
    facts.release += len(release & full.external)
    facts.wasm += len(wasm_release & wasm.external)

    direct = {
        full.names[dep]
        for member in full.workspace
        for dep, _ in full.edges.get(member, ())
        if dep in full.external
    }
    reachable_external = {full.names[pid] for pid in reachable & full.external}

    for name in versions:
        ids = {pid for pid in full.external if full.names[pid] == name}
        wasm_ids = {pid for pid in wasm.external if wasm.names[pid] == name}
        closure = 0
        if name in direct:
            kept = {
                full.names[pid]
                for pid in reach(full, full.workspace, ALL_KINDS, cut_name=name)
                & full.external
            }
            closure = len(reachable_external - kept)
            own = reach(full, ids, ALL_KINDS) & full.external
            facts.closure_of_direct.setdefault(name, set()).update(
                full.names[pid] for pid in own
            )
        computed = {
            "direct": name in direct,
            "exclusive_closure": closure,
            "release": bool(ids & release),
            "wasm": bool(wasm_ids & wasm_release),
            "build_script": bool(ids & full.build_script),
            "proc_macro": bool(ids & full.proc_macro),
        }
        previous = facts.metrics.get(name)
        if previous is None:
            facts.metrics[name] = computed
        else:
            # The same name in a second lock: a property held in any lock holds.
            for key in METRIC_KEYS:
                if key == "exclusive_closure":
                    previous[key] = max(previous[key], computed[key])
                else:
                    previous[key] = previous[key] or computed[key]


def cargo_metadata(manifest: Path, platform: str | None = None) -> dict:
    """``cargo metadata --locked --offline`` for one workspace root."""
    command = [
        "cargo",
        "metadata",
        "--locked",
        "--offline",
        "--format-version",
        "1",
        "--manifest-path",
        str(manifest),
    ]
    if platform is not None:
        command += ["--filter-platform", platform]
    try:
        completed = subprocess.run(
            command, capture_output=True, check=True, text=True, cwd=REPO_ROOT
        )
    except (OSError, subprocess.CalledProcessError) as exc:
        detail = getattr(exc, "stderr", "") or exc
        raise MetadataError(
            f"`{' '.join(command)}` failed, so the ledger metrics cannot be "
            f"computed: {str(detail).strip()}"
        ) from exc
    return json.loads(completed.stdout)


def collect_facts(root: Path) -> LedgerFacts:
    """Facts for every committed lockfile under ``root``."""
    facts = LedgerFacts()
    for lockfile in committed_lockfiles(root):
        manifest = lockfile.parent / "Cargo.toml"
        full = graph_from_metadata(cargo_metadata(manifest))
        wasm = graph_from_metadata(cargo_metadata(manifest, WASM_TARGET))
        merge_facts(facts, lockfile.read_text(encoding="utf-8"), full, wasm)
    return facts


def ledger_failures(packages: dict[str, dict], facts: LedgerFacts) -> list[str]:
    """Every way the ``[packages]`` table disagrees with the resolved graph."""
    failures: list[str] = []
    resolved = set(facts.versions)
    listed = set(packages)
    for name in sorted(resolved - listed):
        failures.append(
            f"FAIL: `{name}` {sorted(facts.versions[name])} is resolved by a "
            "committed Cargo.lock but has no dependency-ledger.toml entry; run "
            "`python3 scripts/check-banned-deps.py --update-metrics` and fill "
            "in its category and reason (or remove the dependency)"
        )
    for name in sorted(listed - resolved):
        failures.append(
            f"FAIL: dependency-ledger.toml lists `{name}`, which no committed "
            "Cargo.lock resolves any more; delete its entry"
        )
    for name in sorted(listed & resolved):
        entry = packages[name]
        category = entry.get("category", "")
        reason = str(entry.get("reason", "")).strip()
        metrics = facts.metrics[name]
        if category == UNCATEGORIZED or not category:
            failures.append(
                f"FAIL: `{name}` is an uncategorized skeleton in "
                "dependency-ledger.toml; give it a category and a reason"
            )
        elif metrics["direct"]:
            if category not in DIRECT_CATEGORIES:
                failures.append(
                    f"FAIL: `{name}` is a direct dependency of a workspace "
                    f"member, so its category must be one of "
                    f"{', '.join(DIRECT_CATEGORIES)} (found `{category}`)"
                )
        elif not category.startswith(TRANSITIVE_PREFIX):
            failures.append(
                f"FAIL: `{name}` is not a direct dependency of any workspace "
                f"member, so its category must be transitive:<direct> "
                f"(found `{category}`)"
            )
        else:
            via = category[len(TRANSITIVE_PREFIX) :]
            if name not in facts.closure_of_direct.get(via, set()):
                failures.append(
                    f"FAIL: `{name}` is categorized {category}, but `{via}` is "
                    "not a direct dependency whose closure reaches it"
                )
        if not reason:
            failures.append(f"FAIL: `{name}` has no reason in dependency-ledger.toml")
        duplicate_reason = str(entry.get("duplicate_reason", "")).strip()
        if name in facts.duplicated and not duplicate_reason:
            failures.append(
                f"FAIL: `{name}` resolves at {len(facts.versions[name])} versions "
                f"{sorted(facts.versions[name])} in one lock with no "
                "`duplicate_reason` recorded"
            )
        if name not in facts.duplicated and duplicate_reason:
            failures.append(
                f"FAIL: `{name}` records a duplicate_reason but resolves at a "
                "single version; delete the stale reason"
            )
        recorded = entry.get("metrics", {})
        if recorded != metrics:
            drifted = sorted(
                key
                for key in set(METRIC_KEYS) | set(recorded)
                if recorded.get(key) != metrics.get(key)
            )
            failures.append(
                f"FAIL: `{name}` metrics drifted ({', '.join(drifted)}): ledger "
                f"says {recorded}, the graph says {metrics}; run "
                "`python3 scripts/check-banned-deps.py --update-metrics`"
            )
    return failures


BARE_KEY_RE = re.compile(r"[A-Za-z0-9_-]+")


def toml_string(value: str) -> str:
    """A TOML basic string."""
    escaped = (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
        .replace("\t", "\\t")
    )
    return f'"{escaped}"'


def toml_value(value: object) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    return toml_string(str(value))


def render_packages(packages: dict[str, dict]) -> str:
    """The ``[packages]`` section, sorted by name, byte-identical per input."""
    lines = [PACKAGES_MARKER, PACKAGES_PREAMBLE, ""]
    for name in sorted(packages):
        entry = packages[name]
        key = name if BARE_KEY_RE.fullmatch(name) else toml_string(name)
        lines.append(f"[packages.{key}]")
        lines.append(f"category = {toml_string(entry.get('category', UNCATEGORIZED))}")
        lines.append(f"reason = {toml_string(entry.get('reason', ''))}")
        if entry.get("duplicate_reason"):
            lines.append(
                f"duplicate_reason = {toml_string(entry['duplicate_reason'])}"
            )
        metrics = entry.get("metrics", {})
        rendered = ", ".join(
            f"{key} = {toml_value(metrics[key])}" for key in METRIC_KEYS if key in metrics
        )
        lines.append(f"metrics = {{ {rendered} }}")
        lines.append("")
    return "\n".join(lines)


def split_ledger(text: str) -> str:
    """The hand-authored head of the ledger (everything above the marker)."""
    head, _, _ = text.partition(PACKAGES_MARKER)
    return head.rstrip("\n") + "\n\n"


def updated_packages(
    packages: dict[str, dict], facts: LedgerFacts
) -> tuple[dict[str, dict], list[str], list[str]]:
    """(new ``[packages]`` table, skeleton names added, stale names kept).

    A vanished package's entry is kept verbatim and reported, never silently
    dropped: the plain gate then fails on it until someone deletes it on
    purpose.
    """
    result: dict[str, dict] = {}
    skeletons: list[str] = []
    stale: list[str] = []
    for name in sorted(set(packages) | set(facts.versions)):
        if name not in facts.versions:
            result[name] = packages[name]
            stale.append(name)
            continue
        entry = dict(packages.get(name, {}))
        if name not in packages:
            entry = {"category": UNCATEGORIZED, "reason": ""}
            skeletons.append(name)
        entry["metrics"] = dict(facts.metrics[name])
        result[name] = entry
    return result, skeletons, stale


def load_ledger(path: Path) -> tuple[str, dict[str, dict]]:
    text = path.read_text(encoding="utf-8")
    return text, tomllib.loads(text).get("packages", {})


def update_metrics() -> int:
    facts = collect_facts(REPO_ROOT)
    text, packages = load_ledger(LEDGER_PATH)
    new_packages, skeletons, stale = updated_packages(packages, facts)
    LEDGER_PATH.write_text(
        split_ledger(text) + render_packages(new_packages), encoding="utf-8"
    )
    for name in skeletons:
        print(f"NEW: `{name}` added as an {UNCATEGORIZED} skeleton; categorize it")
    for name in stale:
        print(
            f"STALE: `{name}` is no longer resolved by any committed Cargo.lock; "
            "its entry was kept — delete it"
        )
    print(
        f"OK: rewrote {len(new_packages)} dependency-ledger.toml entries "
        f"({len(skeletons)} new, {len(stale)} stale)"
    )
    return 0


def report_lines(facts: LedgerFacts) -> list[str]:
    external = len(facts.versions)
    rows = [
        ("lockfile packages ([[package]], all committed locks)", facts.lock_package_count),
        ("  external (non-workspace) lockfile packages", external),
        ("reachable non-workspace packages", facts.reachable),
        ("release graph (normal+build from published crates)", facts.release),
        (f"{WASM_TARGET} release graph", facts.wasm),
        ("dev-only (reachable, outside the release graph)", facts.reachable - facts.release),
    ]
    width = max(len(label) for label, _ in rows)
    return [f"{label:<{width}}  {value:>4}" for label, value in rows]


def fixture_metadata(dev_on_wasm: bool = True) -> dict:
    """A miniature ``cargo metadata`` document: published member ``a``,
    unpublished member ``t``; ``a`` depends on ``x`` (which pulls ``y``) and,
    dev-only, on ``d`` and ``t``; ``t`` depends on ``m``, a proc macro with a build
    script. The wasm variant drops ``d``'s edge like a target filter would."""

    def package(pid: str, publish: object = None, kinds: tuple = ("lib",)) -> dict:
        return {
            "id": pid,
            "name": pid,
            "publish": publish,
            "targets": [{"kind": [kind]} for kind in kinds],
        }

    def dep(pid: str, kind: str | None = None) -> dict:
        return {"pkg": pid, "dep_kinds": [{"kind": kind, "target": None}]}

    a_deps = [dep("x"), dep("t", "dev")]
    if dev_on_wasm:
        a_deps.append(dep("d", "dev"))
    return {
        "workspace_members": ["a", "t"],
        "packages": [
            package("a"),
            package("t", publish=[]),
            package("x"),
            package("y", kinds=("lib", "custom-build")),
            package("d"),
            package("m", kinds=("proc-macro", "custom-build")),
        ],
        "resolve": {
            "nodes": [
                {"id": "a", "deps": a_deps},
                {"id": "t", "deps": [dep("m")]},
                {"id": "x", "deps": [dep("y")]},
                {"id": "y", "deps": []},
                {"id": "d", "deps": []},
                {"id": "m", "deps": []},
            ]
        },
    }


FIXTURE_REGISTRY = 'source = "registry+https://github.com/rust-lang/crates.io-index"'


def fixture_lock(extra: Iterable[tuple[str, str]] = ()) -> str:
    entries = [("a", None), ("t", None)] + [
        (name, "1.0.0") for name in ("x", "y", "d", "m")
    ]
    blocks = []
    for name, version in entries:
        if version is None:
            blocks.append(f'[[package]]\nname = "{name}"\nversion = "0.1.0"\n')
        else:
            blocks.append(
                f'[[package]]\nname = "{name}"\nversion = "{version}"\n'
                f"{FIXTURE_REGISTRY}\n"
            )
    for name, version in extra:
        blocks.append(
            f'[[package]]\nname = "{name}"\nversion = "{version}"\n{FIXTURE_REGISTRY}\n'
        )
    return "version = 4\n\n" + "\n".join(blocks)


FIXTURE_CATEGORIES = {
    "x": ("runtime", "the fixture's direct runtime dependency"),
    "y": ("transitive:x", "reached only through x"),
    "d": ("oracle", "the fixture's dev-only oracle"),
    "m": ("runtime", "the unpublished member's proc macro"),
}


def fixture_facts(lock_text: str) -> LedgerFacts:
    facts = LedgerFacts()
    merge_facts(
        facts,
        lock_text,
        graph_from_metadata(fixture_metadata()),
        graph_from_metadata(fixture_metadata(dev_on_wasm=False)),
    )
    return facts


def fixture_ledger(facts: LedgerFacts) -> dict[str, dict]:
    packages, _, _ = updated_packages({}, facts)
    for name, (category, reason) in FIXTURE_CATEGORIES.items():
        packages[name]["category"] = category
        packages[name]["reason"] = reason
    return packages


def ledger_self_test() -> list[str]:
    """Self-test cases for the allowed half (the dependency ledger)."""
    failures: list[str] = []
    facts = fixture_facts(fixture_lock())
    packages = fixture_ledger(facts)

    # --- metrics are what the fixture graph says.
    expected_metrics = {
        "x": (True, 2, True, True, False, False),
        "y": (False, 0, True, True, True, False),
        "d": (True, 1, False, False, False, False),
        "m": (True, 1, False, False, True, True),
    }
    for name, values in expected_metrics.items():
        got = tuple(facts.metrics[name][key] for key in METRIC_KEYS)
        if got != values:
            failures.append(f"fixture metrics for {name}: got {got}, expected {values}")
    counts = (facts.lock_package_count, facts.reachable, facts.release, facts.wasm)
    if counts != (6, 4, 2, 2):
        failures.append(f"fixture report counts {counts}, expected (6, 4, 2, 2)")

    # --- the complete, categorized fixture ledger passes (the valid neighbour
    #     of every refusal below).
    clean = ledger_failures(packages, facts)
    if clean:
        failures.append(f"a complete fixture ledger was refused: {clean}")

    # --- an injected unlisted package fails.
    injected = fixture_facts(fixture_lock([("intruder", "0.1.0")]))
    if not any("`intruder`" in m and "no dependency-ledger.toml entry" in m
               for m in ledger_failures(packages, injected)):
        failures.append("an injected unlisted package was not flagged")

    # --- a listed package no lock resolves fails.
    ghost = dict(packages, ghost={"category": "runtime", "reason": "gone"})
    if not any("`ghost`" in m for m in ledger_failures(ghost, facts)):
        failures.append("a listed but absent package was not flagged")

    # --- a duplicate version fails without a reason and passes with one.
    duplicated = fixture_facts(fixture_lock([("y", "2.0.0")]))
    if not any("duplicate_reason" in m for m in ledger_failures(packages, duplicated)):
        failures.append("a duplicate version without a reason was not flagged")
    reasoned = {name: dict(entry) for name, entry in packages.items()}
    reasoned["y"]["duplicate_reason"] = "x's two majors resolve side by side"
    if ledger_failures(reasoned, duplicated):
        failures.append(
            f"a duplicate with a recorded reason was refused: "
            f"{ledger_failures(reasoned, duplicated)}"
        )
    if not any("stale reason" in m for m in ledger_failures(reasoned, facts)):
        failures.append("a duplicate_reason with no duplicate was not flagged")

    # --- metrics drift fails.
    drifted = {name: dict(entry) for name, entry in packages.items()}
    drifted["x"]["metrics"] = dict(drifted["x"]["metrics"], wasm=False)
    if not any("drifted" in m for m in ledger_failures(drifted, facts)):
        failures.append("metrics drift was not flagged")

    # --- a skeleton, a wrong transitive owner and a mis-kinded category fail.
    for name, category, needle in (
        ("x", UNCATEGORIZED, "uncategorized"),
        ("y", "transitive:d", "closure reaches it"),
        ("y", "runtime", "must be transitive"),
        ("x", "transitive:x", "must be one of"),
    ):
        broken = {key: dict(entry) for key, entry in packages.items()}
        broken[name]["category"] = category
        if not any(needle in m for m in ledger_failures(broken, facts)):
            failures.append(f"category {category!r} on {name} was not flagged")

    # --- --update-metrics keeps and reports a vanished entry, adds a flagged
    #     skeleton for a new one, and renders deterministically.
    updated, skeletons, stale = updated_packages(ghost, injected)
    if skeletons != ["intruder"] or stale != ["ghost"] or "ghost" not in updated:
        failures.append(
            f"update-metrics skeletons={skeletons} stale={stale}; expected "
            "['intruder'] and a kept ['ghost']"
        )
    if updated["intruder"]["category"] != UNCATEGORIZED:
        failures.append("a new package's skeleton is not flagged UNCATEGORIZED")
    rendered = render_packages(packages)
    if rendered != render_packages(dict(reversed(list(packages.items())))):
        failures.append("the rendered [packages] section depends on input order")
    if tomllib.loads(rendered).get("packages") != packages:
        failures.append("the rendered [packages] section does not round-trip")

    # --- the real tree passes.
    try:
        real_facts = collect_facts(REPO_ROOT)
        _, real_packages = load_ledger(LEDGER_PATH)
    except (TrackedFileDiscoveryError, MetadataError, OSError) as exc:
        failures.append(f"could not check the real dependency ledger: {exc}")
    else:
        real = ledger_failures(real_packages, real_facts)
        if real:
            failures.append(f"the real tree fails the ledger: {real[:3]}")
    return failures


def self_test() -> int:
    failures: list[str] = []

    # --- pre-existing real behavior: exact-name tier-1 match in a plain lock.
    dirty = 'name = "serde"\nname = "oxilangtag"\nversion = "0.1.6"\n'
    clean = 'name = "serde"\nname = "purrdf-iri"\n'
    if any_edge_offenders(dirty) != ["oxilangtag"]:
        failures.append("a banned package in the lock was not flagged")
    if any_edge_offenders(clean):
        failures.append("a clean lock was flagged")

    # --- (A) tier-1 any-edge ban still fails on a lock containing petgraph.
    petgraph_lock = 'name = "serde"\nname = "petgraph"\nversion = "0.6.5"\n'
    if any_edge_offenders(petgraph_lock) != ["petgraph"]:
        failures.append("petgraph in the lock was not flagged (tier-1 regression)")

    # --- (A) the independent manifest scan catches any-edge bans on direct
    #     declarations, including the former direct-only hex ban, in every
    #     dependency table and through a renamed Cargo dependency.
    direct_variants = {
        "top-level dependencies": {"dependencies": {"hex": {"version": "0.4"}}},
        "dev-dependencies": {"dev-dependencies": {"hex": {"version": "0.4"}}},
        "build-dependencies": {"build-dependencies": {"hex": {"version": "0.4"}}},
        "renamed dependency": {
            "dependencies": {"renamed_hex": {"package": "hex", "version": "0.4"}}
        },
        "target-gated dependencies": {
            "target": {
                "cfg(not(target_arch = \"wasm32\"))": {
                    "dependencies": {"hex": {"version": "0.4"}}
                }
            }
        },
    }
    for label, manifest in direct_variants.items():
        if manifest_banned_declarations(manifest, BANNED_ANY_EDGE)[1] != {"hex"}:
            failures.append(f"direct hex dependency ({label}) was not flagged")

    # A synthetic name keeps the direct-only scanner contract covered even
    # though no production dependency has an authorized direct-only exception.
    synthetic = "synthetic-direct-only"
    synthetic_manifest = {
        "dependencies": {"alias": {"package": synthetic, "version": "1"}},
        "workspace": {"dependencies": {"renamed": {"package": synthetic, "version": "1"}}},
    }
    if manifest_banned_declarations(synthetic_manifest, {synthetic: "test"}) != (
        {synthetic},
        {synthetic},
    ):
        failures.append("a renamed synthetic direct-only dependency was missed")

    # A transitive hex is now forbidden too: the root and fixture locks have
    # no such package, so the removal applies to the complete closure.
    transitive_only_lock = (
        'name = "serde"\n'
        'name = "some-third-party-crate"\n'
        "dependencies = [\n"
        ' "hex",\n'
        "]\n"
        'name = "hex"\n'
        'version = "0.4.3"\n'
    )
    clean_member_manifest = {"dependencies": {"serde": {"workspace": True}}}
    if direct_dependency_names(clean_member_manifest) & BANNED_ANY_EDGE.keys():
        failures.append("a clean member manifest was flagged")
    if any_edge_offenders(transitive_only_lock) != ["hex"]:
        failures.append("transitive hex was not flagged by the any-edge ban")
    if any_edge_offenders('name = "rand_core"\n') != ["rand_core"]:
        failures.append("transitive rand_core was not flagged by the any-edge ban")
    if manifest_banned_declarations(
        {"patch": {"crates-io": {"alias": {"package": "hex"}}}}, BANNED_ANY_EDGE
    )[1] != {"hex"}:
        failures.append("a renamed patch declaration was not flagged")
    if manifest_banned_declarations(
        {"replace": {"hex:0.4.3": {"path": "../hex"}}}, BANNED_ANY_EDGE
    )[1] != {"hex"}:
        failures.append("a replace declaration was not flagged")

    # --- (B) substitution: a [[patch.unused]] block naming a banned package.
    patch_unused_lock = (
        'name = "serde"\n\n'
        "[[patch.unused]]\n"
        'name = "petgraph"\n'
        'version = "0.6.5"\n'
        'source = "registry+https://github.com/rust-lang/crates.io-index"\n\n'
        '[[package]]\nname = "purrdf-slice"\nversion = "2.0.2"\n'
    )
    if substitution_offenders(patch_unused_lock) != ["petgraph"]:
        failures.append("a [[patch.unused]] banned package was not flagged")

    # --- (B) substitution: a `replace = "name:version"` key redirecting a
    #     banned package (a construct any_edge_offenders alone cannot see,
    #     since the name sits inside a value string, not a `name = "..."`
    #     line).
    replace_lock = (
        'name = "oxigraph"\n'
        'version = "0.3.0"\n'
        'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
        'replace = "oxigraph:0.4.0"\n'
    )
    if substitution_offenders(replace_lock) != ["oxigraph"]:
        failures.append("a replace= override of a banned package was not flagged")

    # --- (B) substitution scan stays quiet on an unrelated patch/replace.
    benign_lock = (
        "[[patch.unused]]\n"
        'name = "serde"\n'
        'version = "1.0.0"\n'
        'source = "registry+https://github.com/rust-lang/crates.io-index"\n'
    )
    if substitution_offenders(benign_lock):
        failures.append("an unrelated [[patch.unused]] entry was flagged")

    # --- (C) discovery: a `git ls-files -z` listing is filtered down to real
    #     lockfiles by BASENAME, so nested and excluded-directory locks are
    #     found while near-miss names are left alone.
    listing = "\0".join(
        [
            "Cargo.toml",
            "Cargo.lock",
            "crates/excluded-root/Cargo.lock",
            "crates/gts/fuzz/Cargo.lock",
            # Near misses that are NOT lockfiles and must not be scanned.
            "docs/Cargo.lock.md",
            "vendor/Cargo.lock.bak",
            "notes/not-a-Cargo.lock-really.txt",
            "scripts/check-banned-deps.py",
        ]
    )
    discovered = lockfile_paths_from_ls_files(listing)
    expected_discovered = [
        "Cargo.lock",
        "crates/excluded-root/Cargo.lock",
        "crates/gts/fuzz/Cargo.lock",
    ]
    if discovered != expected_discovered:
        failures.append(
            f"lockfile discovery picked {discovered}, expected {expected_discovered}"
        )
    if lockfile_paths_from_ls_files(""):
        failures.append("an empty git listing produced lockfile paths")

    # --- (C) THE OVER-REFUSAL CHECK for the widened scan: matching on the
    #     basename means the wider scan reads lockfiles and ONLY lockfiles. The
    #     repository legitimately names `oxilangtag` in prose and in a frozen
    #     differential-vector corpus (`crates/iri/tests/PROVENANCE.md`,
    #     `crates/iri/tests/langtag_differential_vectors.txt`), which record
    #     which engine a vector came from. Those are not `Cargo.lock`, so
    #     scanning every committed lock must never reach them.
    prose_paths = "\0".join(
        [
            "crates/iri/tests/PROVENANCE.md",
            "crates/iri/tests/langtag_differential_vectors.txt",
        ]
    )
    if lockfile_paths_from_ls_files(prose_paths):
        failures.append(
            "a non-lockfile that legitimately names a banned crate was picked "
            "up by lockfile discovery (over-refusal)"
        )

    # --- (C) a tier-1 failure names the lockfile it came from, so a violation
    #     in a non-root lock points at the right file.
    nested_failures = lock_failures(
        "crates/excluded-root/Cargo.lock",
        'name = "oxilangtag"\nversion = "0.1.6"\n',
    )
    if len(nested_failures) != 1 or "crates/excluded-root/Cargo.lock" not in (
        nested_failures[0]
    ):
        failures.append(
            f"a non-root lock violation did not name its file: {nested_failures}"
        )
    if lock_failures("Cargo.lock", 'name = "serde"\n'):
        failures.append("a clean lock produced a failure message")

    # --- (C) discovery works on THIS repository and finds more than the root
    #     lock. Pins the regression this rule was written for: a gate that
    #     quietly narrows back to `REPO_ROOT / "Cargo.lock"` fails here.
    try:
        repo_locks = committed_lockfiles(REPO_ROOT)
    except TrackedFileDiscoveryError as exc:
        failures.append(f"could not discover this repository's lockfiles: {exc}")
    else:
        relative = sorted(
            lock.relative_to(REPO_ROOT).as_posix() for lock in repo_locks
        )
        if "Cargo.lock" not in relative:
            failures.append(f"the root Cargo.lock was not discovered: {relative}")
        # An independent derivation of the same set: git's own pathspec match
        # for a `Cargo.lock` at any depth. Discovery that narrowed back to the
        # root lock would disagree with it the moment a second lock is committed.
        pathspec = subprocess.run(
            ["git", "ls-files", "-z", "--", "Cargo.lock", ":(glob)**/Cargo.lock"],
            cwd=REPO_ROOT,
            capture_output=True,
            check=False,
        )
        if pathspec.returncode != 0:
            failures.append(
                "git ls-files could not list this repository's lockfiles: "
                f"{pathspec.stderr.decode(errors='replace').strip()}"
            )
        else:
            listed = sorted(
                path for path in pathspec.stdout.decode().split("\0") if path
            )
            if relative != listed:
                failures.append(
                    f"lockfile discovery found {relative}, but git lists {listed}"
                )
        for lock in repo_locks:
            if not lock.is_file():
                failures.append(f"discovered lockfile does not exist: {lock}")

    # --- (D) tier-2 manifest discovery: the same basename filter applied to
    #     `Cargo.toml`, so a manifest in an excluded root is found and a
    #     near-miss name is not.
    manifest_listing = "\0".join(
        [
            "Cargo.toml",
            "crates/iri/Cargo.toml",
            "crates/excluded-root/Cargo.toml",
            "crates/gts/fuzz/Cargo.toml",
            "docs/Cargo.toml.md",
            "vendor/Cargo.toml.orig",
            "scripts/check-banned-deps.py",
        ]
    )
    discovered_manifests = manifest_paths_from_ls_files(manifest_listing)
    expected_manifests = [
        "Cargo.toml",
        "crates/excluded-root/Cargo.toml",
        "crates/gts/fuzz/Cargo.toml",
        "crates/iri/Cargo.toml",
    ]
    if discovered_manifests != expected_manifests:
        failures.append(
            f"manifest discovery picked {discovered_manifests}, expected "
            f"{expected_manifests}"
        )

    # --- (D) `.`-valued and trailing-`.` member entries fold to one path, so
    #     the single-package workspace an excluded fuzz root declares
    #     (`members = ["."]`) does not become a second, unreadable entry.
    if manifest_path_for("crates/gts/fuzz/.") != "crates/gts/fuzz/Cargo.toml":
        failures.append("a trailing `.` member directory did not normalize")
    if manifest_path_for(".") != "Cargo.toml":
        failures.append("the `.` member directory did not normalize to the root")
    fuzz_members = declared_member_manifests(
        REPO_ROOT, "crates/gts/fuzz", {"members": ["."]}
    )
    if fuzz_members != {"crates/gts/fuzz/Cargo.toml"}:
        failures.append(
            "an excluded root's own `members` table did not resolve relative "
            f"to that root: {sorted(fuzz_members)}"
        )

    # --- (D) THE REGRESSION THIS TIER-2 WIDENING EXISTS FOR: the excluded
    #     first-party roots are git-tracked but are NOT workspace members, so
    #     a members-only scan cannot see them. A direct banned package in
    #     either one passed the gate silently before they were included.
    excluded_roots = [
        "crates/gts/fuzz/Cargo.toml",
    ]
    try:
        scanned = first_party_manifests(REPO_ROOT)
    except TrackedFileDiscoveryError as exc:
        failures.append(f"could not discover this repository's manifests: {exc}")
    else:
        root_manifest = tomllib.loads(
            (REPO_ROOT / "Cargo.toml").read_text(encoding="utf-8")
        )
        members = root_manifest["workspace"]["members"]
        for member in members:
            if f"{member}/Cargo.toml" not in scanned:
                failures.append(f"workspace member {member} is not scanned by tier 2")
        for excluded_root in excluded_roots:
            if excluded_root not in scanned:
                failures.append(
                    f"{excluded_root} is a committed first-party manifest outside "
                    "`[workspace] members` and is not scanned by tier 2; a direct "
                    "ban there would pass silently"
                )
        if len(scanned) <= len(members) + 1:
            failures.append(
                "tier 2 scans no more manifests than the workspace members plus "
                f"the root; the excluded roots are what it was widened for "
                f"(found {len(scanned)})"
            )

        # --- (D) Manifest discovery must return only readable manifests;
        #     the independent lock scan handles the transitive closure.
        for scanned_path in scanned:
            if scanned_path.rsplit("/", 1)[-1] != "Cargo.toml":
                failures.append(
                    f"tier 2 was handed a non-manifest to read: {scanned_path}"
                )
            else:
                try:
                    tomllib.loads(
                        (REPO_ROOT / scanned_path).read_text(encoding="utf-8")
                    )
                except (OSError, tomllib.TOMLDecodeError) as exc:
                    failures.append(
                        f"a scanned first-party manifest is unreadable: "
                        f"{scanned_path}: {exc}"
                    )

    # --- the allowed half: the dependency ledger.
    failures.extend(ledger_self_test())

    if failures:
        for message in failures:
            print(f"SELF-TEST FAIL: {message}")
        return 1
    print("OK: check-banned-deps self-test (the gate can still fail)")
    return 0


def main() -> int:
    arguments = sys.argv[1:]
    if "--self-test" in arguments:
        return self_test()
    try:
        if "--update-metrics" in arguments:
            return update_metrics()
        if "--report" in arguments:
            for line in report_lines(collect_facts(REPO_ROOT)):
                print(line)
            return 0
    except (TrackedFileDiscoveryError, MetadataError) as exc:
        print(f"FAIL: {exc}")
        return 1

    failures: list[str] = []

    try:
        lockfiles = committed_lockfiles(REPO_ROOT)
    except TrackedFileDiscoveryError as exc:
        print(f"FAIL: {exc}")
        return 1
    if not lockfiles:
        print(
            "FAIL: no committed Cargo.lock found; the any-edge ban has nothing "
            "to read, which is a broken gate rather than a clean workspace"
        )
        return 1

    for lockfile in lockfiles:
        display_path = lockfile.relative_to(REPO_ROOT).as_posix()
        failures.extend(
            lock_failures(display_path, lockfile.read_text(encoding="utf-8"))
        )

    direct = direct_edge_offenders(REPO_ROOT)
    for name in sorted(direct):
        replacement = (BANNED_ANY_EDGE | BANNED_DIRECT_ONLY)[name]
        files = ", ".join(direct[name])
        failures.append(
            f"FAIL: `{name}` is a direct dependency of {files}; it was "
            f"replaced by {replacement} and must not be reintroduced as a "
            "first-party edge"
        )

    try:
        facts = collect_facts(REPO_ROOT)
        _, packages = load_ledger(LEDGER_PATH)
    except (MetadataError, OSError, tomllib.TOMLDecodeError) as exc:
        print(f"FAIL: {exc}")
        return 1
    failures.extend(ledger_failures(packages, facts))

    if failures:
        for message in failures:
            print(message)
        return 1

    total = len(BANNED_ANY_EDGE) + len(BANNED_DIRECT_ONLY)
    print(f"OK: none of the {total} replaced dependencies re-entered the workspace")
    print(
        f"OK: all {len(packages)} external packages are in dependency-ledger.toml "
        "with current metrics"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
