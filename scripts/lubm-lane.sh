#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Drive the LUBM comparison workload end to end: acquire, generate, convert, query.
#
# LUBM is the workload the OWL knowledge-base literature actually compares against,
# so PurRDF needs to be runnable on it by anyone who wants to check a claim. This
# script is that lane. It is REPORT-ONLY and never a gate: it prints what it
# measured and exits 0, or it fails loudly and exits non-zero. No number it prints
# is asserted anywhere (docs/BENCHMARKS.md, "measured, never asserted").
#
# NOTHING IS VENDORED
# ===================
#
# The UBA generator is GPL-2.0-or-later and this tree is MIT OR Apache-2.0, so the
# generator is RUN, NEVER COPIED IN. The ontology and the 14 queries carry no
# redistribution grant at all. All of it is fetched by digest into an ignored cache
# under `target/` by scripts/benchmark-acquire.py, used from there, and left there.
# Every file this lane writes is build output under `target/`.
#
# THE GENERATOR WRITES TO THE WRONG DIRECTORY ON LINUX, AND IS NOT PATCHED
# =======================================================================
#
# Stock UBA builds its output path as `user.dir + "\" + name` -- a WINDOWS
# separator. On Linux nothing splits that backslash, so the last `/` in the string
# is the one before the working directory's own name: the files land in the PARENT
# of the working directory, named `<cwdbasename>\University0_0.owl`, with a literal
# backslash inside the filename.
#
# The FILE CONTENTS ARE VALID RDF/XML. Only the name is wrong. So this lane RENAMES
# THE OUTPUT AFTER GENERATION rather than patching Generator.java, for three
# reasons that all point the same way:
#
#   * it leaves the GPL-2.0 source unmodified and un-vendored, which is the only
#     licensing posture this tree can take;
#   * patching would require a Java compiler, and the artifact ships prebuilt
#     `classes/` precisely so that a JRE is enough;
#   * a rename is checkable -- the lane counts what it renamed and fails if the
#     count is not what the generator said it wrote.
#
# Each run generates inside its own directory (`<arena>/work`), so the misplaced
# files land in that run's own arena and concurrent runs cannot collide.
#
# DETERMINISM, AND THE ONE PLACE IT WAS NOT FREE
# ==============================================
#
# UBA takes `-index` and `-seed`, and the same pair reproduces the datasets used in
# the LUBM papers. The conversion needed one fix to inherit that: every generated
# file opens with `<owl:Ontology rdf:about="">`, an EMPTY RELATIVE IRI, which
# resolves against the document base. Left to default, that base is the input file's
# own `file://` path -- so the converted N-Quads embedded the scratch directory and
# two runs in differently-named directories differed. `--base` is therefore passed
# explicitly, built from the file's NAME only. It affects exactly two triples per
# file (the document's own `rdf:type owl:Ontology` and its `owl:imports`), and none
# of the 14 queries touches either.
#
# ENTAILMENT REGIMES ARE NOT OPTIONAL METADATA
# ============================================
#
# Eleven of the 14 queries have answers only under an entailment regime; LUBM never
# asserts `Student`, `Professor` or `Chair`. A no-inference engine answers those
# queries 0, instantly, and would "win" any comparison that ignored the regime. So
# every row this lane prints carries the regime it was answered under and the
# dataset it was answered over, and the report states the comparison rule outright.
#
# Knobs, all overridable exactly like the scale-corpus lane's `SCALE_*`:
#
#   LUBM_UNIVERSITIES  how many universities to generate (default 1)
#   LUBM_SEED          UBA's `-seed` (default 0)
#   LUBM_INDEX         UBA's `-index`, the starting university id (default 0)
#   LUBM_ONTO          the `-onto` IRI stamped into the data (default: Lehigh's)
#   LUBM_DOC_BASE      base for each data document's own two header triples
#   LUBM_ENTAIL_SLICE  triples in the smallest entailment rung (default 3000)
#   LUBM_OUT           where the lane works (default target/lubm)
#   LUBM_BIN           a prebuilt `purrdf` to use instead of building one

set -euo pipefail

UNIVERSITIES="${LUBM_UNIVERSITIES:-1}"
SEED="${LUBM_SEED:-0}"
INDEX="${LUBM_INDEX:-0}"
ONTO="${LUBM_ONTO:-http://swat.cse.lehigh.edu/onto/univ-bench.owl}"
# example.org is RFC 2606's reserved documentation domain and this repository's own
# fixture convention. It is a placeholder for the corpus's publication IRI, which a
# locally generated corpus does not have -- not a claim that anything is published
# there. An operator who publishes a corpus sets this to where it actually lives.
DOC_BASE="${LUBM_DOC_BASE:-http://example.org/lubm/}"
ENTAIL_SLICE="${LUBM_ENTAIL_SLICE:-3000}"
OUT="${LUBM_OUT:-target/lubm}"
BIN="${LUBM_BIN:-}"

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CACHE="${REPO_ROOT}/target/bench-artifacts"

