#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Create the crates.io RECORDS the release lane cannot create — and nothing else.
#
# What a crates.io API token can do for this workspace, and what it cannot:
#
#   * It CAN create a crate record: the first-ever publish of a brand-new crate
#     name. crates.io's own publish handler refuses that from the OIDC lane —
#     "Trusted Publishing tokens do not support creating new crates. Publish
#     the crate manually, first" — so a new crate always needs one token
#     publish before a `rust-v*` tag can carry it.
#   * It CANNOT publish a new version of any existing PurRDF crate. Every
#     existing record is locked to Trusted Publishing (`trustpub_only`, visible
#     in the public record JSON), and crates.io answers a token publish with:
#
#       error: failed to publish purrdf-events v0.13.0 to registry at https://crates.io
#       Caused by:
#         the remote server responded with an error (status 403 Forbidden):
#         New versions of this crate can only be published using Trusted
#         Publishing (see https://crates.io/docs/trusted-publishing).
#
#     That is the verbatim answer the previous version of this script met on
#     its FIRST upload, at the front of a 21-crate loop. It failed safe (nothing
#     landed), but a token lane that walks into eighteen 403s is wrong, not
#     strict.
#
# So this script iterates PURRDF_UNBOOTSTRAPPED_CRATES — the committed ledger
# of release crates with no record — and REFUSES to touch any crate that has
# one. Every later version of every crate, the three new ones included, goes
# through the tag-driven Trusted Publishing lane (.github/workflows/release-cargo.yaml).
#
# The second refusal, and why it is here. `cargo publish` — with OR without
# `--no-verify` — resolves the packaged crate's dependency graph against the
# registry to write its lockfile, so a ledger crate whose path-dependencies are
# not on crates.io at the target version cannot be uploaded at all:
#
#       error: failed to prepare local package for uploading
#       Caused by:
#         failed to select a version for the requirement `purrdf-iri = "^0.13.0"`
#         candidate versions found which didn't match: 0.12.0, 0.11.0, 0.10.0, ...
#
# (`cargo publish -p purrdf-cdt --no-verify --dry-run` at 0.13.0, verbatim.)
# `--no-verify` skips the BUILD, not the resolve. Those dependencies reach
# crates.io through the trusted lane, which publishes up to the first crate
# that depends on a ledger crate and STOPS (scripts/publish-release-crates.sh)
# — that is the token step's cue. So this script creates, in one pass, every
# ledger crate whose dependencies are on crates.io at the target version,
# VERIFIED (the dependencies exist, so nothing needs `--no-verify`), DEFERS
# every ledger crate whose dependencies are not up yet — naming them and the
# trusted-lane run that will publish them — and refuses outright only when it
# can create nothing. That is the interleave in docs/RELEASE.md, "Outstanding
# bootstrap".
#
# Usage:
#   scripts/bootstrap-crates-io.sh [VERSION]          create the ledger records
#   scripts/bootstrap-crates-io.sh [VERSION] --plan   the preflight and plan only:
#                                                     no token, no gates, no publish
#   scripts/bootstrap-crates-io.sh --self-test        every refusal, and the
#                                                     valid path, against a mock
#                                                     registry (offline)
#
# VERSION defaults to the workspace version. Environment:
#   CARGO_REGISTRY_TOKEN / CARGO_TOKEN   required for a publish run
#   ALLOW_DIRTY=true                     publish from a dirty tree (default: refuse)
#   PUBLISH_NO_VERIFY=true               skip `cargo publish`'s verification build
#   PUBLISH_COOLDOWN_SECONDS=N           pause after each created record (default 0)
#   PURRDF_RELEASE_CRATES_FILE           an alternative release-set/ledger file
#                                        (self-test only; default scripts/release-crates.sh)
#   PURRDF_CRATES_IO_MOCK                a mock registry directory (scripts/crates-io-api.sh)

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

mode="publish"
VERSION=""
for arg in "$@"; do
  case "$arg" in
    --plan) mode="plan" ;;
    --self-test) mode="self-test" ;;
    -h | --help)
      sed -n '/^# Usage:/,/^$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    -*)
      echo "unknown option: ${arg}" >&2
      exit 2
      ;;
    *)
      if [[ -n "${VERSION}" ]]; then
        echo "more than one version given: ${VERSION} and ${arg}" >&2
        exit 2
      fi
      VERSION="$arg"
      ;;
  esac
