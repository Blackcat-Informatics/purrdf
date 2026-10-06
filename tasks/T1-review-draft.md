# Task 1 independent source review — preliminary

VERDICT: BLOCKED (PRELIMINARY; source is changing and qualification is pending).

Issue: 457. Branch: `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
Worktree: `/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
Assigned source HEAD/base: `c1d5bcd259089769e3679c6e1926b5ce24fbc1c2`.
At the latest identity read, HEAD remains that commit and local `origin/main`
has advanced to `b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac`. This review uses
the assigned base, not an unassessed integration candidate.
Authoritative plan SHA-256:
`ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29`.

## Scope and evidence

Read applicable AGENTS.md, standing parent `.baseline` and `.goals`, the empty
emergency ledger, C/G backend contract, supplied issue with zero comments,
issue-analysis, prior-art-assessment, plan-review and current validation index.
Read every changed/new rdf-core source and the new paged_stack test target;
checked native indexed side-table methods and mutable classification as callers.
Applied Stage task-review, quality, delegation, no-deferrals and validation
guidance. No additional forge retrieval, source edits, Cargo/build/runtime
checks, commit, push or publication occurred. This report is independent source
judgment, not an independent runtime audit. Only this draft report was written.

The validation index still records intake rather than attributable Task 1
qualification. Implementation is actively changing. A stable source/check
handoff is required before final adjudication.

Initial full-read identities:

| File | SHA-256 |
| --- | --- |
| ir/paged/stack.rs | ae6e4396b0280e4130779adc0de70c2ff35b363769e28f0a96f6020b832e62f7 |
| ir/paged/query.rs | 58580770e23537a9e3b6f80e6177a7b0438344629849c60f2164f2757112ac6e |
| ir/mutable.rs | ed0be2d2259e755333740ff9a8a963993777c5342dcf0ded6dcd99e20bb76f89 |
| ir/mutable/delta_view.rs | afae8728b15f4090dad68485a6dce229a7062c2a9db6505abea68d3fe9314fe8 |
| tests/paged_stack.rs | 43f5c51d80aa42403c289cbeae036c5d71830d939bf1263f3be6272f140aa2fc |

Subsequent read captured stack.rs
`b3b60b64a799a2feb55f9a46cfcbf8e12b315da74dfaaa91d4d3e9611540eaaa`,
tests/paged_stack.rs
`94b387e8b59df8edd3317f359f9a13bbefaba60b7a1b98133a6e9c7425026142`,
global.rs
`1e70fa87b947871e2475df520f4203bf84f7a9b0aa3d715c7dc3c3107ece6d8d`,
and delta_view.rs
`e1749ba08186b1fd492191e25db0b46bb2cbccd69626e47e7144f0a2ab21a1bb`.
The global checked interning and remaining shared classifier wiring were read
as they arrived. The tracked working diff SHA-256 at that capture was
`0a2c727b1ccb5db5a9b6ba5e14fc1e3d4527feefbbb18fad988a65a2cbbef092`;
this hash excludes untracked new source files, whose identities are listed
separately. No mixed changing snapshot here constitutes a qualified final tree.

## Findings sent immediately to the parent

### R1 — historical removed reifiers demote newly introduced typed orphan annotations

Status: OPEN; correctness/compaction history-independence blocker.

In stack.rs `logical_rows`, `original` is obtained from
`declarations(subject, graph, false)`. The false branch accepts every physical
reifier, without visibility or chronology. It is not evidence that the selected
annotation occurrence ever had an applicable declaration.

Concrete counterexample: layer 0 contains a reifier for r in graph g; layer 1
removes that reifier; layer 2 introduces an explicitly typed orphan annotation
r/p/o in g. At layer 2 insertion there is no effective reifier. The current
code finds the historical physical reifier, computes original=true and
effective=false, and moves the annotation into the ordinary table. A stack
containing only the same typed orphan instead retains it as an annotation.
Thus equal final typed surfaces can fold to different per-page bytes solely
because one history retains a withdrawn declaration. The native shared
classifier comment explicitly preserves an orphan unless its originally
applicable declaration disappears.

Required repair/evidence: determine original applicability at the selected
occurrence's chronology, preserving cross-page/cross-layer declarations that
actually applied and rejecting already-withdrawn historical declarations. Add
the explicit removed-reifier-before-orphan history and compare typed stream
counts and each compacted page byte to independently built orphan eager input.
Also retain the existing last-surviving-reifier demotion regressions.
This counterexample was inferred directly from source, not executed here.

