# G2 independent repair review

VERDICT: PASS

PF1 HIGH and PF2/CR1 MEDIUM are repaired and have meaningful bounded execution
evidence. No assigned G2 source finding or acceptance requirement remains open.
This verdict is independent source judgment with adjudication of attributable
implementation-run logs; the reviewer did not repeat Cargo or runtime checks.
G3 assembly projection and remaining hosted/workflow gates are explicitly open.

## Qualified identity and review scope

Issue 457; PR 466; branch `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
Worktree: `/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
Assessed main base: `b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac`.
Committed parent: `6aa17ec81526d270160827586a6f695d064a1f31`.
Parent tree: `b5a7e8cc9330b199c4f5c33f99c4f7b33bfc3156`.
Final staged candidate: `fc174197e0ca974f5a5ecd0c55aea760419cadd0`.
Final `raw/G2-source.patch` SHA-256:
`b98f78f43a5655f11c135e1fccbd859d377f35a5bb0da4a6b729d56bb884e569`.
The earlier `ac992a403f98d5aad9bedb8ba821e8e42fd649e8` candidate is historical,
superseded only by the strengthened buffered-side-iterator fixture.

Independently rechecked all six entries of `raw/G2-source-sha256.txt`, current
parent HEAD, the exact candidate changed-path set, actual staged no-prefix patch
digest and staged-index equality to the final tree (exit 0). The staged patch
digest equals the supplied complete patch. Source/Cargo ownership was released
before final adjudication; the parent owns staging and subsequent transport.

| Reviewed file | Final SHA-256 |
| --- | --- |
| crates/rdf-core/src/ir/paged/admission.rs | 8819b801eb38103ecad70d5d72f898525bf3376f91962bdc09e85a75b5fe1e2a |
| crates/rdf-core/src/ir/paged/query.rs | 02fe7264f8ea1021c4c2a41d9c97ccfe12f4af5f8d68d8e233bed1f5bed2c93d |
| crates/rdf-core/src/ir/paged/stack.rs | 1b40f2de5e9c9acf06e650beb3b02792c800cac29b0720aff0c276db8f451827 |
| crates/rdf-core/tests/paged_stack.rs | b09867e0543d3403ca35051f72d859c3baf2bbd1c6cbe8841e7933c5aba44d5b |
| crates/sparql-eval/tests/paged_stack_query.rs | 42c6c6c64f5c2db98e0d5d8396d5d9ead37bf6232ffb1a3f43fa93b16624eec8 |
| crates/rdf-wasm/src/interleaving.rs | 53a72955d951e17ec62f59e648d8dd52391098e189139a1b98999988875d8f75 |

Applied the already-read repository laws, root .baseline/.goals, backend contract
and Stage 2 quality/validation/no-deferrals guidance. Read remediation-plan.md,
gap-feedback-addendum.md, current validation index, final G2 implementation
handoff and relevant logs. Reviewed changed production/test homes and actual
guarded ordinary/prepared and canonical-fold finalization wiring. Reused unchanged
Task 1/2/G1 judgment rather than reconstructing an unrelated broader audit.
No source/index/history/configuration mutation, Cargo/build/runtime execution,
forge access or publication occurred. Only this report was written.

## PF1: candidate work is bounded before slot inspection

The existing `candidate_pages_for_stream` delegates to one range-aware home.
For Default/Named graphs, that home takes the existing stream-specific sorted
graph postings, applies binary lower/upper boundaries, and returns their slice.
It does not iterate/filter every global posting separately for each layer.
Start-zero/end-page-count boundaries avoid unnecessary searches on the native
unbounded path. Empty ranges return no candidates; Any directly traverses only
the requested bounded dense range. Existing graph-index derivation establishes
ascending resident PageIds, making the slice and direct slot mapping sound.

`stream_pattern_range` and `stream_estimate` now use this same candidate home
before touching slots or running axis summaries. Physical ascending order, layer
chronology, subject-bound native indexes, predicate/object/graph row checks and
conservative summary laws are unchanged. Promoted ordinary/demoted annotation
handling still considers their respective physical streams; the repair does not
use logical typing as an unsound physical exclusion. Materialization still uses
the existing accessor/cache/admission/certification/budget/failure homes.
No parallel index, eager reifier table, encoder, accounting implementation or
shipping metrics API was introduced.

