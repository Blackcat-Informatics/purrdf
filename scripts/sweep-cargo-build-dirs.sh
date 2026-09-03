#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0
#
# Reclaim disk from cargo build directories whose workspace no longer exists.
#
# WHY THIS EXISTS
#
# With `[unstable] build-dir-new-layout` and a `build-dir` template containing
# `{workspace-path-hash}`, cargo keys each build directory on a hash of the
# WORKSPACE PATH. Every git worktree therefore gets its own multi-gigabyte tree,
# and — this is the part that costs disk — those trees OUTLIVE the worktree by
# design. `git worktree remove` knows nothing about them, and the hash is one-way,
# so nothing can look at an orphaned directory and say which worktree it belonged
# to. On a checkout that sees heavy branch or worktree churn these accumulate
# without bound and can fill the disk.
#
# HOW A DIRECTORY IS ATTRIBUTED
#
# The hash cannot be inverted, but the build products remember where they came
# from: dep-info (`.d`) files record ABSOLUTE paths to the sources that were
# compiled. Sampling those for a first-party workspace path (one containing
# `crates/` or `bindings/`, as opposed to a registry checkout under
# `~/.cargo/registry`) recovers the workspace that created the directory. If that
# path no longer exists, the directory is an orphan.
#
# WHAT IT WILL NOT DO
#
# A directory whose origin cannot be established is reported UNKNOWN and is NEVER
# deleted, not even with `--delete`. The whole point of the attribution step is to
# avoid deleting a live workspace's cache; a directory that yields no evidence has
# not been attributed, and "no evidence" is not "orphan". Re-run after building in
# the suspect workspace if you want it classified.
#
# USAGE
#
#   scripts/sweep-cargo-build-dirs.sh                # report only (default)
#   scripts/sweep-cargo-build-dirs.sh --delete       # remove ORPHAN directories
#   scripts/sweep-cargo-build-dirs.sh --root DIR     # override the build-dir root
#   scripts/sweep-cargo-build-dirs.sh --self-test    # prove both verdicts on fixtures

set -euo pipefail

root=""
delete=false
mode="report"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --delete) delete=true ;;
    --self-test) mode="self-test" ;;
    --root)
      shift
      root="${1:-}"
      [[ -n "$root" ]] || { echo "--root needs a directory" >&2; exit 2; }
      ;;
    -h|--help) sed -n '5,40p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
  shift
done

# How many dep-info files to sample per directory before giving up on attribution.
# A first-party path usually appears within the first handful; the cap is what
# keeps a sweep over ~200 directories from walking millions of files.
SAMPLE_LIMIT="${SWEEP_SAMPLE_LIMIT:-600}"

# The configured build-dir root, read from cargo's own config rather than assumed.
default_root() {
  local config="${CARGO_HOME:-${HOME}/.cargo}/config.toml"
  [[ -f "$config" ]] || return 1
  # `build-dir = "/some/path/{workspace-path-hash}"` → `/some/path`
  local template
  template="$(grep -oE '^[[:space:]]*build-dir[[:space:]]*=[[:space:]]*"[^"]+"' "$config" \
    | head -1 | sed -E 's/.*"([^"]+)".*/\1/')"
  [[ -n "$template" ]] || return 1
  # Strip every trailing path segment that carries a {placeholder}.
  while [[ "$template" == */* && "$template" == *'{'* ]]; do
    case "${template##*/}" in
      *'{'*) template="${template%/*}" ;;
      *) break ;;
    esac
  done
  printf '%s\n' "$template"
}

# workspace_of <dir>: the first-party workspace path that built `dir`, or empty.
#
# Registry checkouts are excluded explicitly: every build directory is full of
# `~/.cargo/registry/...` sources, and they say nothing about which workspace
# drove the build.
workspace_of() {
  local dir="$1" line path
  while IFS= read -r line; do
    # Absolute source paths, first-party only (a workspace has crates/ or bindings/).
    path="$(grep -ohE '/[^ :]*/(crates|bindings)/[^ :]*' <<<"$line" | head -1 || true)"
    [[ -n "$path" ]] || continue
    [[ "$path" == *"/.cargo/registry/"* ]] && continue
    [[ "$path" == *"/.cargo/git/"* ]] && continue
    # Strip at the FIRST `/crates/` or `/bindings/` segment. Deliberately `sed`
    # rather than `${path%%/@(crates|bindings)/*}`: that extglob form needs
    # `shopt -s extglob` to mean anything, and without it bash treats it as a
    # literal that matches nothing — leaving `workspace` set to the full SOURCE
    # FILE path, which is never a directory, so every build dir was classified
    # ORPHAN and `--delete` ate live caches. The self-test's "SPARED the live
    # workspace" check is what caught it.
    sed -E 's#/(crates|bindings)/.*$##' <<<"$path"
    return 0
  done < <(find "$dir" -name '*.d' -type f 2>/dev/null | head -n "$SAMPLE_LIMIT" \
             | xargs -r grep -ohE '/[^ :]*/(crates|bindings)/[^ :]*' 2>/dev/null | head -40)
  return 0
}

