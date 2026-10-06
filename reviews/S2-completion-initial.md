# Independent Stage 2 completion audit

VERDICT: FINDINGS-OPEN

The layered mechanism has attributable, non-vacuous public-caller execution for
the requested shape and the two acceptance identities. One shared term-ingress
correctness defect remains open. Hosted qualification and final workflow gates
also remain unverified; this report does not permit merge or declare the issue
complete. No requirement was waived and no human scope cut was supplied.

## Identity and independent judgment

- Issue 457, PR 466; branch
  `paudley/457-paged-tier-an-lsm-style-stack-of-sealed`.
- Independently read HEAD `b2edf7450cf654d20ae97bd856d67102d0916b7b`, source tree
  `13496228db0e5ec79074acad9020cac5aab7556e`, and `origin/main`
  `b6f7c9b0f6b84ffe496719e39f2d2ba52d5ed3ac`.
- The caller's captured `raw/S2-merge-tree.txt` is the same source tree. The base
  is an ancestor; there is no untested additional candidate source delta in this
  capture. A later base/head change requires an applicability assessment.
- Independently hashed `plan.md`:
  `ce1721dae29798cf2657336171b9981a0a1d86eec1ef0682392fc7379081ea29`.
  Independently ran both full `sha256sum -c` manifests; every T1 and T2 input
  matched (exit 0), including both new core files and all five consumer files.
- Read the complete completion-adversary and validation references, full issue
  and captured discussion, plan, current index, implementation handoffs, final
  task reviews, captured PR/review/thread/check surfaces, actual changed source,
  applicable AGENTS.md, root `.baseline`/`.goals`, and governing backend contract.
- Read both root and worktree `.deficiencies` directly: neither has an entry
  below its marker. Working source is clean; `?? .stage/` is process evidence.

This is independent adjudication of attributable execution, not a second Cargo
execution. No missing integration demonstration justified repeating the example
or expensive gates. I executed read-only identity, manifest and scan commands;
the implementation agents/parent executed the qualification commands below.
No source, forge, lifecycle or configuration mutation occurred in this audit.

The tested compiler is captured in `raw/T2-toolchain.txt`: rustc
1.100.0-nightly, commit `4b6d04e706108ccfeafe2547fbe857dfe8972bad`, LLVM 23.1.1.
Cargo/make checks used `CARGO_BUILD_JOBS=2`, governed default dev/test profiles
(opt-level 3, assertions/overflow checks on; test debug 0), and warnings denied.
The example used governed dev; wasm used governed release.

## Evidence applicability

T1 final core checks were executed for the eleven-file manifest and reviewed
tree `2e7207441e0adff64104179cd26ef568ad1225b2`. Every applicable production/test/
contract input is byte-identical now. The subsequent base integration changed
five evaluator files, no core source/toolchain/manifest/backend contract; T2
actually exercised that synchronized evaluator. T1 receipts are reused for
their unchanged core behavior, never as evaluator or current whole-workspace CI.

T2 final source is the current tree above. Its seven new evaluator cases reran
after the three mechanical lint repairs; the other 25 tests, executable/helper,
facade and production dependencies remained unchanged. Those earlier receipts
retain their original executions and qualify those unchanged inputs. Historical
failed/intermediate logs are not substituted for final qualification. The new
shared-ingress finding below is not cleared by the older passing global suite.

Commands C1-C9 below ran in the assigned worktree; each exit is recorded as 0
in the attributable task handoff and consistent with the inspected raw output.

