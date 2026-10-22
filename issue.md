# Issue #481: perf(sparql-eval): reduce grouped-output buffer traffic through fallible admission

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

The grouped numeric production query at commit 6273b6173f3b3d98f49629a162ba88da8bc26477 requests 199,960 additional bytes per query relative to dfc0c21ad, at 1/4/32 workers. Allocation-call and retained-byte counts are unchanged. Plain BIND allocation metrics match the baseline; bounded numeric fuel/scratch requested bytes and median peaks improve. This follow-up concerns the remaining general buffer traffic.

The explicit typed fallible group-output allocation cannot reuse the differently typed minted-row vector through the former infallible iterator collection. Retaining the oversized input vector instead increases returned-result capacity. Preserve actionable allocation failure and output admission while investigating a broader buffer-reduction path after the remaining numeric and structural changes settle.

Acceptance:
- Identify ownership and peak overlap of input, minted and admitted output buffers through the actual production callers.
- Reduce requested traffic without increasing retained capacity or weakening typed allocation failures, governor accounting, deterministic ordering, continuation, or first-error selection.
- Use the shared instruction/allocation collectors at 1/4/32 workers and retain complete typed answers and GovernorEvidence. Include public BIND and SHACL allocation controls.
- Compare against the settled source and report all four allocation quantities and instruction counts, including any explicit tradeoff.

Evidence is archived at refs/ghprsq/pr/480/stage: raw/final-head-comparisons.tsv and final-head-qualification.md. The measured fixture verifies every one of its 5,000 exact grouped SUM outputs. Arithmetic-kernel optimization remains in #443; this issue concerns evaluator buffer ownership rather than duplicating that work.


## Comments (0)

