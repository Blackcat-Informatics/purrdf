# Complete grouped intake: candidate union and measured join work

Scope: all supplied #510 and #517 requirements, with their complete bodies and zero comments in issue.md and 517-issue.json. The source-defined requirements are sufficient for implementation; unavailable sibling ADR/measurement documents are not represented as independently inspected evidence.

#510 requires a separate union entry point over the same execute rung; caller depths are independent per stratum and cannot be narrowed from k, cardinality or selectivity. The answer is exactly the deduplicated subject set of the requested native prefixes, retaining every original per-stratum occurrence rank and every producer's depth/evidence/status. Prefix completeness requires each producer to reach its depth or verify exhaustion. A cap, loss, stop, moving attestation or failed producer cannot be renamed exhaustion or silently disappear. Canonical set iteration expresses no relevance order and adds no score. Existing fusion/search behavior and identities remain unchanged.

#517 requires DatasetView::cost(pattern, plan) beside cardinality, exposing host-measured comparable work and the actual examined-row/page/residency information. No library constant converts residency or page counts into work. Join selection uses work; row/selectivity forecasts still use cardinality. The default must preserve original plans, math and ties exactly, including any supported extreme-value case; changing overflow/fallback math is not acceptance by inference. The same admitted statistics snapshot must bind cached cost-dependent order. Determinism is conditional on the same supplied measurements, as for existing statistics.

| Full contract | Task/required proof |
|---|---|
| Independent depths, all validation, no statistics narrowing | Task1 public plan/compile, statistics trap, asymmetric/zero/depth+probe overflow and missing/extra/duplicate cases |
| Native prefixes, deduplicated subjects, exact original ranks | Task1 shared execute/execute_within both schedules, repeated subjects and real TEXT+kNN differential reads |
| Every producer's failure, evidence and completion truth | Task1 native partial failure/healthy sibling, attestation movement, declaration breach, cap shortfall, lost/incomplete index and receipt/protocol negative controls; settle all opened reads |
| Portable bytes and unchanged fusion/search | Task1/3 actual existing native+WASM harness with independent framed answers and complete original retrieval suite/identities |
| Cheaper measured work despite inverted cardinality | Task2 actual BGP first-probe inversion, complete result rows, existing DP and greedy routes and Arc forwarding |
| Default parity, forecast separation, cache/determinism | Task2 independent original planner comparison for default, scopes, seeds, ties and supported large values; row replay under changed work; two actual snapshot-keyed cached orders |
| Complete qualification/main integration | Task3 settled affected checks, native/portable runtime, mandatory full gate/hooks, fresh hosted checks/review, ghprsq and actual two issue closures |

This is one substantive two-issue group with three delivery tasks, not two implementation/review cycles. No #508 bounded workspace prerequisite, generic numeric optimization or new conformance campaign blocks the current resident APIs. Proposal, compiler, local runtime, hosted CI and merged closure remain separate states. Written proposals have not yet met runtime acceptance.

