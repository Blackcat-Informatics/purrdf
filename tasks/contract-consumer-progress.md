The first implementation slice is published at `a84f28cdd` on
`paudley/484-planner-statistics-per-predicate-per`, starting from PR #555's
published source while that predecessor finishes qualification.

Core now exposes snapshot-bound local-ID statistics with explicit exact or
upper-bound evidence. The actual SPARQL cardinality and fallback-cost consumers
read the provider, and retained order keys include its source and revision.
Unknown fields preserve the existing fallback. The selected complete BGP test
changes from authored hot-first order to rare-first order using supplied exact
statistics, preserving all 100 answers. It shares the original pattern fixture
with the existing host-work test; both controls pass. Affected core/evaluator
libraries pass strict Clippy, workspace formatting passes, and normal commit
hooks passed. No runtime dependency or physical row layout changed.

The whole issue remains open. Native/page/segment stored statistics and exact
distinct-domain merging, remaining join-selectivity/rule/EXPLAIN consumers and
all five complete acceptance demonstrations still need implementation. No PR
is opened for this initial slice. This branch continues while PR #555 qualifies;
it does not delay a ready predecessor merge.
