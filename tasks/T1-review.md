# Task 1 independent implementation review

VERDICT: PASS

This verdict qualifies the stable Task 1 core implementation, public contract and
attributable local checks for the normal signed/hooked commit handoff. It does
not claim that commit/push/issue publication occurred, that an integration tree
was qualified, or that the full issue is complete. Task 2 evaluator/example/wasm
acceptance and subsequent review/publication/merge remain outstanding.

## Exact reviewed identity and independence

Issue: 457. Worktree:
`/home/paudley/Active/purrdf/.worktrees/457-paged-tier-an-lsm-style-stack-of-sealed`.
Branch: `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
Assigned HEAD/base: `c1d5bcd259089769e3679c6e1926b5ce24fbc1c2`.
Reviewed staged source tree: `2e7207441e0adff64104179cd26ef568ad1225b2`.
Authoritative plan SHA-256:
`ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29`.
Complete source patch, including both new files:
`raw/T1-source.patch`, SHA-256
`60e5224cdc4e0578ed09ae3bf69b1b5fead667c252685ababf950ad6604282f3`.

Independently checked the staged tree identity and ran SHA-256 verification
against every entry in `raw/T1-source-sha256.txt`; all matched. Source ownership
was released and the parent held it unchanged for this final adjudication.
The advanced origin/main at `b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac`
is not the reviewed integration candidate; synchronization and affected consumer
assessment remain parent-owned work.

Read applicable AGENTS.md, standing parent `.baseline`/`.goals`, empty emergency
ledger, governing C/G backend contract, authoritative plan, complete supplied
issue with zero comments, analyses, independent plan review, validation index,
historical draft and final implementation handoff. Read all changed/new source,
native indexed/table semantics, shared mutable callers, test assertions and
attributable qualification logs. Applied Stage task-review/quality/validation
guidance. No source mutation, Cargo/build/runtime execution, forge retrieval,
commit, push, publication or lifecycle action was performed by this reviewer.
This is independent source and evidence adjudication, not independently repeated
runtime execution. Historical preliminary findings remain in T1-review-draft.md.

| Reviewed file | SHA-256 |
| --- | --- |
| crates/rdf-core/src/ir/global.rs | 1e70fa87b947871e2475df520f4203bf84f7a9b0aa3d715c7dc3c3107ece6d8d |
| crates/rdf-core/src/ir/mod.rs | 7475c8d6ceade8d375a4ded0b4fc1947ed7bc59129004d5b851db1911aa02192 |
| crates/rdf-core/src/ir/mutable.rs | ed0be2d2259e755333740ff9a8a963993777c5342dcf0ded6dcd99e20bb76f89 |
| crates/rdf-core/src/ir/mutable/delta_view.rs | e1749ba08186b1fd492191e25db0b46bb2cbccd69626e47e7144f0a2ab21a1bb |
| crates/rdf-core/src/ir/paged/mod.rs | 7158f88cdff2317a272c0aace6bc7ab057afeedecaf35eaafb9d9a95244192b5 |
| crates/rdf-core/src/ir/paged/provider.rs | e40750c7eefc8e93741d0229afdedd1d0b769f2875d3292361e81d5f9cda1c86 |
| crates/rdf-core/src/ir/paged/query.rs | fb68ae7a9dd237ff9fd2a107fe88308239e94d2408f20160b7677d3de5afc4e4 |
| crates/rdf-core/src/ir/paged/stack.rs | b5a3169ffedff4fd734d33c2e2adb2b3d5ab1cd48fb6861b3f8c494992b5423c |
| crates/rdf-core/src/lib.rs | 08dc1fd162da8106c96362cbe43db7e1369e5129f765f4e5980ef83b70958612 |
| crates/rdf-core/tests/paged_stack.rs | 6f583d191bfeb6c4d715ca8a46cf1a5e2576640ac33009b0ed92927564695947 |
| docs/design/purrdf-backend-contract.md | 87e84c03064406e0cc60a7d21af05e28d7569baf281f0e0b6da6da0eee5c0a0c |

## Task 1 completeness and production wiring

| Task 1 obligation | Source judgment and executed evidence |
| --- | --- |
| Independently sealed chronological bases/deltas, mutable value head and typed mutation answers | Public root/IR exports expose PagedStack, owned snapshot and query view; append refuses pending head reordering. New public tests exercise conflicting dictionary numbering, repeated lower facts, multiple seals, same-head operation ordering and value-based membership. |
| Removals suppress every lower occurrence; newer insertion survives | visible_at applies only strictly newer tombstones within the requested prefix. Newest-first ordinary/annotation dedup selects the effective occurrence before classification. Chronological removal/reinsertion/isolation regression and independent 64-step BTreeSet oracle passed. |
| Snapshot isolation and no lower content reads on setup/sealing | Snapshot owns copied head/removal/declaration values, retains immutable layer Arcs and remaps certified metadata by value. Counted constructor/append/snapshot regression observes zero lower reads. seal_head reads/rebuilds only its new resident batch. Retained snapshots are checked after later writer mutation/sealing. |
| Full-vector G9, including skipped/zero-page/head-only and metadata paths | StackProvider.check_snapshot checks each source's original generation and page count directly; ordinary PageProvider default does the same descriptor check. Page admission, status and read_error consume that home and retain sticky typed failures. Direct metadata drift, source/page-count drift and fault regressions passed. Guarded evaluator constants-only acceptance belongs to Task 2. |
| One global cache, budget, exact receipts and qualified addresses | The private physical composition routes every page/head through one existing PagedQueryView. First requests precede admission refusal; successfully validated pages alone consume totals. Limits are inclusive and cached rereads charge once. New head/global-limit/fault tests plus existing fallible/admission tests passed; receipts include all source descriptors, head/removal/declaration values and route-qualified origins. |
| Sound physical G10 candidates after graph-scoped RDF 1.2 classification | Both physical base/annotation streams are considered before logical partition. Exact base positional admission is retained; side subject/graph counts and conservative predicate/object presence precede admitted row filtering. Reifier presence uses the same page accessor and native subject indexes. Annotation-only demotion, multiple reifiers, promotion graph scope, pruning and summary-drift regressions passed. Logical estimates honestly remain upper bounds. |
| Canonical fold produces actual per-page byte and value-ID parity | canonical_paged_seal guardedly drains independently typed rows/graph membership, sorts canonical typed/value records, partitions by positive row bound, rebuilds through native typed builder and PackBuilder, checked-seals and canonical-remaps the dictionary. Two histories/two bounds and additional mutation/orphan/declaration/overlap tests compare every ordered output page byte and relevant dictionary values against independently constructed effective eager inputs. Empty output is zero pages; failed checkpoints return no artifact. |
| RDF 1.2 identity, partition and graph lifetime | Tests cover scoped blanks, nested triples, directional literals, declaration-only graphs, implicit graph withdrawal, explicit declarations retained across populated seals, newest ordinary/annotation precedence and legitimate native Primary/Reifier overlap. Shared classifier keeps DeltaDatasetView's prior conditions and indexed probes; affected mutable/global tests passed. |
| Hard errors, resource/address admission and repository constraints | Invalid head/removal/graph values use the native validation boundary; source faults never become false absence. Checked dictionary composition retains the existing lookup/miss-insertion home, refuses resident address/allocation failures and is private until success. No runtime dependency, feature, generated artifact edit or ledger exemption was added. All-target clippy and helper hygiene passed. |
| Public cost/utility/ownership contract | G11 documents metadata retention/rebuild cost, possible quadratic cumulative publication work, snapshot reuse, head versus sealed byte charging, chronology, physical/logical estimates, graph lifetime, exact canonical partition, no partial publication, and consumer storage/log/depth/atomic-publication ownership. It contains no issue/process reference. |

The production query path is PagedStackSnapshot.query_view → logical stack
filter/classifier → shared PagedQueryView physical accessor. It does not query an
eager scratch whole-stack dataset. Explicit compaction alone eagerly drains the
effective surface. Head reconstruction uses the existing MutableDataset typed
classification; scoped replay uses DatasetImporter, preserving blank scopes.
No obsolete second admission/accounting or statement-classification body was
introduced. The helper census verifies the concrete one-home law.

The tests' surface() helper collapses streams into a value set, so set equality
alone was not accepted as partition/multiplicity evidence. Relevant regressions
also assert stream counts; canonical tests compare typed eager page bytes and
dictionary values. The ordinary mutation oracle computes expected state
independently of private stack code. No weakened assertion or tautological
whole-stack oracle was found. Both fold and eager input deliberately use the one
specified partitioner; their effective typed inputs are constructed separately.

## Disposition of preliminary findings

| Finding | Final disposition |
| --- | --- |
| R1: withdrawn historical declaration demotes later orphan | RESOLVED. Original association is checked at the occurrence's visible introduction prefix, including its tombstones and excluding future layers. Regression exercises removed-before-orphan and orphan-before-temporary-future-declaration histories with typed counts and independent eager page bytes. |
| R2: metadata checks bypass composed source descriptors | RESOLVED. read_error delegates to check_snapshot and preserves the first typed failure. Immediate metadata reads after zero-page source drift are tested without pre-latching status. First-page request evidence remains in the existing admission home. |
| R3: populated explicit graph declaration lost across seal | RESOLVED. Layers retain explicit declaration sets independently, snapshots/evidence own them, and declare/seal/remove/seal preserves the graph. G11 distinguishes the plain compacted DatasetView surface from extra writer lifetime policy retained by the consumer. |
| R4: subject side probes discard native indexes | RESOLVED. Bound-subject reifier/annotation probes use reifier_quads_of / annotations_of_with_graph, with no per-page iterator box or second cache. Existing physical pruning and affected native/delta regression evidence qualifies the shared path. |

Native G3 deliberately keeps separate table ledgers: equal ordinary and virtual
reifier tuples may legitimately coexist. The final test explicitly retains that
native behavior and compares folded page bytes to native eager input. G11 now
limits exclusivity to the ordinary/annotation pair. No broader cross-stream
disjointness was invented as an acceptance condition.

## Attributable qualification and reuse

Read exact commands/exits in tasks/T1-implementation.md and the named logs.
Commands used the repository's governed floating nightly and default profiles;
dev/test opt-level=3, debug assertions/overflow checks enabled, test debug=0.
Read the root's raw/toolchain.txt capture: rustc 1.100.0-nightly,
commit `4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1.
Each Cargo/make invocation used CARGO_BUILD_JOBS=2. This reviewer inspected
results and source applicability; the implementation agent executed the checks.

