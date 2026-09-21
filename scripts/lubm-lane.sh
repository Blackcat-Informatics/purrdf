#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
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
# The UBA generator is GPL-2.0-or-later and this tree is MIT OR Apache-2.0 OR MulanPSL-2.0, so the
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

# The laws every lane in this repository shares — how it dies, how its scratch
# directory is cleaned up, how the certificates it wrote are revoked when it
# fails, and how the executable that certifies every number is validated — live
# in ONE implementation that all three lanes source. See scripts/lane-common.sh.
LANE="lubm-lane"
LANE_BINARY="purrdf binary"
# shellcheck source=scripts/lane-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/lane-common.sh"

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

# THIS LANE PERSISTS NO CERTIFICATE, and that is a property worth stating rather
# than a gap. `scale-corpus.sh` writes manifests and `watdiv-lane.sh` writes reuse
# stamps; both register them with `lane_certify` so the shared EXIT trap revokes
# them when the run fails. Every step below instead begins with `rm -rf` and
# rebuilds its own inputs, so nothing this lane writes is ever consulted by a
# LATER run and the registry stays empty. The trap is installed all the same, by
# the same `source` line the siblings use, so the day this lane does keep
# something across runs it is already covered by the same law.

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

# A WRITE WHOSE STATUS IS NOT CHECKED IS A SILENT DROP, and `lane_write_checked`
# is where every redirection in all three lanes goes. `$1` is the destination,
# `$2` the role, and the rest is the command whose stdout becomes the file; the
# third argument of the shared function names WHERE the bytes were going, and
# this lane spells it as the path plus its knob.
write_checked() {
  local destination="$1" role="$2"
  shift 2
  lane_write_checked "${destination}" "${role}" \
    "'${destination}' (under LUBM_OUT='${OUT}')" "$@"
}

# `LUBM_OUT` is a knob, so an arena that cannot be created is a LANE failure that
# quotes the bytes back — not a bare `mkdir: cannot create directory ...` with no
# hint which knob supplied the path.
mkdir_checked() {
  lane_mkdir_checked "$1" "LUBM_OUT='${OUT}'"
}

