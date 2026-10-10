# Retained operational result consumer migration map

Read-only implementation assistance from the current selected worktree. The source writer is still changing APIs; anchors below identify the inspected consumers and may move. No source edits, tests, builds, forge calls, or ref actions were performed. This is not another review or qualification verdict.

## Current API and pending boundaries

`src/fallible.rs` now gives `CompleteSparqlResult<E>` a `result: RetainedSparqlResult` and `evidence: RetainedEvidence<E>`. `into_parts()` returns both retained carriers. `src/retained.rs` exports:

- `RetainedSparqlResult::solutions() -> Option<(&[String], &[SolutionRow])>`.
- `graph()` and `auxiliary_graph()` borrow `&RdfDataset`, without exposing an `Arc`.
- `boolean() -> Option<bool>` and `query_form()`.
- `into_solutions() -> Result<RetainedSolutions, Self>`; keep the returned owner alive and call `as_parts()` to borrow names/rows.
- `RetainedEvidence<E>` dereferences to `E`; ordinary clone shares the receipt allocation and admission.
- `RetainedQueryExplanation` dereferences to `QueryExplanation`; ordinary clone shares its payload and admission.

The implementation author confirmed these are intended accessors. Operational `BudgetExhausted.partial` and operational EXPLAIN still used the old raw carriers/signatures at inspection and are pending migration. Do not treat the existence of retained carrier types as wiring those paths. Keep raw resident `SparqlResult`, resident `GovernedOutcome`, and their raw partial carriers unchanged unless an actual shared caller requires migration.

## Migration patterns

| Old consumer pattern | Recommended adaptation |
| --- | --- |
| Enum destructuring or `matches!(complete.result, SparqlResult::Solutions { .. })` | Borrow `complete.result.solutions().expect("SELECT")`; assert actual names/rows. Keep the retained owner alive while the borrow is used. |
| `SparqlResult::Boolean(value)` | `complete.result.boolean() == Some(value)`. Preserve the boolean assertion, rather than merely accepting the ASK form. |
| Graph/auxiliary `Arc` extraction | Use `graph()` / `auxiliary_graph()` for borrowed dataset operations, or retain/clone the result owner. Do not add a raw `Arc` escape hatch to satisfy a helper signature. |
| Destructure `result.into_solutions()` into a raw tuple | Bind the `RetainedSolutions` owner, then `let (variables, rows) = owner.as_parts()`. A temporary owner cannot be dropped while borrowed names/rows are stored. |
| `complete.into_parts().0` passed into an old raw helper | Change the helper/caller to borrow retained solutions or retain the extracted carrier. If the consumer explicitly needs an independent mutable normalization, copy through a borrow and label that allocation as caller-owned. |
| Borrowed `evidence.view`, receipt methods or scalar fields | Most remain source-compatible through `RetainedEvidence` deref. Keep reads borrowed. |
| Equality of whole receipt carriers | Current retained receipts have no `PartialEq`; compare `&*left` and `&*right` (or specific borrowed fields). Compare values, not ownership identity. |
| Move `complete.evidence.governors` or a non-Copy receipt field | Borrow it, or use an explicit `clone()` when an independent caller snapshot is intended. Moving non-Copy fields out through `Deref` is invalid. `GovernorEvidence` contains vectors and is not Copy. |
| Store a complete outcome and exhausted outcome in one `(raw_rows, raw_evidence)` tuple | Keep the relevant retained result/partial/receipt owners, and compare their borrowed data. The complete receipt is already retained; the exhausted-path type is pending. Do not drop a partial owner just to unify the old tuple. |

## Production, examples and benchmark consumers

| Affected file and anchor | Actual use and migration |
| --- | --- |
| `crates/envelope-probe/src/lib.rs:300`, `:747`, `:816`, `:879` | `solution_set(SparqlResult, label)` takes extracted operational SELECT results, moves their row vectors and sorts them. Accept a borrowed retained SELECT view/owner; normalization can deliberately copy names/rows as consumer-owned mutable comparison data. Preserve exact projection and sorted row multiplicities, page prediction/touched equality, graph-retention parity, starved-budget refusal and exact-budget success. `SolutionSet` sorting currently retains duplicates; do not change it to deduplicate. |
| `crates/envelope-probe/examples/wasm_storage_qualification.rs:74` | Pattern-matches `&answer.result`. Use `.solutions()`; preserve the exact one-row/one-cell `o0059` assertion. This example retains the answer while exporting the session and measuring peak: retain that overlap, since it exercises shared storage/result ownership pressure. Do not turn the example into another host campaign for this assistance task. |
| `crates/sparql-eval/examples/paged_stack.rs:68`, `:75`, `:79` | Raw tuple extraction and whole receipt equality. Bind the retained solution owners, use `as_parts()`, and borrow/deref receipt snapshots for equality. Preserve old/current layered-reader answers, prepared/direct parity, cache/page evidence and printed scalar/origin values. |
| `crates/sparql-eval/benches/paged_cross_page_bgp.rs:110`, `:595`, `:601`, `:606`, `:612` | Local raw-result `row_count` is used for resident baselines and retained cold/hot/direct/governed segmented answers. Keep the raw baseline helper; use the retained `.solutions()` row count at these operational calls or one shared borrowed-view helper. Preserve exact counts and cache request/eviction assertions. Repeated benchmark outputs must drop naturally before the next iteration unless retained overlap is the intended workload. |
| `crates/purrdf/src/lib.rs:789` | Umbrella native API smoke test enum-matches an operational ASK. Use `.boolean() == Some(true)`; preserve requested-origin assertion. The `sparql` facade must expose retained carriers and their consumers through the existing public module. |

