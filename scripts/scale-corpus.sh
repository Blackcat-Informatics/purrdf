#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#
# Drive `bench-corpus` (the `purrdf-scale-mixed-v1` profile) across shards.
#
# The generator mints every IRI purely from its index, so shard `k` of `n`
# needs no coordination with shard `j`: the shards are independent processes
# over disjoint slices of one row sequence, and their ordered concatenation is
# byte-identical to a single whole run. That is a property of the ALGORITHM,
# and this driver is what turns it into a lane an operator can actually run.
#
# STREAMING IS THE DEFAULT, and the default is not a preference — it is the
# storage arithmetic. Density rises with the entity index space (~177 bytes
# per row at a 10^6-entity space, ~183 at 10^10), so 10^10 rows over a
# matching 10^10-entity space is ~1.83 TB of N-Quads, not the flat 1.77 TB a
# smaller-scale density would suggest. A run of that size has to be consumed
# as it is produced; writing it down is an operator decision about a
# filesystem that can hold it, never something a lane does implicitly.
# `SCALE_MODE=files` is therefore opt-in, and no continuous-integration
# runner should ever select it.
#
# Modes (`SCALE_MODE`):
#
#   stream  (default) every shard runs concurrently, piped into a sink that
#           retains nothing. The built-in sink reports rows, bytes, density and
#           a SHA-256 per shard; `SCALE_SINK` replaces it with any command that
#           reads standard input (an ingest, a loader, `wc -c`). The shard
#           count IS the concurrency here: choose it for the cores and the
#           consumers actually available, not for a round number.
#   pipe    the shards run in shard order and their bytes go to standard
#           output, unmixed and in order. Those bytes ARE a
#           whole run: `SCALE_MODE=pipe ... | your-loader` loads the same
#           corpus a single unsharded run would have produced. Ordered output
#           cannot outrun its consumer, so this mode trades the concurrency for
#           the ordering; `stream` is the one that uses the cores.
#
#           PIPE IT THIS WAY, from the repository root:
#
#             SCALE_MODE=pipe bash scripts/scale-corpus.sh | your-loader
#
#           i.e. run THIS SCRIPT, not `make`. Every knob is read from the
#           environment, so the form above is the `make scale-corpus` lane with
#           nothing between the payload and the consumer.
#
#           TRAP, and the reason the documented idiom skips `make`: GNU make
#           turns `-w`/`--print-directory` on BY ITSELF for any invocation it
#           detects as a sub-make, which it decides from a nonzero inherited
#           `MAKELEVEL` -- NOT from `MAKEFLAGS`. A nested `$(MAKE)` shows an
#           EMPTY `MAKEFLAGS` and `MAKELEVEL=1`, and the banner appears anyway,
#           so an operator who inspects `MAKEFLAGS`, finds it clean and
#           concludes the pipe is safe has checked the wrong variable. `make`
#           then writes `make[1]: Entering directory '...'` to standard output
#           before this script's payload does, corrupting the loader's first
#           line. THIS REPOSITORY'S `Makefile` SETS `MAKEFLAGS +=
#           --no-print-directory` at its top, which cancels that -- but only on
#           GNU make 4.4 or newer. GNU make 4.3 and earlier decide `-w` at
#           startup, before any makefile text is read, so the assignment cannot
#           help and no makefile-internal fix exists there. Running this script
#           directly sidesteps the whole question on every version; if you must
#           go through `make`, pass the flag yourself:
#           `$(MAKE) --no-print-directory scale-corpus SCALE_MODE=pipe | your-loader`.
#           `docs/BENCHMARKS.md` states the same rule at length.
#   files   opt-in materialization. Each shard is written to its own
#           zero-padded, lexicographically sortable file, so `cat` of the
#           sorted `*.nq` files reproduces a whole run byte for byte.
#
# Every mode emits the manifest — the profile id, the parameters and both
# mixes. A capture of this lane records the manifest beside the output digest;
# neither half is evidence without the other.
#
# EVERY MODE ALSO COUNTS WHAT ACTUALLY LEFT THE LANE, and does so on the SAME
# arithmetic: rows, bytes and whether the last byte was the newline that ends an
# N-Quads row. The manifest is a CLAIM about a corpus; the counts are the
# evidence, and the run fails unless they agree with it. That accounting is not
# free in `pipe` and in `SCALE_SINK` mode — the payload is copied through a
# counter on its way out, in 64 KiB chunks, which is one `memchr` and one `write`
# per chunk on top of generating the row in the first place — and it is the price
# of a manifest that means something. Without it, a binary emitting a valid
# manifest and then zero corpus bytes exited 0 through the pipe, published
# `"quads": 2000, "emitted_lines": 2000`, and delivered nothing at all; a
# truncated final row exited 0 with 116 bytes of unterminated N-Quads.
#
# Any shard that fails fails the whole run, loudly. `bench-corpus` exits 2 on a
# rejected specification and 1 on a write failure, and a driver that let either
# pass would report a corpus that was never generated. Each shard body checks
# every command's status ITSELF instead of trusting `errexit`, which a
# backgrounded subshell does not honour inside the `|| die` context this driver
# runs them from — see the note above `stream_shard`.

