#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Instantiate WatDiv's 20 basic query templates DETERMINISTICALLY, auditably.

The templates ship inside ``watdiv_v06.tar`` under ``watdiv/testsuite/`` and the
frozen 10M dataset ships inside ``watdiv.10M.tar.bz2``; both are fetched by digest
by ``scripts/benchmark-acquire.py`` and NEITHER is vendored — WatDiv grants use
with citation, not redistribution.

A template is not a query. Each one carries ``#mapping`` directives and ``%vN%``
placeholders::

    #mapping v1 wsdbm:Website uniform
    SELECT ?v0 ?v2 ?v3 WHERE {
        ?v0  wsdbm:subscribes  %v1% .
        ?v2  sorg:caption      ?v3 .
        ?v0  wsdbm:likes       ?v2 .
    }

Something must choose which ``wsdbm:Website`` the placeholder becomes, and that
choice is the whole difference between a workload two people can compare and one
they cannot.

WHY THIS PROGRAM EXISTS: UPSTREAM'S INSTANTIATION CANNOT BE REPRODUCED
======================================================================

WatDiv v0.6's own query instantiator draws from the same time-seeded generators
its data generator uses — ``boost::mt19937(time(0))``, ``srand(time(NULL))``,
``std::random_device`` — and the tool exposes no seed flag of any kind. Two runs
therefore emit two different query sets, and neither can be regenerated. A
published WatDiv number whose queries came out of that instantiator is not
reproducible by the person reading it.

The DATASET side of that problem is solved by pinning a frozen OUTPUT rather than
a generation run (see ``scripts/benchmark-acquire.py``). Once the dataset is
digest-pinned, the candidate set behind every ``#mapping`` is FIXED — it is a
property of those exact bytes. So the instantiation can be made a pure function
of the dataset and a seed, and this program makes it one:

    the same frozen dataset + the same seed produce BYTE-IDENTICAL queries.

This is a deliberate improvement on upstream, not a reimplementation of it. It is
also a difference a reader must be told about, so ``provenance.txt`` records the
seed, the dataset digest, every candidate chosen, and how many candidates it was
chosen out of. **A query set generated from a different seed is a DIFFERENT
WORKLOAD**, and comparing a number taken under one seed against a number taken
under another compares two workloads rather than two engines.

HOW A CANDIDATE SET IS BUILT, AND HOW IT IS CHECKED
===================================================

A mapping names a type, such as ``wsdbm:Website``. WatDiv names its entities
``<namespace><Type><decimal>``, so the candidates for a type are the IRIs of that
shape occurring in the frozen dataset in subject or object position. They are
collected in ONE pass and then sorted into a canonical order — by UTF-8 bytes —
so the selection never depends on the order the file happened to mention them in.

That rule is not taken on trust. The frozen tarball ships ``saved.txt``, the
generator's OWN record of how many entities of each type it emitted, and every
scraped candidate set is CROSS-CHECKED against it. A disagreement is a hard
failure naming the type, the scraped count and the declared count: it means
either the naming rule or the dataset is not what this program believes, and
instantiating against a candidate set that might be incomplete would silently
bias every query built from it.

A mapping naming a type ``saved.txt`` does not declare is likewise a hard failure.
Such a type could still be scraped, but with nothing to check the scrape against,
so the candidate set would be unverified rather than merely unusual.

THE SELECTION FUNCTION
======================

``splitmix64`` over a pinned seed, exactly as ``crates/bench/src/lib.rs`` does it:
arithmetic only, no RNG syscalls, no platform floats, no iteration-order
dependence. Each mapping draws from its own stream, tagged by a pinned hash of
``<template>/<variable>``, so no two mappings are correlated and adding or
removing a template cannot shift the choice any other template makes.

``uniform`` is implemented as ACTUALLY uniform: the draw is rejected and retaken
when it falls in the short tail that modulo would fold unevenly. The bias would
have been below one part in 2^44 at these candidate counts and unobservable, but
"uniform" is a claim the mappings make and it costs four lines to make it true
rather than nearly true.

A DISTRIBUTION THIS PROGRAM DOES NOT IMPLEMENT IS A HARD FAILURE NAMING IT.
Falling back to ``uniform`` would produce a query set that looks fine, runs fine,
and silently is not the workload the template asked for.

PURE BGP — NO ENTAILMENT, AND THAT IS NOT AN OMISSION
=====================================================

Every one of the 20 templates is a basic graph pattern: triple patterns and
nothing else. No ``OPTIONAL``, no ``UNION``, no ``FILTER``, no ``MINUS``, no
``GRAPH``, no subquery. WatDiv stresses *structure and selectivity* — how a
planner handles stars, chains and snowflakes over a skewed dataset — and it needs
no inference whatsoever. ``--self-test`` ASSERTS that emptiness rather than
asserting it in prose, so a future template that smuggled in a ``FILTER`` would
fail here instead of quietly changing what the lane measures.

This is the exact opposite of LUBM, where eleven of fourteen queries have no
answers at all without an entailment regime. The two workloads are therefore NOT
comparable with each other: a WatDiv row and a LUBM row measure different
questions over different data under different regimes. Each is comparable only
against another engine answering THE SAME query under the same conditions.
"""

import argparse
import hashlib
import re
import sys
import tempfile
from pathlib import Path
from typing import NamedTuple

REPO_ROOT = Path(__file__).resolve().parent.parent

# Everything this lane reads and writes lives under `target/`, which is ignored.
# WatDiv's use grant is not a redistribution grant, so no byte of it — nor any
# query mechanically derived from its templates — is ever a tracked file.
DEFAULT_OUT = REPO_ROOT / "target" / "watdiv"

# The pinned tarball the templates and the namespace table come from, used
# directly by `--self-test` so the self-test needs no extraction step and no
# multi-gigabyte dataset.
TOOLKIT_TAR = REPO_ROOT / "target" / "bench-artifacts" / "watdiv_v06.tar"

# WatDiv publishes exactly twenty BASIC templates: three complex (C), five
# snowflake (F), five linear (L) and seven star (S). `linear_incremental/` and
# `linear_mixed/` are separate scaling studies with their own README and are NOT
# part of the twenty; they live in subdirectories, which is why only the top
# level of the testsuite directory is read.
EXPECTED_TEMPLATES = 20

_U64 = (1 << 64) - 1


# ── The selection function ──────────────────────────────────────────────────────


def splitmix64(state: int) -> int:
    """The classic public-domain mixing step, identical to the Rust bench crate's.

    Deterministic, allocation-free and identical on every target. The reference
    output for state 0 is pinned in ``--self-test``, so a mistyped constant is a
    test failure rather than a silently different — but still perfectly
    reproducible — workload.
    """
    z = (state + 0x9E3779B97F4A7C15) & _U64
    z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & _U64
    z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & _U64
    return z ^ (z >> 31)


def draw(seed: int, tag: int, index: int) -> int:
    """Mix a seed, a stream tag and an index into one draw (the house idiom)."""
    return splitmix64((seed & _U64) ^ splitmix64((tag & _U64) ^ splitmix64(index & _U64)))


# The one stream tag this program uses. It exists so that a future second kind of
# decision cannot accidentally share a stream with candidate selection.
TAG_MAPPING = 0x7AD1_0001

# The odd stride a rejected draw walks by. Odd, so repeated addition visits the
# whole 64-bit space rather than a subgroup of it.
_RETRY_STRIDE = 0x9E3779B97F4A7C15


def stream_of(template: str, variable: str) -> int:
    """The pinned per-mapping stream index for ``<template>/<variable>``.

    A cryptographic digest is used rather than ``hash()`` because Python's
    ``hash()`` of a string is SALTED PER PROCESS: it would make every run's
    queries different while every line of this file still claimed determinism.
    Only the leading eight bytes are used; nothing here is a security property.
    """
    material = f"{template}/{variable}".encode("utf-8")
    return int.from_bytes(hashlib.sha256(material).digest()[:8], "big")


def uniform_index(seed: int, stream: int, count: int) -> tuple[int, int]:
    """Choose one of ``count`` candidates uniformly. Returns ``(index, draw)``.

    Plain ``draw % count`` folds the top of the 64-bit range unevenly, favouring
    the first ``2**64 % count`` candidates. The draw is therefore rejected and
    retaken when it lands in that tail. At WatDiv's candidate counts the tail is
    vanishingly small — no retry has ever been observed — but the mappings say
    ``uniform`` and this makes the word exactly true.
    """
    if count <= 0:
        sys.exit("FAIL: uniform_index called with an empty candidate set")
    limit = (1 << 64) - ((1 << 64) % count)
    attempt = 0
    while True:
        value = draw(seed, TAG_MAPPING, stream + attempt * _RETRY_STRIDE)
        if value < limit:
            return value % count, value
        attempt += 1


# ── Templates ───────────────────────────────────────────────────────────────────

_MAPPING = re.compile(r"^#mapping\s+(\S+)\s+(\S+)\s+(\S+)\s*$")
_PLACEHOLDER = re.compile(r"%(\w+)%")
_PREFIXED = re.compile(r"(?<![\w:<])([A-Za-z][\w.-]*):[\w.%-]+")

# An angle-bracketed IRI, or a quoted literal INCLUDING its ECHAR escapes.
#
# The escapes are the whole point. A naive `"[^"\n]*"` closes the literal at the
# first `\"` inside it and leaves the remainder of that literal standing in the
# text the prefix scan then reads -- so a perfectly legal
# `"say \"nosuch:name\""` had `nosuch:name` scanned as a prefixed name and the
# template was REFUSED. SPARQL 1.1 §19.7 admits `\"` in a STRING_LITERAL2 via
# ECHAR, so that body is legal and refusing it is an over-refusal: the mirror of
# the silent drop the prefix check replaced.
_LITERAL_OR_IRI = re.compile(
    r"""<[^>\s]*>          # an IRI reference
      | "(?:\\.|[^"\\\n])*"   # a double-quoted literal, escapes included
      | '(?:\\.|[^'\\\n])*'   # a single-quoted literal, escapes included
    """,
    re.VERBOSE,
)
_NAMESPACE = re.compile(r"^#namespace\s+(\S+?)\s*=\s*(\S+)\s*$")

# The distributions this program implements. A mapping naming anything else is a
# hard failure: see the module docstring.
IMPLEMENTED_DISTRIBUTIONS = ("uniform",)

# SPARQL keywords that would make a template something other than a pure basic
# graph pattern. Asserted absent by `--self-test`, never silently tolerated.
NON_BGP_KEYWORDS = (
    "OPTIONAL",
    "UNION",
    "FILTER",
    "MINUS",
    "GRAPH",
    "BIND",
    "VALUES",
    "SERVICE",
    "GROUP BY",
    "HAVING",
    "ORDER BY",
)


class Mapping(NamedTuple):
    """One ``#mapping`` directive: a placeholder, its type, its distribution."""

    variable: str
    type_prefixed: str
    distribution: str


