# Current XPath foundation integration

Status: conflict resolved; qualification, normal merge-commit hooks, push and
current-head hosted/review acceptance are pending. This is not merge readiness.

The previously green PR head is 9e9e67c80. Actual `git merge --no-commit
--no-ff origin/main` against captured base 2aa091423 returned exit1 with the sole
conflicted file docs/BENCHMARKS.md. The ordinary synchronization is justified by
that real conflict, not merely branch age. The merge remains uncommitted.

Both conflicting paragraphs independently changed the benchmark counts: the
XPath branch added three narrated targets, while main updated registered target
counts. Resolution retains main's 111 registered targets and the branch's 34
narrated targets in both paragraphs. An actual exact `[[bench]]` manifest census
with rg and summed per-file counts returned111 on the combined tree. Main's
new counter/harness and governor measurement prose and the XPath benchmark
descriptions are retained. No wholesale ours/theirs resolution was used.
`git diff --cached --check` returned0 and no unresolved path remains.

Automatic merges also combine main's bounded governor worker blocks and row-buffer
reuse with the XPath-aware expression path. Main's shape-expression allocation
documentation changes from101 to85 remain. Main's additive BLAKE3 subtree helpers
and governor/numeric/parallel tests remain; no existing branch implementation was
discarded. The independent portfolio reassessment identifies40 changed main paths
since the old hosted base; hash-only evidence cannot cover this integration.

Required next evidence: actual native selected-law REGEX/REPLACE/refusal and cache
admission tests, CLI public selected-law/refusal callers, shared validation and
SHACL selected-law consumers, current governed/numeric parallel and row-buffer
regressions, affected strict clippy, required API compatibility evidence and final
independent acceptance/review-debt decisions. Unchanged grammar/tables and other
host qualification may be reused only with a source/consumer coverage argument.
Old green CI is retained as historical exact-head evidence, not qualification of
the new combined tree. The separate large-count optimization remains incomplete.

Only ghprsq may perform final integration once the actual refreshed candidate,
checks, feedback, Stage gates and selected evidence archive are ready.
