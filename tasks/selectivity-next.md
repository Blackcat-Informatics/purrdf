# Next production consumers

Native summaries and all three distinct-aware model computations are published
through de36c4840. All 37 affected planner controls pass; actual positive-answer
queries change both DP and greedy plans and keep five answers. Source/exactness
recording is now being implemented in the survey and retained positive-plan
callers. Keep row input exactness separate from heuristic output estimates.

DP now reuses its admitted bound/domain masks across subsets. Greedy and replay
share the domain extension/divisor home. Every shared slot uses the larger of
prior/current domains, with the intersection retained for subsequent joins.
Missing statistics retain the existing fallback and deterministic Binary64
operations/tie-breaking. Solution rows and PatternCost records do not grow.

Predicate domains may be exact for a single physical stream. Distinct counts
from overlapping streams or graphs cannot be summed and called exact. Structural
triple components must keep the existing range estimate unless the corresponding
joint statistic is actually known; the current provider_cardinality guard
preserves that. Provider fingerprint is a cache discriminator, not a portable
content identity to advertise in output.

Paged/segment construction must reuse sealed metadata, translate local IDs into
the common generation and union sorted domains without admitting data pages for
planning. Counts may be added only for disjoint effective rows. The visitor
reports malformed domain encodings as typed errors and compact requests retain
the original graph refusal. Reuse existing delta codecs, original Memory and
actual row/index paths; no cloned whole-row store or unadmitted hash table.

Required demonstrations remain the original five acceptance cases plus the
actual native rule caller, portability, conformance and final qualification.
The ten-million parent query evidence is not a substituted million-claim
statistics-estimate demonstration.