class Template(NamedTuple):
    """One of the twenty basic templates, as published."""

    name: str
    mappings: tuple[Mapping, ...]
    body: str


class Choice(NamedTuple):
    """One recorded instantiation: what was chosen, from what, and by what draw."""

    variable: str
    type_prefixed: str
    distribution: str
    chosen: str
    out_of: int
    position: int
    raw_draw: int


class Query(NamedTuple):
    """An instantiated query, with the record that produced it."""

    name: str
    text: str
    choices: tuple[Choice, ...]


def _template_sort_key(name: str) -> tuple[str, int]:
    """Order the twenty as C1..C3, F1..F5, L1..L5, S1..S7 — letter then number."""
    match = re.fullmatch(r"([A-Za-z]+)(\d+)", name)
    if match is None:
        sys.exit(f"FAIL: template name {name!r} is not a letter-then-number name")
    return match.group(1), int(match.group(2))


def parse_template(name: str, text: str) -> Template:
    """Split one template file into its mapping directives and its query body."""
    mappings: list[Mapping] = []
    body_lines: list[str] = []
    for line in text.splitlines():
        if line.startswith("#mapping"):
            match = _MAPPING.match(line)
            if match is None:
                sys.exit(
                    f"FAIL: {name} has a #mapping line this parser cannot read:\n"
                    f"  {line!r}\n"
                    "  A mapping is `#mapping <var> <type> <distribution>`. Refusing to guess."
                )
            mappings.append(Mapping(match.group(1), match.group(2), match.group(3)))
        elif line.startswith("#"):
            # A comment line that is not a directive. None of the twenty carries
            # one, and dropping it silently would be the wrong default, so it is
            # refused rather than ignored.
            sys.exit(f"FAIL: {name} carries an unrecognised directive line:\n  {line!r}")
        else:
            body_lines.append(line.rstrip())

    body = "\n".join(body_lines).strip("\n")
    if not body:
        sys.exit(f"FAIL: {name} has no query body")

    declared = {mapping.variable for mapping in mappings}
    used = set(_PLACEHOLDER.findall(body))
    if used != declared:
        sys.exit(
            f"FAIL: {name} declares mappings {sorted(declared)} but its body uses "
            f"placeholders {sorted(used)}.\n"
            "  A placeholder with no mapping cannot be instantiated, and a mapping with no\n"
            "  placeholder means the template is not the one this parser thinks it is."
        )
    return Template(name, tuple(mappings), body)


def load_templates(directory: Path) -> list[Template]:
    """Load the twenty basic templates from the top level of *directory*."""
    if not directory.is_dir():
        sys.exit(
            f"FAIL: {directory} is not a directory.\n"
            "  The templates live in watdiv_v06.tar under watdiv/testsuite/. Run\n"
            "  `make benchmark-acquire` and extract it, or run `make watdiv`, which does both."
        )
    files = sorted(path for path in directory.glob("*.txt") if path.is_file())
    if len(files) != EXPECTED_TEMPLATES:
        sys.exit(
            f"FAIL: expected {EXPECTED_TEMPLATES} basic templates in {directory}, "
            f"found {len(files)}.\n"
            "  The twenty are C1-C3, F1-F5, L1-L5 and S1-S7. The linear_incremental/ and\n"
            "  linear_mixed/ subdirectories are separate studies and are not among them."
        )
    templates = [parse_template(path.stem, path.read_text(encoding="utf-8")) for path in files]
    templates.sort(key=lambda template: _template_sort_key(template.name))
    return templates


def load_namespaces(model: Path) -> dict[str, str]:
    """Read the prefix table from WatDiv's own data-model file.

    The bindings are read rather than hard-coded because they are a property of
    the model the dataset was generated from. Two of them are not what a reader
    would guess — WatDiv binds ``foaf:`` to ``http://xmlns.com/foaf/`` without
    the usual ``0.1/`` segment — and a guessed binding would not fail, it would
    match nothing and answer every query zero.
    """
    if not model.exists():
        sys.exit(
            f"FAIL: {model} is not there.\n"
            "  It is watdiv/model/wsdbm-data-model.txt inside the pinned watdiv_v06.tar."
        )
    return load_namespaces_text(model.read_text(encoding="utf-8"))


def expand(prefixed: str, namespaces: dict[str, str]) -> str:
    """Resolve ``prefix:local`` against the model's prefix table, or fail naming it."""
    prefix, sep, local = prefixed.partition(":")
    if not sep:
        sys.exit(f"FAIL: {prefixed!r} is not a prefixed name")
    if prefix not in namespaces:
        sys.exit(
            f"FAIL: prefix {prefix!r} in {prefixed!r} is not declared by the WatDiv data model.\n"
            f"  Declared prefixes: {', '.join(sorted(namespaces))}"
        )
    return namespaces[prefix] + local


# ── Candidates ──────────────────────────────────────────────────────────────────

_ENTITY_LOCAL = re.compile(r"^([A-Za-z]+?)(\d+)$")

CANDIDATES_HEADER = "# purrdf-watdiv-candidates-v1"


class Candidates(NamedTuple):
    """Every entity type in the frozen dataset, with the dataset it came from."""

    dataset_sha256: str
    by_type: dict[str, tuple[str, ...]]