sweep() {
  [[ -n "$root" ]] || root="$(default_root || true)"
  if [[ -z "$root" || ! -d "$root" ]]; then
    echo "No cargo build-dir root found. Pass --root DIR." >&2
    echo "(Looked for a build-dir template in ${CARGO_HOME:-${HOME}/.cargo}/config.toml.)" >&2
    exit 1
  fi
  echo "cargo build-dir root: ${root}"
  echo

  local dir workspace size verdict
  local -a orphans=()
  local live=0 unknown=0 orphan_bytes=0

  # The layout is <root>/<shard>/<hash>; a hash directory is one that holds a
  # CACHEDIR.TAG, which is cargo's own marker for "this is a build directory".
  while IFS= read -r dir; do
    workspace="$(workspace_of "$dir")"
    size="$(du -sh "$dir" 2>/dev/null | cut -f1)"
    if [[ -z "$workspace" ]]; then
      verdict="UNKNOWN  (no first-party source path found; NOT deletable)"
      unknown=$((unknown + 1))
    elif [[ -d "$workspace" ]]; then
      verdict="live     ${workspace}"
      live=$((live + 1))
    else
      verdict="ORPHAN   ${workspace} (gone)"
      orphans+=("$dir")
      orphan_bytes=$((orphan_bytes + $(du -sm "$dir" 2>/dev/null | cut -f1)))
    fi
    printf '  %6s  %s\n          %s\n' "$size" "$dir" "$verdict"
  done < <(find "$root" -mindepth 2 -maxdepth 3 -name CACHEDIR.TAG -type f 2>/dev/null \
             | sed 's#/CACHEDIR.TAG$##' | sort)

  echo
  printf 'live %d, orphan %d (~%d GB), unknown %d\n' \
    "$live" "${#orphans[@]}" "$((orphan_bytes / 1024))" "$unknown"

  if [[ "${#orphans[@]}" -eq 0 ]]; then
    echo "Nothing to reclaim."
    return 0
  fi
  if [[ "$delete" != "true" ]]; then
    echo "Report only. Re-run with --delete to remove the ORPHAN directories above."
    return 0
  fi
  for dir in "${orphans[@]}"; do
    echo "removing ${dir}"
    rm -rf -- "$dir"
  done
  echo "Reclaimed ~$((orphan_bytes / 1024)) GB from ${#orphans[@]} directory(ies)."
}

# ---------------------------------------------------------------------------
# Self-test: BOTH verdicts on fixtures — the orphan that must be swept, and the
# live directory that must survive. A sweeper proven only on the delete case is
# the one that eventually eats a working tree.
# ---------------------------------------------------------------------------
self_test() {
  local tmp failures=0
  tmp="$(mktemp -d)"
  # shellcheck disable=SC2064
  trap "rm -rf '${tmp}'" EXIT

  local fake_root="${tmp}/root"
  local live_ws="${tmp}/live-workspace"
  local dead_ws="${tmp}/dead-workspace"
  mkdir -p "${live_ws}/crates/thing/src"

  # A build directory attributed to a workspace that still exists.
  mkdir -p "${fake_root}/aa/live/debug/deps"
  : > "${fake_root}/aa/live/CACHEDIR.TAG"
  printf '%s/debug/deps/thing.rlib: %s/crates/thing/src/lib.rs\n' \
    "${fake_root}/aa/live" "${live_ws}" > "${fake_root}/aa/live/debug/deps/thing.d"

  # A build directory attributed to a workspace that is gone.
  mkdir -p "${fake_root}/bb/dead/debug/deps"
  : > "${fake_root}/bb/dead/CACHEDIR.TAG"
  printf '%s/debug/deps/thing.rlib: %s/crates/thing/src/lib.rs\n' \
    "${fake_root}/bb/dead" "${dead_ws}" > "${fake_root}/bb/dead/debug/deps/thing.d"

  # A build directory with nothing but registry sources: unattributable.
  mkdir -p "${fake_root}/cc/opaque/debug/deps"
  : > "${fake_root}/cc/opaque/CACHEDIR.TAG"
  printf '%s/debug/deps/x.rlib: %s/.cargo/registry/src/index/ahash-0.8.12/crates/build.rs\n' \
    "${fake_root}/cc/opaque" "${HOME}" > "${fake_root}/cc/opaque/debug/deps/x.d"

  local out
  root="$fake_root"
  delete=true
  out="$(sweep)"

  check() {
    if grep -qF -- "$2" <<<"$out"; then
      printf '  ok      %s\n' "$1"
    else
      printf '  FAILED  %s\n' "$1"
      failures=$((failures + 1))
    fi
  }
  check "the dead workspace's directory is classified ORPHAN" "ORPHAN   ${dead_ws}"
  check "the live workspace's directory is classified live" "live     ${live_ws}"
  check "a registry-only directory is UNKNOWN, not orphan" "UNKNOWN"

  # The verdicts that matter are on DISK, not in the report.
  if [[ -d "${fake_root}/bb/dead" ]]; then
    printf '  FAILED  --delete removed the orphan\n'; failures=$((failures + 1))
  else
    printf '  ok      --delete removed the orphan\n'
  fi
  # THE NEIGHBOURING VALID CASES: deleting too much is the failure mode that
  # costs a rebuild — or a working tree — so both survivors are asserted.
  if [[ -d "${fake_root}/aa/live" ]]; then
    printf '  ok      --delete SPARED the live workspace\n'
  else
    printf '  FAILED  --delete SPARED the live workspace\n'; failures=$((failures + 1))
  fi
  if [[ -d "${fake_root}/cc/opaque" ]]; then
    printf '  ok      --delete SPARED the unattributable directory\n'
  else
    printf '  FAILED  --delete SPARED the unattributable directory\n'; failures=$((failures + 1))
  fi

  if [[ "$failures" -gt 0 ]]; then
    echo "sweep-cargo-build-dirs.sh self-test: ${failures} check(s) FAILED" >&2
    return 1
  fi
  echo "sweep-cargo-build-dirs.sh self-test: orphans are swept and everything else is spared"
}

case "$mode" in
  self-test) self_test ;;
  *) sweep ;;
esac
