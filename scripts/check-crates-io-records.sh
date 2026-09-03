#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Preflight: every crate in the release set already has a crates.io RECORD,
# and every record is locked to the lane about to publish it.
#
# Why this exists. `cargo publish` cannot be undone. The release lane publishes
# the crates in dependency order, one at a time; without this check a release
# tag whose Nth crate has no crates.io record irreversibly publishes the N-1
# crates before it and only then fails. A brand-new crate can never be created
# by the OIDC lane at all — crates.io's publish handler answers a Trusted
# Publishing token with "Trusted Publishing tokens do not support creating new
# crates. Publish the crate manually, first" — so it needs a one-time token
# bootstrap (scripts/bootstrap-crates-io.sh, docs/RELEASE.md). This script
# converts that from a mid-publish blowup into a refusal taken before anything
# leaves the runner.
#
# It checks the RECORD (`/api/v1/crates/<name>`), not a version. The publish
# loop's own version check asks whether THIS version is already up, so it
# answers 404 for every crate on a normal release and can never notice that
# the crate itself does not exist.
#
# Three checks, each a failure on its own:
#
#   1. RECORDS. Every release crate has one, EXCEPT a crate the ledger names:
#      a ledgered crate with no record is "bootstrap pending" — permitted, and
#      reported as such — because the release lane handles it by the
#      interleave (scripts/publish-release-crates.sh skips it and stops cleanly
#      at its first dependent for the token step). A missing record the ledger
#      does NOT name refuses the run, naming the crate: it is a new,
#      undocumented bootstrap requirement.
#   2. LEDGER. `PURRDF_UNBOOTSTRAPPED_CRATES` (the committed ledger of crates
#      known to lack a record) agrees with the registry in BOTH directions when
#      run over the whole release set: the missing-but-unledgered case above,
#      and a ledgered crate that HAS a record — a stale entry; the ledger named
#      `purrdf-datalog` for a full cycle after that crate was published,
#      because nothing checked that direction. One tolerance, needed by the
#      interleave: a tag's tree is frozen while its release is in flight, so a
#      ledgered crate whose record exists AT THE RELEASE VERSION being
#      published (PURRDF_RELEASE_VERSION) was created by this release's own
#      token step and is reported, not refused. A record at any other version
#      is stale, as before.
#   3. LOCK. Every record carries `trustpub_only = true`: crates.io's per-crate
#      "Require trusted publishing" setting, the registry-side statement that
#      an API token cannot publish a new version (crates.io answers one with
#      "403 Forbidden: New versions of this crate can only be published using
#      Trusted Publishing"). It is the only registry-visible evidence of which
#      lane owns a crate; the Trusted Publisher configurations themselves have
#      no public API. A record that is NOT locked is a crate a leaked token
#      could publish — the posture docs/RELEASE.md promises is gone for it —
#      and it is also the step most easily forgotten after a bootstrap, so a
#      fresh record without the lock refuses here, naming the setting to flip.
#
# What this preflight CANNOT see, stated so nobody reads a green run as more
# than it is: whether a Trusted Publisher entry exists for a crate and points
# at this repository and workflow. The OIDC token the lane exchanges is scoped
# to the crates whose entries matched, and a crate with no entry fails AT its
# own `cargo publish` ("The provided access token is not valid for crate
# `<name>`") — after every crate ahead of it has been published. That is a
# loud stop with a partial, re-runnable publish, not a refusal; the lock
# check above is the closest a preflight can get, because the setting can only
# be enabled from the crate's settings page once an entry exists.
#
# Usage:
#   scripts/check-crates-io-records.sh                # the whole release set
#   scripts/check-crates-io-records.sh purrdf-core …  # an explicit list
#                                                     # (no ledger check)
#   scripts/check-crates-io-records.sh --self-test    # every refusal and the
#                                                     # green path, offline
#
# Environment:
#   PURRDF_RELEASE_VERSION      the version being released (`rust-v` prefix
#                               accepted); decides the stale-ledger tolerance
#                               above and goes in the User-Agent. Unset, no
#                               tolerance applies.
#   PURRDF_RELEASE_CRATES_FILE  an alternative release-set/ledger file
#                               (self-test only; default scripts/release-crates.sh)
#   PURRDF_CRATES_IO_MOCK       a mock registry directory (scripts/crates-io-api.sh)

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# shellcheck source=scripts/crates-io-api.sh
source "${repo}/scripts/crates-io-api.sh"