def read_declared(path: Path) -> dict[str, int]:
    """Read the generator's own entity-type census from the frozen ``saved.txt``.

    Its first line is the number of type rows that follow, each ``<type> <count>``.
    This is upstream's statement about the exact bytes we scraped, and it is the
    only independent check on the scrape that exists.
    """
    if not path.exists():
        sys.exit(
            f"FAIL: {path} is not there.\n"
            "  saved.txt ships beside the dataset inside watdiv.10M.tar.bz2 and is the\n"
            "  generator's own entity census. The candidate scrape is not run without it."
        )
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines or not lines[0].strip().isdigit():
        sys.exit(f"FAIL: {path} does not start with a type count")
    total = int(lines[0].strip())
    if len(lines) < total + 1:
        sys.exit(f"FAIL: {path} declares {total} types but holds only {len(lines) - 1} rows")
    declared: dict[str, int] = {}
    for line in lines[1 : total + 1]:
        parts = line.split()
        if len(parts) != 2 or not parts[1].isdigit():
            sys.exit(f"FAIL: {path} has a census row this parser cannot read:\n  {line!r}")
        declared[parts[0]] = int(parts[1])
    return declared


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 22), b""):
            digest.update(chunk)
    return digest.hexdigest()


def scrape_candidates(
    dataset: Path, declared: dict[str, int], namespaces: dict[str, str]
) -> Candidates:
    """Collect every declared entity type from the frozen dataset, in one pass.

    A term is a candidate for type ``T`` when it occurs in subject or object
    position and its IRI is exactly ``<namespace><T><decimal digits>``. Nothing
    is inferred from ``rdf:type``: WatDiv asserts a type triple for products and
    users but not for websites, topics or cities, so a type-triple rule would
    find candidate sets for some mappings and empty ones for others.

    Every scraped count is then checked against the census. A mismatch stops the
    run: an incomplete candidate set does not make instantiation fail, it makes
    it quietly biased, which is far worse.
    """
    if not dataset.exists():
        sys.exit(
            f"FAIL: {dataset} is not there.\n"
            "  It is the frozen dataset inside watdiv.10M.tar.bz2. Run `make watdiv`, which\n"
            "  acquires and extracts it, or extract it yourself into the lane's directory."
        )

    # `expand` is called for its VALIDATION: a census naming a prefix the data
    # model does not declare is a hard failure there rather than an empty set here.
    local_of: dict[str, str] = {}
    namespace_of: dict[str, str] = {}
    for prefixed in declared:
        expand(prefixed, namespaces)
        prefix, _, local = prefixed.partition(":")
        local_of[prefixed] = local
        namespace_of[prefixed] = namespaces[prefix]

    # Group the types by the namespace they live in, so the hot loop tests a
    # handful of namespace prefixes rather than one pattern per type. Longest
    # first: if one declared namespace were ever a prefix of another, the more
    # specific one has to win, and the loops below stop at their first match.
    prefixes = sorted(set(namespace_of.values()), key=lambda ns: (-len(ns), ns))
    by_local: dict[tuple[str, str], str] = {
        (namespace_of[prefixed], local_of[prefixed]): prefixed for prefixed in declared
    }

    seen: set[str] = set()
    malformed = 0
    lines = 0
    with dataset.open("r", encoding="utf-8") as handle:
        for line in handle:
            if not line.strip():
                continue
            lines += 1
            parts = line.split("\t", 2)
            if len(parts) != 3:
                malformed += 1
                continue
            subject = parts[0]
            obj = parts[2]
            for token in (subject, obj):
                if not token.startswith("<"):
                    continue
                end = token.find(">")
                if end < 0:
                    continue
                iri = token[1:end]
                for namespace in prefixes:
                    if iri.startswith(namespace):
                        seen.add(iri)
                        break

    if malformed:
        sys.exit(
            f"FAIL: {malformed} of {lines + malformed} lines in {dataset} are not "
            "tab-separated N-Triples.\n"
            "  The frozen dataset is tab-separated throughout. Skipping those lines would\n"
            "  silently shrink every candidate set, so the run stops instead."
        )

    by_type: dict[str, list[str]] = {prefixed: [] for prefixed in declared}
    for iri in seen:
        for namespace in prefixes:
            if not iri.startswith(namespace):
                continue
            match = _ENTITY_LOCAL.match(iri[len(namespace) :])
            if match is None:
                break
            prefixed = by_local.get((namespace, match.group(1)))
            if prefixed is not None:
                by_type[prefixed].append(iri)
            break

    check_against_census(
        declared,
        {prefixed: tuple(values) for prefixed, values in by_type.items()},
        "This is a FRESH scrape, so the disagreement is with the dataset itself.",
        "scraped",
    )

    # Canonical order, by UTF-8 bytes. The selection must not depend on the order
    # the file happened to mention a term in.
    ordered = {
        prefixed: tuple(sorted(values, key=lambda iri: iri.encode("utf-8")))
        for prefixed, values in by_type.items()
    }
    return Candidates(sha256_of(dataset), ordered)


def write_candidates(candidates: Candidates, path: Path) -> None:
    """Persist the scrape so re-running at another seed need not re-read 1.5 GB."""
    path.parent.mkdir(parents=True, exist_ok=True)
    lines = [CANDIDATES_HEADER, f"# dataset-sha256 {candidates.dataset_sha256}"]
    for prefixed in sorted(candidates.by_type):
        for iri in candidates.by_type[prefixed]:
            lines.append(f"{prefixed}\t{iri}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def check_against_census(
    declared: dict[str, int],
    by_type: dict[str, tuple[str, ...]],
    remedy: str,
    observed_as: str,
) -> None:
    """Refuse unless every declared type's candidate count matches the census.

    ONE implementation, called from both the fresh scrape and the cache-hit path.
    They were two, with two messages that had already drifted apart, and two
    copies of a rule are two rules -- the drift this lane's shared law file was
    created to end.

    Two things genuinely differ between the call sites and are therefore
    parameters rather than a second copy: *remedy*, the advice, which is not the
    same for a fresh scrape as for a cache that may have been edited; and
    *observed_as*, the verb for where the counts came from. "scraped" is a lie on
    the cache path, where nothing was scraped, and "have" throws away the one word
    that tells an operator whether to suspect the dataset or the cache.
    """
    problems = [
        f"    {prefixed}: {observed_as} {len(by_type.get(prefixed, ()))}, "
        f"census declares {expected}"
        for prefixed, expected in sorted(declared.items())
        if len(by_type.get(prefixed, ())) != expected
    ]
    if problems:
        sys.exit(
            "FAIL: the candidate sets do not agree with the dataset's own entity census.\n"
            + "\n".join(problems)
            + "\n  Either the entity naming rule or the frozen dataset is not what this program\n"
            "  believes. Instantiating from a candidate set that may be incomplete would bias\n"
            f"  every query built from it, so nothing is written.\n  {remedy}"
        )


def read_candidates(path: Path, dataset_sha256: str) -> Candidates | None:
    """Reload a cached scrape, but ONLY if it was taken from these exact bytes.

    The cache is keyed by the dataset's digest rather than by its path or its
    mtime, so a cache built from a different dataset is a miss and never a stale
    hit. Returns ``None`` — a plain cache miss — when the file is absent or was
    taken from other bytes.
    """
    if not path.exists():
        return None
    lines = path.read_text(encoding="utf-8").splitlines()
    if len(lines) < 2 or lines[0] != CANDIDATES_HEADER:
        return None
    marker, _, recorded = lines[1].partition(" dataset-sha256 ")
    if marker != "#" or recorded.strip() != dataset_sha256:
        return None
    by_type: dict[str, list[str]] = {}
    for line in lines[2:]:
        prefixed, _, iri = line.partition("\t")
        if iri:
            by_type.setdefault(prefixed, []).append(iri)
    # RE-CANONICALISE RATHER THAN TRUST THE FILE'S ORDER. Candidate order IS the
    # workload: `uniform_index` indexes into these tuples, so the same seed over a
    # differently ordered list is a different query set. On the fresh-scrape path
    # that order is a property of this program (`scrape_candidates` sorts by UTF-8
    # bytes); read back without this sort it became a property of a file under
    # ``target/``. A candidates.tsv written by an earlier version whose canonical
    # order differed has the same dataset digest and the same counts, so it is a
    # cache HIT -- and it would silently produce different queries at one seed.
    # The header version cannot guard that, because ordering is not part of what
    # it names. One sort over a few hundred thousand strings buys back the
    # invariant.
    return Candidates(
        dataset_sha256,
        {
            key: tuple(sorted(value, key=lambda iri: iri.encode("utf-8")))
            for key, value in by_type.items()
        },
    )


