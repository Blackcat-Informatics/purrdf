# Issue analysis — complete bounded operational query workspace

## Authority and evidence

Issue #508 is the complete delivery scope. The issue body and its initially empty comment surface were captured through Stage; this document does not repeat forge intake. The user authorized implementation. The selected isolated worktree is `paudley/508-sparql-eval-complete-bounded-workspace`, based on `3b6ec4d9f76984b62c0b07ede69cdd98baf813f5`. Applicable `AGENTS.md`, `.baseline`, `.goals`, the recovery handoff, Stage planning/quality rules, and the relevant production sources were read. No source changes, builds, or tests were performed for this analysis.

The delivery must complete the stored operational Rust query surface, including EXPLAIN, through the existing engine. Spill and batching from #486 are not prerequisites or substitutes. Preserve normal hooks, deterministic semantics, existing governor and operational-error precedence, portability, sibling worktrees, and protected main. No new runtime dependency, semantic Cargo feature, alternate evaluator, resident-copy fallback, hidden budget suppression, or deferred required behavior is authorized. Issue #471 remains frozen; #280 and submissions to other repositories are excluded. The complete workflow has at most three full `make check` invocations, including failures; the target is one. The qualification ledger initially records zero.

## Observed production gap

`crates/sparql-eval/src/engine/bounded_workspace.rs` admits only a narrow projected SELECT/BGP shape for a bounded view. It rejects other query forms, explicit datasets, other algebra, nested triple patterns, substitutions, and configured extension/focus contexts as `WorkspaceUnpriced`. Its static row/width/stored-term multiplier does not certify computed-value growth or native producer scratch. Merely removing these guards would make admission untruthful.

`DatasetView::reserve_workspace` currently returns a reservation that may borrow its view. `WorkspaceReservation` supports `resize`; it does not itself establish ownership after a query returns. The segmented backend already has an owned `SegmentedReservation` backed by `Arc<Budget>`. Its initial workspace admission can evict unpinned cache entries, while reservation growth currently cannot. Growth must use the same inclusive storage budget and respect cache pins; it must not create a strong reference cycle through cached blocks.

Direct, prepared, governed, scoped execution, and fallible EXPLAIN reach the shared evaluation/materialization machinery in `engine.rs` and `prepared_fallible.rs`. Their workspace is currently a local guard. `CompleteSparqlResult` exposes a clonable raw `SparqlResult` and `into_parts` extracts it without a lease. Solution cloning allocates a new payload; graph cloning shares an engine allocation. Both make a guard attached only to the outer wrapper insufficient. Governed partial answers and returned explanations have the same ownership obligation. A scoped visitor's explicitly created caller-owned return value is a separate allocation responsibility.

`eval_ctx` currently treats a guard as an admitted marker and applies some engine options only to an unbounded view. Full admission must preserve configured-option behavior rather than silently omit it. Existing typed final-boundary logic must continue to discard incomplete operational results and make operational failure outrank derived diagnostics and governor trips.

## Chosen architecture and concrete obligations

Use one owned workspace capability for the existing query path. Certify bounds for known algebra structures, and admit additional computed-value and native-producer allocations before growth where a complete static bound is unavailable. Keep the first typed admission failure available to the final operational boundary. Every reservation calculation and capacity calculation must use checked arithmetic.

Return a separate immutable retained operational result: the payload and its owned lease share one owner. Normal clones share that owner; ownership-carrying extraction retains it. Apply the same rule to partial answers and returned EXPLAIN payloads. Provide borrowed immutable consumption without an engine API that silently exposes a lease-free shared graph or deep-cloned raw result. A caller deliberately copying a borrowed term or row creates its own independent allocation; this does not require prohibiting ordinary value cloning.

The implementer must cover these existing owners, including their simultaneous lifetimes:

