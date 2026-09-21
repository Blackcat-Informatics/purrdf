# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
# shellcheck shell=bash
#
# The one reading of the workspace's PATH dependencies the release scripts share.
#
# This file is SOURCED, never executed. `scripts/publish-release-crates.sh` (the
# trusted lane's publish loop) and `scripts/bootstrap-crates-io.sh` (the token
# step that creates new records) both have to answer the same question before
# they upload anything — which intra-workspace edges will still be in the
# manifest cargo sends to crates.io — and both used to answer it with their own
# copy of a `cargo metadata` walk. The copies disagreed, which is how this file
# came to exist.
#
# The rule they share, and it has exactly one exception:
#
#   `cargo package` removes ONE kind of path dependency from the manifest it
#   uploads — a DEV-dependency with NO version requirement. Everything else
#   survives packaging and has to resolve against the registry, because
#   `cargo publish` resolves the packaged crate's whole graph (dev-dependencies
#   included, with or without `--no-verify`) to write its lockfile.
#
# That exception is what lets a published crate dev-depend on a member that is
# never published (`publish = false`), which `purrdf-alloc-probe` is and which
# seven release crates do: the edge carries a path and no version, so the
# uploaded manifest never mentions it and nothing looks for it on a registry it
# will never be on. `scripts/release-crates.sh` states the versionless rule its
# dependents rely on — give that entry a version and every dependent's publish
# breaks.
#
# Both halves of that are load bearing, in opposite directions:
#
#   * treat MORE edges as stripped than cargo does, and a normal, build or
#     versioned dev edge onto a never-published member sails past every check
#     and dies at the upload, after the crates ahead of it are permanently on
#     crates.io (`workspace_private_path_deps` is the refusal that stops it);
#   * treat FEWER as stripped than cargo does, and the legitimate versionless
#     dev edge looks like a missing dependency — which reads as "wait for the
#     trusted lane" for a crate the trusted lane will never publish, and the
#     release simply stops.
#
# Nothing here ever exits or decides: each function reports a fact and the
# calling script chooses what it means, the same contract as
# `scripts/crates-io-api.sh`. Every answer comes from `cargo metadata`, never a
# hand-written list, and the caller passes the metadata document in — so a
# self-test can ask these questions about a perturbed workspace without one
# existing on disk.

# workspace_never_published <metadata-json>
#
# Every member cargo metadata reports as `publish = false`, one name per line,
# sorted. In `cargo metadata` an unrestricted member has `publish: null` and a
# `publish = false` member has `publish: []`, so the empty LIST is the test; a
# missing key is not. These members have no crates.io record and never will.
workspace_never_published() {
  python3 - "$1" <<'PY'
import json
import sys

metadata = json.load(open(sys.argv[1], encoding="utf-8"))
for name in sorted(p["name"] for p in metadata["packages"] if p.get("publish") == []):
    print(name)
PY
}

# workspace_path_deps <metadata-json> <crate>...
#
# "<crate> <kind> <name>" per PATH dependency of each named crate that SURVIVES
# packaging: every kind, dev-dependencies included, minus the one shape cargo
# removes — a dev-dependency onto a never-published member, carrying a path and
# no version. Cargo metadata reports a versionless requirement as `*`; an
# explicit `version = "*"` is indistinguishable here, and crates.io refuses a
# wildcard requirement outright, so neither can reach the registry unnoticed.
#
# Kinds are otherwise never filtered. A dev-edge onto a PUBLISHED crate is a
# real publish-ordering constraint (scripts/check-publish-order.py holds the
# release order to both edge kinds for the same reason).
workspace_path_deps() {
  local metadata="$1"
  shift
  python3 - "${metadata}" "$@" <<'PY'
import json
import sys

metadata = json.load(open(sys.argv[1], encoding="utf-8"))
packages = {p["name"]: p for p in metadata["packages"]}
never_published = {p["name"] for p in metadata["packages"] if p.get("publish") == []}
for name in sys.argv[2:]:
    for dep in packages.get(name, {}).get("dependencies", []):
        if not dep.get("path"):
            continue
        stripped_by_cargo = (
            dep["name"] in never_published
            and dep["kind"] == "dev"
            and dep["req"] == "*"
        )
        if not stripped_by_cargo:
            print(name, dep["kind"] or "normal", dep["name"])
PY
}

# workspace_private_path_deps <metadata-json> <crate>...
#
# The subset of the above that points at a member which is never published —
# in other words, every edge cargo will KEEP in the uploaded manifest naming a
# crate that will never have a crates.io record. Each one is a publish that
# cannot succeed in any pass, by any lane, at any version: a normal or build
# edge is rejected for carrying no version, a versioned one because the version
# cannot be selected. The manifest is the only place it can be fixed.
#
# Empty output is the normal, correct answer for this workspace.
workspace_private_path_deps() {
  local metadata="$1"
  shift
  local line crate kind dep member
  local -a never=()
  while IFS= read -r line; do
    [[ -n "$line" ]] && never+=("$line")
  done < <(workspace_never_published "${metadata}")
  [[ "${#never[@]}" -eq 0 ]] && return 0
  while read -r crate kind dep; do
    [[ -z "$dep" ]] && continue
    for member in "${never[@]}"; do
      if [[ "$member" == "$dep" ]]; then
        printf '%s %s %s\n' "$crate" "$kind" "$dep"
        break
      fi
    done
  done < <(workspace_path_deps "${metadata}" "$@")
}