# ── Instantiation ───────────────────────────────────────────────────────────────


def instantiate(
    template: Template, seed: int, candidates: Candidates, namespaces: dict[str, str]
) -> Query:
    """Substitute every placeholder in one template, recording each choice."""
    body = template.body
    choices: list[Choice] = []

    for mapping in template.mappings:
        if mapping.distribution not in IMPLEMENTED_DISTRIBUTIONS:
            sys.exit(
                f"FAIL: {template.name} mapping {mapping.variable} names the distribution "
                f"{mapping.distribution!r}, which this instantiator does not implement.\n"
                f"  Implemented: {', '.join(IMPLEMENTED_DISTRIBUTIONS)}.\n"
                "  Falling back to uniform would emit a query set that runs perfectly and is\n"
                "  not the workload the template asked for, so the run stops here instead."
            )

        pool = candidates.by_type.get(mapping.type_prefixed)
        if pool is None:
            sys.exit(
                f"FAIL: {template.name} mapping {mapping.variable} names the type "
                f"{mapping.type_prefixed!r}, which the frozen dataset's own entity census\n"
                "  does not declare. Its candidates could be scraped, but with nothing to check\n"
                "  the scrape against the set would be unverified, so it is refused.\n"
                f"  Declared types: {', '.join(sorted(candidates.by_type))}"
            )
        if not pool:
            sys.exit(
                f"FAIL: {template.name} mapping {mapping.variable} has an EMPTY candidate set "
                f"for {mapping.type_prefixed!r} in this dataset"
            )

        index, raw = uniform_index(seed, stream_of(template.name, mapping.variable), len(pool))
        chosen = pool[index]
        placeholder = f"%{mapping.variable}%"
        if placeholder not in body:
            sys.exit(f"FAIL: {template.name} has no {placeholder} left to substitute")
        # The chosen term is written as a full IRI rather than a prefixed name.
        # Every candidate local name ends in digits, and a prefixed name's local
        # part has escaping rules a full IRI does not, so the unambiguous spelling
        # is the one that cannot be read two ways.
        body = body.replace(placeholder, f"<{chosen}>")
        choices.append(
            Choice(
                mapping.variable,
                mapping.type_prefixed,
                mapping.distribution,
                chosen,
                len(pool),
                index,
                raw,
            )
        )

    survivors = _PLACEHOLDER.findall(body)
    if survivors:
        sys.exit(
            f"FAIL: {template.name} still contains placeholder(s) {sorted(set(survivors))} "
            "after instantiation"
        )

    # Emit only the prefixes the instantiated text actually uses. Substituting a
    # placeholder with a full IRI can retire a prefix entirely, and a PREFIX
    # declaration for a prefix nothing names is noise in a file meant to be read.
    #
    # A NAME THE MODEL DOES NOT DECLARE IS A HARD FAILURE, not something to skip
    # past. `if match in namespaces` reads like a filter for retired prefixes, but
    # it also silently swallowed a prefixed name whose prefix the data model never
    # declared: the PREFIX line was dropped, the emitted .rq referenced an
    # undeclared prefix, the CLI refused to parse it, and the lane reported the
    # parser's complaint as CANNOT-EXECUTE -- a symptom, never the cause. `expand`
    # already treats this exact condition as fatal and names the prefix; this is
    # the same law and now says the same thing.
    # Quoted literals AND angle-bracketed IRIs are removed before scanning.
    # `_LITERAL_OR_IRI` understands ECHAR escapes; a regex that did not closed the
    # literal at the first `\"` and left the rest of it exposed to the prefix scan.
    # `_PREFIXED` is deliberately loose: a literal such as "note: see below"
    # matches it, and so does the `a:b` inside `<http://example.org/a:b>`, because
    # the lookbehind only blocks a match immediately after `<`. Since
    # instantiation substitutes full IRIs into these bodies, scanning the raw text
    # would refuse legal output -- the over-refusal that mirrors the silent drop
    # this check exists to fix. Only prefixes outside literals and IRIs count, in
    # both directions.
    scannable = _LITERAL_OR_IRI.sub(" ", body)
    found = sorted(set(_PREFIXED.findall(scannable)))
    undeclared = [prefix for prefix in found if prefix not in namespaces]
    if undeclared:
        sys.exit(
            f"FAIL: {template.name} uses prefix(es) {undeclared} that the WatDiv data "
            "model does not declare.\n"
            f"  Declared prefixes: {', '.join(sorted(namespaces))}\n"
            "  An emitted query naming an undeclared prefix does not parse, and the "
            "lane would report the parser's complaint as the diagnosis rather than "
            "this."
        )
    used = found
    header = "\n".join(f"PREFIX {prefix}: <{namespaces[prefix]}>" for prefix in used)
    text = (header + "\n" + body if header else body).strip() + "\n"
    return Query(template.name, text, tuple(choices))


def build(
    templates: list[Template], seed: int, candidates: Candidates, namespaces: dict[str, str]
) -> list[Query]:
    return [instantiate(template, seed, candidates, namespaces) for template in templates]


def emit(queries: list[Query], out: Path, seed: int, candidates: Candidates) -> None:
    """Write one ``.rq`` per query, the index the lane reads, and the provenance.

    All of it is build output. WatDiv's terms grant use, not redistribution, and
    a query mechanically derived from a WatDiv template is still derived from it.
    """
    out.mkdir(parents=True, exist_ok=True)
    for query in queries:
        (out / f"{query.name}.rq").write_text(query.text, encoding="utf-8")

    # One row per query, tab-separated, in template order. `regime` is always `-`:
    # WatDiv is pure BGP and needs no inference. It is carried as a COLUMN rather
    # than left out so that a row from this lane and a row from the LUBM lane are
    # visibly answering under different conditions.
    index = ["\t".join(("id", "regime", "mappings", "file"))]
    for query in queries:
        index.append(
            "\t".join(
                (
                    query.name,
                    "-",
                    str(len(query.choices)),
                    f"{query.name}.rq",
                )
            )
        )
    (out / "queries.tsv").write_text("\n".join(index) + "\n", encoding="utf-8")
    (out / "provenance.txt").write_text(provenance(queries, seed, candidates), encoding="utf-8")


def provenance(queries: list[Query], seed: int, candidates: Candidates) -> str:
    """Render the full record: seed, dataset, every choice, and every query."""
    lines = [
        "WATDIV QUERY INSTANTIATION -- PROVENANCE",
        "=" * 72,
        "",
        "Source: the 20 basic templates published in watdiv_v06.tar under",
        "watdiv/testsuite/, fetched by digest and NOT vendored. Each carries #mapping",
        "directives and %vN% placeholders; the choices below turned them into queries.",
        "",
        f"seed             {seed}",
        f"dataset sha256   {candidates.dataset_sha256}",
        "",
        "REPRODUCIBILITY. WatDiv's own instantiator seeds from the wall clock and the",
        "operating system entropy source and offers no seed flag, so its query sets",
        "cannot be regenerated. These were chosen by splitmix64 over the seed above and",
        "a pinned per-mapping stream, from candidate sets scraped from the digest-pinned",
        "dataset above and sorted by UTF-8 bytes. The same dataset and the same seed",
        "reproduce these queries BYTE FOR BYTE. This is a deliberate improvement on",
        "upstream, not a reimplementation of it.",
        "",
        "A QUERY SET FROM A DIFFERENT SEED IS A DIFFERENT WORKLOAD. Two numbers taken",
        "under two seeds compare two workloads, not two engines.",
        "",
        "ENTAILMENT. None. Every template is a pure basic graph pattern, so no regime",
        "applies and none was used. Do not compare these rows against LUBM rows: that",
        "workload answers eleven of fourteen queries only under inference.",
        "",
        "Cite, in anything derived from this: G. Aluc, O. Hartig, M. T. Ozsu and",
        "K. Daudjee, 'Diversified Stress Testing of RDF Data Management Systems',",
        "ISWC 2014, pages 197-212.",
        "",
    ]
    instantiated = 0
    for query in queries:
        lines.append("-" * 72)
        lines.append(f"{query.name}  ({len(query.choices)} mapping(s))")
        if not query.choices:
            lines.append("    no #mapping directives -- the published template is already a query")
        for choice in query.choices:
            instantiated += 1
            lines.append(f"    %{choice.variable}%  type {choice.type_prefixed}")
            lines.append(f"        distribution: {choice.distribution}")
            lines.append(
                f"        chosen:       {choice.chosen}"
                f"  (candidate {choice.position} of {choice.out_of})"
            )
            lines.append(f"        draw:         0x{choice.raw_draw:016x}")
        lines.append("")
        lines.append("    instantiated query:")
        lines.extend(f"        {line}" for line in query.text.rstrip("\n").splitlines())
        lines.append("")
    lines.append("=" * 72)
    lines.append(f"TOTALS: {len(queries)} queries, {instantiated} placeholder(s) instantiated")
    lines.append("")
    return "\n".join(lines)