version="${PURRDF_RELEASE_VERSION:-}"
version="${version#rust-v}"
user_agent="$(crates_io_user_agent "${version:-preflight}")"
release_list="${PURRDF_RELEASE_CRATES_FILE:-${repo}/scripts/release-crates.sh}"

self_test() {
  local tmp
  tmp="$(mktemp -d)"
  # shellcheck disable=SC2064  # expand now: tmp is what to remove.
  trap "rm -rf '${tmp}'" EXIT
  local failures=0

  # shellcheck source=scripts/release-crates.sh
  source "${repo}/scripts/release-crates.sh"
  local -a set=("${PURRDF_RELEASE_CRATES[@]}")

  # ledger_file <name> <crate>... : the real release set with the ledger replaced.
  ledger_file() {
    local name="$1"
    shift
    {
      cat "${repo}/scripts/release-crates.sh"
      printf '\nPURRDF_UNBOOTSTRAPPED_CRATES=(%s)\n' "$*"
    } > "${tmp}/${name}.sh"
    echo "${tmp}/${name}.sh"
  }
  # locked <dir> <crate> <trustpub_only>
  locked() {
    printf '200\n{"crate":{"name":"%s","trustpub_only":%s}}\n' "$2" "$3" > "$1/$2"
  }
  # all_locked <dir> [except...]: every release crate present and locked.
  all_locked() {
    local dir="$1"
    shift
    local crate skip
    for crate in "${set[@]}"; do
      skip=false
      for except in "$@"; do [[ "$except" == "$crate" ]] && skip=true; done
      [[ "$skip" == "false" ]] && locked "$dir" "$crate" true
    done
    return 0
  }

  # arm <label> <expect: pass|refuse> <mock-dir> <ledger-file> <must-mention>...
  arm() {
    local label="$1" expect="$2" mock="$3" ledger="$4"
    shift 4
    local out status=0
    out="$(PURRDF_CRATES_IO_MOCK="${mock}" PURRDF_RELEASE_CRATES_FILE="${ledger}" \
      bash "${BASH_SOURCE[0]}" 2>&1)" || status=$?
    local ok=true
    if [[ "$expect" == "pass" && "$status" -ne 0 ]]; then ok=false; fi
    if [[ "$expect" == "refuse" && "$status" -eq 0 ]]; then ok=false; fi
    local needle
    for needle in "$@"; do
      if ! grep -qF -- "$needle" <<<"$out"; then
        ok=false
        echo "    output does not mention: ${needle}"
      fi
    done
    if [[ "$ok" == "true" ]]; then
      printf '  ok      %s (exit %s)\n' "$label" "$status"
    else
      printf '  FAILED  %s (exit %s, expected %s)\n' "$label" "$status" "$expect"
      while IFS= read -r line; do printf '    | %s\n' "$line"; done <<<"$out"
      failures=$((failures + 1))
    fi
  }

  local VERSION
  VERSION="$(cd "${repo}" && cargo metadata --no-deps --format-version 1 \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
  echo "check-crates-io-records.sh self-test (release set: ${#set[@]} crates; version ${VERSION})"
  local first="${set[0]}" last="${set[-1]}"
  local mock

  # 1. Green: every record present and locked, ledger empty.
  mock="${tmp}/green"; mkdir -p "$mock"; all_locked "$mock"
  arm "every record present and locked, empty ledger" pass \
    "$mock" "$(ledger_file green)" \
    "ledger   PURRDF_UNBOOTSTRAPPED_CRATES agrees with crates.io (0 unbootstrapped)" \
    "lock     every record is trustpub_only=true" \
    "All ${#set[@]} release crates have a crates.io record."

  # 2. A ledgered crate really is missing: permitted — "bootstrap pending" —
  #    and said so, with the interleave named.
  mock="${tmp}/missing"; mkdir -p "$mock"; all_locked "$mock" "$last"
  arm "ledgered crate missing (${last}): bootstrap pending, permitted" pass \
    "$mock" "$(ledger_file missing "$last")" \
    "PENDING  ${last} (no crates.io record; in PURRDF_UNBOOTSTRAPPED_CRATES — bootstrap pending)" \
    "ledger   PURRDF_UNBOOTSTRAPPED_CRATES agrees with crates.io (1 unbootstrapped)" \
    "bootstrap pending for: ${last}" \
    "scripts/bootstrap-crates-io.sh"

  # 2b. A ledgered crate whose record exists AT the release version: created
  #     by this release's token step — tolerated when the version is known...
  mock="${tmp}/created"; mkdir -p "$mock"; all_locked "$mock"
  printf '200\n{"version":{"crate":"%s","num":"%s"}}\n' "$last" "${VERSION}" > "${mock}/${last}@${VERSION}"
  local out status=0
  out="$(PURRDF_CRATES_IO_MOCK="${mock}" PURRDF_RELEASE_CRATES_FILE="$(ledger_file created "$last")" \
    PURRDF_RELEASE_VERSION="rust-v${VERSION}" bash "${BASH_SOURCE[0]}" 2>&1)" || status=$?
  if [[ "$status" -eq 0 ]] && grep -qF "created  ${last} (ledgered, but its ${VERSION} record exists: created by this release's token step" <<<"$out"; then
    printf '  ok      ledgered crate created at %s by this release: tolerated (exit 0)\n' "$VERSION"
  else
    printf '  FAILED  ledgered crate created at %s by this release (exit %s)\n' "$VERSION" "$status"
    while IFS= read -r line; do printf '    | %s\n' "$line"; done <<<"$out"
    failures=$((failures + 1))
  fi
  #     ...and stale without it (no version given: no tolerance).
  arm "ledgered crate with a record, no release version given: stale" refuse \
    "$mock" "$(ledger_file created2 "$last")" \
    "${last} is in PURRDF_UNBOOTSTRAPPED_CRATES but crates.io HAS a record for it (stale ledger entry)"

  # 3. A missing crate the ledger does not name: refuse on the ledger too.
  mock="${tmp}/unledgered"; mkdir -p "$mock"; all_locked "$mock" "$last"
  arm "missing crate absent from the ledger (${last})" refuse \
    "$mock" "$(ledger_file unledgered)" \
    "${last} has no crates.io record but is NOT in PURRDF_UNBOOTSTRAPPED_CRATES"

  # 4. A stale ledger entry: present crate still ledgered.
  mock="${tmp}/stale"; mkdir -p "$mock"; all_locked "$mock"
  arm "stale ledger entry (${first} has a record)" refuse \
    "$mock" "$(ledger_file stale "$first")" \
    "${first} is in PURRDF_UNBOOTSTRAPPED_CRATES but crates.io HAS a record for it (stale ledger entry)"

  # 5. A record that is NOT locked to Trusted Publishing: refuse, naming the
  #    crate and the setting — the green neighbour is arm 1.
  mock="${tmp}/unlocked"; mkdir -p "$mock"; all_locked "$mock" "$first"
  locked "$mock" "$first" false
  arm "record not locked (${first} trustpub_only=false)" refuse \
    "$mock" "$(ledger_file unlocked)" \
    "OPEN     ${first} (trustpub_only=false)" \
    "not locked to Trusted Publishing: ${first}" \
    "Require trusted publishing"

  # 6. A record whose lock state the API did not report: refuse the same way,
  #    never folded into a verdict.
  mock="${tmp}/unknown"; mkdir -p "$mock"; all_locked "$mock" "$first"
  printf '200\n{"crate":{"name":"%s"}}\n' "$first" > "${mock}/${first}"
  arm "record with no trustpub_only field (${first})" refuse \
    "$mock" "$(ledger_file unknown)" \
    "OPEN     ${first} (trustpub_only=unknown)"

  # 7. An explicit crate list: records and locks are checked, the ledger is not.
  mock="${tmp}/explicit"; mkdir -p "$mock"; all_locked "$mock"
  status=0
  out="$(PURRDF_CRATES_IO_MOCK="${mock}" bash "${BASH_SOURCE[0]}" "$first" "$last" 2>&1)" || status=$?
  if [[ "$status" -eq 0 ]] && grep -qF "All 2 release crates have a crates.io record." <<<"$out" \
    && ! grep -qF "ledger " <<<"$out"; then
    printf '  ok      explicit list of 2 crates (exit 0, no ledger check)\n'
  else
    printf '  FAILED  explicit list of 2 crates (exit %s)\n' "$status"
    while IFS= read -r line; do printf '    | %s\n' "$line"; done <<<"$out"
    failures=$((failures + 1))
  fi

  # 8. A registry fault is a stop, never a verdict.
  mock="${tmp}/fault"; mkdir -p "$mock"; all_locked "$mock" "$first"
  printf '503\n' > "${mock}/${first}"
  arm "registry answers 503 for ${first}" refuse \
    "$mock" "$(ledger_file fault)" \
    "ERROR    ${first}" "could not reach a verdict for: ${first}"

  if [[ "$failures" -gt 0 ]]; then
    echo "check-crates-io-records.sh self-test: ${failures} arm(s) FAILED" >&2
    return 1
  fi
  echo "check-crates-io-records.sh self-test: every refusal fires and the green path passes"
}

