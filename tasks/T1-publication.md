The first implementation unit is pushed in commit `3e6bab72d`.

Resident `reifiers_of` now uses target-order bisection instead of a full scan.
It preserves reifier ordering and graph-distinct multiplicity. The graph-free
carrier and resident dataset share the existing cached ordinal builder; no
second sorting or allocation implementation was introduced. Both query warming
paths and retained-byte certification account for four bytes per reifier.

Focused validation passes: both new native library controls, the exact original
admission/refusal/retry/allocation control, all five graph-free carrier tests,
strict affected compilation, native helper check and evaluator/SHACL/ShEx
production consumer compilation. Repeated warmed hits and misses allocate
nothing; refusal precedes allocation and final release returns to zero. The
initial missing Debug-field diagnostic was fixed and the strict rerun passed.
The normal commit hooks and push passed. No timing acceptance was used.

This issue is still open. Component and annotation access through DatasetView,
statement matching, planner/EXPLAIN, the actual ten-million population,
portable/conformance acceptance and final integration remain required. No PR
has been opened for an incomplete contract.
