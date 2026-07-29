#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Preflight: every crate in the release set already has a crates.io RECORD.
#
# Why this exists. `cargo publish` cannot be undone. The release lane publishes
# the crates in dependency order, one at a time; without this check a release
# tag whose Nth crate has no crates.io record irreversibly publishes the N-1
# crates before it and only then fails. crates.io additionally requires a crate
# to exist before a Trusted Publisher can be configured for it, so a brand-new
# crate can never be created by the OIDC lane at all — it needs a one-time token
# bootstrap (docs/RELEASE.md). This script converts that from a mid-publish
# blowup into a refusal taken before anything leaves the runner.
#
# It checks the RECORD (`/api/v1/crates/<name>`), not a version. The publish
# loop's own `crate_version_exists` asks whether THIS version is already up, so
# it answers 404 for every crate on a normal release and can never notice that
# the crate itself does not exist.
#
# Usage:
#   scripts/check-crates-io-records.sh                # the whole release set
#   scripts/check-crates-io-records.sh purrdf-core …  # an explicit list
#
# Environment:
#   PURRDF_RELEASE_VERSION  version string put in the User-Agent (cosmetic).

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

version="${PURRDF_RELEASE_VERSION:-preflight}"
user_agent="purrdf-release/${version} (paudley@blackcatinformatics.ca)"

if [[ "$#" -gt 0 ]]; then
  crates=("$@")
else
  # shellcheck source=scripts/release-crates.sh
  source "${repo}/scripts/release-crates.sh"
  crates=("${PURRDF_RELEASE_CRATES[@]}")
fi

body="$(mktemp)"
trap 'rm -f "$body"' EXIT

# Query one crate record. Echoes "present", "missing", or "error: <detail>".
#
# crates.io answers 403 to a default curl User-Agent, so the UA above is load
# bearing: without it EVERY crate would answer non-200 and, read carelessly,
# would look missing. Only a literal 404 is missing here; anything else is an
# error that stops the run rather than a verdict.
crate_record_state() {
  local crate="$1"
  local status attempt
  for attempt in 1 2 3; do
    status="$(curl -sS --max-time 30 -H "User-Agent: ${user_agent}" \
      -o "$body" -w "%{http_code}" \
      "https://crates.io/api/v1/crates/${crate}" 2>/dev/null || echo "000")"
    case "$status" in
      200)
        echo "present"
        return 0
        ;;
      404)
        echo "missing"
        return 0
        ;;
      000 | 429 | 5??)
        # Transient: no response, rate limited, or a registry-side fault.
        if [[ "$attempt" -lt 3 ]]; then
          sleep $((attempt * 5))
          continue
        fi
        echo "error: crates.io returned ${status} for ${crate} after 3 attempts"
        return 0
        ;;
      *)
        echo "error: unexpected crates.io status ${status} for ${crate} ($(head -c 200 "$body"))"
        return 0
        ;;
    esac
  done
}

missing=()
errors=()

for crate in "${crates[@]}"; do
  state="$(crate_record_state "$crate")"
  case "$state" in
    present) printf 'ok       %s\n' "$crate" ;;
    missing)
      printf 'MISSING  %s\n' "$crate"
      missing+=("$crate")
      ;;
    *)
      printf 'ERROR    %s — %s\n' "$crate" "${state#error: }"
      errors+=("$crate")
      ;;
  esac
  # crates.io asks API clients for roughly one request per second.
  sleep 1
done

if [[ "${#errors[@]}" -gt 0 ]]; then
  cat >&2 <<EOF

crates.io record preflight could not reach a verdict for: ${errors[*]}

Refusing to continue. A publish run that cannot confirm the release set exists
must not start: cargo publish is irreversible, so an unknown answer is treated
as a stop, never as "probably fine".
EOF
  exit 1
fi

if [[ "${#missing[@]}" -gt 0 ]]; then
  cat >&2 <<EOF

crates.io has no crate record for: ${missing[*]}

Refusing to publish. This lane publishes ${#crates[@]} crates one at a time in
dependency order and cargo publish CANNOT be undone, so continuing would
irreversibly publish every crate ahead of the missing one and then fail.

crates.io also requires a crate to exist before a Trusted Publisher can be
configured for it, so this workflow's OIDC token cannot create these records.
The bootstrap is a one-time token publish from a clean local checkout
(docs/RELEASE.md, "Trusted Publisher Setup"):

    CARGO_REGISTRY_TOKEN="\${CARGO_TOKEN}" scripts/bootstrap-crates-io.sh <version>

Then add the Trusted Publisher entry for each newly created crate
(GitHub Actions / Blackcat-Informatics / purrdf / release-cargo.yaml / no
environment) and re-run this tag.
EOF
  exit 1
fi

printf '\nAll %d release crates have a crates.io record.\n' "${#crates[@]}"
