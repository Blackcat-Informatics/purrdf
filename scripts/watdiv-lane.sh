#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Drive the WatDiv comparison workload end to end: acquire, extract, instantiate,
# load, query.
#
# WatDiv is the second workload the RDF-store literature compares engines on, and
# it asks a different question from LUBM. LUBM asks what an engine can DERIVE;
# WatDiv asks how a planner copes with query SHAPE and SELECTIVITY over a skewed
# dataset -- stars, chains, snowflakes and complex patterns, all of them pure
# basic graph patterns with no inference anywhere. This script is that lane. It is
# REPORT-ONLY and never a gate: it prints what it measured and exits 0, or it
# fails loudly and exits non-zero. No number it prints is asserted anywhere
# (docs/BENCHMARKS.md, "measured, never asserted").
#
# NOTHING IS VENDORED
# ===================
#
# WatDiv is citation-ware: free to use provided results cite the ISWC 2014 paper,
# supplied as is. That is a USE grant, not a redistribution grant. So no WatDiv
# byte lives in this tree -- not the toolkit, not the dataset, and not the query
# templates -- and neither do the queries this lane derives from those templates,
# because a query mechanically derived from a template is still derived from it.
# Everything is fetched by digest into an ignored cache under `target/` by
# scripts/benchmark-acquire.py, used from there, and left there. Every file this
# lane writes is build output under `target/`.
#
# THE GENERATOR IS NEVER BUILT, AND THAT IS THE DESIGN
# ====================================================
#
# Stock WatDiv v0.6 seeds itself from `boost::mt19937(time(0))`,
# `srand(time(NULL))` and `std::random_device`, and exposes no seed flag. Two runs
# of the same binary over the same model produce different data, so a WatDiv
# dataset is reproducible only if it was generated ONCE and then pinned by digest.
# (The source would not compile as-is either: it calls `std::random_shuffle`,
# which C++17 removed.) Upstream publishes exactly such frozen datasets, and this
# lane consumes one. THE GENERATOR IS NOT BUILT AND NOT RUN, anywhere, ever.
#
# INSTANTIATION IS DETERMINISTIC HERE, WHICH UPSTREAM CANNOT BE
# =============================================================
#
# The 20 published templates are not queries: each carries `#mapping` directives
# and `%vN%` placeholders that something must fill in. Upstream's instantiator
# draws from the same time-seeded generators, so its query sets cannot be
# regenerated either.
#
# Pinning the dataset fixes that, because it fixes the candidate set behind every
# mapping. scripts/watdiv-queries.py therefore chooses each substitution with
# splitmix64 over WATDIV_SEED and a pinned per-mapping stream, from candidate sets
# scraped out of the frozen dataset and sorted canonically -- and records every
# choice. The same dataset and the same seed reproduce the queries BYTE FOR BYTE.
#
# A QUERY SET FROM A DIFFERENT SEED IS A DIFFERENT WORKLOAD. The seed is printed
# in the summary next to the numbers it governs, for the same reason the LUBM lane
# prints each row's entailment regime: a number whose conditions are not attached
# to it cannot be compared with anything.
#
# NO ENTAILMENT, AND THAT IS NOT AN OMISSION
# ==========================================
#
# Every WatDiv template is a pure basic graph pattern. There is no regime to
# choose and none is used, which is exactly why these rows must NOT be compared
# against LUBM rows: eleven of LUBM's fourteen queries have no answers at all
# without inference. Same engine, different question, different data.
#
# WHAT THE TIMINGS INCLUDE
# ========================
#
# Each query is one process invocation, so its wall time includes opening the data
# source -- which at this scale dominates. Reporting that as query cost would be a
# misleading number, so the lane MEASURES the open cost once with a trivial probe
# and reports it, per query, as a separate column alongside the total.
#
# Knobs, all overridable exactly like the LUBM lane's `LUBM_*`:
#
#   WATDIV_SCALE  which pinned frozen dataset to use (default 10M)
#   WATDIV_SEED   the instantiation seed (default 0)
#   WATDIV_OUT    where the lane works (default target/watdiv)
#   WATDIV_BIN    a prebuilt `purrdf` to use instead of building one

set -euo pipefail

