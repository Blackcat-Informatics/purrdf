#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
# SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0
set -euo pipefail
cd /home/paudley/Active/purrdf/.worktrees/490-typed-capacity-errors-instead-of-u32
stage=.stage/typed-capacity-errors-instead-of-u32
log=$stage/recovery-foundation-runtime.log
binary=$PWD/target/capacity-build/debug/build/purrdf-core/1c7f8ab4d6fee602/out/purrdf_core-1c7f8ab4d6fee602
test ! -e "$log"
read -r available_kib < <(awk '/^MemAvailable:/ {print $2}' /proc/meminfo)
(( available_kib >= 32 * 1024 * 1024 ))
export RUST_TEST_THREADS=4
result=0
systemd-run --user --scope --quiet -p MemoryMax=16G -p MemorySwapMax=0 -- bash -eu -o pipefail -c '
 binary=$1
 "$binary" --list > "$2"
 for name in \
 ir::ingest::tests::event_scope_capacity_preserves_original_count_and_prior_state \
 ir::term::tests::global_arena_range_crosses_four_gib_without_narrowing \
 ir::term::tests::both_arena_widths_use_the_same_append_and_resolution_home \
 ir::term::tests::term_id_from_index_rejects_u32_max \
 ir::term::tests::term_id_option_uses_the_nonzero_niche \
 ir::global::tests::global_term_id_from_index_rejects_u64_max \
 ir::global::tests::global_term_id_index_round_trips \
 ir::global::tests::a_relative_iri_is_refused_and_leaves_the_dictionary_unchanged \
 ir::skolem::term_walk_tests::the_rewrite_and_the_survey_agree_with_their_recursive_references
 do
  rg -Fx "$name: test" "$2"
  "$binary" --exact "$name" --nocapture
 done
' bash "$binary" "$stage/recovery-foundation-runtime-list.txt" > "$log" 2>&1 || result=$?
printf 'terminal_status=%s\n' "$result" >> "$log"
exit "$result"
