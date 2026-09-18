#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
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
#           output, unmixed and unbuffered by this script. Those bytes ARE a
#           whole run: `SCALE_MODE=pipe ... | your-loader` loads the same
#           corpus a single unsharded run would have produced. Ordered output
#           cannot outrun its consumer, so this mode trades the concurrency for
#           the ordering; `stream` is the one that uses the cores.
#
#           TRAP for a wrapper `Makefile` that shells out to `make
#           scale-corpus SCALE_MODE=pipe`: GNU make turns `-w`/`--print-
#           directory` on BY ITSELF for any invocation it detects as a
#           sub-make, which it decides from a nonzero inherited `MAKELEVEL` --
#           NOT from `MAKEFLAGS`. A nested `$(MAKE)` shows an EMPTY `MAKEFLAGS`
#           and `MAKELEVEL=1`, and the banner appears anyway, so an operator who
#           inspects `MAKEFLAGS`, finds it clean and concludes the pipe is safe
#           has checked the wrong variable. `make` then writes `make[1]:
#           Entering directory '...'` to standard output before this script's
#           payload does, corrupting the loader's first line. A plain shell
#           invocation has no recursion state at all and never sees this.
#           THIS REPOSITORY'S `Makefile` SETS `MAKEFLAGS += --no-print-directory`
#           AT ITS TOP so the payload is already clean at any `MAKELEVEL`; a
#           wrapper around some OTHER `Makefile` has to pass
#           `--no-print-directory` (or `-s`) itself, e.g.
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
# Any shard that fails fails the whole run, loudly. `bench-corpus` exits 2 on a
# rejected specification and 1 on a write failure, and a driver that let either
# pass would report a corpus that was never generated.

set -euo pipefail

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

die() {
  echo "scale-corpus: $*" >&2
  exit 1
}

require_positive() {
  local name="$1" value="$2"
  [[ "${value}" =~ ^[0-9]+$ ]] ||
    die "${name} must be a decimal unsigned integer (got '${value}')"
  [[ "${value}" != "0" ]] || die "${name} must be positive"
}

require_positive SCALE_QUADS "${QUADS}"
require_positive SCALE_IRIS "${IRIS}"
require_positive SCALE_SHARDS "${SHARDS}"
[[ "${SEED}" =~ ^[0-9]+$ ]] ||
  die "SCALE_SEED must be a decimal unsigned integer (got '${SEED}')"

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
if [[ -z "${BIN}" ]]; then
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
[[ -n "${BIN}" && -x "${BIN}" ]] ||
  die "no executable bench-corpus binary (set SCALE_BIN to point at one)"

# The built-in sink: consume a shard and retain only its summary. Rows are
# counted as newlines, which is exact because the generator emits exactly one
# line per row.
digest_sink() {
  python3 -c '
import hashlib
import sys

digest = hashlib.sha256()
size = 0
rows = 0
while True:
    chunk = sys.stdin.buffer.read(1 << 20)
    if not chunk:
        break
    digest.update(chunk)
    size += len(chunk)
    rows += chunk.count(b"\n")
density = (size / rows) if rows else 0.0
print(f"rows={rows} bytes={size} bytes_per_row={density:.1f} sha256={digest.hexdigest()}")
'
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

# The whole-run manifest: shard 0 of 1, which is the specification every shard
# is a slice of.
whole_run_manifest() {
  "${BIN}" --seed "${SEED}" --quads "${QUADS}" --iris "${IRIS}" \
    --shard 0 --shards 1 --manifest
}

tmp="$(mktemp -d)"
trap 'rm -rf "${tmp}"' EXIT

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

stream_shard() {
  local index="$1"
  local -a args
  mapfile -t args < <(shard_args "${index}")
  if [[ -n "${SINK}" ]]; then
    # Named in the failure because a sink that parses but cannot RUN (a missing
    # binary, a non-zero exit) otherwise surfaces only as a shard number.
    "${BIN}" "${args[@]}" | bash -c "${SINK}" >"${tmp}/${index}.report" || {
      echo "scale-corpus: SCALE_SINK failed for shard ${index} (command: ${SINK})" >&2
      return 1
    }
  else
    "${BIN}" "${args[@]}" | digest_sink >"${tmp}/${index}.report"
  fi
}

file_shard() {
  local index="$1"
  local -a args
  mapfile -t args < <(shard_args "${index}")
  local name
  name="$(shard_name "${index}")"
  "${BIN}" "${args[@]}" --out "${OUT}/${name}"
  "${BIN}" "${args[@]}" --manifest --out "${OUT}/${name}.manifest.json"
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
emit_manifest() {
  if [[ -n "${MANIFEST_PATH}" ]]; then
    # The write happens exactly once; its diagnostics are CAPTURED rather than
    # discarded, so the redirection error (and anything the generator said) is
    # reported inside the lane's own message instead of leaking as a bare shell
    # line or, worse, being swallowed.
    local write_error
    if ! write_error="$({ whole_run_manifest >"${MANIFEST_PATH}"; } 2>&1)"; then
      die "cannot write the manifest to SCALE_MANIFEST='${MANIFEST_PATH}'
  ${write_error}
  The path is used exactly as given, byte for byte; nothing in it is expanded.
  Check that its parent directory exists and is writable."
    fi
    echo "scale-corpus: manifest written to ${MANIFEST_PATH}" >&2
    return
  fi
  case "$1" in
    stdout) whole_run_manifest ;;
    stderr) whole_run_manifest >&2 ;;
    *)
      whole_run_manifest >"$1"
      echo "scale-corpus: manifest written to $1" >&2
      ;;
  esac
}

case "${MODE}" in
  stream)
    echo "scale-corpus: profile=${PROFILE_ID} mode=stream shards=${SHARDS} (retaining nothing)" >&2
    emit_manifest stdout
    run_all_shards stream_shard || die "at least one shard failed"
    for ((index = 0; index < SHARDS; index++)); do
      printf 'shard %d/%d %s\n' "${index}" "${SHARDS}" "$(cat "${tmp}/${index}.report")"
    done
    if [[ -z "${SINK}" ]]; then
      cat "${tmp}"/*.report | python3 -c '
import sys

rows = 0
size = 0
for line in sys.stdin:
    fields = dict(field.split("=", 1) for field in line.split() if "=" in field)
    rows += int(fields["rows"])
    size += int(fields["bytes"])
density = (size / rows) if rows else 0.0
print(f"total rows={rows} bytes={size} bytes_per_row={density:.1f}")
'
    fi
    ;;
  pipe)
    echo "scale-corpus: profile=${PROFILE_ID} mode=pipe shards=${SHARDS} (ordered whole run on stdout)" >&2
    emit_manifest stderr
    for ((index = 0; index < SHARDS; index++)); do
      mapfile -t args < <(shard_args "${index}")
      "${BIN}" "${args[@]}" ||
        die "shard ${index} of ${SHARDS} failed with status $? — the stream is INCOMPLETE"
    done
    ;;
  files)
    mkdir -p "${OUT}"
    echo "scale-corpus: profile=${PROFILE_ID} mode=files shards=${SHARDS} out=${OUT}" >&2
    emit_manifest "${OUT}/${PREFIX}.manifest.json"
    run_all_shards file_shard || die "at least one shard failed"
    for ((index = 0; index < SHARDS; index++)); do
      name="$(shard_name "${index}")"
      printf '%s bytes=%s\n' "${name}" "$(wc -c <"${OUT}/${name}")"
    done
    echo "scale-corpus: cat ${OUT}/${PREFIX}.shard-*.nq reproduces a whole run byte for byte" >&2
    ;;
esac
