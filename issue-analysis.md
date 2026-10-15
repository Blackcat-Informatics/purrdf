# Current contract and source findings

The issue body and zero comments were fetched through stagectl on October 15.
Its two defects are separate acceptance obligations in one delivery.

The statement-layer evaluator dispatches bound subjects to reifier_quads_of and
annotations_of_with_graph. SegmentedSession implements both; SegmentedStackQuery
implements neither. Its annotation default is empty, and its reifier default
filters the entire unkeyed stream. This proves the missing wiring in source;
the new persisted evaluator regression is compiling to demonstrate the answer.

The stack's existing reifiers and logical_rows routines already accept a bound
ProbePattern and apply exact source translation, chronological removals,
deduplication and annotation reclassification. Both missing overrides should
call those routines, rather than implement physical stream rules again.

The shared reader State retains a dense Vec cache. Every hit scans for its
source-qualified slot and shifts entries to restore recency. Fixing only the
keyed query would leave this second defect. A replacement must preserve source
identity, original block pins, LRU eviction and exact buffer grants including
pending block layouts. Existing regressions exercise these failure boundaries.

Acceptance uses independent counts plus resident bag parity, full 200000-claim
file-backed input, exact touched-term/provider-read bounds, chronological and
named-graph equivalence, 4096-entry cache fixed work and typed refusal/recovery.
Wall-clock observations in the issue do not become performance gates.

This is the implementing session's analysis, not an independent review. No
subagents are used and no approval or completed acceptance is fabricated.
