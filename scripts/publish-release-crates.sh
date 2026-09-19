#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# The crates.io publish loop of the rust-v* release lane — the point of no
# return of .github/workflows/release-cargo.yaml, kept in a script so the one
# piece of logic that decides what is published can be run, dry-run and
# self-tested locally against a mock registry instead of only on a tag.
#
# What it does, per crate of PURRDF_RELEASE_CRATES, in publish order:
#
#   * VERSION already on crates.io           -> skip (re-runs resume);
#   * in PURRDF_UNBOOTSTRAPPED_CRATES, absent -> SKIP, visibly: only an API
#     token can create a record ("Trusted Publishing tokens do not support
#     creating new crates"), so the ledger crate is left for the token step
#     (scripts/bootstrap-crates-io.sh) and the loop goes on;
#   * depends on a skipped ledger crate      -> STOP CLEANLY (exit 0): naming
#     the crate, the ledger crate it waits on, everything published so far and
#     the next step. `cargo publish` resolves the packaged crate's whole graph
#     against the registry before uploading, so this crate cannot be published
#     until the token step has run; re-running the same tag resumes here;
#   * otherwise                              -> `cargo publish --locked`,
#     VERIFIED, then wait until crates.io serves the version.
#
# This is the three-way interleave that lets a release carry brand-new crates:
# the trusted lane publishes up to the first dependent of a ledger crate and
# stops; the token creates the ledger crate's record (its dependencies now
# exist); the maintainer enables Trusted Publishing on the new record; the tag
# run is re-run and resumes. docs/RELEASE.md, "Outstanding bootstrap", is the
# step-by-step procedure with the expected stop messages.
#
# The stop condition is computed from `cargo metadata`, never hand-listed, and
# over EVERY dependency kind, dev-dependencies included: the publish verifies,
# and verification resolves dev-dependencies too, so a dev-edge onto a skipped
# ledger crate would fail exactly like a normal one — after the upload of the
# crates ahead of it. Ordering alone does not save a dev-edge here: the
# ledger crate it points at is not "earlier and published", it is skipped.
#
# Before any of that, the set is refused outright if it carries a path
# dependency onto a member that is never published (`publish = false`). Cargo
# removes exactly ONE such edge from the manifest it uploads — a dev-dependency
# with no version — and keeps every other shape, so a normal, build or versioned
# dev edge onto such a member makes `cargo publish` fail on a crate that will
# never have a crates.io record. That is a defect in the manifests, not a
# bootstrap state, so it is a hard refusal (exit 1) rather than a clean stop.
#
# Usage:
#   scripts/publish-release-crates.sh VERSION             publish (needs CARGO_REGISTRY_TOKEN)
#   scripts/publish-release-crates.sh VERSION --dry-run   the same decisions, no publish
#   scripts/publish-release-crates.sh --self-test         the interleave end to end,
#                                                         offline, against a mock registry
#
# Environment:
#   GITHUB_OUTPUT               if set, `complete=true|false` is appended, so the
#                               workflow can hold the GitHub Release until the
#                               whole set is up
#   PURRDF_RELEASE_CRATES_FILE  an alternative release-set/ledger file (self-test)
#   PURRDF_CRATES_IO_MOCK       a mock registry directory (scripts/crates-io-api.sh)

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

mode="publish"
VERSION=""
for arg in "$@"; do
  case "$arg" in
    --dry-run) mode="dry-run" ;;
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
      VERSION="${arg#rust-v}"
      ;;
  esac
done

metadata_json="$(mktemp)"
trap 'rm -f "${metadata_json}" "${CRATES_IO_BODY:-}"' EXIT
(cd "${repo}" && cargo metadata --no-deps --format-version 1) > "${metadata_json}"
if [[ -z "${VERSION}" ]]; then
  if [[ "$mode" == "self-test" ]]; then
    VERSION="$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["packages"][0]["version"])' "${metadata_json}")"
  else
    echo "usage: $0 VERSION [--dry-run] | --self-test" >&2
    exit 2
  fi
