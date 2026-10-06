# Task 1 implementation handoff

Status: SUCCESS — complete core mechanism, contract and attributable core qualification.

Worktree: `/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
Branch: `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
Assigned source HEAD/base: `c1d5bcd259089769e3679c6e1926b5ce24fbc1c2`.
Plan SHA-256: `ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29`.
No source commit, push, publication, merge, toolchain/configuration change or sibling
process termination was performed by this implementer. Parent owns the independent
final verdict, signed hooked commit, remote verification and issue update. Task 2
consumer acceptance follows that handoff; this report makes no whole-issue claim.

## Stable source identity

`raw/T1-source.patch` contains the complete tracked diff against the assigned HEAD
and the two new source/test files as binary no-index additions; it excludes Stage
artifacts. SHA-256:
`60e5224cdc4e0578ed09ae3bf69b1b5fead667c252685ababf950ad6604282f3`.
`raw/T1-source-sha256.txt` records every changed source/test/contract file hash.
All final qualification commands below ran against these final file contents.
The earlier global/mutable unit runs predate only the added native-overlap
integration regression and final contract wording; their applicable production
source hashes are unchanged, as the manifest and preliminary review captures show.

Files:

| File | Concrete change |
| --- | --- |
| `crates/rdf-core/src/ir/paged/stack.rs` | Public layered writer, immutable owned snapshot, flattened routing provider, logical query view, exact evidence and deterministic typed-row canonical seal |
| `crates/rdf-core/src/ir/paged/query.rs` | Shared descriptor checkpoints and indexed physical stream probes/estimates through the existing admission/cache/budget/latch |
| `crates/rdf-core/src/ir/paged/provider.rs` | Default complete generation/page-count checkpoint extension |
| `crates/rdf-core/src/ir/paged/mod.rs` | Stack exports and typed page-count/source-snapshot errors with chained causes |
| `crates/rdf-core/src/ir/global.rs` | Checked metadata term/dependency re-interning through the existing dictionary home |
| `crates/rdf-core/src/ir/mutable.rs` | Ordered/hashable owned quad values and shared statement classification home |
| `crates/rdf-core/src/ir/mutable/delta_view.rs` | Existing delta visibility and indexed probes use the shared classification rule |
| `crates/rdf-core/src/ir/mod.rs`, `crates/rdf-core/src/lib.rs` | Public root/IR exports |
| `crates/rdf-core/tests/paged_stack.rs` | 19 focused public integration regressions, independent value-set histories and typed eager page-byte comparisons |
| `docs/design/purrdf-backend-contract.md` | G11 public costs, chronology, partition, checkpoint, graph-lifetime, head-charge, canonical-page and receipt laws |

No dependency, feature, manifest, generated source or helper-ledger exemption was added.

## Public API for consumer work

Root exports:
`PagedStack`, `PagedStackSnapshot`, `PagedStackQueryView`, `PagedStackEvidence`,
`PagedStackError`, `StackSource`, `StackPageOrigin`, `CanonicalPagedError`,
`canonical_paged_seal`.

```rust
PagedStack::new(Vec<Arc<PagedDataset>>) -> Result<PagedStack, PagedStackError>
stack.append(Arc<PagedDataset>, Vec<QuadValues>) -> Result<(), PagedStackError>
stack.contains(&QuadValues) -> Result<bool, PagedStackError>
stack.contains_with_limits(&QuadValues, PagedQueryLimits) -> Result<bool, PagedStackError>
stack.insert(QuadValues) -> Result<bool, PagedStackError>
stack.remove(&QuadValues) -> Result<bool, PagedStackError>
stack.declare_named_graph(TermValue) -> Result<bool, PagedStackError>
stack.snapshot() -> Result<PagedStackSnapshot, PagedStackError>
stack.seal_head(NonZeroUsize) -> Result<(), PagedStackError>
stack.compact(PagedQueryLimits, NonZeroUsize)
    -> Result<PagedDataset, CanonicalPagedError<PagedQueryError>>

snapshot.query_view(PagedQueryLimits) -> PagedStackQueryView<'_>
snapshot.dictionary() -> &GlobalDictionary
snapshot.sealed_depth() -> usize
snapshot.compact(PagedQueryLimits, NonZeroUsize)
    -> Result<PagedDataset, CanonicalPagedError<PagedQueryError>>

canonical_paged_seal<D: FallibleDatasetView>(&D, NonZeroUsize)
    -> Result<PagedDataset, CanonicalPagedError<D::Error>>
```

