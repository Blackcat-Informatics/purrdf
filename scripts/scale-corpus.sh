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
# pass would report a corpus that was never generated. Each shard body checks
# every command's status ITSELF instead of trusting `errexit`, which a
# backgrounded subshell does not honour inside the `|| die` context this driver
# runs them from — see the note above `stream_shard`.

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

# The whole-run manifest this run actually wrote to a FILE, if any. See
# `cleanup` below: a manifest is a certificate, and a certificate must never
# outlive the run it certifies.
WROTE_MANIFEST=""

die() {
  echo "scale-corpus: $*" >&2
  exit 1
}

# A WRITE WHOSE STATUS IS NOT CHECKED IS A SILENT DROP. Every redirection in
# this lane goes through here, so an unopenable destination is a LANE failure
# that names the knob and quotes the bytes back — never a bare
# `scripts/scale-corpus.sh: line N: ...: Is a directory` with no hint which knob
# supplied the path. `$1` is the destination, `$2` WHAT is being written, `$3` how
# to name WHERE it was going (the caller spells this so the knob appears exactly
# as an operator set it), `$4` the payload.
#
# The payload is an ARGUMENT rather than standard input on purpose: a pipeline
# would put this function on the right-hand side, where `die`'s `exit` leaves a
# subshell instead of the lane and the failure's propagation depends on
# `pipefail` staying set. Called as a plain command, `die` means what it says.
write_checked() {
  local destination="$1" what="$2" where="$3" payload="$4" write_error
  if ! write_error="$({ printf '%s\n' "${payload}" >"${destination}"; } 2>&1)"; then
    die "cannot write ${what} to ${where}
  ${write_error}
  The path is used exactly as given, byte for byte; nothing in it is expanded.
  Check that its parent directory exists and is writable."
  fi
}

require_positive() {
  local name="$1" value="$2"
  [[ "${value}" =~ ^[0-9]+$ ]] ||
    die "${name} must be a decimal unsigned integer (got '${value}')"
  [[ "${value}" != "0" ]] || die "${name} must be positive"
}