A repository-wide Rust call-site search found no direct operational query/EXPLAIN consumer in CLI, C ABI, wasm binding code, Python binding code, entail or shapes beyond the shared evaluator/umbrella paths listed here. Their resident result helpers are not mechanical migration targets. New consumers introduced by the writer still need the retained contract.

## Native tests grouped by consumer shape

Paths in this table are relative to `crates/sparql-eval/`.

| File and anchors | Required adaptation |
| --- | --- |
| `tests/support/mod.rs:215`, `:222`, `:230`, `:256` | Common `solutions`, `row_count`, `result_size`, and sorted-row helpers accept raw `SparqlResult`. Preserve resident helpers. Put any common borrowed retained-view comparison at this existing support home instead of duplicating a new raw-conversion helper in every test. Avoid an API which releases admission while sharing engine storage. |
| `tests/fallible_query.rs:56`, `:77`, `:102`, `:515`, `:661`, `:688`, `:756`, `:914`, `:959` | ASK enum patterns become `.boolean()`; SELECT helpers become retained borrows or retained extraction. The `CompleteSolutions` repeatability tuple and `expected` receipt storage must use retained owners plus borrowed comparison, or explicit independent snapshots when the test deliberately owns copies. Whole receipt comparison near `:763` needs deref. Preserve operational errors, projection/row content, duplicates, exact page/byte consumption and repeated request determinism. |
| `tests/paged_stack_query.rs:65`, `:110`, `:174`, `:199`, `:265`, `:378`, `:592`, `:622`, `:636`, `:847`, `:863` | `assert_same_result(SparqlResult, SparqlResult, query)` compares forms, solutions and graphs. Add a retained-versus-resident borrowed comparison, preserving graph contents/term counts and solution bags. Replace ASK/SELECT destructuring at operational calls. Whole repeat/exact-boundary receipt equality needs deref; later raw error evidence remains a separate shape unless the writer changes it. Keep caches, layered origins, high IDs, visibility and checkpoint assertions. |
| `tests/prepared_operations.rs:48`, `:193`, `:194`, `:247`, `:250`, `:260`, `:290`, `:300`, `:302`, `:329`, `:348`, `:380`, `:600`, `:650`, `:651`, `:656` | `equal_results` mixes resident `GovernedOutcome` and operational results: keep the resident comparison and add/use a borrowed retained comparison for operational routes. Partial-result comparisons and `prior: Option<(SparqlResult, GovernedEvidence<_>)>` need the pending operational retained partial/receipt shape. `prior = result.evidence.governors` must become a borrowed comparison or explicit snapshot clone. Preserve registry admission, substitutions, every query form, positional partial certificate and shared-operation cumulative governor semantics. |
| `tests/query_completion.rs:727`, `:731`, `:788`, `:943`, `:1054`, `:1309`, `:1313`, `:1336` | `assert_complete_query` uses raw SELECT matching. Its receipt trait call can continue through deref. The typed EXPLAIN dispatch helper explicitly returns `FallibleScopedResult<QueryExplanation, RefusedRead, Receipt>` and must use the actual retained EXPLAIN return shape after wiring. Ready-outcome comparisons that return `.governors` need an explicit snapshot clone on the retained complete route. Pending partial comparisons use its retained result view. Preserve failure injection and final-checkpoint precedence; do not weaken them to accommodate an owner. |
| `tests/segmented_query.rs:139`, `:274`, `:283`, `:332`, `:346`, `:426`, `:642`, `:667`, `:679` | SELECT enum destructuring becomes borrowed access. Complete/partial result-and-evidence match branches must retain the output carriers. Exact release formulas based on the previous scoped execution charge must change for retained publication; see lifetime obligations below. EXPLAIN's old immediate-release assertion is incompatible with retained output. |
| `tests/governed_query.rs:1135`, `:1168`, `:1169`, `:1210` | Only the operational complete/partial sections need retained accessors. Most other tests use raw resident `GovernedOutcome`/`PartialSparqlResult` and should preserve that surface. Keep Certain/AtMost/Unknown and positional-prefix assertions; row_count is not a substitute for a certificate. |
| `tests/request_substitution_feasibility.rs:409` | The fallible door maps `complete.into_parts().0` through the shared raw solution helper while other doors return caller-owned test rows. Borrow retained rows and deliberately copy to the existing comparison representation when needed; this is an explicit independent test copy, not engine extraction. Preserve named projection/layout, prebinding semantics and exact cross-door row comparison. |
| `tests/paged_query_e2e.rs:277` | EXPLAIN uses borrowed explanation methods and receipt fields. Those remain natural with retained carriers/Deref. Preserve exact requested pages and join-order reversal: they prove the operational view's statistics are used without replay. |
| `tests/service_resolver.rs:269`, `:277`, `:286`, `:296` | Explanation method calls remain borrowed. Whole quiet/ordinary receipt equality may need deref after receipt migration. Preserve real configured source/router/header use, exchange counts, fired-stop zero-exchange behavior and typed denied-source diagnostic. These existing unbounded remote tests are not a request to broaden bounded opaque-producer support. |
| `tests/native_xpath.rs:493` | Consumes an operational Query error and its diagnostic only. No current complete-result migration is required. Preserve the typed distinction from operational failure/governor partial answers. |

