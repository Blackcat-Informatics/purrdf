# Review correction assessment

The two outside-diff findings in review 839234b1-45af-4654-a363-7a2eaf5c45d7
are fixed locally and await publication/material recheck. This is owner judgment,
not an independent approval. Source changes use the existing allocation and
segmented-session homes; no tool, dependency or allocation policy is replaced.

Memory::reserve now destroys an unsuccessful replacement before refunding its
admitted layout, on both allocation refusal and capacity-certificate mismatch.
The original error wins if an external admission callback itself refuses refund.
The session Budget permits decreasing an existing reservation, so its retained
bookkeeping cannot keep this rejected layout. Successful growth and original
contents/admission are unchanged. The native 64-bit standard Vec refusal control
failed with 9223372036854775820 admitted bytes instead of 13 before the fix. After
the fix all 25 allocation controls pass; the new case verifies original contents,
capacity, exact original grant, subsequent growth and final release. It uses the
standard fallible Vec interface, no injected allocator or compiler modification.
This is native allocator-refusal evidence, not a claim of forced WASM OOM.

SegmentedSession::block now checks State.error under its mutex before either a
cache hit or request/I/O preflight. This closes the ready-to-lock interleaving
and returns the original cause, rather than only preventing a later latch from
replacing it. The regression deterministically places a provider fault at that
boundary for a cache hit and each independent ceiling. It failed before the
correction; all five session ownership controls now pass. All 13 provider and
14 stack controls, and 22 neighboring public SPARQL cases pass. The full native
file-backed control passes all 2000/200000-claim populations and three fitting
layouts on corrected source. Its exact image/peak quantities match the guide.

Affected all-target warning-denied Clippy passes. The native index reports only
40 stale capacity records, each with one unchanged sink. After source review,
40 constructor-closure identities and 37 allocation-source authorities are
updated. The first guarded edit wrongly expected 40 allocation-source references
and refused to write; the actual records contain 37. The corrected structured
comparison preserves every other authority, sink, class and reason. The actual
helper gate passes all 82 enforced jobs and 118 distinct rows.

Normal commit/push and current remaining qualification are running. Completed
unaffected checks are reused only as explained in qualification-resumption.md;
interrupted full-gate runs are not passes. Current-head hosted checks, material
review, actual integration and delivery remain required.
