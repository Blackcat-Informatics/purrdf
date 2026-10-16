# Distinct-aware planning and replay: owner review

This is owner review of the uncommitted unit, not independent approval or
whole-issue acceptance. DP, greedy ordering and forecast replay now share the
same domain extension/divisor computation. They use predicate-specific subject
and object counts when available; missing data retains the former fallback.
Overlapping stream-domain sizes are explicitly upper bounds. Bounds from other
graph scopes are not invented. Structural component restrictions retain the
existing component-range row estimate.

The cost record and Solution rows do not grow. Metadata is an optional admitted
sidecar, allocated only after a concrete domain is available. DP reuses its bound
and domain masks rather than allocating another mask per subset. Binary64
arithmetic, connectivity, authored tie-breaks and original workspace refusal
remain explicit. Forecast and ordering do not treat estimates as certificates.
The existing positive-algebra caller still reaches the shared step-size home.
Its import was caught and repaired by the first focused package compilation.

The new actual-query control uses a positive shared-object fixture. At three
patterns DP changes the plan; above the DP threshold greedy changes it. Both
still return five actual rows, and replay predicts exactly five with a peak of
five. Original fixture inputs are unchanged through the explicit false setting.
The first test compile correctly rejected a non-Clone compiled pattern; it now
uses the existing pattern factory rather than adding cloning to production.

All 37 existing/new BGP module controls pass, including the frozen Binary64,
range, bag, work-ranking, cache and overflow boundaries. Two old comparisons
needed proper controls: richer metadata can choose another disconnected
component's seed, so the native plan is checked against the connectivity law at
every prefix; the original fallback order remains pinned. The work-ranking test
now compares two views with identical absent distinct metadata, varying only
work. A separate native-domain check predicts the true 700 rows for seven
drivers instead of conflating metadata availability with work. No expected
answer or connectivity requirement was weakened.

Review also found a real new API hole: the default pattern method could return
unknown for a bound subject or missing predicate before checking graph support.
The compact provider now checks its original graph/error owner first. The
targeted control passes all four subject/predicate presence combinations, with
the same typed named-graph cause. The parent's already diagnosed saturating
merged-count repair is also applied at the existing cost_base home.

Strict affected core/evaluator libraries/tests pass. Helper hygiene passes all
82 enforced jobs and 118 exact groups with 2275 paths. Only the forty existing
constructor-closure identities follow the compact guard; all authority source
identities, coordinates, classes, sinks and reasons remain unchanged. No new
exemption, duplicate body, dependency, tool change or custom compiler is added.

Whole-issue work remains: paged/segment metadata and translated sorted unions,
native rule ordering, actual EXPLAIN exactness/source observations, portable and
conformance qualification, and all five required complete demonstrations.
The profile/trace claims must be reconciled with the settled planner before
final delivery. No partial PR or follow-up ticket replaces these requirements.
