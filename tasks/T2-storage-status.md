# Component and annotation implementation status

Historical implementation checkpoint. Later execution and dispositions are in
T2-owner-review.md and the current validation.md; the pending checks below no
longer describe current acceptance.

The current production slice compiles through core, evaluator, planner and
EXPLAIN; strict affected library/test compilation passes. The published head
is still 078f9c973, and this complete Task 2 unit is not yet committed.

Native resident reifier components use seven lazy bound-mask orders with frozen
rows breaking ties. Native annotations use three predicate/object masks.
Compact reifiers share those mask helpers; compact asserted/annotation tables
retain their existing three physical orders. The generic ordinal range iterator
is shared with those tables. Cache publication and original Memory admission
now have one home for native quads and statement arrays as well.

Both warming paths and retained-byte certification cover eight u32 arrays per
reifier, including the exact-target order, plus three per native annotation.
The carrier mask/graph/order matrix passes all six cases. Exact allocation
testing passes refusal before first allocation, refusal after retaining only
the target array, healthy retry without double charge, warmed zero-allocation
probes and final release. Those receipts precede the subsequent trait/planner
additions; their unchanged storage scope remains applicable and the final
focused repetitions are executing against the complete slice.

DatasetView exposes conservative graph-pruned component candidates, exact
annotation matching, access availability and optional cheap cardinalities.
Native and compact overrides and Arc forwarding are implemented. Actual BGP
drivers resolve bound inner components through query_id; PathGraph supplies no
structural constraint. Window's separate candidate caller is adapted. Exact
nested binding and first-error handling remain at their original homes.

Planner cardinalities include statement summaries. Unknown summaries remain
unknown; nonzero statement counts select an explicit cardinality fallback
instead of inventing a conversion into a host's physical work unit. EXPLAIN
records the actual view's available ranges or scan defaults. This metadata is
inline and rendered by the same measured writer as the rest of the receipt.

Candidate work changes require profile 19, with the original eighteen prices
unchanged. Its documented independent SHA-256 recipe yields
8f256fbd39c945b841ef6c2df028881c27fe5af0f5c84c22d4b60ba5d73ef342.
Corpus receipts/digest and generated projections still require actual execution
and reconciliation; no old corpus pass is claimed on this profile.

Initial focused diagnostics are retained: an Arc oracle selected the fallible
trait resolver rather than the inherent resolver; a generic binding projector
needed its iterator lifetime; Window needed the new argument/import; tests
needed the real QueryOptions parameter and tuple-shaped explanation result.
Strict checks also exposed and resolved manual-find/byte-count lints. No hook,
checker, assertion, toolchain, cache or acceptance floor was weakened.

Required remaining acceptance: final focused native runs, current profile
corpus, portable/conformance controls, actual ten-million/six-annotation run,
fixed-work sublinear measurements, source registrations/projections, single
full gate, material source review, PR/checks/merge/closure/mirror/cleanup.