# `LUBM_OUT` IS HONOURED AS WRITTEN. An absolute path is the arena, verbatim; a
# relative one is resolved against the repository root, which is what the
# `target/lubm` default has always meant and what keeps the lane independent of
# the caller's working directory.
#
# Prefixing `REPO_ROOT` unconditionally — which is what this did — quietly turned
# `LUBM_OUT=/mnt/big/arena` into `<repo>/mnt/big/arena`: nothing appeared where
# the operator asked, the arena landed INSIDE the working tree and outside
# `target/`, and the lane reported success. `docs/BENCHMARKS.md` documents an
# absolute `SCALE_OUT` and `SCALE_OUT` already behaves this way, so the trap was
# one the documentation trained an operator straight into.
case "${OUT}" in
  /*) ARENA_ROOT="${OUT}" ;;
  *) ARENA_ROOT="${REPO_ROOT}/${OUT}" ;;
esac

die() {
  echo "lubm-lane: $*" >&2
  exit 1
}

step() {
  echo ""
  echo "=== $* ==="
}

require_uint() {
  local name="$1" value="$2"
  [[ "${value}" =~ ^[0-9]+$ ]] ||
    die "${name} must be a decimal unsigned integer (got '${value}')"
}

require_positive() {
  require_uint "$1" "$2"
  [[ "$2" != "0" ]] || die "$1 must be positive (got '$2')"
}

require_positive LUBM_UNIVERSITIES "${UNIVERSITIES}"
require_positive LUBM_ENTAIL_SLICE "${ENTAIL_SLICE}"
require_uint LUBM_SEED "${SEED}"
require_uint LUBM_INDEX "${INDEX}"
[[ "${DOC_BASE}" == *://* ]] ||
  die "LUBM_DOC_BASE must be an absolute IRI (got '${DOC_BASE}')"
[[ "${ONTO}" == *://* ]] || die "LUBM_ONTO must be an absolute IRI (got '${ONTO}')"

# Milliseconds since the epoch. `bc` is not assumed present, so every duration is
# integer arithmetic over nanoseconds.
now_ms() {
  echo $(($(date +%s%N) / 1000000))
}

# ── 1. Artifacts ────────────────────────────────────────────────────────────────

step "1/7 artifacts (pinned, fetched by digest, never vendored)"
python3 "${REPO_ROOT}/scripts/benchmark-acquire.py" \
  --only uba1.7.zip GeneratorLinuxFix.zip queries-sparql.txt univ-bench.owl ||
  die "artifact acquisition failed -- nothing downstream can be trusted, stopping"

for required in uba1.7.zip univ-bench.owl queries-sparql.txt; do
  [[ -f "${CACHE}/${required}" ]] ||
    die "${required} is missing from ${CACHE} after acquisition reported success"
done

# ── 2. The purrdf binary ────────────────────────────────────────────────────────

step "2/7 purrdf CLI"
if [[ -z "${BIN}" ]]; then
  echo "building purrdf (release)..." >&2
  cargo build --locked --release -p purrdf-cli >&2 ||
    die "cargo build -p purrdf-cli failed"
  BIN="${REPO_ROOT}/target/release/purrdf"
fi
[[ -x "${BIN}" ]] || die "no executable purrdf binary at '${BIN}' (set LUBM_BIN)"
echo "purrdf: ${BIN}"

# ── 3. Unpack the generator (RUN, never vendored) ───────────────────────────────

step "3/7 unpack the UBA generator into target/ (GPL-2.0-or-later: run, never copied into the tree)"
command -v java >/dev/null 2>&1 ||
  die "java is not on PATH; UBA ships prebuilt classes, so a JRE is enough"
command -v unzip >/dev/null 2>&1 || die "unzip is not on PATH"

UBA="${ARENA_ROOT}/uba"
rm -rf "${UBA}"
mkdir -p "${UBA}"
unzip -q -o "${CACHE}/uba1.7.zip" -d "${UBA}" || die "could not unpack uba1.7.zip"
GENERATOR_CLASS="${UBA}/classes/edu/lehigh/swat/bench/uba/Generator.class"
[[ -f "${GENERATOR_CLASS}" ]] ||
  die "uba1.7.zip did not contain prebuilt classes/ -- this lane does not compile Java"
echo "generator classes: ${UBA}/classes"

# ── 4. Generate ─────────────────────────────────────────────────────────────────

step "4/7 generate LUBM(${UNIVERSITIES}, ${INDEX}) seed=${SEED}"
ARENA="${ARENA_ROOT}/gen"
rm -rf "${ARENA}"
WORK="${ARENA}/work"
mkdir -p "${WORK}"

gen_start="$(now_ms)"
(
  cd "${WORK}"
  java -cp "${UBA}/classes" edu.lehigh.swat.bench.uba.Generator \
    -univ "${UNIVERSITIES}" -index "${INDEX}" -seed "${SEED}" -onto "${ONTO}"
) >"${ARENA}/generator.stdout" 2>&1 ||
  die "the UBA generator failed; its output is in ${ARENA}/generator.stdout"
gen_ms=$(($(now_ms) - gen_start))

# The Windows-separator pathology: the files are in ARENA, named `work\NAME`.
# Move each one to the name it was trying to have. The count is checked against
# what actually appeared, and a run that produced nothing is a hard failure rather
# than an empty dataset that converts cleanly and answers every query 0.
renamed=0
shopt -s nullglob
for stray in "${ARENA}/work\\"*; do
  base="${stray##*work\\}"
  mv -- "${stray}" "${WORK}/${base}"
  renamed=$((renamed + 1))
done
shopt -u nullglob

((renamed > 0)) ||
  die "the generator wrote no files under ${ARENA} -- the Linux path pathology may have changed shape"

owl_count=$(find "${WORK}" -maxdepth 1 -name '*.owl' | wc -l)
((owl_count > 0)) || die "no .owl files after renaming ${renamed} stray file(s)"
owl_bytes=$(find "${WORK}" -maxdepth 1 -name '*.owl' -printf '%s\n' | awk '{t+=$1} END {print t+0}')
echo "generated ${owl_count} RDF/XML file(s), ${owl_bytes} bytes, in ${gen_ms} ms"
echo "renamed ${renamed} backslash-named file(s) out of the parent directory"

# ── 5. Convert RDF/XML -> N-Quads through purrdf itself ─────────────────────────

step "5/7 convert RDF/XML -> N-Quads through the purrdf CLI"
NQ="${ARENA_ROOT}/nq"
rm -rf "${NQ}"
mkdir -p "${NQ}"

conv_start="$(now_ms)"
converted=0
while IFS= read -r owl; do
  name="$(basename "${owl}")"
  "${BIN}" convert --from rdfxml --to nquads --base "${DOC_BASE}${name}" \
    "${owl}" "${NQ}/${name}.nq" ||
    die "purrdf convert failed on ${owl} -- the CLI could not parse LUBM's RDF/XML"
  converted=$((converted + 1))
done < <(find "${WORK}" -maxdepth 1 -name '*.owl' | sort)
conv_ms=$(($(now_ms) - conv_start))

((converted == owl_count)) ||
  die "converted ${converted} of ${owl_count} files; refusing to report a partial dataset"

DATA="${ARENA_ROOT}/lubm-data.nq"
# `sort` fixes the concatenation order so the dataset is byte-reproducible. LUBM's
# instance data contains no blank nodes, so concatenating separately converted
# files cannot collide labels -- a property this lane checks below rather than
# assumes.
find "${NQ}" -maxdepth 1 -name '*.nq' | sort | xargs cat >"${DATA}"

if grep -q '^_:' "${DATA}"; then
  die "the generated data contains blank nodes; per-file conversion may have collided labels"
fi

ONTO_NQ="${ARENA_ROOT}/lubm-onto.nq"
"${BIN}" convert --from rdfxml --to nquads --base "${ONTO}" \
  "${CACHE}/univ-bench.owl" "${ONTO_NQ}" ||
  die "purrdf convert failed on the univ-bench ontology"

data_rows=$(wc -l <"${DATA}")
data_bytes=$(wc -c <"${DATA}")
onto_rows=$(wc -l <"${ONTO_NQ}")
data_sha=$(python3 -c '
import hashlib, sys
print(hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest())
' "${DATA}")

echo "data:     ${data_rows} rows, ${data_bytes} bytes, converted in ${conv_ms} ms"
echo "ontology: ${onto_rows} rows"
echo "sha256(lubm-data.nq) = ${data_sha}"
echo "  ^ this digest is the determinism check: the same LUBM_UNIVERSITIES/INDEX/SEED"
echo "    and the same LUBM_DOC_BASE must reproduce it byte for byte."

# ── 6. Normalise the queries ────────────────────────────────────────────────────

step "6/7 normalise the 14 published queries (mechanical rules, recorded)"
QUERIES="${ARENA_ROOT}/queries"
rm -rf "${QUERIES}"
NAMESPACE="${ONTO}#"
python3 "${REPO_ROOT}/scripts/lubm-queries.py" --self-test ||
  die "the query normaliser failed its own self-test"
python3 "${REPO_ROOT}/scripts/lubm-queries.py" --namespace "${NAMESPACE}" --out "${QUERIES}" ||
  die "could not normalise the LUBM queries"
echo "provenance: ${QUERIES}/provenance.txt"

# ── 7. Run the queries ──────────────────────────────────────────────────────────

step "7/7 run the queries, per regime"

# The entailment ladder. Materializing a closure has a FIXED internal ceiling that
# no flag raises, so a regime that cannot close over the whole dataset is offered
# progressively smaller rungs rather than being reported as unsupported. Each rung
# is a real dataset and every reported row count names the rung it came from, so a
# smaller rung never silently masquerades as a full-scale answer.
ENTAIL_FULL="${ARENA_ROOT}/entail-full.nq"
ENTAIL_FILE="${ARENA_ROOT}/entail-file.nq"
ENTAIL_SLICE_NQ="${ARENA_ROOT}/entail-slice.nq"
FIRST_NQ="$(find "${NQ}" -maxdepth 1 -name '*.nq' | sort | head -1)"

cat "${DATA}" "${ONTO_NQ}" >"${ENTAIL_FULL}"
cat "${FIRST_NQ}" "${ONTO_NQ}" >"${ENTAIL_FILE}"
{ head -n "${ENTAIL_SLICE}" "${FIRST_NQ}"; cat "${ONTO_NQ}"; } >"${ENTAIL_SLICE_NQ}"

rows_of() { wc -l <"$1"; }

# Evaluate one query file against one dataset under one regime. Prints
# `status<TAB>rows<TAB>ms<TAB>detail`; never exits non-zero, because a query that
# cannot run is a RESULT this lane reports, not a reason to abandon the run.
run_query() {
  local query_file="$1" dataset="$2" regime="$3"
  local -a flags=(--data "${dataset}" --results-format json)
  [[ "${regime}" == "-" ]] || flags+=(--entailment "${regime}")

  local start stop out rc
  start="$(now_ms)"
  set +e
  out="$("${BIN}" query "${flags[@]}" "$(cat "${query_file}")" 2>&1)"
  rc=$?
  set -e
  stop="$(now_ms)"

  if ((rc != 0)); then
    printf 'CANNOT-EXECUTE\t-\t%s\t%s\n' "$((stop - start))" \
      "$(printf '%s' "${out}" | head -1)"
    return
  fi

  local count
  count="$(printf '%s' "${out}" | python3 -c '
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
' 2>/dev/null)" || count=""

  if [[ -z "${count}" ]]; then
    printf 'BAD-RESULTS\t-\t%s\t%s\n' "$((stop - start))" \
      "the engine exited 0 but its results were not parseable SPARQL JSON"
    return
  fi
  printf 'OK\t%s\t%s\t-\n' "${count}" "$((stop - start))"
}

# Probe a regime ONCE per rung. The closure is a property of the dataset and the
# regime, not of the query, so probing per query would repeat an identical failure
# fourteen times and multiply the run time by the number of queries that share a
# regime.
probe_query="$(mktemp)"
trap 'rm -f "${probe_query}"' EXIT
printf 'SELECT ?s WHERE { ?s ?p ?o } LIMIT 1\n' >"${probe_query}"

declare -A REGIME_DATASET REGIME_RUNG REGIME_NOTE

resolve_regime() {
  local regime="$1"
  if [[ "${regime}" == "-" ]]; then
    REGIME_DATASET["${regime}"]="${DATA}"
    REGIME_RUNG["${regime}"]="full"
    REGIME_NOTE["${regime}"]="no closure needed"
    return
  fi
  local rung name dataset result status
  for rung in "full:${ENTAIL_FULL}" "one-file:${ENTAIL_FILE}" "slice:${ENTAIL_SLICE_NQ}"; do
    name="${rung%%:*}"
    dataset="${rung#*:}"
    result="$(run_query "${probe_query}" "${dataset}" "${regime}")"
    status="${result%%$'\t'*}"
    if [[ "${status}" == "OK" ]]; then
      REGIME_DATASET["${regime}"]="${dataset}"
      REGIME_RUNG["${regime}"]="${name}"
      REGIME_NOTE["${regime}"]="closure completed"
      printf '  regime %-8s rung %-9s %s rows -- CLOSURE OK\n' \
        "${regime}" "${name}" "$(rows_of "${dataset}")"
      return
    fi
    printf '  regime %-8s rung %-9s %s rows -- %s\n' \
      "${regime}" "${name}" "$(rows_of "${dataset}")" \
      "$(printf '%s' "${result}" | cut -f4)"
    REGIME_NOTE["${regime}"]="$(printf '%s' "${result}" | cut -f4)"
  done
  REGIME_DATASET["${regime}"]=""
  REGIME_RUNG["${regime}"]="none"
}

echo "probing each regime's closure against the dataset ladder:"
for regime in "-" rdfs owl-rl; do
  resolve_regime "${regime}"
done

echo ""
printf '%-4s %-36s %-8s %-9s %-15s %8s %8s\n' \
  ID REGIME CLI RUNG STATUS ROWS MS
printf '%s\n' "----------------------------------------------------------------------------------------------------"

query_total_ms=0
executed=0
unexecuted=0
declare -a NOTES=()

while IFS=$'\t' read -r id regime cli file; do
  [[ "${id}" != "id" ]] || continue
  dataset="${REGIME_DATASET[${cli}]:-}"
  rung="${REGIME_RUNG[${cli}]:-none}"
  if [[ -z "${dataset}" ]]; then
    printf '%-4s %-36s %-8s %-9s %-15s %8s %8s\n' \
      "${id}" "${regime}" "${cli}" "-" "CANNOT-EXECUTE" "-" "-"
    NOTES+=("${id}: no rung of the ladder could close ${cli} -- ${REGIME_NOTE[${cli}]}")
    unexecuted=$((unexecuted + 1))
    continue
  fi
  result="$(run_query "${QUERIES}/${file}" "${dataset}" "${cli}")"
  status="$(printf '%s' "${result}" | cut -f1)"
  rows="$(printf '%s' "${result}" | cut -f2)"
  ms="$(printf '%s' "${result}" | cut -f3)"
  detail="$(printf '%s' "${result}" | cut -f4)"
  printf '%-4s %-36s %-8s %-9s %-15s %8s %8s\n' \
    "${id}" "${regime}" "${cli}" "${rung}" "${status}" "${rows}" "${ms}"
  if [[ "${status}" == "OK" ]]; then
    executed=$((executed + 1))
    query_total_ms=$((query_total_ms + ms))
  else
    unexecuted=$((unexecuted + 1))
    NOTES+=("${id}: ${detail}")
  fi
done <"${QUERIES}/regimes.tsv"

echo ""
if ((${#NOTES[@]} > 0)); then
  echo "QUERIES THAT DID NOT EXECUTE, and exactly why:"
  for note in "${NOTES[@]}"; do
    echo "  ${note}"
  done
  echo ""
fi

cat <<REPORT
SUMMARY
  dataset            LUBM(${UNIVERSITIES}, ${INDEX}) seed=${SEED}
  generated          ${owl_count} RDF/XML file(s), ${owl_bytes} bytes, ${gen_ms} ms
  converted          ${data_rows} rows, ${data_bytes} bytes, ${conv_ms} ms
  sha256             ${data_sha}
  queries executed   ${executed} of 14 (${query_total_ms} ms total)
  not executed       ${unexecuted}

HOW TO READ THIS
  Every row names the REGIME it was answered under and the RUNG of the dataset
  ladder it was answered over. Compare two engines on a query ONLY when both
  answered it under the same regime and over the same data. LUBM never asserts
  Student, Professor or Chair, so an engine that applies no inference answers
  eleven of these queries 0 -- instantly, and wrongly.

  A ROW COUNT ON A RUNG BELOW 'full' IS NOT THE PUBLISHED LUBM ANSWER. It is the
  answer over that rung, which is a strict subset of the corpus, so a query whose
  matching individuals fall outside the subset legitimately reports 0. Those counts
  say the regime WORKS and what it costs; they are not comparable against a number
  published for LUBM(${UNIVERSITIES}, ${INDEX}). Only 'full' rows are.

  A rung below 'full' appears when materializing that regime's closure passes a
  FIXED internal ceiling that no command-line flag raises. The probe lines above
  print the observed and permitted counts verbatim, so the limit is visible rather
  than inferred from a missing row.

  This lane is report-only. Nothing here is a gate and no number here is asserted.

CITE
  Y. Guo, Z. Pan and J. Heflin, "LUBM: A Benchmark for OWL Knowledge Base
  Systems", Journal of Web Semantics 3(2).
REPORT
