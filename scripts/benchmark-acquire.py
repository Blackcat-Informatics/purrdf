#!/usr/bin/env python3
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0

"""Fetch the pinned comparison-workload artifacts by digest (`make benchmark-acquire`).

PurRDF's own scale corpus is generated, not downloaded. Comparing PurRDF against
other RDF stores needs the two workloads the literature actually uses — LUBM and
WatDiv — and neither one may live in this repository. This script fetches six
pinned artifacts into a cache under ``target/`` and verifies every byte against a
digest recorded here. Nothing is vendored, nothing is redistributed, and no
unverified byte is ever handed to a caller.

LICENSING — the conclusions that force the fetch-by-digest design
================================================================

The design is not a matter of taste. A licensing review of each upstream
artifact reached three binding conclusions, and the bytes bear all three out:

* **The LUBM UBA data generator is GPL-2.0-or-later.** Every Java source file in
  ``uba1.7.zip`` carries the GNU General Public License header, "either version
  2 of the License, or (at your option) any later version", and
  ``GeneratorLinuxFix.zip`` is a modified copy of that same source carrying the
  same header. PurRDF is MIT OR Apache-2.0. The generator is therefore **RUN,
  NEVER VENDORED**: copying it into this tree would place a copyleft work inside
  a permissively licensed distribution. Running a GPL program to produce data is
  not distribution of that program, and the data it emits is what the benchmark
  consumes.

* **``univ-bench.owl`` carries no license text at all.** The ontology file
  contains no license, copyright, or rights statement of any kind, and the LUBM
  project page publishes none either. Absent an explicit grant there is no
  permission to redistribute, so the ontology is **NOT REDISTRIBUTED**: it is
  fetched by digest at the moment of use and left in the cache. The same holds
  for ``queries-sparql.txt``, which is published on the same page under the same
  silence.

* **WatDiv is citation-ware.** Its publisher's terms are, verbatim in substance,
  that *provided you include a citation to the ISWC 2014 paper, you are free to
  download and use the WatDiv Data and Query Generator*, supplied "as is" with
  all use at your own risk. That is a use grant, not a redistribution grant, so
  WatDiv is **NOT VENDORED** either — and the citation obligation is not
  discharged by this file. Every published result derived from WatDiv must cite:

      G. Aluç, O. Hartig, M. T. Özsu and K. Daudjee. "Diversified Stress Testing
      of RDF Data Management Systems." In Proc. The Semantic Web - ISWC 2014 -
      13th International Semantic Web Conference, 2014, pages 197-212.

  LUBM results should likewise cite Y. Guo, Z. Pan and J. Heflin, "LUBM: A
  Benchmark for OWL Knowledge Base Systems", Journal of Web Semantics 3(2).

So: **nothing is vendored; everything is fetched by digest at use time, into a
cache under ``target/``, which is ignored.** ``--list`` prints the posture of
each artifact so an operator can read the terms without reading this source.

WATDIV GENERATION IS NOT REPRODUCIBLE — ONLY ITS OUTPUT CAN BE FROZEN
=====================================================================

Pinning the WatDiv *tarball* pins the generator's source, and that is all it
pins. Stock WatDiv v0.6 seeds itself from the wall clock and from the operating
system's entropy source, and exposes no seed flag:

* ``src/model.cpp`` builds its Boost generator as
  ``boost::mt19937(static_cast<unsigned>(time(0)))`` at static-initialization
  time, and calls ``srand(time(NULL))`` again inside the generator;
* ``src/statistics.cpp`` calls ``srand(time(NULL))``;
* ``src/volatility_gen.cpp`` constructs ``mt19937`` from ``random_device``;
* the tool's own usage banner offers ``-d``, ``-q`` and ``-s`` modes and no seed
  option of any kind.

Two runs of the same pinned binary over the same model file therefore produce
different data. **The WatDiv GENERATION PROCESS cannot be pinned; only a frozen
OUTPUT can be.** A WatDiv dataset used for a comparison must be generated once
and then itself pinned by digest — treating a WatDiv run as reproducible because
its source tarball is pinned is a false claim about the benchmark.

That is why ``watdiv.10M.tar.bz2`` is pinned here as a first-class artifact
alongside the toolkit, and why **the WatDiv generator is never built and never
run by this repository at all**. It could not be built even if that were wanted:
v0.6 calls ``std::random_shuffle``, which C++17 removed. The dataset the lane
consumes is upstream's own frozen output, verified byte for byte.

Freezing the dataset has a consequence the query side depends on: once the bytes
are pinned, **the candidate set behind every query template's ``#mapping`` is
fixed too**, so query instantiation can be made a pure function of the dataset
and a seed. ``scripts/watdiv-queries.py`` makes it one, which upstream's
equally time-seeded query instantiator cannot.

LUBM is the opposite case and needs no such caveat: UBA accepts ``-index`` and
``-seed``, and ``-index 0 -seed 0`` reproduces the datasets used in the LUBM
papers. Its generation IS pinnable, given the generator it is run with.

WHAT THIS SCRIPT GUARANTEES
===========================

* A digest mismatch is a HARD FAILURE. The offending bytes are moved aside to a
  ``.rejected-<digest>`` file — never deleted silently, never overwritten by a
  fresh download — and the run exits non-zero naming the expected and the actual
  digest.
* A cached file whose digest no longer matches is an ERROR, not a cache miss.
  Nothing is re-fetched over it.
* A re-run against a good cache performs no network access at all.
* ``--self-test`` runs entirely offline and exercises the real verification,
  quarantine, and cache-hit code paths against temporary fixtures.
* Before and after fetching, the run proves the cache is invisible to version
  control using the repository's OWN ignore rules, with the operator's personal
  ignore file disabled — because a clone that lacks that personal file is the
  case in which a multi-megabyte, non-redistributable artifact gets committed.
"""

import argparse
import hashlib
import os
import re
import subprocess
import sys
import http.client
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Callable, NamedTuple

REPO_ROOT = Path(__file__).resolve().parent.parent

# The one place anything is written. Everything under ``target/`` is build
# output; no artifact fetched here is ever written anywhere else in the tree.
CACHE = REPO_ROOT / "target" / "bench-artifacts"


class Artifact(NamedTuple):
    """One pinned upstream file: where it comes from, and what it must hash to.

    ``size`` is redundant against ``sha256`` for correctness and is checked
    anyway, first: a truncated response or a captive-portal error page fails the
    size check with a message that says "truncated", where the digest check alone
    would only say "different".

    ``md5`` is present only where upstream itself publishes one. It is an
    independent statement by the publisher about the same bytes, so checking it
    catches the case where a pin was copied from a file that had already drifted.
    It is not a security property and is never the only check.
    """

    filename: str
    url: str
    sha256: str
    size: int
    md5: str | None
    licence: str
    posture: str


_LUBM_GPL = (
    "The UBA generator is RUN, NEVER VENDORED: copyleft cannot enter this "
    "MIT OR Apache-2.0 tree. Running it to produce data is not distribution of "
    "it. Cite Guo, Pan and Heflin, Journal of Web Semantics 3(2), in results."
)