Measurement hooks are private `cfg(test)` counters at the actual production
partition comparator and `stream_admitted` boundaries. Thread-local storage
isolates test workers. Both statics are inventoried as `Safety::NotCompiledIn`
with the existing TEST_ONLY reason; they do not become wasm runtime state.
Provider counters measure the real trait methods, not a proxy algorithm.

Read meaningful pre-repair failures in `raw/G2-candidates-before-3.log`: eight
sparse probes visited 24 summaries instead of 8, and a two-row/two-page plain
drain visited four Reifier summaries despite empty postings. Earlier candidate
logs that failed test compilation are retained but do not qualify reproduction.
Final serial unit measurements in `raw/G2-paged-unit-qualified.log` establish:

- At 3/67/1027 physical pages, eight repeated Default/Named probes for each of
  Base/Reifier/Annotation inspect exactly eight target summaries; estimates
  inspect one, Any inspects the three-page range once, and empty ranges inspect
  none. Unrelated pages use the opposite graph kind.
- Dense postings of 8/128/1024 pages return the exact three ordered middle IDs
  with 7/15/21 Default/Named boundary comparisons, within the asserted logarithmic
  bound. Any uses zero comparisons; full-range and empty-posting paths need no
  boundary searches.
- Plain RDF stacks of 1/8/32 layers, two independent one-row pages per layer,
  return 2/16/64 rows on initial and cached drains with zero Reifier-summary
  visits. This executes the formerly costly logical classification path.

These are operation-count claims about the repaired metadata bottleneck, not
elapsed-time, whole-query throughput or constant total publication cost claims.

## PF2/CR1: existing latch replaces redundant row checkpoints

Exactly three per-row gates changed: ordinary/annotation filtering before
visibility/dedup, its post-classification gate, and reifier filtering. Each now
uses `physical.failed`, the existing operation-local first-fault latch. The
post-classification gate still suppresses the same row when its declaration probe
latches a failure. No descriptor failure is cleared or replaced by success.

Actual first materialization still checks the complete pinned source descriptors.
Point/metadata `read_error`, engine preflight/finalization, `operation_status`
and canonical-fold checkpoints retain their full authority. Inspected the real
ordinary/prepared completion routes and fold checkpoints before publication.
A raw ID iterator is not a completeness certificate; term resolution may still
legitimately perform row-dependent descriptor reads. This repair's count claim
is limited to the ID-row filter gates.

`raw/G2-descriptors-before.log` fails meaningfully: one row per stream across
two fixed source/pages raises each provider's generation/count calls from seven
after admission to ten after a cached reread. Final core measurements at
1/32/256 rows per stream retain exactly `(generation=4, count=4, materialize=1)`
per provider. Cached repeats add zero descriptor calls/materializations, and
final status is healthy. Provider-call savings are not inferred from I/O caching
alone.

The delayed publication test observes each real healthy operation's checkpoint
sequence, then mutates generation or page count after its penultimate healthy
checkpoint. Ordinary/prepared queries observe 21 checkpoints and fold 14; all
six cases refuse a complete result/artifact with ordinal-0 SourceSnapshot and
the exact generation/count cause, without another materialization. The trigger
comes from the production sequence rather than a copied hardcoded wrapper.
Existing skipped/zero-page/constants/head-only, exact receipts/budgets, typed
fault, RDF 1.2 and eager-page-byte cases also execute in affected suites.

The final cached-drift witness repairs an intermediate evidence gap: it imports
ordinary rows through the native DatasetImporter and adds two actual reifier and
annotation rows. Every stream asserts two healthy rows, opens an iterator and
yields one row before drift. Final status then latches either drift kind and
each real buffered second row is suppressed. Later ordinary/annotation/reifier
reads remain empty after provider recovery. A separate declaration-provider
fault suppresses the very ordinary candidate that caused classification failure.
The strengthened witness passed on the final candidate; no vacuous empty-side
iterator assertion is used to establish immediate suppression.

## Attributable checks and reuse