# ── Self-test ───────────────────────────────────────────────────────────────────


def _toolkit_member(name: str) -> str:
    """Read one file out of the pinned toolkit tarball, without extracting it."""
    import tarfile

    if not TOOLKIT_TAR.exists():
        sys.exit(
            f"FAIL: {TOOLKIT_TAR} is not in the cache.\n"
            "  Run `make benchmark-acquire` first: WatDiv is fetched by digest at use time\n"
            "  and never vendored here."
        )
    with tarfile.open(TOOLKIT_TAR) as archive:
        handle = archive.extractfile(name)
        if handle is None:
            sys.exit(f"FAIL: {name} is not in {TOOLKIT_TAR}")
        return handle.read().decode("utf-8")


def _toolkit_templates() -> list[Template]:
    """The real twenty, read straight out of the pinned tarball."""
    import tarfile

    if not TOOLKIT_TAR.exists():
        sys.exit(f"FAIL: {TOOLKIT_TAR} is not in the cache. Run `make benchmark-acquire` first.")
    names: list[str] = []
    with tarfile.open(TOOLKIT_TAR) as archive:
        for member in archive.getmembers():
            # Top level of testsuite/ only: the linear_* subdirectories are
            # separate studies, not part of the twenty.
            if re.fullmatch(r"watdiv/testsuite/[A-Za-z]+\d+\.txt", member.name):
                names.append(member.name)
        names.sort()
        templates = []
        for name in names:
            handle = archive.extractfile(name)
            if handle is None:
                sys.exit(f"FAIL: {name} could not be read from {TOOLKIT_TAR}")
            stem = name.rsplit("/", 1)[1][: -len(".txt")]
            templates.append(parse_template(stem, handle.read().decode("utf-8")))
    templates.sort(key=lambda template: _template_sort_key(template.name))
    return templates


def _fixture_candidates(namespaces: dict[str, str]) -> Candidates:
    """A small synthetic pool covering every type the twenty templates map.

    Synthetic so the self-test is fast and needs no multi-gigabyte dataset. The
    pools are deliberately of DIFFERENT sizes, so a selection bug that ignored the
    pool size would show up as the same index everywhere.
    """
    sizes = {
        "wsdbm:AgeGroup": 9,
        "wsdbm:City": 240,
        "wsdbm:Country": 25,
        "wsdbm:ProductCategory": 15,
        "wsdbm:Retailer": 1200,
        "wsdbm:SubGenre": 145,
        "wsdbm:Topic": 250,
        "wsdbm:User": 100,
        "wsdbm:Website": 5000,
    }
    by_type = {}
    for prefixed, size in sizes.items():
        base = expand(prefixed, namespaces)
        values = sorted(
            (f"{base}{n}" for n in range(size)), key=lambda iri: iri.encode("utf-8")
        )
        by_type[prefixed] = tuple(values)
    return Candidates("0" * 64, by_type)


# ── The offline half of the self-test ───────────────────────────────────────────
#
# `self_test()` needs the fetched toolkit tarball, so it runs only inside
# `make watdiv`, after a download. That left the arithmetic every published
# WatDiv number depends on -- splitmix64, the stream derivation, the rejection
# sampler, and instantiation itself -- outside every gate in this repository,
# which is how a change to `_RETRY_STRIDE`, `TAG_MAPPING` or `stream_of` could
# have altered every query set at one seed with all checks still green.
#
# Everything below is synthetic and runs offline, so it can live in `make check`.

_PINNED_FIXTURE_SEED_0 = "e377b94c325592336ef1a40f37dc7165faf0251e9f9883643cffe5225635276c"

_FIXTURE_NAMESPACES = {
    "wsdbm": "http://db.uwaterloo.ca/~galuc/wsdbm/",
    "sorg": "http://schema.org/",
}


def _fixture_templates() -> list[Template]:
    """Three synthetic templates: one mapping, two mappings, and none at all.

    The no-mapping case is the control. It must be seed-INDEPENDENT, so a change
    that moved something it should not have is distinguishable from a change that
    moved a mapping.
    """
    return [
        Template(
            "X1",
            (Mapping("v1", "wsdbm:User", "uniform"),),
            "SELECT ?v0 WHERE {\n  %v1% wsdbm:likes ?v0 .\n}",
        ),
        Template(
            "X2",
            (Mapping("v1", "wsdbm:City", "uniform"), Mapping("v2", "wsdbm:User", "uniform")),
            "SELECT ?v0 WHERE {\n  ?v0 sorg:name %v1% .\n  %v2% wsdbm:friendOf ?v0 .\n}",
        ),
        Template("X3", (), "SELECT ?v0 WHERE {\n  ?v0 wsdbm:likes ?v1 .\n}"),
    ]


def _fixture_pool() -> Candidates:
    """Two pools of deliberately different sizes, canonically ordered."""
    by_type = {}
    for prefixed, size in (("wsdbm:User", 100), ("wsdbm:City", 7)):
        base = expand(prefixed, _FIXTURE_NAMESPACES)
        local = prefixed.partition(":")[2]
        by_type[prefixed] = tuple(
            sorted((f"{base}{local}{n}" for n in range(size)), key=lambda iri: iri.encode("utf-8"))
        )
    return Candidates("fixture", by_type)


def _fixture_digest(seed: int) -> str:
    """Digest the fixture query set at *seed* as a manifest of per-query digests.

    NOT a concatenation of names and texts. That stream is ambiguous -- nothing
    delimits the end of one text from the start of the next name -- so two
    different query sets can share one digest, which would make this pin
    unfalsifiable in exactly the cases it exists to catch. This mirrors
    `lane_query_set_digest`, and it is the same law: a directory digest is a
    manifest of per-file digests, never a concatenation.
    """
    records = sorted(
        f"{hashlib.sha256(query.text.encode('utf-8')).hexdigest()}  {query.name}"
        for query in build(_fixture_templates(), seed, _fixture_pool(), _FIXTURE_NAMESPACES)
    )
    return hashlib.sha256(("\n".join(records) + "\n").encode("utf-8")).hexdigest()


