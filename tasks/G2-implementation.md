# G2 implementation

Status: SUCCESS — PF1 and PF2/CR1 are implemented and all assigned G2 checks passed.
PF1 HIGH and PF2/CR1 MEDIUM are implemented coherently. No whole-issue,
hosted-CI, signing/publication or integration claim is made by this report.

## Source and authority

Parent HEAD 6aa17ec81526d270160827586a6f695d064a1f31, committed tree
b5a7e8cc9330b199c4f5c33f99c4f7b33bfc3156. Isolated assigned worktree and
branch remain unchanged. Parent subsequently staged exactly the frozen six files,
producing first candidate tree ac992a403f98d5aad9bedb8ba821e8e42fd649e8,
subsequently superseded by a one-file test-fixture strengthening. Final source
identity is the six-file manifest below; parent verified and restaged it as
fc174197e0ca974f5a5ecd0c55aea760419cadd0.
This implementer did not stage files. Read applicable AGENTS.md, root .baseline/.goals,
backend contract, Stage 2 workflow and validation/quality/no-deferrals,
remediation-plan.md, gap-analysis.md, gap-feedback-addendum.md, specialist report,
current validation index and existing task/consumer evidence. The root explicitly
extended ownership to interleaving.rs for the required test-only inventory.
No dependency, feature, manifest/toolchain/workflow, generated artifact, G1 source,
sibling worktree or private memory change. No staging, commit, push or forge mutation.

Final six-file manifest: raw/G2-source-sha256.txt. Full tracked patch against the
parent HEAD: raw/G2-source.patch, SHA-256
b98f78f43a5655f11c135e1fccbd859d377f35a5bb0da4a6b729d56bb884e569.
The pre-clippy and pre-side-iterator snapshots are preserved separately and
are not the final source. The only later change after broad core qualification
strengthens the cached drift fixture with two real rows in each typed stream
and pending ordinary/reifier/annotation iterators; production hashes did not change.

- ir/paged/admission.rs extends the existing candidate home with range restriction.
- ir/paged/query.rs routes raw probes and estimates through that home and retains
  private test-only counters at actual production boundaries.
- ir/paged/stack.rs changes only the three per-row filter gates to physical.failed.
- core/tests/paged_stack.rs measures real provider descriptors and tests sticky drift.
- eval/tests/paged_stack_query.rs exercises delayed cached publication refusal.
- rdf-wasm/src/interleaving.rs inventories the two private cfg(test) counters using
  Safety::NotCompiledIn and the pre-existing TEST_ONLY reason.

## Implemented behavior

candidate_pages_for_stream delegates to its range-aware form. Named and Default
use the existing stream-specific graph postings, whose sorted slices are bounded
before any slot is examined. Each nontrivial lower/upper boundary uses partition_point;
other-layer postings are not filtered/iterated. Start zero and end page_count
require no searches, preserving the existing full-range native candidate cost.
Any addresses the requested dense physical range directly. Empty ranges yield no
candidates. The range is bounded by the sealed resident page count.

Raw stream_pattern_range and stream_estimate map these exact candidates to the
existing slots, then perform their existing axis summary admission. Indexed ordinary
and subject-bound reifier/annotation scans, row graph/predicate/object checks,
ascending physical order, layer chronology, and the one physical page accessor,
cache, byte/page budgets, fault latch and unconditional page certification remain.
No new graph index, admission law, reifier materialization table or public metrics API.

logical_rows uses physical.failed before visibility/dedup and after classification.
reifiers uses the same latch before visibility/dedup. The post-classification gate
still drops a candidate whose declaration probe faults. Source descriptor checks
remain in point/metadata read_error paths, actual materialization, operation_status,
engine preflight/finalization and every canonical-fold checkpoint. A raw iterator
is not a completeness certificate. Resolving public term values can still cause
row-dependent descriptor calls; the reduced-count claim covers ID filter gates only.

## Meaningful witnesses and operation counts

The exact new tests ran against the original production seams before repair:

- raw/G2-candidates-before-3.log, exit101: eight sparse probes visited24 slot
  summaries instead of8; a plain two-page/two-row logical drain visited4 Reifier
  summaries even though the graph's Reifier postings were empty. Both tests fail.
- raw/G2-descriptors-before.log, exit101: at one row in each of two streams over
  two fixed source/pages, each provider had7 generation/page_count calls after
  admission and10 after the cached reread. The cached-filter assertion fails.
- Earlier raw/G2-candidates-before.log and -2.log are failed test compilation
  attempts (missing test trait import and unnecessary qualification), not witnesses.

The retained instrumentation is cfg(test), private, thread-local per test worker,
and registered in the required inventory. It increments actual production posting
comparison and stream_admitted boundaries, not a replica or timing estimate.
Provider counts are from generation/page_count/materialize at actual trait methods.

Final operation shapes:

- Sparse target graph: physical pages3,67,1027; a fixed three-page chronological
  range contains one row of each Base/Reifier/Annotation stream. The other pages
  use the opposite graph kind. Each stream's eight repeated Default/Named probes
  visit exactly8 summaries, estimate visits1, Any range visits3 once, empty ranges0.
- Long postings:8,128,1024 pages each contain one default and one named Reifier
  row. A fixed three-page middle range returns exactly its three page IDs, in order;
  actual boundary comparisons are7,15,21 for Default/Named,0 for Any. Counts fit a
  logarithmic upper bound; full-range candidates and empty postings need0 searches.
- Plain RDF logical path:1,8,32 layers, two independently sealed one-row pages
  per layer, one default and one named row. Full healthy drains and cached repeats
  return2,16,64 rows with exactly0 Reifier-summary visits. This measures the
  original ordinary-row classification bottleneck, not just provider I/O.