set -euo pipefail

# The laws every lane in this repository shares — how it dies, how its scratch
# directory is cleaned up, how the certificates it wrote are revoked when it
# fails, and how the executable that certifies every number is validated — live
# in ONE implementation that all three lanes source. See scripts/lane-common.sh.
LANE="scale-corpus"
LANE_BINARY="bench-corpus binary"
# shellcheck source=scripts/lane-common.sh
source "$(dirname "${BASH_SOURCE[0]}")/lane-common.sh"

readonly PROFILE_ID="purrdf-scale-mixed-v1"

# 1592642302 is the generator's own default seed (0x5EED_CAFE) in the decimal
# form its CLI parses, so an unset SCALE_SEED reproduces a plain `bench-corpus`
# run rather than silently choosing a different corpus.
SEED="${SCALE_SEED:-1592642302}"
QUADS="${SCALE_QUADS:-1000000}"
IRIS="${SCALE_IRIS:-100000}"
SHARDS="${SCALE_SHARDS:-8}"
MODE="${SCALE_MODE:-stream}"
OUT="${SCALE_OUT:-}"
SINK="${SCALE_SINK:-}"
MANIFEST_PATH="${SCALE_MANIFEST:-}"
BIN="${SCALE_BIN:-}"

# WHAT THE MANIFEST CERTIFIES AND WHAT THE LANE PRODUCED ARE TWO DIFFERENT
# NUMBERS, and only the second one is evidence. The manifest is produced by
# asking the binary what it INTENDS to emit; this is the count of what actually
# left the lane, and the run fails unless they are the same number.
#
# The guard is at RUN level and deliberately not at shard level: with
# `--quads 1 --shards 8` the generator gives shard 7 the single row and shards
# 0..6 an empty range, so an empty SHARD is a legitimate result and refusing it
# would be over-refusal. An empty RUN never is: `SCALE_QUADS` is validated
# positive above, so the whole run owes exactly `SCALE_QUADS` rows.
require_run_rows() {
  local produced="$1" where="$2"
  ((produced != QUADS)) || return 0
  ((produced > 0)) ||
    die "the run produced 0 rows ${where} for SCALE_QUADS=${QUADS} — nothing was generated.
  A zero-row run is a FAILURE, not a fast one, and its digest is the SHA-256 of
  the empty string rather than a corpus. No manifest is published for it.
  Suspect the binary this lane ran."
  die "the run produced ${produced} rows ${where}, but its manifest certifies
  emitted_lines=${QUADS}. A manifest is this lane's CERTIFICATE of what the corpus
  IS, so it is never published for a corpus that was only partly produced — a
  truncated run loaded as if it were whole is the worst outcome this lane has.
  Suspect the binary this lane ran."
}

# Copies standard input to standard output UNCHANGED and records what passed
# through, as `rows bytes newline_terminated` in the file named by `$1`.
#
# This is how `pipe` and `SCALE_SINK` account for their output: those modes hand
# the bytes to something outside this lane, so the only place the lane can count
# them is on the way past. The chunk size is the size of a pipe buffer, so a
# consumer sees the payload at the same granularity the kernel was going to hand
# it over at anyway.
#
# `newline_terminated` is the third number because an N-Quads row ENDS with a
# newline: a shard that stopped mid-row has produced bytes that are not a corpus,
# and a row count alone cannot see it.
count_through() {
  python3 -c '
import os
import sys

destination = sys.argv[1]
chunk_bytes = int(sys.argv[2])
out = sys.stdout.buffer
read = sys.stdin.buffer.read
rows = 0
size = 0
last = b""
try:
    while True:
        chunk = read(chunk_bytes)
        if not chunk:
            break
        out.write(chunk)
        rows += chunk.count(b"\n")
        size += len(chunk)
        last = chunk[-1:]
    out.flush()
except BrokenPipeError:
    # THE CONSUMER LEFT, and that is this lane failing rather than this counter
    # failing. It is reported in the voice of the lane, with the shutdown flush of
    # the interpreter redirected away so the diagnostic is not followed by a
    # traceback about the same broken pipe.
    os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
    sys.stderr.write(
        "scale-corpus: the consumer of this pipe closed it after %d rows (%d bytes);\n"
        "  the run is INCOMPLETE and nothing about it is certified\n" % (rows, size)
    )
    raise SystemExit(1)
with open(destination, "w", encoding="utf-8") as handle:
    handle.write("%d %d %d\n" % (rows, size, 1 if last == b"\n" else 0))
' "$1" "${LANE_STREAM_CHUNK_BYTES}"
}