done

# `cargo metadata --no-deps`, read once. The workspace version comes from it,
# and so does every ledger crate's path-dependency list (below).
metadata_json="$(mktemp)"
trap 'rm -f "${metadata_json}" "${CRATES_IO_BODY:-}"' EXIT
(cd "${repo}" && cargo metadata --no-deps --format-version 1) > "${metadata_json}"
if [[ -z "${VERSION}" ]]; then
  VERSION="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["packages"][0]["version"])' "${metadata_json}")"
fi

# One definition of the release set AND of the ledger, shared with the release
# workflow and with scripts/check-crates-io-records.sh. The release set is read
# only to place ledger crates in publish order; this script walks the LEDGER.
release_list="${PURRDF_RELEASE_CRATES_FILE:-${repo}/scripts/release-crates.sh}"
# shellcheck source=scripts/release-crates.sh
source "${release_list}"
# shellcheck source=scripts/crates-io-api.sh
source "${repo}/scripts/crates-io-api.sh"
crates=("${PURRDF_UNBOOTSTRAPPED_CRATES[@]}")
release_set=("${PURRDF_RELEASE_CRATES[@]}")
# `preflight`'s decision, readable by every step that follows it. Declared here so
# the plan is visibly a VALUE the script carries rather than something recomputed
# — the previous shape recomputed nothing and simply re-walked `crates`, which is
# how a DEFERRED crate reached `cargo package`.
PLAN_TO_CREATE=()
PLAN_DEFERRED=()

# plan_cargo_args: fill `cargo_args` with the `-p` list every per-crate step gets.
#
# ONE construction, used by the gates, `cargo package`, and `--plan`'s printout —
# so what `--plan` shows is the array the irreversible step is handed, not a
# description of it. That identity is the point: the 179-case self-test passed a
# script that packaged the whole LEDGER because no arm ever looked at these
# arguments, only at the plan text above them.
cargo_args=()
plan_cargo_args() {
  local crate
  cargo_args=()
  for crate in "${PLAN_TO_CREATE[@]}"; do
    cargo_args+=("-p" "$crate")
  done
}
user_agent="$(crates_io_user_agent "${VERSION}")"

# Pause after publishing a crate that CREATED a new crates.io record, before
# the next publish. Default 0: crates.io's new-crate rate limit is enforced AT
# the publish — a limited `cargo publish` exits non-zero on the registry's 429,
# `set -e` stops this script before the next crate, nothing is half-uploaded,
# and a re-run resumes because every record already created is skipped. The
# old default of 620 s modelled the limit's ten-minute refill unconditionally
# and added ~31 minutes of dead time to a three-record run. Set it only if a
# run actually meets a 429 with records still to create.
PUBLISH_COOLDOWN_SECONDS="${PUBLISH_COOLDOWN_SECONDS:-0}"
# Set PUBLISH_NO_VERIFY=true to skip `cargo publish`'s verification build. See
# the comment above the publish loop for why the default is to verify — and
# the header for why the flag does NOT get a crate past missing dependencies.
PUBLISH_NO_VERIFY="${PUBLISH_NO_VERIFY:-false}"

# ---------------------------------------------------------------------------
# Registry questions. Any answer other than present/missing is a hard stop —
# never read as "missing", because "missing" is what licenses a record CREATE.
# ---------------------------------------------------------------------------

# registry_or_die <state>: present/missing pass through; anything else stops.
# Called in the PARENT shell after every capture, never inside the captured
# function: an `exit` inside `$(...)` only ends the subshell, and the parent
# would read the empty answer as "not present" — the one misreading that
# licenses a record CREATE.
registry_or_die() {
  local state="$1"
  case "$state" in
    present | missing) ;;
    *)
      echo "crates.io preflight could not reach a verdict — ${state#error: }" >&2
      echo "Refusing to continue: an unknown registry answer is a stop, never \"probably fine\"." >&2
      exit 1
      ;;
  esac
}

record_state() {
  crates_io_record_state "$1" "${user_agent}"
}