# A DATASET IS THE THING EVERY NUMBER BELOW IS ABOUT, so an empty one is a hard
# failure and never a row in the report. An empty dataset converts cleanly,
# answers every one of the 14 queries 0, and digests to the SHA-256 of the empty
# string. See the digest law at step 5, and `lane_require_nonempty_file` for why
# that digest is described in the diagnostic and never printed.
require_nonempty_file() {
  lane_require_nonempty_file "$1" "$2"
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

# `LUBM_BIN` NAMES THE EXECUTABLE EVERY NUMBER IN THIS REPORT IS ABOUT, so it is
# validated by the SAME implementation `SCALE_BIN` and `WATDIV_BIN` are:
# `lane_require_executable` for the path, `lane_run_probe` for whether it runs,
# `lane_require_probe_said_something` for whether it produced anything.
#
# `[[ -x ]]` ALONE IS NOT A CHECK FOR AN EXECUTABLE: a DIRECTORY carries the
# execute bit, so `LUBM_BIN=/tmp` passed it. And the execute bit is not proof the
# binary runs: `/bin/false` carries it too, reached step 5, and made the lane die
# with `purrdf convert failed on ... -- the CLI could not parse LUBM's RDF/XML`.
# That diagnostic was FALSE in every particular — the CLI never ran, the RDF/XML
# parses, and the fault was in the knob — and an operator following it would go
# hunting a parser bug that does not exist.
#
# AND RUNNING IS NOT CONVERTING. A CLI that answers `--version` and then writes an
# empty file for every conversion is the shape that reached step 5, converted all
# 15 files to nothing, passed its own `converted == owl_count` check 15 of 15,
# and published `sha256(lubm-data.nq)` — the SHA-256 of the empty string — as the
# provenance of a corpus that did not exist. So the probe here is a ROUND TRIP,
# exactly as `scale-corpus.sh` round-trips a manifest: the binary is handed a
# one-triple RDF/XML document and must hand back N-Quads. That is what makes the
# empty-conversion refusal reachable before step 1, with no JRE, no network and
# no generated corpus to mistake the fault for.
#
# `$1` is how to name the binary in a diagnostic; `$2` is 1 when `LUBM_BIN`
# supplied the path and 0 when this lane built it. Sets `PURRDF_VERSION`.
validate_purrdf_bin() {
  local provenance="$1" from_knob="$2"
  lane_require_executable LUBM_BIN "${from_knob}" "${BIN}" "the purrdf CLI"

  lane_run_probe "${provenance}" "for its version" "${BIN}" --version
  lane_require_probe_said_something "${provenance}" "for its version" \
    "A binary that says nothing is not the purrdf CLI, and this lane records what it
  printed beside every number in the report as the provenance of that number."
  PURRDF_VERSION="${LANE_PROBE_OUT}"

  # One triple, in the reserved documentation domain this repository's fixtures
  # use. It exercises the same `--from rdfxml --to nquads --base` path step 5
  # uses on LUBM's own files, and nothing about it depends on LUBM.
  local probe_in="${LANE_TMP}/probe.rdf" probe_out="${LANE_TMP}/probe.nq"
  cat >"${probe_in}" <<'PROBE_RDFXML'
<?xml version="1.0" encoding="utf-8"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://example.org/lane-probe#">
  <rdf:Description rdf:about="http://example.org/lane-probe/s">
    <ex:p rdf:resource="http://example.org/lane-probe/o"/>
  </rdf:Description>
</rdf:RDF>
PROBE_RDFXML
  rm -f "${probe_out}"
  lane_run_probe "${provenance}" \
    "to convert a one-triple RDF/XML document to N-Quads" \
    "${BIN}" convert --from rdfxml --to nquads \
    --base "http://example.org/lane-probe/probe.rdf" "${probe_in}" "${probe_out}"
  [[ -s "${probe_out}" ]] ||
    die "${provenance} exited 0 converting a one-triple RDF/XML document and produced
  NO N-QUADS AT ALL.
  A CLI that converts everything to nothing converts LUBM's files to nothing too,
  one empty file at a time, and every one of those conversions exits 0. The lane
  would then publish a digest as the provenance of a corpus that does not exist.
  It stops here instead, before a byte is fetched and before the generator runs."
  # And what it DID produce has to be N-Quads, by the same check the converted
  # dataset gets at step 5. One law, one implementation.
  lane_require_nquads "${probe_out}" \
    "the N-Quads ${provenance} produced from a one-triple document"
}

# VALIDATED HERE, BEFORE STEP 1. A knob error is not worth a download, a JRE, a
# Java generator run and eight megabytes of RDF/XML before it is noticed, and
# every one of those is a chance for the real fault to be mistaken for a problem
# with the corpus. An unset `LUBM_BIN` is validated at step 2 instead, once the
# build that produces the binary has run.
BIN_FROM_KNOB=0
if [[ -n "${BIN}" ]]; then
  BIN_FROM_KNOB=1
  validate_purrdf_bin "LUBM_BIN='${BIN}'" 1
fi

# AND SO IS THE ARENA. `LUBM_OUT` is a knob exactly as `LUBM_BIN` is, and a knob
# error is not worth a download either: an unusable arena discovered at step 3 has
# already cost a network fetch, and an operator reading the failure has to work
# out which of the two knobs it was about. The arena is created here instead, once,
# and every later `mkdir_checked` under it is then a subdirectory of a path already
# proved usable.
mkdir_checked "${ARENA_ROOT}"

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
if ((BIN_FROM_KNOB == 0)); then
  echo "building purrdf (release)..." >&2
  cargo build --locked --release -p purrdf-cli >&2 ||
    die "cargo build -p purrdf-cli failed"
  BIN="${REPO_ROOT}/target/release/purrdf"
  validate_purrdf_bin "the binary this lane built, '${BIN}'" 0
fi
echo "purrdf: ${BIN}  (${PURRDF_VERSION})"

# ── 3. Unpack the generator (RUN, never vendored) ───────────────────────────────

step "3/7 unpack the UBA generator into target/ (GPL-2.0-or-later: run, never copied into the tree)"
command -v java >/dev/null 2>&1 ||
  die "java is not on PATH; UBA ships prebuilt classes, so a JRE is enough"
command -v unzip >/dev/null 2>&1 || die "unzip is not on PATH"

UBA="${ARENA_ROOT}/uba"
rm -rf "${UBA}"
mkdir_checked "${UBA}"
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
mkdir_checked "${WORK}"

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
((owl_bytes > 0)) ||
  die "the generator wrote ${owl_count} .owl file(s) totalling zero bytes; there is
  no corpus to convert and nothing downstream would be measuring LUBM"
echo "generated ${owl_count} RDF/XML file(s), ${owl_bytes} bytes, in ${gen_ms} ms"
echo "renamed ${renamed} backslash-named file(s) out of the parent directory"

# ── 5. Convert RDF/XML -> N-Quads through purrdf itself ─────────────────────────

step "5/7 convert RDF/XML -> N-Quads through the purrdf CLI"
NQ="${ARENA_ROOT}/nq"
rm -rf "${NQ}"
mkdir_checked "${NQ}"

conv_start="$(now_ms)"
converted=0
while IFS= read -r owl; do
  name="$(basename "${owl}")"
  convert_status=0
  "${BIN}" convert --from rdfxml --to nquads --base "${DOC_BASE}${name}" \
    "${owl}" "${NQ}/${name}.nq" || convert_status=$?
  # WHAT IS KNOWN HERE IS THE EXIT STATUS, AND NOTHING ELSE. The previous
  # wording asserted a cause it had not established -- "the CLI could not parse
  # LUBM's RDF/XML" -- and with LUBM_BIN=/bin/false it was false three times
  # over: the CLI never ran, the RDF/XML parses, and the fault was in the knob.
  # The binary is proved to run at step 2 now, so a failure here is genuinely
  # about this invocation; the message says which one and what it said.
  ((convert_status == 0)) ||
    die "the purrdf CLI exited ${convert_status} converting RDF/XML -> N-Quads.
  binary   ${BIN}
  input    ${owl}
  output   ${NQ}/${name}.nq
  base     ${DOC_BASE}${name}
  Whatever the CLI printed is above this line. The generated RDF/XML is not
  assumed to be at fault: it was produced by the pinned UBA generator and nothing
  here has established a cause."
  # EXITING 0 IS NOT CONVERTING. A CLI that exits 0 and writes nothing (or an
  # empty file) used to reach the report as `data: 0 rows, 0 bytes` on a SUCCESS
  # line -- the exact shape this guard exists to stop, one file at a time so the
  # failure names the file rather than the whole dataset.
  require_nonempty_file "${NQ}/${name}.nq" "the N-Quads conversion of ${name}"
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
mapfile -t nq_files < <(find "${NQ}" -maxdepth 1 -name '*.nq' | sort)
((${#nq_files[@]} > 0)) ||
  die "no .nq files under ${NQ} after ${converted} conversion(s) reported success"
write_checked "${DATA}" "the concatenated LUBM dataset" cat "${nq_files[@]}"
require_nonempty_file "${DATA}" "the LUBM dataset"

if grep -q '^_:' "${DATA}"; then
  die "the generated data contains blank nodes; per-file conversion may have collided labels"
fi

ONTO_NQ="${ARENA_ROOT}/lubm-onto.nq"
onto_status=0
"${BIN}" convert --from rdfxml --to nquads --base "${ONTO}" \
  "${CACHE}/univ-bench.owl" "${ONTO_NQ}" || onto_status=$?
((onto_status == 0)) ||
  die "the purrdf CLI exited ${onto_status} converting the univ-bench ontology.
  binary   ${BIN}
  input    ${CACHE}/univ-bench.owl
  output   ${ONTO_NQ}"
require_nonempty_file "${ONTO_NQ}" "the converted univ-bench ontology"

data_rows=$(wc -l <"${DATA}")
data_bytes=$(wc -c <"${DATA}")
onto_rows=$(wc -l <"${ONTO_NQ}")

# A DIGEST IS A CERTIFICATE. It is emitted below as "the determinism check", and
# a capture of this lane records it as the dataset's provenance -- so it must
# never be emitted for output that is empty or that failed to be produced. The
# guards above make that structurally true; these two make it true of the numbers
# printed beside it, which are the other half of the same certificate.
((data_rows > 0)) ||
  die "the LUBM dataset at ${DATA} has ${data_bytes} bytes but not one N-Quads row.
  No digest is published for it: a zero-row dataset answers every one of the 14
  queries 0, instantly, and would report as a very fast engine."
((onto_rows > 0)) ||
  die "the converted univ-bench ontology at ${ONTO_NQ} has no rows; eleven of the
  14 queries have answers only under a regime that needs it."

# NON-EMPTY IS NOT "IS WHAT IT CLAIMS TO BE", and the digest below is published
# for THIS file. The check is the same one the binary's own probe passed before
# step 1, by the same implementation.
lane_require_nquads "${DATA}" "the LUBM dataset"
lane_require_nquads "${ONTO_NQ}" "the converted univ-bench ontology"

# CONVERTING IS NOT THE SAME AS READING THE INPUT, and a row count alone cannot
# tell them apart: a CLI that truncated every conversion turned 8,280,209 bytes of
# RDF/XML into `converted 15 rows, 1935 bytes` and put that on a SUCCESS line,
# under a published digest, with all 14 queries reporting CANNOT-EXECUTE and an
# exit status of 0.
#
# The floor is deliberately generous, because it has to be right rather than
# tight: N-Quads repeats every IRI in full while RDF/XML abbreviates with
# namespace prefixes, so the conversion of a LUBM corpus is LARGER than its input
# — 17,435,822 bytes out of 8,280,209 in, a ratio of 2.1, for LUBM(1, 0). One
# eighth of the input is therefore some sixteen times below anything a real
# conversion produces, and no legitimate run comes near it.
((data_bytes * 8 >= owl_bytes)) ||
  die "the conversion produced ${data_bytes} bytes of N-Quads from ${owl_bytes} bytes of
  RDF/XML. N-Quads is LARGER than the RDF/XML it came from (a real LUBM conversion
  runs about twice the input), so this is not a conversion of that corpus — it is a
  fraction of one. No digest is published for it and no query is run against it."

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
FIRST_NQ="${nq_files[0]}"
# `${nq_files}` is non-empty by the guard at step 5, but the rungs are DATASETS
# that queries are answered over and reported against, so each one is checked
# for being one. The unchecked `cat "${FIRST_NQ}" ...` here is where the
# empty-corpus run finally died, at 7/7, on a bare `cat: '': No such file or
# directory` that named neither the lane, the knob, nor purrdf -- six steps and
# one published digest after the dataset was already known to be empty.
require_nonempty_file "${FIRST_NQ}" "the first converted LUBM file"

write_checked "${ENTAIL_FULL}" "the 'full' rung of the entailment ladder" \
  cat "${DATA}" "${ONTO_NQ}"
write_checked "${ENTAIL_FILE}" "the 'one-file' rung of the entailment ladder" \
  cat "${FIRST_NQ}" "${ONTO_NQ}"
slice_rung() {
  head -n "${ENTAIL_SLICE}" "${FIRST_NQ}"
  cat "${ONTO_NQ}"
}
write_checked "${ENTAIL_SLICE_NQ}" "the 'slice' rung of the entailment ladder" slice_rung
for rung_file in "${ENTAIL_FULL}" "${ENTAIL_FILE}" "${ENTAIL_SLICE_NQ}"; do
  require_nonempty_file "${rung_file}" "a rung of the entailment ladder"
done

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
probe_query="${LANE_TMP}/probe.rq"
write_checked "${probe_query}" "the regime probe query" \
  printf 'SELECT ?s WHERE { ?s ?p ?o } LIMIT 1\n'

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
nonempty=0
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
    ((rows == 0)) || nonempty=$((nonempty + 1))
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

# A LANE IN WHICH NOTHING RAN HAS MEASURED NOTHING WHILE LOOKING EXACTLY LIKE A
# FAST ENGINE, and this lane used to exit 0 from precisely that state: fourteen
# CANNOT-EXECUTE rows, a published digest above them, and a SUCCESS line for a
# dataset that was fifteen rows long. `watdiv-lane.sh` has had this guard since it
# was written; a law that holds in one lane and not its siblings is not a law.
((executed > 0)) ||
  die "not one of the 14 queries executed; this lane measured nothing.
  Every row above says why it could not run. A report whose every row is
  CANNOT-EXECUTE is a failed run, not a fast one."
# LUBM's own answers are the other half. Q1 and Q14 are answered with NO
# entailment over the full dataset, and both have matching individuals in any
# real LUBM corpus at any scale, so a run in which every executed query matched
# nothing is a run over something that is not LUBM. (A zero on an individual
# query is a real answer and always reported as one: Q2 is legitimately 0, and a
# rung below 'full' legitimately answers 0 for individuals outside its subset.)
((nonempty > 0)) ||
  die "all ${executed} queries executed and every one matched zero rows.
  That is vacuous, not fast: Q1 and Q14 are answered without entailment over the
  full ${data_rows}-row dataset and have matching individuals in any real LUBM
  corpus. Suspect the conversion, the dataset, or the query normalisation."

cat <<REPORT
SUMMARY
  dataset            LUBM(${UNIVERSITIES}, ${INDEX}) seed=${SEED}
  generated          ${owl_count} RDF/XML file(s), ${owl_bytes} bytes, ${gen_ms} ms
  converted          ${data_rows} rows, ${data_bytes} bytes, ${conv_ms} ms
  sha256             ${data_sha}
  queries executed   ${executed} of 14 (${query_total_ms} ms total)
  matched nothing    $((executed - nonempty))
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