- Parsed/prepared/substitution scaffolding and reporting/control state before bounded entry work, including retained schema or cache state attributable to the query.
- Solution schemas, rows and spills beyond inline row cells; input/output overlap, join indexes and wildcard lists, DISTINCT/MINUS/UNION/OPTIONAL state, sort keys, grouping and aggregate state, subquery state, and property-path frontiers/caches.
- VM temporaries and minted/overlaid arenas. CONCAT, REPLACE, GROUP_CONCAT, numeric arithmetic, regex compilation/matching, and constructed composite/triple values cannot be bounded solely by the largest stored term. Use existing numeric scratch and row admission homes rather than duplicate implementations.
- All four materialized forms, CONSTRUCT/DESCRIBE builders and freezing overlap, result auxiliary datasets, permitted partial results, and EXPLAIN measurements/publication.
- Native text query analysis, ranking/candidate vectors, cursor state, bound argument ownership, and produced rows. `rows_per_invocation` alone is not a byte bound. Native functions, SPARQL-bodied functions, custom aggregates and other admitted producers need truthful transient/retained capacity contracts. Caller-owned prebuilt indexes and borrowed focus data must be distinguished from new engine/producer allocations.
- RDF 1.2 reifier/annotation virtual streams in candidate counts and actual allocation paths. A bound inferred only from ordinary quads is insufficient. Empty named graphs, explicit active datasets, nested triple patterns, and focus/extension options must use the same semantics as resident execution.

Ownership admission may be conservative when certified, but must cover live allocations rather than claim that a cumulative allocation count is a measured live peak. Allocation failure must remain actionable and typed. Release ephemeral ownership after its allocation dies; retain published engine ownership until its last owner dies. Failure paths release all newly acquired ownership.

## Complete acceptance map

| Required contract | Production surface and minimum proof |
| --- | --- |
| Healthy bounded/resident parity | Exercise SELECT, ASK, CONSTRUCT and DESCRIBE through a real bounded `FallibleDatasetView`; compare solution bags or graph semantics for default/named/empty graphs, explicit active datasets, GRAPH joins, correlated OPTIONAL/BIND, UNION, FILTER/expressions, DISTINCT/order/slice, grouping/aggregates, subqueries, paths, nested triple patterns, and RDF 1.2 virtual streams. Do not substitute an unrestricted wrapper. |
| Entry and configuration parity | Run direct and prepared operational entries, prepared substitutions, governed entries, and their applicable option-bearing variants against the same fixtures. Cover native text correlation, admitted extension/aggregate/function contexts and focus graph; assert meaningful engine-option parity. Preserve existing unbounded behavior. |
| Operational EXPLAIN | Direct/prepared operational EXPLAIN produces useful cardinality evidence and actual admitted measurements. Its work and retained explanation are priced; no resident dataset copy or suppressed storage budget is used. |
| Typed failure precedence | Inject read, checkpoint and admission failures at lookup, intermediate growth, drain and final publication. A typed operational error wins over expression/query diagnostics and governor exhaustion. No incomplete operational result is published. |
| Independent refusal modes | Separately prove low capacity, checked-size overflow, cancellation, governor exhaustion and genuine allocation failure. Shape/context refusal is not an acceptable substitute for a capacity or unsupported producer contract error. Preserve governed partial-answer certification where publication is permitted. |
| Peak and retained ownership | Use the existing allocation probe and a real budgeted view. Show admitted bytes cover measured live query execution/result ownership, with prebuilt caller data excluded explicitly. Check computed-value growth and native text scratch. Original-result drop after shallow clone, ownership-carrying extraction, partial-result retention and EXPLAIN retention must keep the engine lease alive; last-owner drop and every failure restore the expected baseline. |
| Qualification and completion | Add focused native regressions using existing helpers, run relevant existing conformance/affected checks, then the required full gate in the prepared qualification lane. Observe the workflow's full-gate ledger and normal hooks. Record source identity and unrun/pending acceptance honestly; do not claim issue completion with any required behavior absent. |

## Delivery boundary and risks

This is one substantive implementation task across the shared owners and all actual callers, followed by one qualification/PR task. It is not a sequence of per-operator commits or ceremonial reviews. A partial prerequisite may be useful internally, but does not close this issue.

The main risks are public migration from a borrowed reservation to an owned capability, retained-result/partial/explanation API migration, overlooked overlapping native allocations, backend growth versus cache eviction, and option or operational-error drift. Migrate actual repository callers and test implementations together. Preserve the resident fast path where admission is a genuine no-op. Any remaining required owner or bounded behavior is a FAILED acceptance criterion with its exact blocker, not an accepted deferral.