`src/user_fn.rs:2136` onward exercises operational native entry/EXPLAIN/scoped/construct failure dispatch in its unit tests, then reads `error.evidence()`; it maps successful outcomes to `()` rather than consuming raw complete payloads. No retained-result conversion is currently required there. The internal `engine.rs` and `engine/prepared_fallible.rs` production publishers are the writer's ownership migration homes, not independent caller patches.

## Partial and EXPLAIN migrations still to wire

Operational errors currently declare `BudgetExhausted.partial: PartialAnswers` over the raw `PartialSparqlResult`. Operational tests call `partial.result().expect(...).result()` and sometimes clone that raw result. The retained operational partial carrier must preserve the Certain/AtMost/Unknown classification, `is_positional_prefix`, and ownership-carrying extraction. It must provide a retained result view, not `&SparqlResult` exposing graph/auxiliary `Arc`s. Its ordinary clones must be shallow, and operational failure must still publish no partial result. Primary consumers are prepared_operations, governed_query, query_completion and segmented_query above. Do not globally reinterpret resident partial tests as operational consumers.

Operational EXPLAIN currently returns a raw explanation/evidence tuple. Wiring the exported `RetainedQueryExplanation` preserves `.render()`, `.join_orders()`, `.ledger()`, `.relations()` and `.evidence()` through deref. Update explicit helper return annotations and whole receipt equality only as required by the chosen receipt carrier. Retain final-checkpoint failure discard and source-before-cancellation precedence. The existing resident explanation APIs stay raw.

## Lifetime and allocation expectations to change substantively

- `tests/segmented_query.rs:667` says publication returned and released its guards while the explanation is still alive; `:682` expects repeated calls to restore the same live count immediately. Replace this with proof that the explanation/receipt owners keep required retained admission, ordinary clone shares it, dropping the original leaves the clone admitted, and the last covered owner releases it. Repeated simultaneous results are permitted to accumulate real retained charges.
- `tests/segmented_query.rs:240` onward derives one exact released execution charge from `execute_fallible` and then reuses it for owned complete and exhausted publication (`:298`). Scoped completion and retained publication have different lifetimes now. Compare their actual required owners and release boundaries, without replacing the old formula with an arbitrary constant or weakening the peak bound.
- `tests/query_completion.rs` fake reservations and final receipts prove live guards at checkpoints. Keep that proof, then add post-return/clone/extraction/final-drop evidence for complete/partial/EXPLAIN owners. Route-specific checkpoint behavior remains semantic; a new lease is not grounds for blanket count weakening.
- Test both extracted result and extracted receipt retention. Dropping one member of `into_parts()` does not by itself prove the shared admission has ended. With the current complete carriers, both can retain the same query account.
- Borrowed names/rows/datasets do not outlive their carrier. Sorting and explicit copies in a test or probe are client allocations; make those intentional and keep them outside the engine-only allocation measurement where appropriate. Do not count a test's copied rows as evidence that engine deep copies were admitted.
- Failure outcomes and scoped completion still release newly acquired execution allocations. Retained success need not equal the original source live count until its last owner drops. Account for deliberate cache state/eviction when comparing baselines; do not infer a leak from legitimately retained output or hide one by ignoring the final drop.

This map identifies actual consumers and their required adaptations only. All migration, implementation and acceptance remain with the sole source writer and the existing complete-delivery task.
