<!-- SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca> -->
<!-- SPDX-License-Identifier: CC-BY-4.0 -->

# Issue 387 final runtime evidence review addendum

Decision: **ACCEPT** for the runtime implementation, targeted qualification and
runtime evidence/report addendum. This supplements the independent review of the
six original investigation acceptance criteria. Repository-wide gates, hosted
review, PR creation and merge remain separate workflow obligations; this review
does not credit an uncompleted gate.

Reviewed worktree:
`/home/paudley/Active/purrdf/.worktrees/387-blank-scope-investigation`.
No source edits, builds or benchmark executions were performed for this review.

## Evidence consistency

`docs/design/evidence/scope-runtime-benchmarks.json` contains exactly three case
records, each with baseline and current payloads: six estimates and 60 samples.
Every payload equals its archived `estimates.json` under
`.stage/387/scope-evidence/runtime-freshness/{baseline,current}-bench/`.
Independent recomputation of all six medians and MADs agrees with the artifacts.
Every estimate declares schema 1, nanoseconds, ten finite positive samples,
95% bootstrap intervals and 10,000 resamples; each interval contains its median.
Both archived benchmark logs end with `3 benchmarks measured; 0 failed`.

The JSON environment equals the archived environment record. All 19 baseline
and 19 current source hashes match their archived source bytes; all 19 current
hashes also match the current worktree. The source manifests identify the actual
captured snapshots rather than treating a dirty working tree as its HEAD.
Compiler identity, AMD RYZEN AI MAX+ 395 hardware, native platform, capture times,
command, workload and sampling parameters are retained. The bench profile is
consistent with Cargo's release inheritance: opt-level 3, fat LTO, one codegen
unit, debug 0 and strip none. SPDX metadata is preserved.

The source benchmark uses the existing 30,000-person dataset, warms the engine's
plan and BGP-order caches outside the measured loops, attaches no governors,
and times whole-result construction and destruction. Case workloads and output
counts match the record: 30,000 source rows produce 30,000 statements for the
blank-free and one-blank templates, and 60,000 statements for the repeated-label
template. Its ten-sample configuration is explicit in the benchmark source.

| Case | Baseline median ms | Current median ms | Current 95% interval ms |
| --- | ---: | ---: | --- |
| Carries dataset blanks, allocates none | 10.100 | 9.108 | 9.024760–9.186710 |
| One fresh blank per row | 15.863 | 12.742 | 12.626378–12.792788 |
| One fresh blank shared by two triples | 29.090 | 19.676 | 19.138919–22.669827 |

All three report table rows round correctly. The stated co-reference interval
19.14–22.67 ms rounds correctly, and its two high severe outliers match the
payload. The prose explicitly observes movement in the blank-free control,
avoids attributing the whole difference to an isolated kernel, and confines the
measurements to this native host. These ten-sample execution observations remain
separate from the 100-sample scope-prototype comparisons and wasm layouts.

## Runtime governor and identity completion

The final report's budget paragraph accurately describes per-input registration
checkpoints, independent arena watermarks, retained UDF reservation copies,
inclusive budgets, typed exhausted query outcomes and atomic UPDATE refusal.
It does not turn runtime freshness into a proof supplied by the AST prototype.

The shared mint's exhaustion stays outside `EvalError`, whose hard source-read
channel remains intact. CONSTRUCT discards in-flight template output and carries
a certain empty graph certificate when a template allocation trips. UPDATE
returns its typed trip before publishing staged mutations. The two-pass
CONSTRUCT path checks the barrier before later graph/value reads. Incomplete
list construction discards its in-flight auxiliary cells. SERVICE ingestion
stops before committing a partially reidentified row. Previously certified
WHERE lifting and the deterministic transient pure-worker accounting law are
preserved.

The arena watermark belongs to `ScratchInterner`, so clear, replacement and
independently evaluated copies have coherent ownership. Reattaching the same
governor Arc preserves its watermark; attaching a distinct receipt accounts the
full retained arena. Parent lazy reservations and child copies are charged at
their own checkpoints against the same request budget.

The final archived native qualification log reports actual passes for 1,353
evaluator library tests, 25 governed query tests, 22 governed UPDATE tests,
17 prepared-execution tests, 16 scope-interaction tests and 13 fallible-view
tests, with zero failures. The strict core/eval all-target clippy log completes
without warnings. The core log independently records 1,124 library tests and
two blank-publication tests passing, including all three new GlobalDictionary
composite-literal identity-ingress controls.

The new passing controls include:

- `construct_blank_reservations_obey_scratch_bytes_after_where_completes`:
  ordinary, two-blank and two-pass templates; below/equality/above budgets.
- `insert_template_blank_reservations_trip_atomically_after_where_completes`:
  individual INSERT, rollback of a prior DATA operation, and cumulative retention
  in successive UPDATE arenas. The third control correctly charges 76 bytes:
  c1 costs 34 and the collision-free published namespace append0_c2 costs 42.
- `bnode_reservation_exhaustion_is_a_governor_outcome_not_an_unbound_answer`:
  typed exhaustion and an explicit BOUND observer.
- `late_values_reservation_stops_at_the_first_over_budget_identity`:
  128 distinct injected late DEFAULT identities under zero budget retain only
  the first 40-byte label and perform no mint; inclusive complete budgets pass.
- `list_cell_reservations_obey_scratch_bytes_and_discard_incomplete_cells`:
  second-cell exhaustion, buffer rollback and inclusive complete list controls.
- `user_function_reservation_copies_and_later_parent_growth_each_charge_scratch`:
  real registered UDF execution whose larger parent arena cannot hide child
  reservations or later returned-value retention.
- `arena_clear_replacement_and_independent_copy_preserve_shared_scratch_accounting`:
  actual BNODE executions prove clear/replacement/copy accounting, repeated
  checkpoint idempotency and same/different-receipt behavior.

No remaining finding in the reviewed runtime wiring or evidence/report addendum.
Final `make check`, wasm build, complete conformance, generated-artifact
verification and hosted PR/review/merge status must be recorded from their actual
final outcomes by the parent workflow.