# The laws every lane in this repository shares — how it dies, how its scratch
# directory is cleaned up, how the certificates it wrote are revoked when it
# fails, and how the executable that certifies every number is validated — live
# in ONE implementation that all three lanes source. See scripts/lane-common.sh.
LANE="watdiv-lane"
LANE_BINARY="purrdf binary"
# shellcheck source=scripts/lane-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/lane-common.sh"

SCALE="${WATDIV_SCALE:-10M}"
SEED="${WATDIV_SEED:-0}"
OUT="${WATDIV_OUT:-target/watdiv}"
BIN="${WATDIV_BIN:-}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CACHE="${REPO_ROOT}/target/bench-artifacts"

# A WRITE WHOSE STATUS IS NOT CHECKED IS A SILENT DROP, and `lane_write_checked`
# is where every redirection in all three lanes goes. `$1` is the destination,
# `$2` the role, and the rest is the command whose stdout becomes the file.
write_checked() {
  local destination="$1" role="$2"
  shift 2
  lane_write_checked "${destination}" "${role}" \
    "'${destination}' (under WATDIV_OUT='${OUT}')" "$@"
}

# `WATDIV_OUT` is a knob, so an arena that cannot be created is a LANE failure
# quoting the bytes back, not a bare `mkdir: cannot create directory ...`.
mkdir_checked() {
  lane_mkdir_checked "$1" "WATDIV_OUT='${OUT}'"
}

# EXISTING IS NOT BEING PRODUCED. An artifact this lane goes on to digest, load,
# query or report a size for must be a regular, non-empty file; a zero-byte one
# reports as a very fast engine and digests to the SHA-256 of the empty string.
require_nonempty_file() {
  lane_require_nonempty_file "$1" "$2"
}

# The 8-byte magic every native pack begins with. NON-EMPTY IS NOT "IS A PACK":
# a CLI that wrote eight bytes of something else and exited 0 passed an emptiness
# test, was STAMPED as this dataset's pack, and every later run then reported
# "reusing the one already stamped with this dataset digest and this binary" and
# queried those eight bytes.
readonly PACK_MAGIC="PURRPCK1"

step() {
  echo ""
  echo "=== $* ==="
}

require_uint() {
  local name="$1" value="$2"
  [[ "${value}" =~ ^[0-9]+$ ]] ||
    die "${name} must be a decimal unsigned integer (got '${value}')"
}

require_uint WATDIV_SEED "${SEED}"

# Only the 10M dataset is pinned. A larger scale is a one-line addition to
# scripts/benchmark-acquire.py -- but only by whoever fetches and hashes it
# themselves. Falling back to 10M when asked for 100M would report a number for
# the wrong corpus under the right name, so the request is REFUSED by name.
[[ "${SCALE}" == "10M" ]] ||
  die "WATDIV_SCALE='${SCALE}' is not pinned; only 10M is.
  Upstream also publishes watdiv.100M.tar.bz2 and watdiv.1000M.tar.bz2. To use one,
  fetch it, hash it YOURSELF, and add it to ARTIFACTS in scripts/benchmark-acquire.py.
  This lane will not invent a digest and will not silently run a different corpus."

TARBALL="watdiv.${SCALE}.tar.bz2"
DATASET_NAME="watdiv.${SCALE}.nt"

