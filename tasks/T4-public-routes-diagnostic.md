# Task 4 public parser route diagnosis

The first persistent projection implementation still excluded accepted nested
`SELECT *` queries in two public parser forms. At 128/256/512 levels, the
unchanged counting-allocator probe measured trailing `VALUES` requested bytes
of 19,254,102 / 74,166,102 / 290,944,854 and RDFLib-profile requested bytes
of 9,911,126 / 37,654,358 / 146,618,198. Both accepted doublings exceeded
the plan's strict below-3x bound. These are historical failed results, not
qualification.

The production cause was `finish_select`: trailing `VALUES` forced the old
explicit projection despite the existing ordered scope, while RDFLib's
`contextual_visible_variables` scan was still called unconditionally even
after the persistent route became eligible. The parser now appends trailing
VALUES columns to the same `VarScope` before snapshotting; it uses the
persistent projection in RDFLib mode when no contextual `MINUS` distinction
exists and only computes contextual visibility for the explicit path. A
`contextual_minus` bit initially propagated through groups; the first repair
still left MINUS-bearing RDFLib nesting on a repeated explicit projection path.
At 128/256/512 levels that route again exceeded 3x requested bytes. The current
`VarScope` carries contextual-only MINUS names with a generation marker and
publishes them at each SELECT boundary, retaining RDFLib's projection law
without copying the saved prefix.

After this correction, the public 2,000/4,000/8,000 trailing-VALUES route
requested 27,831,942 / 56,054,662 / 112,884,102 bytes and the RDFLib route
requested 18,679,174 / 37,557,126 / 75,505,030 bytes, all accepted doublings
below 3x. The selected contextual-MINUS projection parity control passed.
Final source and optimized WASM qualification are still open.

The later corrected RDFLib contextual-MINUS 2,000/4,000/8,000 parser route
requests 33,411,454 / 67,402,750 / 135,961,342 bytes. Instrumented variable
visits/copies are 26,000 / 52,000 / 104,000. Its distinct-name carrier emits
281,566 / 567,566 / 1,139,566 bytes. A separate review found that MINUS-right
reuse of a prefix variable, or a FILTER-wrapped MINUS right, defeated the
first serializer shortcut and expanded each saved projection. The renderer now
uses one outer-snapshot first-position table to distinguish retained-prefix
names from local additions and reads the actual contextual right scope. The
shared-name BGP carrier emits 276,672 / 556,672 / 1,116,672 bytes; the
FILTER-wrapped form emits 394,672 / 792,672 / 1,588,672 bytes. Both public
roundtrips and carrier requested/peak byte doublings pass below 3x through
8,000 levels. This is native scoped evidence, not final portable acceptance.

Two more material serializer findings followed that measurement. A MINUS-right
name equal to the current BIND target was appended twice, and the hidden-name
preflight revisited every earlier saved Star snapshot. Both were corrected in
the same renderer: local additions deduplicate the current BIND target, and
the outer snapshot is read once while structurally covered inner snapshots
are skipped. The current-BIND overlap emits 281,562 / 567,562 / 1,139,562
bytes, roundtrips at 8,000 levels, and the instrumented carrier variable
visits are 21,999 / 43,999 / 87,999. Public duplicate-mention visitors are
unchanged; their separate consumers require source-specific assessment.

The carrier had a separate source-consumer issue after parser correction:
`SELECT *` over a trailing-VALUES Join fell through the serializer's public-AST
rewrite guard, which recomputed each inner scope. The existing shared
projection-prefix comparison now checks an appended sequence, and the same
serializer body recognizes parser-built Join/Extend/Values snapshots. The
2,000/4,000/8,000 trailing-VALUES parse/serialize/parse control passes, retains
2×depth projected columns, and emits 316,666 / 636,666 / 1,276,666 source
bytes. The carrier window requests 6,553,728 / 13,107,328 / 26,214,528
bytes, each doubling below 3x. No second renderer or variable-view wrapper
was added.