### R2 — metadata reads do not perform composed snapshot checkpoints

Status: OPEN; full-vector G9 read gate blocker.

query.rs `PagedQueryView::read_error` still reads provider.generation directly.
The composed StackProvider always returns INITIAL. `page` and
`operation_status` now correctly use provider.check_snapshot, but
`resolve`, `term_id_by_value`, `named_graphs` and `has_named_graph` rely on
read_error. A source can drift and these metadata-only calls can still yield
terms or an explicitly empty graph until some caller first checks status or
requests a page. Page-count drift has the same gap even on an ordinary provider.

The current skipped/zero-page drift regression first invokes operation_status,
which latches the error before checking metadata methods and therefore masks
this path. Route read_error through the same full descriptor check, retaining
the first typed failure. Add metadata reads immediately after source drift,
without an earlier status call, including a zero-page source and page-count
drift. Preserve the first-request receipt law while correcting this gate.
No runtime reproduction was run by this reviewer.

### R3 — graph declaration lifetime needs a concrete consistent public law

Status: OPEN policy/API mismatch for adjudication; do not silently descope.

The public writer says it declares a graph independently of rows. A caller can
insert a row in g, declare g, seal_head, then remove the last row. The current
canonical partition omits Graph(g) when g is populated and snapshot empty_graphs
retains only graph names with zero physical rows. The declaration is therefore
lost across this seal. If the caller removes the row before sealing, the same
declaration survives. Lifetime depends on batching.

The deliberate canonical declaration-ONLY record policy is sound for implicit
graphs: emitting graph-only records for every populated implicit graph would
remember graphs incorrectly. It does not by itself define the public explicit
declaration operation's lifetime. Either retain explicit declaration state
independently within the authorized mechanism, or state and justify the chosen
limited law so the public claim and tests match it. Test both operation orders
and a seal boundary. This is not a request to invent a generic implicit/explicit
distinction that DatasetView does not expose.

### R4 — new subject-bound side probes discard existing indexes

Status: OPEN performance/one-path concern.

query.rs stream_pattern_range calls page.reifier_quads and
page.annotation_quads for subject-bound reads, then filters every row. Native
reifier_quads_of and annotations_of_with_graph already locate the matching
sorted run in O(log n). The existing paged view consumes these indexes.
Every ordinary candidate now probes external_declaration, so this fallback
can multiply an ordinary scan by whole admitted reifier-table scans.

Preserve subject-indexed side-table paths through the shared accessor, keeping
exact physical summary pruning and one cache/budget/error latch. No benchmark
or quantitative speed claim was made here. Source establishes the lost index.

## Positive source observations, with practical limits

- Constructor/append/snapshot source checks and metadata remapping do not
  materialize lower pages; the frozen resident head is the only eagerly read
  new source. Snapshot work is accurately documented as linear in retained
  dictionary values/translations; sealed metadata is cached for a mutable batch.
- Chronological per-layer tombstones and newer occurrences implement lower
  removal/reinsertion rather than union-minus-all-removals. Newest-first logical
  dedup is applied before stream partition, including primary/orphan overlap.
- Physical base and annotation candidates are both consulted for logical
  reads, and existing exact base admission is retained. Annotation/reifier
  predicate/object tests are conservative presence checks followed by filtering.
  Cardinality estimates add both physical streams and materialize nothing.
- One physical PagedQueryView carries cache, first-request order, global page
  and byte limits and sticky failure. Stack faults route back to dense physical
  pages, with qualified origins in the receipt. Complete source vectors include
  zero-page/removal-only layers; head values/graphs and ordered removals are owned.
- Fold checkpoints guard collection and final publication. Canonical typed
  records/page bounds, actual PackBuilder bytes and canonical global renumbering
  reuse existing homes, rather than translation-only compaction. Empty effective
  input produces zero pages. No runtime dependency or feature was added.
- The shared classifier refactor preserves DeltaDatasetView's prior visibility,
  reifier and duplicate conditions by inspection; regression execution remains
  necessary. Later global checked interning changes were read, but are not
  qualified by this preliminary review.

## Test and caller evidence still required

The new tests contain useful public stack scenarios, independent ordinary-value
set mutation sequences, explicit typed RDF 1.2 cases, page-byte/dictionary
comparisons and operational faults. They do not establish executed behavior
until attributable logs bind the current source and required profiles.