fi

# One definition of the release set and of the ledger, shared with the
# preflight (scripts/check-crates-io-records.sh) and the token bootstrap
# (scripts/bootstrap-crates-io.sh).
release_list="${PURRDF_RELEASE_CRATES_FILE:-${repo}/scripts/release-crates.sh}"
# shellcheck source=scripts/release-crates.sh
source "${release_list}"
# shellcheck source=scripts/crates-io-api.sh
source "${repo}/scripts/crates-io-api.sh"
# The one reading of which path dependencies survive packaging, shared with the
# token bootstrap so the two lanes cannot disagree about what cargo strips.
# shellcheck source=scripts/workspace-deps.sh
source "${repo}/scripts/workspace-deps.sh"
crates=("${PURRDF_RELEASE_CRATES[@]}")
ledger=("${PURRDF_UNBOOTSTRAPPED_CRATES[@]}")
user_agent="$(crates_io_user_agent "${VERSION}")"

in_list() {
  local needle="$1"
  shift
  local item
  for item in "$@"; do
    [[ "$item" == "$needle" ]] && return 0
  done
  return 1
}

# registry_or_die <state>: present/missing pass; anything else is a hard stop.
# Called in the parent shell after every capture — an `exit` inside `$(...)`
# only ends the subshell, and an empty answer must never read as "missing".
registry_or_die() {
  case "$1" in
    present | missing) ;;
    *)
      echo "crates.io could not give a verdict — ${1#error: }" >&2
      echo "Stopping: an unknown registry answer is a stop, never \"probably fine\"." >&2
      exit 1
      ;;
  esac
}

version_state() {
  crates_io_version_state "$1" "${VERSION}" "${user_agent}"
}

# private_dep_guard <metadata> <crate>...: refuse, before anything is uploaded,
# every surviving path dependency of the release set onto a member that is never
# published (workspace_private_path_deps states why each one is fatal). This is
# the stop that costs nothing; the alternative is the same stop at the upload,
# with the crates ahead of it permanently on the registry.
private_dep_guard() {
  local crate kind dep
  local -a offending=()
  while read -r crate kind dep; do
    [[ -z "$dep" ]] && continue
    offending+=("${crate} -> ${dep} (${kind} dependency)")
  done < <(workspace_private_path_deps "$@")
  [[ "${#offending[@]}" -eq 0 ]] && return 0
  {
    echo "the release set keeps path dependencies onto members that are never published (publish = false):"
    printf '  %s\n' "${offending[@]}"
    echo "cargo removes exactly one shape from the manifest it uploads, a dev-dependency with no version; every other shape is kept and the publish fails on a crate that will never have a crates.io record."
    echo "Stopping: point each of these at a published crate, or make it a versionless dev-dependency. Nothing was published."
  } >&2
  exit 1
}