| Command | Exit and reviewed result |
| --- | --- |
| cargo test --locked -p purrdf-core --test paged_stack --test paged_backend --test paged_fallible --test paged_admission_law --test mutable_declared_graph_drain | 0; raw/T1-focused-qualified-2.log: 82 pass, zero failures, zero filtered across these targets, including 19 stack regressions |
| cargo test --locked -p purrdf-core --lib ir::mutable | 0; raw/T1-mutable-qualified.log: 42 pass, zero failures; includes delta_view regressions |
| cargo test --locked -p purrdf-core --lib ir::global | 0; raw/T1-global-qualified.log: 23 pass, zero failures, including deep/nested identity checks |
| cargo clippy --locked -p purrdf-core --all-targets -- -D warnings | 0; raw/T1-clippy-handoff.log: successful final all-target check without warnings |
| make helpers-hygiene | 0; raw/T1-helpers-qualified.log: self-tests and 81 enforced jobs, 23 reasoned variants, 91 distinct rows, 1828 files; 80 hash domains current; no open duplicate-home finding |
| rustfmt --edition 2024 --check over all ten changed/new Rust source/test files | 0; exact explicit file list in implementation handoff; raw/T1-format-qualified.log empty |
| git diff --check | 0; raw/T1-whitespace-qualified.log empty |

