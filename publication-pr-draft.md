Rows now retain their information state, completeness and frame through SPARQL and Datalog execution. Callers can request a verdict, summary, rows or a queryable RDF 1.2 ledger, and explicitly accept undecided reasons without losing answer order or ranked identity. Retrieval scopes apply inside each producer before depth.

The same evaluator gains exact physical shape selection after logical rewriting: dictionary matching over analyzed token streams, ordered windows, typed bucketed series and caller-registered shapes. Literal and shaped paths share evidence, resource admission and error semantics. Native row ownership keeps the ordinary 64-bit row at 40 bytes; correlated evidence and aggregate records share their original query owner. Completed closed empty searches and incomplete searches retain distinct existential results.

Validation includes 1,513 evaluator library tests, 20 ingress tests with 36 actual empty-search combinations, 17 prepared tests, three row-allocation tests, all 74 SPARQL manifest cases, both evaluator traces, 15 normative governor cases, 36 native-profile cases, 19 native physical controls, and 110 scalar plus 15 SIMD WASM executions. Governor profile v15 records the two explained evidence-bearing fuel changes (11 to 10) with unchanged answers and other charges.

The final local round is **FAIL** (terminal 2): metadata, workspace strict checks and preceding hygiene pass; the complete native/doctest collection has exactly one failed target, `purrdf-testkit --test temp` (6 pass/1 fail). A retained pre-relocation binary embeds the old `CARGO_TARGET_TMPDIR`, while the executable-derived path is canonical; both resolve to the same directory but fail the exact lexical assertion. The subsequent preserve-order consumer, final core-hygiene step and nested release-crate WASM build were not reached. Follow-up #531 records the failure and unexecuted suffix. Independent implementation/source reviews pass under the revised scope; the publication audit records the failed original qualification separately from pending hosted/integration steps. Normal commit hooks passed. Hosted checks remain pending publication.

The user changed the delivery boundary explicitly: "You will NEVER have exclusive timing so remove that work from the list. evaluator branch will be pushed/merged after next round regardless of status, file an issue with any followup work."

The exclusive 36-lane throughput and 78 physical timing campaign is canceled, not passed. Final-source throughput and physical timing remain unqualified. Five fixed-work pairs measured 4.217578% fewer instructions per DISTINCT query; that is not a throughput result. [Follow-up findings and remaining evidence](https://github.com/Blackcat-Informatics/purrdf/issues/531) are visible before publication. No exclusive campaign is reinstated by that follow-up.

Hosted findings are also recorded in #531: doc fails on two stale engine links, MSRV fails on recursive `SharedWorkspace`/`IntoIterator` inference at the shapes SCC caller, and workspace fails Clippy's items-after-test-module rule. Other jobs remain pending/running or skipped. These are not passing checks. The user's explicit merge-after-round direction governs the revised shipping boundary; main has no configured required status checks or approving-review count, and normal hooks/ghprsq remain enforced.

The authoritative combined plan and validation index are in the selected Stage directory `.stage/sparql-eval-datalog-per-row-evidence`, archived separately by the merge workflow. This supersedes the earlier separate three-issue/two-issue publication boundaries.

Closes #515
Closes #516
Closes #511
Closes #518
Closes #512