At least one base descriptor is required; a base may have zero pages. Bases and
appended generations are oldest first and keep their independent dictionary/PageId
spaces. Pending head values, removals or graph declarations must be sealed before
append, preventing implicit mutation reordering. All boolean membership/mutation
answers are fallible. Query views implement DatasetView and FallibleDatasetView
with `Error = PagedQueryError`, `Evidence = PagedStackEvidence`. Call the guarded
consumer entry point and inspect terminal readiness before publishing results.
Reuse one immutable snapshot across query operations.

Evidence owns exact ordered source generation/page-count descriptors, head rows,
head graph declarations, each layer/head removal set and retained explicit graph
declaration sets; aggregate paging receipts have qualified requested origins and
exact head reference-byte charge. This includes skipped and zero-page sources.

## Mechanism and complete core acceptance

Construction/append/snapshot only read certified metadata for lower generations;
the snapshot remaps independent dictionaries by RDF value, including nested term
dependencies. A single private physical PagedDataset routes all pages and the
resident head into the existing PagedQueryView. There is no per-layer cache/budget
or eager full-stack RdfDataset on the read/mutation/head-seal path. Snapshot cost
is O(retained dictionary values + translations + copied head/removal metadata),
including dependency terms; repeated publications can accumulate quadratic work.
Mutable membership checks head state first and reuses sealed snapshot metadata
within the batch, while each operation owns a fresh I/O cache.

Chronological tombstones suppress all older copies and newer reinsertion wins.
The latest visible ordinary/annotation occurrence chooses physical typing before
logical filtering; annotation wins a same-source overlap. Native primary typing
beside own-source reifiers is preserved; external graph-scoped reifiers promote
primary rows. An annotation demotes only if a declaration was visible at its
introduction prefix and no effective declaration remains. Explicit orphans remain
typed. Native Primary/Reifier value overlap retains both separate streams. These
rules use the same classifier as DeltaDatasetView, retaining source-specific
visibility and indexed cursors.

Every checkpoint directly compares every underlying generation AND page count;
no combined hash proves equality. Point metadata reads and head rows use the same
sticky fault gate. Admission keeps the existing first-request receipt law and
single-source stale-generation requested-page address. Raw summaries remain exact;
logical cardinalities are conservative. Ordinary probes retain native positional
indexes, side subject probes retain native sorted-run indexes, and all classification
reads pass through the same page accessor. Typed faults preserve original error
causes and qualified routes. Failed folds yield no partial artifact.

Head admission charges one deterministic reference-byte page once per operation;
empty head contributes no page. Sealed heads charge actual encoded bytes. Explicit
head graph lifetime survives populated seals and subsequent last-row removals,
while implicit populated graphs disappear after their final row. Plain compaction
preserves the current DatasetView surface; DatasetView cannot encode the extra
populated-explicit-versus-implicit writer lifetime distinction. G11 documents that
base replacement carries that additional declaration policy in the consumer.

Canonical partition orders typed records ordinary/reifier/annotation/declaration-only
graph, then QuadValues tuple/value order. Each row or declaration counts toward
the positive per-page bound; zero records produce zero pages. Rows are rebuilt
through the existing typed builder/pack/seal homes and global dictionary canonical
remapping. Tests compare actual ordered PackBuilder bytes for each page, not a
translation proxy, against independently constructed effective typed eager inputs.
Scoped blanks, nested triples, directional literals, graph-scoped reifiers,
annotation-only pages and declared empty graphs survive.