def offline_self_test() -> int:
    """The tarball-free checks, so the instantiator's arithmetic is gated."""
    ok = True

    def check(condition: bool, label: str) -> None:
        nonlocal ok
        print(f"{'OK' if condition else 'SELF-TEST FAIL'}: {label}")
        ok = ok and condition

    check(
        splitmix64(0) == 0xE220_A839_7B1D_CDAF,
        "splitmix64 matches its published reference vector for 0",
    )

    # THE REPRODUCIBILITY CLAIM, PINNED. The lane prints a query-set digest and
    # calls it the reproducibility check, but nothing compared it to a recorded
    # value -- `verify_query_set` compares it only to itself, as a concurrency
    # tripwire. So an edit to the mixing constants, the stream tag, the retry
    # stride or the prefix emission changed every query set at one seed while
    # every gate stayed green. This is that missing comparison, over fixtures
    # rather than over a multi-gigabyte corpus, which is what lets it run here.
    actual = _fixture_digest(0)
    check(
        actual == _PINNED_FIXTURE_SEED_0,
        f"the fixture query set at seed 0 reproduces its pinned digest ({actual[:16]}…)",
    )
    check(_fixture_digest(0) == actual, "two builds at one seed agree byte for byte")
    check(
        _fixture_digest(7) != actual,
        "a different seed is a different workload, not the same one",
    )

    # The control: a template with no mapping has nothing to draw, so the seed
    # must not reach it. Without this, "the seed changed the output" cannot be
    # told apart from "the seed changed something it had no business touching".
    unmapped = [t for t in _fixture_templates() if not t.mappings]
    at_zero = build(unmapped, 0, _fixture_pool(), _FIXTURE_NAMESPACES)
    at_seven = build(unmapped, 7, _fixture_pool(), _FIXTURE_NAMESPACES)
    check(
        [q.text for q in at_zero] == [q.text for q in at_seven],
        "a template with no mapping is seed-independent",
    )

    # `uniform_index` is exercised directly, because the digest above only pins
    # what REACHES a query: one draw per mapping. A sweep shows the selector
    # spans its pool rather than favouring an index, which is the property the
    # digest cannot see.
    #
    # THE RETRY BRANCH IS COVERED, at a count where rejection actually fires.
    #
    # An earlier version of this comment declined to cover it, reasoning that
    # rejection lands in a window `2**64 % count` wide and so is unreachable. That
    # is true of WATDIV'S POOL SIZES -- at count 7 the window is 2, about one part
    # in 1.1e19 -- and false of `uniform_index`'s own contract, which this fixture
    # is free to exercise at any count. At `2**63 + 1` the window is half the
    # space, so every stream retries, and `_RETRY_STRIDE` moves from "argued
    # unreachable" to pinned. Declining coverage that costs two lines was a gap
    # dressed as a principle.
    reachable = {uniform_index(0, stream, 7)[0] for stream in range(4000)}
    check(
        reachable == set(range(7)),
        f"every index of a 7-candidate pool is reachable (saw {len(reachable)}/7)",
    )

    # A count whose reject window is half the space: the loop must terminate, stay
    # in range, and demonstrably have taken the retry path.
    wide = (1 << 63) + 1
    draws = [uniform_index(0, stream, wide) for stream in range(200)]
    check(
        all(0 <= index < wide for index, _ in draws),
        "every draw at a half-rejecting count is still in range",
    )
    check(
        sum(1 for _, attempts in draws if attempts > 1) > 0,
        "the rejection retry path is actually taken at a half-rejecting count "
        f"(retried {sum(1 for _, attempts in draws if attempts > 1)} of {len(draws)})",
    )
    # The stride's ODDNESS is the invariant that makes the retry walk sound: only an
    # odd addend generates the whole additive group mod 2**64, so only an odd stride
    # is guaranteed to reach an acceptable draw rather than cycling inside a
    # subgroup. Exercising the path does NOT pin this -- an even stride still finds a
    # draw quickly when the reject window is wide, which is exactly what a mutation
    # to an even value demonstrated -- so the property is asserted directly.
    check(_RETRY_STRIDE % 2 == 1, f"the retry stride is odd ({_RETRY_STRIDE:#x})")

    # Candidate order read back from a cache must be re-canonicalised, not
    # trusted: order IS the workload, and a cache written in another order has
    # the same digest and the same counts.
    with tempfile.TemporaryDirectory() as raw:
        scratch = Path(raw) / "candidates.tsv"
        pool = _fixture_pool()
        write_candidates(pool, scratch)
        shuffled = scratch.read_text(encoding="utf-8").splitlines()
        body = shuffled[2:]
        scratch.write_text(
            "\n".join([*shuffled[:2], *reversed(body)]) + "\n", encoding="utf-8"
        )
        reloaded = read_candidates(scratch, pool.dataset_sha256)
        check(
            reloaded is not None and reloaded.by_type == pool.by_type,
            "a candidates cache written in another order reloads canonically",
        )

    # THE PREFIX REFUSAL, BOTH DIRECTIONS, ON THE PRODUCTION FUNCTION.
    #
    # `instantiate` hard-fails on a prefix the data model does not declare. The
    # cases below were run once in a shell when that refusal was written and then
    # discarded, which is not coverage -- and the over-refusal half is the one that
    # matters, because `_PREFIXED` is deliberately loose and instantiation
    # substitutes full IRIs into these bodies. The valid neighbours come first: a
    # refusal that rejected them would be the mirror of the silent drop this
    # replaced.
    def instantiate_body(body: str, mappings: tuple[Mapping, ...] = ()) -> Query:
        return instantiate(
            Template("P1", mappings, body), 0, _fixture_pool(), _FIXTURE_NAMESPACES
        )

    accepted = {
        "declared prefixes only": "SELECT ?v0 WHERE {\n  ?v0 wsdbm:likes ?v1 .\n}",
        "a colon inside a quoted literal": (
            'SELECT ?v0 WHERE {\n  ?v0 sorg:name "note: see below" .\n}'
        ),
        # THE COLON INSIDE THE ESCAPED QUOTES IS THE CONTROL. This fixture used
        # `"say \"hi\""` first, and `hi` carries no colon -- so it passed whether or
        # not the literal stripper understood ECHAR escapes, and it certified a LIVE
        # over-refusal as a false positive. A control that cannot distinguish the
        # case it exists for is not a control; this one fails if the stripper
        # regresses.
        "escaped quotes wrapping a colon": (
            'SELECT ?v0 WHERE {\n  ?v0 sorg:name "say \\"nosuch:name\\"" . ?v0 wsdbm:likes ?v1 .\n}'
        ),
        "a colon inside an IRI path": (
            "SELECT ?v0 WHERE {\n  ?v0 <http://example.org/a:b> ?v1 .\n}"
        ),
        "a real substituted WatDiv IRI": (
            "SELECT ?v0 WHERE {\n  <http://db.uwaterloo.ca/~galuc/wsdbm/User1> wsdbm:likes ?v0 .\n}"
        ),
    }
    for label, body in accepted.items():
        try:
            instantiate_body(body)
        except SystemExit as exc:
            print(f"SELF-TEST FAIL: a legal body was refused ({label}): {exc.code}")
            ok = False
        else:
            print(f"OK: a legal body is accepted -- {label}")

    for label, body in {
        "an undeclared prefix": "SELECT ?v0 WHERE {\n  ?v0 nosuch:name ?v1 .\n}",
        "an undeclared prefix beside an IRI": (
            "SELECT ?v0 WHERE {\n  <http://example.org/x> bogus:p ?v1 .\n}"
        ),
    }.items():
        try:
            instantiate_body(body)
        except SystemExit as exc:
            message = str(exc.code)
            if "does not declare" in message:
                print(f"OK: {label} is refused, naming the prefix")
            else:
                print(f"SELF-TEST FAIL: {label} refused for the wrong reason: {message}")
                ok = False
        else:
            print(f"SELF-TEST FAIL: {label} was accepted; the emitted query would not parse")
            ok = False

    # THE CENSUS PARSER HAD NO TEST AT ALL, in either direction -- and
    # `saved.txt` is, by this module's own docstring, the only independent check
    # on the scrape that exists. `scrape_candidates` is well covered, but only
    # ever against a hand-built `declared` dict, so nothing exercised the code
    # that produces that dict from the frozen file.
    def expect_exit(thunk, label: str, must_contain: list[str]) -> None:
        nonlocal ok
        try:
            thunk()
        except SystemExit as exc:
            message = str(exc.code)
            if not exc.code or isinstance(exc.code, int) and exc.code == 0:
                print(f"SELF-TEST FAIL: {label} exited zero")
                ok = False
                return
            missing = [needle for needle in must_contain if needle not in message]
            if missing:
                print(f"SELF-TEST FAIL: {label} did not say {missing}: {message}")
                ok = False
                return
            print(f"OK: {label}")
            return
        print(f"SELF-TEST FAIL: {label} did not refuse at all")
        ok = False

    with tempfile.TemporaryDirectory() as raw:
        root = Path(raw)

        # The valid neighbour FIRST: a well-formed census must parse, or every
        # refusal below would "pass" against a parser that rejects everything.
        good = root / "saved.txt"
        good.write_text("2\nwsdbm:User 100\nwsdbm:City 7\n", encoding="utf-8")
        parsed = read_declared(good)
        check(
            parsed == {"wsdbm:User": 100, "wsdbm:City": 7},
            f"a well-formed census parses to its declared counts (got {parsed})",
        )

        missing = root / "absent.txt"
        expect_exit(lambda: read_declared(missing), "a missing census is refused", ["FAIL:"])

        headless = root / "headless.txt"
        headless.write_text("wsdbm:User 100\n", encoding="utf-8")
        expect_exit(
            lambda: read_declared(headless),
            "a census with no leading type count is refused",
            ["does not start with a type count"],
        )

        short = root / "short.txt"
        short.write_text("5\nwsdbm:User 100\n", encoding="utf-8")
        expect_exit(
            lambda: read_declared(short),
            "a census declaring more rows than it holds is refused",
            ["declares 5 types"],
        )

        malformed = root / "malformed.txt"
        malformed.write_text("1\nnot-a-census-row\n", encoding="utf-8")
        expect_exit(
            lambda: read_declared(malformed),
            "a census row the parser cannot read is refused, quoting it",
            ["cannot read"],
        )

        # `load_templates` is the loader the LANE uses, and the tarball-backed
        # self-test exercises a different one (`_toolkit_templates`), so its
        # directory and count refusals were untested on both sides.
        empty = root / "no-templates"
        empty.mkdir()
        expect_exit(
            lambda: load_templates(empty),
            f"a directory holding fewer than {EXPECTED_TEMPLATES} templates is refused",
            ["FAIL:"],
        )
        expect_exit(
            lambda: load_templates(root / "nowhere"),
            "a templates path that is not a directory is refused",
            ["FAIL:"],
        )

    print("OFFLINE SELF-TEST PASS" if ok else "OFFLINE SELF-TEST FAIL")
    return 0 if ok else 1