# A RUN THAT PRODUCED NOTHING IS A FAILED RUN, and it must never be reported as
# a fast one. The guard is at RUN level and deliberately not at shard level: with
# `--quads 1 --shards 8` the generator gives shard 7 the single row and shards
# 0..6 an empty range, so an empty SHARD is a legitimate result and refusing it
# would be over-refusal. An empty RUN never is: `SCALE_QUADS` is validated
# positive above, so the whole run owes at least one row.
require_nonempty_run() {
  local produced="$1" unit="$2"
  ((produced > 0)) ||
    die "the run produced 0 ${unit} for SCALE_QUADS=${QUADS} — nothing was generated.
  A zero-row run is a FAILURE, not a fast one, and its digest is the SHA-256 of
  the empty string rather than a corpus. Suspect the binary this lane ran."
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
# substituted run.
if [[ "${BIN_FROM_KNOB}" == "1" ]]; then
  [[ -e "${BIN}" ]] ||
    die "SCALE_BIN='${BIN}' does not exist
  The path is used exactly as given, byte for byte; nothing in it is expanded.
  Leave SCALE_BIN unset to have this lane build bench-corpus itself."
  [[ -f "${BIN}" ]] ||
    die "SCALE_BIN='${BIN}' is not a regular file"
  [[ -x "${BIN}" ]] ||
    die "SCALE_BIN='${BIN}' is not executable"
else
  [[ -n "${BIN}" && -x "${BIN}" ]] ||
    die "the release build produced no executable bench-corpus binary (set SCALE_BIN to point at one)"
fi

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

tmp="$(mktemp -d)"

# A MANIFEST IS A CERTIFICATE, SO ONE MUST NEVER OUTLIVE THE RUN IT CERTIFIES.
#
# `file_shard` already applies that law at SHARD level. It was not applied at RUN
# level, and the whole-run manifest is written BEFORE the first shard starts (it
# has to be — it is also the proof that the binary runs), so a run that lost a
# shard left the arena holding three of four corpus files beside a manifest
# reading `"quads": 2000, "emitted_lines": 2000` with no marker of failure at
# all. Anything that later read that arena would certify a corpus that was never
# produced.
#
# The removal hangs off the EXIT trap rather than off `die` so it also covers an
# `errexit` death that never reaches `die`.
cleanup() {
  local status=$?
  rm -rf "${tmp}"
  if ((status != 0)) && [[ -n "${WROTE_MANIFEST}" ]]; then
    rm -f -- "${WROTE_MANIFEST}"
    echo "scale-corpus: removed the whole-run manifest ${WROTE_MANIFEST} — this run \
failed, and a manifest must never certify a corpus that was not produced" >&2
  fi
}
trap cleanup EXIT

# The whole-run manifest: shard 0 of 1, which is the specification every shard
# is a slice of. It is produced ONCE, here, before any mode banner and before
# any shard runs — which also makes it the lane's proof that the binary chosen
# above actually RUNS. The execute bit is not that proof: `/bin/false` carries
# it, and before this check `SCALE_BIN=/bin/false` reached the first real use
# and died there as a bare non-zero exit with nothing on stderr but the banner.
# A knob that names an executable has to fail in the lane's own voice, quoting
# the bytes it used and whatever the binary itself said.
if [[ "${BIN_FROM_KNOB}" == "1" ]]; then
  bin_provenance="SCALE_BIN='${BIN}'"
else
  bin_provenance="the binary this lane built, '${BIN}'"
fi
manifest_status=0
WHOLE_RUN_MANIFEST="$("${BIN}" --seed "${SEED}" --quads "${QUADS}" --iris "${IRIS}" \
  --shard 0 --shards 1 --manifest 2>"${tmp}/manifest.err")" || manifest_status=$?
if ((manifest_status != 0)); then
  # Whatever the binary itself said is reported inside the lane's message, not
  # leaked as a bare line above it — and a binary that said nothing (`/bin/false`
  # says nothing) contributes no empty line either.
  manifest_detail=""
  if [[ -s "${tmp}/manifest.err" ]]; then
    manifest_detail="$(sed 's/^/  /' "${tmp}/manifest.err")
"
  fi
  die "${bin_provenance} failed to produce the whole-run manifest (exit ${manifest_status})
${manifest_detail}  The path is used exactly as given, byte for byte; nothing in it is expanded.
  Nothing downstream can be trusted when the specification itself cannot be
  produced, so no shard was started."
fi

# EXITING 0 IS NOT PRODUCING A MANIFEST, and the difference is the whole of the
# empty-certificate bug. `SCALE_BIN=/bin/true` exits 0 and writes nothing: the
# status check above passed, `WHOLE_RUN_MANIFEST` was the EMPTY STRING, the lane
# emitted it as a blank line where the specification belongs, ran four shards
# that produced nothing, and printed
# `sha256=e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` —
# THE SHA-256 OF THE EMPTY STRING — as each shard's digest, then `total rows=0`,
# then exited 0.
#
# So the manifest is checked for being a manifest, not merely for an exit status:
# it must parse, it must name THIS profile, and its parameters must be the ones
# the lane asked for. That is also what proves the binary is a `bench-corpus` and
# not some other executable that happens to print JSON.
manifest_error="$(printf '%s' "${WHOLE_RUN_MANIFEST}" | python3 -c '
import json
import sys

raw = sys.stdin.read()
if not raw.strip():
    print("it produced NO OUTPUT AT ALL (a binary that exits 0 and says nothing "
          "is not a binary that generated a corpus)")
    sys.exit(0)
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
    # Named in the failure because a sink that parses but cannot RUN (a missing
    # binary, a non-zero exit) otherwise surfaces only as a shard number.
    if ! "${BIN}" "${args[@]}" | bash -c "${SINK}" >"${tmp}/${index}.report"; then
      echo "scale-corpus: SCALE_SINK failed for shard ${index} (command: ${SINK})" >&2
      return 1
    fi
    return 0
  fi
  if ! "${BIN}" "${args[@]}" | digest_sink >"${tmp}/${index}.report"; then
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
# through the other. One law, one implementation: `write_checked`.
emit_manifest() {
  if [[ -n "${MANIFEST_PATH}" ]]; then
    write_checked "${MANIFEST_PATH}" "the manifest" \
      "SCALE_MANIFEST='${MANIFEST_PATH}'" "${WHOLE_RUN_MANIFEST}"
    WROTE_MANIFEST="${MANIFEST_PATH}"
    echo "scale-corpus: manifest written to ${MANIFEST_PATH}" >&2
    return
  fi
  case "$1" in
    stdout) whole_run_manifest ;;
    stderr) whole_run_manifest >&2 ;;
    *)
      write_checked "$1" "the whole-run manifest" \
        "'$1', its default destination under SCALE_OUT='${OUT}'" "${WHOLE_RUN_MANIFEST}"
      WROTE_MANIFEST="$1"
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
      total_line="$(cat "${tmp}"/*.report | python3 -c '