| Core obligation | Named public regression/evidence |
| --- | --- |
| Independent bases/deltas, colliding global/PageIds; metadata-only setup | `construction_append_and_snapshot_do_not_read_lower_pages` |
| Mutable batch, same-head sequences, multiple older copies, reinsertion/isolation | `chronological_removal_reinsertion_same_head_and_snapshot_isolation`; 64-step `deterministic_value_set_mutations_match_an_independent_oracle_at_each_snapshot` |
| Full-vector drift, zero/skipped sources, page-count mismatch, direct metadata/head gates | `drift_of_skipped_or_zero_page_sources_gates_the_entire_head_and_metadata`; `page_count_drift_is_checked_without_materialization_and_is_sticky`; `metadata_point_reads_detect_source_drift_without_an_earlier_status_checkpoint` |
| Inclusive/zero/below global limits, exact head charging/cache, no partial fold | `head_charges_once_under_global_limits_and_folds_have_no_partial_success`; construction receipts; existing paged limits/admission tests |
| Typed provider/cancel/deadline/charge/layout/summary/midread drift faults | `typed_faults_changed_charges_and_layouts_refuse_reads_mutation_and_fold`; `summary_drift_is_refused_even_with_equal_local_values_counts_and_charges` |
| Graph/subject pruning and sound classification; one-versus-last reifier removal | `graph_pruning_partial_removals_empty_declarations_and_scoped_head_values_survive`; `annotation_only_pages_demote_only_after_last_original_reifier_in_the_graph`; `new_reifiers_promote_older_primary_rows_only_in_the_matching_graph` |
| Chronological orphan association and ordinary/annotation exclusivity | `orphan_annotation_association_uses_its_visible_introduction_prefix`; `newer_typed_occurrence_wins_before_partition_and_native_primary_typing_is_retained` |
| Native separate Primary/Reifier overlap | `native_equal_primary_and_virtual_reifier_rows_retain_both_streams` |
| Explicit versus implicit graph lifetime | `explicit_head_graph_lifetime_survives_sealing_while_populated_then_last_row_removal`; `sealed_implicit_graph_disappears_when_its_last_effective_row_is_removed` |
| Actual ordered page bytes, global dictionary, complex terms, annotation-only and empty output | `canonical_fold_matches_independent_eager_typed_surface_and_dictionary_for_two_histories_and_bounds`; deterministic mutations/orphan/graph lifetime/overlap regressions also compare eager bytes |
| Invalid input and append chronology hard failures | `invalid_rows_never_mutate_the_head_and_pending_head_never_reorders` |
| Shared delta classification remains lawful | 42 `ir::mutable` unit tests, including delta_view regressions, and six declaration-drain integration tests |

## Independent preliminary review remediation

The historical BLOCKED draft remains intact in `tasks/T1-review-draft.md` and its
repair recheck records no additional established source defect. Final independent
adjudication belongs to the parent/reviewer after this stable handoff.

| Finding | Repair and attributable regression |
| --- | --- |
| R1 historical declaration incorrectly demotes later orphan | Prefix-bounded declaration visibility including chronological tombstones, excluding future sources; orphan prefix regression asserts typed counts and eager actual page bytes |
| R2 direct metadata bypasses composite descriptor gate | `read_error` delegates to `check_snapshot`; metadata regression calls term lookup/resolve/graph reads immediately after source drift without first latching status |
| R3 populated explicit head graph loses lifetime after seal | Layer retains explicit graph declarations separately; snapshot/evidence pin them; declare/seal/remove/seal regression preserves graph and eager bytes; plain carrier boundary documented |
| R4 subject probes scan complete side tables | Native `reifier_quads_of` and `annotations_of_with_graph` indexed runs retained through same physical accessor; existing pruning and shared delta tests pass |