wait_for_crate_version() {
  local crate="$1" state
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

emit_output() {
  if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
    echo "$1" >> "${GITHUB_OUTPUT}"
  fi
}

# ---------------------------------------------------------------------------
# The loop.
# ---------------------------------------------------------------------------

run_loop() {
  local crate state kind dep dep_state idx
  local -a published=() skipped=() waiting=() not_attempted=()
  local verb="published"
  [[ "$mode" == "dry-run" ]] && verb="would publish"

  # Manifest defect, not a bootstrap state: refused before the first upload.
  private_dep_guard "${metadata_json}" "${crates[@]}"

  for idx in "${!crates[@]}"; do
    crate="${crates[$idx]}"
    state="$(version_state "$crate")"
    registry_or_die "$state"
    if [[ "$state" == "present" ]]; then
      echo "${crate} ${VERSION} already exists on crates.io; skipping"
      continue
    fi
    if in_list "$crate" "${ledger[@]}"; then
      echo "skipping ${crate}: bootstrap pending — no crates.io record; it is in PURRDF_UNBOOTSTRAPPED_CRATES and only the token step (scripts/bootstrap-crates-io.sh) can create it"
      skipped+=("$crate")
      continue
    fi
    waiting=()
    while read -r _ kind dep; do
      [[ -z "$dep" ]] && continue
      in_list "$dep" "${ledger[@]}" || continue
      in_list "$dep" "${published[@]}" && continue
      dep_state="$(version_state "$dep")"
      registry_or_die "$dep_state"
      [[ "$dep_state" == "present" ]] || waiting+=("${dep} ${VERSION} (${kind})")
    done < <(workspace_path_deps "${metadata_json}" "$crate")
    if [[ "${#waiting[@]}" -gt 0 ]]; then
      not_attempted=("${crates[@]:$idx}")
      cat <<EOF

STOP: ${crate} ${VERSION} depends on $(IFS=,; echo "${waiting[*]}"), which is in PURRDF_UNBOOTSTRAPPED_CRATES and not on crates.io yet.
cargo publish resolves ${crate}'s dependency graph against the registry before uploading, so it cannot be published until the token step has created that record. This run stops here, cleanly; nothing after ${crate} was attempted.

  ${verb} in this run: ${published[*]:-nothing}
  skipped (bootstrap pending): ${skipped[*]:-none}
  not attempted: ${not_attempted[*]}

Next — the interleave (docs/RELEASE.md, "Outstanding bootstrap"):
  1. from a clean checkout of this tag, create the ledger records whose dependencies are now on crates.io:
       CARGO_REGISTRY_TOKEN="\${CARGO_TOKEN}" scripts/bootstrap-crates-io.sh ${VERSION}
  2. on crates.io, for EACH record it created: add the Trusted Publisher entry
     (GitHub Actions / Blackcat-Informatics / purrdf / release-cargo.yaml / no environment)
     and enable "Require trusted publishing" — this release no longer touches that crate, but the NEXT release's run would 403 at it without the entry, and the preflight refuses it without the lock;
  3. re-run this workflow run (gh run rerun <run-id>): published versions are skipped, so it resumes at ${crate}.
EOF
      emit_output "complete=false"
      return 0
    fi
    if [[ "$mode" == "dry-run" ]]; then
      echo "would publish ${crate} ${VERSION} (cargo publish -p ${crate} --locked)"
    else
      # VERIFIED: cargo unpacks the .crate and builds it against the registry
      # before the upload that cannot be undone. Resolvable because every
      # dependency is either already served (wait_for_crate_version) or is a
      # ledger crate the token step created — or this loop stopped above.
      cargo publish -p "$crate" --locked
      wait_for_crate_version "$crate"
      # Give the registry index a short propagation window before publishing
      # dependents that name the freshly-published version.
      sleep 15
    fi
    published+=("$crate")
  done

  echo
  if [[ "${#skipped[@]}" -gt 0 ]]; then
    # Every remaining crate published; only ledger crates with no dependent in
    # the set are still absent. The set is not complete until they exist.
    cat <<EOF
INCOMPLETE: every crate that could be published is on crates.io, but these ledger crates still have no record: ${skipped[*]}
Create them with the token step, enable Trusted Publishing on each, then re-run this workflow run to confirm the set (it will publish nothing and report complete).
EOF
    emit_output "complete=false"
    return 0
  fi
  echo "COMPLETE: all ${#crates[@]} release crates are on crates.io at ${VERSION} (${verb}: ${published[*]:-none})."
  emit_output "complete=true"
}

# ---------------------------------------------------------------------------
# Self-test: the interleave, end to end, against a mock registry. Each round
# is a dry-run; between rounds the mock is advanced exactly as the real
# procedure would advance the registry — the crates the loop "would publish"
# appear, then the token step creates every ledger crate whose dependencies
# now exist. The rounds must terminate, every STOP must name the first crate
# in publish order that depends on the ledger crate it waits on (computed
# independently here, from the metadata), and the final round must say
# COMPLETE.
# ---------------------------------------------------------------------------

self_test() {
  local tmp
  tmp="$(mktemp -d)"
  # shellcheck disable=SC2064  # expand now: tmp is what to remove.
  trap "rm -rf '${tmp}'; rm -f '${metadata_json}'" EXIT
  local failures=0

  # The fixture ledger: the real one when non-empty, else one crate with
  # dependents, so the STOP arm is always exercised.
  local -a fixture
  if [[ "${#ledger[@]}" -gt 0 ]]; then
    fixture=("${ledger[@]}")
  else
    fixture=("${crates[1]}")
  fi
  local ledger_file="${tmp}/ledger.sh"
  {
    cat "${repo}/scripts/release-crates.sh"
    printf '\nPURRDF_UNBOOTSTRAPPED_CRATES=(%s)\n' "${fixture[*]}"
  } > "${ledger_file}"

  local mock="${tmp}/registry"
  mkdir -p "$mock"
  present() { printf '200\n{"version":{"crate":"%s","num":"%s"}}\n' "$1" "${VERSION}" > "${mock}/$1@${VERSION}"; }
  is_present() { [[ -f "${mock}/$1@${VERSION}" ]]; }

  # first_dependent <ledger-crate>: the first release crate, in publish order,
  # with a path dependency (any kind) on it — the crate the loop must stop at.
  first_dependent() {
    python3 - "${metadata_json}" "$1" "${crates[@]}" <<'PY'
import json
import sys

metadata = json.load(open(sys.argv[1], encoding="utf-8"))
target = sys.argv[2]
order = sys.argv[3:]
deps = {
    p["name"]: {d["name"] for d in p["dependencies"] if d.get("path")}
    for p in metadata["packages"]
}
for crate in order:
    if target in deps.get(crate, set()):
        print(crate)
        break
PY
  }

  # token_step: create every fixture crate whose path deps are all present.
  token_step() {
    local crate dep ok created=()
    for crate in "${fixture[@]}"; do
      is_present "$crate" && continue
      ok=true
      while read -r _ _ dep; do
        [[ -z "$dep" ]] && continue
        is_present "$dep" || ok=false
      done < <(workspace_path_deps "${metadata_json}" "$crate")
      if [[ "$ok" == "true" ]]; then
        present "$crate"
        created+=("$crate")
      fi
    done
    echo "${created[*]:-}"
  }

  echo "publish-release-crates.sh self-test (fixture ledger: ${fixture[*]}; version ${VERSION})"
  local round=0 out status crate stop_crate expect created
  while :; do
    round=$((round + 1))
    if [[ "$round" -gt 10 ]]; then
      echo "  FAILED  the interleave did not terminate in 10 rounds"
      failures=$((failures + 1))
      break
    fi
    status=0
    out="$(PURRDF_CRATES_IO_MOCK="${mock}" PURRDF_RELEASE_CRATES_FILE="${ledger_file}" \
      bash "${BASH_SOURCE[0]}" "${VERSION}" --dry-run 2>&1)" || status=$?
    if [[ "$status" -ne 0 ]]; then
      echo "  FAILED  round ${round}: dry-run exited ${status}"
      while IFS= read -r line; do printf '    | %s\n' "$line"; done <<<"$out"
      failures=$((failures + 1))
      break
    fi
    # Advance the mock by what the loop would have published.
    while IFS= read -r crate; do
      [[ -n "$crate" ]] && present "$crate"
    done < <(sed -n 's/^would publish \([a-z0-9-]*\) .*/\1/p' <<<"$out")

    if grep -q '^COMPLETE:' <<<"$out"; then
      printf '  ok      round %s: COMPLETE — every release crate on crates.io\n' "$round"
      break
    fi
    stop_crate="$(sed -n 's/^STOP: \([a-z0-9-]*\) .*/\1/p' <<<"$out")"
    if [[ -n "$stop_crate" ]]; then
      # The STOP must be at the first dependent of a still-absent ledger crate,
      # every earlier non-ledger crate must have been published, and the crate
      # itself must not have been.
      expect=""
      for crate in "${fixture[@]}"; do
        if ! is_present "$crate"; then
          expect="$(first_dependent "$crate")"
          [[ -n "$expect" ]] && break
        fi
      done
      if [[ "$stop_crate" == "$expect" ]] && ! grep -q "^would publish ${stop_crate} " <<<"$out" \
        && grep -q "^skipping ${crate}: bootstrap pending" <<<"$out" \
        && grep -q "gh run rerun" <<<"$out" && grep -q 'enable "Require trusted publishing"' <<<"$out"; then
        printf '  ok      round %s: STOP at %s, waiting on %s (first dependent in publish order, computed independently)\n' "$round" "$stop_crate" "$crate"
      else
        printf '  FAILED  round %s: STOP at %s, expected %s (waiting on %s)\n' "$round" "$stop_crate" "$expect" "$crate"
        while IFS= read -r line; do printf '    | %s\n' "$line"; done <<<"$out"
        failures=$((failures + 1))
        break
      fi
    elif grep -q '^INCOMPLETE:' <<<"$out"; then
      printf '  ok      round %s: INCOMPLETE — only dependent-free ledger crates remain\n' "$round"
    else
      echo "  FAILED  round ${round}: neither STOP, INCOMPLETE nor COMPLETE"
      while IFS= read -r line; do printf '    | %s\n' "$line"; done <<<"$out"
      failures=$((failures + 1))
      break
    fi
    created="$(token_step)"
    printf '          token step creates: %s\n' "${created:-nothing (its dependencies are not up yet)}"
    if [[ -z "$created" ]]; then
      echo "  FAILED  round ${round}: the token step could create nothing, so the interleave is stuck"
      failures=$((failures + 1))
      break
    fi
  done

  # A registry fault is a stop with a non-zero exit, never a verdict.
  local fault="${tmp}/fault"
  mkdir -p "$fault"
  printf '502\n' > "${fault}/${crates[0]}@${VERSION}"
  status=0
  out="$(PURRDF_CRATES_IO_MOCK="${fault}" PURRDF_RELEASE_CRATES_FILE="${ledger_file}" \
    bash "${BASH_SOURCE[0]}" "${VERSION}" --dry-run 2>&1)" || status=$?
  if [[ "$status" -ne 0 ]] && grep -q "returned 502" <<<"$out"; then
    printf '  ok      registry answers 502: exit %s, no decision taken\n' "$status"
  else
    printf '  FAILED  registry answers 502: exit %s\n' "$status"
    failures=$((failures + 1))
  fi

  # Path dependencies onto a member that is never published. Cargo strips one
  # shape and keeps the rest, so the filter has to be exact in BOTH directions:
  # each kept shape must be refused, and the stripped one must pass in silence,
  # because that is the shape the release crates really carry today. Each
  # fixture is the committed metadata with one edge injected onto the first
  # release crate, so the shapes are cargo's own and only the edge is invented.
  local private_member fixture
  private_member="$(workspace_never_published "${metadata_json}" | head -n 1)"
  if [[ -z "$private_member" ]]; then
    echo "  FAILED  no publish = false member to build the private-dependency fixtures from"
    failures=$((failures + 1))
  else
    # inject_dep <kind> <req> <out>: the metadata with one path dependency of
    # the first release crate onto the private member added. <kind> is "normal"
    # (cargo metadata writes that kind as null), "build" or "dev".
    inject_dep() {
      python3 - "${metadata_json}" "${crates[0]}" "${private_member}" "$1" "$2" "$3" <<'PY'
import json
import sys

metadata_path, crate, private, kind, req, out = sys.argv[1:7]
metadata = json.load(open(metadata_path, encoding="utf-8"))
for package in metadata["packages"]:
    if package["name"] == crate:
        package["dependencies"].append({
            "name": private,
            "source": None,
            "req": req,
            "kind": None if kind == "normal" else kind,
            "rename": None,
            "optional": False,
            "uses_default_features": True,
            "features": [],
            "target": None,
            "registry": None,
            "path": "/injected-by-self-test",
        })
with open(out, "w", encoding="utf-8") as handle:
    json.dump(metadata, handle)
PY
    }

    # must_refuse <kind> <req> <label>: cargo keeps this shape, so the guard has
    # to stop the run and name the edge rather than let the upload find it.
    must_refuse() {
      local shape_fixture="${tmp}/private-${3// /-}.json" shape_status=0 shape_out
      inject_dep "$1" "$2" "${shape_fixture}"
      shape_out="$(private_dep_guard "${shape_fixture}" "${crates[@]}" 2>&1)" || shape_status=$?
      if [[ "$shape_status" -eq 1 ]] && grep -q "${crates[0]} -> ${private_member} ($1 dependency)" <<<"$shape_out"; then
        printf '  ok      %s onto %s: refused, exit %s, edge named\n' "$3" "${private_member}" "$shape_status"
      else
        printf '  FAILED  %s onto %s: exit %s, expected exit 1 naming the edge\n' "$3" "${private_member}" "$shape_status"
        while IFS= read -r line; do printf '    | %s\n' "$line"; done <<<"$shape_out"
        failures=$((failures + 1))
      fi
    }

    must_refuse normal '*' "a normal dependency"
    must_refuse build '*' "a build dependency"
    must_refuse dev '^0.0.1' "a versioned dev-dependency"

    # The other direction, twice: the injected legitimate shape, and the
    # committed workspace itself — where every release crate's edge onto that
    # member is exactly this shape. A filter that refused either would break the
    # release lane outright, which is worse than the hole it closes.
    fixture="${tmp}/private-versionless-dev.json"
    inject_dep dev '*' "${fixture}"
    status=0
    out="$(private_dep_guard "${fixture}" "${crates[@]}" 2>&1)" || status=$?
    if [[ "$status" -eq 0 && -z "$out" ]]; then
      printf '  ok      a versionless dev-dependency onto %s: accepted in silence (cargo strips it)\n' "${private_member}"
    else
      printf '  FAILED  a versionless dev-dependency onto %s: exit %s, expected exit 0 and no output\n' "${private_member}" "$status"
      while IFS= read -r line; do printf '    | %s\n' "$line"; done <<<"$out"
      failures=$((failures + 1))
    fi
    status=0
    out="$(private_dep_guard "${metadata_json}" "${crates[@]}" 2>&1)" || status=$?
    if [[ "$status" -eq 0 && -z "$out" ]]; then
      printf '  ok      the committed workspace: accepted, so the refusals above are a filter and not a ban\n'
    else
      printf '  FAILED  the committed workspace: exit %s, the control case must pass\n' "$status"
      while IFS= read -r line; do printf '    | %s\n' "$line"; done <<<"$out"
      failures=$((failures + 1))
    fi
  fi

  if [[ "$failures" -gt 0 ]]; then
    echo "publish-release-crates.sh self-test: ${failures} arm(s) FAILED" >&2
    return 1
  fi
  echo "publish-release-crates.sh self-test: the interleave terminates, every STOP is at the right crate, faults stop the loop, and a path dependency onto a never-published member is refused unless cargo would strip it"
}

case "$mode" in
  self-test) self_test ;;
  *)
    if [[ "$mode" == "publish" && -z "${CARGO_REGISTRY_TOKEN:-}" ]]; then
      echo "CARGO_REGISTRY_TOKEN is not set (use --dry-run to see the decisions without publishing)" >&2
      exit 1
    fi
    cd "${repo}"
    run_loop
    ;;
esac