# Sums the per-shard counts this run wrote and applies the law to the total.
# Every mode reaches here, by the same arithmetic, which is the point: a run
# accounted for in one mode and unaccounted for in another is not accounted for.
RUN_ROWS=0
RUN_BYTES=0
account_for_run() {
  local where="$1"
  local index rows=0 bytes=0 shard_rows shard_bytes terminated
  for ((index = 0; index < SHARDS; index++)); do
    [[ -f "${LANE_TMP}/${index}.count" ]] ||
      die "shard ${index} of ${SHARDS} reported success but left no record of what it
  produced, so this run cannot be accounted for and nothing is certified."
    read -r shard_rows shard_bytes terminated <"${LANE_TMP}/${index}.count"
    # An EMPTY shard is legitimate (see `require_run_rows`); a shard that emitted
    # bytes and did not end on a row boundary is not.
    ((shard_bytes == 0)) || ((terminated == 1)) ||
      die "shard ${index} of ${SHARDS} emitted ${shard_bytes} bytes that do not end on a
  row boundary — the last row is TRUNCATED. Those bytes are not N-Quads, so no
  manifest is published for them."
    rows=$((rows + shard_rows))
    bytes=$((bytes + shard_bytes))
  done
  RUN_ROWS="${rows}"
  RUN_BYTES="${bytes}"
  require_run_rows "${rows}" "${where}"
}

lane_require_uint SCALE_SEED SEED
lane_require_positive SCALE_QUADS QUADS
lane_require_positive SCALE_IRIS IRIS
lane_require_positive SCALE_SHARDS SHARDS

case "${MODE}" in
  stream | pipe | files) ;;
  *) die "SCALE_MODE must be one of stream, pipe, files (got '${MODE}')" ;;
esac

if [[ "${MODE}" == "files" && -z "${OUT}" ]]; then
  die "SCALE_MODE=files requires SCALE_OUT=<directory> — materializing a run is always explicit"
fi

# `SCALE_SINK` IS THE ONE PARAMETER THAT IS A COMMAND. Every other knob is a
# path, a count or a keyword, and every one of them arrives here as environment
# bytes that no shell ever parses — `make` `export`s them rather than
# interpolating them into a recipe line, so a backtick or a `$` in a path is
# just a character in a path. The sink is different by design: it is documented
# as "any command that reads standard input", so it is EXECUTED, and this is the
# single place that does it.
#
# Being the exception is not licence to fail badly. A sink that cannot even be
# parsed is caught here, once, before a single shard runs, and reported as a
# lane diagnostic — not as a bare `bash: -c: unexpected EOF` from inside a
# backgrounded shard, and never as an exit-0 run that consumed nothing.
if [[ -n "${SINK}" ]]; then
  sink_parse_error="$(bash -n -c "${SINK}" 2>&1)" ||
    die "SCALE_SINK is not a runnable shell command: ${sink_parse_error#bash: }
  SCALE_SINK is executed as a command by design (it replaces the built-in digest
  sink), so it must parse as one. Got: ${SINK}"
fi

# Resolve the binary through the build that produces it rather than through a
# guessed path: `build.target-dir` may point anywhere, and a stale binary from
# a previous checkout would certify the wrong bytes. `SCALE_BIN` skips the
# build for a caller that already has one (the shards are then pure execs).
BIN_FROM_KNOB=1
if [[ -z "${BIN}" ]]; then
  BIN_FROM_KNOB=0
  echo "scale-corpus: building bench-corpus (release)..." >&2
  BIN="$(cargo build --locked --release -p purrdf-bench --bin bench-corpus \
    --message-format=json-render-diagnostics |
    python3 -c '
import json
import sys

