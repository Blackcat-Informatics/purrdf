# Exact-target consumer owner review

This unit implements the exact-triple subset of Task 2. It is owner review,
not independent approval or a completed issue contract.

DatasetView's default starts from the existing graph-pruned stream and filters
the target, preserving default order and multiplicity. RdfDataset overrides
with the same target range now shared by reifiers_of and virtual quads. Arc
forwards the new seam. TripleRead routes it through the existing graph-free
target order, its ready check and original sticky first-error home. No graph
column, remap, boxed iterator or second ordinal implementation is introduced.
The shared statement traversal selects the target seam only for an unbound
reifier and bound object. Bound reifiers retain the just-landed keyed path;
other probes retain existing graph pruning and residual behavior. Ground
quoted triples compile as Pos::Bound in the actual compile_term_admitted home.

Range/graph parity through Arc passes. Compact target hits/misses and both
allowed graph scopes pass. Named-graph refusal latches even for a missing
target, and later default probes preserve that cause. All five compact carrier
tests pass. The shared traversal's cold probe retains exactly 257*4 bytes in
one allocation; later named/default/any probes allocate nothing and match the
full filtered stream. A whole-table filter could not pass the cold index
allocation assertion. Six affected BGP reifier controls pass.

Strict compilation of both affected packages' libraries and tests passes after
moving the new test module below production items; the initial items-after-test
diagnostic is retained. The actual native helper check passes after refreshing
only the existing source/constructor registrations. No new proof or exemption
was added. Indexing exact targets does not change charged candidates, since
the old traversal filtered this same exact object before its callback.

Components, annotation directions, planner/EXPLAIN, scale, portability and
whole-issue acceptance remain open. This unit is ready for normal publication
within the existing branch, not a standalone issue-closing PR.
