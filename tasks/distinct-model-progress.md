Distinct-aware DP, greedy ordering and estimate replay are committed and pushed
at `de36c4840` through normal hooks. They share one domain/divisor computation;
unknown domains retain the original fallback. Overlapping stream-domain sizes
remain explicit upper bounds. The optional admitted metadata sidecar does not
widen solution rows or cost records.

All 37 affected planner controls pass. A positive-answer query changes its plan
through both DP and greedy ordering while retaining all five answers; replay
predicts five rows and a peak of five. Compact graph-first refusal passes all
four subject/predicate presence combinations. Strict affected libraries/tests
and all 82 helper jobs pass. No timing acceptance or broad suite was added.

EXPLAIN source/exactness recording is now being implemented through the actual
survey and retained positive-plan callers. Paged/segment providers, translated
domain union, native rule ordering and the five complete issue demonstrations
remain required. No partial PR or whole-issue completion is claimed.