executable = ""
for line in sys.stdin:
    try:
        record = json.loads(line)
    except json.JSONDecodeError:
        continue
    if (
        record.get("reason") == "compiler-artifact"
        and record.get("executable")
        and record.get("target", {}).get("name") == "bench-corpus"
    ):
        executable = record["executable"]
print(executable)
')"
fi

# `SCALE_BIN` NAMES THE EXECUTABLE THAT CERTIFIES THE BYTES, so it is the last
# knob that may fail obscurely. It is validated here, up front, before a banner
# claims a profile and before a single shard runs, and every diagnostic quotes
# the bytes the lane actually used — a typo that resolves to some OTHER
# executable on the same path must be a lane failure, never a silently
# substituted run. The three tests, in this order and with this wording, are the
# ones `lubm-lane.sh` and `watdiv-lane.sh` apply to their own binary knobs,
# because they are one implementation shared by all three.
lane_require_executable SCALE_BIN "${BIN_FROM_KNOB}" "${BIN}" bench-corpus

# The built-in sink: consume a shard, retain only its summary, and record the
# same three accounting numbers `count_through` does so that every mode is
# accounted for by the same arithmetic. Rows are counted as newlines, which is
# exact because the generator emits exactly one line per row.
digest_sink() {
  python3 -c '
import hashlib
import sys

destination = sys.argv[1]
chunk_bytes = int(sys.argv[2])
digest = hashlib.sha256()
size = 0
rows = 0
last = b""
while True:
    chunk = sys.stdin.buffer.read(chunk_bytes)
    if not chunk:
        break
    digest.update(chunk)
    size += len(chunk)
    rows += chunk.count(b"\n")
    last = chunk[-1:]
density = (size / rows) if rows else 0.0
with open(destination, "w", encoding="utf-8") as handle:
    handle.write("%d %d %d\n" % (rows, size, 1 if last == b"\n" else 0))
print(f"rows={rows} bytes={size} bytes_per_row={density:.1f} sha256={digest.hexdigest()}")
' "$1" "${LANE_STREAM_CHUNK_BYTES}"
}

shard_args() {
  printf '%s\n' --seed "${SEED}" --quads "${QUADS}" --iris "${IRIS}" \
    --shard "$1" --shards "${SHARDS}"
}