- ID filters:1,32,256 ordinary rows in one source page and the same number of
  distinct Reifier rows in another. Two sources/pages stay fixed. Each provider
  records4 generation reads,4 page_count reads,1 materialization. Cached repeats
  add0 descriptor calls and0 materializations; final operation_status is healthy.
- Cached final drift: admit two actual ordinary rows, run each real healthy
  operation, observe its descriptor sequence, then inject generation/page_count
  mutation just after the penultimate healthy checkpoint. Ordinary and prepared
  publication observe21 checkpoints; fold observes14. All six cases refuse the
  complete answer/artifact with SourceSnapshot ordinal0 and its exact typed drift
  cause, without any additional page materialization. The test derives the trigger
  from the production healthy operation, not copied checkpoint counts.
- Pending ordinary, reifier and annotation iterators each have one of two
  actual rows still buffered when final-status drift latches. Each suppresses
  that buffered row immediately; subsequent
  ordinary/annotation/reifier/cached reads stay empty after provider recovery.
  A declaration-provider refusal during ordinary classification drops the very
  candidate that triggered the fault, exercising the retained post-classification gate.

These are deterministic operation counts, not elapsed-time or throughput claims.
No temporary production probe, standalone benchmark or diagnostic executable remains.

## Qualification index

All Cargo commands use CARGO_BUILD_JOBS=2 and the existing Stage-owned floating
nightly. raw/G2-toolchain.txt records rustc1.100.0-nightly,
4b6d04e706108ccfeafe2547fbe857dfe8972bad, LLVM23.1.1. No toolchain was installed
or selected. Existing T2 effective-profile evidence remains applicable because
manifest, compiler, environment/profile overrides and command profile stayed unchanged:
opt3, debug assertions and overflow checks enabled, testdebug0. Tests below print
[optimized] and actual production inputs are bound by the six-file manifest.

| Command / log | Result / applicability |
| --- | --- |
| cargo test --locked -p purrdf-core --lib ir::paged -- --nocapture --test-threads=1 / raw/G2-paged-unit-qualified.log | exit0,18pass on final six-file source; raw/G2-paged-unit-first.log is earlier qualifying pre-fast-path execution |
| cargo test --locked -p purrdf-core --test paged_stack --test paged_backend --test paged_fallible --test paged_admission_law --test mutable_declared_graph_drain -- --nocapture --test-threads=1 / raw/G2-core-qualified.log | exit0,85pass (22+34+12+11+6), final production source; only pending-iterator fixture later strengthened, whose focused rerun below passed |
| cargo test --locked -p purrdf-sparql-eval --test paged_stack_query --test fallible_query --test paged_query_e2e --test delta_view -- --nocapture / raw/G2-evaluator-first.log | exit0,33pass (8+17+7+1), final production/evaluator source; unchanged after test-only core clone and ledger registration |
| cargo clippy --locked -p purrdf-core -p purrdf-sparql-eval -p purrdf-rdf -p purrdf --all-targets -- -D warnings / raw/G2-clippy-qualified.log | exit0, final production/facade/evaluator inputs; first run found one redundant last-use provider clone, fixed with a move; affected core test later strengthened and rechecked below |
| cargo test --locked -p purrdf-core --test paged_stack cached_id_rows_require_a_final_checkpoint_and_a_latched_fault_suppresses_open_iterators -- --nocapture / raw/G2-side-iterator-qualified.log | exit0,1pass/21filtered, final strengthened real buffered-side-row fixture |
| cargo clippy --locked -p purrdf-core --all-targets -- -D warnings / raw/G2-clippy-final-test.log | exit0, final candidate after one-file test strengthening; unchanged evaluator/RDF/umbrella all-target evidence reused |
| cargo run --locked -p purrdf-sparql-eval --example paged_stack / raw/G2-example.log | exit0, actual public Bob/Robert receipts and one-page canonical carrier identity verified |
| cargo test --locked -p purrdf --lib facade_exposes_the_completed_umbrella / raw/G2-facade.log | exit0,1pass/43filtered on final source; actual root facade stack/query/evidence/fold smoke |
| cargo build --locked --release --target wasm32-unknown-unknown --lib -p purrdf-core -p purrdf-sparql-eval -p purrdf-rdf -p purrdf / raw/G2-wasm-qualified.log | exit0, final affected four-package release library build; compilation only, no wasm runtime execution |
| make helpers-hygiene / raw/G2-helpers-qualified.log | exit0,81jobs/23variants/91distinct/1833files/80domains on final source |
| make thread-local-hygiene / raw/G2-thread-local.log | exit0,53statics, valid-neighbour/refusal selftests pass |
| explicit six-file rustfmt --check / raw/G2-format.log plus changed final test check / raw/G2-format-final-test.log | exit0,clean; unchanged five files reuse first check |
| git diff --check / raw/G2-whitespace.log and parent-staged final git diff --cached --check / raw/G2-staged-whitespace-final.log | exit0,clean |
| sha256sum -c final manifest / raw/G2-frozen-manifest-check.txt | six hashes verified |

All assigned G2 implementation and validation requirements are met. The final
six-file candidate is fc174197e0ca974f5a5ecd0c55aea760419cadd0 and all manifest
hashes were verified again after the last build. Existing source/configuration
checks are reused only where their inputs are unchanged, as the rows above state.
The final post-staging whitespace check and deferral scan are clean (the source
added-line rg scan returned1 because there were no matches).

No wasm runtime, full workspace check, assembly projection, hosted CI, commit
hooks, forge publication or integration was performed by this implementer. The
existing effective-profile receipt remains applicable, as described above. These
separate workflow gates belong to the parent and are not claimed passed here.
No source requirement was cut or moved elsewhere; no deficiency entry was added.

Source writer and Cargo ownership released. All own execution sessions ended.