# `WATDIV_BIN` NAMES THE EXECUTABLE EVERY NUMBER IN THIS REPORT IS ABOUT, and it
# is validated by the SAME implementation `SCALE_BIN` and `LUBM_BIN` are:
# `lane_require_executable` for the path, `lane_run_probe` for whether it runs,
# `lane_require_probe_said_something` for whether it produced anything.
#
# `[[ -x ]]` ALONE IS NOT A CHECK FOR AN EXECUTABLE: a DIRECTORY carries the
# execute bit, so `WATDIV_BIN=/tmp` reached the version probe and was refused
# there with "is not a working purrdf binary" — true, but silent about WHY. The
# tests below name the fault.
#
# AND RUNNING IS NOT LOADING. A CLI that answers `--version` and then writes eight
# bytes of something that is not a pack passed every check this lane had, was
# STAMPED as this dataset's pack, and was reused by every later run. So the probe
# here is a ROUND TRIP, exactly as `scale-corpus.sh` round-trips a manifest: the
# binary is handed one triple of N-Triples and must hand back something that
# begins with the bytes every pack begins with.
#
# `$1` is how to name the binary in a diagnostic; `$2` is 1 when `WATDIV_BIN`
# supplied the path and 0 when this lane built it. Sets `PURRDF_VERSION`.
validate_purrdf_bin() {
  local provenance="$1" from_knob="$2"
  lane_require_executable WATDIV_BIN "${from_knob}" "${BIN}" "the purrdf CLI"

  lane_run_probe "${provenance}" "for its version" "${BIN}" --version
  # `PURRDF_VERSION` is half of `PACK_KEY`, the stamp that decides whether a
  # LATER run may reuse this run's pack. An empty version (`/bin/true` exits 0
  # and prints nothing) would make that stamp certify a pack built by an unknown
  # binary.
  lane_require_probe_said_something "${provenance}" "for its version" \
    "A binary that says nothing is not the purrdf CLI, and this lane records what it
  printed as half of the stamp that lets a later run reuse this run's pack."
  PURRDF_VERSION="${LANE_PROBE_OUT}"

  # One triple, in the reserved documentation domain this repository's fixtures
  # use. It exercises the same `--from ntriples --to pack` path step 5 uses on the
  # frozen dataset, and nothing about it depends on WatDiv.
  local probe_in="${LANE_TMP}/probe.nt" probe_out="${LANE_TMP}/probe.pack"
  printf '%s\n' \
    '<http://example.org/lane-probe/s> <http://example.org/lane-probe/p> <http://example.org/lane-probe/o> .' \
    >"${probe_in}"
  rm -f "${probe_out}"
  lane_run_probe "${provenance}" \
    "to load a one-triple N-Triples document into a pack" \
    "${BIN}" convert --from ntriples --to pack "${probe_in}" "${probe_out}"
  lane_require_magic "${probe_out}" \
    "the pack ${provenance} produced from a one-triple document" \
    "${PACK_MAGIC}" "a purrdf pack"
}

# VALIDATED HERE, BEFORE STEP 1. A knob error is not worth a 58 MB download and a
# gigabyte of bzip2 extraction before it is noticed, and every one of those is a
# chance for the real fault to be mistaken for a problem with the corpus. An
# unset `WATDIV_BIN` is validated at step 2 instead, once the build that produces
# the binary has run.
BIN_FROM_KNOB=0
if [[ -n "${BIN}" ]]; then
  BIN_FROM_KNOB=1
  validate_purrdf_bin "WATDIV_BIN='${BIN}'" 1
fi

# Milliseconds since the epoch. `bc` is not assumed present, so every duration is
# integer arithmetic over nanoseconds.
now_ms() {
  echo $(($(date +%s%N) / 1000000))
}

