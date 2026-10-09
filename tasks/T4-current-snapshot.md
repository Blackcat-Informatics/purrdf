# Owned current-source counterfactual

Root prepared `/opt/purrdf-308-current-telemetry.591np1/nested-dev-source`
from the resolved current staged tree
`ff72f20806a30bb7d90ea5886713a9994e7100eb`, including the qualified
`artifact_metadata(message)` correction. The shared local clone imports that
exact tree with `git read-tree` and materializes its index with
`git checkout-index --all`; it does not check out the older committed source,
overlay stale deleted paths, or create a synthetic commit.

Actual snapshot preparation session31049 exited0. Before the counterfactual,
`git diff --exit-code` exited0, proving the entire materialized tracked source
matches the imported staged tree. The sole subsequent working-tree difference
is the existing c_smoke.rs debug-assertions branch selecting `dev` instead of
`test`, identical to the production hosted counterfactual. Actual whole-tree
`git diff --name-only` identifies only that file; its full diff changes one
string. Original worktree source, index, MERGE_HEAD and refs were not altered.

The owned parent is recorded in `T4-current-telemetry-root.txt`. Actual production
doc/capi requests, runtime receipts and timing qualification remain unrun. The
snapshot is for the bounded real C caller seam, not a hosted measurement or a
new portfolio branch. Preserve it through qualification and retained evidence.