# Zero-padded to at least five digits so the shard files of any run a single
# filesystem can hold sort lexicographically into shard order; wider shard
# counts widen the field rather than breaking the ordering.
PAD=5
last_index=$((SHARDS - 1))
if ((${#last_index} > PAD)); then PAD=${#last_index}; fi
PREFIX="${PROFILE_ID}.seed${SEED}.quads${QUADS}.iris${IRIS}"

shard_name() {
  printf '%s.shard-%0*d-of-%0*d.nq' "${PREFIX}" "${PAD}" "$1" "${PAD}" "${SHARDS}"
}

# A CERTIFICATE MUST NEVER OUTLIVE THE RUN IT CERTIFIES, and this lane writes two
# kinds: the whole-run manifest and one manifest per shard file.
#
# The whole-run manifest is registered with `lane_certify` as soon as it is
# written, and the shared EXIT trap revokes it when the run fails. The SHARD
# manifests cannot be registered that way — `file_shard` runs in a backgrounded
# subshell whose array writes this shell never sees — so they are swept here
# instead, by the rule that defines them: a shard manifest beside no shard file
# is a certificate for output that was not produced.
#
# `file_shard` removes the manifest of a shard whose corpus WRITE failed, which
# is one route in. It is not the only one: a binary that exits 0 without creating
# the file leaves a manifest beside nothing at all, a binary that creates an
# EMPTY file leaves one beside a file that contradicts it, and a run that dies at
# any later step leaves every shard manifest it had already written. Keying the
# sweep on the shard file's absence would cover only the first of those, so the
# rule is the plain one instead: A RUN THAT FAILED CERTIFIES NOTHING. Every shard
# manifest belonging to this run's parameters goes, exactly as the whole-run
# manifest above it does.
#
# In a REUSED arena that may take a certificate an earlier, successful run wrote.
# That is correct and not collateral damage: this run has already overwritten the
# corpus files those certificates described, so what they certify no longer
# exists either.
lane_revoke_derived_certificates() {
  [[ "${MODE}" == "files" && -n "${OUT}" ]] || return 0
  local index name manifest
  for ((index = 0; index < SHARDS; index++)); do
    name="$(shard_name "${index}")"
    manifest="${OUT}/${name}.manifest.json"
    [[ -f "${manifest}" ]] || continue
    rm -f -- "${manifest}"
    echo "scale-corpus: removed the shard manifest ${manifest} — this run failed, and \
a certificate must never outlive the shard it certifies" >&2
  done
}

# The whole-run manifest: shard 0 of 1, which is the specification every shard
# is a slice of. It is produced ONCE, here, before any mode banner and before
# any shard runs — which also makes it the lane's proof that the binary chosen
# above actually RUNS. The execute bit is not that proof: `/bin/false` carries
# it, and before this check `SCALE_BIN=/bin/false` reached the first real use
# and died there as a bare non-zero exit with nothing on stderr but the banner.
# A knob that names an executable has to fail in the lane's own voice, quoting
# the bytes it used and whatever the binary itself said.
#
# This round trip is why the shared binary check takes the probe as a parameter
# rather than fixing it at `--version`: proving a binary RUNS is running it, and
# proving it is THIS lane's binary is checking what came back. `bench-corpus`
# has no `--version`; its manifest is the stronger probe, because it also has to
# name this profile and these parameters.
if [[ "${BIN_FROM_KNOB}" == "1" ]]; then
  bin_provenance="SCALE_BIN='${BIN}'"
else
  bin_provenance="the binary this lane built, '${BIN}'"
fi
lane_run_probe "${bin_provenance}" "to print the whole-run manifest" \
  "${BIN}" --seed "${SEED}" --quads "${QUADS}" --iris "${IRIS}" \
  --shard 0 --shards 1 --manifest
lane_require_probe_said_something "${bin_provenance}" "to print the whole-run manifest" \
  "The manifest is this lane's CERTIFICATE of what the corpus is, so it is never
  published for output that was not produced. No shard was started."
WHOLE_RUN_MANIFEST="${LANE_PROBE_OUT}"

# EXITING 0 IS NOT PRODUCING A MANIFEST, and the difference is the whole of the
# empty-certificate bug. `SCALE_BIN=/bin/true` exits 0 and writes nothing: the
# status check above passed, `WHOLE_RUN_MANIFEST` was the EMPTY STRING, the lane
# emitted it as a blank line where the specification belongs, ran four shards
# that produced nothing, and printed
# `sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` —
# THE SHA-256 OF THE EMPTY STRING — as each shard's digest, then `total rows=0`,
# then exited 0. The silence itself is refused above; what remains here is the
# stronger question of whether the output is a MANIFEST.
#
# So the manifest is checked for being a manifest, not merely for an exit status:
# it must parse, it must name THIS profile, and its parameters must be the ones
# the lane asked for. That is also what proves the binary is a `bench-corpus` and
# not some other executable that happens to print JSON.
manifest_error="$(printf '%s' "${WHOLE_RUN_MANIFEST}" | python3 -c '
import json
import sys

raw = sys.stdin.read()
try:
    record = json.loads(raw)
except json.JSONDecodeError as error:
    print(f"its output is not JSON ({error}); first 200 bytes: {raw[:200]!r}")
    sys.exit(0)
expected = {
    "profile": sys.argv[1],
    "seed": int(sys.argv[2]),
    "quads": int(sys.argv[3]),
    "iris": int(sys.argv[4]),
    "emitted_lines": int(sys.argv[3]),
}
for key, want in expected.items():
    if key not in record:
        print(f"its manifest has no {key!r} field")
        sys.exit(0)
    if record[key] != want:
        print(f"its manifest reports {key}={record[key]!r}, not the {want!r} this run asked for")
        sys.exit(0)
' "${PROFILE_ID}" "${SEED}" "${QUADS}" "${IRIS}")" ||
  die "could not inspect the whole-run manifest produced by ${bin_provenance}"
if [[ -n "${manifest_error}" ]]; then
  die "${bin_provenance} exited 0 but did not produce a usable whole-run manifest:
  ${manifest_error}
  The manifest is this lane's CERTIFICATE of what the corpus is, so it is never
  published for output that was not produced. No shard was started."
fi

# Replays the manifest captured above. The binary is not re-run: every mode
# reports the SAME specification bytes the shards were checked against.
whole_run_manifest() {
  printf '%s\n' "${WHOLE_RUN_MANIFEST}"
}

# Runs every shard concurrently and waits for all of them, collecting the
# failures instead of stopping at the first: a half-reported run hides which
# shards were actually produced. `$1` names a function taking the shard index.
run_all_shards() {
  local body="$1"
  local -a pids=()
  local index status=0
  for ((index = 0; index < SHARDS; index++)); do
    "${body}" "${index}" &
    pids+=("$!")
  done
  local -a failed=()
  for ((index = 0; index < SHARDS; index++)); do
    if ! wait "${pids[index]}"; then
      failed+=("${index}")
      status=1
    fi
  done
  if ((status != 0)); then
    echo "scale-corpus: FAILED shards: ${failed[*]}" >&2
  fi
  return "${status}"
}

# A SHARD BODY MAY NEVER RELY ON `errexit`, AND NEITHER OF THESE DOES.
#
# These run as backgrounded subshells, and `run_all_shards <body> || die` puts
# the call that starts them on the LEFT of `||`. That suppresses `errexit` for
# the whole dynamic extent of the call — the backgrounded subshells included —
# so a body whose FAILING command is not its last one exited 0 and `wait` saw
# success. (A plain background subshell does honour `errexit`; the `||` context
# is what defeats it. Reproduced on the production surface: a leftover directory
# where shard 1 of 4's file belongs made the corpus write fail, the manifest
# write after it succeed, and `make scale-corpus SCALE_MODE=files` exit 0 —
# printing `bytes=0` for that shard and then its byte-for-byte guarantee, which
# the sorted `cat` disproved. `stream_shard` was correct only by accident,
# because its failing command happened to be last.)
#
# So every command that can fail has its status tested explicitly, and the first
# failure returns non-zero at once — which is what `wait` observes, regardless
# of command order and regardless of the caller's `||` context.

stream_shard() {
  local index="$1"
  local -a args
  # Stated rather than inherited. This body always runs in a subshell, so the
  # setting is local to it, and a generator that dies mid-shard must fail the
  # shard even though the sink downstream of it exits 0 on a short read.
  set -o pipefail
  # `shard_args` is a `printf` over already-validated values; it has no failure
  # mode, and `mapfile`'s status would not report the substituted process's
  # anyway. Every command below that CAN fail is tested.
  mapfile -t args < <(shard_args "${index}")
  if [[ -n "${SINK}" ]]; then
    # A SINK IS NOT AN EXEMPTION FROM ACCOUNTING. `SCALE_SINK` hands the bytes to
    # something outside this lane, which is exactly why the lane has to count
    # them on the way past: with a zero-row binary and `SCALE_SINK='cat
    # >/dev/null'` this mode exited 0, published the manifest, and reported
    # shards that had produced nothing at all.
    #
    # Named in the failure because a sink that parses but cannot RUN (a missing
    # binary, a non-zero exit) otherwise surfaces only as a shard number.
    if ! "${BIN}" "${args[@]}" | count_through "${LANE_TMP}/${index}.count" |
      bash -c "${SINK}" >"${LANE_TMP}/${index}.report"; then
      echo "scale-corpus: SCALE_SINK failed for shard ${index} (command: ${SINK})" >&2
      return 1
    fi
    return 0
  fi
  if ! "${BIN}" "${args[@]}" | digest_sink "${LANE_TMP}/${index}.count" \
    >"${LANE_TMP}/${index}.report"; then
    echo "scale-corpus: shard ${index} of ${SHARDS} failed to generate" >&2
    return 1
  fi
}

file_shard() {
  local index="$1"
  local -a args
  # `shard_args` is a `printf` over already-validated values; it has no failure
  # mode, and `mapfile`'s status would not report the substituted process's
  # anyway. Every command below that CAN fail is tested.
  mapfile -t args < <(shard_args "${index}")
  local name
  name="$(shard_name "${index}")"
  if ! "${BIN}" "${args[@]}" --out "${OUT}/${name}"; then
    echo "scale-corpus: shard ${index} of ${SHARDS} could not be written to ${OUT}/${name}" >&2
    # A SHARD MANIFEST IS A CERTIFICATE, so one must never outlive the shard it
    # certifies. A reused arena can hold the manifest of an earlier, successful
    # run of this same shard, and leaving it in place beside a corpus file that
    # was not produced is exactly the certification this fix exists to stop.
    rm -f "${OUT}/${name}.manifest.json"
    return 1
  fi
  # EXITING 0 IS NOT WRITING THE FILE, and the manifest is written next — so a
  # binary that exits 0 having created nothing would leave a certificate beside
  # no shard at all. The existence test goes BEFORE the manifest write for that
  # reason, and the manifest of an earlier, successful run of this same shard in
  # a reused arena is removed with it.
  if [[ ! -f "${OUT}/${name}" ]]; then
    echo "scale-corpus: shard ${index} of ${SHARDS} exited 0 but wrote no file at ${OUT}/${name}" >&2
    rm -f "${OUT}/${name}.manifest.json"
    return 1
  fi
  if ! "${BIN}" "${args[@]}" --manifest --out "${OUT}/${name}.manifest.json"; then
    echo "scale-corpus: shard ${index} of ${SHARDS} could not write its manifest to ${OUT}/${name}.manifest.json" >&2
    return 1
  fi
}

# Emits the whole-run manifest to `SCALE_MANIFEST` if the caller named a file,
# and otherwise to the default destination for this mode: `stdout` where the
# manifest is part of the report, `stderr` where stdout carries corpus bytes,
# or a path beside the output.
#
# The destination is a keyword rather than `/dev/stdout` on purpose: `>` on
# `/dev/stdout` TRUNCATES when standard output is a regular file, so a run
# redirected to a log would have overwritten everything printed before the
# manifest with the manifest.
#
# `SCALE_MANIFEST` is taken LITERALLY — it is a path, not an expression, and
# every character in it is part of the filename. An unopenable path is a lane
# failure with the path quoted back verbatim, so an operator can see exactly
# which bytes were used; the one thing this must never do is write somewhere
# else and report success.
#
# EVERY manifest write here is status-checked, and by the SAME code. The
# `SCALE_MANIFEST` branch had the three-line diagnostic and the default-path
# branch below it had a bare `whole_run_manifest >"$1"`, so the identical fault —
# an unwritable destination — produced a lane diagnostic naming the knob through
# one branch and a bare `scripts/scale-corpus.sh: line N: ...: Is a directory`
# through the other. One law, one implementation: `lane_write_checked`, which is
# the same one `lubm-lane.sh` and `watdiv-lane.sh` write their files through.
emit_manifest() {
  if [[ -n "${MANIFEST_PATH}" ]]; then
    lane_write_checked "${MANIFEST_PATH}" "the manifest" \
      "SCALE_MANIFEST='${MANIFEST_PATH}'" whole_run_manifest
    lane_certify "${MANIFEST_PATH}" "the whole-run manifest"
    echo "scale-corpus: manifest written to ${MANIFEST_PATH}" >&2
    return
  fi
  case "$1" in
    stdout) whole_run_manifest ;;
    stderr) whole_run_manifest >&2 ;;
    *)
      lane_write_checked "$1" "the whole-run manifest" \
        "'$1', its default destination under SCALE_OUT='${OUT}'" whole_run_manifest
      lane_certify "$1" "the whole-run manifest"
      echo "scale-corpus: manifest written to $1" >&2
      ;;
  esac
}