ledger=()
check_ledger=false
if [[ "$#" -gt 0 && "$1" == "--self-test" ]]; then
  self_test
  exit
elif [[ "$#" -gt 0 ]]; then
  crates=("$@")
else
  # shellcheck source=scripts/release-crates.sh
  source "${release_list}"
  crates=("${PURRDF_RELEASE_CRATES[@]}")
  ledger=("${PURRDF_UNBOOTSTRAPPED_CRATES[@]}")
  check_ledger=true
fi

trap 'rm -f "${CRATES_IO_BODY:-}"' EXIT

in_ledger() {
  local entry
  for entry in "${ledger[@]}"; do [[ "$entry" == "$1" ]] && return 0; done
  return 1
}

missing=()
present=()
errors=()
unlocked=()

for crate in "${crates[@]}"; do
  state="$(crates_io_record_state "$crate" "${user_agent}")"
  case "$state" in
    present)
      lock="$(crates_io_trustpub_only)"
      if [[ "$lock" == "true" ]]; then
        printf 'ok       %s (trustpub_only=true)\n' "$crate"
      else
        printf 'OPEN     %s (trustpub_only=%s)\n' "$crate" "$lock"
        unlocked+=("$crate")
      fi
      present+=("$crate")
      ;;
    missing)
      if [[ "$check_ledger" == "true" ]] && in_ledger "$crate"; then
        printf 'PENDING  %s (no crates.io record; in PURRDF_UNBOOTSTRAPPED_CRATES — bootstrap pending)\n' "$crate"
      else
        printf 'MISSING  %s\n' "$crate"
      fi
      missing+=("$crate")
      ;;
    *)
      printf 'ERROR    %s — %s\n' "$crate" "${state#error: }"
      errors+=("$crate")
      ;;
  esac
  # crates.io asks API clients for roughly one request per second.
  [[ -n "${PURRDF_CRATES_IO_MOCK:-}" ]] || sleep 1
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