| ID | Command | Inspected observation / expected result |
| --- | --- | --- |
| C1 | `CARGO_BUILD_JOBS=2 cargo test --locked -p purrdf-core --test paged_stack --test paged_backend --test paged_fallible --test paged_admission_law --test mutable_declared_graph_drain` | `raw/T1-focused-qualified-2.log`: 82 pass (19 stack, 34 backend, 12 fallible, 11 admission, 6 drain), none failed/ignored/filtered. Expected public core laws and existing admission/graph paths hold. |
| C2 | `CARGO_BUILD_JOBS=2 cargo test --locked -p purrdf-core --lib ir::mutable` and `... --lib ir::global` | `raw/T1-mutable-qualified.log`: 42 pass; `raw/T1-global-qualified.log`: 23 pass; intended filtered modules ran. Expected shared classifier and dictionary regressions pass. The global suite lacks cold checked/unchecked composite re-ingress, so it does not establish F1. |
| C3 | `CARGO_BUILD_JOBS=2 cargo test --locked -p purrdf-sparql-eval --test paged_stack_query --test fallible_query --test paged_query_e2e --test delta_view` | `raw/T2-evaluator-qualified.log`: 32 pass, no failures/ignored/filtered (7 new stack, 17 fallible, 7 paged e2e, 1 delta). Both ordinary and prepared guarded production entry points return the asserted eager observations or typed Operational refusal. |
| C4 | `CARGO_BUILD_JOBS=2 cargo test --locked -p purrdf-sparql-eval --test paged_stack_query` | `raw/T2-new-final.log`: final seven cases pass, no failures/ignored/filtered. Expected repaired graph/path/promotion coverage plus operational laws execute on final spelling. |
| C5 | `CARGO_BUILD_JOBS=2 cargo run --locked -p purrdf-sparql-eval --example paged_stack` | `raw/T2-example.log`: retained Bob = 2 layers/2 pages/202 bytes; current Robert = 3 sealed layers/4 pages/1088 bytes, Head and source-qualified origins; compacted 1 page and ordered carrier identity asserted. Expected retained answer survives writer edits and prepared reread preserves exact receipt. |
| C6 | `CARGO_BUILD_JOBS=2 cargo test --locked -p purrdf --lib facade_exposes_the_completed_umbrella` | `raw/T2-facade.log`: intended root consumer passes 1, 43 unrelated filtered. Expected root-only stack construction/mutation, guarded ASK(true) with Head origin and one-page fold. |
| C7 | `CARGO_BUILD_JOBS=2 cargo clippy --locked -p purrdf-core -p purrdf-sparql-eval -p purrdf-rdf -p purrdf --all-targets -- -D warnings` | `raw/T2-clippy-qualified.log`: affected all-target check finishes successfully without warnings. Expected current affected closure compiles/lints. |
| C8 | `CARGO_BUILD_JOBS=2 cargo build --locked --release --target wasm32-unknown-unknown --lib -p purrdf-core -p purrdf-sparql-eval -p purrdf-rdf -p purrdf` | `raw/T2-wasm-qualified.log`: release compilation succeeds in 3m00s for selected libraries and transitive facade closure. Expected no new native-only dependency. This is compilation, not runtime parity. |
| C9 | `CARGO_BUILD_JOBS=2 make helpers-hygiene`; `CARGO_BUILD_JOBS=2 make build-profile-hygiene` | `raw/T2-helpers-qualified.log`: 81 jobs/23 variants/91 distinct rows/1833 files, 80 domains, no open copies; `raw/T2-profiles-qualified.log`: 984 resolved units across 2 gate invocations, 42 members, actual opt3/assertions/overflow checks. Expected one-home and effective profile gates hold. |

## Completeness matrix