case "${MODE}" in
  stream)
    echo "scale-corpus: profile=${PROFILE_ID} mode=stream shards=${SHARDS} (retaining nothing)" >&2
    emit_manifest stdout
    run_all_shards stream_shard || die "at least one shard failed"
    # ACCOUNTED FOR IN BOTH HALVES OF THIS MODE, AND BEFORE A WORD IS REPORTED.
    # The totals line is only printed when this lane's own sink produced the
    # per-shard reports; the ACCOUNTING happens either way, because `SCALE_SINK`
    # changes who consumes the bytes and not whether they were produced.
    #
    # The ORDER is the point. A per-shard report carries that shard's digest, and
    # a zero-row shard's digest is
    # `sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
    # — the SHA-256 of the empty string, which `lane_require_nonempty_file` says
    # is DESCRIBED and never EMITTED. Reporting first and accounting second put
    # that digest on stdout for a run the next line was about to refuse: the exit
    # status was 1 and nothing was certified, but the lane had still published the
    # empty certificate it exists to withhold. Nothing is reported for a run that
    # is about to fail.
    account_for_run "across all ${SHARDS} shards"
    for ((index = 0; index < SHARDS; index++)); do
      printf 'shard %d/%d %s\n' "${index}" "${SHARDS}" "$(cat "${LANE_TMP}/${index}.report")"
    done
    if [[ -z "${SINK}" ]]; then
      density=0.0
      if ((RUN_ROWS > 0)); then
        density="$(python3 -c 'import sys; print(f"{int(sys.argv[1]) / int(sys.argv[2]):.1f}")' \
          "${RUN_BYTES}" "${RUN_ROWS}")"
      fi
      printf 'total rows=%s bytes=%s bytes_per_row=%s\n' "${RUN_ROWS}" "${RUN_BYTES}" "${density}"
    else
      printf 'total rows=%s bytes=%s through SCALE_SINK\n' "${RUN_ROWS}" "${RUN_BYTES}"
    fi
    ;;
  pipe)
    echo "scale-corpus: profile=${PROFILE_ID} mode=pipe shards=${SHARDS} (ordered whole run on stdout)" >&2
    emit_manifest stderr
    for ((index = 0; index < SHARDS; index++)); do
      mapfile -t args < <(shard_args "${index}")
      # Counted on the way out. stdout belongs to the consumer here, so the count
      # is the only place this lane can see what it delivered — and the totals go
      # to stderr, where they cannot corrupt the payload.
      "${BIN}" "${args[@]}" | count_through "${LANE_TMP}/${index}.count" ||
        die "shard ${index} of ${SHARDS} failed with status $? — the stream is INCOMPLETE"
    done
    account_for_run "onto standard output"
    echo "scale-corpus: delivered ${RUN_ROWS} rows, ${RUN_BYTES} bytes on stdout" >&2
    ;;
  files)
    # `SCALE_OUT` gets the same treatment `SCALE_MANIFEST` already had: an
    # unusable directory is a LANE failure that quotes the bytes back, not a
    # bare `mkdir: cannot create directory ...` with no hint which knob supplied
    # it. The two messages are deliberately the same shape.
    lane_mkdir_checked "${OUT}" "SCALE_OUT='${OUT}'"
    echo "scale-corpus: profile=${PROFILE_ID} mode=files shards=${SHARDS} out=${OUT}" >&2
    emit_manifest "${OUT}/${PREFIX}.manifest.json"
    run_all_shards file_shard || die "at least one shard failed"
    # The guarantee on the last line is only worth printing if every shard file
    # is actually there to be `cat`ed AND holds the rows the manifest certifies,
    # so each one is measured with its status checked. `wc -c < <a directory>`
    # succeeds as a redirection and reports `bytes=0`, which is precisely how a
    # missing shard used to be announced as an empty one.
    #
    # `wc -lc` is ONE pass for both numbers, and `tail -c 1` is a seek: the cost
    # of accounting for a materialized corpus is a single sequential read of what
    # was just written, and it is what turns the manifest beside these files from
    # a claim into a certificate.
    #
    # MEASURING IS NOT REPORTING, and this mode has to do the first to be able to
    # do the second — the per-shard counts below are what `account_for_run` adds
    # up. So the lines are held rather than printed, and printed only once the
    # run as a whole has been accounted for, by the same law `stream` follows: a
    # run that is about to be refused reports nothing.
    shard_reports=()
    for ((index = 0; index < SHARDS; index++)); do
      name="$(shard_name "${index}")"
      [[ -f "${OUT}/${name}" ]] ||
        die "shard ${index} of ${SHARDS} reported success but ${OUT}/${name} is not a regular file"
      if ! measured="$(wc -lc <"${OUT}/${name}")"; then
        die "cannot measure the shard file ${OUT}/${name}"
      fi
      read -r rows bytes <<<"${measured}"
      # NON-EMPTY IS NOT "IS N-QUADS", and this lane's output is N-Quads too. An
      # EMPTY shard file is legitimate here (see `require_run_rows`), so only the
      # shards that produced something are asked to have produced the right thing
      # — by the same implementation `lubm-lane.sh` checks its dataset with.
      if ((bytes > 0)); then
        lane_require_nquads "${OUT}/${name}" "shard ${index} of ${SHARDS}"
      fi
      terminated=1
      if ((bytes > 0)) && [[ "$(tail -c 1 -- "${OUT}/${name}")" != "" ]]; then
        # `$(...)` strips trailing newlines, so a final newline reads back as the
        # empty string and anything else reads back as itself.
        terminated=0
      fi
      shard_reports+=("$(printf '%s rows=%s bytes=%s' "${name}" "${rows}" "${bytes}")")
      printf '%d %d %d\n' "${rows}" "${bytes}" "${terminated}" >"${LANE_TMP}/${index}.count"
    done
    # Materializing a corpus of zero bytes and then printing the byte-for-byte
    # guarantee over it would certify emptiness; materializing HALF of one and
    # printing it would be worse. An individual shard file may legitimately be
    # empty (see `require_run_rows`); the run may not.
    account_for_run "across all ${SHARDS} shard files"
    for ((index = 0; index < SHARDS; index++)); do
      printf '%s\n' "${shard_reports[index]}"
    done
    echo "scale-corpus: cat ${OUT}/${PREFIX}.shard-*.nq reproduces a whole run byte for byte" >&2
    ;;
esac
