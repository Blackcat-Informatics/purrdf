retrieval: preserve candidate prefixes and order joins by measured host work

Closes #510
Closes #517

Candidate requests now carry independent caller-selected depths through the existing planner, compiler and executor. The union consumer returns an unscored subject set while retaining every original producer rank, depth, settlement, failure and completeness/index/order evidence. Asymmetric depths, explicit zero, partial native failures and real TEXT/kNN producers are verified. Existing fusion/search identities and execution remain intact; the shared ranked-stream home owns receipt validation for both consumers.

DatasetView now exposes host-measured AccessCost beside cardinality. The original DP/greedy optimizer orders work by that measurement while retaining cardinality for prefix selectivity and replay. Its fallback preserves original u64 accumulation, rounding, overflow and deterministic ties; an explicit cardinality-only flag retains provenance rather than inferring it from numeric coincidence. Statistics fingerprints still invalidate cached orders. There is one optimizer/executor and no page/residency multiplier, semantic Cargo feature, runtime dependency or hidden workspace prerequisite.

Affected all-target Clippy, full native retrieval/evaluator matrix, real-producer and failure controls, actual portable8/8 acceptance and required full-check-2 passed. The final three-file review correction passed affected all-target Clippy,49 planner tests and normal signed hooks. The assembly manifest names the exact production planner specialization with unchanged seven-configuration instruction constraints. All final-head hosted checks pass; four conditional native-profile/projection jobs are expected skips, not claimed execution.

The actual clean integration candidate combines d608b0584 with1bfae60de, tree a63858606154299e32f6a5ea5df636499736de69. Hosted integration shards checked out matching PR merge4d993f24 and passed candidate_union10/10 and real_producers15/15, closing the text declaration identity interaction. No conflicts required resolution. Final input refresh remains mandatory.

Independent applied-contract-review.md found no missing behavior or silent deferral. Independent review-debt.md covers the exact final delta: both concrete review findings are fixed; the only thread is resolved/outdated. The aggregate docstring percentage request is declined with a posted reason because the relevant public contracts are documented; quota-limited bot processing is not represented as a complete review of that delta.

Authoritative plan and selected evidence: .stage/retrieval-a-union-stage-beside-fuse-per/plan.md and .stage/retrieval-a-union-stage-beside-fuse-per. Validation, integration assessment, full logs, portable controls and review dispositions are retained there. Both complete contracts land together; unrelated storage/workspace/model campaigns are not claimed complete.

Defect-Class: none