Earlier cargo check --locked -p purrdf-core succeeded in T1-check-second.log,
but that older command alone was not treated as final compilation evidence.
Final tests/clippy compile the final affected code. Mutable/global unit evidence
predates only the new native-overlap integration assertion and contract prose;
their production hashes remained unchanged and those results are reused with
that applicability assessment. Final focused/all-target checks include the new
test and final source. Historical failed and intermediate passing logs are
retained; none replaces the final named qualification. No full-workspace,
release-profile, wasm, hosted-CI or evaluator run is inferred from core tests.

## Completion boundary

No open required Task 1 implementation/source-review finding remains. The
independent emergency ledger was empty and remains outside reviewer mutation.
This source/qualification PASS permits the parent to perform the authorized
signed commit with mandatory hooks, then push/verify and update the issue.
Hooks have not yet run and are never replaced by the checks adjudicated here.

Task 2 public NativeSparqlEngine query/prepared-query bag equivalence, runnable
consumer, affected evaluator checks and wasm qualification are NOT MET yet.
Synchronization with the advanced base, hosted CI, later independent completion
audit, PR publication, ghprsq integration/audit and scoped cleanup are also not
established by this task review. They remain explicit authorized next steps,
not a scope cut or whole-issue success with caveats.

VERDICT: PASS