version_state() {
  crates_io_version_state "$1" "${VERSION}" "${user_agent}"
}

# workspace_path_deps <crate>: one "<kind> <name>" line per PATH dependency of
# the crate, every kind — `cargo publish` resolves the packaged crate's whole
# graph, dev-dependencies included, to write its lockfile, so every kind must
# exist on the registry at the target version.
workspace_path_deps() {
  python3 - "${metadata_json}" "$1" <<'PY'
import json
import sys

metadata = json.load(open(sys.argv[1], encoding="utf-8"))
for package in metadata["packages"]:
    if package["name"] != sys.argv[2]:
        continue
    for dep in package["dependencies"]:
        if dep.get("path"):
            print(dep["kind"] or "normal", dep["name"])
PY
}

in_list() {
  local needle="$1"
  shift
  local item
  for item in "$@"; do
    [[ "$item" == "$needle" ]] && return 0
  done
  return 1
}

# ---------------------------------------------------------------------------
# Preflight + plan. Runs before the token check, before the gates, before
# anything irreversible — and is the whole of `--plan`.
# ---------------------------------------------------------------------------

preflight() {
  if [[ "${#crates[@]}" -eq 0 ]]; then
    cat >&2 <<EOF
PURRDF_UNBOOTSTRAPPED_CRATES in ${release_list} is empty: nothing to bootstrap.

Every crate in the release set has a crates.io record, and a token cannot
publish a new version of an existing record (crates.io: "New versions of this
crate can only be published using Trusted Publishing"). Push a rust-v* tag.
EOF
    return 1
  fi

  # to_create ESCAPES this function (declared at file scope, below `preflight`'s
  # definition site is too late — see PLAN_TO_CREATE). Everything after the plan
  # must iterate the PLAN, not the ledger: a DEFERRED crate's dependencies are by
  # definition not on the registry, so `cargo package` dies on it before the first
  # upload. Observed verbatim at token step 1 of the 0.13.0 bootstrap.
  local -a to_create=() to_skip=() has_record=() deferred=() not_in_set=()
  local crate idx dep_line kind dep dep_state state

  echo "crates.io bootstrap plan for ${VERSION} (ledger: ${crates[*]}):"
  for idx in "${!crates[@]}"; do
    crate="${crates[$idx]}"
    if ! in_list "$crate" "${release_set[@]}"; then
      printf '  REFUSE         %s (in the ledger but not in the release set)\n' "$crate"
      not_in_set+=("$crate")
      continue
    fi
    state="$(record_state "$crate")"
    registry_or_die "$state"
    if [[ "$state" == "present" ]]; then
      local locked
      locked="$(crates_io_trustpub_only)"
      state="$(version_state "$crate")"
      registry_or_die "$state"
      if [[ "$state" == "present" ]]; then
        printf '  skip           %s (%s already on crates.io: created by an earlier run — remove it from PURRDF_UNBOOTSTRAPPED_CRATES)\n' "$crate" "${VERSION}"
        to_skip+=("$crate")
      else
        printf '  REFUSE         %s (crate record exists, trustpub_only=%s)\n' "$crate" "$locked"
        has_record+=("${crate} (trustpub_only=${locked})")
      fi
      continue
    fi
    local -a deps_ok=() deps_missing=()
    while IFS= read -r dep_line; do
      [[ -z "$dep_line" ]] && continue
      kind="${dep_line%% *}"
      dep="${dep_line#* }"
      # A dependency that is itself an EARLIER ledger entry and is being
      # created (or already exists) in this pass is satisfied; one that is
      # deferred is not — and a LATER one is an ordering bug
      # check-publish-order.py refuses, reported as missing here too.
      if in_list "$dep" "${crates[@]:0:$idx}" && in_list "$dep" "${to_create[@]}" "${to_skip[@]}"; then
        deps_ok+=("${dep} (${kind}; created earlier in this pass)")
        continue
      fi
      dep_state="$(version_state "$dep")"
      registry_or_die "$dep_state"
      if [[ "$dep_state" == "present" ]]; then
        deps_ok+=("${dep} ${VERSION} (${kind})")
      else
        deps_missing+=("${dep} ${VERSION} (${kind})")
      fi
    done < <(workspace_path_deps "$crate")
    if [[ "${#deps_missing[@]}" -gt 0 ]]; then
      printf '  DEFER          %s (no record; its dependencies are not on crates.io yet: %s — the next trusted-lane run publishes them)\n' "$crate" "$(IFS=,; echo "${deps_missing[*]}")"
      deferred+=("${crate} waits on $(IFS=,; echo "${deps_missing[*]}")")
    else
      printf '  CREATE RECORD  %s (no crates.io record; dependencies on crates.io: %s)\n' "$crate" "$(IFS=,; echo "${deps_ok[*]:-none}")"
      to_create+=("$crate")
    fi
  done

  local failed=false
  if [[ "${#not_in_set[@]}" -gt 0 ]]; then
    failed=true
    {
      echo
      echo "REFUSING: these ledger entries are not release crates: ${not_in_set[*]}"
      echo "PURRDF_UNBOOTSTRAPPED_CRATES must name crates in PURRDF_RELEASE_CRATES (scripts/check-publish-order.py gates this offline)."
    } >&2
  fi
  if [[ "${#has_record[@]}" -gt 0 ]]; then
    failed=true
    {
      echo
      echo "REFUSING: these ledger crates already have a crates.io record:"
      for crate in "${has_record[@]}"; do echo "  - ${crate}"; done
      cat <<'EOF'

A token cannot publish a new version of an existing PurRDF crate. Every
existing record is locked to Trusted Publishing, and crates.io answers a token
publish with (status 403 Forbidden):
  New versions of this crate can only be published using Trusted Publishing
— the answer the 21-crate version of this script met on its first upload.
New versions go through the rust-v* tag lane.

PURRDF_UNBOOTSTRAPPED_CRATES in scripts/release-crates.sh is a ledger of
crates WITHOUT a record; scripts/check-crates-io-records.sh holds it to the
registry in both directions. Fix the ledger, never the registry.
EOF
    } >&2
  fi
  if [[ "${#deferred[@]}" -gt 0 && "${#to_create[@]}" -eq 0 && "${#to_skip[@]}" -eq 0 && "$failed" == "false" ]]; then
    failed=true
    {
      echo
      echo "REFUSING: nothing can be created yet — every ledger crate depends on versions crates.io does not have:"
      for dep in "${deferred[@]}"; do echo "  - ${dep}"; done
      cat <<EOF

\`cargo publish\` resolves the packaged crate's dependency graph against the
registry to write its lockfile — with or without --no-verify — so each crate
above would fail with "failed to select a version for the requirement" before
anything is uploaded. Those dependencies are published by the trusted lane:
push (or re-run) the rust-v${VERSION} tag first. Its publish loop
(scripts/publish-release-crates.sh) publishes everything up to the first crate
that depends on a ledger crate, then STOPS and names this script as the next
step; run it again then. docs/RELEASE.md, "Outstanding bootstrap".
EOF
    } >&2
  fi
  if [[ "$failed" == "true" ]]; then
    return 1
  fi

  # THE PLAN, published to the steps that act on it. Everything after this point
  # — the wasm32 check, the test run, `cargo package` and the publish loop —
  # iterates PLAN_TO_CREATE, never `crates`. The two differ exactly when a crate
  # is DEFERRED, which is the case that broke: `cargo package -p purrdf-geo` at
  # 0.13.0 died with "failed to select a version for the requirement
  # `purrdf-core = ^0.13.0`" because geo was deferred and its dependency was not
  # on the registry yet — before a single record had been created.
  PLAN_TO_CREATE=("${to_create[@]}")
  PLAN_DEFERRED=("${deferred[@]}")

  echo
  if [[ "${#to_create[@]}" -eq 0 ]]; then
    echo "Nothing to create at ${VERSION} in this pass: ${to_skip[*]} already exist${deferred[0]:+; deferred: ${#deferred[@]} (see above)}."
    echo "Next: re-run the rust-v${VERSION} workflow run (gh run rerun <run-id>); it resumes where it stopped."
    return 0
  fi
  cat <<EOF
${#to_create[@]} crate record(s) will be CREATED by this token in this pass: ${to_create[*]}
${deferred[0]:+deferred to a later pass (dependencies not up yet): ${#deferred[@]} — see DEFER above
}Creating a record is permanent. Afterwards, on crates.io, for EACH new crate:
add the Trusted Publisher entry (GitHub Actions / Blackcat-Informatics /
purrdf / release-cargo.yaml / no environment) and enable "Require trusted
publishing" so the record is locked like its siblings — this release does not
touch the crate again, but the next release's run would 403 at it without the
entry, and the preflight refuses it without the lock. Then re-run the rust-v${VERSION}
workflow run (gh run rerun <run-id>): it resumes at the crate it stopped on.
EOF
}

# ---------------------------------------------------------------------------
# Self-test: every refusal above, and the valid path, against a mock registry.
# ---------------------------------------------------------------------------

self_test() {
  local tmp
  tmp="$(mktemp -d)"
  # shellcheck disable=SC2064  # expand now: tmp is what to remove.
  trap "rm -rf '${tmp}'; rm -f '${metadata_json}'" EXIT
  local failures=0

  # The fixture ledger: the real one when it is non-empty, else the last two
  # release crates, so the arms below still exercise dependency handling after
  # the real ledger has been drained.
  local -a fixture
  if [[ "${#crates[@]}" -gt 0 ]]; then
    fixture=("${crates[@]}")
  else
    fixture=("${release_set[@]: -2}")
  fi

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

  # record <dir> <crate> <trustpub_only>; version <dir> <crate>
  record() {
    printf '200\n{"crate":{"name":"%s","trustpub_only":%s}}\n' "$2" "$3" > "$1/$2"
  }
  version() {
    printf '200\n{"version":{"crate":"%s","num":"%s"}}\n' "$2" "${VERSION}" > "$1/$2@${VERSION}"
  }
  # deps_present <dir>: every path-dependency of every fixture crate, at VERSION —
  # EXCEPT a dependency that is itself a fixture crate.
  #
  # A ledger crate is BY DEFINITION not on the registry, so minting a version
  # record for one contradicts the fixture. It also silently broke arm 5 the day
  # the real ledger emptied: the fallback fixture became the last two release
  # crates, `purrdf-wasm` depends on `purrdf`, and so `deps_present` wrote
  # `purrdf@VERSION` for the very crate that arm needs seen as "record exists, but
  # not at this version" — flipping the plan from REFUSE to `skip` and the arm's
  # exit code from 1 to 0. The plan already handles a ledger crate depending on an
  # earlier one through its in-pass check ("created earlier in this pass"), which
  # is the real behaviour this should be exercising; arm 2b encodes the same
  # exclusion for its own expectations.
  deps_present() {
    local crate dep_line dep
    for crate in "${fixture[@]}"; do
      while IFS= read -r dep_line; do
        [[ -z "$dep_line" ]] && continue
        dep="${dep_line#* }"
        in_list "$dep" "${fixture[@]}" && continue
        version "$1" "$dep"
      done < <(workspace_path_deps "$crate")
    done
  }

  # arm <label> <expect: pass|refuse> <mock-dir> <ledger-file> <must-mention>...
  arm() {
    local label="$1" expect="$2" mock="$3" ledger="$4"
    shift 4
    local out status=0
    out="$(PURRDF_CRATES_IO_MOCK="${mock}" PURRDF_RELEASE_CRATES_FILE="${ledger}" \
      bash "${BASH_SOURCE[0]}" "${VERSION}" --plan 2>&1)" || status=$?
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

  echo "bootstrap-crates-io.sh self-test (fixture ledger: ${fixture[*]}; version ${VERSION})"
  local existing="${release_set[0]}"
  local mock

  # 1. A ledger naming a crate that HAS a record: refuse, naming it.
  mock="${tmp}/existing"; mkdir -p "$mock"
  record "$mock" "$existing" true
  arm "ledger names an existing crate (${existing}, trustpub_only=true)" refuse \
    "$mock" "$(ledger_file existing "$existing")" \
    "REFUSE         ${existing} (crate record exists, trustpub_only=true)" \
    "New versions of this crate can only be published using Trusted Publishing"

  # 2. The ledger with every dependency absent at VERSION: refuse, naming
  #    every missing dependency of every ledger crate.
  mock="${tmp}/deps-absent"; mkdir -p "$mock"
  local -a expect_missing=()
  local crate dep_line dep
  for crate in "${fixture[@]}"; do
    while IFS= read -r dep_line; do
      [[ -z "$dep_line" ]] && continue
      dep="${dep_line#* }"
      in_list "$dep" "${fixture[@]}" || expect_missing+=("${crate} waits on")
      in_list "$dep" "${fixture[@]}" || expect_missing+=("${dep} ${VERSION} (${dep_line%% *})")
    done < <(workspace_path_deps "$crate")
  done
  arm "every ledger crate's dependencies absent at ${VERSION}: nothing creatable" refuse \
    "$mock" "$(ledger_file absent "${fixture[@]}")" \
    "REFUSING: nothing can be created yet" \
    "failed to select a version for the requirement" \
    "scripts/publish-release-crates.sh" \
    "${expect_missing[@]}"

  # 2b. Token step 1 of the interleave: the FIRST ledger crate's dependencies
  #     are up (the trusted lane stopped at its first dependent), the others'
  #     are not — create the first, DEFER the rest, proceed.
  mock="${tmp}/step1"; mkdir -p "$mock"
  while IFS= read -r dep_line; do
    [[ -n "$dep_line" ]] && version "$mock" "${dep_line#* }"
  done < <(workspace_path_deps "${fixture[0]}")
  local -a expect_step1=("CREATE RECORD  ${fixture[0]} (no crates.io record")
  local -a others=("${fixture[@]:1}")
  for crate in "${others[@]}"; do
    # A later ledger crate whose dependencies happen to be a subset of the
    # first's would be creatable too; only assert DEFER for those that are not.
    local deferred_here=false
    while IFS= read -r dep_line; do
      [[ -z "$dep_line" ]] && continue
      dep="${dep_line#* }"
      in_list "$dep" "${fixture[@]}" && continue
      [[ -f "${mock}/${dep}@${VERSION}" ]] || deferred_here=true
    done < <(workspace_path_deps "$crate")
    [[ "$deferred_here" == "true" ]] && expect_step1+=("DEFER          ${crate} (no record; its dependencies are not on crates.io yet")
  done
  arm "token step 1: ${fixture[0]}'s dependencies present, the rest deferred" pass \
    "$mock" "$(ledger_file step1 "${fixture[@]}")" \
    "1 crate record(s) will be CREATED by this token in this pass: ${fixture[0]}" \
    "gh run rerun" \
    "${expect_step1[@]}"

  # 2c. THE ARGUMENTS HANDED TO THE IRREVERSIBLE STEP, at that same round-1
  #     registry state. This is the arm whose absence let the real defect ship: the
  #     plan above said CREATE one / DEFER the rest and was RIGHT, while every step
  #     after it re-walked the whole ledger and `cargo package` died on a deferred
  #     crate's missing dependency before a single record was created. Asserting the
  #     plan text is not asserting the args; only this is.
  arm "token step 1: the gates and package pass get ONLY ${fixture[0]}" pass \
    "$mock" "$(ledger_file step1args "${fixture[@]}")" \
    "cargo args: -p ${fixture[0]}"
  # And the neighbouring VALID case: with every dependency up, nothing is deferred
  # and the args must cover the whole ledger. A fix that narrowed the args by
  # refusing too much would pass the arm above and fail this one.
  mock="${tmp}/allargs"; mkdir -p "$mock"
  deps_present "$mock"
  local expect_all_args="cargo args:"
  for crate in "${fixture[@]}"; do expect_all_args+=" -p ${crate}"; done
  arm "nothing deferred: the args cover every ledger crate" pass \
    "$mock" "$(ledger_file allargs "${fixture[@]}")" \
    "$expect_all_args"

  # 3. The valid path: no records, every dependency present — the plan proceeds
  #    and names every ledger crate as a record to CREATE.
  mock="${tmp}/valid"; mkdir -p "$mock"
  deps_present "$mock"
  local -a expect_create=()
  for crate in "${fixture[@]}"; do expect_create+=("CREATE RECORD  ${crate} (no crates.io record"); done
  arm "valid: no records, dependencies present" pass \
    "$mock" "$(ledger_file valid "${fixture[@]}")" \
    "${#fixture[@]} crate record(s) will be CREATED by this token in this pass" \
    "${expect_create[@]}"

  # 4. Resumability: the first ledger crate was created at VERSION by an
  #    earlier run — skipped with a notice, the rest still proceed.
  mock="${tmp}/resume"; mkdir -p "$mock"
  deps_present "$mock"
  record "$mock" "${fixture[0]}" false
  version "$mock" "${fixture[0]}"
  arm "resumable: ${fixture[0]} already created at ${VERSION}" pass \
    "$mock" "$(ledger_file resume "${fixture[@]}")" \
    "skip           ${fixture[0]} (${VERSION} already on crates.io: created by an earlier run"

  # 5. A ledger crate whose record exists at some OTHER version is an existing
  #    crate, whatever the ledger says: refuse.
  mock="${tmp}/other-version"; mkdir -p "$mock"
  deps_present "$mock"
  record "$mock" "${fixture[0]}" true
  arm "ledger crate has a record at another version" refuse \
    "$mock" "$(ledger_file other "${fixture[@]}")" \
    "REFUSE         ${fixture[0]} (crate record exists, trustpub_only=true)"

  # 6. An empty ledger: nothing for a token to do.
  mock="${tmp}/empty"; mkdir -p "$mock"
  arm "empty ledger" refuse "$mock" "$(ledger_file empty)" \
    "is empty: nothing to bootstrap"

  # 7. A registry fault is a stop, never "missing" — with every dependency
  #    present, so the ONLY thing standing between this arm and a green plan
  #    is the fault itself.
  mock="${tmp}/fault"; mkdir -p "$mock"
  deps_present "$mock"
  printf '500\n' > "${mock}/${fixture[0]}"
  arm "registry answers 500" refuse "$mock" "$(ledger_file fault "${fixture[@]}")" \
    "could not reach a verdict" "returned 500"

  if [[ "$failures" -gt 0 ]]; then
    echo "bootstrap-crates-io.sh self-test: ${failures} arm(s) FAILED" >&2
    return 1
  fi
  echo "bootstrap-crates-io.sh self-test: every refusal fires and the valid path proceeds"
}

case "$mode" in
  self-test)
    self_test
    exit
    ;;
  plan)
    preflight
    plan_cargo_args
    echo
    # The ARGUMENTS, not a description of them: this is the array `cargo check
    # --target wasm32`, `cargo test`, `cargo package` and the publish loop receive.
    echo "cargo args: ${cargo_args[*]:-(none)}"
    echo "--plan: stopping before the token check, the gates and any publish."
    exit
    ;;
esac

# ---------------------------------------------------------------------------
# A publish run.
# ---------------------------------------------------------------------------

if [[ -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
  if [[ -n "${CARGO_TOKEN:-}" ]]; then
    export CARGO_REGISTRY_TOKEN="${CARGO_TOKEN}"
  else
    echo "Set CARGO_TOKEN or CARGO_REGISTRY_TOKEN before bootstrapping crates.io (or run with --plan)" >&2
    exit 1
  fi
fi

if [[ "${ALLOW_DIRTY:-false}" != "true" ]]; then
  if ! git -C "${repo}" diff --quiet || ! git -C "${repo}" diff --cached --quiet \
    || [[ -n "$(git -C "${repo}" ls-files --others --exclude-standard)" ]]; then
    cat >&2 <<'EOF'
Refusing to publish from a dirty tree.

Commit the release source first, or set ALLOW_DIRTY=true if you intentionally
want crates.io to receive source that does not correspond to a clean git tree.
EOF
    exit 1
  fi
fi

# The irreversible plan, stated BEFORE the long gates run and while the
# operator can still stop. Any refusal above exits here.
preflight
echo

cd "${repo}"
cargo fmt --all --check
cargo check --workspace --lib --tests --locked
python3 scripts/check-versions.py
python3 scripts/check-publish-order.py
if command -v rustup >/dev/null; then
  if ! rustup target list --installed | grep -qx 'wasm32-unknown-unknown'; then
    rustup target add wasm32-unknown-unknown
  fi
fi
# THE PLAN, not the ledger. A DEFERRED crate is one whose dependencies are not on
# crates.io at ${VERSION}; building, testing or packaging it here fails on exactly
# that, and it is not being published in this pass anyway.
if [[ "${#PLAN_TO_CREATE[@]}" -eq 0 ]]; then
  echo "Nothing to create in this pass; the gates and publish loop have no crate to run on."
  exit 0
fi
plan_cargo_args
if [[ "${#PLAN_DEFERRED[@]}" -gt 0 ]]; then
  echo "Gates and packaging cover this pass's ${#PLAN_TO_CREATE[@]} crate(s): ${PLAN_TO_CREATE[*]}"
  echo "(${#PLAN_DEFERRED[@]} deferred crate(s) are excluded — their dependencies are not on crates.io yet.)"
fi
cargo check --locked --target wasm32-unknown-unknown --lib "${cargo_args[@]}"
cargo test --locked "${cargo_args[@]}"
rm -rf target/package
# Package every ledger crate before any is published, so a packaging failure
# (a missing file, a path dependency with no version) is found before the
# first irreversible upload rather than midway through the set. `--no-verify`
# here only skips the build: the preflight already proved every dependency
# resolves, and the loop below verifies each crate for real.
cargo package --locked --no-verify "${cargo_args[@]}"

# The publish loop VERIFIES by default: `cargo publish` unpacks the `.crate` it
# is about to upload and builds it against the registry, so a broken artifact
# is refused BEFORE the one step that cannot be undone. The preflight is what
# makes that build resolvable — every dependency of every ledger crate is on
# crates.io at ${VERSION}, or this script never got here — and
# scripts/check-publish-order.py proves the ledger is in dependency order,
# dev-edges included. PUBLISH_NO_VERIFY=true is for a verification failure
# that is demonstrably not a broken artifact (a registry outage mid-run); it
# leaves your own build as the last check before permanence.
publish_args=(--locked)
if [[ "${PUBLISH_NO_VERIFY}" == "true" ]]; then
  publish_args+=(--no-verify)
  echo "PUBLISH_NO_VERIFY=true: cargo publish will NOT build the packaged crates before uploading them" >&2
fi

wait_for_crate_version() {
  local crate="$1"
  local state
  for _ in $(seq 1 30); do
    state="$(version_state "$crate")"
    registry_or_die "$state"
    if [[ "$state" == "present" ]]; then
      return 0
    fi
    sleep 10
  done
  echo "Timed out waiting for crates.io to expose ${crate} ${VERSION}" >&2
  exit 1
}

created=()
for idx in "${!PLAN_TO_CREATE[@]}"; do
  crate="${PLAN_TO_CREATE[$idx]}"
  state="$(version_state "$crate")"
  registry_or_die "$state"
  if [[ "$state" == "present" ]]; then
    echo "${crate} ${VERSION} already exists on crates.io; skipping"
    continue
  fi
  cargo publish -p "$crate" "${publish_args[@]}"
  wait_for_crate_version "$crate"
  created+=("$crate")
  if ((idx + 1 < ${#PLAN_TO_CREATE[@]})) && [[ "${PUBLISH_COOLDOWN_SECONDS}" != "0" ]]; then
    sleep "${PUBLISH_COOLDOWN_SECONDS}"
  fi
done

cat <<EOF

Created crates.io record(s): ${created[*]:-none}

This token's job for them is done; it cannot publish another version once
they are locked. Now, on crates.io, for EACH of them:
  1. add the Trusted Publisher entry (GitHub Actions / Blackcat-Informatics /
     purrdf / release-cargo.yaml / no environment);
  2. enable "Require trusted publishing" (trustpub_only), as on every sibling
     — the next release's run would 403 at the crate without the entry, and
     the preflight refuses it without the lock;
then re-run the rust-v${VERSION} workflow run (gh run rerun <run-id>): it
resumes at the crate it stopped on. If it stops again, run this script again
for the ledger crates it deferred. After the release is complete, remove the
new crates from PURRDF_UNBOOTSTRAPPED_CRATES in scripts/release-crates.sh and
from the "Outstanding bootstrap" section of docs/RELEASE.md.
EOF