_LUBM_UNLICENSED = (
    "NO license, copyright, or rights statement accompanies this file upstream. "
    "Absent an explicit grant there is no permission to redistribute it, so it "
    "is fetched by digest at use time and never copied into this repository."
)

_WATDIV_POSTURE = (
    "CITATION-WARE: free to download and use provided results cite Aluc, Hartig, "
    "Ozsu and Daudjee, 'Diversified Stress Testing of RDF Data Management "
    "Systems', ISWC 2014, pages 197-212. Supplied 'as is', all use at your own "
    "risk. A use grant is not a redistribution grant, so it is not vendored. "
    "DETERMINISM: stock v0.6 seeds from time(0) / srand(time(NULL)) / "
    "random_device and has no seed flag, so the GENERATION PROCESS cannot be "
    "pinned -- only a frozen OUTPUT dataset can be."
)

_WATDIV_FROZEN_OUTPUT = (
    "A FROZEN OUTPUT, not a generation run. Because stock v0.6 has no seed flag "
    "(see the posture above), the only reproducible WatDiv dataset is one that "
    "was generated once and then pinned by digest -- which is exactly what "
    "upstream publishes and what this pin covers. The bytes behind this URL are "
    "therefore the dataset itself, and no generator is built or run to obtain "
    "it. UPSTREAM PUBLISHES NO CHECKSUM FOR IT: the size and sha256 below are "
    "OUR OWN, taken from the bytes served on 2026-09-18, exactly as the LUBM "
    "pins above are ours. There is no publisher checksum to cross-check them "
    "against, so `md5` is None rather than invented. "
    + _WATDIV_POSTURE.split(" DETERMINISM:")[0]
)

ARTIFACTS: tuple[Artifact, ...] = (
    Artifact(
        filename="uba1.7.zip",
        url="https://swat.cse.lehigh.edu/projects/lubm/uba1.7.zip",
        sha256="3d44f468e36b7f3cd532f0f5693a019d020877846397b1fa657367cbd53f380a",
        size=35075,
        md5=None,
        licence="GPL-2.0-or-later",
        posture=_LUBM_GPL,
    ),
    Artifact(
        filename="GeneratorLinuxFix.zip",
        url="https://swat.cse.lehigh.edu/projects/lubm/GeneratorLinuxFix.zip",
        sha256="cc92e7a8373306086c593b519b15a5991f859d40e220e04cebb994d0cdc44be4",
        size=9141,
        md5=None,
        licence="GPL-2.0-or-later",
        posture=(
            "A single modified Generator.java carrying the same GNU General "
            "Public License header as the UBA sources it patches. " + _LUBM_GPL
        ),
    ),
    Artifact(
        filename="queries-sparql.txt",
        url="https://swat.cse.lehigh.edu/projects/lubm/queries-sparql.txt",
        sha256="c34fd26ecb6fb9f0a2f185d73b72505ef0abf25705d831e6eed3c896557cd104",
        size=7195,
        md5=None,
        licence="unlicensed (no grant published)",
        posture="The 14 LUBM test queries. " + _LUBM_UNLICENSED,
    ),
    Artifact(
        filename="univ-bench.owl",
        url="https://swat.cse.lehigh.edu/onto/univ-bench.owl",
        sha256="e6eca926fcb7d6c7925ea0c48f7c5d79abc2818f89e7432fd16561aafeb3f67a",
        size=14433,
        md5=None,
        licence="unlicensed (no grant published)",
        posture="The Univ-Bench domain ontology. " + _LUBM_UNLICENSED,
    ),
    Artifact(
        filename="watdiv_v06.tar",
        url="https://dsg.uwaterloo.ca/watdiv/watdiv_v06.tar",
        sha256="fb8d930b74b3fbc8f948101bfaf658a90d2f74002f1fefda465c45ffd33a71d2",
        size=307200,
        # Published beside the download link on the WatDiv project page.
        md5="9eac247dfdec044d7fa0141ea3ad361f",
        licence="citation-ware",
        posture=_WATDIV_POSTURE,
    ),
    Artifact(
        filename="watdiv.10M.tar.bz2",
        url="https://dsg.uwaterloo.ca/watdiv/watdiv.10M.tar.bz2",
        sha256="1d0a8a4725c98974eb7347ce3e6d9cab44f9f40389589809674254151b745af6",
        size=58558746,
        # Upstream publishes an md5 beside the v0.6 toolkit download but none
        # beside the frozen datasets. Inventing one would be a fabricated
        # publisher statement, so there is none here.
        md5=None,
        licence="citation-ware",
        posture=_WATDIV_FROZEN_OUTPUT,
    ),
)

# The frozen datasets upstream publishes, by scale. Only the 10M one is PINNED
# above, and that is the whole point of listing the others by name: a larger
# scale is a one-line addition to ARTIFACTS, but only after whoever wants it has
# FETCHED AND HASHED IT THEMSELVES. A digest that nobody verified is worse than
# no digest at all -- it turns a download into an unchecked download that prints
# "OK". `scripts/watdiv-lane.sh` refuses an unpinned scale by name and points
# here rather than quietly falling back to 10M.
# Every scale upstream publishes. Which of them this repository has actually
# PINNED is derived from ARTIFACTS rather than restated here, so the two cannot
# disagree: `pinned_watdiv_scales` reads the pins and `unpinned_watdiv_scales` is
# the difference. Both are printed by `--list`, and `scripts/watdiv-lane.sh`
# points an operator at that output instead of carrying its own copy of the list.
WATDIV_SCALES: tuple[str, ...] = ("10M", "100M", "1000M")

# The row count of each pinned dataset AFTER extraction, as a tracked pin.
#
# The tarball's digest is pinned and re-verified every run, so the bytes that go
# INTO an extraction are certain. What comes out is certified only by a stamp the
# first extraction itself wrote -- trust-on-first-use -- and a stamp is not a pin.
# `docs/design/purrdf-bench-lane-laws.md` states that a known count may go
# unasserted only where a digest re-verified AGAINST A PIN already fixes it; the
# corpus digest is re-verified against a stamp, so by that law this count must be
# asserted, and here it is.
#
# Measured from an extraction of the digest-verified tarball rather than copied
# from prose, and cross-checked against the figure `docs/BENCHMARKS.md` publishes.
WATDIV_DATASET_ROWS: dict[str, int] = {"10M": 10_916_457}

# How many BASIC query templates WatDiv publishes: three complex, five snowflake,
# five linear and seven star. One number, pinned once, because the lane needs it
# three times -- the templates it extracts, the queries it instantiates, and the
# denominator of its report -- and three typed literals are three chances to
# disagree. `docs/design/purrdf-bench-lane-laws.md` says a denominator is only a true
# statement if it is derived rather than typed; this is where it is derived from.
WATDIV_BASIC_TEMPLATES: int = 20

# How many queries LUBM publishes. Pinned for the same reason as the template count
# above and read the same way: the LUBM lane enforced it at two shell sites and
# `lubm-queries.py` at two more, four typed literals for one published fact, while the
# sibling lane had already been moved to a pin. The design note says a denominator is
# only a true statement if it is derived rather than typed; this is what both lanes now
# derive from.
LUBM_PUBLISHED_QUERIES: int = 14

