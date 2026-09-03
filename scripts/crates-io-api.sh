# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
# shellcheck shell=bash
#
# The one crates.io read path the release scripts share.
#
# This file is SOURCED, never executed. `scripts/bootstrap-crates-io.sh` and
# `scripts/check-crates-io-records.sh` both ask crates.io the same three
# questions — does a crate RECORD exist, does a crate VERSION exist, and is the
# record locked to Trusted Publishing — and both used to carry their own curl
# loop with slightly different retry and error rules. There is now one.
#
# Every question is answered from the public, unauthenticated API:
#
#   GET /api/v1/crates/<name>            200 = record exists, 404 = no record
#   GET /api/v1/crates/<name>/<version>  200 = version exists, 404 = no version
#
# and the record body carries `crate.trustpub_only`, the per-crate setting
# crates.io added in 2025-11 ("When true, this crate can only be published via
# Trusted Publishing, not with API tokens" — the column comment in the crates.io
# schema). That flag is the only registry-visible evidence of which lane can
# publish a crate; there is no public API for the Trusted Publisher
# configurations themselves.
#
# Rules every caller inherits:
#
#   * The User-Agent is load bearing. crates.io answers 403 to a default curl
#     agent, and a 403 read carelessly looks like "missing". Only a literal 404
#     is ever "missing"; every other non-200 status is an error that stops the
#     caller rather than a verdict.
#   * 000 (no response), 429 (rate limited) and 5xx are retried three times
#     with a growing pause; anything else is reported at once.
#   * crates.io asks API clients for about one request per second; callers
#     sleep between crates, not this file, so a mocked run stays instant.
#
# Mocking, for the scripts' `--self-test` arms and for proving a refusal
# without a registry: set PURRDF_CRATES_IO_MOCK to a directory. A record query
# for `<name>` reads `<dir>/<name>`, a version query for `<name>/<version>`
# reads `<dir>/<name>@<version>`; the file's first line is the HTTP status and
# the rest is the body. A file that does not exist answers 404, so a fixture
# that forgets a dependency makes the script REFUSE (the safe direction) rather
# than proceed.

# Set by crates_io_get: the HTTP status and the path of a file holding the body.
# The body file is created HERE, at source time, not lazily inside crates_io_get:
# callers run the state helpers in command substitutions (subshells), so a path
# chosen there would never reach the parent, and crates_io_trustpub_only would
# read nothing. Callers remove it on exit.
CRATES_IO_STATUS=""
CRATES_IO_BODY="$(mktemp)"

# crates_io_user_agent <version-or-label>
crates_io_user_agent() {
  echo "purrdf-release/$1 (paudley@blackcatinformatics.ca)"
}

# crates_io_get <api-path-under-/api/v1/crates/> <user-agent>
#
# Populates CRATES_IO_STATUS and CRATES_IO_BODY. Never exits; the caller decides
# what a status means. Transient statuses are retried here.
crates_io_get() {
  local path="$1"
  local user_agent="$2"
  if [[ -n "${PURRDF_CRATES_IO_MOCK:-}" ]]; then
    local fixture="${PURRDF_CRATES_IO_MOCK}/${path/\//@}"
    if [[ -f "$fixture" ]]; then
      CRATES_IO_STATUS="$(head -n 1 "$fixture")"
      tail -n +2 "$fixture" > "${CRATES_IO_BODY}"
    else
      CRATES_IO_STATUS="404"
      : > "${CRATES_IO_BODY}"
    fi
    return 0
  fi
  local attempt
  for attempt in 1 2 3; do
    CRATES_IO_STATUS="$(curl -sS --max-time 30 -H "User-Agent: ${user_agent}" \
      -o "${CRATES_IO_BODY}" -w "%{http_code}" \
      "https://crates.io/api/v1/crates/${path}" 2>/dev/null || echo "000")"
    case "${CRATES_IO_STATUS}" in
      000 | 429 | 5??)
        if [[ "$attempt" -lt 3 ]]; then
          sleep $((attempt * 5))
          continue
        fi
        ;;
    esac
    break
  done
}

# crates_io_record_state <crate> <user-agent>
#
# Echoes "present", "missing", or "error: <detail>". After "present",
# CRATES_IO_BODY holds the record JSON (see crates_io_trustpub_only).
crates_io_record_state() {
  local crate="$1"
  crates_io_get "${crate}" "$2"
  case "${CRATES_IO_STATUS}" in
    200) echo "present" ;;
    404) echo "missing" ;;
    000 | 429 | 5??)
      echo "error: crates.io returned ${CRATES_IO_STATUS} for ${crate} after 3 attempts"
      ;;
    *)
      echo "error: unexpected crates.io status ${CRATES_IO_STATUS} for ${crate} ($(head -c 200 "${CRATES_IO_BODY}"))"
      ;;
  esac
}

# crates_io_version_state <crate> <version> <user-agent>
#
# Echoes "present", "missing", or "error: <detail>".
crates_io_version_state() {
  local crate="$1"
  local version="$2"
  crates_io_get "${crate}/${version}" "$3"
  case "${CRATES_IO_STATUS}" in
    200) echo "present" ;;
    404) echo "missing" ;;
    000 | 429 | 5??)
      echo "error: crates.io returned ${CRATES_IO_STATUS} for ${crate} ${version} after 3 attempts"
      ;;
    *)
      echo "error: unexpected crates.io status ${CRATES_IO_STATUS} for ${crate} ${version} ($(head -c 200 "${CRATES_IO_BODY}"))"
      ;;
  esac
}

# crates_io_trustpub_only
#
# Reads `crate.trustpub_only` out of the record body the last
# crates_io_record_state left in CRATES_IO_BODY. Echoes "true", "false", or
# "unknown" (field absent or body unparseable — an older API shape, or a mock
# fixture that gave a status and no body). Never guesses: "unknown" is reported
# as such by the callers, not folded into either verdict.
crates_io_trustpub_only() {
  python3 - "${CRATES_IO_BODY}" <<'PY'
import json
import sys

try:
    with open(sys.argv[1], encoding="utf-8") as handle:
        record = json.load(handle)
    value = record["crate"]["trustpub_only"]
except (OSError, ValueError, KeyError, TypeError):
    print("unknown")
else:
    print("true" if value is True else "false" if value is False else "unknown")
PY
}
