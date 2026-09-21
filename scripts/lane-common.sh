#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# The laws every benchmark lane in this repository shares, in ONE implementation.
#
# WHY each of these is a law, what a lane's digest does and does not certify, and
# why the per-lane `write_checked`/`mkdir_checked`/`require_nonempty_file` wrappers
# are adapters that must NOT be "de-duplicated" away:
# docs/design/purrdf-bench-lane-laws.md
#
# WHY THIS FILE EXISTS
# ====================
#
# `scale-corpus.sh`, `lubm-lane.sh` and `watdiv-lane.sh` each carried their own
# copy of the same three rules — how a lane dies, how a lane's scratch directory
# is cleaned up, and how the executable that certifies every number is checked.
# Three copies of a rule are three rules: the binary check drifted in five
# places between two of the copies and was structurally different in the third,
# and a repair applied to one lane silently left the siblings unrepaired. A law
# that holds in one lane and not its siblings is not a law, so the shared part
# lives here and is sourced, and the part that genuinely differs per lane is a
# PARAMETER of these functions rather than a second copy of them.
#
# WHAT A LANE MUST SET BEFORE SOURCING THIS FILE
# ==============================================
#
#   LANE         the name every diagnostic is prefixed with (`scale-corpus`)
#   LANE_BINARY  prose for the kind of executable this lane runs, used in the
#                binary diagnostics (`purrdf binary`, `bench-corpus binary`)
#
# Sourcing installs the EXIT trap, creates `LANE_TMP`, and defines `die`.
#
# A CERTIFICATE MUST NEVER OUTLIVE THE RUN IT CERTIFIES
# =====================================================
#
# A lane writes two kinds of file: OUTPUT (a corpus, a dataset, a pack) and
# CERTIFICATES about that output (a manifest, a reuse stamp, a digest record). A
# certificate is what a LATER reader — a capture, an operator, the next run of
# this same lane — consults instead of re-deriving the fact. So a certificate
# left behind by a run that FAILED is worse than no certificate at all: it
# asserts something about output that was never produced, and nothing
# downstream can tell the difference.
#
# Every lane therefore registers each certificate it writes with
# [`lane_certify`], and the shared EXIT trap revokes every registered
# certificate when the run exits non-zero — including an `errexit` death that
# never reaches `die`. The removal is announced, because a certificate silently
# vanishing is its own puzzle.

# A lane must name itself before sourcing this file: every diagnostic below is
# prefixed with it, and a bare message with no lane name is exactly the shape
# these lanes exist to stop.
: "${LANE:?lane-common.sh requires LANE to be set to the name of the lane}"
: "${LANE_BINARY:?lane-common.sh requires LANE_BINARY to name the kind of executable the lane runs}"

# COLLATION IS AN INPUT TO EVERY DIGEST A LANE PUBLISHES, so it is pinned here
# rather than left to the caller's environment.
#
# A lane that prints `sha256(...)` and calls it the determinism check promises
# that the inputs it names, and nothing else, reproduce those bytes. `sort(1)`
# obeys `LC_COLLATE`, and it is the input nobody writes down because a tool
# supplies it. Under `LC_ALL=C` the comparison is bytewise, so `.` (0x2E)
# precedes `0` (0x30) and `University0_1.owl` sorts before `University0_10.owl`;
# under a UTF-8 collation punctuation is ignorable at the primary level and the
# two reverse. A lane concatenating in `find | sort` order therefore builds a
# different corpus, publishes a different digest, and picks a different file as
# its smallest rung on two hosts that differ only in their environment -- while
# every knob the lane enumerates as reproducing that digest is identical.
#
# This also pins the locale of every tool a lane runs, the generator included,
# so a decimal separator or a case-folding rule cannot reach generated output
# either. The certificate is only worth as much as the enumeration behind it,
# and the cheapest way to enumerate collation is to remove it as a variable.
#
# A change justified by enumeration must not itself be under-enumerated, so the
# one locale-derived JVM behaviour that could reach generated output is written
# down: on JDK 18 and later, JEP 400 fixes `file.encoding` to UTF-8 regardless of
# locale (measured: `LC_ALL=C java -XshowSettings:properties` reports
# `file.encoding = UTF-8`, with only `native.encoding`/`sun.jnu.encoding` becoming
# ASCII, and those govern FILENAME decoding rather than content). On JDK 17 and
# earlier `file.encoding` did follow the locale, and UBA writes through a
# default-charset `FileWriter` -- so on such a JVM a non-ASCII character would be
# emitted as `?`. LUBM content is ASCII, which is why this is benign rather than
# why it is unexamined.
export LC_ALL=C