# The ledger check, before the verdict: it must fail even on a run where every
# crate is present (a ledger still naming a bootstrapped crate) and even on a
# run that is going to refuse anyway (a missing crate the ledger does not name).
if [[ "$check_ledger" == "true" ]]; then
  ledger_problems=()
  for crate in "${missing[@]}"; do
    listed=false
    for entry in "${ledger[@]}"; do [[ "$entry" == "$crate" ]] && listed=true; done
    if [[ "$listed" == "false" ]]; then
      ledger_problems+=("${crate} has no crates.io record but is NOT in PURRDF_UNBOOTSTRAPPED_CRATES")
    fi
  done
  for entry in "${ledger[@]}"; do
    for crate in "${present[@]}"; do
      [[ "$entry" == "$crate" ]] || continue
      if [[ -n "$version" ]]; then
        state="$(crates_io_version_state "$crate" "$version" "${user_agent}")"
        case "$state" in
          present)
            printf 'created  %s (ledgered, but its %s record exists: created by this release'"'"'s token step; drop it from the ledger after the release)\n' "$crate" "$version"
            continue 2
            ;;
          missing) ;;
          *)
            echo "ERROR    ${crate} — ${state#error: }" >&2
            exit 1
            ;;
        esac
      fi
      ledger_problems+=("${entry} is in PURRDF_UNBOOTSTRAPPED_CRATES but crates.io HAS a record for it (stale ledger entry)")
    done
  done
  if [[ "${#ledger_problems[@]}" -gt 0 ]]; then
    {
      echo
      echo "PURRDF_UNBOOTSTRAPPED_CRATES in scripts/release-crates.sh disagrees with crates.io:"
      for problem in "${ledger_problems[@]}"; do echo "  - ${problem}"; done
      cat <<EOF

That array is the ledger docs/RELEASE.md's bootstrap section is derived from
(scripts/check-doc-claims.py holds the prose to it), so a wrong entry there is
wrong documentation about what a release tag will do. Fix the array to match
the registry — it is a record of crates.io state, not a wish — then re-run.
EOF
    } >&2
    exit 1
  fi
  printf 'ledger   PURRDF_UNBOOTSTRAPPED_CRATES agrees with crates.io (%d unbootstrapped)\n' "${#ledger[@]}"