# Values that are deterministic functions of the PINNED artifacts and the DEFAULT
# knobs, recorded here so a lane can ASSERT them rather than print them.
#
# Each of these was already deterministic and already printed; printing a known
# value instead of checking it is the "missed refusal" this repository's lane-laws
# document names. They live beside the artifact pins because that is what they are
# derived from -- one copy of each number, next to the bytes that determine it.
#
# A lane compares one of these ONLY when the knobs it depends on are at their
# defaults, and says so when they are not. A different seed or scale is a different
# workload, and asserting a default's value against it would be an over-refusal.
#
# WHAT A SELF-DERIVED PIN DOES AND DOES NOT BUY, stated because the difference is
# easy to overstate. The four DIGESTS below were produced by running the code they
# now pin, so they catch CHANGE and not CORRECTNESS: they will fail the day
# generation, conversion, normalisation or instantiation alters its output, which is
# the regression worth catching, and they would not have caught a value that was
# wrong from the start. What makes them trustworthy is not this file -- it is that
# each is a deterministic function of an artifact pinned by digest against its
# publisher, so the inputs are certain even though the recorded output is our own
# measurement of them.
#
# The two ROW COUNTS are different in kind and stronger: `rows.Q1` and `rows.Q14`
# are LUBM's OWN PUBLISHED ANSWERS for LUBM(1, 0), corroborated by the paper rather
# than by this tree, so those two are an independent oracle and not self-derived at
# all. They are the only external check either lane has.
WORKLOAD_PINS: dict[str, str] = {
    # sha256 over the normalised LUBM query set, at the default ontology namespace.
    "lubm.queries.sha256": (
        "5ad5a5c735bc86625c0f008fc78a2f7cfc30f063de25ad33a36339e2e49774a8"
    ),
    # LUBM's own published answers for LUBM(1, 0) seed 0. These two queries need NO
    # entailment, so they run on the full corpus on any engine and their counts are
    # the only oracle this lane has. `> 0` was letting a conversion bug that halved
    # either one pass silently.
    "lubm.1.0.seed0.rows.Q1": "4",
    "lubm.1.0.seed0.rows.Q14": "5916",
    # sha256 over the EXTRACTED WatDiv corpus. The tarball is pinned and verified
    # every run, but what comes out of an extraction was certified only by a stamp
    # the first extraction itself wrote -- trust-on-first-use, which cannot detect a
    # first extraction that was already wrong because that extraction is what wrote
    # the record. Pinning it removes the TOFU entirely and makes the lane-laws
    # exception ("a digest verified against a pin") true rather than aspirational.
    "watdiv.10M.corpus.sha256": (
        "7cfe0341d578a677d3b5d562eaaf94d67aff8587d9e0ef3d83cc82765b77cddd"
    ),
    # sha256 over the concatenated LUBM corpus at the default knobs -- AND FOR A
    # NAMED BINARY, which is why the version is in the key.
    #
    # These bytes are the purrdf serializer's OUTPUT: the lane converts each
    # generated RDF/XML document with `purrdf convert` and digests the
    # concatenation. So the binary is an input to this digest exactly as the seed
    # is, and a pin taken with one version does not apply to another --
    # `watdiv-lane.sh` already reasons this way about its pack, whose stamp key is
    # the dataset digest AND the binary version.
    #
    # Putting the version in the key rather than in a second condition makes a
    # version bump a MISSING pin, which the lane reports as "not checked for this
    # binary", instead of a mismatch that would blame generation or conversion for
    # a difference the new serializer is entitled to.
    "lubm.1.0.seed0.corpus.sha256.purrdf-2.0.2": (
        "b3fbfcc822092428fcf6e03757f0ca555e39c2b29304bc8638c9d9d05f875308"
    ),
    # sha256 over the instantiated WatDiv query set, at scale 10M and seed 0.
    "watdiv.10M.seed0.queries.sha256": (
        "2fabc0ef56b5d18bb9a7c9d6a4aa5c661043500103d6f133087d39d41fa59301"
    ),
}


