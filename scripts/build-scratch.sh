#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
#
# A scratch directory for a gate that GENERATES artifacts and then reads them back.
#
# Why this is not `mktemp -d`
# ---------------------------
#
# `mktemp -d` answers under `$TMPDIR`, which is `/tmp` unless a caller says
# otherwise. Every generate-then-compare gate in this repository has the same shape:
#
#     tmp="$(mktemp -d)"
#     cargo run ... > "$tmp/first.json"      # may COMPILE first, for minutes
#     cargo run ... > "$tmp/second.json"
#
# The window between creating that directory and writing the last file is as long as
# a cold `cargo` build, and `/tmp` is not a place a directory is guaranteed to survive
# for minutes. Age-based cleaners are standard (`systemd-tmpfiles-clean` ships enabled
# on most distributions and removes by atime), container and CI runners sweep it, and
# an operator may simply clear it. When that happens mid-gate the redirect fails with
#
#     line 28: /tmp/tmp.XXXXXXXX/transcode-loss-matrix.json: No such file or directory
#
# which `make` reports as a plain non-zero exit. Nothing in that names the cause, so
# the gate looks like it caught a real defect in the artifact it was checking. A gate
# whose failure cannot be distinguished from the failure it exists to report is worse
# than no gate: it spends the reader's trust on a coin flip.
#
# The scratch therefore lives beside the build output it is derived from. That
# directory is owned by the build, is already excluded from version control, and no
# cleaner touches it on a timer. `CARGO_TARGET_DIR` is honoured so a caller who has
# relocated the build output gets the scratch relocated with it, rather than having
# two conventions to keep in sync.
#
# Cleanup stays the caller's `trap`: this creates the directory, it does not own it.

# Create and print a fresh scratch directory for `label`.
#
# `label` only has to make the path readable when a gate is interrupted and the
# directory is left behind; uniqueness comes from `mktemp`'s own suffix.
build_scratch_dir() {
  local label="${1:?build_scratch_dir: a label is required}"
  local repo root
  # `BASH_SOURCE[0]` is THIS file even when sourced, so the repository root is found
  # relative to the helper rather than to whichever script called it or to $PWD.
  repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
  root="${CARGO_TARGET_DIR:-${repo}/target}/gate-scratch"
  mkdir -p "${root}"
  mktemp -d "${root}/${label}.XXXXXX"
}