import sys

rows = 0
size = 0
for line in sys.stdin:
    fields = dict(field.split("=", 1) for field in line.split() if "=" in field)
    rows += int(fields["rows"])
    size += int(fields["bytes"])
density = (size / rows) if rows else 0.0
print(f"total rows={rows} bytes={size} bytes_per_row={density:.1f}")
')"
      printf '%s\n' "${total_line}"
      total_rows="${total_line#total rows=}"
      require_nonempty_run "${total_rows%% *}" "rows"
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
    # `SCALE_OUT` gets the same treatment `SCALE_MANIFEST` already had: an
    # unusable directory is a LANE failure that quotes the bytes back, not a
    # bare `mkdir: cannot create directory ...` with no hint which knob supplied
    # it. The two messages are deliberately the same shape.
    if ! mkdir_error="$(mkdir -p "${OUT}" 2>&1)"; then
      die "cannot create the output directory SCALE_OUT='${OUT}'
  ${mkdir_error}
  The path is used exactly as given, byte for byte; nothing in it is expanded.
  Check that its parent directory exists and is writable."
    fi
    echo "scale-corpus: profile=${PROFILE_ID} mode=files shards=${SHARDS} out=${OUT}" >&2
    emit_manifest "${OUT}/${PREFIX}.manifest.json"
    run_all_shards file_shard || die "at least one shard failed"
    # The guarantee on the last line is only worth printing if every shard file
    # is actually there to be `cat`ed, so the sizes are read with their status
    # checked. `wc -c < <a directory>` succeeds as a redirection and reports
    # `bytes=0`, which is precisely how a missing shard used to be announced as
    # an empty one.
    run_bytes=0
    for ((index = 0; index < SHARDS; index++)); do
      name="$(shard_name "${index}")"
      [[ -f "${OUT}/${name}" ]] ||
        die "shard ${index} of ${SHARDS} reported success but ${OUT}/${name} is not a regular file"
      if ! bytes="$(wc -c <"${OUT}/${name}")"; then
        die "cannot size the shard file ${OUT}/${name}"
      fi
      printf '%s bytes=%s\n' "${name}" "${bytes}"
      run_bytes=$((run_bytes + bytes))
    done
    # Materializing a corpus of zero bytes and then printing the byte-for-byte
    # guarantee over it would certify emptiness. An individual shard file may
    # legitimately be empty (see `require_nonempty_run`); the run may not.
    require_nonempty_run "${run_bytes}" "bytes across all ${SHARDS} shard files"
    echo "scale-corpus: cat ${OUT}/${PREFIX}.shard-*.nq reproduces a whole run byte for byte" >&2
    ;;
esac