surface() collapses typed streams into one BTreeSet; it can hide partition bugs
and duplicate rows. Explicit stream-count assertions exist in selected tests;
the new R1 history and fold roundtrip need those observations, not just surface
set equality. Eager input construction is separate from the stack, while both
deliberately use the one canonical partitioner; this is appropriate for byte
identity, provided the independently established typed effective input is right.

Task 2 owns real guarded NativeSparqlEngine consumer equivalence and wasm
demonstrations; none was observed here. That is planned work rather than an
invented Task 1 defect. Task 1 required focused core/delta/paged regressions,
clippy and shipped contract/API costs have not yet been handed off as current
evidence. The backend contract was still unchanged at the inspected identity.
No final Task 1 PASS, runtime audit, commit readiness or whole-issue completion
is established by this draft.

## Repair recheck — source judgment, final identity handoff pending

The implementation subsequently repaired R1-R4. This appended disposition
preserves the historical findings above; it does not relabel the original
changing-source review or failed historical commands as passing qualification.

- R1: declarations now probe visibility at the selected occurrence's introduction
  prefix, including removals through that layer and excluding future layers.
  Regression `orphan_annotation_association_uses_its_visible_introduction_prefix`
  covers both already-removed older declarations and temporarily visible future
  declarations. It asserts ordinary/annotation counts and actual page-byte parity
  with an independently built explicit orphan eager input.
- R2: `PagedQueryView::read_error` now calls the same provider.check_snapshot
  descriptor home as status and first page admission, retaining the sticky cause.
  The immediate metadata regression calls term lookup/resolve/has_named_graph
  directly after zero-page source drift, before sampling status. Logical page
  candidates still enter the existing page accessor before row filtering, so an
  attempted first page retains its receipt, whereas an initial point-read
  checkpoint fault produces no invented page request.
- R3: Layer retains explicit declared_graphs separately from typed page rows;
  seal_head takes the immutable declaration set into the new layer. Snapshots
  preserve those values and evidence exposes graph_declarations. The regression
  declares g while populated, seals, removes the last row and seals again, then
  compares the remaining graph and exact folded page bytes to an independently
  constructed declaration-only eager input. G11 honestly distinguishes this
  writer metadata from plain PagedDataset compaction's current DatasetView
  surface: retaining writer lifetime across base replacement is consumer policy.
- R4: subject-bound side probes now call native reifier_quads_of and
  annotations_of_with_graph, retaining existing sorted-run indexes without
  adding per-page boxes or a second page admission/cache path.

Current inspected SHA-256 captures during this repair recheck:

| File | SHA-256 |
| --- | --- |
| ir/paged/stack.rs | 8d6740b71ef877023c8f546f2c0f1e5b8befaff8a7265a95ee578d3af5bb1af0 |
| ir/paged/query.rs | 96f94a5523a582657d56f6ade9b9f355d00e0ae3cc41413d28e581ada510012f |
| tests/paged_stack.rs | 041a9a8bc339ee5aea298cac1bad08b631af8fea41f9a00803ac9940a5af9579 |
| docs/design/purrdf-backend-contract.md | 5b83aae8a4cc050e78e3bf09ffa7075bf8eed1b145cce35ca3967d9f1076c53a |

The changed global and mutable source captures remain as listed above. These
remain pre-handoff observations while formatting/clippy repairs may occur.

Read supplied runtime logs rather than executing Cargo:

| Log | Observed result |
| --- | --- |
| raw/T1-focused-final.log | 81 pass: declared_graph_drain 6, admission_law 11, paged_backend 34, paged_fallible 12, paged_stack 18; zero failures |
| raw/T1-global-qualified.log | 23 global unit tests pass; 1132 filtered |
| raw/T1-mutable-qualified.log | 42 mutable tests pass, including delta_view regressions; 1113 filtered |
| raw/T1-clippy-qualified-2.log | Ends in successful Finished dev profile; exact invocation/exit handoff remains pending |
| raw/T1-delta-final.log | Historical compile failure caused by wrong empty-set type, subsequently repaired |
| raw/T1-clippy-final.log | Historical test lint failures, subsequently repaired |

The native G3 seal expressly maintains separate stream ledgers because ordinary
and reifier virtual quads may legitimately share the same value tuple. Therefore
reifier-overlap suppression is not an invented new requirement. The new G11
dedup prose should narrow its “at most one logical statement stream” claim to
ordinary/annotation streams; this clarification was sent to the parent.

No additional source correctness defect was established in this recheck. Final
Task 1 adjudication remains pending a stable source identity and full attributable
command/exit evidence, including confirmation of which qualified clippy log is
current. No final PASS or independently executed runtime audit is claimed here.