# `WATDIV_OUT` IS HONOURED AS WRITTEN. An absolute path is the arena, verbatim; a
# relative one is resolved against the repository root, which is what the
# `target/watdiv` default has always meant and what keeps the lane independent of
# the caller's working directory.
#
# Prefixing `REPO_ROOT` unconditionally — which is what this did — quietly turned
# `WATDIV_OUT=/mnt/big/arena` into `<repo>/mnt/big/arena`: nothing appeared where
# the operator asked, the arena landed INSIDE the working tree and outside
# `target/`, and the lane reported success. `docs/BENCHMARKS.md` documents an
# absolute `SCALE_OUT` and `SCALE_OUT` already behaves this way, so the trap was
# one the documentation trained an operator straight into.
case "${OUT}" in
  /*) ARENA="${OUT}" ;;
  *) ARENA="${REPO_ROOT}/${OUT}" ;;
esac
DATASET="${ARENA}/${DATASET_NAME}"
CENSUS="${ARENA}/saved.txt"
TESTSUITE="${ARENA}/testsuite"
MODEL="${ARENA}/model/wsdbm-data-model.txt"
QUERIES="${ARENA}/queries"
PACK="${ARENA}/${DATASET_NAME%.nt}.pack"
DATA_STAMP="${ARENA}/.dataset-stamp"
PACK_STAMP="${ARENA}/.pack-stamp"

# AND SO IS THE ARENA. `WATDIV_OUT` is a knob exactly as `WATDIV_BIN` is, and a
# knob error is not worth a download either: an unusable arena discovered at step 3
# has already cost a 58 MB fetch, and an operator reading the failure has to work
# out which of the two knobs it was about. The arena is created here instead, once,
# before step 1.
mkdir_checked "${ARENA}"


# ── 1. Artifacts ────────────────────────────────────────────────────────────────

step "1/7 artifacts (pinned, fetched by digest, never vendored)"
python3 "${REPO_ROOT}/scripts/benchmark-acquire.py" --only watdiv_v06.tar "${TARBALL}" ||
  die "artifact acquisition failed -- nothing downstream can be trusted, stopping"

for required in "${TARBALL}" watdiv_v06.tar; do
  require_nonempty_file "${CACHE}/${required}" \
    "${required}, which acquisition reported it had cached,"
done

TARBALL_SHA="$(python3 -c '
import hashlib, sys
digest = hashlib.sha256()
with open(sys.argv[1], "rb") as handle:
    for chunk in iter(lambda: handle.read(1 << 22), b""):
        digest.update(chunk)
print(digest.hexdigest())
' "${CACHE}/${TARBALL}")"
echo "frozen dataset tarball: ${TARBALL}  sha256 ${TARBALL_SHA}"

# ── 2. The purrdf binary ────────────────────────────────────────────────────────

step "2/7 purrdf CLI"
if ((BIN_FROM_KNOB == 0)); then
  echo "building purrdf (release)..." >&2
  cargo build --locked --release -p purrdf-cli >&2 ||
    die "cargo build -p purrdf-cli failed"
  BIN="${REPO_ROOT}/target/release/purrdf"
  validate_purrdf_bin "the binary this lane built, '${BIN}'" 0
fi
echo "purrdf: ${BIN}  (${PURRDF_VERSION})"

# ── 3. Extract the frozen dataset and the 20 templates ──────────────────────────

step "3/7 extract the frozen dataset and the 20 published templates into target/"
command -v tar >/dev/null 2>&1 || die "tar is not on PATH"
command -v bzip2 >/dev/null 2>&1 || die "bzip2 is not on PATH; the dataset is bzip2-compressed"

mkdir_checked "${ARENA}"

# Extraction is keyed by the tarball's DIGEST, not by the presence of a file or by
# its mtime, so a stale extraction from other bytes is a miss and never a hit.
#
# A STAMP IS A CERTIFICATE: it is what a LATER run consults to skip this work
# entirely. So it is written only after the artifacts it certifies have been
# checked for being artifacts, and the reuse test demands a NON-EMPTY dataset —
# `-f` alone would have made a zero-byte `watdiv.10M.nt` reusable forever.
extract_start="$(now_ms)"
if [[ -f "${DATA_STAMP}" && "$(cat "${DATA_STAMP}")" == "${TARBALL_SHA}" && -s "${DATASET}" && -s "${CENSUS}" ]]; then
  echo "dataset: reusing the extraction already stamped with this tarball's digest"
  extracted="reused"
else
  rm -f "${DATASET}" "${CENSUS}" "${DATA_STAMP}"
  echo "extracting ${TARBALL} (this takes a minute; it expands to well over a gigabyte)..."
  tar xjf "${CACHE}/${TARBALL}" -C "${ARENA}" ||
    die "could not extract ${TARBALL}"
  require_nonempty_file "${DATASET}" "${DATASET_NAME}, which ${TARBALL} must contain,"
  require_nonempty_file "${CENSUS}" \
    "saved.txt (the entity census the candidate scrape is checked against), which ${TARBALL} must contain,"
  write_checked "${DATA_STAMP}" "the dataset stamp" printf '%s\n' "${TARBALL_SHA}"
  # A STAMP IS A CERTIFICATE THIS RUN WROTE, so it does not survive this run
  # failing: the next run would otherwise skip the extraction on the strength of
  # a certificate written by a run that never finished.
  lane_certify "${DATA_STAMP}" "the dataset stamp"
  extracted="extracted"
fi
extract_ms=$(($(now_ms) - extract_start))

require_nonempty_file "${DATASET}" "the WatDiv dataset"
require_nonempty_file "${CENSUS}" "the WatDiv entity census (saved.txt)"

# The toolkit tar is small; re-extracting it every run costs nothing and removes a
# whole class of stale-state question.
rm -rf "${TESTSUITE}" "${ARENA}/model"
tar xf "${CACHE}/watdiv_v06.tar" -C "${ARENA}" --strip-components=1 \
  watdiv/testsuite watdiv/model ||
  die "could not extract the templates and data model from watdiv_v06.tar"

# The twenty BASIC templates are the top level of testsuite/. linear_incremental/
# and linear_mixed/ are separate scaling studies with their own README and are not
# among them; they are removed so nothing downstream can accidentally count them.
rm -rf "${TESTSUITE}/linear_incremental" "${TESTSUITE}/linear_mixed"

template_count=$(find "${TESTSUITE}" -maxdepth 1 -name '*.txt' | wc -l)
((template_count == 20)) ||
  die "expected 20 basic templates in ${TESTSUITE}, found ${template_count}"
[[ -f "${MODEL}" ]] || die "watdiv_v06.tar did not contain model/wsdbm-data-model.txt"

data_rows=$(wc -l <"${DATASET}")
data_bytes=$(wc -c <"${DATASET}")
((data_rows > 0)) ||
  die "the extracted dataset at ${DATASET} has ${data_bytes} bytes but not one triple;
  every row this lane prints would be a zero over an empty corpus"
echo "dataset:   ${data_rows} triples, ${data_bytes} bytes (${extracted} in ${extract_ms} ms)"
echo "templates: ${template_count} basic templates, prefix table from $(basename "${MODEL}")"

# ── 4. Instantiate the templates deterministically ──────────────────────────────

step "4/7 instantiate the 20 templates deterministically at seed ${SEED}"
python3 "${REPO_ROOT}/scripts/watdiv-queries.py" --self-test ||
  die "the query instantiator failed its own self-test"

rm -rf "${QUERIES}"
inst_start="$(now_ms)"
python3 "${REPO_ROOT}/scripts/watdiv-queries.py" \
  --templates "${TESTSUITE}" \
  --model "${MODEL}" \
  --dataset "${DATASET}" \
  --census "${CENSUS}" \
  --candidates "${ARENA}/candidates.tsv" \
  --seed "${SEED}" \
  --out "${QUERIES}" ||
  die "could not instantiate the WatDiv templates"
inst_ms=$(($(now_ms) - inst_start))

# Digest the emitted .rq files in name order. This is the reproducibility handle
# -- the same dataset and the same seed must reproduce it exactly -- and it is
# also the tripwire that makes a CONCURRENT RUN visible (see verify_query_set).
queries_digest() {
  python3 -c '
import hashlib, pathlib, sys
digest = hashlib.sha256()
for path in sorted(pathlib.Path(sys.argv[1]).glob("*.rq")):
    digest.update(path.name.encode("utf-8"))
    digest.update(path.read_bytes())
print(digest.hexdigest())
' "${QUERIES}"
}

# A DIGEST IS A CERTIFICATE, so it is never published for output that was not
# produced. `queries_digest` over an EMPTY directory is
# e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 -- the SHA-256
# of the empty string -- and printing that under "the reproducibility check"
# would certify a workload of no queries.
rq_count=$(find "${QUERIES}" -maxdepth 1 -name '*.rq' | wc -l)
((rq_count == 20)) ||
  die "the instantiator reported success but wrote ${rq_count} .rq file(s) to ${QUERIES}, not 20.
  No query-set digest is published for a set that is not the 20 published templates."
require_nonempty_file "${QUERIES}/queries.tsv" "the instantiated query index"

queries_sha="$(queries_digest)"

echo "instantiated in ${inst_ms} ms"
echo "provenance: ${QUERIES}/provenance.txt"
echo "sha256(queries @ seed ${SEED}) = ${queries_sha}"
echo "  ^ the reproducibility check: this dataset and this seed must reproduce it"
echo "    byte for byte. A DIFFERENT SEED IS A DIFFERENT WORKLOAD, not a re-run."

# ── 5. Load through the purrdf CLI ──────────────────────────────────────────────

step "5/7 load the dataset through the purrdf CLI into a native pack"

# Loading once and querying the pack is not an optimization dodge: it is what a
# store does. Handing 20 queries the raw N-Triples file would re-parse well over a
# gigabyte twenty times and measure the parser, not the planner. The pack is
# written by the same CLI under test, so nothing external is doing the loading.
PACK_KEY="${TARBALL_SHA} ${PURRDF_VERSION}"
load_start="$(now_ms)"
# `-s` rather than `-f`: a zero-byte pack beside a matching stamp would be reused
# by every subsequent run, and the stamp is written below only after the pack has
# been checked for being one. Same law as the dataset stamp above.
if [[ -f "${PACK_STAMP}" && "$(cat "${PACK_STAMP}")" == "${PACK_KEY}" && -s "${PACK}" ]]; then
  echo "pack: reusing the one already stamped with this dataset digest and this binary"
  loaded="reused"
else
  rm -f "${PACK}" "${PACK_STAMP}"
  convert_status=0
  "${BIN}" convert --from ntriples --to pack "${DATASET}" "${PACK}" || convert_status=$?
  # What is known here is the exit status. The binary was proved to run at step 2
  # and the dataset was proved non-empty at step 3, so a failure is about this
  # invocation; the message names it rather than asserting a cause.
  ((convert_status == 0)) ||
    die "the purrdf CLI exited ${convert_status} loading the WatDiv dataset into a pack.
  binary   ${BIN}
  input    ${DATASET}
  output   ${PACK}
  Whatever the CLI printed is above this line."
  # EXITING 0 IS NOT LOADING, AND NON-EMPTY IS NOT A PACK. The stamp is written
  # only once the pack has been checked for BEING a pack, so a stamp never
  # certifies an artifact that is not the one it names. Eight bytes of something
  # else passed the emptiness test, got stamped, survived the failure that
  # followed, and were reused by every later run as this dataset's pack.
  lane_require_magic "${PACK}" "the pack the CLI reported it had written" \
    "${PACK_MAGIC}" "a purrdf pack"
  write_checked "${PACK_STAMP}" "the pack stamp" printf '%s\n' "${PACK_KEY}"
  # And the stamp does not survive this run failing: every later run consults it
  # INSTEAD of loading, so a stamp left behind by a run that did not finish is a
  # certificate for a pack nothing ever finished certifying.
  lane_certify "${PACK_STAMP}" "the pack stamp"
  loaded="loaded"
fi
load_ms=$(($(now_ms) - load_start))
# Both branches end here, so BOTH are checked: a pack that was reused on the
# strength of a stamp is checked for being a pack exactly as a freshly written
# one is. A stamp written before this check existed is not evidence.
lane_require_magic "${PACK}" "the pack, after loading reported success," \
  "${PACK_MAGIC}" "a purrdf pack"
pack_bytes=$(wc -c <"${PACK}")
echo "pack: ${pack_bytes} bytes (${loaded} in ${load_ms} ms)"

# ── 6. Measure what a query's wall time includes before it does anything ────────

step "6/7 measure the fixed data-source open cost"

rows_of_json() {
  python3 -c '
import json
import sys

# Count the answer sequence exactly. Row counting by lines would be wrong the
# moment a literal contained a newline, and a benchmark that miscounts its
# answers is worse than one that does not run.
document = json.load(sys.stdin)
if "boolean" in document:
    print(1 if document["boolean"] else 0)
else:
    print(len(document["results"]["bindings"]))
'
}

# Evaluate one query file against the pack. Prints `status<TAB>rows<TAB>ms<TAB>detail`.
# It never exits non-zero, because a query that cannot run is a RESULT this lane
# reports rather than a reason to abandon the run and print nothing.
run_query() {
  local query_text="$1"
  local start stop out rc
  start="$(now_ms)"
  set +e
  out="$("${BIN}" query --data "${PACK}" --results-format json "${query_text}" 2>&1)"
  rc=$?
  set -e
  stop="$(now_ms)"

  if ((rc != 0)); then
    printf 'CANNOT-EXECUTE\t-\t%s\t%s\n' "$((stop - start))" \
      "$(printf '%s' "${out}" | head -1)"
    return
  fi

  local count
  count="$(printf '%s' "${out}" | rows_of_json 2>/dev/null)" || count=""
  if [[ -z "${count}" ]]; then
    printf 'BAD-RESULTS\t-\t%s\t%s\n' "$((stop - start))" \
      "the engine exited 0 but its results were not parseable SPARQL JSON"
    return
  fi
  printf 'OK\t%s\t%s\t-\n' "${count}" "$((stop - start))"
}

# A trivial pattern that binds on the first row. Whatever this costs is what EVERY
# query below pays before evaluating anything, so subtracting it is what turns a
# process wall time into something like a query cost.
probe_result="$(run_query 'SELECT ?s WHERE { ?s ?p ?o } LIMIT 1')"
probe_status="$(printf '%s' "${probe_result}" | cut -f1)"
[[ "${probe_status}" == "OK" ]] ||
  die "the open-cost probe could not run against ${PACK}: $(printf '%s' "${probe_result}" | cut -f4)"
OPEN_MS="$(printf '%s' "${probe_result}" | cut -f3)"
echo "opening the pack and answering a one-row pattern: ${OPEN_MS} ms"
echo "  ^ every TOTAL below includes this. EVAL is TOTAL minus it -- an estimate of"
echo "    evaluation cost, not a measurement of it."

# ── 7. Run the 20 queries ───────────────────────────────────────────────────────

step "7/7 run the 20 instantiated queries (pure BGP, no entailment)"

# TWO RUNS SHARING ONE WATDIV_OUT DESTROY EACH OTHER, and the damage is silent
# unless something looks for it: step 4 begins with `rm -rf "${QUERIES}"`, so a
# second run starting while this one is partway through its twenty queries
# deletes and rewrites the set underneath it. The rows already printed then
# belong to one query set and the rows still to come belong to another, which is
# the most expensive kind of wrong number -- it looks completely normal.
#
# The query set is therefore re-digested before the loop and again after it. A
# change means a concurrent run, and it is named as one rather than surfacing as
# a puzzling "some file is missing".
verify_query_set() {
  local when="$1" now
  now="$(queries_digest)"
  [[ "${now}" == "${queries_sha}" ]] && return 0
  die "the instantiated query set CHANGED ${when}.
  expected ${queries_sha}
  found    ${now}
  Another run is almost certainly using the same WATDIV_OUT ('${OUT}') and rewrote
  the queries underneath this one: step 4 starts by deleting that directory. Numbers
  from a run whose queries changed mid-flight are not numbers, so this run stops.
  Give each concurrent run its own arena, for example WATDIV_OUT=target/watdiv-\$\$."
}

verify_query_set "between instantiation and the first query"

shape_of() {
  case "${1:0:1}" in
    C) echo "complex" ;;
    F) echo "snowflake" ;;
    L) echo "linear" ;;
    S) echo "star" ;;
    *) echo "unknown" ;;
  esac
}

printf '%-4s %-10s %-9s %-15s %10s %9s %9s\n' \
  ID SHAPE MAPPINGS STATUS ROWS TOTAL_MS EVAL_MS
printf '%s\n' "--------------------------------------------------------------------------------"

total_ms=0
executed=0
unexecuted=0
nonempty=0
total_rows=0
declare -a NOTES=()
declare -a EMPTY=()

while IFS=$'\t' read -r id regime mappings file; do
  [[ "${id}" != "id" ]] || continue
  # A missing file is almost always a concurrent run having just deleted the
  # directory, so ask that question first: it gives the real diagnosis instead of
  # a filename that vanished for no stated reason.
  if [[ ! -f "${QUERIES}/${file}" ]]; then
    verify_query_set "while ${id} was about to run"
    die "${file} is missing from ${QUERIES}, yet the query set digest is unchanged"
  fi
  result="$(run_query "$(cat "${QUERIES}/${file}")")"
  status="$(printf '%s' "${result}" | cut -f1)"
  rows="$(printf '%s' "${result}" | cut -f2)"
  ms="$(printf '%s' "${result}" | cut -f3)"
  detail="$(printf '%s' "${result}" | cut -f4)"

  if [[ "${status}" == "OK" ]]; then
    eval_ms=$((ms - OPEN_MS))
    ((eval_ms >= 0)) || eval_ms=0
    executed=$((executed + 1))
    total_ms=$((total_ms + ms))
    total_rows=$((total_rows + rows))
    if ((rows > 0)); then
      nonempty=$((nonempty + 1))
    else
      EMPTY+=("${id}")
    fi
  else
    eval_ms="-"
    unexecuted=$((unexecuted + 1))
    NOTES+=("${id}: ${detail}")
  fi

  printf '%-4s %-10s %-9s %-15s %10s %9s %9s\n' \
    "${id}" "$(shape_of "${id}")" "${mappings}" "${status}" "${rows}" "${ms}" "${eval_ms}"
done <"${QUERIES}/queries.tsv"

# The twenty rows above are only one measurement if they were all answered over
# the same query set. Checking afterwards is what proves they were.
verify_query_set "while the twenty queries were running"

echo ""
if ((${#NOTES[@]} > 0)); then
  echo "QUERIES THAT DID NOT EXECUTE, and exactly why:"
  for note in "${NOTES[@]}"; do
    echo "  ${note}"
  done
  echo ""
fi

if ((${#EMPTY[@]} > 0)); then
  echo "QUERIES THAT EXECUTED AND MATCHED NOTHING: ${EMPTY[*]}"
  echo "  A zero here is a REAL ANSWER, not a failure, and WatDiv's deliberate skew is"
  echo "  what produces it: a property is concentrated on some entities and absent from"
  echo "  others, so a pattern that demands several at once can legitimately match none."
  echo "  For a query with a mapping, the uniform draw landed on a candidate the rest of"
  echo "  the pattern does not join with; which candidate, and out of how many, is in"
  echo "  ${QUERIES}/provenance.txt. For a query with NO mapping (the MAPPINGS column"
  echo "  reads 0) nothing was chosen at all: the published template is empty over this"
  echo "  dataset, which is a fact about the corpus rather than about this lane."
  echo ""
fi

# A lane in which nothing ran, or in which nothing matched anything, has measured
# nothing while looking exactly like a fast engine. Both are hard failures: they
# are the failure mode this whole workload exists to expose.
((executed > 0)) ||
  die "not one of the 20 queries executed; this lane measured nothing"
((nonempty > 0)) ||
  die "all ${executed} queries executed and every one matched zero rows.
  That is vacuous, not fast: 20 basic graph patterns over ${data_rows} triples cannot
  all legitimately be empty. Suspect the prefix table, the load, or the instantiation."

cat <<REPORT
SUMMARY
  dataset            WatDiv ${SCALE} frozen output, ${data_rows} triples, ${data_bytes} bytes
  dataset sha256     ${TARBALL_SHA}  (the pinned tarball)
  loaded             ${pack_bytes}-byte pack, ${loaded} in ${load_ms} ms
  seed               ${SEED}
  queries sha256     ${queries_sha}
  queries executed   ${executed} of 20 (${total_ms} ms total, ${total_rows} rows)
  matched nothing    ${#EMPTY[@]}
  not executed       ${unexecuted}
  open cost          ${OPEN_MS} ms, included in every TOTAL_MS above

HOW TO READ THIS
  Every row is a PURE BASIC GRAPH PATTERN answered with NO ENTAILMENT REGIME, over
  the frozen dataset named above, from the query set named by its digest above. All
  three of those belong to the number; none of them is a default.

  DO NOT COMPARE THESE ROWS AGAINST LUBM ROWS. WatDiv measures how a planner copes
  with query shape and selectivity and needs no inference at all; eleven of LUBM's
  fourteen queries have no answers whatsoever without it. Same engine, different
  question, different data, different regime.

  DO NOT COMPARE ACROSS SEEDS. The seed decides which candidate each %vN% mapping
  became, so two seeds are two workloads. Compare two engines on a query only when
  both answered the same instantiated query over the same dataset.

  TOTAL_MS is process wall time and includes the ${OPEN_MS} ms open cost measured in
  step 6. EVAL_MS is that subtraction -- an estimate, and at this scale a small
  difference between two large numbers, so treat it as an indication of where the
  work is rather than as a measurement.

  This lane is report-only. Nothing here is a gate and no number here is asserted.

CITE
  G. Aluc, O. Hartig, M. T. Ozsu and K. Daudjee. "Diversified Stress Testing of RDF
  Data Management Systems." In Proc. The Semantic Web - ISWC 2014 - 13th
  International Semantic Web Conference, 2014, pages 197-212.

  WatDiv is citation-ware: the grant to use it is conditioned on that citation, so
  every published result derived from this lane must carry it.
REPORT