def sha256_of(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def md5_of(path: Path) -> str:
    """Compute MD5, to compare against a checksum the PUBLISHER published.

    MD5 is not relied on for integrity here and never stands alone: every
    artifact is also checked against a SHA-256 pin recorded in this file. This
    function exists to compare our bytes against the publisher's own statement
    about those bytes, which is the only form in which that statement exists.
    """
    digest = hashlib.md5(usedforsecurity=False)  # noqa: S324 - publisher-published checksum, not a security primitive
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def verify_digest(path: Path, expected: str, label: str) -> None:
    actual = sha256_of(path)
    if actual != expected:
        sys.exit(
            f"FAIL: {label} digest mismatch\n  expected {expected}\n  actual   {actual}"
        )
    print(f"OK: {label} digest {expected}")


def quarantine(path: Path, actual: str) -> Path:
    """Move bytes that failed verification aside, preserving them as evidence.

    A file that fails its pin is never deleted and never overwritten by a fresh
    download. It is renamed to ``<name>.rejected-<digest prefix>`` so the next
    run sees a cache miss rather than the same bad bytes, while the bad bytes
    remain on disk for an operator to inspect. The digest is part of the name so
    two different failures do not overwrite each other. A ``.part.<pid>`` suffix
    is dropped first, so a failed download is quarantined under the name it was
    trying to become rather than under the scratch name it happened to have — and
    two processes quarantining the same bad bytes converge on one file instead of
    leaving a pid-tagged copy each.
    """
    stem = re.sub(r"\.part(?:\.\d+)?$", "", path.name)
    held = path.with_name(f"{stem}.rejected-{actual[:16]}")
    os.replace(path, held)
    return held


def _scratch_for(dest: Path) -> Path:
    """The scratch name a download writes before it has earned *dest*'s name."""
    return dest.with_name(f"{dest.name}.part.{os.getpid()}")


def _verify_and_install(tmp: Path, dest: Path, artifact: Artifact) -> None:
    """Promote *tmp* to *dest* only if every pinned identity matches.

    *tmp* has already been written. It is ``os.replace``d into place — atomic
    within a filesystem — only after size, SHA-256, and (where upstream publishes
    one) MD5 all agree with the pin. A truncated response, a proxy error page, or
    an interrupted transfer therefore never appears at *dest* under the
    artifact's real name; it is quarantined instead, and the process exits
    non-zero.
    """
    try:
        actual_size = tmp.stat().st_size
        if actual_size != artifact.size:
            held = quarantine(tmp, sha256_of(tmp))
            sys.exit(
                f"FAIL: {artifact.filename} is the wrong size (truncated or replaced upstream)\n"
                f"  url      {artifact.url}\n"
                f"  expected {artifact.size} bytes\n"
                f"  actual   {actual_size} bytes\n"
                f"  quarantined at {held}\n"
                "  nothing was installed into the cache"
            )

        actual = sha256_of(tmp)
        if actual != artifact.sha256:
            held = quarantine(tmp, actual)
            sys.exit(
                f"FAIL: {artifact.filename} sha256 mismatch\n"
                f"  url      {artifact.url}\n"
                f"  expected {artifact.sha256}\n"
                f"  actual   {actual}\n"
                f"  quarantined at {held}\n"
                "  nothing was installed into the cache; check the pin or the upstream file"
            )

        if artifact.md5 is not None:
            actual_md5 = md5_of(tmp)
            if actual_md5 != artifact.md5:
                held = quarantine(tmp, actual)
                sys.exit(
                    f"FAIL: {artifact.filename} md5 disagrees with the publisher's own checksum\n"
                    f"  url      {artifact.url}\n"
                    f"  expected {artifact.md5}\n"
                    f"  actual   {actual_md5}\n"
                    f"  quarantined at {held}\n"
                    "  the sha256 pin matched, so the PIN is what is wrong here, not the download"
                )

        os.replace(tmp, dest)
    finally:
        tmp.unlink(missing_ok=True)


def _install_verified_bytes(data: bytes, dest: Path, artifact: Artifact) -> None:
    """Install *data* at *dest* only if every pinned identity matches."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    # The scratch name carries this process's pid. The cache is a fixed directory
    # under ``target/`` shared by every lane regardless of which arena each one
    # was given, so two concurrent acquisitions — two lanes, or one lane run twice
    # with different ``*_OUT`` — raced on a single ``.part``: both wrote it, and
    # whichever lost ``os.replace`` got a bare ``FileNotFoundError``, or one
    # process hashed the other's partial bytes and quarantined a download that was
    # never corrupt. Separating arenas does not separate the cache, so the cache
    # has to be safe on its own terms. ``os.replace`` onto *dest* stays atomic and
    # every candidate is verified before it, so a unique scratch name is the whole
    # fix.
    tmp = _scratch_for(dest)
    tmp.write_bytes(data)
    _verify_and_install(tmp, dest, artifact)


# A stalled connection with no timeout blocks a lane FOREVER, with the last thing
# printed being "fetching <url>" and no diagnostic ever following it. Python's
# default socket timeout is None, so this has to be stated. The retry is small and
# its backoff is fixed rather than jittered: a transient reset partway through a
# large transfer should not discard the run, and this repository does not
# introduce nondeterminism it does not need.
_FETCH_TIMEOUT_SECONDS = 60
_FETCH_ATTEMPTS = 3
_FETCH_BACKOFF_SECONDS = 2


def _download_verified(artifact: Artifact, dest: Path) -> None:
    """Fetch *artifact* over the network and install it only if it verifies.

    The response is streamed to the scratch file rather than read into memory:
    the pinned WatDiv dataset is tens of megabytes today and this file documents
    how to pin the 1000M one, at which point buffering the whole body would make
    acquisition the memory peak of a lane that otherwise streams everything.
    """
    print(f"  fetching {artifact.url}")
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp = _scratch_for(dest)
    for attempt in range(1, _FETCH_ATTEMPTS + 1):
        try:
            with (
                urllib.request.urlopen(  # noqa: S310 - pinned https URL
                    artifact.url, timeout=_FETCH_TIMEOUT_SECONDS
                ) as response,
                tmp.open("wb") as handle,
            ):
                for chunk in iter(lambda: response.read(1 << 22), b""):  # noqa: B023
                    handle.write(chunk)
            break
        # `http.client.IncompleteRead` is an HTTPException, NOT an OSError, so the
        # commonest mid-transfer truncation -- the exact failure the retry below
        # exists for -- escaped it as a traceback and left a scratch file behind.
        except (urllib.error.URLError, OSError, http.client.HTTPException) as error:
            tmp.unlink(missing_ok=True)
            if attempt == _FETCH_ATTEMPTS:
                sys.exit(
                    f"FAIL: could not fetch {artifact.filename} after "
                    f"{_FETCH_ATTEMPTS} attempt(s)\n"
                    f"  url    {artifact.url}\n"
                    f"  error  {error}\n"
                    "  nothing was installed into the cache"
                )
            print(f"  attempt {attempt} failed ({error}); retrying")
            time.sleep(_FETCH_BACKOFF_SECONDS * attempt)
    _verify_and_install(tmp, dest, artifact)


Fetcher = Callable[[Artifact, Path], None]


def ensure_cached(
    artifact: Artifact, cache_dir: Path, fetch: Fetcher = _download_verified
) -> str:
    """Return ``"cache-hit"`` or ``"downloaded"``, or exit non-zero.

    A present cache entry is re-verified on every run, never trusted by
    existence. If it no longer matches its pin that is an ERROR and not a cache
    miss: the file is quarantined and the run stops. Re-downloading over a file
    that failed verification would turn a detected corruption into a silent one,
    and would mask a pin that is simply wrong.
    """
    dest = cache_dir / artifact.filename
    if dest.exists():
        actual = sha256_of(dest)
        if actual != artifact.sha256:
            held = quarantine(dest, actual)
            sys.exit(
                f"FAIL: cached {artifact.filename} no longer matches its pin\n"
                f"  path     {dest}\n"
                f"  expected {artifact.sha256}\n"
                f"  actual   {actual}\n"
                f"  quarantined at {held}\n"
                "  a cached file that stops verifying is an error, not a cache miss: nothing was\n"
                "  re-downloaded over it. Inspect the quarantined copy, then remove it to refetch."
            )
        # THE PUBLISHER'S MD5 IS CHECKED HERE TOO, because the run PRINTS it.
        #
        # On a cache hit no download happens, so `_verify_and_install` -- which
        # checks md5 before any byte earns the cached name -- was never reached. A
        # WRONG PIN therefore went unexamined while `acquire` still printed
        # "md5 <value> (publisher-published)": a false report of an independent
        # publisher confirmation that nothing had confirmed. Re-checking here is
        # not a duplicate of that rule, it is the same rule on the other path.
        if artifact.md5 is not None:
            actual_md5 = md5_of(dest)
            if actual_md5 != artifact.md5:
                held = quarantine(dest, actual)
                sys.exit(
                    f"FAIL: cached {artifact.filename} disagrees with the publisher's own "
                    "checksum\n"
                    f"  path     {dest}\n"
                    f"  expected {artifact.md5}\n"
                    f"  actual   {actual_md5}\n"
                    f"  quarantined at {held}\n"
                    "  the sha256 pin matched, so the PIN is what is wrong here, not the bytes.\n"
                    "  No publisher confirmation is printed for a checksum that does not agree."
                )
        return "cache-hit"

    cache_dir.mkdir(parents=True, exist_ok=True)
    fetch(artifact, dest)
    return "downloaded"


def _offending_status_lines(status_text: str, top: str) -> list[str]:
    """Return the ``git status --porcelain`` lines that expose *top* to a commit.

    A porcelain line is ``XY <path>``; a rename carries ``old -> new``; a path
    containing an unusual character is quoted. Both sides of a rename and the
    unquoted form are tested, and a match is *top* itself or something beneath
    it — so ``target``, ``target/`` and ``target/bench-artifacts/x`` all match
    while ``targeted-notes.md`` does not. Prefix comparison without the
    separator would flag that last one and turn this guard into noise.
    """
    hits: list[str] = []
    for line in status_text.splitlines():
        if len(line) < 4:
            continue
        for side in line[3:].split(" -> "):
            candidate = side.strip().strip('"').rstrip("/")
            if candidate == top or candidate.startswith(top + "/"):
                hits.append(line)
                break
    return hits


def _licence_posture_sentence() -> str:
    """Describe the pinned set's licensing posture, counted from ARTIFACTS.

    Restating these counts in prose is how a licensing diagnostic goes stale: the
    sentence this replaced still described four artifacts, and named one as GPL,
    after a second GPL artifact and two more pins had been added.
    """
    gpl = sum(1 for a in ARTIFACTS if a.licence.startswith("GPL"))
    ungranted = sum(1 for a in ARTIFACTS if a.licence.startswith("unlicensed"))
    citation = sum(1 for a in ARTIFACTS if a.licence.startswith("citation"))
    parts = []
    if gpl:
        parts.append(f"{gpl} of these artifacts {'is' if gpl == 1 else 'are'} GPL-2.0-or-later")
    if ungranted:
        parts.append(f"{ungranted} carry no licence grant at all")
    if citation:
        parts.append(f"{citation} are citation-ware with no redistribution grant")
    return ", ".join(parts)


def assert_cache_invisible_to_git(cache_dir: Path) -> None:
    """Prove the cache cannot be committed, using the REPOSITORY's own rules.

    The operator's personal ignore file is disabled for this check on purpose.
    An artifact that is only invisible because of a personal ``core.excludesFile``
    is visible in every clone that lacks it, and these artifacts are exactly the
    ones that must never be committed — the posture counts in the diagnostic are
    derived from ARTIFACTS rather than restated, so adding a pin cannot leave a
    licensing sentence describing the set it used to be. Only lines that name the
    cache tree are inspected, so unrelated untracked files an operator ignores
    personally do not make this fail.
    """
    top = cache_dir.relative_to(REPO_ROOT).parts[0]
    result = subprocess.run(
        [
            "git",
            "-C",
            str(REPO_ROOT),
            "-c",
            f"core.excludesFile={os.devnull}",
            "status",
            "--porcelain",
            "--untracked-files=normal",
        ],
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        sys.exit(
            "FAIL: cannot prove the artifact cache is invisible to version control\n"
            f"  git exited {result.returncode}: {result.stderr.strip()}\n"
            "  refusing to download non-redistributable artifacts into a tree whose\n"
            "  ignore state cannot be established"
        )

    offenders = _offending_status_lines(result.stdout, top)
    if offenders:
        listed = "\n".join(f"    {line}" for line in offenders)
        sys.exit(
            f"FAIL: '{top}/' IS VISIBLE TO GIT — THESE ARTIFACTS COULD BE COMMITTED\n"
            f"{listed}\n"
            f"  The repository's own .gitignore must cover '{top}'. It currently does not\n"
            "  (a personal core.excludesFile does not count: a fresh clone has no such file).\n"
            f"  {_licence_posture_sentence()}, so committing them is a licensing\n"
            "  incident, not an untidy tree.\n"
            "  Fix .gitignore before running this again. Nothing was fetched."
        )
    print(f"OK: '{top}/' is ignored by the repository's own rules; the cache cannot be committed")


def select_artifacts(names: list[str] | None) -> tuple[Artifact, ...]:
    """Return the pinned artifacts named by *names*, or all of them if *names* is None.

    An unknown name is a HARD FAILURE that names the bad value and lists every valid
    one — never a silent no-op (fetch nothing) and never a silent fallback (fetch
    everything). This is what lets each lane ask for only the artifacts it
    actually consumes, instead of every lane paying for every pinned artifact
    regardless of which one it uses. ``GeneratorLinuxFix.zip`` is the reason that
    distinction is worth having: it is pinned here so the licensing analysis is on
    the record, and no lane fetches it, because no lane uses it.

    Order follows ``ARTIFACTS``, not *names*, and a name repeated in *names* is
    fetched once: this selects a subset, it does not resequence or multiply it.
    """
    if names is None:
        return ARTIFACTS
    by_filename = {artifact.filename: artifact for artifact in ARTIFACTS}
    unknown = sorted({name for name in names if name not in by_filename})
    if unknown:
        sys.exit(
            f"FAIL: --only names unknown artifact(s): {', '.join(unknown)}\n"
            f"  valid names: {', '.join(sorted(by_filename))}\n"
            "  refusing to fetch nothing and refusing to fetch everything instead"
        )
    wanted = set(names)
    return tuple(artifact for artifact in ARTIFACTS if artifact.filename in wanted)


def pinned_watdiv_scales() -> tuple[str, ...]:
    """The WatDiv scales that are actually pinned, read from ARTIFACTS."""
    return tuple(
        name.removeprefix("watdiv.").removesuffix(".tar.bz2")
        for name in (a.filename for a in ARTIFACTS)
        if name.startswith("watdiv.") and name.endswith(".tar.bz2")
    )


def unpinned_watdiv_scales() -> tuple[str, ...]:
    """The scales upstream publishes that this repository has NOT pinned."""
    pinned = set(pinned_watdiv_scales())
    return tuple(scale for scale in WATDIV_SCALES if scale not in pinned)


def list_artifacts() -> int:
    """Print every pinned artifact with its licence posture and exit 0."""
    print(f"cache: {CACHE}")
    print(
        "Nothing below is vendored. Every file is fetched by digest at use time.\n"
        "The LUBM generator is GPL-2.0-or-later and is RUN, never copied into this tree.\n"
        "Results derived from these workloads must carry the citations noted below."
    )
    for artifact in ARTIFACTS:
        print()
        print(f"{artifact.filename}")
        print(f"  url      {artifact.url}")
        print(f"  sha256   {artifact.sha256}")
        if artifact.md5 is not None:
            print(f"  md5      {artifact.md5}  (published by upstream)")
        print(f"  size     {artifact.size} bytes")
        print(f"  licence  {artifact.licence}")
        for index, line in enumerate(_wrap(artifact.posture, 74)):
            print(f"  posture  {line}" if index == 0 else f"           {line}")
    print()
    for scale, rows in sorted(WATDIV_DATASET_ROWS.items()):
        print(f"watdiv.{scale} extracted row count (pinned): {rows}")
    print(f"WatDiv scales pinned here:  {', '.join(pinned_watdiv_scales())}")
    print(f"published upstream, NOT pinned: {', '.join(unpinned_watdiv_scales())}")
    print(
        "  An unpinned scale is refused by name rather than silently substituted.\n"
        "  To use one, fetch it, hash it yourself, and add it to ARTIFACTS."
    )
    return 0


def _wrap(text: str, width: int) -> list[str]:
    """Wrap *text* to *width* columns on spaces, without importing textwrap."""
    lines: list[str] = []
    current = ""
    for word in text.split():
        if current and len(current) + 1 + len(word) > width:
            lines.append(current)
            current = word
        else:
            current = f"{current} {word}" if current else word
    if current:
        lines.append(current)
    return lines


def _expect_exit(thunk: Callable[[], object], label: str, must_contain: list[str]) -> bool:
    """Assert *thunk* exits non-zero with a message carrying every *must_contain*."""
    try:
        thunk()
    except SystemExit as exc:
        message = str(exc.code)
        if not exc.code or isinstance(exc.code, int) and exc.code == 0:
            print(f"SELF-TEST FAIL: {label} exited zero")
            return False
        missing = [needle for needle in must_contain if needle not in message]
        if missing:
            print(f"SELF-TEST FAIL: {label} message omits {missing}\n  message: {message}")
            return False
        print(f"OK: self-test — {label}")
        return True
    print(f"SELF-TEST FAIL: {label} did not fail at all")
    return False


def self_test() -> int:
    """Exercise the real verification, quarantine and cache logic. OFFLINE.

    No case here touches the network and no case writes inside the repository:
    every fixture lives in a temporary directory. A fetcher that raises on call
    stands in for the network, so a cache-hit path that secretly re-downloaded
    would fail here rather than merely being slow.
    """
    ok = True
    good = b"pinned benchmark artifact bytes\n"
    good_sha = hashlib.sha256(good).hexdigest()
    good_md5 = hashlib.md5(good, usedforsecurity=False).hexdigest()  # noqa: S324 - fixture checksum
    bad = b"corrupted\n"

    fixture = Artifact(
        filename="fixture.bin",
        url="https://example.org/fixture.bin",
        sha256=good_sha,
        size=len(good),
        md5=good_md5,
        licence="test fixture",
        posture="test fixture",
    )

    def explode(_artifact: Artifact, _dest: Path) -> None:
        raise AssertionError("network fetch attempted on a path that must not fetch")

    with tempfile.TemporaryDirectory(prefix="benchmark-acquire-selftest-") as raw:
        tmp = Path(raw)

        # 1. verify_digest accepts matching bytes (the neighbouring VALID case of
        #    the failure below — a verifier that rejects everything also "passes"
        #    a mismatch test).
        sample = tmp / "sample.bin"
        sample.write_bytes(good)
        try:
            verify_digest(sample, good_sha, "self-test sample")
        except SystemExit:
            print("SELF-TEST FAIL: verify_digest rejected bytes that match their digest")
            ok = False
        else:
            print("OK: self-test — verify_digest accepts matching bytes")

        # 2. verify_digest rejects wrong bytes and names both digests.
        wrong = tmp / "wrong.bin"
        wrong.write_bytes(bad)
        ok &= _expect_exit(
            lambda: verify_digest(wrong, good_sha, "self-test sample"),
            "verify_digest rejects wrong bytes naming expected and actual",
            ["FAIL:", good_sha, hashlib.sha256(bad).hexdigest()],
        )

        # 3. Cache hit: a correct cache entry is re-verified and NOT re-fetched.
        hit_dir = tmp / "cache-hit"
        hit_dir.mkdir()
        (hit_dir / fixture.filename).write_bytes(good)
        try:
            status = ensure_cached(fixture, hit_dir, fetch=explode)
        except AssertionError:
            print("SELF-TEST FAIL: a good cache entry triggered a fetch")
            ok = False
        else:
            if status != "cache-hit":
                print(f"SELF-TEST FAIL: good cache entry reported {status!r}, expected 'cache-hit'")
                ok = False
            else:
                print("OK: self-test — a verified cache entry is a cache hit with no fetch")

        # 4. Cache miss: the fetcher runs and the installed bytes are exact.
        miss_dir = tmp / "cache-miss"

        def install_good(artifact: Artifact, dest: Path) -> None:
            _install_verified_bytes(good, dest, artifact)

        status = ensure_cached(fixture, miss_dir, fetch=install_good)
        installed = miss_dir / fixture.filename
        if status != "downloaded" or not installed.exists() or installed.read_bytes() != good:
            print(f"SELF-TEST FAIL: cache miss did not install verified bytes (status {status!r})")
            ok = False
        else:
            print("OK: self-test — a cache miss installs bytes that verify")

        # 5. Corrupt cache entry: hard failure, quarantined, never re-fetched.
        rot_dir = tmp / "cache-rot"
        rot_dir.mkdir()
        rotten = rot_dir / fixture.filename
        rotten.write_bytes(bad)
        ok &= _expect_exit(
            lambda: ensure_cached(fixture, rot_dir, fetch=explode),
            "a cached file that stops verifying is a hard failure, not a cache miss",
            ["FAIL:", good_sha, hashlib.sha256(bad).hexdigest(), "quarantined at"],
        )
        if rotten.exists():
            print("SELF-TEST FAIL: the corrupt cache entry was left in place under its real name")
            ok = False
        elif not list(rot_dir.glob(f"{fixture.filename}.rejected-*")):
            print("SELF-TEST FAIL: the corrupt cache entry was not quarantined")
            ok = False
        else:
            print("OK: self-test — corrupt cache bytes are quarantined, not deleted or overwritten")

        # 6. A fetch that returns wrong bytes never lands at the real name.
        fetch_dir = tmp / "bad-fetch"
        fetch_dir.mkdir()
        target = fetch_dir / fixture.filename
        ok &= _expect_exit(
            lambda: _install_verified_bytes(bad, target, fixture),
            "wrong fetched bytes are rejected before installation",
            ["FAIL:", "quarantined at"],
        )
        if target.exists():
            print("SELF-TEST FAIL: unverified fetched bytes were installed at the real name")
            ok = False
        elif list(fetch_dir.glob(fixture.filename + ".part*")):
            print("SELF-TEST FAIL: a .part file was left behind after a rejected fetch")
            ok = False
        else:
            print("OK: self-test — unverified fetched bytes never reach the cached name")

        # 7. A right-sized file with the wrong content still fails on sha256,
        #    so the cheap size check cannot stand in for the digest.
        same_size = bytes(len(good))
        ok &= _expect_exit(
            lambda: _install_verified_bytes(same_size, fetch_dir / "sized.bin", fixture),
            "right size with wrong content still fails the digest",
            ["sha256 mismatch", good_sha],
        )

        # 7b. The md5 refusal, which is the one branch that was covered only on
        #     its VALID side: every case above carries a fixture whose md5 is
        #     right, so nothing ever proved the refusal fires, or that its
        #     message says what it is for. It is the only check that can fail
        #     with the sha256 pin already matching, which means the PIN is wrong
        #     rather than the download — and that sentence had never been
        #     executed.
        mismatched_md5 = fixture._replace(md5="0" * 32)
        ok &= _expect_exit(
            lambda: _install_verified_bytes(good, fetch_dir / "md5.bin", mismatched_md5),
            "bytes matching sha256 but not the publisher's md5 are refused",
            ["md5 disagrees with the publisher's own checksum", "the PIN is what is wrong here"],
        )
        if (fetch_dir / "md5.bin").exists():
            print("SELF-TEST FAIL: bytes failing the md5 cross-check were installed anyway")
            ok = False
        else:
            print("OK: self-test — an md5 mismatch is refused and nothing is installed")

    # 7c. The licensing sentence is DERIVED from ARTIFACTS, so it is checked like
    #     any other derived value. It is reached only from a `sys.exit` branch, so
    #     nothing else in the gate would notice it going stale -- which is how its
    #     hand-written predecessor came to describe four artifacts when there were
    #     six, and to call one GPL when two are.
    posture = _licence_posture_sentence()
    gpl = sum(1 for a in ARTIFACTS if a.licence.startswith("GPL"))
    if f"{gpl} of these artifacts" not in posture or "GPL-2.0-or-later" not in posture:
        print(f"SELF-TEST FAIL: the licensing sentence does not count the GPL pins: {posture}")
        ok = False
    elif str(len(ARTIFACTS)) == posture:
        print("SELF-TEST FAIL: the licensing sentence is a bare total, not a posture breakdown")
        ok = False
    else:
        print(f"OK: self-test — the licensing posture sentence is derived from the pins ({posture})")

    # 7c-bis. THE CACHE-HIT MD5 REFUSAL, whose valid side was the only one executed.
    #         It was added because a wrong pin printed "md5 ... (publisher-published)"
    #         on every warm-cache run with nothing hashing the bytes -- and then it too
    #         went untested, which is the same omission one layer down.
    with tempfile.TemporaryDirectory() as raw:
        warm = Path(raw)
        (warm / fixture.filename).write_bytes(good)
        wrong_md5 = fixture._replace(md5="0" * 32)
        ok &= _expect_exit(
            lambda: ensure_cached(wrong_md5, warm, explode),
            "a cached entry whose publisher md5 disagrees is refused, not reported OK",
            ["disagrees with the publisher", "the PIN is what is wrong here"],
        )
        if (warm / fixture.filename).exists():
            print("SELF-TEST FAIL: the cache entry failing the md5 cross-check was left in place")
            ok = False
        elif not list(warm.glob(f"{fixture.filename}.rejected-*")):
            print("SELF-TEST FAIL: the cache entry failing the md5 cross-check was not quarantined")
            ok = False
        else:
            print("OK: self-test — a cached md5 mismatch is quarantined and nothing is refetched")

    # 7d. THE PINS THIS FILE EXISTS TO HOLD, which had no check of their own. The
    #     artifact pins were covered and the workload pins were not -- and one of them
    #     carries a binary version in its key, so a version bump silently makes it
    #     unreachable unless something asserts that every recorded key is one a lane can
    #     actually construct.
    seen_keys = sorted(WORKLOAD_PINS)
    if len(seen_keys) != len(set(seen_keys)):
        print("SELF-TEST FAIL: a workload pin name is recorded twice")
        ok = False
    elif any(not key or key != key.strip() for key in seen_keys):
        print("SELF-TEST FAIL: a workload pin name is empty or carries stray whitespace")
        ok = False
    else:
        print(f"OK: self-test — {len(seen_keys)} workload pin name(s) are well formed and unique")

    digests = {k: v for k, v in WORKLOAD_PINS.items() if k.endswith(".sha256") or ".sha256." in k}
    malformed = {
        key: value
        for key, value in digests.items()
        if len(value) != 64 or any(c not in "0123456789abcdef" for c in value)
    }
    if malformed:
        print(f"SELF-TEST FAIL: workload pin(s) are not 64 lowercase hex characters: {malformed}")
        ok = False
    else:
        print(f"OK: self-test — all {len(digests)} digest pin(s) are 64 lowercase hex characters")

    counts = {k: v for k, v in WORKLOAD_PINS.items() if ".rows." in k}
    if not counts or any(not v.isdigit() or int(v) <= 0 for v in counts.values()):
        print(f"SELF-TEST FAIL: a published-answer pin is not a positive integer: {counts}")
        ok = False
    else:
        print(f"OK: self-test — all {len(counts)} published-answer pin(s) are positive integers")

    # The binary-keyed pin must name the version this workspace builds, or the lane it
    # serves will report "no pin recorded" for the binary it just built -- a pin that
    # is recorded and unreachable, which is worse than one that is absent.
    version_keyed = [k for k in WORKLOAD_PINS if ".purrdf-" in k]
    workspace_version = None
    cargo_toml = REPO_ROOT / "Cargo.toml"
    if cargo_toml.exists():
        for line in cargo_toml.read_text(encoding="utf-8").splitlines():
            stripped = line.strip()
            if stripped.startswith("version = "):
                workspace_version = stripped.split('"')[1]
                break
    if not version_keyed:
        print("SELF-TEST FAIL: no version-keyed pin is recorded, so nothing pins the serializer")
        ok = False
    elif workspace_version is None:
        print("SELF-TEST FAIL: could not read the workspace version to check the pin key")
        ok = False
    elif not all(key.endswith(f".purrdf-{workspace_version}") for key in version_keyed):
        print(
            f"SELF-TEST FAIL: version-keyed pin(s) {version_keyed} do not name the workspace "
            f"version {workspace_version}, so the lane will report no pin for the binary it built"
        )
        ok = False
    else:
        print(
            f"OK: self-test — every version-keyed pin names the workspace version "
            f"({workspace_version})"
        )

    if LUBM_PUBLISHED_QUERIES <= 0:
        print(f"SELF-TEST FAIL: the LUBM query count pin is not positive ({LUBM_PUBLISHED_QUERIES})")
        ok = False
    else:
        print(f"OK: self-test — the LUBM published-query count is pinned ({LUBM_PUBLISHED_QUERIES})")

    if WATDIV_BASIC_TEMPLATES <= 0 or not WATDIV_DATASET_ROWS:
        print("SELF-TEST FAIL: the template count or the dataset row pins are empty")
        ok = False
    elif any(rows <= 0 for rows in WATDIV_DATASET_ROWS.values()):
        print(f"SELF-TEST FAIL: a pinned dataset row count is not positive: {WATDIV_DATASET_ROWS}")
        ok = False
    elif set(WATDIV_DATASET_ROWS) - set(pinned_watdiv_scales()):
        print(
            "SELF-TEST FAIL: a row count is pinned for a scale whose artifact is not: "
            f"{sorted(set(WATDIV_DATASET_ROWS) - set(pinned_watdiv_scales()))}"
        )
        ok = False
    else:
        print("OK: self-test — the template count and every dataset row pin are positive and pinned")

    # 8. The git-visibility matcher flags the cache tree and nothing adjacent.
    flagged = _offending_status_lines(
        "?? target\n"
        "?? target/\n"
        "?? target/bench-artifacts/uba1.7.zip\n"
        "?? targeted-notes.md\n"
        " M crates/bench/src/lib.rs\n"
        'R  "old name" -> "target/bench-artifacts/x"\n',
        "target",
    )
    if len(flagged) != 4:
        print(f"SELF-TEST FAIL: git-visibility matcher flagged {len(flagged)} lines, expected 4")
        print(f"  flagged: {flagged}")
        ok = False
    elif any("targeted-notes" in line or "crates/bench" in line for line in flagged):
        print(f"SELF-TEST FAIL: git-visibility matcher flagged an unrelated path: {flagged}")
        ok = False
    else:
        print("OK: self-test — the git-visibility matcher flags the cache tree and nothing else")

    # 9. Every pin is well formed. A malformed pin can never match anything, so
    #    it would turn this script into an unconditional refusal.
    seen: set[str] = set()
    for artifact in ARTIFACTS:
        problems = []
        if len(artifact.sha256) != 64 or not all(c in "0123456789abcdef" for c in artifact.sha256):
            problems.append("sha256 is not 64 lowercase hex characters")
        if artifact.md5 is not None and (
            len(artifact.md5) != 32 or not all(c in "0123456789abcdef" for c in artifact.md5)
        ):
            problems.append("md5 is not 32 lowercase hex characters")
        if artifact.size <= 0:
            problems.append("size is not positive")
        if not artifact.url.startswith("https://"):
            problems.append("url is not https")
        if "/" in artifact.filename or artifact.filename in seen:
            problems.append("filename is not a unique bare name")
        seen.add(artifact.filename)
        if problems:
            print(f"SELF-TEST FAIL: pin for {artifact.filename}: {'; '.join(problems)}")
            ok = False
    if len(seen) == len(ARTIFACTS):
        print(f"OK: self-test — all {len(ARTIFACTS)} pins are well formed and uniquely named")

    # 10. --only selection: no argument fetches everything (unchanged behaviour);
    #     a real name selects exactly that subset (the neighbouring VALID case);
    #     an unknown name is a hard failure naming itself and listing every valid
    #     name (never a silent fetch-nothing, never a silent fetch-everything); a
    #     name repeated does not duplicate the artifact; and a request naming both
    #     a known and an unknown artifact still refuses, rather than fetching the
    #     known one and dropping the rest quietly.
    if select_artifacts(None) != ARTIFACTS:
        print("SELF-TEST FAIL: select_artifacts(None) did not return every pinned artifact")
        ok = False
    else:
        print("OK: self-test — select_artifacts(None) selects every pinned artifact")

    one_name = ARTIFACTS[0].filename
    other_name = ARTIFACTS[1].filename
    selected = select_artifacts([one_name])
    if selected != (ARTIFACTS[0],):
        print(f"SELF-TEST FAIL: select_artifacts([{one_name!r}]) returned {selected!r}")
        ok = False
    else:
        print(f"OK: self-test — select_artifacts selects a single named artifact ({one_name})")

    selected = select_artifacts([other_name, one_name, one_name])
    if selected != (ARTIFACTS[0], ARTIFACTS[1]):
        print(
            "SELF-TEST FAIL: select_artifacts with a repeated name and reversed order "
            f"returned {selected!r}, expected ARTIFACTS order with no duplicate"
        )
        ok = False
    else:
        print(
            "OK: self-test — select_artifacts follows ARTIFACTS order and de-duplicates "
            "a repeated name"
        )

    ok &= _expect_exit(
        lambda: select_artifacts(["not-a-real-artifact.zip"]),
        "select_artifacts refuses an unknown name, naming it and listing valid names",
        ["FAIL:", "not-a-real-artifact.zip", one_name, other_name],
    )

    # The neighbouring valid case, run immediately after the refusal above: proof
    # the refusal is not sticky and a real name still works right after a bad one.
    if select_artifacts([one_name]) != (ARTIFACTS[0],):
        print(
            "SELF-TEST FAIL: a valid name failed right after an unknown-name refusal "
            "-- the refusal must not be sticky"
        )
        ok = False
    else:
        print("OK: self-test — a valid --only name still works immediately after a refusal")

    ok &= _expect_exit(
        lambda: select_artifacts([one_name, "not-a-real-artifact.zip"]),
        "select_artifacts refuses a mix of one known and one unknown name",
        ["FAIL:", "not-a-real-artifact.zip"],
    )

    print("SELF-TEST PASS" if ok else "SELF-TEST FAIL")
    return 0 if ok else 1


def acquire(artifacts: tuple[Artifact, ...]) -> int:
    """Fetch-and-verify *artifacts* into the cache, printing a PASS summary. Exits non-zero on failure."""
    assert_cache_invisible_to_git(CACHE)
    print(f"cache: {CACHE}")

    fetched = 0
    for artifact in artifacts:
        status = ensure_cached(artifact, CACHE)
        if status == "downloaded":
            fetched += 1
        verify_digest(CACHE / artifact.filename, artifact.sha256, artifact.filename)
        if artifact.md5 is not None:
            # Not re-checked HERE, because both paths that can reach this line
            # have already checked it: `_verify_and_install` before any byte earns
            # the cached name, and `ensure_cached` on a cache hit. A third copy
            # would be a weaker statement of a rule stated twice already. What
            # matters is that no path prints this line without having verified it
            # -- the earlier version of this comment argued against duplication
            # while leaving the cache-hit path unverified, which is how a false
            # publisher confirmation got printed.
            print(f"OK: {artifact.filename} md5 {artifact.md5} (publisher-published)")
        print(f"     {status}  {artifact.licence}")

    assert_cache_invisible_to_git(CACHE)
    print(
        f"PASS: {len(artifacts)} artifacts verified ({fetched} fetched, "
        f"{len(artifacts) - fetched} already cached) in {CACHE}\n"
        "      None of them is vendored or redistributable from here. Results derived from\n"
        "      WatDiv must cite Aluc, Hartig, Ozsu and Daudjee (ISWC 2014, pages 197-212);\n"
        "      the LUBM generator is GPL-2.0-or-later and is run, never copied into this tree.\n"
        "      WatDiv v0.6 has no seed flag: pin a generated dataset, never a generation run."
    )
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--list",
        action="store_true",
        help="print every pinned artifact, its URL, its digest and its licence posture",
    )
    parser.add_argument(
        "--template-count",
        action="store_true",
        help="print the pinned count of published WatDiv basic templates and exit",
    )
    parser.add_argument(
        "--lubm-query-count",
        action="store_true",
        help="print the pinned count of published LUBM queries and exit",
    )
    parser.add_argument(
        "--workload-pin",
        metavar="NAME",
        help="print the recorded value of a workload pin and exit",
    )
    parser.add_argument(
        "--dataset-rows",
        metavar="SCALE",
        help="print the pinned extracted row count for SCALE and exit",
    )
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument(
        "--only",
        nargs="+",
        metavar="NAME",
        default=None,
        help=(
            "fetch only the named artifact(s) (bare filenames, e.g. uba1.7.zip) instead of "
            "every pinned artifact. An unknown name is a hard failure that lists the valid "
            "ones. Omit --only to fetch everything, unchanged from before this flag existed."
        ),
    )
    args = parser.parse_args()

    if args.self_test:
        return self_test()
    if args.template_count:
        print(WATDIV_BASIC_TEMPLATES)
        return 0
    if args.lubm_query_count:
        print(LUBM_PUBLISHED_QUERIES)
        return 0
    if args.workload_pin is not None:
        value = WORKLOAD_PINS.get(args.workload_pin)
        if value is None:
            sys.exit(
                f"FAIL: no workload pin named {args.workload_pin!r}.\n"
                f"  Recorded: {', '.join(sorted(WORKLOAD_PINS))}.\n"
                "  This tool will not invent a value for a workload it has not recorded."
            )
        print(value)
        return 0
    if args.dataset_rows is not None:
        rows = WATDIV_DATASET_ROWS.get(args.dataset_rows)
        if rows is None:
            sys.exit(
                f"FAIL: no extracted row count is pinned for WatDiv scale "
                f"{args.dataset_rows!r}. Pinned: {', '.join(sorted(WATDIV_DATASET_ROWS))}.\n"
                "  A scale whose row count is not pinned cannot have that count asserted, and\n"
                "  this tool will not invent one."
            )
        print(rows)
        return 0
    if args.list:
        return list_artifacts()

    return acquire(select_artifacts(args.only))


if __name__ == "__main__":
    sys.exit(main())