die() {
  echo "${LANE}: $*" >&2
  exit 1
}

# The certificates this run has written, in the order it wrote them.
LANE_CERTIFICATES=()

# Records that `$1` is a certificate THIS run wrote. `$2` is how to describe it
# in the revocation notice.
lane_certify() {
  LANE_CERTIFICATES+=("$1"$'\t'"$2")
}

# Removes every certificate this run wrote. Called only from the failure path of
# the EXIT trap.
lane_revoke_certificates() {
  local entry path description
  ((${#LANE_CERTIFICATES[@]} > 0)) || return 0
  for entry in "${LANE_CERTIFICATES[@]}"; do
    path="${entry%%$'\t'*}"
    description="${entry#*$'\t'}"
    [[ -e "${path}" ]] || continue
    rm -f -- "${path}"
    echo "${LANE}: removed ${description} ${path} — this run failed, and a \
certificate must never outlive the run it certifies" >&2
  done
}

LANE_TMP="$(mktemp -d)"

# A HELPER THAT MUST WRITE OUTSIDE `LANE_TMP` STILL OWES THE TRAP ITS PATH.
# `lane_query_set_digest` captures its interpreter's stderr with `mktemp` rather
# than into `LANE_TMP`, deliberately: a certificate must not acquire a dependency
# on a directory whose failure it is the thing reporting. The cost of moving out
# was that the trap no longer knew about the file, so an interrupt mid-digest left
# it behind in TMPDIR -- demonstrated with SIGINT during a 600 MB corpus. Buying
# independence from `LANE_TMP` must not cost the cleanup guarantee it provided, so
# such a file is registered here and swept on every exit path, signals included.
LANE_STRAY_FILES=()

lane_track_stray() {
  LANE_STRAY_FILES+=("$1")
}

# THE CHUNK SIZE EVERY STREAMED READ USES, read from the one place that defines it.
# A shell cannot import a module, so the number is fetched once here and passed into
# the embedded Python blocks as an argument -- which is what keeps the shell sites
# and the Python sites from being two definitions again. They were: six copies under
# two names across five files, two of them already drifted -- one to a quarter of the
# shared size and one to a sixteenth -- and because every chunk size produces a
# correct digest, that divergence had nothing to report it.
LANE_STREAM_CHUNK_BYTES="$(python3 "$(dirname "${BASH_SOURCE[0]}")/lane_chunk.py")" ||
  die "cannot read the shared stream chunk size from scripts/lane_chunk.py"
[[ "${LANE_STREAM_CHUNK_BYTES}" =~ ^[0-9]+$ ]] ||
  die "scripts/lane_chunk.py printed '${LANE_STREAM_CHUNK_BYTES}', which is not a byte count"
readonly LANE_STREAM_CHUNK_BYTES

lane_cleanup() {
  local status=$?
  rm -rf "${LANE_TMP}"
  ((${#LANE_STRAY_FILES[@]} == 0)) || rm -f -- "${LANE_STRAY_FILES[@]}"
  if ((status != 0)); then
    lane_revoke_certificates
    # A lane may own certificates it cannot name in advance — one per shard, for
    # instance, written by a subshell that cannot reach this shell's array. It
    # defines `lane_revoke_derived_certificates` to sweep those.
    if declare -F lane_revoke_derived_certificates >/dev/null; then
      lane_revoke_derived_certificates
    fi
  fi
}
trap lane_cleanup EXIT

# A WRITE WHOSE STATUS IS NOT CHECKED IS A SILENT DROP. Every redirection that writes an
# ARTIFACT goes through here, so an unopenable destination is a LANE failure naming the
# knob and quoting the bytes back — never a bare `scripts/<lane>.sh: line N: ...: Is a
# directory` with no hint which knob supplied the path.
#
# Redirections into `LANE_TMP` are the exception and do not come here: that directory is
# created by this file and removed by its trap, so a failure there is not a knob error.
# The claim used to read "every redirection", which was false.
#
# `$1` is the destination, `$2` WHAT is being written, `$3` how to name WHERE it
# was going (the caller spells this so the knob appears exactly as an operator
# set it), and the rest is the command whose standard output becomes the file.
#
# The payload arrives as a COMMAND rather than on standard input on purpose: a
# pipeline would put this function on the right-hand side, where `die`'s `exit`
# leaves a subshell instead of the lane and the failure's propagation depends on
# `pipefail` staying set. Called as a plain command, `die` means what it says.
lane_write_checked() {
  local destination="$1" what="$2" where="$3"
  shift 3
  local write_error
  if ! write_error="$({ "$@" >"${destination}"; } 2>&1)"; then
    die "cannot write ${what} to ${where}
  ${write_error}
  The path is used exactly as given, byte for byte; nothing in it is expanded.
  Check that its parent directory exists and is writable."
  fi
}

# `mkdir -p` WITH ITS STATUS CHECKED AND THE KNOB NAMED. An unusable directory is
# a LANE failure quoting the bytes back, never a bare `mkdir: cannot create
# directory ...` with no hint which knob supplied the path. `$1` is the
# directory, `$2` names the knob and the bytes it carried.
lane_mkdir_checked() {
  local directory="$1" knob="$2" mkdir_error
  if ! mkdir_error="$(mkdir -p "${directory}" 2>&1)"; then
    die "cannot create '${directory}' under ${knob}
  ${mkdir_error}
  The path is used exactly as given, byte for byte; nothing in it is expanded.
  Check that its parent directory exists and is writable."
  fi
}

# EXISTING IS NOT BEING PRODUCED. An artifact a lane goes on to digest, load,
# query or report a size for must be a regular, NON-EMPTY file; a zero-byte one
# reports as a very fast engine and digests to the SHA-256 of the empty string.
#
# The SHA-256 of the empty string is deliberately NOT quoted in the message. It
# is the digest a lane must never publish, so printing it inside the diagnostic
# would put it in the lane's own output — an operator grepping a log for it
# would hit the very message saying it was refused. Described, never emitted.
lane_require_nonempty_file() {
  local path="$1" role="$2"
  [[ -f "${path}" ]] ||
    die "${role} was not produced at '${path}' even though the step before it reported success"
  [[ -s "${path}" ]] ||
    die "${role} at '${path}' is EMPTY.
  An empty artifact is a failure wearing a success's clothes: it loads cleanly,
  answers every query 0, and digests to the SHA-256 of the empty string. No
  digest and no size is published for it."
}

# ── Unsigned-integer knobs: validated and NORMALISED in one place ───────────────
#
# Validation alone is not enough, and that gap shipped: `require_uint` accepted
# `007`, so a lane printed `seed=007` in the summary beside a digest it advertised
# as reproducible from that seed, while the tool that consumed the value parsed it
# as 7 and recorded `seed 7` in the provenance file. One run, two published labels
# for one number, in the two artifacts whose whole job is attributing a number to
# its conditions -- and `007` and `7` produce byte-identical output, so a reader
# comparing the two reports concludes the digest is not seed-sensitive.
#
# Each lane had its own copy of these, and they had already drifted (one dropped
# the offending value from its message). They take the knob's NAME and the
# VARIABLE's name, so normalisation happens where validation does and no caller
# can have one without the other.
#
# THE NORMALISATION ITSELF WAS THE SECOND HALF OF THE SAME BUG. It read
# `$((10#${value}))`, which is base-10-explicit -- correct against the octal
# reading, and silent about the other way a digit string fails to survive
# arithmetic. Bash integers are SIGNED 64-BIT and wrap without a word:
#
#   9223372036854775808  -> -9223372036854775808   (a negative seed, from digits)
#   18446744073709551617 ->                    1
#   99999999999999999999 ->  7766279631452241919
#
# Every one of those passes `^[0-9]+$`, and the substituted value was then handed
# to the generator, printed in the SUMMARY as the requested run, and folded into
# the pin key -- the exact shape this helper exists to prevent, one layer down.
# `SCALE_QUADS=18446744073709551617` published a ONE-QUAD corpus as the run the
# operator asked for.
#
# So the range is checked BEFORE any arithmetic happens, and leading zeros are
# stripped as TEXT. The bound is bash's own representable maximum rather than a
# lane policy: a value this helper cannot carry to its caller intact must be
# refused by name, not quietly replaced by a different number.
lane_require_uint() {
  local name="$1"
  local -n _lane_uint_value="$2"
  [[ "${_lane_uint_value}" =~ ^[0-9]+$ ]] ||
    die "${name} must be a decimal unsigned integer (got '${_lane_uint_value}')"
  # The leading run of zeros, removed textually. `${value%%[!0]*}` is that run;
  # an all-zero value leaves nothing behind, which is the one case that needs a
  # floor.
  local stripped="${_lane_uint_value#"${_lane_uint_value%%[!0]*}"}"
  [[ -n "${stripped}" ]] || stripped="0"
  local max="9223372036854775807"
  if ((${#stripped} > ${#max})) ||
    { ((${#stripped} == ${#max})) && [[ "${stripped}" > "${max}" ]]; }; then
    die "${name} must be at most ${max} (got '${_lane_uint_value}').
  Bash arithmetic is signed 64-bit and wraps silently, so accepting this would
  substitute a different number -- possibly a negative one -- and then publish it
  as the value you asked for."
  fi
  _lane_uint_value="${stripped}"
}

lane_require_positive() {
  local name="$1"
  lane_require_uint "${name}" "$2"
  local -n _lane_positive_value="$2"
  ((_lane_positive_value > 0)) ||
    die "${name} must be positive (got '${_lane_positive_value}')"
}

# The SHA-256 of `$1`, streamed. Every lane needs this and each one had grown its
# own inline copy, which is how the chunk size, the file handling and eventually
# the meaning drift apart. Streamed rather than read whole because the artifacts
# these lanes digest are corpora, and the scale knobs above them have no ceiling:
# a digest step that is O(corpus) resident becomes a lane's memory peak exactly
# when an operator turns the interesting knob up.
lane_sha256_file() {
  local path="$1"
  [[ -f "${path}" ]] ||
    die "cannot digest '${path}': it does not exist"
  # PRESENT IS NOT READABLE, here too. Without this the lane emits a bare Python
  # PermissionError traceback with no lane name and no knob -- for a corpus that
  # may be over a gigabyte in an arena an operator has touched. That is the
  # misdiagnosis class `lane_require_query_file` was extended to stop, one helper
  # over.
  [[ -r "${path}" ]] ||
    die "cannot digest '${path}': it exists but this process cannot read it.
  Check the file's mode and owner; nothing about the corpus or the binary is at
  fault here."
  python3 -c '
import hashlib, sys
digest = hashlib.sha256()
with open(sys.argv[1], "rb") as handle:
    for chunk in iter(lambda: handle.read(int(sys.argv[2])), b""):
        digest.update(chunk)
print(digest.hexdigest())
' "${path}" "${LANE_STREAM_CHUNK_BYTES}"
}

# A DIAGNOSTIC MUST SURVIVE THE RECORD IT TRAVELS IN. A lane's result rows are
# tab-separated, so a multi-line engine message cannot be dropped into one
# verbatim -- and the answer to that was `head -1`, which silently discarded
# everything after the first line while the report told the reader the message
# was printed verbatim. Several of this engine's errors carry the part that
# matters on a later line: a witness, or the head and cell of a malformed list.
# Flatten, so the whole message survives in a field that can hold it.
lane_flatten_detail() {
  printf '%s' "$1" | tr '\t' ' ' |
    awk 'NF { lines[n++] = $0 } END { for (i = 0; i < n; i++) printf "%s%s", (i ? " | " : ""), lines[i] }'
}

# ── The query set: counted, certified, and re-checked while it is being read ────
#
# These three laws were written for one lane and left out of its sibling, which is
# the shape `make_bench_lanes.rs` exists to warn about. They live here so that
# cannot happen again, and each takes the things that genuinely differ -- the
# directory, the expected count, the knob that named the arena -- as parameters.

# The digest of an emitted query set, in name order. This is the reproducibility
# handle AND the tripwire for a concurrent run, so the name is folded in beside
# the bytes: two files that swapped contents must not digest the same.
#
# EVERY file in the directory is covered, not just the `.rq` ones. The index the
# result loop actually reads and the provenance record that says which candidate
# each substitution drew -- and out of how many -- were outside the certificate
# while the report pointed the reader at them for the explanation of every empty
# result. A change to what is RECORDED was invisible to the digest that certifies
# the run, and the index could be rewritten underneath the loop without tripping
# the concurrency check. Globbing the directory rather than a list of names also
# means a sidecar added later cannot quietly fall outside the certificate.
# A MANIFEST OF PER-FILE DIGESTS, never a concatenation of names and bytes.
#
# Streaming `name || NUL || content` into one hash is AMBIGUOUS, because nothing
# delimits the end of content from the start of the next name. A directory holding
# one file `a` whose content is `b\0c` streams exactly what a directory holding an
# empty `a` and a `b` containing `c` streams, so two different query sets share one
# digest and a substitution survives the tripwire that is supposed to catch it.
# Demonstrated, not theorised.
#
# Each file is therefore hashed independently, and what gets hashed is a manifest
# of fixed-width `<digest>  <name>` records in byte order. A digest is 64 hex
# characters and a record is one line, so no content can be mistaken for a name. A
# name carrying a newline or a NUL would break that representation, so it is
# refused rather than silently encoded — these names come from the lane's own
# generators, so one is an anomaly worth stopping for.
lane_query_set_digest() {
  local directory="$1" status=0 errors
  # `mktemp` rather than `LANE_TMP`: a digest is a certificate, and a certificate must
  # not depend on a directory whose failure it cannot report.
  errors="$(mktemp)" || die "could not create a temporary file to capture digest errors"
  # Outside `LANE_TMP` by choice, so the trap is told about it explicitly.
  lane_track_stray "${errors}"
  python3 -c '
import hashlib, pathlib, stat, sys

root = pathlib.Path(sys.argv[1])
# THE DIRECTORY VANISHING IS THE STATE THIS DIGEST EXISTS TO DIAGNOSE, so it must
# not be the one state that produces a traceback. A concurrent run deletes this
# directory at the top of its own instantiation step, and `iterdir()` then raises
# FileNotFoundError -- so the carefully written "another run is almost certainly
# using the same arena" message never printed in precisely the case it was
# written for. No apostrophes in here: this block is single-quoted to the shell.
if not root.is_dir():
    sys.exit(f"FAIL: {root} is not a directory; it was removed or replaced underneath this run")
records = []
for path in sorted(root.iterdir(), key=lambda p: p.name.encode("utf-8")):
    # A NON-REGULAR ENTRY IS REFUSED, not skipped. Filtering to `is_file()` made the
    # claim above false: a subdirectory queued alongside the queries had its contents
    # excluded from the certificate that is also the concurrency tripwire, silently.
    # Refusing keeps the certificate closed over what is actually there, and matches
    # the `-maxdepth 1` the count uses.
    # `is_file()` FOLLOWS SYMLINKS, so filtering on it accepted a link whose target
    # lives outside this directory -- and the certificate then moved when that outside
    # file changed, while `find -maxdepth 1 -type f` counted one fewer entry than the
    # manifest recorded. A certificate closed over a directory cannot depend on bytes
    # that are not in it. `lstat` asks about the entry itself.
    #
    # The certificate is deliberately WIDER than the query count, said here because an
    # earlier comment claimed the two matched. This covers every regular file in the
    # directory -- for LUBM, fourteen `.rq` files plus `regimes.tsv` and
    # `provenance.txt` -- while `lane_require_query_count` counts `*.rq` alone. That is
    # the intent: the index the result loop reads and the provenance record are part of
    # what a run IS, and leaving them outside the certificate was a finding. What the
    # two guards share is depth, not membership.
    if not stat.S_ISREG(path.lstat().st_mode):
        kind = "a symbolic link" if path.is_symlink() else "not a regular file"
        sys.exit(
            f"FAIL: {root} holds {path.name!r}, which is {kind}. A query set is a flat "
            "directory of regular files: anything else either cannot be certified, or "
            "would make this digest depend on bytes outside the directory it certifies."
        )
    if "\n" in path.name or "\x00" in path.name:
        sys.exit(
            f"FAIL: {path.name!r} carries a newline or a NUL, which a manifest record "
            "cannot represent unambiguously"
        )
    records.append(f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}")
if not records:
    sys.exit(
        f"FAIL: {root} holds no files, so there is no query set to certify. The digest of "
        "an empty manifest would certify a workload of no queries."
    )
print(hashlib.sha256(("\n".join(records) + "\n").encode("utf-8")).hexdigest())
' "${directory}" 2>"${errors}" || status=$?
  # EVERY DIAGNOSTIC NAMES THE LANE, including the ones raised from the embedded
  # Python: `sys.exit` there writes a bare `FAIL:` line, bash dies on the failed
  # substitution, and the refusal used to arrive with no lane name on it.
  #
  # The capture is guarded, because the first version of this was worse than what it
  # replaced. It redirected into `LANE_TMP` and then `cat`ted unconditionally, so a
  # scratch directory that could not be written made a VALID query set fail -- and
  # fail with an EMPTY message, since there was nothing to read. A certificate must
  # not acquire an unnamed precondition, and a helper must never refuse with nothing
  # to say. `lane_run_probe` already had this shape; this copied the redirect and not
  # the guard.
  if ((status != 0)); then
    local detail=""
    [[ -s "${errors}" ]] && detail="$(lane_flatten_detail "$(cat "${errors}")")"
    rm -f -- "${errors}"
    # THE FALLBACK'S OWN TEXT WAS WRONG TWICE OVER, and no test reached it, which
    # is how both survived. It carried the `'"'"'` idiom for a literal quote --
    # correct in a single-quoted context, pasted into a double-quoted one where a
    # quote is already literal, so it printed `'''${directory}'''`. And its advice
    # named a case that cannot arrive here: a missing interpreter writes "command
    # not found" INTO the capture, so `-s` is true and `detail` is used instead.
    #
    # What actually reaches this arm is a non-zero status with nothing written,
    # which is characteristically a killed interpreter. So the status is quoted --
    # it is the only evidence there is -- and the advice names that.
    if [[ -n "${detail}" ]]; then
      die "${detail}"
    fi
    die "could not digest the query set at '${directory}': the interpreter exited
  ${status} and wrote nothing. A non-zero status with no message is characteristic
  of a process killed by a signal rather than one that failed and explained itself
  -- check whether this host ran out of memory."
  fi
  rm -f -- "${errors}"
}

# CAPTURING A BINARY'S STDERR IS A LANE LAW, NOT A PER-LANE HABIT. Both lanes had
# their own copy of the same two lines, and both copies carried the same defect
# this file records one helper over: an unconditional `cat` of a `LANE_TMP` file.
#
# The consequence is worse in a query loop than in a digest, because the loop
# attributes blame per row. When `LANE_TMP` is unusable the REDIRECT fails, so the
# binary never runs at all; bash reports that on the lane's own stderr, `rc` is 1
# and the capture is empty -- and the row reads `CANNOT-EXECUTE ... the binary
# exited 1 without saying anything`, for every query, ending in a summary saying
# not one of them executed. A scratch failure published as a total failure of the
# binary under test, with a stray `cat:` line as the only hint.
#
# So the two halves are separated and both are shared. `lane_reset_capture` proves
# the path is writable BEFORE the redirect that depends on it, and names the
# scratch directory rather than the binary; `lane_capture_stderr` reads it only if
# there is something there.
lane_reset_capture() {
  local path="$1"
  : >"${path}" ||
    die "cannot create the stderr capture at '${path}'.
  That path is inside the scratch directory this run made for itself, so nothing
  about the binary, the corpus or any knob you set is at fault -- the scratch
  directory has become unusable. Check TMPDIR and the free space on it."
}

lane_capture_stderr() {
  local path="$1"
  # Nothing to read is the ordinary case: a binary that succeeded said nothing.
  [[ -s "${path}" ]] || return 0
  lane_flatten_detail "$(cat "${path}")"
}

# A GENERATOR REPORTING SUCCESS IS NOT A FULL QUERY SET. `$1` is the directory,
# `$2` the count the pinned workload is defined to have, `$3` the knob that named
# the arena. No digest is published for a set that is not the whole set: a partial
# one runs, reports fast, and is a different workload wearing the same name.
lane_require_query_count() {
  local directory="$1" expected="$2" knob="$3" actual
  actual=$(find "${directory}" -maxdepth 1 -type f -name '*.rq' | wc -l)
  ((actual == expected)) ||
    die "the instantiator reported success but wrote ${actual} .rq file(s) to
  '${directory}' (under ${knob}), not ${expected}. No query-set digest is published
  for a set that is not the ${expected} published queries."
}

# Re-derives the query set's digest and refuses if it moved. `$1` is the
# directory, `$2` the digest recorded when the set was written, `$3` when this
# check is happening, `$4` the knob that named the arena.
#
# The arena is derived from one knob, so two runs with default knobs share it and
# each begins by deleting this directory. A run whose queries changed mid-flight
# did not measure the workload it reports, and that is the most expensive kind of
# wrong number: it looks exactly like a right one.
lane_verify_query_set() {
  local directory="$1" expected="$2" when="$3" knob="$4" now
  now="$(lane_query_set_digest "${directory}")"
  [[ "${now}" == "${expected}" ]] && return 0
  die "the instantiated query set CHANGED ${when}.
  expected ${expected}
  found    ${now}
  Another run is almost certainly using the same arena (${knob}) and rewrote the
  queries underneath this one. Numbers from a run whose queries changed mid-flight
  are not numbers, so this run stops. Give each concurrent run its own arena."
}

# A MISSING QUERY FILE MUST NAME ITSELF. Reading one with a command substitution
# yields the empty string, and an empty query is a USAGE error from the engine --
# so the lane would print the engine's parse complaint as the diagnosis for what
# is actually the harness's own missing artifact, blaming the thing under test.
# `$1` is the path, `$2` the query's id, `$3` the knob that named the arena.
lane_require_query_file() {
  local path="$1" id="$2" knob="$3"
  [[ -f "${path}" ]] ||
    die "${id} is missing from '${path}' (under ${knob}) at the moment it was to be
  run. The engine is not at fault and has not been asked: a query file that is not
  there reads as the empty string, and the engine's complaint about an empty query
  would be reported here as though the corpus or the binary were wrong."
  # PRESENT IS NOT READABLE. A file that exists and has bytes but cannot be read
  # -- a mode an interrupted run left behind, a file owned by another operator in
  # a shared arena -- reaches `cat` under `set +e`, yields the empty string, and
  # the engine is asked to parse nothing. Its complaint about that would be
  # reported as the diagnosis, which is the same misdiagnosis a missing file used
  # to produce, arriving through a different door.
  [[ -r "${path}" ]] ||
    die "${id} at '${path}' (under ${knob}) exists but cannot be READ by this
  process. Reading it would yield the empty string, and the engine's complaint
  about an empty query would be reported here as though the binary or the corpus
  were at fault. Check the file's mode and owner."
  [[ -s "${path}" ]] ||
    die "${id} at '${path}' (under ${knob}) is EMPTY. An empty query is a usage
  error, and reporting the engine's complaint about it would blame the binary for
  an artifact this lane produced."
}

# NON-EMPTY IS NOT "IS WHAT IT CLAIMS TO BE". A CLI that writes eight bytes and
# exits 0 passes an emptiness test, and a lane that stamps that file as a
# reusable artifact hands every later run a certificate for something that is
# not the artifact at all. `$1` is the path, `$2` the role, `$3` the magic bytes
# the format begins with, `$4` the format's name.
lane_require_magic() {
  local path="$1" role="$2" magic="$3" format="$4" head
  lane_require_nonempty_file "${path}" "${role}"
  head="$(head -c "${#magic}" -- "${path}")" || die "cannot read ${role} at '${path}'"
  [[ "${head}" == "${magic}" ]] ||
    die "${role} at '${path}' is not ${format}: its first ${#magic} bytes are not the
  magic every file of that format begins with. It is $(wc -c <"${path}") byte(s) of
  something else, and nothing about it is certified, stamped or reported."
}

# The same law for the format that has no magic bytes. N-QUADS IS CHECKED BY ITS
# SHAPE: every row ends with a `.`, so a file whose FIRST row does not is not the
# serialization the lane says it produced, however many bytes it holds. `$1` is
# the path, `$2` the role.
#
# Reading one line is O(1) whatever the file's size, which is what makes this
# affordable on a corpus a lane may never be able to read twice.
lane_require_nquads() {
  local path="$1" role="$2" first
  lane_require_nonempty_file "${path}" "${role}"
  first="$(head -n 1 -- "${path}")" || die "cannot read ${role} at '${path}'"
  [[ "${first}" == *. ]] ||
    die "${role} at '${path}' is not N-Quads: its first row does not end in the '.'
  that every N-Quads row ends with. Nothing about it is certified, digested or
  reported.
  first row: ${first:0:120}"
}

# ── The executable every number in a lane's report is about ─────────────────────
#
# `[[ -x ]]` ALONE IS NOT A CHECK FOR AN EXECUTABLE: a DIRECTORY carries the
# execute bit, so a bare executability test accepted `<knob>=/tmp` and the lane
# went on to generate megabytes of input on that "binary"'s behalf. And the
# execute bit is not proof the binary RUNS: `/bin/false` carries it too, reached
# the first real use, and died there with a diagnostic that blamed the CORPUS
# for a fault that was in the knob — an operator following it would go hunting a
# parser bug that does not exist.
#
# `$1` is the knob's NAME, `$2` is 1 when the knob supplied the path and 0 when
# the lane built the binary itself, `$3` is the path, `$4` names what an unset
# knob would have built.
lane_require_executable() {
  local knob="$1" from_knob="$2" path="$3" built="$4"
  if ((from_knob == 1)); then
    [[ -e "${path}" ]] ||
      die "${knob}='${path}' does not exist
  The path is used exactly as given, byte for byte; nothing in it is expanded.
  Leave ${knob} unset to have this lane build ${built} itself."
    [[ -f "${path}" ]] ||
      die "${knob}='${path}' is not a regular file (a directory carries the execute
  bit too, so an executability test alone would have accepted it)"
    [[ -x "${path}" ]] || die "${knob}='${path}' is not executable"
  else
    [[ -n "${path}" && -x "${path}" ]] ||
      die "the release build produced no executable ${built} binary at '${path}' (set ${knob} to point at one)"
  fi
}

# PROVING A BINARY RUNS IS RUNNING IT. Runs `${@:3}` and requires exit 0,
# reporting whatever the binary itself said inside the lane's own message rather
# than leaking it as a bare line above one. `$1` is how to name the binary, `$2`
# completes the sentence "when asked ...". The probe's standard output is left
# in `LANE_PROBE_OUT`.
#
# This runs BEFORE the lane's first step, which is the whole point: a knob error
# is not worth a download, a JRE, and eight megabytes of generated input before
# it is noticed, and every one of those is a chance for the real fault to be
# mistaken for a problem with the corpus.
lane_run_probe() {
  local provenance="$1" asked="$2"
  shift 2
  local status=0
  LANE_PROBE_OUT="$("$@" 2>"${LANE_TMP}/probe.err")" || status=$?
  if ((status != 0)); then
    local detail=""
    if [[ -s "${LANE_TMP}/probe.err" ]]; then
      detail="$(sed 's/^/  /' "${LANE_TMP}/probe.err")
"
    fi
    die "${provenance} is not a working ${LANE_BINARY}: it exited ${status} when asked ${asked}
${detail}  This check runs before the lane's first step, so nothing has been fetched,
  generated, extracted, converted or loaded, and no failure downstream can be
  mistaken for a problem with the corpus."
  fi
}

# EXITING 0 IS NOT PRODUCING OUTPUT, and the difference is the whole of the
# empty-certificate bug: a binary that exits 0 and says nothing left the lane
# holding the EMPTY STRING where the specification belonged, and every digest it
# then published was the SHA-256 of that. `$1` is how to name the binary, `$2`
# completes "when asked ...", `$3` is what the lane would have gone on to
# certify on the strength of this silence.
lane_require_probe_said_something() {
  local provenance="$1" asked="$2" consequence="$3"
  [[ -n "${LANE_PROBE_OUT}" ]] ||
    die "${provenance} exited 0 when asked ${asked} but printed NOTHING.
  ${consequence}"
}