| Required behavior | Real demonstration, observation and expectation | Current status |
| --- | --- | --- |
| One or more sealed bases plus chronological sealed deltas | C1 counted public construction/append/snapshot and seal-head do not materialize older pages. C3-C5 use independently sealed bases whose ordinal zero differs by value and whose PageId zero collides, then multiple appended/sealed batches. Metadata translations retain each local summary. Expected independent IDs never alias by number and setup never eagerly collapses lower content. | DEMONSTRATED for exercised public shape |
| Mutable head with terms absent from lower dictionaries; consumer can retain/reconstruct it | C1 public fallible contains/insert/remove and invalid-input atomicity; C3-C5 insert replacement Robert/new values in a resident head and seal only the batch. C6 consumes root facade. Expected requested values are admitted and reach real query callers. Stack owns no persistence/log. | DEMONSTRATED |
| Per-generation by-value removals suppress every lower copy and reinsertion survives | C1 64-step independent value-set model, same-head histories and deletion-only seals; C3-C4 separate BTreeSet model, repeated physical facts, 32 generated operations, withdrawal and newer head/sealed reinsertion. C5 retained/current join changes Bob→Robert. Expected effective set and query bags match independent eager inputs. | DEMONSTRATED |
| One pinned stack G9 snapshot including skipped/zero-page descriptors | C1 retained readers, point metadata and head drift checks; C3-C4 drift source ordinal 1 generation/page count, with zero/skipped pages, constants and head-only queries through both public entry points. Expected original readers unaffected by writer mutation; provider drift returns sticky SourceSnapshot with exact cause, zero content requests for descriptor-only refusal. | DEMONSTRATED |
| Whole-operation G7/G8 page/byte budgets and exact all-layer receipts | C3-C4 counted actual providers plus Head: 3 pages, exact 28+head_bytes, newest-first qualified origins, one charge per cache entry, identical prepared reread receipt; exact inclusive limits pass, zero/one-below limits refuse before rejected materialization. Provider/cancel/deadline/corrupt/changed-content failures are typed Operational, no diagnostics/partial-answer payload; failed fold returns no artifact. C5 emits concrete whole-operation receipts. | DEMONSTRATED |
| G10 remains exact locally and sound after tombstones/typing changes | C1 counted subject/graph pruning, summary drift and annotation-only promotion/demotion; C3-C4 actual engine queries compare independently typed eager tables after last reifier removal and later graph-scoped promotion, both resident and sealed. Expected excluded pages unread, admitted changed summary/content refused, logical estimates conservative. Shared physical accessor retains native indexes and one cache/budget/latch. | DEMONSTRATED for exercised cases |
| Read equivalence through the shipped evaluator, including RDF 1.2 and graphs | C3-C4 19-query matrix through query_fallible_view and query_prepared_fallible_view: exact ordered solution cells/bags, OPTIONAL unbound, UNION/DISTINCT, MINUS/NOT EXISTS, aggregates/subquery, nonempty paths, graph variable/constant/empty membership, ASK/CONSTRUCT and slicing. Independent typed input covers scoped equal labels, nested terms, directional literals and native Primary/Reifier overlap. Four-edge and anchor-cycle witnesses prevent vacuous parity. No drained stack substitutes for oracle or caller. | DEMONSTRATED on representative query surface; F1 prevents clearing complete shared identity contract |
| Canonical fold yields eager byte-identical pages and canonical value IDs | C1 public compact/canonical seal compares dictionary values/IDs and every ordered PackBuilder page; C3-C5 compare actual ordered emitted carriers from distinct physical histories and independently maintained typed effective inputs, several positive bounds, explicit empty graphs and all-removed zero pages. Same canonical partitioner is intentionally shared; effective typed inputs are independently built. Faults publish no fold. Expected tombstones disappear and page bytes/value IDs depend on effective typed surface and bound. | DEMONSTRATED for exercised inputs; composite re-ingress F1 remains open |
| Cross-layer term identity and existing shared ingress semantics | Value remapping, scoped/nested/directional witnesses above pass. However checked/unchecked cold literal re-ingress now bypasses embedded composite blank registration, unlike the retained ordinary literal home. Existing full-dictionary traversal or already-populated tests can mask this. | FAILING: F1; needs cold parity regression and repaired shared home |
| Depth/storage/log/atomic publication boundaries | C3-C5 demonstrate multiple sealed layers with exact descriptor depth in receipts; G11 states consumer compaction timing/depth policy, no hidden depth ceiling. Issue explicitly excludes durable storage/log/when-to-compact and requests consumer atomic publication. Snapshot metadata cost is documented; no benchmark/performance speedup was asserted. | DEMONSTRATED mechanism; excluded policies are not missing requirements |
| Portable supported production utility | C5 executable uses exported public mechanism and real guarded/prepared engine, then consumes fold as actual carriers; C6 root-only consumer proves facade wiring, C8 selected release wasm build. No dependencies/features/alternate implementation were added. | DEMONSTRATED native utility and wasm compilation; no wasm runtime claim |
| Required review, publication, hosted qualification, merge/audit and cleanup | Full initial captured review/comment/thread surfaces read: empty reviews/inline threads at capture and pending hosted checks. Normal signed commits/hooks/push/readbacks and issue/PR plan publication exist. Final completion recheck, required feedback/checks and Stage3 ghprsq result/archive/issue closure/scoped cleanup have not occurred in this capture. | UNVERIFIED; required next workflow steps, not source scope cuts |