Read final `tasks/G2-implementation.md` Status SUCCESS with exact command/exit
handoff and ownership release. Every final check below reports exit 0. Cargo
uses `CARGO_BUILD_JOBS=2`, locked resolution and unchanged governed profiles.
`raw/G2-toolchain.txt` captures rustc 1.100.0-nightly,
`4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1. Prior effective profile
evidence applies to unchanged compiler/configuration: opt-level 3 with assertions
and overflow checks, test debug 0; it is not relabeled a new full profile run.

| Executed command | Result / receipt |
| --- | --- |
| `cargo test --locked -p purrdf-core --lib ir::paged -- --nocapture --test-threads=1` | 18 pass; final candidate/admission production measurements; `raw/G2-paged-unit-qualified.log` |
| `cargo test --locked -p purrdf-core --test paged_stack --test paged_backend --test paged_fallible --test paged_admission_law --test mutable_declared_graph_drain -- --nocapture --test-threads=1` | 85 pass: 22/34/12/11/6; `raw/G2-core-qualified.log` |
| `cargo test --locked -p purrdf-core --test paged_stack cached_id_rows_require_a_final_checkpoint_and_a_latched_fault_suppresses_open_iterators -- --nocapture` | Final strengthened fixture: one pass, 21 unrelated filtered; `raw/G2-side-iterator-qualified.log` |
| `cargo test --locked -p purrdf-sparql-eval --test paged_stack_query --test fallible_query --test paged_query_e2e --test delta_view -- --nocapture` | 33 pass: 8/17/7/1; `raw/G2-evaluator-first.log` |
| `cargo clippy --locked -p purrdf-core -p purrdf-sparql-eval -p purrdf-rdf -p purrdf --all-targets -- -D warnings` | Warning-free; `raw/G2-clippy-qualified.log` |
| `cargo clippy --locked -p purrdf-core --all-targets -- -D warnings` | Warning-free after final fixture strengthening; `raw/G2-clippy-final-test.log` |
| `cargo run --locked -p purrdf-sparql-eval --example paged_stack` | Retained Bob 2 pages/202 bytes, current Robert 4 pages/1088 bytes, qualified origins, one folded page with actual ordered carrier equality; `raw/G2-example.log` |
| `cargo test --locked -p purrdf --lib facade_exposes_the_completed_umbrella` | Actual root facade smoke passes one; 43 unrelated filtered; `raw/G2-facade.log` |
| `cargo build --locked --release --target wasm32-unknown-unknown --lib -p purrdf-core -p purrdf-sparql-eval -p purrdf-rdf -p purrdf` | Selected library release compilation completed; `raw/G2-wasm-qualified.log` |
| `make helpers-hygiene` | 81 jobs, 23 reasoned variants, no open copies, 91 distinct rows, 1833 files, 80 prefix-free domains; `raw/G2-helpers-qualified.log` |
| `make thread-local-hygiene` | Self-tests and 53-static inventory pass; `raw/G2-thread-local.log` |
| Six-file rustfmt plus final changed-test check; working/staged whitespace checks | Clean; `raw/G2-format.log`, `raw/G2-format-final-test.log`, `raw/G2-whitespace.log`, `raw/G2-staged-whitespace-final.log` |

The only intervening delta after the 85-case core run strengthens one integration
fixture; production and other cases are unchanged. Its focused final rerun and
affected all-target core clippy qualify the changed input. Earlier four-package
clippy/evaluator executions retain unchanged production/evaluator/facade inputs,
as confirmed by pre-clippy and final manifests; only a redundant provider clone
was moved and the later side fixture strengthened. The example, facade, helper
and wasm receipts qualify final production. Earlier pre-fast-path unit output is
historical; the final 18-case serial run qualifies the final admission behavior.
No check is represented as rerun when it was reused. No assertion or gate was
weakened, dependency/feature/exemption added, or source requirement cut.

No required G2 work remains. Wasm evidence proves compilation only. This report
does not establish full-workspace/conformance execution, hosted CI, G3 assembly
projection, signed/hooked commit, push/readback, feedback publication, final
completion audit or integration. Those distinct parent-owned gates remain.
