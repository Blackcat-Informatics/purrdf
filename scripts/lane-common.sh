#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# The laws every benchmark lane in this repository shares, in ONE implementation.
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

lane_cleanup() {
  local status=$?
  rm -rf "${LANE_TMP}"
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

# A WRITE WHOSE STATUS IS NOT CHECKED IS A SILENT DROP. Every redirection a lane
# makes goes through here, so an unopenable destination is a LANE failure that
# names the knob and quotes the bytes back — never a bare `scripts/<lane>.sh:
# line N: ...: Is a directory` with no hint which knob supplied the path.
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
  python3 -c '
import hashlib, sys
digest = hashlib.sha256()
with open(sys.argv[1], "rb") as handle:
    for chunk in iter(lambda: handle.read(1 << 22), b""):
        digest.update(chunk)
print(digest.hexdigest())
' "${path}"
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