## Open correctness finding

F1 — Cold validated literal ingress drops referenced composite blank identities.

`crates/rdf-core/src/ir/global.rs:535-545` re-interns a validated literal using
`try_intern_lookup(GlobalTermLookup::Literal { ... })` directly. The previous
`reintern_validated` arm called `intern_literal`; the existing home at lines
569-588 first calls `cdt_embedded_blanks`, then interns each label with its
existing scope before inserting the literal. The new checked path omits that
behavior, and `reintern_validated` itself now delegates to it.

I independently inspected both paths and the existing regression. In
`intern_value_registers_composite_blanks_inside_triple_terms` (lines 1147-1181),
ordinary `intern` fills the dictionary before `reintern_validated` is called.
That passing assertion cannot witness a cold miss. Page translation, paged
compaction and stack metadata all depend on the changed validated home; full
source-dictionary walks can already include the identities and hide the loss.
No failing reproduction was executed by this auditor, so this observation is
source-established and the missing cold demonstration is named explicitly.

Required fix: preserve composite embedded-identity registration in a shared
checked literal-ingress home (retain typed allocation/address failures), route
both ordinary and validated ingress through its intended behavior, and add
cold checked/unchecked parity regressions for scoped/nested List/Map/triple
composites plus opaque ordinary lexical bytes and idempotence. Qualify affected
global/paged/stack consumers and appropriate portability/one-home gates, then
independently recheck this finding on the repaired identity. The parent owns
this visible remediation; no new follow-up issue or ledger descope clears it.

## Scope/deferral adjudication and remaining gates

Independently ran the prescribed added-diff scan against verified base (exit 1,
no matches), and prose scan over plan, PR body draft, task commit texts and
published task-update inputs (exit 1, no matches). Read the captured actual PR
body, full issue comments, actual logical commit messages and base-sync message;
no hidden TODO, stub or follow-up was found. Also inspected “latent”, “best
effort”, “owned by another issue” and “separate deliverable” leads; none occurred
in those inputs. No source assertion/golden was weakened to pass: the ordinary
dictionary witness and typed-empty expectations remain concrete.

The plain folded base preserves current typed rows and graph membership. A
populated graph's extra explicit-versus-implicit writer lifetime policy is not
encoded by DatasetView; retaining that policy across base replacement is
documented separately. This is not a silent cut of the current effective-surface
byte/read identities. Extra non-RDF sidecars are likewise outside the requested
mechanism. This does not excuse F1, which changes an existing identity home.

Read actual Makefile, hooks and CI definition. Normal commits ran staged fast
hygiene/rustfmt/ratchet gates; they did not run clippy/tests/full helper census.
Those have their separate actual receipts above. CI splits full workspace gates,
tests/conformance and portable/runtime lanes. Initial captured hosted results are
pending, never passing evidence. No full local make check or wasm runtime run is
inferred from focused qualification. Recheck final head/base/candidate identity,
F1 disposition, complete review debt and successful required checks before the
Stage2 exit; Stage3 then owns actual integration/audit and cleanup.