fi

# The lock check, also before the verdict, for the same reason: a record that
# is open to token publishes is wrong whether or not this run publishes.
if [[ "${#unlocked[@]}" -gt 0 ]]; then
  cat >&2 <<EOF

crates.io records not locked to Trusted Publishing: ${unlocked[*]}

Every PurRDF crate record is published by the rust-v* lane through Trusted
Publishing and nothing else; crates.io enforces that with the per-crate
"Require trusted publishing" setting (trustpub_only), which makes a token
publish a 403. A record without it is one a leaked token could publish, and
this preflight cannot tell a deliberate relaxation from a bootstrap that
stopped one step early. On crates.io, open each crate's Settings page, add
the Trusted Publisher entry if it is not there yet (GitHub Actions /
Blackcat-Informatics / purrdf / release-cargo.yaml / no environment), enable
"Require trusted publishing", and re-run this tag.
EOF
  exit 1
fi
if [[ "${#present[@]}" -gt 0 ]]; then
  printf 'lock     every record is trustpub_only=true (%d)\n' "${#present[@]}"
fi

pending=()
unledgered=()
for crate in "${missing[@]}"; do
  if [[ "$check_ledger" == "true" ]] && in_ledger "$crate"; then
    pending+=("$crate")
  else
    unledgered+=("$crate")
  fi
done

if [[ "${#unledgered[@]}" -gt 0 ]]; then
  cat >&2 <<EOF

crates.io has no crate record for: ${unledgered[*]}

Refusing to publish. This lane publishes ${#crates[@]} crates one at a time in
dependency order and cargo publish CANNOT be undone; a crate with no record
that the ledger does not name is an undocumented bootstrap requirement, and
the publish loop would only skip it if PURRDF_UNBOOTSTRAPPED_CRATES named it.

crates.io refuses to create a crate from a Trusted Publishing token ("Trusted
Publishing tokens do not support creating new crates"), so this workflow cannot
create the record; creating one is the ONE thing an API token still does for
this workspace (it cannot publish a new version of any existing crate — they
are all locked). Add the crate to PURRDF_UNBOOTSTRAPPED_CRATES in
scripts/release-crates.sh and to the "Outstanding bootstrap" section of
docs/RELEASE.md, re-tag, and follow the interleave that section describes:
the lane publishes up to the crate's first dependent and stops, then

    scripts/bootstrap-crates-io.sh --plan          # the preflight and plan, no token
    CARGO_REGISTRY_TOKEN="\${CARGO_TOKEN}" scripts/bootstrap-crates-io.sh

creates the record, you add its Trusted Publisher entry and enable "Require
trusted publishing", and the workflow run is re-run.
EOF
  exit 1
fi

if [[ "${#pending[@]}" -gt 0 ]]; then
  cat <<EOF

bootstrap pending for: ${pending[*]}

Each is in PURRDF_UNBOOTSTRAPPED_CRATES and has no crates.io record. The publish
loop (scripts/publish-release-crates.sh) skips each of them, publishes every
crate that does not depend on one, and STOPS cleanly at the first crate that
does — naming the token step, scripts/bootstrap-crates-io.sh, as what comes
next. The set is not complete until every one has been created and this
workflow run has been re-run (docs/RELEASE.md, "Outstanding bootstrap").
EOF
  exit 0
fi

printf '\nAll %d release crates have a crates.io record.\n' "${#crates[@]}"