Parent refinements also repaired scoped-blank head replay with DatasetImporter,
newest physical ordinary/annotation occurrence precedence, exact standalone drift
page receipts, checked dictionary capacity, empty graph record policy and accurate
head charging. No exemption was introduced to quiet the helper census.

## Exact qualification commands and observed exits

All commands ran from the worktree above. Cargo used the live governed floating
nightly/Stage slot and repository default profiles without override: dev/test
opt-level 3, debug assertions and overflow checks enabled; test debug=0. Every
Cargo/make invocation below set `CARGO_BUILD_JOBS=2`.

| Exact command | Exit | Durable stdout/stderr and result |
| --- | --- | --- |
| `CARGO_BUILD_JOBS=2 cargo check --locked -p purrdf-core` | 0 | `raw/T1-check-second.log`; optimized dev build |
| `CARGO_BUILD_JOBS=2 cargo test --locked -p purrdf-core --test paged_stack --test paged_backend --test paged_fallible --test paged_admission_law --test mutable_declared_graph_drain` | 0 | FINAL `raw/T1-focused-qualified-2.log`; 82 pass: stack19/backend34/fallible12/admission11/graph-drain6, zero failures |
| `CARGO_BUILD_JOBS=2 cargo test --locked -p purrdf-core --lib ir::mutable` | 0 | `raw/T1-mutable-qualified.log`; 42 pass, zero failures,1113 filtered |
| `CARGO_BUILD_JOBS=2 cargo test --locked -p purrdf-core --lib ir::global` | 0 | `raw/T1-global-qualified.log`; 23 pass, zero failures,1132 filtered |
| `CARGO_BUILD_JOBS=2 cargo clippy --locked -p purrdf-core --all-targets -- -D warnings` | 0 | FINAL `raw/T1-clippy-handoff.log`; successful optimized dev profile, no warnings |
| `CARGO_BUILD_JOBS=2 make helpers-hygiene` | 0 | `raw/T1-helpers-qualified.log`; self-tests, all81 enforced ledger jobs/23 variants/91 distinct rows/1828files hold; 80 domains current; zero duplicate-home findings |
| `rustfmt --edition 2024 --check crates/rdf-core/src/ir/global.rs crates/rdf-core/src/ir/mod.rs crates/rdf-core/src/ir/mutable.rs crates/rdf-core/src/ir/mutable/delta_view.rs crates/rdf-core/src/ir/paged/mod.rs crates/rdf-core/src/ir/paged/provider.rs crates/rdf-core/src/ir/paged/query.rs crates/rdf-core/src/ir/paged/stack.rs crates/rdf-core/src/lib.rs crates/rdf-core/tests/paged_stack.rs` | 0 | `raw/T1-format-qualified.log`; empty output |
| `git diff --check` | 0 | `raw/T1-whitespace-qualified.log`; empty output |

Historical diagnostic logs are retained and do NOT qualify the final tree:
`T1-check-initial.log` (exit101, initial type errors),
`T1-stack-initial.log` (exit101, test compilation),
`T1-stack-second.log` (exit101,12pass/3 expectation failures),
`T1-clippy-initial.log` (exit101, production lint repairs),
`T1-clippy-final.log` (exit101, test lint repairs),
`T1-clippy-qualified.log` (exit101, intermediate test edit syntax),
`T1-delta-final.log` (exit101, intermediate empty-set inference).
The fixed intermediate passing logs (`T1-stack-third`, `T1-focused-final`,
`T1-focused-qualified`, `T1-clippy-qualified-2`) remain historical receipts, not
substitutes for the final named qualification above.

## Handoff boundary

Source/tests/contract are stable; no concrete Task 1 implementation or core
qualification gaps remain. No Cargo process from this implementer remains active.
Writer ownership is released to the parent after this report. The mandatory commit
hooks were not run because this child is not authorized to commit; the parent must
run them through the normal signed commit. No evaluator, example, wasm, hosted CI,
integration-candidate or whole-issue acceptance is inferred from core checks.