def self_test() -> int:
    """Assert the rules' SCOPE, not merely that instantiation ran."""
    ok = True

    def check(condition: bool, label: str) -> None:
        nonlocal ok
        print(f"{'OK' if condition else 'SELF-TEST FAIL'}: {label}")
        ok = ok and condition

    def expect_exit(thunk, label: str, must_contain: list[str]) -> None:
        nonlocal ok
        try:
            thunk()
        except SystemExit as exc:
            message = str(exc.code)
            # `SystemExit(None)` is a ZERO exit too, and this missed it -- so a
            # refusal that vanished into a bare `sys.exit()` would be judged by
            # its message rather than by the fact that it succeeded. The sibling
            # helper in benchmark-acquire.py already reads it this way.
            if not exc.code or isinstance(exc.code, int) and exc.code == 0:
                print(f"SELF-TEST FAIL: {label} exited zero")
                ok = False
                return
            missing = [needle for needle in must_contain if needle not in message]
            if missing:
                print(f"SELF-TEST FAIL: {label} message omits {missing}\n  message: {message}")
                ok = False
                return
            print(f"OK: {label}")
            return
        print(f"SELF-TEST FAIL: {label} did not fail at all")
        ok = False

    # 1. The mixing function is the one it claims to be. splitmix64's published
    #    first output for state 0 pins every constant in it at once.
    check(splitmix64(0) == 0xE220A8397B1DCDAF, "splitmix64 matches its published reference vector")

    # 2. Uniform selection is uniform: no candidate index is unreachable and the
    #    modulo tail is rejected rather than folded. Sweeping the stream space is
    #    the check that the rejection branch cannot strand an index.
    reachable = {uniform_index(0, stream, 7)[0] for stream in range(4000)}
    check(reachable == set(range(7)), f"every candidate index is reachable (saw {len(reachable)}/7)")

    # 3. Exactly twenty templates, and exactly the twenty WatDiv publishes.
    templates = _toolkit_templates()
    names = [template.name for template in templates]
    expected = (
        ["C1", "C2", "C3"]
        + [f"F{n}" for n in range(1, 6)]
        + [f"L{n}" for n in range(1, 6)]
        + [f"S{n}" for n in range(1, 8)]
    )
    check(names == expected, f"exactly the 20 basic templates load, in order (got {len(names)})")

    # 4. The three complex templates carry no mapping and the other seventeen do.
    #    This is rule SCOPE: a parser that dropped every directive would still
    #    "load 20 templates" and emit twenty queries with placeholders in them.
    unmapped = {t.name for t in templates if not t.mappings}
    check(unmapped == {"C1", "C2", "C3"}, f"exactly C1-C3 carry no #mapping (got {sorted(unmapped)})")
    total_mappings = sum(len(t.mappings) for t in templates)
    check(total_mappings == 17, f"the twenty declare 17 mappings in total (got {total_mappings})")

    # 5. Every published mapping names a distribution this program implements.
    #    If upstream ever ships one that does not, the hard failure below is what
    #    must happen — never a silent fallback.
    named = {mapping.distribution for template in templates for mapping in template.mappings}
    check(
        named <= set(IMPLEMENTED_DISTRIBUTIONS),
        f"every published mapping names an implemented distribution (saw {sorted(named)})",
    )

    namespaces = load_namespaces_text(_toolkit_member("watdiv/model/wsdbm-data-model.txt"))
    check(len(namespaces) >= 10, f"the data model declares its prefix table ({len(namespaces)})")
    check(
        namespaces.get("foaf") == "http://xmlns.com/foaf/",
        "foaf: is read from the model, not guessed as the usual 0.1/ namespace",
    )

    pool = _fixture_candidates(namespaces)
    queries = build(templates, 0, pool, namespaces)

    # 6. Every placeholder is substituted and none survives into an emitted query.
    check(
        not any(_PLACEHOLDER.search(query.text) for query in queries),
        "no %vN% placeholder survives into any emitted query",
    )
    check(
        not any("%" in query.text for query in queries),
        "no emitted query contains a per-cent sign at all",
    )
    substituted = sum(len(query.choices) for query in queries)
    check(substituted == 17, f"all 17 placeholders were substituted (got {substituted})")

    # 7. Every chosen term is really in the pool it was drawn from, at the index
    #    recorded. A record that did not match the query would make the whole
    #    provenance file decorative.
    faithful = all(
        pool.by_type[choice.type_prefixed][choice.position] == choice.chosen
        and f"<{choice.chosen}>" in query.text
        for query in queries
        for choice in query.choices
    )
    check(faithful, "every recorded choice is the term the query actually carries")

    # 8. Two runs at one seed are byte-identical; two seeds differ. This is the
    #    property the whole design exists for, so it is asserted, not assumed.
    again = build(templates, 0, pool, namespaces)
    check(
        [q.text for q in queries] == [q.text for q in again],
        "two runs at one seed produce byte-identical queries",
    )
    other = build(templates, 1, pool, namespaces)
    differing = sum(1 for a, b in zip(queries, other) if a.text != b.text)
    check(
        differing > 0,
        f"a different seed produces a different workload ({differing} of 20 queries differ)",
    )
    # ...and the three templates with nothing to instantiate must NOT differ:
    # a seed that changed them would mean something other than a mapping moved.
    fixed = {a.name for a, b in zip(queries, other) if a.text == b.text}
    check(
        {"C1", "C2", "C3"} <= fixed,
        f"the three mapping-free templates are seed-independent (fixed: {sorted(fixed)})",
    )

    # 9. PURE BGP. Asserted, not stated: this is what licenses the lane to run
    #     with no entailment regime at all.
    offenders = [
        (query.name, keyword)
        for query in queries
        for keyword in NON_BGP_KEYWORDS
        if keyword in query.text.upper()
    ]
    check(not offenders, f"every query is a pure basic graph pattern (offenders: {offenders})")

    # 10. A substituted variable is never also projected. Substituting one that
    #     was would leave the query projecting a variable that no longer occurs —
    #     legal SPARQL, permanently unbound, and a silent change of meaning.
    projected_bugs = [
        (template.name, mapping.variable)
        for template in templates
        for mapping in template.mappings
        if f"?{mapping.variable}" in template.body.partition("WHERE")[0]
    ]
    check(not projected_bugs, f"no mapped variable is projected (offenders: {projected_bugs})")

    # 11. THE REFUSALS, each with its neighbouring VALID case, because a refusal
    #     that fires on everything also passes a test that only tries the bad input.
    good_mapping = Template(
        "T1", (Mapping("v1", "wsdbm:Website", "uniform"),), "SELECT ?v0 WHERE { ?v0 a %v1% . }"
    )
    try:
        instantiate(good_mapping, 0, pool, namespaces)
    except SystemExit:
        print("SELF-TEST FAIL: a valid uniform mapping over a declared type was refused")
        ok = False
    else:
        print("OK: a valid uniform mapping over a declared type is accepted")

    expect_exit(
        lambda: instantiate(
            Template("T2", (Mapping("v1", "wsdbm:Website", "normal"),), "SELECT ?v0 WHERE { %v1% }"),
            0,
            pool,
            namespaces,
        ),
        "an unimplemented distribution hard-fails NAMING it, with no fallback",
        ["FAIL:", "normal", "does not implement"],
    )
    expect_exit(
        lambda: instantiate(
            Template(
                "T3", (Mapping("v1", "wsdbm:Nonesuch", "uniform"),), "SELECT ?v0 WHERE { %v1% }"
            ),
            0,
            pool,
            namespaces,
        ),
        "a type the dataset census does not declare hard-fails naming it",
        ["FAIL:", "wsdbm:Nonesuch", "census"],
    )
    expect_exit(
        lambda: parse_template("T4", "#mapping v1 wsdbm:Website\nSELECT ?v0 WHERE { %v1% }"),
        "a malformed #mapping line is refused rather than guessed at",
        ["FAIL:", "cannot read"],
    )
    expect_exit(
        lambda: parse_template("T5", "SELECT ?v0 WHERE { %v9% }"),
        "a placeholder with no mapping is refused",
        ["FAIL:", "v9"],
    )
    expect_exit(
        lambda: parse_template("T6", "#mapping v1 wsdbm:Website uniform\nSELECT ?v0 WHERE { ?v0 }"),
        "a mapping with no placeholder is refused",
        ["FAIL:", "v1"],
    )

    # 12. THE SCRAPE AND ITS CENSUS CROSS-CHECK, run for real against tiny
    #     fixture datasets. Both arms are needed: a cross-check that rejected
    #     every scrape would pass a test that only ever fed it a bad one.

    wsdbm = namespaces["wsdbm"]
    triples = [
        (f"{wsdbm}City0", f"{wsdbm}parentCountry", f"{wsdbm}Country1"),
        (f"{wsdbm}City10", f"{wsdbm}parentCountry", f"{wsdbm}Country0"),
        (f"{wsdbm}City2", f"{wsdbm}parentCountry", f"{wsdbm}Country1"),
    ]

    def write_nt(path: Path, rows) -> None:
        path.write_text(
            "".join(f"<{s}>\t<{p}>\t<{o}> .\n" for s, p, o in rows), encoding="utf-8"
        )

    with tempfile.TemporaryDirectory(prefix="watdiv-selftest-") as raw:
        tmp = Path(raw)
        data = tmp / "fixture.nt"
        write_nt(data, triples)
        honest = {"wsdbm:City": 3, "wsdbm:Country": 2}

        scraped = None
        try:
            scraped = scrape_candidates(data, honest, namespaces)
        except SystemExit:
            print("SELF-TEST FAIL: a scrape that AGREES with its census was refused")
            ok = False
        else:
            print("OK: a scrape that agrees with its census is accepted")

        if scraped is not None:
            # Canonical order is by UTF-8 bytes, so City10 sorts before City2.
            check(
                scraped.by_type["wsdbm:City"]
                == (f"{wsdbm}City0", f"{wsdbm}City10", f"{wsdbm}City2"),
                "candidates come back in canonical UTF-8 byte order",
            )
            # ...and that order does not depend on the order the file listed them.
            shuffled = tmp / "shuffled.nt"
            write_nt(shuffled, list(reversed(triples)))
            check(
                scrape_candidates(shuffled, honest, namespaces).by_type
                == scraped.by_type,
                "the scrape is independent of the order the dataset lists terms in",
            )

        expect_exit(
            lambda: scrape_candidates(data, {"wsdbm:City": 4, "wsdbm:Country": 2}, namespaces),
            "a scrape that DISAGREES with its census hard-fails naming both counts",
            ["FAIL:", "wsdbm:City", "scraped 3", "declares 4"],
        )

        ragged = tmp / "ragged.nt"
        ragged.write_text(f"<{wsdbm}City0> <{wsdbm}parentCountry> <{wsdbm}Country1> .\n", "utf-8")
        expect_exit(
            lambda: scrape_candidates(ragged, honest, namespaces),
            "a dataset that is not tab-separated is refused, not silently skipped",
            ["FAIL:", "tab-separated"],
        )

    # 13. The candidates cache is keyed by the dataset digest, so a cache taken
    #     from other bytes is a MISS and can never be a stale hit.
    with tempfile.TemporaryDirectory(prefix="watdiv-selftest-") as raw:
        cache = Path(raw) / "candidates.tsv"
        write_candidates(Candidates("a" * 64, pool.by_type), cache)
        hit = read_candidates(cache, "a" * 64)
        miss = read_candidates(cache, "b" * 64)
        check(
            hit is not None and hit.by_type == pool.by_type,
            "a candidates cache taken from the same dataset round-trips exactly",
        )
        check(miss is None, "a candidates cache taken from other bytes is a miss, not a stale hit")

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def load_namespaces_text(text: str) -> dict[str, str]:
    """The prefix table, parsed from the model file's text (see `load_namespaces`)."""
    namespaces: dict[str, str] = {}
    for line in text.splitlines():
        match = _NAMESPACE.match(line)
        if match is not None:
            namespaces[match.group(1)] = match.group(2)
    if not namespaces:
        sys.exit("FAIL: the WatDiv data model declares no #namespace lines")
    return namespaces


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Instantiate WatDiv's 20 basic templates deterministically."
    )
    parser.add_argument(
        "--templates",
        type=Path,
        default=DEFAULT_OUT / "testsuite",
        help="directory holding the 20 published template files",
    )
    parser.add_argument(
        "--model",
        type=Path,
        default=DEFAULT_OUT / "model" / "wsdbm-data-model.txt",
        help="WatDiv's data-model file, which declares the prefix table",
    )
    parser.add_argument(
        "--dataset", type=Path, default=DEFAULT_OUT / "watdiv.10M.nt", help="the frozen dataset"
    )
    parser.add_argument(
        "--census",
        type=Path,
        default=DEFAULT_OUT / "saved.txt",
        help="the generator's own entity census, shipped beside the dataset",
    )
    parser.add_argument(
        "--candidates",
        type=Path,
        default=DEFAULT_OUT / "candidates.tsv",
        help="where the candidate scrape is cached (keyed by the dataset digest)",
    )
    parser.add_argument("--seed", type=int, default=0, help="the pinned instantiation seed")
    parser.add_argument("--out", type=Path, help="directory to write .rq files into")
    parser.add_argument("--provenance", action="store_true", help="print the full record")
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument(
        "--offline-self-test",
        action="store_true",
        help="the tarball-free checks, suitable for a gate",
    )
    args = parser.parse_args()

    if args.offline_self_test:
        return offline_self_test()

    if args.self_test:
        return self_test()

    if args.seed < 0:
        sys.exit(f"FAIL: --seed must be a non-negative integer (got {args.seed})")

    templates = load_templates(args.templates)
    namespaces = load_namespaces(args.model)
    declared = read_declared(args.census)

    if not args.dataset.exists():
        sys.exit(
            f"FAIL: {args.dataset} is not there.\n"
            "  Run `make watdiv`, which acquires the pinned tarball and extracts it."
        )
    digest = sha256_of(args.dataset)
    candidates = read_candidates(args.candidates, digest)
    if candidates is None:
        print(f"scraping candidate sets from {args.dataset} (one pass)...", file=sys.stderr)
        candidates = scrape_candidates(args.dataset, declared, namespaces)
        write_candidates(candidates, args.candidates)
        print(f"cached candidate sets in {args.candidates}", file=sys.stderr)
    else:
        print(f"candidate sets: cache hit on {args.candidates}", file=sys.stderr)

    # The census cross-check is re-run against whatever we are about to use,
    # cache hit or fresh scrape alike. A cache is a file, and a file can be
    # edited; re-checking it costs a dictionary comparison and means the counts
    # behind the queries were verified on THIS run rather than on some earlier
    # one whose result we are trusting by reputation.
    check_against_census(
        declared,
        candidates.by_type,
        f"Delete {args.candidates} to force a fresh scrape.",
        "have",
    )

    queries = build(templates, args.seed, candidates, namespaces)
    if args.out:
        emit(queries, args.out, args.seed, candidates)
        print(f"wrote {len(queries)} instantiated queries to {args.out} at seed {args.seed}")
    if args.provenance or not args.out:
        print(provenance(queries, args.seed, candidates))
    return 0


if __name__ == "__main__":
    sys.exit(main())
